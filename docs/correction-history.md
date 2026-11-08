# Pawn-structure correction history: design

A design note for a search change: a table, indexed by the pawn structure
and the side to move, that learns during the search how far the network's
static evaluation tends to be from what a deeper search finds in positions
with that structure, and adds that back before the evaluation drives pruning.
It records the measurement that motivated it, the decisions fixed before the
run, and the plan for testing. The search it changes is described in
[roadmap.md](roadmap.md) (Phases 4 and 4b); the network it corrects in
[nnue.md](nnue.md).

## Why

The static evaluation decides a lot of the search's pruning: reverse futility
(`search.rs:577`), null-move eligibility (593), quiet-move futility (710) and
quiescence stand-pat (942–955) all read it. Every one of those decisions is as
good as the evaluation at that node. A network's error isn't pure noise: some
of it is systematic, and the pawn structure is the part of a position that
changes slowest and shapes the plans of both sides, so positions sharing one
tend to share the error too. Strong engines have carried such a table since
2024 ("correction history"); in Stockfish the first, pawn-structure version
was worth about +10 Elo at short time controls, and engines nearer this one's
strength reported more. Those figures are from memory, not checked here.

The idea is cheap: one hash of the two pawn bitboards and one table read per
evaluated node, one update per stored node. Unlike a wider network, it should
cost nothing measurable per node, so the timed SPRT leg should follow the
fixed-node one.

## Measured before building

**The question:** does the pawn structure predict the evaluation's error
against the search? If not, no table can help.

**Method.** A scratch build at `v0.46.2` (`78779e6`; not in the repository)
logged, at every node of `bench` (50 positions, depth 10) where the search
stored a result and this plan's eligibility held (not in check, not a mate
score, best move neither a capture nor a promotion): the side to move, a hash
of the two pawn bitboards, the depth, the bound, and
`search score − static eval`. Then, grouping by side and structure, each
sample's error was predicted by the mean of the *other* samples of its
structure (leave-one-out, so nothing predicts itself), shrunk towards the
global mean by `n / (n + k)`, as a bounded moving average would hold it. The
figure is how much of the mean squared error that prediction removes, over
the samples whose structure was seen at least twice.

| Sample | Nodes | Structures | Seen twice or more | Mean \|error\| | Std | MSE removed, k = 0 / 2 / 4 / 8 |
|:--|--:|--:|--:|--:|--:|:--|
| Exact scores | 2,625 | 714 | 88% | 64 cp | 108 | **+39% / +38% / +35% / +29%** |
| Exact scores, depth ≥ 3 | 1,439 | 410 | 89% | 84 cp | 134 | **+51% / +49% / +44% / +36%** |
| Stockfish's rule: exact, or a bound in the error's direction | 57,622 | 8,703 | 93% | 86 cp | 159 | **+29% / +30% / +29% / +27%** |
| **Negative control:** exact scores, structure keys shuffled | 2,625 | 719 | 88% | 64 cp | 108 | −21% / −7% / −4% / −2% |

Of 125,359 eligible stores, 2,625 were exact (2.1%), 46,542 lower bounds and
76,192 upper bounds. For structures seen four or more times (216 of them), the
median structure mean was 26 cp and the 90th percentile 140 cp, against
reverse futility's margin of 80 cp a ply and futility's 100 cp: corrections of
a size that changes those decisions.

**Reading it.** The control is what a useless key gives, so the 29–51% is
signal. Two things flatter it: samples of one structure inside one search
tree are correlated (siblings and transpositions), and the Stockfish-rule row
selects bounds by the sign of the error. In a game the table also learns
across moves, which the single-position bench can't show. Take "a third to a
half of the evaluation's error is structure-correlated" as the order of
magnitude, not a figure.

## The table

