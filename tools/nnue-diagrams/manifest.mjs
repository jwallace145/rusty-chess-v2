// The manifest schema for one NNUE architecture snapshot, its validation,
// and the facts derived from it (the architecture signature, the file size
// the topology implies, the chronological order).
//
// A manifest records a net's *topology*: what the engine computes when it
// evaluates a position. Training provenance (data, labels, λ, schedule) is
// summarised only as far as the gallery needs to say what changed between
// two nets; the record of each experiment stays in docs/nets.md.
import { readFileSync, readdirSync } from "node:fs";
import { basename, join } from "node:path";

export const SCHEMA = "rusty-chess-nnue-diagram/1";

/** `<name>-l<λ×100>-r<Train run id>`, as docs/nets.md defines net ids. */
export const NET_ID = /^([a-z][a-z0-9]*)-l(\d{3})-r(\d+)$/;

export const STATUSES = ["default", "retired", "superseded", "dropped"];
export const CHANGE_KINDS = ["first", "topology", "data", "schedule", "lambda", "labels"];
export const VERDICTS = ["kept", "dropped", "inconclusive"];

/** Every byte of the engine's file format is an i16 (crates/search/src/nnue.rs). */
const BYTES_PER_WEIGHT = 2;
/** bullet pads the saved file to a multiple of 64 bytes; the engine accepts up to 63 of padding. */
const FILE_PADDING = 64;

export class ManifestError extends Error {
  constructor(file, message) {
    super(`${file}: ${message}`);
    this.file = file;
  }
}

const isInt = (v) => Number.isInteger(v);
const isPosInt = (v) => Number.isInteger(v) && v > 0;
const isStr = (v) => typeof v === "string" && v.length > 0;
const isNullOr = (pred) => (v) => v === null || pred(v);

/**
 * The size of a net file with this topology, in bytes, before padding: the
 * engine's `nnue::net_file_bytes` (feature weights, feature bias, each
 * bucket's output weights, each bucket's output bias).
 */
export function fileBytes(topology) {
  const { features_per_perspective: f, king_buckets: k, hidden_width: h, output_buckets: b } = topology;
  return BYTES_PER_WEIGHT * (f * k * h + h + b * (2 * h + 1));
}

/** `fileBytes` rounded up to the padding bullet writes. */
export function paddedFileBytes(topology) {
  return Math.ceil(fileBytes(topology) / FILE_PADDING) * FILE_PADDING;
}

/** Trainable parameters: the feature transformer's weights and biases plus every output head's. */
export function parameterCount(topology) {
  return fileBytes(topology) / BYTES_PER_WEIGHT;
}

/** The ledger's architecture notation, e.g. `(768 × 4 → 512) × 2 → 8`. */
export function signature(topology) {
  const { features_per_perspective: f, king_buckets: k, hidden_width: h, perspectives: p, output_buckets: b } = topology;
  const input = k > 1 ? `${f} × ${k}` : `${f}`;
  return `(${input} → ${h}) × ${p} → ${b}`;
}

/** A short tag of the topology's optional features: `K4 · B8`, `K1 · B1`. */
export function featureTag(topology) {
  return `H${topology.hidden_width} · K${topology.king_buckets} · B${topology.output_buckets}`;
}

/** Parses a net id into its parts, or returns null. */
export function parseNetId(id) {
  const m = NET_ID.exec(id);
  if (!m) return null;
  return { name: m[1], lambda: Number(m[2]) / 100, run: BigInt(m[3]) };
}

function fail(file, message) {
  throw new ManifestError(file, message);
}

function requireKeys(file, obj, where, keys) {
  if (obj === null || typeof obj !== "object" || Array.isArray(obj)) fail(file, `${where} must be an object`);
  for (const k of keys) if (!(k in obj)) fail(file, `${where} is missing "${k}"`);
  for (const k of Object.keys(obj)) if (!keys.includes(k)) fail(file, `${where} has an unknown field "${k}"`);
}

function check(file, cond, message) {
  if (!cond) fail(file, message);
}

/**
 * Validates one parsed manifest. Every topology field is required and
 * checked against what the engine and the trainer can represent; nothing
 * is defaulted. Returns the manifest with its derived fields attached.
 */
