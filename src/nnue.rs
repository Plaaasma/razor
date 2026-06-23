//! NNUE evaluation — THREAT net v1 with INCREMENTAL threat features.
//!
//! Network `((768 piece + 12800 attacker-quadrant threats) -> HIDDEN)x2 -> 1`,
//! SCReLU, trained with bullet. HIDDEN = 768. SPLIT-QUANT feature weights: the
//! 768 PIECE rows are kept i16 (full eval precision), the 12800 THREAT rows are
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
//! Threat feature (attacker-quadrant, NO full attacker square):
//!   att_pt, vic_pt in {P,N,B,R,Q} (king excluded as attacker AND victim),
//!   enemy = (att_color != vic_color), vic_sq in the perspective's orientation,
//!   att_quad in 0..4 = the 2x2 board quadrant of the attacker square:
//!     att_quad = ((sq >> 5) << 1) | ((sq & 7) >= 4)
//!     (bit1 = rank-half sq>=32, bit0 = file-half file>=4)
//!   local = (((att_pt*4 + att_quad)*5 + vic_pt)*2 + enemy)*64 + vic_sq (0..12800)
//!   index = 768 + local
//! White perspective uses the RAW attacker square for att_quad and vic_sq as-is;
//! black uses (att_sq^56) for att_quad and (vic_sq^56) for the victim square (the
//! attacker square is oriented with the SAME ^56 as the victim). enemy is
//! perspective-invariant. This index is bit-identical to the proven recompute
//! reference (matches/candidates/nnue-thr.rs) and bullet ThreatInputsQuad.
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

// KING-BUCKETED pieces (HalfKA-hm) + quad threats. The piece block is now
// 768*16 king-bucketed, file-mirrored rows; the threat block (king-independent)
// follows at THREAT_OFF.
const NUM_KING_BUCKETS: usize = 16;
const NUM_PIECE: usize = 768 * NUM_KING_BUCKETS; // 12288 piece rows (i16)
const NUM_THREAT: usize = 6400; // ENEMY-ONLY attacker-quadrant threat rows (i8); was 12800 with the friendly-defense half
const THREAT_OFF: usize = NUM_PIECE; // 12288 (threat rows begin after all bucketed piece rows)
const NUM_FEAT: usize = NUM_PIECE + NUM_THREAT; // 25088
const NUM_OUTPUT_BUCKETS: usize = 8; // MaterialCount<8>
const _: () = assert!(NUM_FEAT == NUM_PIECE + NUM_THREAT);
const QA: i32 = 255;
const QB: i32 = 64;
const SCALE: i32 = 400;

// 16-entry mirrored king-bucket layout EXPANDED to 64 squares, bit-identical to
// bullet's ChessBucketsMirrored::new(layout) where layout[r*4+mf] = (r/2)*4+mf:
//   KING_BUCKETS[sq] = ((sq/8)/2)*4 + FOLD[sq%8]
// FOLD folds the 8 files to 4 mirrored half-files (file mirror is applied
// separately as the ^7 flip on the feature index, exactly like the trainer).
const FOLD: [usize; 8] = [0, 1, 2, 3, 3, 2, 1, 0];
const KING_BUCKETS: [usize; 64] = {
    let mut t = [0usize; 64];
    let mut sq = 0;
    while sq < 64 {
        t[sq] = ((sq / 8) / 2) * 4 + FOLD[sq % 8];
        sq += 1;
    }
    t
};

/// Per-perspective king context: `(base = 768*bucket, flip = 0 or 7)`. `flip`
/// (7) mirrors only the file of the piece square (bits 0-2); the col/pt blocks
/// are untouched since 7 < 64. Bit-identical to ChessBucketsMirrored's
/// `get(ksq) = ((ksq%8>3)?7:0, 768*buckets[ksq])`.
///
/// PERSPECTIVE ORIENTATION (load-bearing): a perspective keys its king context by
/// the king square IN THAT PERSPECTIVE'S ORIENTATION. The White-POV `w`
/// accumulator keys pieces by the raw square, so it passes the raw White king.
/// The Black-POV `b` accumulator keys pieces by `sq^56`, so it must pass the
/// Black king `^56` — exactly bullet's `opp_ksq = non_stm_king ^ 56`. (`^56`
/// flips only the rank, so `flip` is unchanged, but the bucket — which depends on
/// rank — is not, hence the king square MUST be oriented before this call.)
#[inline(always)]
fn king_ctx(king_sq: Square) -> (usize, usize) {
    let ks = king_sq as usize;
    (768 * KING_BUCKETS[ks], if (ks & 7) > 3 { 7 } else { 0 })
}

/// The White-POV (w) and Black-POV (b) king contexts for a position. The black
/// king is oriented with `^56` to match the black perspective's `sq^56` indexing.
#[inline(always)]
fn king_ctx_both(pos: &Position) -> (usize, usize, usize, usize) {
    let (wk_base, wk_flip) = king_ctx(pos.king_sq(Color::White));
    let (bk_base, bk_flip) = king_ctx(pos.king_sq(Color::Black) ^ 56);
    (wk_base, wk_flip, bk_base, bk_flip)
}

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
    // Per-output-bucket l1 weights, bucket-major [bucket][2*HIDDEN] (matches the
    // trainer's l1w.transpose()), and per-bucket l1 bias. The bucket is selected
    // at eval() by MaterialCount<8>.
    output_weights: Box<[[i16; 2 * HIDDEN]; NUM_OUTPUT_BUCKETS]>,
    output_bias: [i16; NUM_OUTPUT_BUCKETS],
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
    // Guard against a layout regression: the net is NUM_FEAT feature rows of
    // HIDDEN i16, then HIDDEN i16 feature bias, then NUM_OUTPUT_BUCKETS rows of
    // 2*HIDDEN i16 output weights, then NUM_OUTPUT_BUCKETS i16 output bias —
    // plus up to 63 bytes of bullet's to_quantised_buffer padding.
    const EXPECT: usize = NUM_FEAT * HIDDEN * 2 + HIDDEN * 2 + NUM_OUTPUT_BUCKETS * 2 * HIDDEN * 2 + NUM_OUTPUT_BUCKETS * 2;
    assert!(
        bytes.len() >= EXPECT && bytes.len() < EXPECT + 64,
        "net byte length {} not in [{EXPECT}, {}+64) — wrong net for this layout",
        bytes.len(),
        EXPECT
    );
    let mut at = 0usize;
    // The net stores all NUM_FEAT feature rows as i16, in row order: the 768*16
    // king-bucketed piece rows 0..THREAT_OFF first, then the 12800 threat rows.
    // SPLIT-QUANT routing: piece rows are stored i16 as-is (full precision);
    // threat rows are clamped to [-127, 127] and stored i8 (half bandwidth). The
    // clamp is the ONLY lossy step. THREAT_OFF is now NUM_PIECE = 12288.
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
    // Output weights are saved bucket-major [bucket][2*HIDDEN] (the trainer's
    // l1w.transpose()): read all 2*HIDDEN weights of bucket 0, then bucket 1, ...
    let mut output_weights: Box<[[i16; 2 * HIDDEN]; NUM_OUTPUT_BUCKETS]> =
        vec![[0i16; 2 * HIDDEN]; NUM_OUTPUT_BUCKETS].into_boxed_slice().try_into().unwrap();
    for bkt in output_weights.iter_mut() {
        for w in bkt.iter_mut() {
            *w = rd_i16(bytes, &mut at);
        }
    }
    // The screlu_dot i16-domain kernel computes c*w via _mm256_mullo_epi16, which
    // returns only the low 16 bits — exact ONLY while |c*w| < 32768. c = clamp to
    // [0,255], so this requires every output weight |w| <= 127 (QA-quantized nets
    // satisfy this). A net that violates it would silently corrupt eval with no
    // test failure, so pin it at load.
    assert!(
        output_weights.iter().flatten().all(|&w| (-127..=127).contains(&w)),
        "output weight exceeds i16-domain screlu_dot bound |w|<=127; net incompatible with the fast eval kernel"
    );
    let mut output_bias = [0i16; NUM_OUTPUT_BUCKETS];
    for ob in output_bias.iter_mut() {
        *ob = rd_i16(bytes, &mut at);
    }
    Network { piece_weights, threat_weights, feature_bias, output_weights, output_bias }
}

