//! Syzygy endgame tablebase probing, backed by the `shakmaty-syzygy` crate (the
//! one third-party dependency Razor accepts — a correct .rtbw/.rtbz decoder is
//! not worth hand-porting). Tables are loaded from the UCI `SyzygyPath` option
//! and probed at search leaves; see `search.rs` for the gating.

use crate::position::Position;
use shakmaty::fen::Fen;
use shakmaty::{CastlingMode, Chess};
use shakmaty_syzygy::{Tablebase, Wdl};
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

static TB: RwLock<Option<Tablebase<Chess>>> = RwLock::new(None);
/// Largest table (in men) actually loaded; 0 = tablebases disabled. Read on the
/// search hot path, so it is a plain atomic rather than behind the lock.
static MAX_PIECES: AtomicUsize = AtomicUsize::new(0);

/// Load tablebases from a `SyzygyPath` value (directories separated by `;` or
/// `:`). Replaces any previously loaded set. Returns the largest table in men
/// (0 if nothing usable was found).
pub fn init(path: &str) -> usize {
    let mut tb = Tablebase::new();
    // Path separator follows the platform (and Stockfish): ';' on Windows, where
    // ':' is part of the drive letter; ':' elsewhere.
    let sep = if cfg!(windows) { ';' } else { ':' };
    for dir in path
        .split(sep)
        .map(str::trim)
        .filter(|d| !d.is_empty() && !d.eq_ignore_ascii_case("<empty>"))
    {
        let _ = tb.add_directory(dir);
    }
    let max = tb.max_pieces();
    if let Ok(mut guard) = TB.write() {
        *guard = if max > 0 { Some(tb) } else { None };
    }
    MAX_PIECES.store(max, Ordering::Relaxed);
    max
}

/// Largest loaded table in men (0 when tablebases are off). Cheap hot-path gate.
#[inline(always)]
pub fn max_pieces() -> usize {
    MAX_PIECES.load(Ordering::Relaxed)
}

/// Probe WDL for a position that just had a zeroing move (halfmove clock 0), so
/// `probe_wdl_after_zeroing` is exact. Returns the result from the side-to-move
/// point of view: `Some(1)` win, `Some(-1)` loss, `Some(0)` draw. Cursed wins and
/// blessed losses collapse to a draw (not forcible within the 50-move rule).
/// `None` when no table covers the position. Caller must ensure no castling
/// rights and few enough men.
pub fn probe_wdl(pos: &Position) -> Option<i32> {
    let guard = TB.read().ok()?;
    let tb = guard.as_ref()?;
    let chess: Chess = pos
        .to_fen()
        .parse::<Fen>()
        .ok()?
        .into_position(CastlingMode::Standard)
        .ok()?;
    match tb.probe_wdl_after_zeroing(&chess).ok()? {
        Wdl::Win => Some(1),
        Wdl::Loss => Some(-1),
        _ => Some(0),
    }
}