| | |
|:--|:--|
| Storage | `[[i32; 16384]; 2]`, owned by `Searcher` beside the history tables, allocated once, 128 KB. Indexed by the side to move and the top 14 bits of a mix of the pawn bitboards: `(white_pawns × 0x9E3779B97F4A7C15) ^ (black_pawns.rotate_left(32) × 0xC2B2AE3D27D4EB4F)`. There is no pawn Zobrist key to reuse; `Position::key()` is the full key. Collisions are accepted: the table is a statistic, and the bench's 50 positions held 8,703 structures |
| Units | Stored in 1/256 cp (`GRAIN = 256`), clamped at ±128 cp (`±128 × 256`). The applied correction is `value / 256`, so it is bounded by construction |
| Reading | `corrected = clamp(raw + value / 256, …)`, kept below the mate range (`is_mate_score` must stay false). `Board::evaluate()` stays the raw network; the correction lives in the search |
| Where it applies | Everywhere the search reads the static evaluation: reverse futility, null-move eligibility, quiet-move futility, and quiescence stand-pat, including the stand-pat value stored in the table as a depth-0 lower bound. Never to a search score, a table score or a mate score, and never twice |
| Update point | Where `negamax` stores to the transposition table, once the score and bound are known: `excluded == NULL`, the node was prunable (not in check, not at the root, no mate-score window), `best` is not a mate score, and the best move is not a capture or a promotion (an absent best move on a fail-low counts as quiet) |
| Bound rule | Exact: always. Lower bound: only if `best > corrected`, since a fail-high says only that the position is at least that good. Upper bound: only if `best < corrected`. Otherwise the bound carries no evidence in its direction and the node is skipped. This is Stockfish's rule; exact-only would learn from 2% of stores |
| Update | `target = clamp(best − raw, −128, 128) × 256`; `w = min(depth + 1, 16)`; `value += (target − value) × w / 256`, integer division, then clamp. Deterministic; one table, no aging |
| Lifecycle | Cleared with the history tables in `clear_hash`, which `ucinewgame` and `bench` use, so the bench stays reproducible. Kept across iterations, aspiration re-searches and the moves of a game, like the histories |
| Constants | Three: the size (2¹⁴ a side), the cap (±128 cp) and the weight schedule (`min(depth + 1, 16)` of 256). Fixed here, before the run; the first PR tunes none of them |

**Aborted searches** store nothing (the existing rule), so they update nothing.
A singular-extension search has `excluded != NULL` and is skipped. Nodes in
check have no static evaluation and are skipped. The quiescence search
doesn't update: its scores are shallow and mostly bounds.

## Plan

1. **This note and the roadmap item** (`docs`), recording the measurement and
   the decisions above before any code.
2. **The feature** (`feat(search)`), one PR: the table, its reads at the four
   sites and the update at the store point, with `stats` counters for updates
   by bound, skips by the bound rule, clamps and the sum of applied
   corrections. Tests: an empty table leaves every evaluation unchanged; the
   same structure with other pieces moved indexes the same entry and white
   and black to move different ones; an update moves the entry towards the
   target and repeated updates stay within the cap; a fail-high never lowers an
   entry and a fail-low never raises one; the corrected value never reaches
   the mate range. The bench signature changes, as a `feat` does; both are
   recorded.
3. **The SPRT**, both legs against `v0.46.2`'s default.

| | |
|:--|:--|
| Question | Does learning the network's structure-correlated error during the search make the engine stronger, per node and at a clock? |
| The one change | The correction table and its use at the four evaluation sites. Margins, reductions, the network, the refresh cache, move ordering and time management are untouched |
| Expectation | Fixed nodes: positive, if the signal above carries into play. Timed: about the same, since the cost per node is a hash and a table read. Speed per node within the bench comment's noise |
| Gate | H1 in both legs. Anything else drops the feature, and the result is recorded here either way. The efficiency bars stand: `max_ebf` 2 and `min_fail_high_first` 0.87; no exception is planned, so a crossing fails the PR |
| Not changed by a failure | If it fails, the measurement above stands; the record should say whether the table learned too little (updates per game), too much (clamps, mean applied correction) or the right thing with no effect |

## Measured

**The implementation** (`crates/search/src/corrhist.rs`, used from `search.rs`):
`CorrectionHistory` holds `[[i32; 16384]; 2]`; `key` mixes the pawn bitboards,
`correction` and `corrected` read, `is_evidence` is the bound rule and `update`
the moving average, all as the table above fixed them. `negamax` computes
`raw_eval` and reads `static_eval` as the corrected value, so reverse futility,
null move and futility see the correction; quiescence corrects its stand-pat
the same way; the update sits inside the transposition-table store. The
`stats` build counts reads, the applied correction, updates by bound, stores
skipped by the bound rule and errors beyond the cap, under `correction` in the
telemetry summary. `clear_hash` clears the table with the histories.

