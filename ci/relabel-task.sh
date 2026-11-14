#!/usr/bin/env bash
# Re-score stored self-play positions with this commit's default net, on a
# dedicated instance, run by the Relabel workflow through
# jwall-engineering-infra's remote-task workflow (profile relabel). It starts
# in the repository root at the workflow's commit, with OUT_DIR set; the labels
# go to S3, and OUT_DIR carries only the task's summary and log.
# docs/relabel.md is the design.
#
#   ci/relabel-task.sh --dataset s3://BUCKET/data/selfplay/v1
#                      --labels s3://BUCKET/data/selfplay/relabel
#                      --generations 0,1,2 --index I --tasks N --minutes M
#                      [--nodes 5000] [--target-cpu native|x86-64-v3]
#
# The unit of work is one self-play worker's shards in one task: this task
# takes every Nth worker, starting at I, so N tasks share a generation without
# talking to each other. It groups its workers into batches of at least four
# shards a core, because a worker alone can have far fewer shards than the
# instance has cores, and the rest would sit idle. For each batch it downloads
# the shards, relabels them on every core, and uploads each shard's positions
# and then its manifest, so a manifest's presence in S3 means the shard is
# complete.
#
# Resumable: shards whose relabelled manifest is already in S3 are skipped, so
# a re-run or a top-up only does what is left. It stops starting batches after
# --minutes. On SIGTERM (a spot interruption: about two minutes' notice) it
# stops the relabelling, uploads every complete shard and exits; a shard in
# progress is lost, nothing is corrupted. The originals are only ever read.
# Never exits 75, which the runner reserves for interruptions.

set -euo pipefail
# Byte order for every sort: the split into tasks must be the same everywhere,
# including in ci/relabel-manager.py.
export LC_ALL=C

dataset="" labels="" generations="" index="" tasks="" minutes="" nodes=5000 target_cpu=native
while [ $# -gt 0 ]; do
  case "$1" in
    --dataset) dataset="${2%/}"; shift 2 ;;
    --labels) labels="${2%/}"; shift 2 ;;
    --generations) generations="$2"; shift 2 ;;
    --index) index="$2"; shift 2 ;;
    --tasks) tasks="$2"; shift 2 ;;
    --minutes) minutes="$2"; shift 2 ;;
    --nodes) nodes="$2"; shift 2 ;;
    --target-cpu) target_cpu="$2"; shift 2 ;;
    *) echo "relabel-task: unknown argument $1" >&2; exit 2 ;;
  esac
done
for v in dataset labels generations index tasks minutes; do
  [ -n "${!v}" ] || { echo "relabel-task: --$v is required" >&2; exit 2; }
done
case "$dataset" in s3://*) ;; *) echo "relabel-task: --dataset must be an s3:// URI" >&2; exit 2 ;; esac
case "$labels" in s3://*) ;; *) echo "relabel-task: --labels must be an s3:// URI" >&2; exit 2 ;; esac
[[ "$generations" =~ ^[0-9]+(,[0-9]+)*$ ]] || { echo "relabel-task: --generations must be numbers separated by commas" >&2; exit 2; }
[[ "$target_cpu" =~ ^[a-z0-9_-]+$ ]] || { echo "relabel-task: --target-cpu must be a CPU name" >&2; exit 2; }
: "${OUT_DIR:?OUT_DIR is not set}"

export RUSTFLAGS="${RUSTFLAGS--C target-cpu=$target_cpu}" CARGO_TERM_COLOR=never
work=$(mktemp -d "${SCRATCH_DIR:-/tmp}/relabel.XXXXXX")
t0=$(date +%s)
since() { echo $(( $(date +%s) - t0 )); }
sha=$(git rev-parse HEAD)

cargo build --release --locked --quiet -p braveheart-tools --bin relabel
label_id=$(target/release/relabel --net default --nodes "$nodes" --print-label-id)
out_root="$labels/$label_id"
echo "== relabel: $label_id, generations $generations, task $index of $tasks, $minutes minutes; commit ${sha:0:7} ($(since)s)"
echo "   from $dataset/, to $out_root/"

