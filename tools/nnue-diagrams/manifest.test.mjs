import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { ManifestError, fileBytes, loadManifests, paddedFileBytes, parseNetId, signature, validateManifest } from "./manifest.mjs";

/** A valid manifest; `patch` overrides any top-level or topology field. */
export function manifest(patch = {}, topology = {}) {
  const K = topology.king_buckets ?? 4;
  const B = topology.output_buckets ?? 8;
  return {
    schema: "rusty-chess-nnue-diagram/1",
    net_id: "net6k4-l050-r41282883885",
    topology: {
      feature_set: "Chess768",
      features_per_perspective: 768,
      perspectives: 2,
      shared_feature_transformer: true,
      king_buckets: K,
      king_bucket_layout: K > 1 ? "KING_BUCKET_LAYOUT, files e–h mirrored" : null,
      hidden_width: 512,
      activation: "SCReLU",
      output_buckets: B,
      output_bucket_rule: B > 1 ? "(pieces − 2) / 4" : null,
      quantization: { qa: 255, qb: 64, scale: 400 },
      ...topology,
    },
    file_bytes: null,
    status: "retired",
    status_note: "Default v0.42.0–v0.42.1; played generation 5",
    generation: 5,
    default_releases: "v0.42.0–v0.42.1",
    training: { generations: "0–4", labels: "O", lambda: 0.5, superbatches: 80 },
    change: { from: "net6-l050-r41273387683", kind: "topology", summary: "4 king buckets" },
    experiment: { pr: 175, baseline: "net6-l050-r41273387683", timed: "+21.7 ± 9.0", fixed_nodes: "+40.0 ± 12.4", verdict: "kept" },
    ledger_anchor: "net6k4-l050-r41282883885",
    provenance: { architecture_source: ["docs/nets.md"], training_json: null, notes: null },
    ...patch,
  };
}

const rejects = (m, pattern) => assert.throws(() => validateManifest(m, "m.json"), pattern);

test("the engine's file sizes follow from the topology", () => {
  // crates/search/src/nnue.rs: net_file_bytes, and docs/nnue.md's tables.
  const shape = (H, K, B) => manifest({}, { hidden_width: H, king_buckets: K, output_buckets: B }).topology;
  assert.equal(fileBytes(shape(128, 1, 1)), 197_378);
  assert.equal(fileBytes(shape(256, 1, 1)), 394_754);
  assert.equal(fileBytes(shape(512, 1, 1)), 789_506);
  assert.equal(fileBytes(shape(768, 1, 1)), 1_184_258);
  assert.equal(fileBytes(shape(512, 1, 8)), 803_856);
  assert.equal(fileBytes(shape(512, 4, 8)), 3_163_152);
  assert.equal(fileBytes(shape(768, 4, 8)), 4_744_720);
  // The ledger's recorded sizes are bullet's padded files.
  assert.equal(paddedFileBytes(shape(512, 4, 8)), 3_163_200);
  assert.equal(paddedFileBytes(shape(768, 4, 8)), 4_744_768);
  assert.equal(paddedFileBytes(shape(512, 1, 8)), 803_904);
  assert.equal(paddedFileBytes(shape(512, 1, 1)), 789_568);
  assert.equal(paddedFileBytes(shape(256, 1, 1)), 394_816);
});

test("the signature is the ledger's notation", () => {
  const shape = (H, K, B) => manifest({}, { hidden_width: H, king_buckets: K, output_buckets: B }).topology;
  assert.equal(signature(shape(128, 1, 1)), "(768 → 128) × 2 → 1");
  assert.equal(signature(shape(512, 1, 8)), "(768 → 512) × 2 → 8");
  assert.equal(signature(shape(512, 4, 8)), "(768 × 4 → 512) × 2 → 8");
  assert.equal(signature(shape(768, 4, 8)), "(768 × 4 → 768) × 2 → 8");
});

test("net ids parse into name, λ and run", () => {
  assert.deepEqual(parseNetId("net2x80-l025-r41141703294"), { name: "net2x80", lambda: 0.25, run: 41141703294n });
  assert.equal(parseNetId("net8"), null);
  assert.equal(parseNetId("Net8-l050-r1"), null);
});

test("a complete manifest validates and derives its fields", () => {
  const m = validateManifest(manifest({ file_bytes: 3_163_200 }));
  assert.equal(m.signature, "(768 × 4 → 512) × 2 → 8");
  assert.equal(m.featureTag, "H512 · K4 · B8");
  assert.equal(m.parameters, 1_581_576);
  assert.equal(m.expectedFileBytes, 3_163_200);
});

test("missing and unknown fields fail loudly", () => {
  const m = manifest();
  delete m.topology.king_buckets;
  rejects(m, /topology is missing "king_buckets"/);
  rejects(manifest({ extra: 1 }), /unknown field "extra"/);
  rejects(manifest({ schema: "rusty-chess-nnue-diagram/0" }), /schema must be/);
  rejects(manifest({ net_id: "net 8" }), /not <name>-l<λ×100>-r<run id>/);
});

