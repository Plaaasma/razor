//! Bitboards and attack generation. Sliders use PEXT (BMI2) indexing into
//! tables built once at startup; the 13900K has fast PEXT/PDEP. A portable
//! fallback computes the same index by masked multiplication-free packing
//! (slow path, only used when BMI2 is unavailable, e.g. the Spark's aarch64
//! build — NEON path revisited in Phase 3).

use crate::types::{Color, Square, file_of, rank_of, square};
use std::sync::OnceLock;

pub type Bitboard = u64;

pub const FILE_A: Bitboard = 0x0101_0101_0101_0101;
pub const FILE_H: Bitboard = FILE_A << 7;
pub const RANK_1: Bitboard = 0xff;
pub const RANK_2: Bitboard = RANK_1 << 8;
pub const RANK_7: Bitboard = RANK_1 << 48;
pub const RANK_8: Bitboard = RANK_1 << 56;

#[inline(always)]
pub const fn bb(sq: Square) -> Bitboard {
    1u64 << sq
}

#[inline(always)]
pub fn lsb(b: Bitboard) -> Square {
    debug_assert!(b != 0);
    b.trailing_zeros() as Square
}

#[inline(always)]
pub fn pop_lsb(b: &mut Bitboard) -> Square {
    let sq = lsb(*b);
    *b &= *b - 1;
    sq
}

/// Iterate over set squares of a bitboard.
pub struct BitIter(pub Bitboard);

impl Iterator for BitIter {
    type Item = Square;

    #[inline(always)]
    fn next(&mut self) -> Option<Square> {
        if self.0 == 0 {
            None
        } else {
            Some(pop_lsb(&mut self.0))
        }
    }
}

#[inline(always)]
pub fn shift_north(b: Bitboard) -> Bitboard {
    b << 8
}

#[inline(always)]
pub fn shift_south(b: Bitboard) -> Bitboard {
    b >> 8
}

// ---------------------------------------------------------------------------
// Leaper attacks (computed at compile time)
// ---------------------------------------------------------------------------

const fn leaper_attacks(sq: u8, deltas: &[(i8, i8)]) -> Bitboard {
    let f = (sq & 7) as i8;
    let r = (sq >> 3) as i8;
    let mut acc = 0u64;
    let mut i = 0;
    while i < deltas.len() {
        let (df, dr) = deltas[i];
        let nf = f + df;
        let nr = r + dr;
        if nf >= 0 && nf < 8 && nr >= 0 && nr < 8 {
            acc |= 1u64 << (nr * 8 + nf);
        }
        i += 1;
    }
    acc
}

const fn build_leaper_table(deltas: &[(i8, i8)]) -> [Bitboard; 64] {
    let mut t = [0u64; 64];
    let mut sq = 0;
    while sq < 64 {
        t[sq] = leaper_attacks(sq as u8, deltas);
        sq += 1;
    }
    t
}

pub static KNIGHT_ATTACKS: [Bitboard; 64] = build_leaper_table(&[
    (1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2),
]);

pub static KING_ATTACKS: [Bitboard; 64] = build_leaper_table(&[
    (0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1),
]);

const fn build_pawn_attacks(color: usize) -> [Bitboard; 64] {
    let mut t = [0u64; 64];
    let mut sq = 0;
    while sq < 64 {
        t[sq] = if color == 0 {
            leaper_attacks(sq as u8, &[(-1, 1), (1, 1)])
        } else {
            leaper_attacks(sq as u8, &[(-1, -1), (1, -1)])
        };
        sq += 1;
    }
    t
}

pub static PAWN_ATTACKS: [[Bitboard; 64]; 2] = [build_pawn_attacks(0), build_pawn_attacks(1)];

#[inline(always)]
pub fn pawn_attacks(c: Color, sq: Square) -> Bitboard {
    PAWN_ATTACKS[c.idx()][sq as usize]
}

// ---------------------------------------------------------------------------
// Slider attacks: PEXT tables
// ---------------------------------------------------------------------------

const ROOK_DELTAS: [(i8, i8); 4] = [(0, 1), (1, 0), (0, -1), (-1, 0)];
const BISHOP_DELTAS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, -1), (-1, 1)];

/// Slider attacks by ray walking — used to build tables and as reference.
fn slider_attacks_slow(sq: Square, occ: Bitboard, deltas: &[(i8, i8)]) -> Bitboard {
    let f = file_of(sq) as i8;
    let r = rank_of(sq) as i8;
    let mut acc = 0u64;
    for &(df, dr) in deltas {
        let (mut nf, mut nr) = (f + df, r + dr);
        while (0..8).contains(&nf) && (0..8).contains(&nr) {
            let nsq = square(nf as u8, nr as u8);
            acc |= bb(nsq);
            if occ & bb(nsq) != 0 {
                break;
            }
            nf += df;
            nr += dr;
        }
    }
    acc
}

