# Self-play datasets

The ledger of every self-play generation and every run that made it. Each
net in [nets.md](nets.md) names the generations it trained on; this page says
what is in them. How the data is made and laid out is in
[datagen.md](datagen.md), and how it is checked in
[datagen-runbook.md](datagen-runbook.md).

All data is in `s3://jwall-engineering-rusty-chess-nnue/data/selfplay/v1/gen=N/run=RUN/`,
append-only and kept forever. Every number here comes from the shard
manifests or from `datagen-audit`, never from the workflow's progress lines.

## Generations

| Gen | Played by | Positions | Games | Shards | Runs | Full audit | Trained into |
|--:|:--|--:|--:|--:|--:|:--|:--|
| 0 | PeSTO (`v0.24.0`–`v0.25.0`) | 231,240,344 | 2,387,961 | 9,329 | 3 | GO: 0 P0 failures | net 1, net 2, net2x80, net 3, net 4, net4w512, net 5 |
| 1 | net 1 (`v0.29.0`) | 368,354,466 | 3,703,015 | 14,716 | 3 | GO: 0 P0 failures, 0 P1 warnings | net 2, net2x80, net 3, net 4, net4w512, net 5 |
| 2 | net2x80 (`v0.31.1`) | 307,037,810 | 3,265,807 | 12,353 | 1 | GO: 0 P0 problems; 2,304/2,304 games regenerated from their seeds | net 3, net 4, net4w512, net 5 |
| 3 | net4w512 (`v0.36.0`) | 628,213,254 | 6,562,925 | 25,158 | 1 | GO: 0 P0 problems, 0 P1 warnings; 2,304/2,304 games regenerated from their seeds | net 5 |

## Generation 0: PeSTO

**Settings** (identical in every manifest): 5,000 nodes per move; 8 random
plies, plus one more at random (the rule at the time, so half the openings
gave white an extra move; generation 1 uses 8 or 10). Openings were replayed
beyond ±400 cp. Win adjudication at 2,000 cp held for 4 plies; draw
adjudication at ≤ 10 cp for 12 plies after ply 80; at most 400 plies;
25,000 positions a shard.

