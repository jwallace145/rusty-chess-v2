# Contributing

This repo follows the conventions in [docs/repo-conventions.md](docs/repo-conventions.md). The short version: one-line Conventional Commits, every change through a pull request, squash or rebase merges only, releases cut by Release Please, and every search or evaluation change justified by a number.

## Workflow

1. Branch from `main`.
2. Make the change. Keep the code formatted (`cargo fmt`) and run the local checks below.
3. Open a PR with a typed template (see below). The PR title must be a valid Conventional Commit.
4. For `feat` and `perf` PRs, fill in the bench signature. CI runs `bench` and uploads `bench.jsonl` and `stats.jsonl` as artifacts; if the PR description carries a `Bench: <nodes>` line, CI checks that it matches.
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
| `Bench` | `rusty-chess bench` in release mode; uploads `bench.jsonl` and `stats.jsonl`; checks a `Bench:` line in the PR body if present |

NPS on shared runners is noisy, so CI reports it but gates only on the node signature. Compare NPS on a quiet local machine.

## Releases

Release Please opens and maintains a release PR as changes land on `main`. Merging that PR tags a new version (starting at `v0.1.0`), updates `CHANGELOG.md`, bumps every crate's `version` in the workspace (`release-type: rust`), and triggers the `release` job, which builds macOS arm64, macOS x86_64 and Linux x86_64 binaries and attaches them to the GitHub release. Release PRs are authored by `jwallace145-release-bot[bot]` so that CI runs on them.

Merging the release PR is the human gate for every published build.