use std::sync::OnceLock;
static NET: OnceLock<Network> = OnceLock::new();

pub fn net() -> &'static Network {
    NET.get_or_init(load)
}

/// White-perspective piece feature row, keyed by the WHITE king context. The
/// plain Chess768 white-POV index `(col==White?0:384) + 64*pt + sq` is XORed by
/// `wk_flip` (mirrors only the sq file) then offset by `wk_base = 768*bucket`.
/// Algebraically identical to bullet's `stm_bucket + (stm ^ stm_flip)` for the
/// White perspective.
#[inline(always)]
fn wp_feat(col: Color, pt: usize, sq: Square, wk_base: usize, wk_flip: usize) -> usize {
    let idx768 = (if col == Color::White { 0 } else { 384 }) + 64 * pt + sq as usize;
    wk_base + (idx768 ^ wk_flip)
}

/// Black-perspective piece feature row, keyed by the BLACK king context (the
/// black-POV index already applies `sq^56`; `bk_flip` then mirrors the file).
#[inline(always)]
fn bp_feat(col: Color, pt: usize, sq: Square, bk_base: usize, bk_flip: usize) -> usize {
    let idx768 = (if col == Color::Black { 0 } else { 384 }) + 64 * pt + (sq ^ 56) as usize;
    bk_base + (idx768 ^ bk_flip)
}

/// Threat feature base offset (everything but the victim square), keyed by
/// `idx = (att_pt*4 + att_quad)*5 + vic_pt` for att_pt,vic_pt in 0..5, att_quad
/// in 0..4 (100 entries). ENEMY-ONLY: there is no enemy dimension (every kept
/// threat is enemy==1), so the layout drops the `*2 + enemy` factor. value =
/// `idx*64` (max 99*64 = 6336, fits u16). Hoists the per-toggle multiply chain
/// out of the innermost producer loop into a single table read; the final
/// feature index is just `THREAT_OFF + THR_BASE[idx] + vic_sq` (mirrored for
/// black). Pure arithmetic identity with the inline computation.
const THR_BASE: [u16; 100] = {
    let mut t = [0u16; 100];
    let mut i = 0;
    while i < 100 {
        t[i] = (i as u16) * 64; // i == (att_pt*4+att_quad)*5+vic_pt
        i += 1;
    }
    t
};

/// The 2x2 board quadrant of a square in the perspective's orientation, 0..4.
/// bit1 = rank-half (sq>=32), bit0 = file-half (file>=4). The caller orients
/// `sq` first (raw for white, ^56 for black), exactly as it orients the victim
/// square.
#[inline(always)]
fn att_quad(sq: Square) -> usize {
    (((sq >> 5) << 1) | (((sq & 7) >= 4) as u8)) as usize
}

/// White-perspective THREAT-LOCAL feature index in 0..12800 (the row index into
/// `threat_weights`; the global feature index would be `THREAT_OFF + this`).
/// `att_sq`/`vic_sq` are the raw attacker/victim squares; att_quad uses the raw
/// attacker square, vic_sq is used as-is.
#[inline(always)]
fn thr_feat_w(att_pt: usize, vic_pt: usize, _enemy: usize, att_sq: Square, vic_sq: Square) -> usize {
    THR_BASE[(att_pt * 4 + att_quad(att_sq)) * 5 + vic_pt] as usize + vic_sq as usize
}

