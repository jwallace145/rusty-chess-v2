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

| Screen | Options | Nodes/move | Games | Elo ± 95% | LOS | Pentanomial (LL, LD, WL+DD, WD, WW) | LLR | Stop |
|:--|:--|--:|--:|--:|--:|:--|--:|:--|
| `s1-nmp-a` | `NMP_BASE=3 NMP_DIV=3 NMP_EVAL_DIV=200` | 40,000 | 2,000 | **+13.0 ± 9.4** | 99.7% | 29, 211, 462, 252, 46 | 1.54 | Game cap. Put forward: experiment 1 |
| `s1-imp-rfp` | `RFP_IMPROVING=1`: one ply less reverse-futility margin when improving | 40,000 | 2,000 | −9.9 ± 10.6 | 3.4% | 73, 233, 434, 198, 62 | −1.38 | Game cap. **Dropped** |
| `s1-lmr-cut` | `LMR_CUT=1`: one more ply of reduction at expected cut nodes | 40,000 | 1,796 of 2,000 seen | −11.8 ± 10.5 | not kept | not kept | −1.52 | **Log lost** (see the incident below); the last reading the agent saw. Re-queued as `s1-lmr-cut-rerun` |
| `s1-imp-lmr` | `LMR_IMPROVING=1`: one more ply of reduction when not improving | 40,000 | 2,000 | −5.9 ± 10.0 | 12.3% | not kept | −1.07 | Game cap. **Dropped** |

*(more rows as the screens finish)*

**Incident, 23:33 MST: a screen's log was destroyed by the agent.** The runner
script (`screen.sh`) was edited in place while `s1-lmr-cut` was playing. Bash
reads a script as it runs, so the running copy resumed in the middle of the
new text, ran a fragment that truncated the screen's `fc.log`, and wrote no
summary line. The reading in the table is the last the agent polled from the
live log, at 1,796 games; it cannot be checked now. The script is now one
function that is read whole before it runs, is only ever replaced with `mv`,
and writes each run to its own timestamped log. The screen is re-run for the
record. `s1-imp-lmr`'s pentanomial counts were lost to the same edit's format
change (its Elo, LOS, W / L / D and LLR are from its log, which is intact).

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

### Experiments

| # | PR | The one change | Bench before → after | Timed SPRT | Fixed-node SPRT | Verdict |
|--:|:--|:--|:--|:--|:--|:--|
| 1 | #246 | Null-move reduction `2 + depth/4` → `3 + depth/3 + min((eval − beta)/200, 3)` | 970013 → 939252 | **+23.9 ± 9.0** (H1, 1,706 games) | **+27.7 ± 9.7** (H1, 1,508 games) | **Gate met.** For the owner to merge |

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

#### Match M1: experiment 1 at 10+0.1

Recorded on 2026-11-12 at 23:41 MST, before the dispatch.

| | |
|:--|:--|
| Question | Does #246's gain hold at twice the SPRT's time control? A null-move reduction this deep is the kind of change that can win at 5+0.05 and give some back at a longer control |
| What runs | The `Match` workflow from `main`: candidate `feat/null-move-adaptive-reduction` (`b54010f`) against `main` (`0a45da0`), 10+0.1, 2,000 games, hash 64 MB, 99% interval; one on-demand `c8a.16xlarge` (the `match` profile) |
| A measurement, not a gate | The SPRT above is the gate and it is met. This informs the owner's merge decision |
| Read beforehand | Positive with a 99% interval clear of zero: the gain holds. An interval that includes zero: a flag on the PR, and a reason for a longer match before merging. Negative: reported on the PR as a reason not to merge yet |
| Cost estimate | About 20 minutes of one instance at $3.45 an hour: $1.20 |

### Cost and resources

| Workload | Run | Estimate before | Actual (estimate from run time) |
|:--|:--|--:|--:|
| SPRT, experiment 1 (#246) | [43378687011](https://github.com/jwallace145/rusty-chess-v2/actions/runs/43378687011) | $2.30–5.80 | About $0.95: the two legs' jobs took 9 min 21 s and 6 min 49 s |

*(more rows as workloads run; on-demand `c8a.16xlarge` at about $3.45 an hour a leg; job time is used for instance time, a slight overestimate)*

### Not verified, and open

*(filled at the end of the session)*
