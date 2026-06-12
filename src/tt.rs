//! Transposition table: fixed-size, power-of-two-free indexing via the
//! multiply-shift trick, single entry per slot, replace-always-if-deeper-or-new.

use crate::eval::{MATE_BOUND, Score};
use crate::types::{MOVE_NONE, Move};

pub const BOUND_EXACT: u8 = 0;
pub const BOUND_LOWER: u8 = 1; // score >= beta (fail high)
pub const BOUND_UPPER: u8 = 2; // score <= alpha (fail low)

#[derive(Copy, Clone, Default)]
pub struct TtEntry {
    pub key: u64,
    pub mv: u16,
    pub score: i16,
    pub depth: u8,
    pub bound: u8,
}

pub struct Tt {
    entries: Vec<TtEntry>,
}

impl Tt {
    pub fn new(mb: usize) -> Tt {
        let mut tt = Tt { entries: Vec::new() };
        tt.resize(mb);
        tt
    }

    pub fn resize(&mut self, mb: usize) {
        let n = mb.max(1) * 1024 * 1024 / size_of::<TtEntry>();
        self.entries = vec![TtEntry::default(); n];
    }

    pub fn clear(&mut self) {
        self.entries.fill(TtEntry::default());
    }

    #[inline(always)]
    fn index(&self, key: u64) -> usize {
        ((key as u128 * self.entries.len() as u128) >> 64) as usize
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TtEntry> {
        let e = self.entries[self.index(key)];
        if e.key == key { Some(e) } else { None }
    }

    #[inline(always)]
    pub fn probe_move(&self, key: u64) -> Move {
        match self.probe(key) {
            Some(e) => Move(e.mv),
            None => MOVE_NONE,
        }
    }

    pub fn store(&mut self, key: u64, mv: Move, score: Score, depth: u32, bound: u8) {
        let idx = self.index(key);
        let e = &mut self.entries[idx];
        // keep deeper data for the same position unless we bring a new exact bound
        if e.key == key && e.depth as u32 > depth && bound != BOUND_EXACT {
            return;
        }
        // preserve the old move if the new store has none
        let mv = if mv == MOVE_NONE && e.key == key { Move(e.mv) } else { mv };
        *e = TtEntry { key, mv: mv.0, score: score as i16, depth: depth as u8, bound };
    }
}

/// Mate scores are ply-relative at the node; make them root-relative for
/// storage and back again on probe.
#[inline(always)]
pub fn score_to_tt(s: Score, ply: usize) -> Score {
    if s >= MATE_BOUND {
        s + ply as Score
    } else if s <= -MATE_BOUND {
        s - ply as Score
    } else {
        s
    }
}

#[inline(always)]
pub fn score_from_tt(s: Score, ply: usize) -> Score {
    if s >= MATE_BOUND {
        s - ply as Score
    } else if s <= -MATE_BOUND {
        s + ply as Score
    } else {
        s
    }
}
