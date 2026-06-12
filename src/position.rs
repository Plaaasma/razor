//! Position representation: bitboards per piece type + per color, mailbox for
//! piece lookup, copy-make. Zobrist updated incrementally and verified against
//! a from-scratch recomputation in debug builds.

use crate::bitboard::*;
use crate::types::*;
use crate::zobrist;

#[derive(Copy, Clone)]
pub struct Position {
    pub piece_bb: [Bitboard; 6],
    pub color_bb: [Bitboard; 2],
    /// 12 = empty, else color * 6 + piece_type
    pub mailbox: [u8; 64],
    pub stm: Color,
    pub castling: u8,
    /// En passant target square, or 64 if none.
    pub ep: Square,
    pub halfmove: u8,
    pub fullmove: u16,
    pub key: u64,
}

pub const NO_SQUARE: Square = 64;
const EMPTY: u8 = 12;

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

impl Position {
    pub fn empty() -> Position {
        Position {
            piece_bb: [0; 6],
            color_bb: [0; 2],
            mailbox: [EMPTY; 64],
            stm: Color::White,
            castling: 0,
            ep: NO_SQUARE,
            halfmove: 0,
            fullmove: 1,
            key: 0,
        }
    }

    pub fn startpos() -> Position {
        Position::from_fen(START_FEN).unwrap()
    }

    #[inline(always)]
    pub fn occupied(&self) -> Bitboard {
        self.color_bb[0] | self.color_bb[1]
    }

    #[inline(always)]
    pub fn pieces(&self, c: Color, pt: PieceType) -> Bitboard {
        self.piece_bb[pt.idx()] & self.color_bb[c.idx()]
    }

    #[inline(always)]
    pub fn king_sq(&self, c: Color) -> Square {
        lsb(self.pieces(c, PieceType::King))
    }

    #[inline(always)]
    pub fn piece_on(&self, sq: Square) -> Option<(Color, PieceType)> {
        let v = self.mailbox[sq as usize];
        if v == EMPTY {
            None
        } else {
            let c = if v < 6 { Color::White } else { Color::Black };
            Some((c, PieceType::from_idx((v % 6) as usize)))
        }
    }

    #[inline(always)]
    fn put(&mut self, c: Color, pt: PieceType, sq: Square) {
        self.piece_bb[pt.idx()] |= bb(sq);
        self.color_bb[c.idx()] |= bb(sq);
        self.mailbox[sq as usize] = c.idx() as u8 * 6 + pt.idx() as u8;
        self.key ^= zobrist::piece_key(c, pt, sq);
    }

    #[inline(always)]
    fn remove(&mut self, c: Color, pt: PieceType, sq: Square) {
        self.piece_bb[pt.idx()] &= !bb(sq);
        self.color_bb[c.idx()] &= !bb(sq);
        self.mailbox[sq as usize] = EMPTY;
        self.key ^= zobrist::piece_key(c, pt, sq);
    }

    /// All pieces of color `c` attacking square `sq`, given occupancy `occ`.
    pub fn attackers_to(&self, sq: Square, c: Color, occ: Bitboard) -> Bitboard {
        let queens = self.pieces(c, PieceType::Queen);
        (pawn_attacks(c.flip(), sq) & self.pieces(c, PieceType::Pawn))
            | (KNIGHT_ATTACKS[sq as usize] & self.pieces(c, PieceType::Knight))
            | (KING_ATTACKS[sq as usize] & self.pieces(c, PieceType::King))
            | (bishop_attacks(sq, occ) & (self.pieces(c, PieceType::Bishop) | queens))
            | (rook_attacks(sq, occ) & (self.pieces(c, PieceType::Rook) | queens))
    }

    #[inline(always)]
    pub fn in_check(&self) -> bool {
        self.attackers_to(self.king_sq(self.stm), self.stm.flip(), self.occupied()) != 0
    }

    /// Pieces of side-to-move pinned to their own king.
    pub fn pinned(&self) -> Bitboard {
        let us = self.stm;
        let them = us.flip();
        let ksq = self.king_sq(us);
        let occ = self.occupied();
        let mut pinned = 0u64;

        let queens = self.pieces(them, PieceType::Queen);
        let snipers = (rook_attacks(ksq, 0) & (self.pieces(them, PieceType::Rook) | queens))
            | (bishop_attacks(ksq, 0) & (self.pieces(them, PieceType::Bishop) | queens));

        for sniper in BitIter(snipers) {
            let blockers = between(ksq, sniper) & occ;
            if blockers.count_ones() == 1 && blockers & self.color_bb[us.idx()] != 0 {
                pinned |= blockers;
            }
        }
        pinned
    }

