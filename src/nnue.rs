//! NNUE evaluation — THREAT net v1 with INCREMENTAL threat features.
//!
//! Network `((768 piece + 3200 victim-centric threats) -> HIDDEN)x2 -> 1`,
//! SCReLU, trained with bullet. HIDDEN = 768. SPLIT-QUANT feature weights: the
//! 768 PIECE rows are kept i16 (full eval precision), the 3200 THREAT rows are
//! stored i8 (half the bandwidth) — quantized from the i16 net at load() by
//! clamping each threat weight to [-127, 127] (99.85% of them already fit;
//! maxabs in the source net is 505). The threat i8 rows are widened to i16
//! INSIDE the add/sub kernels (never materialized as a widened row).
//! feature_bias, output_weights, and output_bias are i16. The i16 accumulator
//! is bit-identical to a build whose threat weights were the same i8-clamped
//! values; eval precision on the (dominant) piece term is unchanged from the
//! pure-i16 net.
//!
//! Two accumulators are maintained, one per board perspective (not per side to
//! move): `w` indexes features as if White is to move, `b` as if Black is. BOTH
//! piece AND threat features are summed into the same `w`/`b` vectors, so
//! `eval()` is a plain SCReLU dot over (piece + threat) — unchanged from the
//! piece-only net. They are updated incrementally on each move and never need a
//! full refresh during search.
//!
//! Piece feature (col, pt, sq):
//!   white perspective: (col==White ? 0 : 384) + 64*pt + sq
//!   black perspective: (col==Black ? 0 : 384) + 64*pt + (sq ^ 56)
//!
//! Threat feature (victim-centric, NO attacker square):
//!   att_pt, vic_pt in {P,N,B,R,Q} (king excluded as attacker AND victim),
//!   enemy = (att_color != vic_color), vic_sq in the perspective's orientation.
//!   local = ((att_pt*5 + vic_pt)*2 + enemy)*64 + vic_sq      (0..3200)
//!   index = 768 + local
//! White perspective uses vic_sq as-is; black uses vic_sq^56. enemy is
//! perspective-invariant. This index is bit-identical to the proven recompute
//! reference (matches/candidates/nnue-thr.rs) and bullet ThreatInputs.
//!
//! THREATS ARE INCREMENTAL: `refresh()` builds them from scratch (the oracle);
//! `apply()` maintains them via a discovered-ray delta producer ported from
//! Reckless/PlentyChess (single/move/mutate primitives, per-move-type
//! decomposition). Correctness is gated by the per-node debug-assert in
//! search.rs (`acc == refresh(pos)` on every searched node).

use crate::bitboard::{self, bb, BitIter, KING_ATTACKS, KNIGHT_ATTACKS};
use crate::eval::Score;
use crate::position::Position;
use crate::types::{flag, Color, Move, PieceType, Square};

const HIDDEN: usize = 768;
// the AVX2 FT kernels (add_row_i16/_i8 etc.) stride 16 i16 lanes with no
// remainder loop; HIDDEN must be a multiple of 16.
const _: () = assert!(HIDDEN % 16 == 0, "HIDDEN must be a multiple of 16 for the AVX2 FT kernels");
const NUM_FEAT: usize = 768 + 3200; // 3968
const NUM_PIECE: usize = 768; // feature rows 0..767 (i16)
const NUM_THREAT: usize = 3200; // feature rows 768..3967 (i8), index via -768
const THREAT_OFF: usize = NUM_PIECE; // 768
const QA: i32 = 255;
const QB: i32 = 64;
const SCALE: i32 = 400;

const NET_BYTES: &[u8] = include_bytes!("../nets/razorthr.nnue");

