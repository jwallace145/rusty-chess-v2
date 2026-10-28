# Benchmark history

Every release, measured on a standardized `ubuntu-latest` runner from the shipped build (`x86-64-v3`). Rows are written by the `Release benchmarks` workflow into the Release Please PR, so each one is reviewed before its release is cut. The bench is 50 positions at a fixed depth with a fixed hash; the perft suite is `tests/perft.epd`. NPS varies between runner instances by up to 2x, so compare signatures and efficiency first and NPS only for large changes. Each release's raw data (`bench.jsonl`, `stats.jsonl`, `perft.jsonl`, logs and `summary.json`) is attached to its [GitHub release](https://github.com/jwallace145/rusty-chess-v2/releases) as files and as one tarball, and kept in [benchmarks/](../benchmarks/) at each tag. What each column means is explained in [CONTRIBUTING.md](../CONTRIBUTING.md#coverage-suites-and-performance-bars).

| Version | Date | Bench signature | Search NPS | Perft NPS | Perft suite | Fail-high first | EBF | TT hit | Instr / node |
|:--|:--|--:|--:|--:|:-:|--:|--:|--:|--:|
| v0.13.0 | 2026-10-28 | 387,631 | 4.79M | 256.73M | pass (63) | 89.2% | 1.91 | 35.8% | 1592 |
| v0.12.0 | 2026-10-28 | 601,010 | 6.46M | 262.19M | pass (63) | 89.8% | 2.17 | 28.7% | 1305 |
| v0.11.0 | 2026-10-27 | 716,057 | 6.39M | 201.46M | pass (63) | 93.1% | 2.17 | 36.0% | 1305 |
| v0.10.0 | 2026-10-27 | 7,082,718 | 11.30M | 281.40M | pass (63) | 97.5% | 3.90 | 35.6% | 1215 |
| v0.9.0 | 2026-10-27 | 7,143,901 | 14.29M | 368.37M | pass (63) | 97.5% | 3.86 | 35.7% | 1212 |
| v0.8.0 | 2026-10-27 | 8,243,750 | 7.37M | 199.85M | pass (63) | 97.5% | 4.18 | 34.8% | 1199 |
| v0.7.0 | 2026-10-27 | 8,243,750 | 7.90M | 201.52M | pass (63) | 97.5% | 4.18 | 34.8% | 1199 |
| v0.6.0 | 2026-10-27 | 8,986,067 | 8.75M | 203.09M | pass (63) | 96.8% | 4.33 | 33.7% | — |
| v0.5.0 | 2026-10-27 | 14,942,735 | 12.62M | 299.32M | pass (63) | 85.6% | 4.74 | 29.4% | — |
| v0.4.0 | 2026-10-27 | 14,255,682 | 6.32M | 207.67M | pass (63) | 78.7% | 4.62 | 29.5% | — |
| v0.3.0 | 2026-10-27 | 68,269,801 | 6.85M | 208.20M | pass (63) | 69.8% | 7.00 | — | — |
| v0.2.0 | 2026-10-27 | 0 | — | 203.66M | pass (63) | — | — | — | — |
| v0.1.0 | 2026-10-27 | — | — | — | — | — | — | — | — |
