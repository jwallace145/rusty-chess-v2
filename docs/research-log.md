# Research log

A running record of research sessions: what was read, which hypotheses were
considered and how they were ranked, what was screened and dropped before it
reached a pull request, and what each experiment measured. An experiment's own
PR and its page in `docs/` stay the detailed record; this log is the index,
and the only home of the exploratory work that never becomes a PR.

Two kinds of evidence appear here, and they are never mixed:

- **Exploratory:** local screening matches on a laptop, at fixed nodes, from a
  scratch build. They choose what to test. They are not results, and no Elo
  claim rests on them ([AGENTS.md](../AGENTS.md): the standardized runner is
  the record).
- **Confirmatory:** the `SPRT` workflow on the dedicated instances, both legs,
  against the PR's base. One pre-chosen variant per PR, its plan written here
  before the label goes on.

## Session 2026-11-12: search selectivity after net8

An overnight session run by a coding agent (Claude Code) with the owner away.
The owner delegated research, experiments and pull requests, and kept every
merge, release and infrastructure decision.

### Summary

Written at the end of the session. Everything below it is the record it
summarises: each plan as it was written before its run, then the result.

**What was found.**

1. **Two search changes passed the gate, H1 in both SPRT legs, and wait for
   the owner:** an adaptive null-move reduction (#246, **+23.9 ± 9.0** timed,
   **+27.7 ± 9.7** at fixed nodes, and +29.4 in a 2,000-game match at 10+0.1),
   and one more ply of late-move reduction at expected cut nodes (#254,
   **+9.8 ± 5.6** and **+10.6 ± 5.8**). They were tested separately against
   `main`; how they add is not measured.
2. **One change is the owner's call:** futility pruning of captures (#252),
   **+17.7 ± 7.9** at fixed nodes (H1) and **+4.9 ± 3.8** timed, where the SPRT
   reached its cap without a verdict at LOS 99.4%. With 10,000 more timed games,
   played as a match whose reading was fixed beforehand, the timed gain is
   **+4.0, 95% interval +1.3 to +6.7**. The gate as run is not met.
