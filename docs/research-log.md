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
<!-- screens:end -->

The table is rendered from each screen's fastchess log by a script
(`lab/render_screens.py` on `research/overnight-lab`); more rows follow as
screens finish.

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
- **Not measured:** how the scale has moved from net to net (it should be
  tracked, in [nets.md](nets.md), so a margin's drift is visible); the values in
  positions that are already unbalanced, where the output is flatter.

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

| 2 | #248 | Reverse futility and the null move read the table's score as the evaluation, where its bound allows | 970013 → 942699 | Pending | Pending | Pending. **Crosses the fail-high-first bar (0.853 against 0.87)**: the owner's decision whatever the SPRT says |

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
| [43380894175](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43380894175) | 06:45 | The same inputs again. Pending |

#### Match M2: one more ply of null-move reduction, at 200,000 nodes

Recorded on 2026-11-13 at 00:03 MST, before the dispatch. **Exploratory:** a
screen on the standardized instance, under the amended rule for changes that
act at depth.

| | |
|:--|:--|
| Question | Experiment 1 moved the null-move reduction one step and gained +24 to +28. Is the best setting further along the same axis? |
| What runs | `Match` from `main`: candidate `research/nmp-base4` (`daf0970`: `NULL_MOVE_REDUCTION` 4, nothing else) against `feat/null-move-adaptive-reduction` (`b54010f`, #246), 200,000 nodes a move, 3,000 games, hash 64 MB, 99% interval; one on-demand `c8a.16xlarge` |
| Read beforehand | +8 or more: a PR stacked on #246, with its own plan and SPRT against #246's head. Between 0 and +8: recorded, no PR. Below 0: dropped |
| Why not straight to an SPRT | It is the second value tried on one axis: a screen first keeps the SPRT a test of one pre-chosen variant |
| Cost estimate | About 14 minutes of one instance: $0.80 |

#### SPSA run S1: the margins and reduction constants together

Recorded on 2026-11-13 at 00:03 MST, before the run. **Exploratory and local.**

| | |
|:--|:--|
| Question | The roadmap's open item: re-tune what was tuned to PeSTO's scores. Is there a jointly better setting of the search's margins and reduction constants for net8, on top of #246? |
| Method | SPSA (Spall): every iteration perturbs all parameters at once by ±c in a random direction, plays 12 game pairs between the two perturbed engines, and moves each parameter along its perturbation in proportion to wins less losses. Gains decay as a/(A + k)^0.602 and c/k^0.101 with A a tenth of the iterations; the end-of-run learning rate is 0.003. `lab/spsa.py` on `research/overnight-lab`, lab binary `lab5`, seed 20261113 |
| Games | 40,000 nodes a move, the SPRT's openings and adjudication; 3,000 iterations of 24 games, 72,000 games, or until 05:15 MST, whichever comes first. The stopping rule is fixed here; the parameters at the stop are the result |
| Base | `main` with #246's null-move reduction in a fractional form, `R = (192 + 22·depth)/64 + min((eval − beta)/200, 3)`, which has #246's bench signature (939252) |
| Parameters (start; range; final perturbation) | Reverse futility margin (80; 40–150; 8) and depth (6; 3–10; 0.5). Futility base (100; 0–260; 14), margin (100; 40–200; 10) and depth (6; 3–10; 0.5). Late move pruning base (3; 1–8; 0.5), depth (4; 2–9; 0.5) and scale (100%; 40–170; 8). SEE quiet margin (64; 20–140; 8) and depth (8; 4–12; 0.5). LMR table base (0.75; 0.20–1.50; 0.08) and divisor (2.25; 1.50–3.30; 0.12), and its history divisor (512; 128–2,048; 48). Aspiration half-width (35; 8–80; 4). Null move: base (192/64; 96–330; 12), depth slope (22/64; 8–44; 2) and evaluation divisor (200; 80–400; 20) |
| What the result becomes | The parameters at the stop, rounded, first play a fresh local screen against the base (2,000 games at 40,000 nodes, not the tuning games). At +8 or more they become one PR stacked on #246, gate H1 in both SPRT legs against #246's head. Below that, the run is recorded and no PR is opened |
| Risks named beforehand | Tuned at 40,000 nodes, where depth-dependent constants (the LMR table, the null-move slope) are under-read or mis-read; 72,000 games is a short run for 17 parameters, so the result is a direction more than an optimum; one PR that moves many constants cannot say which one mattered |

### Cost and resources

| Workload | Run | Estimate before | Actual (estimate from run time) |
|:--|:--|--:|--:|
| SPRT, experiment 1 (#246) | [43378687011](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43378687011) | $2.30–5.80 | About $0.95: the two legs' jobs took 9 min 21 s and 6 min 49 s |

| Match M1, first attempt | [43380319041](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43380319041) | $1.20 | About $0.20: one instance for about three minutes, then the failed setup |

*(more rows as workloads run; on-demand `c8a.16xlarge` at about $3.45 an hour a leg; job time is used for instance time, a slight overestimate)*

### Not verified, and open

*(filled at the end of the session)*