pub struct Network {
    // SPLIT-QUANT feature weights. PIECE rows stay i16 (full eval precision on
    // the dominant term); THREAT rows are i8 (half the bandwidth, clamped from
    // i16 at load). The threat FT widens each i8 weight to i16 inside its
    // add/sub kernels and NEVER materializes a widened row in memory.
    // feature_bias/output stay i16.
    piece_weights: Box<[[i16; HIDDEN]; NUM_PIECE]>,
    threat_weights: Box<[[i8; HIDDEN]; NUM_THREAT]>,
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
    // Runtime net override for datagen / rented farm: if RAZOR_NET names a
    // readable file, use it; otherwise the embedded net.
    let owned;
    let bytes: &[u8] = match std::env::var("RAZOR_NET").ok().and_then(|p| std::fs::read(p).ok()) {
        Some(b) => {
            owned = b;
            &owned
        }
        None => NET_BYTES,
    };
    let mut at = 0usize;
    // The net stores all NUM_FEAT feature rows as i16, in row order: piece rows
    // 0..767 first, then threat rows 768..3967. SPLIT-QUANT routing: piece rows
    // are stored i16 as-is (full precision); threat rows are clamped to
    // [-127, 127] and stored i8 (half bandwidth). The clamp is the ONLY lossy
    // step — 99.85% of threat weights already fit; the rest are saturated.
    let mut piece_weights: Box<[[i16; HIDDEN]; NUM_PIECE]> =
        vec![[0i16; HIDDEN]; NUM_PIECE].into_boxed_slice().try_into().unwrap();
    let mut threat_weights: Box<[[i8; HIDDEN]; NUM_THREAT]> =
        vec![[0i8; HIDDEN]; NUM_THREAT].into_boxed_slice().try_into().unwrap();
    for row in 0..NUM_FEAT {
        if row < THREAT_OFF {
            for w in piece_weights[row].iter_mut() {
                *w = rd_i16(bytes, &mut at);
            }
        } else {
            let dst = &mut threat_weights[row - THREAT_OFF];
            for w in dst.iter_mut() {
                *w = rd_i16(bytes, &mut at).clamp(-127, 127) as i8;
            }
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
    Network { piece_weights, threat_weights, feature_bias, output_weights, output_bias }
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

/// Threat feature base offset (everything but the victim square), keyed by
/// `idx = (att_pt*5 + vic_pt)*2 + enemy` for att_pt,vic_pt in 0..5, enemy in
/// 0..2. value = `((att_pt*5+vic_pt)*2+enemy)*64`. Hoists the per-toggle
/// multiply chain out of the innermost producer loop (called twice per toggle)
/// into a single table read; the final feature index is just
/// `768 + THR_BASE[idx] + vic_sq` (mirrored for black). Pure arithmetic
/// identity with the old inline computation.
const THR_BASE: [u16; 50] = {
    let mut t = [0u16; 50];
    let mut i = 0;
    while i < 50 {
        t[i] = (i as u16) * 64; // i == (att_pt*5+vic_pt)*2+enemy
        i += 1;
    }
    t
};

/// White-perspective THREAT-LOCAL feature index in 0..3200 (the row index into
/// `threat_weights`; the global feature index would be `THREAT_OFF + this`).
/// vic_sq is the raw victim square.
#[inline(always)]
fn thr_feat_w(att_pt: usize, vic_pt: usize, enemy: usize, vic_sq: Square) -> usize {
    THR_BASE[(att_pt * 5 + vic_pt) * 2 + enemy] as usize + vic_sq as usize
}

/// Black-perspective THREAT-LOCAL feature index in 0..3200 (victim square
/// mirrored).
#[inline(always)]
fn thr_feat_b(att_pt: usize, vic_pt: usize, enemy: usize, vic_sq: Square) -> usize {
    THR_BASE[(att_pt * 5 + vic_pt) * 2 + enemy] as usize + (vic_sq ^ 56) as usize
}

#[inline(always)]
fn piece_attacks(pt: usize, c: Color, from: Square, occ: u64) -> u64 {
    match pt {
        0 => bitboard::pawn_attacks(c, from),
        1 => KNIGHT_ATTACKS[from as usize],
        2 => bitboard::bishop_attacks(from, occ),
        3 => bitboard::rook_attacks(from, occ),
        4 => bitboard::queen_attacks(from, occ),
        _ => KING_ATTACKS[from as usize],
    }
}

/// Enumerate all active threats in `pos`; calls `f(att_pt, vic_pt, enemy,
/// vic_sq)` once per threat. King is excluded as attacker AND victim. The
/// victim mask is ALL occupied squares (both colors); `enemy` records whether
/// attacker and victim differ in color. This is the from-scratch oracle and is
/// bit-identical to the recompute reference's for_each_threat.
#[inline]
fn for_each_threat<F: FnMut(usize, usize, usize, Square)>(pos: &Position, mut f: F) {
    let occ = pos.occupied();
    for att_c in [Color::White, Color::Black] {
        for att_pt in 0..5usize {
            for from in BitIter(pos.pieces(att_c, PieceType::ALL[att_pt])) {
                let mut tgt = piece_attacks(att_pt, att_c, from, occ) & occ;
                while tgt != 0 {
                    let vsq = tgt.trailing_zeros() as Square;
                    tgt &= tgt - 1;
                    if let Some((vic_c, vic_pt)) = pos.piece_on(vsq) {
                        let vp = vic_pt.idx();
                        if vp == 5 {
                            continue; // king victim excluded
                        }
                        let enemy = (att_c != vic_c) as usize;
                        f(att_pt, vp, enemy, vsq);
                    }
                }
            }
        }
    }
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

    /// Full rebuild from a position: piece features then threat features. This
    /// is the from-scratch oracle the per-node correctness gate compares
    /// against, and is used for root init and `evaluate()`.
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
        for_each_threat(pos, |att_pt, vic_pt, enemy, vsq| {
            acc.thr_add(att_pt, vic_pt, enemy, vsq);
        });
        acc
    }

    // Scalar add/remove/eval: the compiler auto-vectorizes these i16 loops
    // under target-cpu=native (verified hand-written AVX2 gave no speedup —
    // 2026-06-13). Keeps the aarch64/Spark build simple too.

    #[inline(always)]
    fn add(&mut self, col: Color, pt: usize, sq: Square) {
        let net = net();
        add_row_i16(&mut self.w, &net.piece_weights[wp_feat(col, pt, sq)]);
        add_row_i16(&mut self.b, &net.piece_weights[bp_feat(col, pt, sq)]);
    }

    #[inline(always)]
    fn remove(&mut self, col: Color, pt: usize, sq: Square) {
        let net = net();
        sub_row_i16(&mut self.w, &net.piece_weights[wp_feat(col, pt, sq)]);
        sub_row_i16(&mut self.b, &net.piece_weights[bp_feat(col, pt, sq)]);
    }

    /// Add a single threat feature row into w/b (used by the from-scratch
    /// refresh oracle; the incremental path batches instead — see ThreatBatch).
    /// Threat rows are i8 (split-quant) and widened to i16 inside the kernel.
    #[inline(always)]
    fn thr_add(&mut self, att_pt: usize, vic_pt: usize, enemy: usize, vsq: Square) {
        let net = net();
        add_row_i8(&mut self.w, &net.threat_weights[thr_feat_w(att_pt, vic_pt, enemy, vsq)]);
        add_row_i8(&mut self.b, &net.threat_weights[thr_feat_b(att_pt, vic_pt, enemy, vsq)]);
    }

    /// Evaluate from the side-to-move's perspective. Threats are already folded
    /// into w/b, so this is a plain SCReLU dot — unchanged from the piece-only
    /// net.
    pub fn eval(&self, stm: Color) -> Score {
        let net = net();
        let (us, them) = if stm == Color::White { (&self.w, &self.b) } else { (&self.b, &self.w) };
        // i32 accumulation. At HIDDEN<=768 the sum stays in range and i32 is
        // ~8% faster than i64 (the i64 widening defeats i16 SIMD auto-vec).
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

// ---------------------------------------------------------------------------
// Batched threat delta application. The producer pushes signed (w_idx, b_idx)
// feature-row toggles into a buffer; after the whole move is enumerated the
// buffer is netted (a threat removed then re-added by the same move cancels at
// the index level) and only the residual rows are applied to w/b in one fused
// pass. This is the dominant nps lever: per-toggle HIDDEN-length i16 add/sub is
// ~3x the producer-geometry cost, and most toggles cancel, so netting first
// then applying the few survivors recovers most of the piece-only speed.
// ---------------------------------------------------------------------------

const MAX_DELTAS: usize = 128; // hard upper bound on toggles per move

struct ThreatBatch {
    // each entry is (w_feature_idx, b_feature_idx, signed_count)
    w_idx: [u16; MAX_DELTAS],
    b_idx: [u16; MAX_DELTAS],
    sign: [i16; MAX_DELTAS],
    len: usize,
}

impl ThreatBatch {
    #[inline(always)]
    fn new() -> ThreatBatch {
        ThreatBatch { w_idx: [0; MAX_DELTAS], b_idx: [0; MAX_DELTAS], sign: [0; MAX_DELTAS], len: 0 }
    }

    /// Push one toggled threat tuple, skipping king attacker/victim. Coalesces
    /// against the existing buffer so removed-then-readded cancels to zero.
    #[inline(always)]
    fn toggle(&mut self, att_pt: usize, vic_pt: usize, enemy: usize, vsq: Square, add: bool) {
        if att_pt == 5 || vic_pt == 5 {
            return; // king never an attacker or victim
        }
        let wf = thr_feat_w(att_pt, vic_pt, enemy, vsq) as u16;
        let bf = thr_feat_b(att_pt, vic_pt, enemy, vsq) as u16;
        let s: i16 = if add { 1 } else { -1 };
        // coalesce with an existing identical feature pair (cheap for the small
        // buffers a single move produces — the loop body is branch-predictable
        // and the buffer is hot in cache)
        for i in 0..self.len {
            if self.w_idx[i] == wf {
                self.sign[i] += s;
                return;
            }
        }
        debug_assert!(self.len < MAX_DELTAS, "threat delta buffer overflow");
        let i = self.len;
        self.w_idx[i] = wf;
        self.b_idx[i] = bf;
        self.sign[i] = s;
        self.len = i + 1;
    }

    /// Apply the netted residual to the accumulator in one fused pass. The
    /// add/sub of each survivor THREAT row into w/b is the dominant FT cost on
    /// the threat path, so the common +1/-1 cases run the AVX2 widen-fused i8
    /// kernel (i8 row -> i16 add/sub); the rare |sign| > 1 survivor runs the
    /// widen-fused multiply-add (`madd_row_i8`). All three never store a widened
    /// row to memory. Indices in w_idx/b_idx are THREAT-LOCAL (rows into
    /// `threat_weights`, 0..3200).
    #[inline]
    fn apply_to(&self, acc: &mut Accumulator) {
        let net = net();
        // Software-prefetch every survivor threat row up front. The i8 threat
        // weight table is ~2.3 MB and survivor indices (~5-6/move) are scattered
        // across it, so each row is a likely L2/L3 miss; issuing the loads
        // before the dependent AVX2 widen+add/sub hides that latency behind
        // useful work. 768 B/row spans 12 cache lines, so we touch the row head
        // (the rest streams in once the kernel starts).
        #[cfg(target_feature = "avx2")]
        unsafe {
            use std::arch::x86_64::_mm_prefetch;
            use std::arch::x86_64::_MM_HINT_T0;
            for i in 0..self.len {
                if self.sign[i] == 0 {
                    continue;
                }
                _mm_prefetch(net.threat_weights.as_ptr().add(self.w_idx[i] as usize) as *const i8, _MM_HINT_T0);
                _mm_prefetch(net.threat_weights.as_ptr().add(self.b_idx[i] as usize) as *const i8, _MM_HINT_T0);
            }
        }
        for i in 0..self.len {
            let s = self.sign[i];
            if s == 0 {
                continue;
            }
            let wf = &net.threat_weights[self.w_idx[i] as usize];
            let bf = &net.threat_weights[self.b_idx[i] as usize];
            if s == 1 {
                add_row_i8(&mut acc.w, wf);
                add_row_i8(&mut acc.b, bf);
            } else if s == -1 {
                sub_row_i8(&mut acc.w, wf);
                sub_row_i8(&mut acc.b, bf);
            } else {
                madd_row_i8(&mut acc.w, wf, s);
                madd_row_i8(&mut acc.b, bf, s);
            }
        }
    }
}

/// `dst[k] += src[k]` over HIDDEN for an i16 PIECE row. AVX2 (16 lanes/iter)
/// when available: straight i16 load+add+store, no widening (piece weights are
/// already i16, full precision). Scalar fallback otherwise.
#[inline(always)]
fn add_row_i16(dst: &mut [i16; HIDDEN], src: &[i16; HIDDEN]) {
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        let mut k = 0;
        while k < HIDDEN {
            let a = _mm256_loadu_si256(dst.as_ptr().add(k) as *const __m256i);
            let b = _mm256_loadu_si256(src.as_ptr().add(k) as *const __m256i);
            _mm256_storeu_si256(dst.as_mut_ptr().add(k) as *mut __m256i, _mm256_add_epi16(a, b));
            k += 16;
        }
    }
    #[cfg(not(target_feature = "avx2"))]
    for k in 0..HIDDEN {
        dst[k] += src[k];
    }
}

/// `dst[k] -= src[k]` over HIDDEN for an i16 PIECE row. See `add_row_i16`.
#[inline(always)]
fn sub_row_i16(dst: &mut [i16; HIDDEN], src: &[i16; HIDDEN]) {
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        let mut k = 0;
        while k < HIDDEN {
            let a = _mm256_loadu_si256(dst.as_ptr().add(k) as *const __m256i);
            let b = _mm256_loadu_si256(src.as_ptr().add(k) as *const __m256i);
            _mm256_storeu_si256(dst.as_mut_ptr().add(k) as *mut __m256i, _mm256_sub_epi16(a, b));
            k += 16;
        }
    }
    #[cfg(not(target_feature = "avx2"))]
    for k in 0..HIDDEN {
        dst[k] -= src[k];
    }
}

/// `dst[k] += src[k]` over HIDDEN, widening the i8 THREAT `src` to i16 INLINE.
/// AVX2 (16 lanes/iter) when available: load 16 i8 with `_mm_loadu_si128`,
/// sign-extend to 16 i16 with `_mm256_cvtepi8_epi16`, add into the i16
/// accumulator. The widened row is NEVER stored to memory — fusing the widen
/// into the add is the whole point of the split-quant i8 threat design (it
/// halves threat-row bandwidth without materializing a widened row).
#[inline(always)]
fn add_row_i8(dst: &mut [i16; HIDDEN], src: &[i8; HIDDEN]) {
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        let mut k = 0;
        while k < HIDDEN {
            let a = _mm256_loadu_si256(dst.as_ptr().add(k) as *const __m256i);
            let b = _mm256_cvtepi8_epi16(_mm_loadu_si128(src.as_ptr().add(k) as *const __m128i));
            _mm256_storeu_si256(dst.as_mut_ptr().add(k) as *mut __m256i, _mm256_add_epi16(a, b));
            k += 16;
        }
    }
    #[cfg(not(target_feature = "avx2"))]
    for k in 0..HIDDEN {
        dst[k] += src[k] as i16;
    }
}

/// `dst[k] -= src[k]` over HIDDEN, widening the i8 THREAT `src` to i16 INLINE.
/// AVX2 (16 lanes/iter) when available; see `add_row_i8` for the widen-fusion
/// rationale.
#[inline(always)]
fn sub_row_i8(dst: &mut [i16; HIDDEN], src: &[i8; HIDDEN]) {
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        let mut k = 0;
        while k < HIDDEN {
            let a = _mm256_loadu_si256(dst.as_ptr().add(k) as *const __m256i);
            let b = _mm256_cvtepi8_epi16(_mm_loadu_si128(src.as_ptr().add(k) as *const __m128i));
            _mm256_storeu_si256(dst.as_mut_ptr().add(k) as *mut __m256i, _mm256_sub_epi16(a, b));
            k += 16;
        }
    }
    #[cfg(not(target_feature = "avx2"))]
    for k in 0..HIDDEN {
        dst[k] -= src[k] as i16;
    }
}

