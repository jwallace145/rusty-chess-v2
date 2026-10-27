# Chess Engine Roadmap

A living checklist. Oct 26, 2026 · @Jimmy, maintained as the engine is built.

**Status:** Phases 0–2 complete and released as `v0.3.0`. Phase 3 (move ordering and the transposition table) in progress.

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
| Search speed | Search NPS over 50 bench positions at depth 6 | `rusty-chess bench` | `search.min_nps` 2M local, 1M CI |
| Search efficiency | Fail-high-first, EBF, TT hit rate, qsearch share, cutoffs by stage | `bench` with `--features stats` | `efficiency.min_fail_high_first`, `max_ebf` (set at the Phase 3 gate) |
| Strength | Elo against the PR's base at a fixed node budget per move | fastchess SPRT on CI, `sprt` label | H1 accepted, or the PR does not merge |

- **Stats counters** (`--features stats`, every increment a macro that compiles to nothing otherwise): nodes and qsearch nodes per ply; TT probes, hits, cutoffs, collisions; fail-highs by move index (index 0 is fail-high-first); cutoffs by move-picker stage; null-move cutoffs, LMR reductions and re-searches, futility skips.
- **JSON lines** (`"v":1`): one `iter` record per iteration, one `summary` per search, one `bench` and one `perft` record per run. The schema doubles as a future API response shape.
- **Bench signature.** `bench` searches 50 fixed positions to a fixed depth with a fixed hash size and prints the total node count. A changed signature means search behaviour changed; the same signature with a different NPS is a pure speed change. The signature goes in the PR description as `Bench: <nodes>`; CI checks the last such line against its own run.
- **CI on every PR.** The `Bench` job runs bench (with and without stats) and the perft suite, uploads the JSON lines, runs `perf-gate` (speed bars reported on PRs and enforced on release PRs, efficiency bars always enforced, bars only ratchet up), and posts one sticky comment with deltas against the latest release and a collapsed explanation of every row.
- **Every release** carries its `bench.jsonl`, `stats.jsonl` and `perft.jsonl` as assets and the signature in its notes; the PR comment's baseline is read from there.
- **SPRT on CI.** A PR labelled `sprt` builds the PR and its base with identical flags on the same runner and plays fastchess games at a fixed node budget per move from our own generated openings. Fixed nodes make the result hardware-independent; NPS is judged by the bench gate in the same comment. Elo, LOS, LLR and the verdict are posted as a sticky comment; H0 fails the job. Time-control SPRT on a quiet dedicated box is a later option.

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

### Phase 3: Move ordering and the transposition table — in progress (the critical gate)

This is the gate that stopped attempt one. Each item is its own PR with the stats delta visible in the bench comment and an SPRT run.