    // -----------------------------------------------------------------------
    // FEN
    // -----------------------------------------------------------------------

    pub fn from_fen(fen: &str) -> Result<Position, String> {
        let mut pos = Position::empty();
        let parts: Vec<&str> = fen.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(format!("bad FEN: {fen}"));
        }

        let mut rank = 7i8;
        let mut file = 0i8;
        for ch in parts[0].chars() {
            match ch {
                '/' => {
                    rank -= 1;
                    file = 0;
                }
                '1'..='8' => file += ch as i8 - '0' as i8,
                _ => {
                    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                        return Err(format!("bad FEN board: {fen}"));
                    }
                    let c = if ch.is_ascii_uppercase() { Color::White } else { Color::Black };
                    let pt = match ch.to_ascii_lowercase() {
                        'p' => PieceType::Pawn,
                        'n' => PieceType::Knight,
                        'b' => PieceType::Bishop,
                        'r' => PieceType::Rook,
                        'q' => PieceType::Queen,
                        'k' => PieceType::King,
                        _ => return Err(format!("bad FEN piece '{ch}'")),
                    };
                    pos.put(c, pt, square(file as u8, rank as u8));
                    file += 1;
                }
            }
        }

        pos.stm = match parts[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(format!("bad FEN stm: {fen}")),
        };

        for ch in parts[2].chars() {
            match ch {
                'K' => pos.castling |= castling::WK,
                'Q' => pos.castling |= castling::WQ,
                'k' => pos.castling |= castling::BK,
                'q' => pos.castling |= castling::BQ,
                '-' => {}
                _ => return Err(format!("bad FEN castling: {fen}")),
            }
        }

        pos.ep = if parts[3] == "-" {
            NO_SQUARE
        } else {
            parse_square(parts[3]).ok_or_else(|| format!("bad FEN ep: {fen}"))?
        };

        pos.halfmove = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
        pos.fullmove = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(1);

        if pos.stm == Color::Black {
            pos.key ^= zobrist::KEYS.side_to_move;
        }
        pos.key ^= zobrist::KEYS.castling[pos.castling as usize];
        if pos.ep != NO_SQUARE {
            pos.key ^= zobrist::KEYS.ep_file[file_of(pos.ep) as usize];
        }

        // basic legality: both kings present, side not to move not in check
        // (otherwise movegen can "capture" a king and corrupt the search)
        if pos.pieces(Color::White, PieceType::King).count_ones() != 1
            || pos.pieces(Color::Black, PieceType::King).count_ones() != 1
        {
            return Err(format!("illegal FEN (king count): {fen}"));
        }
        let prev = pos.stm.flip();
        if pos.attackers_to(pos.king_sq(prev), pos.stm, pos.occupied()) != 0 {
            return Err(format!("illegal FEN (side not to move is in check): {fen}"));
        }

        Ok(pos)
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for rank in (0..8).rev() {
            let mut empties = 0;
            for file in 0..8 {
                match self.piece_on(square(file, rank)) {
                    None => empties += 1,
                    Some((c, pt)) => {
                        if empties > 0 {
                            s.push((b'0' + empties) as char);
                            empties = 0;
                        }
                        let ch = pt.to_char();
                        s.push(if c == Color::White { ch.to_ascii_uppercase() } else { ch });
                    }
                }
            }
            if empties > 0 {
                s.push((b'0' + empties) as char);
            }
            if rank > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.stm == Color::White { 'w' } else { 'b' });
        s.push(' ');
        if self.castling == 0 {
            s.push('-');
        } else {
            if self.castling & castling::WK != 0 { s.push('K'); }
            if self.castling & castling::WQ != 0 { s.push('Q'); }
            if self.castling & castling::BK != 0 { s.push('k'); }
            if self.castling & castling::BQ != 0 { s.push('q'); }
        }
        s.push(' ');
        if self.ep == NO_SQUARE {
            s.push('-');
        } else {
            s.push_str(&square_name(self.ep));
        }
        s.push_str(&format!(" {} {}", self.halfmove, self.fullmove));
        s
    }

    // -----------------------------------------------------------------------
    // Make move (copy-make)
    // -----------------------------------------------------------------------

    /// Apply a legal move, returning the resulting position.
    pub fn make(&self, mv: Move) -> Position {
        let mut pos = *self;
        let us = pos.stm;
        let them = us.flip();
        let from = mv.from();
        let to = mv.to();
        let (_, pt) = pos.piece_on(from).expect("make: no piece on from-square");

        // clear old ep key; new one set below if this is a double push
        if pos.ep != NO_SQUARE {
            pos.key ^= zobrist::KEYS.ep_file[file_of(pos.ep) as usize];
            pos.ep = NO_SQUARE;
        }

        pos.halfmove += 1;
        if pt == PieceType::Pawn {
            pos.halfmove = 0;
        }

        match mv.flags() {
            flag::QUIET => {
                pos.remove(us, pt, from);
                pos.put(us, pt, to);
            }
            flag::DOUBLE_PUSH => {
                pos.remove(us, pt, from);
                pos.put(us, pt, to);
                let ep_sq = if us == Color::White { to - 8 } else { to + 8 };
                // only set ep if an enemy pawn can actually capture — keeps
                // zobrist keys canonical for repetition detection
                if pawn_attacks(us, ep_sq) & pos.pieces(them, PieceType::Pawn) != 0 {
                    pos.ep = ep_sq;
                    pos.key ^= zobrist::KEYS.ep_file[file_of(ep_sq) as usize];
                }
            }
            flag::CASTLE_KING | flag::CASTLE_QUEEN => {
                let (rook_from, rook_to) = match (us, mv.flags()) {
                    (Color::White, flag::CASTLE_KING) => (7u8, 5u8),
                    (Color::White, _) => (0, 3),
                    (Color::Black, flag::CASTLE_KING) => (63, 61),
                    (Color::Black, _) => (56, 59),
                };
                pos.remove(us, PieceType::King, from);
                pos.put(us, PieceType::King, to);
                pos.remove(us, PieceType::Rook, rook_from);
                pos.put(us, PieceType::Rook, rook_to);
            }
            flag::EN_PASSANT => {
                let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
                pos.remove(them, PieceType::Pawn, cap_sq);
                pos.remove(us, PieceType::Pawn, from);
                pos.put(us, PieceType::Pawn, to);
            }
            flag::CAPTURE => {
                let (_, cap_pt) = pos.piece_on(to).expect("capture: empty to-square");
                pos.remove(them, cap_pt, to);
                pos.remove(us, pt, from);
                pos.put(us, pt, to);
                pos.halfmove = 0;
            }
            f if f & 8 != 0 => {
                // promotions, with or without capture
                if mv.is_capture() {
                    let (_, cap_pt) = pos.piece_on(to).expect("promo capture: empty to-square");
                    pos.remove(them, cap_pt, to);
                }
                pos.remove(us, PieceType::Pawn, from);
                pos.put(us, mv.promo_piece(), to);
            }
            _ => unreachable!(),
        }

        // castling rights: clear when king or rook moves or rook is captured
        let old_castling = pos.castling;
        const RIGHTS_MASK: [u8; 64] = {
            let mut m = [0xffu8; 64];
            m[4] = !(castling::WK | castling::WQ);
            m[0] = !castling::WQ;
            m[7] = !castling::WK;
            m[60] = !(castling::BK | castling::BQ);
            m[56] = !castling::BQ;
            m[63] = !castling::BK;
            m
        };
        pos.castling &= RIGHTS_MASK[from as usize] & RIGHTS_MASK[to as usize];
        if pos.castling != old_castling {
            pos.key ^= zobrist::KEYS.castling[old_castling as usize];
            pos.key ^= zobrist::KEYS.castling[pos.castling as usize];
        }

        pos.stm = them;
        pos.key ^= zobrist::KEYS.side_to_move;
        if us == Color::Black {
            pos.fullmove += 1;
        }

        debug_assert_eq!(pos.key, pos.recompute_key(), "incremental zobrist mismatch");
        pos
    }

    /// From-scratch key, for debug verification.
    pub fn recompute_key(&self) -> u64 {
        let mut key = 0u64;
        for sq in BitIter(self.occupied()) {
            let (c, pt) = self.piece_on(sq).unwrap();
            key ^= zobrist::piece_key(c, pt, sq);
        }
        if self.stm == Color::Black {
            key ^= zobrist::KEYS.side_to_move;
        }
        key ^= zobrist::KEYS.castling[self.castling as usize];
        if self.ep != NO_SQUARE {
            key ^= zobrist::KEYS.ep_file[file_of(self.ep) as usize];
        }
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::init_attack_tables;

    #[test]
    fn fen_roundtrip() {
        init_attack_tables();
        for fen in [
            START_FEN,
            "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
            "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
            "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        ] {
            assert_eq!(Position::from_fen(fen).unwrap().to_fen(), fen);
        }
    }
}
