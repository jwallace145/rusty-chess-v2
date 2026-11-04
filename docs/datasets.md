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
| 0 | PeSTO (`v0.24.0`–`v0.25.0`) | 231,240,344 | 2,387,961 | 9,329 | 3 | GO: 0 P0 failures | net 1, net 2, net2x80 (and its λ sweep), net 3, net 4, net4w512, net 5, net5x120, net5b8, net6, net6k4, net7 |
| 1 | net 1 (`v0.29.0`) | 368,354,466 | 3,703,015 | 14,716 | 3 | GO: 0 P0 failures, 0 P1 warnings | net 2, net2x80 (and its λ sweep), net 3, net 4, net4w512, net 5, net5x120, net5b8, net6, net6k4, net7 |
| 2 | net2x80 (`v0.31.1`) | 307,037,810 | 3,265,807 | 12,353 | 1 | GO: 0 P0 problems; 2,304/2,304 games regenerated from their seeds | net 3, net 4, net4w512, net 5, net5x120, net5b8, net6, net6k4, net7 |
| 3 | net4w512 (`v0.36.0`) | 628,213,254 | 6,562,925 | 25,158 | 1 | GO: 0 P0 problems, 0 P1 warnings; 2,304/2,304 games regenerated from their seeds | net 5, net5x120, net5b8, net6, net6k4, net7 |
| 4 | net 5 (`v0.37.1`) | 495,147,759 | 5,252,191 | 19,838 | 2 | GO: 0 P0 problems, 0 P1 warnings; 3,072/3,072 games regenerated from their seeds | net6, net6k4, net7 |
| 5 | net6k4 (`v0.42.0`) | 318,934,530 | 3,409,848 | 12,755 | 1 | GO: 0 P0 problems, 0 P1 warnings; 2,304/2,304 games regenerated from their seeds | net7 |

**Generation 6** will be played by net7 (`datagen.toml` is at `generation = 6`).
No run has started.

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

**The two benchmark runs for #136** (`upload: false`) measured the native
build against `x86-64-v3` before this generation, with the same seed (7), net
and settings, at `v0.31.0` (`bf27704`), on spot. Neither is part of the
generation:

| Run | Target | Minutes | Positions | Games | Shards | Positions/s at 150 s |
|:--|:--|:--|--:|--:|--:|--:|
| `20261030-41144384553-1` | `native` | 5 asked; **reclaimed by spot** at 23:29:26 UTC after 188 s | 4,062,796 | 43,130 | 192 | 25,235 |
| `20261030-41144392189-1` | `x86-64-v3` | 5 | 6,920,847 | 73,583 | 320 | 23,037 |

- **The native build is about 9.5% faster.**
- **The games are the same, as far as the interrupted run can show.** All 128
  shards the native run finished normally are byte-identical to the other run's
  (3,207,361 positions, 33,979 games), by the SHA-256s in their manifests.
- **The other 64 can't be compared.** They're the shards the interruption
  flushed early (10,947–14,896 positions, against 25,001–25,265), and the
  artifacts hold only checksums, so a prefix match can't be checked.
- **Nothing was uploaded:** both `task.json` files say `"uploaded": false`. The
  native run's log line "192 complete shards are in S3" is the interrupted
  path's fixed wording. S3 itself wasn't listed.

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

## Generation 4: net 5

