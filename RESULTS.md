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
| 10 | 2026-06-12 | Aspiration windows | lmr (v0.3.0) → aspiration | [0, 10] | 8+0.08 | 281-462-197 | accepted | **PASS** | Elo +31.1±15.9, 940 games. New master = aspiration |
| 11 | 2026-06-12 | SEE pruning (qsearch) — attempt 1 | aspiration → seeprune | [0, 10] | 8+0.08 | (2717 games) | unresolved | **ABORTED** | Observed +6.0, mid-bounds grind with no convergence. Protocol moved to mid-project bounds [0,5] (brief §5); re-running. Games discarded — SPRT can't switch bounds mid-test |
| 12 | 2026-06-12 | Late move pruning | futility → lmp | [0, 5] | 8+0.08 (Spark/aarch64) | 275-518-427 | H0 accepted | **FAIL −43.5±14.9** | 1220 games. Far too aggressive (2+d² threshold with current ordering quality). REVERTED. checkext/timemargin candidates rebuilt without LMP. Brief §5 was right: "expect several to fail on first attempt; debug margins, don't abandon" — retune later with better ordering |
| 13 | 2026-06-12 | Check extensions (no-LMP rebuild) | futility → checkext2 | [0, 5] | 8+0.08 (Spark/aarch64) | 578-1000-424 | accepted | **PASS** | Elo +26.8±10.6, 2002 games. Confirms checkext independent of LMP |
| 14 | 2026-06-12 | RFP — attempt 1 | seeprune → rfp | [0, 5] | 8+0.08 | (769 games) | killed | **ABORTED (operator)** | Observed +71.9 (60.2%) — overwhelming but test killed in queue-cleanup crossfire before formal resolution. Re-running clean; see row for attempt 2 |
| 15 | 2026-06-12 | SEE pruning — final disposition | aspiration → seeprune | [-5, 0] + history | 8+0.08 | (2491 games attempt 3) | unresolved | **KEPT (operator, neutral)** | ~8400 games across 3 attempts: +6.0/2717, +0.5/3212, +13.7/2491 — all ≥0, no formal resolution (effect sits at bounds). No gain claim. SEE retained as infrastructure for batch-3 ordering/guards |
| 16 | 2026-06-12 | MoveOverhead 20ms default | checkext2 → timemargin2 | [-5, 0] | 8+0.08 (Spark) | (3157 games) | unresolved | **REJECTED (operator)** | Observed −4.8, pinned at H0 boundary. 20ms/move reserve ≈ −5 Elo thinking-time tax on idle box. Default set to 0; UCI option retained for laggy GUIs. No further SPRT needed (default 0 = behavior-identical to checkext2) |
| 17 | 2026-06-12 | Reverse futility pruning — attempt 2 | seeprune → rfp | [0, 5] | 8+0.08 | 461-487-292 | accepted | **PASS** | Elo +47.7±15.1, 1240 games. New master = rfp |
| 18 | 2026-06-12 | Futility pruning | rfp → futility | [0, 5] | 8+0.08 | 1545-1574-1023 | unresolved | **KEPT (operator)** | +9.9 at 2276g drifting to +4.8 at 4142g — straddles bounds midpoint, won't resolve. Consistently positive (never negative), so kept per neutral/positive-keep protocol. No firm gain claim (~+5). New master = futility |

<!-- Append rows below as tests complete. Never delete rows. -->