/// `dst[k] += s * src[k]` over HIDDEN, widening the i8 THREAT `src` to i16
/// INLINE and scaling by the (rare) survivor count `|s| > 1`. AVX2: widen 16 i8
/// -> i16, then `_mm256_mullo_epi16` against a broadcast of `s` before adding.
/// The widened row is never stored. Scalar fallback widens then multiplies in
/// i16, matching the `±1` kernels' arithmetic exactly.
#[inline(always)]
fn madd_row_i8(dst: &mut [i16; HIDDEN], src: &[i8; HIDDEN], s: i16) {
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        let mul = _mm256_set1_epi16(s);
        let mut k = 0;
        while k < HIDDEN {
            let a = _mm256_loadu_si256(dst.as_ptr().add(k) as *const __m256i);
            let b = _mm256_cvtepi8_epi16(_mm_loadu_si128(src.as_ptr().add(k) as *const __m128i));
            let scaled = _mm256_mullo_epi16(b, mul);
            _mm256_storeu_si256(dst.as_mut_ptr().add(k) as *mut __m256i, _mm256_add_epi16(a, scaled));
            k += 16;
        }
    }
    #[cfg(not(target_feature = "avx2"))]
    for k in 0..HIDDEN {
        dst[k] += s * src[k] as i16;
    }
}

