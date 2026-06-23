//! Fully legal move generation using check masks and pin restriction.
//! No pseudo-legal filtering: every generated move is legal by construction
//! (en passant uses an explicit occupancy attack test, the one genuinely
//! tricky case).

use crate::bitboard::*;
use crate::position::{NO_SQUARE, Position, castle_index};
use crate::types::*;

pub const MAX_MOVES: usize = 256;

pub struct MoveList {
    pub moves: [Move; MAX_MOVES],
    pub len: usize,
}

impl MoveList {
    pub fn new() -> MoveList {
        MoveList { moves: [MOVE_NONE; MAX_MOVES], len: 0 }
    }

    #[inline(always)]
    pub fn push(&mut self, mv: Move) {
        debug_assert!(self.len < MAX_MOVES);
        self.moves[self.len] = mv;
        self.len += 1;
    }

    pub fn iter(&self) -> impl Iterator<Item = Move> + '_ {
        self.moves[..self.len].iter().copied()
    }
}

/// All legal moves.
pub fn generate_moves(pos: &Position, list: &mut MoveList) {
    gen_impl::<false>(pos, list)
}

/// Captures, en passant, and promotions only (quiescence search).
pub fn generate_captures(pos: &Position, list: &mut MoveList) {
    gen_impl::<true>(pos, list)
}

fn gen_impl<const CAPS_ONLY: bool>(pos: &Position, list: &mut MoveList) {
    let us = pos.stm;
    let them = us.flip();
    let ksq = pos.king_sq(us);
    let occ = pos.occupied();
    let ours = pos.color_bb[us.idx()];
    let theirs = pos.color_bb[them.idx()];

    let checkers = pos.attackers_to(ksq, them, occ);
    let pinned = pos.pinned();
    // in captures-only mode, restrict every piece's targets to enemy pieces
    let cap_targets = if CAPS_ONLY { theirs } else { !0u64 };

    // --- king moves (always considered; exclude squares attacked with the
    // king removed from occupancy so sliders see through it) ---
    let occ_no_king = occ ^ bb(ksq);
    for to in BitIter(KING_ATTACKS[ksq as usize] & !ours & cap_targets) {
        if pos.attackers_to(to, them, occ_no_king) == 0 {
            let f = if theirs & bb(to) != 0 { flag::CAPTURE } else { flag::QUIET };
            list.push(Move::new(ksq, to, f));
        }
    }

    if checkers.count_ones() >= 2 {
        return; // double check: only king moves
    }

    let check_mask = if checkers != 0 {
        between(ksq, lsb(checkers)) | checkers
    } else {
        !0u64
    };

    // --- knights (a pinned knight can never move) ---
    for from in BitIter(pos.pieces(us, PieceType::Knight) & !pinned) {
        for to in BitIter(KNIGHT_ATTACKS[from as usize] & !ours & check_mask & cap_targets) {
            let f = if theirs & bb(to) != 0 { flag::CAPTURE } else { flag::QUIET };
            list.push(Move::new(from, to, f));
        }
    }

    // --- sliders ---
    for (pt, attack_fn) in [
        (PieceType::Bishop, bishop_attacks as fn(Square, Bitboard) -> Bitboard),
        (PieceType::Rook, rook_attacks),
        (PieceType::Queen, queen_attacks),
    ] {
        for from in BitIter(pos.pieces(us, pt)) {
            let mut targets = attack_fn(from, occ) & !ours & check_mask & cap_targets;
            if pinned & bb(from) != 0 {
                targets &= line_through(ksq, from);
            }
            for to in BitIter(targets) {
                let f = if theirs & bb(to) != 0 { flag::CAPTURE } else { flag::QUIET };
                list.push(Move::new(from, to, f));
            }
        }
    }

    // --- pawns ---
    let pawns = pos.pieces(us, PieceType::Pawn);
    let (push_dir, start_rank, promo_rank): (i8, Bitboard, Bitboard) = match us {
        Color::White => (8, RANK_2, RANK_8),
        Color::Black => (-8, RANK_7, RANK_1),
    };

    for from in BitIter(pawns) {
        let pin_line = if pinned & bb(from) != 0 { line_through(ksq, from) } else { !0u64 };

        // pushes (captures-only mode keeps just the promotion pushes)
        let one = (from as i8 + push_dir) as Square;
        if occ & bb(one) == 0 {
            if bb(one) & check_mask & pin_line != 0 {
                if bb(one) & promo_rank != 0 {
                    for pf in [flag::PROMO_Q, flag::PROMO_N, flag::PROMO_R, flag::PROMO_B] {
                        list.push(Move::new(from, one, pf));
                    }
                } else if !CAPS_ONLY {
                    list.push(Move::new(from, one, flag::QUIET));
                }
            }
            if !CAPS_ONLY && bb(from) & start_rank != 0 {
                let two = (from as i8 + 2 * push_dir) as Square;
                if occ & bb(two) == 0 && bb(two) & check_mask & pin_line != 0 {
                    list.push(Move::new(from, two, flag::DOUBLE_PUSH));
                }
            }
        }

        // captures
        for to in BitIter(pawn_attacks(us, from) & theirs & check_mask & pin_line) {
            if bb(to) & promo_rank != 0 {
                for pf in [flag::PROMO_CAP_Q, flag::PROMO_CAP_N, flag::PROMO_CAP_R, flag::PROMO_CAP_B] {
                    list.push(Move::new(from, to, pf));
                }
            } else {
                list.push(Move::new(from, to, flag::CAPTURE));
            }
        }
    }

    // --- en passant: verify by direct attack test on resulting occupancy
    // (covers the horizontal double-removal pin and check evasion) ---
    if pos.ep != NO_SQUARE {
        let cap_sq = (pos.ep as i8 - push_dir) as Square;
        for from in BitIter(pawn_attacks(them, pos.ep) & pawns) {
            let new_occ = occ ^ bb(from) ^ bb(cap_sq) | bb(pos.ep);
            let queens = pos.pieces(them, PieceType::Queen);
            let slider_check = (rook_attacks(ksq, new_occ)
                & (self_or(pos.pieces(them, PieceType::Rook), queens)))
                | (bishop_attacks(ksq, new_occ)
                    & (self_or(pos.pieces(them, PieceType::Bishop), queens)));
            // non-slider checks can't be created by ep beyond what check_mask
            // covers, but the captured pawn might have been the checker:
            let still_checked = if checkers != 0 && checkers != bb(cap_sq) {
                // some other piece is checking; ep can only help by blocking
                bb(pos.ep) & check_mask == 0
            } else {
                false
            };
            if slider_check == 0 && !still_checked {
                list.push(Move::new(from, pos.ep, flag::EN_PASSANT));
            }
        }
    }

    // --- castling (Chess960-general: king and rooks may start on any file) ---
    if !CAPS_ONLY && checkers == 0 {
        let ksq = pos.king_sq(us);
        let (kbit, qbit, kto, kr_to, qto, qr_to) = match us {
            Color::White => (castling::WK, castling::WQ, 6u8, 5u8, 2u8, 3u8),
            Color::Black => (castling::BK, castling::BQ, 62, 61, 58, 59),
        };
        if pos.castling & kbit != 0 {
            gen_castle(pos, list, them, occ, ksq,
                pos.castle_rook[castle_index(us, true)], kto, kr_to, flag::CASTLE_KING);
        }
        if pos.castling & qbit != 0 {
            gen_castle(pos, list, them, occ, ksq,
                pos.castle_rook[castle_index(us, false)], qto, qr_to, flag::CASTLE_QUEEN);
        }
    }
}

