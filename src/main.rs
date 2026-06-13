mod bitboard;
mod datagen;
mod eval;
mod movegen;
mod perft;
mod position;
mod search;
mod see;
mod tt;
mod types;
mod uci;
mod zobrist;

use position::Position;

/// UCI output that tolerates a closed stdout pipe. `println!` panics (and
/// aborts) when the match runner tears the pipe down mid-print — that was the
/// entire 0xc0000409 "crash" saga of 2026-06-12.
#[macro_export]
macro_rules! send {
    ($($arg:tt)*) => {{
        use std::io::Write;
        let _ = writeln!(std::io::stdout(), $($arg)*);
    }};
}

fn main() {
    // panic messages also go to a file: engines under a match runner have no
    // visible stderr, which made the pipe-panic diagnosis painful
    std::panic::set_hook(Box::new(|info| {
        eprintln!("{info}");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(r"H:\RazorBot\logs\razor-panic.log")
        {
            use std::io::Write;
            let _ = writeln!(f, "[{}] {info}", std::process::id());
        }
    }));

    bitboard::init_attack_tables();

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("perft") => {
            let depth = args.get(2).and_then(|d| d.parse().ok()).unwrap_or(6);
            let fen = args.get(3).map(String::as_str).unwrap_or(position::START_FEN);
            let pos = Position::from_fen(fen).unwrap();
            let t = std::time::Instant::now();
            let nodes = perft::perft(&pos, depth);
            let secs = t.elapsed().as_secs_f64();
            println!("perft({depth}) = {nodes} in {secs:.2}s ({:.1}M nps)", nodes as f64 / secs / 1e6);
        }
        Some("perftsuite") => {
            if !perft::run_suite() {
                std::process::exit(1);
            }
        }
        Some("bench") => {
            // fixed-depth search over a varied position set: deterministic
            // node-count signature for non-functional-change verification
            const BENCH_DEPTH: u32 = 5;
            const BENCH_FENS: &[&str] = &[
                position::START_FEN,
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
                "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                "rnbqkb1r/pp2pppp/3p1n2/8/3NP3/8/PPP2PPP/RNBQKB1R w KQkq - 1 5",
                "4rrk1/pp1n3p/3q2pQ/2p1pb2/2PP4/2P3N1/P2B2PP/4RRK1 b - - 7 19",
                "6k1/6p1/7p/8/4B3/5K2/8/8 w - - 0 1",
                "8/8/8/5N2/8/p7/8/2NK3k w - - 0 1",
                "5k2/5p2/4B1p1/7p/7P/4PK2/8/8 w - - 0 48",
            ];
            let t = std::time::Instant::now();
            let mut total = 0u64;
            for fen in BENCH_FENS {
                let pos = Position::from_fen(fen).unwrap();
                let mut tt = tt::Tt::new(16);
                let mut s = search::Searcher::new(&mut tt);
                let mut limits = search::Limits::infinite();
                limits.depth = Some(BENCH_DEPTH);
                s.go(&pos, &limits, &[]);
                total += s.nodes;
            }
            let ms = t.elapsed().as_millis().max(1) as u64;
            println!("Nodes searched  : {total}");
            println!("Time (ms)       : {ms}");
            println!("Nodes/second    : {}", total * 1000 / ms);
        }
        Some("datagen") => {
            // datagen <out.txt> <num_positions> [seed]
            let out = args.get(2).map(String::as_str).unwrap_or("data.txt");
            let target: usize = args.get(3).and_then(|n| n.parse().ok()).unwrap_or(100_000);
            let seed: u64 = args.get(4).and_then(|n| n.parse().ok()).unwrap_or(1);
            let t = std::time::Instant::now();
            match datagen::run(out, target, seed) {
                Ok((written, games)) => {
                    let secs = t.elapsed().as_secs_f64();
                    println!(
                        "datagen done: {written} positions from {games} games in {secs:.1}s ({:.0} pos/s) -> {out}",
                        written as f64 / secs
                    );
                }
                Err(e) => {
                    eprintln!("datagen error: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => uci::Uci::new().run(),
    }
}
