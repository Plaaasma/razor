//! NNUE evaluation. Perspective network `(768 -> 512)x2 -> 1`, SCReLU, int16
//! quantized, trained with bullet (see `training/razor_net.rs`). The net bytes
//! are embedded at compile time.
//!
//! Feature indexing matches bullet's `Chess768` exactly (verified both stm
//! colors): for a piece of color `col`, type `pt` (0..5), square `sq`:
//!   us_feat   = (col==stm ? 0 : 384) + 64*pt + relsq(stm, sq)
//!   them_feat = (col==stm ? 384 : 0) + 64*pt + relsq(!stm, sq)
//! where relsq(White, sq) = sq and relsq(Black, sq) = sq ^ 56.
//!
//! v1 refreshes both accumulators from scratch each eval. Incremental update
//! (the "efficiently updatable" part) is a later optimization — gated behind
//! its own SPRT since it must be a pure speedup, not an eval change.

use crate::bitboard::BitIter;
use crate::eval::Score;
use crate::position::Position;
use crate::types::{Color, PieceType};

const HIDDEN: usize = 512;
const QA: i32 = 255;
const QB: i32 = 64;
const SCALE: i32 = 400;

const NET_BYTES: &[u8] = include_bytes!("../nets/razor1.nnue");

struct Network {
    /// [768][512] feature weights (column-major in file == per-feature columns)
    feature_weights: Box<[[i16; HIDDEN]; 768]>,
    feature_bias: [i16; HIDDEN],
    /// [1024] output weights: first 512 for stm acc, last 512 for ntm acc
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

#[inline(always)]
fn screlu(x: i16) -> i32 {
    let y = (x as i32).clamp(0, QA);
    y * y
}

#[inline(always)]
fn relsq(persp: Color, sq: u8) -> usize {
    (if persp == Color::White { sq } else { sq ^ 56 }) as usize
}

/// NNUE static eval, side-to-move relative centipawns.
pub fn evaluate(pos: &Position) -> Score {
    let net = NET.get_or_init(load);
    let stm = pos.stm;

    // accumulators start at the feature bias
    let mut us = net.feature_bias;
    let mut them = net.feature_bias;

    for pt in PieceType::ALL {
        let pti = pt.idx();
        for col in [Color::White, Color::Black] {
            for sq in BitIter(pos.pieces(col, pt)) {
                let friendly = col == stm;
                let us_feat = if friendly { 0 } else { 384 } + 64 * pti + relsq(stm, sq);
                let them_feat =
                    if friendly { 384 } else { 0 } + 64 * pti + relsq(stm.flip(), sq);
                let uw = &net.feature_weights[us_feat];
                let tw = &net.feature_weights[them_feat];
                for i in 0..HIDDEN {
                    us[i] += uw[i];
                    them[i] += tw[i];
                }
            }
        }
    }

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
