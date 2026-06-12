//! UCI protocol loop. Phase 1: position handling + legal-random `go` so the
//! engine can play full games for the G1 selfplay crash test. Real search
//! replaces `pick_move` in Phase 2.

use crate::movegen::{MoveList, generate_moves};
use crate::perft;
use crate::position::Position;
use crate::types::*;
use std::io::BufRead;

pub struct Uci {
    pos: Position,
    /// xorshift state for random move selection (Phase 1 placeholder)
    rng: u64,
}

impl Uci {
    pub fn new() -> Uci {
        Uci { pos: Position::startpos(), rng: 0x1234_5678_9abc_def0 }
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
                    println!("id name Vendetta 0.1.0");
                    println!("id author Liam");
                    println!("option name Hash type spin default 16 min 1 max 4096");
                    println!("option name Threads type spin default 1 min 1 max 32");
                    println!("uciok");
                }
                "isready" => println!("readyok"),
                "ucinewgame" => self.pos = Position::startpos(),
                "setoption" => {} // no options that matter yet
                "position" => self.cmd_position(line),
                "go" => self.cmd_go(line),
                "stop" => {}
                "perft" => {
                    let depth = parts.next().and_then(|d| d.parse().ok()).unwrap_or(5);
                    perft::divide(&self.pos, depth);
                }
                "fen" => println!("{}", self.pos.to_fen()),
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

        if let Some(moves) = moves_part.trim().strip_prefix("moves") {
            for tok in moves.split_whitespace() {
                match find_uci_move(&pos, tok) {
                    Some(mv) => pos = pos.make(mv),
                    None => {
                        eprintln!("info string illegal move {tok}");
                        return;
                    }
                }
            }
        }
        self.pos = pos;
    }

    fn cmd_go(&mut self, _line: &str) {
        // Phase 1: random legal move. Time controls parsed (and honored) in Phase 2.
        let mut list = MoveList::new();
        generate_moves(&self.pos, &mut list);
        if list.len == 0 {
            println!("bestmove 0000");
            return;
        }
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        let mv = list.moves[(self.rng % list.len as u64) as usize];
        println!("bestmove {mv}");
    }
}

/// Match a UCI move string against the legal moves of `pos`.
pub fn find_uci_move(pos: &Position, s: &str) -> Option<Move> {
    let mut list = MoveList::new();
    generate_moves(pos, &mut list);
    list.iter().find(|mv| mv.to_string() == s)
}
