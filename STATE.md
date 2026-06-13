# RAZOR — Project State

> Journal per project brief (`H:\RazorBot\aggressive_engine_prompt.md`). Re-read brief at session start. Update this file every session.

## Current Status

- **Date:** 2026-06-13 (session 2 cont.)
- **Phase:** 3 — NNUE. **v0.5.0 tagged: NNUE eval validated (+50.9 Elo vs v0.4.0 PSQT).** M1 milestone check vs SF18 running.
- **Engine version:** Razor 0.5.0 (tag `v0.5.0`, commit 9ecf34e). NNUE `(768→512)x2→1` razor1 net, incremental dual-perspective accumulator, 3.97M nps. Archived `matches\masters\razor-v0.5.0.exe`. PSQT eval retained as `UseNNUE=false` fallback.
- **Strength chain:** v0.2.0 → +1290 (search) → v0.3.0 → +123 LTC → v0.4.0 → +51 NNUE → v0.5.0 → **+209 net2 (gen2 NNUE-labeled)** → v0.6.0. The net-generation loop is the biggest lever now.
- **v0.6.0 (tag, commit 34fcb7a):** net2, same `(768→512)x2→1` arch as net1 but trained on NNUE-labeled data. bench 26,242. `masters\razor-v0.6.0.exe`. LTC confirm vs v0.5.0 running.

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