/// Relevant occupancy mask: rays excluding board-edge endpoints.
fn relevant_mask(sq: Square, deltas: &[(i8, i8)]) -> Bitboard {
    let f = file_of(sq) as i8;
    let r = rank_of(sq) as i8;
    let mut acc = 0u64;
    for &(df, dr) in deltas {
        let (mut nf, mut nr) = (f + df, r + dr);
        while (0..8).contains(&(nf + df)) && (0..8).contains(&(nr + dr)) {
            acc |= bb(square(nf as u8, nr as u8));
            nf += df;
            nr += dr;
        }
    }
    acc
}

struct PextTable {
    masks: [Bitboard; 64],
    offsets: [u32; 64],
    attacks: Vec<Bitboard>,
}

fn build_pext_table(deltas: &[(i8, i8)]) -> PextTable {
    let mut masks = [0u64; 64];
    let mut offsets = [0u32; 64];
    let mut attacks = Vec::new();
    for sq in 0..64u8 {
        masks[sq as usize] = relevant_mask(sq, deltas);
        offsets[sq as usize] = attacks.len() as u32;
        let mask = masks[sq as usize];
        let bits = mask.count_ones();
        // enumerate all subsets of mask in pext order (index = pext(occ, mask))
        for idx in 0..(1u64 << bits) {
            let occ = deposit_bits(idx, mask);
            attacks.push(slider_attacks_slow(sq, occ, deltas));
        }
    }
    PextTable { masks, offsets, attacks }
}

/// Software PDEP: scatter the low bits of `val` into the set positions of `mask`.
fn deposit_bits(mut val: u64, mut mask: u64) -> u64 {
    let mut res = 0u64;
    while mask != 0 {
        let low = mask & mask.wrapping_neg();
        if val & 1 != 0 {
            res |= low;
        }
        mask ^= low;
        val >>= 1;
    }
    res
}

#[inline(always)]
fn pext(val: u64, mask: u64) -> u64 {
    #[cfg(target_feature = "bmi2")]
    unsafe {
        std::arch::x86_64::_pext_u64(val, mask)
    }
    #[cfg(not(target_feature = "bmi2"))]
    {
        // portable extract-bits fallback
        let mut res = 0u64;
        let mut m = mask;
        let mut i = 0;
        while m != 0 {
            let low = m & m.wrapping_neg();
            if val & low != 0 {
                res |= 1 << i;
            }
            m ^= low;
            i += 1;
        }
        res
    }
}

static ROOK_TABLE: OnceLock<PextTable> = OnceLock::new();
static BISHOP_TABLE: OnceLock<PextTable> = OnceLock::new();

/// Must be called once at startup before any attack lookups.
pub fn init_attack_tables() {
    ROOK_TABLE.get_or_init(|| build_pext_table(&ROOK_DELTAS));
    BISHOP_TABLE.get_or_init(|| build_pext_table(&BISHOP_DELTAS));
    LINE_BETWEEN.get_or_init(build_between);
    LINE_THROUGH.get_or_init(build_through);
    LINE_RAY_PAST.get_or_init(build_ray_past);
}

#[inline(always)]
pub fn rook_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    let t = ROOK_TABLE.get().unwrap();
    let i = t.offsets[sq as usize] as usize + pext(occ, t.masks[sq as usize]) as usize;
    t.attacks[i]
}

#[inline(always)]
pub fn bishop_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    let t = BISHOP_TABLE.get().unwrap();
    let i = t.offsets[sq as usize] as usize + pext(occ, t.masks[sq as usize]) as usize;
    t.attacks[i]
}

#[inline(always)]
pub fn queen_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    rook_attacks(sq, occ) | bishop_attacks(sq, occ)
}

// ---------------------------------------------------------------------------
// Between / through lines (for pins and check evasion)
// ---------------------------------------------------------------------------

static LINE_BETWEEN: OnceLock<Box<[[Bitboard; 64]; 64]>> = OnceLock::new();
static LINE_THROUGH: OnceLock<Box<[[Bitboard; 64]; 64]>> = OnceLock::new();
static LINE_RAY_PAST: OnceLock<Box<[[Bitboard; 64]; 64]>> = OnceLock::new();

fn build_between() -> Box<[[Bitboard; 64]; 64]> {
    let mut t = vec![[0u64; 64]; 64];
    for a in 0..64u8 {
        for b in 0..64u8 {
            for deltas in [&ROOK_DELTAS, &BISHOP_DELTAS] {
                if slider_attacks_slow(a, 0, deltas) & bb(b) != 0 {
                    t[a as usize][b as usize] =
                        slider_attacks_slow(a, bb(b), deltas) & slider_attacks_slow(b, bb(a), deltas);
                }
            }
        }
    }
    t.into_boxed_slice().try_into().unwrap()
}

