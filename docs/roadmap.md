# Chess Engine Roadmap

Oct 26, 2026 · @Jimmy

## Goals and principles

The goal is a single-threaded Rust UCI engine whose classical search is measurably strong before any NNUE work starts. Every change has to be justified by a number.

**Why attempt one stalled.** Speed was only part of it: 300k NPS is slow, but survivable. The bigger issue was a beta-cutoff rate around 10–14%, which means move ordering was close to random. Alpha-beta then degrades toward minimax, and the effective branching factor goes from about 2 to over 10. At that point no NPS gain helps. Depth comes from search efficiency first and raw speed second.

**Decisions made**

- Fresh repo, Rust, single-threaded first (the transposition table is designed so Lazy SMP can be added later)
- Classical search must hit its efficiency targets before evaluation gets fancy; NNUE is the destination
- Telemetry is JSON lines plus CI artifacts; no dashboard for now
- Distribution via a Homebrew tap; an HTTP API is a possible later phase

**Principles**

1. **Correct before fast.** Perft passes on the full suite before any search code is written, and it stays in CI permanently.
2. **Measure three layers separately:** move generation speed, search speed, and search efficiency. Each one fails differently.
3. **Elo is the final judge.** Every search or evaluation change gets an SPRT run. A feature that doesn't gain Elo gets reverted, even if it looks right.
4. **One feature per commit.** The bench signature makes behavior changes visible, so regressions can be bisected.
5. **No heap allocation in the search path.**

## Telemetry and benchmarking

Telemetry is built into the engine, compiled out of release builds, and its output is machine-readable so commits can be diffed by script or by AI.

| Layer | What it measures | How | Target |
| --- | --- | --- | --- |
| Move generation | Perft NPS (bulk counting) | `engine perft` + Criterion | 100M+ NPS |
| Search speed | Search NPS | `engine bench` | 2M+ NPS with PeSTO eval |
| Search efficiency | Effective branching factor, fail-high on first move, TT hit rate, qsearch node share | `stats` feature counters | Fail-high-first ≥ 90%, EBF ≈ 2 |
| Strength | Elo against the previous version | fastchess SPRT | Positive per merged change |

**Stats counters.** A `SearchStats` struct sits behind a Cargo feature (`--features stats`), and every increment is a macro that compiles to nothing when the feature is off. It counts:

- Nodes and qsearch nodes, per depth
- TT probes, hits, cutoffs, and collisions
- Fail-highs by move index (index 0 is the fail-high-first rate)
- Cutoffs by move-picker stage: TT move, good captures, killers, history quiets, bad captures
- Pruning events: null-move cutoffs, LMR reductions and re-searches, futility skips

**JSON lines.** One record per iteration of iterative deepening, and one summary per search. Example:

```json
{"type":"iter","fen":"...","depth":9,"nodes":412331,"qnodes":198220,"nps":2310000,"ebf":2.1,"fh_first":0.91,"tt_hit":0.38,"score_cp":34,"pv":"e2e4 e7e5 ..."}
```

The schema is versioned (`"v":1`) so old artifacts stay comparable.

**Deterministic bench.** `engine bench` searches about 50 fixed positions to a fixed depth with a fixed hash size. It prints a total node count (the *signature*) and the NPS. A changed signature means search behavior changed; a changed NPS with the same signature means a pure speed change. The signature goes in every commit message, as Stockfish does.

**Coverage suites and performance bars.** Each layer in the table has a coverage suite that only grows and a bar that only rises, both committed. Move generation is covered by `tests/perft.epd` (lines are appended, never removed or changed) and search by the bench positions. The bars live in `perf-targets.toml`, per environment for speed because shared runners are slow and noisy. `perf-gate` in `crates/tools` compares a run's JSON lines to them: speed bars are reported on every PR and enforced on release PRs and `main`, efficiency bars are always enforced, and a PR that lowers a bar fails. A bar is set when its layer first ships and raised whenever the engine clears it, so every release is at least as fast and as efficient as the last.