// ---------------------------------------------------------------------------
// Incremental threat delta producer (faithful port of Reckless
// push_threats_single / _on_move / _on_mutate). A move is replayed on a running
// board ONE PRIMITIVE AT A TIME; after each primitive mutates the running state,
// the observer fires against state ALREADY reflecting that single change. So
// every producer call sees a consistent occupancy and reasons about only one
// square toggling — exactly Reckless's invariant — and the per-move batches
// telescope to refresh(child) - refresh(parent).
//
// CLONE ELIMINATION: the running board used to be a full `Position` copy
// (`piece_bb + color_bb + [u8;64] mailbox + scalars + zobrist key`, ~152 B,
// re-copied on every searched node) mutated primitive-by-primitive. The
// producer only ever reads piece/color BITBOARDS and a per-square piece lookup,
// so the running board is now a compact `RunBoard` of just the eight u64
// bitboards (64 B, half the copy, no zobrist key to maintain). `piece_on` is
// derived from those bitboards. Delta SEMANTICS are byte-for-byte unchanged —
// only the storage/lookup of occupancy and piece-type info changed; the per-node
// refresh-vs-incremental assert in search.rs gates that exactly.
//
// `board` is the RUNNING board (post-primitive). For a removal the toggled
// piece is already gone from `board`; for an addition it is already present.
// pt/c/sq are passed explicitly (not read from board) because the toggled
// piece's identity is what changed. `occ` is the running occupancy snapshot.
// ---------------------------------------------------------------------------

/// Compact running board for the threat replay: the per-piece-type and
/// per-color bitboards plus a mailbox for O(1) `piece_on`. No scalars, no
/// zobrist key (the threat replay never needs them), so this is lighter than a
/// full `Position` copy and — crucially — its mutation helpers skip the zobrist
/// XOR work `Position::put`/`remove` do. `piece_on` stays a single mailbox read,
/// which the producer's scan loops call heavily; a bitboard-derived lookup
/// measured slower here.
#[derive(Copy, Clone)]
struct RunBoard {
    piece_bb: [u64; 6],
    color_bb: [u64; 2],
    mailbox: [u8; 64],
}

