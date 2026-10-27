# Lessons from v1 (rusty-chess)

Oct 26, 2026

A post-mortem of the first engine (`jwallace145/rusty-chess`), written to feed the v2 roadmap. Findings come from reading the search, quiescence, transposition table, evaluation, and move generation code, not from profiling. Line references are to `src/search/minimax.rs` unless noted.

v1 topped out around 1400 Elo at roughly 300k NPS with a depth cap of 8. The diagnosis at the time was "too slow." The code shows two search bugs that cost more than the speed did, plus a measurement that pointed at the wrong problem.

## Bugs that made it play badly

### 1. The root never narrowed alpha

`search_depth` (~line 448) searched every root move with the full window `(i32::MIN + 1, i32::MAX)`. It tracked `best_score` separately but never passed it back as alpha. Each root subtree was therefore searched at full width as if it were the first move.

Most of alpha-beta's savings come from the root window tightening after the first move; this discarded all of it.

**v2 guard:** the root is the same negamax function as every other node, with `-beta, -alpha` passed to children and alpha raised on improvement. The fail-high-first counter at ply 0 should sit near 90% on bench.

### 2. Timeouts returned 0 as a real score

`alpha_beta_with_time` returned `0` when time expired. The parent treated that as a legitimate score, compared it against siblings, and stored it in the TT. Then `search_depth` returned `(best_score, Some(best_move))` from the aborted iteration, and the iterative deepening loop (~line 343) overwrote the previous completed iteration's best move with it.

Result: whenever time expired mid-iteration, the engine could play a move chosen from a half-searched tree where unsearched branches scored as draws, with a TT now seeded with fake zeros. This is the most likely source of the random-looking blunders.

**v2 guard:** an aborted iteration is discarded entirely. The search sets a `stopped` flag; on unwinding, no TT stores happen and no root result is accepted. The best move is always the one from the last *completed* iteration. The bench signature (node count at fixed depth, no time limit) must be deterministic across runs; any variance means abort handling is leaking into scores.

### 3. Quiescence scores stored as `Exact`

At depth 0 (~line 570), the qsearch result was stored in the TT with `Bound::Exact`. Qsearch is fail-hard and returns `alpha` or `beta` bounds, not exact scores. Any later probe of that position at depth 0 got a bound back as if it were the true evaluation.

**v2 guard:** bound type is derived from the final score's relation to the original alpha/beta at every node, including leaves. If qsearch gets its own TT entries, they use the same rule.

### 4. PV-first ordering was dead code

Iteration *d* stored its best move at `pv_table[d]`; iteration *d+1* read `pv_table[d+1]`, which was always `None`. The TT move mostly covered for it, so play didn't suffer much, but nothing surfaced the fact that a feature was contributing zero cutoffs.

**v2 guard:** cutoffs are counted per move-picker stage. A stage with zero cutoffs over a bench run is a bug until proven otherwise.

## Why NPS was stuck at 300k

### 5. Legality by make-and-check, twice per leaf

`generate_legal_moves` made every pseudo-legal move on a board copy and called `in_check`. At depth 0 the node ran that full legal generation anyway (to detect mate) before calling qsearch, which generated noisy moves and ran the same make-and-check filter a second time. Every leaf paid for two full legality passes.

**v2 guard:** legal move generation with pin masks and check masks, no make-and-test. Perft NPS is the gate: 100M+ bulk-counted.

### 6. The evaluator

Thirteen boxed `dyn BoardEvaluator` sub-evaluators ran in full at every qsearch node (`src/eval/evaluator.rs`). Mobility, threat, line pressure, and fork detection each scanned attacks across the board. Nothing was incremental. Eval was almost certainly the largest slice of the profile, larger than move generation.

The eval also had known bugs (threat evaluator rewarding easily-neutralized threats, king safety double-counting) that were debugged by hand with no way to check whether a fix helped.

**v2 guard:** evaluation is frozen at tapered PeSTO piece-square tables, updated incrementally in make/unmake, until the Phase 4 exit. Every eval change after that passes SPRT or is reverted. A mirror-symmetry test (`eval(pos) == eval(mirror(pos))`) runs on every bench position.

### 7. Allocation and sorting in the hot loop

`move_buffer.to_vec()` allocated at every interior node; `Vec::with_capacity(64)` allocated at every qsearch node. `sort_by_key` recomputed `piece_on` lookups on every comparison rather than once per move.

**v2 guard:** stack-allocated move lists, a staged move picker that scores each move once, and a counting allocator in debug builds that panics if the search path allocates.

## The metric pointed at the wrong problem

The "10–14% beta cutoff rate" that drove the slowness diagnosis was `beta_cutoffs / nodes_explored`. Qsearch nodes were not counted at all, and leaf nodes dominated the count, so the ratio said little about ordering quality. A well-ordered tree can show similar numbers.

The number that would have exposed bug 1 immediately is fail-high-on-first-move. Bug 2 would have shown as a bench signature that varied between runs at the same depth. Bug 4 would have shown as a zero count on the PV stage.

**v2 guard:** the `stats` feature counts fail-highs by move index, cutoffs by picker stage, TT probe/hit/cutoff, qsearch node share, and effective branching factor, emitted as JSON lines per iteration. These are the Phase 3 exit criteria.

## Missing features

v1 had no principal variation search, null-move pruning, late move reductions, aspiration windows, static exchange evaluation, or check evasions in quiescence. Mate scores were offset by remaining depth rather than ply, which made them inconsistent across TT reuse. Depth was capped at 8 and time management ran until a minimum time had elapsed, continuing past the depth cap.

Combined with bugs 1 and 2, effective depth was probably 5 to 6, which is consistent with the ~1400 result. These are Phase 4 items in the roadmap, each gated by SPRT.

## Summary for the agent

When writing the v2 search loop, the three things most likely to recur are:

1. Forgetting to narrow alpha at the root (treat the root as an ordinary node).
2. Letting an aborted search leak scores into the TT or the root result.
3. Storing bounded scores as exact.

The bench signature, the fail-high-first counter, and per-stage cutoff counts exist specifically to catch these. Run them after every search change.
