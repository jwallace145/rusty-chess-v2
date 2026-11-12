# How the architecture evolved

The topology changes among the nets in [README.md](README.md), each with the
net before and after it and what the SPRT measured. Every other step in the
ledger changed the data, the schedule, λ or the labels while the network
stayed the same; those are in the README's table and in [nets.md](../nets.md).

The Elo figures are the PR's SPRT comment as [nets.md](../nets.md) records
them: self-play against the previous net at 5+0.05 and at 200,000 nodes a
move, so they overstate gains against other engines. The gate changed over
time: through #180 only the timed leg decided; since #184 both legs must
accept H1.

<!-- nnue-diagrams:evolution:start -->
| Step | Before | After | What changed | Measured (SPRT, both legs) | Record |
|--:|:--|:--|:--|:--|:--|
| 1 | — | [`net1`](diagrams/net1-l050-r41022503872.svg)<br>`(768 → 128) × 2 → 1` | The first net: bullet's reference (768 → 128) × 2 → 1 SCReLU network, trained on generation 0 (PeSTO's self-play) | +587.3 ± 82.7 timed, +563.4 ± 93.8 at fixed nodes vs PeSTO (#122, kept) | [ledger](../nets.md#net1-l050-r41022503872) · [manifest](manifests/net1-l050-r41022503872.json) |
| 2 | [`net3`](diagrams/net3-l050-r41160495666.svg)<br>`(768 → 128) × 2 → 1` | [`net4`](diagrams/net4-l050-r41165115607.svg)<br>`(768 → 256) × 2 → 1` | Hidden width 256 instead of 128: the first architecture change | +74.2 ± 16.4 timed, +88.1 ± 16.4 at fixed nodes vs net3 (#150, kept) | [ledger](../nets.md#net4-l050-r41165115607) · [manifest](manifests/net4-l050-r41165115607.json) |
| 3 | [`net4`](diagrams/net4-l050-r41165115607.svg)<br>`(768 → 256) × 2 → 1` | [`net4w512`](diagrams/net4w512-l050-r41170687764.svg)<br>`(768 → 512) × 2 → 1` | Hidden width 512 instead of 256 | +44.3 ± 13.1 timed, +73.6 ± 15.9 at fixed nodes vs net4 (#152, kept) | [ledger](../nets.md#net4w512-l050-r41170687764) · [manifest](manifests/net4w512-l050-r41170687764.json) |
| 4 | [`net5`](diagrams/net5-l050-r41229287546.svg)<br>`(768 → 512) × 2 → 1` | [`net5b8`](diagrams/net5b8-l050-r41265636815.svg)<br>`(768 → 512) × 2 → 8` | 8 output heads, one chosen per position by its piece count, instead of a single head | +6.5 ± 4.4 timed, +12.4 ± 6.6 at fixed nodes vs net5 (#164, kept) | [ledger](../nets.md#net5b8-l050-r41265636815) · [manifest](manifests/net5b8-l050-r41265636815.json) |
| 5 | [`net6`](diagrams/net6-l050-r41273387683.svg)<br>`(768 → 512) × 2 → 8` | [`net6k4`](diagrams/net6k4-l050-r41282883885.svg)<br>`(768 × 4 → 512) × 2 → 8` | 4 mirrored king buckets select each perspective's input weights (trained with a factoriser, merged into the file) instead of one view | +21.7 ± 9.0 timed, +40.0 ± 12.4 at fixed nodes vs net6 (#175, kept) | [ledger](../nets.md#net6k4-l050-r41282883885) · [manifest](manifests/net6k4-l050-r41282883885.json) |
| 6 | [`net7rl012`](diagrams/net7rl012-l050-r42054816508.svg)<br>`(768 × 4 → 512) × 2 → 8` | [`net7rl012w768`](diagrams/net7rl012w768-l050-r42411319602.svg)<br>`(768 × 4 → 768) × 2 → 8` | Hidden width 768 instead of 512, on net7rl012's data and labels | −9.0 ± 6.4 (H0) timed, +28.9 ± 10.2 (H1) at fixed nodes vs net7rl012 (#214, dropped) | [ledger](../nets.md#net7rl012w768-l050-r42411319602) · [manifest](manifests/net7rl012w768-l050-r42411319602.json) |
<!-- nnue-diagrams:evolution:end -->

## What each change was

- **Width, 128 → 256 → 512 (net 4, net4w512).** The hidden layer per
  perspective grew twice on the same data. Both steps won their timed SPRT,
  the wider net costing 11–18% speed a node, which is why 512 kept less of
  its fixed-node gain at a clock. [nets.md: What we've learned](../nets.md#what-weve-learned).
- **Output buckets, 1 → 8 heads (net5b8).** Eight output layers over the
  same hidden layer, one chosen by the piece count `(pieces − 2) / 4`, so
  endgames, middlegames and openings each weight the activations their own
  way. A single evaluation still computes one head. [nnue.md: Output buckets](../nnue.md#output-buckets).
- **King buckets, 1 → 4 views (net6k4).** Each side's 768 input features get
  four sets of weights, chosen by where its own king stands, with the board
  mirrored when that king is on files e–h. Only a side's own king moving can
  change its view, which is when the engine rebuilds that side's accumulator.
  The factoriser used in training is merged into the saved weights, so the
  file and the diagram show plain `768 × 4 → H`. [king-buckets.md](../king-buckets.md).
- **Width, 512 → 768 (net7rl012w768, dropped).** The one topology change
  that did not survive: clearly smarter per node (+28.9 ± 10.2 at fixed
  nodes) and clearly slower (about 14% a node), it lost the timed leg. The
  refresh cache that followed made the wide path cheaper, and a re-test is
  in the ledger's next experiments. [nets.md](../nets.md#net7rl012w768-l050-r42411319602).

What is not an architecture change, and so not a row above: adding a
generation of self-play data, a longer schedule, λ, and relabelling. Each of
those produced a new net with its own snapshot in the gallery, under the
same picture as the net before it.
