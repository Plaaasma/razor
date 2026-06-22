//! UCI protocol loop.
//!
//! The engine exposes the Stockfish-compatible option set that Razor can back
//! with real behaviour (see the `uci` handler). Search runs on a background
//! thread so `stop`, `ponderhit`, and `quit` stay responsive while thinking —
//! that also makes pondering work: a `go ponder` search runs unbounded until
//! `ponderhit` (which installs a time budget) or `stop` arrives.

use crate::movegen::{MoveList, generate_moves};
use crate::perft;
use crate::position::Position;
use crate::search::{Limits, SearchControl, search_with_control};
use crate::tt::Tt;
use crate::types::*;
use std::io::{BufRead, Write};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// A search running on its own thread, plus the handle the loop uses to steer it.
struct Running {
    handle: std::thread::JoinHandle<(Move, Move)>,
    ctrl: Arc<SearchControl>,
    /// true if launched as `go ponder`: a `ponderhit` should install a budget;
    /// a `stop` discards the result (GUI guessed the opponent move wrong).
    pondering: bool,
    /// time budget (ms) computed at `go ponder` time; applied on `ponderhit`.
    ponder_budget_ms: u64,
}

pub struct Uci {
    pos: Position,
    /// zobrist keys of game positions strictly before `pos`
    history: Vec<u64>,
    tt: Arc<Tt>,
    move_overhead: u64,
    threads: usize,
    multipv: usize,
    show_wdl: bool,
    ponder_enabled: bool,
    nodestime: u64,
    skill: i32,
    /// UCI_LimitStrength: when on, `skill` is derived from `uci_elo`
    limit_strength: bool,
    uci_elo: i32,
    /// optional debug log of all UCI I/O (Debug Log File option)
    log: Option<std::fs::File>,
    /// the in-flight search, if any
    running: Option<Running>,
}

