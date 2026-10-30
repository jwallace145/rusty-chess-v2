#!/usr/bin/env bash
# Download a sample of a Datagen run's shards for `datagen-audit`, flattened
# into one directory (<shard>.json, <shard>.positions.txt.gz,
# <shard>.games.jsonl.gz). Read-only: it never writes to the bucket. Needs AWS
# credentials that can read the dataset bucket (the owner's profile; the task
# instances cannot read it).
#
#   ci/datagen-sample.sh s3://BUCKET/PREFIX/selfplay/v1/gen=0/run=RUN/ OUT_DIR [WORKERS_PER_TASK] [SHARDS_PER_WORKER]
#
# Picks WORKERS_PER_TASK workers spread across each task (default 8) and their
# first SHARDS_PER_WORKER shards (default 3): about 1.8M positions from three
# 64-core tasks, enough for every statistic in docs/datagen-runbook.md.

set -euo pipefail
run="${1:?the S3 prefix of a run, ending in run=.../}"
out="${2:?an output directory}"
per_task="${3:-8}"
per_worker="${4:-3}"
run="${run%/}/"
mkdir -p "$out"

tasks=$(aws s3 ls "$run" | awk '/PRE task=/ {gsub(/PRE |\//, ""); print}')
[ -n "$tasks" ] || { echo "datagen-sample: no task= folders under $run" >&2; exit 1; }
n=0
for t in $tasks; do
  task=${t#task=}
  # Workers that have at least one complete shard (a manifest).
  # read -ra rather than mapfile: macOS ships bash 3.2.
  read -ra workers <<< "$(aws s3 ls "$run$t/manifests/" | awk '{print $4}' | sed -E 's/.*-w([0-9]+)-s[0-9]+\.json/\1/' | sort -u | tr '\n' ' ')"
  count=${#workers[@]}
  [ "$count" -gt 0 ] || continue
  step=$(( count / per_task )); [ "$step" -ge 1 ] || step=1
  for ((i = 0; i < count && i / step < per_task; i += step)); do
    w=${workers[$i]}
    for ((s = 0; s < per_worker; s++)); do
      name=$(printf '%s-w%s-s%05d' "$task" "$w" "$s")
      # Only shards whose manifest exists are complete.
      if aws s3 cp --only-show-errors "$run$t/manifests/$name.json" "$out/$name.json" 2>/dev/null; then
        aws s3 cp --only-show-errors "$run$t/positions/$name.txt.gz" "$out/$name.positions.txt.gz"
        aws s3 cp --only-show-errors "$run$t/games/$name.jsonl.gz" "$out/$name.games.jsonl.gz"
        n=$((n + 1))
      fi
    done
  done
done
echo "datagen-sample: $n shards in $out ($(du -sh "$out" | cut -f1))"