const RB_EMPTY: u8 = 12;

impl RunBoard {
    #[inline(always)]
    fn from_pos(pos: &Position) -> RunBoard {
        RunBoard { piece_bb: pos.piece_bb, color_bb: pos.color_bb, mailbox: pos.mailbox }
    }

    #[inline(always)]
    fn occupied(&self) -> u64 {
        self.color_bb[0] | self.color_bb[1]
    }

    #[inline(always)]
    fn pieces(&self, c: Color, pt: PieceType) -> u64 {
        self.piece_bb[pt.idx()] & self.color_bb[c.idx()]
    }

    /// Color + piece-type index at an occupied square (one mailbox read). Caller
    /// guarantees `sq` is occupied: every call site scans a mask AND'd with
    /// occupancy.
    #[inline(always)]
    fn piece_on(&self, sq: Square) -> (Color, usize) {
        let v = self.mailbox[sq as usize];
        let c = if v < 6 { Color::White } else { Color::Black };
        (c, (v % 6) as usize)
    }

    #[inline(always)]
    fn remove(&mut self, c: Color, pt: usize, sq: Square) {
        let m = !bb(sq);
        self.piece_bb[pt] &= m;
        self.color_bb[c.idx()] &= m;
        self.mailbox[sq as usize] = RB_EMPTY;
    }

    #[inline(always)]
    fn put(&mut self, c: Color, pt: usize, sq: Square) {
        let m = bb(sq);
        self.piece_bb[pt] |= m;
        self.color_bb[c.idx()] |= m;
        self.mailbox[sq as usize] = c.idx() as u8 * 6 + pt as u8;
    }
}

/// One piece (pt, c) appears (add=true) / vanishes (add=false) at `sq`.
/// `occ` is the occupancy with `sq` in its post-primitive state.
///
/// `no_rays` is the SHARED-RAY ELISION mask: a set of squares (the relocate
/// endpoints) such that any slider collinear with BOTH `sq` and every square in
/// `no_rays` has its discovered "beyond" toggle computed identically in both
/// halves of the relocate (from-removal and to-addition) — equal-and-opposite,
/// they cancel in the ThreatBatch coalescer after both full geometry passes. We
/// skip emitting the second one. relocate() passes `bb(from)|bb(to)`; change()
/// and the from-scratch refresh path pass 0 (then `line_through & 0 != 0` is
/// always false, so nothing is ever skipped — no behavior change off the
/// relocate path).
fn single(batch: &mut ThreatBatch, board: &RunBoard, occ: u64, pt: usize, c: Color, sq: Square, add: bool, no_rays: u64) {
    // Slider geometry for CLASS B, computed first so CLASS A can reuse it for
    // queens. `queen_a = b_att | r_att` is bit-identical to
    // `queen_attacks(sq, occ)` by construction.
    let all_b = board.pieces(Color::White, PieceType::Bishop) | board.pieces(Color::Black, PieceType::Bishop);
    let all_r = board.pieces(Color::White, PieceType::Rook) | board.pieces(Color::Black, PieceType::Rook);
    let all_q = board.pieces(Color::White, PieceType::Queen) | board.pieces(Color::Black, PieceType::Queen);
    let bq = all_b | all_q;
    let rq = all_r | all_q;
    let b_att = bitboard::bishop_attacks(sq, occ);
    let r_att = bitboard::rook_attacks(sq, occ);
    let queen_a = b_att | r_att;

    // CLASS A — the toggled piece's own outgoing threats. For a queen, reuse
    // `queen_a` instead of re-issuing the two PEXT lookups inside
    // piece_attacks(4, ...).
    let mut attacked = if pt == 4 { queen_a & occ } else { piece_attacks(pt, c, sq, occ) & occ };
    while attacked != 0 {
        let v = attacked.trailing_zeros() as Square;
        attacked &= attacked - 1;
        let (vc, vpt) = board.piece_on(v);
        batch.toggle(pt, vpt, (c != vc) as usize, v, add);
    }

    // CLASS B — incoming sliders attacking sq + discovered/blocked rays.
    let diag = b_att & bq;
    let orth = r_att & rq;
    let mut sliders = (diag | orth) & occ;
    while sliders != 0 {
        let s = sliders.trailing_zeros() as Square;
        sliders &= sliders - 1;
        let (sc, s_idx) = board.piece_on(s);
        // discovered ray: the one square the slider on `s` newly sees / loses
        // sight of when the piece on `sq` toggles. INVERTED flag: appear blocks
        // (remove that beyond-threat), vanish unblocks (add it).
        //
        // SHARED-RAY ELISION: when this slider is collinear with BOTH relocate
        // endpoints (both bits of `no_rays` lie on the full inclusive line
        // through s and sq), the from-half and to-half each emit the same
        // beyond-toggle with opposite sign, so they net to zero. Skip them both.
        // We test the FULL line (`line_through`, both endpoints inclusive), not
        // `ray_past` (which excludes the near endpoint and would mis-classify
        // far-side sliders). The `no_rays != 0` guard keeps the change()/refresh
        // path (no_rays == 0) firing every beyond emit: an empty mask is
        // trivially a subset of any line, so it must NOT count as "collinear with
        // both endpoints".
        let elide = no_rays != 0 && (bitboard::line_through(s, sq) & no_rays) == no_rays;
        if !elide {
            let beyond = bitboard::ray_past(s, sq) & occ & queen_a;
            if beyond != 0 {
                let u = beyond.trailing_zeros() as Square;
                let (uc, upt) = board.piece_on(u);
                batch.toggle(s_idx, upt, (sc != uc) as usize, u, !add);
            }
        }
        // the slider's direct threat on the toggled piece itself (ALWAYS fires
        // in both halves — it nets correctly through the coalescer).
        batch.toggle(s_idx, pt, (sc != c) as usize, sq, add);
    }

    // CLASS C — incoming leapers (pawn/knight/king) attacking sq.
    let bp = board.pieces(Color::Black, PieceType::Pawn) & bitboard::pawn_attacks(Color::White, sq);
    let wp = board.pieces(Color::White, PieceType::Pawn) & bitboard::pawn_attacks(Color::Black, sq);
    let knights = (board.pieces(Color::White, PieceType::Knight)
        | board.pieces(Color::Black, PieceType::Knight))
        & KNIGHT_ATTACKS[sq as usize];
    let kings = (board.pieces(Color::White, PieceType::King)
        | board.pieces(Color::Black, PieceType::King))
        & KING_ATTACKS[sq as usize];
    let mut leapers = (bp | wp | knights | kings) & occ;
    while leapers != 0 {
        let a = leapers.trailing_zeros() as Square;
        leapers &= leapers - 1;
        let (ac, apt) = board.piece_on(a);
        // king-as-attacker dropped in batch.toggle.
        batch.toggle(apt, pt, (ac != c) as usize, sq, add);
    }
}

