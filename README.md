# rusty-chess-v2

A single-threaded UCI chess engine in Rust whose classical search is
measurably strong before any NNUE work starts. Every change is justified by a
number: perft for correctness, bench for speed, stats for search efficiency,
and SPRT for Elo.

The full plan is in [docs/roadmap.md](docs/roadmap.md). The repository rules
are in [docs/repo-conventions.md](docs/repo-conventions.md), restated for
contributors in [CONTRIBUTING.md](CONTRIBUTING.md) and for coding agents in
[AGENTS.md](AGENTS.md).

## Layout

| Path | Contents |
| --- | --- |
| `src/` | The `rusty-chess` binary (root package): UCI protocol, `bench` and `perft` subcommands. |
| `crates/core` | Board, bitboards, magics, move generation, Zobrist hashing, make/unmake. No dependencies. |
| `crates/search` | Search, transposition table, move picker, evaluation, `SearchStats` counters and the JSON lines telemetry writer. |
| `crates/tools` | Perft suite runner, bench and stats diffing, data generation later. |
| `tests/` | UCI smoke tests, the perft suite `perft.epd`, mate-in-N positions. |
| `perf-targets.toml` | The performance bars `perf-gate` enforces; they only go up. |
| `benches/` | Criterion microbenchmarks for the binary; crates keep their own under `crates/*/benches`. |

## Building and running

```sh
cargo build --release
./target/release/rusty-chess                 # UCI on stdin/stdout
./target/release/rusty-chess bench           # signature and NPS over 50 fixed positions
./target/release/rusty-chess bench --depth 12 --hash 16 --out bench.jsonl
./target/release/rusty-chess perft --depth 5 # Phase 1
```

Local builds target the host CPU (`.cargo/config.toml`); release CI builds a
portable baseline instead.

## Measuring

Three layers are measured separately because each one fails differently.

| Layer | Command | Output |
| --- | --- | --- |
| Move generation speed | `rusty-chess perft` | perft NPS (bulk counting) |
| Search speed | `rusty-chess bench` | the node **signature** and NPS |
| Search efficiency | `cargo build --release --features stats`, then `bench` | fail-high-first rate, EBF, TT hit rate, qsearch share, cutoffs by move-picker stage |

The `stats` feature compiles the counters in; without it every `stat!` in
the search expands to nothing. `bench --out FILE` writes JSON lines: one
`summary` record per position and one `bench` record at the end, each with a
schema version `v`. CI runs bench on every PR and uploads `bench.jsonl` and
`stats.jsonl` as artifacts; if the PR description carries a `Bench: <nodes>`
line, CI checks that it matches.

Each layer also has a coverage suite that only grows (`tests/perft.epd`
for move generation, the bench positions for search) and a performance bar
in `perf-targets.toml` that only rises. `perf-gate` compares a run's JSON
lines to the bars; CI reports speed bars on every PR, enforces them on
release PRs, and always enforces the efficiency bars. Every PR gets a
bench comment, refreshed on each push, with the deltas against the latest
release, whose telemetry is attached to the GitHub release as assets. See
[CONTRIBUTING.md](CONTRIBUTING.md).

## Phases

| Phase | Gate |
| --- | --- |
| 0. Scaffolding and telemetry skeleton | CI uploads `bench.jsonl` on every PR; Release Please cuts `v0.1.0` |
| 1. Board and move generation | Full perft suite passes; bulk perft ≥ 100M NPS |
| 2. Baseline search | Legal games under fastchess, stable bench signature, stats on every search |
| 3. Move ordering and transposition table | Fail-high-first ≥ 90%, EBF ≤ 3 |
| 4. Selectivity | EBF ≈ 2, each technique gains Elo under SPRT |
| 5. First release | `brew install` works; publicly rated |
| 6. NNUE | NNUE build beats PeSTO build by 100+ Elo |
| 7. Extensions | Lazy SMP, HTTP API |

## License

MIT, see [LICENSE](LICENSE).
