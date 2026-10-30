# Self-play training data

The NNUE nets are trained on the engine's own games. Generation 0 is played
with the hand-written PeSTO evaluation; every later generation is played by the
engine carrying the previous net, so each net's data comes from a stronger
player than the last (the decision and its reasons are in
[roadmap.md](roadmap.md), Phase 6). This page specifies how the data is made,
how it is laid out in S3, and what every file contains, so that any
generation can be found, filtered, re-labelled and trained on years later
without guessing.

## How positions are made

`datagen` (in `crates/tools`) runs one worker per core. Each worker:

1. plays `random_plies` random legal moves from the start position (or two
   more, at random; always an even number, so both colours make as many
   random moves), and replays the
   opening if it ends in check, in a finished game, or with the search scoring
   it beyond `max_opening_cp`;
2. plays the game with the engine's own search at `nodes` per move, both
   sides, the transposition table and histories cleared at the start of each
   game;
3. keeps a position when it is not in check, the best move is quiet (not a
   capture or promotion, whose score is about the exchange rather than the
   position), and the score is not a mate score; the label is the search's
   score at that position;
4. ends the game on mate, stalemate, a repetition, fifty moves, insufficient
   material, win adjudication (`win_plies` consecutive plies at or beyond
   `win_cp` for the same side), draw adjudication (from `draw_after_ply`,
   `draw_plies` consecutive plies within `draw_cp`) or `max_plies`; and
5. writes the game's positions with its result once the game is over.

Every setting is in [`datagen.toml`](../datagen.toml), and every shard's
manifest repeats the values it was made with. A worker's games are
deterministic for its seed: the run's seed mixed with the worker's index.

## Shards

A shard is about `shard_positions` positions (whole games) from one worker,
written as three files that share a name, `<task>-w<worker>-s<seq>`:

| File | Contents |
|:--|:--|
| `<shard>.positions.txt` | One training record per line (below). |
| `<shard>.games.jsonl` | One game per line, in full (below), so positions can be re-labelled with a stronger engine or re-filtered without replaying. |
| `<shard>.json` | The manifest (below). |

Each file is written to a temporary name and renamed into place, the manifest
last: a manifest's presence means its shard is complete. The uploader works
the same way in S3, uploading the manifest after the two data files. A spot
interruption therefore loses at most the shard each worker has in progress
(about a minute of play at the committed settings), never a corrupted file.

### Training record (`positions`)

```text
<fen> | <score> | <result>
rnbqk1nr/ppp2pb1/3pp1p1/7p/2P4P/N2P4/PPQ1PPP1/R1B1KBNR w KQkq - 2 6 | -51 | 0.5
```

- `fen`: the full FEN, half-move and full-move counters included.
- `score`: the search's score in centipawns from **white's** point of view.
- `result`: the game's result from **white's** point of view: `1.0`, `0.5`
  or `0.0`.

This is the plain-text layout `bullet` and most NNUE trainers read or convert
from; the conversion to a trainer's binary format, and the blend of score and
result (the WDL weight), are training-time choices recorded with each net, not
properties of the data. Check the converter against a few records before the
first training run.

### Game record (`games`)

```json
{"start_fen": "...", "moves": ["f8g7", "e2e4", ...], "scores": [-41, -51, ...],
 "result": 0.5, "termination": "win-adjudication"}
```

- `start_fen`: the position after the random plies.
- `moves`: every move the engine played from there, in UCI notation.
- `scores`: the search score of each move, from the **side to move's** point
  of view, mate scores as the engine reports them (beyond ±29,872).
- `result`: from white's point of view.
- `termination`: `checkmate`, `stalemate`, `repetition`, `fifty-moves`,
  `insufficient-material`, `win-adjudication`, `draw-adjudication` or
  `max-plies`.

### Manifest

```json
{
  "schema": "rusty-chess-selfplay/1",
  "shard": "t0-w017-s00042",
  "worker": 17, "seq": 42, "seed": 1234567,
  "provenance": {"run": "...", "task": "t0", "engine_version": "0.23.0",
                 "engine_sha": "86e9e97", "engine_bench": 1529684},
  "settings": {"generation": 0, "nodes": 5000, "random_plies": 8, ...},
  "games": 256, "positions": 24330, "results": [88, 83, 85],
  "terminations": {"win-adjudication": 169, "repetition": 51, ...},
  "seconds": 60.0,
  "files": ["t0-w017-s00042.positions.txt", "t0-w017-s00042.games.jsonl"],
  "sha256": ["<hex of the positions file>", "<hex of the games file>"]
}
```

`results` counts white wins, draws and black wins. `sha256` is the SHA-256 of
each record file's *uncompressed* contents, in `files` order; shards written
before it was added have no `sha256` (gzip's own CRC still catches corruption). In S3 the two data files
are gzip-compressed and carry a `.gz` suffix; the manifest names them without
it.

## Layout in S3

The bucket comes from the `dataset` component of
`jwall-engineering-sites.yaml`. Keys are partitioned Hive-style, so a training
job (or Athena) can select by prefix:

```text
s3://<dataset bucket>/selfplay/v1/                    schema version 1
  gen=0/                                              the net generation that played
    run=20261030-<github run id>-<attempt>/           one Datagen workflow run (a re-run is a new attempt)
      task=t0/                                        one instance
        positions/t0-w000-s00000.txt.gz
        games/t0-w000-s00000.jsonl.gz
        manifests/t0-w000-s00000.json
        task.json                                     the task's totals, written when it ends
      task=t1/
        ...
```

- **One generation, all runs:** `selfplay/v1/gen=0/`.
- **What a run contains, without downloading data:** list
  `run=.../task=*/manifests/` and sum `positions`.
- **A training set:** every `positions/*.txt.gz` under the chosen generations
  and runs, filtered by manifest (engine commit, settings) where it matters.
- **Re-labelling:** read `games/`, replay each game from `start_fen`, and score
  the positions with a newer engine; write the result as a new `v` or a new
  partition, never over the original.

A `task.json` is absent when spot interrupted the instance; its completed
shards are still valid, and nothing needs cleaning up. Data is never
overwritten: a new run, and a re-run of an old one, always gets a new `run=`
partition and new seeds. After an interruption, start another run to make up
the time.

### Size

Measured on the pilot's shards, a stored position costs about 18 bytes:
11.8 for the gzipped training record, 5.7 for its share of the gzipped game
records, 0.2 for manifests. 300 million positions are about 5.3 GB, a
billion about 18 GB: a few dollars a year in S3. Converted to a trainer's
binary format (about 32 bytes a position, uncompressed) they need more room
on the training machine than in S3.

## Running it

The `Datagen` workflow runs `ci/datagen-task.sh` on one or more dedicated spot
instances (`instances`, at most 3 at once within the task vCPU budget) for a
set number of `minutes`, every task with its own seed. Each instance builds
`datagen` at the workflow's commit, generates on every core, compresses and
uploads each shard as it completes, and on a spot interruption stops the
workers (which finish their games and save their partial shards), uploads
everything complete and exits. At the committed settings one 64-core instance
makes about 100 million positions an hour.

**[datagen-runbook.md](datagen-runbook.md)** is the procedure for a run: settings
review, smoke test, monitoring, interruptions, and the audit
(`datagen-audit check`) every dataset passes before training, with its
correctness checks, quality bars and generation 0's reference results.

Locally, for a pilot:

```sh
cargo run --release -p rusty-chess-tools --bin datagen -- \
  --config datagen.toml --out /tmp/datagen --run pilot --task local \
  --threads 8 --max-minutes 1
```
