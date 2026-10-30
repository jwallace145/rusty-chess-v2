# Chess Engine Roadmap

A living checklist. Started Oct 26, 2026 · @Jimmy, maintained as the engine is built; last reviewed 2026-10-30 (`v0.26.0`).

**Status:** Phases 0–4b complete and released (`v0.17.0` closed Phase 4, `v0.25.0` Phase 4b). **Phase 6 (NNUE) is in progress:** generation 0's self-play data (231M positions) is generated and fully audited, the engine's NNUE inference (`v0.26.0`) and the `bullet` trainer are built and cross-checked, and the first net trains on a GPU task as soon as the `gpu` profile is applied. Phase 5 (first public release) runs alongside. Strength: about 2700–2750 on the Stockfish ladder (`v0.26.0`: 2728 ± 38), PeSTO evaluation.

## How to use this document

- Every item is a checkbox. Check it when the PR that delivers it merges, and write the PR number and the measured number next to it. Numbers are the point: "done" without a number is not done.
- Exit gates are checklist items too. A phase is complete when every gate item is checked. The next phase does not start before that.
- New learnings go under [Learnings](#learnings) with a date. New phases are appended; nothing is deleted, only checked or struck through with a reason.
- Everything is measured on standardized hardware, never on someone's laptop: benchmarks on every PR and release on `ubuntu-latest` runners; SPRT, matches and data generation on dedicated, ephemeral 64-core `c8a.16xlarge` instances (jwall-engineering-infra's remote tasks); NNUE training on a dedicated GPU instance (`g6.xlarge`, NVIDIA L4). Local numbers are for development only.

## Goals and principles

The goal is a single-threaded Rust UCI engine whose classical search is measurably strong before any NNUE work starts. Every change has to be justified by a number.

**Why attempt one stalled.** Speed was only part of it: 300k NPS is slow, but survivable. The bigger issue was a search that never narrowed alpha at the root and leaked aborted scores into results, so alpha-beta degraded toward minimax. At that point no NPS gain helps. The full post-mortem is [lessons-from-rusty-chess-v1.md](lessons-from-rusty-chess-v1.md); its three recurring bugs are rules in [AGENTS.md](../AGENTS.md).

**Decisions made**

- Fresh repo, Rust, single-threaded first (the transposition table is designed so Lazy SMP can be added later)
- Classical search must hit its efficiency targets before evaluation gets fancy; NNUE is the destination
- Telemetry is JSON lines plus CI artifacts and release assets; no dashboard
- Distribution via a Homebrew tap; an HTTP API is a possible later phase
- From scratch: no chess libraries anywhere in the dependency tree. The PeSTO table values are the one borrowed input, credited, and replaced by our own NNUE in Phase 6

**Principles**

1. **Correct before fast.** Perft passes on the full suite before any search code is written, and it stays in CI permanently.
2. **Measure three layers separately:** move generation speed, search speed, and search efficiency. Each one fails differently.
3. **Elo is the final judge.** Every search or evaluation change gets an SPRT run. A feature that doesn't gain Elo gets reverted, even if it looks right.
4. **One feature per commit.** The bench signature makes behavior changes visible, so regressions can be bisected.
5. **No heap allocation in the search path.** A counting allocator test enforces it.
6. **Standardized measurement.** Benchmarks, SPRT, matches, data generation and training run on infrastructure we configure, so every author's change is measured on the same hardware with the same flags.
7. **Data before nets.** Every dataset passes `datagen-audit` (zero correctness failures, quality bars) before a net trains on it, and every net passes the engine-to-trainer check (`nnue-eval`) before it is tested.

## Measurement

