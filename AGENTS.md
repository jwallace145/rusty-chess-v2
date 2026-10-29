# Guidance for coding agents

This repository is a single-threaded UCI chess engine in Rust. Read [README.md](README.md) for the layout, [docs/roadmap.md](docs/roadmap.md) for the phased plan and its exit gates, [docs/lessons-from-rusty-chess-v1.md](docs/lessons-from-rusty-chess-v1.md) for the bugs that sank the first attempt and the guards that catch them, and [CONTRIBUTING.md](CONTRIBUTING.md) for the full rules. The rules below are the ones you MUST follow.

## Rules

- **Never push to `main`.** Every change goes through a pull request on a branch. There are no exceptions after the initial scaffold.
- **Commits are a single line**: `type(scope)?: subject`, at most 100 characters. Allowed types: `feat`, `fix`, `docs`, `chore`, `refactor`, `ci`, `build`, `perf`, `style`, `test`, `revert`. The only thing allowed after the header is a blank line and `Co-authored-by: Name <email>` trailers. No body text. Put details in the PR description.
- **Open PRs with a typed template** and fill in every section:

  ```sh
  gh pr create --template <feat|perf|fix|docs|chore|refactor|ci>.md
  ```

  The PR title MUST be a valid Conventional Commit; squash merges use it as the commit message.
- **Run the repo's checks locally before opening the PR** (see below).
- **Never change search or eval without a bench run.** Put the `Bench: <nodes>` signature before and after in the PR description. A changed signature on a `perf` PR is a bug; a `perf` PR must leave the signature unchanged. Use `feat` for anything that changes the signature and `fix` for correctness (perft, TT, mate scores).
- **Every measured layer ships with a coverage suite and a performance bar, and both only grow.** Move generation is covered by `tests/perft.epd` (append positions, never remove or change a line), search by the bench positions, efficiency by the `stats` counters. The bars live in `perf-targets.toml`; raise one when the engine clears it, never lower one. CI's `Bench` job runs `perf-gate`: speed bars are reported on every PR and enforced on release PRs, efficiency bars are always enforced, and a PR that loosens a bar fails. The same job posts the bench comment with deltas against the latest release; read it before claiming a speed or efficiency change, and quote its numbers in the PR description.
- **Never claim an Elo gain without an SPRT result in the PR.** A `feat` that touches search or eval gets the `sprt` label; the `SPRT` workflow plays the PR against its base twice at once, timed at the `[sprt] tc` control and at fixed nodes per move, and posts both in one comment. The **timed** result is the gate; the fixed-node result is informational and says whether the search got smarter per node (fixed nodes reward a change that makes nodes dearer, so it alone overstated five releases' gains about sevenfold). Quote both, and the comment's reading of the pair, in the PR description. Only an accepted H1 in the timed test passes the check: H0 and an inconclusive result both fail it, and a feature that does not gain Elo is reverted even if it looks right. The owner may merge over a failed check for a reason stated in the PR (an inconclusive result at high LOS, a correctness fix); an agent never does. Never run SPRT on a laptop and paste the numbers; the standardized runner is the record.

## Engine rules

- Correct before fast: perft stays in CI permanently and must pass before any search change merges.
- The root is an ordinary negamax node: children get `-beta, -alpha` and alpha rises on improvement. Never search root moves with a full window.
- An aborted search is discarded entirely: once the stop flag is set, no transposition table store happens, no score is compared, and the result is the last *completed* iteration. The bench signature must be identical run to run; any variance means abort handling is leaking.
- A transposition table bound is derived from the final score against the original alpha and beta at every node, quiescence included. Never store a bounded score as exact.
- A move-picker stage with zero cutoffs over a bench run is a bug until proven otherwise; read the per-stage counters.
- Mate scores are offset by ply from the root, never by remaining depth.
- No heap allocation in the search path. Move lists are stack-allocated.
- Telemetry (`--features stats`) compiles to nothing in release builds; every counter is a macro.
- One feature per commit so the bench signature can be bisected.
- Eval is frozen at PeSTO until the Phase 4 exit gate holds.
- Cargo workspace members are listed explicitly in `Cargo.toml` and every crate carries its own `version`; Release Please bumps them together. Never edit versions or `CHANGELOG.md` by hand.

## Before opening a PR

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --locked
cargo test --workspace --locked --features stats
cargo run --release -- bench   # note the signature
npm ci && npm test
```

## Things that need the owner

Repository secrets (`RELEASE_BOT_CLIENT_ID`, `RELEASE_BOT_PRIVATE_KEY`), the release bot app installation, repository settings, and the Homebrew tap. List them in the PR instead of attempting them.