**Settings** (identical in every manifest): the same as generations 1–3.
Played by `net5-l050-r41229287546` (512 hidden, generations 0–3; λ 0.5), the
default in `v0.37.0`–`v0.37.1`, with the native build. Two engine commits,
the same engine: the top-up was built from `1a6c3d3`, which differs from
`v0.37.1` (`60a5252`) only in `docs/nets.md` and `docs/roadmap.md` (#160);
both report 0.37.1, bench 1101129 and net 5's sha256 `acd8e049…906b`.

| Run | Date | Engine | Instances × minutes | Market | Positions | Games | Notes |
|:--|:--|:--|:--|:--|--:|--:|:--|
| `20261101-41259904529-1` | 2026-11-01 | `v0.37.1` `60a5252` | 3 × 135 asked; t1 ran 16 | spot | 374,338,166 | 3,970,953 | t0 177.8M, t2 177.5M, about 21,950 positions/s an instance. **t1 was reclaimed by spot at 01:26 UTC, 16 minutes in:** its 777 complete shards (19.0M positions) were kept, only the games in progress lost. GitHub can't re-run a job while its run is going, so a top-up replaced it |
| `20261101-41263763971-1` | 2026-11-01 | `0.37.1` `1a6c3d3` | 1 × 125 asked; ran about 90 | spot | 120,809,593 | 1,281,238 | Top-up replacing t1. **Cancelled** at about 120M to limit cost once the generation passed the night's reduced target of about 300M; it ended at about 495M |

**Full audit** (`ci/datagen-full-audit.sh` on 16 cores, built at `1a6c3d3`;
a 13-minute download of 8.1 GB, then 12 minutes): **GO**.
1. **Inventory:** 0 P0 problems over 19,838 shards, including the
   interrupted task and the cancelled run: no gaps, orphans, duplicate
   shards or shared seeds; one settings block, one generation, one net.
2. **Every record:** 0 failures over all 495,147,759 records, and
   bulletformat exact at 32 bytes a record.
3. **No P1 warnings** in any of the 101 batches.
4. **Determinism: 3,072 of 3,072 games regenerated identically** from their
   seeds (12 per worker, all 256 workers of both runs).

| Quality | Generation 2 | Generation 3 | Generation 4 |
|:--|--:|--:|--:|
| Results (W / D / B) | 32.4 / 43.0 / 24.7% | 30.3 / 46.4 / 23.3% | 29.7 / 47.2 / 23.1% |
| White–black gap | 7.7 points | 7.0 points | 6.6 points |
| Games ended by win adjudication | 56.6% | 53.2% | 52.5% |
| Duplicates within a batch (200 shards) | 1.48% | 1.76% | 1.71% |
| Median game length (engine plies) | 112 | 112 | 109 |
| Records a game | 94.0 | 95.7 | 94.3 |
| \|score\| ≤ 100 | 31.0% | 31.8% | 32.4% |
| Endgame share | 46.0% | 47.1% | 46.0% |

**Compared with generation 3:**
- **draws up again,** +0.8 points to 47.2% (generation 2 → 3 was +3.4).
  Every generation so far has drawn more; the audit's bar is 55%, and as
  draws rise the game results carry less signal for training;
- the colour gap a little smaller, repetition level, games slightly shorter;
- no sign of collapse toward fewer distinct positions within a batch.

**Cost:** about $8 on spot, an estimate from instance-hours at generation
3's spot price (about $1.15/h), not billed figures.

## Generation 5: net6k4

**Settings** (identical in every manifest): the same as generations 1–4.
Played by `net6k4-l050-r41282883885` (512 hidden, 8 output buckets, **4 king
buckets**; generations 0–4), the default in `v0.42.0` (`a921f3f`), with the
native build. One engine commit throughout.

| Run | Date | Engine | Instances × minutes | Market | Positions | Games | Notes |
|:--|:--|:--|:--|:--|--:|--:|:--|
| `20261101-41352126811-1` | 2026-11-01 | `v0.42.0` `a921f3f` | 3 × 150 asked; t1 ran 84, t0 and t2 about 95 | spot | 318,934,530 | 3,409,848 | t0 111.0M, t1 98.5M, t2 109.5M. **t1 was reclaimed by spot at 22:52 UTC**, 84 minutes in (its 3,960 complete shards kept). The run was **cancelled** at 23:05 to limit cost, at about 317M of the 500M asked for, with no top-up. About **19,570 positions/s an instance**, 11% below generation 4's: net6k4 is about 11% slower per node |

**Full audit** (`ci/datagen-full-audit.sh` on 16 cores, built at
`a921f3f`): **GO**.
1. **Inventory:** 0 P0 problems over 12,755 shards, the interrupted task
   included: no gaps, orphans, duplicate shards or shared seeds; one settings
   block, one engine, one generation, one net.
2. **Every record:** 0 failures over all 318,934,530 records, and
   bulletformat exact at 32 bytes a record.
3. **No P1 warnings** in any of the 65 batches.
4. **Determinism: 2,304 of 2,304 games regenerated identically** from their
   seeds (12 per worker, all 192 workers), with net6k4 named in every
   manifest and played by the build.

| Quality | Generation 3 | Generation 4 | Generation 5 |
|:--|--:|--:|--:|
| Results (W / D / B) | 30.3 / 46.4 / 23.3% | 29.7 / 47.2 / 23.1% | 29.2 / 47.7 / 23.1% |
| White–black gap | 7.0 points | 6.6 points | 6.1 points |
| Games ended by win adjudication | 53.2% | 52.5% | 52.0% |
| Duplicates within a batch (200 shards) | 1.76% | 1.71% | **0.98%** |
| Median game length (engine plies) | 112 | 109 | 108 |
| Records a game | 95.7 | 94.3 | 93.5 |
| \|score\| ≤ 100 | 31.8% | 32.4% | 34.1% |
| Endgame share | 47.1% | 46.0% | 45.6% |

**Compared with generation 4:**
- **draws up again, by less:** +0.5 points to 47.7% (generations 3 → 4:
  +0.8; 2 → 3: +3.4), still well inside the 55% bar;
- **within-batch repetition nearly halved,** 1.71% → 0.98%: the king-bucketed
  net's games vary more;
- more level positions (|score| ≤ 100 up 1.7 points), a smaller colour gap,
  slightly shorter games.

**Cost:** about $5–6 on spot (about 4.7 instance-hours), an estimate, not
billed figures.

## Relabelled label sets

A label set re-scores a generation's stored positions with a stronger net and
keeps the originals untouched ([relabel.md](relabel.md)). **No label set exists
yet.** Every net so far trained on the original labels.

| Run | Date | Commit | Asked | Market | Generation | Records | Notes |
|:--|:--|:--|:--|:--|:--|--:|:--|
| [41533675760](https://github.com/jwallace145/rusty-chess-v2/actions/runs/41533675760) | 2026-11-02 | `b7f12f7` | 1 × 10 minutes, 5,000 nodes, net7 | on demand, $3.4486/h, after 5 minutes without spot capacity | 0 | 0 | Smoke test. **Failed:** `relabel` panicked on the first worker, about 2 minutes after launch, on manifests that predate checksums (exit 134, fixed in #187). The log shows 0 shards and 0 records before the failure, so nothing should have been uploaded; S3 wasn't listed. Under $0.20, an estimate. Fixed binary run locally on 6 generation 0 shards (#187); no cloud run since |

## Adding a run or a generation

After every Datagen or Relabel run, including smoke tests, benchmarks and
failed or cancelled runs:

1. Add a row to the generation's table, with the run id, date, engine
   version and commit, instances × minutes asked and run, market, positions
   and games (summed from the manifests), and what happened (interruptions,
   capacity, cancellations).
2. When the generation is complete, run `ci/datagen-full-audit.sh`
   ([datagen-runbook.md](datagen-runbook.md)) and record its verdict, the
   determinism result and the quality numbers, including how they moved from
   the previous generation.
3. Update the Generations table, including which nets trained on it.
4. Take numbers from the manifests, `task.json` or `datagen-audit`, and label
   anything else, such as costs, as an estimate. The full rule is in
   [AGENTS.md](../AGENTS.md#experiments-and-the-research-record).

## Audit log

Checks of this ledger against its primary sources.

**2026-11-03,** at `v0.44.1`:

| Check | Result |
|:--|:--|
| Every `Datagen` run listed by GitHub (13) has an entry here | Two were missing, the #136 benchmark runs, now under generation 2 with the comparison #136 promised |
| Each generation's runs sum to its positions and games | Exact for all six |
| Shards of generations 0–5 against net7's `split.json` (94,149) and net6's (81,394 for generations 0–4) | Exact |
| Positions of each net's generations against its training + held-out records | Exact for nets 1, 2, 3, 5, 6 and 7 |
| Records a game in the quality tables, generations 1–5 | Match positions ÷ games |

**Not checked:** the shard manifests themselves. S3 needed AWS credentials this
audit didn't have, so the totals are checked against each other and against
`training.json`, not recounted. No audit verdict was re-run.
