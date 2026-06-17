//! Transposition table: fixed-size, multiply-shift indexing, single slot per
//! index, depth-preferred replacement. LOCKLESS for SMP: each slot is two
//! AtomicU64 (key^data, data); a probe is valid iff `key ^ data == probe_key`,
//! which also detects a torn read from a concurrent writer (Hyatt's xor trick).
//! All access is `&self` so the table can be shared across search threads.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::eval::{MATE_BOUND, Score};
use crate::types::{MOVE_NONE, Move};

pub const BOUND_EXACT: u8 = 0;
pub const BOUND_LOWER: u8 = 1; // score >= beta (fail high)
pub const BOUND_UPPER: u8 = 2; // score <= alpha (fail low)

/// Unpacked entry returned by `probe`.
#[derive(Copy, Clone)]
pub struct TtEntry {
    pub key: u64,
    pub mv: u16,
    pub score: i16,
    pub depth: u8,
    pub bound: u8,
}

struct Slot {
    key: AtomicU64,
    data: AtomicU64,
}

pub struct Tt {
    slots: Vec<Slot>,
}

#[inline(always)]
fn pack(mv: u16, score: i16, depth: u8, bound: u8) -> u64 {
    (mv as u64) | ((score as u16 as u64) << 16) | ((depth as u64) << 32) | ((bound as u64) << 40)
}

#[inline(always)]
fn unpack(d: u64) -> (u16, i16, u8, u8) {
    (d as u16, (d >> 16) as u16 as i16, (d >> 32) as u8, (d >> 40) as u8)
}

impl Tt {
    pub fn new(mb: usize) -> Tt {
        let mut tt = Tt { slots: Vec::new() };
        tt.resize(mb);
        tt
    }

    pub fn resize(&mut self, mb: usize) {
        let n = (mb.max(1) * 1024 * 1024 / 16).max(1); // 16 bytes per slot
        self.slots = (0..n).map(|_| Slot { key: AtomicU64::new(0), data: AtomicU64::new(0) }).collect();
    }

    /// Wipe all entries. `&self` (atomic stores) so it works on a shared table.
    pub fn clear(&self) {
        for s in &self.slots {
            s.key.store(0, Ordering::Relaxed);
            s.data.store(0, Ordering::Relaxed);
        }
    }

    #[inline(always)]
    fn index(&self, key: u64) -> usize {
        ((key as u128 * self.slots.len() as u128) >> 64) as usize
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TtEntry> {
        let s = &self.slots[self.index(key)];
        let k = s.key.load(Ordering::Relaxed);
        let d = s.data.load(Ordering::Relaxed);
        // valid + untorn iff the stored key xor data recovers the probe key
        // (d != 0 rejects the empty slot, whose stored entries always have depth>=1)
        if d != 0 && k ^ d == key {
            let (mv, score, depth, bound) = unpack(d);
            Some(TtEntry { key, mv, score, depth, bound })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn probe_move(&self, key: u64) -> Move {
        match self.probe(key) {
            Some(e) => Move(e.mv),
            None => MOVE_NONE,
        }
    }

    pub fn store(&self, key: u64, mv: Move, score: Score, depth: u32, bound: u8) {
        let s = &self.slots[self.index(key)];
        let k = s.key.load(Ordering::Relaxed);
        let d = s.data.load(Ordering::Relaxed);
        let same = d != 0 && k ^ d == key;
        // keep deeper data for the same position unless we bring a new exact bound
        let mv = if same {
            let (omv, _, odepth, _) = unpack(d);
            if odepth as u32 > depth && bound != BOUND_EXACT {
                return;
            }
            // preserve the old move if the new store has none
            if mv == MOVE_NONE { Move(omv) } else { mv }
        } else {
            mv
        };
        let nd = pack(mv.0, score as i16, depth as u8, bound);
        s.key.store(key ^ nd, Ordering::Relaxed);
        s.data.store(nd, Ordering::Relaxed);
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