export function validateManifest(raw, file = "<manifest>") {
  requireKeys(file, raw, "manifest", [
    "schema",
    "net_id",
    "topology",
    "file_bytes",
    "status",
    "status_note",
    "generation",
    "default_releases",
    "training",
    "change",
    "experiment",
    "ledger_anchor",
    "provenance",
  ]);
  check(file, raw.schema === SCHEMA, `schema must be "${SCHEMA}", got ${JSON.stringify(raw.schema)}`);
  const id = parseNetId(raw.net_id);
  check(file, id !== null, `net_id ${JSON.stringify(raw.net_id)} is not <name>-l<λ×100>-r<run id>`);

  const t = raw.topology;
  requireKeys(file, t, "topology", [
    "feature_set",
    "features_per_perspective",
    "perspectives",
    "shared_feature_transformer",
    "king_buckets",
    "king_bucket_layout",
    "hidden_width",
    "activation",
    "output_buckets",
    "output_bucket_rule",
    "quantization",
  ]);
  check(file, t.feature_set === "Chess768", `topology.feature_set must be "Chess768" (the only input set the engine reads), got ${JSON.stringify(t.feature_set)}`);
  check(file, t.features_per_perspective === 768, `topology.features_per_perspective must be 768 for Chess768, got ${JSON.stringify(t.features_per_perspective)}`);
  check(file, t.perspectives === 2, `topology.perspectives must be 2 (side to move and opponent), got ${JSON.stringify(t.perspectives)}`);
  check(file, t.shared_feature_transformer === true, "topology.shared_feature_transformer must be true: the two perspectives share one weight set");
  check(file, isPosInt(t.king_buckets), `topology.king_buckets must be a positive integer, got ${JSON.stringify(t.king_buckets)}`);
  if (t.king_buckets === 1) {
    check(file, t.king_bucket_layout === null, "topology.king_bucket_layout must be null when king_buckets is 1");
  } else {
    check(file, isStr(t.king_bucket_layout), `topology.king_bucket_layout must name the layout when king_buckets is ${t.king_buckets}`);
  }
  check(
    file,
    isPosInt(t.hidden_width) && t.hidden_width % 16 === 0,
    `topology.hidden_width must be a positive multiple of 16 (the trainer's rule), got ${JSON.stringify(t.hidden_width)}`,
  );
  check(file, t.activation === "SCReLU", `topology.activation must be "SCReLU" (the only activation the engine computes), got ${JSON.stringify(t.activation)}`);
  check(file, isPosInt(t.output_buckets), `topology.output_buckets must be a positive integer, got ${JSON.stringify(t.output_buckets)}`);
  if (t.output_buckets === 1) {
    check(file, t.output_bucket_rule === null, "topology.output_bucket_rule must be null when output_buckets is 1");
  } else {
    check(file, isStr(t.output_bucket_rule), `topology.output_bucket_rule must give the rule when output_buckets is ${t.output_buckets}`);
  }
  requireKeys(file, t.quantization, "topology.quantization", ["qa", "qb", "scale"]);
  for (const k of ["qa", "qb", "scale"]) check(file, isPosInt(t.quantization[k]), `topology.quantization.${k} must be a positive integer`);

  check(file, isNullOr(isPosInt)(raw.file_bytes), "file_bytes must be a positive integer or null (unknown)");
  if (raw.file_bytes !== null) {
    const exact = fileBytes(t);
    const padded = paddedFileBytes(t);
    check(
      file,
      raw.file_bytes === exact || raw.file_bytes === padded,
      `file_bytes ${raw.file_bytes} does not fit the topology ${signature(t)}: expected ${exact} (unpadded) or ${padded} (padded to ${FILE_PADDING})`,
    );
  }

  check(file, STATUSES.includes(raw.status), `status must be one of ${STATUSES.join(", ")}, got ${JSON.stringify(raw.status)}`);
  check(file, isStr(raw.status_note), "status_note must be a non-empty string");
  check(file, isNullOr(isPosInt)(raw.generation), "generation (the self-play generation the net plays or played) must be a positive integer or null");
  check(
    file,
    raw.default_releases === null || /^v\d+\.\d+\.\d+(–v\d+\.\d+\.\d+)?$/.test(raw.default_releases),
    "default_releases must be a tag (v0.36.0), a range (v0.37.0–v0.38.0) or null",
  );
  check(file, (raw.status === "dropped") === (raw.default_releases === null), "a dropped net was never a default, and every other status names the releases it was the default in");

  requireKeys(file, raw.training, "training", ["generations", "labels", "lambda", "superbatches"]);
  check(file, isStr(raw.training.generations), "training.generations must be a non-empty string");
  check(file, isStr(raw.training.labels), "training.labels must be a non-empty string");
  check(file, typeof raw.training.lambda === "number" && raw.training.lambda >= 0 && raw.training.lambda <= 1, "training.lambda must be a number in [0, 1]");
  check(file, raw.training.lambda === id.lambda, `training.lambda ${raw.training.lambda} disagrees with the net id's l${String(Math.round(id.lambda * 100)).padStart(3, "0")}`);
  check(file, isPosInt(raw.training.superbatches), "training.superbatches must be a positive integer");

  requireKeys(file, raw.change, "change", ["from", "kind", "summary"]);
  check(file, raw.change.from === null || parseNetId(raw.change.from) !== null, "change.from must be a net id or null");
  check(file, CHANGE_KINDS.includes(raw.change.kind), `change.kind must be one of ${CHANGE_KINDS.join(", ")}, got ${JSON.stringify(raw.change.kind)}`);
  check(file, (raw.change.kind === "first") === (raw.change.from === null), 'change.kind "first" is for the net with no predecessor, and only that one');
  check(file, isStr(raw.change.summary), "change.summary must be a non-empty string");

  requireKeys(file, raw.experiment, "experiment", ["pr", "baseline", "timed", "fixed_nodes", "verdict"]);
  check(file, isNullOr(isPosInt)(raw.experiment.pr), "experiment.pr must be a PR number or null");
  check(file, raw.experiment.baseline === null || isStr(raw.experiment.baseline), "experiment.baseline must be a string or null");
  check(file, isNullOr(isStr)(raw.experiment.timed), "experiment.timed must be a string or null");
  check(file, isNullOr(isStr)(raw.experiment.fixed_nodes), "experiment.fixed_nodes must be a string or null");
  check(file, VERDICTS.includes(raw.experiment.verdict), `experiment.verdict must be one of ${VERDICTS.join(", ")}, got ${JSON.stringify(raw.experiment.verdict)}`);

  check(file, isStr(raw.ledger_anchor), "ledger_anchor must be a non-empty string (a heading in docs/nets.md)");

  requireKeys(file, raw.provenance, "provenance", ["architecture_source", "training_json", "notes"]);
  check(
    file,
    Array.isArray(raw.provenance.architecture_source) && raw.provenance.architecture_source.length > 0 && raw.provenance.architecture_source.every(isStr),
    "provenance.architecture_source must list at least one source",
  );
  check(file, isNullOr(isStr)(raw.provenance.training_json), "provenance.training_json must be a string or null");
  check(file, isNullOr(isStr)(raw.provenance.notes), "provenance.notes must be a string or null");

  return {
    ...raw,
    file,
    id,
    signature: signature(t),
    featureTag: featureTag(t),
    parameters: parameterCount(t),
    expectedFileBytes: paddedFileBytes(t),
  };
}

