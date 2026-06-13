//! NNUE evaluation. Perspective network `(768 -> 512)x2 -> 1`, SCReLU, int16
//! quantized, trained with bullet (see `training/razor_net.rs`).
//!
//! Two accumulators are maintained, one per board perspective (not per side to
//! move): `w` indexes pieces as if White is to move, `b` as if Black is. They
//! are updated incrementally on each move (add/remove the moved/captured piece
//! features) and never need a full refresh during search. At eval time the
//! side-to-move picks which accumulator is "us".
//!
//! White-perspective feature for piece (col, pt, sq):
//!   (col==White ? 0 : 384) + 64*pt + sq
//! Black-perspective feature:
//!   (col==Black ? 0 : 384) + 64*pt + (sq ^ 56)
//! (verified identical to bullet's Chess768 for both stm colors.)

use crate::bitboard::BitIter;
use crate::eval::Score;
use crate::position::Position;
use crate::types::{Color, PieceType, Square};

const HIDDEN: usize = 512;
const QA: i32 = 255;
const QB: i32 = 64;
const SCALE: i32 = 400;

const NET_BYTES: &[u8] = include_bytes!("../nets/razor1.nnue");

pub struct Network {
    feature_weights: Box<[[i16; HIDDEN]; 768]>,
    feature_bias: [i16; HIDDEN],
    output_weights: [i16; 2 * HIDDEN],
    output_bias: i16,
}

fn rd_i16(bytes: &[u8], at: &mut usize) -> i16 {
    let v = i16::from_le_bytes([bytes[*at], bytes[*at + 1]]);
    *at += 2;
    v
}

fn load() -> Network {
    let mut at = 0usize;
    let mut feature_weights = Box::new([[0i16; HIDDEN]; 768]);
    for col in feature_weights.iter_mut() {
        for w in col.iter_mut() {
            *w = rd_i16(NET_BYTES, &mut at);
        }
    }
    let mut feature_bias = [0i16; HIDDEN];
    for b in feature_bias.iter_mut() {
        *b = rd_i16(NET_BYTES, &mut at);
    }
    let mut output_weights = [0i16; 2 * HIDDEN];
    for w in output_weights.iter_mut() {
        *w = rd_i16(NET_BYTES, &mut at);
    }
    let output_bias = rd_i16(NET_BYTES, &mut at);
    Network { feature_weights, feature_bias, output_weights, output_bias }
}

use std::sync::OnceLock;
static NET: OnceLock<Network> = OnceLock::new();

pub fn net() -> &'static Network {
    NET.get_or_init(load)
}

#[inline(always)]
fn wp_feat(col: Color, pt: usize, sq: Square) -> usize {
    (if col == Color::White { 0 } else { 384 }) + 64 * pt + sq as usize
}

#[inline(always)]
fn bp_feat(col: Color, pt: usize, sq: Square) -> usize {
    (if col == Color::Black { 0 } else { 384 }) + 64 * pt + (sq ^ 56) as usize
}

#[derive(Clone)]
pub struct Accumulator {
    pub w: [i16; HIDDEN],
    pub b: [i16; HIDDEN],
}

impl Accumulator {
    /// All-zero accumulator, for pre-sizing the search stack.
    pub fn zeroed() -> Accumulator {
        Accumulator { w: [0; HIDDEN], b: [0; HIDDEN] }
    }

    /// Full rebuild from a position.
    pub fn refresh(pos: &Position) -> Accumulator {
        let net = net();
        let mut acc = Accumulator { w: net.feature_bias, b: net.feature_bias };
        for pt in PieceType::ALL {
            for col in [Color::White, Color::Black] {
                for sq in BitIter(pos.pieces(col, pt)) {
                    acc.add(col, pt.idx(), sq);
                }
            }
        }
        acc
    }

