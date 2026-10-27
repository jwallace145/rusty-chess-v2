# Contributing

This repo follows the conventions in [docs/repo-conventions.md](docs/repo-conventions.md). The short version: one-line Conventional Commits, every change through a pull request, squash or rebase merges only, releases cut by Release Please, and every search or evaluation change justified by a number.

## Workflow

1. Branch from `main`.
2. Make the change. Keep the code formatted (`cargo fmt`) and run the local checks below.
3. Open a PR with a typed template (see below). The PR title must be a valid Conventional Commit.
4. For `feat` and `perf` PRs, fill in the bench signature. CI runs `bench` and uploads `bench.jsonl` and `stats.jsonl` as artifacts; if the PR description carries a `Bench: <nodes>` line, CI checks that the last one (the "after" value) matches its own run.
5. Merge with **squash** or **rebase**. Merge commits are disabled and history stays linear.

Direct pushes to `main` are not allowed. The only exception was the initial scaffold commit during bootstrap.

## Commit messages: one line, nothing else

Every commit message is a single line:

```
type(scope)?: subject
```

- At most 100 characters.
- `type` is one of `feat`, `fix`, `docs`, `chore`, `refactor`, `ci`, `build`, `perf`, `style`, `test`, `revert`.
- Breaking changes are marked with `!`, for example `feat!: drop the legacy bench format`.
- The only content allowed after the header is a blank line followed by `Co-authored-by: Name <email>` trailers.
- No body text. Details belong in the PR description.

commitlint enforces this with `@commitlint/config-conventional` plus a local rule, `only-coauthor-trailers` (`tools/commitlint/only-coauthor-trailers.mjs`), which fails on any line after the header that is not blank or a `Co-authored-by` trailer. CI runs it over every commit in the PR range, so a rebase merge never lands a non-conforming commit on `main`.

### Commit types for engine work

| Type | Use for | Bench signature |
| --- | --- | --- |
| `perf` | Speed changes: same search, faster | Unchanged |
| `feat` | Search or eval changes | Changes; needs a passing SPRT |
| `fix` | Correctness: perft, TT, mate scores | May change |

### Why the PR title matters

Squash merges use the PR title as the commit message on `main`, and Release Please reads those messages to decide the next version and to write `CHANGELOG.md`. A `feat` bumps the minor version, a `fix` bumps the patch, and `!` bumps the major (the minor while pre-1.0). CI lints the title with the same rules.

### Dependabot PRs

Dependabot cannot write one-line, lowercase commit messages, so its PRs skip the commit message check and the title check. Merge them with **squash** and a lowercase Conventional Commit subject, for example:

```sh
gh pr merge <n> --squash --subject "ci: bump actions/checkout from 6 to 7"
```

Release Please only cuts a release when a commit of a type that appears in `CHANGELOG.md` lands on `main`: `feat`, `fix`, `perf`, `revert` or `docs`. `chore`, `ci`, `build`, `refactor`, `style` and `test` are hidden from the changelog and never release on their own.

## Pull request templates

Templates live in `.github/PULL_REQUEST_TEMPLATE/`, one per type: `feat.md`, `perf.md`, `fix.md`, `docs.md`, `chore.md`, `refactor.md`, `ci.md`. Pick one with

```sh
gh pr create --template fix.md
```

or by appending `?template=fix.md` to the compare URL. Every template has Summary, Motivation and context, What changed, How it was verified, and a Checklist. `fix.md` adds Bug, Root cause, and Reproduction. `feat.md` adds the bench signature before and after plus the SPRT result; `perf.md` adds the bench signature plus NPS before and after on a quiet machine. Fill in every section.

## Local setup