/// A piece (pt, c) relocates from `from` to `to` on the running board `b`
/// (quiet / double-push / ep-pawn / castle-king). Mutates `b` to reflect the
/// relocation, then fires the from-removal and to-addition halves against the
/// canonical Reckless snapshot (occupancy with BOTH endpoints cleared), so the
/// from-scan's rays through `from` are not stopped by the arrived piece.
fn relocate(batch: &mut ThreatBatch, b: &mut RunBoard, pt: usize, c: Color, from: Square, to: Square) {
    b.remove(c, pt, from);
    b.put(c, pt, to);
    let occ = b.occupied() ^ bb(to);
    // Sliders collinear with both endpoints emit equal-and-opposite beyond-rays
    // across the two halves; the shared-ray mask lets single() skip the second.
    let no_rays = bb(from) | bb(to);
    single(batch, b, occ, pt, c, from, false, no_rays);
    single(batch, b, occ, pt, c, to, true, no_rays);
}

/// A piece (pt, c) appears (add=true) / vanishes (add=false) at `sq` on the
/// running board `b`. Mutates `b` then fires single() against the post-mutation
/// occupancy.
fn change(batch: &mut ThreatBatch, b: &mut RunBoard, pt: usize, c: Color, sq: Square, add: bool) {
    if add {
        b.put(c, pt, sq);
    } else {
        b.remove(c, pt, sq);
    }
    single(batch, b, b.occupied(), pt, c, sq, add, 0);
}

/// A piece at `sq` changes IDENTITY in place on the running board `b`: from
/// (old_pt, old_c) to (new_pt, new_c). Occupancy is unchanged (sq stays
/// occupied). Used for capture (enemy victim -> our mover, a color+type change)
/// and promotion (pawn -> promoted piece). Mutates `b` (old off, new on) then
/// re-points all threats touching `sq`. NO discovered-ray handling — occupancy
/// at sq did not change, only identity.
fn mutate(
    batch: &mut ThreatBatch,
    b: &mut RunBoard,
    old_pt: usize,
    old_c: Color,
    new_pt: usize,
    new_c: Color,
    sq: Square,
) {
    b.remove(old_c, old_pt, sq);
    b.put(new_c, new_pt, sq);
    let board: &RunBoard = b;
    let occ = board.occupied();
    // outgoing: old piece's attacks off, new piece's attacks on.
    let mut a_old = piece_attacks(old_pt, old_c, sq, occ) & occ;
    while a_old != 0 {
        let v = a_old.trailing_zeros() as Square;
        a_old &= a_old - 1;
        let (vc, vpt) = board.piece_on(v);
        batch.toggle(old_pt, vpt, (old_c != vc) as usize, v, false);
    }
    let mut a_new = piece_attacks(new_pt, new_c, sq, occ) & occ;
    while a_new != 0 {
        let v = a_new.trailing_zeros() as Square;
        a_new &= a_new - 1;
        let (vc, vpt) = board.piece_on(v);
        batch.toggle(new_pt, vpt, (new_c != vc) as usize, v, true);
    }
    // incoming: every slider/leaper attacking sq re-points (old_pt,old_c) victim
    // -> (new_pt,new_c) victim. The enemy bit depends on the victim's color too,
    // so it is recomputed per old/new color.
    let all_b = board.pieces(Color::White, PieceType::Bishop) | board.pieces(Color::Black, PieceType::Bishop);
    let all_r = board.pieces(Color::White, PieceType::Rook) | board.pieces(Color::Black, PieceType::Rook);
    let all_q = board.pieces(Color::White, PieceType::Queen) | board.pieces(Color::Black, PieceType::Queen);
    let bq = all_b | all_q;
    let rq = all_r | all_q;
    let diag = bitboard::bishop_attacks(sq, occ) & bq;
    let orth = bitboard::rook_attacks(sq, occ) & rq;
    let bp = board.pieces(Color::Black, PieceType::Pawn) & bitboard::pawn_attacks(Color::White, sq);
    let wp = board.pieces(Color::White, PieceType::Pawn) & bitboard::pawn_attacks(Color::Black, sq);
    let knights = (board.pieces(Color::White, PieceType::Knight)
        | board.pieces(Color::Black, PieceType::Knight))
        & KNIGHT_ATTACKS[sq as usize];
    let kings = (board.pieces(Color::White, PieceType::King)
        | board.pieces(Color::Black, PieceType::King))
        & KING_ATTACKS[sq as usize];
    let mut atk = ((diag | orth) | bp | wp | knights | kings) & occ;
    while atk != 0 {
        let a = atk.trailing_zeros() as Square;
        atk &= atk - 1;
        let (ac, ai) = board.piece_on(a);
        batch.toggle(ai, old_pt, (ac != old_c) as usize, sq, false);
        batch.toggle(ai, new_pt, (ac != new_c) as usize, sq, true);
    }
}

