// What the rest of the repository says about the nets, read so the gallery
// can be checked against it: the ledger's summary table (every trained net),
// its headings (the anchors the manifests link to), the engine's embedded
// and retired nets, and the embedded net file itself.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

/** The net ids in the first column of docs/nets.md's Summary table, in order. */
export function summaryNetIds(netsMarkdown) {
  const lines = netsMarkdown.split("\n");
  const start = lines.findIndex((l) => /^## Summary\s*$/.test(l));
  if (start < 0) throw new Error("docs/nets.md has no `## Summary` heading");
  const ids = [];
  for (const line of lines.slice(start + 1)) {
    if (/^## /.test(line)) break;
    const m = /^\|\s*`([^`]+)`\s*\|/.exec(line);
    if (m) ids.push(m[1]);
  }
  if (ids.length === 0) throw new Error("docs/nets.md's Summary table has no net rows");
  return ids;
}

/** GitHub's anchor for a heading: lowercased, punctuation dropped, spaces to hyphens. */
export function headingSlug(heading) {
  return heading
    .trim()
    .toLowerCase()
    .replace(/[^\p{L}\p{N}\s_-]/gu, "")
    .replace(/\s/g, "-");
}

/** Every `##`/`###` heading of a Markdown document, as a set of GitHub anchors. */
export function headingSlugs(markdown) {
  const slugs = new Set();
  for (const line of markdown.split("\n")) {
    const m = /^#{1,6}\s+(.+?)\s*$/.exec(line);
    if (m) slugs.add(headingSlug(m[1]));
  }
  return slugs;
}

function idsInBlock(source, constName) {
  const re = new RegExp(`pub const ${constName}\\b[^=]*=\\s*\\[([\\s\\S]*?)\\];`);
  const m = re.exec(source);
  if (!m) throw new Error(`crates/search/src/nnue.rs has no \`pub const ${constName}\` array`);
  return [...m[1].matchAll(/\bid:\s*"([^"]+)"/g)].map((x) => x[1]);
}

/** The engine's embedded (`NETS`) and retired (`RETIRED_NETS`) net ids. */
export function engineNets(nnueRs) {
  return { embedded: idsInBlock(nnueRs, "NETS"), retired: idsInBlock(nnueRs, "RETIRED_NETS") };
}

/** The `.nnue` files the binary embeds, by net id, with their sizes. */
export function embeddedNetFiles(dir) {
  const out = new Map();
  for (const f of readdirSync(dir).filter((f) => f.endsWith(".nnue")).sort()) {
    out.set(f.slice(0, -".nnue".length), statSync(join(dir, f)).size);
  }
  return out;
}

export function readRepoFile(root, rel) {
  return readFileSync(join(root, rel), "utf8");
}