/// Black-perspective THREAT-LOCAL feature index in 0..12800 (attacker square and
/// victim square both mirrored with ^56).
#[inline(always)]
fn thr_feat_b(att_pt: usize, vic_pt: usize, _enemy: usize, att_sq: Square, vic_sq: Square) -> usize {
    THR_BASE[(att_pt * 4 + att_quad(att_sq ^ 56)) * 5 + vic_pt] as usize
        + (vic_sq ^ 56) as usize
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
/// att_sq, vic_sq)` once per threat. King is excluded as attacker AND victim.
/// The victim mask is ALL occupied squares (both colors); `enemy` records
/// whether attacker and victim differ in color. `att_sq` is the attacker's
/// square (the `from` of the scan), needed for the attacker quadrant. This is
/// the from-scratch oracle and is bit-identical to the recompute reference's
/// for_each_threat.
#[inline]
fn for_each_threat<F: FnMut(usize, usize, usize, Square, Square)>(pos: &Position, mut f: F) {
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
                        if enemy == 0 {
                            continue; // ENEMY-ONLY: friendly-defense threats dropped
                        }
                        f(att_pt, vp, enemy, from, vsq);
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
    // Cached per-perspective king context (base = 768*bucket, flip = 0|7). `wk_*`
    // drives the w (White-POV) piece rows, `bk_*` the b (Black-POV) piece rows.
    // Recomputed only on refresh / on a boundary-crossing king move; the
    // incremental add/remove read these so they never recompute the king ctx.
    wk_base: usize,
    wk_flip: usize,
    bk_base: usize,
    bk_flip: usize,
}

impl Accumulator {
    /// All-zero accumulator, for pre-sizing the search stack.
    pub fn zeroed() -> Accumulator {
        Accumulator { w: [0; HIDDEN], b: [0; HIDDEN], wk_base: 0, wk_flip: 0, bk_base: 0, bk_flip: 0 }
    }

    /// Full rebuild from a position: piece features then threat features. This
    /// is the from-scratch oracle the per-node correctness gate compares
    /// against, and is used for root init, `evaluate()`, and a boundary-crossing
    /// king refresh. Sets the king ctx FIRST (both perspectives, from the two
    /// kings) so the piece add() calls key the correct bucket+mirror rows.
    pub fn refresh(pos: &Position) -> Accumulator {
        let net = net();
        let (wk_base, wk_flip, bk_base, bk_flip) = king_ctx_both(pos);
        let mut acc = Accumulator { w: net.feature_bias, b: net.feature_bias, wk_base, wk_flip, bk_base, bk_flip };
        for pt in PieceType::ALL {
            for col in [Color::White, Color::Black] {
                for sq in BitIter(pos.pieces(col, pt)) {
                    acc.add(col, pt.idx(), sq);
                }
            }
        }
        for_each_threat(pos, |att_pt, vic_pt, enemy, asq, vsq| {
            acc.thr_add(att_pt, vic_pt, enemy, asq, vsq);
        });
        acc
    }

    // Scalar add/remove/eval: the compiler auto-vectorizes these i16 loops
    // under target-cpu=native (verified hand-written AVX2 gave no speedup —
    // 2026-06-13). Keeps the aarch64/Spark build simple too.

    #[inline(always)]
    fn add(&mut self, col: Color, pt: usize, sq: Square) {
        let net = net();
        add_row_i16(&mut self.w, &net.piece_weights[wp_feat(col, pt, sq, self.wk_base, self.wk_flip)]);
        add_row_i16(&mut self.b, &net.piece_weights[bp_feat(col, pt, sq, self.bk_base, self.bk_flip)]);
    }

    #[inline(always)]
    fn remove(&mut self, col: Color, pt: usize, sq: Square) {
        let net = net();
        sub_row_i16(&mut self.w, &net.piece_weights[wp_feat(col, pt, sq, self.wk_base, self.wk_flip)]);
        sub_row_i16(&mut self.b, &net.piece_weights[bp_feat(col, pt, sq, self.bk_base, self.bk_flip)]);
    }

    /// Add a single threat feature row into w/b (used by the from-scratch
    /// refresh oracle; the incremental path batches instead — see ThreatBatch).
    /// Threat rows are i8 (split-quant) and widened to i16 inside the kernel.
    /// `asq` is the attacker's square (for the attacker quadrant).
    #[inline(always)]
    fn thr_add(&mut self, att_pt: usize, vic_pt: usize, enemy: usize, asq: Square, vsq: Square) {
        let net = net();
        add_row_i8(&mut self.w, &net.threat_weights[thr_feat_w(att_pt, vic_pt, enemy, asq, vsq)]);
        add_row_i8(&mut self.b, &net.threat_weights[thr_feat_b(att_pt, vic_pt, enemy, asq, vsq)]);
    }

    /// Re-key the MOVING side's perspective piece rows from the old king context
    /// to `(new_base, new_flip)`, in place. Called after a boundary-crossing king
    /// move: `apply_lazy` has already produced correct threats (both perspectives,
    /// king-independent) and a correct NON-moving perspective; only the moving
    /// perspective's piece rows are still keyed by the parent (old) king ctx. For
    /// every piece on the CHILD board, subtract its old-ctx row and add its
    /// new-ctx row to that one perspective vector, then store the new ctx. This is
    /// bit-identical to `refresh(child)` but keeps the 82%-of-refresh threat
    /// enumeration AND the unaffected non-moving perspective incremental, so a
    /// king crossing costs ~one perspective's piece re-key instead of a full
    /// rebuild. The old ctx is read from `self` (apply_lazy left it = parent's).
    #[inline]
    fn reindex_moving(&mut self, child: &Position, mover: Color, new_base: usize, new_flip: usize) {
        let net = net();
        // Collect every piece's old-ctx row (sign -1) and new-ctx row (sign +1)
        // for the moving perspective, then apply them in ONE register-tiled pass
        // instead of up to 64 full-width sub/add streams. Max 32 pieces * 2 = 64.
        let mut idx = [0u16; 64];
        let mut sign = [0i16; 64];
        let mut n = 0usize;
        if mover == Color::White {
            let (ob, of) = (self.wk_base, self.wk_flip);
            for pt in PieceType::ALL {
                for col in [Color::White, Color::Black] {
                    for sq in BitIter(child.pieces(col, pt)) {
                        let p = pt.idx();
                        idx[n] = wp_feat(col, p, sq, ob, of) as u16;
                        sign[n] = -1;
                        idx[n + 1] = wp_feat(col, p, sq, new_base, new_flip) as u16;
                        sign[n + 1] = 1;
                        n += 2;
                    }
                }
            }
            apply_piece_columns(&mut self.w, &net.piece_weights, &idx, &sign, n);
            self.wk_base = new_base;
            self.wk_flip = new_flip;
        } else {
            let (ob, of) = (self.bk_base, self.bk_flip);
            for pt in PieceType::ALL {
                for col in [Color::White, Color::Black] {
                    for sq in BitIter(child.pieces(col, pt)) {
                        let p = pt.idx();
                        idx[n] = bp_feat(col, p, sq, ob, of) as u16;
                        sign[n] = -1;
                        idx[n + 1] = bp_feat(col, p, sq, new_base, new_flip) as u16;
                        sign[n + 1] = 1;
                        n += 2;
                    }
                }
            }
            apply_piece_columns(&mut self.b, &net.piece_weights, &idx, &sign, n);
            self.bk_base = new_base;
            self.bk_flip = new_flip;
        }
    }

    /// Evaluate from the side-to-move's perspective, selecting the output bucket
    /// by material count (MaterialCount<8>: bucket = (occ_count - 2) / 4, clamped
    /// to 0..7). Threats are already folded into w/b, so this is a plain SCReLU
    /// dot over the selected bucket's output weights.
    pub fn eval(&self, pos: &Position) -> Score {
        let net = net();
        let (us, them) = if pos.stm == Color::White { (&self.w, &self.b) } else { (&self.b, &self.w) };
        // MaterialCount<8>: divisor = ceil(32/8) = 4 (matches bullet outputs.rs).
        let n = pos.occupied().count_ones() as usize;
        let bkt = ((n - 2) / 4).min(NUM_OUTPUT_BUCKETS - 1);
        let ow = &net.output_weights[bkt];
        // i32 accumulation. At HIDDEN<=768 the sum stays in range and i32 is
        // ~8% faster than i64 (the i64 widening defeats i16 SIMD auto-vec).
        const _: () = assert!(HIDDEN <= 768, "i32 eval accumulation may overflow above 768; restore i64 in eval()");
        let mut output: i32 = screlu_dot(us, &ow[..HIDDEN]) + screlu_dot(them, &ow[HIDDEN..]);
        output /= QA;
        output += net.output_bias[bkt] as i32;
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

/// Dot of one perspective's SCReLU activations against its output-weight half:
/// `sum_i screlu(acc[i]) * w[i]`, returned in i32. BIT-IDENTICAL to the scalar
/// `for i { out += screlu(acc[i]) * w[i] as i32 }` it replaces.
///
/// i16-DOMAIN KERNEL (16 lanes/op vs the auto-vectorized i32 path's 8). Let
/// `c = clamp(acc[i], 0, 255)` (0..255, fits i16). Then `screlu(x)*w = c*c*w`
/// is computed as `c * (c*w)`:
///   - `p = c*w` fits i16 EXACTLY: |c*w| <= 255*127 = 32385 < 32768, so
///     `_mm256_mullo_epi16(c, w)` returns the true product, never wrapping
///     (gated on every output weight satisfying |w| <= 127 — true for this net,
///     QA-quantized; an assert below pins it).
///   - `_mm256_madd_epi16(c, p)` then forms, per adjacent i16 pair,
///     `c0*p0 + c1*p1` in an i32 lane = the pairwise sum of `c*c*w` terms.
///     Each `c*c*w` <= 65025*127 = 8_258_175; the pair sum <= 16_516_350, far
///     inside i32, and madd's i32 result does NOT saturate here (would require
///     |c|=|p|=32768). The lane sums accumulate in i32 and horizontal-add at
///     the end. Integer add is associative/commutative, so the regrouped sum
///     equals the scalar left-to-right total exactly. HIDDEN is a multiple of 16
///     (asserted at module top), so there is no remainder loop.
#[inline]
fn screlu_dot(acc: &[i16], w: &[i16]) -> i32 {
    debug_assert_eq!(acc.len(), HIDDEN);
    debug_assert_eq!(w.len(), HIDDEN);
    #[cfg(target_feature = "avx2")]
    unsafe {
        use std::arch::x86_64::*;
        // SAFETY: acc and w are both exactly HIDDEN i16 (debug_asserted) and
        // HIDDEN % 16 == 0 (module assert), so every 16-lane load is in bounds;
        // AVX2 is guaranteed by the cfg gate.
        let lo = _mm256_setzero_si256(); // i16 clamp floor (0)
        let hi = _mm256_set1_epi16(QA as i16); // i16 clamp ceil (255)
        let mut sum = _mm256_setzero_si256(); // 8 i32 partial sums
        let mut i = 0;
        while i < HIDDEN {
            let x = _mm256_loadu_si256(acc.as_ptr().add(i) as *const __m256i);
            // c = clamp(x, 0, 255) in i16
            let c = _mm256_min_epi16(_mm256_max_epi16(x, lo), hi);
            let wv = _mm256_loadu_si256(w.as_ptr().add(i) as *const __m256i);
            // p = c*w, exact in i16 (|c*w| <= 32385); then madd(c, p) = c*c*w
            // pairwise-summed into i32.
            let p = _mm256_mullo_epi16(c, wv);
            sum = _mm256_add_epi32(sum, _mm256_madd_epi16(c, p));
            i += 16;
        }
        // horizontal-add the 8 i32 lanes
        let hi128 = _mm256_extracti128_si256(sum, 1);
        let lo128 = _mm256_castsi256_si128(sum);
        let s128 = _mm_add_epi32(lo128, hi128);
        let s64 = _mm_add_epi32(s128, _mm_shuffle_epi32(s128, 0b01_00_11_10));
        let s32 = _mm_add_epi32(s64, _mm_shuffle_epi32(s64, 0b00_00_00_01));
        _mm_cvtsi128_si32(s32)
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        let mut output: i32 = 0;
        for i in 0..HIDDEN {
            output += screlu(acc[i]) * w[i] as i32;
        }
        output
    }
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
    /// `att_sq` is the attacker's square (for the attacker quadrant): the white
    /// quadrant comes from the raw `att_sq`, the black quadrant from `att_sq^56`
    /// — the same orientation thr_feat_w/b apply to the victim square.
    #[inline(always)]
    fn toggle(&mut self, att_pt: usize, vic_pt: usize, enemy: usize, att_sq: Square, vsq: Square, add: bool) {
        if att_pt == 5 || vic_pt == 5 {
            return; // king never an attacker or victim
        }
        if enemy == 0 {
            return; // ENEMY-ONLY: friendly-defense threats are not features
        }
        let wf = thr_feat_w(att_pt, vic_pt, enemy, att_sq, vsq) as u16;
        let bf = thr_feat_b(att_pt, vic_pt, enemy, att_sq, vsq) as u16;
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

    /// Apply the netted residual to the accumulator with REGISTER-TILED fused
    /// add/sub. Each perspective accumulator is processed one TILE_H-lane tile at
    /// a time: the tile is loaded into TILE_REGS ymm registers once, every
    /// surviving threat column for that tile is folded in (i8 -> i16 widened in
    /// register, never materialized; +1 adds, -1 subtracts, the rare |s|>1
    /// multiply-adds), and the tile is stored once. Accumulator load/store
    /// traffic is therefore NUM_TILES passes regardless of survivor count, vs the
    /// old one full-width load+store per survivor. Bit-identical to the per-row
    /// kernels: the same signed i16 column sums, only reordered/regrouped (integer
    /// add is associative and commutative; the i16 accumulator never overflows,
    /// gated by the per-node assert). Indices in w_idx/b_idx are THREAT-LOCAL
    /// (rows into `threat_weights`, 0..12800).
    #[inline]
    fn apply_to(&self, acc: &mut Accumulator) {
        let net = net();
        // Software-prefetch every survivor threat row up front. The i8 threat
        // weight table is ~9.4 MB and survivor indices (~5-6/move) are scattered
        // across it, so each row is a likely L2/L3 miss; issuing the loads
        // before the dependent AVX2 widen+add/sub hides that latency behind
        // useful work. 768 B/row spans 12 cache lines, so we touch the row head
        // (the rest streams in once the kernel starts).
        #[cfg(target_feature = "avx2")]
        unsafe {
            #[cfg(not(feature = "noprefetch"))]
            {
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
            tiled_apply_avx2(&mut acc.w, &net.threat_weights, &self.w_idx, &self.sign, self.len);
            tiled_apply_avx2(&mut acc.b, &net.threat_weights, &self.b_idx, &self.sign, self.len);
        }
        #[cfg(not(target_feature = "avx2"))]
        {
            tiled_apply_scalar(&mut acc.w, &net.threat_weights, &self.w_idx, &self.sign, self.len);
            tiled_apply_scalar(&mut acc.b, &net.threat_weights, &self.b_idx, &self.sign, self.len);
        }
    }
}

// Register-tiling geometry for the threat FT-apply. vec_t = __m256i = 16 i16
// lanes; TILE_REGS ymm registers hold one TILE_H-lane tile of a perspective
// accumulator, and NUM_TILES tiles cover HIDDEN. 8 tile regs + the i8 column load
// + cvtepi8 widen (+ the rare set1 multiplier) use ~10 of the 16 architectural
// ymm — comfortable headroom, no spill (confirmed empirically: applybench
// 451->311 ns/apply, midgame nps +8%; 8 measured slightly faster than 12 on the
// apply microbench and tied in search). If HIDDEN changes, keep TILE_REGS a
// divisor of HIDDEN/16.
const TILE_REGS: usize = 8;
const TILE_H: usize = TILE_REGS * 16; // 128 i16 lanes per tile
const NUM_TILES: usize = HIDDEN / TILE_H; // 6
const _: () = assert!(HIDDEN % TILE_H == 0, "HIDDEN must be a multiple of TILE_H for the tiled apply");

/// Register-tiled fused apply of the netted threat survivors into ONE perspective
/// accumulator. `idx`/`sign` are the batch's w_idx-or-b_idx and signs; only the
/// first `len` entries are live and sign==0 entries are skipped. Each TILE_H-lane
/// tile of `acc` is loaded into TILE_REGS ymm once, every survivor's i8 column
/// (widened to i16 in register, never materialized) is added/subtracted/scaled
/// into the registers, and the tile is stored once. Bit-identical to repeated
/// add_row_i8/sub_row_i8/madd_row_i8.
#[cfg(target_feature = "avx2")]
#[inline]
fn tiled_apply_avx2(
    acc: &mut [i16; HIDDEN],
    weights: &[[i8; HIDDEN]; NUM_THREAT],
    idx: &[u16; MAX_DELTAS],
    sign: &[i16; MAX_DELTAS],
    len: usize,
) {
    use std::arch::x86_64::*;
    // SAFETY: idx[i] < NUM_THREAT (threat-local indices) so each row pointer stays
    // in `weights`, and off + r*16 < HIDDEN so every acc/column load+store is in
    // bounds; AVX2 is guaranteed by the cfg gate.
    unsafe {
    let wbase = weights.as_ptr() as *const i8; // threat row r begins at wbase + r*HIDDEN
    let mut t = 0;
    while t < NUM_TILES {
        let off = t * TILE_H;
        // load this tile of the accumulator into registers once
        let mut reg = [_mm256_setzero_si256(); TILE_REGS];
        let mut r = 0;
        while r < TILE_REGS {
            reg[r] = _mm256_loadu_si256(acc.as_ptr().add(off + r * 16) as *const __m256i);
            r += 1;
        }
        // fold every survivor's column for this tile into the registers
        let mut i = 0;
        while i < len {
            let s = sign[i];
            if s != 0 {
                let col = wbase.add(idx[i] as usize * HIDDEN + off);
                if s == 1 {
                    let mut r = 0;
                    while r < TILE_REGS {
                        let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                        reg[r] = _mm256_add_epi16(reg[r], w);
                        r += 1;
                    }
                } else if s == -1 {
                    let mut r = 0;
                    while r < TILE_REGS {
                        let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                        reg[r] = _mm256_sub_epi16(reg[r], w);
                        r += 1;
                    }
                } else {
                    // rare |s| > 1: widen then multiply-add (low 16 bits of s*w
                    // == the wrapped repeated sum, identical to madd_row_i8).
                    let mul = _mm256_set1_epi16(s);
                    let mut r = 0;
                    while r < TILE_REGS {
                        let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                        reg[r] = _mm256_add_epi16(reg[r], _mm256_mullo_epi16(w, mul));
                        r += 1;
                    }
                }
            }
            i += 1;
        }
        // store this tile once
        let mut r = 0;
        while r < TILE_REGS {
            _mm256_storeu_si256(acc.as_mut_ptr().add(off + r * 16) as *mut __m256i, reg[r]);
            r += 1;
        }
        t += 1;
    }
    } // unsafe
}

/// FUSED register-tiled apply of BOTH the netted piece survivors (i16 columns)
/// and the netted threat survivors (i8 columns widened in register) into ONE
/// perspective accumulator, in a SINGLE tile-once pass. The default path does two
/// separate tiled passes (`apply_piece_columns` then `tiled_apply_avx2`), each of
/// which loads all NUM_TILES tiles of `acc` and stores them — i.e. two full
/// load+store streams over the 768-i16 (1.5 KB) perspective accumulator per move.
/// Folding both batches into one pass loads+stores each tile exactly ONCE,
/// halving accumulator load/store traffic. Bit-identical to running the two
/// kernels back-to-back: integer add is associative/commutative, the per-tile
/// register sum is the same signed i16 column sum, and the i16 accumulator never
/// overflows (per-node assert gates it). `p_*` index `piece_weights` (0..NUM_PIECE),
/// `t_*` index `threat_weights` (0..NUM_THREAT).
#[cfg(target_feature = "avx2")]
#[inline]
fn fused_apply_avx2(
    acc: &mut [i16; HIDDEN],
    piece_w: &[[i16; HIDDEN]; NUM_PIECE],
    p_idx: &[u16],
    p_sign: &[i16],
    p_len: usize,
    threat_w: &[[i8; HIDDEN]; NUM_THREAT],
    t_idx: &[u16; MAX_DELTAS],
    t_sign: &[i16; MAX_DELTAS],
    t_len: usize,
) {
    use std::arch::x86_64::*;
    // SAFETY: p_idx[i] < NUM_PIECE and t_idx[i] < NUM_THREAT keep every row pointer
    // in its table, off + r*16 < HIDDEN keeps every acc/column access in bounds, and
    // AVX2 is guaranteed by the cfg gate.
    unsafe {
        let pbase = piece_w.as_ptr() as *const i16; // piece row r begins at pbase + r*HIDDEN
        let tbase = threat_w.as_ptr() as *const i8; // threat row r begins at tbase + r*HIDDEN
        let mut t = 0;
        while t < NUM_TILES {
            let off = t * TILE_H;
            // load this tile of the accumulator into registers ONCE
            let mut reg = [_mm256_setzero_si256(); TILE_REGS];
            let mut r = 0;
            while r < TILE_REGS {
                reg[r] = _mm256_loadu_si256(acc.as_ptr().add(off + r * 16) as *const __m256i);
                r += 1;
            }
            // fold every PIECE survivor's i16 column for this tile
            let mut i = 0;
            while i < p_len {
                let s = p_sign[i];
                if s != 0 {
                    let col = pbase.add(p_idx[i] as usize * HIDDEN + off);
                    if s == 1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_add_epi16(reg[r], w);
                            r += 1;
                        }
                    } else if s == -1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_sub_epi16(reg[r], w);
                            r += 1;
                        }
                    } else {
                        let mul = _mm256_set1_epi16(s);
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_add_epi16(reg[r], _mm256_mullo_epi16(w, mul));
                            r += 1;
                        }
                    }
                }
                i += 1;
            }
            // fold every THREAT survivor's i8 column for this tile (widened in reg)
            let mut i = 0;
            while i < t_len {
                let s = t_sign[i];
                if s != 0 {
                    let col = tbase.add(t_idx[i] as usize * HIDDEN + off);
                    if s == 1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                            reg[r] = _mm256_add_epi16(reg[r], w);
                            r += 1;
                        }
                    } else if s == -1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                            reg[r] = _mm256_sub_epi16(reg[r], w);
                            r += 1;
                        }
                    } else {
                        let mul = _mm256_set1_epi16(s);
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                            reg[r] = _mm256_add_epi16(reg[r], _mm256_mullo_epi16(w, mul));
                            r += 1;
                        }
                    }
                }
                i += 1;
            }
            // store this tile ONCE
            let mut r = 0;
            while r < TILE_REGS {
                _mm256_storeu_si256(acc.as_mut_ptr().add(off + r * 16) as *mut __m256i, reg[r]);
                r += 1;
            }
            t += 1;
        }
    }
}

