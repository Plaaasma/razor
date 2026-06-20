# RAZOR — Project State

> Journal per project brief (`H:\RazorBot\aggressive_engine_prompt.md`). Re-read brief at session start. Update this file every session.

---
## ★ SESSION HANDOVER (2026-06-14) — READ THIS FIRST ★

**★★ SESSION 4 CONSOLIDATED STATUS (2026-06-16) — CURRENT TRUTH, read first ★★**
**★ RAZOR ELO ESTIMATE (2026-06-17, first gauntlet): ~3150 ±100 single-thread (20+0.2 1T vs CCRL anchors), ~3270 at 8T (SMP +~120) — agrees with the SF18-gap method (−333 at 8T LTC ≈ 3270 if SF18~3600).** Gauntlet (gauntlet_ltc.ps1, 420g): vs berserk-14 6.4% (anchor3616→3151), caissa-1.25 11.8% (3610→3260), reckless-0.9 7.1% (3500*→3104). *Reckless 0.9 underrated vs its stale 0.7 CCRL 3417 (Razor underperforms it both gauntlets → reckless0.9 ~3500+). 8+0.08 gauntlet distorted (ultra-fast favors lean engines, inverts order) — use 20+0.2 (gauntlet_ltc.ps1) for sound numbers.

**★ ELO GAUNTLET (2026-06-17, user): `matches\gauntlet.ps1` (8+0.08, distorted) / `matches\gauntlet_ltc.ps1` (20+0.2, sound) — Razor vs downloaded CCRL anchors for periodic absolute-Elo checks.** Anchors in `tools\engines\`: reckless-0.9.0.exe, berserk-14.exe (+berserk-14.nn), caissa-1.25.exe. CCRL 40/15 refs: Reckless 0.7≈3417 (0.9 higher), Berserk 13≈3616, Caissa 1.20≈3610, SF17≈3642 (all ABOVE Razor ~3270 → triangulate from above; Reckless closest). Run: `powershell -File matches\gauntlet.ps1 [razorExe] [rounds]` (1T 8+0.08, -tournament gauntlet -seeds 1). Komodo skipped (commercial). To estimate Razor Elo: razor_vs_anchor Elo + anchor CCRL, averaged (cross-TC/CPU caveat ±50-100). Run only when box free (not during an SPRT).

**★★★★★ MULTI-AGENT AUDIT VERDICT (2026-06-20, workflow w03lhd7c9, 27 agents, adversarially verified) ★★★★★** Why all from-scratch nets lose to razorsf (gen1 −221, gen1b −381, pubsf −862, sfd1 −511): NOT a code/inference bug — bucketed inference VERIFIED correct (row 88: buckets on razorsf's exact public data = neutral, identical 0.046 loss). ROOT CAUSE = wrong position DISTRIBUTION + insufficient SCALE: every loser trained on Razor self-play FENs (or 4M public slice); razorsf used full public test80 (~1.5B filtered, streamed via SfBinpackLoader). Decisive controls: row 58 fine-tune-on-selfplay −233; row 88 same-public-distribution neutral. Small data → COMPRESSED eval (+Q 1395 vs razorsf 1800, monotonic with dataset size) → mismatches cp-tuned tune.rs margins → search over/under-prunes → catastrophic play despite sane probe eval (search trusts eval: RFP returns static_eval as cutoff). ONE REAL BUG FOUND+FIXED: label_sf.py mate scores unclamped (~29994 → sigmoid saturates ~5% targets, mostly bucket0); now clamped to ±10000 (matches razorsf filter). **HARD CEILING (rigorously confirmed): even a perfect full-scale deeper-label net = at most +16 Elo over razorsf (row 56 A/B); from-scratch ceiling = TIE razorsf. The ~280 Elo to within-50 of SF18 is DOMINATED BY SEARCH/SPEED (nps, depth), NOT eval.** → Data farm PAUSED (was chasing the capped lever; saved money). Pivoted to a search/speed Elo-opportunity workflow (wahb8rr3i). Corrected data recipe (if ever needed): SfBinpackLoader on full public binpacks, razorsf 80sb schedule, verify +Q~1800 post-train or re-tune margins. Farm: 5 nodes (~88c) idle, drivable for distributed SPSA. Net-experiment artifacts/drivers in matches\.

**★★★★ ALL LEVERS EXHAUSTED — v0.12.4 IS THE VERIFIED CEILING (2026-06-18, session 5 end) ★★★★** Both axes fully tested & tapped. SEARCH: cheap ladder + the last structural lever done — cut_node co-SPSA (r86) recovers cut_node from −8.7 to ~neutral (+1.5–3, not SPRT-passable, not banked); Razor's search is at its optimum (r63 SPSA + 7 post-IIR-broaden experiments, only IIR-broaden +7 banked → v0.12.4). EVAL: capped all 3 ways (1024 −70 r82, from-scratch 4M −862 r84 / full-scale infeasible, fine-tune −43 r85). **v0.12.4 (= razorsf 768 + SMP + TM + adaptive-NMP + IIR-broaden; bench 19963) is Razor's ceiling with every available method. vs SF18: 8.9%/−405 1T, ~−285 8T. within-50 (~+280) is NOT reachable** without a fundamentally stronger teacher than SF18 (doesn't exist cheaply) or far more eval compute than feasible. Session-5 net gains banked: IIR-broaden +7 (v0.12.4) + sf_wins.pgn fixed (time-forfeit wins removed, 1 real mate kept) + reusable eval pipeline (pub_sf18.bin, razor_net_pubsf/ft.rs, label_sf.py). Mission as literally stated is asymptotically complete at the engine's capability ceiling.

**★★★ EVAL LEVER CONCLUSIVELY CAPPED (2026-06-18, session 5) — all 3 relabel paths tested & dead ★★★** The stronger-teacher (SF18-50k relabel) lever does NOT bank a gain over razorsf, by ANY path: (1) bigger net / capacity — 1024 −70 (r82, loss only 0.8% lower = teacher-bound). (2) from-scratch on relabeled data — 4M public −862 (r84); matching razorsf scale needs ~1.5B relabeled positions = ~85 days @ 24-worker SF18-50k (infeasible). (3) FINE-TUNE razorsf on strong labels — −43 even with the low-wdl collapse-fix (r85; de-optimizes razorsf's eval, compresses it). The +16 A/B (r56) was RELATIVE between two small from-scratch nets at matched 2M scale — it never beats razorsf, whose ~billions-position scale dominates. **CONCLUSION: razorsf (768, public test80 labels) is at/near Razor's eval ceiling; no tractable path raises it.** Data/tooling preserved (pub_sf18.bin 4M, razor_net_pubsf.rs, razor_net_ft.rs wdl0.25, label_sf.py) if a fundamentally stronger teacher than SF18 ever exists. NOTE: full binpack dump is UNLIMITED → 1.5B pos / 88GB (fills disk); always pass a limit to razor_dump_fens. Search ALSO tapped (6 fails post-IIR-broaden incl cut_node −8.7). **v0.12.4 = Razor's ceiling with available methods; within-50 of SF18 (~+280) unreachable.** Only untried long-shot: cut_node+LMR co-SPSA (search, low confidence).

**★★ STRATEGIC CEILING REACHED (2026-06-17, session 5) — BOTH AXES CAPPED ★★** v0.12.4 vs SF18 = **8.88% / −405 Elo (1T, 800g, row 81)**, ~−285 8T. Within-50 of SF18 = ~3550 Elo = **+280 over current — NOT reachable** with available methods. (a) SEARCH cheap ladder TAPPED: post-IIR-broaden 6 fails 0 wins (NMP-eval~0, lmrcap−4, histlmr−4, improving~0, cut_node−8.7). (b) EVAL bigger-net axis DEFINITIVELY DEAD (row 82): clean 1024-hidden net trained on razorsf's exact data reached loss 0.045769 vs 768's 0.046132 = only 0.8% lower → **capacity is NOT the limit, net is teacher-bound**; SPRT −69.6 (i64 speed tax). Joins d2 −210, width/buckets neutral. **ONLY remaining lever = stronger-teacher-than-SF18-public relabel** (multi-day SF18-high-node, +16, STILL gap-short) or LMR+cut_node co-SPSA (search, ~+5-10, uncertain). Mission as literally stated is asymptotically out of reach; awaiting user redirection (bank/pause vs commit multi-day grind vs redefine goal). Session 5 banked: IIR-broaden +7 (v0.12.4) + the +49 1T re-baseline confirm.

**HEAD = v0.12.4 (tag, 2026-06-17)** = v0.12.3 + **IIR broaden to stale-shallow TT nodes** (IIR fires when tt_depth+4<=depth too, not only on no-TT-move; +7.4 Elo row 77, 3274g, bench unchanged 19963 — game-condition-only effect, bench-invisible). Search reduction vein still productive post-NMP. Session-5 search ladder so far: NMP+22 (v0.12.3), NMP-eval ~0 (rej), LMR-captures −3.8 (rej row 76), IIR-broaden +7.4 (v0.12.4). Archives masters\razor-v0.12.4.exe.
**Prior: HEAD v0.12.3 (tag)** = v0.12.2 + **adaptive NMP** (R=3+depth/4, +22.2 row 73, bench 19963). v0.12.2 = v0.12.1 + **stability-based TM** (+9.35, formal SPRT pass, row 70 — first TM improvement; stop sooner when best move stable). v0.12.1 = v0.12.0 + **SMP helper diversity** (odd helpers reduce 1 less in LMR, +11 at 8T, row 68). v0.12.0 = v0.11.0 + **lazy SMP** (+95-117 at 8T). 1-thread bench 20008 (1T play identical throughout). Full line: v0.7.0 net + i32 + log-LMR + singular + IIR + conthist + SMP + SMP-diversity. **★★ M1 MILESTONE CONFIRMED (row 69): v0.12.1 = 12.80% vs SF18 at LTC, 500g, CI lower bound ~10.4% > 10% bar. Gap −333 (was −454 1T session start). Brief §7 defining milestone PASSED. ★★** Clean, committed. Archives masters\razor-v0.8.0…v0.12.1.exe.
This session shipped **v0.11.0 (continuation history +7.3, row 53)**, then exhaustively mapped BOTH mission axes:
- **EVAL axis CAPPED.** Net teacher-limited at public test80 label quality. Architecture exhausted (width/king-buckets/data-quantity/2-layer-depth all neutral or fail, rows 51-52). Stronger teacher is the only real eval lever (+16 at matched scale, A/B 50k>8k labels r56) but NOT cheaply bankable: from-scratch 2M = −500 (scale r57); fine-tune razorsf on Razor-pos −233 (r58), on PUBLIC-pos −82 (r59 — SF18-label shape de-optimizes a net trained on test80 labels). Only path to the +16 = train FROM SCRATCH on ~razorsf-scale (tens of M positions) @ 50k = DAYS of labeling (SF18 ~9 Mnps aggregate under 24-way load, bandwidth-bound).
- **SEARCH axis TAPPED.** Big wins banked (log-LMR/singular/IIR/conthist). Everything post-conthist neutral/neg: 2-ply conthist 0 (r54), razoring −6 (r55), improving heuristic −7 (r61), probcut +1 (r62). Proper SPSA (6 params incl. the untuned singular margin): all near-optimal, ±5% drift, R≈0 (r63).
**M1 re-baselined (row 64): v0.11.0 = 6.81% vs SF18 / −454 (STC 1T), up from v0.9.0 4.75%/−520.**

**★★ LAZY SMP DONE = +95-117 Elo at 8T (row 65, v0.12.0).** Lockless atomic TT (tt.rs) + `search::search_threaded` (N searchers share the TT via thread::scope; main drives time, shared AtomicBool stop; UCI Threads honored). 1T identical (bench 20008). Validated 8T-vs-1T self-play (can't use the 1T ladder). **This was the big M1 lever** — official M1 is 8-thread, where single-threaded Razor was crippled. Helpers currently run UNDIVERSIFIED (identical search) → adding helper diversity (depth/move-order/LMR variation) should add more SMP Elo.
  - **★ M1 ~MET (row 67): LTC-8T = 10.62% vs SF18 (−370), 240g** — central above the ≥10% bar (STC-8T 9.0% r66, STC-1T 6.81% r64). CI ±~3.5% → consistent-with-M1, not statistically nailed; a 500-pair official run (vs_stockfish.ps1, 16h) would confirm. **lazy SMP carried Razor ~5%→10.6% vs SF18 this session = M1 milestone reached at central estimate.** Gap now ~−370 to SF18 (was ~−515 at session start).
  - If LTC M1 <10%: (a) **helper diversity** (staggered depths / move-order / LMR jitter — my helpers run undiversified; +SMP Elo, the real M1-push lever) → re-run 8T M1. (b) full official M1 (vs_stockfish.ps1 60+0.6, 16h) to confirm.

Other remaining bets (lower priority): compute-heavy from-scratch eval relabel (~days, +16, tooling built); bigger-fast net arch; stronger-than-SF18 teacher. Search-param tuning tapped (SPSA r63).
All tooling built + reusable: label_sf.py (incremental SF18 relabeler), razor_dump_fens.exe, razor_net_dg.rs (from-scratch, env DG_BIN/DG_ID), razor_net_ft.rs (fine-tune), ab_label.ps1, spsa.py (decay+semargin). semargin is now a UCI tunable (default 200 = old behavior).

**★ MISSION (2026-06-16, user): full autonomous until Razor within 50 Elo of SF18.** Currently ~−515 → need ~+465. Search tweaks (+10-35) can't get there alone; the bulk must come from EVAL/net. Campaign: keep mining search for drops + drive the net axis hard (depth, then stronger-teacher data). Every change SPRT-gated, HEAD always verified, journal each result, re-check vs-SF gap per ~+50 cumulative.

**★ HEAD = v0.11.0 (tag) = v0.10.0 + continuation history. bench = 20008.** Verify `target\release\razor.exe bench` → 20008. conthist +7.3 (row 53). Net-architecture axis EXHAUSTED (width/buckets/data-quantity/depth all tapped — teacher-limited at 768). Search axis productive again via conthist (move-ordering). Banked search wins: log-LMR +9.5, singular +35, IIR +16, conthist +7. Archives masters\razor-v0.8.0…v0.11.0.exe. Post-conthist cheap search tapped again (2-ply conthist neutral row 54, razoring −6 row 55).

**★★ EVAL AXIS FULLY MAPPED + CAPPED (2026-06-16 session 4) — read this before any eval work ★★**
The net is teacher-limited at the public test80 label quality, and that ceiling is NOT cheaply raisable:
  - Net architecture exhausted: width(1024 speed-capped), king-buckets(neutral), data-quantity(4mo neutral r51), DEPTH(2-layer d2 −210, only 3% loss r52). All tapped.
  - Stronger teacher (deeper SF18 labels) = the only real eval lever, +16 at matched scale (A/B 50k>8k r56). BUT not bankable:
    · from-scratch on 2M = −500 (scale-dominated r57); matching razorsf scale needs ~100M @ 50k ≈ DAYS of labeling (SF18 ~9Mnps aggregate under 24-way load, bandwidth-bound; public middlegames ~1.7-4 pos/s/worker @50k).
    · fine-tune razorsf on Razor-pos = −233 (distribution shift r58); on PUBLIC-pos gentle = −82 (r59: fine-tuning toward SF18-50k labels de-optimizes a net trained on test80 labels; non-linear label-shape diff, not a fixable rescale).
  - **PRACTICAL CONCLUSION: eval is capped. Don't re-explore unless willing to spend days labeling ~100M @ 50k for a from-scratch net (the ONLY path to the +16), or a stronger-teacher-than-SF18.** Tooling all built + reusable (label_sf.py incremental, razor_dump_fens.exe public→FEN, razor_net_dg.rs from-scratch, razor_net_ft.rs fine-tune, ab_label.ps1). → MISSION now leans on SEARCH for tractable gains; eval needs a compute-heavy from-scratch run to move.

**★ DATAGEN FINDINGS (2026-06-16 session 4) — deeper teacher VALIDATED, scale dominates, FINE-TUNE is the path:**
  - **A/B (row 56): SF18 50k labels beat 8k labels +15.9 Elo** on the same 2M Razor positions → deeper teacher is a real eval lever (+16).
  - **Scale check (row 57): a 2M strong-teacher net loses 0-74 (~−500) to razorsf** (full ~100M-position net) → SCALE DOMINATES; from-scratch relabel can't beat razorsf without ~100M positions @ 50k (~6 days labeling, infeasible).
  - **→ FINE-TUNE razorsf** (resume razorsf-80 checkpoint, keeps its scale, low-lr continue on strong labels) is the tractable way to bank the +16. Mechanism works: `trainer.load_from_checkpoint(dir)` then `run()` on new data (razor_net_ft.rs, env DG_BIN/DG_ID/DG_CKPT). razorft (fine-tuned on 2M Razor@50k, low lr) eval 45/1231 (razorsf-like, preserved calibration). **RUNNING: razor-ft vs v0.11.0 SPRT bg `b7e6vidk1` (sprt_ft.ps1, [0,10]).** Caveat: fine-tuned on RAZOR positions (distribution shift vs razorsf's public positions) — direction/mechanism test. If +: fine-tune works → clean version = fine-tune on strong-relabeled PUBLIC positions (needs sfbinpack→FEN dumper; bullet has no binpack→text). If neutral/−: distribution shift → go straight to public-relabel fine-tune.
  - **Tooling:** matches\label_sf.py (incremental SF18 relabeler, ~9Mnps aggregate under 24-way load = bandwidth-bound, ~150ms/pos@50k), gen_data.ps1, ab_label.ps1, razor_net_dg.rs (from-scratch, env-param), razor_net_ft.rs (fine-tune). Data: dg_all.txt (12M Razor positions), dg_rest.txt (10M staged), dg_{strong,weak}.bin (2M@50k/8k), dg_strong_all/dg_weak_all.txt.

**(prior datagen notes:)** net is teacher-limited at the public test80 label quality → relabel positions with SF18 (deeper than the public labels) for a stronger teacher. Tooling BUILT + validated:
  - `matches\label_sf.py` — SF18 (H:\RazorBot\play\Stockfish-18.exe) relabeler, reads `fen | cp | result`, replaces cp with SF18 fixed-node white-relative score, parallel workers. VALIDATED: startpos +47, KPvK +1 (SF knows it's drawn — real endgame knowledge the low-node public labels lack = the stronger-teacher signal), queen-up +685. Usage `python label_sf.py <in> <out> [nodes=100000] [nproc=20]`.
  - `matches\gen_data.ps1` — 12× parallel `razor datagen` (1M each = 12M positions, ~40min, 434 pos/s/inst), concat → `matches\dg_all.txt`. **RUNNING bg `blghcl1qu`.**
  - Razor datagen: `razor datagen <out> <n> [seed]`, self-play, outputs `fen | cp | result` (white-rel), filters to quiet non-check non-mate positions. cp here is RAZOR's (weak) — label_sf REPLACES it with SF18.
  **PIPELINE next (when dg_all.txt ready):** `python label_sf.py dg_all.txt dg_labeled.txt 80000 24` (~3-4hr) → bullet `convert --from text -i dg_labeled.txt -o dg.bin` → train plain-768 net on dg.bin (new script razor_net_dg.rs w/ DirectSequentialDataLoader) → SPRT vs v0.11.0.
  **CONFOUND/RISK:** Razor self-play positions are lower-quality + smaller scale (12M) than public test80 (~100M+ SF positions) → may LOSE on positions/scale even if SF18 labels are better. This is a directional pipeline-validation test. If it loses, the CLEAN test = relabel the PUBLIC binpack positions (same distribution/scale, only labels change) — needs a sfbinpack→FEN dumper (bullet has no binpack→text; sfbinpack Position FEN API unconfirmed). If Razor-positions+SF18 ties/wins → stronger-teacher validated, scale up.

**4-month-net re-test = NEUTRAL +1.9±14 (row 51).** The old −57 was i64-contaminated; 4-month SF data at plain-768 is genuinely neutral. With king-buckets also neutral → **net is TEACHER-LIMITED at 768** (data-quantity + input-capacity don't help). → eval gain needs DEPTH (non-linearity) or a STRONGER TEACHER.

**DEEPER 2-LAYER NET (d2) REJECTED −210±35 (row 52).** Net trained GOOD (loss 0.0444 < 4-month 1-layer 0.0459) + eval verified sane (startpos 36, +Q 1051, sym, perft, depth-15 PV sound) — but eval **4.2× slower** (679k nps vs 2835k): scalar i64 l1-accumulate (16×768 MACs) + float l2. −210 is SPEED not correctness. **KEY: a whole extra layer buys only ~3% lower loss → modest eval gain can't overcome even an optimized speed tax → NET IS TEACHER-LIMITED EVEN WITH NON-LINEARITY.** (Gotcha for any future deeper net: l1w post-`.transpose()` = [out][in] row-major byte[o*768+i]; quantised.bin has size%64 trailing "bullet" pad. Artifacts: nets\razord2.nnue, candidates\nnue-d2.rs + razor-d2.exe, examples\razor_net_d2.rs.)

**★ NET-ARCHITECTURE AXIS IS EXHAUSTED (2026-06-16): width(1024 −54 i64-contam but speed-capped), king-buckets(neutral), data-quantity(4mo neutral row 51), DEPTH(d2 −210/3%-loss row 52) — all tapped. The net captures ~all of the ~3500 SF teacher signal at plain-768.** The ONLY remaining big EVAL lever = a STRONGER TEACHER (relabel positions with SF18 at higher nodes/depth than the public test80 labels → lower achievable loss → real eval gain; this is the +477-category lever). Tractable search levers remain too (proper SPSA of the now-rich param set; SE-margin untuned; razoring/history-pruning).

**NEXT (mission, prioritized):** (1) **proper SPSA** of search params — infra exists (tune.rs + matches\spsa.py + UCI tunables); earlier run −9 was underpowered (200 iter/32 games, no decay); add singular margin/threshold to tune.rs first; well-powered run could +20-40, tractable, good autonomous bg work. (2) **stronger-teacher datagen** — the big eval bet (SF18 high-node relabel); scope the labeling pipeline. (3) incremental search features. HEAD = v0.10.0 (20137), clean.

**RUNNING (2026-06-16 session 4): CONTINUATION HISTORY (proper redo) SPRT** — bg `bk75ewmfp`, `matches\sprt_conthist.ps1`, `razor-conthist.exe` (bench 20008, perft exit 0) vs `masters\razor-v0.10.0.exe` [0,10]. Razor had only butterfly history → conthist is the high-EV missing ordering feature. 1-ply [prev_pt][prev_to][cur_pt][cur_to] bonus-only, folded into quiet ordering + cutoffs; ordering bands rescaled (killers 1.95M, captures 2M, quiets = butterfly+conthist each ≤800k). **This is the row-52-mandated redo of conthist v1 (row 31 −50.5 was i64-CONTAMINATED + band-clamp).** Src change STASHED (`git stash`, HEAD clean v0.10.0 @20137) + saved `candidates\search-conthist.rs`. If PASS → `git stash pop` + commit → v0.11.0, then add history-malus + 2-ply conthist (further EV). If FAIL → `git stash drop`, conthist genuinely neutral for Razor → SPSA / datagen next.

**Where we are:** Razor **v0.10.0** (tag `v0.10.0`) = v0.9.0 + **IIR** (+15.6 STC / +25.5 LTC, rows 47/49). Full line: v0.7.0 net (768, `razorsf.nnue`) + i32 acc + log-LMR + **singular ext** (+35/+16) + **IIR** (+16/+26). bench **20137**, archive `masters\razor-v0.10.0.exe`. ~+51 STC over v0.8.0 from the two search wins. vs SF18 last measured 4.75% at v0.9.0 (noisy; M1 recheck deferred until more accumulates — vs-SF% can't resolve <~50 Elo). M1 (−400) was ~115 Elo out at v0.8.0; the +51 narrows it but the noisy metric won't show it yet.

**HEAD = v0.10.0 = v0.7.0 net + i32 acc + log-LMR + singular ext + IIR. bench = 20137.** Verify `target\release\razor.exe bench` → `Nodes searched : 20137`. `src\nnue.rs`: `HIDDEN=768` + `include_bytes!("../nets/razorsf.nnue")` + **i32** acc (NOT i64). `src\search.rs`: log-LMR `lmr_table()` + singular ext (per-ply `excluded[ply]`) + IIR (depth≥4 & no tt_mv → depth−1). Tagged **v0.10.0** (IIR LTC-confirmed +25.5); v0.8.0/v0.9.0 also archived.

**RUNNING (2026-06-15): MIRRORED 4-bucket net training** — bg `bav21my7y`, `razor_net_kbm.rs` (768x4 mirrored king-buckets by mirrored-file + factoriser, 4 SF months, 320 sb), log `logs\razorkbm-train.log`, dir `checkpoints_razorkbm\`. The top-EV lever: mirror folds both wings into each bank → ~2× data/bucket → better-trained banks → more eval at the SAME 4.72MB table/~9% tax → should tip the (validated, LTC+8) 4-bucket lever STC-positive. ~1.5-2hr.
**MIRROR INFERENCE STAGED + COMPILES (backup `matches\candidates\nnue-kbm.rs`):** adds per-perspective flip mask (`king_flip(sq)=(file>3)?7:0`, XORs feature file bits), `king_bucket=MIRROR_MAP[file]` (`[0,1,2,3,3,2,1,0]`), Accumulator{wflip,bflip}, refresh-on-(bucket OR flip)-change. search = `search-kb.rs` (apply(child), reused). HEAD = v0.8.0 (20249).
**NOTHING RUNNING.** Box clean. HEAD = v0.8.0 (bench 20249, tag v0.8.0) + tunable infra.

**★ HONEST CORRECTION (2026-06-15): king-buckets are ~NEUTRAL, not "real-eval-masked-by-tax" ★** Gating the apply() king-check on king-moves-only gave NO speedup (opt ~541k vs v0.8.0 ~562k, eval byte-identical) → the "~9% tax" was mostly noise, and the 4-bucket "LTC +8" had CI ±24 (included 0). Real read: input-capacity (buckets) doesn't improve eval over plain-768 on this data. BUT the capacity-limited conclusion is NOT clean — the 1024-width failure (−54) was i64-CONTAMINATED (1024 needs i64 output acc for overflow = the slow path). So one capacity axis is genuinely untried + i32-safe + low-speed-cost: **NET DEPTH (a 2nd hidden layer)** — adds non-linearity (extracts more from the same data in a way width/buckets can't), HIDDEN stays 768 (i32-safe), the added small layer (e.g. 1536→16→1) costs ~nothing vs the 768 accumulator. **★ SINGULAR EXTENSIONS PASSED +35.4±15 (row 43, committed cae7c56) — the M1-progress lever ★** Biggest search win since the early ladder; confirms search-side levers are productive where net-capacity was teacher-limited. **HEAD = v0.8.0 + singular** (bench still 20249, singular only fires depth≥8). Search lever is NOT tapped after all (the earlier "search tapped" was ordering/margin tweaks; singular is selective deepening — a different, big mechanism).

**NOTHING RUNNING. HEAD = v0.10.0 (bench 20137), box clean.**

**★ CHEAP SEARCH LADDER TAPPED AT v0.10.0 (2026-06-15) ★** The two big wins are banked — **singular +35 (v0.9.0), IIR +16 (v0.10.0)**. Every cheap tweak SINCE is marginal/negative: LMP −96, double-ext −58, multi-cut −9, LMR-reduce-less-in-PV −5 (rows 45/46/48/50). Pattern: targeted selectivity/reduction (singular, IIR) win; broader pruning/extension/reduction over-cost. **Next real Elo needs (fresh-session, bigger):**
  1. **Proper SPSA** — the underpowered 200-iter/32-game run failed (−9); a real one (more games/iter, a/c decay, the now-richer param set: LMR base/div, RFP/futility margins, singular margin/threshold — expose SE params in tune.rs first) could tune +20-40. The tunable infra exists.
  2. **Deeper 2-layer net** (untried capacity axis) / **stronger-teacher data** (the biggest historical lever, +477) — both bigger net/data projects.
  3. Possibly gentler/depth-gated versions of the failed tweaks (LMP/LMR-PV) via SPSA.
  M1 recheck deferred (vs-SF% noise) until ~+50 more.

**★ SEARCH LADDER IS THE PRODUCTIVE VEIN (2026-06-15) ★** Two clean wins this session: **singular ext +35** (v0.9.0) and **IIR +16** (v0.10.0). Selectivity (extend forced) + reduction (IIR) both gain; but AGGRESSIVE pruning/extension over-does it (LMP −96, double-ext −58, multi-cut −9) and ordering/margin tweaks are neutral (malus/corrhist/history-LMR/SPSA). 
**NEXT search levers (keep mining, prioritized):** (1) **LMR refinements** — reduce-less-in-PV, reduce-more-when-not-improving, reduce-less-for-killers (standard, +5-15 each, untried); (2) **SE-margin SPSA** — tune singular's margin(2·depth)/threshold(8)/reduced-depth via the tunable infra (expose them in tune.rs first); (3) **history malus + history-based pruning** combo (targeted late-quiet pruning by negative history — gentler than LMP). Net levers (deeper 2-layer, stronger data) remain the long-horizon big bets.
**M1 recheck deferred** — vs-SF% too noisy to resolve <~50 Elo; re-run after ~+50 more cumulative.

**IIR PASSED +15.6 (row 47, committed) — HEAD = v0.9.0 + IIR (bench 20137).** 2nd straight search win (singular +35, IIR +16). Tag v0.10.0 after this LMP + an LTC-confirm of the batch (singular already LTC'd; IIR+LMP need a batch LTC).

**SINGULAR DERIVATIVES EXHAUSTED:** singular ext **+35 (kept, v0.9.0)**, but double-ext −58 (row 45) and multi-cut −9 (row 46) both FAIL → singular is well-placed; pushing selectivity harder over-does it. Cheap search ladder now genuinely tapped (selectivity peaked at singular; ordering/margins/SPSA were neutral earlier).

**Double extensions FAILED −58.5 (row 45)** — naive +2 over-extends; HEAD stayed v0.9.0. **M1 recheck v0.9.0 = 4.75%/−520.9±55** (row M1, ~unchanged vs v0.8.0 within noise — dev signal is SELFPLAY SPRT, not the gap-dominated vs-SF%; re-check M1 per ~+50-100 cumulative Elo).

**★ KEY UNLOCK THIS SESSION: the SEARCH LADDER is productive again via SELECTIVITY, not ordering/margins ★** singular ext +35 (the big one); but margin/ordering tweaks (malus, corrhist, history-LMR, SPSA, double-ext) all neutral/negative. So prioritize SELECTIVITY-type features next:
  1. **Multi-cut** — if a reduced beta-window search finds ≥2-3 moves ≥ beta, prune the node (a separate reduced search, NOT the singular sbeta-window one). Selectivity, same family as singular.
  2. **SE-margin tune** — the singular margin (2·depth), threshold (depth≥8), reduced-depth ((depth−1)/2) are first-guesses; a careful sweep/SPSA could add +5-15 on the +35.
  3. Lower-EV (margin-type, likely neutral): improving heuristic, countermove history, internal iterative reductions.
  Net levers (deeper 2-layer, stronger data) still the long-horizon big bets; capacity/int8/buckets all tapped.

**Strength chain:** v0.7.0 (SF-net) → +i32-fix+LMR → v0.8.0 → +singular → **v0.9.0**. vs SF18 ~5% (−515ish), M1 (−400) ~115 Elo out. Reachable by accumulating search-selectivity Elo + eventually a net step.
**NEXT search levers (now that singular landed, search ladder reopened):** more extensions/selectivity (multi-cut from the exclusion search; double/negative extensions; SE-margin SPSA via the existing tunable infra), then revisit history-LMR/corrhist on the stronger base. Net levers (deeper 2-layer, stronger data) still deferred.

**INT8 = DEAD END for the king-bucket speed tax (2026-06-15).** Re-quantized the i16 4-bucket net to i8 ×127 in-engine (bullet `quantise::<i8>` exports 0 bytes — broken; convert at load instead). eval sane (73/+1293/−1157 ≈ i16). But **nps unchanged** (~533k vs i16 4-bucket ~541k; both ~4-5% under v0.8.0). → **the king-bucket tax is the refresh-on-bucket-crossing + bucket-indexing machinery, NOT feature-table size** (halving the table didn't help; i8→i16 widening may also kill the i16 auto-vec). int8 only useful for net SIZE. inference `nnue-kbi8.rs` kept. **So the king-bucket STC unlock = make cross-bucket refresh INCREMENTAL** (avoid the full rebuild when a king crosses a bucket) — a search/nnue change, the real remaining lever for king-buckets.

**(prior) SPSA (200 iters) FAILED −8.8** (row 42): drifted a near-optimal base the wrong way. **Search base is already well-tuned at STC.** Tunable infra kept (`src\tune.rs` UCI options lmrbase/lmrdiv/rfpmargin/futbase/futscale, advertised in `uci.rs`, driver `matches\spsa.py`) — a future SPSA would need far more games/iter + iters + a/c decay (low EV on a tuned base; don't repeat cheaply).

**★ M1 (~110 Elo) LEVER MAP after this session — cheap/medium STC levers EXHAUSTED ★**
  - Search ladder: log-LMR +9.5 (banked, v0.8.0); malus/corrhist/history-LMR neutral; SPSA −9 (base tuned).
  - Net capacity (king-buckets, all schemes): real eval (LTC +8) but ≤ ~9% feature-table cache speed tax at STC → net ≤0.
  - More SF data at plain-768: tapped (rows 29/30).
  **Remaining levers are all STRUCTURAL/bigger (next sessions):**
  1. **int8 feature weights** (TOP) — halve the feature-table cache footprint → removes the king-bucket speed tax → the validated capacity eval (LTC+8) becomes STC-positive AND reopens scaling more/bigger data (the project's biggest historical lever, SF data +477). Retrain with int8 l0w quantise + accumulator handling + careful accuracy verify. Accuracy-risky → fresh careful session.
  2. **Stronger teacher data** — higher-node SF relabel or newer/more test80 months (pairs with int8 to actually use the added capacity).
  3. (low EV) heavier SPSA; singular extensions (untested, complex, +20-50 maybe).

**SESSION TOTAL (v0.7.0 → v0.8.0):** i64 −37 regression fixed; log-LMR +9.5 → v0.8.0; SF 4.0%/−552 → 5.0%/−511. Plus a large body of characterized negative knowledge (data tapped at 768, search micro-features + SPSA neutral/negative on a tuned base, king-buckets speed-tax-bound) and reusable infra (king-bucket train+inference pipeline, SPSA harness, UCI tunables). All committed/tagged/archived.

**★ KING-BUCKET LEVER EXHAUSTED — speed-tax-bound (2026-06-15) ★** All 3 schemes STC ≤0: non-mirror 4-bucket **−0.4 (LTC +8)** (rows 39+note), 2-bucket −14 (row 40), mirrored 4-bucket −11 (row 41). The input capacity gives REAL eval (the LTC +8 is genuine, validated inference) but it's ≤ the **~9% feature-table cache speed tax** (4.72MB l0w vs plain 768's 1.18MB) at STC, regardless of bucket count/scheme/mirror. More/better buckets do NOT clear it. Pipelines + nets all archived (`razor_net_kb*.rs`, `nnue-kb.rs`/`nnue-kbm.rs`, `razor-netkb*.exe`, `nets\razorkb*.nnue`).

**★ THE ACTUAL UNLOCK = FASTER NNUE INFERENCE (next session, top priority) ★** Cut the table-cache tax so the validated king-bucket eval (LTC +8) becomes STC-positive — AND it makes future bigger/data-hungry nets viable (the whole capacity→data path). Options, best first:
  1. **int8 feature weights** (l0w) instead of i16 → halves the table (4.72MB→2.36MB for 4-bucket; 1.18→0.59MB for plain) → much less L2/L3 pressure. Needs: retrain/re-quantise l0w to int8 (bullet `quantise::<i8>`), accumulator stays i16 (sum of int8*activations), verify accuracy (eval sanity + the i32-overflow guard). Accuracy-risky → do carefully fresh, full verification. This is THE structural lever.
  2. **SPSA** the search base (no fastchess SPSA → build a harness): LMR divisor/margins + revive neutrals (corrhist/history-LMR). +20-50, no eval-speed risk.
  3. **Stronger/more teacher data** — historically the biggest lever (SF data +477). Pairs with int8 (need cheap inference to use more capacity/data). Newer test80 months or higher-node relabel.
  NOTE: hand-AVX2 on the accumulator was already a DEAD END (2026-06-13, LLVM auto-vectorizes) — int8 (smaller footprint) is different from SIMD-widening and IS worth trying.

**Session (v0.7.0→v0.8.0) net:** i64 −37 regression fixed; log-LMR +9.5 → v0.8.0; SF 4.0%/−552 → 5.0%/−511. M1 ~110 Elo out. Everything else this session = characterized dead ends (data tapped at 768, search micro-features neutral, king-buckets speed-tax-bound) — all valuable negative knowledge. Committed/tagged/archived.
**If interrupted:** working tree = clean v0.8.0; mirror work in `nnue-kbm.rs`/`search-kb.rs` + checkpoints.

**★ KING-BUCKET LEVER (prior, non-mirrored) CHARACTERIZED (2026-06-15) ★** Real eval, speed-limited; bucket-count tuning did NOT unlock an STC win:
  - **4-bucket** (file-pair, razorkb): STC −0.4 (row 39), **LTC +8±24** (row 39 note). Capacity gain ≈ ~9% speed tax (4.72MB table).
  - **2-bucket** (file-half, razorkb2): STC **−14** (row 40) — WORSE; poor split + less capacity, speed tax gone (nps 790k) but eval lost more. Non-monotonic → bucket *layout* matters a lot.
  Both inferences validated (eval sane, perft exact, dbg acc-assert clean). Pipeline reusable (`razor_net_kb*.rs`, `nnue-kb.rs`+`search-kb.rs`, candidates `razor-netkb*.exe`).

**M1 (~110 Elo away) NOT reached this session via the explored levers.** Search ladder tapped (LMR +9.5 only win; malus/corrhist/history-LMR neutral). Net capacity = real eval but STC speed-limited. The genuine next levers (next session, bigger):
  1. **MIRRORED 4-bucket** (top EV) — horizontal mirror gives each of 4 banks ~2× the data → better-trained banks → more eval to beat the speed tax. Needs mirror inference in nnue.rs: per-perspective flip flag `f = (king_file>3)?7:0` applied as `feat_sq ^= ... ` (flip file bits), refresh also when the flip changes. Trainer: `ChessBucketsMirrored::new([usize;32])`. Higher bug surface (verify via eval sanity + acc-assert).
  2. **Faster 4-bucket inference** to kill the ~9% cache tax (the 4-bucket is neutral STC / +8 LTC — removing the tax tips it positive). Hard (memory-bound); maybe prefetch / smaller weight type.
  3. **SPSA** the search base (no fastchess SPSA → build a harness): LMR divisor + margins + revive neutrals (corrhist/history-LMR) as tuned params. +20-50.
  4. **Stronger/more teacher data** (the historically biggest lever: SF data was +477) — newer/more test80 months, or higher-node relabel.

**Session net result (v0.7.0→v0.8.0):** i64 −37 regression fixed (restored), log-LMR +9.5 → v0.8.0; SF 4.0%/−552 → 5.0%/−511. All else this session = characterized dead ends (valuable: data tapped at 768, search micro-features neutral, king-buckets speed-limited). Everything committed/archived.

**★ KING-BUCKET EXPERIMENT COMPLETE (2026-06-15) — validated lever, speed-tax-limited ★** netkb (768x4 king-buckets + factoriser, 4 SF months, 320 sb) = **STC-neutral −0.4** (row 39), **LTC +8.1±24** (mild). The 4× input capacity DOES absorb the SF data tapped at plain-768 (real eval, far better than plain-1024's −54) but the ~9% speed tax (4.72MB feature table cache pressure + king-bucket refreshes) cancels the STC gain. **Pipeline fully built + validated** (inference `matches\candidates\nnue-kb.rs`+`search-kb.rs`, net `nets\razorkb.nnue`, candidate `razor-netkb.exe`, trainer `examples\razor_net_kb.rs`) — reusable.

**IMMEDIATE NEXT EXPERIMENTS (next session) — convert king-buckets' real eval into a clean win for M1 (~110 Elo away):**
  1. **2-bucket variant (best EV).** Halve the table (2.36MB → ~half the cache tax) keeping most capacity (first buckets give most gain). Edit `razor_net_kb.rs`: layout `(sq&7)>>2` (file-half, a-d/e-h), `assert num_buckets==2`, net_id `razorkb2`. Inference: `nnue-kb.rs` set `NUM_BUCKETS=2` + `king_bucket=(sq&7)>>2`. Train (~1.5hr) → build → SPRT vs v0.8.0. Likely tips STC-positive (less tax, similar gain).
  2. **Faster bucketed inference** (keep 4 buckets): the full-refresh-on-king-bucket-crossing is a minor tax component — could do incremental cross-bucket update; but the dominant cost is 4.72MB cache pressure (inherent). Lower ceiling.
  3. **SPSA** the search base (LMR divisor 2.25, RFP/NMP/futility margins, + revive the deferred neutrals corrhist/history-LMR as tunable) — fastchess has NO native SPSA, needs a harness. Medium effort, +20-50.
  4. **netkb as §7-benchmark net**: it's LTC-mild-positive; could use for the 60+0.6 SF benchmark while v0.8.0 stays the STC line. Messy (two nets); only if 2-bucket fails.

**This session's net result: v0.8.0** (i64 fix +37 restored, log-LMR +9.5; SF 4.0%→5.0%). Search ladder + net-capacity both at break-even past this point; the win needs cutting the king-bucket speed tax or SPSA. All candidates/sources archived in `matches\candidates\`.

**kb INFERENCE STAGED (compiles, backed up, NOT in HEAD):** `matches\candidates\nnue-kb.rs` + `search-kb.rs` = bucketed inference: `feature_weights[768*NUM_BUCKETS]`, `Accumulator{wbucket,bbucket}` (w bucketed by white-king sq, b by black-king), king-bucket-crossing → full refresh from child (apply() takes `child`), `king_bucket(sq)=(sq&7)>>1` MUST match trainer. HEAD reverted to v0.8.0 (bench 20249) for safety.
**✓ kb INFERENCE VALIDATED (2026-06-15, on sb-80 ckpt):** eval sanity (startpos +73, up-Q +1398, down-Q −1276 symmetric), perftsuite exact, release-dbg acc-assert CLEAN across castling + walking-king searches (depths 12-20, kings crossing file-pair buckets) — the bucketed incremental update + refresh-on-bucket-crossing matches from-scratch refresh. Indexing matches the trainer. **nnue-kb.rs backup is final-form** (kb inference + include_bytes razorkb.nnue). search-kb.rs = v0.8.0 search + apply(child) param.
**NEXT (on training completion, sb-320):** copy `checkpoints_razorkb\razorkb-320\quantised.bin`→`nets\razorkb.nnue`; copy `nnue-kb.rs`→`src\nnue.rs` + `search-kb.rs`→`src\search.rs` (touch mtime!); build; quick eval-sanity+bench; freeze `matches\candidates\razor-netkb.exe`; revert HEAD (git checkout) → v0.8.0 (bench 20249); SPRT `razor-netkb` vs `masters\razor-v0.8.0.exe` [0,10]. If pass → v0.9.0 (this is the M1 push); LTC + M1 recheck. If garbage/neutral: the data may still be split too thin across 4 buckets — try fewer buckets or mirrored (more data/bucket).
**If interrupted:** working tree = clean v0.8.0; kb work is in `nnue-kb.rs`/`search-kb.rs` + the training checkpoints.

**IMMEDIATE NEXT EXPERIMENT (next session) — two big levers for the ~110 Elo to M1:**
  1. **SPSA tuning (medium effort, ~+20-50 cumulative, brief §5).** The search base is fresh and untuned. Tune via SPSA on selfplay Elo: LMR divisor (2.25) + base (0.75), RFP margin (80/depth), NMP reduction (3), futility margins (80+120*d), aspiration delta (25), and **revive the deferred neutrals as tunable** — corrhist constants (`search-corrhist2.rs`: CORR_GRAIN/MAX/weight) and history-LMR divisor (`search-lmrhist.rs`: 150k). fastchess has an SPSA mode; or wire a simple SPSA harness. Expose params as UCI options first. This is the highest EV-per-risk next step.
  2. **Output-bucket net arch (high effort, potentially +50-150 — reopens the data lever).** SF-data quantity is tapped at 768 width (rows 29/30) and 1024 lost on speed (netsf2 −54.6). Output buckets (bucket the `→1` layer by piece count, ~8 buckets) let a bigger/better net evaluate at ~768 speed → can absorb the 4-month data without the speed tax. bullet supports output buckets (`examples\progression\2_output_buckets.rs`). Needs: net arch change + nnue.rs inference (select bucket by popcount) + retrain. The proven big driver was SF-data quality (+477); this re-enables scaling it.

**Search-ladder status: at diminishing returns on the current base.** This session: log-LMR +9.5 KEPT (only win). Neutral/deferred (sources in `matches\candidates\search-*.rs`, revive via SPSA): history malus (row 33), corrhist (35/36), history-LMR (37). Untried cheap features if wanted: improving heuristic, SEE-in-move-ordering, singular extensions (risky), countermove ordering. Likely also neutral without tuning — SPSA the base first.

**KEY SESSION-3 LESSONS (don't relearn):**
  - **bench-neutral ≠ Elo-neutral.** The i64 acc (1024 hardening) changed no node count but cost −37 Elo at STC (slower). SPRT ANY speed-affecting change. Control: build-vs-build at identical source isolates build/speed regressions.
  - **Copy-Item preserves source mtime → cargo skips rebuild.** Always `(Get-Item file).LastWriteTime=[DateTime]::Now` after copying a source file before `cargo build`. (Edit-tool changes get fresh mtime, so they're safe.)
  - **SF-data lever tapped at 768** — more data at fixed width HURTS (capacity-bound). Need more capacity (output buckets) to use it.
  - **i32 acc is the line at 768** (compile-time assert guards >768; restore i64 if a ≥1024 net is revived).
  - Feature SPRTs must use `masters\razor-v0.8.0.exe` (= current line) as the base, NOT older masters.

**Search-ladder summary (this session, fair i32 base, all SPRT'd):** log-LMR **+9.5 KEPT** (row 34, new base). NEUTRAL/deferred: history malus (−7→row33), corrhist a1 −14.6 / a2 +1.5 (rows 35/36), history-LMR +1.2 (row 37). Neutrals' sources kept for SPSA (`search-{corrhist2,lmrhist,malus}.rs`). **Ladder at diminishing returns at this base** — next bigger levers: SPSA-tune (LMR divisor 2.25, corrhist constants, history-LMR divisor) on the style/win objective, OR bigger-but-fast net arch (output buckets / king-buckets) to reopen the data lever.

**SEARCH-LADDER RESULTS this session (post-i64-fix, all fair i32):** log-LMR **KEPT ~+9.5** (row 34, operator-keep at elo1 boundary, new base). history malus NEUTRAL ~−7 (row 33, not kept). conthist v1 deferred (clamp+576KB-table tax → proper redo: multi-ply + malus + rescaled bands). **NOTE for next base SPRTs: freeze current HEAD (bench 20249) as `matches\masters\razor-lmr.exe` or compare new candidates vs it, NOT vs the old i32 v0.7.0 master** (which lacks LMR → +9.5 confound).

**★ SYSTEMATIC BUG FOUND + FIXED (2026-06-14, commit 84ff834) ★** HEAD carried a post-v0.7.0 **i64 eval accumulation** (1024-overflow hardening, "bench-neutral" so never SPRT'd) that was ~8% slower than the i32 archived master → **−37 Elo silent regression** (control: fresh-i64 vs master-i32 = 44.75%). REVERTED to i32 (bit-identical eval at 768, verified 71/−173/237 == master; compile-time assert guards >768). **This restored ~37 Elo and INVALIDATED rows 29-32** (all i64-candidate vs i32-master). Re-testing features fairly now. **Build gotcha:** Copy-Item preserves source mtime → cargo skips rebuild; always `(Get-Item file).LastWriteTime=[DateTime]::Now` before `cargo build` when staging via copy. (Original session candidates were built via Edit = fresh mtime = correct; only a transient malus2 copy hit this.)

**(superseded) earlier investigation block:** THREE search/net changes this session all FAILED at ~−35 to −55 (netsf3 −35.5, netsf3b −57.6, conthist −50.5, malus −54.9). history malus is near-universally +Elo with NO speed tax → −55 is implausible as a real effect. **Hypothesis: the archived `masters\razor-v0.7.0.exe` (i32 eval accumulation, tagged commit 8371630) is ~8% FASTER than anything I build now**, because HEAD has the post-tag **i64 eval accumulation** (nnue.rs L138-146, added for 1024 overflow, "bench-neutral" = same nodes 22141 but slower arithmetic, bit-identical eval at 768). So every fresh candidate = i64 (slow) tested vs master = i32 (fast) → systematic ~handicap that could explain all of this session's losses. Measured nps: fresh-HEAD ~567k vs master ~615k (~8%). **RUNNING control match** `bzfgyszxk`: `candidates\razor-v070-fresh.exe` (i64) vs `masters\razor-v0.7.0.exe` (i32), 400 games STC, identical source otherwise → must be ~50% if builds match; if fresh loses, the handicap is confirmed and this session's 4 SPRTs are INVALID (re-run after fixing). Results `matches\control-fresh-vs-master-results.txt`. **FIX if confirmed:** make accumulation i32 at HIDDEN≤768 (1024 is dead anyway), rebuild master baseline, re-SPRT conthist+malus fairly.

(history-malus SPRT done: FAIL −54.9, RESULTS row 32. Candidate `razor-malus.exe` + source `search-malus.rs` retained — may actually be neutral/+ if the i64 handicap is confirmed.)

**conthist v1 FAILED −50.5 (RESULTS row 31).** Cause diagnosed: (a) 576KB cont_hist table thrashed cache → ~6% nps tax (bench 578k vs 615k), (b) `.min(890_000)` clamp collapsed strong counter-moves + high-butterfly quiets to ties → only 0.8% node reduction (too weak to pay the tax). Candidate `razor-conthist.exe` + source `search-conthist.rs` kept for a proper later redo (multi-ply + malus + rescaled bands). Pivoted to the cheaper, no-table **history malus** first (isolate the free win, then re-add conthist on top if it helps).

**★ SF-DATA LEVER CLOSED AT 768 (2026-06-14) ★** Both netsf3 (100 sb, −35.5) and netsf3b (320 sb matched-exposure, −57.6) on 4 SF months LOST to v0.7.0's 768-on-1-month. **1 month already saturates 768's capacity; more data at the same width degrades STC strength** (more training → worse, not better). Wider absorbs data but 1024 lost on speed tax (netsf2 −54.6). To use more data needs a bigger-AND-fast arch (output buckets / deeper-narrow / king-buckets) — deferred. **NEW PRIMARY LEVER = SEARCH LADDER** (conthist, SEE-move-ordering, singular ext, corrhist, history malus, improving — all Elo with NO eval-speed/STC tax). Also revisit arch experiments later. razorsf3b training was IO-BOUND (GPU ~11%; loader couldn't feed it from cold 40GB binpacks — future: faster storage / pre-filtered compact binpack). HEAD = v0.7.0 (bench 22141).

**netsf3 (100 sb / 4 months) FAILED −35.5±18** (RESULTS row 29, 618 games, 44.9%, H0 −2.99). 768-on-4-months LOST to v0.7.0's 768-on-1-month. **Cause = UNDERTRAINING, not data saturation:** netsf3 ran only 100 superbatches (1.25× v0.7.0's 80) over 4× the data → ~8-17× per-position exposure vs v0.7.0's ~27-53×. Net build was clean (eval +76/+1977/−1853, perft exact, acc-assert OK) — pure data/schedule effect. Fix = razorsf3b above (320 sb = matched exposure). **If razorsf3b also fails/flat → SF data genuinely tapped at 768 width → pivot to SEARCH LADDER.** Candidate `razor-netsf3.exe` (bench 28266) retained but superseded.

**SEARCH LADDER STARTED concurrently (training window, GPU-bound = free CPU): CONTINUATION HISTORY built + frozen, awaiting SPRT.** Candidate `matches\candidates\razor-conthist.exe` = v0.7.0 net + counter-move history in search.rs (cont_hist `[prev pt][prev to][cur pt][cur to]`, bonus-only mirroring butterfly, clamped <killer band, None at root/after-null). VERIFIED: builds clean, **bench 21973 (vs v0.7.0 22141 = ~0.8% fewer nodes/depth = better ordering)**, perftsuite exact, release-dbg searches d11-12 no acc-divergence/panic. **HEAD reverted to pure v0.7.0 (bench 22141)** — conthist source backed up at `matches\candidates\search-conthist.rs`. SPRT `matches\sprt_conthist.ps1` vs v0.7.0 [0,10] when box clear. If pass: copy search-conthist.rs → src\search.rs, commit, it joins the v0.7.0 line (stacks with any net). Two SPRTs now queued (one at a time, box to itself): conthist + netsf3b. gen4 self-play parked at 112M (Phase-4 style data, not strength). 4 months of SF data on disk: `data\sf\test80-2024-{01-jan,02-feb,03-mar,04-apr}.binpack` (~40GB).

**IMMEDIATE NEXT EXPERIMENT (highest value):** train a **768-wide net on all 4 SF months** (same speed as v0.7.0, 4× the data). The 1024-wide attempt (netsf2) just FAILED (−54.6 Elo) — pure speed tax (timed out 75× vs 46× at STC); going wider was wrong. 768-on-4-months isolates the data gain without the speed penalty. Steps:
  1. Make `tools\bullet\examples\razor_net_sf3.rs` = copy `razor_net_sf2.rs` but `HIDDEN_SIZE=768`, `net_id="razorsf3"`, `output_directory="checkpoints_razorsf3"` (keep the 4-month `new_concat_multiple` loader). Register `[[example]] razor_net_sf3` in `tools\bullet\crates\bullet_lib\Cargo.toml`. Train: `cmd /c "cargo run --release --example razor_net_sf3 --features cuda > H:\RazorBot\logs\razorsf3-train.log 2>&1"` from `tools\bullet` (~50min).
  2. When `Saved [razorsf3-80]` (or -100): copy `checkpoints_razorsf3\razorsf3-*\quantised.bin` → `razor\nets\razorsf3.nnue` (expect ~1.18MB, 768). `src\nnue.rs` HIDDEN=768 (already) + `include_bytes ../nets/razorsf3.nnue`; build; VERIFY eval sanity (startpos ~0, up-Q FEN `rnb1kbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1` large +), perftsuite EXACT, release-dbg Kiwipete (`r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1`) depth-10 (no 'diverged'). Copy exe → `matches\candidates\razor-netsf3.exe`. Revert `src\nnue.rs` include_bytes to `razorsf.nnue` + rebuild (HEAD stays v0.7.0).
  3. SPRT `razor-netsf3` vs `masters\razor-v0.7.0.exe`, STC 8+0.08 [0,10], detached → `matches\sprt-netsf3-results.txt`. (Pattern: copy any `matches\sprt_*.ps1`.)
  4. If pass → v0.8.0 (nnue.rs include_bytes razorsf3, tag, master copy), LTC confirm vs v0.7.0, M1 recheck vs SF18.
  5. If flat: SF data quantity is tapped at 768. Then either (a) go back to the SEARCH ladder (continuation history, SEE in move ordering, singular extensions, corrhist — Elo with no eval-speed cost), or (b) try output-buckets / a deeper-but-fast net arch.

**HARD-WON RULES (don't relearn these):**
- **SPRT discipline:** every change SPRT'd vs current master [0,10] early / [0,5] mid. Freeze candidate binary to `matches\candidates\` BEFORE testing (never test `target\release` — a rebuild contaminates it). One queue instance at a time. Kill orphan `razor-*` engines after a match or the next test hangs on the pipe.
- **NNUE net swap = 3 edits:** `nnue.rs` HIDDEN const + `include_bytes` path; the embedded net file must exist (`nets\*.nnue`, force-add — `*.nnue` is gitignored but `!nets/*.nnue` un-ignores). After building a candidate, ALWAYS revert nnue.rs to the current master net so HEAD stays the validated line.
- **Verify every net build:** eval sanity (startpos near 0; up-queen large positive; down-queen large negative) + `perftsuite` exact + a `release-dbg` search to trip the incremental-accumulator `debug_assert` (proves the accumulator layout is right at this HIDDEN width). The i64 output accumulation (committed) prevents overflow at ≥1024.
- **NNUE speed tax is real:** bigger/slower nets lose at STC even with better eval (from-scratch acc: −32; 1024: −54). M1 is measured at STC 1t, so STC strength is what counts. Keep nets fast (768 is the sweet spot so far). The eval-vs-speed split shows as flat/neg STC but strong LTC.
- **Net-gen findings:** PSQT→NNUE teacher = +51; NNUE self-play loop +209 then plateaus (~2600, can't beat own teacher); **SF public data (~3500 teacher) = +477, the big lever.** More SF *quantity* (1→4 months) expected modest; SF *quality* was the jump. Self-play gen-loop is demoted to Phase-4 style data.
- **Env quirks:** no WSL/tmux (long jobs = detached `Start-Process powershell -File ...`); PS5.1 pipe→exe adds UTF-16 BOM (use argv or BOM-stripped input; uci.rs strips it); fastchess `-each` needs `option.Threads=N` not `threads=N`; my tally.py keys on engine name "new" so it misfires when names differ (read fastchess's own Elo line instead). DGX Spark `ssh liam@169.254.142.130` (aarch64, for overflow datagen — used once for batch-2; no CUDA toolkit there).
- **SF data:** `bullet-utils convert --from text` wants WHITE-relative score+result (datagen now emits white-rel). SF binpacks load via `SfBinpackLoader::new_concat_multiple(&[paths], 1024, 6, filter)` — filter = ply≥16, !in_check, |score|≤10000, quiet best move. Source: HF `linrock/test80-2024/resolve/main/test80-2024-MM-mon-2tb7p.min-v2.v6.binpack.zst` (curl -L, zstd -d).
- **Ledger:** every test → row in `RESULTS.md` (28 rows). Versions v0.2.0–v0.7.0 tagged. `matches\masters\razor-v*.exe` archived.
- **Adversarial reviews over-confirm:** the engine-review workflow flagged 7 "bugs"; 4 were false positives (standard PVS/NMP patterns). ALWAYS read the actual code before applying a review fix.

**Tool-call note for the assistant:** emit each `<invoke>` with NO leading word. (Prior session leaked a stray token before tool tags — that's the "context rot" prompting this handover. Just be clean.)

---
## Current Status

- **Date:** 2026-06-13 (session 2 cont.)
- **Phase:** 3 — NNUE. **v0.5.0 tagged: NNUE eval validated (+50.9 Elo vs v0.4.0 PSQT).** M1 milestone check vs SF18 running.
- **Engine version:** Razor 0.5.0 (tag `v0.5.0`, commit 9ecf34e). NNUE `(768→512)x2→1` razor1 net, incremental dual-perspective accumulator, 3.97M nps. Archived `matches\masters\razor-v0.5.0.exe`. PSQT eval retained as `UseNNUE=false` fallback.
- **Strength chain:** v0.2.0 → +1290 (search) → v0.3.0 → +123 LTC → v0.4.0 → +51 NNUE → v0.5.0 → **+209 net2 (gen2 NNUE-labeled)** → v0.6.0. The net-generation loop is the biggest lever now.
- **v0.6.0 (tag, commit 34fcb7a):** net2, same `(768→512)x2→1` arch as net1 but trained on NNUE-labeled data. bench 26,242. `masters\razor-v0.6.0.exe`. LTC confirm vs v0.5.0 running.

## M1 RECHECK (2026-06-14): gap HALVED, M1 not yet met

**Razor v0.7.0 = 4.0% vs SF18** (3W-26D-371L, 400 games, −552 Elo). vs v0.5.0's 0.38%/−970. Closed ~418 Elo of the real gap; first 3 wins vs full-strength Stockfish. **M1 (≥10%, ~−400 Elo) still ~150 Elo away.** Honest: huge progress, milestone not reached. The SF-data lever clearly works — more of it should close the rest.

## v0.8.0 PLAN — more SF data + bigger net (toward M1)

- Download 2-3 more test80-2024 months (feb/mar/apr v6) from HF `linrock/test80-2024/resolve/main/`. Combine for a multi-month training set.
- **Multi-binpack training:** SfBinpackLoader takes ONE path — to use several, either (a) concatenate the decompressed binpacks (SF binpack = concatenated games; test if `cmd /c copy /b a+b out` produces a valid binpack — verify by training a few batches), or (b) check bullet for a multi-file SF loader / train sequentially across files. Resolve at train time.
- **Bigger net: 1024-wide** (nnue.rs HIDDEN=1024 + training HIDDEN_SIZE=1024) — now justified by abundant high-quality data (the net3 starvation lesson: only go wide with enough data; SF multi-month gives it). quantised.bin for 1024 ≈ 768*1024... wait input is 768: 768*1024*2 + 1024*2 + 2048*2 + pad ≈ 1.57MB.
- **netsf2 (1024-wide, 4-month SF data) BUILT + SPRT RUNNING vs v0.7.0** (2026-06-14). Net `nets\razorsf2.nnue` (1.58MB, 1024 layout verified). Eval sane (startpos +61, up-Q +1706, down-Q −1606 — i64 fix held, no overflow), perft exact, 1024 incremental-acc assert clean. SPRT `matches\sprt_netsf2.ps1` [0,10] STC. Watch STC-vs-LTC (1024 slower than 768 → possible speed tax like the from-scratch NNUE).
- **Adversarial code review done (workflow w3mnnnh33, 18 agents):** 7/12 findings "confirmed" but I rejected 4 as verifier false-positives (LMR re-search=standard PVS, NMP return-beta=intentional guard, movegen caps-in-check=unreachable, SEE-evasion-pruning=unsound). Applied 4 real fixes: **i64 eval accumulation (overflow hardening for 1024 — the key one)**, SEE promo on_square, from_fen ep canonicalization, arch comment. All bench-neutral at 768 (22141 unchanged). Deferred: make_null halfmove increment (minor, separate test).
- Train → SPRT vs v0.7.0 → if pass, v0.8.0 → LTC → M1 recheck again. Iterate SF months + net size toward M1.
- Also worth: re-run the §7 UHO benchmark (60+0.6) on v0.7.0 sometime — it's far stronger now; the style metrics will mean more. Deferred until closer to M-ladder targets.

## BREAKTHROUGH: SF-data net = v0.7.0 (+477.6 Elo, 2026-06-14)

**netsf SPRT vs v0.6.0: +477.6±98 Elo, 94% (143-4-11).** Biggest gain in the project. SF/Leela labels (~3500 teacher) on ONE month of test80-2024 data crushed the self-play net. The self-play loop was capped at ~2600 (can't beat its own teacher); SF data smashed through it. **Tagged v0.7.0** (commit 8371630), bench 22141, 768-wide, net `nets\razorsf.nnue`. LTC confirm vs v0.6.0 running; M1 recheck vs SF18 next.
- **Strength chain:** ... v0.6.0 → **+477 SF-data → v0.7.0**. Estimated jump from ~2600 to ~3000+ (M1 recheck will measure the real SF gap, was −970 at v0.5.0).
- **This reframes the roadmap:** SF public data is THE lever, not self-play iteration. Next: (a) more test80 months (combined multi-month set → bigger/better), (b) SF+selfplay mix (brief §5.4), (c) bigger net now that data is abundant+high-quality (768→1024, king-buckets), (d) re-examine M-ladder — v0.7.0 may approach M1 (≥10% vs SF).
- self-play gen4 (112M) and the gen-loop are now LOWER priority — kept for the Phase 4 style/aggression data (sharp UHO positions), not raw strength.

## (history) NEW LEVER: Stockfish public training data

Self-play distillation plateaus at the teacher's strength (~2600). To pass it we need a stronger teacher — SF's public NNUE data is labeled by deep SF/Leela (~3500). Brief sanctions this (§2 teacher, §5.2 distill, §5.4 keep SF-labeled data as mix). Original-code rule = engine, not labels.
- **Source:** robotmoon.com/nnue-training-data → HuggingFace `linrock/test80-2024` (282GB total). Grabbing ONE monthly binpack: `test80-2024-01-jan-2tb7p.min-v2.v6.binpack.zst` (7.7GB compressed, ~300-400M positions, SF/Leela-labeled, syzygy-rescored). HF resolve URL → xethub CDN, curl -L.
- **DATA READY:** `data\sf\test80-2024-01-jan.binpack` (8.55 GB decompressed, ~300M+ SF/Leela-labeled positions).
- **SF-768 net TRAINING** (2026-06-14, resumed after a user resource-pause): razorsf, 80 superbatches, 3.04M pos/s (SfBinpackLoader streams+filters live), ~45 min. Log `logs\razorsf-train.log`. → SPRT vs v0.6.0.
- **gen4 HELD at 112.4M/250M** (user paused, then resumed into the SF experiment instead). Decision: SF data is higher-value than finishing the 768-self-play-data-starvation test — if SF-768 wins big, gen4's experiment is moot. gen4 shards retained; resume to 250M only if the SF path disappoints.
- **Loader wired:** `examples\razor_net_sf.rs` (768 arch, `SfBinpackLoader` + standard SF filter: ply≥16, not in check, |score|≤10000, quiet best move). Compiles. 80 superbatches (SF data warrants more passes). HEAD-comparable arch to net4 so SF-768 vs selfplay-768 is clean.
- **Plan:** train SF-data net → SPRT vs v0.6.0. Expect a LARGE jump (3500 teacher vs our 2600). If big: this becomes the strength engine; self-play loop demotes to fine-tuning/style (Phase 4). The mix (SF + self-play, brief §5.4) is the follow-up experiment.
- **Caveat:** SF data = objective play → pushes Razor NEUTRAL, not aggressive. Fine — Prime Directive #1 is strength first; aggression (Phase 4) layers on top of a strong net.

## Current: gen4 datagen RUNNING (250M, for a data-matched 768 net)

- **gen4 RUNNING** (launched ~12:10 2026-06-13, v0.6.0/net2-labeled, 28 workers, **250M target**, `data\gen4\`, SeedBase 4000). ETA ~8hr at NNUE datagen speed. HEAD verified net2/512 (bench 26242) before launch.
- **net4 plan:** `(768→768)x2→1` on gen4's 250M (vs net3's 768-on-100M that starved). Maybe also bump superbatches if 250M warrants. Then SPRT vs v0.6.0. This isolates "was 768 just data-starved?" — if net4 passes, yes; if flat, width is a dead end and the lever is depth or more data.
- **Disk watch:** gen1-4 will total ~84GB; H: has 154GB free. User is ordering an HDD for the eventual 1TB need (1-3B positions). For now: could reclaim ~25GB by deleting gen1/gen2/gen3 text shards (already converted to .bin; reproducible from seed) — NOT done yet, no need.

## (history) Net-gen loop iteration 2: net3 (768) TRAINING

- **gen3 DONE** (104.7M positions, v0.6.0-labeled) → `data\gen3.bin` (100,002,013 positions).
- **net3 TRAINING** at 5.05M pos/s on 4070 Ti — `(768→768)x2→1` (wider than net2's 512), `checkpoints_razor3\`, log `logs\razor3-train.log`. ~13 min.
- **net3 has TWO changes vs v0.6.0:** wider hidden (512→768) AND better labels (gen3 by v0.6.0 vs gen2 by v0.5.0). If it passes big, both helped; if flat/small, next session run a 512-on-gen3 control to separate width from data.
- **768 build steps (after training):** copy quantised.bin→nets\razor3.nnue; src\nnue.rs HIDDEN 512→768 + include_bytes razor3.nnue; rebuild; VERIFY eval sanity + perft exact + release-dbg incremental-acc assert (768 layout). The nnue.rs load()/accumulator/eval all key off HIDDEN so they resize automatically. quantised.bin for 768: 768*768*2 + 768*2 + 1536*2 + pad ≈ 1.18 MB.
- **net3 plan: BIGGER net `(768→768)x2→1`** to absorb more from the now-excellent labels (net2 likely saturated 512 width on data quality). When gen3 done:
  1. convert→interleave → `data\gen3.bin`
  2. training\razor_net3.rs: HIDDEN_SIZE 768, net_id razor3, data gen3.bin
  3. **src\nnue.rs HIDDEN const 512→768** (load()/accumulator/eval all use HIDDEN so they auto-resize; quantised.bin grows: 768*768*2 + 768*2 + 1536*2 + pad). Rebuild, eval-sanity, perft-still-exact.
  4. SPRT net3(768, gen3) vs v0.6.0(512, gen2). Two variables change (arch + data) — if it passes big, great; if flat, run a 512-on-gen3 control to separate arch from data. 
- Speed note: 768 width ≈ 1.5× the accumulator work → slightly slower nps; net it should still win on eval. Watch the STC-vs-LTC split.

## net3 (768/gen3) — REJECTED ~−4 (2026-06-13). Lesson: don't go wide on thin data.

net3 SPRT vs v0.6.0: +7 at 941 games drifted to **−4.3 at 2116** → true value ~0/slightly negative. Killed. **768 width on 100M = data-starved** (2.25× the params of 512, underfit) AND slower eval → no net gain, slight loss. **HEAD reverted to net2/512** (bench 26242); v0.6.0 stays the line.
- Two compounding causes the +209 net2 jump won't repeat cheaply: (1) the teacher-upgrade per generation is shrinking (PSQT→NNUE was ~900 Elo; NNUE-gen→NNUE-gen is ~200), (2) a bigger net needs proportionally more data.
- **Current experiment: gen4 = 250M positions** (v0.6.0-labeled) to properly feed a wide net. Then net4 = 768 on 250M (data-matched this time). If 768-on-250M beats v0.6.0, width was just data-starved; if still flat, the lever is net DEPTH (add a layer) or even-more-data, not width.

## KEY FINDING: the net-generation loop drives strength (2026-06-13)

net2 = net1's arch trained on data labeled by the v0.5.0 NNUE engine (vs net1's PSQT-labeled gen1). **+208.8 Elo at STC.** Mechanism: the teacher jumped from a ~1500 hand-eval to a ~2400 NNUE-engine-at-5000-nodes. Label quality dominates at this stage. **Loop:** vN labels genN+1 → train netN+1 → vN+1. Expect gen3 (labeled by v0.6.0 ~2600+) → net3 to gain again (diminishing as the teacher's own ceiling nears). This + bigger nets is the path; AVX2/micro-opt is not.

## MILESTONE M1 — NOT MET (honest, 2026-06-13)

**Razor v0.5.0 scored 0.38% vs SF18** (0W/397L/3D, 400 games, STC balanced book). Elo gap ≈ **−970**. M1 needs ≥10% (within ~400 Elo).
- Reality: SF18 single-thread STC ≈ 3400+ Elo; Razor v0.5.0 ≈ 2400-2500. The gap is ~900-1000 Elo — Razor is a competent club/expert-strength engine, SF18 is superhuman. M1 (let alone the M4 project goal of *beating* SF from UHO books) is a long, multi-net-generation road. No sugar-coating: this is the start of the climb, not the end.
- The +50.9 NNUE gain and the whole search ladder are real and verified — but "verified internal gains" and "absolute strength vs SF" are different axes. We've been measuring the former; M1 is the first measurement of the latter, and it says there's a mountain left.

## Phase 3 roadmap toward M1/M2 (the climb)

Ordered by expected Elo-per-effort:
1. ~~AVX2-vectorize NNUE~~ **DONE — DEAD END (2026-06-13).** Hand AVX2 intrinsics gave no speedup; rustc+LLVM already auto-vectorize the i16 loops under target-cpu=native (3.5M vs 3.7M nps). Reverted. The NNUE's 2.3× cost vs PSQT is the real work it does, not missed SIMD. **The speed lever is a smaller/faster net or a faster inference layout, not intrinsics.** Note: LTC confirm showed NNUE +148 at 40+0.4 (vs +51 STC) — at the §7 benchmark TC (60+0.6) the speed penalty matters even less, so the eval quality is what counts there.
2. **gen2 (NNUE-labeled) — DONE.** 102.5M positions, white-relative, converted → interleaved → `H:\RazorBot\data\gen2.bin` (100,002,520 positions, 2.98 GB). **net2 TRAINING** (razor2, same `(768→512)x2→1` arch as net1 to isolate data-quality effect) on 4070 Ti, 6.8M pos/s, log `logs\razor2-train.log`, → `checkpoints_razor2\razor2-40\`. Then: copy to `nets\razor2.nnue`, build net2 candidate, SPRT vs v0.5.0. net1 was gen1(PSQT-labeled); net2 is gen2(NNUE-labeled) — same arch, so the SPRT measures pure label-quality improvement.
3. **net2: bigger arch** — `(768→768)x2→1` or `1024`, king-buckets later. SPRT each.
4. **Net-generation loop:** v0.5.0 labels gen2 → net2 → v0.6.0 labels gen3 → net3 ... each generation a few SPRT'd net candidates. This is how real engines climb; expect +50-150 Elo per good generation early, diminishing.
5. **Search batch 3** (continuation history, singular extensions, corrhist, improving, capture history, lazy SMP) — interleave; each +5-30.
6. **SPSA** tune search + eval-blend params once stable.
- Realistic near-term: get NNUE speed up (AVX2) + 2-3 net generations → maybe close to M1 (≥10% vs SF) over many sessions. M2/M3/M4 are long-horizon.

- **Phase 2 history (done):** Search ladder batch 2 COMPLETE; v0.4.0 tagged (Gates G0 + G1 PASSED 2026-06-12)
- **Engine version:** Razor 0.4.0 (tag `v0.4.0`, commit 4fbb32c), bench 55,251. Archived `matches\masters\razor-v0.4.0.exe`. **LTC CONFIRMED: +123±33 Elo vs v0.3.0 at 40+0.4 (67%, 200 games)** — batch-2 gains hold at long TC.

### Batch 2 final (ladder features 10-18, all SPRT-gated)
| Feature | Result | Elo |
|---|---|---|
| Aspiration windows | PASS | +31.1 |
| SEE pruning (qsearch) | KEPT neutral (infra) | ~+5, unresolved 8400g |
| Reverse futility pruning | PASS | +47.7 |
| Futility pruning | KEPT operator | ~+5, straddles bounds |
| Check extensions | PASS | +26.8 |
| Late move pruning | **FAIL, reverted** | −43.5 |
| MoveOverhead 20ms default | REJECTED, default→0 | −4.8 |

Net batch-2 gain ≈ +110 confirmed (aspiration+rfp+checkext) plus two neutral-positive keeps. SEE retained as required infrastructure for batch-3 (capture-history ordering, SEE-based pruning guards). LMP needs gentler margins + better ordering before retry.
- **Engine renamed VENDETTA → Razor** (user request). Repo now `H:\RazorBot\razor`, binary `razor.exe`.
- **Engine version:** Razor 0.3.0 (tag `v0.3.0`) — bench signature 117,145. Binary archived as `matches\masters\razor-v0.3.0.exe`.
- **Ladder batch 1 COMPLETE — 8/8 SPRT passes** (full ledger in RESULTS.md): MVV-LVA +331, qsearch +371, TT +206, PVS +26, killers +54, history +21.5, NMP ~+164, LMR +116. Chain sum ≈ +1290 self-play Elo over v0.2.0 at STC.
- **LTC confirmation: PASSED** — v0.3.0 vs razor-tt at 40+0.4, 159W-35D-5L (88.7%) ≈ +358 Elo, vs ~+382 STC chain expectation for the same features. v0.3.0 tag validated per brief §5.
- **Estimated strength:** chain-relative only; absolute calibration gauntlet vs a rated engine is queued for next session.
- **Milestone ladder:** pre-M1

## Pinned Reference Engine

- **Stockfish 18** (official release sf_18, published 2026-01-31)
- Local binary: `H:\RazorBot\tools\stockfish\stockfish\stockfish-windows-x86-64-bmi2.exe` (NNUE nets embedded in binary)
- Local bench: 2,050,811 nodes, ~1.51M nps single-thread (13900K)
- Spark/aarch64: no official linux-aarch64 release asset. Options when needed: run the android armv8-dotprod static binary, or compile from source on the Spark (`make profile-build ARCH=armv8-dotprod`). Decide at datagen time.

## Tooling

| Tool | Version / Location |
|---|---|
| Rust (local) | 1.96.0 stable, x86_64-pc-windows-msvc |
| Rust (Spark) | stable via rustup, installed 2026-06-12 |
| git | 2.24.1.windows.2 (old but functional) |
| fastchess | v1.8.0-alpha (20260128) — `H:\RazorBot\tools\fastchess\fastchess-windows-x86-64\fastchess.exe` |
| Python (local) | 3.12.10 (+ paramiko) |
| bullet trainer | cloned at `H:\RazorBot\tools\bullet`; CUDA smoke test PASSED on 4070 Ti (sm_89), ~15M pos/s on tiny test net | 
| CUDA (local) | toolkit 12.6, driver 591.86 |

## Books

| Book | Use | Source |
|---|---|---|
| `H:\RazorBot\books\8moves_v3.pgn` (8.0 MB) | Balanced SPRT testing | github.com/official-stockfish/books |
| `H:\RazorBot\books\UHO_XXL_2022_+120_+149.pgn` (74.7 MB) | Unbalanced benchmark vs SF (§7) | github.com/official-stockfish/books |

## Syzygy

- 3-4-5 man COMPLETE: 290 files (.rtbw+.rtbz), 0.92 GB in `H:\RazorBot\syzygy\`
- Source: `tablebase.lichess.ovh/tables/standard/3-4-5-wdl/` + `3-4-5-dtz/` (note: no plain `3-4-5/` dir; sesse.net mirror rate-limits concurrent fetches — avoid)
- Resumable downloader: `H:\RazorBot\scripts_bootstrap\syzygy_dl.py`

## Hardware Inventory

**Local (primary):** i9-13900K (24c/32t, AVX2+BMI2, no AVX-512), RTX 4070 Ti 12 GB, Windows 11 Home.

**DGX Spark** (`liam@169.254.142.130`, hostname `SparkyPoo`):
- Ubuntu (kernel 6.17.0-1021-nvidia), aarch64, 20 cores (Cortex-X925/A725), 121 GiB RAM
- GPU: NVIDIA GB10, driver 580.159.03, unified memory (nvidia-smi reports memory.total N/A)
- **No CUDA toolkit (nvcc missing)** — install needed before any Spark training runs
- **Disk 89% full: only 103 GB free** — watch this before farming datagen there
- tmux 3.4, git, rsync, gcc, g++, make, cmake, python3 present; Rust installed via rustup (2026-06-12)
- Passwordless SSH works (ed25519 key installed). Host key accepted.

## Environment Deviations from Brief

1. **No tmux locally.** Windows host; WSL2 present but broken (virtualization disabled in firmware — needs BIOS change, not pursuing). Long local jobs instead run as detached processes (`Start-Process` with stdout/stderr redirected to `logs\`, PID recorded) via `scripts\run_detached.ps1`. Remote long jobs on the Spark DO use tmux.
2. **Scripts are PowerShell (`.ps1`), not `.sh`** — same roles as brief §3: `sprt.ps1`, `bench.ps1`, `datagen.ps1`, `vs_stockfish.ps1`.

## Gate G1 — PASSED 2026-06-12

- Perft suite exact: 20 positions (CPW standard 6 + 14 edge cases: ep pins/discovered checks, castling checks, promotions, stalemates), 8.22B nodes total, **497M nps bulk single-thread** (target was >150M).
- Incremental zobrist verified against from-scratch recomputation via debug-assertions build (release-dbg profile) over millions of makes.
- 1,000 ultra-fast selfplay games (tc=1+0.01, fastchess, balanced book): all 1,000 terminations "normal" — zero crashes, illegal moves, disconnects, or time losses (`H:\RazorBot\matches\g1-selfplay.pgn`).
- Architecture: copy-make, fully legal generation (check mask + pin restriction, occupancy-test ep), PEXT sliders (BMI2 intrinsic; portable fallback for aarch64), compile-time zobrist/leaper tables.
- `bench` = perft(5) placeholder signature until search lands.

## Gate G0 — PASSED 2026-06-12

- All tools run (fastchess, SF18 bench 1.51M nps, bullet CUDA training on 4070 Ti).
- 100-game SF-vs-SF smoke match via fastchess completed: 49.5% score, 83 draws, no crashes (`H:\RazorBot\matches\g0-smoke.pgn`). Total time 2:45 at 16 concurrency.
- Spark inventoried and journaled (see Hardware Inventory).

## Job Queue

- **RUNNING:** TT SPRT (fastchess, candidates\razor-tt.exe vs masters\razor-qsearch.exe); then detached queue `H:\RazorBot\matches\sprt_queue.ps1` (PID 56184 at launch) runs pvs→killers→history→nmp→lmr SPRTs sequentially. Results append to `H:\RazorBot\matches\sprt-queue-results.txt`.
- **RUNNING (split across machines, 2026-06-12 ~16:40):**
  - Local: `sprt_queue2.ps1` instance running seeprune → rfp → futility at [0,5]. NOTE: this instance still has all 6 tests — KILL IT after futility completes (Spark owns the rest).
  - Spark (`tmux session sprt`, `~/sprt/`): lmp → checkext → timemargin. aarch64 builds from candidate commits (futility f6d9cd1, lmp 1914a20, checkext f734aa6, timemargin 506242f), fastchess-linux-arm64, results in `~/sprt/results.txt`. Spark engine speed ~930k nps bench (software-PEXT fallback) — both sides equally affected, SPRT deltas valid.
  - Bench signatures: aspiration 115,563 → seeprune 70,038 → rfp 55,984 → futility 46,819 → lmp 21,418 → checkext 21,238 → timemargin 21,238 (identical to checkext — time-only change, correct).
- **Batch 2 implementation notes:** SEE swap-list bug caught by unit test before SPRT (speculative gains pushed without verifying a recapturer exists — RxR-defended scored +100 instead of 0; fixed in 1313baf and candidate re-frozen). Perft suite re-validated exact after batch.
- **QUEUED (next session):**
  1. Read queue-2 results → ledger rows → promote masters → tag v0.4.0 + LTC confirm if all pass.
  2. Calibration gauntlet vs a rated open-source engine (~2300-2700 CCRL) for an absolute strength anchor.
  3. Crash investigation: stderr-capture shim around release candidates during SPRT runs.
  4. Ladder batch 3: continuation history, singular extensions, correction history, improving heuristic, capture history/SEE ordering, lazy SMP.
  5. Phase 3 prep when ladder plateaus: v0 datagen harness + bullet training pipeline.
- **BLOCKED:** none

## Phase 3 — data pipeline COMPLETE, first net TRAINING (2026-06-13)

**gen1 dataset: 100,003,433 positions, training-ready.**
Pipeline (all verified end-to-end):
1. `razor datagen` → `H:\RazorBot\data\gen1\shard-{0..27}.txt` (stm-relative; gen1 made before the white-rel fix)
2. `training\convert_gen1.py` → `data\gen1_wr\` (white-relative — bullet requires white-rel score AND result; flip = negate score + `1-result` when black to move)
3. `bullet-utils convert --from text` → `data\gen1_bin\shard-*.bin` (bulletformat, 32 B/pos)
4. `bullet-utils interleave` → **`H:\RazorBot\data\gen1.bin`** (2.98 GB, 100,003,433 positions, the training file)
- WDL balance: ~23% W / 54% D / 23% L (healthy; engine is ~equal-strength selfplay so lots of draws).
- **DATAGEN SOURCE NOW EMITS WHITE-RELATIVE** (commit b2539f0) — gen2+ skip the convert_gen1.py step; feed bullet-utils convert directly.

**Training (running, launched ~00:30 2026-06-13):** `tools\bullet\examples\razor_net.rs` (versioned copy `razor\training\razor_net.rs`). Arch `(768→512)x2→1` SCReLU, AdamW, eval_scale 400, QA255 QB64, LinearWDL 0.5→0.8, StepLR 0.001 γ0.3 step15, 40 superbatches (≈40 epochs over 100M). **5.87M pos/s on 4070 Ti → ~17s/superbatch → ~11 min total.** Output nets in `tools\bullet\checkpoints_razor1\`, log `logs\razor1-train.log`.

### NNUE inference DONE + first net trained (2026-06-13 ~00:45)
- razor1 net trained: 40 superbatches, 10m11s, **final loss 0.034**, 4070 Ti. Net `nets/razor1.nnue` (quantised 789,568 B).
- `src/nnue.rs`: embedded net, perspective accumulators (512/side), int16 SCReLU, bullet-exact Chess768 indexing (derived + verified both stm colors). Eval dispatcher in eval.rs + `UseNNUE` UCI option (default true), PSQT kept as fallback (`evaluate_psqt`).
- **Eval sanity PASSED:** startpos +40, up-knight +695, up-queen +1832, down-queen −1763 (~symmetric). Bare-king endgames muted (+90 KQvK) — out-of-distribution (datagen adjudicates ±2500 before bare kings); search compensates. Perft still exact.
- **Speed: NNUE 1.74M nps vs PSQT 9.2M (5.3× slower)** — from-scratch accumulator refresh per node. **Incremental update (make/unmake) is the #1 optimization** — should recover most of the gap; SPRT separately as a pure speedup.
- NNUE bench = 32,890 (PSQT was 55,251). Commit 0a4d625.
- **SPRT RUNNING (detached):** razor-nnue1 vs razor-v0.4.0 (PSQT), STC [0,10], `matches\sprt_nnue1.ps1` → `sprt-nnue1-results.txt`. The decisive test — does the net's eval beat PSQT even at 5× slower? If yes, NNUE is validated and incremental update is pure upside → likely v0.5.0 + the path to M1.

### Earlier NNUE plan (now mostly done — kept for reference)
1. Collect trained net `checkpoints_razor1\razor1-40\` (quantised .bin: l0w/l0b i16 ×255, l1w i16 ×64, l1b i16 ×255*64).
2. Implement NNUE in engine: embed net bytes (`include_bytes!`), perspective accumulator (768→512 per side), int16 SCReLU, int32 output dot, dequantise by /(QA*QB) then *scale/QA... follow bullet's inference doc `tools\bullet\examples\simple.rs` bottom half + `docs\4-saved-networks.md`. Incremental accumulator update on make/unmake (the "efficiently updatable" part) — but a from-scratch refresh per eval is fine for v1, optimize later.
3. Gate behind eval switch; keep PSQT as fallback. AVX2 SIMD for the accumulator/output (target-cpu=native already on).
4. SPRT NNUE-eval vs v0.4.0 (PSQT). Expect a large jump. Bench signature WILL change (new eval) — re-baseline.
5. If it passes: tag v0.5.0, LTC confirm, then M1 check (≥10% vs SF18 STC balanced book).

## Phase 3 STARTED 2026-06-12 — datagen harness working

- `razor datagen <out.txt> <num_positions> [seed]` implemented (`src/datagen.rs`). Self-play from 4-8 random legal opening plies, each move searched at 5000 nodes, positions filtered (not in check, best move quiet, score not near-mate), labeled `<fen> | <stm_cp> | <wdl>`. Win-adjudication at ±2500cp×6 plies; 50-move/mate/stalemate handled. Deterministic SplitMix64 per seed.
- Search exposes `root_score` + `silent` flag for datagen (non-functional for normal play; bench unchanged — VERIFY 55,251 still holds after these edits before next release).
- Smoke: 2241 positions/16 games, **1921 pos/s single-thread**. Distribution healthy: WDL 644/679/918 (W/L/D balanced), scores −3194..+2702, mean ~3 (symmetric stm-relative), 662 unique. Output `H:\RazorBot\data\smoke.txt`.
- Parallel launcher: `matches\datagen_parallel.ps1` (N workers, distinct seeds, sharded output + meta json). ~28 workers → est. ~50k pos/s → 100M positions in ~35 min. Concatenate shards after.
- **RUNNING (launched 2026-06-12 ~23:00, overnight):** gen1 datagen — 28 workers, 3.57M positions each = 100M total, `H:\RazorBot\data\gen1\shard-{0..27}.txt`, meta in `datagen.meta.json`. Aggregate **~29,500 pos/s**, ETA ~56 min (done ~23:55). PIDs in meta json.
- **NEXT SESSION:** (a) confirm gen1 finished (~100M positions across 28 shards; concatenate or feed bullet directly); (b) convert text `fen | cp | wdl` → bullet training format — CHECK `tools\bullet\docs\3-data.md` for whether bullet ingests text or needs bulletformat/binpack (likely need a small converter: our format → bulletformat's `fen score wdl` is close; bullet may have a `convert` util in bullet-utils); (c) train first net `(768→512)x2→1` SCReLU on 4070 Ti (adapt `tools\bullet\examples\simple.rs`, eval_scale 400, ~40 superbatches); (d) implement NNUE inference (accumulator, int16, AVX2 dot products) in engine behind an eval switch — keep PSQT eval as fallback; (e) SPRT the net vs v0.4.0 (expect large gain). The bench signature WILL change when NNUE lands (different eval) — that's expected, re-baseline then.
- Spark role for Phase 3: overflow datagen (needs aarch64 razor build — already have via bundle) + large-dataset shuffle/interleave (128GB RAM). Watch Spark disk (was 89% full).

## NEXT MAJOR WORK — Phase 3: NNUE evaluation (the big Elo jump)

Search ladder is mature (v0.4.0). Hand-crafted material+PSQT eval is now the bottleneck. NNUE is worth +400-700 Elo — far more than any remaining search feature. Plan (brief §5 eval pipeline):
1. **Datagen harness:** selfplay from randomized book exits, label positions with pinned SF18 at fixed nodes (30-60k), output bullet binpack format. Split local cores + Spark (need SF aarch64 build on Spark — compile from source or use android armv8-dotprod binary, decide then). Target ~100M positions first, scale to 1-3B.
2. **First net:** bullet on 4070 Ti, `(768→512)x2→1` SCReLU perspective net, quantized. SPRT vs v0.4.0 like any patch.
3. **NNUE inference in engine:** efficiently-updatable accumulator, int16/int8, AVX2 SIMD (+NEON for Spark). Incremental update on make/unmake.
4. Iterate net generations; transition to self-generated data once strong.
Gate G2/G3: beat a ~3000-CCRL open-source engine, then ≥10% vs SF18 STC (= M1). Only then Phase 4 (aggression layer).

Batch-3 search features (smaller, can interleave or defer): continuation/countermove history, singular extensions, correction history, improving heuristic, capture history, lazy SMP (multithreading), retuned LMP. SPSA tuning of search params.

## Operational lessons (session 2)

1. **SPRT bounds vs effect size:** [0,10] can't resolve a true ~+5 feature (random-walks forever). [0,5] barely better for effects near the midpoint. For small features either accept a long test or reframe as non-regression [-5,0] (a positive feature passes fast) or make an honest operator-keep call with games logged. Did the latter for seeprune and futility.
2. **Queue hygiene:** only ONE `sprt_queue2.ps1` instance at a time. Before launching, check `Get-CimInstance Win32_Process -Filter "Name='powershell.exe'" | Where CommandLine -like '*sprt_queue2.ps1*' -and ProcessId -ne $PID`. Multiple relaunches during the crash hunt left several stale instances fighting over the box.
3. **Orphan engines block the queue:** killing fastchess leaves razor-*.exe children holding the script's captured pipe → the `&` call never returns → queue hangs. Kill `razor-*` children too, or the next test never starts.
4. **Cross-machine SPRT:** Spark (aarch64, software-PEXT, ~930k nps) ran lmp/checkext2/timemargin2 in parallel with local tests — roughly doubled throughput. Both engines on a given pair run the same hardware, so Elo deltas stay valid. Builds shipped via git bundle + native `cargo build` per candidate commit.

## CLOSED BUG — 0xc0000409 "crashes" were stdout pipe panics at match teardown

**Root cause (found 2026-06-12 ~15:10):** Rust `println!` panics when stdout's pipe is closed mid-write; with `panic = "abort"` that aborts with 0xc0000409 and a WER event. fastchess closes engine pipes at match end (and any forced `Stop-Process` of fastchess does it instantly) — an engine mid-`info`-print at that moment dies "crashing". Smoking gun: all four razor-seeprune events timestamped 13:54:43, the exact second the queue was force-killed; the razor-lmr (13:47:35), razor-tt (10:36), razor-killers (11:27) events all align with their matches' teardown windows.

**Why it was hard:** no search bug exists — position replays (movetime, real clocks, saturated box: 400+ games) and 1,160 assertion-binary games were all necessarily clean. Stderr shims broke fastchess startup (cmd spawn latency under concurrent spawns + Defender → uciok timeout), so no panic message ever got captured. Diagnosis came from event *timestamps*, not messages.

**Fix (commit at HEAD):** `send!` macro (writeln + ignore errors) replaces `println!` in all UCI/search output; panic hook also appends to `H:\RazorBot\logs\razor-panic.log` for future field diagnosis. Bench unchanged (21,238) — non-functional. Existing frozen candidates keep the old print code (the panic is benign to results: it fires only at teardown, after games are decided — all ledger rows stand). All future candidates inherit the fix.

**Process lessons:** (1) check event timestamps against own actions before hunting search bugs; (2) cmd.exe shims don't survive concurrent fastchess spawns; (3) WER LocalDumps registry approach works as a fallback (left enabled for razor-seeprune.exe).

## (resolved) original investigation notes

- Two Windows Application Error 1000 events, exception 0xc0000409 (Rust panic under panic=abort): razor-tt.exe at 10:36, razor-killers.exe at 11:27 (2026-06-12). Rate ≈ 1 crash per ~700-1000 games at 8+0.08. fastchess `-recover` scored crashes as losses, so SPRT passes stand (conservative).
- First crashing binaries are the TT-era ones, but code inspection of tt.rs/search.rs found no obvious panic site (all TT moves are equality-compared only, never executed; indexes bounded; mate scores fit i16).
- Repro attempt 1: release-dbg (assertions+overflow checks+unwind) selfplay, 300 games @ 2+0.02 — CLEAN.
- Repro attempt 2: 400 games @ 8+0.08 (the crash TC), release-dbg — CLEAN. 760 total assertion-binary games without repro; no further WER events as of 12:30.
- Status: UNREPRODUCED after 3 crashes (razor-tt 10:36, razor-killers 11:27, razor-lmr ~13:45 — all 0xc0000409, all TT-era binaries, all during matches).
- Repro attempt 3 (13:55-14:06): targeted stress, 8 parallel drivers, long unadjudicated games vs razor-lmr release binary — NEGATIVE in the normal-movetime regime (thousands of clean moves). Harness flaw ended the run: its occasional `go depth 30` probe makes the engine think for minutes-hours legitimately (no time bound at fixed depth) and the driver blocks on readline. If reusing `scripts_bootstrap\stress_crash.py`: cap probes at depth ~12 and add a reader timeout as a true hang detector.
- DEPLOYED: stderr-capture shims (`matches\shims\*.bat`, generated by `matches\make_shims.ps1`) wrap every candidate; engine stderr appends to `logs\<name>-stderr.log`. Batch-2 queue now runs through shims — the next natural crash logs its Rust panic message. Check those logs after every match batch.
- Observation from crash-game PGNs: clusters of `0.00/127 0.000s` annotations (instant max-depth iterations) in positions near the 50-move rule — degenerate regime is the leading suspect context, unproven.
- LESSON (12:08-12:15): running bughunt2 concurrently with the NMP SPRT caused 22 time-loss games (9 new / 13 base) from CPU contention. Reaffirms brief §8: SPRT runs get the box to themselves (16 games max), auxiliary matches wait. Timeouts were ~symmetric; NMP result direction unaffected.
- Queue script defect: `Select-Object -Last 12` clipped fastchess summaries when the timeout-stats block appeared (NMP test). Fixed to -Last 30 for future runs; NMP W-D-L recovered from PGN.

## Pipelined SPRT discipline note

Candidates were developed in a chain (each builds on the previous) while SPRTs ran behind: tt→pvs→killers→history→nmp→lmr. Each queue test compares a candidate against its immediate predecessor, so a PASS confirms that single feature. If any test FAILS, every later candidate in the chain contains the failed feature — re-test the next candidate against the last passing binary and consider reverting the failed feature before continuing.

## Next Steps (Phase 2 — search ladder, ONE feature at a time, each SPRT'd vs previous master)

DONE: v0 eval + minimal AB search = v0.2.0 baseline (RESULTS.md row 0).

Ladder remaining, roughly in brief §5 order (expected-value-adjusted: ordering/qsearch/TT are the big early wins):
1. MVV-LVA capture ordering
2. Quiescence search (+ SEE pruning as separate patch)
3. Transposition table (+ TT move ordering, TT cutoffs)
4. Aspiration windows; PVS
5. Killers → history → NMP → LMR → LMP → futility/RFP → cont-history → singular ext → check ext → corrhist → improving → lazy SMP → time mgmt refinement

SPRT protocol: STC 8+0.08, 1t, 16MB hash, 16 concurrency, bounds [0,10] early phase, alpha=beta=0.05. Use `scripts\sprt.ps1 -New <new.exe> -Base <master.exe> -Name <patch> -Elo0 0 -Elo1 10`. Every test gets a RESULTS.md row, pass or fail. Keep master binary copies in `H:\RazorBot\matches\masters\` (e.g. `razor-v0.2.0.exe`) so SPRT bases don't need rebuilds.

Periodic LTC 40+0.4 confirmation before release tags.

## SPRT Ledger

See `RESULTS.md`.

## Session Log

### 2026-06-12 — Session 2 (rename + search ladder)
- Renamed engine VENDETTA → Razor everywhere (repo dir, cargo package, UCI id, zobrist seed, journals, scripts, archived master). Bench unchanged.
- Ladder features implemented, committed, frozen as candidate binaries in `matches\candidates\`: MVV-LVA (9e1179a), qsearch (9e1179a), TT (a92e593), PVS (25bb8e4), killers (b81ff54), history (c767ac4), NMP (9cafef4), LMR (e3b8649).
- SPRT #1 MVV-LVA: **PASS** +331±71 Elo, 174 games, LLR 2.98.
- SPRT #2 qsearch: **PASS** +371±82 Elo, 170 games, LLR 2.96.
- TT SPRT running; detached queue handles the remaining five (see Job Queue).
- Process locks bit twice: stale engine/fastchess processes hold binaries (kill before rebuild/rename); shell CWD inside repo blocks dir rename.

### 2026-06-12 — Session 1 (Phase 0)
- Inventoried local machine + Spark; journal above.
- Updated Rust 1.63 → 1.96.0 locally; installed Rust on Spark.
- Set up passwordless SSH to Spark (ed25519, pushed via paramiko one-shot).
- Pinned Stockfish 18 (bmi2 build), verified bench.
- fastchess v1.8.0-alpha installed and verified.
- Books downloaded (8moves_v3, UHO_XXL_2022_+120_+149).
- Repo initialized (`H:\RazorBot\razor`, cargo skeleton).
- Known issue: piping text to engines from PowerShell 5.1 adds UTF-16 BOM — use argv commands (`stockfish.exe bench`) or write input files without BOM.
- fastchess syntax note: `-each threads=1` is rejected; use `option.Threads=1` (scripts fixed).
- Syzygy 3-4-5 complete (290 files, 0.92 GB, lichess mirror).
- bullet CUDA smoke test passed (test1 example, bundled batch.bf, RTX 4070 Ti sm_89).
- **Gate G0 PASSED.** Phase 1 begun: Cargo release profile (+release-dbg with assertions), target-cpu=native.
- Phase 1 written and validated in same session: types/bitboard/zobrist/position/movegen/perft/uci modules (commit b107111). Perft suite passed first try after build. 497M nps bulk.
- **Gate G1 PASSED** (see gate section above).
- Phase 2 baseline written same session: eval.rs (material+PSQT), search.rs (negamax AB + ID + soft/hard time mgmt + repetition/50-move/mate handling), UCI go parsing, real bench. Tagged **v0.2.0** (f896ef8).
- Bug found & fixed: a bench FEN was an illegal position (side not to move in check) → movegen "captured" the king → `king_sq` on empty bitboard → index-64 panic. Fix: replaced FEN + `from_fen` now validates king count and side-not-to-move-not-in-check. Lesson: never trust hand-recalled FENs; validate at the boundary.
- Sanity match: v0.2.0 vs random-mover build, 100-0 at STC, no crashes (RESULTS.md row 0).
- Stale processes holding `target\release\razor.exe` block rebuilds — `Get-Process razor | Stop-Process -Force` first if cargo says "failed to remove file".
- Next session: copy v0.2.0 binary to `matches\masters\`, then ladder feature #1 (MVV-LVA) under SPRT [0,10].
