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
- **Hidden layer:** H neurons per perspective: 128, 256, 512 or 768, read from
  the file's size (*Widths* below). The default net has 512 since `v0.36.0`.
  The two perspectives share their weights.
- **Activation:** squared clipped ReLU, `clamp(x, 0, QA)²`.
- **Output:** side-to-move perspective first, then the opponent's,
  concatenated into one weighted sum and scaled to centipawns.

A diagram of this network for every net trained so far, generated from a
manifest of each net's topology, is in
[nnue-architectures/](nnue-architectures/README.md).

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
  the pieces that moved, were captured or promoted, or castled. When a king
  move changes its side's view (king buckets, below), that side's accumulator
  comes from the refresh cache instead: the accumulator from the last time the
  king stood in that view, brought up to date with the pieces that moved since.
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

The engine reads nets of **128, 256, 512 or 768** hidden neurons per
perspective (`nnue::WIDTHS`), told apart by the file's size: 197,378, 394,754,
789,506 or 1,184,258 bytes, plus up to 63 of padding. With 8 output buckets
and 4 king buckets, a 768-neuron net is 4,744,720 bytes.
- **Compiled per width:** each width's code is compiled separately
  (`Net<H>`, `Acc<H>`), and chosen once per evaluation and per move
  (`NnueState`). So the 128-neuron code is the same as before 256 existed,
  and every 128 net's bench signature is unchanged.
