# VENDETTA — Project State

> Journal per project brief (`H:\RazorBot\aggressive_engine_prompt.md`). Re-read brief at session start. Update this file every session.

## Current Status

- **Date:** 2026-06-12
- **Phase:** 0 — Environment setup (Gate G0 in progress)
- **Engine version:** none yet (cargo skeleton only)
- **Estimated strength:** n/a
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
| bullet trainer | NOT YET SET UP (task pending) |

## Books

| Book | Use | Source |
|---|---|---|
| `H:\RazorBot\books\8moves_v3.pgn` (8.0 MB) | Balanced SPRT testing | github.com/official-stockfish/books |
| `H:\RazorBot\books\UHO_XXL_2022_+120_+149.pgn` (74.7 MB) | Unbalanced benchmark vs SF (§7) | github.com/official-stockfish/books |

## Syzygy

- 3-4-5 man downloading from `tablebase.lichess.ovh/tables/standard/3-4-5/` to `H:\RazorBot\syzygy\` (resumable script: `H:\RazorBot\scripts_bootstrap\syzygy_dl.py`)

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

## Job Queue

- **RUNNING:** Syzygy 3-4-5 download (background)
- **QUEUED:** bullet setup + GPU smoke test; G0 smoke match (SF vs SF, 100 games)
- **BLOCKED:** none

## Next Steps

1. Finish Syzygy download (verify file count + size).
2. Clone `bullet`, verify CUDA training works on 4070 Ti (check local CUDA toolkit first).
3. Run G0 smoke match: SF vs SF, 100 games, fastchess, balanced book → Gate G0 PASS.
4. Begin Phase 1: bitboards + movegen + perft.

## SPRT Ledger

See `RESULTS.md`.

## Session Log

### 2026-06-12 — Session 1 (Phase 0)
- Inventoried local machine + Spark; journal above.
- Updated Rust 1.63 → 1.96.0 locally; installed Rust on Spark.
- Set up passwordless SSH to Spark (ed25519, pushed via paramiko one-shot).
- Pinned Stockfish 18 (bmi2 build), verified bench.
- fastchess v1.8.0-alpha installed and verified.
- Books downloaded (8moves_v3, UHO_XXL_2022_+120_+149).
- Repo initialized (`H:\RazorBot\vendetta`, cargo skeleton).
- Known issue: piping text to engines from PowerShell 5.1 adds UTF-16 BOM — use argv commands (`stockfish.exe bench`) or write input files without BOM.
