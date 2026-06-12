//! Core chess types: colors, pieces, squares, moves, castling rights.

use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline(always)]
    pub const fn flip(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }

    #[inline(always)]
    pub const fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
#[repr(u8)]
pub enum PieceType {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

impl PieceType {
    pub const ALL: [PieceType; 6] = [
        PieceType::Pawn,
        PieceType::Knight,
        PieceType::Bishop,
        PieceType::Rook,
        PieceType::Queen,
        PieceType::King,
    ];

    #[inline(always)]
    pub const fn idx(self) -> usize {
        self as usize
    }

    pub const fn from_idx(i: usize) -> PieceType {
        match i {
            0 => PieceType::Pawn,
            1 => PieceType::Knight,
            2 => PieceType::Bishop,
            3 => PieceType::Rook,
            4 => PieceType::Queen,
            _ => PieceType::King,
        }
    }

    pub fn to_char(self) -> char {
        b"pnbrqk"[self.idx()] as char
    }
}

/// Square index 0..64, a1 = 0, h1 = 7, a8 = 56, h8 = 63.
pub type Square = u8;

pub const fn square(file: u8, rank: u8) -> Square {
    rank * 8 + file
}

#[inline(always)]
pub const fn file_of(sq: Square) -> u8 {
    sq & 7
}

#[inline(always)]
pub const fn rank_of(sq: Square) -> u8 {
    sq >> 3
}

pub fn square_name(sq: Square) -> String {
    format!("{}{}", (b'a' + file_of(sq)) as char, (b'1' + rank_of(sq)) as char)
}

pub fn parse_square(s: &str) -> Option<Square> {
    let b = s.as_bytes();
    if b.len() != 2 || !(b'a'..=b'h').contains(&b[0]) || !(b'1'..=b'8').contains(&b[1]) {
        return None;
    }
    Some(square(b[0] - b'a', b[1] - b'1'))
}

/// Move flags (4 bits). Captures have bit 2 set, promotions bit 3.
pub mod flag {
    pub const QUIET: u16 = 0;
    pub const DOUBLE_PUSH: u16 = 1;
    pub const CASTLE_KING: u16 = 2;
    pub const CASTLE_QUEEN: u16 = 3;
    pub const CAPTURE: u16 = 4;
    pub const EN_PASSANT: u16 = 5;
    pub const PROMO_N: u16 = 8;
    pub const PROMO_B: u16 = 9;
    pub const PROMO_R: u16 = 10;
    pub const PROMO_Q: u16 = 11;
    pub const PROMO_CAP_N: u16 = 12;
    pub const PROMO_CAP_B: u16 = 13;
    pub const PROMO_CAP_R: u16 = 14;
    pub const PROMO_CAP_Q: u16 = 15;
}

/// 16-bit move: bits 0-5 from, 6-11 to, 12-15 flags.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Move(pub u16);

pub const MOVE_NONE: Move = Move(0);

impl Move {
    #[inline(always)]
    pub const fn new(from: Square, to: Square, flags: u16) -> Move {
        Move(from as u16 | (to as u16) << 6 | flags << 12)
    }

    #[inline(always)]
    pub const fn from(self) -> Square {
        (self.0 & 0x3f) as Square
    }

    #[inline(always)]
    pub const fn to(self) -> Square {
        (self.0 >> 6 & 0x3f) as Square
    }

    #[inline(always)]
    pub const fn flags(self) -> u16 {
        self.0 >> 12
    }

    #[inline(always)]
    pub const fn is_capture(self) -> bool {
        self.flags() & flag::CAPTURE != 0
    }

    #[inline(always)]
    pub const fn is_promo(self) -> bool {
        self.flags() & 8 != 0
    }

    #[inline(always)]
    pub const fn is_en_passant(self) -> bool {
        self.flags() == flag::EN_PASSANT
    }

    #[inline(always)]
    pub const fn is_castle(self) -> bool {
        self.flags() == flag::CASTLE_KING || self.flags() == flag::CASTLE_QUEEN
    }

    pub const fn promo_piece(self) -> PieceType {
        match self.flags() & 3 {
            0 => PieceType::Knight,
            1 => PieceType::Bishop,
            2 => PieceType::Rook,
            _ => PieceType::Queen,
        }
    }
}

impl fmt::Display for Move {
    /// UCI notation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == MOVE_NONE {
            return write!(f, "0000");
        }
        write!(f, "{}{}", square_name(self.from()), square_name(self.to()))?;
        if self.is_promo() {
            write!(f, "{}", self.promo_piece().to_char())?;
        }
        Ok(())
    }
}

/// Castling rights as 4 bits: 1 = white kingside, 2 = white queenside,
/// 4 = black kingside, 8 = black queenside.
pub mod castling {
    pub const WK: u8 = 1;
    pub const WQ: u8 = 2;
    pub const BK: u8 = 4;
    pub const BQ: u8 = 8;
}
