//! Training-data generation (Phase 3). Self-play from randomized openings,
//! each position labeled with a fixed-node search score and the eventual game
//! result (WDL), both from the side-to-move's perspective.
//!
//! Output (text, one position per line, bullet-ingestible):
//!   `<fen> | <score_cp> | <result>`
//! where score_cp and result are WHITE-relative (bullet's `convert --from text`
//! requirement): result ∈ {1.0 win, 0.5 draw, 0.0 loss} for white.

use crate::eval::{MATE_BOUND, Score};
use crate::movegen::{MoveList, generate_moves};
use crate::position::Position;
use crate::search::{Limits, Searcher};
use crate::tt::Tt;
use crate::types::Color;
use std::io::{BufWriter, Write};

/// Deterministic SplitMix64 — reproducible self-play from a seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const NODES_PER_MOVE: u64 = 5_000;
const MAX_GAME_PLIES: usize = 400;
const RANDOM_PLIES_MIN: usize = 4;
const RANDOM_PLIES_MAX: usize = 8;
/// adjudicate a win after this many consecutive plies above the score cutoff
const ADJ_WIN_PLIES: usize = 6;
const ADJ_WIN_CP: Score = 2500;

struct Sample {
    fen: String,
    score: Score,
    stm: Color,
}

/// Returns (samples_written, games_played).
pub fn run(out_path: &str, target: usize, seed: u64) -> std::io::Result<(usize, usize)> {
    let file = std::fs::File::create(out_path)?;
    let mut w = BufWriter::new(file);
    let mut rng = Rng(seed ^ 0xda7a_9e0e_da7a_9e0e);

    let tt = Tt::new(16);
    let mut written = 0usize;
    let mut games = 0usize;
    let start = std::time::Instant::now();

    while written < target {
        tt.clear();
        let (samples, white_points) = play_game(&tt, &mut rng);
        games += 1;

        for s in &samples {
            // bullet wants white-relative score AND result
            let white_cp = if s.stm == Color::White { s.score } else { -s.score };
            writeln!(w, "{} | {} | {:.1}", s.fen, white_cp, white_points)?;
            written += 1;
        }

        if games % 50 == 0 {
            let secs = start.elapsed().as_secs_f64().max(0.001);
            eprintln!(
                "datagen: {written} positions, {games} games, {:.0} pos/s",
                written as f64 / secs
            );
            w.flush()?;
        }
    }
    w.flush()?;
    Ok((written, games))
}

/// Play one self-play game. Returns (filtered samples, white game points in
/// {1.0, 0.5, 0.0}).
fn play_game(tt: &Tt, rng: &mut Rng) -> (Vec<Sample>, f64) {
    let mut pos = Position::startpos();
    let mut history: Vec<u64> = Vec::new();
    let mut samples: Vec<Sample> = Vec::new();

    // randomized opening: a few uniformly random legal moves for diversity
    let random_plies = RANDOM_PLIES_MIN + rng.below(RANDOM_PLIES_MAX - RANDOM_PLIES_MIN + 1);
    for _ in 0..random_plies {
        let mut list = MoveList::new();
        generate_moves(&pos, &mut list);
        if list.len == 0 {
            return (samples, 0.5); // dead opening, discard as draw
        }
        history.push(pos.key);
        pos = pos.make(list.moves[rng.below(list.len)]);
    }

    let mut white_points = 0.5;
    let mut adj_count = 0usize;
    let mut adj_sign = 0i32;

    for _ply in 0..MAX_GAME_PLIES {
        let mut list = MoveList::new();
        generate_moves(&pos, &mut list);
        if list.len == 0 {
            // checkmate or stalemate
            white_points = if pos.in_check() {
                if pos.stm == Color::White { 0.0 } else { 1.0 }
            } else {
                0.5
            };
            break;
        }
        if pos.halfmove >= 100 {
            white_points = 0.5;
            break;
        }

        let mut s = Searcher::new(tt);
        s.silent = true;
        let mut limits = Limits::infinite();
        limits.nodes = Some(NODES_PER_MOVE);
        let best = s.go(&pos, &limits, &history);
        let score = s.root_score; // stm-relative cp

        // filter for clean training positions: not in check, best move quiet,
        // score not a (near-)mate. These are the positions an NNUE learns from.
        let in_check = pos.in_check();
        if !in_check && !best.is_capture() && !best.is_promo() && score.abs() < MATE_BOUND {
            samples.push(Sample { fen: pos.to_fen(), score, stm: pos.stm });
        }

        // win adjudication from white's perspective
        let white_cp = if pos.stm == Color::White { score } else { -score };
        let sign = if white_cp > ADJ_WIN_CP {
            1
        } else if white_cp < -ADJ_WIN_CP {
            -1
        } else {
            0
        };
        if sign != 0 && sign == adj_sign {
            adj_count += 1;
        } else {
            adj_count = if sign != 0 { 1 } else { 0 };
            adj_sign = sign;
        }
        if adj_count >= ADJ_WIN_PLIES {
            white_points = if adj_sign > 0 { 1.0 } else { 0.0 };
            break;
        }

        history.push(pos.key);
        pos = pos.make(best);
    }

    (samples, white_points)
}
