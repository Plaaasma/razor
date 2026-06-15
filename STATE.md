# RAZOR — Project State

> Journal per project brief (`H:\RazorBot\aggressive_engine_prompt.md`). Re-read brief at session start. Update this file every session.

---
## ★ SESSION HANDOVER (2026-06-14) — READ THIS FIRST ★

**Where we are:** Razor **v0.8.0** (tag `v0.8.0`) = v0.7.0 net (768, `razorsf.nnue`) + i32 acc (restored, was a −37 i64 regression in HEAD) + log-LMR (+9.5 STC / +6.95 LTC). bench **20249**. vs SF18 v0.7.0 was 4.0%/−552; v0.8.0 adds ~+7-9 Elo → M1 recheck pending (still expect ~−540, M1 −400 a way off). archive `masters\razor-v0.8.0.exe`.

**HEAD = v0.7.0 line + i32 acc revert + log-LMR. bench = 20249** (was 22141 at v0.7.0 tag; i32 revert kept it 22141, then LMR changed the tree → 20249). Verify `target\release\razor.exe bench` → `Nodes searched : 20249`. `src\nnue.rs` must have `HIDDEN=768` + `include_bytes!("../nets/razorsf.nnue")` + **i32** accumulation (NOT i64). search.rs has the log-LMR `lmr_table()`. Tagged **v0.8.0** (LTC-confirmed +6.95). Archived `masters\razor-v0.8.0.exe`.

**RUNNING (2026-06-15): LMR LTC-confirm** — bg `bo8vi5imh`, `matches\ltc_lmr.ps1` (40+0.4, 300 games), live `ltc-lmr-live.log`, summary `ltc-lmr-summary.txt`. `masters\razor-lmrbase.exe` (i32+LMR) vs `masters\razor-v0.7.0.exe`. Confirms the LMR gain holds at LTC before tagging v0.8.0. After: tag v0.8.0 (bench 20249, archive masters\razor-v0.8.0.exe) → M1 recheck vs SF18.

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
