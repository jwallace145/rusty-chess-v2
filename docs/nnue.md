# NNUE evaluation

The engine evaluates with an efficiently updatable neural network, trained
on its own self-play games, instead of the hand-written PeSTO tables. The net
is embedded in the binary, so an install is still one file; PeSTO stays
available for comparisons. The data the nets are trained on is described in
[datagen.md](datagen.md), and the training in [training.md](training.md).

## Architecture

```text
768 inputs ──▶ H hidden (our perspective)  ─┐
768 inputs ──▶ H hidden (their perspective) ┴─▶ SCReLU ──▶ 1 output
```

- **Inputs:** one feature per colour, piece kind and square, seen from a
  perspective. "Our" pieces are features 0–383 and the opponent's 384–767,
  each block ordered pawn, knight, bishop, rook, queen, king with 64 squares
  each. Black's perspective flips the board vertically (`square ^ 56`). This
  is `bullet`'s `Chess768` input.
- **Hidden layer:** H neurons per perspective: 128, 256 or 512, read from
  the file's size (*Widths* below). The default net has 512 since `v0.36.0`.
  The two perspectives share their weights.
- **Activation:** squared clipped ReLU, `clamp(x, 0, QA)²`.
- **Output:** side-to-move perspective first, then the opponent's,
  concatenated into one weighted sum and scaled to centipawns.

## Quantization

| Constant | Value | Meaning |
|:--|--:|:--|
| `QA` | 255 | hidden-layer scale: activations are clipped to `0..=QA` |
| `QB` | 64 | output-weight scale |
| `SCALE` | 400 | centipawns per unit of the network's output |

```text
eval = (Σ screlu(ours[i]) · w[i] + Σ screlu(theirs[i]) · w[H + i]) / QA + bias
eval = eval · SCALE / (QA · QB)
```

The sum is accumulated in 64-bit integers. At these scales the trainer's
clipped weights would come within a few percent of overflowing `i32`.

## File format

All values are `i16`, little-endian, in this order. A trailing pad to a
multiple of 64 bytes is allowed. Anything else is refused with the expected
size.

