# Chess Engine Roadmap

A living checklist. Oct 26, 2026 · @Jimmy, maintained as the engine is built.

**Status:** Phases 0–4 complete and released (`v0.17.0` closes Phase 4). Phase 4b (second-tier search) and Phase 5 (first release) run in parallel; Phase 6 (NNUE) starts when the 4b list is exhausted and the ladder reads about 2800.

## How to use this document

- Every item is a checkbox. Check it when the PR that delivers it merges, and write the PR number and the measured number next to it. Numbers are the point: "done" without a number is not done.
- Exit gates are checklist items too. A phase is complete when every gate item is checked. The next phase does not start before that.
- New learnings go under [Learnings](#learnings) with a date. New phases are appended; nothing is deleted, only checked or struck through with a reason.
- Everything is measured on standardized CI runners (`ubuntu-latest`), never on someone's laptop: benchmarks on every PR and release, SPRT games on labelled PRs. Local numbers are for development only.

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
6. **Standardized measurement.** Benchmarks and SPRT run on CI runners we configure, so every author's change is measured on the same hardware with the same flags.

## Measurement

| Layer | What it measures | How | Bar (`perf-targets.toml`) |
| --- | --- | --- | --- |
| Move generation | Perft NPS (bulk counting) over `tests/perft.epd` | `rusty-chess perft --suite` | `movegen.min_perft_nps` 100M local and CI |
| Search speed | Paired NPS ratio against the base on one runner; instructions per node (cachegrind, chip-independent); absolute NPS with the CPU model, informational | `speed-report paired`, `speed-report instructions`, `rusty-chess bench` | `search.min_nps` 2M local, 1M CI (absolute, release PRs) |
| Search efficiency | Fail-high-first, EBF, TT hit rate, qsearch share, cutoffs by stage | `bench` with `--features stats` | `efficiency.min_fail_high_first`, `max_ebf` (set at the Phase 3 gate) |
| Strength | Elo against the PR's base, timed and at fixed nodes | fastchess SPRT on CI, `sprt` label | H1 accepted in the timed test, or the PR does not merge |
| Absolute strength | Rating against Stockfish `UCI_Elo` levels at 10+0.1 | the `Rating` workflow, on every release | Tracked per release (a ballpark until a public rating in Phase 5) |

- **Stats counters** (`--features stats`, every increment a macro that compiles to nothing otherwise): nodes and qsearch nodes per ply; TT probes, hits, cutoffs, collisions; fail-highs by move index (index 0 is fail-high-first); cutoffs by move-picker stage; null-move cutoffs, LMR reductions and re-searches, futility skips.
- **JSON lines** (`"v":1`): one `iter` record per iteration, one `summary` per search, one `bench` and one `perft` record per run. The schema doubles as a future API response shape.
- **Bench signature.** `bench` searches 50 fixed positions to a fixed depth with a fixed hash size and prints the total node count. A changed signature means search behaviour changed; the same signature with a different NPS is a pure speed change. The signature goes in the PR description as `Bench: <nodes>`; CI checks the last such line against its own run.
- **CI on every PR.** The `Bench` job runs bench (with and without stats) and the perft suite, uploads the JSON lines, runs `perf-gate` (speed bars reported on PRs and enforced on release PRs, efficiency bars always enforced, bars only ratchet up), and posts one sticky comment with deltas against the latest release and a collapsed explanation of every row.
- **Every release** carries its `bench.jsonl`, `stats.jsonl` and `perft.jsonl` as assets and the signature in its notes; the PR comment's baseline is read from there.
- **SPRT on CI.** A PR labelled `sprt` builds the PR and its base with identical flags and plays two fastchess SPRTs at once from our own generated openings: timed at 5+0.05, which is the gate, and at a fixed node budget per move, which is informational. One sticky comment carries both and reads the pair; only an accepted H1 in the timed test passes. Until 2026-10-29 the fixed-node test alone was the gate, and it overstated changes that make nodes dearer (see the learnings).

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

### Phase 4: Selectivity — in progress

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

### Phase 4b: Second-tier search — in progress

The standard techniques the engine still lacks, each worth 10–40 Elo and all of them kept by an NNUE engine. One PR and one release each; an accepted H1 is required (#77). Ordered by expected value per effort.

- [ ] Continuation history (one and two plies back) and a counter-move heuristic in the move picker
- [x] Futility pruning of quiet moves at low depth: at null-window nodes up to depth 6, not in check, after the first move, a quiet move (checking moves included) is skipped when the static evaluation plus 100 + 100 cp per ply of depth left does not reach alpha (`futility_skips`). Depth-10 bench: signature 1991209 → 1717816 (−13.7%), fail-high-first 0.906, a million quiets skipped. The first form exempted checking moves (1890442, −5.1%): +25.6 ± 14.6 at fixed nodes (H1) but +5.9 ± 11.1 at 5+0.05, inconclusive at the 1000-pair cap, the check test per candidate eating the gain. This form: +25.6 ± 14.4 timed (H1, 1226 games), +45.0 ± 19.7 at fixed nodes (H1); time to depth −10.9%, EBF 1.77 → 1.64 (#81, v0.20.0)
- [x] SEE pruning in the main search: at nodes up to depth 8, not in check, after the first move, a quiet move whose static exchange loses more than 64 cp per ply of depth left is skipped (`see_skips`). Depth-10 bench on top of IIR: signature 1416355 → 1075807 (−24.0%), fail-high-first 0.899 → 0.890, time to depth about 18% lower. Pruning losing captures too (20 cp × depth²) cut to 1078392 but dropped fail-high-first to 0.873, below its 0.88 bar, so captures are left to the picker's bad-capture stage. +26.1 ± 14.3 timed (H1, 1122 games), +46.4 ± 20.0 at fixed nodes (H1); time to depth −19.9%, EBF 1.78 → 1.59 (#93, v0.22.0)
- [x] ~~The improving heuristic (static evaluation against two plies ago) driving the reverse futility and late move pruning margins~~ **Dropped on record; revisit after NNUE** (Phase 6). A node is improving when its static evaluation beats the one two plies earlier (four, when that side was in check). Two forms, on top of v0.22.0 (signature 1075807): reverse futility pruning with one ply's less margin when improving *and* late move pruning searching half as many quiets when not (815462, time to depth −25%) was −1.9 ± 10.9 timed and +8.5 ± 10.9 at fixed nodes, inconclusive at the old [0, 10] cap; reverse futility alone (942258, time to depth −13%) was H0 at fixed nodes, −29.6 ± 11.9, and −16.9 ± 10.9 timed when spot interrupted it at 1918 games. Both raised EBF (1.59 → 1.65 and 1.68); reverse futility cutoffs nearly doubled (81k → 157k), most of them wrong. A PeSTO evaluation swings too much from ply to ply for "better than two plies ago" to mean much; with NNUE it should (#95). Also untried: one more LMR ply when not improving (762220 nodes), for the reduction-tuning item
- [ ] Singular extensions: below the root, from depth 8, when the table holds a lower bound or exact score for its move searched at least depth − 3, the other moves are searched at half depth against a window 2 cp per ply below that score; if none reaches it the table move is searched a ply deeper, and if the node clears beta even without it, it fails high at once (multi-cut). Extensions stop at twice the iteration's depth (`se_extensions`, `se_multicuts`). Depth-10 bench: signature 1075807 → 1529684 (+42%, the extended lines and the verification searches), 377 extensions and 160 multi-cuts. +16.6 ± 8.1 timed (H1, 3260 games, the first gate at [0, 5]), +27.1 ± 10.6 at fixed nodes (H1); time to depth +44.9%, EBF 1.59 → 1.81, the deeper lines' cost (#99)
- [x] Internal iterative reductions at nodes without a table move: below the root, from depth 4, a node with no table move is searched one ply shallower (`iir_reductions`). Depth-10 bench on top of futility pruning: signature 1717816 → 1416355 (−17.5%), fail-high-first 0.906 → 0.899, time to depth about 18% lower. An early read against v0.19.0 (without futility pruning) on the dedicated instances: +28.2 ± 14.8 timed, H1 (#90). Against v0.20.0, on the dedicated instances: +16.7 ± 11.0 timed (H1, 1936 games), +22.8 ± 13.6 at fixed nodes (H1); time to depth −18.2% (#89, v0.21.0)
- [ ] History-adjusted late move reductions and reduction tuning: a quiet move's reduction moves one ply per 512 points of its combined history (butterfly plus both continuation tables), less for good history and more for bad, clamped to 0..depth left. Depth-10 bench on top of singular extensions: signature 1529684 → 1697503 (+11%), fail-high-first 0.885 → 0.882. Divisors 8,192 to 2,048 barely changed the tree (our bonus is depth² capped at 1,200, so scores stay small). The first form, 1,024 (1546957), was H1 at fixed nodes (+7.5 ± 5.1) but +2.8 ± 5.6 at a clock when spot interrupted it at 6,430 games; SPRT of this form pending (#101)
- [ ] Pondering (`go ponder`, `ponderhit`), for Lichess, where the opponent's clock is free time
- [ ] **Exit:** the list exhausted (each item kept on H1 or dropped on record); the ladder reads about 2800; then Phase 6

### Phase 5: First release

- [ ] A rating ladder against limited Stockfish on every release, attached with the logs and PGNs, as a ballpark between public ratings
- [ ] Homebrew tap bumped by the `release` job after the Release Please PR merges (release notes already carry the signature and telemetry since Phase 0)
- [x] Lichess bot account: `lichess/run.sh` connects the engine through lichess-bot for development (see `lichess/README.md`), and an always-on `c8a.medium` run by jwall-engineering-infra plays every published release automatically (#74). The rating it earns is the real-world number; it is not comparable to the ladder's or CCRL's
- [ ] Opening book as a release asset: a from-scratch Polyglot book builder in `crates/tools` from a master-games collection (data credited like PeSTO), published as `rusty-chess-book.bin` beside the binaries; the Lichess bot plays it through lichess-bot's polyglot setting with weighted random selection. The engine itself stays book-free: SPRT, the ladder and CCRL testers all play from their own openings. **Decided 2026-10-28:** no `OwnBook` in the binary; a book built from the engine's own games replaces the master-games book after Phase 6
- [ ] Release matrix for CCRL testers: a Windows x86-64 build and a baseline x86-64 build beside the x86-64-v3 one, named by CPU level
- [ ] Optional: submission to CCRL testers
- [ ] **Exit:** `brew install` works on macOS arm64 and x86_64; the engine is publicly rated somewhere

### Phase 6: NNUE

- [ ] Data generation: self-play at fixed nodes with the engine's own search, writing (position, score, result) records. `datagen` and the `Datagen` workflow (spec: [datagen.md](datagen.md), settings: `datagen.toml`) play on dedicated spot instances and upload shards to the dataset bucket as they complete; generation 0 is played with PeSTO. **Decided 2026-10-30:** the nets are trained on self-play only, never on another engine's evaluations: the engine stays wholly original (for CCRL, tournaments' originality rules and the project itself), the labels come from search plus game results rather than from PeSTO alone, so a first net still learns tactics and king safety PeSTO misses, and each generation is re-generated by the engine carrying the previous net. Distilling Stockfish's scores would reach a given strength sooner and was declined for originality
- [ ] Train with an existing trainer (e.g. `bullet`) first; a from-scratch trainer optional later. Pilot: about 10M positions and a 768 → 128 × 2 → 1 net to prove the pipeline before scaling to 200–500M
- [ ] Inference: (768 → N)×2 → 1, incrementally updated accumulator, SIMD via `std::arch`, quantized int16 weights, net embedded with `include_bytes!`
- [ ] **Exit:** the NNUE build beats the PeSTO build under SPRT by a wide margin (100+ Elo is typical for a first net)
- [ ] Iterate: generation n + 1 is played by the engine with net n (raise `generation` in `datagen.toml`), retrained, and kept on an SPRT win over net n
- [ ] Revisit what Phase 4b dropped for want of a good evaluation: the improving heuristic (#95), with the forms and numbers recorded there; and re-tune the margins tied to the evaluation's scale (reverse futility, futility, SEE, null move, aspiration windows)

### Phase 7: Optional extensions

- [ ] Lazy SMP
- [ ] HTTP API (`api` crate on `search`, `POST /analyze`, worker pool of single-threaded searches, per-request time cap; the JSON lines schema is the response shape). Open question: hosting, and whether requests are public or authenticated
- [ ] Larger nets and longer iteration once Phase 6's loop is proven (Phase 6 itself iterates the first nets)

## Repo layout and conventions

```
rusty-chess-v2/
  Cargo.toml   # root package `rusty-chess` (the binary) plus the [workspace]
  src/         # UCI protocol + binary entry point (bench, perft subcommands)
  crates/
    core/      # board, bitboards, magics, movegen, zobrist, make/unmake (no deps)
    search/    # search, TT, move picker, eval, SearchStats, JSON lines telemetry
    tools/     # perft suite runner, perf-gate, bench-report, openings, sprt-report
  tests/       # perft EPD suite, UCI smoke tests, self-play, eval symmetry, openings
  .github/workflows/  # ci.yml, bench.yml, sprt.yml, release-please.yml
```

The workspace root is itself a package because Release Please's `rust` strategy runs its package updater on the root `Cargo.toml` after bumping every member, and fails on a virtual workspace. Members are listed explicitly with their own `version`; `release-type: simple` with an annotated version would leave `Cargo.lock` stale and break `--locked` builds.

| Tool | Purpose |
| --- | --- |
| Criterion | Microbenchmarks for hot functions (when needed) |
| samply / perf | Flamegraphs on `bench` |
| cargo-show-asm | Inspect codegen of hot loops |
| fastchess | SPRT strength testing, on CI |
| cargo-llvm-cov | Coverage, mainly for core |
| Release Please | Versioning and changelog |

The repo follows [repo-conventions.md](repo-conventions.md) in full; [CONTRIBUTING.md](../CONTRIBUTING.md) restates it. Engine-specific: the bench signature goes in the PR description, not the commit; `perf` for speed changes with an unchanged signature, `feat` for search or eval changes that change it, `fix` for correctness; a `feat` whose SPRT failed does not merge; `AGENTS.md` adds the rules agents must follow.

## Distribution and future API

Ship it as a Homebrew-installable UCI binary first. Release CI builds macOS arm64 and x86_64 and Linux x86_64 binaries for a portable CPU baseline (x86-64-v3 / apple-m1) and attaches them to the GitHub release with checksums; Phase 5 adds the tap formula bump. An NNUE net, when it exists, is embedded so the install remains a single file. The API, if pursued, is a thin wrapper added only after the engine is strong.

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
| Noisy perf numbers | Chasing ±3% NPS changes | Standardized runners; noise thresholds in the comment; node signature and fixed-node SPRT over NPS |

## Learnings

- **2026-10-30** A heuristic that reads the evaluation's trend is only as good as the evaluation. The improving heuristic, standard in strong engines, lost at fixed nodes in its mildest form (−29.6 ± 11.9): with PeSTO the static evaluation swings enough between plies that "better than two plies ago" mostly picks noise, and the extra reverse futility cutoffs it allowed were mostly wrong. Dropped until NNUE (#95). Also on 2026-10-30, the SPRT moved to [0, 5] with a 5000-pair cap (#96) after two PRs ended inconclusive at [0, 10]'s 2000 games, and the first live spot interruption was reported as designed: exit 75, no verdict, the partial log kept for reading.
- **2026-10-29** A fixed-node SPRT overstates any change that makes nodes dearer: it gives the change the same number of nodes, so the extra work per node is free. The accepted fixed-node SPRTs from v0.13.0 to v0.18.0 summed to about +190 Elo; a 400-game match of the two releases at 10+0.1 measured +26.1 ± 23.4 (#83). Per release at 10+0.1: the quiescence fix with SEE pruning +22.6 ± 24.3 (fixed-node SPRT +75.6), the quiescence transposition table −11.3 ± 23.4 (+69.4), continuation history −7.0 ± 21.7 (+18.1). The SPRT now runs timed and at fixed nodes together, and only the timed result gates; the two neutral changes are `perf` targets before any revert.
- **2026-10-28** Time management cannot be tested where its waste is: at 5+0.05 the base's overruns past its soft budget finished as deeper moves, and three timed SPRTs of three different plans all lost or tied. A guard against overruns is a long-control change and needs a long-control test; until the standardized runner exists, leave the plan alone.
- **2026-10-28** Nodes per second and fixed-node SPRT both mis-read a change that alters what a node is. Quiescence SEE pruning read −35% NPS and 🔴 while reaching the same depth 16% sooner, and its fixed-node Elo flattered it. The bench comment now carries a paired time-to-depth row and leaves NPS unmarked when the signature changed (#68); time to depth is the speed number for a `feat`, NPS for a `perf`.
- **2026-10-26** Release Please's `rust` strategy needs the workspace root to be a package; a virtual workspace fails with "is not a package manifest". The binary moved to the root (#6).
- **2026-10-27** GitHub-hosted runners vary 2x in NPS between runs of the same code (perft 200M–380M). Deltas under 10% are shown as unchanged; speed bars are enforced only on release PRs; SPRT uses fixed nodes.
- **2026-10-27** Release Please force-pushes and edits its PR in the same second; a shared concurrency group cancelled one of the two CI runs and the cancelled check run blocked the merge. Edits and pushes now have separate streams and code jobs skip on edits (#21).
- **2026-10-27** A stacked PR whose base was squash-merged shows a conflict if it touched the same lines again; rebase it onto `main` before merging. Merge stacks bottom-up.
- **2026-10-27** The PR templates list "Bench before" then "Bench after"; CI checks the last `Bench:` line in the body (#19).
- **2026-10-27** SPRT length tracks how close the gain is to the hypotheses, not how important the change is: killers (+97 Elo) resolved in 378 games, history (+55) in 554, the move picker (+18.5, under fixed nodes that hide its speed gain) took 1,976.
- **2026-10-27** Move ordering moved fail-high-first from 0.70 to 0.975 across four PRs, but EBF only from 7.00 to 4.18: at depth 6, past good ordering, EBF is set by what the search refuses to prune, which is Phase 4's job.
- **2026-10-27** GitHub runners are a mix of CPU models, so absolute NPS compared across runs mostly measures the chip (v0.5.0's baseline came from a fast one, and a PR looked 30% slower). Speed is now judged by paired A/B runs of PR and base on the same runner, and by cachegrind instructions per node, which is chip-independent and recorded for every release. Absolute NPS is informational, with the CPU model recorded beside it.
- **2026-10-27** The bench comment's EBF row showed the last bench position's EBF, not the bench's: Phase 2 read 3.35 but was 7.00, the transposition table read 2.12 but was 4.62. EBF is now the sum of every search's final-iteration nodes over the sum of its previous iteration's. The EBF ≤ 3 gate was never met; it still needs killers, history and, realistically, Phase 4's pruning to hold at depth 6.
- **2026-10-27** SPRT run time is set by the hypotheses, not by how good the change is: each game contributes at most the evidence that separates `elo0` from `elo1`, so a 200-Elo gain still needs ~200 games with `elo1 = 10`, and two identical engines take the longest (H0 with zero signal). Keep `elo1` modest to catch small real gains; use `sprt-quick` to test plumbing.
- **2026-10-28** Quiescence search stood pat in check: a side in check at the horizon kept a static score it had no right to, and a checkmated side at the horizon got a centipawn evaluation instead of mate. From `7k/8/6K1/8/8/8/6Q1/8 w`, depth 1 said +1071 instead of mate in one; from `6k1/3q1ppp/8/8/8/8/8/R5K1 w` it said −705 for Ra8+, which is mate in two. The fix searches every legal evasion at an in-check quiescence node and returns mate when there is none; the only approximation left is a checked side with a move at the last ply, scored statically. Bench nodes +23% at depth 10 (#63). A horizon node is not exempt from the rules that apply at depth.
- **2026-10-28** A rating ladder only measures between its rungs: by v0.12.0 (~2480) the engine scored 98% on the 1500 rung, which gives no finite error bar and no information, and 65% on the top rung. The rungs moved to 2200/2500/2800/3100, the ladder history is recorded in `perf-targets.toml`, and a release is re-rated on the new rungs so the two scales overlap. Numbers from different ladders are not directly comparable: the levels disagree with each other by a hundred points or more, and which ones dominate the inverse-variance average changes with the rungs.
- **2026-10-27** Identical deterministic engines produce mirror-image game pairs: Elo exactly 0 with a zero error bar and an undefined LOS. Rendered as `n/a` rather than `NaN`.
- **2026-10-27** The v1 post-mortem's three recurring bugs (root window, aborted scores, bounded scores stored as exact) are rules in `AGENTS.md`; its two missing guards (eval symmetry, allocation counter) are tests (#20, #22).
