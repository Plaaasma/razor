//! Minimal correct search: full-width negamax alpha-beta with iterative
//! deepening and time management. Deliberately featureless — the brief's §5
//! ladder adds one SPRT-gated improvement at a time on top of this baseline.

use crate::eval::{self, DRAW, MATE, MATE_BOUND, Score};
use crate::movegen::{MoveList, generate_moves};
use crate::position::Position;
use crate::types::{MOVE_NONE, Move};
use std::time::Instant;

pub struct Limits {
    /// our clock in ms
    pub time: Option<u64>,
    /// our increment in ms
    pub inc: Option<u64>,
    pub movetime: Option<u64>,
    pub depth: Option<u32>,
    pub nodes: Option<u64>,
}

impl Limits {
    pub fn infinite() -> Limits {
        Limits { time: None, inc: None, movetime: None, depth: None, nodes: None }
    }
}

pub const MAX_PLY: usize = 128;

pub struct Searcher {
    pub nodes: u64,
    start: Instant,
    soft_limit_ms: u64,
    hard_limit_ms: u64,
    node_limit: u64,
    max_depth: u32,
    stopped: bool,
    /// zobrist keys of the game so far + search path, for repetition detection
    keys: Vec<u64>,
    /// plies since last irreversible move, aligned with `keys`
    best_root: Move,
}

impl Searcher {
    pub fn new() -> Searcher {
        Searcher {
            nodes: 0,
            start: Instant::now(),
            soft_limit_ms: u64::MAX,
            hard_limit_ms: u64::MAX,
            node_limit: u64::MAX,
            max_depth: MAX_PLY as u32 - 1,
            stopped: false,
            keys: Vec::with_capacity(1024),
            best_root: MOVE_NONE,
        }
    }

    /// `history` = zobrist keys of all game positions strictly BEFORE `pos`
    /// (invariant throughout the search: `self.keys` holds the positions
    /// preceding the node being visited).
    pub fn go(&mut self, pos: &Position, limits: &Limits, history: &[u64]) -> Move {
        self.start = Instant::now();
        self.nodes = 0;
        self.stopped = false;
        self.keys.clear();
        self.keys.extend_from_slice(history);
        self.best_root = MOVE_NONE;

        // time allocation: soft = when not to start a new iteration,
        // hard = abort mid-search
        if let Some(mt) = limits.movetime {
            self.soft_limit_ms = mt.saturating_sub(20);
            self.hard_limit_ms = mt.saturating_sub(10);
        } else if let Some(t) = limits.time {
            let inc = limits.inc.unwrap_or(0);
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
        for depth in 1..=self.max_depth {
            let score = self.negamax(pos, depth, -MATE, MATE, 0);
            if self.stopped {
                break;
            }
            best = self.best_root;
            let ms = self.start.elapsed().as_millis() as u64;
            let nps = if ms > 0 { self.nodes * 1000 / ms } else { 0 };
            println!(
                "info depth {depth} score {} nodes {} nps {nps} time {ms} pv {best}",
                format_score(score),
                self.nodes
            );
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

    fn is_repetition(&self, key: u64, halfmove: u8) -> bool {
        let lookback = (halfmove as usize).min(self.keys.len());
        // same side to move only: step 2; skip the current position itself
        self.keys
            .iter()
            .rev()
            .take(lookback)
            .skip(1)
            .step_by(2)
            .any(|&k| k == key)
    }

    fn negamax(&mut self, pos: &Position, depth: u32, mut alpha: Score, beta: Score, ply: usize) -> Score {
        if depth == 0 || ply >= MAX_PLY - 1 {
            return eval::evaluate(pos);
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

        let mut list = MoveList::new();
        generate_moves(pos, &mut list);

        if list.len == 0 {
            return if pos.in_check() { -MATE + ply as Score } else { DRAW };
        }

        let mut best = -MATE;
        self.keys.push(pos.key);
        for mv in list.iter() {
            let child = pos.make(mv);
            let score = -self.negamax(&child, depth - 1, -beta, -alpha, ply + 1);
            if self.stopped {
                break;
            }
            if score > best {
                best = score;
                if ply == 0 {
                    self.best_root = mv;
                }
                if score > alpha {
                    alpha = score;
                    if alpha >= beta {
                        break;
                    }
                }
            }
        }
        self.keys.pop();
        best
    }
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