test("a width or bucket count is never defaulted or silently accepted", () => {
  rejects(manifest({}, { hidden_width: 100 }), /hidden_width must be a positive multiple of 16/);
  rejects(manifest({}, { hidden_width: "512" }), /hidden_width must be/);
  rejects(manifest({}, { king_buckets: 0 }), /king_buckets must be a positive integer/);
  rejects(manifest({}, { output_buckets: 2.5 }), /output_buckets must be a positive integer/);
  rejects(manifest({}, { king_buckets: 1, king_bucket_layout: "KING_BUCKET_LAYOUT" }), /king_bucket_layout must be null when king_buckets is 1/);
  rejects(manifest({}, { king_buckets: 4, king_bucket_layout: null }), /king_bucket_layout must name the layout/);
  rejects(manifest({}, { output_buckets: 1, output_bucket_rule: "(pieces − 2) / 4" }), /output_bucket_rule must be null/);
  rejects(manifest({}, { output_buckets: 8, output_bucket_rule: null }), /output_bucket_rule must give the rule/);
  rejects(manifest({}, { activation: "ReLU" }), /activation must be "SCReLU"/);
  rejects(manifest({}, { perspectives: 1 }), /perspectives must be 2/);
  rejects(manifest({}, { shared_feature_transformer: false }), /must be true/);
});

test("file_bytes must fit the topology, padded or not", () => {
  validateManifest(manifest({ file_bytes: 3_163_152 }));
  validateManifest(manifest({ file_bytes: 3_163_200 }));
  rejects(manifest({ file_bytes: 803_904 }), /file_bytes 803904 does not fit the topology \(768 × 4 → 512\) × 2 → 8: expected 3163152 \(unpadded\) or 3163200/);
});

test("status, releases, λ and the change are cross-checked", () => {
  rejects(manifest({ status: "kept" }), /status must be one of/);
  rejects(manifest({ status: "dropped" }), /a dropped net was never a default/);
  rejects(manifest({ status: "retired", default_releases: null }), /names the releases/);
  rejects(manifest({ default_releases: "0.42.0" }), /default_releases must be a tag/);
  rejects(manifest({ training: { generations: "0–4", labels: "O", lambda: 0.25, superbatches: 80 } }), /disagrees with the net id's l050/);
  rejects(manifest({ change: { from: null, kind: "topology", summary: "x" } }), /"first" is for the net with no predecessor/);
  rejects(manifest({ change: { from: "net6-l050-r41273387683", kind: "first", summary: "x" } }), /"first" is for the net with no predecessor/);
  rejects(manifest({ change: { from: "net6-l050-r41273387683", kind: "retrain", summary: "x" } }), /change.kind must be one of/);
  rejects(manifest({ experiment: { pr: 1, baseline: null, timed: null, fixed_nodes: null, verdict: "won" } }), /verdict must be one of/);
});

function dirWith(entries) {
  const dir = mkdtempSync(join(tmpdir(), "nnue-manifests-"));
  for (const [name, content] of Object.entries(entries)) writeFileSync(join(dir, name), typeof content === "string" ? content : JSON.stringify(content));
  return dir;
}

test("loading a directory orders by run id and checks references, names and the default", () => {
  const a = manifest({ net_id: "net6-l050-r41273387683", status: "superseded", default_releases: "v0.40.0–v0.41.0", change: { from: null, kind: "first", summary: "x" }, generation: null });
  const b = manifest({ status: "default", default_releases: "v0.42.0" });
  const dir = dirWith({ "net6k4-l050-r41282883885.json": b, "net6-l050-r41273387683.json": a });
  const loaded = loadManifests(dir);
  assert.deepEqual(
    loaded.map((m) => m.net_id),
    ["net6-l050-r41273387683", "net6k4-l050-r41282883885"],
  );

  assert.throws(() => loadManifests(dirWith({ "net6k4-l050-r41282883885.json": manifest({ status: "default", default_releases: "v0.42.0" }) })), /change.from names net6-l050-r41273387683, which has no manifest/);
  assert.throws(() => loadManifests(dirWith({ "other.json": b })), /file must be named <net_id>.json/);
  assert.throws(() => loadManifests(dirWith({ "net6k4-l050-r41282883885.json": "{not json" })), /not valid JSON/);
  assert.throws(() => loadManifests(dirWith({ "net6k4-l050-r41282883885.json": b, "net6-l050-r41273387683.json": { ...a, status: "default", default_releases: "v0.40.0" } })), /exactly one manifest must have status "default"; found 2/);
  assert.throws(() => loadManifests(dirWith({ "net6k4-l050-r41282883885.json": { ...b, status: "retired" }, "net6-l050-r41273387683.json": a })), /found 0/);
  assert.ok(new ManifestError("f", "m") instanceof Error);
});