impl Uci {
    pub fn new() -> Uci {
        Uci {
            pos: Position::startpos(),
            history: Vec::new(),
            tt: Arc::new(Tt::new(16)),
            move_overhead: 0,
            threads: 1,
            multipv: 1,
            show_wdl: false,
            ponder_enabled: false,
            nodestime: 0,
            skill: 20,
            limit_strength: false,
            uci_elo: 1320,
            log: None,
            running: None,
        }
    }

    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            // strip a UTF-8 BOM if a Windows shell prepended one
            let line = line.trim_start_matches('\u{feff}').trim().to_string();
            if line.is_empty() {
                continue;
            }
            self.log_io(&format!("< {line}"));
            let mut parts = line.split_whitespace();
            match parts.next().unwrap() {
                "uci" => self.cmd_uci(),
                "isready" => {
                    // a pending ponder/infinite search keeps running; isready must
                    // still answer promptly per spec
                    self.send("readyok");
                }
                "ucinewgame" => {
                    self.stop_search(true);
                    self.pos = Position::startpos();
                    self.history.clear();
                    self.tt.clear();
                }
                "setoption" => self.cmd_setoption(&line),
                "position" => {
                    self.stop_search(true);
                    self.cmd_position(&line);
                }
                "go" => self.cmd_go(&line),
                "ponderhit" => self.cmd_ponderhit(),
                "stop" => self.stop_search(false),
                "perft" => {
                    let depth = parts.next().and_then(|d| d.parse().ok()).unwrap_or(5);
                    perft::divide(&self.pos, depth);
                }
                "fen" => self.send(&self.pos.to_fen()),
                "eval" => self.send(&crate::eval::evaluate(&self.pos).to_string()),
                "quit" => {
                    self.stop_search(true);
                    break;
                }
                _ => {}
            }
        }
    }

    fn cmd_uci(&self) {
        self.send("id name Razor 0.2.0");
        self.send("id author Liam");
        // --- options Razor backs with real behaviour ---
        self.send("option name Hash type spin default 16 min 1 max 4096");
        self.send("option name Clear Hash type button");
        self.send("option name Threads type spin default 1 min 1 max 32");
        self.send("option name MultiPV type spin default 1 min 1 max 256");
        self.send("option name Ponder type check default false");
        self.send("option name Move Overhead type spin default 0 min 0 max 5000");
        self.send("option name nodestime type spin default 0 min 0 max 10000");
        self.send("option name UCI_ShowWDL type check default false");
        self.send("option name Skill Level type spin default 20 min 0 max 20");
        self.send("option name UCI_LimitStrength type check default false");
        self.send("option name UCI_Elo type spin default 1320 min 1320 max 3190");
        self.send("option name EvalFile type string default <internal>");
        self.send("option name UseNNUE type check default true");
        // SPSA-tunable search params (see tune.rs)
        self.send("option name lmrbase type spin default 75 min 0 max 300");
        self.send("option name lmrdiv type spin default 225 min 100 max 500");
        self.send("option name rfpmargin type spin default 80 min 20 max 200");
        self.send("option name futbase type spin default 80 min 0 max 300");
        self.send("option name futscale type spin default 120 min 20 max 300");
        self.send("option name semargin type spin default 200 min 50 max 500");
        // Debug Log File last: it's infrastructure, not a play-affecting knob
        self.send("option name Debug Log File type string default ");
        self.send("uciok");
    }

    fn cmd_setoption(&mut self, line: &str) {
        // setoption name <id> value <x>  (id and value may contain spaces)
        let rest: Vec<&str> = line.split_whitespace().skip(1).collect();
        let ni = rest.iter().position(|&t| t == "name");
        let vi = rest.iter().position(|&t| t == "value");
        let Some(ni) = ni else { return };
        let name = match vi {
            Some(vi) => rest[ni + 1..vi].join(" "),
            None => rest[ni + 1..].join(" "),
        };
        let value = match vi {
            Some(vi) => rest[vi + 1..].join(" "),
            None => String::new(),
        };
        let key = name.to_lowercase();
        match key.as_str() {
            "hash" => {
                if let Ok(mb) = value.parse::<usize>() {
                    self.resize_tt(mb.clamp(1, 4096));
                }
            }
            "clear hash" => self.tt.clear(),
            "threads" => {
                if let Ok(t) = value.parse::<usize>() {
                    self.threads = t.clamp(1, 32);
                }
            }
            "multipv" => {
                if let Ok(n) = value.parse::<usize>() {
                    self.multipv = n.clamp(1, 256);
                }
            }
            "ponder" => self.ponder_enabled = value.eq_ignore_ascii_case("true"),
            // accept both SF's "Move Overhead" and Razor's old "MoveOverhead"
            "move overhead" | "moveoverhead" => {
                if let Ok(ms) = value.parse::<u64>() {
                    self.move_overhead = ms.min(5000);
                }
            }
            "nodestime" => {
                if let Ok(n) = value.parse::<u64>() {
                    self.nodestime = n.min(10000);
                }
            }
            "uci_showwdl" => self.show_wdl = value.eq_ignore_ascii_case("true"),
            "skill level" => {
                if let Ok(s) = value.parse::<i32>() {
                    self.skill = s.clamp(0, 20);
                }
            }
            "uci_limitstrength" => {
                self.limit_strength = value.eq_ignore_ascii_case("true");
            }
            "uci_elo" => {
                if let Ok(e) = value.parse::<i32>() {
                    self.uci_elo = e.clamp(1320, 3190);
                }
            }
            "evalfile" => {
                let p = value.trim();
                if p.is_empty() || p.eq_ignore_ascii_case("<internal>") {
                    crate::nnue::set_eval_file("");
                } else if crate::nnue::set_eval_file(p) {
                    if crate::nnue::net_loaded() {
                        self.send("info string EvalFile set but the net is already loaded; restart to apply");
                    }
                } else {
                    self.send(&format!("info string EvalFile not found: {p}"));
                }
            }
            "usennue" => {
                let on = value.eq_ignore_ascii_case("true");
                crate::eval::USE_NNUE.store(on, Ordering::Relaxed);
            }
            "debug log file" => {
                let p = value.trim();
                self.log = if p.is_empty() {
                    None
                } else {
                    std::fs::OpenOptions::new().create(true).append(true).open(p).ok()
                };
            }
            _ => {
                // runtime-tunable search params for SPSA (no-op if id unknown)
                if let Ok(v) = value.parse::<i32>() {
                    crate::tune::set(&key, v);
                }
            }
        }
    }

    /// Effective skill 0..20: UCI_LimitStrength maps UCI_Elo onto the skill
    /// scale; otherwise the Skill Level option is used directly.
    fn effective_skill(&self) -> i32 {
        if self.limit_strength {
            // linear map of [1320,3190] Elo onto [0,20] skill
            let t = (self.uci_elo - 1320) as f64 / (3190 - 1320) as f64;
            (t * 20.0).round() as i32
        } else {
            self.skill
        }
    }

    fn cmd_position(&mut self, line: &str) {
        let line = line.strip_prefix("position").unwrap().trim();
        let (mut pos, moves_part) = if let Some(rest) = line.strip_prefix("startpos") {
            (Position::startpos(), rest)
        } else if let Some(rest) = line.strip_prefix("fen") {
            let rest = rest.trim();
            let fen_end = rest.find("moves").unwrap_or(rest.len());
            match Position::from_fen(&rest[..fen_end]) {
                Ok(p) => (p, &rest[fen_end..]),
                Err(e) => {
                    self.send(&format!("info string {e}"));
                    return;
                }
            }
        } else {
            return;
        };

        let mut history = Vec::new();
        if let Some(moves) = moves_part.trim().strip_prefix("moves") {
            for tok in moves.split_whitespace() {
                match find_uci_move(&pos, tok) {
                    Some(mv) => {
                        history.push(pos.key);
                        pos = pos.make(mv);
                    }
                    None => {
                        self.send(&format!("info string illegal move {tok}"));
                        return;
                    }
                }
            }
        }
        self.pos = pos;
        self.history = history;
    }

    fn cmd_go(&mut self, line: &str) {
        // a previous search must finish before a new one starts
        self.stop_search(true);

        let mut limits = Limits::infinite();
        limits.multipv = self.multipv;
        limits.nodestime = self.nodestime;
        limits.skill = self.effective_skill();

        let mut parts = line.split_whitespace().skip(1).peekable();
        let mut t: [Option<u64>; 4] = [None; 4];
        let mut ponder = false;
        while let Some(tok) = parts.next() {
            match tok {
                "wtime" => t[0] = next_num(&mut parts),
                "btime" => t[1] = next_num(&mut parts),
                "winc" => t[2] = next_num(&mut parts),
                "binc" => t[3] = next_num(&mut parts),
                "movestogo" => limits.movestogo = next_num(&mut parts),
                "movetime" => limits.movetime = next_num(&mut parts),
                "depth" => limits.depth = next_num(&mut parts).map(|d| d as u32),
                "nodes" => limits.nodes = next_num(&mut parts),
                "mate" => {
                    // search for a mate in N moves: cap the depth at 2N plies
                    if let Some(n) = next_num(&mut parts) {
                        limits.depth = Some((2 * n).min(crate::search::MAX_PLY as u64 - 1) as u32);
                    }
                }
                "infinite" => limits.infinite = true,
                "ponder" => {
                    ponder = true;
                    limits.infinite = true;
                }
                "searchmoves" => {
                    // remaining tokens are moves to restrict the root search to
                    while let Some(&m) = parts.peek() {
                        if let Some(mv) = find_uci_move(&self.pos, m) {
                            limits.searchmoves.push(mv);
                            parts.next();
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        limits.overhead = self.move_overhead;
        let (wtime, btime, winc, binc) = (t[0], t[1], t[2], t[3]);
        limits.time = if self.pos.stm == Color::White { wtime } else { btime };
        limits.inc = if self.pos.stm == Color::White { winc } else { binc };

        // budget to install on ponderhit: what a normal timed search would use
        let ponder_budget_ms = ponder_budget(&limits);
        // depth/nodes/movetime/timed searches block to completion below, matching
        // the classic synchronous loop; only ponder/infinite stay in the
        // background so the loop can read `ponderhit`/`stop`. Read before move.
        let background = limits.infinite;

        let ctrl = Arc::new(SearchControl::new());
        let tt = Arc::clone(&self.tt);
        let pos = self.pos;
        let history = self.history.clone();
        let threads = self.threads;
        let show_wdl = self.show_wdl;
        let ctrl_thread = Arc::clone(&ctrl);
        let handle = std::thread::spawn(move || {
            search_with_control(&tt, threads, &pos, &limits, &history, false, &ctrl_thread, show_wdl)
        });
        self.running = Some(Running { handle, ctrl, pondering: ponder, ponder_budget_ms });

        if !background {
            self.finish_search();
        }
    }

    fn cmd_ponderhit(&mut self) {
        // opponent played the predicted move: convert the running ponder search
        // into a normal timed search. The searcher rebases its own deadline to
        // (current elapsed + budget) the moment it sees the ponderhit flag, so
        // the clock runs from the hit, not from the free pondering.
        if let Some(r) = &self.running {
            if r.pondering {
                r.ctrl.ponder_budget_ms.store(r.ponder_budget_ms, Ordering::Relaxed);
                r.ctrl.ponderhit.store(true, Ordering::Relaxed);
            }
        }
        if let Some(r) = self.running.as_mut() {
            r.pondering = false;
        }
        self.finish_search();
    }

    /// Stop the in-flight search (if any). `discard` true = we don't print its
    /// bestmove (ucinewgame/position/quit teardown, or a wrong-guess ponder
    /// stop); false = a UCI `stop`, which must still emit the bestmove.
    fn stop_search(&mut self, discard: bool) {
        let Some(r) = self.running.take() else { return };
        r.ctrl.stop.store(true, Ordering::Relaxed);
        let (best, ponder) = r.handle.join().unwrap_or((MOVE_NONE, MOVE_NONE));
        if !discard {
            self.emit_bestmove(best, ponder);
        }
    }

    /// Block until the current search finishes and print its bestmove.
    fn finish_search(&mut self) {
        let Some(r) = self.running.take() else { return };
        let (best, ponder) = r.handle.join().unwrap_or((MOVE_NONE, MOVE_NONE));
        self.emit_bestmove(best, ponder);
    }

    fn emit_bestmove(&mut self, best: Move, ponder: Move) {
        if self.ponder_enabled && ponder != MOVE_NONE {
            self.send(&format!("bestmove {best} ponder {ponder}"));
        } else {
            self.send(&format!("bestmove {best}"));
        }
    }

    /// Resize the TT. Replaces the shared Arc with a freshly-sized table; safe
    /// only when no search holds a clone (we stop any running search first).
    fn resize_tt(&mut self, mb: usize) {
        self.stop_search(true);
        self.tt = Arc::new(Tt::new(mb));
    }

    fn send(&self, s: &str) {
        // tolerate a closed stdout pipe (see main.rs `send!` saga)
        let _ = writeln!(std::io::stdout(), "{s}");
        self.log_io(&format!("> {s}"));
    }

    fn log_io(&self, s: &str) {
        if let Some(mut f) = self.log.as_ref() {
            let _ = writeln!(f, "{s}");
        }
    }
}

/// Pull the next whitespace token as a u64.
fn next_num<'a, I: Iterator<Item = &'a str>>(parts: &mut std::iter::Peekable<I>) -> Option<u64> {
    parts.next().and_then(|v| v.parse::<u64>().ok())
}

/// Approximate the soft time budget a normal timed search would pick from these
/// limits, for ponderhit. Mirrors `Searcher::go`'s allocation.
fn ponder_budget(limits: &Limits) -> u64 {
    if let Some(mt) = limits.movetime {
        mt.saturating_sub(limits.overhead)
    } else if let Some(t) = limits.time {
        let inc = limits.inc.unwrap_or(0);
        let t = t.saturating_sub(limits.overhead).max(1);
        let div = limits.movestogo.map(|m| m.clamp(1, 50)).unwrap_or(25);
        let alloc = t / div + inc / 2;
        (3 * alloc).min(t / 3).max(1)
    } else {
        u64::MAX
    }
}

/// Match a UCI move string against the legal moves of `pos`.
pub fn find_uci_move(pos: &Position, s: &str) -> Option<Move> {
    let mut list = MoveList::new();
    generate_moves(pos, &mut list);
    list.iter().find(|mv| mv.to_string() == s)
}