/// Produce the child accumulator after playing `mv`. `pos` is the PARENT
/// position (before the move); the post-move board is reconstructed internally
/// on the RunBoard, so no child position argument is needed. Maintains both
/// piece AND threat features incrementally.
///
/// PIECE features use the existing add/remove against the move decomposition.
/// THREAT features are replayed on a mutable copy `b` of the parent: each
/// primitive mutates `b` then fires the producer against the post-primitive
/// board, exactly mirroring Reckless's make_move observer sequencing. The
/// primitive ORDER per move type matches Position::make (with castling reordered
/// to all-removes-before-all-adds so the king's transit and rook relocation see
/// consistent occupancies for discovered rays).
pub fn apply_lazy(parent: &Accumulator, pos: &Position, mv: Move) -> Accumulator {
    let mut acc = parent.clone();
    let us = pos.stm;
    let them = us.flip();
    let from = mv.from();
    let to = mv.to();
    let (_, pt) = pos.piece_on(from).expect("nnue apply: empty from");
    let pti = pt.idx();

    // running board for the threat replay, starts as the parent; threat toggles
    // are collected in `batch`, netted, then applied to `acc` in one fused pass.
    // Compact (8 u64 bitboards) — no Position clone, no mailbox/zobrist copy.
    let mut b = RunBoard::from_pos(pos);
    let mut batch = ThreatBatch::new();

    match mv.flags() {
        flag::QUIET | flag::DOUBLE_PUSH => {
            acc.remove(us, pti, from);
            acc.add(us, pti, to);
            relocate(&mut batch, &mut b, pti, us, from, to);
        }
        flag::CASTLE_KING | flag::CASTLE_QUEEN => {
            let (rook_from, rook_to) = match (us, mv.flags()) {
                (Color::White, flag::CASTLE_KING) => (7u8, 5u8),
                (Color::White, _) => (0, 3),
                (Color::Black, flag::CASTLE_KING) => (63, 61),
                (Color::Black, _) => (56, 59),
            };
            let ki = PieceType::King.idx();
            let ri = PieceType::Rook.idx();
            // piece features
            acc.remove(us, ki, from);
            acc.add(us, ki, to);
            acc.remove(us, ri, rook_from);
            acc.add(us, ri, rook_to);
            // threats: all removes before all adds (4 single primitives). The
            // king contributes no own threats (excluded) but its occupancy
            // change still toggles other pieces' discovered rays, so it must run.
            change(&mut batch, &mut b, ki, us, from, false);
            change(&mut batch, &mut b, ri, us, rook_from, false);
            change(&mut batch, &mut b, ki, us, to, true);
            change(&mut batch, &mut b, ri, us, rook_to, true);
        }
        flag::EN_PASSANT => {
            let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
            let pi = PieceType::Pawn.idx();
            // piece features
            acc.remove(them, pi, cap_sq);
            acc.remove(us, pi, from);
            acc.add(us, pi, to);
            // threats: pawn relocates from->to (the EP victim is still on cap_sq
            // at this point, blocking/exposing rays as in the real intermediate
            // state), THEN the victim is removed from its third square.
            relocate(&mut batch, &mut b, pi, us, from, to);
            change(&mut batch, &mut b, pi, them, cap_sq, false);
        }
        flag::CAPTURE => {
            let (_, cap_pt) = pos.piece_on(to).expect("nnue apply: empty capture");
            let ci = cap_pt.idx();
            // piece features
            acc.remove(them, ci, to);
            acc.remove(us, pti, from);
            acc.add(us, pti, to);
            // threats: mirror Reckless capture order — mover LEAVES `from` first
            // (opening rays through `from`, with the victim still on `to`), then
            // the victim square's occupant changes identity from captured(enemy)
            // -> mover(us) (a mutate; occupancy at `to` never toggles).
            change(&mut batch, &mut b, pti, us, from, false);
            mutate(&mut batch, &mut b, ci, them, pti, us, to);
        }
        f if f & 8 != 0 => {
            // promotions, with or without capture
            let promo = mv.promo_piece().idx();
            let pi = PieceType::Pawn.idx();
            if mv.is_capture() {
                let (_, cap_pt) = pos.piece_on(to).expect("nnue apply: empty promo-cap");
                let ci = cap_pt.idx();
                // piece features
                acc.remove(them, ci, to);
                acc.remove(us, pi, from);
                acc.add(us, promo, to);
                // threats: mover (pawn) leaves `from`; victim on `to` mutates to
                // PAWN (the mover landing); then PAWN mutates to the promoted
                // piece. The intermediate pawn-on-to is the fictional state
                // Reckless materializes via two chained mutates — mandatory so
                // incoming-attacker victim re-pointing is computed per type.
                change(&mut batch, &mut b, pi, us, from, false);
                mutate(&mut batch, &mut b, ci, them, pi, us, to);
                mutate(&mut batch, &mut b, pi, us, promo, us, to);
            } else {
                // quiet promotion: pawn relocates from->to, then mutates to promo.
                acc.remove(us, pi, from);
                acc.add(us, promo, to);
                relocate(&mut batch, &mut b, pi, us, from, to);
                mutate(&mut batch, &mut b, pi, us, promo, us, to);
            }
        }
        _ => unreachable!(),
    }
    batch.apply_to(&mut acc);
    acc
}

/// Eager shim keeping the 4-arg signature for the search call sites and the
/// nnue tests. `_child` is unused (the producer reconstructs the post-move board
/// on the RunBoard itself); delegates to `apply_lazy`.
pub fn apply(parent: &Accumulator, pos: &Position, _child: &Position, mv: Move) -> Accumulator {
    apply_lazy(parent, pos, mv)
}

/// From-scratch eval (uci `eval`, datagen). Search uses the incremental stack.
pub fn evaluate(pos: &Position) -> Score {
    Accumulator::refresh(pos).eval(pos.stm)
}

