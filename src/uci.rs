//! UCI protocol loop.

use crate::movegen::{MoveList, generate_moves};
use crate::perft;
use crate::position::Position;
use crate::search::{Limits, Searcher};
use crate::tt::Tt;
use crate::types::*;
use std::io::BufRead;

pub struct Uci {
    pos: Position,
    /// zobrist keys of game positions strictly before `pos`
    history: Vec<u64>,
    tt: Tt,
    move_overhead: u64,
}

impl Uci {
    pub fn new() -> Uci {
        Uci { pos: Position::startpos(), history: Vec::new(), tt: Tt::new(16), move_overhead: 0 }
    }

    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            // strip a UTF-8 BOM if a Windows shell prepended one
            let line = line.trim_start_matches('\u{feff}').trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split_whitespace();
            match parts.next().unwrap() {
                "uci" => {
                    crate::send!("id name Razor 0.2.0");
                    crate::send!("id author Liam");
                    crate::send!("option name Hash type spin default 16 min 1 max 4096");
                    crate::send!("option name Threads type spin default 1 min 1 max 32");
                    crate::send!("option name MoveOverhead type spin default 0 min 0 max 1000");
                    crate::send!("option name UseNNUE type check default true");
                    crate::send!("uciok");
                }
                "isready" => crate::send!("readyok"),
                "ucinewgame" => {
                    self.pos = Position::startpos();
                    self.history.clear();
                    self.tt.clear();
                }
                "setoption" => {
                    // setoption name <id> value <x>
                    let rest: Vec<&str> = parts.collect();
                    let ni = rest.iter().position(|&t| t == "name");
                    let vi = rest.iter().position(|&t| t == "value");
                    if let (Some(ni), Some(vi)) = (ni, vi) {
                        let name = rest[ni + 1..vi].join(" ").to_lowercase();
                        let value = rest[vi + 1..].join(" ");
                        if name == "hash" {
                            if let Ok(mb) = value.parse::<usize>() {
                                self.tt.resize(mb.clamp(1, 4096));
                            }
                        } else if name == "moveoverhead" {
                            if let Ok(ms) = value.parse::<u64>() {
                                self.move_overhead = ms.min(1000);
                            }
                        } else if name == "usennue" {
                            let on = value.eq_ignore_ascii_case("true");
                            crate::eval::USE_NNUE
                                .store(on, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
                "position" => self.cmd_position(line),
                "go" => self.cmd_go(line),
                "stop" => {}
                "perft" => {
                    let depth = parts.next().and_then(|d| d.parse().ok()).unwrap_or(5);
                    perft::divide(&self.pos, depth);
                }
                "fen" => crate::send!("{}", self.pos.to_fen()),
                "eval" => crate::send!("{}", crate::eval::evaluate(&self.pos)),
                "quit" => break,
                _ => {}
            }
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
                    eprintln!("info string {e}");
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
                        eprintln!("info string illegal move {tok}");
                        return;
                    }
                }
            }
        }
        self.pos = pos;
        self.history = history;
    }

    fn cmd_go(&mut self, line: &str) {
        let mut limits = Limits::infinite();
        let mut parts = line.split_whitespace().skip(1);
        let (wtime, btime, winc, binc);
        let mut t: [Option<u64>; 4] = [None; 4];
        while let Some(tok) = parts.next() {
            let mut num = || parts.next().and_then(|v| v.parse::<u64>().ok());
            match tok {
                "wtime" => t[0] = num(),
                "btime" => t[1] = num(),
                "winc" => t[2] = num(),
                "binc" => t[3] = num(),
                "movetime" => limits.movetime = num(),
                "depth" => limits.depth = num().map(|d| d as u32),
                "nodes" => limits.nodes = num(),
                _ => {}
            }
        }
        (wtime, btime, winc, binc) = (t[0], t[1], t[2], t[3]);
        limits.overhead = self.move_overhead;
        limits.time = if self.pos.stm == Color::White { wtime } else { btime };
        limits.inc = if self.pos.stm == Color::White { winc } else { binc };

        let mut searcher = Searcher::new(&mut self.tt);
        let best = searcher.go(&self.pos, &limits, &self.history);
        crate::send!("bestmove {best}");
    }
}

/// Match a UCI move string against the legal moves of `pos`.
pub fn find_uci_move(pos: &Position, s: &str) -> Option<Move> {
    let mut list = MoveList::new();
    generate_moves(pos, &mut list);
    list.iter().find(|mv| mv.to_string() == s)
}
