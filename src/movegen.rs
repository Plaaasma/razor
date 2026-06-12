//! Fully legal move generation using check masks and pin restriction.
//! No pseudo-legal filtering: every generated move is legal by construction
//! (en passant uses an explicit occupancy attack test, the one genuinely
//! tricky case).

use crate::bitboard::*;
use crate::position::{NO_SQUARE, Position};
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

pub fn generate_moves(pos: &Position, list: &mut MoveList) {
    let us = pos.stm;
    let them = us.flip();
    let ksq = pos.king_sq(us);
    let occ = pos.occupied();
    let ours = pos.color_bb[us.idx()];
    let theirs = pos.color_bb[them.idx()];

    let checkers = pos.attackers_to(ksq, them, occ);
    let pinned = pos.pinned();

    // --- king moves (always considered; exclude squares attacked with the
    // king removed from occupancy so sliders see through it) ---
    let occ_no_king = occ ^ bb(ksq);
    for to in BitIter(KING_ATTACKS[ksq as usize] & !ours) {
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
        for to in BitIter(KNIGHT_ATTACKS[from as usize] & !ours & check_mask) {
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
            let mut targets = attack_fn(from, occ) & !ours & check_mask;
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

        // pushes
        let one = (from as i8 + push_dir) as Square;
        if occ & bb(one) == 0 {
            if bb(one) & check_mask & pin_line != 0 {
                if bb(one) & promo_rank != 0 {
                    for pf in [flag::PROMO_Q, flag::PROMO_N, flag::PROMO_R, flag::PROMO_B] {
                        list.push(Move::new(from, one, pf));
                    }
                } else {
                    list.push(Move::new(from, one, flag::QUIET));
                }
            }
            if bb(from) & start_rank != 0 {
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

    // --- castling ---
    if checkers == 0 {
        let (k_right, q_right, k_path, q_path_empty, q_path_safe, kfrom, kto_k, kto_q) = match us {
            Color::White => (
                castling::WK,
                castling::WQ,
                bb(5) | bb(6),          // f1, g1 empty + safe
                bb(1) | bb(2) | bb(3),  // b1, c1, d1 empty
                bb(2) | bb(3),          // c1, d1 safe
                4u8, 6u8, 2u8,
            ),
            Color::Black => (
                castling::BK,
                castling::BQ,
                bb(61) | bb(62),
                bb(57) | bb(58) | bb(59),
                bb(58) | bb(59),
                60, 62, 58,
            ),
        };
        if pos.castling & k_right != 0
            && occ & k_path == 0
            && BitIter(k_path).all(|sq| pos.attackers_to(sq, them, occ) == 0)
        {
            list.push(Move::new(kfrom, kto_k, flag::CASTLE_KING));
        }
        if pos.castling & q_right != 0
            && occ & q_path_empty == 0
            && BitIter(q_path_safe).all(|sq| pos.attackers_to(sq, them, occ) == 0)
        {
            list.push(Move::new(kfrom, kto_q, flag::CASTLE_QUEEN));
        }
    }
}

#[inline(always)]
fn self_or(a: Bitboard, b: Bitboard) -> Bitboard {
    a | b
}
