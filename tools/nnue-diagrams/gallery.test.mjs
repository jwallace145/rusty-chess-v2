import assert from "node:assert/strict";
import { test } from "node:test";

import { altText, applyBlocks, blocksFor, evolutionBlock, galleryBlock, relativeLinks, timelineBlock } from "./gallery.mjs";
import { headingSlug, headingSlugs, summaryNetIds, engineNets } from "./ledger.mjs";
import { validateManifest } from "./manifest.mjs";
import { manifest } from "./manifest.test.mjs";

const nets = [
  validateManifest(manifest({ net_id: "net6-l050-r41273387683", status: "superseded", default_releases: "v0.40.0–v0.41.0", generation: null, change: { from: null, kind: "first", summary: "first" }, ledger_anchor: "net6-l050-r41273387683" }, { king_buckets: 1 })),
  validateManifest(manifest({ status: "default", default_releases: "v0.42.0" })),
];

test("blocks are replaced between their markers and nowhere else", () => {
  const page = "intro\n<!-- nnue-diagrams:a:start -->\nold\n<!-- nnue-diagrams:a:end -->\noutro\n";
  assert.equal(applyBlocks(page, { a: "new" }), "intro\n<!-- nnue-diagrams:a:start -->\nnew\n<!-- nnue-diagrams:a:end -->\noutro\n");
  assert.throws(() => applyBlocks("no markers", { a: "x" }), /block is missing/);
  assert.throws(() => applyBlocks(`${page}${page}`, { a: "x" }), /more than once/);
});

test("the timeline and gallery link every snapshot and its ledger section", () => {
  const timeline = timelineBlock(nets);
  const gallery = galleryBlock(nets);
  for (const m of nets) {
    assert.ok(timeline.includes(`diagrams/${m.net_id}.svg`));
    assert.ok(timeline.includes(`manifests/${m.net_id}.json`));
    assert.ok(timeline.includes(`../nets.md#${m.net_id}`));
    assert.ok(gallery.includes(`<img src="diagrams/${m.net_id}.svg"`));
  }
  assert.match(timeline, /\| 1 \| `net6-l050-r41273387683` \| `\(768 → 512\) × 2 → 8` \| first \| Superseded: Default v0\.42\.0/);
  assert.match(timelineBlock([validateManifest(manifest({ status: "dropped", default_releases: null, status_note: "Dropped (#1): lost", change: { from: null, kind: "first", summary: "x" } }))]), /\| x \| Dropped \(#1\): lost \|/);
  assert.match(timeline, /\*\*Topology\*\*: 4 king buckets \(from `net6`\)/);
  assert.match(timeline, /#175 vs net6: \+21\.7 ± 9\.0 timed, \+40\.0 ± 12\.4 at fixed nodes \(kept\)/);
});

test("the evolution table has the first net and every topology change, with before and after", () => {
  const e = evolutionBlock(nets);
  assert.match(e, /\| 1 \| — \| \[`net6`\]\(diagrams\/net6-l050-r41273387683\.svg\)/);
  assert.match(e, /\| 2 \| \[`net6`\]\(diagrams\/net6-l050-r41273387683\.svg\)<br>`\(768 → 512\) × 2 → 8` \| \[`net6k4`\]\(diagrams\/net6k4-l050-r41282883885\.svg\)<br>`\(768 × 4 → 512\) × 2 → 8` \| 4 king buckets \|/);
  const dataOnly = validateManifest(manifest({ net_id: "net7-l050-r41363264869", status: "superseded", default_releases: "v0.43.0", change: { from: "net6k4-l050-r41282883885", kind: "data", summary: "gen 5" } }));
  assert.equal(evolutionBlock([...nets, dataOnly]), e);
});

test("alt text says the topology in words", () => {
  assert.equal(
    altText(nets[1]),
    "Architecture of net6k4-l050-r41282883885: (768 × 4 → 512) × 2 → 8. Chess768 features, 4 mirrored king buckets select each perspective's input weights, two perspective accumulators of 512 sharing one weight set, SCReLU, 1024 activations into one of 8 output heads chosen by piece count, scaled to centipawns.",
  );
  assert.match(altText(nets[0]), /one input view \(no king buckets\)/);
  assert.match(altText(manifest({}, { output_buckets: 1 }) && validateManifest(manifest({}, { output_buckets: 1 }))), /a single output head/);
});

test("blocksFor knows each page's blocks and relative links are extracted", () => {
  assert.deepEqual(Object.keys(blocksFor("README.md", nets)), ["current", "timeline", "gallery"]);
  assert.deepEqual(Object.keys(blocksFor("evolution.md", nets)), ["evolution"]);
  assert.deepEqual(relativeLinks('[a](x.md#y) <img src="d/e.svg"> [b](https://x) [c](#top) <a href="../n.md">'), ["x.md#y", "d/e.svg", "../n.md"]);
});

test("the ledger's summary ids, heading slugs and the engine's net lists are read", () => {
  const md = "# NNUE nets\n\n## Summary\n\n| Net | Status |\n|:--|:--|\n| `net1-l050-r1` | x |\n| `net2-l050-r2` | y |\n\n## net1-l050-r1\n\n## The λ sweep: net2x80 at λ 0.25 and 0.75\n";
  assert.deepEqual(summaryNetIds(md), ["net1-l050-r1", "net2-l050-r2"]);
  assert.equal(headingSlug("The λ sweep: net2x80 at λ 0.25 and 0.75"), "the-λ-sweep-net2x80-at-λ-025-and-075");
  assert.ok(headingSlugs(md).has("net1-l050-r1"));
  assert.throws(() => summaryNetIds("# nothing"), /no `## Summary`/);
  const rs = 'pub const NETS: [EmbeddedNet; 1] = [EmbeddedNet {\n    id: "net8-l050-r4",\n}];\npub const RETIRED_NETS: [RetiredNet; 2] = [\n    RetiredNet { id: "net1-l050-r1", generation: 1, sha256: "a" },\n    RetiredNet { id: "net2-l050-r2", generation: 2, sha256: "b" },\n];\n';
  assert.deepEqual(engineNets(rs), { embedded: ["net8-l050-r4"], retired: ["net1-l050-r1", "net2-l050-r2"] });
});