/// COPY-FUSED apply: like `fused_apply_avx2` but reads the PARENT perspective
/// vector `src` and writes the CHILD perspective vector `dst`, folding both piece
/// and threat survivor columns in the same tiled pass. This is SF's `apply`
/// (parent fromTile -> child toTile): the per-tile load of `src` IS the copy, so
/// the separate `parent.clone()` 3KB memcpy in `apply_lazy` is eliminated — the
/// 6 tile loads we already perform double as the copy of the unchanged lanes.
/// Bit-identical to `clone()` THEN `fused_apply_avx2` on the same buffer: the
/// per-tile register starts at the parent value either way, and the same signed
/// column sums fold in. `src` and `dst` MUST NOT alias (distinct stack slots).
#[cfg(target_feature = "avx2")]
#[inline]
fn copy_fused_apply_avx2(
    dst: &mut [i16; HIDDEN],
    src: &[i16; HIDDEN],
    piece_w: &[[i16; HIDDEN]; NUM_PIECE],
    p_idx: &[u16],
    p_sign: &[i16],
    p_len: usize,
    threat_w: &[[i8; HIDDEN]; NUM_THREAT],
    t_idx: &[u16; MAX_DELTAS],
    t_sign: &[i16; MAX_DELTAS],
    t_len: usize,
) {
    use std::arch::x86_64::*;
    // SAFETY: identical bounds to fused_apply_avx2; src and dst are both [i16; HIDDEN]
    // so every off + r*16 < HIDDEN load/store is in bounds; AVX2 guaranteed by cfg.
    unsafe {
        let pbase = piece_w.as_ptr() as *const i16;
        let tbase = threat_w.as_ptr() as *const i8;
        let mut t = 0;
        while t < NUM_TILES {
            let off = t * TILE_H;
            // load this tile of the PARENT into registers ONCE (this IS the copy)
            let mut reg = [_mm256_setzero_si256(); TILE_REGS];
            let mut r = 0;
            while r < TILE_REGS {
                reg[r] = _mm256_loadu_si256(src.as_ptr().add(off + r * 16) as *const __m256i);
                r += 1;
            }
            // fold every PIECE survivor's i16 column for this tile
            let mut i = 0;
            while i < p_len {
                let s = p_sign[i];
                if s != 0 {
                    let col = pbase.add(p_idx[i] as usize * HIDDEN + off);
                    if s == 1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_add_epi16(reg[r], w);
                            r += 1;
                        }
                    } else if s == -1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_sub_epi16(reg[r], w);
                            r += 1;
                        }
                    } else {
                        let mul = _mm256_set1_epi16(s);
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_add_epi16(reg[r], _mm256_mullo_epi16(w, mul));
                            r += 1;
                        }
                    }
                }
                i += 1;
            }
            // fold every THREAT survivor's i8 column for this tile (widened in reg)
            let mut i = 0;
            while i < t_len {
                let s = t_sign[i];
                if s != 0 {
                    let col = tbase.add(t_idx[i] as usize * HIDDEN + off);
                    if s == 1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                            reg[r] = _mm256_add_epi16(reg[r], w);
                            r += 1;
                        }
                    } else if s == -1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                            reg[r] = _mm256_sub_epi16(reg[r], w);
                            r += 1;
                        }
                    } else {
                        let mul = _mm256_set1_epi16(s);
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_cvtepi8_epi16(_mm_loadu_si128(col.add(r * 16) as *const __m128i));
                            reg[r] = _mm256_add_epi16(reg[r], _mm256_mullo_epi16(w, mul));
                            r += 1;
                        }
                    }
                }
                i += 1;
            }
            // store this tile into the CHILD ONCE
            let mut r = 0;
            while r < TILE_REGS {
                _mm256_storeu_si256(dst.as_mut_ptr().add(off + r * 16) as *mut __m256i, reg[r]);
                r += 1;
            }
            t += 1;
        }
    }
}

