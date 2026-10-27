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
| `crates/core` | Board, bitboards, magics, move generation, Zobrist hashing, make/unmake. No dependencies. |
| `crates/search` | Search, transposition table, move picker, evaluation, `SearchStats` telemetry. |
| `crates/uci` | UCI protocol and the `rusty-chess` binary with `bench` and `perft` subcommands. |
| `crates/tools` | Perft suite runner, bench and stats diffing, data generation later. |
| `benches/` | Criterion microbenchmarks. |
| `tests/` | Perft EPD suite, UCI smoke tests, mate-in-N positions. |

## Building

```sh
cargo build --release
./target/release/rusty-chess bench
./target/release/rusty-chess perft --depth 5
```

Local builds target the host CPU (`.cargo/config.toml`). Build with
`--features stats` to enable the search statistics counters; they compile to
nothing otherwise.

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
