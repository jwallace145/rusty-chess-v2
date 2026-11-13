// End-to-end: the real manifests, rendered with the pinned D2, against a copy
// of the repository's docs so that `check` can be shown to fail on a stale
// or missing file without touching the working tree.
import assert from "node:assert/strict";
import { cpSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { PATHS, main } from "./cli.mjs";
import { REPO_ROOT } from "./render.mjs";

function repoCopy() {
  const root = mkdtempSync(join(tmpdir(), "nnue-diagrams-repo-"));
  for (const rel of [PATHS.gallery, PATHS.netFiles]) {
    mkdirSync(join(root, rel), { recursive: true });
    cpSync(join(REPO_ROOT, rel), join(root, rel), { recursive: true });
  }
  // The pages link to other docs and to the tool's README; the link check needs them.
  mkdirSync(join(root, "crates/search/src"), { recursive: true });
  mkdirSync(join(root, "tools/nnue-diagrams"), { recursive: true });
  for (const f of readdirSync(join(REPO_ROOT, "docs")).filter((f) => f.endsWith(".md"))) cpSync(join(REPO_ROOT, "docs", f), join(root, "docs", f));
  cpSync(join(REPO_ROOT, "tools/nnue-diagrams/README.md"), join(root, "tools/nnue-diagrams/README.md"));
  cpSync(join(REPO_ROOT, PATHS.nnueRs), join(root, PATHS.nnueRs));
  return root;
}

const quiet = async (fn) => {
  const errors = [];
  const { error, log } = console;
  console.error = (...a) => errors.push(a.join(" "));
  console.log = () => {};
  try {
    return { code: await fn(), errors };
  } finally {
    console.error = error;
    console.log = log;
  }
};

test("the committed snapshots are current, and check fails on a stale or missing file", { timeout: 120_000 }, async () => {
  const root = repoCopy();
  try {
    const first = await quiet(() => main(["check"], root));
    assert.equal(first.code, 0, `the committed snapshots should pass; run \`npm run nnue:diagrams\`:\n${first.errors.join("\n")}`);

    const svg = join(root, PATHS.diagrams, "net8-l050-r42564788061.svg");
    writeFileSync(svg, `${readFileSync(svg, "utf8")}<!-- edited -->\n`);
    let r = await quiet(() => main(["check"], root));
    assert.equal(r.code, 1);
    assert.ok(r.errors.some((e) => /net8-l050-r42564788061\.svg: stale/.test(e)), r.errors.join("\n"));

    rmSync(join(root, PATHS.diagrams, "net1-l050-r41022503872.d2"));
    r = await quiet(() => main(["check"], root));
    assert.ok(r.errors.some((e) => /net1-l050-r41022503872\.d2: missing/.test(e)), r.errors.join("\n"));

    // check never writes: the stale file is still stale afterwards.
    assert.match(readFileSync(svg, "utf8"), /<!-- edited -->/);

    // generate repairs both, and a second generate changes nothing.
    assert.equal((await quiet(() => main(["generate"], root))).code, 0);
    const after = readFileSync(svg, "utf8");
    assert.equal((await quiet(() => main(["check"], root))).code, 0);
    assert.equal((await quiet(() => main(["generate"], root))).code, 0);
    assert.equal(readFileSync(svg, "utf8"), after);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("check fails when the ledger has a net without a snapshot, or a snapshot has no ledger row", { timeout: 120_000 }, async () => {
  const root = repoCopy();
  try {
    const ledger = join(root, PATHS.ledger);
    writeFileSync(ledger, readFileSync(ledger, "utf8").replace("## Summary\n", "## Summary\n\n| `net9-l050-r99999999999` | planned |\n"));
    let r = await quiet(() => main(["check"], root));
    assert.equal(r.code, 1);
    assert.ok(r.errors.some((e) => /`net9-l050-r99999999999` is in the Summary table but has no manifest/.test(e)), r.errors.join("\n"));

    cpSync(join(REPO_ROOT, PATHS.ledger), ledger);
    const stray = join(root, PATHS.manifests, "net5x120-l050-r41237487627.json");
    const m = JSON.parse(readFileSync(stray, "utf8"));
    m.net_id = "net5x121-l050-r41237487628";
    m.ledger_anchor = "nowhere";
    writeFileSync(join(root, PATHS.manifests, "net5x121-l050-r41237487628.json"), JSON.stringify(m));
    r = await quiet(() => main(["check"], root));
    assert.equal(r.code, 1);
    assert.ok(r.errors.some((e) => /net5x121-l050-r41237487628\.json: not in docs\/nets\.md's Summary table/.test(e)), r.errors.join("\n"));
    assert.ok(r.errors.some((e) => /ledger_anchor "nowhere" matches no heading/.test(e)), r.errors.join("\n"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("check fails when a status disagrees with the engine, or an orphan file appears", { timeout: 120_000 }, async () => {
  const root = repoCopy();
  try {
    const path = join(root, PATHS.manifests, "net8x160-l050-r43390801430.json");
    const m = JSON.parse(readFileSync(path, "utf8"));
    m.file_bytes = 3_163_152;
    writeFileSync(path, JSON.stringify(m));
    writeFileSync(join(root, PATHS.diagrams, "notes.txt"), "x");
    const r = await quiet(() => main(["check"], root));
    assert.equal(r.code, 1);
    assert.ok(r.errors.some((e) => /file_bytes 3163152 but crates\/search\/nets\/net8x160-l050-r43390801430\.nnue is 3163200 bytes/.test(e)), r.errors.join("\n"));
    assert.ok(r.errors.some((e) => /notes\.txt: unexpected file/.test(e)), r.errors.join("\n"));
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("an invalid manifest stops everything with its file and reason", { timeout: 60_000 }, async () => {
  const root = repoCopy();
  try {
    const path = join(root, PATHS.manifests, "net6-l050-r41273387683.json");
    const m = JSON.parse(readFileSync(path, "utf8"));
    m.topology.output_buckets = 3;
    writeFileSync(path, JSON.stringify(m));
    await assert.rejects(() => quiet(() => main(["check"], root)), /net6-l050-r41273387683\.json: file_bytes 803904 does not fit the topology \(768 → 512\) × 2 → 3/);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("list prints every snapshot in training order", async () => {
  const lines = [];
  const { log } = console;
  console.log = (s) => lines.push(s);
  try {
    assert.equal(await main(["list"]), 0);
  } finally {
    console.log = log;
  }
  assert.equal(lines.length, 19);
  assert.match(lines[0], /^net1-l050-r41022503872 /);
  assert.match(lines.at(-1), /^net8x160-l050-r43390801430 .*default/);
});