/// Scalar fallback (non-avx2, e.g. aarch64/Spark). Same tile-once load/store
/// shape so the arithmetic order matches the AVX2 kernel; the inner i16 loop
/// auto-vectorizes well under target-cpu=native.
#[cfg(not(target_feature = "avx2"))]
#[inline]
fn tiled_apply_scalar(
    acc: &mut [i16; HIDDEN],
    weights: &[[i8; HIDDEN]; NUM_THREAT],
    idx: &[u16; MAX_DELTAS],
    sign: &[i16; MAX_DELTAS],
    len: usize,
) {
    let mut t = 0;
    while t < NUM_TILES {
        let off = t * TILE_H;
        let mut tile = [0i16; TILE_H];
        for j in 0..TILE_H {
            tile[j] = acc[off + j];
        }
        for i in 0..len {
            let s = sign[i];
            if s == 0 {
                continue;
            }
            let row = &weights[idx[i] as usize];
            if s == 1 {
                for j in 0..TILE_H {
                    tile[j] += row[off + j] as i16;
                }
            } else if s == -1 {
                for j in 0..TILE_H {
                    tile[j] -= row[off + j] as i16;
                }
            } else {
                for j in 0..TILE_H {
                    tile[j] += s * row[off + j] as i16;
                }
            }
        }
        for j in 0..TILE_H {
            acc[off + j] = tile[j];
        }
        t += 1;
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

/// Register-tiled fused apply of signed PIECE rows into ONE perspective
/// accumulator — the i16 piece analogue of `tiled_apply_avx2` (no i8 widen, piece
/// weights are already i16). Each TILE_H-lane tile of `acc` is loaded into
/// TILE_REGS ymm once, every listed column (`idx[i]` a 0..NUM_PIECE piece-row
/// index) is added (sign +1) / subtracted (-1) / multiply-added (rare |s|>1), and
/// the tile is stored once — so acc load/store traffic is NUM_TILES passes
/// regardless of how many rows are folded, vs one full-width load+store per row.
/// Bit-identical to repeated add_row_i16/sub_row_i16: the same signed i16 column
/// sums into the same lanes, only regrouped (integer add is associative; the i16
/// accumulator never overflows, gated by the per-node assert).
#[cfg(target_feature = "avx2")]
#[inline]
fn apply_piece_columns(acc: &mut [i16; HIDDEN], weights: &[[i16; HIDDEN]; NUM_PIECE], idx: &[u16], sign: &[i16], len: usize) {
    use std::arch::x86_64::*;
    // SAFETY: idx[i] < NUM_PIECE so each row pointer stays in `weights`, and
    // off + r*16 < HIDDEN so every acc/column load+store is in bounds; AVX2 is
    // guaranteed by the cfg gate.
    unsafe {
        let wbase = weights.as_ptr() as *const i16; // piece row r begins at wbase + r*HIDDEN
        let mut t = 0;
        while t < NUM_TILES {
            let off = t * TILE_H;
            let mut reg = [_mm256_setzero_si256(); TILE_REGS];
            let mut r = 0;
            while r < TILE_REGS {
                reg[r] = _mm256_loadu_si256(acc.as_ptr().add(off + r * 16) as *const __m256i);
                r += 1;
            }
            let mut i = 0;
            while i < len {
                let s = sign[i];
                if s != 0 {
                    let col = wbase.add(idx[i] as usize * HIDDEN + off);
                    if s == 1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_add_epi16(reg[r], w);
                            r += 1;
                        }
                    } else if s == -1 {
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_sub_epi16(reg[r], w);
                            r += 1;
                        }
                    } else {
                        let mul = _mm256_set1_epi16(s);
                        let mut r = 0;
                        while r < TILE_REGS {
                            let w = _mm256_loadu_si256(col.add(r * 16) as *const __m256i);
                            reg[r] = _mm256_add_epi16(reg[r], _mm256_mullo_epi16(w, mul));
                            r += 1;
                        }
                    }
                }
                i += 1;
            }
            let mut r = 0;
            while r < TILE_REGS {
                _mm256_storeu_si256(acc.as_mut_ptr().add(off + r * 16) as *mut __m256i, reg[r]);
                r += 1;
            }
            t += 1;
        }
    }
}