Requirements: a stable Rust toolchain (`>= 1.85`, edition 2024), Node.js `>= 22`, and [pre-commit](https://pre-commit.com). For strength testing, [fastchess](https://github.com/Disservin/fastchess); for profiling, [samply](https://github.com/mstange/samply) and [cargo-show-asm](https://github.com/pacak/cargo-show-asm).

```sh
npm ci
pre-commit install --hook-type commit-msg --hook-type pre-commit
cargo build --release
```

The `commit-msg` hook runs commitlint on every commit; the `pre-commit` hook runs `cargo fmt`. CI is the authoritative check; the hooks exist so mistakes are caught before they are pushed.

### Checks that must pass

| Check | Runs |
| --- | --- |
| `Commit messages` | commitlint over `base..head` and the rule's unit tests (`npm test`) |
| `PR title` | Conventional Commit check on the title |
| `Format` | `cargo fmt --all --check` |
| `Clippy` | `cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| `Tests` | `cargo test --workspace --locked`, with and without `--features stats` |
| `Bench` | `rusty-chess bench` in release mode; uploads `bench.jsonl` and `stats.jsonl`; checks a `Bench:` line in the PR body if present; runs `perf-gate` against `perf-targets.toml` |

NPS on shared runners is noisy, so CI reports it but gates only on the node signature. Compare NPS on a quiet local machine.

## Coverage suites and performance bars

Each measured layer has a coverage suite that only grows and a performance bar that only rises:

| Layer | Coverage | Measured by | Bar |
| --- | --- | --- | --- |
| Move generation | `tests/perft.epd` (append positions; never remove or change a line) | `rusty-chess perft --suite`, `perft.jsonl` | `movegen.min_perft_nps` |
| Search speed | the bench positions in `src/positions.rs` | `rusty-chess bench`, `bench.jsonl` | `search.min_nps` |
| Search efficiency | the same bench, built with `--features stats` | `stats.jsonl` | `efficiency.min_fail_high_first`, `efficiency.max_ebf` |

The bars are committed in `perf-targets.toml`, per environment for speed (`local` for a quiet machine, `ci` for shared runners). `perf-gate` (in `crates/tools`) compares a run's JSON lines to them:

```sh
cargo run --release -p rusty-chess-tools --bin perf-gate -- \
  --targets perf-targets.toml --env local --enforce-noisy bench.jsonl stats.jsonl perft.jsonl
```

In CI the `Bench` job runs the gate with `--env ci`. Speed bars are reported on every PR and enforced on release PRs and on `main`; efficiency bars are deterministic and always enforced; a PR that lowers any bar fails the ratchet check against `main`. When the engine clears a bar comfortably, raise it in the same PR. A bar of `0` is not enforced yet; set it when the layer first ships.

### The bench comment

Every PR gets one bench comment, posted by the `Bench` job and refreshed in place on every push. It compares the PR's run with the latest release's telemetry assets (`bench.jsonl`, `stats.jsonl`, `perft.jsonl` attached to the GitHub release by the `telemetry` job; for a release that predates them, the tagged commit's Bench run; failing both, the latest run on the base branch, labelled `main@<sha>`): the bench signature, search NPS, perft NPS and suite result, fail-high first and EBF, each with a delta and a marker (🟢 improved, 🔴 regressed, ⚪ unchanged or within noise, 🆕 no baseline). Rows appear only for layers that are measured. The noise thresholds are the `[comment]` section of `perf-targets.toml`; the gate's verdict is one line and its table is folded into the details. `bench-report` in `crates/tools` renders it, so it can also be run locally against two directories of JSON lines:

```sh
cargo run --release -p rusty-chess-tools --bin bench-report -- \
  --targets perf-targets.toml --head runs/pr --base runs/v0.1.0 --base-label v0.1.0
```

## Strength tests: SPRT on CI

Every `feat` that touches search or evaluation must gain Elo. Add the `sprt` label to the PR and the `SPRT` workflow builds the PR and its merge base on the same `ubuntu-latest` runner with the same flags, then plays fastchess games between them from `tests/openings/openings.epd` (positions our own engine generated; regenerate with `cargo run --release -p rusty-chess-tools --bin openings -- --out tests/openings/openings.epd`). The settings live under `[sprt]` in `perf-targets.toml`:

- Games are played at a **fixed node budget per move**, not a time control, so the verdict does not depend on how fast the shared runner happened to be. Speed is judged separately by the bench gate in the same PR. A change that is better at equal nodes but slower shows both, in two rows.
- `elo0`/`elo1` are the hypotheses, `alpha`/`beta` the error rates, `max_rounds` the cap. Tighten `elo1` as gains get smaller.
- The result is one sticky comment: Elo ± error, LOS, games, W/L/D, LLR, and the verdict. H1 accepted is 🟢; H0 accepted is 🔴 and fails the job; the cap without a verdict is ⚪ and needs more games or a smaller `elo1`.
- The log and PGN are uploaded as the `sprt` artifact.

A full run is 10–35 minutes of runner time (a clear gain resolves in a few hundred games; "no difference" takes the longest), which is why they are opt-in by label rather than on every push. The `sprt-quick` label plays at most 50 pairs: a two-minute smoke test of the pipeline that never reaches a verdict. A quiet dedicated runner can replace `ubuntu-latest` later without changing anything else.

## Releases

Release Please opens and maintains a release PR as changes land on `main`. Merging that PR tags a new version (starting at `v0.1.0`), updates `CHANGELOG.md`, bumps every crate's `version` in the workspace (`release-type: rust`), and triggers the `release` job, which builds macOS arm64, macOS x86_64 and Linux x86_64 binaries and attaches them to the GitHub release, and the `telemetry` job, which benches the Linux build (with and without `stats`, plus the perft suite), attaches `bench.jsonl`, `stats.jsonl` and `perft.jsonl` to the release and writes the bench signature into the release notes. Those assets are the baseline every PR's bench comment compares against. Release PRs are authored by `jwallace145-release-bot[bot]` so that CI runs on them.

Merging the release PR is the human gate for every published build.