- [x] SPRT on CI: `sprt.yml` on the `sprt` label, node-limited fastchess games against the PR base from our own openings, sticky comment, H0 fails; `sprt-quick` for a two-minute pipeline smoke test (#24)
- [x] Openings generator in `crates/tools` and a committed `tests/openings/openings.epd`: 300 positions, 8 random plies, within ±60 cp at depth 4, fixed seed (#24)
- [x] Transposition table: 10-byte entries in 3-entry clusters, key verification, depth-and-age replacement, mate scores adjusted by ply, TT move searched first, bound derived from the original window at every node, no store after an abort; `Hash` UCI option; `tt_hit` in the iteration records. SPRT +197.5 ± 43.4 Elo in 212 games; fail-high-first 0.70 → 0.79, EBF 7.00 → 4.62 (bench-wide), signature 68269801 → 14255682 (#26)
- [ ] Benchmark history: every release records its numbers in `docs/benchmarks.md`, its CHANGELOG entry, and its release notes with deltas against the previous release
- [ ] Staged move picker: TT move → good captures (MVV-LVA, filtered by SEE) → killers → history quiets → bad captures; every stage shows cutoffs on bench
- [ ] Two killer slots per ply
- [ ] Butterfly history with gravity
- [ ] **Exit:** fail-high-first ≥ 0.90 and EBF ≤ 3 on bench; the bars set in `perf-targets.toml`; each item passed SPRT on its own. Phase 4 does not start until both numbers hold

### Phase 4: Selectivity

- [ ] Principal variation search
- [ ] Aspiration windows
- [ ] Null-move pruning
- [ ] Late move reductions (log-based table)
- [ ] Reverse futility pruning
- [ ] Late move pruning
- [ ] Check extensions, mate-distance pruning
- [ ] Improved time management
- [ ] Bench depth raised once EBF allows it (the signature changes; say so in the PR)
- [ ] **Exit:** EBF ≈ 2, each technique gained Elo under SPRT, roughly 2400–2700 CCRL range

### Phase 5: First release

- [ ] Homebrew tap bumped by the `release` job after the Release Please PR merges (release notes already carry the signature and telemetry since Phase 0)
- [ ] Optional: a Lichess bot account; submission to CCRL testers
- [ ] **Exit:** `brew install` works on macOS arm64 and x86_64; the engine is publicly rated somewhere

### Phase 6: NNUE

- [ ] Data generation: self-play at fixed nodes with the Phase 4 engine, writing (position, score, result) records
- [ ] Train with an existing trainer (e.g. `bullet`) first; a from-scratch trainer optional later
- [ ] Inference: (768 → N)×2 → 1, incrementally updated accumulator, SIMD via `std::arch`, quantized int16 weights, net embedded with `include_bytes!`
- [ ] **Exit:** the NNUE build beats the PeSTO build under SPRT by a wide margin (100+ Elo is typical for a first net)

### Phase 7: Optional extensions

- [ ] Lazy SMP
- [ ] HTTP API (`api` crate on `search`, `POST /analyze`, worker pool of single-threaded searches, per-request time cap; the JSON lines schema is the response shape). Open question: hosting, and whether requests are public or authenticated
- [ ] Iterated nets trained on NNUE self-play data

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

- **2026-10-26** Release Please's `rust` strategy needs the workspace root to be a package; a virtual workspace fails with "is not a package manifest". The binary moved to the root (#6).
- **2026-10-27** GitHub-hosted runners vary 2x in NPS between runs of the same code (perft 200M–380M). Deltas under 10% are shown as unchanged; speed bars are enforced only on release PRs; SPRT uses fixed nodes.
- **2026-10-27** Release Please force-pushes and edits its PR in the same second; a shared concurrency group cancelled one of the two CI runs and the cancelled check run blocked the merge. Edits and pushes now have separate streams and code jobs skip on edits (#21).
- **2026-10-27** A stacked PR whose base was squash-merged shows a conflict if it touched the same lines again; rebase it onto `main` before merging. Merge stacks bottom-up.
- **2026-10-27** The PR templates list "Bench before" then "Bench after"; CI checks the last `Bench:` line in the body (#19).
- **2026-10-27** The bench comment's EBF row showed the last bench position's EBF, not the bench's: Phase 2 read 3.35 but was 7.00, the transposition table read 2.12 but was 4.62. EBF is now the sum of every search's final-iteration nodes over the sum of its previous iteration's. The EBF ≤ 3 gate was never met; it still needs killers, history and, realistically, Phase 4's pruning to hold at depth 6.
- **2026-10-27** SPRT run time is set by the hypotheses, not by how good the change is: each game contributes at most the evidence that separates `elo0` from `elo1`, so a 200-Elo gain still needs ~200 games with `elo1 = 10`, and two identical engines take the longest (H0 with zero signal). Keep `elo1` modest to catch small real gains; use `sprt-quick` to test plumbing.
- **2026-10-27** Identical deterministic engines produce mirror-image game pairs: Elo exactly 0 with a zero error bar and an undefined LOS. Rendered as `n/a` rather than `NaN`.
- **2026-10-27** The v1 post-mortem's three recurring bugs (root window, aborted scores, bounded scores stored as exact) are rules in `AGENTS.md`; its two missing guards (eval symmetry, allocation counter) are tests (#20, #22).
