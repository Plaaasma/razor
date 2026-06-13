//! v0 evaluation: material + piece-square tables, white-relative internally,
//! returned from the side-to-move's perspective. Just enough to launch the
//! search ladder and datagen (brief §5); NNUE replaces this in Phase 3.

use crate::bitboard::BitIter;
use crate::position::Position;
use crate::types::{Color, PieceType};

pub type Score = i32;

pub const MATE: Score = 30_000;
pub const MATE_BOUND: Score = 29_000;
pub const DRAW: Score = 0;

use std::sync::atomic::{AtomicBool, Ordering};

/// Eval backend switch. NNUE by default; flip to false (UCI `UseNNUE=false`)
/// for the hand-crafted PSQT eval — used to SPRT NNUE vs PSQT from one binary.
pub static USE_NNUE: AtomicBool = AtomicBool::new(true);

/// Static eval from the side-to-move's perspective. Dispatches to NNUE or PSQT.
#[inline(always)]
pub fn evaluate(pos: &Position) -> Score {
    if USE_NNUE.load(Ordering::Relaxed) {
        crate::nnue::evaluate(pos)
    } else {
        evaluate_psqt(pos)
    }
}

pub const PIECE_VALUE: [Score; 6] = [100, 320, 330, 500, 900, 0];

// PSQTs, white perspective, a1 = index 0. Hand-rolled values reflecting
// standard principles: center control, development, king shelter, 7th-rank
// rooks, advanced pawns.
#[rustfmt::skip]
const PSQT: [[Score; 64]; 6] = [
    // pawn
    [
          0,   0,   0,   0,   0,   0,   0,   0,
          2,   4,   0, -12, -12,   6,   6,   2,
          2,  -2,  -4,   2,   2,  -8,  -2,   2,
          0,   2,   6,  16,  16,   4,   0,  -2,
          6,   8,  12,  20,  20,  10,   4,   2,
         16,  20,  24,  28,  28,  22,  16,  12,
         40,  44,  44,  44,  44,  40,  36,  36,
          0,   0,   0,   0,   0,   0,   0,   0,
    ],
    // knight
    [
        -40, -24, -16, -12, -12, -16, -24, -40,
        -24, -12,  -2,   4,   4,  -2, -12, -24,
        -14,   2,  10,  14,  14,  10,   2, -14,
        -10,   6,  16,  22,  22,  16,   6, -10,
         -8,   8,  18,  24,  24,  18,   8,  -8,
        -12,   4,  14,  18,  18,  14,   4, -12,
        -20,  -8,   2,   6,   6,   2,  -8, -20,
        -36, -20, -12,  -8,  -8, -12, -20, -36,
    ],
    // bishop
    [
        -16,  -8, -10,  -6,  -6, -10,  -8, -16,
         -4,   8,   4,   2,   2,   4,   8,  -4,
         -2,   6,   8,   6,   6,   8,   6,  -2,
          0,   4,   8,  12,  12,   8,   4,   0,
          0,   4,   8,  12,  12,   8,   4,   0,
         -2,   4,   8,   8,   8,   8,   4,  -2,
         -6,   2,   2,   2,   2,   2,   2,  -6,
        -14,  -8,  -8,  -6,  -6,  -8,  -8, -14,
    ],
    // rook
    [
         -4,  -2,   2,   6,   6,   2,  -2,  -4,
         -6,  -4,   0,   4,   4,   0,  -4,  -6,
         -6,  -2,   0,   4,   4,   0,  -2,  -6,
         -4,   0,   2,   4,   4,   2,   0,  -4,
          0,   2,   4,   6,   6,   4,   2,   0,
          4,   8,  10,  10,  10,  10,   8,   4,
         14,  16,  18,  20,  20,  18,  16,  14,
          8,  10,  10,  12,  12,  10,  10,   8,
    ],
    // queen
    [
        -12,  -8,  -6,   2,   2,  -6,  -8, -12,
         -6,   0,   4,   4,   4,   4,   0,  -6,
         -4,   2,   6,   6,   6,   6,   2,  -4,
         -2,   4,   6,   8,   8,   6,   4,  -2,
         -2,   4,   6,   8,   8,   6,   4,  -2,
         -4,   2,   6,   6,   6,   6,   2,  -4,
         -6,   0,   4,   4,   4,   4,   0,  -6,
        -12,  -8,  -4,   0,   0,  -4,  -8, -12,
    ],
    // king (middlegame-ish: shelter on back rank wings)
    [
         16,  24,   8, -12, -12,   2,  24,  18,
          8,   8,  -8, -20, -20,  -8,  10,  10,
        -12, -16, -24, -32, -32, -24, -16, -12,
        -24, -28, -36, -44, -44, -36, -28, -24,
        -32, -36, -44, -52, -52, -44, -36, -32,
        -36, -40, -48, -56, -56, -48, -40, -36,
        -40, -44, -52, -60, -60, -52, -44, -40,
        -44, -48, -56, -64, -64, -56, -48, -44,
    ],
];

/// Hand-crafted material + PSQT eval, side-to-move relative.
pub fn evaluate_psqt(pos: &Position) -> Score {
    let mut score = 0;
    for pt in PieceType::ALL {
        for sq in BitIter(pos.pieces(Color::White, pt)) {
            score += PIECE_VALUE[pt.idx()] + PSQT[pt.idx()][sq as usize];
        }
        for sq in BitIter(pos.pieces(Color::Black, pt)) {
            // mirror rank for black
            score -= PIECE_VALUE[pt.idx()] + PSQT[pt.idx()][(sq ^ 56) as usize];
        }
    }
    if pos.stm == Color::White { score } else { -score }
}
