//! NNUE evaluation. Perspective network `(768 -> HIDDEN)x2 -> 1`, SCReLU, int16
//! quantized, trained with bullet (see `training/razor_net.rs`). HIDDEN is the
//! const below (512/768/1024 across net generations).
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
use std::sync::Mutex;

const HIDDEN: usize = 768;
const QA: i32 = 255;
const QB: i32 = 64;
const SCALE: i32 = 400;

const NET_BYTES: &[u8] = include_bytes!("../nets/razorsf.nnue");

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

/// UCI `EvalFile` override path. Set (before the first eval) by the UCI loop;
/// `load()` reads an external .nnue from here in preference to `RAZOR_NET` and
/// the embedded net. The net is initialized once (OnceLock) on first eval, and
/// UCI sets options before the first `go`, so an EvalFile set at launch takes
/// effect. Unset → embedded net, so bench is unchanged.
static EVAL_FILE: Mutex<Option<String>> = Mutex::new(None);

/// Point the NNUE loader at an external net file (UCI `EvalFile`). Only effective
/// if called before the first evaluation, since the net loads once and is then
/// fixed for the process. Returns false if the file can't be read.
pub fn set_eval_file(path: &str) -> bool {
    if path.is_empty() || path.eq_ignore_ascii_case("<empty>") {
        *EVAL_FILE.lock().unwrap() = None;
        return true;
    }
    if std::fs::metadata(path).is_err() {
        return false;
    }
    *EVAL_FILE.lock().unwrap() = Some(path.to_string());
    true
}

/// Whether the net has already been initialized (so EvalFile changes after this
/// point won't take effect — the UCI loop warns when that happens).
pub fn net_loaded() -> bool {
    NET.get().is_some()
}

fn load() -> Network {
    // Runtime net override: the UCI `EvalFile` option (via set_eval_file) wins,
    // then the RAZOR_NET env var (multigen datagen loop / rented farm), else the
    // embedded net. Lets each generation datagen with its new net without
    // recompiling. When neither is set, behaviour (and bench) is identical to
    // the embedded build.
    let owned;
    let eval_file = EVAL_FILE.lock().unwrap().clone();
    let from_path = eval_file
        .and_then(|p| std::fs::read(p).ok())
        .or_else(|| std::env::var("RAZOR_NET").ok().and_then(|p| std::fs::read(p).ok()));
    let bytes: &[u8] = match from_path {
        Some(b) => {
            owned = b;
            &owned
        }
        None => NET_BYTES,
    };
    let mut at = 0usize;
    let mut feature_weights = Box::new([[0i16; HIDDEN]; 768]);
    for col in feature_weights.iter_mut() {
        for w in col.iter_mut() {
            *w = rd_i16(bytes, &mut at);
        }
    }
    let mut feature_bias = [0i16; HIDDEN];
    for b in feature_bias.iter_mut() {
        *b = rd_i16(bytes, &mut at);
    }
    let mut output_weights = [0i16; 2 * HIDDEN];
    for w in output_weights.iter_mut() {
        *w = rd_i16(bytes, &mut at);
    }
    let output_bias = rd_i16(bytes, &mut at);
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

    // Scalar add/remove/eval: the compiler auto-vectorizes these i16 loops
    // under target-cpu=native (verified hand-written AVX2 intrinsics gave no
    // speedup over this — 2026-06-13). Keeps the aarch64/Spark build simple too.

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
        // i32 accumulation. At HIDDEN<=768 the sum stays in range and i32 is
        // ~8% faster than i64 (the i64 widening defeats i16 SIMD auto-vec) —
        // measured worth ~37 Elo at STC (control: i64 build lost 44.75% to this
        // i32 line, RESULTS). A >=1024 net's 2*HIDDEN-term sum can overflow i32;
        // restore i64 here if one is ever revived (guarded below).
        const _: () = assert!(HIDDEN <= 768, "i32 eval accumulation may overflow above 768; restore i64 in eval()");
        let mut output: i32 = 0;
        for i in 0..HIDDEN {
            output += screlu(us[i]) * net.output_weights[i] as i32;
            output += screlu(them[i]) * net.output_weights[HIDDEN + i] as i32;
        }
        output /= QA;
        output += net.output_bias as i32;
        output *= SCALE;
        output /= QA * QB;
        output as Score
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
