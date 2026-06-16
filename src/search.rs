//! Search: negamax alpha-beta with iterative deepening, MVV-LVA ordering,
//! quiescence, transposition table. Features land one at a time, each
//! SPRT-gated (brief §5).

use crate::eval::{self, DRAW, MATE, MATE_BOUND, Score};
use crate::movegen::{MoveList, generate_moves};
use crate::position::Position;
use crate::tt::{BOUND_EXACT, BOUND_LOWER, BOUND_UPPER, Tt, score_from_tt, score_to_tt};
use crate::types::{MOVE_NONE, Move, PieceType};
use std::sync::OnceLock;
use std::time::Instant;

pub struct Limits {
    /// our clock in ms
    pub time: Option<u64>,
    /// our increment in ms
    pub inc: Option<u64>,
    pub movetime: Option<u64>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
    /// per-move communication overhead reserve in ms (UCI MoveOverhead)
    pub overhead: u64,
}

impl Limits {
    pub fn infinite() -> Limits {
        // overhead default 0: the 20ms reserve measured ≈−5 Elo at STC on an
        // idle box (SPRT timemargin2, 3157 games). Users with laggy GUIs can
        // raise the MoveOverhead UCI option.
        Limits { time: None, inc: None, movetime: None, depth: None, nodes: None, overhead: 0 }
    }
}

pub const MAX_PLY: usize = 128;

pub struct Searcher<'a> {
    pub nodes: u64,
    tt: &'a mut Tt,
    start: Instant,
    soft_limit_ms: u64,
    hard_limit_ms: u64,
    node_limit: u64,
    max_depth: u32,
    stopped: bool,
    /// zobrist keys of the positions preceding the node being visited
    keys: Vec<u64>,
    best_root: Move,
    /// root score (stm-relative cp) of the last completed iteration; used by datagen
    pub root_score: Score,
    /// suppress `info` output (datagen runs millions of searches)
    pub silent: bool,
    /// NNUE accumulator stack, one per ply (incremental update); index = ply
    acc: Vec<crate::nnue::Accumulator>,
    /// NNUE active this search (captured from the global flag at go())
    use_nnue: bool,
    /// two killer moves per ply: quiets that caused beta cutoffs
    killers: [[Move; 2]; MAX_PLY],
    /// butterfly history: [stm][from][to] cutoff counter for quiet ordering
    history: Box<[[[i32; 64]; 64]; 2]>,
    /// move excluded from search at each ply (for singular-extension verification
    /// searches); MOVE_NONE normally. Per-ply to avoid threading a param through
    /// every negamax call site.
    excluded: [Move; MAX_PLY],
}

