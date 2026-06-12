//! Perft validation (brief §4.2): exact node counts or stop-the-world.

use crate::movegen::{MoveList, generate_moves};
use crate::position::Position;

pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let mut list = MoveList::new();
    generate_moves(pos, &mut list);
    if depth == 1 {
        return list.len as u64; // bulk counting
    }
    let mut nodes = 0;
    for mv in list.iter() {
        nodes += perft(&pos.make(mv), depth - 1);
    }
    nodes
}

pub fn divide(pos: &Position, depth: u32) {
    let mut list = MoveList::new();
    generate_moves(pos, &mut list);
    let mut total = 0;
    for mv in list.iter() {
        let n = if depth <= 1 { 1 } else { perft(&pos.make(mv), depth - 1) };
        total += n;
        println!("{mv}: {n}");
    }
    println!("total: {total}");
}

/// (fen, depth, expected) — standard CPW suite plus tricky edge cases
/// (en-passant pins/checks, castling checks, promotions, stalemates).
pub const PERFT_SUITE: &[(&str, u32, u64)] = &[
    // CPW standard positions
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 6, 119_060_324),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", 5, 193_690_690),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 7, 178_633_661),
    ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", 6, 706_045_033),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", 5, 89_941_194),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", 6, 6_923_051_137),
    // edge cases (TalkChess/Peter Ellis Jones set)
    ("3k4/3p4/8/K1P4r/8/8/8/8 b - - 0 1", 6, 1_134_888),       // ep discovered check
    ("8/8/4k3/8/2p5/8/B2P2K1/8 w - - 0 1", 6, 1_015_133),      // ep pin
    ("8/8/1k6/2b5/2pP4/8/5K2/8 b - d3 0 1", 6, 1_440_467),     // ep capture checks
    ("5k2/8/8/8/8/8/8/4K2R w K - 0 1", 6, 661_072),            // short castle check
    ("3k4/8/8/8/8/8/8/R3K3 w Q - 0 1", 6, 803_711),            // long castle check
    ("r3k2r/1b4bq/8/8/8/8/7B/R3K2R w KQkq - 0 1", 4, 1_274_206), // castling rights
    ("r3k2r/8/3Q4/8/8/5q2/8/R3K2R b KQkq - 0 1", 4, 1_720_476),  // castling prevented
    ("2K2r2/4P3/8/8/8/8/8/3k4 w - - 0 1", 6, 3_821_001),       // promote out of check
    ("8/8/1P2K3/8/2n5/1q6/8/5k2 b - - 0 1", 5, 1_004_658),     // discovered check
    ("4k3/1P6/8/8/8/8/K7/8 w - - 0 1", 6, 217_342),            // promote to check
    ("8/P1k5/K7/8/8/8/8/8 w - - 0 1", 6, 92_683),              // underpromotion
    ("K1k5/8/P7/8/8/8/8/8 w - - 0 1", 6, 2_217),               // self stalemate
    ("8/k1P5/8/1K6/8/8/8/8 w - - 0 1", 7, 567_584),            // stalemate & checkmate
    ("8/8/2k5/5q2/5n2/8/5K2/8 b - - 0 1", 4, 23_527),          // double check
];

/// Run the full suite. Returns true iff every count is exact.
pub fn run_suite() -> bool {
    let mut ok = true;
    let start = std::time::Instant::now();
    let mut total_nodes = 0u64;
    for &(fen, depth, expected) in PERFT_SUITE {
        let pos = Position::from_fen(fen).unwrap();
        let t = std::time::Instant::now();
        let got = perft(&pos, depth);
        total_nodes += got;
        let status = if got == expected { "ok  " } else { "FAIL" };
        if got != expected {
            ok = false;
        }
        println!(
            "{status} d{depth} {got:>13} (want {expected:>13}) {:>7.2}s  {fen}",
            t.elapsed().as_secs_f64()
        );
    }
    let secs = start.elapsed().as_secs_f64();
    println!(
        "suite {} in {secs:.1}s, {total_nodes} nodes, {:.1}M nps",
        if ok { "PASSED" } else { "FAILED" },
        total_nodes as f64 / secs / 1e6
    );
    ok
}