| Block | Count |
|:--|--:|
| feature weights, feature-major (`[768 × K][HIDDEN]`, king bucket 0's first) | 98,304 × K |
| feature bias | 128 |
| output weights, per bucket (our perspective, then theirs) | 256 × B |
| output biases | B |

That is 197,378 bytes at `HIDDEN = 128` with one bucket (B = 1). A bucketed
net (B = 8, *Output buckets* below) stores bucket 0's output weights first,
then bucket 1's, and so on, then the eight biases. It is the layout `bullet`'s simple
example saves for this architecture. **Before the first trained net is
used,** check a handful of positions against the trainer's own evaluation of
them. A mismatch means the layout or a scale is off, not that the net is
weak.

## How the engine keeps it up to date

`Board` keeps a stack of accumulators, one per ply: the hidden layer before
activation, for both perspectives.

- **`make_move`** pushes a copy and adds or subtracts the weight columns of
  the pieces that moved, were captured or promoted, or castled.
- **`unmake_move`** pops.
- **`set_root`** (the start of every search) moves the current entry to the
  bottom, so a search can go `MAX_PLY` deep whatever the game's length.
- **A long game played forward,** move by move without unmaking (UCI
  `position … moves`, self-play), restarts the stack from the bottom when it
  fills.

Nothing allocates during a search. Tests check the accumulators against a
full recomputation at every ply of random games, through castling, en
passant, promotions and captures, made and unmade.

## Widths

The engine reads nets of **128, 256 or 512** hidden neurons per perspective
(`nnue::WIDTHS`), told apart by the file's size: 197,378, 394,754 or 789,506
bytes, plus up to 63 of padding.
- **Compiled per width:** each width's code is compiled separately
  (`Net<H>`, `Acc<H>`), and chosen once per evaluation and per move
  (`NnueState`). So the 128-neuron code is the same as before 256 existed,
  and every 128 net's bench signature is unchanged.
- **Exactness:** the output sums each side in 32-bit runs of 256 neurons and
  adds the runs in 64 bits. With output weights within ±127, a run's sum is
  at most 255² × 127 × 256 = 2,114,092,800, inside 32 bits, so the result is
  exact at any width: the same as summing in 64 bits. At 128 and 256 there's
  one run a side, as before 512 existed. Tests check the extreme cases: at
  256 the two sides together pass 32 bits, and at 512 one side alone does.

## Output buckets

A net can have **8 output layers** instead of one, chosen per position by
the number of pieces on the board (kings and pawns included):
`(pieces − 2) / 4`, so 2–5 pieces is bucket 0 and 30–32 bucket 7. This is
`bullet`'s `MaterialCount<8>`. The hidden layer is shared; only the final
weighting changes, so endgames, middlegames and openings each get their own.

- **Read from the size:** `nnue::OUTPUT_BUCKETS` is 1 or 8, told apart by
  file size like the widths. The six shapes are thousands of bytes apart:

  | Width | 1 bucket | 8 buckets |
  |--:|--:|--:|
  | 128 | 197,378 | 200,976 |
  | 256 | 394,754 | 401,936 |
  | 512 | 789,506 | 803,856 |

- **Cost:** one evaluation still computes one output: the bucket's weights
  over the same 2 × H activations, plus a population count of the board.
  The extra weights are 7 × 2H + 7 values (about 14 KB at 512).
- **Single-bucket nets are unchanged:** they always use bucket 0, so every
  embedded net's bench signature is the same as before buckets existed.
- **Exactness:** each bucket's sum is the same computation as a
  single-bucket net's, so the bound above holds in every bucket. Tests check
  the extremes and random accumulators in every bucket of every shape.

## King buckets

A net can index its inputs by **where each side's own king stands**: 4 king
buckets, each with its own 768 input columns, and the board mirrored left to
right when that king is on files e–h. It is how an NNUE learns king safety
without hand-made features. The design, the bucket layout and the training
are in [king-buckets.md](king-buckets.md).

- **Read from the size:** `nnue::KING_BUCKETS` is 1 or 4, told apart by file
  size like the widths and output buckets. A 4 always means
  `KING_BUCKET_LAYOUT`: a1–b1 → 0, c1–d1 → 1, rank 2 → 2, ranks 3–8 → 3, from
  each side's own point of view, files e–h mirrored.
- **Indexing:** `768 × bucket + (the 768-feature index XOR 7 when mirrored)`,
  bullet's `ChessBucketsMirrored`. One king bucket is bucket 0 unmirrored, so
  every single-view net evaluates exactly as before.
- **The accumulator:** only a side's own king moving can change its view. A
  king move that changes it rebuilds that side's accumulator from the
  position after the move; every other move is the usual incremental update.
  There is no refresh cache yet.
- **Cost:** none measurable for single-view nets (the paired bench, 1.003 ±
  0.021). A 4-bucket net at 512 × 8 is 3,163,152 bytes, but a position only
  reads its own buckets' columns.

## The embedded nets

The binary carries one net for each generation that has data or is about to
be played, so the data each one played can still be re-searched and any of
them can be selected by id. A net that replaces another before playing any
generation replaces its entry: net 2 (`net2-l050-r41110004652`, `v0.30.0`)
gave generation 2's place to net2x80 before any generation-2 data existed, and
net 3 (`net3-l050-r41160495666`, `v0.33.0`) gave generation 3's to net 4, and
net 4 (`net4-l050-r41165115607`, `v0.35.0`) gave it to net4w512, net5b8
(`net5b8-l050-r41265636815`, `v0.39.0`) gave generation 5's to net6, and net6
(`net6-l050-r41273387683`, `v0.40.0`) gave it to net6k4. The
newest is the default. Each net's full history (data, parameters, validation
curve, SPRT and ladder results) is in [nets.md](nets.md).