impl<'a> Searcher<'a> {
    pub fn new(tt: &'a mut Tt) -> Searcher<'a> {
        Searcher {
            nodes: 0,
            tt,
            start: Instant::now(),
            soft_limit_ms: u64::MAX,
            hard_limit_ms: u64::MAX,
            node_limit: u64::MAX,
            max_depth: MAX_PLY as u32 - 1,
            stopped: false,
            keys: Vec::with_capacity(1024),
            best_root: MOVE_NONE,
            root_score: 0,
            silent: false,
            acc: vec![crate::nnue::Accumulator::zeroed(); MAX_PLY + 2],
            use_nnue: true,
            killers: [[MOVE_NONE; 2]; MAX_PLY],
            history: Box::new([[[0; 64]; 64]; 2]),
            excluded: [MOVE_NONE; MAX_PLY],
        }
    }

    /// `history` = zobrist keys of all game positions strictly BEFORE `pos`.
    pub fn go(&mut self, pos: &Position, limits: &Limits, history: &[u64]) -> Move {
        self.use_nnue = eval::USE_NNUE.load(std::sync::atomic::Ordering::Relaxed);
        if self.use_nnue {
            self.acc[0] = crate::nnue::Accumulator::refresh(pos);
        }
        self.start = Instant::now();
        self.nodes = 0;
        self.stopped = false;
        self.keys.clear();
        self.keys.extend_from_slice(history);
        self.best_root = MOVE_NONE;

        if let Some(mt) = limits.movetime {
            self.soft_limit_ms = mt.saturating_sub(limits.overhead);
            self.hard_limit_ms = mt.saturating_sub(limits.overhead / 2);
        } else if let Some(t) = limits.time {
            let inc = limits.inc.unwrap_or(0);
            // budget from the clock minus a communication reserve, so process
            // and GUI latency can't flag us in fast games
            let t = t.saturating_sub(limits.overhead).max(1);
            let alloc = t / 25 + inc / 2;
            self.soft_limit_ms = alloc.min(t.saturating_sub(30));
            self.hard_limit_ms = (3 * alloc).min(t / 3).max(1);
        } else {
            self.soft_limit_ms = u64::MAX;
            self.hard_limit_ms = u64::MAX;
        }
        self.node_limit = limits.nodes.unwrap_or(u64::MAX);
        self.max_depth = limits.depth.unwrap_or(MAX_PLY as u32 - 1).min(MAX_PLY as u32 - 1);

        let mut best = MOVE_NONE;
        let mut prev_score = 0;
        for depth in 1..=self.max_depth {
            // aspiration window around the previous iteration's score; widen
            // exponentially on fail until the result lands inside
            let mut delta = 25;
            let (mut alpha, mut beta) = if depth >= 4 {
                ((prev_score - delta).max(-MATE), (prev_score + delta).min(MATE))
            } else {
                (-MATE, MATE)
            };
            let score = loop {
                let s = self.negamax(pos, depth, alpha, beta, 0, false);
                if self.stopped {
                    break s;
                }
                if s <= alpha {
                    beta = (alpha + beta) / 2;
                    alpha = (s - delta).max(-MATE);
                } else if s >= beta {
                    beta = (s + delta).min(MATE);
                } else {
                    break s;
                }
                delta *= 2;
            };
            if self.stopped {
                break;
            }
            prev_score = score;
            self.root_score = score;
            best = self.best_root;
            let ms = self.start.elapsed().as_millis() as u64;
            let nps = if ms > 0 { self.nodes * 1000 / ms } else { 0 };
            if !self.silent {
                crate::send!(
                    "info depth {depth} score {} nodes {} nps {nps} time {ms} pv {best}",
                    format_score(score),
                    self.nodes
                );
            }
            if ms >= self.soft_limit_ms {
                break;
            }
        }
        if best == MOVE_NONE {
            // never finished depth 1 (extreme time pressure): pick any legal move
            let mut list = MoveList::new();
            generate_moves(pos, &mut list);
            if list.len > 0 {
                best = list.moves[0];
            }
        }
        best
    }

    fn check_limits(&mut self) {
        if self.nodes >= self.node_limit {
            self.stopped = true;
        }
        if self.nodes % 2048 == 0 && self.start.elapsed().as_millis() as u64 >= self.hard_limit_ms {
            self.stopped = true;
        }
    }

    /// Eval at the node for `ply`: NNUE from the incremental accumulator, or
    /// the hand-crafted PSQT.
    #[inline(always)]
    fn eval(&self, pos: &Position, ply: usize) -> Score {
        if self.use_nnue {
            #[cfg(debug_assertions)]
            {
                let fresh = crate::nnue::Accumulator::refresh(pos);
                debug_assert!(
                    self.acc[ply].w == fresh.w && self.acc[ply].b == fresh.b,
                    "incremental accumulator diverged from refresh at ply {ply}"
                );
            }
            self.acc[ply].eval(pos.stm)
        } else {
            eval::evaluate_psqt(pos)
        }
    }

    fn is_repetition(&self, key: u64, halfmove: u8) -> bool {
        let lookback = (halfmove as usize).min(self.keys.len());
        // same side to move only: step 2; skip the position one ply back
        self.keys
            .iter()
            .rev()
            .take(lookback)
            .skip(1)
            .step_by(2)
            .any(|&k| k == key)
    }

    fn negamax(
        &mut self,
        pos: &Position,
        depth: u32,
        mut alpha: Score,
        beta: Score,
        ply: usize,
        is_null: bool,
    ) -> Score {
        if depth == 0 || ply >= MAX_PLY - 1 {
            return self.qsearch(pos, alpha, beta, ply);
        }

        self.nodes += 1;
        self.check_limits();
        if self.stopped {
            return 0;
        }

        // draws by rule (checked at non-root nodes)
        if ply > 0 && (pos.halfmove >= 100 || self.is_repetition(pos.key, pos.halfmove)) {
            return DRAW;
        }

        // consume this ply's excluded move (set by a singular verification search)
        let excluded = self.excluded[ply];
        self.excluded[ply] = MOVE_NONE;

        // TT probe: cutoff at non-root nodes (skipped during an exclusion search),
        // move ordering + singular test everywhere
        let mut tt_mv = MOVE_NONE;
        let mut tt_score = 0;
        let mut tt_depth = 0u32;
        let mut tt_bound = 0u8;
        if let Some(e) = self.tt.probe(pos.key) {
            tt_mv = Move(e.mv);
            tt_score = score_from_tt(e.score as Score, ply);
            tt_depth = e.depth as u32;
            tt_bound = e.bound;
            if ply > 0 && excluded == MOVE_NONE && tt_depth >= depth {
                match e.bound {
                    BOUND_EXACT => return tt_score,
                    BOUND_LOWER if tt_score >= beta => return tt_score,
                    BOUND_UPPER if tt_score <= alpha => return tt_score,
                    _ => {}
                }
            }
        }

        // internal iterative reduction: with no TT move at a decent depth the
        // move ordering is poor, so search one ply shallower (cheaper; the
        // shallow result fills the TT and the next iteration re-searches well).
        let depth = if depth >= 4 && tt_mv == MOVE_NONE && excluded == MOVE_NONE {
            depth - 1
        } else {
            depth
        };

        let in_check = pos.in_check();
        let static_eval = if in_check { -MATE } else { self.eval(pos, ply) };

        // reverse futility: eval is so far above beta that even a large
        // margin per remaining ply can't bring it back down
        if !in_check
            && ply > 0
            && depth <= 8
            && beta.abs() < MATE_BOUND
            && static_eval - crate::tune::get(&crate::tune::RFP_MARGIN) * depth as Score >= beta
        {
            return static_eval;
        }

        // null-move pruning: if passing the turn still fails high at reduced
        // depth, the position is too good to need a real search. Skipped in
        // check, after another null, near mate, and without non-pawn material
        // (zugzwang guard).
        if !is_null
            && ply > 0
            && !in_check
            && depth >= 3
            && beta.abs() < MATE_BOUND
            && static_eval >= beta
            && pos.color_bb[pos.stm.idx()]
                != pos.pieces(pos.stm, PieceType::Pawn) | pos.pieces(pos.stm, PieceType::King)
        {
            let child = pos.make_null();
            if self.use_nnue {
                self.acc[ply + 1] = self.acc[ply].clone();
            }
            self.keys.push(pos.key);
            let score = -self.negamax(&child, depth - 3, -beta, -beta + 1, ply + 1, true);
            self.keys.pop();
            if self.stopped {
                return 0;
            }
            if score >= beta {
                return if score >= MATE_BOUND { beta } else { score };
            }
        }

        let mut list = MoveList::new();
        generate_moves(pos, &mut list);

        if list.len == 0 {
            return if in_check { -MATE + ply as Score } else { DRAW };
        }

        self.order_moves(pos, &mut list, tt_mv, ply);

        let alpha_orig = alpha;
        let mut best = -MATE;
        let mut best_mv = MOVE_NONE;
        let mut move_count = 0u32;
        // check extension: evasions are forced sequences, search them deeper
        let new_depth = depth - 1 + u32::from(in_check);
        self.keys.push(pos.key);
        for mv in list.iter() {
            if mv == excluded {
                continue;
            }
            move_count += 1;
            // futility: at low depth, quiets can't lift a hopeless static
            // eval back to alpha — skip them (never the first move, so mate
            // and stalemate scores stay correct)
            if move_count > 1
                && !in_check
                && depth <= 3
                && !mv.is_capture()
                && !mv.is_promo()
                && alpha.abs() < MATE_BOUND
                && static_eval
                    + crate::tune::get(&crate::tune::FUT_BASE)
                    + crate::tune::get(&crate::tune::FUT_SCALE) * depth as Score
                    <= alpha
            {
                continue;
            }
            // singular extension: if tt_mv alone holds when re-searched at
            // reduced depth with it excluded (every other move falls below
            // tt_score - margin), it's forced -> extend it. Run before make() so
            // the exclusion search's accumulator writes don't clobber acc[ply+1].
            let mut ext = 0u32;
            if depth >= 8
                && mv == tt_mv
                && excluded == MOVE_NONE
                && tt_depth >= depth - 3
                && tt_bound != BOUND_UPPER
                && tt_score.abs() < MATE_BOUND
            {
                let sbeta = tt_score - 2 * depth as Score;
                self.excluded[ply] = tt_mv;
                let s = self.negamax(pos, (depth - 1) / 2, sbeta - 1, sbeta, ply, is_null);
                self.excluded[ply] = MOVE_NONE;
                if s < sbeta {
                    ext = 1;
                }
            }
            let child = pos.make(mv);
            if self.use_nnue {
                let a = crate::nnue::apply(&self.acc[ply], pos, mv);
                self.acc[ply + 1] = a;
            }
            // PVS: full window only for the first move; the rest get a null
            // window probe (late quiets at reduced depth — LMR), re-searched
            // on fail-high
            let score = if move_count == 1 {
                -self.negamax(&child, new_depth + ext, -beta, -alpha, ply + 1, false)
            } else {
                let mut r = 0;
                if depth >= 3 && move_count > 3 && !in_check && !mv.is_capture() && !mv.is_promo()
                {
                    // log-based reduction: reduce more as depth and move index
                    // grow (the old `1 + (mc>8)` capped at 2, far too shallow at
                    // high depth). Re-searched at full depth on fail-high (below).
                    let d = (depth as usize).min(63);
                    let mc = (move_count as usize).min(63);
                    r = lmr_table()[d][mc];
                    // keep at least depth 1 after reduction
                    r = r.min(new_depth.saturating_sub(1));
                }
                let mut s =
                    -self.negamax(&child, new_depth - r, -alpha - 1, -alpha, ply + 1, false);
                if r > 0 && s > alpha && !self.stopped {
                    s = -self.negamax(&child, new_depth, -alpha - 1, -alpha, ply + 1, false);
                }
                if s > alpha && s < beta && !self.stopped {
                    s = -self.negamax(&child, new_depth, -beta, -alpha, ply + 1, false);
                }
                s
            };
            if self.stopped {
                break;
            }
            if score > best {
                best = score;
                best_mv = mv;
                if ply == 0 {
                    self.best_root = mv;
                }
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        if !mv.is_capture() {
                            if mv != self.killers[ply][0] {
                                self.killers[ply][1] = self.killers[ply][0];
                                self.killers[ply][0] = mv;
                            }
                            let h = &mut self.history[pos.stm.idx()][mv.from() as usize]
                                [mv.to() as usize];
                            *h += (depth * depth) as i32;
                            // keep history below the killer band in move ordering
                            if *h > 800_000 {
                                *h /= 2;
                            }
                        }
                        break;
                    }
                }
            }
        }
        self.keys.pop();