fn build_through() -> Box<[[Bitboard; 64]; 64]> {
    let mut t = vec![[0u64; 64]; 64];
    for a in 0..64u8 {
        for b in 0..64u8 {
            for deltas in [&ROOK_DELTAS, &BISHOP_DELTAS] {
                if slider_attacks_slow(a, 0, deltas) & bb(b) != 0 {
                    t[a as usize][b as usize] = (slider_attacks_slow(a, 0, deltas)
                        & slider_attacks_slow(b, 0, deltas))
                        | bb(a)
                        | bb(b);
                }
            }
        }
    }
    t.into_boxed_slice().try_into().unwrap()
}

/// Squares strictly BEYOND b on the a->b ray (the shadow b casts away from a),
/// if aligned, else empty. = a's empty-board ray minus its b-blocked ray.
/// Used by incremental NNUE threats for discovered/blocked slider rays.
fn build_ray_past() -> Box<[[Bitboard; 64]; 64]> {
    let mut t = vec![[0u64; 64]; 64];
    for a in 0..64u8 {
        for b in 0..64u8 {
            for deltas in [&ROOK_DELTAS, &BISHOP_DELTAS] {
                if slider_attacks_slow(a, 0, deltas) & bb(b) != 0 {
                    let full = slider_attacks_slow(a, 0, deltas);
                    let blocked = slider_attacks_slow(a, bb(b), deltas);
                    t[a as usize][b as usize] = full & !blocked;
                }
            }
        }
    }
    t.into_boxed_slice().try_into().unwrap()
}

/// Squares strictly beyond b on the a->b ray, else empty.
#[inline(always)]
pub fn ray_past(a: Square, b: Square) -> Bitboard {
    LINE_RAY_PAST.get().unwrap()[a as usize][b as usize]
}

/// Squares strictly between a and b if aligned, else empty.
#[inline(always)]
pub fn between(a: Square, b: Square) -> Bitboard {
    LINE_BETWEEN.get().unwrap()[a as usize][b as usize]
}

/// Full line through a and b (inclusive) if aligned, else empty.
#[inline(always)]
pub fn line_through(a: Square, b: Square) -> Bitboard {
    LINE_THROUGH.get().unwrap()[a as usize][b as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pext_tables_match_slow_path() {
        init_attack_tables();
        // spot-check a handful of squares with pseudo-random occupancies
        let mut state = 0x9e3779b97f4a7c15u64;
        for sq in [0u8, 7, 27, 36, 56, 63] {
            for _ in 0..1000 {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let occ = state & state.rotate_left(31);
                assert_eq!(rook_attacks(sq, occ), slider_attacks_slow(sq, occ, &ROOK_DELTAS));
                assert_eq!(bishop_attacks(sq, occ), slider_attacks_slow(sq, occ, &BISHOP_DELTAS));
            }
        }
    }

    #[test]
    fn ray_past_sanity() {
        init_attack_tables();
        use crate::types::parse_square as p;
        let (a1, a4) = (p("a1").unwrap(), p("a4").unwrap());
        // beyond a4 on the a1->a4 ray = a5,a6,a7,a8
        assert_eq!(ray_past(a1, a4).count_ones(), 4);
        assert_eq!(ray_past(a1, a4) & bb(p("a5").unwrap()), bb(p("a5").unwrap()));
        assert_eq!(ray_past(a1, a4) & bb(p("a3").unwrap()), 0); // a3 is between, not beyond
        // a1 is the edge away from a4 -> nothing beyond
        assert_eq!(ray_past(a4, a1), 0);
        // diagonal: beyond c3 on a1->c3 ray = d4,e5,f6,g7,h8
        let (a1d, c3) = (p("a1").unwrap(), p("c3").unwrap());
        assert_eq!(ray_past(a1d, c3).count_ones(), 5);
        // non-aligned -> empty
        assert_eq!(ray_past(p("a1").unwrap(), p("b3").unwrap()), 0);
    }

    #[test]
    fn between_sanity() {
        init_attack_tables();
        use crate::types::parse_square as p;
        let (a1, h8) = (p("a1").unwrap(), p("h8").unwrap());
        assert_eq!(between(a1, h8).count_ones(), 6);
        let (e1, e8) = (p("e1").unwrap(), p("e8").unwrap());
        assert_eq!(between(e1, e8).count_ones(), 6);
        let (a1, b3) = (p("a1").unwrap(), p("b3").unwrap());
        assert_eq!(between(a1, b3), 0);
    }
}
