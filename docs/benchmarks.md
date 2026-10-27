# Benchmark history

Every release, measured on a standardized `ubuntu-latest` runner from the shipped build (`x86-64-v3`). Rows are written by the `Release benchmarks` workflow into the Release Please PR, so each one is reviewed before its release is cut. The bench is 50 positions at a fixed depth with a fixed hash; the perft suite is `tests/perft.epd`. NPS varies between runner instances by up to 2x, so compare signatures and efficiency first and NPS only for large changes. What each column means is explained in [CONTRIBUTING.md](../CONTRIBUTING.md#coverage-suites-and-performance-bars).

| Version | Date | Bench signature | Search NPS | Perft NPS | Perft suite | Fail-high first | EBF | TT hit |
|:--|:--|--:|--:|--:|:-:|--:|--:|--:|
| v0.4.0 | 2026-10-27 | 14,255,682 | 6.32M | 207.67M | pass (63) | 78.7% | 4.62 | 29.5% |
| v0.3.0 | 2026-10-27 | 68,269,801 | 6.85M | 208.20M | pass (63) | 69.8% | 7.00 | — |
| v0.2.0 | 2026-10-27 | 0 | — | 203.66M | pass (63) | — | — | — |
| v0.1.0 | 2026-10-27 | — | — | — | — | — | — | — |
