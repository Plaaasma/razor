# RAZOR — SPRT Ledger

Every test gets a row, pass or fail. Bounds in Elo, alpha=beta=0.05 unless noted.
Standard STC: 8+0.08s, 1 thread, 16 MB hash, 16 concurrent, balanced book (8moves_v3.pgn).
Standard LTC confirm: 40+0.4s.

| # | Date | Patch | Base → New | Bounds | TC | W-D-L | LLR | Result | Notes |
|---|------|-------|------------|--------|----|----|-----|--------|-------|
| 0 | 2026-06-12 | v0 eval + minimal AB search | random-mover (b107111) → v0.2.0 | n/a (sanity, not SPRT) | 8+0.08 | 100-0-0 | n/a | PASS | functional gate only; first real SPRT starts with ladder feature #1 |
| 1 | 2026-06-12 | MVV-LVA capture ordering | v0.2.0 → mvvlva | [0, 10] | 8+0.08 | 138-27-9 | 2.98 | **PASS** | Elo +331±71, 174 games. New master = mvvlva |
| 2 | 2026-06-12 | Quiescence search | mvvlva → qsearch | [0, 10] | 8+0.08 | 142-20-8 | 2.96 | **PASS** | Elo +371±82, 170 games. New master = qsearch |
| 3 | 2026-06-12 | Transposition table | qsearch → tt | [0, 10] | 8+0.08 | 142-44-28 | 2.98 | **PASS** | Elo +206±48, 214 games. New master = tt |
| 4 | 2026-06-12 | PVS | tt → pvs | [0, 10] | 8+0.08 | 465-400-373 | 3.00 | **PASS** | Elo +25.9±14.5, 1238 games, 34 min. Small effect → long test. New master = pvs |

<!-- Append rows below as tests complete. Never delete rows. -->
