#!/usr/bin/env node
// The NNUE architecture diagrams: generate, check or list them.
//
//   node tools/nnue-diagrams/cli.mjs generate [--net <net id>]
//   node tools/nnue-diagrams/cli.mjs check    [--net <net id>]
//   node tools/nnue-diagrams/cli.mjs list
//
// `generate` writes a .d2 and a .svg per manifest and refreshes the generated
// blocks of the gallery pages. `check` renders everything again in memory and
// fails, without writing, on any invalid manifest, missing or stale file,
// orphan file, broken link, net in docs/nets.md without a snapshot, or
// disagreement with the engine's embedded and retired nets.
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, normalize, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { BLOCKS, applyBlocks, blocksFor, relativeLinks } from "./gallery.mjs";
import { embeddedNetFiles, engineNets, headingSlug, headingSlugs, summaryNetIds } from "./ledger.mjs";
import { ManifestError, loadManifests } from "./manifest.mjs";
import { REPO_ROOT, Renderer } from "./render.mjs";
import { d2Source } from "./template.mjs";

export const PATHS = {
  gallery: "docs/nnue-architectures",
  manifests: "docs/nnue-architectures/manifests",
  diagrams: "docs/nnue-architectures/diagrams",
  ledger: "docs/nets.md",
  nnueRs: "crates/search/src/nnue.rs",
  netFiles: "crates/search/nets",
};

function parseArgs(argv) {
  const [command, ...rest] = argv;
  const opts = { net: null };
  for (let i = 0; i < rest.length; i++) {
    if (rest[i] === "--net" && rest[i + 1]) opts.net = rest[++i];
    else throw new Error(`unknown argument ${rest[i]}`);
  }
  return { command, opts };
}

/** Everything the generator would write, as { path (repo-relative), content }. */
export async function plan(root, manifests, renderer, only = null) {
  const files = [];
  for (const m of manifests) {
    if (only !== null && m.net_id !== only) continue;
    const source = d2Source(m);
    files.push({ path: `${PATHS.diagrams}/${m.net_id}.d2`, content: source });
    files.push({ path: `${PATHS.diagrams}/${m.net_id}.svg`, content: await renderer.render(source) });
  }
  for (const page of Object.keys(BLOCKS)) {
    const path = `${PATHS.gallery}/${page}`;
    const current = readFileSync(join(root, path), "utf8");
    files.push({ path, content: applyBlocks(current, blocksFor(page, manifests)) });
  }
  return files;
}

/** The consistency checks beyond file freshness. Returns a list of problems. */
export function crossChecks(root, manifests, generated) {
  const problems = [];
  const ids = new Set(manifests.map((m) => m.net_id));

  // Every trained net in the ledger has a snapshot, and every snapshot is in the ledger.
  const ledger = readFileSync(join(root, PATHS.ledger), "utf8");
  const ledgerIds = summaryNetIds(ledger);
  for (const id of ledgerIds) if (!ids.has(id)) problems.push(`${PATHS.ledger}: \`${id}\` is in the Summary table but has no manifest in ${PATHS.manifests}/`);
  for (const id of ids) if (!ledgerIds.includes(id)) problems.push(`${PATHS.manifests}/${id}.json: not in ${PATHS.ledger}'s Summary table`);

  // Every ledger anchor resolves to a heading.
  const slugs = headingSlugs(ledger);
  for (const m of manifests) {
    if (!slugs.has(headingSlug(m.ledger_anchor))) problems.push(`${m.file}: ledger_anchor "${m.ledger_anchor}" matches no heading in ${PATHS.ledger}`);
  }

  // The engine's embedded and retired nets agree with the statuses.
  const engine = engineNets(readFileSync(join(root, PATHS.nnueRs), "utf8"));
  const files = embeddedNetFiles(join(root, PATHS.netFiles));
  for (const id of engine.embedded) {
    const m = manifests.find((x) => x.net_id === id);
    if (!m) problems.push(`${PATHS.nnueRs}: embedded net ${id} has no manifest`);
    else if (m.status !== "default") problems.push(`${m.file}: the engine embeds this net, so its status must be "default", not "${m.status}"`);
    const size = files.get(id);
    if (size === undefined) problems.push(`${PATHS.netFiles}/${id}.nnue is missing`);
    else if (m && m.file_bytes !== size) problems.push(`${m.file}: file_bytes ${m.file_bytes} but ${PATHS.netFiles}/${id}.nnue is ${size} bytes`);
  }
  for (const id of engine.retired) {
    const m = manifests.find((x) => x.net_id === id);
    if (!m) problems.push(`${PATHS.nnueRs}: retired net ${id} has no manifest`);
    else if (m.status !== "retired") problems.push(`${m.file}: ${PATHS.nnueRs} lists this net in RETIRED_NETS, so its status must be "retired", not "${m.status}"`);
  }
  for (const m of manifests) {
    if (m.status === "default" && !engine.embedded.includes(m.net_id)) problems.push(`${m.file}: status "default" but the engine embeds ${engine.embedded.join(", ")}`);
    if (m.status === "retired" && !engine.retired.includes(m.net_id)) problems.push(`${m.file}: status "retired" but ${PATHS.nnueRs}'s RETIRED_NETS does not list it`);
  }

  // No orphan files beside the snapshots.
  for (const [dir, exts] of [
    [PATHS.manifests, [".json"]],
    [PATHS.diagrams, [".d2", ".svg"]],
  ]) {
    for (const f of readdirSync(join(root, dir))) {
      const ext = exts.find((e) => f.endsWith(e));
      if (!ext) problems.push(`${dir}/${f}: unexpected file (only ${exts.join(" and ")} snapshots belong here)`);
      else if (!ids.has(f.slice(0, -ext.length))) problems.push(`${dir}/${f}: no manifest with this net id`);
    }
  }

  // Every relative link in the generated pages resolves.
  for (const g of generated.filter((f) => f.path.endsWith(".md"))) {
    const dir = dirname(join(root, g.path));
    for (const link of new Set(relativeLinks(g.content))) {
      const [file, anchor] = link.split("#");
      const target = normalize(join(dir, decodeURIComponent(file)));
      if (!existsSync(target)) {
        problems.push(`${g.path}: link to ${link} points at a missing file`);
        continue;
      }
      if (anchor && file.endsWith(".md") && !headingSlugs(readFileSync(target, "utf8")).has(anchor)) {
        problems.push(`${g.path}: link to ${link} names an anchor ${relative(root, target)} does not have`);
      }
    }
  }
  return problems;
}