**CI artifacts.** On every push, CI runs perft, bench, and bench with stats, then uploads `bench.jsonl` and `stats.jsonl` as artifacts. Every release also carries its own `bench.jsonl`, `stats.jsonl` and `perft.jsonl` as GitHub release assets, written by the `telemetry` job from the shipped Linux build, with the bench signature in the release notes. `bench-report` diffs a PR's run against the latest release's assets and posts one sticky PR comment with the deltas, refreshed on every push; the `[comment]` section of `perf-targets.toml` sets the noise thresholds below which a delta shows as unchanged. NPS on shared runners is noisy, so it is reported but not gated; the node signature and perft are gated.

**Profiling.** Run `samply` or `perf` on `bench` for flamegraphs, and `cargo-show-asm` to inspect hot functions. The release profile uses `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, and `target-cpu=native` locally.

**Strength testing.** fastchess runs SPRT matches of each branch against `main` at a short time control (e.g. 8+0.08s) with an opening book. This runs locally, since CI runners are too slow and noisy for it. A later option is a small always-on test box.

## Repo layout and tooling

Use a Cargo workspace so the core library can later be reused by the CLI, an API server, and NNUE training tools without dependency bloat.

```
rusty-chess-v2/
  Cargo.toml   # root package `rusty-chess` (the binary) plus the [workspace]
  src/         # UCI protocol + binary entry point (bench, perft subcommands)
  crates/
    core/      # board, bitboards, magics, movegen, zobrist, make/unmake (no deps)
    search/    # search, TT, move picker, eval, SearchStats, JSON lines telemetry
    tools/     # perft suite runner, stats diff script, datagen later
  benches/     # Criterion: movegen, make/unmake, eval, TT probe
  tests/       # perft EPD suite, UCI smoke tests, mate-in-N positions
  .github/workflows/  # ci.yml, bench.yml (artifacts), release-please.yml (release job)
```

The workspace root is itself a package, the `rusty-chess` binary, rather than a virtual workspace. Release Please's `rust` strategy runs its package updater on the root `Cargo.toml` after bumping every member, and fails on a root without `[package]`; the root `benches/` and `tests/` directories need a root package anyway. Members are listed explicitly (no globs) and each carries its own `version`, which the strategy bumps together with the root and `Cargo.lock`. Path dependencies between crates carry no `version` field.

| Tool | Purpose |
| --- | --- |
| Criterion | Microbenchmarks for hot functions |
| samply / perf | Flamegraphs on `bench` |
| cargo-show-asm | Inspect codegen of hot loops |
| fastchess | SPRT strength testing |
| cargo-llvm-cov | Coverage, mainly for core |
| Release Please | Versioning and changelog, per repo conventions |

Use `core` with no external dependencies; `arrayvec` is the only likely exception, and a hand-rolled fixed array works too.

## Repo conventions

The repo follows the standard conventions document in full; nothing in this plan overrides it. What matters most for an engine repo is how releases and bench signatures interact.

**Carried over as-is**

- One-line Conventional Commits (≤ 100 chars), with only `Co-authored-by` trailers allowed; enforced by commitlint locally via `pre-commit` and authoritatively in CI
- Everything after `chore: initial scaffold` lands through a PR with a conventional title; squash or rebase only, linear history, no bypass actors
- Typed PR templates in `.github/PULL_REQUEST_TEMPLATE/`, plus `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, MIT `LICENSE`, Dependabot for `github-actions`
- Release Please with the manifest config, authored by `jwallace145-release-bot[bot]`, starting at `0.1.0`; it owns `CHANGELOG.md`
- Actions pinned to major versions, least-privilege `permissions:` on every job

**Engine-specific additions**

