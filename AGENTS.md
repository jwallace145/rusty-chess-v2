# Guidance for coding agents

This repository is a single-threaded UCI chess engine in Rust. Read [README.md](README.md) for the layout, [docs/roadmap.md](docs/roadmap.md) for the phased plan and its exit gates, and [CONTRIBUTING.md](CONTRIBUTING.md) for the full rules. The rules below are the ones you MUST follow.

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
- **Never claim an Elo gain without an SPRT result in the PR.** A `feat` that touches search or eval needs a fastchess SPRT against `main` (Elo, LOS, games) in the PR description. A failed SPRT does not merge; a feature that does not gain Elo is reverted even if it looks right.

## Engine rules

- Correct before fast: perft stays in CI permanently and must pass before any search change merges.
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
cargo run --release -p rusty-chess-uci -- bench   # note the signature
npm ci && npm test
```

## Things that need the owner

Repository secrets (`RELEASE_BOT_CLIENT_ID`, `RELEASE_BOT_PRIVATE_KEY`), the release bot app installation, repository settings, and the Homebrew tap. List them in the PR instead of attempting them.