function compare(root, generated) {
  const problems = [];
  for (const g of generated) {
    const full = join(root, g.path);
    if (!existsSync(full) || !statSync(full).isFile()) {
      problems.push(`${g.path}: missing; run \`npm run nnue:diagrams\``);
    } else if (readFileSync(full, "utf8") !== g.content) {
      problems.push(`${g.path}: stale; run \`npm run nnue:diagrams\` and commit the result`);
    }
  }
  return problems;
}

export async function main(argv, root = REPO_ROOT) {
  const { command, opts } = parseArgs(argv);
  if (!["generate", "check", "list"].includes(command)) {
    console.error("usage: cli.mjs generate|check [--net <net id>] | list");
    return 2;
  }
  const manifests = loadManifests(join(root, PATHS.manifests));
  if (opts.net !== null && !manifests.some((m) => m.net_id === opts.net)) throw new Error(`no manifest for ${opts.net}`);

  if (command === "list") {
    for (const m of manifests) console.log(`${m.net_id.padEnd(36)} ${m.signature.padEnd(26)} ${m.status.padEnd(10)} ${m.change.kind}: ${m.change.summary}`);
    return 0;
  }

  const renderer = await Renderer.create();
  try {
    const generated = await plan(root, manifests, renderer, opts.net);
    if (command === "generate") {
      for (const g of generated) {
        mkdirSync(dirname(join(root, g.path)), { recursive: true });
        writeFileSync(join(root, g.path), g.content);
      }
      console.log(`wrote ${generated.length} files for ${opts.net ?? `${manifests.length} nets`} with D2 ${await renderer.engineVersion()} (${renderer.packageVersion})`);
      return 0;
    }
    const problems = [...compare(root, generated), ...(opts.net === null ? crossChecks(root, manifests, generated) : [])];
    if (problems.length > 0) {
      for (const p of problems) console.error(`nnue-diagrams: ${p}`);
      console.error(`nnue-diagrams: ${problems.length} problem${problems.length === 1 ? "" : "s"}`);
      return 1;
    }
    console.log(`nnue-diagrams: ${manifests.length} snapshots current with D2 ${await renderer.engineVersion()} (${renderer.packageVersion})`);
    return 0;
  } finally {
    renderer.close();
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  main(process.argv.slice(2)).then(
    (code) => process.exit(code),
    (e) => {
      console.error(`nnue-diagrams: ${e instanceof ManifestError ? e.message : (e.stack ?? e.message)}`);
      process.exit(1);
    },
  );
}