- **Bench signature in the PR description, not the commit.** Commits are one line, so the `Bench: <nodes>` line and the SPRT result go in the PR template's "How it was verified" section. CI posts the bench and stats diff as a PR comment, so the number is visible without reading artifacts.
- **Commit types for engine work:** `perf` for speed changes with an unchanged bench signature, `feat` for search or eval changes that change the signature, `fix` for correctness (perft, TT, mate scores). A `feat` whose SPRT failed does not merge.
- **PR template additions:** `feat.md` gains a "Bench signature before / after" line and an "SPRT result" line (Elo, LOS, games); `perf.md` is added with the same bench line plus NPS before/after on a quiet machine.
- **CI jobs beyond the baseline:** `test` (perft suite, unit tests), `bench` (uploads `bench.jsonl` and `stats.jsonl`, gates on node signature and perft only), and `release` (builds macOS arm64/x86\_64 and Linux x86\_64 binaries, runs `needs: release-please` and `if: release_created`, attaches binaries to the GitHub release, then bumps the Homebrew tap formula).
- **Release type:** `release-type: rust` so Release Please bumps the root package, every workspace member's `Cargo.toml` and `Cargo.lock` together. This requires the workspace root to be a package (see the layout above); `release-type: simple` with an annotated version line would leave `Cargo.lock` stale and break `--locked` builds on the release PR. The merged Release Please PR is the human gate for every published build.
- **`AGENTS.md`** restates the four agent rules and adds two: never change search or eval without a bench run, and never claim an Elo gain without an SPRT result in the PR.

## Phased roadmap

There are seven phases, and each one ends at a hard gate. Phase 3 is the gate that stopped attempt one, so it gets the most attention. Elo figures are rough ballparks, not commitments.

&#91;embedded content: roadmap · 8 phases, 7 exit gates\]

Each phase starts only after the previous gate holds on bench and stats; Phase 3's search-efficiency gate is the one to protect.

### Phase 0: Scaffolding and telemetry skeleton

- `chore: initial scaffold` pushed directly to `main` (the only direct push): workspace, release profile, `README.md`, `LICENSE`
- Conventions PR: commitlint, `pre-commit` hook, PR templates (including `perf.md`), `AGENTS.md`, `CONTRIBUTING.md`, Dependabot, and `ci.yml` with commitlint, pr-title, fmt, clippy, and test jobs
- Release Please PR: manifest config with `release-type: rust` over a root package, the `release-please.yml` workflow, and the `release` job that builds macOS arm64, macOS x86_64 and Linux x86_64 binaries and attaches them to the GitHub release
- Apply merge settings and the `main` ruleset with `gh api`; verify a multi-line commit fails CI and a bad PR title fails
- `stats` feature with no-op macros, the JSON lines writer, and `bench` / `perft` subcommands, even before they do much
- **Exit:** CI uploads a (trivial) `bench.jsonl` artifact on every PR, and the first Release Please PR has cut `v0.1.0`.

### Phase 1: Board and move generation

- Bitboards, magic bitboards (with PEXT behind a feature for later), precomputed attack and ray tables
- Compact 16-bit moves, a stack-allocated move list, legal movegen using pin and check masks
- Make/unmake with an undo stack, and incremental Zobrist hashing (verified against a full recompute in debug builds)
- **Exit:** the full perft suite passes (startpos, Kiwipete, and the standard EPD set to depth 6+), and bulk-counting perft reaches ≥ 100M NPS single-threaded. Held as of the first move generator: 63 positions, 283 checks, about 280M NPS on an M-series laptop; `movegen.min_perft_nps` in `perf-targets.toml` is set to 100M locally and on CI (the runner measured about 380M).

### Phase 2: Baseline search

- Negamax alpha-beta, iterative deepening, quiescence search (captures only)
- Tapered PeSTO piece-square tables, updated incrementally in make/unmake
- UCI, simple time management, repetition and 50-move detection
- **Exit:** legal games under fastchess with no crashes, a stable bench signature, and stats JSON on every search. This is the Elo baseline that every later SPRT run measures against.

### Phase 3: Move ordering and transposition table (the critical gate)