aws configure set default.s3.max_concurrent_requests 32
retry() {
  local i
  for i in 1 2 3; do
    "$@" && return 0
    echo "relabel-task: retrying after a failed transfer ($i)" >&2
    sleep 5
  done
  return 1
}

# Every source manifest, and every relabelled one already done, as
# gen=N/run=R/task=T/<shard>.
list_shards() { # s3://root, then generations
  local root=$1 g
  local key_prefix=${root#s3://*/}
  IFS=',' read -ra gens <<< "$generations"
  for g in "${gens[@]}"; do
    aws s3 ls --recursive "$root/gen=$g/" \
      | awk '{print $4}' | grep '/manifests/.*\.json$' \
      | sed -e "s#^$key_prefix/##" -e 's#/manifests/#/#' -e 's#\.json$##' || true
  done
}
list_shards "$dataset" | sort > "$work/source.txt"
list_shards "$out_root" | sort > "$work/done.txt"
comm -23 "$work/source.txt" "$work/done.txt" > "$work/todo.txt"
echo "   $(wc -l < "$work/source.txt") shards, $(wc -l < "$work/done.txt") already relabelled ($(since)s)"

# A worker is its shards' name up to -s<seq>: gen=N/run=R/task=T/t0-w017. Every
# Nth worker of the whole list is this task's, so the split doesn't depend on
# what is already done, and a top-up with the same --tasks repeats it.
sed -E 's/-s[0-9]+$//' "$work/source.txt" | sort -u \
  | awk -v i="$index" -v n="$tasks" '(NR - 1) % n == i' > "$work/mine.txt"

# Enough shards that a batch keeps every core busy until its last round.
batch_target=$(( $(getconf _NPROCESSORS_ONLN) * 4 ))

stop=0
child=0
on_term() {
  stop=1
  echo "relabel-task: SIGTERM, stopping and uploading complete shards"
  [ "$child" = 0 ] || kill "$child" 2>/dev/null || true
}
trap on_term TERM

uploaded=0 workers=0 records=0 changed=0 mates=0 sum_change=0
upload_done() { # a task directory, relative to the root
  local rel=$1
  local dir="$work/out/$rel" fresh="$work/fresh" m n=0
  [ -d "$dir/manifests" ] || return 0
  rm -rf "$fresh"; mkdir -p "$fresh/positions" "$fresh/manifests"
  for m in "$dir"/manifests/*.json; do
    [ -e "$m" ] || break
    local shard
    shard=$(basename "$m" .json)
    # Only what this run made: done shards were skipped and have no positions.
    [ -e "$dir/positions/$shard.txt.gz" ] || continue
    mv "$dir/positions/$shard.txt.gz" "$fresh/positions/"
    mv "$m" "$fresh/manifests/"
    n=$((n + 1))
  done
  [ "$n" -gt 0 ] || return 0
  # The relabel block's statistics, one field a line in the pretty JSON.
  read -r r c m2 a2 < <(cat "$fresh"/manifests/*.json | awk -F'[:,]' '
    /"records":/ {r += $2} /"changed":/ {c += $2} /"mates_clamped":/ {m += $2} /"sum_abs_change":/ {a += $2}
    END {printf "%d %d %d %d\n", r, c, m, a}')
  records=$((records + r)) changed=$((changed + c)) mates=$((mates + m2)) sum_change=$((sum_change + a2))
  # Positions first: a manifest never lands before its data.
  retry aws s3 cp --recursive --only-show-errors "$fresh/positions" "$out_root/$rel/positions/"
  retry aws s3 cp --recursive --only-show-errors --content-type application/json "$fresh/manifests" "$out_root/$rel/manifests/"
  uploaded=$((uploaded + n))
}

download_batch() { # the batch's workers, one sync per task directory
  local rel unit name
  for rel in $(sed 's#/[^/]*$##' "$work/batch.txt" | sort -u); do
    local done_args=() source_args=()
    while IFS= read -r unit; do
      [ "${unit%/*}" = "$rel" ] || continue
      name=${unit##*/}
      done_args+=(--include "$name-s*.json")
      source_args+=(--include "manifests/$name-s*.json" --include "positions/$name-s*" --include "games/$name-s*")
    done < "$work/batch.txt"
    mkdir -p "$work/in/$rel" "$work/out/$rel/manifests"
    # A partly done worker: its finished manifests make the tool skip them.
    retry aws s3 sync --only-show-errors "$out_root/$rel/manifests/" "$work/out/$rel/manifests/" \
      --exclude "*" "${done_args[@]}"
    retry aws s3 sync --only-show-errors "$dataset/$rel/" "$work/in/$rel/" --exclude "*" "${source_args[@]}"
    [ "$stop" = 0 ] || return 0
  done
}

