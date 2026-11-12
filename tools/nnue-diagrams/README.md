# nnue-diagrams

Generates and checks the NNUE architecture gallery in
[`docs/nnue-architectures/`](../../docs/nnue-architectures/README.md).

```sh
npm run nnue:diagrams                 # write every .d2, .svg and the generated page blocks
npm run nnue:diagrams -- --net <id>   # one net's .d2 and .svg (the pages are always refreshed)
npm run nnue:diagrams:check           # render again in memory and compare; never writes
npm run nnue:diagrams:list            # the snapshots in training order
```

| File | Does |
|:--|:--|
| `manifest.mjs` | The manifest schema and its validation. Every topology field is required; nothing is defaulted. Derives the ledger's architecture notation, the parameter count and the file size the topology implies, which is checked against `file_bytes` when the manifest has one |
| `template.mjs` | A manifest to D2 source: the forward pass as modules, two perspective accumulators reading one shared weight set, the own-king view when K > 1, SCReLU, the concatenation, the one head the piece count selects from the bank of B, the scaling to centipawns |
| `render.mjs` | D2 source to SVG with `@terrastruct/d2`, D2's WebAssembly build, pinned to an exact version in `package.json`; the SVG is normalised so whitespace hooks can't make a fresh render look stale |
| `gallery.mjs` | The generated blocks of `README.md` and `evolution.md`, between `<!-- nnue-diagrams:<block>:start -->` and `end` markers; the prose around them is hand-written |
| `ledger.mjs` | Reads `docs/nets.md`'s Summary table and headings, the engine's `NETS` and `RETIRED_NETS`, and the embedded `.nnue` file |
| `cli.mjs` | `generate`, `check`, `list` |

`check` fails on: an invalid or duplicate manifest, a missing or stale `.d2`,
`.svg` or page, an orphan file, a net in the ledger's Summary table without a
snapshot (or the reverse), a `ledger_anchor` that matches no heading, a status
that disagrees with the engine's embedded and retired nets, a `file_bytes`
that disagrees with the embedded file, a broken relative link in the pages,
or a D2 package that is missing or not the pinned version. It never rewrites
a file.

**Reproducibility contract.** D2 runs as WebAssembly with ELK's JavaScript
layout, so the same manifest renders the same bytes on every machine and in
CI; CI's check re-renders every diagram and compares. The SVG embeds the
fonts it needs as data URIs and nothing in it depends on the date, the host
or a random seed. Changing the pinned D2 version is a regeneration of every
SVG in one PR.

Tests: `npm test` runs `*.test.mjs` here with the rest of the repository's
Node tests.