/// Scalar fallback for `apply_piece_columns` (non-avx2), same tile-once shape.
#[cfg(not(target_feature = "avx2"))]
#[inline]
fn apply_piece_columns(acc: &mut [i16; HIDDEN], weights: &[[i16; HIDDEN]; NUM_PIECE], idx: &[u16], sign: &[i16], len: usize) {
    let mut t = 0;
    while t < NUM_TILES {
        let off = t * TILE_H;
        let mut tile = [0i16; TILE_H];
        for j in 0..TILE_H {
            tile[j] = acc[off + j];
        }
        for i in 0..len {
            let s = sign[i];
            if s == 0 {
                continue;
            }
            let row = &weights[idx[i] as usize];
            if s == 1 {
                for j in 0..TILE_H {
                    tile[j] += row[off + j];
                }
            } else if s == -1 {
                for j in 0..TILE_H {
                    tile[j] -= row[off + j];
                }
            } else {
                for j in 0..TILE_H {
                    tile[j] += s * row[off + j];
                }
            }
        }
        for j in 0..TILE_H {
            acc[off + j] = tile[j];
        }
        t += 1;
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
fn single(batch: &mut ThreatBatch, board: &RunBoard, occ: u64, pt: usize, c: Color, sq: Square, add: bool, no_rays: u64, bq: u64, rq: u64) {
    // `bq`/`rq` (bishop|queen and rook|queen occupancy, color-merged) are passed
    // IN by the caller — they depend only on the board piece-sets, not on `sq`, so
    // relocate()'s two single() calls over the same post-relocation board share one
    // build instead of recomputing them twice. Slider geometry for CLASS B; the
    // sq-dependent PEXT pair stays here. `queen_a = b_att | r_att` is bit-identical
    // to `queen_attacks(sq, occ)` by construction.
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
        // attacker is the toggled piece itself, sitting at `sq`.
        batch.toggle(pt, vpt, (c != vc) as usize, sq, v, add);
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
                // attacker is the slider on `s` (both the beyond and direct).
                batch.toggle(s_idx, upt, (sc != uc) as usize, s, u, !add);
            }
        }
        // the slider's direct threat on the toggled piece itself (ALWAYS fires
        // in both halves — it nets correctly through the coalescer).
        batch.toggle(s_idx, pt, (sc != c) as usize, s, sq, add);
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
        // king-as-attacker dropped in batch.toggle. attacker is the leaper at `a`.
        batch.toggle(apt, pt, (ac != c) as usize, a, sq, add);
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
    // bq/rq are sq-independent (board piece-sets only, unchanged across both
    // halves) — build once and share, instead of recomputing inside each single().
    let all_b = b.pieces(Color::White, PieceType::Bishop) | b.pieces(Color::Black, PieceType::Bishop);
    let all_r = b.pieces(Color::White, PieceType::Rook) | b.pieces(Color::Black, PieceType::Rook);
    let all_q = b.pieces(Color::White, PieceType::Queen) | b.pieces(Color::Black, PieceType::Queen);
    let bq = all_b | all_q;
    let rq = all_r | all_q;
    single(batch, b, occ, pt, c, from, false, no_rays, bq, rq);
    single(batch, b, occ, pt, c, to, true, no_rays, bq, rq);
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
    let all_b = b.pieces(Color::White, PieceType::Bishop) | b.pieces(Color::Black, PieceType::Bishop);
    let all_r = b.pieces(Color::White, PieceType::Rook) | b.pieces(Color::Black, PieceType::Rook);
    let all_q = b.pieces(Color::White, PieceType::Queen) | b.pieces(Color::Black, PieceType::Queen);
    single(batch, b, b.occupied(), pt, c, sq, add, 0, all_b | all_q, all_r | all_q);
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
    // outgoing: old piece's attacks off, new piece's attacks on. Attacker is the
    // piece sitting at `sq` in both cases.
    let mut a_old = piece_attacks(old_pt, old_c, sq, occ) & occ;
    while a_old != 0 {
        let v = a_old.trailing_zeros() as Square;
        a_old &= a_old - 1;
        let (vc, vpt) = board.piece_on(v);
        batch.toggle(old_pt, vpt, (old_c != vc) as usize, sq, v, false);
    }
    let mut a_new = piece_attacks(new_pt, new_c, sq, occ) & occ;
    while a_new != 0 {
        let v = a_new.trailing_zeros() as Square;
        a_new &= a_new - 1;
        let (vc, vpt) = board.piece_on(v);
        batch.toggle(new_pt, vpt, (new_c != vc) as usize, sq, v, true);
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
        // attacker is the incoming slider/leaper at `a`; victim square is `sq`.
        batch.toggle(ai, old_pt, (ac != old_c) as usize, a, sq, false);
        batch.toggle(ai, new_pt, (ac != new_c) as usize, a, sq, true);
    }
}