/**
 * Loads and validates every manifest in `dir`, in chronological order (by
 * the `Train` run id in the net id), checking that ids are unique, that
 * each file is named after its net, that every reference to another net
 * resolves, and that exactly one net is the default.
 */
export function loadManifests(dir) {
  const files = readdirSync(dir)
    .filter((f) => f.endsWith(".json"))
    .sort();
  const manifests = files.map((f) => {
    const path = join(dir, f);
    let raw;
    try {
      raw = JSON.parse(readFileSync(path, "utf8"));
    } catch (e) {
      throw new ManifestError(path, `not valid JSON: ${e.message}`);
    }
    const m = validateManifest(raw, path);
    if (basename(f, ".json") !== m.net_id) fail(path, `file must be named <net_id>.json; the manifest says ${m.net_id}`);
    return m;
  });
  const ids = new Set();
  for (const m of manifests) {
    if (ids.has(m.net_id)) fail(m.file, `duplicate net_id ${m.net_id}`);
    ids.add(m.net_id);
  }
  for (const m of manifests) {
    if (m.change.from !== null && !ids.has(m.change.from)) fail(m.file, `change.from names ${m.change.from}, which has no manifest`);
    if (m.experiment.baseline !== null && parseNetId(m.experiment.baseline) && !ids.has(m.experiment.baseline)) {
      fail(m.file, `experiment.baseline names ${m.experiment.baseline}, which has no manifest`);
    }
  }
  const defaults = manifests.filter((m) => m.status === "default");
  if (manifests.length > 0 && defaults.length !== 1) {
    fail(dir, `exactly one manifest must have status "default"; found ${defaults.length}: ${defaults.map((m) => m.net_id).join(", ") || "none"}`);
  }
  manifests.sort((a, b) => (a.id.run < b.id.run ? -1 : a.id.run > b.id.run ? 1 : 0));
  for (let i = 1; i < manifests.length; i++) {
    if (manifests[i].id.run === manifests[i - 1].id.run) {
      fail(manifests[i].file, `shares Train run ${manifests[i].id.run} with ${manifests[i - 1].net_id}; the order would be ambiguous`);
    }
  }
  return manifests;
}