        // don't store during an exclusion search (the result omits a move)
        if !self.stopped && excluded == MOVE_NONE {
            let bound = if best >= beta {
                BOUND_LOWER
            } else if best > alpha_orig {
                BOUND_EXACT
            } else {
                BOUND_UPPER
            };
            self.tt.store(pos.key, best_mv, score_to_tt(best, ply), depth, bound);
        }
        best
    }

    /// Quiescence: stand pat, then captures/promotions only. When in check,
    /// search all evasions instead (no stand pat — sitting still is not an
    /// option in check).
    fn qsearch(&mut self, pos: &Position, mut alpha: Score, beta: Score, ply: usize) -> Score {
        self.nodes += 1;
        self.check_limits();
        if self.stopped {
            return 0;
        }
        if ply >= MAX_PLY - 1 {
            return self.eval(pos, ply);
        }

        let in_check = pos.in_check();
        let mut best;
        if in_check {
            best = -MATE + ply as Score;
        } else {
            best = self.eval(pos, ply);
            if best >= beta {
                return best;
            }
            if best > alpha {
                alpha = best;
            }
        }

        let mut list = MoveList::new();
        if in_check {
            generate_moves(pos, &mut list);
            if list.len == 0 {
                return -MATE + ply as Score;
            }
        } else {
            crate::movegen::generate_captures(pos, &mut list);
        }
        self.order_moves(pos, &mut list, MOVE_NONE, ply);

        for mv in list.iter() {
            // prune captures that lose material by SEE (not while in check —
            // those are evasions, and not promotions — too tactical)
            if !in_check && !mv.is_promo() && crate::see::see(pos, mv) < 0 {
                continue;
            }
            let child = pos.make(mv);
            if self.use_nnue {
                let a = crate::nnue::apply(&self.acc[ply], pos, mv);
                self.acc[ply + 1] = a;
            }
            let score = -self.qsearch(&child, -beta, -alpha, ply + 1);
            if self.stopped {
                break;
            }
            if score > best {
                best = score;
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }
        best
    }
}

