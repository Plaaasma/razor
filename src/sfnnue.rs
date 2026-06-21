//! Stockfish NNUE compatibility layer (DIAGNOSTIC): load + evaluate with a
//! classic SFNNv5 HalfKAv2_hm net (nn-3c0aa92af1da.nnue, dims 22528-1024-15-32-8)
//! to test whether a genuinely-SOTA eval lifts Razor. Enabled only when the env
//! var SF_NET points at the .nnue file; otherwise this module is inert and HEAD
//! eval is unchanged. Non-incremental (full accumulator refresh per eval) —
//! fine for the fixed-nodes diagnostic.
//!
//! All formulas are source-exact from tools/stockfish/stockfish/src
//! (half_ka_v2_hm.{h,cpp}, affine_transform.h, nnue_feature_transformer.h).

use crate::bitboard::BitIter;
use crate::eval::Score;
use crate::position::Position;
use crate::types::{Color, PieceType};

const IN: usize = 22528;
const L1: usize = 1024; // feature-transformer / accumulator width
const HALF: usize = L1 / 2; // 512
const L2: usize = 15; // FC_0 logical outputs (fc0 emits L2+1)
const FC0_OUT: usize = L2 + 1; // 16
const L3: usize = 32;
const BUCKETS: usize = 8;
const PS_NB: usize = 704; // 11 * 64

// Read the on-disk int8 FC weights in scrambled order? Classic distributed nets
// are canonical (row-major), so a scalar port reads identity. Flip to true only
// if the fc_0 cross-check shows scrambled garbage.
const DESCRAMBLE: bool = false;

struct Bucket {
    fc0_b: [i32; FC0_OUT],
    fc0_w: Box<[i8]>, // [FC0_OUT * 1024] logical [out*1024 + in]
    fc1_b: [i32; L3],
    fc1_w: Box<[i8]>, // [L3 * 32]
    fc2_b: i32,
    fc2_w: [i8; 32],
}

struct Network {
    ft_bias: Box<[i16]>,  // [L1]
    ft_w: Box<[i16]>,     // [IN * L1], feature-major: ft_w[feat*L1 + d]
    psqt: Box<[i32]>,     // [IN * BUCKETS], psqt[feat*BUCKETS + bkt]
    buckets: Vec<Bucket>, // [BUCKETS]
}

// ---- feature indexing (HalfKAv2_hm) ----

#[inline(always)]
fn orient(ksq: u8) -> u8 {
    if (ksq & 7) < 4 { 7 } else { 0 }
}

// KingBuckets bucket number per raw square (×PS_NB gives the feature offset).
const KB: [u32; 64] = [
    28, 29, 30, 31, 31, 30, 29, 28, 24, 25, 26, 27, 27, 26, 25, 24, 20, 21, 22, 23, 23, 22, 21, 20,
    16, 17, 18, 19, 19, 18, 17, 16, 12, 13, 14, 15, 15, 14, 13, 12, 8, 9, 10, 11, 11, 10, 9, 8, 4,
    5, 6, 7, 7, 6, 5, 4, 0, 1, 2, 3, 3, 2, 1, 0,
];

// PieceSquareIndex[perspective][sf_piece], sf_piece = (color==Black?8:0)+(pt.idx()+1).
// values are the within-bucket offsets (0..=640).
const PS_W: [u32; 16] = [0, 0, 128, 256, 384, 512, 640, 0, 0, 64, 192, 320, 448, 576, 640, 0];
const PS_B: [u32; 16] = [0, 64, 192, 320, 448, 576, 640, 0, 0, 0, 128, 256, 384, 512, 640, 0];

#[inline(always)]
fn make_index(persp: Color, s: u8, sfpc: usize, ksq: u8) -> usize {
    let flip = if persp == Color::Black { 56u8 } else { 0 };
    let sq = (s ^ orient(ksq) ^ flip) as usize;
    let ps = if persp == Color::White { PS_W[sfpc] } else { PS_B[sfpc] } as usize;
    let bkt = KB[(ksq ^ flip) as usize] as usize * PS_NB;
    sq + ps + bkt
}

