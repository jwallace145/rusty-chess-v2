import assert from "node:assert/strict";
import { test } from "node:test";

import { validateManifest } from "./manifest.mjs";
import { manifest } from "./manifest.test.mjs";
import { d2Source, d2String } from "./template.mjs";

const source = (H, K, B, patch = {}) => d2Source(validateManifest(manifest(patch, { hidden_width: H, king_buckets: K, output_buckets: B })));

test("D2 strings are quoted and escaped, and control characters are refused", () => {
  assert.equal(d2String('a "b" \\ c'), '"a \\"b\\" \\\\ c"');
  assert.equal(d2String("two\nlines"), '"two\\nlines"');
  assert.throws(() => d2String("bell\u0007"), /control character U\+0007/);
});

test("manifest text never reaches D2 unquoted", () => {
  const s = source(512, 4, 8, { status_note: 'x" } evil: { shape: circle' });
  assert.ok(s.includes('x\\" } evil: { shape: circle'));
  assert.ok(!s.includes("\nevil"));
});

test("(128, K1, B1): one view, one head, nothing later backported", () => {
  const s = source(128, 1, 1);
  assert.match(s, /\(768 → 128\) × 2 → 1/);
  assert.match(s, /board -> ft\.us/);
  assert.doesNotMatch(s, /views/);
  assert.doesNotMatch(s, /king bucket/i);
  assert.doesNotMatch(s, /select/);
  assert.match(s, /head: "Single output head\\n256 → 1: 256 weights \+ bias"/);
  assert.doesNotMatch(s, /other heads/);
  assert.match(s, /Shared weights W\\n768 → 128: 98,304 weights \+ 128 biases/);
  assert.match(s, /SCReLU on every neuron\\nclamp\(x, 0, 255\)²/);
  assert.match(s, /Concatenate: 256 activations\\n\[side to move 128 \| opponent 128\]/);
});

test("(512, K1, B8): a bank of eight heads and one selected, no king buckets", () => {
  const s = source(512, 1, 8);
  assert.match(s, /\(768 → 512\) × 2 → 8/);
  assert.doesNotMatch(s, /views/);
  assert.match(s, /Bank of 8 output heads, 1024 → 1 each/);
  assert.match(s, /The one selected head\\n1024 → 1: 1,024 weights \+ bias/);
  assert.match(s, /7 other heads\\nnot evaluated for this position/);
  assert.match(s, /\(pieces − 2\) \/ 4 {2}→ {2}one of 8/);
  assert.match(s, /select -> heads\.chosen/);
  assert.match(s, /concat -> heads\.rest: "not used" \{ class: faint \}/);
  // Nothing sums the heads or produces an 8-wide output vector.
  assert.doesNotMatch(s, /8 outputs|sum(med)? (of|over) (the )?heads|heads -> /i);
  assert.doesNotMatch(s, /heads\.rest -> /);
});

test("(512, K4, B8): per-perspective king views selecting one of four weight slices", () => {
  const s = source(512, 4, 8);
  assert.match(s, /\(768 × 4 → 512\) × 2 → 8/);
  assert.match(s, /views: "Own-king view, one per perspective: 4 king buckets"/);
  assert.match(s, /its own king's bucket: KING_BUCKET_LAYOUT, files e–h mirrored/);
  assert.match(s, /board -> views\.us/);
  assert.match(s, /views\.us -> ft\.us: "selects 1 of 4 slices of W"/);
  assert.match(s, /768 × 4 → 512: 1,572,864 weights \+ 512 biases\\n4 slices of 768 × 512, one per king bucket/);
  assert.match(s, /w -> us: "same W"/);
  assert.match(s, /w -> them: "same W"/);
  // The input is never drawn as 3,072 concatenated features.
  assert.doesNotMatch(s, /3,?072/);
});

test("(768, K4, B8): the width flows through every dimension", () => {
  const s = source(768, 4, 8);
  assert.match(s, /\(768 × 4 → 768\) × 2 → 8/);
  assert.match(s, /768 × 4 → 768: 2,359,296 weights \+ 768 biases/);
  assert.match(s, /Concatenate: 1536 activations/);
  assert.match(s, /Bank of 8 output heads, 1536 → 1 each/);
  assert.match(s, /1,536 weights \+ bias/);
});

test("a training-only change draws the same topology under a different caption", () => {
  const a = source(512, 4, 8);
  const b = source(512, 4, 8, {
    net_id: "net7rl012-l050-r42054816508",
    status_note: "Default v0.45.0–v0.47.2; played generation 6",
    training: { generations: "0–5", labels: "N7 on 0–2, O on 3–5", lambda: 0.5, superbatches: 80 },
    change: { from: "net7-l050-r41363264869", kind: "labels", summary: "relabelled" },
  });
  const body = (s) => s.split("\n").filter((l) => !l.startsWith("#") && !l.startsWith("title:") && !l.startsWith("caption:")).join("\n");
  assert.notEqual(a, b);
  assert.equal(body(a), body(b));
  assert.match(a, /net6k4-l050-r41282883885/);
  assert.match(b, /net7rl012-l050-r42054816508/);
  // A topology change does change the body.
  assert.notEqual(body(a), body(source(768, 4, 8)));
});

test("the source is deterministic and ends with one newline", () => {
  assert.equal(source(512, 4, 8), source(512, 4, 8));
  assert.match(source(128, 1, 1), /[^\n]\n$/);
});
