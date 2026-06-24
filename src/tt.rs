//! Transposition table: fixed-size, multiply-shift indexing, single slot per
//! index, depth-preferred replacement. LOCKLESS for SMP: each slot is ONE
//! AtomicU64, so a load/store is inherently tear-free (no Hyatt xor needed).
//! Validity uses a 16-bit key-check packed alongside the data; combined with the
//! index bits this is ~2^-42 false-positive (Stockfish-grade). All access is
//! `&self` so the table can be shared across search threads.
//!
//! Slot layout (one u64): [mv:16 | score:16 | depth:8 | bound:8 | keycheck:16].
//! A never-written slot is 0; every stored entry has depth>=1 (only negamax
//! stores, and only at depth>=1), so `data == 0` is an unambiguous empty marker.
//! At 8 bytes/slot the table holds 2x the entries of the old 16-byte design, so
//! `hashfull` rises half as fast (matching Stockfish's density at equal Hash MB).

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
    data: AtomicU64,
}

/// 16-bit key-check. The multiply-shift index is dominated by the high bits of
/// `key`, so the low 16 bits are ~independent of it — a good disambiguator.
#[inline(always)]
fn keycheck(key: u64) -> u16 {
    key as u16
}

#[inline(always)]
fn pack(kc: u16, mv: u16, score: i16, depth: u8, bound: u8) -> u64 {
    (mv as u64)
        | ((score as u16 as u64) << 16)
        | ((depth as u64) << 32)
        | ((bound as u64) << 40)
        | ((kc as u64) << 48)
}

/// Returns (keycheck, mv, score, depth, bound).
#[inline(always)]
fn unpack(d: u64) -> (u16, u16, i16, u8, u8) {
    ((d >> 48) as u16, d as u16, (d >> 16) as u16 as i16, (d >> 32) as u8, (d >> 40) as u8)
}

pub struct Tt {
    slots: Vec<Slot>,
}

impl Tt {
    pub fn new(mb: usize) -> Tt {
        let mut tt = Tt { slots: Vec::new() };
        tt.resize(mb);
        tt
    }

    pub fn resize(&mut self, mb: usize) {
        let n = (mb.max(1) * 1024 * 1024 / 8).max(1); // 8 bytes per slot
        self.slots = (0..n).map(|_| Slot { data: AtomicU64::new(0) }).collect();
    }

    /// Wipe all entries. `&self` (atomic stores) so it works on a shared table.
    pub fn clear(&self) {
        for s in &self.slots {
            s.data.store(0, Ordering::Relaxed);
        }
    }

    /// TT fill estimate in permille (0..1000), for `info hashfull`. Samples the
    /// first min(1000, len) slots (Stockfish's approach); a slot is filled iff its
    /// `data` is non-zero. Read-only atomic loads — never affects search.
    pub fn hashfull(&self) -> u32 {
        let n = self.slots.len().min(1000);
        if n == 0 {
            return 0;
        }
        let filled = self.slots[..n].iter().filter(|s| s.data.load(Ordering::Relaxed) != 0).count();
        (filled * 1000 / n) as u32
    }

    #[inline(always)]
    fn index(&self, key: u64) -> usize {
        ((key as u128 * self.slots.len() as u128) >> 64) as usize
    }

    /// Prefetch the TT slot for `key`. Call as soon as a child key is known
    /// (before the accumulator update + recursion) so the cold-slot DRAM load
    /// overlaps work that has to happen anyway. Pure hint: cannot change results,
    /// bench node count, or perft — only timing.
    #[inline(always)]
    pub fn prefetch(&self, key: u64) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let p = self.slots.as_ptr().add(self.index(key)) as *const i8;
            core::arch::x86_64::_mm_prefetch(p, core::arch::x86_64::_MM_HINT_T0);
        }
        #[cfg(not(target_arch = "x86_64"))]
        let _ = key;
    }

    #[inline(always)]
    pub fn probe(&self, key: u64) -> Option<TtEntry> {
        let d = self.slots[self.index(key)].data.load(Ordering::Relaxed);
        // d == 0 is the empty slot (every stored entry has depth>=1, so a written
        // slot is never all-zero); the keycheck then validates the position.
        if d != 0 {
            let (kc, mv, score, depth, bound) = unpack(d);
            if kc == keycheck(key) {
                return Some(TtEntry { key, mv, score, depth, bound });
            }
        }
        None
    }

    #[inline(always)]
    pub fn probe_move(&self, key: u64) -> Move {
        match self.probe(key) {
            Some(e) => Move(e.mv),
            None => MOVE_NONE,
        }
    }

    pub fn store(&self, key: u64, mv: Move, score: Score, depth: u32, bound: u8) {
        let kc = keycheck(key);
        let s = &self.slots[self.index(key)];
        let d = s.data.load(Ordering::Relaxed);
        let same = d != 0 && unpack(d).0 == kc;
        // keep deeper data for the same position unless we bring a new exact bound
        let mv = if same {
            let (_, omv, _, odepth, _) = unpack(d);
            if odepth as u32 > depth && bound != BOUND_EXACT {
                return;
            }
            // preserve the old move if the new store has none
            if mv == MOVE_NONE { Move(omv) } else { mv }
        } else {
            mv
        };
        s.data.store(pack(kc, mv.0, score as i16, depth as u8, bound), Ordering::Relaxed);
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