    #[inline(always)]
    fn add(&mut self, col: Color, pt: usize, sq: Square) {
        let net = net();
        let wf = &net.feature_weights[wp_feat(col, pt, sq)];
        let bf = &net.feature_weights[bp_feat(col, pt, sq)];
        for i in 0..HIDDEN {
            self.w[i] += wf[i];
            self.b[i] += bf[i];
        }
    }

    #[inline(always)]
    fn remove(&mut self, col: Color, pt: usize, sq: Square) {
        let net = net();
        let wf = &net.feature_weights[wp_feat(col, pt, sq)];
        let bf = &net.feature_weights[bp_feat(col, pt, sq)];
        for i in 0..HIDDEN {
            self.w[i] -= wf[i];
            self.b[i] -= bf[i];
        }
    }

    /// Evaluate from the side-to-move's perspective.
    pub fn eval(&self, stm: Color) -> Score {
        let net = net();
        let (us, them) = if stm == Color::White { (&self.w, &self.b) } else { (&self.b, &self.w) };
        let mut output = 0i32;
        for i in 0..HIDDEN {
            output += screlu(us[i]) * net.output_weights[i] as i32;
            output += screlu(them[i]) * net.output_weights[HIDDEN + i] as i32;
        }
        output /= QA;
        output += net.output_bias as i32;
        output *= SCALE;
        output /= QA * QB;
        output
    }
}

#[inline(always)]
fn screlu(x: i16) -> i32 {
    let y = (x as i32).clamp(0, QA);
    y * y
}

/// Produce the child accumulator after playing `mv` in `pos` (the position
/// BEFORE the move). Mirrors `Position::make`'s piece manipulation exactly.
pub fn apply(parent: &Accumulator, pos: &Position, mv: crate::types::Move) -> Accumulator {
    use crate::types::flag;
    let mut acc = parent.clone();
    let us = pos.stm;
    let them = us.flip();
    let from = mv.from();
    let to = mv.to();
    let (_, pt) = pos.piece_on(from).expect("nnue apply: empty from");
    let pti = pt.idx();

    match mv.flags() {
        flag::QUIET | flag::DOUBLE_PUSH => {
            acc.remove(us, pti, from);
            acc.add(us, pti, to);
        }
        flag::CASTLE_KING | flag::CASTLE_QUEEN => {
            let (rook_from, rook_to) = match (us, mv.flags()) {
                (Color::White, flag::CASTLE_KING) => (7u8, 5u8),
                (Color::White, _) => (0, 3),
                (Color::Black, flag::CASTLE_KING) => (63, 61),
                (Color::Black, _) => (56, 59),
            };
            acc.remove(us, PieceType::King.idx(), from);
            acc.add(us, PieceType::King.idx(), to);
            acc.remove(us, PieceType::Rook.idx(), rook_from);
            acc.add(us, PieceType::Rook.idx(), rook_to);
        }
        flag::EN_PASSANT => {
            let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
            acc.remove(them, PieceType::Pawn.idx(), cap_sq);
            acc.remove(us, PieceType::Pawn.idx(), from);
            acc.add(us, PieceType::Pawn.idx(), to);
        }
        flag::CAPTURE => {
            let (_, cap_pt) = pos.piece_on(to).expect("nnue apply: empty capture");
            acc.remove(them, cap_pt.idx(), to);
            acc.remove(us, pti, from);
            acc.add(us, pti, to);
        }
        f if f & 8 != 0 => {
            if mv.is_capture() {
                let (_, cap_pt) = pos.piece_on(to).expect("nnue apply: empty promo-cap");
                acc.remove(them, cap_pt.idx(), to);
            }
            acc.remove(us, PieceType::Pawn.idx(), from);
            acc.add(us, mv.promo_piece().idx(), to);
        }
        _ => unreachable!(),
    }
    acc
}

/// From-scratch eval (uci `eval` command, datagen). Search uses the
/// incremental accumulator stack instead.
pub fn evaluate(pos: &Position) -> Score {
    Accumulator::refresh(pos).eval(pos.stm)
}