3. **Four changes did not pass and are dropped:** the table score as the
   pruning evaluation (#248: +16.5 ± 7.4 timed, +2.8 ± 3.7 at fixed nodes, and
   it crosses the fail-high-first bar), quiescence futility (#249: −10.8 ± 6.7
   at fixed nodes), a fourth ply of null-move reduction (#250: +1.5 ± 3.7
   timed; its fixed-node leg never played), and continuation history four
   plies back (#255: +2.1 ± 3.8 timed, −1.2 ± 4.0 at fixed nodes).
4. **A net trained overnight passed the gate too:** net8x160, net8's recipe at
   twice the schedule on identical data, beat net8 by **+11.6 ± 6.1** timed and
   **+8.2 ± 5.1** at fixed nodes (#256), from a held-out loss 0.58% lower on
   identical records. net8 was under-trained: its 80 superbatches were 2.6
   passes over 3.03B positions. #256 is a draft that still needs its gallery
   entries.
5. **The search's constants are about right.** net8's pawn is worth about 260
   evaluation units, so every evaluation-based margin is 3.4 times tighter in
   pawns than when it was tuned under PeSTO, and it does not matter: reading
   the evaluation at 125% and at 80% scored −0.6 and +0.6 Elo, and the result
   of an SPSA run over 17 constants screened at +1.6 ± 9.0. Both search gains
   were structural, not a better value of an existing constant.
6. **Shallow screens mislead, and the screening rule was amended three times,
   in the open.** Local screens at 40,000 nodes a move had the wrong size or
   sign for all three changes taken from them straight to an SPRT (+13.0 was
   +27.7, +15.8 was +2.8, +12.7 was −10.8), and read −12 for the change that
   became #254. They were right about one (capture futility, about +16 at
   both). From 00:34 MST candidates were screened on the instance at the
   SPRT's own budget instead, and a single screen there still moved in its
   SPRT: +27.3 was +10.6, +11.6 was −1.2, +12.2 was +17.7.

**The pull requests.** None is merged; merging is the owner's.

| PR | What | State when the session ended | Recommendation |
|:--|:--|:--|:--|
| #246 | Adaptive null-move reduction | Gate met; every check green | **Merge candidate.** First: it is the larger gain |
| #254 | LMR one ply deeper at expected cut nodes | Gate met; every check green | **Merge candidate.** After #246 it needs a rebase (the null-move search's child takes the new argument) and, by the repository's rule, its SPRT again, which is also the first measurement of the two together |
| #252 | Futility pruning of captures | Fixed-node H1; timed inconclusive at the cap, pooled +4.0 (+1.3 to +6.7) | **The owner's decision:** the rules' "inconclusive at high LOS" case |
| #248 | Table score as the pruning evaluation | Draft. Timed H1, fixed-node inconclusive; crosses the fail-high-first bar | Not as it stands: two decisions against the rules for a gain the fixed-node leg does not see |
| #249 | Quiescence futility | Draft. Fixed-node H0 | Close |
| #250 | Null-move base 3 → 4, stacked on #246 | Draft. Timed inconclusive at +1.5; fixed-node leg never played | Close |
| #255 | Continuation history four plies back | Draft. Both legs inconclusive at the cap, +2.1 and −1.2; a node 6% dearer | Close |
| #256 | Embed net8x160 | Draft. Gate met; two checks red by construction (the gallery needs a manifest that names the net's release) | **Merge candidate, once its gallery entries are written** for the release the owner chooses. Tested against `main` alone, as #246 and #254 were |
| #253 | `nnue-scale`, and the scale of every net | Green. A tool and documentation | Merge candidate; no Elo claim |
| #251 | `Match` workflow: portable `RUSTFLAGS` | Green. Not exercised end to end | Merge candidate: it removes a failure that cost two retries tonight |
| #247 | This log, the net8 ledger correction, net8x160's record, the roadmap | Ready for review | Merge after review |

**Since the session ended:** the owner merged #246 and released it as `v0.49.0`
(2026-11-13), and #254 was rebased onto it and passed its SPRT again, H1 in both legs: **+8.1 ± 4.9** timed and **+7.6 ± 4.7** at fixed nodes; the owner then merged it (`059caa3`). #256 was rebased onto `v0.50.0` in turn and passed its SPRT again, H1 in both legs: **+12.3 ± 6.4** timed and **+30.8 ± 10.8** at fixed nodes; the owner merged it too (`e80876f`). #252 was then rebased onto `v0.51.0` and did not pass its SPRT there: −0.5 ± 3.8, LOS 39.3% (inconclusive at 10,000 games, LLR −1.42) timed and **−4.2 ± 4.4** (H0, 7,038 games) at fixed nodes. The
table above is the state at the session's end; what followed is in
[Follow-up 2026-11-13](#follow-up-2026-11-13-experiment-6-again-on-top-of-246).

**What it cost.** About $35.30 of AWS at the session's estimates, against a
self-imposed cap of $40 (the table at the end); nothing was left running.
Eight SPRTs (one with a single leg played), twelve matches (three more
attempts failed, one of them after launching an instance), one training run,
one local SPSA run and sixteen local screens.

**Where the remaining strength is, on this evidence.**
- **In structure the search does not have, tested at the gate's budget.** Both
  search gains were of that kind, and the session had time for only a few such ideas.
  Screened and close: the two singular-extension variants (+5.2 and +6.4 in
  3,000 games each, never tested together). Built in the lab and never screened
  at 200,000 nodes: capture history in ordering, losing-capture reductions,
  razoring.
- **Not in the constants,** unless a much longer tuning run says otherwise.
- **In the net's schedule, and then in its data.** Twice the schedule was
  worth about +10 for one training run. Generation 7 should be played by an
  engine that already has #246, #254 and this net (its labels are that engine's
  searches), and the net trained on it at about five passes over the data.
- **In how the two SPRT legs are read.** They disagreed in both directions
  tonight, and the session did not explain it. A match of the same change at
  two or three node budgets would.

### Scope, authority and limits

| | |
|:--|:--|
| Started | 2026-11-12, about 23:00 MST (06:00 UTC on 11-13), from `main` at `0a45da0` (`v0.48.1`) |
| Goal | Find a change that gains Elo under the repository's own gate: H1 in both the timed and the fixed-node SPRT |
| Allowed | Branches, PRs, the `SPRT`, `Match` and `Train` workflows, local measurements |
| Not done by the agent | Merging, releases, branch rules, infrastructure, `Relabel` dispatches (each needs the owner's approval), lowering a performance bar (AGENTS.md: never lower one; a crossing is reported and left to the owner) |
| Originality | No other engine's evaluations, weights, games or code, anywhere. Techniques come from the published literature and are implemented from their descriptions |
| Spend cap | **$40** of estimated AWS cost for the session, set by the agent because the account has no AWS Budget configured (`aws budgets describe-budgets` returned none). Each workload's estimate is logged below before it starts |
| Capacity | The `sprt` task profile allows 128 vCPUs, so **one dual SPRT at a time**; `match` allows one 64-core instance beside it; `gpu` one `g6.xlarge` |

### Where the engine stood

- **Engine:** `v0.48.1`, bench signature `970013` at depth 10, fail-high-first
  0.874 (bar 0.87), EBF 1.88 (bar 2.0), 4,066 instructions a node (#235's
  bench comment). No open PRs.
- **Net:** `net8-l050-r42564788061`, (768 × 4 → 512) × 2 → 8, generations 0–6,
  the default since `v0.48.0`. It passed both legs against net7rl012:
  **+14.1 ± 6.9** timed, **+8.8 ± 5.3** at fixed nodes (#235).
- **A gap in the ledger:** [nets.md](nets.md) still listed net8 as "SPRT
  pending". Corrected in this PR from #235's SPRT comment.
- **Search:** everything in [roadmap.md](roadmap.md) Phases 3–4b, plus the
  pawn correction history (#221). Its constants were all set with PeSTO as the
  evaluation, except the correction history's. The roadmap's open Phase 6 item
  is exactly that: "Re-tune what was tuned to PeSTO's scores … revisit the
  improving heuristic (#95) and one more LMR ply when not improving".

### What the record says about where Elo is left

1. **The net's knowledge is no longer the cheap lever.** The last four data or
   label steps gained +6.8, +16.3, +1.9 (dropped) and +14.1 timed, each for a
   generation of self-play or a relabelling run plus about two GPU hours. A
   wider net (768) is +28.9 per node and −9.0 at a clock (#214).
2. **The search has not been touched since the evaluation changed,** except
   for the correction history (+7.4). Every margin (reverse futility 80 cp a
   ply, futility 100 + 100, null move R = 2 + depth/4, LMP 3 + depth², LMR's
   table) dates from PeSTO, an evaluation several hundred Elo weaker and much
   noisier ply to ply. The one heuristic dropped for that noise, improving
   (#95), was dropped "until NNUE".
3. **Fixed-node gains that make nodes dearer do not survive a clock;** changes
   that prune (fewer nodes to a depth) have kept theirs: futility +25.6 timed,
   IIR +16.7, SEE pruning +26.1.
4. **Proxies have misled twice** (the correction-history schedule, the
   major-piece table). A pre-test has to play games.

### Hypotheses, ranked

Ranked before any screen was run, on the brief's six criteria: expected
strength, quality of evidence, chance the implementation works, cost to build
and test, effect on speed, and risk of regressions or confounding. "Evidence"
cites this repository's record first and the literature second; the reference
list is at the end of the session.

| # | Hypothesis | Expected gain | Evidence | Works? | Cost | Speed | Risk | Rank |
|--:|:--|:--|:--|:--|:--|:--|:--|:-:|
| H1 | **Null-move pruning with a deeper, evaluation-dependent reduction.** R = 2 + depth/4 is the Phase 4 value; adaptive null move (Heinz 1999) grows R with depth and with the margin by which the evaluation clears beta | High | The technique was +130.8 here at its first, mildest setting; with an accurate evaluation, "eval ≥ beta" is far more reliable than it was | High | One expression | Fewer nodes to a depth | Zugzwang; fail-high-first sits 0.004 over its bar and whole-node pruning lowers it | **1** |
| H2 | **The improving heuristic,** modulating reverse futility, late move pruning and LMR | Medium to high | The roadmap's own deferred item; lost with PeSTO (#95) for a reason NNUE removes | Medium | Small: an evaluation stack | Fewer nodes | It failed once; three sites are three changes, to be separated | **2** |
| H3 | **LMR by node type:** more at expected cut nodes, less at PV nodes | Medium | Standard; LMR is the largest single gain in the record (+204) and has had one adjustment since (history, +15.2) | Medium | Small: a cut-node flag through the recursion | Mixed | PV-node relief costs nodes | **3** |
| H4 | **Futility pruning in quiescence** (stand pat + victim + margin ≤ alpha) | Low to medium | Quiescence is half the nodes; SEE pruning there was +111.7 | Medium | A few lines | Fewer nodes | Low | **4** |
| H5 | **Singular extension variants:** double extension, negative extension | Low to medium | Singular extensions were +16.6 timed | Medium | Small | Dearer (deeper lines) | Tree growth; EBF bar | **5** |
| H6 | **The table score as the evaluation for pruning,** and no table cutoffs at PV nodes | Low | Standard; unmeasured here | Medium | A few lines | Neutral | Low | 6 |
| H7 | **Faster-adapting history** (a larger bonus than depth², with LMR's divisor rescaled) | Low to medium | `LMR_HISTORY_DIVISOR`'s own comment: scores stay in the hundreds, so history barely moves reductions | Low to medium | Small | Neutral | Two constants move together | 7 |
| H8 | **Capture history** for ordering and pruning captures | Medium | Standard; captures are ordered by MVV-LVA only | Medium | Medium: the picker | Slightly dearer | Low | 8 |
| H9 | **Tune all margins together by SPSA** in self-play | High | The roadmap item; the standard method | Medium | High: a tuner, UCI-exposed parameters, tens of thousands of games | Neutral | Over-fitting to one time control | 9 (build cost) |
| H10 | **Time management** by best-move stability and node share | Medium at a clock | Three earlier plans lost (#59, #73), all PeSTO-era | Medium | Small | n/a | Cannot pass the gate by construction: the fixed-node leg is zero. An owner's merge-over decision | 10 |
| H11 | **768 hidden neurons again,** on net8's data, now that the refresh cache exists | +29 per node, about zero at a clock | #214, and the cache's measured +2.6% relative gain | Low for the timed leg | 2.2 GPU hours and an SPRT | 11% slower a node | The same failure as #214 | 11 |
| H12 | **Drop generation 0 or 1 from training** (PeSTO-era games give noisy results at λ 0.5) | Unknown sign | None here | Low | A `Train` run and an SPRT | None | A training run for a coin flip | 12 |
| H13 | **Generation 7 and net9** | About +5 to +7 | The shrinking series +21.9, +14.5, +6.8 (+14.1 for the largest generation) | Medium | About $11 of spot, a full audit, 2 GPU hours, an SPRT: most of a night | None | Near `elo1`, so likely the cap | 13 |
| H14 | **Speed:** table prefetch, the table move before move generation, lazy accumulators | A few Elo each | The profile in [nnue.md](nnue.md): at most 13% of updates are skippable; move generation is 10% of a node | High | Small to medium | Faster | Below what an SPRT at `elo1` 5 can confirm; `perf` PRs judged by the bench comment | 14 |

**Chosen first: H1, then H2, then H3,** each as its own PR against `main`.
The net experiments (H11–H13) are not run this session: each needs hours of
serial GPU and SPRT time for an expected gain at or below the search items',
and the only training knobs reachable without merging trainer changes are the
`Train` workflow's inputs.

**Added on 2026-11-13 at 01:07 MST: H15, net8 is under-trained.** Not in the
ranking above, which took the ledger's "at 1.5B positions a longer schedule is
saturated" as settled and set the net experiments aside. Counting passes says
otherwise: 80 superbatches were 5.5 passes over net 5's data and are 2.6 over
net8's, and the ledger's own advice is to revisit length when the data grows a
lot. It costs a `Train` run and an SPRT and uses the GPU instance, which was
idle, so it does not take from the search experiments. Its plan and record are
in [nets.md](nets.md#net8x160-l050-r43390801430).

### Method

**The lab build.** A scratch branch, `research/overnight-lab` (pushed for
reproducibility, never a PR), adds runtime parameters for each hypothesis as
UCI options `L_<NAME>`, every one defaulting to `main`'s compiled-in value. The
check that the defaults change nothing is the bench signature: `970013`, equal
to `main`'s.

**Screens (exploratory).** The lab binary with one option set plays the same
binary with none, with fastchess `v1.8.2-alpha` built from source, on an Apple
M3 Max (12 games at once), from `tests/openings/openings.epd` with the SPRT's
adjudication settings (`sprt-report args`), at a fixed node budget per move.
Fixed nodes make the games independent of the laptop's speed. Stage 1 is
40,000 nodes a move and at most 2,000 games with fastchess's SPRT at
[0, 5] as an early stop.

**The rule, written before the first screen finished:** a variant is put
forward for a PR when its stage-1 estimate is at least +8 Elo or its early
stop accepts H1; between 0 and +8 it may get a second screen at more nodes or
games; below 0 it is dropped. Every screen is listed below, kept or not.
What the rule cannot do is certify anything: 2,000 games is ±10 Elo, the
node budget is a fifth of the SPRT's fixed-node leg and about a tenth of the
timed one's, and picking the best of several variants flatters the pick. The
confirmatory SPRT is a fresh sample on the standardized hardware.

**PRs (confirmatory).** One variant per PR, chosen before its SPRT, built as
constants on a branch from `main`. The clean build's bench signature must
equal the lab build's with the same options, which ties the PR to what was
screened. The plan (question, the one change, held the same, gate) is
recorded here before the `sprt` label goes on. Gate: H1 in both legs.

**Performance bars.** Fail-high-first is 0.874 against a bar of 0.87, so a
whole-node pruning change can cross it. Decided before any run: the agent
does not lower a bar. A PR that crosses one is reported with its SPRT and
left for the owner, with the mechanism measured from the `stats` counters.

### Screens

Exploratory, local, fixed nodes; candidate against the default parameters.
`LLR` is fastchess's at [0, 5]; "stop" says how the screen ended.

<!-- screens:begin -->
| Screen | Options (against the defaults unless noted) | Nodes/move | Games | Elo ± 95% | LOS | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) | LLR | How it ended, and the decision |
|:--|:--|--:|--:|--:|--:|:-:|:--|--:|:--|
| `s1-nmp-a` | `NMP_BASE=3 NMP_DIV=3 NMP_EVAL_DIV=200`: null-move R = 3 + depth/3 + min((eval − beta)/200, 3) | 40,000 | 2,000 | **+13.0 ± 9.4** | 99.7% | 547 / 472 / 981 | 29, 211, 462, 252, 46 | 1.54 | Game cap. **Put forward: experiment 1 (#246)** |
| `s1-imp-rfp` | `RFP_IMPROVING=1`: one ply less reverse-futility margin when improving | 40,000 | 2,000 | −9.9 ± 10.6 | 3.4% | 516 / 573 / 911 | 73, 233, 434, 198, 62 | −1.38 | Game cap. **Dropped** |
| `s1-lmr-cut` | `LMR_CUT=1`: one more ply of reduction at expected cut nodes | 40,000 | 1,796 of 2,000 seen | −11.8 ± 10.5 | not kept | not kept | not kept | −1.52 | **Log lost** (the incident below): the last reading the agent saw. Re-queued as `s1-lmr-cut-rerun` |
| `s1-imp-lmr` | `LMR_IMPROVING=1`: one more ply of reduction when not improving | 40,000 | 2,000 | −5.9 ± 10.0 | 12.3% | 449 / 483 / 1068 | 36, 281, 423, 201, 59 | −0.95 | Game cap. **Dropped** |
| `s1-qs-fut150` | `QS_FUT=150`: skip a quiescence capture when stand pat + 150 + the victim's classical value ≤ alpha | 40,000 | 556 | −106.3 ± 21.2 | 0.0% | 82 / 247 / 227 | 48, 108, 87, 31, 4 | −2.95 | H0 by early stop. **Dropped as built, and the reason is a finding**: the rule added classical piece values (100 to 900) to an evaluation whose pawn is worth about 300 (see "The net's material scale") |
| `s1-tt-eval` | `TT_EVAL=1`: the table's score as the evaluation for reverse futility and the null move, where its bound allows | 40,000 | 2,000 | **+15.8 ± 10.1** | 99.9% | 556 / 465 / 979 | 44, 202, 432, 263, 59 | 1.77 | Game cap. **Put forward: experiment 2 (#248)** |
| `sc-rfp120` | `RFP_MARGIN=120`: reverse-futility margin 80 → 120 a ply (wider, towards the net's scale) | 40,000 | 2,000 | −4.5 ± 10.6 | 20.2% | 516 / 542 / 942 | 71, 237, 387, 257, 48 | −0.74 | Game cap. **Dropped**; the wider values queued behind it were withdrawn |
| `sc-fp200` | `FP_BASE=200 FP_MARGIN=200`: futility 100 + 100 a ply → 200 + 200 | 40,000 | 2,000 | −0.3 ± 9.8 | 47.2% | 516 / 518 / 966 | 54, 215, 452, 237, 42 | −0.25 | Game cap. **Dropped**: no difference |
| `c1-tteval-on-nmp` | `TT_EVAL=1` **with experiment 1's null-move form on both sides** (`NMP_BASE=3 NMP_DIV=3 NMP_EVAL_DIV=200`) | 40,000 | 2,000 | +7.1 ± 9.7 | 92.5% | 520 / 479 / 1001 | 42, 222, 425, 275, 36 | 0.72 | Game cap. A stacking check, not a candidate: experiment 2's change reads about half its stand-alone screen on top of experiment 1 |
| `s2-qsfut-net300` | `QS_FUT=300 QS_FUT_NET=1`: quiescence futility with a margin of 300 and the victim at the net's measured values (300, 775, 880, 1,100, 1,680) | 40,000 | 2,000 | **+12.7 ± 10.1** | 99.3% | 543 / 470 / 987 | 38, 229, 417, 254, 62 | 1.38 | Game cap. **Put forward: experiment 3 (#249)** |
| `s3-probcut200` | `PC_MARGIN=200`: ProbCut at depth ≥ 5, any capture that does not lose material, its search at depth − 4 against beta + 200 | 40,000 | 2,000 | −15.7 ± 9.8 | 0.1% | 472 / 562 / 966 | 60, 254, 429, 230, 27 | −2.23 | Game cap. **Dropped as built.** A stricter capture filter (`PC_SEE_DIV=3`) is in the lab build but was not screened here: ProbCut acts at depth, so its test belongs on the instance at 200,000 nodes |
| `s4-capfut200` | `CAPFUT=200`: futility pruning of captures in the main search at depth ≤ 6, when eval + 300 + 200·depth + the victim's net-scale value ≤ alpha | 40,000 | 2,000 | **+15.1 ± 9.5** | 99.9% | 571 / 484 / 945 | 34, 200, 455, 267, 44 | 1.79 | Game cap. Over the threshold; **held for a replication** under the rule added at 00:31 (`r1-capfut200`) |
| `c2-capfut-on-qsfut` | `CAPFUT=200` **with experiment 3's quiescence rule on both sides** (`QS_FUT=300 QS_FUT_NET=1`) | 40,000 | 2,000 | +9.9 ± 9.8 | 97.7% | 545 / 488 / 967 | 42, 202, 469, 231, 56 | 1.07 | Game cap. A stacking check: the main-search rule still reads positive on top of the quiescence rule |
| `s3-lmrnoisy1` | `LMR_NOISY=1`: one ply of late-move reduction for losing captures | 40,000 | 2,000 | +7.1 ± 9.7 | 92.6% | 495 / 454 / 1051 | 23, 261, 420, 244, 52 | 0.73 | Game cap. Under the threshold. **Not pursued** |
| `r1-capfut200` | `CAPFUT=200` again, fresh games: the replication of `s4-capfut200` | 40,000 | 2,000 | **+16.7 ± 9.4** | 100.0% | 571 / 475 / 954 | 33, 193, 464, 265, 45 | 2.01 | Game cap. It replicates at 40,000 nodes (the two pooled: about +15.9). **Judged on the instance by match M6**, not put forward from here |
<!-- screens:end -->

The table is rendered from each screen's fastchess log by a script
(`lab/render_screens.py` on `research/overnight-lab`). **The logs are kept on
that branch,** under `lab/runs/`, with SPSA run S1's state and log, the queue
runners' records, and under `lab/analysis/` the three one-off calculations
behind the clock table, experiment 2's depth table and match M18's pooled row
(`e203c2b`, added at the end of the session; the session's scratch directory
does not outlive it). The repository's commit hooks normalised trailing
whitespace in the copies; they are otherwise what the session read.

**The harness's smoke test and negative control, 23:14 MST, before the first
screen** (`pilot-identical`; recorded here at the end of the session, when the
log was checked against the run directory and this run was found missing from
it). The lab binary against itself with no option set, 40,000 nodes a move, 60
pairs: 31 wins, 31 losses, 58 draws, **every pair balanced** (pentanomial 0, 0,
60, 0, 0). Two identical engines at fixed nodes play an opening's two games as
mirror images, so the harness adds nothing of its own to a result, and a
screen that reads away from zero is reading the option. It is not in the table
because it tests the harness, not a change.

**Incident, 23:33 MST: a screen's log was destroyed by the agent.** The runner
script (`screen.sh`) was edited in place while `s1-lmr-cut` was playing. Bash
reads a script as it runs, so the running copy resumed in the middle of the
new text, ran a fragment that truncated the screen's `fc.log`, and wrote no
summary line. The reading in the table is the last the agent polled from the
live log, at 1,796 games; it cannot be checked now. The script is now one
function that is read whole before it runs, is only ever replaced with `mv`,
and writes each run to its own timestamped log. The screen is re-run for the
record.

**Correction, 23:44 MST.** The first push of the `s1-imp-lmr` row (`8584ef4`)
gave its LLR as −1.07 and its pentanomial counts as "not kept". Both were
written from memory and were wrong: the screen's log is intact and reads LLR
−0.95 and 36, 281, 423, 201, 59. From here on every row of this table is
rendered from the run's fastchess log by a script, not typed.

**Amendment to the screening rule, 23:40 MST, after experiment 1's SPRT and
before any later screen was chosen for a PR.** The null-move screen read +13.0
at 40,000 nodes; the SPRT read +27.7 at 200,000 and +23.9 timed. A change whose
effect grows with depth is under-read by a shallow screen. So: (1) stage 1
stays at 40,000 nodes for changes that act near the leaves (pruning margins,
quiescence, move ordering); (2) changes that act only at depth (singular
extensions, reductions) are not judged at 40,000 nodes; they get a `Match` at
200,000 nodes on the standardized instance (about $1, 3,000 games, ±9 Elo), or
go to their SPRT directly when the case is strong; (3) continuous margins are
not screened one value at a time: they go to one SPSA run (below). The
advance threshold (+8, or an early H1) is unchanged.

**A second amendment to the screening rule, 00:32 MST: a screen must
replicate.** Three screens have now been followed by a measurement at the
SPRT's settings: +13.0 became +27.7 and +23.9 (experiment 1); +15.8 became +2.8
at fixed nodes (experiment 2); +12.7 is reading about +3 timed after 2,500 games
(experiment 3, still running). Part of that is depth, as the first amendment
said. Part is selection: about twenty variants have been screened at ±10, and
the ones put forward are the ones that read highest, which is where noise
points up. From here a variant that reads +8 or more is screened again, fresh
games, and goes forward only if the two screens pooled still read +8 or more.
Experiments 1 to 3 were chosen under the earlier rule and are not re-judged by
this one.

**A third amendment, 00:34 MST: 40,000-node screens no longer choose
experiments.** Experiment 3's fixed-node leg is reading about −9 after 1,900
games, where its screen read +12.7: with experiment 2, that is two changes in
a row whose 40,000-node screen had the wrong size or the wrong sign at 200,000
nodes, and experiment 1's was wrong by half the other way. The screen is not a
smaller version of the SPRT; it is a different test. From here a candidate is
screened **on the instance, at the SPRT's fixed-node budget**, by the `Match`
workflow (3,000 games at 200,000 nodes, about $0.70 and 12 minutes, ±9 Elo at
95%), with the same threshold: +8 or more goes forward to a PR and an SPRT.
The laptop's 40,000-node queue is stopped after the screen in progress
(`r1-capfut200`, the replication of `s4-capfut200`), and the screens still
queued are withdrawn unrun: `s1-caphist8`, `s1-cont4`, `s1-seecap90`,
`s1-imp-lmp`, `s4-probcut200s3` and `s1-lmr-cut-rerun`. So the lost
`s1-lmr-cut` screen is not re-run after all: its only record is the reading
noted above. The laptop runs SPSA run S1 instead, whose result is also judged
on the instance before any PR.

**Screens withdrawn before they ran** (queued, then removed; nothing was
played): `sc-rfp160` and `sc-rfp240` (reverse-futility margin 160 and 240),
after `sc-rfp120` read −4.5; `sc-fp300` (futility 300 + 300), after `sc-fp200`
read −0.4; `sc-asp70`, `s1-lmr-base100`, `s1-lmr-div200`, `s1-rfp-m100`,
`s1-rfp-m65`, `s1-asp20`, `s1-lmp-d8` and `s1-rfp-d8`, single values of
continuous constants that the SPSA run S1 tunes together instead;
`s1-se-dbl20` and `s1-se-neg` (singular-extension variants), which act only
at depths a 40,000-node search barely reaches; `s1-razor3`, `s1-hist1-8192`
and `s1-tt-nopv`, the lowest-ranked, for time; `s2-tteval-down` and
`s2-tteval-up` (the two halves of experiment 2's rule), deferred until its
SPRT says whether they matter.

### The net's material scale

**Measured on 2026-11-12 at 23:43 MST, after `s1-qs-fut150` lost 106 Elo**, a
result too bad to be the heuristic itself.

**The question.** What is a piece worth to net8, in the units the search's
margins are written in?

**Method.** `crates/search/examples/piece_values.rs` on `research/overnight-lab`:
for each position, each non-king piece is removed in turn and the static
evaluation (the raw network, side to move's view) re-read; the change, signed
for the piece's owner, is that piece's value there. Positions in check are
skipped; castling rights are dropped in the modified position. Two sets: the 50
bench positions (all phases) and the 300 SPRT openings. A local measurement.
**Control:** the same program on PeSTO, whose piece values are known.

| Piece | PeSTO, bench | PeSTO, openings | **net8, bench** (mean, 10th–90th percentile) | **net8, openings** | net8 ÷ PeSTO |
|:--|--:|--:|:--|:--|:--|
| Pawn | 86 | 77 | **336** (174–518) | **263** (145–373) | 3.4–3.9× |
| Knight | 331 | 319 | **843** (730–1,034) | **708** (619–801) | 2.2–2.5× |
| Bishop | 350 | 351 | **943** (820–1,187) | **818** (714–922) | 2.3–2.7× |
| Rook | 472 | 457 | **1,249** (947–1,870) | **955** (854–1,050) | 2.1–2.6× |
| Queen | 1,001 | 1,011 | **1,866** (1,468–2,540) | **1,500** (1,346–1,634) | 1.5–1.9× |

The control reads PeSTO's table values, so the method measures what it says.

**What it says.**
- **net8's pawn is worth about 300 evaluation units, not 100.** The net is
  trained on `λ·result + (1 − λ)·sigmoid(score / 400)`, so its output is on the
  scale where 400 units is one unit of log-odds of winning, and between engines
  this strong a clean pawn is worth far more than a quarter of that. Large
  pieces read less than 3× only because the output saturates.
- **Every evaluation-based margin in `search.rs` is therefore about 3.5× tighter
  in pawns than when it was tested under PeSTO:** reverse futility's 80 a ply
  was 0.9 pawns a ply and is now about 0.27; futility's 100 + 100 a ply was 1.1
  + 1.1 pawns and is about 0.33 + 0.33; the aspiration window's ±35 was 0.4
  pawns and is 0.12; the correction history's cap of 128 is 0.4 pawns. Nothing
  re-tested them as the scale moved, generation by generation.
- **It does not say which way to move them.** Tighter margins in pawns are what
  a much more accurate evaluation should allow, and the net's units are close
  to the internal units strong engines publish margins in. The screens that
  tightened further lost (`s1-imp-rfp` −9.9, `s1-imp-lmr` −5.9). The `sc-*`
  screens below ask the other direction.
- **Any rule that mixes the evaluation with classical piece values is wrong by
  3×.** That is what `s1-qs-fut150` did. Static exchange evaluation is safe: it
  compares classical values only with each other.
- **Not measured:** the values in positions that are already unbalanced,
  where the output is flatter. How the scale moved from net to net was not
  measured at first and now is, below.

**The scale by net, measured at 01:30 MST.** The same program on every net that
was a default and is kept in the bucket (`NET=<file>`; each file's SHA-256
matches `nnue::RETIRED_NETS`, except net7's, which that list doesn't carry and
which matches the `net_sha256` in its `training.json`, checked afterwards). Mean value over the 300
SPRT openings, and over the 50 bench positions in brackets:

| Evaluation | Pawn | Knight | Bishop | Rook | Queen | Reverse futility's 80 a ply, in pawns |
|:--|--:|--:|--:|--:|--:|--:|
| PeSTO | 77 (86) | 319 (331) | 351 (350) | 457 (472) | 1,011 (1,001) | 1.04 |
| net 1 | 148 (187) | 560 (607) | 617 (647) | 755 (854) | 1,667 (1,745) | 0.54 |
| net2x80 | 184 (229) | 562 (617) | 629 (678) | 794 (933) | 1,748 (1,905) | 0.43 |
| net4w512 | 211 (256) | 568 (642) | 647 (701) | 781 (946) | 1,350 (1,556) | 0.38 |
| net 5 | 227 (278) | 580 (656) | 651 (725) | 806 (993) | 1,481 (1,683) | 0.35 |
| net6k4 | 238 (283) | 573 (644) | 655 (712) | 777 (927) | 1,109 (1,385) | 0.34 |
| net7 | 242 (287) | 562 (639) | 647 (708) | 761 (928) | 1,179 (1,426) | 0.33 |
| net7rl012 | 263 (332) | 703 (841) | 818 (941) | 981 (1,268) | 1,455 (1,842) | 0.30 |
| net8 | 263 (336) | 708 (843) | 818 (943) | 955 (1,249) | 1,500 (1,866) | 0.30 |

- **The pawn has grown with every net,** from 148 to 263, and the search's
  margins have stood still, so each net made every evaluation-based margin a
  little tighter in pawns than the last. Reverse futility's 80 a ply was tested
  at 1.04 pawns a ply and stands at 0.30.
- **The pieces jumped once.** Knights, bishops and rooks sat near 560, 650 and
  780 from net 1 to net7, then rose by a quarter in one step, to 703, 818 and
  981, with net7rl012: the net trained on generations 0–2 relabelled by net7,
  whose labels [datasets.md](datasets.md#net7-l050-r41363264869-n5000) records
  as larger than the originals.
- **That step is also the largest net gain since king buckets** (+16.3 timed,
  +18.1 at fixed nodes, #202), and the ledger reads it as better labels making a
  stronger net. This table offers a second reading that the record cannot tell
  from the first: a net whose evaluation is a quarter larger makes every margin
  in the search a fifth tighter, and the gain could be partly that. The second
  relabelling, whose labels sat close to the originals and whose scale barely
  moved, gained nothing (#210). **Not established here; matches M16 and M17
  ask the question the cheap way,** by scaling net8's evaluation in the search
  and seeing which way the engine wants its margins.
- **For the ledger:** each net's scale belongs in [nets.md](nets.md) beside its
  loss, measured when it is trained, so a change in the search's effective
  margins is seen when it happens.

### How the clock is spent at 5+0.05

**Measured on 2026-11-12 at 23:56 MST,** as the premise check for H10 (time
management), from the 1,713 games of experiment 1's timed leg (the `sprt-timed`
artifact of run 43378687011; both engines, which share the time plan). Each
move's time and depth are fastchess's PGN comments; the clock is rebuilt from
5 s plus 0.05 s a move.

| Own move | Moves | Mean time | Median time | Mean depth | Mean clock before the move | Time ÷ clock |
|--:|--:|--:|--:|--:|--:|--:|
| 1 | 3,426 | 294 ms | 265 ms | 15.2 | 5.00 s | 5.9% |
| 5 | 3,426 | 303 ms | 271 ms | 15.9 | 3.94 s | 7.7% |
| 10 | 3,426 | 239 ms | 213 ms | 15.5 | 2.81 s | 8.5% |
| 20 | 3,322 | 144 ms | 127 ms | 15.4 | 1.42 s | 10.1% |
| 30 | 3,005 | 97 ms | 85 ms | 16.5 | 0.71 s | 13.6% |
| 40 | 2,457 | 72 ms | 63 ms | 17.6 | 0.35 s | 20.9% |
| 50 | 1,883 | 58 ms | 53 ms | 19.1 | 0.18 s | 31.8% |
| 60 | 1,355 | 51 ms | 48 ms | 19.5 | 0.13 s | 38.0% |

Median game 104 plies; no time forfeits in 1,713 games; a side ends a game with
a median of 0.20 s on its clock.

**What it says.** The plan spends 6 to 9% of the clock a move from the start,
about twice its nominal thirtieth (an iteration that starts under the soft
budget runs on towards the hard one), so the clock is at 1.4 s by move 20 and
the engine lives on the increment from about move 45, at 50 ms a move against
300 ms in the opening.

**What it doesn't say, and why H10 is not run.** That this is wrong. #59 already
tested a slower burn at this control (182 ms a move in the first ten moves
against 269) and scored −4.2 ± 10.8, then matched the clock use with a
stability cut and scored −7.5 ± 11.3. The record's reading stands: at 5+0.05
the front-loaded plan is at least as good as the alternatives tried, and a
time-management change is a long-control question that 5+0.05 cannot answer.
A time-management change also cannot pass the gate by construction (its
fixed-node leg is two identical engines).

### Experiments

| # | PR | The one change | Bench before → after | Timed SPRT | Fixed-node SPRT | Verdict |
|--:|:--|:--|:--|:--|:--|:--|
| 1 | #246 | Null-move reduction `2 + depth/4` → `3 + depth/3 + min((eval − beta)/200, 3)` | 970013 → 939252 | **+23.9 ± 9.0** (H1, 1,706 games) | **+27.7 ± 9.7** (H1, 1,508 games) | **Gate met.** For the owner to merge |
| 2 | #248 | Reverse futility and the null move read the table's score as the evaluation, where its bound allows | 970013 → 942699 | **+16.5 ± 7.4** (H1, 2,714 games) | +2.8 ± 3.7 (inconclusive at 10,000 games) | **Gate not met: dropped by its plan.** A speed gain only the owner could merge over; it also crosses the fail-high-first bar (0.853 against 0.87) |
| 3 | #249 | Futility pruning in quiescence, the victim valued at the net's scale | 970013 → 990066 | +3.6 ± 3.9 (inconclusive at 10,000 games) | **−10.8 ± 6.7** (H0, 3,290 games) | **Gate not met: dropped.** Recommended for closing |
| 4 | #250, stacked on #246 | Null-move reduction's base 3 → 4 | 939252 → 925602 (against #246) | +1.5 ± 3.7 (inconclusive at 10,000 games) | Not played (its instance failed to launch) | **Gate not met: dropped.** Recommended for closing |
| 5 | #252 | Futility pruning of captures in the main search, the victim valued at the net's scale | 970013 → 970114 | +4.9 ± 3.8, LOS 99.4% (inconclusive at 10,000 games, LLR 2.61) | **+17.7 ± 7.9** (H1, 2,668 games) | **Gate not met as run.** Smarter per node; with match M18's 10,000 more timed games the timed gain is +4.0, 95% interval +1.3 to +6.7. The "inconclusive at high LOS" case: the owner's decision |
| 5, second run (after the session) | #252, rebased onto `v0.51.0` | The same patch, against the engine with #246, #254 and #256 | 919304 → 878901 | −0.5 ± 3.8, LOS 39.3% (inconclusive at 10,000 games, LLR −1.42) | **−4.2 ± 4.4** (H0, 7,038 games) | **Gate not met** against `v0.51.0`. Closed by the owner on 2026-11-13. [Follow-up 2026-11-13](#experiment-5-again-capture-futility-against-v0510-252) |
| 6 | #254 | One more ply of late-move reduction at expected cut nodes | 970013 → 988822 | **+9.8 ± 5.6** (H1, 5,004 games) | **+10.6 ± 5.8** (H1, 4,322 games) | **Gate met.** For the owner to merge |
| 6, second run (after the session) | #254, rebased onto #246 | The same patch, against `v0.49.0` | 939252 → 958984 | **+8.1 ± 4.9** (H1, 6,124 games) | **+7.6 ± 4.7** (H1, 6,482 games) | **Gate met again,** on top of #246. Merged by the owner on 2026-11-13 (`059caa3`). [Follow-up 2026-11-13](#follow-up-2026-11-13-experiment-6-again-on-top-of-246) |
| 7 | #255 | Continuation history four plies back, in quiet ordering and the LMR history term | 970013 → 1025352 | +2.1 ± 3.8 (inconclusive at 10,000 games) | −1.2 ± 4.0 (inconclusive at 10,000 games) | **Gate not met: dropped.** Recommended for closing |
| T1 | #256 | The embedded net: net8 → net8x160, net8's recipe with 160 superbatches for 80 | 970013 → 990172 | **+11.6 ± 6.1** (H1, 3,910 games) | **+8.2 ± 5.1** (H1, 6,250 games) | **Gate met.** A draft that needs its gallery entries; for the owner to merge |
| T1, second run (after the session) | #256, rebased onto #246 and #254 | The same net, against net8 in `v0.50.0`'s search | 958984 → 919304 | **+12.3 ± 6.4** (H1, 3,890 games) | **+30.8 ± 10.8** (H1, 1,516 games) | **Gate met again,** in `v0.50.0`'s search. Merged by the owner on 2026-11-13 (`e80876f`). [Follow-up 2026-11-13](#training-t1s-net-again-net8x160-against-net8-in-v0500s-search-256) |

#### Experiment 1: adaptive null-move reduction (#246)

Recorded on 2026-11-12 at 23:26 MST, before the `sprt` label was added.

| | |
|:--|:--|
| Question | With net8 as the evaluation, does a deeper null-move reduction that also grows with the margin by which the static evaluation clears beta gain Elo, per node and at a clock? |
| The one change | `R`, the null-move search's depth reduction: `2 + depth / 4` becomes `3 + depth / 3 + min((eval − beta) / 200, 3)` (`null_move_reduction` in `search.rs`). `feat/null-move-adaptive-reduction` at `b54010f` against `main` at `0a45da0` |
| Held the same | Null-move pruning's conditions (depth ≥ 3, not in check, not after a null move, non-pawn material, corrected static evaluation ≥ beta, off in a singular-extension search), the mate-score clamp, every other margin, the net, time management |
| Why this variant | The textbook adaptive shape (Heinz 1999; depth and evaluation terms), written down as H1 before any screen. It is the only null-move variant screened or tested |
| Screen (exploratory) | `s1-nmp-a`: +13.0 ± 9.4 in 2,000 games at 40,000 nodes; the lab build with these options has the PR's signature, 939252 |
| Bench, local | 970013 → 939252 (−3.2% nodes to depth 10); fail-high-first 0.8740 → 0.8773 (bar 0.87); EBF 1.877 → 1.896 (bar 2.0); both bars pass |
| Expectation | Fixed nodes: +5 to +15. Timed: the same or a little more, since the saving is nodes and a node costs the same |
| Gate | H1 in both legs. Anything else drops the change, recorded here either way |
| Risks named beforehand | Zugzwang at depth (no verification search); a gain at 40,000 nodes that shrinks at 200,000 and at a clock, where the reductions are larger in plies |
| Cost estimate | Two on-demand `c8a.16xlarge` for 20 to 50 minutes: $2.30 to $5.80 |

**Result** ([run 43378687011](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43378687011),
#246, `b54010f` against `main` at `0a45da0`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge`; one attempt; the plan's commit `779b4ae` at 06:25
UTC, the label at 06:26):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+23.9 ± 9.0** | 100.0% | 1706 | 394 / 277 / 1035 | 2.96 | 4, 153, 452, 210, 34 | H1 |
| Fixed nodes, 200,000 per move | **+27.7 ± 9.7** | 100.0% | 1508 | 346 / 226 / 936 | 2.96 | 9, 112, 420, 176, 37 | H1 |

CI bench (base `v0.48.1`): signature 970,013 → 939,252; speed per node
−2.0% ± 11.4% and time to depth −0.6% ± 11.1% (a noisy runner; neither figure
says anything at that width); 4,033 instructions a node; fail-high-first 87.7%
(87.4%); EBF 1.90 (1.88, marked worse, inside the bar of 2); the performance
gate passed. Every CI check on #246 passed.

**The recorded gate is met:** H1 in both legs, one variant, chosen before the
run. The comment's reading: "smarter per node, and worth its cost at a clock:
merge." The agent does not merge.

**Reading it.**
- **The gain is the largest for a search change since Phase 4b,** and both
  legs agree, as a change that saves nodes at no cost per node should.
- **The screen under-read it by half** (+13.0 at 40,000 nodes): the deeper
  reductions are worth more plies the deeper the search. See the amendment to
  the screening rule above.
- **Not measured:** a time control longer than 5+0.05, where deeper null-move
  reductions meet more zugzwang-prone endgames and the engine has no
  verification search (match M1, below, asks); strength against other engines
  (self-play against the base overstates gains); a second variant (none was
  tried, by design).

#### Experiment 2: the table's score as the evaluation for whole-node pruning (#248)

Recorded on 2026-11-12 at 23:50 MST, before the `sprt` label was added.

| | |
|:--|:--|
| Question | A node that reaches reverse futility or the null move with a table entry has been searched before, to less depth. Is that search's score a better basis for the two decisions than the static evaluation? |
| The one change | `pruning_eval`: an exact table score replaces the corrected static evaluation, a lower bound only raises it, an upper bound only lowers it, a mate score never does; used in reverse futility's condition and return value and in the null move's `eval ≥ beta`. `feat/tt-score-as-pruning-eval` at `fd1e41f` against `main` at `0a45da0` |
| Held the same | Both rules' margins and conditions; futility pruning of single moves and the correction history, which keep the static evaluation; the null-move reduction (this branch does not contain #246); everything else |
| Why this variant | The standard form, with both directions of the bound rule, written down as H6 before any screen. One variant screened, one tested |
| Screen (exploratory) | `s1-tt-eval`: +15.8 ± 10.1 in 2,000 games at 40,000 nodes; the lab build with `L_TT_EVAL=1` has the PR's signature, 942699 |
| Bench, local | 970013 → 942699 (−2.8% nodes to depth 10); EBF 1.877 → 1.918 (bar 2.0, passes); **fail-high-first 0.8740 → 0.8532, under the bar of 0.87** |
| The bar | Not lowered, as decided before any run. The counters say why it fell: reverse-futility cutoffs +19,653, cutoffs by the table move −17,792, first-move cutoffs −14,926, later-move cutoffs +625 (16,471 → 17,096). The nodes removed are the ones that cut off on the table move, the mechanism `perf-targets.toml` records for reverse futility itself (#53). CI's `Bench` check fails on the PR; the PR gives the exception's wording for the owner to add or refuse |
| Expectation | Fixed nodes: +5 to +15. Timed: about the same; the change costs a few comparisons a node |
| Gate | H1 in both legs. Anything else drops the change. With H1 in both, merging still needs the owner's decision on the bar |
| Risks named beforehand | A shallow table score trusted at a deeper node: wrong cutoffs that grow with depth, so a gain at 40,000 nodes that shrinks at 200,000; interaction with #246 (both make whole-node pruning cut more) |
| Cost estimate | Two on-demand `c8a.16xlarge` for 10 to 50 minutes: $1.20 to $5.80 |

**Result** ([run 43381561111](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43381561111),
#248, `fd1e41f` against `main` at `0a45da0`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge`; one attempt; the plan's commit `e119227` at 06:50
UTC, the label at 06:51):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+16.5 ± 7.4** | 100.0% | 2714 | 609 / 480 / 1625 | 2.96 | 29, 254, 669, 369, 36 | H1 |
| Fixed nodes, 200,000 per move | +2.8 ± 3.7 | 93.1% | 10000 | 2168 / 2087 / 5745 | 1.10 | 82, 1140, 2461, 1249, 68 | Inconclusive at the 5,000-round cap |

CI bench (base `v0.48.1`): signature 970,013 → 942,699; speed per node
+0.9% ± 2.4%; time to depth −3.7% ± 2.3% (marked better); 4,014 instructions a
node; fail-high-first 85.3% (87.4%) and EBF 1.92 (1.88), both marked worse; **the
performance gate failed** on `efficiency.min_fail_high_first`, as the plan said
it would.

**The recorded gate is not met:** the fixed-node leg did not accept H1, so the
change is dropped, as the plan said. The comment's reading: "no smarter per
node, but faster: a speed gain at a clock, which fails the gate (both tests must
pass); the owner may merge over it for a speed change." #248 is left open as a
draft for that decision, which would also need the bar's exception; the agent
does not recommend merging it as it stands.

**Reading it.**
- **Per node it is worth little:** +2.8, interval −0.9 to +6.5. The screen's
  +15.8 at 40,000 nodes did not carry to 200,000: the risk the plan named, and
  the reverse of experiment 1.
- **At a clock it gained, by more than its speed explains.** 3.7% less time to
  a depth is a few Elo, not sixteen. The legs differ by 13.7 Elo with a standard
  error of about 4.2, so the difference is not noise, and nothing here explains
  it. Untested: that the rule is worth more in the long searches of a timed
  game's first moves (about 300 ms, several times the fixed-node budget).
- **Not tested:** the half of the rule that only lowers the evaluation, which
  keeps fail-high-first over its bar on the bench (0.872).

**Depth by phase of the game, measured afterwards** from both legs' games (the
`sprt-timed` and `sprt-nodes` artifacts; the mean depth fastchess recorded for
each engine's moves, and the mean time a move):

| Own moves | Timed: PR depth | Timed: base depth | Timed: time a move | Fixed nodes: PR depth | Fixed nodes: base depth | Fixed nodes: time a move |
|:--|--:|--:|--:|--:|--:|--:|
| 1–10 | 15.58 | 15.46 | 288 ms | 13.45 | 13.37 | 110 ms |
| 11–20 | 15.25 | 15.03 | 179 ms | 13.95 | 13.77 | 106 ms |
| 21–30 | 15.86 | 15.61 | 117 ms | 15.11 | 14.93 | 101 ms |
| 31–45 | 17.14 | 16.84 | 79 ms | 17.15 | 17.01 | 91 ms |
| 46 on | 20.80 | 20.00 | 52 ms | 22.38 | 21.56 | 75 ms |

- **The change buys depth in both legs,** a tenth of a ply early and about 0.8
  late, so the fixed-node leg's +2.8 is not for want of depth: the extra depth
  is bought with pruning that is right less often, and per node the two about
  cancel.
- **The two legs are not "the same search at different speeds".** A timed game
  gives its first ten moves 2.6 times the fixed-node leg's time (288 ms against
  110) and its late moves two thirds of it (52 ms against 75). So a change
  whose value depends on how long the search runs is weighed differently by
  the two legs. Whether that is what separates them here is match M10's
  question (#248 at 10+0.1), if the spend cap leaves room for it.

#### Experiment 3: futility pruning in quiescence, at the net's piece values (#249)

Recorded on 2026-11-13 at 00:12 MST, before the `sprt` label was added.

| | |
|:--|:--|
| Question | Half the search's nodes are quiescence nodes. Does skipping captures that cannot bring the score back to alpha gain Elo, once what a capture can gain is counted in the evaluation's own units? |
| The one change | In `quiescence`, out of check and not for a promotion, a capture is skipped when `stand pat + 300 + value(victim) ≤ alpha`, with the victim valued at 300, 775, 880, 1,100 and 1,680 (pawn to queen), the measured values in "The net's material scale". The skipped capture's reach becomes the node's bound. `feat/quiescence-futility` at `a6b722c` against `main` at `0a45da0` |
| Held the same | The stand pat, SEE pruning of losing captures, the table probe and stores, evasions in check, the main search, the net |
| Why this variant | The published rule (H4), with both of its numbers read off the measurement rather than tuned: the margin is one measured pawn, the values are the measured ones. A first form with the classical values and a margin of 150 was screened before the measurement and lost 106 Elo (`s1-qs-fut150`); that failure is what prompted the measurement. So two forms were screened and one, chosen by the measurement, is tested |
| Screen (exploratory) | `s2-qsfut-net300`: +12.7 ± 10.1 in 2,000 games at 40,000 nodes; the lab build with these options has the PR's signature, 990066 |
| Bench, local | 970013 → 990066 (+2.1% nodes to depth 10: the main search grew 5.8%, quiescence shrank 1.6%); fail-high-first 0.8740 → 0.8752; EBF 1.877 → 1.849; both bars pass; 69,881 captures skipped |
| Expectation | Fixed nodes: +5 to +12. Timed: about the same; the rule costs a table read and an addition a capture |
| Gate | H1 in both legs. Anything else drops the change |
| Risks named beforehand | The values are a property of this net and will drift with the next; endgames, where the published rule is usually switched off, and this one is not; a quiescence rule acts at the leaves of every search, so the 40,000-node screen should carry over better than experiment 2's did |
| Cost estimate | Two on-demand `c8a.16xlarge` for 15 to 50 minutes: $1.70 to $5.80. One leg waits for experiment 2's fixed-node leg to free its instance slot |

**Result** ([run 43384123736](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43384123736),
#249, `a6b722c` against `main` at `0a45da0`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge`; one attempt; the plan's commit `0403c4a` at 07:12:51
UTC, the label at 07:12:53; the fixed-node leg waited about 13 minutes for an
instance slot before it played):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | +3.6 ± 3.9 | 96.5% | 10000 | 2113 / 2010 / 5877 | 1.57 | 123, 1058, 2552, 1127, 140 | Inconclusive at the 5,000-round cap |
| Fixed nodes, 200,000 per move | **−10.8 ± 6.7** | 0.1% | 3290 | 641 / 743 / 1906 | −2.95 | 48, 400, 832, 336, 29 | H0 |

CI bench (base `v0.48.1`): signature 970,013 → 990,066; speed per node
+0.9% ± 2.0%; time to depth +1.2% ± 2.0%; 4,090 instructions a node;
fail-high-first 87.5%; EBF 1.85 (1.88, marked better); the performance gate
passed.

**The recorded gate is not met:** the fixed-node leg accepted H0, so the change
is dropped, as the plan said. The comment's reading: "no gain at a clock: do not
merge." #249 is left open as a draft so the record is easy to find; the
recommendation is to close it.

**Reading it.**
- **Per node the rule weakens the engine at 200,000 nodes,** where its screen
  read +12.7 ± 10.1 at 40,000. The two differ by 23 Elo, 3.8 standard errors.
  The plan's guess that a leaf rule would carry over from the shallow screen
  better than experiment 2's did was wrong.
- **At a clock it is about neutral.** A timed game gives about half its moves,
  the later ones, less search than 200,000 nodes (52% of the moves in
  experiment 2's timed leg took less time than the same phase's moves in its
  fixed-node leg), which is nearer the budget where the screen read well. Not
  tested.
- **What survives is the measurement** of the net's material scale, not the
  rule.

#### Experiment 4: the null-move reduction's base from 3 to 4, stacked on #246 (#250)

Recorded on 2026-11-13 at 00:32 MST, before the `sprt` label was added.

| | |
|:--|:--|
| Question | Experiment 1 moved the null-move reduction one step and gained. Is the best setting a further ply along the same axis? |
| The one change | `NULL_MOVE_REDUCTION` 3 → 4, so `R = 4 + depth/3 + min((eval − beta)/200, 3)`. `feat/null-move-reduction-base-4` at `25b1716`, **against #246's head `b54010f`**: the PR's base branch is #246's, so the SPRT's base is too. It depends on #246 |
| Held the same | The depth and evaluation divisors, the evaluation term's cap, null-move pruning's conditions, everything else |
| Why this variant | The next whole ply on the axis. It is the second value tried on this axis (3, then 4); no other was screened |
| Screen (exploratory, on the instance) | Match M2: +8.1, 99% interval −1.0 to +17.3, in 3,000 games at 200,000 nodes, the same edit as `research/nmp-base4` (`daf0970`) |
| Bench, local | 939252 → 925602 against #246 (−1.5% nodes to depth 10); fail-high-first 0.8773 → 0.8791; EBF 1.896 → 1.878; both bars pass |
| Expectation | Fixed nodes: +3 to +12. Timed: about the same. Near `elo1`, so a real gain of this size can still end at the cap without a verdict |
| Gate | H1 in both legs against #246's head. Anything else drops the change |
| Risks named beforehand | The screen's interval includes zero; zugzwang exposure grows with the reduction and there is no verification search; a gain at 5+0.05 that a longer control gives back |
| Cost estimate | Two on-demand `c8a.16xlarge` for 20 to 50 minutes: $2.30 to $5.80 |

**Result** ([run 43386646994](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43386646994),
#250, `25b1716` against #246's head `b54010f`; elo0 0, elo1 5, α = β = 0.05; on
demand; one attempt; the plan's commit `7962c1b` at 07:32:37 UTC, the label at
07:33:50):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | +1.5 ± 3.7 | 79.0% | 10000 | 1939 / 1895 / 6166 | 0.13 | 103, 1050, 2648, 1098, 101 | Inconclusive at the 5,000-round cap |
| Fixed nodes, 200,000 per move | — | — | 0 | — | — | — | Not played: the launch incident below |

The timed figures are from the leg's `sprt.md` and `sprt.log` in the
`sprt-timed` artifact. **The sticky comment on #250 does not show them:** it
reads "Gate: no result … re-run the SPRT", "Did not finish; see the job log"
for the timed leg and "No capacity" for the fixed-node one. The timed leg did
finish, at the cap; the comment job seems not to find a lone leg's log when the
other leg left no artifact (not investigated).

**The recorded gate is not met:** the timed leg did not accept H1, so the change
is dropped. By the rule written during the incident, before this result, the
fixed-node leg is not re-run. #250 is left open as a draft; the recommendation
is to close it.

**Reading it.** At a clock a fourth ply of null-move reduction adds nothing
measurable (interval −2.2 to +5.2); the only fixed-node evidence is match M2's
+8.1. Experiment 1's step showed at both. So on this axis the first step took
what there was at 5+0.05, and match M7, which asked where the axis turns over,
is withdrawn unrun: this answers it at a clock.

#### Experiment 5: futility pruning of captures in the main search (#252)

Recorded on 2026-11-13 at 01:34 MST, before the `sprt` label was added.

| | |
|:--|:--|
| Question | The search prunes hopeless quiet moves at shallow depth and searches every capture. Does pruning captures that cannot bring the score back gain Elo, once what a capture can gain is counted in the evaluation's own units? |
| The one change | In `negamax`'s move loop, at a null-window node not in check, a capture that is not the first move and not a promotion is skipped when `static eval + 300 + 200·depth + value(victim) ≤ alpha`, at depth ≤ 6, with the victim at 300, 775, 880, 1,100 and 1,680. `feat/capture-futility` at `cb6dfbc` against `main` at `0a45da0` |
| Held the same | Futility pruning of quiets, late move pruning, SEE pruning, reductions, quiescence, the net |
| Why this variant | The first form built: the margin is one measured pawn plus two thirds of one a ply, the depth limit is the quiet rule's, the values are the measured ones. One variant screened (three times at 40,000 nodes, once at 200,000) and one tested |
| Screen (exploratory, on the instance) | Match M6: **+12.2, 99% interval +3.2 to +21.2**, 3,000 games at 200,000 nodes; the lab build that played it has the PR's signature, 970114. At 40,000 nodes: +15.1 ± 9.5, +16.7 ± 9.4 and, on top of experiment 3's rule, +9.9 ± 9.8 |
| Bench, local | 970013 → 970114 (101 nodes more to depth 10); fail-high-first 0.8740 → 0.8740; EBF 1.877 → 1.898; both bars pass; 13,312 captures skipped |
| Expectation | Fixed nodes: +5 to +15, from M6. Timed: about the same; the rule costs a table read and an addition a capture |
| Gate | H1 in both legs. Anything else drops the change |
| Risks named beforehand | Experiment 3 was the same idea in quiescence and lost at 200,000 nodes; this one has been screened there and read positive, which is a reason to test it and not a reason to expect it to pass. M6 is the best of the five 200,000-node screens read so far (M2 to M6), so its +12.2 is inflated by that choice; the SPRT is a fresh sample. (The first push of this row said "nine matches", a miscount, corrected in the next commit.) The values belong to this net |
| Cost estimate | Two on-demand `c8a.16xlarge` for 15 to 50 minutes: $1.70 to $5.80. No other SPRT is running; a match and the training run are, which leaves room under the account's quota |

**Result** ([run 43394229688](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43394229688),
#252, `cb6dfbc` against `main` at `0a45da0`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge`; one attempt; the plan's commit `08ec8d8` at 08:34:03
UTC, the label at 08:34:06):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | +4.9 ± 3.8 | 99.4% | 10000 | 2050 / 1908 / 6042 | 2.61 | 105, 1071, 2512, 1201, 111 | Inconclusive at the 5,000-round cap |
| Fixed nodes, 200,000 per move | **+17.7 ± 7.9** | 100.0% | 2668 | 642 / 506 / 1520 | 2.95 | 36, 263, 613, 373, 49 | H1 |

CI bench (base `v0.48.1`): signature 970,013 → 970,114; speed per node
+0.8% ± 10.1% and time to depth −0.2% ± 9.9% (a noisy runner: no information
about cost at that width); 4,124 instructions a node; fail-high-first 87.4%;
EBF 1.90 (1.88, marked worse, inside the bar); the performance gate passed.

**The recorded gate is not met:** the timed leg reached the cap with LLR 2.61
of the 2.94 it needed. By the plan the change does not pass. The comment's
reading: "smarter per node, but the extra cost per node eats the gain at a
clock: do not merge; worth a `perf` pass to make it cheaper and a re-test."

**Reading it.**
- **Per node the gain is real and repeated:** +17.7 ± 7.9 here, +12.2 in match
  M6 at the same budget, three positive screens at 40,000 nodes.
- **At a clock it is small and probably not zero:** +4.9 with a 95% interval of
  +1.1 to +8.7. `elo1` is 5, and a true gain of 5 reaches the cap without a
  verdict about half the time; this is that case, not evidence of no gain.
- **The comment's stock explanation, a dearer node, is not supported by a
  measurement:** the bench's speed figures are ±10% and say nothing. The rule
  adds a table read and an addition for a capture at a shallow node. Why a
  change worth 17 per node is worth 5 at a clock is not known; experiment 4
  showed the same shape (+8.1 in a fixed-node match, +1.5 timed), and
  experiment 2 the opposite one.
- **The SPRT is not re-run.** A second attempt at the same test after an
  inconclusive first is a second chance at a 5% error, and the first 10,000
  games would be thrown away. More games are what the verdict asks for, so
  they are added as a fixed-size measurement instead (match M18, below) and
  read together with these.

**Played again against `v0.51.0`** after the session's three gains had merged:
see [Experiment 5 again](#experiment-5-again-capture-futility-against-v0510-252). The results above are the first run's and
match M18's, and are not changed by it.

#### Experiment 6: more late-move reduction at expected cut nodes (#254)

Recorded on 2026-11-13 at 02:28 MST, before the `sprt` label was added.

| | |
|:--|:--|
| Question | Late move reductions treat every node off the principal variation alike. Does reducing late quiet moves one ply more at nodes expected to fail high gain Elo? |
| The one change | `negamax` carries `cut_node` (the root is not one; a full-window child never is; a reduced search's child always is; any other null-window child is the opposite of its parent), and a late quiet move's reduction is one ply larger when it is set. `feat/lmr-cut-node` at `0e188c9` against `main` at `0a45da0` |
| Held the same | The reduction table, the history adjustment, which moves are reduced, the re-search rule, every pruning rule, the net |
| Why this variant | The standard form (H3's first half), one ply. The other half, less reduction at principal-variation nodes, grew the bench tree by 59% and was not screened. One variant tested |
| Screen (exploratory, on the instance) | Match M11: **+27.3, 99% interval +17.9 to +36.6**, 3,000 games at 200,000 nodes; the lab build that played it has the PR's signature, 988822. **At 40,000 nodes the same change read about −12** (`s1-lmr-cut`, whose log was lost), and it was dropped until the third amendment sent the dismissed changes back to be screened at the right budget |
| Bench, local | 970013 → 988822 (+1.9% nodes to depth 10: the main search 2.5% smaller, quiescence 6.2% larger); fail-high-first 0.8740 → 0.8755; EBF 1.877 → 1.861; both bars pass; 1.9% of reduced moves re-searched (1.8%) |
| Expectation | Fixed nodes: +10 to +25, from M11 less what picking the best of the nine 200,000-node matches read so far adds (M2 to M6, M11, M12, M16, M17; the first push of this row said "eight"). Timed: uncertain. The change costs nothing per node, but experiments 4 and 5 both kept less at a clock than at fixed nodes, and experiment 1 kept all of it |
| Gate | H1 in both legs. Anything else drops the change |
| Risks named beforehand | A reduction worth more with depth should be worth as much or more at a clock's deeper early searches, as experiment 1's was; if the timed leg reads well below the fixed-node one, that is the third change tonight to do so and says something about the two legs rather than about this change. A deeper reduction can miss tactics a re-search does not catch |
| Cost estimate | Two on-demand `c8a.16xlarge` for 10 to 50 minutes: $1.20 to $5.80. No other SPRT is running |

**Result** ([run 43401686917](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43401686917),
#254, `0e188c9` against `main` at `0a45da0`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge`; one attempt; the plan's commit `e972072` at 09:28:55
UTC, the label at 09:29:01):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+9.8 ± 5.6** | 100.0% | 5004 | 1070 / 929 / 3005 | 2.97 | 62, 507, 1245, 604, 84 | H1 |
| Fixed nodes, 200,000 per move | **+10.6 ± 5.8** | 100.0% | 4322 | 947 / 815 / 2560 | 2.95 | 49, 408, 1117, 536, 51 | H1 |

CI bench (base `v0.48.1`): signature 970,013 → 988,822; speed per node
+0.1% ± 6.8%; time to depth +2.1% ± 6.9%; 4,069 instructions a node;
fail-high-first 87.6%; EBF 1.86; the performance gate passed, as did every CI
check on #254.

**The recorded gate is met:** H1 in both legs, one variant. The comment's
reading: "smarter per node, and worth its cost at a clock: merge." The agent
does not merge.

**Reading it.**
- **About +10 at both, and the legs agree.**
- **The screen over-read it by more than half:** M11 said +27.3 with a 99%
  interval of +17.9 to +36.6, and the SPRT's fixed-node leg, the same test on
  fresh games, says +10.6 ± 5.8. The interval of the best of nine matches is not
  an interval for the change. Experiment 5's screen (+12.2) held up (+17.7);
  this one shrank; both passed their fixed-node legs.
- **A change dropped at 40,000 nodes for reading −12 is worth +10.** That is the
  session's clearest evidence for the third amendment.
- **Not measured:** how it stacks with experiment 1. Both were tested against
  `main`; both make the search look less at what it expects to fail.

**Played again on top of experiment 1** after #246 merged: see
[Follow-up 2026-11-13](#follow-up-2026-11-13-experiment-6-again-on-top-of-246). The result above is the first run's and is
not changed by it.

#### Experiment 7: continuation history four plies back (#255)

Recorded on 2026-11-13 at 02:41 MST, before the `sprt` label was added (which
waits for experiment 6's SPRT to finish and its instances to terminate).

| | |
|:--|:--|
| Question | The search scores a quiet move by what was played one and two plies back. Does also knowing the move four plies back, our own move before last, order quiets and steer reductions better? |
| The one change | A third `Continuation` table keyed by the move four plies back, read in `quiet_score` (so in quiet ordering and in the LMR history term) and updated with the same bonus and malus as the others. `feat/continuation-history-4` at `1bb7459` against `main` at `0a45da0` |
| Held the same | The butterfly history, the two existing tables, the bonus, gravity and aging, the LMR history divisor, everything else |
| Why this variant | The usual next distance (H8's neighbour in the first ranking; it was a lab toggle from the start). One variant, screened once, at 200,000 nodes |
| Screen (exploratory, on the instance) | Match M8: **+11.6, 99% interval +1.9 to +21.2**, 3,000 games at 200,000 nodes; the lab build that played it has the PR's signature, 1025352 |
| Bench, local | 970013 → 1025352 (+5.7% nodes to depth 10); fail-high-first 0.8740 → 0.8781; EBF 1.877 → 1.875; both bars pass |
| Expectation | Fixed nodes: +5 to +12. Timed: less. Each quiet scored costs one more lookup in a 1.2 MB table, and continuation history one and two plies back was itself +18 at fixed nodes and neutral in a 10+0.1 match (#79). The likeliest outcome is the one the gate exists for: H1 per node, not at a clock |
| Gate | H1 in both legs. Anything else drops the change |
| Risks named beforehand | The per-node cost, above. M8 is the third-best of ten 200,000-node matches read so far (M2 to M6, M8, M11, M12, M16, M17), so it carries some of the same inflation as the others chosen that way |
| Cost estimate | Two on-demand `c8a.16xlarge` for 20 to 50 minutes: $2.30 to $5.80 |

**Result** ([run 43427691305](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43427691305),
#255, `1bb7459` against `main` at `0a45da0`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge`; one attempt; the plan's commit `50f6049` at 09:41
UTC, the label at 12:44:10, after the gap in the session):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | +2.1 ± 3.8 | 86.5% | 10000 | 1952 / 1891 / 6157 | 0.55 | 103, 1075, 2593, 1116, 113 | Inconclusive at the 5,000-round cap |
| Fixed nodes, 200,000 per move | −1.2 ± 4.0 | 28.1% | 10000 | 2019 / 2053 / 5928 | −1.87 | 100, 1305, 2258, 1203, 134 | Inconclusive at the 5,000-round cap |

CI bench (base `v0.48.1`): signature 970,013 → 1,025,352; **speed per node
−6.1% ± 3.2%, time to depth +12.7% ± 3.9%** (marked worse); 4,257 instructions
a node; fail-high-first 87.8%; EBF 1.88; the performance gate passed.

**The recorded gate is not met:** neither leg accepted H1, so the change is
dropped. The comment's reading: "no gain at a clock: do not merge." #255 is a
draft; the recommendation is to close it.

**Reading it.** Nothing per node (−1.2 ± 4.0 over 10,000 games, against M8's
+11.6 over 3,000) and nothing at a clock, with a node 6% dearer, the cost the
plan named. M8 was the third-best of ten matches; its interval was not an
interval for the change. Of the three changes taken from a 200,000-node match
to an SPRT, one held (experiment 5: +12.2, then +17.7), one shrank and passed
(experiment 6: +27.3, then +10.6) and this one vanished.

#### Match M1: experiment 1 at 10+0.1

Recorded on 2026-11-12 at 23:41 MST, before the dispatch.

| | |
|:--|:--|
| Question | Does #246's gain hold at twice the SPRT's time control? A null-move reduction this deep is the kind of change that can win at 5+0.05 and give some back at a longer control |
| What runs | The `Match` workflow from `main`: candidate `feat/null-move-adaptive-reduction` (`b54010f`) against `main` (`0a45da0`), 10+0.1, 2,000 games, hash 64 MB, 99% interval; one on-demand `c8a.16xlarge` (the `match` profile) |
| A measurement, not a gate | The SPRT above is the gate and it is met. This informs the owner's merge decision |
| Read beforehand | Positive with a 99% interval clear of zero: the gain holds. An interval that includes zero: a flag on the PR, and a reason for a longer match before merging. Negative: reported on the PR as a reason not to merge yet |
| Cost estimate | About 20 minutes of one instance at $3.45 an hour: $1.20 |

**Attempts.**

| Run | Started (UTC) | Outcome |
|:--|:--|:--|
| [43380319041](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43380319041) | 2026-11-13 06:40 | **Failed at instance setup, no games played:** `rustup` could not download `channel-rust-1.99.1.toml.sha256` from static.rust-lang.org, so the task never started; the instance (`i-02961d40797a3deda`) terminated itself after about three minutes. A network failure, not the workflow or the engine |
| [43380894175](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43380894175) | 06:45 | The same inputs again. Completed: 2,000 games in 1,111 s of play |

**Result** (run 43380894175's report; `b54010f`, bench 939,252, against `main` at
`0a45da0`, bench 970,013; 10+0.1, hash 64 MB, 60 games at once on an AMD EPYC
9R45, fastchess `v1.8.2-alpha`):

| Elo (candidate − baseline) | 99% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) |
|--:|:-:|--:|--:|:-:|:--|
| **+29.4** | +18.8 to +40.1 | 100.0% | 2000 | 442 / 273 / 1285 | 7, 155, 534, 270, 34 |

**Reading it, by the rule written beforehand:** positive with a 99% interval
clear of zero, so the gain holds at twice the SPRT's time control, and is no
smaller there (+23.9 ± 9.0 at 5+0.05). The zugzwang risk named in experiment
1's plan did not show at this control. Not measured: anything longer than
10+0.1, and other engines.

#### Match M2: one more ply of null-move reduction, at 200,000 nodes

Recorded on 2026-11-13 at 00:01 MST, before the dispatch. **Exploratory:** a
screen on the standardized instance, under the amended rule for changes that
act at depth.

| | |
|:--|:--|
| Question | Experiment 1 moved the null-move reduction one step and gained +24 to +28. Is the best setting further along the same axis? |
| What runs | `Match` from `main`: candidate `research/nmp-base4` (`daf0970`: `NULL_MOVE_REDUCTION` 4, nothing else) against `feat/null-move-adaptive-reduction` (`b54010f`, #246), 200,000 nodes a move, 3,000 games, hash 64 MB, 99% interval; one on-demand `c8a.16xlarge` |
| Read beforehand | +8 or more: a PR stacked on #246, with its own plan and SPRT against #246's head. Between 0 and +8: recorded, no PR. Below 0: dropped |
| Why not straight to an SPRT | It is the second value tried on one axis: a screen first keeps the SPRT a test of one pre-chosen variant |
| Cost estimate | About 14 minutes of one instance: $0.80 |

**Attempts.**

| Run | Started (UTC) | Outcome |
|:--|:--|:--|
| [43383364015](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43383364015) | 2026-11-13 07:06 | **Failed in the settings job, before any instance:** `rustc` crashed inside the `serde_derive` proc macro while building `match-report` on the GitHub-hosted runner ("could not compile `rusty-chess-search`"). The same job had passed twice 25 minutes earlier, so this reads as a runner or build-cache fault, not the branch. No cost |
| [43384379664](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43384379664) | 07:15 | The same inputs again. Completed: 3,000 games in 600 s of play |

**Result** (run 43384379664's report; `daf0970`, bench 925,602, against
`b54010f`, bench 939,252; 200,000 nodes a move, hash 64 MB):

| Elo (candidate − baseline) | 99% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) |
|--:|:-:|--:|--:|:-:|:--|
| +8.1 | −1.0 to +17.3 | 98.9% | 3000 | 640 / 570 / 1790 | 25, 315, 770, 345, 45 |

**By the rule written beforehand** (+8 or more: a stacked PR with its own SPRT)
it goes forward, by a tenth of an Elo, as experiment 4. The report's own
reading is "no significant difference" at 99%: this is a screen, and the SPRT
is the test.

#### Matches M3 to M6: screens on the instance at 200,000 nodes

Recorded on 2026-11-13 at 00:34 MST, before the first dispatch. **Exploratory**,
under the third amendment to the screening rule.

| Match | Candidate (a lab branch whose defaults carry one change), against `main` | The change |
|:--|:--|:--|
| M3 | `research/lab-se-double20` (`debe861`, bench 1,058,291) | Singular extensions: a second ply when the singular search fails low by 20 more than its margin, at most 6 times a line, not at PV nodes |
| M4 | `research/lab-probcut200s3` (`41c856e`, bench 961,758) | ProbCut at depth ≥ 5: a capture whose exchange wins at least (beta + 200 − eval)/3 in classical units, searched at depth − 4 against beta + 200 |
| M5 | `research/lab-se-neg` (`add0ae5`, bench 947,856) | Singular extensions: one ply less for the table move when it is not singular and the table score is at or above beta, or the node is an expected cut node |
| M6 | A lab branch with `CAPFUT=200` (made before its dispatch) | Futility pruning of captures in the main search at depth ≤ 6, the victim at the net's scale: the rule `s4-capfut200` and `c2-capfut-on-qsfut` read +15.1 and +9.9 for at 40,000 nodes |

| | |
|:--|:--|
| What runs | `Match` from `main`, one at a time: 200,000 nodes a move, 3,000 games, hash 64 MB, 99% interval; one on-demand `c8a.16xlarge` each |
| The lab build as candidate | It has runtime parameter reads the engine doesn't, so it is a little slower a node; at fixed nodes that changes nothing. With no parameter set it has `main`'s bench signature |
| Read beforehand | +8 or more: built cleanly on a branch from `main`, with a plan and an SPRT. Under +8: recorded, no PR. The order is fixed: M3, M4, M5, M6; a match is skipped only if the spend cap requires it, and that is recorded |
| Cost estimate | About $0.70 each, $2.80 in all |

**Attempts.**

| Match | Run | Started (UTC) | Outcome |
|:--|:--|:--|:--|
| M3 | [43386732727](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43386732727) | 2026-11-13 07:34 | **Failed in the settings job, before any instance:** `rustc interrupted by SIGILL` compiling `rusty-chess-search`. The second failure of this kind tonight, and the one that explains both: `match.yml` sets no `RUSTFLAGS`, so its GitHub-hosted jobs build for the runner's own CPU and `rust-cache` restores that build on a different CPU. #251 proposes the one-line fix. No cost |
| M3 | [43387376307](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43387376307) | 07:39 | The same inputs again. Completed: 3,000 games in 630 s of play |
| M4 | [43388091282](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43388091282) | 07:46 | Dispatched behind M3; the settings job passed at the first attempt. It waited for M5 and launched at 08:03. Completed: 3,000 games in 630 s of play |
| M5 | [43388637501](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43388637501) | 07:51 | Dispatched behind M4; the settings job passed at the first attempt. It took the instance slot before M4 did (the slot is not first come, first served). Completed: 3,000 games in 601 s of play |

**Results** (each run's report; 200,000 nodes a move, hash 64 MB, against `main`
at `0a45da0`, bench 970,013):

| Match | Elo (candidate − baseline) | 99% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) | By the rule |
|:--|--:|:-:|--:|--:|:-:|:--|:--|
| M3, double singular extension | +5.2 | −4.0 to +14.4 | 92.7% | 3000 | 585 / 540 / 1875 | 35, 310, 770, 345, 40 | Under +8: recorded, no PR |
| M5, negative singular extension | +6.4 | −3.7 to +16.4 | 94.9% | 3000 | 695 / 640 / 1665 | 45, 350, 665, 385, 55 | Under +8: recorded, no PR |
| M4, ProbCut with a capture filter | −10.4 | −20.2 to −0.7 | 0.3% | 3000 | 600 / 690 / 1710 | 55, 380, 700, 330, 35 | Weaker, with a 99% interval below zero: **dropped** |
| M6, capture futility in the main search | **+12.2** | +3.2 to +21.2 | 100.0% | 3000 | 685 / 580 / 1735 | 25, 295, 765, 380, 35 | **+8 or more: goes forward as experiment 5 (#252)** |
<!-- matches:m3-m6 -->

M6 ran as [43391890629](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43391890629)
(`research/lab-capfut200`, `02ff8a9`, bench 970,114), dispatched at 08:16 UTC by
the queue runner, the settings job passing at the first attempt; 3,000 games in
660 s of play.

#### Matches M7 to M10

Recorded on 2026-11-13 at 00:50 MST, before any of them was dispatched. They
run after M6, one at a time, in the order M10, M7, M8, M9.

| Match | Candidate against baseline | Settings | The question, and what the answer becomes |
|:--|:--|:--|:--|
| M10 | `feat/tt-score-as-pruning-eval` (`fd1e41f`, #248) against `main` | 10+0.1, 2,000 games | **A measurement for the owner's decision on #248,** as M1 was for #246: does the timed gain (+16.5 ± 7.4 at 5+0.05) hold at twice the control? Predicted beforehand: smaller, because the fixed-node leg says the change is worth little per node and a longer control is more nodes a move. At +8 or less with an interval that includes zero, the reading is that the gain belongs to short searches and the gate was right; at +16 or more, that reading is wrong and the gain is a real one at a clock |
| M7 | `research/lab-nmp5` (`df48070`: null-move base 5, divisors as #246) against `feat/null-move-reduction-base-4` (`25b1716`, #250) | 200,000 nodes, 3,000 games | Where the null-move axis turns over. **Information only: no PR tonight whatever it reads,** since it would be a second PR stacked on #246 and a third value on one axis |
| M8 | `research/lab-cont4` (`7606d2c`) against `main` | 200,000 nodes, 3,000 games | Continuation history four plies back, in quiet ordering and the LMR history term. +8 or more: a clean branch, a plan and an SPRT |
| M9 | `research/lab-caphist8` (`ccbf034`) against `main` | 200,000 nodes, 3,000 games | Capture history added to capture ordering (an eighth of its score beside MVV-LVA). +8 or more: a clean branch, a plan and an SPRT |

Cost estimate: $1.20 for M10 and about $0.70 each for the others, $3.30 in all.
Any of them is dropped, and recorded as not run, if the spend cap requires it.

**Results so far** (each run's report; 200,000 nodes a move, hash 64 MB, against
`main` at `0a45da0`, bench 970,013):

| Match | Run | Elo (candidate − baseline) | 99% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) | By the rule |
|:--|:--|--:|:-:|--:|--:|:-:|:--|:--|
| M8, continuation history four plies back | [43400673040](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43400673040) | **+11.6** | +1.9 to +21.2 | 99.9% | 3000 | 705 / 605 / 1690 | 50, 285, 720, 405, 40 | **+8 or more: goes forward as experiment 7 (#255)** |
<!-- matches:m7-m10 -->

M7 was withdrawn unrun (experiment 4's result made it moot). **M9, M15 and M10
were withdrawn unrun at 05:45 MST for the spend cap:** about $28.40 had been
spent, and the two SPRTs still owed (experiment 7 and the net of training T1)
can cost $10.60 between them if both run to the cap. So capture history in
ordering (M9), both singular-extension variants together (M15) and #248 at
10+0.1 (M10) were not measured; the lab branches for the first two are pushed
(`research/lab-caphist8`, `research/lab-se-both`).

**A gap in the session.** Between 02:50 and 05:42 MST the agent was not
running (its session needed re-authentication). Runs already dispatched
finished on their own; nothing new was started in that time, which is why
experiment 7's SPRT, planned for about 02:55, started at 05:44.

#### Matches M11 to M14: what the 40,000-node screens dismissed

Recorded on 2026-11-13 at 01:03 MST, before any of them was dispatched.

**Why.** The third amendment says a 40,000-node screen cannot judge a change at
the SPRT's budget, in either direction. It applies to the screens that said no
as much as to the ones that said yes: the improving heuristic and the cut-node
reduction were dropped on readings of −5.9 to −11.8 at 40,000 nodes, and
experiment 1 read half its real size there. The improving heuristic is the
roadmap's named item to revisit with a network, so it gets a test at the right
budget rather than a dismissal at the wrong one.

| Match | Candidate (a lab branch whose default carries one change), against `main` | The change | Its 40,000-node screen |
|:--|:--|:--|:--|
| M11 | `research/lab-lmr-cut` (`e77459a`, bench 988,822) | One more ply of late-move reduction at expected cut nodes | −11.8 ± 10.5 at 1,796 games (log lost) |
| M12 | `research/lab-rfp-improving` (`d3dcc6b`, bench 879,149) | Reverse futility with one ply less margin when improving | −9.9 ± 10.6 |
| M13 | `research/lab-lmr-improving` (`1bc202d`, bench 851,461) | One more ply of late-move reduction when not improving | −5.9 ± 10.0 |
| M14 | `research/lab-lmp-improving` (`98deaa7`, bench 896,900) | Late move pruning from half the index when not improving | Not screened |

**Results** (each run's report):

| Match | Run | Elo (candidate − baseline) | 99% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) | By the rule |
|:--|:--|--:|:-:|--:|--:|:-:|:--|:--|
| M12, reverse futility with improving | [43397194328](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43397194328) | −12.2 | −22.2 to −2.2 | 0.1% | 3000 | 580 / 685 / 1735 | 70, 365, 705, 320, 40 | Weaker, the 99% interval below zero: **dropped**, now on a measurement at the SPRT's budget |
| M11, one more ply of LMR at expected cut nodes | [43398918563](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43398918563) | **+27.3** | +17.9 to +36.6 | 100.0% | 3000 | 715 / 480 / 1805 | 25, 265, 705, 460, 45 | **+8 or more: goes forward as experiment 6 (#254).** Its 40,000-node screen had read about −12 |
<!-- matches:m11-m14 -->

M13 and M14 (the improving heuristic at LMR and at late move pruning) were
withdrawn unrun at 02:21 MST to pay for match M18; see its plan.

Each: `Match` from `main`, 200,000 nodes a move, 3,000 games, hash 64 MB. +8 or
more: a clean branch, a plan and an SPRT. Under +8: recorded, no PR; for the
three improving sites that closes the roadmap's "revisit" with a measurement at
200,000 nodes, whichever way it reads. About $0.70 each.

**Match M15, added at 01:05 MST after M3 and M5 and because of them.** The
two singular-extension halves read +5.2 and +6.4, each under the threshold and
each with an interval that includes zero. Together they are the usual modern
form of the technique, and together they might clear what neither does alone.
That hypothesis was formed after seeing both results, and is recorded as such.
M15: `research/lab-se-both` (`71713cb`, bench 1,081,890: `SE_DOUBLE=20` and
`SE_NEG=1`) against `main`, 200,000 nodes, 3,000 games, after M11. +8 or more:
a clean branch, a plan and an SPRT.

**Performance bars of the probes** (local `perf-gate` on the lab build, depth-10
bench), recorded before their matches so a crossing is known in advance: the
double extension reads EBF 2.012 and both halves together 2.051, over the bar
of 2.0; reverse futility with improving (M12) reads EBF 2.040, also over it.
All the others pass both bars. As decided at the start, a bar is not lowered
by the agent: one of these three going forward would be a PR whose `Bench`
check fails, for the owner to decide, as #248 is.

**The order of the remaining matches, amended at 01:03 MST** (the plans above
gave M10, M7, M8, M9): M6, M12, M11, M8, M9, M10, M13, M14, M7. The ones that
can become a PR tonight go first, since each needs an SPRT after it; M10 and M7
are measurements for the owner and lose nothing by running later. Every match
planned is still run unless the spend cap stops it, and a stop is recorded.

#### Matches M16 and M17: which way does the search want its margins?

Recorded on 2026-11-13 at 01:29 MST, before either was dispatched.

| | |
|:--|:--|
| Question | Every evaluation-based margin in the search is now about 3.4 times tighter in pawns than when it was tuned, and tightened by another fifth with net7rl012. Taken together, are they too tight, too loose, or about right for net8? |
| The probe | One lab parameter, `EVAL_PCT`, scales the evaluation as the search reads it. At 125 every margin is a fifth tighter at once (reverse futility, futility, the aspiration window, the singular margin, the correction history's cap, the null move's evaluation term); at 80, a quarter looser. It is exactly equivalent to rescaling those constants together, up to rounding |
| M16 | `research/lab-eval125` (`d4bb6a5`, bench 942,637) against `main`, 200,000 nodes, 3,000 games |
| M17 | `research/lab-eval80` (`63dca34`, bench 1,170,683; EBF 2.005 on the bench, a hair over the bar) against `main`, 200,000 nodes, 3,000 games |
| Read beforehand | One side at +8 or more and the other at or below zero: the margins are off in that direction, and the change becomes a PR that rescales the constants themselves (not the evaluation: scaling the evaluation would also scale the scores `datagen` records as labels), with its own match to confirm the rescaled constants behave as the probe did, then a plan and an SPRT. Both within ±8: the margins as a set are about right at net8's scale, and the reading that net7rl012's gain was a margin effect loses its support. Both positive or both negative is not expected and would be recorded as it is |
| Not a feature | The probe also scales the scores the search reports, so it is never a PR itself |
| Cost estimate | About $0.70 each. They run next, ahead of the matches still queued: this question came out of the session's measurement and is its best-motivated one |

**Results** (each run's report; 200,000 nodes a move, hash 64 MB, against `main`
at `0a45da0`, bench 970,013):

| Match | Run | Elo (candidate − baseline) | 99% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) |
|:--|:--|--:|:-:|--:|--:|:-:|:--|
| M16, evaluation at 125% (margins a fifth tighter) | [43393664014](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43393664014) | −0.6 | −10.0 to +8.8 | 43.7% | 3000 | 640 / 645 / 1715 | 45, 340, 715, 375, 25 |
| M17, evaluation at 80% (margins a quarter looser) | [43395387426](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43395387426) | +0.6 | −8.3 to +9.5 | 56.7% | 3000 | 640 / 635 / 1725 | 25, 345, 750, 360, 20 |

**By the reading written beforehand: both within ±8, so the margins as a set
are about right at net8's scale.** A fifth tighter and a quarter looser are the
same engine to within 9 or 10 Elo either way, at the SPRT's fixed-node budget.

- **The roadmap's "re-tune what was tuned to PeSTO's scores" has its first
  measurement, and it is flat** for the evaluation-based margins taken
  together. The constants are 3.4 times tighter in pawns than when they were
  tuned and the engine does not care about another quarter either way. One
  constant could still be wrong in a way the set hides (SPSA run S1 looks
  constant by constant), but there is no direction here to chase.
- **The second reading of net7rl012's gain loses its support:** making net8's
  evaluation a quarter larger, which is what that net's relabelling did to the
  pieces, gains nothing. The ledger's reading, better labels, stands.
- **It is not a statement about a clock.** Both probes change how many nodes a
  depth costs (−2.8% and +20.7% on the bench), which a fixed-node match does
  not charge for. The looser one would likely lose at a clock.
- Each cost about $0.75 (the tasks ran 649 s and 680 s).

#### Match M18: more timed games for experiment 5

Recorded on 2026-11-13 at 02:21 MST, before the dispatch.

| | |
|:--|:--|
| Question | The timed leg ended at the cap at +4.9 ± 3.8. With twice the games, is the timed gain clear of zero? |
| What runs | `Match` from `main`: `feat/capture-futility` (`cb6dfbc`) against `main`, **5+0.05, 10,000 games, hash 16 MB** (the SPRT's settings, where a match's default hash is 64), 95% interval; one on-demand `c8a.16xlarge` |
| How it is read, fixed here | The two samples (the SPRT's timed leg and this match) are pooled by inverse variance; they are independent games at the same settings. **A pooled 95% interval that excludes zero** says the change gains at a clock by a small amount: with the fixed-node H1 that is the "inconclusive at high LOS" case in which the repository's rules let the owner merge over a failed check, and #252 is put to him as that. **An interval that includes zero** says the timed gain is not shown, and the recommendation is to drop it. Either way the gate as run stays not met; this match does not change the SPRT's verdict |
| Cost estimate | About 47 minutes of one instance: $2.70. To pay for it, matches M13 and M14 (the improving heuristic at LMR and at late move pruning) are withdrawn unrun: M12 has just read −12.2 for the heuristic at its main site, they could not become PRs under the bars' rules without it, and they were the lowest-ranked in the queue |

**Attempts.** [43402595891](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43402595891),
dispatched at 09:36 UTC once M8 had finished; the settings job passed at the
first attempt. Completed: 10,000 games in 2,611 s of play.

**Result** (the run's report; `cb6dfbc`, bench 970,114, against `main` at
`0a45da0`, bench 970,013; 5+0.05, hash 16 MB, 60 games at once):

| Sample | Elo | 95% interval | LOS | Games | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) |
|:--|--:|:-:|--:|--:|:-:|:--|
| The SPRT's timed leg | +4.9 | +1.1 to +8.7 | 99.4% | 10000 | 2050 / 1908 / 6042 | 105, 1071, 2512, 1201, 111 |
| Match M18 | +3.0 | −0.8 to +6.9 | 93.8% | 10000 | 2016 / 1929 / 6055 | 120, 1071, 2530, 1160, 119 |
| **Pooled** | **+4.0** | **+1.3 to +6.7** | | 20000 | 4066 / 3837 / 12097 | 225, 2142, 5042, 2361, 230 |

The pooled row is computed from the summed pentanomial counts (the score of a
pair and its variance over 10,000 pairs); pooling the two estimates by inverse
variance, as the plan worded it, gives the same +4.0 and +1.3 to +6.7. The same
calculation reproduces each sample's reported figures.

**By the reading fixed beforehand: the pooled 95% interval excludes zero.** The
change gains at a clock by a small amount, about +4, and per node by more
(+17.7 ± 7.9 in the SPRT, +12.2 in M6). M18 alone would not have said so: its
own interval includes zero, and it read lower than the SPRT's leg. **The gate
as run is still not met.** #252 is put to the owner as the case the
repository's rules name, an inconclusive result at high LOS, for him to merge
over or not; the agent's own reading is that it is a real but small gain,
about the size `elo1` cannot reliably confirm.

#### SPSA run S1: the margins and reduction constants together

Recorded on 2026-11-13 at 00:01 MST, before the run. **Exploratory and local.**

| | |
|:--|:--|
| Question | The roadmap's open item: re-tune what was tuned to PeSTO's scores. Is there a jointly better setting of the search's margins and reduction constants for net8, on top of #246? |
| Method | SPSA (Spall): every iteration perturbs all parameters at once by ±c in a random direction, plays 12 game pairs between the two perturbed engines, and moves each parameter along its perturbation in proportion to wins less losses. Gains decay as a/(A + k)^0.602 and c/k^0.101 with A a tenth of the iterations; the end-of-run learning rate is 0.003. `lab/spsa.py` on `research/overnight-lab`, lab binary `lab5`, seed 20261113 |
| Games | 40,000 nodes a move, the SPRT's openings and adjudication; 3,000 iterations of 24 games, 72,000 games, or until 05:15 MST, whichever comes first. The stopping rule is fixed here; the parameters at the stop are the result |
| Base | `main` with #246's null-move reduction in a fractional form, `R = (192 + 22·depth)/64 + min((eval − beta)/200, 3)`, which has #246's bench signature (939252) |
| Parameters (start; range; final perturbation) | Reverse futility margin (80; 40–150; 8) and depth (6; 3–10; 0.5). Futility base (100; 0–260; 14), margin (100; 40–200; 10) and depth (6; 3–10; 0.5). Late move pruning base (3; 1–8; 0.5), depth (4; 2–9; 0.5) and scale (100%; 40–170; 8). SEE quiet margin (64; 20–140; 8) and depth (8; 4–12; 0.5). LMR table base (0.75; 0.20–1.50; 0.08) and divisor (2.25; 1.50–3.30; 0.12), and its history divisor (512; 128–2,048; 48). Aspiration half-width (35; 8–80; 4). Null move: base (192/64; 96–330; 12), depth slope (22/64; 8–44; 2) and evaluation divisor (200; 80–400; 20) |
| What the result becomes | The parameters at the stop, rounded, first play a fresh local screen against the base (2,000 games at 40,000 nodes, not the tuning games). At +8 or more they become one PR stacked on #246, gate H1 in both SPRT legs against #246's head. Below that, the run is recorded and no PR is opened |
| Risks named beforehand | Tuned at 40,000 nodes, where depth-dependent constants (the LMR table, the null-move slope) are under-read or mis-read; 72,000 games is a short run for 17 parameters, so the result is a direction more than an optimum; one PR that moves many constants cannot say which one mattered |

**Amended on 2026-11-13 at 00:17 MST, before the run started.** Experiment
2's SPRT arrived after the plan above: its change read +15.8 at 40,000 nodes
and is reading about +3 at 200,000, the opposite error to experiment 1's
(+13.0 against +27.7). A 40,000-node game misjudges depth-dependent changes in
both directions, and seven of S1's parameters are depth-dependent. So S1 runs
at **100,000 nodes** a move instead: the same 17 parameters, starts, ranges
and schedule, **1,500 iterations of 24 games (36,000 games)** or until 05:15
MST. And one rule is added for what the result becomes: **a parameter changes
in the PR only if the run moved it by at least its final perturbation**; the
rest stay as they are, and the fresh screen (also at 100,000 nodes) is of that
reduced set. Fewer games at more nodes is a noisier run; the risk named above
about a direction rather than an optimum is larger.

**Result.** Stopped by its timer at 05:15 MST after **955 of 1,500
iterations, 22,920 games** (5,017 wins for the "plus" side, 5,047 for the
"minus", 12,856 draws, as it should be for a symmetric perturbation). Where
each parameter ended, and how far that is in units of its final perturbation:

| Parameter | Start | End | Moved |
|:--|--:|--:|--:|
| Late move pruning scale, % | 100 | 111.4 | **+1.43** |
| LMR table divisor (×100) | 225 | 208.4 | **−1.38** |
| Null-move evaluation divisor | 200 | 178.6 | **−1.07** |
| SEE quiet margin | 64 | 71.9 | +0.98 |
| Late move pruning base | 3 | 3.41 | +0.82 |
| Futility margin | 100 | 108.1 | +0.81 |
| LMR table base (×100) | 75 | 80.9 | +0.73 |
| Aspiration half-width | 35 | 32.1 | −0.72 |
| LMR history divisor | 512 | 479.0 | −0.69 |
| SEE depth | 8 | 8.31 | +0.61 |
| Null-move depth slope (×64) | 22 | 23.0 | +0.52 |
| Futility base | 100 | 106.5 | +0.46 |
| Reverse futility margin | 80 | 82.2 | +0.27 |
| Null-move base (×64) | 192 | 193.8 | +0.15 |
| Reverse futility depth | 6 | 5.93 | −0.14 |
| Futility depth | 6 | 6.05 | +0.11 |
| Late move pruning depth | 4 | 3.96 | −0.09 |

**By the rule added in the amendment, three parameters change** (the ones the
run moved by at least their final perturbation): late move pruning 11% later,
the LMR divisor 2.25 → 2.08 (more reduction), the null-move evaluation divisor
200 → 179. **The fresh screen of that reduced set** against the run's base
(`v1-spsa-s1-reduced`: `lab6`, 100,000 nodes, 2,000 games, both sides with the
base's null-move form): **+1.6 ± 9.0**, LOS 63.4%, W / L / D 406 / 397 / 1197,
pentanomial 30, 222, 488, 229, 31. **Under +8, so by the plan the run is
recorded and no PR is opened.** It was not taken to the instance.

**Reading it.** Nothing moved far: the largest move is 1.4 final perturbations
in 22,920 games, and the set that moved is worth nothing measurable. With
matches M16 and M17 (the margins as a set are flat to ±25%) this says the same
thing twice: at net8's scale the search's constants are near enough to where
they should be that a night of games cannot find a better setting. The two
gains the session did find were structural (a different null-move formula, a
new distinction in LMR), not a better value of an existing constant. Not
tested: the run's parameters at the SPRT's budget; a longer run.

**A dry run of the script first** (`spsa-test`, 23:32 MST on 2026-11-12;
recorded at the end of the session, like the harness's pilot above): 3
iterations of 4 pairs at 10,000 nodes, seed 1, with the 13 parameters the
script had then; 24 games (3 wins, 6 losses, 15 draws for the "plus" side). It
checked that the script plays, parses a result and moves the parameters. Its
parameters were discarded.

**The run.** Started at 00:39:05 MST on `lab6` (`research/overnight-lab` at
`b13d894`), 100,000 nodes, 12 pairs an iteration. Stopped by its stop file
after 11 iterations and resumed from its saved state at 00:42 with 12 games at
once instead of 14: the two extra games ran on the laptop's efficiency cores
and each iteration waited for them. The resume replays the random generator,
so the sequence of perturbations is the one a straight run would have drawn.
At 17 s an iteration it will reach about 950 of its 1,500 iterations by the
05:15 stop, so the gains will not have decayed as far as the schedule assumes.

#### Incident: experiment 4's fixed-node leg failed at launch

At 07:58:04 UTC the fixed-node job of experiment 4's SPRT (run 43386646994)
failed, 24 minutes after it began waiting for an instance and 8 seconds after
experiment 3's timed leg, whose slot it was waiting for, completed. The job's
log is not readable until the run ends; the cause below is inferred from the
account's state, not read from the log. The account's on-demand quota for
standard instances is 256 vCPUs (`aws service-quotas`, L-1216C47A). At that
moment there were three 64-vCPU instances (experiment 3's timed leg shutting
down, experiment 4's timed leg, match M5) and the Lichess bot's one vCPU: 193.
A fourth 64-vCPU launch makes 257. The task profiles' own budgets (`sprt` 128,
`match` 64) allowed it, since they don't count an instance that is shutting
down or the other profile's.

**What the agent did wrong:** it labelled experiment 4 while experiment 3 was
still running, to save waiting, with a match running beside them. **From here**
a new SPRT is labelled only after the previous one's instances have
terminated. Experiment 4's timed leg was not affected and plays on. Its
fixed-node leg is re-run (`gh run rerun --failed`) once the run completes, if
the timed leg accepted H1; if it did not, the gate is already not met and the
fixed-node leg is recorded as not played.

#### Training T1: net8x160

The plan is in [nets.md](nets.md#net8x160-l050-r43390801430), recorded in `2160aea` at
08:07:36 UTC, before the dispatch at 08:07:49.

| Run | Started (UTC) | Outcome |
|:--|:--|:--|
| [43390801430](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43390801430) | 2026-11-13 08:07 | Net id `net8x160-l050-r43390801430`. An on-demand `g6.xlarge` (`i-0573b746d7245e4b6`, us-east-1b) launched at the first try, at 08:08:36. **Trained** in 7,118 s; the job ended at 10:09:08 |

**Result of the training** (the net's `training.json`; the full record is in
[nets.md](nets.md#net8x160-l050-r43390801430)): the data's SHA-256s equal
net8's, the engine check passed (mean 7.28 cp), and the held-out loss is
**0.019921 against net8's 0.020037 on identical records, −0.58%**, a little
better than the plan's 0.2 to 0.5%. Its SPRT is #256, a draft that embeds the
net; the gate is H1 in both legs against net8.

**The SPRT** ([run 43434302426](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43434302426),
#256, `44112fb` against `main` at `0a45da0`, which is net8x160 against net8 in
one engine; elo0 0, elo1 5, α = β = 0.05; on demand, two `c8a.16xlarge` in
us-east-1a; one attempt; the gate was recorded with the plan, before training;
the label at 13:30:52 UTC, after experiment 7's instances had terminated):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+11.6 ± 6.1** | 100.0% | 3910 | 881 / 750 / 2279 | 2.96 | 34, 392, 995, 477, 57 | H1 |
| Fixed nodes, 200,000 per move | **+8.2 ± 5.1** | 99.9% | 6250 | 1462 / 1314 / 3474 | 2.95 | 43, 781, 1409, 769, 123 | H1 |

CI bench (base `v0.48.1`): signature 970,013 → 990,172; speed per node
−0.4% ± 3.4%; time to depth +2.6% ± 3.5%; 4,062 instructions a node;
fail-high-first 87.5% (87.4%); EBF 1.77 (1.88, marked better); the performance
gate passed. `Commit messages` and `NNUE diagrams` fail on #256 by
construction: the gallery wants a manifest for the embedded net, and a
manifest names the release the net became the default in.

**The recorded gate is met:** H1 in both legs. The comment's reading: "smarter
per node, and worth its cost at a clock: merge." The agent does not merge, and
#256 stays a draft: it needs its gallery entries before it can merge, and they
name a release that does not exist yet.

**Reading it.**
- **net8 was under-trained.** Twice the schedule on identical data is worth
  +11.6 ± 6.1 at a clock and +8.2 ± 5.1 per node, with a node costing the
  same. The ledger's "a longer schedule is saturated" was measured at 1.47B
  positions, where 80 superbatches were 5.5 passes; at 3.03B they were 2.6.
- **It needed no new data and no code:** one `Train` run and one SPRT, about
  $4.20 at the session's estimates.
- **The loss said so beforehand, and that is not a rule.** −0.58% on identical
  records came with a gain here, as net2x80's −0.36% did; net7rl012w768's
  −1.00% lost at a clock, where its node was dearer. The SPRT is the verdict.
- **Not measured:** how it adds to #246 and #254 (all three were tested against
  `main` alone); a second training run (this is one run from one seed);
  anything but self-play against net8.

### Cost and resources

| Workload | Run | Estimate before | Actual (estimate from run time) |
|:--|:--|--:|--:|
| SPRT, experiment 1 (#246) | [43378687011](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43378687011) | $2.30–5.80 | About $0.95: the two legs' jobs took 9 min 21 s and 6 min 49 s |
| Match M1, first attempt | [43380319041](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43380319041) | $1.20 | About $0.20: one instance for about three minutes, then the failed setup |
| Match M1, second attempt | [43380894175](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43380894175) | $1.20 | About $1.15: the task ran 1,130 s, about 20 minutes with launch and teardown |
| SPRT, experiment 2 (#248) | [43381561111](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43381561111) | $1.20–5.80 | About $2.75: the legs' jobs took 13 min 51 s and 34 min 0 s |
| Match M2, first attempt | [43383364015](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43383364015) | $0.80 | Nothing: it failed before launching an instance |
| Match M2, second attempt | [43384379664](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43384379664) | $0.80 | About $0.70: the task ran 619 s, about 12 minutes with launch and teardown |
| Match M3 | [43386732727](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43386732727), [43387376307](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43387376307) | $0.70 | About $0.75 for the second attempt (the task ran 649 s); nothing for the first |
| SPRT, experiment 3 (#249) | [43384123736](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43384123736) | $1.70–5.80 | About $3.30: the timed leg's job took 44 min 58 s; the fixed-node leg's task ran 674 s after a 13-minute wait that used no instance |
| Matches M4 and M5 | [43388091282](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43388091282), [43388637501](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43388637501) | $1.40 | About $1.50: the tasks ran 649 s and 620 s |
| SPRT, experiment 4 (#250) | [43386646994](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43386646994) | $2.30–5.80 | About $2.60: one instance for 45 minutes (the timed leg); the fixed-node leg launched nothing |
| Match M6 | [43391890629](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43391890629) | $0.70 | About $0.80: the task ran 679 s |
| SPRT, experiment 5 (#252) | [43394229688](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43394229688) | $1.70–5.80 | About $3.20: the legs' jobs took 10 min 23 s and 45 min 26 s |
| Matches M16, M17 and M12 | [43393664014](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43393664014), [43395387426](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43395387426), [43397194328](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43397194328) | $2.10 | About $2.25: the tasks ran 649 s, 680 s and 619 s |
| Matches M11 and M8 | [43398918563](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43398918563), [43400673040](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43400673040) | $1.40 | About $1.45: the tasks ran 619 s and 649 s |
| SPRT, experiment 6 (#254) | [43401686917](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43401686917) | $1.20–5.80 | About $2.30: the legs' jobs took 15 min 25 s and 24 min 3 s |
| Match M18 | [43402595891](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43402595891) | $2.70 | About $2.60: the task ran 2,630 s |
| Training T1, net8x160 | [43390801430](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43390801430) | $2.20 | About $1.85: the job ran 2 h 1 min on a `g6.xlarge` |
| SPRT, experiment 7 (#255) | [43427691305](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43427691305) | $2.30–5.80 | About $4.60: the legs' jobs took 45 min 28 s and 34 min 55 s |
| SPRT, training T1's net (#256) | [43434302426](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43434302426) | $2.00–6.00 | About $2.35: the legs' jobs took 18 min 53 s and 21 min 54 s |
| **Total** | | | **About $35.30**, against the session's cap of $40 |

On-demand `c8a.16xlarge` at about $3.45 an hour a leg; job time is used for
instance time, a slight overestimate. Every figure is an estimate from run
time, not a bill; no instance was left running.

### Not verified, and open

**Not verified.**
- **How the three passing changes add.** #246, #254 and #256 were each tested
  against `main` alone. The two search changes both make the search spend less
  on what it expects to fail, so they may overlap, and a net's gain is measured
  in the search it was tested with. #254's SPRT after its rebase onto #246 is
  the first of those measurements.
- **Anything but self-play against a PR's own base.** No rating run and no
  other engine; self-play overstates gains. One change (#246) was also measured
  at 10+0.1. Nothing was measured at a longer control, and #254 and #252 only
  at the SPRT's two settings.
- **Why the two SPRT legs disagreed** for #248 (+16.5 timed, +2.8 at fixed
  nodes) and #252 (+4.9 and +17.7). The depth-by-phase table under experiment 2
  shows the legs give a move different searches; whether that is the cause was
  match M10's question, and M10 was withdrawn for the spend cap.
- **Experiment 4's fixed-node leg,** which never played.
- **`s1-lmr-cut`'s reading** (−11.8 ± 10.5 at 1,796 games): its log was
  destroyed, and the figure is the last the agent saw.
- **#251's fix, end to end.** `Match` is dispatched from `main`, so the change
  cannot run before it merges. The diagnosis (a build for the runner's own CPU
  restored from the cache onto a different CPU) fits both failures; it was not
  reproduced on purpose.
- **Speed per node for #246, #252 and #254.** Their CI bench comments read
  ±7% to ±11% on speed, which says nothing either way. The instruction counts
  are steadier: 4,033, 4,124 and 4,069 a node, against 4,066 for the base
  (#235's bench comment).
- **The costs.** Every figure in the table above is a run time multiplied by
  the on-demand price, not a bill. Cost Explorer showed nothing for 2026-11-13
  when the session ended (read at 13:38 UTC; it runs about a day behind).
  Storage, data transfer, the logs and the GitHub-hosted runner minutes are not
  in the table.
- **Training noise.** net8x160 is one run from one seed.
- **What jwall-engineering-infra 0.19.0 changed on an instance.** The pin moved
  between net8's training and net8x160's (#240), and the difference was not
  read.
- **Most of the references:** see [References](#references).

**Open.**
- **Matches planned and withdrawn unrun:** M7 (null-move base 5), M9 (capture
  history in ordering), M10 (#248 at 10+0.1), M13 and M14 (the improving
  heuristic at LMR and at late move pruning), M15 (both singular-extension
  variants together). Their lab branches are pushed, so each is one `Match`
  dispatch, about $0.75.
- **Built in the lab and never screened at the gate's budget:** razoring,
  history pruning, SEE pruning of captures, the linear history bonus, no table
  cutoffs at PV nodes, a late-move reduction for losing captures (+7.1 ± 9.7 at
  40,000 nodes, which this session showed is not a reading to trust), and less
  reduction at PV nodes (59% more nodes on the bench; not screened).
- **SPSA run S1** stopped at 955 of its 1,500 iterations, and its result was
  screened at 100,000 nodes only.
- **#250's sticky comment is wrong** about what happened ("re-run the SPRT");
  the PR's description and experiment 4 above have the timed leg's result. The
  comment job does not find a lone leg's log when the other leg left no
  artifact; not investigated.
- **The NNUE gallery has no state for a net under test,** which is why #256
  cannot be green before its verdict (its description).
- **Branches pushed for reproducibility and never meant to merge:**
  `research/overnight-lab`, the `research/lab-*` probes and
  `research/nmp-base4`. The owner's to keep or delete; `research/overnight-lab`
  is the only home of the screens' logs.
- **The SPRT and match artifacts expire.** The games and logs of every run
  cited here are GitHub Actions artifacts, kept until 2027-02-11 (90 days).
  Nothing was copied out of them but what this log quotes.
- **The account has no AWS Budget.** The $40 cap was the agent's own; a budget
  with an alert would make the limit the owner's.
- **A gap:** the agent was not running from 02:50 to 05:42 MST, so the session
  did nearly three hours less than its length suggests.

**Suggested next, in the order the evidence supports.**
1. **Decide #246, then re-test #254 on top of it.** That SPRT is both #254's
   gate after the rebase and the first measurement of the two together.
2. **Decide #256 and its release,** so its gallery entries can be written; a
   rebase onto the new `main` tests the net again in the search it will ship
   with.
3. **Play generation 7 only after those are decided.** Its labels are the
   searches of the engine that plays it, so it should be the strongest one, and
   train the net on it at about five passes over the data, not 80 superbatches.
4. **Screen at the gate's budget, and expect the best of several to shrink.**
   One 200,000-node match (3,000 games, about $0.75) per idea: M15 and M9 are
   ready to dispatch.
5. **Find out what separates the two legs.** One change that splits them (#248
   or #252) as a match at two or three node budgets and at 10+0.1 would say
   whether it is search length. Until then a split verdict cannot be read.
6. **Record each net's scale when it is trained** (#253's tool), so a change in
   the search's effective margins is seen when it happens.
7. **Not the constants.** Two independent measurements found nothing there; a
   tuning run long enough to disagree would be far longer than this one.

### References

What was read for the hypotheses and the PRs. **How it was gathered:** a
subagent of the session fetched each page on 2026-11-12 and summarised it. The
session then had three pages fetched again and checked the claims its PRs rest
on against verbatim quotes; those are marked ✓. The rest are cited on the
subagent's summary and were not re-read, and two citations in PR descriptions
were written from general knowledge without a page being fetched: Donninger,
"Null Move and Deep Search", *ICCA Journal* 16(3), 1993 (#246), and the wiki's
Node Types page (#254). **Originality:** these are descriptions of techniques
and public test statistics. No engine's source, evaluations, games or weights
were used, and every implementation was written from a description against
this repository's own search. Several pages of the Stockfish project were
opened for their commit messages and test figures (games to an SPRT pass, never
Elo taken as this engine's); such a page also shows its diff, and that only the
prose was read is the subagent's statement.

| Technique | Sources | Used for |
|:--|:--|:--|
| Null-move pruning, adaptive reduction | Chess Programming Wiki, [Null Move Pruning](https://www.chessprogramming.org/Null_Move_Pruning) ✓ ("Some programs vary R with depth, or based on evaluation features"; "a factor scaled by the difference between (potentially TT-corrected) evaluation and beta can also be added to the reduction value"). E. A. Heinz, "Adaptive Null-Move Pruning", *ICCA Journal* 22(3), 1999, through [his own summary](https://people.csail.mit.edu/heinz/iccaj_db/node34.html) and through O. David-Tabibi and N. Netanyahu, ["Verified Null-Move Pruning"](https://arxiv.org/abs/0808.1125); the paper's full text was not read (the publisher refused the request) | Experiments 1 and 4, match M2 |
| The table score as the pruning evaluation | The same page's "(potentially TT-corrected) evaluation", found by the check above after #248 was written; #248's own citation says it was written from general knowledge | Experiment 2 |
| Delta pruning in quiescence | Wiki, [Delta Pruning](https://www.chessprogramming.org/Delta_Pruning) ✓ ("the safety margin (delta) should be around 200 centipawns, depending on the piece values used by the program"; "delta pruning should be switched off in the late endgame"), [Quiescence Search](https://www.chessprogramming.org/Quiescence_Search), [Static Exchange Evaluation](https://www.chessprogramming.org/Static_Exchange_Evaluation) | Experiment 3 |
| Futility and reverse futility pruning | Wiki, [Futility Pruning](https://www.chessprogramming.org/Futility_Pruning) ✓ (among the moves still searched: "captures (either all or less typically only those that are capable of raising the score above alpha + margin)"), [Reverse Futility Pruning](https://www.chessprogramming.org/Reverse_Futility_Pruning); E. A. Heinz, [extended futility pruning](https://people.csail.mit.edu/heinz/dt/node19.html) | Experiment 5; screens `sc-rfp120`, `sc-fp200` |
| Late move reductions by node type | Wiki, [Late Move Reductions](https://www.chessprogramming.org/Late_Move_Reductions) | Experiment 6, match M11 |
| History tables | Wiki, [History Heuristic](https://www.chessprogramming.org/History_Heuristic) (read for its capture-history section; continuation history at further distances is cited from general knowledge), [History Leaf Pruning](https://www.chessprogramming.org/History_Leaf_Pruning) | Experiment 7, match M8; the lab's capture history and history pruning |
| The improving heuristic | Wiki, [Improving](https://www.chessprogramming.org/Improving) | Screens `s1-imp-rfp`, `s1-imp-lmr`; match M12 |
| Singular extensions | Wiki, [Singular Extensions](https://www.chessprogramming.org/Singular_Extensions), [Multi-Cut](https://www.chessprogramming.org/Multi-Cut) | Matches M3 and M5 |
| ProbCut | Wiki, [ProbCut](https://www.chessprogramming.org/ProbCut); M. Buro's [publication list](https://skatgame.net/mburo/publications.html) (the papers themselves were not read) | Screen `s3-probcut200`, match M4 |
| Razoring | Wiki, [Razoring](https://www.chessprogramming.org/Razoring) | A lab option, never screened |
| Correction history | Wiki, [Static Evaluation Correction History](https://www.chessprogramming.org/Static_Evaluation_Correction_History) | Background: #221 |
| Time management | Wiki, [Time Management](https://www.chessprogramming.org/Time_Management) | H10, not run |
| SPSA | Wiki, [SPSA](https://www.chessprogramming.org/SPSA) and [Stockfish's Tuning Method](https://www.chessprogramming.org/Stockfish%27s_Tuning_Method); J. C. Spall's [SPSA page](https://www.jhuapl.edu/SPSA/); OpenBench's [SPSA Tuning Workloads](https://github.com/AndyGrant/OpenBench/wiki/SPSA-Tuning-Workloads) (the perturbation as about a twentieth of a parameter's range) | SPSA run S1 |
| SPRT and match statistics | Wiki, [Sequential Probability Ratio Test](https://www.chessprogramming.org/Sequential_Probability_Ratio_Test) and [Match Statistics](https://www.chessprogramming.org/Match_Statistics); the fishtest wiki, [Fishtest Mathematics](https://official-stockfish.github.io/docs/fishtest-wiki/Fishtest-Mathematics.html) and [Creating my first test](https://official-stockfish.github.io/docs/fishtest-wiki/Creating-my-first-test.html) (a node-budget test is "independent of engine speed" and does "not substitute tests under real TC") | Method; the pooled reading of match M18 |
| NNUE training | Wiki, [NNUE](https://www.chessprogramming.org/NNUE); bullet's docs, [basics](https://github.com/jw1912/bullet/blob/main/docs/1-basics.md) and [data](https://github.com/jw1912/bullet/blob/main/docs/3-data.md) | H11 to H13, H15 and training T1 |

**Test records opened for their figures only** (commit messages and pull-request
descriptions of the Stockfish project, each for how many games a published
patch took to pass): the improving heuristic in LMR
([fff6b9f](https://github.com/official-stockfish/Stockfish/commit/fff6b9f0614243a14c7ba6aa8c846d8d4025d3a6)),
capture history
([4bc1198](https://github.com/official-stockfish/Stockfish/commit/4bc11984fc5a148ee8f1b55d6ac47c4a397cc8b8)),
history pruning
([4eca981](https://github.com/official-stockfish/Stockfish/commit/4eca981e557704f4ce3db829adf380b8fc058926)),
singular-extension variants
([f69106f](https://github.com/official-stockfish/Stockfish/commit/f69106f7bb82f97ba07fb06a31395039d08af9f4),
[33a858e](https://github.com/official-stockfish/Stockfish/commit/33a858eaa1f792b3413384a3d0993dba36aca92e),
[39da50e](https://github.com/official-stockfish/Stockfish/commit/39da50ed23ee3f1bd32a58c8f02471faa9a9fd63)),
razoring ([#3921](https://github.com/official-stockfish/Stockfish/pull/3921)),
correction history
([#4950](https://github.com/official-stockfish/Stockfish/pull/4950),
[#5598](https://github.com/official-stockfish/Stockfish/pull/5598),
[#5617](https://github.com/official-stockfish/Stockfish/pull/5617)) and time
management
([e6482b7](https://github.com/official-stockfish/Stockfish/commit/e6482b7d97a6620cebdd569cce575d42ed80c871),
[#6729](https://github.com/official-stockfish/Stockfish/pull/6729),
[discussion 4292](https://github.com/official-stockfish/Stockfish/discussions/4292)).
None of the session's choices rests on them: the ranking was made from this
repository's record before the list came back.

**What the reference review could not check:** Heinz's 1999 paper in full; the
ProbCut papers (no PDF reader); Anantharaman, Campbell and Hsu on singular
extensions, and the original LMR articles, which are cited through the wiki
only; commit dates. Several wiki pages carried a banner saying they were a
temporary, read-only recovery copy.

## Follow-up 2026-11-13: experiment 6 again, on top of #246

After the session above had ended, the owner merged #246 (`cd51dee`, 15:24:42
UTC) and released it as `v0.49.0` (`9663137`, 15:29:21 UTC), then asked for #254
to be rebased. A rebase is a push, and a push to a pull request that carries
the `sprt` label plays its SPRT again, against the new base. That run is #254's
gate now, and the first measurement of two of the session's changes together,
which the session left open ("Not verified, and open", above). The same agent
did the work; the owner was present.

### Experiment 6, second run: #254 against `v0.49.0`

Recorded on 2026-11-13 at 08:41 MST (15:41 UTC), before the rebased
branch was pushed.

| | |
|:--|:--|
| Question | Does one more ply of late-move reduction at expected cut nodes still gain Elo when the base already has the adaptive null-move reduction? Both make the search look less at what it expects to fail, so they may overlap |
| The one change | The base. `feat/lmr-cut-node` at `695192d` against `main` at `9663137` (`v0.49.0`, which has #246). The patch is the first run's: the rebase from `0e188c9` had no conflict and `git range-diff` reports the two commits equal |
| Held the same | The SPRT's settings (elo0 0, elo1 5, α = β = 0.05; 5+0.05 and 200,000 nodes a move; the same openings and cap), the instance type and market, the net (net8) |
| Bench, local | 939252 → 958984 (+2.1% nodes to depth 10: the main search 4.2% smaller, quiescence 8.3% larger); fail-high-first 0.8773 → 0.8756 (bar 0.87); EBF 1.896 → 1.846 (bar 2.0); both bars pass; 1.9% of reduced moves re-searched (1.7%). The base's figures are the release's own record (`benchmarks/v0.49.0/`); they are counts, the same on any machine |
| Expectation | Positive, and at or under the first run's +10: between +5 and +10 in both legs. A test that stops at H1 reads high on average, as the first run did, and the two changes may overlap, so the lower half is the likelier. A true gain under +5 would most likely run to the cap without a verdict |
| Gate | H1 in both legs, against this base. The first run's pass does not carry over: a rebased change is judged on the engine it would merge into |
| What each outcome means | **H1 in both:** the gate is met for the engine as it is now. **H0 in either:** the change loses on top of #246, whatever it did alone, and is recommended for closing. **Inconclusive in either:** the check fails; the two changes overlap, and whether to merge at a high LOS is the owner's decision under AGENTS.md |
| Cost estimate | Two on-demand `c8a.16xlarge` for 10 to 50 minutes: $1.20 to $5.80, outside the session's $35.30 and asked for by the owner. No SPRT or match is running, and no instance but the Lichess bot's (`describe-instances`, 15:38 UTC); the release's rating ladder runs on a GitHub-hosted runner |

Local checks on `695192d` before the push: `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0),
`cargo test --workspace --locked` (270 passed) and with `--features stats`
(271 passed), `npm test` (59 pass); the signature is 958984 in
two runs and in the `stats` build.

**Result** ([run 43455334547](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43455334547),
#254, `695192d` against `main` at `9663137`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge` in us-east-1a; one attempt; the plan's commit
`36efe7b` at 15:41:21 UTC, the push at 15:42:35):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+8.1 ± 4.9** | 99.9% | 6124 | 1268 / 1126 / 3730 | 2.97 | 67, 631, 1538, 745, 81 | H1 |
| Fixed nodes, 200,000 per move | **+7.6 ± 4.7** | 99.9% | 6482 | 1407 / 1266 / 3809 | 2.96 | 97, 593, 1666, 842, 43 | H1 |

CI bench (base `v0.49.0`): signature 939,252 → 958,984; speed per node +0.8% ± 2.5%; time to depth +1.3% ± 2.4%; 4,035 instructions a node (4,033 for `v0.49.0`); fail-high-first 87.6% (87.7%); EBF 1.85 (1.90, marked better); perft pass; the performance gate passed.

**The gate recorded before the run is met:** H1 in both legs, against the engine that has #246. The agent does not merge. The comment's reading: "Gate: pass: smarter per node, and worth its cost at a clock: merge."

**Reading it.**
- **The gain holds on top of #246, and the legs agree again:** +8.1 ± 4.9 at a clock and +7.6 ± 4.7 per node.
- **It reads a little under the first run, and not by enough to say the two changes overlap.** Against `v0.48.1` the same patch read +9.8 ± 5.6 and +10.6 ± 5.8; this run is 1.7 and 3.0 lower, and the difference between two such readings carries an error of about ±7.5 (the two errors combined). A small overlap is not ruled out either.
- **Both legs needed more games than before** (6,124 and 6,482, against 5,004 and 4,322), which fits a somewhat smaller gain; where an SPRT stops is noisy and proves little by itself.
- **Inside the plan's expectation** of +5 to +10.
- **Still not measured:** the two changes together as one match against `v0.48.1` (adding #246's SPRT to this one adds two readings that each stopped at H1, which reads high); the net of #256 on top of them; this change at a longer time control; anything but self-play against the base.

**Cost:** about $3.00 (the legs' jobs took 28 min 34 s and 24 min 1 s, at
about $3.45 an hour a leg; an estimate from run time, not a bill), outside the
session's table. With the session's $35.30, about $38.30 in all. Both
instances terminated (16:06:35 and 16:11:10 UTC).

**Merged.** The owner merged #254 at 17:04:18 UTC as `059caa3`. Its tree is the
tested commit's (`git rev-parse` gives the same tree for `059caa3` and
`695192d`), and `main`'s bench after the merge reads 958984, so the engine on
`main` is the one that played. Its release, `v0.50.0`, was an open release pull
request (#258) when this was written.

### `v0.49.0` on the ladder: the first reading of #246 that is not self-play

The session listed "anything but self-play against a PR's own base" as not
verified. The release's own rating run has since played (`Rating` run
[43453142633](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43453142633),
the owner's release workflow; figures from the release notes):

| Release | Engine | Rating at 10+0.1 on Stockfish 19's `UCI_Elo` scale |
|:--|:--|--:|
| `v0.48.1` | The session's baseline | 3178 ± 33 |
| `v0.49.0` | The same, with #246 | 3181 ± 30 |
| `v0.50.0` (added when its run finished) | With #246 and #254 | 3219 ± 32 |
| `v0.51.0` (added when its run finished) | With #246, #254 and net8x160 | 3220 ± 35 |

- **The ladder neither shows #246's gain nor contradicts it.** The two
  readings differ by +3, and the difference of two such readings carries an
  error of about ±45 (the two errors combined; 400 games each, 100 a rung,
  and the two runs were on different runner CPUs).
  #246's SPRT read +23.9 ± 9.0 and +27.7 ± 9.7 in self-play, and +29.4 in a
  self-play match at the ladder's own 10+0.1.
- **So the session's gains remain measured in self-play only.** A gain of 25
  against the engine's own previous version is commonly worth less against
  other opponents, and this ladder cannot say how much less: it would take
  several thousand games a release to resolve 25 Elo.
- **Not a result of this session:** the run is the release's, started by the
  owner's merge. It is recorded in [nets.md](nets.md) with the other releases.
- **`v0.50.0`, added when its run finished** (`Rating` run
  [43472083980](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43472083980),
  18:19 UTC): 3219 ± 32 for the engine with both search changes, still under
  net8. That is +41 over `v0.48.1`, with an error of about ±46 for the
  difference (+38 over `v0.49.0`, ±44). It is in the direction of the two
  gains and about their size (their SPRTs add to +32.0 timed and +35.3 at fixed
  nodes), and the interval still includes zero: consistent with the self-play
  results, not a confirmation of them. The rungs moved unevenly (the 3100 and
  3190 rungs read 3264 and 3246 where `v0.49.0`'s read 3189 and 3176, and the
  2950 rung read lower, 3150 for 3175), which is what 100 games a rung gives.
- **`v0.51.0`, added when its run finished** (`Rating` run
  [43481823225](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43481823225),
  19:18 UTC): 3220 ± 35 for the engine with all three gains. That is +1 over
  `v0.50.0`, with an error of about ±47 for the difference, and +42 over
  `v0.48.1`, with about ±48. The net's gain in self-play (+12.3 ± 6.4 at a
  clock) is far below what this ladder resolves; over the three releases
  together the ladder moved in the direction the SPRTs say and by an amount
  its own error still covers.

### Training T1's net again: net8x160 against net8 in `v0.50.0`'s search (#256)

The owner released #254 as `v0.50.0` (`4451cfe`, 17:21:47 UTC) and asked for
#256 to be rebased so the net ships in `v0.51.0`. That answers what #256 was
waiting for (the release its gallery entries name), and by the repository's
rule the rebase's push plays the SPRT again: the net against net8 in the
search that now has both of the session's search changes, the last of the
"how do the three add" measurements the session left open.

Recorded on 2026-11-13 at 10:37 MST (17:37 UTC), before the rebased
branch was pushed. The same plan is in `docs/nets.md` on #256's branch
(`35fbdc9`), which is where the ledger's copy of this net's record now
travels.

| | |
|:--|:--|
| Question | Does net8x160 still beat net8 when both play in `v0.50.0`'s search, which has the adaptive null-move reduction (#246) and one more ply of late-move reduction at expected cut nodes (#254)? A net's gain is measured in the search it was tested with |
| The one change from the first run | The base: `main` at `4451cfe` (`v0.50.0`) for `0a45da0` (`v0.48.1`). The net is the same file (SHA-256 `8ea67605…556c`). The commit that embeds it is the first run's, rebased without conflict (`62bdf9f`; `git range-diff` reports the two equal); a second commit (`35fbdc9`) adds the gallery and ledger entries and changes no engine code |
| Held the same | The SPRT's settings (elo0 0, elo1 5, α = β = 0.05; 5+0.05 and 200,000 nodes a move; the same openings and cap), the instance type and market |
| Bench, local | 958984 → 919304 (−4.1% nodes to depth 10: the main search 0.5% smaller, quiescence 7.3% smaller); fail-high-first 0.8756 → 0.8778 (bar 0.87); EBF 1.846 → 1.861 (bar 2.0); both bars pass. The base's figures are the release's own record (`benchmarks/v0.50.0/`); they are counts, the same on any machine |
| Expectation | Positive, +5 to +10 in both legs. The first run's readings stopped at H1, which reads high, so the lower half is the likelier, and a leg reaching the cap without a verdict is a real possibility |
| Gate | H1 in both legs, against this base. The first run's pass does not carry over: the net is judged in the engine it would ship in |
| What each outcome means | **H1 in both:** the net is the default from `v0.51.0`. **H0 in either:** #256 is closed and the net is recorded as dropped in this search, whatever it did in the earlier one. **Inconclusive in either:** the check fails, and whether to merge at a high LOS is the owner's decision under AGENTS.md |
| Cost estimate | Two on-demand `c8a.16xlarge` for 10 to 50 minutes: $1.20 to $5.80, asked for by the owner; with what came before, about $38.30 so far. No SPRT or match is running, and no instance but the Lichess bot's (`describe-instances`, 17:36 UTC); `v0.50.0`'s rating ladder runs on a GitHub-hosted runner |

**What the push carries besides the rebase.** The gallery entries #256 could
not have until the release was named: net8x160's manifest and diagram
("default since `v0.51.0`"), net8's manifest moved to "superseded" (default in
`v0.48.0`–`v0.50.0`; it played no generation), the regenerated gallery page,
four test expectations, and the ledger: net8x160's Summary row and section in
`docs/nets.md`, and its row in `docs/datasets.md`.

- **`docs/nets.md` on #256 is this branch's copy plus the swap.** The gallery
  check needs the net's row and section in #256's own tree, and net8's row
  could not be rewritten truthfully on `main`'s copy, which still says "SPRT
  pending". So #256 also carries this branch's corrections of net8's and
  net7rl012's records.
- **The two pull requests now overlap in that file,** and whichever merges
  second needs a rebase. For this branch that is free; for #256 it is a push,
  which plays the SPRT again. #256 should merge first.
- **The manifest quotes the first SPRT's figures,** because it is committed
  before the second is played, and says so. The second run's figures reach the
  ledger through this branch after #256 merges (agreed with the owner
  beforehand).

Local checks on `35fbdc9` before the push: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0), `cargo test --workspace --locked` (270 passed) and with `--features stats` (271 passed), `npm test` (59 pass) and `npm run nnue:diagrams:check` (19 snapshots current); the signature is 919304 in two runs and in the `stats` build.

**Result** ([run 43474643514](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43474643514),
#256, `35fbdc9` against `main` at `4451cfe`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge` in us-east-1b; one attempt; the plan's commits
`35fbdc9` (17:36:04 UTC) and `02283a5` (17:37:10), the push at 17:37:29):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+12.3 ± 6.4** | 100.0% | 3890 | 898 / 760 / 2232 | 2.95 | 51, 396, 921, 518, 59 | H1 |
| Fixed nodes, 200,000 per move | **+30.8 ± 10.8** | 100.0% | 1516 | 398 / 264 / 854 | 2.95 | 15, 140, 341, 220, 42 | H1 |

CI bench (base `v0.50.0`): signature 958,984 → 919,304; speed per node +2.5% ± 2.0%; time to depth −6.4% ± 1.8% (marked better); 4,016 instructions a node (4,035 for `v0.50.0`); fail-high-first 87.8% (87.6%); EBF 1.86 (1.85); perft pass; the performance gate passed.
Every CI check on `35fbdc9` passed, `NNUE diagrams` and `Commit messages` among them.

**The gate recorded before the run is met:** H1 in both legs, in the search the net would ship in. The agent does not merge. The comment's reading: "Gate: pass: smarter per node, and worth its cost at a clock: merge."

**Reading it.**
- **The net beats net8 in the search it would ship in, on both legs:** +12.3 ± 6.4 at a clock and +30.8 ± 10.8 per node.
- **At a clock it reads as it did in `v0.48.1`'s search** (+11.6 ± 6.1 then; 0.7 higher now, against an error of about ±8.8 for the difference of two such readings). The two search changes neither took from the net's gain there nor added to it.
- **At fixed nodes it reads far higher than before:** +8.2 ± 5.1 then, 22.6 higher now, against an error of about ±11.9 for the difference. A leg that stops after 1,516 games reads high, and this is one run, so the size is not established. It may be that a search which leans harder on the evaluation (the null-move reduction now grows with the evaluation's margin over beta) is served more by a better one; that was not tested.
- **The two legs disagree again,** by 18.5 against an error of about ±12.6 for their difference, as they did for #248 and #252. Not explained here either: the net does not make a node dearer (the CI bench reads speed per node +2.5% ± 2.0%, and time to depth 6.4% shorter).
- **Against the plan's expectation** of +5 to +10 in both legs: the timed leg is just above it and the fixed-node leg far above.
- **Still not measured:** the three changes together as one match against `v0.48.1` (adding three SPRTs adds readings that each stopped at H1, which reads high); a ladder rating of this net, which is not released; a second training run; anything but self-play against the previous net.

**Cost:** about $1.50 (the legs' jobs took 19 min 12 s and 6 min 55 s, at
about $3.45 an hour a leg; an estimate from run time, not a bill). With the
session's $35.30 and #254's second run, about $39.80 in all. Both
instances terminated (17:44:24 and 17:56:42 UTC).

**Merged.** The owner merged #256 at 18:14:22 UTC as `e80876f` and released it as `v0.51.0` (`458fd7b`, 18:21:05 UTC). The merged commit's tree is the tested commit's (`git rev-parse` gives the same tree for `e80876f` and `35fbdc9`), and the release's recorded bench signature is 919304, so the engine released is the one that played. All three of the session's gains are on `main` now: #246 in `v0.49.0`, #254 in `v0.50.0` and the net in `v0.51.0`.

**This branch after the merge.** `main` was merged into it rather than the
branch rebased, so that the commits this log cites as the record of a plan
made before its run (`779b4ae`, `e972072`, `2160aea`, `36efe7b`, `02283a5` and
the rest) stay in its history under the same names. `docs/nets.md` is `main`'s
copy, which #256 landed, with the second run's result added; the gallery
manifest quotes that run now.

### Experiment 5 again: capture futility against `v0.51.0` (#252)

The owner merged this log (#247, `363e172`, 18:29:19 UTC) and asked for #252 to
be rebased. #252 is the change the session left undecided: H1 at fixed nodes
(+17.7 ± 7.9) and a timed leg that reached its cap at +4.9 ± 3.8 (LOS 99.4%,
LLR 2.61); pooled with match M18's 10,000 further timed games it reads +4.0,
95% interval +1.3 to +6.7. The rebase's push plays its SPRT again, now against an engine
that has all three of the session's gains.

Recorded on 2026-11-13 at 11:34 MST (18:34 UTC), before the rebased
branch was pushed.

| | |
|:--|:--|
| Question | Does futility pruning of captures still gain, per node and at a clock, in `v0.51.0`: with the adaptive null-move reduction (#246), one more ply of late-move reduction at expected cut nodes (#254) and net8x160 (#256)? |
| The one change from the first run | The base: `main` at `363e172` (`v0.51.0` and this log's docs) for `0a45da0` (`v0.48.1`). The patch is the first run's: `feat/capture-futility` at `1aa1655`, rebased without conflict from `cb6dfbc`, and `git range-diff` reports the two commits equal |
| Held the same | The SPRT's settings (elo0 0, elo1 5, α = β = 0.05; 5+0.05 and 200,000 nodes a move; the same openings and cap), the instance type and market, and every constant of the patch |
| Left as it was, though its premise moved | `CAPTURE_VALUE`, what a capture is worth to the evaluation, was measured on net8 (300, 775, 880, 1,100, 1,680 for pawn to queen). The default is net8x160 now, whose knight, bishop, rook and queen read 2% to 7% smaller on the SPRT openings and whose pawn reads 2% larger ([nets.md](nets.md#net8x160-l050-r43390801430)). The table was not re-measured: that would be a second variable, and a table that is too large only prunes less |
| Bench, local | 919304 → 878901 (−4.4% nodes to depth 10: the main search 3.1% smaller, quiescence 5.6% smaller; against `v0.48.1` the tree's size did not change, 970013 → 970114); fail-high-first 0.8778 → 0.8802 (bar 0.87); EBF 1.861 → 1.847 (bar 2.0); both bars pass; 11,825 captures skipped. The base's figures are the release's own record (`benchmarks/v0.51.0/`); they are counts, the same on any machine |
| Expectation | Fixed nodes: H1 again, somewhere near the first run's reading. Timed: about +4, as the 20,000 timed games already played say. At +4.0 the test reaches H1 before its cap about four times in ten (a normal approximation from the first run's own LLR, 2.61 at +4.9 after 10,000 games), so the likeliest single outcome is the first run's again |
| Gate | H1 in both legs, against this base. Nothing carries over from the first run or from M18 |
| What each outcome means | **H1 in both:** the gate is met for the engine as it is now. **H0 in either:** the change loses on this engine and is recommended for closing. **Timed inconclusive again:** the check fails. A third timed sample of about the same size would say the gain at a clock is real and under the test's `elo1`, and whether to merge at a high LOS is the owner's decision under AGENTS.md. This run's own interval is what is reported; it is not pooled with the earlier games into a verdict, which were played against a different engine |
| Cost estimate | Two on-demand `c8a.16xlarge` for 10 to 50 minutes: $1.20 to $5.80, likely near the top, since the timed leg is expected to run long. Asked for by the owner; the session and the re-tests since stand at about $39.80, so this one takes the total past the $40 the agent had set for the session. No SPRT or match is running, and no instance but the Lichess bot's (`describe-instances`, 18:34 UTC) |

Local checks on `1aa1655` before the push: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0), `cargo test --workspace --locked` (271 passed) and with `--features stats` (272 passed), `npm test` (59 pass); the signature is 878901 in two runs and in the `stats` build.

**Result** ([run 43484214994](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43484214994),
#252, `1aa1655` against `main` at `363e172`; elo0 0, elo1 5, α = β = 0.05; on
demand, two `c8a.16xlarge` in us-east-1a; one attempt; the plan's commit
`1f16417` at 18:34:43 UTC, the push at 18:36:02):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | −0.5 ± 3.8 | 39.3% | 10000 | 1964 / 1979 / 6057 | −1.42 | 111, 1100, 2594, 1083, 112 | Inconclusive at the 5,000-round cap |
| Fixed nodes, 200,000 per move | **−4.2 ± 4.4** | 3.3% | 7038 | 1371 / 1455 / 4212 | −2.95 | 71, 799, 1864, 713, 72 | H0 |

CI bench (base `v0.51.0`): signature 919,304 → 878,901; speed per node −4.2% ± 6.4% and time to depth +0.1% ± 7.0% (a noisy runner: neither says anything at that width); 4,039 instructions a node (4,016 for `v0.51.0`); fail-high-first 88.0% (87.8%); EBF 1.85 (1.86); perft pass; the performance gate passed. These are the second attempt of the push's bench job: a bench job started by the description's edit just before the push ran on the old commit and overwrote the comment, and the job was re-run to restore it.
Every CI check on `1aa1655` passed.

**The gate is not met:** H0 in a leg. By the plan the change loses on this engine and is recommended for closing. The comment's reading: "Gate: fail: no gain at a clock: do not merge"

**Reading it.**
- **On the engine as it is now the rule does not gain, and per node it reads as a loss:** −4.2 ± 4.4 at fixed nodes (LOS 3.3%), where H0 was accepted after 7,038 games, and −0.5 ± 3.8 at a clock (LOS 39.3%) after the full 10,000.
- **That is not the first run repeated with worse luck, and it is the fixed-node leg that says so.** The same patch read +17.7 ± 7.9 there against `v0.48.1`; this run is 21.9 lower, against an error of about ±9.0 for the difference of two such readings. At a clock the 20,000 earlier games read +4.0 (+1.3 to +6.7) and this run is 4.5 lower, against about ±4.7: at the edge of what chance allows, so the clock alone would not have shown it.
- **So the gain belonged to the engine it was measured in.** Three things changed underneath it (#246, #254 and the net), and which of them took it away was not measured. Two things are on record and neither is an explanation: the table of piece values was measured on net8 and not re-measured, and the rule now removes 4.4% of the bench tree where on `v0.48.1` it left the tree's size alone.
- **The expectation written before the run was wrong:** it was the first run's result again (H1 at fixed nodes, about +4 at a clock).
- **What the repository's rule bought.** On the first run's evidence a merge over the failed timed check was defensible: H1 at fixed nodes, LOS 99.4% at a clock, a pooled interval clear of zero. The rebase and its re-test are what showed the rule does not belong in this engine.
- **Not measured:** the rule with its table re-measured for net8x160; a version acting at fewer depths or with a wider margin; which of the three merged changes it overlaps. Each would be a new variant with its own plan, not a reading of this one.

**Cost:** about $3.95 (the legs' jobs took 45 min 1 s and 24 min 0 s, at
about $3.45 an hour a leg; an estimate from run time, not a bill). With the
session's $35.30 and the two re-tests before it, about $43.75 in all, past
the $40 the agent had set for the session; this run was the owner's request.
Both instances terminated (19:00:04 and 19:21:03 UTC).

**Closed.** The owner read the result and asked for #252 to be commented on and
closed; the agent posted the comment and closed it at 19:23:58 UTC. The branch
is kept (`feat/capture-futility` at `1aa1655`, the commit the SPRT tested).
What was not tried, each a new variant that would need its own plan: the
piece-value table re-measured for net8x160; fewer depths or a wider margin.
