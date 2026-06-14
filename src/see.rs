//! Static exchange evaluation: swap-list algorithm with x-ray attack
//! recomputation. Used to prune losing captures in quiescence (and later for
//! ordering and pruning guards).

use crate::bitboard::{BitIter, bb};
use crate::eval::Score;
use crate::position::Position;
use crate::types::{Move, PieceType};

/// Piece values for exchanges; king huge so a king "recapture" into further
/// attackers resolves as losing.
const SEE_VALUE: [Score; 6] = [100, 320, 330, 500, 900, 20_000];

/// Exchange value of `mv` in centipawns from the mover's perspective.
/// Quiet moves return 0 minus whatever the opponent wins back on the
/// destination square.
pub fn see(pos: &Position, mv: Move) -> Score {
    let to = mv.to();
    let from = mv.from();

    let mut occ = pos.occupied();
    let mut stm = pos.stm;
    let mut gain = [0i32; 32];
    let mut d = 0usize;

    gain[0] = if mv.is_en_passant() {
        let cap_sq = if pos.stm == crate::types::Color::White { to - 8 } else { to + 8 };
        occ ^= bb(cap_sq);
        SEE_VALUE[PieceType::Pawn.idx()]
    } else {
        match pos.piece_on(to) {
            Some((_, pt)) => SEE_VALUE[pt.idx()],
            None => 0,
        }
    };

    // piece currently standing on the target square (the previous capturer);
    // for a promotion the piece that lands is the promoted type, not the pawn
    let mut on_square = if mv.is_promo() {
        mv.promo_piece()
    } else {
        pos.piece_on(from).expect("see: empty from").1
    };
    occ ^= bb(from);
    stm = stm.flip();

    loop {
        // a speculative gain entry only exists if stm actually has a
        // recapturer on the current occupancy (x-rays included via recompute)
        let attackers = pos.attackers_to(to, stm, occ) & occ;
        if attackers == 0 {
            break;
        }
        let mut found = None;
        for pt in PieceType::ALL {
            let set = attackers & pos.pieces(stm, pt);
            if set != 0 {
                found = Some((BitIter(set).next().unwrap(), pt));
                break;
            }
        }
        let (sq, pt) = found.unwrap();

        d += 1;
        gain[d] = SEE_VALUE[on_square.idx()] - gain[d - 1];
        on_square = pt;
        occ ^= bb(sq);
        stm = stm.flip();
        if d + 1 >= gain.len() {
            break;
        }
    }

    // negamax the swap list
    while d > 0 {
        gain[d - 1] = -std::cmp::max(-gain[d - 1], gain[d]);
        d -= 1;
    }
    gain[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::init_attack_tables;
    use crate::movegen::{MoveList, generate_moves};

    fn find(pos: &Position, s: &str) -> Move {
        let mut list = MoveList::new();
        generate_moves(pos, &mut list);
        list.iter().find(|m| m.to_string() == s).unwrap()
    }

    #[test]
    fn see_basics() {
        init_attack_tables();
        // RxR defended by pawn: rook takes rook (+500), pawn recaptures (-500) = 0...
        // 1k6/8/8/3p4/4r3/8/8/4R2K w: Rxe4, d5 pawn recaptures: +500 - 500 = 0
        let pos = Position::from_fen("1k6/8/8/3p4/4r3/8/8/4R2K w - - 0 1").unwrap();
        assert_eq!(see(&pos, find(&pos, "e1e4")), 0);

        // QxP defended by pawn: +100 - 900 = -800
        let pos = Position::from_fen("1k6/8/2p5/3p4/8/8/3Q4/3K4 w - - 0 1").unwrap();
        assert_eq!(see(&pos, find(&pos, "d2d5")), -800);

        // PxP undefended: +100
        let pos = Position::from_fen("1k6/8/8/3p4/4P3/8/8/4K3 w - - 0 1").unwrap();
        assert_eq!(see(&pos, find(&pos, "e4d5")), 100);

        // NxP, pawn recaptures, our bishop behind retakes pawn? simple chain:
        // n takes p (+100), p takes n (-320) -> -220
        let pos = Position::from_fen("1k6/8/2p5/3p4/8/4N3/8/4K3 w - - 0 1").unwrap();
        assert_eq!(see(&pos, find(&pos, "e3d5")), -220);
    }
}
