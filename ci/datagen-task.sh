#!/usr/bin/env bash
# Self-play training data on a dedicated instance, run by the Datagen workflow
# through jwall-engineering-infra's remote-task workflow. It starts in the
# repository root at the workflow's commit, with OUT_DIR set; the data goes to
# S3, not to OUT_DIR (which carries only the task's summary and upload log).
# docs/datagen.md specifies the layout and formats.
#
#   ci/datagen-task.sh --dataset s3://BUCKET/selfplay/v1 --run RUN --task TASK
#                      --minutes M --seed S
#
# Builds datagen at this commit, runs it on every core with the settings in
# datagen.toml, and uploads each shard as soon as its manifest appears:
# positions and games gzip-compressed, the manifest last. On SIGTERM (a spot
# interruption: about two minutes' notice) it asks the workers to stop, uploads
# every complete shard and exits; a shard in progress is lost, nothing is
# corrupted. Never exits 75, which the runner reserves for interruptions.

set -euo pipefail

dataset="" run="" task="" minutes="" seed=""
while [ $# -gt 0 ]; do
  case "$1" in
    --dataset) dataset="${2%/}"; shift 2 ;;
    --run) run="$2"; shift 2 ;;
    --task) task="$2"; shift 2 ;;
    --minutes) minutes="$2"; shift 2 ;;
    --seed) seed="$2"; shift 2 ;;
    *) echo "datagen-task: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$dataset" ] && [ -n "$run" ] && [ -n "$task" ] && [ -n "$minutes" ] && [ -n "$seed" ] \
  || { echo "datagen-task: --dataset, --run, --task, --minutes and --seed are required" >&2; exit 2; }
case "$dataset" in s3://*) ;; *) echo "datagen-task: --dataset must be an s3:// URI" >&2; exit 2 ;; esac
: "${OUT_DIR:?OUT_DIR is not set}"

# x86-64-v3 for the c8a instances; a caller may set its own (a local test on another CPU).
export RUSTFLAGS="${RUSTFLAGS--C target-cpu=x86-64-v3}" CARGO_TERM_COLOR=never
work=$(mktemp -d /tmp/datagen.XXXXXX)
shards="$work/shards"
stop="$work/STOP"
mkdir -p "$shards"
t0=$(date +%s)
since() { echo $(( $(date +%s) - t0 )); }

generation=$(awk -F= '/^generation *=/ {gsub(/[ \t]/, "", $2); print $2; exit}' datagen.toml)
prefix="$dataset/gen=$generation/run=$run/task=$task"
cores=$(nproc)
sha=$(git rev-parse HEAD)
echo "== datagen: run $run task $task, generation $generation, $cores workers, $minutes minutes, seed $seed"
echo "   commit ${sha:0:7}; uploading to $prefix/"

cargo build --release --locked --quiet
cargo build --release --locked --quiet -p rusty-chess-tools --bin datagen
bench=$(target/release/rusty-chess bench | awk -F: '/Nodes searched/ {gsub(/ /, "", $2); print $2}')
echo "engine $(target/release/rusty-chess --version), bench $bench ($(since)s)"

uploaded=0
# The AWS CLI uploads a directory's files concurrently; 32 at once keeps a
# batch of shards well ahead of 64 workers (about 66 shards a minute).
aws configure set default.s3.max_concurrent_requests 32
retry() {
  local i
  for i in 1 2 3; do
    "$@" && return 0
    echo "datagen-task: retrying after a failed upload ($i)" >&2
    sleep 5
  done
  return 1
}
# Upload every complete shard in one batch: move them into a staging
# directory, gzip in parallel, then three recursive copies (positions, games,
# then manifests, so a manifest never lands before its data). One shard at a
# time with three CLI calls each managed about 24 shards a minute, less than a
# 64-core instance produces.
upload_ready() {
  local batch="$work/batch" m name names=()
  for m in "$shards"/*.json; do
    [ -e "$m" ] || break
    names+=("$(basename "$m" .json)")
  done
  [ "${#names[@]}" -gt 0 ] || return 0
  rm -rf "$batch"
  mkdir -p "$batch/positions" "$batch/games" "$batch/manifests"
  for name in "${names[@]}"; do
    mv "$shards/$name.positions.txt" "$batch/positions/$name.txt"
    mv "$shards/$name.games.jsonl" "$batch/games/$name.jsonl"
    mv "$shards/$name.json" "$batch/manifests/$name.json"
  done
  find "$batch/positions" "$batch/games" -type f -print0 | xargs -0 -n 8 -P 8 gzip -6
  retry aws s3 cp --recursive --only-show-errors "$batch/positions" "$prefix/positions/"
  retry aws s3 cp --recursive --only-show-errors "$batch/games" "$prefix/games/"
  retry aws s3 cp --recursive --only-show-errors --content-type application/json "$batch/manifests" "$prefix/manifests/"
  uploaded=$((uploaded + ${#names[@]}))
  local ts
  ts=$(date -u +%FT%TZ)
  for name in "${names[@]}"; do echo "$ts $name"; done >> "$OUT_DIR/uploaded.log"
  rm -rf "$batch"
}

target/release/datagen --config datagen.toml --out "$shards" --run "$run" --task "$task" \
  --threads "$cores" --seed "$seed" --max-minutes "$minutes" --stop-file "$stop" \
  --engine-sha "$sha" --engine-bench "$bench" > >(tee "$OUT_DIR/datagen.log") 2>&1 &
# $! is datagen itself (not tee), so the wait below gets its exit code.
dg_pid=$!

interrupted=0
on_term() {
  interrupted=1
  echo "datagen-task: SIGTERM, stopping the workers and uploading complete shards"
  touch "$stop"
}
trap on_term TERM

while kill -0 "$dg_pid" 2>/dev/null; do
  upload_ready
  # In the background, so a SIGTERM runs the trap at once.
  sleep 15 & wait $! || true
done
set +e
wait "$dg_pid"; code=$?
set -e
upload_ready
trap - TERM

positions=$(grep -o 'done: [0-9]* games, [0-9]* positions' "$OUT_DIR/datagen.log" | awk '{print $4}')
games=$(grep -o 'done: [0-9]* games' "$OUT_DIR/datagen.log" | awk '{print $2}')
cat > "$OUT_DIR/task.json" <<JSON
{"schema": "rusty-chess-selfplay/1", "run": "$run", "task": "$task", "generation": $generation,
 "engine_sha": "$sha", "engine_bench": ${bench:-null}, "workers": $cores, "seed": $seed,
 "minutes": $minutes, "games": ${games:-0}, "positions": ${positions:-0}, "shards": $uploaded,
 "seconds": $(since), "interrupted": $([ "$interrupted" = 1 ] && echo true || echo false),
 "prefix": "$prefix/"}
JSON
cat "$OUT_DIR/task.json"
if [ "$interrupted" = 1 ]; then
  echo "datagen-task: interrupted; $uploaded complete shards are in S3"
  exit 143
fi
aws s3 cp --only-show-errors --content-type application/json "$OUT_DIR/task.json" "$prefix/task.json"
[ "$code" = 0 ] || { echo "datagen exited $code"; exit 1; }
echo "datagen-task: $uploaded shards, ${positions:-0} positions in $(since)s"
