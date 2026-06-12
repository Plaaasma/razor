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
| 5 | 2026-06-12 | Killer moves | pvs → killers | [0, 10] | 8+0.08 | 227-213-138 | 2.97 | **PASS** | Elo +53.9±22, 578 games. New master = killers. Note: one razor-killers crash mid-test (0xc0000409 panic), scored as loss via -recover — investigation open |
| 6 | 2026-06-12 | History heuristic | killers → history | [0, 10] | 8+0.08 | 531-501-440 | 2.96 | **PASS** | Elo +21.5±13, 1472 games, 40 min. New master = history |
| 7 | 2026-06-12 | Null-move pruning | history → nmp | [0, 10] | 8+0.08 | 147-63-38 | n/a (accepted) | **PASS** | ≈+164 Elo (72.0%), 248 games, 6:39. Summary clipped by queue script (timeout stats); W-D-L recovered from PGN. 22 timeouts total (9 new / 13 base) from CPU contention with concurrent bughunt run — roughly symmetric, result direction unaffected. New master = nmp |
| 8 | 2026-06-12 | Late move reductions | nmp → lmr | [0, 10] | 8+0.08 | 133-120-39 | 2.97 | **PASS** | Elo +116±31, 292 games, 8:20. New master = lmr → tagged v0.3.0 |
| 9 | 2026-06-12 | LTC confirm: v0.3.0 vs tt-master | fixed 200 games | n/a | 40+0.4 | 159-35-5 | n/a | **CONFIRMED** | 88.7% ≈ +358 Elo at LTC vs STC chain expectation ~+382 for the same five features — gains hold at LTC. v0.3.0 validated |

<!-- Append rows below as tests complete. Never delete rows. -->