| Layer | What it measures | How | Bar (`perf-targets.toml`) |
| --- | --- | --- | --- |
| Move generation | Perft NPS (bulk counting) over `tests/perft.epd` | `rusty-chess perft --suite` | `movegen.min_perft_nps` 100M local and CI |
| Search speed | Paired NPS ratio against the base on one runner; instructions per node (cachegrind, chip-independent); absolute NPS with the CPU model, informational | `speed-report paired`, `speed-report instructions`, `rusty-chess bench` | `search.min_nps` 2M local, 1M CI (absolute, release PRs) |
| Search efficiency | Fail-high-first, EBF, TT hit rate, qsearch share, cutoffs by stage | `bench` with `--features stats` | `efficiency.min_fail_high_first`, `max_ebf` (set at the Phase 3 gate) |
| Strength | Elo against the PR's base, timed (5+0.05) and at fixed nodes (200k), bounds [0, 5], at most 5000 pairs | fastchess SPRT on dedicated 64-core instances, `sprt` label | H1 accepted in the timed test, or the PR does not merge |
| Head-to-head | Elo between any two refs with a 99% pentanomial interval | the `Match` workflow, on a dedicated instance | A measurement, not a gate |
| Absolute strength | Rating against Stockfish 19 at `UCI_Elo` 2800/2950/3100/3190 at 10+0.1 | the `Rating` workflow, on every release | Tracked per release; ±35–40 at 100 games a rung, a ballpark until a public rating in Phase 5 |
| Training data | Correctness (every record against its replayed game, every result against its final position, the trainer's parser) and quality (draw rate, duplicates, score spread, phases) | `datagen-audit check` | Zero P0 failures; P1 bars in `datagen.toml` |
| Nets | The engine's quantized evaluation against the trainer's float evaluation | `nnue-eval --check` | Within max(50 cp, 5%) per position, mean ≤ 20 cp |

- **Stats counters** (`--features stats`, every increment a macro that compiles to nothing otherwise): nodes and qsearch nodes per ply; TT probes, hits, cutoffs, collisions; fail-highs by move index (index 0 is fail-high-first); cutoffs by move-picker stage; null-move cutoffs, LMR reductions and re-searches, futility skips.
- **JSON lines** (`"v":1`): one `iter` record per iteration, one `summary` per search, one `bench` and one `perft` record per run. The schema doubles as a future API response shape.
- **Bench signature.** `bench` searches 50 fixed positions to a fixed depth with a fixed hash size and prints the total node count. A changed signature means search behaviour changed; the same signature with a different NPS is a pure speed change. The signature goes in the PR description as `Bench: <nodes>`; CI checks the last such line against its own run.
- **CI on every PR.** The `Bench` job runs bench (with and without stats) and the perft suite, uploads the JSON lines, runs `perf-gate` (speed bars reported on PRs and enforced on release PRs, efficiency bars always enforced, bars only ratchet up), and posts one sticky comment with deltas against the latest release and a collapsed explanation of every row.
- **Every release** carries its `bench.jsonl`, `stats.jsonl` and `perft.jsonl` as assets and the signature in its notes; the PR comment's baseline is read from there.
- **SPRT.** A PR labelled `sprt` builds the PR and its base with identical flags and plays two fastchess SPRTs at once from our own generated openings, each on its own dedicated 64-core `c8a.16xlarge` (60 games at once): timed at 5+0.05, the gate, on demand so spot cannot interrupt it; and at 200k nodes a move, informational, on spot. Bounds [0, 5] Elo, at most 5000 pairs: a +25 change settles in about 10 minutes, a neutral one runs to the cap in about 50. One sticky comment carries both and reads the pair; only an accepted H1 in the timed test passes. A spot interruption is reported as such, never as a result. History: fixed nodes alone gated until 2026-10-29 and overstated changes that make nodes dearer; GitHub-hosted runners (4 games at once, [0, 10], 1000 pairs) ran it until #91 and #96.

## Phases

### Phase 0: Scaffolding and telemetry skeleton — complete, `v0.1.0`

- [x] `chore: initial scaffold` pushed directly to `main`, the only direct push ever
- [x] Conventions PR: commitlint, pre-commit, PR templates including `perf.md`, `AGENTS.md`, `CONTRIBUTING.md`, Dependabot, `ci.yml` (#1)
- [x] Release Please with `release-type: rust` and the `release` job building macOS arm64/x86_64 and Linux x86_64 binaries (#2, #6)
- [x] Merge settings and the `main` ruleset applied; a multi-line commit fails CI (#5), a non-conventional title fails (#4), a co-author trailer passes
- [x] `stats` feature with no-op macros, the JSON lines writer, `bench` and `perft` subcommands (#3)
- [x] **Exit:** CI uploads `bench.jsonl` on every PR; `v0.1.0` cut by Release Please with binaries attached
- [x] Added since: the perft suite runner and `perf-gate` (#10), the PR bench comment (#11, #16), release telemetry assets (#12), the post-mortem and its guards (#20, #22)

### Phase 1: Board and move generation — complete, `v0.2.0`

- [x] Squares, bitboards, `const` leaper and line tables, fancy magic bitboards with magics from an in-repo finder (#13)
- [x] 16-bit moves, `Position` with FEN, incremental Zobrist keys verified against a full recompute, make/unmake with an undo record (#14)
- [x] Legal move generation with check and pin masks, en passant verified by hand, perft with bulk counting, `perft --suite` writing `perft.jsonl` (#14)
- [x] **Exit:** the full suite passes: 63 positions, 283 checks to depth 7; bulk perft 278M NPS on an M-series laptop, 200–380M on CI runners (bar 100M)

### Phase 2: Baseline search — complete, `v0.3.0`

- [x] Tapered PeSTO evaluation, updated incrementally in make/unmake, mirror-symmetric on every bench position (#18, #22)
- [x] Iterative deepening, fail-soft negamax alpha-beta, captures-only quiescence, MVV-LVA ordering, mate scores by ply (#19)
- [x] UCI `position`, `go` with every limit, `stop`, `ucinewgame`, `d`; search on a worker thread; simple time management (#19)
- [x] Repetition and fifty-move detection (#18)
- [x] **Exit:** legal games (in-repo self-play test plus UCI games), signature `68269801` at depth 6 identical across builds and runs, `iter` and `summary` records on every search
- [x] Baseline numbers to beat: search NPS 7–11M, fail-high-first 0.70, EBF 7.00 (bench-wide; an earlier 3.35 was one position's), qsearch share 0.92

### Phase 3: Move ordering and the transposition table — complete, `v0.7.0`

This is the gate that stopped attempt one. Each item is its own PR with the stats delta visible in the bench comment and an SPRT run.

- [x] SPRT on CI: `sprt.yml` on the `sprt` label, node-limited fastchess games against the PR base from our own openings, sticky comment, only an accepted H1 passes (since 2026-10-28; before that an inconclusive run passed); `sprt-quick` for a two-minute pipeline smoke test (#24)
- [x] Openings generator in `crates/tools` and a committed `tests/openings/openings.epd`: 300 positions, 8 random plies, within ±60 cp at depth 4, fixed seed (#24)
- [x] Transposition table: 10-byte entries in 3-entry clusters, key verification, depth-and-age replacement, mate scores adjusted by ply, TT move searched first, bound derived from the original window at every node, no store after an abort; `Hash` UCI option; `tt_hit` in the iteration records. SPRT +197.5 ± 43.4 Elo in 212 games; fail-high-first 0.70 → 0.79, EBF 7.00 → 4.62 (bench-wide), signature 68269801 → 14255682 (#26)
- [x] Benchmark history: each release is measured once on its Release Please PR; the raw data is committed under `benchmarks/v<version>/` and attached to the GitHub release, and `docs/benchmarks.md`, the CHANGELOG entry and the release notes are derived from it (#30, #32)
- [x] Hardware-independent speed: PRs are judged by paired A/B runs against their base on one runner and by cachegrind instructions per node, which every release records; absolute NPS is informational with the CPU model beside it (#36)
- [x] Staged move picker: TT move → good captures (MVV-LVA, filtered by SEE) → killers → history quiets → bad captures; every stage shows cutoffs on bench. SPRT +18.5 ± 12.1 Elo in 1,976 games; fail-high-first 0.79 → 0.86 (#29)
- [x] Two killer slots per ply. SPRT +97.1 ± 32.6 Elo in 378 games; fail-high-first 0.86 → 0.97, 40% fewer bench nodes (#33)
- [x] Butterfly history with gravity. SPRT +55.0 ± 22.5 Elo in 554 games; fail-high-first 0.968 → 0.975, EBF 4.33 → 4.18 (#34)
- [x] **Exit:** fail-high-first ≥ 0.90 on bench: **0.975** (signature 8,243,750; every picker stage cuts off: TT 97,681 / good captures 502,264 / killers 221,741 / history quiets 8,795 / bad captures 165)
- [x] **Exit:** each item passed SPRT on its own (TT, picker, killers, history: all H1)
- [x] **Exit:** the bars set in `perf-targets.toml`: `min_fail_high_first = 0.95`, `max_ebf = 4.5`, enforced on every PR and only ratcheting
- [x] **Exit:** EBF ≤ 3 on bench: not met by ordering alone (**4.18**; 7.00 at Phase 2, 4.62 with the TT; history moved it by 0.15). **Decided 2026-10-27:** moved into Phase 4 as an intermediate gate, since the remaining reduction is what PVS, null-move pruning and LMR do

### Phase 4: Selectivity — complete, `v0.17.0`

Each item is its own PR with an SPRT run; the first intermediate gate is the EBF ≤ 3 carried over from Phase 3.

- [x] Principal variation search: the first move gets the full window, every later move a null window re-searched in full when it beats alpha. SPRT inconclusive at the 1000-round cap, +11.6 ± 11.4 Elo, LOS 97.7% in 2000 games (merged on LOS); EBF 4.18 → 3.86, signature 8243750 → 7143901 (#44, v0.9.0)
- [x] Aspiration windows: ±35 cp around the previous score from depth 4, widened on failure (`aspiration_researches` counter). SPRT inconclusive at the cap, +13.2 ± 10.8 Elo, LOS 99.2% in 2000 games (merged on LOS); signature 7143901 → 7082718 (#46, v0.10.0)
- [x] Null-move pruning: R = 2 + depth/4 from depth 3, non-pawn-material guard, never twice in a row, mate scores from a null search clamped to beta. SPRT +130.8 ± 36.4 Elo, LOS 100% in 278 games; EBF 3.90 → 2.86 (the ≤ 3 gate), signature 7082718 → 2932004 (#47, v0.11.0)
- [x] Late move reductions: log table 0.75 + ln(depth)·ln(index)/2.25 for quiet moves from index 2 at depth ≥ 3, not in or giving check, re-searched at full depth when the reduced search beats alpha. SPRT +204.2 ± 46.8 Elo, LOS 100% in 212 games; EBF 2.86 → 2.17, fail-high-first 0.975 → 0.93 (bar lowered to 0.92 with a declared exception), signature 2932004 → 716057 (#48, v0.11.0)
- [x] Reverse futility pruning: at null-window nodes up to depth 6, not in check, return the static evaluation when it beats beta by 80 cp per ply of depth left (`rfp_cutoffs` counter). Signature 716057 → 601010, fail-high-first 0.93 → 0.90 (bar lowered to 0.88, EBF bar tightened to 2.5); SPRT +65.2 ± 24.2 Elo, LOS 100% in 464 games (#53, v0.12.0)
- [x] Late move pruning: at null-window nodes up to depth 4, not in check and not being mated, quiet moves from index 3 + depth² on are skipped without a search (`lmp_skips` counter). Signature 601010 → 387631, EBF 2.17 → 1.91, fail-high-first 0.90 → 0.89; SPRT +143.2 ± 34.9 Elo, LOS 100% in 246 games (#55, v0.13.0)
- [x] Check extensions, mate-distance pruning: the window is clamped to the mates reachable from the ply (`mdp_cutoffs`), and late move pruning never skips a move that gives check, decided before the move is made by `Position::gives_check` (direct, discovered, en passant and castling checks; verified against making the move on every suite position) and evaluated only for a move about to be skipped (`lmp_checks_kept`). The one-ply check extension was built and dropped: at fixed nodes it was +9.7 ± 12.4 Elo (LOS 94%, inconclusive) for 30% more nodes and 9% more instructions per node, a net loss at a clock. Depth-10 bench: signature 3629136 → 3775127 (+4%; mate-distance pruning alone leaves it unchanged); a mate in two whose mating move is a quiet check late in the list (`k7/8/2K5/8/8/8/8/7R w`) is found at depth 5 where v0.13.0 misses it; SPRT +24.5 ± 14.0 Elo, LOS 100% in 1252 games; paired speed −2.3% ± 0.6%, 1612 instructions per node (#57, v0.14.0)
- [x] Quiescence: in check, every evasion is searched and having none is mate (a correctness fix: it stood pat before); losing captures skipped by static exchange evaluation, never in check (`qsee_skips`). Depth-10 bench: quiescence nodes 3.83M → 1.68M (−56%), signature 4629402 → 2566403 (−45%), EBF 1.69 → 2.02 (the ratio counts quiescence nodes; bar loosened 2.0 → 2.2 with a declared exception); SPRT +111.7 ± 32.8 Elo for the SEE pruning alone (296 games), +75.6 ± 26.4 for both against `main` (420 games); time to depth −2.5% (#63, #65, v0.15.0)
- [x] Quiescence: transposition table probed (any depth suffices at the horizon), the stored move searched first, stand-pat cutoffs and results stored at depth 0 with the bound from the original window (`qtt_probes`, `qtt_cutoffs`). Depth-10 bench: signature 2566403 → 2077084 (−19%), quiescence nodes −24%, EBF 2.02 → 1.82 (bar back to 2.0, exception removed), main-search TT hit 0.28 → 0.50; SPRT +69.4 ± 24.7 Elo (436 games); time to depth −11.5% (#66, v0.16.0)
- [x] Improved time management: measured, nothing kept. Three timed SPRTs at 5+0.05 found no version better than the original plan (a 30th of the clock plus ¾ of the increment, 3× hard): #59 with new budgets and a stable-move cut, −4.2 ± 10.8 and, retuned to spend the same clock as `main`, −7.5 ± 11.3; #73 with only the guard against starting an iteration predicted to overrun the hard limit, clearly negative and cancelled. The PGNs showed the base reaching the same depth with the same clock use, so the plan's overruns are not waste at short controls. Kept from the work: the `sprt-tc` timed mode (#70) and the `Move Overhead` option (#71, v0.17.0). The long-control overrun the plan is known to have waits for a long-control test, which needs the standardized runner of Phase 5
- [x] **Intermediate gate:** EBF ≤ 3 on bench (carried over from Phase 3; 4.18 at its close): 2.86 with null-move pruning, 2.17 with late move reductions
- [x] Bench depth raised once EBF allows it (the signature changes; say so in the PR): 6 → 10 after late move pruning left the depth-6 bench under 100 ms; signature 387631 → 3629136, EBF 1.91 → 1.77, fail-high-first 0.89 → 0.905 at the new depth, EBF bar tightened 2.5 → 2.0 (#60)
- [x] **Exit:** EBF ≈ 2: **1.82** on the depth-10 bench. Each technique gained Elo under SPRT: H1 accepted for null-move pruning (+130.8), late move reductions (+204.2), reverse futility pruning (+65.2), late move pruning (+143.2), mate-distance pruning with the checking-move exemption (+24.5), the quiescence fix with SEE pruning (+75.6) and the quiescence transposition table (+69.4); PVS (+11.6, LOS 97.7%) and aspiration windows (+13.2, LOS 99.2%) merged on LOS before the H1-only rule (#77). Strength: about 2600 on the Stockfish ladder (v0.15.0 ~2638 ± 43, v0.16.0 ~2587 ± 43); the CCRL figure needs a tester and a Windows build (Phase 5). **Decided 2026-10-28:** the engine should be stronger before NNUE, since its self-play data is only as good as its search, so Phase 4b below precedes Phase 6; evaluation terms are not worth writing because NNUE replaces them

### Phase 4b: Second-tier search — complete, `v0.25.0`

The standard techniques the engine still lacked, each worth 10–40 Elo and all of them kept by an NNUE engine. One PR and one release each; an accepted H1 in the timed SPRT is required (#77). Ordered as built.

- [x] Continuation history one and two plies back (`PieceTo` tables beside the butterfly history, gravity, in the quiet ordering and the history malus). Fixed-node SPRT +18.1 (H1, before the timed gate existed); a 10+0.1 match against v0.17.0 measured −7.0 ± 21.7, neutral at a clock (#79, v0.18.0). Tables stored as `i16`, halving them: same signature, +2.0% speed, −2.0% time to depth (#80, v0.19.0). The separate counter-move heuristic was not built: the one-ply continuation table holds what it would. A `perf` pass to make continuation history cheaper is still open
- [x] Futility pruning of quiet moves at low depth: at null-window nodes up to depth 6, not in check, after the first move, a quiet move (checking moves included) is skipped when the static evaluation plus 100 + 100 cp per ply of depth left does not reach alpha (`futility_skips`). Depth-10 bench: signature 1991209 → 1717816 (−13.7%), fail-high-first 0.906, a million quiets skipped. The first form exempted checking moves (1890442, −5.1%): +25.6 ± 14.6 at fixed nodes (H1) but +5.9 ± 11.1 at 5+0.05, inconclusive at the 1000-pair cap, the check test per candidate eating the gain. This form: +25.6 ± 14.4 timed (H1, 1226 games), +45.0 ± 19.7 at fixed nodes (H1); time to depth −10.9%, EBF 1.77 → 1.64 (#81, v0.20.0)
- [x] Internal iterative reductions at nodes without a table move: below the root, from depth 4, a node with no table move is searched one ply shallower (`iir_reductions`). Depth-10 bench on top of futility pruning: signature 1717816 → 1416355 (−17.5%), fail-high-first 0.906 → 0.899, time to depth about 18% lower. An early read against v0.19.0 (without futility pruning) on the dedicated instances: +28.2 ± 14.8 timed, H1 (#90). Against v0.20.0, on the dedicated instances: +16.7 ± 11.0 timed (H1, 1936 games), +22.8 ± 13.6 at fixed nodes (H1); time to depth −18.2% (#89, v0.21.0)
- [x] SEE pruning in the main search: at nodes up to depth 8, not in check, after the first move, a quiet move whose static exchange loses more than 64 cp per ply of depth left is skipped (`see_skips`). Depth-10 bench on top of IIR: signature 1416355 → 1075807 (−24.0%), fail-high-first 0.899 → 0.890, time to depth about 18% lower. Pruning losing captures too (20 cp × depth²) cut to 1078392 but dropped fail-high-first to 0.873, below its 0.88 bar, so captures are left to the picker's bad-capture stage. +26.1 ± 14.3 timed (H1, 1122 games), +46.4 ± 20.0 at fixed nodes (H1); time to depth −19.9%, EBF 1.78 → 1.59 (#93, v0.22.0)
- [x] ~~The improving heuristic (static evaluation against two plies ago) driving the reverse futility and late move pruning margins~~ **Dropped on record; revisit after NNUE** (Phase 6). A node is improving when its static evaluation beats the one two plies earlier (four, when that side was in check). Two forms, on top of v0.22.0 (signature 1075807): reverse futility pruning with one ply's less margin when improving *and* late move pruning searching half as many quiets when not (815462, time to depth −25%) was −1.9 ± 10.9 timed and +8.5 ± 10.9 at fixed nodes, inconclusive at the old [0, 10] cap; reverse futility alone (942258, time to depth −13%) was H0 at fixed nodes, −29.6 ± 11.9, and −16.9 ± 10.9 timed when spot interrupted it at 1918 games. Both raised EBF (1.59 → 1.65 and 1.68); reverse futility cutoffs nearly doubled (81k → 157k), most of them wrong. A PeSTO evaluation swings too much from ply to ply for "better than two plies ago" to mean much; with NNUE it should (#95). Also untried: one more LMR ply when not improving (762220 nodes), for the reduction-tuning item
- [x] Singular extensions: below the root, from depth 8, when the table holds a lower bound or exact score for its move searched at least depth − 3, the other moves are searched at half depth against a window 2 cp per ply below that score; if none reaches it the table move is searched a ply deeper, and if the node clears beta even without it, it fails high at once (multi-cut). Extensions stop at twice the iteration's depth (`se_extensions`, `se_multicuts`). Depth-10 bench: signature 1075807 → 1529684 (+42%, the extended lines and the verification searches), 377 extensions and 160 multi-cuts. +16.6 ± 8.1 timed (H1, 3260 games, the first gate at [0, 5]), +27.1 ± 10.6 at fixed nodes (H1); time to depth +44.9%, EBF 1.59 → 1.81, the deeper lines' cost (#99, v0.23.0)
- [x] History-adjusted late move reductions and reduction tuning: a quiet move's reduction moves one ply per 512 points of its combined history (butterfly plus both continuation tables), less for good history and more for bad, clamped to 0..depth left. Depth-10 bench on top of singular extensions: signature 1529684 → 1697503 (+11%), fail-high-first 0.885 → 0.882. Divisors 8,192 to 2,048 barely changed the tree (our bonus is depth² capped at 1,200, so scores stay small). The first form, 1,024 (1546957), was H1 at fixed nodes (+7.5 ± 5.1) but +2.8 ± 5.6 at a clock when spot interrupted it at 6,430 games; this form: +15.2 ± 7.6 timed (H1, 3580 games, timed leg on demand), +19.9 ± 9.2 at fixed nodes (H1); time to depth +16.6%, EBF 1.81 → 1.90 (#101, v0.24.0). Reduction tuning beyond the history term is not done; one more LMR ply when not improving waits for NNUE with the improving heuristic
- [x] Pondering (`go ponder`, `ponderhit`), for Lichess, where the opponent's clock is free time: the clock is off while pondering and starts at `ponderhit` with the `go ponder` limits; no `bestmove` before `ponderhit` or `stop`; every `bestmove` names its expected reply (`ponder <move>`); the `Ponder` option is advertised. On for the Lichess bot (`lichess/config.yml`). No search change (bench signature unchanged), so no SPRT: fastchess does not ponder. Verified by protocol tests and by hand (`bestmove e7e5 ponder g1f3` after `ponderhit`) (#108, v0.25.0)
- [x] **Exit:** the list exhausted, each item kept on H1 or dropped on record (the improving heuristic). Together the kept items cut time to depth by more than half (futility −10.9%, IIR −18.2%, SEE pruning −19.9%; singular extensions and history LMR spend some back on deeper lines) and the timed SPRTs summed to about +100 Elo; a 10+0.1 match of v0.22.0 against v0.20.0 measured **+33.8** (99% interval +20.4 to +47.3) for IIR and SEE pruning alone, and v0.23.0 against v0.22.0 **+27.2** (+14.4 to +40.0) for singular extensions. The ladder reads about 2700–2750, not the 2800 hoped for. **Decided 2026-10-30:** go to Phase 6 anyway: the remaining strength is in the evaluation, and the ladder at 100 games a rung (±35–40) cannot resolve 15–25 Elo steps, while matches can
- [x] Testing infrastructure built during Phase 4b, all on the standardized hardware:
  - Timed and fixed-node SPRTs together, the timed one the gate (#84); a configurable `Match` workflow between any two refs (#83)
  - SPRT legs on dedicated 64-core spot instances through jwall-engineering-infra's remote tasks: a dual SPRT in 7 minutes instead of over 2 hours, about 14× (timed) and 20× (fixed nodes) the games per minute (#91)
  - SPRT bounds [0, 5] with at most 5000 pairs, after two PRs ran out of games at [0, 10] (#96); the timed leg on demand after two spot interruptions left gates without a verdict (#105)
  - `Match` on a dedicated instance, with a pentanomial interval at 95, 99 or 99.9% and the games a conclusive answer would take (#97): 2000 games at 10+0.1 in about 20 minutes
  - Ladder rungs moved to 2500/2650/2800/2950 when 2200 saturated at 92–96%, with v0.18.0–v0.23.0 re-rated on them (#102)

### Phase 5: First release

- [x] A rating ladder against limited Stockfish on every release, attached with the logs and PGNs, as a ballpark between public ratings: the `Rating` workflow runs on every published release and writes the Strength section of its notes. Rungs 2800/2950/3100/3190 against Stockfish 19 since v0.28.0's first net (2500/2650/2800/2950 from #102, 2200/2500/2800/3100 before, 1500/1800/2100/2400 through v0.13.0; Ubuntu's Stockfish package, 17.1 last, until then). Still open: the ladder on a dedicated instance with about 400 games a rung (±19 instead of ±35–40)
- [ ] Homebrew tap bumped by the `release` job after the Release Please PR merges (release notes already carry the signature and telemetry since Phase 0)
- [x] Lichess bot account: `lichess/run.sh` connects the engine through lichess-bot for development (see `lichess/README.md`), and an always-on `c8a.medium` run by jwall-engineering-infra plays every published release automatically (#74). The rating it earns is the real-world number; it is not comparable to the ladder's or CCRL's
- [x] The bot's openings: Lichess's opening explorer (masters games, the best-scoring move played at least 50 times) through lichess-bot, never the engine (#86, v0.19.0); pondering on the opponent's clock (#108, v0.25.0)
- [ ] Opening book as a release asset: a from-scratch Polyglot book builder in `crates/tools` from a master-games collection (data credited like PeSTO), published as `rusty-chess-book.bin` beside the binaries; the Lichess bot plays it through lichess-bot's polyglot setting with weighted random selection. The engine itself stays book-free: SPRT, the ladder and CCRL testers all play from their own openings. **Decided 2026-10-28:** no `OwnBook` in the binary; a book built from the engine's own games replaces the master-games book after Phase 6
- [ ] Release matrix for CCRL testers: a Windows x86-64 build and a baseline x86-64 build beside the x86-64-v3 one, named by CPU level
- [ ] Optional: submission to CCRL testers
- [ ] **Exit:** `brew install` works on macOS arm64 and x86_64; the engine is publicly rated somewhere

### Phase 6: NNUE — in progress

The destination named in the goals: our own network, trained on our own games. **Decided 2026-10-30:** the nets are trained on self-play only, never on another engine's evaluations. The engine stays wholly original (for CCRL, tournaments' originality rules and the project itself); the labels are search scores plus game results rather than PeSTO alone, so even a first net learns tactics, king safety and structure PeSTO misses; and each generation is played by the engine carrying the previous net. Distilling Stockfish's scores would reach a given strength sooner and was declined for originality.

**Data**

- [x] `datagen`: self-play at fixed nodes with the engine's own search, from openings of 8 or 10 random plies, writing (position, score, result) records with full game records and manifests; spec [datagen.md](datagen.md), settings `datagen.toml` (#103). The `Datagen` workflow plays on up to three dedicated 64-core spot instances, about 100M positions an instance-hour, and uploads each shard as it completes to an append-only, kept-forever dataset bucket declared in `jwall-engineering-sites.yaml` (#103, #107, #110); a re-run gets its own partition and seeds, runs are capped at 300 minutes (#106)
- [x] `datagen-audit` and [datagen-runbook.md](datagen-runbook.md): every record checked against its replayed game, every result against its final position, every record through bullet's parser, checksums, counts and seeds (P0, must be zero), quality bars (P1); the deep checks (adjudication, repetition, scores against outcomes, openings). Generation-1 settings tuned from generation 0's measurements: win adjudication 1200 cp (99.98% right in 5000), opening limit 250 cp, even random plies; spot interruptions lose only the games in progress (#113)
- [x] **Generation 0:** 231,240,344 positions from 2,387,961 games in 9,329 shards (4.0 GB), engine `v0.25.0`/`3e3bdbd` with PeSTO at 5000 nodes, three runs on 2026-10-30. **Audited in full:** zero failures over every record and every game, 0 of 231M parse failures in bulletformat; draw rate 34–35%, 39% of scores within ±100 cp, 46% endgame positions, about 2% duplicates

**Training**

- [x] The trainer: `trainer/`, a separate Cargo project on `bullet` (CUDA or Apple Metal) so the engine never needs a GPU toolkit. `prepare` splits by whole worker (no game spans the split) and shuffles with a recorded seed, byte-reproducibly; `train` builds bullet's reference (768 → 128) × 2 → 1 SCReLU network at the engine's quantization and scale (400, enforced). `ci/train-task.sh` downloads, prepares, trains, checks and uploads with a full `training.json` ([training.md](training.md), #115)
- [x] The engine-to-trainer check, `nnue-eval`: an independent implementation reproduced the trainer's float evaluation exactly and the engine's quantized evaluation exactly (75 of 75), so the gap is quantization alone (mean 9 cp, worst 28); a perspective bug reads 740 (#115)
- [ ] GPU training tasks: jwall-engineering-infra v0.15.0's `gpu` profile (one on-demand `g6.xlarge`, NVIDIA L4; G and VT quota 5 vCPUs, spot 0) (#116), the `GPU smoke` acceptance test and the `Train` workflow (#117)
- [ ] **Net 1:** 40 superbatches (4.0B positions) at λ 0.5 on generation 0; the run's check passes, training loss falls, validation loss flattens rather than turning up (more passes over 231M positions than ideal; fewer next time if it does)

**Engine**

- [x] Inference: the network, its file format (bullet's simple layout; QA 255, QB 64, SCALE 400, SCReLU, N = 128), a per-ply accumulator stack in `Board` updated by make and unmake with nothing allocated in search, the UCI `EvalFile` option and `bench --eval-file`; PeSTO stays the default (#112, v0.26.0)
- [ ] Net 1 embedded with `include_bytes!` and made the default (PeSTO kept selectable through `EvalFile` for comparisons), `datagen` playing with it; a new bench signature. **The gate:** the timed SPRT against PeSTO. Expected: +80 to +200 Elo at a clock, the fixed-node leg higher (the scalar net is about 19% slower than PeSTO)
- [ ] SIMD for the accumulator and the output (AVX2 on the x86-64-v3 builds, NEON on Apple silicon): a `perf` PR, signature unchanged, winning back the speed gap
- [ ] Re-tune what was tuned to PeSTO's scores: the margins of reverse futility, futility, SEE pruning, null move and aspiration windows; revisit the improving heuristic (#95) and one more LMR ply when not improving
- [ ] **Exit:** the NNUE build beats the PeSTO build under SPRT by a wide margin (100+ Elo is typical for a first net)

**Iterating**

- [ ] Generation 1, played by the engine with net 1 (`generation = 1` in `datagen.toml`, with the tuned settings), audited; net 2 trained on it and kept on a timed SPRT win over net 1. The λ experiment (λ ∈ {0, 0.25, 0.5, 0.75, 1}, one prepared dataset, strength deciding) alongside ([datagen-runbook.md](datagen-runbook.md))
- [x] The ladder moved up when the engine outgrew 2950: v0.28.0 scored 88% on 2650 (about 3000). Rungs 2800/2950/3100/3190 against an official, pinned Stockfish 19, with v0.26.0–v0.28.0 re-rated for the overlap
- [ ] A new anchor once the engine outgrows 3190, the top of Stockfish's `UCI_Elo` range (past about 3300 limited Stockfish cannot bracket it): full Stockfish at a fixed node budget, calibrated against the last limited ladder

### Phase 7: Optional extensions

- [ ] Lazy SMP
- [ ] HTTP API (`api` crate on `search`, `POST /analyze`, worker pool of single-threaded searches, per-request time cap; the JSON lines schema is the response shape). Open question: hosting, and whether requests are public or authenticated
- [ ] Larger nets (256–1024 hidden neurons, more layers, output buckets) and longer iteration once Phase 6's loop is proven, with more data per generation and GPU spot training (a quota increase)

## Repo layout and conventions

```
rusty-chess-v2/
  Cargo.toml   # root package `rusty-chess` (the binary) plus the [workspace]
  src/         # UCI protocol (pondering, EvalFile) + binary entry point (bench, perft subcommands)
  crates/
    core/      # board, bitboards, magics, movegen, zobrist, make/unmake (no deps)
    search/    # search, TT, move picker, histories, PeSTO eval, NNUE inference, SearchStats, telemetry
    tools/     # perft, perf-gate, bench and speed reports, openings, sprt/match/rating reports,
               # datagen, datagen-audit, nnue-eval
  trainer/     # bullet trainer (CUDA or Metal); its own workspace, never in the engine's build
  ci/          # task scripts for the dedicated instances: strength (SPRT), match, datagen, train, gpu-smoke
  tests/       # perft EPD suite, UCI smoke tests, self-play, eval symmetry, openings
  lichess/     # the bot's lichess-bot configuration
  docs/        # roadmap, nnue, training, datagen, datagen runbook, benchmarks, post-mortem
  .github/workflows/  # ci, bench, sprt, match, rating, datagen, train, gpu-smoke,
                      # release-please, release-bench, release-to-lichess
```

The workspace root is itself a package because Release Please's `rust` strategy runs its package updater on the root `Cargo.toml` after bumping every member, and fails on a virtual workspace. Members are listed explicitly with their own `version`; `release-type: simple` with an annotated version would leave `Cargo.lock` stale and break `--locked` builds.

| Tool | Purpose |
| --- | --- |
| Criterion | Microbenchmarks for hot functions (when needed) |
| samply / perf | Flamegraphs on `bench` |
| cargo-show-asm | Inspect codegen of hot loops |
| fastchess | SPRT, matches and the rating ladder |
| bullet | NNUE training (pinned commit), CUDA or Metal |
| jwall-engineering-infra | Dedicated CPU and GPU task instances, the dataset bucket, the Lichess bot |
| cargo-llvm-cov | Coverage, mainly for core |
| Release Please | Versioning and changelog |

The repo follows [repo-conventions.md](repo-conventions.md) in full; [CONTRIBUTING.md](../CONTRIBUTING.md) restates it. Engine-specific: the bench signature goes in the PR description, not the commit; `perf` for speed changes with an unchanged signature, `feat` for search or eval changes that change it, `fix` for correctness; a `feat` whose SPRT failed does not merge; `AGENTS.md` adds the rules agents must follow.

## Distribution and future API

Ship it as a Homebrew-installable UCI binary first. Release CI builds macOS arm64 and x86_64 and Linux x86_64 binaries for a portable CPU baseline (x86-64-v3 / apple-m1) and attaches them to the GitHub release with checksums; Phase 5 adds the tap formula bump. An NNUE net is embedded with `include_bytes!` so the install remains a single file. The API, if pursued, is a thin wrapper added only after the engine is strong.

## Risks and anti-patterns

| Risk | Symptom | Guard |
| --- | --- | --- |
| Poor move ordering | Low fail-high-first, EBF > 4 | Phase 3 gate; per-stage cutoff counters; a stage with zero cutoffs is a bug |
| Aborted search leaking scores | Bench signature varies between runs | Aborted iterations discarded; no TT store after abort; the signature is checked by CI |
| Allocation in search | NPS flat as depth grows, `malloc` in the flamegraph | Stack move lists; the counting-allocator test fails the build |
| Expensive evaluation | Eval dominates the profile | Incremental PSTs; profile eval share on every bench |
| Bugs disguised as features | A pruning change "works" but loses Elo | SPRT on CI for every change; revert on fail |
| TT bugs | Random blunders, unstable signature | Key verification; mate scores by ply; bounds from the original window |
| Eval bugs (e.g. double counting) | Odd positional play | Mirror-symmetry test on every bench position |
| Over-tuning early | Hours on eval terms before search is sound | Eval frozen at PeSTO until the Phase 4 exit |
| Noisy perf numbers | Chasing ±3% NPS changes | Standardized runners; noise thresholds in the comment; node signature, paired time to depth and instructions per node over NPS |
| Fixed-node results flattering slower code | SPRT gains that a clock doesn't show | The timed SPRT is the gate; matches at 10+0.1 check releases (#83, #84) |
| Bad training data | A net that looks fine and plays badly | `datagen-audit`: zero correctness failures before training; quality bars; deep checks per generation |
| Trainer and engine disagreeing | A net that evaluates differently in the engine | `nnue-eval --check` on every net; the scale enforced by the trainer |
| Over-training a small dataset | Validation loss rising late in training | Validation on held-out workers; fewer superbatches or more data next generation |

## Learnings

- **2026-10-30** A test that only sees part of the data can't vouch for the rest: the 1.8M-record sample audit proved the generator correct, but only the full pass (48 batches, all 231M records, 12 minutes) proved every file intact. Both are cheap; run the full one on every dataset that trains a net.
- **2026-10-30** The trainer's score scale is also the engine's output scale: a net trained at `eval_scale` K is read as centipawns by multiplying by K, so training at 200 for an engine using 400 would double every evaluation and every pruning margin in effect. The fitted score-to-result scale of PeSTO-era data (K ≈ 200) is a property of the labels, not a training knob; λ is (#115).
- **2026-10-30** Kill a process group and you kill everything in it: the spot runner's SIGTERM to the task script's group killed `datagen` too, before it could save its shards in progress (about 350k positions lost on the first interruption). `datagen` now runs in its own session and stops through a file (#113). An uploader one shard at a time with three CLI calls kept up with 8 cores and fell five minutes behind 64 (#110); measure throughput at the real scale.
- **2026-10-30** Parallelism buys waiting time, not compute: 60 games at once on one 64-core instance made a dual SPRT 7 minutes instead of over 2 hours at the same cost in core-hours, and that is what made [0, 5] bounds affordable (#91, #96). A gate must not run on interruptible capacity: two timed legs lost to spot in one day moved it on demand for about $1–3 a PR (#105).
- **2026-10-30** A small NNUE trains in minutes, not days: the (768 → 128) × 2 → 1 net has about 99,000 parameters and 32 active inputs a position, and an Apple M-series GPU trains it at 4.7M positions a second, bullet's 40-superbatch schedule in about 15 minutes. The cost of NNUE is the data (hours of 64-core self-play) and the SPRTs, not the training.
- **2026-10-30** A heuristic that reads the evaluation's trend is only as good as the evaluation. The improving heuristic, standard in strong engines, lost at fixed nodes in its mildest form (−29.6 ± 11.9): with PeSTO the static evaluation swings enough between plies that "better than two plies ago" mostly picks noise, and the extra reverse futility cutoffs it allowed were mostly wrong. Dropped until NNUE (#95). Also on 2026-10-30, the SPRT moved to [0, 5] with a 5000-pair cap (#96) after two PRs ended inconclusive at [0, 10]'s 2000 games, and the first live spot interruption was reported as designed: exit 75, no verdict, the partial log kept for reading.
- **2026-10-29** A fixed-node SPRT overstates any change that makes nodes dearer: it gives the change the same number of nodes, so the extra work per node is free. The accepted fixed-node SPRTs from v0.13.0 to v0.18.0 summed to about +190 Elo; a 400-game match of the two releases at 10+0.1 measured +26.1 ± 23.4 (#83). Per release at 10+0.1: the quiescence fix with SEE pruning +22.6 ± 24.3 (fixed-node SPRT +75.6), the quiescence transposition table −11.3 ± 23.4 (+69.4), continuation history −7.0 ± 21.7 (+18.1). The SPRT now runs timed and at fixed nodes together, and only the timed result gates; the two neutral changes are `perf` targets before any revert.
- **2026-10-28** Time management cannot be tested where its waste is: at 5+0.05 the base's overruns past its soft budget finished as deeper moves, and three timed SPRTs of three different plans all lost or tied. A guard against overruns is a long-control change and needs a long-control test; until the standardized runner exists, leave the plan alone.
- **2026-10-28** Nodes per second and fixed-node SPRT both mis-read a change that alters what a node is. Quiescence SEE pruning read −35% NPS and 🔴 while reaching the same depth 16% sooner, and its fixed-node Elo flattered it. The bench comment now carries a paired time-to-depth row and leaves NPS unmarked when the signature changed (#68); time to depth is the speed number for a `feat`, NPS for a `perf`.
- **2026-10-28** Quiescence search stood pat in check: a side in check at the horizon kept a static score it had no right to, and a checkmated side at the horizon got a centipawn evaluation instead of mate. From `7k/8/6K1/8/8/8/6Q1/8 w`, depth 1 said +1071 instead of mate in one; from `6k1/3q1ppp/8/8/8/8/8/R5K1 w` it said −705 for Ra8+, which is mate in two. The fix searches every legal evasion at an in-check quiescence node and returns mate when there is none; the only approximation left is a checked side with a move at the last ply, scored statically. Bench nodes +23% at depth 10 (#63). A horizon node is not exempt from the rules that apply at depth.
- **2026-10-28** A rating ladder only measures between its rungs: by v0.12.0 (~2480) the engine scored 98% on the 1500 rung, which gives no finite error bar and no information, and 65% on the top rung. The rungs moved to 2200/2500/2800/3100, the ladder history is recorded in `perf-targets.toml`, and a release is re-rated on the new rungs so the two scales overlap. Numbers from different ladders are not directly comparable: the levels disagree with each other by a hundred points or more, and which ones dominate the inverse-variance average changes with the rungs.
- **2026-10-27** GitHub-hosted runners vary 2x in NPS between runs of the same code (perft 200M–380M). Deltas under 10% are shown as unchanged; speed bars are enforced only on release PRs; SPRT uses fixed nodes.
- **2026-10-27** Release Please force-pushes and edits its PR in the same second; a shared concurrency group cancelled one of the two CI runs and the cancelled check run blocked the merge. Edits and pushes now have separate streams and code jobs skip on edits (#21).
- **2026-10-27** A stacked PR whose base was squash-merged shows a conflict if it touched the same lines again; rebase it onto `main` before merging. Merge stacks bottom-up.
- **2026-10-27** The PR templates list "Bench before" then "Bench after"; CI checks the last `Bench:` line in the body (#19).
- **2026-10-27** SPRT length tracks how close the gain is to the hypotheses, not how important the change is: killers (+97 Elo) resolved in 378 games, history (+55) in 554, the move picker (+18.5, under fixed nodes that hide its speed gain) took 1,976.
- **2026-10-27** Move ordering moved fail-high-first from 0.70 to 0.975 across four PRs, but EBF only from 7.00 to 4.18: at depth 6, past good ordering, EBF is set by what the search refuses to prune, which is Phase 4's job.
- **2026-10-27** GitHub runners are a mix of CPU models, so absolute NPS compared across runs mostly measures the chip (v0.5.0's baseline came from a fast one, and a PR looked 30% slower). Speed is now judged by paired A/B runs of PR and base on the same runner, and by cachegrind instructions per node, which is chip-independent and recorded for every release. Absolute NPS is informational, with the CPU model recorded beside it.
- **2026-10-27** The bench comment's EBF row showed the last bench position's EBF, not the bench's: Phase 2 read 3.35 but was 7.00, the transposition table read 2.12 but was 4.62. EBF is now the sum of every search's final-iteration nodes over the sum of its previous iteration's. The EBF ≤ 3 gate was never met; it still needs killers, history and, realistically, Phase 4's pruning to hold at depth 6.
- **2026-10-27** SPRT run time is set by the hypotheses, not by how good the change is: each game contributes at most the evidence that separates `elo0` from `elo1`, so a 200-Elo gain still needs ~200 games with `elo1 = 10`, and two identical engines take the longest (H0 with zero signal). Keep `elo1` modest to catch small real gains; use `sprt-quick` to test plumbing.
- **2026-10-27** Identical deterministic engines produce mirror-image game pairs: Elo exactly 0 with a zero error bar and an undefined LOS. Rendered as `n/a` rather than `NaN`.
- **2026-10-27** The v1 post-mortem's three recurring bugs (root window, aborted scores, bounded scores stored as exact) are rules in `AGENTS.md`; its two missing guards (eval symmetry, allocation counter) are tests (#20, #22).
- **2026-10-26** Release Please's `rust` strategy needs the workspace root to be a package; a virtual workspace fails with "is not a package manifest". The binary moved to the root (#6).