impl<'a> Searcher<'a> {
    /// TT move first, then MVV-LVA captures (most valuable victim, least
    /// valuable attacker tiebreak), then killers, then quiets by history.
    fn order_moves(&self, pos: &Position, list: &mut MoveList, tt_mv: Move, ply: usize) {
        use crate::eval::PIECE_VALUE;
        use crate::types::PieceType;

        let killers = &self.killers[ply];
        let mut scores = [0i32; crate::movegen::MAX_MOVES];
        for i in 0..list.len {
            let mv = list.moves[i];
            if mv == tt_mv {
                scores[i] = 10_000_000;
            } else if mv.is_capture() {
                let victim = if mv.is_en_passant() {
                    PieceType::Pawn
                } else {
                    pos.piece_on(mv.to()).unwrap().1
                };
                let attacker = pos.piece_on(mv.from()).unwrap().1;
                scores[i] =
                    1_000_000 + 10 * PIECE_VALUE[victim.idx()] - PIECE_VALUE[attacker.idx()];
            } else if mv == killers[0] {
                scores[i] = 900_000;
            } else if mv == killers[1] {
                scores[i] = 899_999;
            } else {
                scores[i] = self.history[pos.stm.idx()][mv.from() as usize][mv.to() as usize];
            }
        }
        // insertion sort, descending, moves and scores in tandem (lists are
        // short and mostly quiet, so this beats a full sort)
        for i in 1..list.len {
            let (mv, sc) = (list.moves[i], scores[i]);
            let mut j = i;
            while j > 0 && scores[j - 1] < sc {
                list.moves[j] = list.moves[j - 1];
                scores[j] = scores[j - 1];
                j -= 1;
            }
            list.moves[j] = mv;
            scores[j] = sc;
        }
    }
}

/// Late-move-reduction amounts indexed by [depth][move_count], capped at 63.
/// r = 0.75 + ln(d)*ln(mc)/2.25 — the standard log curve: gentle early, steep
/// for late quiets at high depth. Computed once.
fn lmr_table() -> &'static [[u32; 64]; 64] {
    static LMR: OnceLock<[[u32; 64]; 64]> = OnceLock::new();
    LMR.get_or_init(|| {
        // read tunables once (SPSA sets options before the first search)
        let base = crate::tune::get(&crate::tune::LMR_BASE) as f64 / 100.0;
        let div = (crate::tune::get(&crate::tune::LMR_DIV) as f64 / 100.0).max(0.1);
        let mut t = [[0u32; 64]; 64];
        for d in 1..64 {
            for m in 1..64 {
                let r = base + (d as f64).ln() * (m as f64).ln() / div;
                t[d][m] = r.max(0.0) as u32;
            }
        }
        t
    })
}

fn format_score(s: Score) -> String {
    if s.abs() >= MATE_BOUND {
        let plies = MATE - s.abs();
        let moves = (plies + 1) / 2;
        format!("mate {}", if s > 0 { moves } else { -moves })
    } else {
        format!("cp {s}")
    }
}