- **Exactness:** the output sums each side in 32-bit runs of 256 neurons and
  adds the runs in 64 bits. With output weights within ±127, a run's sum is
  at most 255² × 127 × 256 = 2,114,092,800, inside 32 bits, so the result is
  exact at any width: the same as summing in 64 bits. At 128 and 256 there's
  one run a side, as before 512 existed. Tests check the extreme cases: at
  256 the two sides together pass 32 bits, and at 512 one side alone does. At
  768 there are three runs a side.

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
  Since the [refresh cache](#the-refresh-cache-for-king-bucket-rebuilds) the
  rebuild applies only the pieces that moved since the king last stood in that
  view, about 5.5 weight columns on the bench instead of one per piece.
- **Cost:** none measurable for single-view nets (the paired bench, 1.003 ±
  0.021). A 4-bucket net at 512 × 8 is 3,163,152 bytes, but a position only
  reads its own buckets' columns.

## The refresh cache for king-bucket rebuilds

**Implemented** as a `perf` change with the bench signature unchanged. The
measurements and the plan below were recorded before any code existed (#215)
and are unchanged; what was measured afterwards is at the end.

### What the profile says

Two local measurements at `v0.46.0` (`1cd5486`) on an Apple M3 Max, each in a
scratch build that isn't in the repository. Both used the bench positions.

**How often each accumulator operation runs** (temporary counters in
`NnueState`, `bench` at depth 10):

| Net | Pushes (`make_move`) | Evaluations | Pushes never evaluated | Rebuilds (a king changed its view) |
|:--|--:|--:|--:|--:|
| net7rl012, 512 | 933,625 | 811,605 | 122,020 (13.1%) | 78,672 (8.4% of pushes) |
| net7rl012w768 | 969,221 | 848,347 | 120,874 (12.5%) | 81,756 (8.4%) |

The search reads the static evaluation at every node that isn't in check
before any pruning, and quiescence takes stand-pat at every node not in check;
only transposition cutoffs, nodes in check and repetition draws skip it. So 87%
of pushed accumulators are evaluated, and deferring updates until evaluation
("lazy updates") could skip at most 13% of the update work.

**Where the time goes** (`sample`, 1 ms, `bench` at depth 15, with `update`,
`Net::refresh` and `Net::evaluate` marked `#[inline(never)]` so the sampler can
tell them apart; self time, samples with fewer than 5 omitted):

| Function | 512: samples | share | 768: samples | share |
|:--|--:|--:|--:|--:|
| `negamax` (with what's inlined into it) | 1,463 | 31.2% | 1,672 | 27.2% |
| NNUE `update` (incremental, both perspectives) | 719 | 15.3% | 1,157 | 18.8% |
| NNUE `evaluate` (the forward pass) | 548 | 11.7% | 912 | 14.8% |
| NNUE `refresh` (rebuild after a view change) | 379 | 8.1% | 698 | 11.3% |
| `generate_with` (move generation) | 461 | 9.8% | 467 | 7.6% |
| `quiescence`, `see`, `attackers_to`, make and unmake, TT store, the rest | 1,122 | 23.9% | 1,252 | 20.3% |
| **All NNUE** | **1,646** | **35.1%** | **2,767** | **44.9%** |
| Listed self samples | 4,692 | | 6,158 | |

The runs: 512 searched 16,140,428 nodes in 6,992 ms (433 ns a node); 768
searched 17,031,170 in 8,863 ms (520 ns a node). Splitting each by the shares
above gives nanoseconds a node:

| | 512 | 768 | 768 − 512 |
|:--|--:|--:|--:|
| `update` | 66 | 98 | +32 |
| `evaluate` | 51 | 77 | +26 |
| `refresh` | 35 | 59 | +24 |
| **NNUE** | **152** | **233** | **+81** |
| Everything else | 281 | 287 | +6 |
| **A node** | **433** | **520** | **+87** |

"Everything else" being equal in the two runs is the consistency check: the
width changed only the NNUE terms. A rebuild costs about 12–14 incremental
updates (435 ns against 35 at 512; 730 against 51 at 768), as expected from
the bias plus about 25 columns against 2–4.

**What it means for making 768 affordable.** The CI bench found net7rl012w768
13.9% ± 6.9% slower a node and 20.9% ± 9.4% longer to depth, and its SPRT
gained +28.9 ± 10.2 at fixed nodes but lost −9.0 ± 6.4 timed
([nets.md](nets.md#net7rl012w768-l050-r42411319602)). The three NNUE terms are
the only ones that grew. In order of what each could give back, per node at 768:

1. **Rebuilds, 59 ns.** 8.4% of pushes rebuild a perspective because the
   layout's boundaries (files d/e, ranks 1/2, f1/g1 after castling) are where
   kings step. A refresh cache makes a rebuild cost a few columns instead of
   about 25: an estimated 40–50 ns back.
2. **Updates, 98 ns.** Lazy updates can skip the 13% that are never
   evaluated: at most 13 ns back, before their bookkeeping.
3. **The forward pass, 77 ns.** Only SIMD or quantization work changes it,
   and nothing here is planned for it.

So the cache comes first. At 512 the rebuilds are 35 of 433 ns, 8.1% of the
search, so the same change should show on the default net too.

### The change

| | |
|:--|:--|
| Question | Does caching one accumulator per perspective and view make king-bucket rebuilds cheap enough to show in search speed, without changing a single evaluation? |
| The one change | `NnueState` gains a cache: for each perspective and each of the 8 views (4 buckets × mirrored or not), an `Acc<H>` and the per-piece bitboards it reflects, 12 `Bitboard`s. A view change brings the cached entry up to date by adding the columns of pieces present now but not then and subtracting those present then but not now, copies it to the stack, and records the new placement. Entries start as the bias with empty bitboards, so a view's first use is a full build, and no validity flag is needed |
| Held the same | The network, the weights, the quantization, the file format, every incremental update, the search, the stack, `set_root`, the stack's restart in long games, and the output. The cache reflects the board, not the game, so a new root or an unmade move never invalidates it |
| Exactness | Wrapping `i16` addition is associative and commutative, so adding and subtracting columns in any order from the cached entry gives exactly `refresh(pos)`. The bench signature must be unchanged: `976528` at `v0.46.0` |
| Cost | One cache per `Board`, allocated once in `NnueState::new`: 2 × 8 × `H` × 2 bytes plus 2 × 8 × 12 × 8 bytes, 24.6 KB at 768. Nothing allocates in the search. A new network gets a new `NnueState`, so no entry outlives its network |
| Kept or dropped by | **Kept** if the signature is unchanged and the CI bench comment's paired speed interval clears the noise bar (`paired_noise_pct`, 1%) on the default net, or its time to depth does. **Dropped** if neither clears it. The timed SPRT is run but doesn't decide: a few percent of speed is worth a few Elo, below what `elo1` 5 can confirm, and the fixed-node leg is zero by construction. The owner merges over the check for a speed change, with the reason stated, as the rules allow |

**Tests:**
- Every rebuild's result equals `Net::refresh` of the same position, in the
  existing random-game tests (which run through castling and king walks) and
  in games built to cross every boundary of the layout in both directions and
  return.
- A cached entry reused after other pieces moved: the diff path with
  additions, subtractions and both.
- **Negative control:** a deliberately corrupted cache entry changes the
  evaluation, so the tests exercise the diff path and don't pass through a
  fallback rebuild.
- All four widths, through the shapes the width tests already iterate.

**Measured and recorded:** the counters above again, so the share of rebuilds
that found a useful entry and the columns they applied are known; the local
alternating bench at 512 and 768 with `--eval-file`; the CI bench comment; and
the SPRT, with the result either way.

**Not in this change:** lazy updates, which are second in the list above and
need their own measurement after this one; a finer bucket layout, which is a
training change; and any SIMD work on the forward pass.

### Measured

**The implementation:** `Plies<H>` holds the stack and, per perspective, one
`CacheEntry<H>` per view: an `Acc<H>` and 12 `Bitboard`s, `[color][kind]`.
A rebuild walks the 12 piece sets, adds the columns of squares set now but not
in the entry and subtracts those set then but not now, records the new sets
and copies the entry to the stack. Entries start as the bias with no pieces.
The `stats` build counts rebuilds and the columns they applied, and `bench`
prints them.

**Exactness:** the bench signature is `976528` before and after, identical run
to run and with `--features stats`. Every rebuild equals `Net::refresh` of the
same position in the random-walk tests and in a scripted walk of both kings
across every boundary of the layout and back, with a capture between visits.
With the counters compiled in, that walk also checks the work: a first visit
to a view applies one column per piece (14, then 13 after the capture), and a
return applies only the pieces that moved (6, 4, 5 and 5). **Negative
control:** spoiling every cached accumulator by one makes the next return's
accumulator differ from a fresh recomputation, so the tests would catch a
rebuild that silently fell back to one.

**Rebuild work on the bench** (`--features stats`, depth 10):

| Net | Rebuilds | Columns applied | A rebuild | Before (one column per piece) |
|:--|--:|--:|--:|--:|
| net7rl012, 512 | 78,672 | 428,767 | 5.5 | about 25 |
| net7rl012w768 | 81,756 | 453,005 | 5.5 | about 25 |

**Speed, a local measurement** (Apple M3 Max, `bench --depth 12`, the base
`v0.46.1` and this change alternating, six runs each; the signatures were
equal in every run):

| | Base time | Cache time | Base nodes/s | Cache nodes/s | Paired nodes/s, cache ÷ base |
|:--|--:|--:|--:|--:|--:|
| net7rl012, 512 (the default) | 1,348 ms | 1,281 ms | 2.51M | 2.65M | **1.052 ± 0.006** |
| net7rl012w768 (`--eval-file`) | 1,645 ms | 1,517 ms | 2.06M | 2.23M | **1.085 ± 0.024** |

The plan estimated 40–50 ns a node back at 768 of the 87 ns gap; a 7.8%
shorter time is about 40 ns. Against the 512 default without the cache,
net7rl012w768 is now 11.3% slower a node here, from 18.2%. The CI bench
comment on the PR is the record on the shared runner.

## The embedded net, and the retired ones

The binary carries **only the default net**, in `nnue::NETS` (one entry),
read with `include_bytes!` from `crates/search/nets/<net id>.nnue`. It once
carried one net per generation; with the king-bucketed nets at 3.2 MB each,
that was 8.3 MB of a 9.2 MB binary at net7 (#180), and only the default is ever used in play.

Every earlier generation's net is **retired**: listed in `nnue::RETIRED_NETS`
with its id, generation and the SHA-256 of its file, and kept forever in the
dataset bucket as `data/nets/<net id>/net.nnue` beside its `training.json`
(`nnue::NET_STORE`).

| Net | Played generation | Trained on | Bench signature | |
|:--|--:|:--|--:|:--|
| `net1-l050-r41022503872` | 1 | generation 0 (231M positions) | 1201313 | retired |
| `net2x80-l050-r41131366975` | 2 | generations 0 and 1 (600M positions), 80 superbatches | 1146589 | retired |
| `net4w512-l050-r41170687764` (**512 hidden**) | 3 | generations 0–2 (906M positions), 80 superbatches | 1203006 | retired |
| `net5-l050-r41229287546` (512 hidden) | 4 | generations 0–3 (1.53B positions), 80 superbatches | 1101129 | retired |
| `net6k4-l050-r41282883885` (512 hidden, 8 output buckets, **4 king buckets**) | 5 | generations 0–4 (2.03B positions), 80 superbatches | 1046838 | retired |
| `net7rl012-l050-r42054816508` (512 hidden, 8 output buckets, 4 king buckets) | 6 | generations 0–5 (2.35B positions), generations 0–2 on label set `N7`, 80 superbatches | 976528 | retired: played generation 6 |
| `net8-l050-r42564788061` (512 hidden, 8 output buckets, 4 king buckets) | 7 | generations 0–6 (3.20B positions), generations 0–2 on label set `N7`, 80 superbatches | 970013 | **embedded, the default** |

- **Using a retired net:** download its file and pass the path
  (`setoption name EvalFile value <file>`, `bench --eval-file <file>`), and
  check its SHA-256 against the table. Asking for a retired net by id says
  exactly that, with the URL and the hash.
- **Re-searching a generation's data** (the audit's determinism step) needs
  the net that played it, and gets it by building the tool at the data's
  engine commit ([datagen-runbook.md](datagen-runbook.md)): that build embeds
  the net of its time.
- **A new default** replaces the `NETS` entry, and the net it replaces moves
  to `RETIRED_NETS` if it played a generation. A net replaced before playing
  any generation (net 2, net 3, net 4, net5b8, net6, net7) is in neither: it never
  played data, and its file stays in the bucket. The swap changes the bench
  signature, and is kept only on a timed SPRT win over the net it replaces.
- Each net's full history (data, parameters, validation curve, SPRT and
  ladder results) is in [nets.md](nets.md).

## Choosing the evaluation

- **UCI:** `setoption name EvalFile value` takes:
  - `/path/to/net.nnue` to load a net file;
  - the embedded net's id (`net8-l050-r42564788061`) for the default; a
    retired net's id names its file in the bucket instead;
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
with PeSTO. A profile at `v0.46.0` says what pays next: the king-bucket
rebuilds, then the incremental updates, then the forward pass
([the refresh cache plan](#the-refresh-cache-for-king-bucket-rebuilds)).
Updating the accumulators only for positions that are evaluated would skip
13% of the updates, because the search evaluates nearly every node.

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