// ---- parser ----

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.b[self.at..self.at + 4].try_into().unwrap());
        self.at += 4;
        v
    }
    fn i16(&mut self) -> i16 {
        let v = i16::from_le_bytes(self.b[self.at..self.at + 2].try_into().unwrap());
        self.at += 2;
        v
    }
    fn i32(&mut self) -> i32 {
        let v = i32::from_le_bytes(self.b[self.at..self.at + 4].try_into().unwrap());
        self.at += 4;
        v
    }
    fn i8(&mut self) -> i8 {
        let v = self.b[self.at] as i8;
        self.at += 1;
        v
    }
}

#[inline]
fn scramble(i: usize, pad: usize, out: usize) -> usize {
    (i / 4) % (pad / 4) * out * 4 + i / pad * 4 + i % 4
}

fn read_affine(r: &mut Reader, out: usize, pad: usize) -> (Vec<i32>, Box<[i8]>) {
    let mut bias = vec![0i32; out];
    for b in bias.iter_mut() {
        *b = r.i32();
    }
    let mut w = vec![0i8; out * pad].into_boxed_slice();
    for i in 0..out * pad {
        let v = r.i8();
        let idx = if DESCRAMBLE { scramble(i, pad, out) } else { i };
        w[idx] = v;
    }
    (bias, w)
}

fn load(bytes: &[u8]) -> Network {
    let mut r = Reader { b: bytes, at: 0 };
    let ver = r.u32();
    assert_eq!(ver, 0x7AF3_2F20, "SF net version mismatch");
    let _hash = r.u32();
    let desc_len = r.u32() as usize;
    r.at += desc_len;
    let _ft_hash = r.u32();

    let mut ft_bias = vec![0i16; L1].into_boxed_slice();
    for b in ft_bias.iter_mut() {
        *b = r.i16();
    }
    let mut ft_w = vec![0i16; IN * L1].into_boxed_slice();
    for w in ft_w.iter_mut() {
        *w = r.i16();
    }
    let mut psqt = vec![0i32; IN * BUCKETS].into_boxed_slice();
    for p in psqt.iter_mut() {
        *p = r.i32();
    }

    let mut buckets = Vec::with_capacity(BUCKETS);
    for _ in 0..BUCKETS {
        let _arch_hash = r.u32();
        let (fc0_b, fc0_w) = read_affine(&mut r, FC0_OUT, L1); // pad = ceil(1024,32)=1024
        let (fc1_b, fc1_w) = read_affine(&mut r, L3, 32); // in 30 -> pad 32
        let (fc2_b, fc2_w) = read_affine(&mut r, 1, 32); // in 32 -> pad 32
        buckets.push(Bucket {
            fc0_b: fc0_b.try_into().unwrap(),
            fc0_w,
            fc1_b: fc1_b.try_into().unwrap(),
            fc1_w,
            fc2_b: fc2_b[0],
            fc2_w: fc2_w[..32].try_into().unwrap(),
        });
    }
    assert_eq!(r.at, bytes.len(), "SF net trailing bytes / size mismatch");
    Network { ft_bias, ft_w, psqt, buckets }
}

use std::sync::OnceLock;
static NET: OnceLock<Option<Network>> = OnceLock::new();

fn net() -> Option<&'static Network> {
    NET.get_or_init(|| std::env::var("SF_NET").ok().and_then(|p| std::fs::read(p).ok()).map(|b| load(&b)))
        .as_ref()
}

pub fn active() -> bool {
    net().is_some()
}

#[inline(always)]
fn ksq(pos: &Position, c: Color) -> u8 {
    pos.pieces(c, PieceType::King).trailing_zeros() as u8
}

