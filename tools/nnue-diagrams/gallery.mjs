// The generated parts of docs/nnue-architectures/README.md and evolution.md:
// the current default, the chronological table, the thumbnail grid and the
// table of topology changes. The prose around them is written by hand; the
// generator replaces only the text between a block's start and end markers.
import { headingSlug } from "./ledger.mjs";

export const BLOCKS = {
  "README.md": ["current", "timeline", "gallery"],
  "evolution.md": ["evolution"],
};

const marker = (name, edge) => `<!-- nnue-diagrams:${name}:${edge} -->`;

/** Replaces the text between each block's markers; every block must be present exactly once. */
export function applyBlocks(markdown, blocks) {
  let out = markdown;
  for (const [name, body] of Object.entries(blocks)) {
    const start = marker(name, "start");
    const end = marker(name, "end");
    const i = out.indexOf(start);
    const j = out.indexOf(end);
    if (i < 0 || j < 0 || j < i) throw new Error(`the ${start} … ${end} block is missing or malformed`);
    if (out.indexOf(start, i + 1) >= 0 || out.indexOf(end, j + 1) >= 0) throw new Error(`the ${name} block appears more than once`);
    out = `${out.slice(0, i + start.length)}\n${body.trim()}\n${out.slice(j)}`;
  }
  return out;
}

const escapeHtml = (s) => String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const cell = (s) => String(s).replace(/\|/g, "\\|").replace(/\n/g, " ");

export const diagramPath = (m) => `diagrams/${m.net_id}.svg`;
export const sourcePath = (m) => `diagrams/${m.net_id}.d2`;
export const manifestPath = (m) => `manifests/${m.net_id}.json`;
export const ledgerLink = (m) => `../nets.md#${headingSlug(m.ledger_anchor)}`;

/** The image's alt text: the topology in words, for readers without SVG. */
export function altText(m) {
  const t = m.topology;
  const king = t.king_buckets > 1 ? `${t.king_buckets} mirrored king buckets select each perspective's input weights` : "one input view (no king buckets)";
  const heads = t.output_buckets > 1 ? `one of ${t.output_buckets} output heads chosen by piece count` : "a single output head";
  return `Architecture of ${m.net_id}: ${m.signature}. Chess768 features, ${king}, two perspective accumulators of ${t.hidden_width} sharing one weight set, SCReLU, ${2 * t.hidden_width} activations into ${heads}, scaled to centipawns.`;
}

const STATUS_WORDS = {
  default: "**Default**",
  retired: "Retired",
  superseded: "Superseded",
  dropped: "Dropped",
};

const CHANGE_WORDS = {
  first: "First net",
  topology: "**Topology**",
  data: "Data",
  schedule: "Schedule",
  lambda: "λ",
  labels: "Labels",
};

function experimentCell(m) {
  const e = m.experiment;
  if (e.pr === null) return "—";
  const legs = [e.timed === null ? null : `${e.timed} timed`, e.fixed_nodes === null ? null : `${e.fixed_nodes} at fixed nodes`].filter(Boolean).join(", ");
  const vs = e.baseline === null ? "" : ` vs ${shortName(e.baseline)}`;
  return `#${e.pr}${vs}: ${legs} (${e.verdict})`;
}

/** The ledger's short name for a net: the part of the id before `-l`. */
export const shortName = (id) => id.replace(/-l\d{3}-r\d+$/, "");

function linksCell(m) {
  return `[diagram](${diagramPath(m)}) · [source](${sourcePath(m)}) · [manifest](${manifestPath(m)}) · [ledger](${ledgerLink(m)})`;
}

export function currentBlock(manifests) {
  const m = manifests.find((x) => x.status === "default");
  const t = m.topology;
  return `**\`${m.net_id}\`** — \`${m.signature}\` (${m.featureTag}), ${m.status_note.replace(/\.$/, "")}.
${t.hidden_width} hidden neurons per perspective, ${t.king_buckets} king bucket${t.king_buckets > 1 ? "s" : ""}, ${t.output_buckets} output head${t.output_buckets > 1 ? "s" : ""}; ${m.parameters.toLocaleString("en-US")} parameters.
[Diagram source](${sourcePath(m)}) · [manifest](${manifestPath(m)}) · [the ledger's record](${ledgerLink(m)})

<p align="center">
  <a href="${diagramPath(m)}"><img src="${diagramPath(m)}" alt="${escapeHtml(altText(m))}"></a>
</p>`;
}