- TT with 2–4 entry buckets, a depth-preferred replacement policy, and the TT move searched first
- Staged move picker: TT move → good captures (MVV-LVA, filtered by SEE) → killers → history-ordered quiets → bad captures
- Butterfly history with gravity, two killer slots per ply
- **Exit:** fail-high-first ≥ 90% and EBF ≤ 3 on bench, measured with the stats feature. Each item passes SPRT on its own. Phase 4 does not start until both numbers hold.

### Phase 4: Selectivity

- Principal variation search, aspiration windows
- Null-move pruning, late move reductions (log-based table), reverse futility pruning, late move pruning
- Check extensions, mate-distance pruning, and improved time management
- **Exit:** EBF ≈ 2, each technique gains Elo under SPRT, and the engine is roughly in the 2400–2700 CCRL range.

### Phase 5: First release

- Homebrew tap bumped by the `release` job after the Release Please PR merges (release notes already carry the bench signature and telemetry assets since Phase 0)
- Optional: a Lichess bot account for real-world games, and submission to CCRL testers
- **Exit:** `brew install` works on macOS (arm64 and x86\_64) and the engine is publicly rated somewhere.

### Phase 6: NNUE

- Data generation: self-play at fixed nodes with the Phase 4 engine, writing (position, score, result) records
- Train with an existing trainer (e.g. `bullet`) first; a from-scratch trainer is optional later
- Inference: a (768 → N)×2 → 1 architecture, an incrementally updated accumulator, and SIMD via `std::arch`, with quantized int16 weights
- **Exit:** the NNUE build beats the PeSTO build under SPRT by a wide margin (100+ Elo is typical for a first net).

### Phase 7: Optional extensions

- Lazy SMP, an HTTP API (see below), and iterated nets trained on NNUE self-play data

## Distribution and future API

Ship it as a Homebrew-installable UCI binary first. The API is a thin wrapper added only after the engine is strong.

**Homebrew.** Release CI builds macOS arm64 and x86\_64 (plus Linux x86\_64) binaries and attaches them to a GitHub release. A formula update is then pushed to a tap repo. Build for a portable CPU baseline (x86-64-v3 / apple-m1) instead of `native`, so installed binaries run everywhere. An NNUE net, when it exists, is embedded with `include_bytes!` so the install remains a single file.

**API (if pursued).** A separate `api` crate depends on `search` directly, without talking UCI over a subprocess.

- `POST /analyze` with `{fen, movetime_ms | depth}` returns best move, score, PV, and the stats summary
- A worker pool of single-threaded searches, each with its own TT; enforce a per-request time cap and rate limits
- The JSON lines schema doubles as the API response shape, so telemetry and API share one format

**Open question:** where it would be hosted (a small VPS versus a container service) and whether requests are public or authenticated.

## Risks and anti-patterns

Most engines that stall do so for a handful of known reasons. Each one below has a specific guard.

| Risk | Symptom | Guard |
| --- | --- | --- |
| Poor move ordering | Low fail-high-first, EBF > 4 | Phase 3 gate; per-stage cutoff counters |
| Allocation in search | NPS flat as depth grows, `malloc` in the flamegraph | Stack move lists; a counting allocator in debug builds that panics if search allocates |
| Expensive evaluation | Eval dominates the profile | Incremental PSTs; profile eval share on every bench |
| Bugs disguised as features | A pruning change "works" but loses Elo | SPRT for every change; revert on fail |
| TT bugs | Random blunders, unstable bench signature | Store the full key or verification bits; adjust mate scores by ply |
| Eval bugs (e.g. double counting) | Odd positional play | Symmetry test: eval(pos) == eval(mirrored pos) for every bench FEN |
| Over-tuning early | Hours on eval terms before search is sound | Eval frozen at PeSTO until the Phase 4 exit |
| Noisy perf numbers | Chasing ±3% NPS changes | Compare on a quiet local machine; trust node signature over CI NPS |