const MAX_PIECE_DELTAS: usize = 8; // max piece toggles per move (castle = 4)

/// Per-move PIECE-feature toggles, batched like `ThreatBatch` so a move's 2-4
/// piece row changes apply in ONE register-tiled pass per perspective
/// (`apply_piece_columns`) instead of a full-width load/add/store per row. Row
/// indices are computed from the accumulator's (parent) king ctx at toggle time,
/// exactly as `add()`/`remove()` do — valid because the ctx is unchanged until
/// `apply_to`.
struct PieceBatch {
    w_idx: [u16; MAX_PIECE_DELTAS],
    b_idx: [u16; MAX_PIECE_DELTAS],
    sign: [i16; MAX_PIECE_DELTAS],
    len: usize,
}

impl PieceBatch {
    #[inline(always)]
    fn new() -> PieceBatch {
        PieceBatch { w_idx: [0; MAX_PIECE_DELTAS], b_idx: [0; MAX_PIECE_DELTAS], sign: [0; MAX_PIECE_DELTAS], len: 0 }
    }

    #[inline(always)]
    fn toggle(&mut self, acc: &Accumulator, col: Color, pt: usize, sq: Square, add: bool) {
        let i = self.len;
        self.w_idx[i] = wp_feat(col, pt, sq, acc.wk_base, acc.wk_flip) as u16;
        self.b_idx[i] = bp_feat(col, pt, sq, acc.bk_base, acc.bk_flip) as u16;
        self.sign[i] = if add { 1 } else { -1 };
        self.len = i + 1;
    }