One statistic changed from the plan's wording: "updates held at the cap" is
impossible, since the target is clamped and a moving average never passes its
target (integer division stops it a step short, so an entry tops out at 127
cp). The counter reports errors beyond the cap instead, which is what the
"corrections too large" risk needs.

**Bench, local** (Apple M3 Max, depth 10, at `v0.46.2` plus this change):

| | `v0.46.2` | With the table |
|:--|--:|--:|
| Signature | 976528 | **997279** (+2.1% nodes; identical run to run, and in the `stats` build) |
| Fail-high-first | 0.8783 | 0.8790 |
| EBF | 1.841 | **1.953** (bar 2) |
| Reverse futility cutoffs | 196,570 | 199,052 |
| Null-move cutoffs | 15,316 | 15,743 |
| Futility skips | 836,985 | 844,453 |
| Late-move-pruning skips | 961,946 | 990,132 |
| LMR re-searches | 3,100 | 3,247 |
| Quiescence nodes | 487,783 | 501,796 |
| Nodes/second | 2.61–2.65M | 2.57–2.65M |

The local performance gate passes. Speed per node is within this laptop's
noise, as a hash and a table read should be; the CI bench comment's paired
run is the record.

**What the table did on the bench** (`stats`, the 50 positions searched
independently, the table cleared between them):

| | |
|:--|--:|
| Reads (corrected evaluations) | 827,081 |
| Updates: exact / fail-high / fail-low | 2,747 / 24,212 / 30,503 = 57,462 |
| Stores skipped by the bound rule | 69,828 (55% of eligible stores) |
| Updates whose error was beyond ±128 cp | 11,155 (19%) |
| Mean applied correction | **3.8 cp** |

The mean applied correction is small next to the structure means measured
above (26 cp median) because on a single position's depth-10 search each
entry is young: about 6.6 updates a structure, each moving the entry by at
most 16/256 of the error. In a game the same structures recur move after move
and the entries keep learning. Whether they learn enough at 5+0.05 is the
first risk in the table below, and the SPRT answers it; nothing was tuned to
change it, as the plan requires.