pub fn evaluate_sf(pos: &Position) -> Score {
    let net = net().expect("evaluate_sf called without SF_NET");
    let wk = ksq(pos, Color::White);
    let bk = ksq(pos, Color::Black);

    let mut acc_w = vec![0i32; L1];
    let mut acc_b = vec![0i32; L1];
    for (d, &bias) in net.ft_bias.iter().enumerate() {
        acc_w[d] = bias as i32;
        acc_b[d] = bias as i32;
    }
    let mut psqt_w = [0i32; BUCKETS];
    let mut psqt_b = [0i32; BUCKETS];

    for pt in PieceType::ALL {
        for col in [Color::White, Color::Black] {
            let sfpc = if col == Color::Black { 8 } else { 0 } + pt.idx() + 1;
            for s in BitIter(pos.pieces(col, pt)) {
                let iw = make_index(Color::White, s, sfpc, wk);
                let ib = make_index(Color::Black, s, sfpc, bk);
                let (ow, ob) = (iw * L1, ib * L1);
                for d in 0..L1 {
                    acc_w[d] += net.ft_w[ow + d] as i32;
                    acc_b[d] += net.ft_w[ob + d] as i32;
                }
                for k in 0..BUCKETS {
                    psqt_w[k] += net.psqt[iw * BUCKETS + k];
                    psqt_b[k] += net.psqt[ib * BUCKETS + k];
                }
            }
        }
    }

    let bkt = (pos.occupied().count_ones() as usize - 1) / 4;
    let (us, them) = if pos.stm == Color::White { (&acc_w, &acc_b) } else { (&acc_b, &acc_w) };
    let (psq_us, psq_them) = if pos.stm == Color::White { (&psqt_w, &psqt_b) } else { (&psqt_b, &psqt_w) };

    // FT transform -> uint8 [L1]; classic net: clamp [0,127], product >> 7.
    let mut ft = [0u8; L1];
    for (p, persp) in [us, them].iter().enumerate() {
        let off = HALF * p;
        for j in 0..HALF {
            let a = persp[j].clamp(0, 127);
            let b = persp[j + HALF].clamp(0, 127);
            ft[off + j] = ((a * b) >> 7) as u8;
        }
    }

    let b = &net.buckets[bkt];
    // fc0
    let mut fc0 = [0i32; FC0_OUT];
    for o in 0..FC0_OUT {
        let mut s = b.fc0_b[o];
        let wrow = o * L1;
        for i in 0..L1 {
            s += ft[i] as i32 * b.fc0_w[wrow + i] as i32;
        }
        fc0[o] = s;
    }
    // activations
    let mut sqr = [0i32; FC0_OUT];
    let mut cr0 = [0i32; FC0_OUT];
    for i in 0..FC0_OUT {
        sqr[i] = ((fc0[i] as i64 * fc0[i] as i64) >> 19).min(127) as i32;
        cr0[i] = (fc0[i] >> 6).clamp(0, 127);
    }
    // fc1 input (30) = [sqr[0..15], cr0[0..15]], padded to 32
    let mut in1 = [0i32; 32];
    for i in 0..L2 {
        in1[i] = sqr[i];
        in1[L2 + i] = cr0[i];
    }
    let mut fc1 = [0i32; L3];
    for o in 0..L3 {
        let mut s = b.fc1_b[o];
        let wrow = o * 32;
        for i in 0..32 {
            s += in1[i] * b.fc1_w[wrow + i] as i32;
        }
        fc1[o] = s;
    }
    let mut cr1 = [0i32; 32];
    for i in 0..L3 {
        cr1[i] = (fc1[i] >> 6).clamp(0, 127);
    }
    // fc2
    let mut fc2 = b.fc2_b;
    for i in 0..L3 {
        fc2 += cr1[i] * b.fc2_w[i] as i32;
    }
    let fwd = fc0[L2] * 9600 / 8128;
    let positional = fc2 + fwd;
    let psqt = (psq_us[bkt] - psq_them[bkt]) / 2;
    // calibrate to razorsf cp scale (+Q ~1800) so search margins (tuned to that
    // scale) aren't confounded: raw +Q ~2978 * 3/5 ~= 1787.
    ((psqt + positional) / 16) * 3 / 5
}
