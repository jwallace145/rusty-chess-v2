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

## What could go wrong

| Risk | How it shows | What's planned |
|:--|:--|:--|
| Too few updates at 5+0.05 to learn in a game | Few updates a search in the counters; a fixed-node gain with no timed gain | The bound rule, not exact-only, from the start: 22× the updates on the bench |
| Corrections too large | Many clamps; a rise in reverse futility or futility cutoffs with worse results | The ±128 cp cap; the weight schedule's slow start at shallow depth |
| The bench's tree-internal correlation overstated the signal | A fixed-node result near zero | The SPRT decides; the record keeps the pre-test and the result side by side |
| EBF crosses the bar of 2 | The performance gate fails whatever the SPRT says | No exception is planned; the PR would be dropped or reworked |
| A collision pairs two unlike structures | Noise in both entries | Accepted; 2¹⁴ entries a side against thousands of structures a game |