**Bench, CI** (#221's bench comment, base `v0.46.2`, the same shared runner):
signature 976528 → 997279; speed per node −2.0% ± 1.6%; time to depth
+4.2% ± 1.7%; 4,044 instructions a node (+0.6% on #217's 4,018); fail-high-first
87.9% (87.8%); EBF 1.95 (1.84), under the bar of 2; perft pass; performance
gate pass. The comment marks EBF and time to depth red: the search visits 2.1%
more nodes to a depth and each costs a hash, a read and the update. Whether the
better pruning decisions were worth that is the SPRT's question.

**SPRT against `v0.46.2`** ([run 42434032696](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42434032696),
#221, `5e1e76e` against `main` at `58f8e89`; elo0 0, elo1 5, α = β = 0.05; on
demand; one attempt):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **+7.4 ± 4.7** | 99.9% | 6612 | 1359 / 1218 / 4035 | 2.97 | 61, 682, 1705, 771, 87 | H1 |
| Fixed nodes, 200,000 per move | **+9.8 ± 5.7** | 100.0% | 5238 | 1189 / 1042 / 3007 | 2.96 | 34, 665, 1173, 614, 133 | H1 |

**The recorded gate is met:** H1 in both legs, with the plan's constants
untuned. The comment's reading: "smarter per node, and worth its cost at a
clock: merge." Merged as `ff672f3` and released in `v0.47.0`. Self-play against
the previous build at one time control, one implementation.

### Learning-speed pre-test

**The question.** The table removed +7.4 Elo's worth of error while applying a
mean of 3.8 cp on the bench, against structure means of 26 cp. Does it learn
too slowly, or cap too low, to carry what a game teaches it? Measured before
changing a constant, at `v0.47.0` (`e86fb45`).

**Method.** A scratch build (not in the repository) gave the searcher twelve
*passive* correction tables beside the real one, each under a different
schedule, all fed by the same eligible stores and never read by the search.
At every exact store each passive table was scored before its update: the
error of the raw evaluation against the search score, and of the evaluation
corrected by that table. The engine then played 40 self-play games of 100
plies through UCI from the SPRT's openings at 200,000 nodes a move, the
fixed-node budget, with `ucinewgame` between games, so the tables learned
across a game's moves as they do in play. 4,000 searches and 233,802 exact
stores a candidate. Scored by the share of mean squared error and of mean
absolute error removed; the first weights the large late-game errors, the
second doesn't.

| Weight scale (of `depth + 1`, ≤ 16) | Cap | MSE removed | MAE removed | Plies 0–10 | 10–30 | 30–60 | 60–100 (MSE) | Mean \|correction\| |
|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| 256 (current) | 128 (current) | 18.5% | 21.2% | 23.8% | 32.0% | 30.0% | 16.3% | 35 cp |
| 256 | 256 | 27.3% | 24.7% | 25.0% | 38.3% | 37.5% | 25.5% | 46 |
| 256 | 512 | 37.9% | 26.4% | 24.9% | 41.5% | 43.4% | 37.1% | 53 |
| 128 | 128 | 19.0% | 22.5% | 28.5% | 35.2% | 31.9% | 16.5% | 38 |
| 128 | 256 | 27.7% | 26.0% | 29.5% | 41.9% | 39.6% | 25.5% | 49 |
| 128 | 512 | 38.1% | 27.5% | 29.3% | 45.0% | 45.4% | 36.9% | 56 |
| 64 | 128 | 19.3% | 23.5% | 32.6% | 38.1% | 33.5% | 16.5% | 40 |
| 64 | 256 | 27.9% | 26.8% | 33.1% | 44.9% | 41.0% | 25.4% | 52 |
| **64** | **512** | **37.9%** | **28.1%** | 32.5% | 47.8% | 46.3% | 36.4% | 59 |
| 32 | 128 | 19.4% | 24.1% | 35.4% | 40.2% | 34.4% | 16.4% | 43 |
| 32 | 256 | 27.6% | 27.1% | 34.9% | 46.7% | 41.3% | 24.9% | 55 |
| 32 | 512 | 37.0% | 27.9% | 33.3% | 48.9% | 45.7% | 35.3% | 62 |

**What it says.** Two separate effects. **The cap** is where most of the
headroom is: ±128 clips 19% of updates, and each doubling lifts the error
removed, from 18% to 38% by MSE and 21% to 26% by MAE at the current speed.
**Learning speed** matters in the first ten plies (24% → 33% as the scale
falls from 256 to 64) and hardly at all later; 4× faster is near the plateau
and 8× adds nothing. Together, scale 64 with cap 512 removes 28% of absolute
error against 21% today, and 38% of squared error against 18%, close to the
29–51% that the structure means could explain in the first measurement.

**Caveats.** The stream of stores came from a search that used the *current*
table, so a passive table's corrections never changed the tree that scored
them; in play they would. Samples of one structure inside one search are
correlated, which flatters every row alike. The games were 100 plies at fixed
nodes, not 5+0.05 at a clock, though the node counts are similar. And none of
this measures what larger corrections do to pruning decisions; only the SPRT
does.

**Local gate for the two leading schedules** (`bench` at depth 10 from
`e86fb45`, `perf-gate --env local`; the current schedule's row is from #221):

| Schedule | Signature | EBF (bar 2) | Fail-high-first (bar 0.87) | Mean applied correction | Clamped updates |
|:--|--:|--:|--:|--:|--:|
| 256 / 128 (current) | 997279 | 1.953 | 0.879 | 3.8 cp | 19% |
| 64 / 256 | 996922 | 1.859 | 0.876 | 13.8 cp | 8.5% |
| 64 / 512 | 1047521 | 1.941 | 0.876 | 15.5 cp | 2.3% |

Both pass. Speed per node isn't quoted: the work per node is identical and the
laptop was loaded during these runs; the CI bench comment is the record.

### Plan: a faster schedule and a higher cap

Recorded before the run, on 2026-11-08, from `v0.47.0`. **Dropped: it lost
−62.7 ± 15.3 timed and −43.7 ± 13.2 at fixed nodes (#225; the result is below
the table).**

| | |
|:--|:--|
| Question | Does letting the table learn faster and carry larger corrections, which removed 28% of the evaluation's absolute error in the pre-test against 21% today, make the engine stronger? |
| The one change | The schedule: `WEIGHT_SCALE` 256 → **64** and `MAX_CORRECTION` 128 → **512** cp. The pre-test measured the two effects separately (speed early in the game, cap throughout) and chose the pair by one metric fixed beforehand; they are tested together as one schedule |
| Held the same | The table, its index, the bound rule, the four application sites, the eligibility rules, the network, the search and the margins |
| Expectation | Fixed nodes: positive. Timed: about the same, since the cost per node is unchanged. The bench searches 5% more nodes to a depth at this schedule, with EBF 1.94 |
| Gate | H1 in both legs against `v0.47.0`'s engine. Anything else drops the schedule, and the result is recorded here either way. The efficiency bars stand, no exception planned |
| If it fails | 64 / 256 is the fallback, as its own experiment: the pre-test's second choice, with a lower EBF (1.86) and the same node count as today |


**Result** ([run 42503876512](https://github.com/jwallace145/rusty-chess-v2/actions/runs/42503876512),
#225, `5f6c9cf` against `main` at `2acd8d4`; elo0 0, elo1 5, α = β = 0.05; on
demand; one attempt):

| Leg | Elo | LOS | Games | W / L / D | LLR | Pentanomial (LL, LD, WL+DD, WD, WW) | Result |
|:--|--:|--:|--:|:-:|--:|:--|:--|
| Timed, 5+0.05 | **−62.7 ± 15.3** | 0.0% | 762 | 115 / 251 / 396 | −2.95 | 31, 130, 171, 42, 7 | H0 |
| Fixed nodes, 200,000 per move | **−43.7 ± 13.2** | 0.0% | 984 | 188 / 311 / 485 | −2.95 | 33, 148, 226, 79, 6 | H0 |

CI bench (base `v0.47.0`): signature 997279 → 1047521; speed per node
−0.8% ± 2.7%; time to depth +6.0% ± 2.9%; EBF 1.94; fail-high-first 87.6%;
the performance gate passed. **The recorded gate isn't met**, so the schedule is
dropped and the table keeps 256 / 128.

**What it says.** The pre-test ranked schedules by how well the corrected
evaluation predicted the search's exact scores, and the winner by that measure
lost about 44 Elo per node in play. Three things it didn't measure:

1. **Pruning is decided in the tails, not on average.** A correction four
   times larger (15.5 cp mean on the bench against 3.8) feeds reverse
   futility, futility and stand-pat directly. Being closer to the search score
   on average says nothing about how often a large correction pushes a node
   over a margin it shouldn't cross; the games say it did, often.
2. **The passive tables never shaped the tree that scored them.** Their
   corrections were read from a search that used the old schedule, so the
   pre-test could not see a feedback loop: bigger corrections change which
   nodes are searched, which changes what the table learns.
3. **Exact stores are a biased sample.** They're the nodes whose score fell
   inside the window, which are the quiet ones; the corrections that hurt act
   on nodes that fail high or low.

So the offline error metric is useful for one question, whether a table has
signal at all (the first measurement, which the +7.4 confirmed), and not for
tuning what the table does to pruning. The fallback named in the plan
(64 / 256) is not run on this evidence: the cap is the likelier cause, and
the right next pre-test for a schedule is one that plays games.


| Risk | How it shows | What's planned |
|:--|:--|:--|
| Too few updates at 5+0.05 to learn in a game | Few updates a search in the counters; a fixed-node gain with no timed gain | The bound rule, not exact-only, from the start: 22× the updates on the bench |
| Corrections too large | Many clamps; a rise in reverse futility or futility cutoffs with worse results | The ±128 cp cap; the weight schedule's slow start at shallow depth |
| The bench's tree-internal correlation overstated the signal | A fixed-node result near zero | The SPRT decides; the record keeps the pre-test and the result side by side |
| EBF crosses the bar of 2 | The performance gate fails whatever the SPRT says | No exception is planned; the PR would be dropped or reworked |
| A collision pairs two unlike structures | Noise in both entries | Accepted; 2¹⁴ entries a side against thousands of structures a game |
