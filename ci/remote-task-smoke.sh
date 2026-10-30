#!/bin/bash
# Smoke test for jwall-engineering-infra's remote tasks: prints what the box
# is, writes one output file, and ticks for a while so a run can be watched
# live (and cancelled mid-task). Argument: the number of 10-second ticks.
set -euo pipefail

ticks="${1:-6}"
echo "smoke: $(nproc) CPUs on $(hostname)"
lscpu | grep -E '^(Model name|Thread\(s\) per core|Core\(s\) per socket|Socket\(s\)):'
rustc --version
cargo --version

mkdir -p "$OUT_DIR"
{
  echo "hello from $(hostname)"
  echo "cpus=$(nproc)"
  date -u +%Y-%m-%dT%H:%M:%SZ
} > "$OUT_DIR/hello.txt"

for i in $(seq 1 "$ticks"); do
  echo "tick $i of $ticks"
  sleep 10
done
echo "smoke: done"