| Net | Plays generation | Trained on | Bench signature |
|:--|--:|:--|--:|
| `net1-l050-r41022503872` | 1 | generation 0 (231M positions) | 1201313 |
| `net2x80-l050-r41131366975` | 2 | generations 0 and 1 (600M positions), 80 superbatches | 1146589 |
| `net4w512-l050-r41170687764` (**512 hidden**) | 3 | generations 0–2 (906M positions), 80 superbatches | 1203006 |
| `net5-l050-r41229287546` (512 hidden) | 4 | generations 0–3 (1.53B positions), 80 superbatches | 1101129 |
| `net6k4-l050-r41282883885` (default, 512 hidden, 8 output buckets, **4 king buckets**) | 5 | generations 0–4 (2.03B positions), 80 superbatches | 1046838 |

Each file is `crates/search/nets/<net id>.nnue`, listed in `nnue::NETS` with
its generation and read with `include_bytes!`. Its record (`training.json`,
the losses, the split, the validation curve) is in the dataset bucket under
`data/nets/<net id>/`, and its SHA-256 there matches the embedded file.

A new default net is appended to `NETS`, which changes the default, its
generation and the bench signature, and is kept only on a timed SPRT win
over the net it replaces.

## Choosing the evaluation

- **UCI:** `setoption name EvalFile value` takes:
  - `/path/to/net.nnue` to load a net file;
  - an embedded net's id (`net1-l050-r41022503872`) for that net;
  - `pesto` for PeSTO;
  - `<default>`, or an empty value, to return to the default net.

  A file of the wrong size is reported in an `info string` and ignored.
- **Bench:** `rusty-chess bench` uses the default net, and its signature is
  the net's. `--eval-file` takes a file, an embedded net's id or `pesto`.
  PeSTO's signature is unchanged since `v0.24.0` (1697503).
- **Self-play:** `datagen.toml`'s `generation` picks the player:
  - generation 0 is PeSTO;
  - generation N is the embedded net that plays it;
  - `datagen` refuses any other generation;
  - `datagen-audit` re-searches a dataset with the evaluation its manifests
    name.

## Speed

Both hot paths are plain Rust written in a shape the compiler vectorizes;
there is no `unsafe` and no per-architecture code.

- **The output** is summed in `i32` as `(c · w) · c`, with `c` the clipped
  hidden value. On `x86-64-v3` this compiles to 16-bit multiplies and
  pairwise multiply-adds (`vpmullw`, `vpmaddwd`), 16 values an instruction,
  where the first version's `i64` sum used 64-bit lanes (4 an instruction).
  It is exact, so the bench signature is the same:
  - the loader refuses output weights beyond ±127 (the trainer's clip at
    `QB = 64`);
  - with that bound, `c · w` fits an `i16` and the whole sum an `i32`;
  - a test checks it against the `i64` form at the extremes.
- **The accumulators** of the next ply are written from the current ones in
  one pass per perspective (`add_sub`, `add_sub_sub`, `add_add_sub_sub`),
  instead of a copy followed by one pass per moved piece.

| Bench (M-series, five alternating runs) | Time | Nodes/second |
|:--|--:|--:|
| Scalar `i64` output, separate updates (`v0.28.0`) | 534 ms | 2.25M |
| `i32` output | 425 ms | 2.83M |
| `i32` output and fused updates | 410 ms | 2.93M |
| PeSTO, for reference (its own bench tree) | 544–585 ms | 2.9–3.1M |

With net 1 the engine now searches about as many nodes a second as it did
with PeSTO. Next, if profiles say it pays: updating the accumulators only
for positions that are evaluated, and larger nets (where the output and
update costs grow with the hidden layer).

## Next

1. **A pilot net:** the trainer, the preparation and the trainer-to-engine
   check are in [training.md](training.md). The first pilot nets matched the
   engine exactly, apart from quantization error (mean 9 cp).
2. **Net 1, embedded and the default**, kept on a timed SPRT win over
   PeSTO (+587 ± 83 at 5+0.05); `datagen` played generation 1 with it.
3. **Net 2**, trained on generations 0 and 1, the default if it wins its
   timed SPRT over net 1; `datagen` then plays generation 2 with it.
4. **Next:** a longer schedule (net 2's held-out loss was still falling at
   superbatch 40), the λ experiment, and a wider hidden layer, each measured
   against the net before it ([nets.md](nets.md)).
