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

In CI the `Bench` job runs the gate with `--env ci`. Speed bars are reported on every PR and enforced on release PRs and on `main`; efficiency bars are deterministic and always enforced; a PR that lowers any bar fails the ratchet check against `main`. When the engine clears a bar comfortably, raise it in the same PR. A bar of `0` is not enforced yet; set it when the layer first ships. A bar may be lowered only by naming it under `[ratchet] exceptions` in the same file with the reason, so the loosening is reviewed in the PR that makes it; that list is the record of every such decision, and an entry is removed once the bar is back above its old value.

### The bench comment

Every PR gets one bench comment, posted by the `Bench` job and refreshed in place on every push. Speed is judged in two hardware-independent ways, because GitHub's runners are a mix of CPU models: **Speed vs base** alternates bench runs of the PR and its merge base on the same runner and reports the ratio with a 95% interval, so the runner's speed cancels out; **Instructions / node** counts instructions with cachegrind, which is the same on any x86-64 machine and so compares exactly against the release. The absolute NPS rows are informational and never marked better or worse. Otherwise the comment compares the PR's run with the latest release's telemetry assets (`bench.jsonl`, `stats.jsonl`, `perft.jsonl` attached to the GitHub release by the `telemetry` job; for a release that predates them, the tagged commit's Bench run; failing both, the latest run on the base branch, labelled `main@<sha>`): the bench signature, search NPS, perft NPS and suite result, fail-high first and EBF, each with a delta and a marker (🟢 improved, 🔴 regressed, ⚪ unchanged or within noise, 🆕 no baseline). Rows appear only for layers that are measured. The noise thresholds are the `[comment]` section of `perf-targets.toml`; the gate's verdict is one line and its table is folded into the details. `bench-report` in `crates/tools` renders it, so it can also be run locally against two directories of JSON lines:

```sh
cargo run --release -p rusty-chess-tools --bin bench-report -- \
  --targets perf-targets.toml --head runs/pr --base runs/v0.1.0 --base-label v0.1.0
```

## Strength tests: SPRT on CI

Every `feat` that touches search or evaluation must gain Elo. Add the `sprt` label to the PR and the `SPRT` workflow builds the PR and its merge base on the same machine with the same flags, then plays fastchess games between them from `tests/openings/openings.epd` (positions our own engine generated; regenerate with `cargo run --release -p rusty-chess-tools --bin openings -- --out tests/openings/openings.epd`). The settings live under `[sprt]` in `perf-targets.toml`:

- Each leg runs on its own dedicated 64-core `c8a.16xlarge`, launched for the job and terminated after it by jwall-engineering-infra's `remote-task.yml`, and plays 60 games at once. The timed leg, the gate, runs on demand (about $3.45 an hour) so a spot reclamation cannot leave a PR without a verdict; the fixed-node leg runs on spot (about $1.27). The work is `ci/strength-task.sh`, run at the PR's head commit; the job log streams it live. A spot interruption or a launch with no capacity shows in the comment as such, with no verdict, and fails the check: re-run it. Setting the repository variable `SPRT_RUNNER` to `github` switches back to GitHub-hosted runners at 4 games at once. Fork PRs cannot use the instances.
- Two SPRTs run at once and post one comment. **SPRT (timed)** plays at the `[sprt] tc` time control (5+0.05): what the engine does at a clock, its cost per node included. It is the gate. **SPRT (fixed nodes)** plays at `nodes_per_move` (200,000): whether the search got smarter per node, whatever a node costs. It is informational and never fails the PR.
- Why both: a fixed-node test gives a change that makes nodes dearer extra work for free. From v0.13.0 to v0.18.0 the accepted fixed-node SPRTs summed to about +190 Elo; a 10+0.1 match between the two releases measured +26. The comment reads the pair: both H1 is a merge; fixed-node H1 without a timed H1 is a search gain eaten by its cost, worth a `perf` pass and a re-test rather than abandoning; a timed H1 without a fixed-node one is a speed gain, also a merge; neither is no gain.
- Within one run both engines share the runner, so its speed affects them equally: timed results are noisier than fixed-node ones but not biased.
- `elo0`/`elo1` are the hypotheses, `alpha`/`beta` the error rates, `max_rounds` the cap. Since Phase 4b's gains got smaller they are [0, 5] with a 5000-pair cap. Tighten `elo1` as gains get smaller.
- The result is one sticky comment with the gate's outcome, the reading of the pair, and both runs: Elo ± error, LOS, games, W/L/D, LLR and the verdict each. In the timed run, H1 accepted is 🟢 and the only result that passes the check; H0 accepted is 🔴; the cap without a verdict is ⚪, fails the job too, and needs more games or a smaller `elo1`. The owner can merge over the failed check for a reason stated in the PR, such as an inconclusive result at high LOS or a correctness fix or a smaller `elo1`.
- Each run's log and PGN are uploaded as the `sprt-timed` and `sprt-nodes` artifacts.

A full run on GitHub-hosted runners took up to five hours (a clear gain resolves in a few hundred games; "no difference" takes the longest), which is why they are opt-in by label rather than on every push. The `sprt-quick` label plays both runs to at most 50 pairs: a two-minute smoke test of the pipeline that never reaches a verdict. On the dedicated instances, at [0, 5], a clear gain (+25) settles in about 10 minutes, +15 in about 18 and +10 in about 30; a change worth nothing runs to the 5000-pair cap, about 48 minutes.

## Head-to-head matches: the Match workflow

An SPRT asks one question about one change and stops as soon as it knows the answer. Some questions need a measurement instead: how much stronger is v0.18.0 than v0.13.0 at a real clock, does a change that passed at 200k nodes still help at 2M, did the last five releases add up. The `Match` workflow answers those. Run it from the Actions tab (**Match → Run workflow**) or from a terminal:

```sh
gh workflow run Match -f candidate=v0.18.0 -f baseline=v0.13.0 -f tc=10+0.1 -f games=2000 \
  -f note="did Phase 4's fixed-node gains survive a clock?"
gh workflow run Match -f candidate=main -f baseline=v0.18.0 -f nodes=2000000 -f games=200
```

| Input | Default | Meaning |
|:--|:--|:--|
| `candidate` | `main` | the engine being judged: tag, branch or SHA |
| `baseline` | | the engine it is judged against |
| `tc` | `10+0.1` | seconds + increment per game, ignored when `nodes` is set |
| `nodes` | blank | a fixed node budget per move instead of a clock |
| `games` | `2000` | even: each opening is played from both sides |
| `confidence` | `99` | two-sided confidence of the interval: `95`, `99` or `99.9` |
| `hash` | `64` | MB per engine |
| `note` | | why the match is being run, shown at the top of the report |
| `runner` | `remote` | `remote`: a dedicated 64-core `c8a.16xlarge` spot instance, 60 games at once; `github`: a GitHub-hosted runner |
| `concurrency` | `4` | games at once on the GitHub-hosted runner only; the instance plays its physical cores less four |

Run it from `main`: the instance's AWS role trusts only `main` and pull requests. Refs are resolved to commits on GitHub, and `ci/match-task.sh` then fetches both commits on the instance, builds them at once with the same flags and benches them before playing, so the report records the commit and bench signature of exactly what played; the tools, openings and settings come from the workflow's commit. Bad settings fail the first job in seconds, before an instance is launched. The log shows the running result and games per minute every 30 seconds; the report goes to the run's summary page, and the report, the full fastchess log, every game's PGN and the timings are attached as the `match` artifact. A spot interruption is reported as one, with no result: re-run.

**Reading the result.** Games come in pairs from one opening with colours swapped, so the pair is the unit of evidence and its five possible scores (the pentanomial) give the variance. The report computes the candidate's Elo over the baseline and its interval at the chosen confidence from those counts, and reads it as stronger, weaker or no significant difference. At 99%, one match in a hundred shows a difference that isn't there; at 95%, one in twenty. The error shrinks with the square root of the games: at the draw rates we see, 2000 games give about ±14 Elo at 99%, 8000 about ±7. An inconclusive report says how many games would settle the difference it measured, and every report says how many ±5 Elo would take. Choose the number of games before the run and let it finish; stopping early because the result looks significant inflates the false positives. Re-running with more games is fine.

A match is a measurement, not a merge gate. Both engines always share one machine, so the comparison is fair; absolute numbers at a clock depend on that machine's speed, which is why the instance is always the same type. For a decision about one change, use the SPRT.

## Strength estimate: the rating ladder

The `Rating` workflow estimates absolute strength by playing the engine against an official Stockfish release (pinned in the workflow, Stockfish 19 now) limited to a ladder of `UCI_Elo` levels (`[rating]` in `perf-targets.toml`: 2800, 2950, 3100 and 3190, at 10+0.1, 100 games each, from our openings; the rungs move up as the engine outgrows them, and the file records every ladder with the releases rated on it). Each level gives "level plus the measured Elo difference"; the levels combine by inverse variance into one estimate with a 95% error bar, and a level won or lost in every game only bounds it.

- It runs on every published release, attaching `rating.json`, `rating.md`, the logs and PGNs to the release and adding a Strength section to its notes.
- Run it by hand for any tag, branch or SHA from the Actions tab, or `gh workflow run Rating -f ref=main`; `attach` puts the result on a release tag's release.
- It takes 40 to 50 minutes of runner time. Limited Stockfish is a ballpark scale, good for tracking strength between releases; a public rating (a Lichess bot, CCRL) is the real number.

## Releases

Release Please opens and maintains a release PR as changes land on `main`. Merging that PR tags a new version (starting at `v0.1.0`), updates `CHANGELOG.md`, bumps every crate's `version` in the workspace (`release-type: rust`), and triggers the `release` job, which builds macOS arm64, macOS x86_64 and Linux x86_64 binaries and attaches them to the GitHub release, and the `telemetry` job, which benches the Linux build (with and without `stats`, plus the perft suite), attaches `bench.jsonl`, `stats.jsonl` and `perft.jsonl` to the release and writes the bench signature into the release notes. Those assets are the baseline every PR's bench comment compares against. Release PRs are authored by `jwallace145-release-bot[bot]` so that CI runs on them.

Merging the release PR is the human gate for every published build.

### Benchmark history

Every release records its numbers so improvements are preserved across versions:

- When the Release Please PR opens or updates, the `Release benchmarks` workflow builds the code that will ship with the release CPU baseline and runs the bench, the stats bench and the perft suite **once**. It commits that run to the release PR with the release bot's token: the raw files under `benchmarks/v<version>/` (`bench.jsonl`, `stats.jsonl`, `perft.jsonl`, their logs and `summary.json`), a row in [docs/benchmarks.md](docs/benchmarks.md), and a `### Benchmarks` section under the version's heading in `CHANGELOG.md` with deltas against the previous release. Every number is derived from those files. Review them with the rest of the release PR. Release Please still writes everything else in the changelog; the workflow only adds that section, and records a version once.
- When the release is cut, the `telemetry` job attaches that same `benchmarks/v<version>/` directory to the GitHub release, file by file and as one `rusty-chess-v<version>-benchmarks.tar.gz`, and appends a Benchmarks table with deltas to the release notes. Historical analysis is a matter of downloading the tarballs (or `summary.json`) from the releases page; the same files are in the repo at each tag.
- `bench-history` in `crates/tools` does the rendering; `cargo run -p rusty-chess-tools --bin bench-history -- notes --version X --dir run --prev-dir prev --prev-version Y` previews it locally.