| Run | Date | Engine | Instances | Positions | Games | Notes |
|:--|:--|:--|--:|--:|--:|:--|
| `20261030-41004581936-1` | 2026-10-30 | `v0.24.0` `af669ec` | 1 | 8,197,507 | 84,496 | Smoke test |
| `20261030-41006956708-1` | 2026-10-30 | `v0.25.0` `3e3bdbd` | 1 | 8,461,614 | 87,384 | Second smoke test, after the uploader throughput fix (#110) |
| `20261030-41007876197-1` | 2026-10-30 | `v0.25.0` `3e3bdbd` | 3 (spot) | 214,581,223 | 2,216,081 | t0 49.4M, t1 81.6M, t2 83.5M. t0 was interrupted by spot at 05:51 and lost about 350k positions in progress (the process-group bug fixed in #113). Stopped at about 200M on purpose |

**Audit** (#113, the full pass over every record): 0 P0 failures; bulletformat
parsed all 231M records.

**Quality:**

| Metric | Value |
|:--|:--|
| Results (W / D / B) | 32.2% / 34.5% / 33.3% |
| Duplicate positions | 2.2% |
| \|score\| ≤ 100 | 39.0% |
| Endgame share | 46% |
| Games ended by win adjudication | 64% |

**Findings that shaped generation 1** (details in the runbook):
- At 2,000 cp every adjudicated game was already lost, while 1,200 cp is
  right in 99.98% of cases.
- Games that ended in repetition were level at depth about 95% of the
  time, so the draw label agrees with the score.
- Odd random plies favoured white.

## Generation 1: net 1

**Settings** (identical in every manifest): 5,000 nodes per move; 8 or 10
random plies (always even); openings replayed beyond ±250 cp; win
adjudication at 1,200 cp for 4 plies; the rest as generation 0. Every run
was built with net 1 (`net1-l050-r41022503872`) as the default.

| Run | Date | Engine | Instances × minutes | Market | Positions | Games | Notes |
|:--|:--|:--|:--|:--|--:|--:|:--|
| `20261030-41084379912-1` | 2026-10-30 | `v0.29.0` `708adbd` | 1 × 5 | not recorded | 7,488,073 | 75,190 | Smoke test: about 25,000 positions/s an instance, 11% below PeSTO |
| `20261030-41086960808-1` | 2026-10-30 | `v0.29.0` `708adbd` | 3 × 100 asked; 2 ran about 85 | on demand | 276,116,559 | 2,775,313 | t0 139.5M, t1 136.6M. t2 never launched: no `c8a.16xlarge` capacity, spot or on demand, in any us-east-1 zone. Spot capacity was also gone for t0 and t1, which fell back to on demand. Cancelled early to limit cost |
| `20261030-41091882414-1` | 2026-10-30 | `v0.29.0` `fe9e9f2` | 1 × 100 asked; ran about 55 | on demand | 84,749,834 | 852,512 | Top-up replacing t2. `fe9e9f2` differs from `708adbd` only in the audit's colour-gap bar (#127). Cancelled with the main run |

**Audit:** the full pass (75 batches, about 5 minutes on 16 cores) gave
**GO**:
- 0 P0 failures over all 368,354,466 records;
- bulletformat exact: 32 bytes a record;
- every shard complete, with no orphans after the cancel;
- 256 workers, 256 distinct seeds.

Label spot checks (2,000 fresh re-searches a run): correlation 0.99, and
the same sign 99.5–99.6% of the time.

**Quality:**

| Metric | Value |
|:--|:--|
| Results (W / D / B) | 31.8% / 44.7% / 23.5% |
| Duplicates within a batch | 3.1% |
| \|score\| ≤ 100 | 32.0% (smoke sample) |
| Endgame share | 48.7% (smoke sample) |
| Games ended by win adjudication | 55% |

**Compared with generation 0:**
- **More draws and a white edge.** The colour gap is 8.3 points (the
  bar was widened to 10 in #127): a stronger player shows white's
  first-move advantage, and draws rose from 34.5% to 44.7%.
- **Fewer one-sided games.** Win adjudication fell from 64% to 55%.

**Cost:** about $15–20. It ran on demand rather than spot, because spot
capacity was gone. This is an estimate from the instance-hours, not a bill.

## Generation 2: net2x80

**Settings** (identical in every manifest): the same as generation 1. That's
5,000 nodes a move, 8 or 10 random plies, openings replayed beyond ±250 cp,
and win adjudication at 1,200 cp for 4 plies. Played by
`net2x80-l050-r41131366975` (net 2 trained for 80 superbatches; λ 0.5),
datagen built for the instance's own CPU (`native`, #136).

| Run | Date | Engine | Instances × minutes | Market | Positions | Games | Notes |
|:--|:--|:--|:--|:--|--:|--:|:--|
| `20261031-41153094884-1` | 2026-10-31 | `v0.31.1` `75017a5` | 3 × 70 | spot | 307,037,810 | 3,265,807 | t0 98.1M, t1 104.2M, t2 104.7M. No interruptions. The first run with the native build, about 25,150 positions/s an instance |

The two benchmark runs for #136 (`upload: false`) wrote nothing to the
generation.

**Full audit** (`ci/datagen-full-audit.sh`, 7 minutes on 16 cores): **GO**.
1. **Inventory:**
   - every shard complete, with no orphans;
   - shard numbers gap-free for all 192 workers;
   - no duplicate shards, and all seeds unique;
   - one settings block, one engine, one generation;
   - each task's `task.json` totals equal its manifests.
2. **Every record:** 0 failures over all 307,037,810 records, and bulletformat
   exact at 32 bytes a record.
3. **No P1 warnings** in any of the 63 batches.
4. **Determinism: 2,304 of 2,304 games regenerated identically** from their
   seeds (12 per worker, all 192 workers).
   - This build plays generation 2 with net2x80 (sha256 `731b30d2…677e`).
   - **Negative controls:** the same seeds with net 1, or at 2,500 nodes,
     regenerated 0 of 48 games identically.
   - So **net2x80 at 5,000 nodes played the whole generation**, even though
     these manifests predate the recorded `provenance.net`.

| Quality | Generation 1 | Generation 2 |
|:--|--:|--:|
| Results (W / D / B) | 31.8 / 44.7 / 23.5% | 32.4 / 43.0 / 24.7% |
| White–black gap | 8.3 points | 7.7 points |
| Games ended by win adjudication | 55% | 56.6% |
| Duplicates within a batch (200 shards) | 3.1% | 1.48% |
| Median game length (engine plies) | 114 | 112 |
| Records a game | 99.5 | 94.0 |
| \|score\| ≤ 100 | 32.0% (smoke sample) | 31.0% |
| Endgame share | 48.7% (smoke sample) | 46.0% |

**Compared with generation 1:**
- slightly more decisive (draws −1.7 points);
- half the within-batch repetition;
- a little shorter.
- **No sign of the self-play collapsing toward fewer positions,** but these
  are within-generation measures. Duplicate positions *across* generations and
  structural novelty aren't measured yet: that needs the cross-generation
  comparison mode the audit cross-reference recommends.

**Cost:** about $5 on spot (3 instances, about 85 minutes including build and
upload).

## Generation 3: net4w512

**Settings** (identical in every manifest): the same as generations 1 and 2.
Played by `net4w512-l050-r41170687764` (512 hidden neurons, on generations
0–2; λ 0.5), the default in `v0.36.0`, with the native build. The first
generation whose manifests record the net (`provenance.net`, sha256
`5d24472c…5b68`).

| Run | Date | Engine | Instances × minutes | Market | Positions | Games | Notes |
|:--|:--|:--|:--|:--|--:|--:|:--|
| `20261031-41180888933-1` | 2026-10-31 | `v0.36.0` `f7605e5` | 3 × 160 | spot | 628,213,254 | 6,562,925 | t0 204.4M, t1 208.2M, t2 215.6M. No interruptions. About 21,800 positions/s an instance, 13% below generation 2's net2x80: the 512-neuron net is slower per node |

**Full audit** (`ci/datagen-full-audit.sh` on 16 cores, after a 17-minute
download of the 10 GB generation): **GO**.
1. **Inventory:** 0 P0 problems over 25,158 shards. One settings block, one
   engine, one generation and one net throughout.
2. **Every record:** 0 failures over all 628,213,254 records, and
   bulletformat exact at 32 bytes a record.
3. **No P1 warnings** in any of the 127 batches.
4. **Determinism: 2,304 of 2,304 games regenerated identically** from their
   seeds (12 per worker, all 192 workers), with the tool built at
   `f7605e5`. The net named in the manifests is the net this build plays.

| Quality | Generation 1 | Generation 2 | Generation 3 |
|:--|--:|--:|--:|
| Results (W / D / B) | 31.8 / 44.7 / 23.5% | 32.4 / 43.0 / 24.7% | 30.3 / 46.4 / 23.3% |
| White–black gap | 8.3 points | 7.7 points | 7.0 points |
| Games ended by win adjudication | 55% | 56.6% | 53.2% |
| Duplicates within a batch (200 shards) | 3.1% | 1.48% | 1.76% |
| Median game length (engine plies) | 114 | 112 | 112 |
| Records a game | 99.5 | 94.0 | 95.7 |
| \|score\| ≤ 100 | 32.0% (smoke sample) | 31.0% | 31.8% |
| Endgame share | 48.7% (smoke sample) | 46.0% | 47.1% |

**Compared with generation 2:**
- **more drawn** (+3.4 points), the reverse of generation 1 → 2: the stronger
  net converts fewer games to a win adjudication;
- a slightly smaller colour gap;
- within-batch repetition about level (1.76% against 1.48%);
- the same game length.

**Cost:** about $9 on spot (3 × `c8a.16xlarge` at $1.149/h for about 2.7
hours each; estimated from the run's spot price, not billed figures).

## Adding a run or a generation

After every Datagen run, including smoke tests and failed or cancelled runs:

1. Add a row to the generation's table, with the run id, date, engine
   version and commit, instances × minutes asked and run, market, positions
   and games (summed from the manifests), and what happened (interruptions,
   capacity, cancellations).
2. When the generation is complete, run `ci/datagen-full-audit.sh`
   ([datagen-runbook.md](datagen-runbook.md)) and record its verdict, the
   determinism result and the quality numbers, including how they moved from
   the previous generation.
3. Update the Generations table, including which nets trained on it.
