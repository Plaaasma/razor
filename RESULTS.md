# RAZOR — SPRT Ledger

Every test gets a row, pass or fail. Bounds in Elo, alpha=beta=0.05 unless noted.
Standard STC: 8+0.08s, 1 thread, 16 MB hash, 16 concurrent, balanced book (8moves_v3.pgn).
Standard LTC confirm: 40+0.4s.

| # | Date | Patch | Base → New | Bounds | TC | W-D-L | LLR | Result | Notes |
|---|------|-------|------------|--------|----|----|-----|--------|-------|
| 0 | 2026-06-12 | v0 eval + minimal AB search | random-mover (b107111) → v0.2.0 | n/a (sanity, not SPRT) | 8+0.08 | 100-0-0 | n/a | PASS | functional gate only; first real SPRT starts with ladder feature #1 |

<!-- Append rows below as tests complete. Never delete rows. -->
