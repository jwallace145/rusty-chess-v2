# Benchmark history

Every release, measured on a standardized `ubuntu-latest` runner from the shipped build (`x86-64-v3`). Rows are written by the `Release benchmarks` workflow into the Release Please PR, so each one is reviewed before its release is cut. The bench is 50 positions at a fixed depth with a fixed hash; the perft suite is `tests/perft.epd`. The depth was 6 through v0.13.0 and is 10 from the next release: Phase 4's pruning had cut the depth-6 bench under 100 ms, too short for the paired speed runs to resolve, and its EBF read a shallower tree than real games search. Signature, EBF, fail-high first, TT hit and instructions per node are not comparable across that line, and the first row after it shows deltas against a different depth. NPS varies between runner instances by up to 2x, so compare signatures and efficiency first and NPS only for large changes. Each release's raw data (`bench.jsonl`, `stats.jsonl`, `perft.jsonl`, logs and `summary.json`) is attached to its [GitHub release](https://github.com/jwallace145/rusty-chess-v2/releases) as files and as one tarball, and kept in [benchmarks/](../benchmarks/) at each tag. What each column means is explained in [CONTRIBUTING.md](../CONTRIBUTING.md#coverage-suites-and-performance-bars).

| Version | Date | Bench signature | Search NPS | Perft NPS | Perft suite | Fail-high first | EBF | TT hit | Instr / node |
|:--|:--|--:|--:|--:|:-:|--:|--:|--:|--:|
| v0.35.0 | 2026-10-31 | 1,190,429 | 1.83M | 202.02M | pass (63) | 87.9% | 1.86 | 53.6% | 3381 |
| v0.34.0 | 2026-10-31 | 1,128,154 | 2.38M | 200.84M | pass (63) | 88.6% | 1.88 | 52.8% | 3114 |
| v0.33.0 | 2026-10-31 | 1,128,154 | 3.72M | 366.92M | pass (63) | 88.6% | 1.88 | 52.8% | 3075 |
| v0.32.0 | 2026-10-31 | 1,146,589 | 2.10M | 202.83M | pass (63) | 88.4% | 1.89 | 54.2% | 3113 |
| v0.31.1 | 2026-10-31 | 1,146,589 | 2.79M | 292.75M | pass (63) | 88.4% | 1.89 | 54.2% | 3113 |
| v0.31.0 | 2026-10-30 | 1,146,589 | 3.00M | 290.54M | pass (63) | 88.4% | 1.89 | 54.2% | 3113 |
| v0.30.0 | 2026-10-30 | 1,120,617 | 2.16M | 200.90M | pass (63) | 88.6% | 1.88 | 54.3% | 3076 |
| v0.29.0 | 2026-10-30 | 1,201,313 | 1.88M | 222.91M | pass (63) | 88.8% | 1.90 | 53.1% | 3037 |
| v0.28.0 | 2026-10-30 | 1,201,313 | 2.11M | 235.87M | pass (63) | 88.8% | 1.90 | 53.1% | 3543 |
| v0.27.0 | 2026-10-30 | 1,697,503 | 3.47M | 291.70M | pass (63) | 88.2% | 1.90 | 53.5% | 2721 |
| v0.26.0 | 2026-10-30 | 1,697,503 | 2.81M | 203.34M | pass (63) | 88.2% | 1.90 | 53.5% | 2721 |
| v0.25.0 | 2026-10-30 | 1,697,503 | 2.53M | 200.71M | pass (63) | 88.2% | 1.90 | 53.5% | 2664 |
| v0.24.0 | 2026-10-30 | 1,697,503 | 4.98M | 364.87M | pass (63) | 88.2% | 1.90 | 53.5% | 2667 |
| v0.23.0 | 2026-10-30 | 1,529,684 | 2.62M | 203.09M | pass (63) | 88.5% | 1.81 | 55.0% | 2621 |
| v0.22.0 | 2026-10-30 | 1,075,807 | 2.96M | 228.21M | pass (63) | 89.0% | 1.59 | 60.7% | 2617 |
| v0.21.0 | 2026-10-30 | 1,416,355 | 3.48M | 265.07M | pass (63) | 89.9% | 1.78 | 57.0% | 2541 |
| v0.20.0 | 2026-10-30 | 1,717,816 | 3.11M | 201.89M | pass (63) | 90.6% | 1.64 | 53.3% | 2540 |
| v0.19.0 | 2026-10-29 | 1,991,209 | 3.05M | 290.41M | pass (63) | 90.2% | 1.77 | 50.1% | 2370 |
| v0.18.0 | 2026-10-29 | 1,991,209 | 4.95M | 364.67M | pass (63) | 90.2% | 1.77 | 50.1% | 2495 |
| v0.17.0 | 2026-10-28 | 2,077,084 | 5.69M | 374.28M | pass (63) | 90.5% | 1.82 | 50.1% | 2233 |
| v0.16.0 | 2026-10-28 | 2,077,084 | 4.20M | 360.44M | pass (63) | 90.5% | 1.82 | 50.1% | 2233 |
| v0.15.0 | 2026-10-28 | 2,566,403 | 3.70M | 202.65M | pass (63) | 89.9% | 2.02 | 27.9% | 2164 |
| v0.14.0 | 2026-10-28 | 3,775,127 | 8.82M | 371.94M | pass (63) | 90.3% | 1.79 | 28.6% | 1550 |
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