export function timelineBlock(manifests) {
  const header = [
    "| # | Net | Topology | Change from the net before | Status | Experiment | Links |",
    "|--:|:--|:--|:--|:--|:--|:--|",
  ];
  const rows = manifests.map((m, i) => {
    const from = m.change.from === null ? "" : ` (from \`${shortName(m.change.from)}\`)`;
    const change = m.change.kind === "first" ? cell(m.change.summary) : `${CHANGE_WORDS[m.change.kind]}: ${cell(m.change.summary)}${from}`;
    const status = m.status_note.toLowerCase().startsWith(m.status) ? cell(m.status_note) : `${STATUS_WORDS[m.status]}: ${cell(m.status_note)}`;
    return `| ${i + 1} | \`${m.net_id}\` | \`${m.signature}\` | ${change} | ${status} | ${cell(experimentCell(m))} | ${linksCell(m)} |`;
  });
  return [...header, ...rows].join("\n");
}

export function galleryBlock(manifests, perRow = 3) {
  const cells = manifests.map((m) => {
    const tag = m.status === "dropped" ? "dropped" : m.status === "default" ? "default" : m.status;
    return `    <td align="center" valign="top" width="${Math.floor(100 / perRow)}%">
      <a href="${diagramPath(m)}"><img src="${diagramPath(m)}" alt="${escapeHtml(altText(m))}" width="240"></a><br>
      <sub><code>${escapeHtml(m.net_id)}</code><br>${escapeHtml(m.signature)} · ${tag}<br>
      <a href="${sourcePath(m)}">source</a> · <a href="${manifestPath(m)}">manifest</a> · <a href="${ledgerLink(m)}">ledger</a></sub>
    </td>`;
  });
  const rows = [];
  for (let i = 0; i < cells.length; i += perRow) rows.push(`  <tr>\n${cells.slice(i, i + perRow).join("\n")}\n  </tr>`);
  return `<table>\n${rows.join("\n")}\n</table>`;
}

export function evolutionBlock(manifests) {
  const steps = manifests.filter((m) => m.change.kind === "first" || m.change.kind === "topology");
  const byId = new Map(manifests.map((m) => [m.net_id, m]));
  const header = ["| Step | Before | After | What changed | Measured (SPRT, both legs) | Record |", "|--:|:--|:--|:--|:--|:--|"];
  const rows = steps.map((m, i) => {
    const before = m.change.from === null ? "—" : `[\`${shortName(m.change.from)}\`](${diagramPath(byId.get(m.change.from))})<br>\`${byId.get(m.change.from).signature}\``;
    const after = `[\`${shortName(m.net_id)}\`](${diagramPath(m)})<br>\`${m.signature}\``;
    const e = m.experiment;
    const measured =
      e.pr === null
        ? "—"
        : `${e.timed ?? "—"} timed, ${e.fixed_nodes ?? "—"} at fixed nodes${e.baseline ? ` vs ${shortName(e.baseline)}` : ""} (#${e.pr}, ${e.verdict})`;
    return `| ${i + 1} | ${before} | ${after} | ${cell(m.change.summary)} | ${cell(measured)} | [ledger](${ledgerLink(m)}) · [manifest](${manifestPath(m)}) |`;
  });
  return [...header, ...rows].join("\n");
}

export function blocksFor(file, manifests) {
  const all = { current: currentBlock, timeline: timelineBlock, gallery: galleryBlock, evolution: evolutionBlock };
  return Object.fromEntries(BLOCKS[file].map((name) => [name, all[name](manifests)]));
}

/** Every relative link target (Markdown links and HTML href/src) in a text. */
export function relativeLinks(text) {
  const out = [];
  for (const m of text.matchAll(/\]\(([^)\s]+)\)/g)) out.push(m[1]);
  for (const m of text.matchAll(/\b(?:href|src)="([^"]+)"/g)) out.push(m[1]);
  return out.filter((l) => !/^[a-z]+:/i.test(l) && !l.startsWith("#"));
}
