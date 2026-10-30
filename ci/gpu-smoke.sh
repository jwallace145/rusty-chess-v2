#!/usr/bin/env bash
# The GPU profile's acceptance test, run by the `GPU smoke` workflow on one
# fresh g6.xlarge (jwall-engineering-infra's remote-task, profile gpu): the
# GPU and CUDA work, bullet's CUDA backend builds our trainer, and the
# dataset grants hold in both directions. A few minutes, well under a dollar.
# Starts in the repository root at the workflow's commit, with OUT_DIR and
# SCRATCH_DIR set. Never exits 75, which the runner reserves for interruptions.

set -euo pipefail
: "${OUT_DIR:?OUT_DIR is not set}"
scratch="${SCRATCH_DIR:-/tmp}/gpu-smoke"
mkdir -p "$scratch"
bucket=s3://jwall-engineering-rusty-chess-nnue
stamp=$(date -u +%Y%m%dT%H%M%SZ)
summary="$OUT_DIR/summary.txt"
: > "$summary"
pass=0 fail=0
check() {  # check NAME EXPECTED(ok|denied) COMMAND...
  local name=$1 want=$2; shift 2
  local got
  if "$@" > "$scratch/check.out" 2>&1; then got=ok; else got=denied; fi
  if [ "$got" = "$want" ]; then
    pass=$((pass + 1)); echo "PASS $name: $got (expected $want)" | tee -a "$summary"
  else
    fail=$((fail + 1)); echo "FAIL $name: $got (expected $want)" | tee -a "$summary"
    sed 's/^/    /' "$scratch/check.out" | head -5
  fi
}

echo "== GPU"
nvidia-smi | tee "$OUT_DIR/nvidia-smi.txt"
nvcc --version | tee -a "$summary"

echo "== CUDA: compile and run a kernel"
cat > "$scratch/add.cu" <<'CU'
#include <cstdio>
__global__ void add(const float* a, const float* b, float* c, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) c[i] = a[i] + b[i];
}
int main() {
    const int n = 1 << 20;
    float *a, *b, *c;
    cudaMallocManaged(&a, n * sizeof(float));
    cudaMallocManaged(&b, n * sizeof(float));
    cudaMallocManaged(&c, n * sizeof(float));
    for (int i = 0; i < n; i++) { a[i] = i; b[i] = 2.0f * i; }
    add<<<(n + 255) / 256, 256>>>(a, b, c, n);
    cudaError_t e = cudaDeviceSynchronize();
    if (e != cudaSuccess) { printf("CUDA error: %s\n", cudaGetErrorString(e)); return 1; }
    for (int i = 0; i < n; i++) if (c[i] != 3.0f * i) { printf("wrong at %d\n", i); return 1; }
    printf("kernel ok: %d additions on the GPU\n", n);
    return 0;
}
CU
check "a CUDA kernel compiles with nvcc" ok nvcc -O2 -o "$scratch/add" "$scratch/add.cu"
check "the kernel runs and adds correctly" ok "$scratch/add"

echo "== bullet's CUDA backend builds the trainer"
start=$(date +%s)
check "cargo build trainer --features cuda" ok \
  env CARGO_TARGET_DIR="$scratch/trainer-target" cargo build --release --locked --manifest-path trainer/Cargo.toml --features cuda
echo "   (build took $(( $(date +%s) - start ))s)" | tee -a "$summary"

echo "== dataset grants"
one=$(aws s3 ls "$bucket/data/selfplay/v1/gen=0/run=20261030-41007876197-1/task=t1/manifests/" | awk 'NR == 1 {print $4}')
check "list data/selfplay/" ok aws s3 ls "$bucket/data/selfplay/v1/gen=0/"
check "read an object under data/selfplay/" ok \
  aws s3 cp --only-show-errors "$bucket/data/selfplay/v1/gen=0/run=20261030-41007876197-1/task=t1/manifests/$one" "$scratch/"
echo "smoke test $stamp" > "$scratch/hello.txt"
check "write under data/nets/" ok aws s3 cp --only-show-errors "$scratch/hello.txt" "$bucket/data/nets/smoke-$stamp/hello.txt"
check "list data/ outside the grants" denied aws s3 ls "$bucket/data/"
check "write under data/selfplay/" denied \
  aws s3 cp --only-show-errors "$scratch/hello.txt" "$bucket/data/selfplay/SMOKE-TEST-MUST-BE-DENIED-$stamp.txt"

echo "== $pass passed, $fail failed" | tee -a "$summary"
[ "$fail" = 0 ]
