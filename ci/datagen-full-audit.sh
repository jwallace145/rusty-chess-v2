#!/usr/bin/env bash
# The full audit of a whole self-play generation, run locally on a download of
# it (read-only; docs/datagen-runbook.md, "After a run"):
#
#   ci/datagen-full-audit.sh GEN_DIR OUT_DIR [SHARDS_PER_BATCH] [JOBS] [GAMES_PER_WORKER]
#
#   GEN_DIR  the generation as in S3: run=*/task=*/{manifests,positions,games}/
#            (aws s3 sync s3://BUCKET/data/selfplay/v1/gen=N/ GEN_DIR)
#   OUT_DIR  reports: inventory.json, one log per batch, determinism.log,
#            summary.json; written fresh
#
# 1. inventory: every manifest and file, without the records (complete
#    shards, no orphans, gap-free shard numbers, no duplicate shards, unique
#    seeds, one settings block, engine and net per generation).
# 2. check --bullet on every shard, SHARDS_PER_BATCH (200) at a time, JOBS
#    (8) at once: every record rebuilt from its replayed game, every result
#    from its final position, checksums, counts, bulletformat's parser.
# 3. totals over all batches (summary.json): records, games, P0 failures,
#    bulletformat bytes, results.
# 4. determinism: every worker's first GAMES_PER_WORKER (12) games regenerated
#    from its seed. Build this tool at the data's engine_sha (in every
#    manifest): identical games prove that engine, net and settings played them.
#
# Exits 1 if any step finds a P0 problem. Needs Rust and python3.
set -euo pipefail
gen=$(cd "${1:?a generation directory}" && pwd)
out="${2:?an output directory}"
per="${3:-200}"
jobs="${4:-8}"
games="${5:-12}"
root="$(cd "$(dirname "$0")/.." && pwd)"
rm -rf "$out"; mkdir -p "$out/batches" "$out/logs"
out=$(cd "$out" && pwd)

echo "==> building datagen-audit at $(git -C "$root" rev-parse --short HEAD)"
(cd "$root" && cargo build --release --locked --quiet -p rusty-chess-tools --bin datagen-audit)
audit="$root/target/release/datagen-audit"
status=0

echo "==> 1. inventory"
"$audit" inventory "$gen" --json "$out/inventory.json" | tee "$out/logs/inventory.log" | grep -E "^==|PASS|FAIL|engine|net |shards," || status=1

echo "==> 2. check --bullet, $per shards a batch, $jobs at once"
n=0
for task in "$gen"/run=*/task=*; do
  tag=$(echo "$task" | sed -E 's#.*/run=([^/]+)/task=([^/]+)#\1-\2#')
  find "$task/manifests" -name '*.json' -exec basename {} .json \; | sort | split -l "$per" -a 3 - "$out/batches/$tag."
  for chunk in "$out/batches/$tag".*; do
    b="$chunk.d"; mkdir -p "$b"
    while read -r name; do
      ln -s "$task/manifests/$name.json" "$b/$name.json"
      ln -s "$task/positions/$name.txt.gz" "$b/$name.positions.txt.gz"
      ln -s "$task/games/$name.jsonl.gz" "$b/$name.games.jsonl.gz"
    done < "$chunk"
    n=$((n + 1))
  done
done
echo "    $n batches"
# The script is single-quoted on purpose: sh -c gets its own $0..$3.
# shellcheck disable=SC2016
find "$out/batches" -name '*.d' -type d | sort | xargs -P "$jobs" -I{} sh -c \
  '"$0" check "$1" --config "$2/datagen.toml" --bullet > "$3/logs/$(basename "$1" .d).log" 2>&1; echo "$(basename "$1" .d) $?"' \
  "$audit" {} "$root" "$out" > "$out/exits.txt"
failed=$(awk '$2 != 0' "$out/exits.txt" | wc -l | tr -d ' ')
echo "    batches with a P0 failure: $failed"
[ "$failed" = 0 ] || status=1

echo "==> 3. totals"
python3 - "$out" <<'PY' || status=1
import glob, json, re, sys
out = sys.argv[1]
T = dict(shards=0, games=0, records=0, failures=0, bullet_records=0, bullet_parse_fail=0,
         bullet_unexpected=0, bullet_bytes=0, bullet_inexact=0, p0=0, white=0.0, draw=0.0, black=0.0)
logs = [f for f in glob.glob(f"{out}/logs/*.log") if not f.endswith(("inventory.log", "determinism.log"))]
for f in logs:
    s = open(f).read()
    m = re.search(r"Replay of (\d+) shards: (\d+) games, (\d+) records", s)
    if not m:
        sys.exit(f"{f}: no replay summary")
    T["shards"] += int(m[1]); T["games"] += int(m[2]); T["records"] += int(m[3])
    T["failures"] += int(re.search(r"^failures: (\d+)", s, re.M)[1])
    T["p0"] += sum(int(v) for v in re.findall(r"^  FAIL [^:]+: (\d+)$", s, re.M))
    b = re.search(r"bulletformat 1\.8\.0: (\d+) records; parse failures (\d+); score, result or piece count not as expected (\d+); binary (\d+) bytes, exact (\w+)", s)
    T["bullet_records"] += int(b[1]); T["bullet_parse_fail"] += int(b[2]); T["bullet_unexpected"] += int(b[3])
    T["bullet_bytes"] += int(b[4]); T["bullet_inexact"] += b[5] != "true"
    r = re.search(r"results: white ([\d.]+)%\s+draw ([\d.]+)%\s+black ([\d.]+)%", s)
    for k, v in zip(("white", "draw", "black"), r.groups()):
        T[k] += float(v) / 100 * int(m[2])
g = T["games"] or 1
summary = {**{k: T[k] for k in T if k not in ("white", "draw", "black")},
           "batches": len(logs), "bullet_bytes_exact": T["bullet_bytes"] == 32 * T["records"],
           "results": {k: round(100 * T[k] / g, 2) for k in ("white", "draw", "black")}}
json.dump(summary, open(f"{out}/summary.json", "w"), indent=2)
print(json.dumps(summary, indent=2))
bad = T["failures"] or T["p0"] or T["bullet_parse_fail"] or T["bullet_unexpected"] or T["bullet_inexact"] or not summary["bullet_bytes_exact"]
sys.exit(1 if bad else 0)
PY

echo "==> 4. determinism: every worker's first $games games"
dd="$out/batches/determinism"; mkdir -p "$dd"
for task in "$gen"/run=*/task=*; do
  t=$(basename "$task"); r=$(basename "$(dirname "$task")")
  for m in "$task"/manifests/*-s00000.json; do
    name=$(basename "$m" .json)
    ln -s "$m" "$dd/$r-$t-$name.json"
    ln -s "$task/positions/$name.txt.gz" "$dd/$r-$t-$name.positions.txt.gz"
    ln -s "$task/games/$name.jsonl.gz" "$dd/$r-$t-$name.games.jsonl.gz"
  done
done
"$audit" determinism "$dd" "$games" | tee "$out/logs/determinism.log" | grep -E "engine_sha|net:|^==|PASS|FAIL" || status=1

echo
if [ "$status" = 0 ]; then echo "full audit: GO"; else echo "full audit: STOP (see $out)"; fi
exit "$status"