/// Inclusive bitboard of squares between two squares on the same rank.
#[inline]
fn span(a: u8, b: u8) -> Bitboard {
    let (lo, hi) = (a.min(b), a.max(b));
    let mut m = 0u64;
    let mut s = lo;
    while s <= hi {
        m |= 1u64 << s;
        s += 1;
    }
    m
}

/// Emit a Chess960-general castling move if legal: every square the king and rook
/// traverse must be empty (except where the two pieces currently sit), and no
/// square the king passes through (origin..=target inclusive) may be attacked —
/// with both movers lifted from the occupancy, so a rook vacating its square
/// cannot conceal a discovered check on the king. Legal by construction.
#[inline]
fn gen_castle(
    pos: &Position,
    list: &mut MoveList,
    them: Color,
    occ: Bitboard,
    ksq: u8,
    rsq: u8,
    kto: u8,
    rto: u8,
    flags: u16,
) {
    let movers = bb(ksq) | bb(rsq);
    let king_path = span(ksq, kto);
    let rook_path = span(rsq, rto);
    if (king_path | rook_path) & occ & !movers != 0 {
        return;
    }
    let occ_ns = occ & !movers;
    if BitIter(king_path).all(|sq| pos.attackers_to(sq, them, occ_ns) == 0) {
        list.push(Move::new(ksq, kto, flags));
    }
}

#[inline(always)]
fn self_or(a: Bitboard, b: Bitboard) -> Bitboard {
    a | b
}