run_batch() {
  [ -s "$work/batch.txt" ] || return 0
  if [ $(( $(since) / 60 )) -ge "$minutes" ]; then
    echo "relabel-task: $minutes minutes are up, stopping before worker $((at - $(wc -l < "$work/batch.txt") + 1)) of $total"
    stop=2
    return 0
  fi
  rm -rf "$work/in" "$work/out"
  download_batch
  [ "$stop" = 0 ] || return 0
  # In the background, in its own session, so a SIGTERM reaches only this
  # script and the trap can stop it and upload what is complete.
  if command -v setsid >/dev/null; then
    setsid target/release/relabel --in "$work/in" --out "$work/out" --net default --nodes "$nodes" --engine-sha "$sha" \
      >> "$OUT_DIR/relabel.log" 2>&1 &
  else
    target/release/relabel --in "$work/in" --out "$work/out" --net default --nodes "$nodes" --engine-sha "$sha" \
      >> "$OUT_DIR/relabel.log" 2>&1 &
  fi
  child=$!
  set +e
  wait "$child"; code=$?
  # A trap interrupts `wait`; wait again for the child to finish dying.
  [ "$stop" = 0 ] || wait "$child" 2>/dev/null
  set -e
  child=0
  local rel
  for rel in $(sed 's#/[^/]*$##' "$work/batch.txt" | sort -u); do
    upload_done "$rel"
  done
  workers=$((workers + $(wc -l < "$work/batch.txt")))
  echo "   worker $at of $total: $uploaded shards, $records records so far, $(( records / ($(since) + 1) )) records/s ($(since)s)"
  if [ "$stop" = 0 ] && [ "$code" != 0 ]; then
    tail -3 "$OUT_DIR/relabel.log"
    echo "relabel-task: relabel exited $code on the batch ending at worker $at" >&2
    exit 1
  fi
  : > "$work/batch.txt"
  batch_shards=0
}

total=$(( $(wc -l < "$work/mine.txt") ))
at=0
batch_shards=0
: > "$work/batch.txt"
while IFS= read -r unit && [ "$stop" = 0 ]; do
  at=$((at + 1))
  # Nothing left of this worker: don't download it.
  left=$(grep -c -F "$unit-s" "$work/todo.txt" || true)
  [ "$left" -gt 0 ] || continue
  echo "$unit" >> "$work/batch.txt"
  batch_shards=$((batch_shards + left))
  [ "$batch_shards" -lt "$batch_target" ] || run_batch
done < "$work/mine.txt"
[ "$stop" != 0 ] || run_batch
trap - TERM

todo_at_start=$(( $(wc -l < "$work/todo.txt") ))
cat > "$OUT_DIR/task.json" <<JSON
{"schema": "rusty-chess-relabel/1", "label_id": "$label_id", "generations": "$generations",
 "index": $index, "tasks": $tasks, "nodes": $nodes, "engine_sha": "$sha", "minutes": $minutes,
 "workers": $workers, "shards": $uploaded, "records": $records, "changed": $changed,
 "sum_abs_change": $sum_change, "mates_clamped": $mates, "shards_todo_at_start": $todo_at_start,
 "seconds": $(since), "interrupted": $([ "$stop" = 1 ] && echo true || echo false),
 "out_of_time": $([ "$stop" = 2 ] && echo true || echo false), "prefix": "$out_root/"}
JSON
cat "$OUT_DIR/task.json"
if [ "$stop" = 1 ]; then
  echo "relabel-task: interrupted; $uploaded complete shards are in S3"
  exit 143
fi
echo "relabel-task: $uploaded shards, $records records in $(since)s"