/// REFERENCE eval matching the proven recompute candidate (matches/candidates/
/// nnue-thr.rs): piece accumulator widened to i32, threats recomputed into the
/// i32 scratch, final dot in i64. Used only to gate that the folded-into-i16
/// production eval is bit-identical (i.e. the i16 accumulator never overflows).
/// SPLIT-QUANT: this reference reads the SAME i8-clamped `threat_weights` the
/// production path uses, so it gates incremental-vs-recompute on the identical
/// (clamped) weights — it does NOT compare against the pre-clamp pure-i16 eval.
#[cfg(test)]
pub fn evaluate_recompute_ref(pos: &Position) -> Score {
    let net = net();
    // piece-only accumulator (bias + piece rows), in i16 — same as the candidate
    let mut acc = Accumulator { w: net.feature_bias, b: net.feature_bias };
    for pt in PieceType::ALL {
        for col in [Color::White, Color::Black] {
            for sq in BitIter(pos.pieces(col, pt)) {
                acc.add(col, pt.idx(), sq);
            }
        }
    }
    let mut sw = [0i32; HIDDEN];
    let mut sb = [0i32; HIDDEN];
    for i in 0..HIDDEN {
        sw[i] = acc.w[i] as i32;
        sb[i] = acc.b[i] as i32;
    }
    for_each_threat(pos, |att_pt, vic_pt, enemy, vsq| {
        let wf = &net.threat_weights[thr_feat_w(att_pt, vic_pt, enemy, vsq)];
        let bf = &net.threat_weights[thr_feat_b(att_pt, vic_pt, enemy, vsq)];
        for k in 0..HIDDEN {
            sw[k] += wf[k] as i32;
            sb[k] += bf[k] as i32;
        }
    });
    let (us, them) = if pos.stm == Color::White { (&sw, &sb) } else { (&sb, &sw) };
    let mut output: i64 = 0;
    for i in 0..HIDDEN {
        output += screlu_i32(us[i]) as i64 * net.output_weights[i] as i64;
        output += screlu_i32(them[i]) as i64 * net.output_weights[HIDDEN + i] as i64;
    }
    output /= QA as i64;
    output += net.output_bias as i64;
    output *= SCALE as i64;
    output /= (QA * QB) as i64;
    output as Score
}

#[cfg(test)]
#[inline(always)]
fn screlu_i32(x: i32) -> i32 {
    let y = x.clamp(0, QA);
    y * y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::init_attack_tables;

    const PARITY_FENS: &[&str] = &[
        crate::position::START_FEN,
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "4k3/1P6/8/8/8/8/r5K1/8 w - - 0 1",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        "4rrk1/pp1n3p/3q2pQ/2p1pb2/2PP4/2P3N1/P2B2PP/4RRK1 b - - 7 19",
    ];

    /// The production folded-into-i16 eval must equal the i32-scratch recompute
    /// reference for every position — i.e. the i16 accumulator never overflows
    /// once threats are folded in.
    #[test]
    fn folded_eval_matches_recompute_reference() {
        init_attack_tables();
        for fen in PARITY_FENS {
            let pos = Position::from_fen(fen).unwrap();
            let folded = evaluate(&pos);
            let reference = evaluate_recompute_ref(&pos);
            assert_eq!(folded, reference, "eval parity mismatch at {fen}: folded={folded} ref={reference}");
        }
    }

    // Hazard FENs exercising every silent-corruption trap the spec lists:
    // captures, en passant (with rook on the EP rank for same-rank discovery),
    // both castles (back-rank discovered rays), quiet + capture promotions,
    // double-push creating an ep target, sliders crossing both from and to.
    const HAZARD_FENS: &[&str] = &[
        crate::position::START_FEN,
        // Kiwipete: captures, castling both sides, many sliders
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        // en passant available (f6), rook on the rank for discovery after ep
        "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
        // ep with a rook on the 5th rank: a1 rook gains a discovered threat
        "8/8/8/K1pP3r/8/8/8/7k w - c6 0 1",
        // castle-rich, open back ranks
        "r3k2r/pppq1ppp/2nbbn2/3pp3/3PP3/2NBBN2/PPPQ1PPP/R3K2R w KQkq - 0 1",
        // promotions: quiet and capture, both colors, with knights to capture
        "n1n5/PPPk4/8/8/8/8/4Kppp/5N1N b - - 0 1",
        // promotion with a rook behind for discovered ray
        "4k3/1P6/8/8/8/8/r5K1/8 w - - 0 1",
        // dense middlegame: sliders crossing both from and to on quiets/captures
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        // double-push creating an ep target next to an enemy pawn
        "rnbqkbnr/pp1ppppp/8/8/2p5/5N2/PPPPPPPP/RNBQKB1R w KQkq - 0 2",
    ];

    /// Recursively walk the legal-move tree to `depth`, asserting that for every
    /// move the incrementally-applied accumulator (piece + threat) is BIT-
    /// IDENTICAL to a from-scratch refresh of the child. This is the spec's
    /// correctness gate as a permanent test: it covers every move type reachable
    /// from the hazard FENs and runs in debug where divergence would panic.
    fn walk(pos: &Position, depth: u32) {
        let parent = Accumulator::refresh(pos);
        let mut list = crate::movegen::MoveList::new();
        crate::movegen::generate_moves(pos, &mut list);
        for mv in list.iter() {
            let child = pos.make(mv);
            let inc = apply(&parent, pos, &child, mv);
            let fresh = Accumulator::refresh(&child);
            assert!(
                inc.w == fresh.w && inc.b == fresh.b,
                "incremental != refresh after {mv} from {} (flags {})",
                pos.to_fen(),
                mv.flags()
            );
            if depth > 1 {
                walk(&child, depth - 1);
            }
        }
    }

    #[test]
    fn incremental_matches_refresh_all_move_types() {
        init_attack_tables();
        // depth 2 from the diverse hazard set hits every move type many times and
        // keeps the test fast; the in-search per-node gate covers deeper trees.
        for fen in HAZARD_FENS {
            let pos = Position::from_fen(fen).unwrap();
            walk(&pos, 2);
        }
    }
}
