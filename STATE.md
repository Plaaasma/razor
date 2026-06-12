# RAZOR — Project State

> Journal per project brief (`H:\RazorBot\aggressive_engine_prompt.md`). Re-read brief at session start. Update this file every session.

## Current Status

- **Date:** 2026-06-12 (session 2)
- **Phase:** 2 — Search ladder in progress (Gates G0 + G1 PASSED 2026-06-12)
- **Engine renamed VENDETTA → Razor** (user request). Repo now `H:\RazorBot\razor`, binary `razor.exe`.
- **Engine version:** Razor 0.2.0 baseline (tag `v0.2.0`) + ladder features through LMR in source (HEAD e3b8649). SPRT confirmation in flight — see ledger and "Job Queue".
- **bench signatures along the ladder** (depth 5, 10 FENs): v0.2.0 600,313 → +MVV-LVA 108,645 → +qsearch 2,269,001 → +TT 929,142 → +PVS 555,077 → +killers 510,238 → +history 507,500 → +NMP 478,199 → +LMR 117,145
- **SPRT-confirmed:** MVV-LVA +331 Elo (174 games), qsearch +371 Elo (170 games). TT/PVS/killers/history/NMP/LMR queued.
- **Estimated strength:** no claim until ladder SPRTs complete.
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
- **QUEUED (next session):** read queue results → fill RESULTS.md rows → promote final master + tag v0.3.0 → LTC 40+0.4 confirmation of accumulated gains → continue ladder (aspiration windows, LMP, futility/RFP, SEE pruning in qsearch, continuation history, time mgmt refinement).
- **BLOCKED:** none

## OPEN BUG — rare panic in release binaries (investigation running)

- Two Windows Application Error 1000 events, exception 0xc0000409 (Rust panic under panic=abort): razor-tt.exe at 10:36, razor-killers.exe at 11:27 (2026-06-12). Rate ≈ 1 crash per ~700-1000 games at 8+0.08. fastchess `-recover` scored crashes as losses, so SPRT passes stand (conservative).
- First crashing binaries are the TT-era ones, but code inspection of tt.rs/search.rs found no obvious panic site (all TT moves are equality-compared only, never executed; indexes bounded; mate scores fit i16).
- Repro attempt 1: release-dbg (assertions+overflow checks+unwind) selfplay, 300 games @ 2+0.02 — CLEAN.
- Repro attempt 2: 400 games @ 8+0.08 (the crash TC), release-dbg — CLEAN. 760 total assertion-binary games without repro; no further WER events as of 12:30.
- Status: UNREPRODUCED. Crash rate ~1/1000 games in release candidates; could be a panic the release-dbg build masks differently, or genuinely rare state. Next angles (next session): (a) stderr-capture shim around release candidates during regular SPRT runs so the panic message gets logged at natural occurrence; (b) longer release-dbg soak overnight on idle box. Do NOT run soaks concurrently with SPRTs (see below).
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
