# NNUE within-50 plan (from multi-engine survey, 2026-06-20, row 94)

Eval is the dominant gap (row 94: Razor search + SF eval = +308 Elo @ equal nodes).
Big-net failed before (-80) on WEAK labels, not arch — lever is DATA/RECIPE + capacity to hold a strong eval.
Survey: Viridithas/Stormphrax/Reckless/PlentyChess (all Rust/C++ + bullet, top-10 CCRL) converged on one shape.

## Teacher available now
- `sfnnue.rs::evaluate_sf()` = SF-class STATIC eval, in-process (~204k nps full-refresh).
- `razor-sfnet.exe` = Razor search + SF eval = the +308 teacher (3600-class @ equal nodes). Use as datagen engine / labeler for SF-eval-backed SEARCH labels (stronger than static).

## Target production arch (v3)
- King-bucketed mirrored inputs (start 10 buckets; Reckless 10 / Plenty 12 / Obsidian 13 / SF 16).
- L1 = 1024 per perspective x2 (Razor now 768).
- Tail L1 -> 16 -> 32 -> 1. 8 OUTPUT BUCKETS by piece count ((occ-1)/4, idiom already in sfnnue.rs:213).
- SCReLU FT; pairwise-multiply FT->L1 after int8 lands.
- Quant (Reckless template, bullet-default): FT i16 QA=255, L1 i8 QB=64, L2/L3 f32, FT_SHIFT=9, SCALE retune (~380-400).
- DEFER threat inputs to v4 (Reckless ~66.9k rows / Plenty 59808+4560 / SF SFNNv10) — highest ceiling, most work.

## Data pipeline (the gap)
- GENERATE self-play positions (Razor or razor-sfnet), LABEL with SF-class eval (evaluate_sf static, or razor-sfnet fixed-node search = stronger) BLENDED with game WDL.
  - Leapfrog vs peers: they self-label with own weak eval; Razor labels with SF-class eval in-process.
- Datagen params (peer consensus): 8-9 random opening plies; discard |verify|>500-1000cp openings; win-adj |score|>=1250/5ply, draw-adj |score|<=10/10ply@70; Syzygy rescore endgames (6-piece avail, lambda=0 override).
- Filter (Viridithas): min_ply 16, min_pieces 4, drop in-check + tactical, max_eval 20000.
- WDL blend (Caissa, verified): target=lerp(WDL, sigmoid(eval/scale), baseLambda*exp(-moveCount/120)), base~0.7.
- Format: bulletformat (Razor trains in bullet) or viriformat.
- Scale: billions over generations; iterate (regen with latest net, purge old — Caissa loop). GPU/Spark train, 88-core farm gen+label.

## Recipe (bullet)
- ~800 superbatches (production); 10-40 SB smoke first. Batch 16384. lr 1e-3 step gamma 0.1 / cosine. ~1 epoch.
- WDL lambda ~0.7 (ideally move-count weighted). LOSS = error^2.0-2.5 power (Viridithas +15-24 Elo vs squared).
- AdamW, weight decay ON, beta1 0.9->0.95 (+4), tighter FT weight clip (+7-9).

## Fast inference (>1.5M nps)
- Keep FT incremental (apply/add/remove already do). King-bucket move -> refresh table (Stormphrax pattern), only on king moves.
- int8 L1 + VNNI/maddubs (the nps unlock for L1=1024). New src/nnue/simd/ (AVX512-VNNI > AVX2 > NEON for Spark > scalar). Reckless src/nnue/simd/* = Rust template.
- NNZ sparsity on FT output (Reckless propagate_l1 over nnz; Berserk LOOKUP_INDICES[256][8]).
- Pairwise FT after int8. Tiny f32 tail negligible.

## Execution order (each SPRT-gated, RESULTS row, HEAD always verified)
1. CALIBRATION: confirm evaluate_sf cp-scale (sfnnue.rs:275 ~+1800/Q) vs training SCALE; fix before mass gen.
2. v1: NEW DATA on CURRENT arch (768x768x1, no code change) — SF-teacher-labeled self-gen. Isolates data lever. SPRT vs HEAD.
3. v2: arch rewrite (king+output buckets, L1=1024, i64 accum, int8/NNZ SIMD), retrain same data. SPRT.
4. v3: scale data (billions, generations) + recipe polish (power-loss, move-count lambda). SPRT each.
5. v4: threat inputs. SPRT.

## Honest Elo
+250-330 over Razor (~3290 -> ~3540-3620 public-data tier) is the realistic near-term. Within-50-of-SF likely needs v4 threats + more generations. Do NOT over-promise from v1.