    #[inline(always)]
    fn apply_to(&self, acc: &mut Accumulator) {
        let net = net();
        apply_piece_columns(&mut acc.w, &net.piece_weights, &self.w_idx, &self.sign, self.len);
        apply_piece_columns(&mut acc.b, &net.piece_weights, &self.b_idx, &self.sign, self.len);
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
/// Enumerate a move's piece + threat feature toggles into the two batches,
/// keyed by `ctx`'s king context (the parent's; the ctx is move-invariant except
/// for a king crossing, which `reindex_moving` repairs afterward). Shared by the
/// clone path (`apply_lazy`) and the copy-fused path (`apply_lazy_into`); both
/// read the parent's ctx for `pb.toggle`, so neither needs the cloned `w`/`b`.
#[inline(always)]
fn decompose_move(ctx: &Accumulator, pos: &Position, mv: Move) -> (PieceBatch, ThreatBatch) {
    let us = pos.stm;
    let them = us.flip();
    let from = mv.from();
    let to = mv.to();
    let (_, pt) = pos.piece_on(from).expect("nnue apply: empty from");
    let pti = pt.idx();

    let mut b = RunBoard::from_pos(pos);
    let mut batch = ThreatBatch::new();
    let mut pb = PieceBatch::new();

    match mv.flags() {
        flag::QUIET | flag::DOUBLE_PUSH => {
            pb.toggle(ctx, us, pti, from, false);
            pb.toggle(ctx, us, pti, to, true);
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
            pb.toggle(ctx, us, ki, from, false);
            pb.toggle(ctx, us, ki, to, true);
            pb.toggle(ctx, us, ri, rook_from, false);
            pb.toggle(ctx, us, ri, rook_to, true);
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
            pb.toggle(ctx, them, pi, cap_sq, false);
            pb.toggle(ctx, us, pi, from, false);
            pb.toggle(ctx, us, pi, to, true);
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
            pb.toggle(ctx, them, ci, to, false);
            pb.toggle(ctx, us, pti, from, false);
            pb.toggle(ctx, us, pti, to, true);
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
                pb.toggle(ctx, them, ci, to, false);
                pb.toggle(ctx, us, pi, from, false);
                pb.toggle(ctx, us, promo, to, true);
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
                pb.toggle(ctx, us, pi, from, false);
                pb.toggle(ctx, us, promo, to, true);
                relocate(&mut batch, &mut b, pi, us, from, to);
                mutate(&mut batch, &mut b, pi, us, promo, us, to);
            }
        }
        _ => unreachable!(),
    }
    (pb, batch)
}

pub fn apply_lazy(parent: &Accumulator, pos: &Position, mv: Move) -> Accumulator {
    let mut acc = parent.clone();
    let (pb, batch) = decompose_move(parent, pos, mv);
    #[cfg(not(feature = "fused"))]
    {
        pb.apply_to(&mut acc);
        batch.apply_to(&mut acc);
    }
    #[cfg(feature = "fused")]
    {
        #[cfg(target_feature = "avx2")]
        {
            let net = net();
            fused_apply_avx2(
                &mut acc.w, &net.piece_weights, &pb.w_idx, &pb.sign, pb.len,
                &net.threat_weights, &batch.w_idx, &batch.sign, batch.len,
            );
            fused_apply_avx2(
                &mut acc.b, &net.piece_weights, &pb.b_idx, &pb.sign, pb.len,
                &net.threat_weights, &batch.b_idx, &batch.sign, batch.len,
            );
        }
        #[cfg(not(target_feature = "avx2"))]
        {
            pb.apply_to(&mut acc);
            batch.apply_to(&mut acc);
        }
    }
    acc
}

/// COPY-FUSED apply_lazy: write the child accumulator into a caller-provided slot
/// `child`, reading the unchanged lanes directly from `parent` in the same tiled
/// pass that folds the move's toggles. Eliminates the `parent.clone()` 3KB memcpy
/// of `apply_lazy` — the search owns an accumulator stack indexed by ply, so the
/// child slot is already allocated; this turns "clone parent THEN modify" into
/// "copy-and-modify parent->child in one streaming pass". Bit-identical to
/// `apply_lazy` (same column sums folded onto the same parent baseline). Only the
/// AVX2 path is copy-fused; the scalar fallback clones into `child` then applies.
/// Requires `parent` and `child` to be DISTINCT slots (search guarantees this:
/// `apply_into(&self.acc[ply], &mut self.acc[ply+1], ...)`, ply != ply+1).
#[cfg(feature = "cfused")]
pub fn apply_lazy_into(parent: &Accumulator, child: &mut Accumulator, pos: &Position, mv: Move) {
    // Carry the parent's king ctx forward; the copy-fused kernel writes w/b.
    child.wk_base = parent.wk_base;
    child.wk_flip = parent.wk_flip;
    child.bk_base = parent.bk_base;
    child.bk_flip = parent.bk_flip;
    let (pb, batch) = decompose_move(parent, pos, mv);
    #[cfg(target_feature = "avx2")]
    {
        let net = net();
        copy_fused_apply_avx2(
            &mut child.w, &parent.w, &net.piece_weights, &pb.w_idx, &pb.sign, pb.len,
            &net.threat_weights, &batch.w_idx, &batch.sign, batch.len,
        );
        copy_fused_apply_avx2(
            &mut child.b, &parent.b, &net.piece_weights, &pb.b_idx, &pb.sign, pb.len,
            &net.threat_weights, &batch.b_idx, &batch.sign, batch.len,
        );
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        child.w = parent.w;
        child.b = parent.b;
        pb.apply_to(child);
        batch.apply_to(child);
    }
}

/// Apply `mv` to the parent accumulator. KING-CROSS: if the mover is the side-to-
/// move's own king AND the move crosses a bucket/mirror boundary, the moving
/// side's king ctx changes — so every piece row of THAT perspective re-indexes.
/// We do NOT full-refresh the child (that would also re-enumerate all threats —
/// ~82% of refresh cost — and rebuild the unaffected non-moving perspective).
/// Instead we apply the move incrementally (`apply_lazy` gives correct threats on
/// both perspectives and a correct non-moving perspective, since threats are
/// king-independent and the non-moving king's ctx is unchanged), then re-key only
/// the moving perspective's pieces from the old ctx to the new one. The result is
/// bit-identical to `refresh(child)` but a crossing costs ~one perspective's
/// piece re-key instead of a full rebuild. Otherwise the cached ctx stays valid
/// and the plain `apply_lazy` path is used.
///
/// NECESSARY-AND-SUFFICIENT: a perspective's piece rows are a pure function of
/// (bucket[that king], flip[that king], placement); the ONLY move that alters a
/// king's bucket/flip is a move of that king. Castling routes here too (from is
/// the king square, piece_on(from) == King; `apply_lazy` handles the castle flag).
/// The per-node debug gate (search.rs) is the oracle: refresh derives ctx
/// independently, so any ctx/re-key bug panics in debug and during the walk() test.
pub fn apply(parent: &Accumulator, pos: &Position, child: &Position, mv: Move) -> Accumulator {
    let from = mv.from();
    if let Some((c, PieceType::King)) = pos.piece_on(from) {
        if c == pos.stm {
            // Compare the king ctx IN THE MOVING SIDE'S PERSPECTIVE orientation:
            // White king drives the w accumulator (raw squares); Black king drives
            // the b accumulator (squares ^56). A change in either base or flip
            // means that perspective's piece rows all shift -> incremental re-key.
            let (from_o, to_o) = if c == Color::White { (from, mv.to()) } else { (from ^ 56, mv.to() ^ 56) };
            let (nb, nf) = king_ctx(to_o);
            let (ob, of) = king_ctx(from_o);
            if nb != ob || nf != of {
                let mut acc = apply_lazy(parent, pos, mv);
                acc.reindex_moving(child, c, nb, nf);
                return acc;
            }
        }
    }
    apply_lazy(parent, pos, mv)
}

/// COPY-FUSED `apply`: write the child accumulator directly into `out` (a slot the
/// search owns, `self.acc[ply+1]`) instead of returning by value. Same king-cross
/// logic as `apply`, but the incremental case goes through `apply_lazy_into` so
/// the parent's unchanged lanes are streamed parent->child without the 3KB
/// `parent.clone()` memcpy. King-cross still re-keys the moving perspective in
/// place on `out` after the copy-fused incremental fill. Bit-identical to `apply`.
/// `parent` and `out` MUST be distinct slots (ply != ply+1, guaranteed by caller).
#[cfg(feature = "cfused")]
pub fn apply_into(parent: &Accumulator, out: &mut Accumulator, pos: &Position, child: &Position, mv: Move) {
    let from = mv.from();
    if let Some((c, PieceType::King)) = pos.piece_on(from) {
        if c == pos.stm {
            let (from_o, to_o) = if c == Color::White { (from, mv.to()) } else { (from ^ 56, mv.to() ^ 56) };
            let (nb, nf) = king_ctx(to_o);
            let (ob, of) = king_ctx(from_o);
            if nb != ob || nf != of {
                apply_lazy_into(parent, out, pos, mv);
                out.reindex_moving(child, c, nb, nf);
                return;
            }
        }
    }
    apply_lazy_into(parent, out, pos, mv);
}

/// From-scratch eval (uci `eval`, datagen). Search uses the incremental stack.
pub fn evaluate(pos: &Position) -> Score {
    Accumulator::refresh(pos).eval(pos)
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
    // piece-only accumulator (bias + king-bucketed piece rows), in i16 — same as
    // the candidate. The king ctx must be set before add() so the bucket/mirror
    // rows match.
    let (wk_base, wk_flip, bk_base, bk_flip) = king_ctx_both(pos);
    let mut acc = Accumulator { w: net.feature_bias, b: net.feature_bias, wk_base, wk_flip, bk_base, bk_flip };
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
    for_each_threat(pos, |att_pt, vic_pt, enemy, asq, vsq| {
        let wf = &net.threat_weights[thr_feat_w(att_pt, vic_pt, enemy, asq, vsq)];
        let bf = &net.threat_weights[thr_feat_b(att_pt, vic_pt, enemy, asq, vsq)];
        for k in 0..HIDDEN {
            sw[k] += wf[k] as i32;
            sb[k] += bf[k] as i32;
        }
    });
    let (us, them) = if pos.stm == Color::White { (&sw, &sb) } else { (&sb, &sw) };
    let n = pos.occupied().count_ones() as usize;
    let bkt = ((n - 2) / 4).min(NUM_OUTPUT_BUCKETS - 1);
    let ow = &net.output_weights[bkt];
    let mut output: i64 = 0;
    for i in 0..HIDDEN {
        output += screlu_i32(us[i]) as i64 * ow[i] as i64;
        output += screlu_i32(them[i]) as i64 * ow[HIDDEN + i] as i64;
    }
    output /= QA as i64;
    output += net.output_bias[bkt] as i64;
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
        // KING-MARCH hazards: a bare-king walk crosses bucket/mirror boundaries
        // so apply() must route through a full refresh and stay bit-identical.
        // White Ke1 can step to d1 (no cross), f1 (file 3->4 mirror flip), e2
        // (rank bucket 0); the depth-2 walk then continues e2->e3 (rank bucket
        // 0->1, the r/2 boundary at ranks 1&2). Black Ke8 mirrors.
        "4k3/8/8/8/8/8/8/4K3 w - - 0 1",
        // king near the rank-3/4 bucket boundary (ranks 3&4) with pieces so the
        // walk hits crossings amid a populated board, both sides to move.
        "8/3k4/8/3K4/8/8/8/8 w - - 0 1",
        // castle-into-mirror-half: e1-g1 (king crosses to file 6) and e1-c1
        // (king to file 2) both change the king bucket/mirror -> refresh.
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
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
