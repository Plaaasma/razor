mod bitboard;
mod movegen;
mod perft;
mod position;
mod types;
mod uci;
mod zobrist;

use position::Position;

fn main() {
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
            // placeholder until search exists (Phase 2): fixed perft as a
            // stable node-count signature for non-functional-change checks
            let pos = Position::startpos();
            let t = std::time::Instant::now();
            let nodes = perft::perft(&pos, 5);
            println!("Nodes searched  : {nodes}");
            println!("Time (ms)       : {}", t.elapsed().as_millis());
        }
        _ => uci::Uci::new().run(),
    }
}
