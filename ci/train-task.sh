#!/usr/bin/env bash
# Train one NNUE net on a GPU instance, run by the Train workflow through
# jwall-engineering-infra's remote-task workflow (GPU profile). It starts in
# the repository root at the workflow's commit, with OUT_DIR set (and
# SCRATCH_DIR for the large files, /tmp if unset). docs/training.md describes
# the steps and the outputs.
#
#   ci/train-task.sh --dataset s3://BUCKET/data/selfplay/v1/gen=0/ [--dataset .../gen=1/ ...]
#                    --nets s3://BUCKET/data/nets
#                    --net-id ID --wdl λ --superbatches N
#                    [--batches-per-superbatch 6104] [--val-fraction 0.05]
#                    [--split-seed 1] [--shuffle-seed 1] [--backend cuda|metal]
#                    [--checkpoint-every 4] [--val-positions 4000000]
#
# 1. downloads every complete shard's positions and manifest under each
#    --dataset (repeat it to train on several generations at once);
# 2. prepares train.data and val.data: whole workers per split, a seeded
#    shuffle, every record parsed by bulletformat (rusty-chess-trainer prepare);
# 3. trains the (768 → 128) × 2 → 1 net (rusty-chess-trainer train), saving a
#    checkpoint every --checkpoint-every superbatches and scoring each on
#    --val-positions held-out records and as many training records;
# 4. checks the engine's quantized evaluation against the trainer's float one
#    on held-out positions (nnue-eval --check); a mismatch fails the task;
# 5. writes training.json (data, split, schedule, versions, hardware, losses,
#    the validation curve, the check) and uploads the net and its records to
#    --nets/<net id>/,
#    refusing to overwrite a net that exists. Never exits 75, which the runner
#    reserves for spot interruptions.

set -euo pipefail

datasets=() nets="" net_id="" wdl="" superbatches="" batches=6104 val_fraction=0.05 split_seed=1 shuffle_seed=1 backend=cuda
checkpoint_every=4 val_positions=4000000
while [ $# -gt 0 ]; do
  case "$1" in
    --dataset) datasets+=("${2%/}"); shift 2 ;;
    --nets) nets="${2%/}"; shift 2 ;;
    --net-id) net_id="$2"; shift 2 ;;
    --wdl) wdl="$2"; shift 2 ;;
    --superbatches) superbatches="$2"; shift 2 ;;
    --batches-per-superbatch) batches="$2"; shift 2 ;;
    --val-fraction) val_fraction="$2"; shift 2 ;;
    --split-seed) split_seed="$2"; shift 2 ;;
    --shuffle-seed) shuffle_seed="$2"; shift 2 ;;
    --backend) backend="$2"; shift 2 ;;
    --checkpoint-every) checkpoint_every="$2"; shift 2 ;;
    --val-positions) val_positions="$2"; shift 2 ;;
    *) echo "train-task: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ "${#datasets[@]}" -gt 0 ] || { echo "train-task: --dataset is required" >&2; exit 2; }
for v in nets net_id wdl superbatches; do
  [ -n "${!v}" ] || { echo "train-task: --${v//_/-} is required" >&2; exit 2; }
done
[[ "$net_id" =~ ^[a-z0-9][a-z0-9._-]*$ ]] || { echo "train-task: --net-id must be lower-case letters, digits, '.', '_' or '-'" >&2; exit 2; }
: "${OUT_DIR:?OUT_DIR is not set}"
scratch="${SCRATCH_DIR:-/tmp}/train-$net_id"
mkdir -p "$scratch"
t0=$(date +%s)
since() { echo $(( $(date +%s) - t0 )); }

# Append-only: a net id is used once. The GPU role may write data/nets/ but
# not list it, so on a GPU task this finds nothing and uniqueness comes from
# the Train workflow, which ends every id with its run id; with credentials
# that can list (a local run), it refuses an id already used.
if aws s3 ls "$nets/$net_id/" >/dev/null 2>&1; then
  echo "train-task: $nets/$net_id/ already exists; choose another --net-id" >&2
  exit 1
fi

sha=$(git rev-parse HEAD)
gpu="none"; driver=""; cuda=""
if command -v nvidia-smi >/dev/null; then
  gpu=$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader | head -1)
  driver=$(nvidia-smi --query-gpu=driver_version --format=csv,noheader | head -1)
  cuda=$(nvcc --version 2>/dev/null | sed -n 's/.*release \([0-9.]*\).*/\1/p' || true)
fi
echo "== train: $net_id, λ $wdl, $superbatches superbatches of $batches batches; commit ${sha:0:7}; GPU: $gpu"

echo "-- building ($(since)s)"
export CARGO_TERM_COLOR=never
# bullet's CUDA build script links from $CUDA_PATH/lib64 and panics without it;
# the GPU image puts nvcc on PATH but doesn't export it.
if [ "$backend" = cuda ] && [ -z "${CUDA_PATH:-}" ]; then
  CUDA_PATH=$(dirname "$(dirname "$(readlink -f "$(command -v nvcc)")")")
  export CUDA_PATH
fi
(cd trainer && cargo build --release --locked --quiet --features "$backend")
cargo build --release --locked --quiet -p rusty-chess-tools --bin nnue-eval

# One directory per dataset (its generation's folder name, gen=N), so shards
# of different generations never share a path; prepare walks them all, and
# every worker's key includes its run, so the split stays per worker.
for d in "${datasets[@]}"; do
  echo "-- downloading positions and manifests from $d ($(since)s)"
  aws s3 sync --only-show-errors "$d/" "$scratch/shards/$(basename "$d")/" \
    --exclude "*" --include "*/manifests/*.json" --include "*/positions/*.txt.gz"
done

echo "-- preparing ($(since)s)"
trainer/target/release/rusty-chess-trainer prepare --shards "$scratch/shards" --out "$scratch/data" \
  --val-fraction "$val_fraction" --split-seed "$split_seed" --shuffle-seed "$shuffle_seed"
rm -rf "$scratch/shards"

echo "-- training ($(since)s)"
threads=$(nproc 2>/dev/null || sysctl -n hw.ncpu)
trainer/target/release/rusty-chess-trainer train --data "$scratch/data" --net-id "$net_id" --wdl "$wdl" \
  --superbatches "$superbatches" --batches-per-superbatch "$batches" --threads "$threads" --out "$scratch/out" \
  --checkpoint-every "$checkpoint_every" --val-positions "$val_positions" \
  2>&1 | tee "$scratch/train.log"
trained=$(since)

echo "-- checking the engine's evaluation against the trainer's ($(since)s)"
set +e
check=$(target/release/nnue-eval --net "$scratch/out/net.nnue" --check "$scratch/out/eval_check.json" 2>&1)
check_code=$?
set -e
echo "$check" | tee "$scratch/check.txt"

net_sha=$(sha256sum "$scratch/out/net.nnue" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$scratch/out/net.nnue" | cut -d' ' -f1)
bullet_rev=$(sed -n 's/^bullet_lib = .*rev = "\([0-9a-f]*\)".*/\1/p' trainer/Cargo.toml)
python3 - "$scratch" "$OUT_DIR/training.json" "${datasets[@]}" <<PY
import json, sys
scratch, out, datasets = sys.argv[1], sys.argv[2], [d + "/" for d in sys.argv[3:]]
check = open(f"{scratch}/check.txt").read().strip().splitlines()
split = json.load(open(f"{scratch}/data/split.json"))
log = open(f"{scratch}/out/$net_id-$superbatches/log.txt").read().split()
losses = [float(l.split(",")[2]) for l in log if l.count(",") == 2]
json.dump({
    "schema": "rusty-chess-net/2",
    "net_id": "$net_id",
    "net_sha256": "$net_sha",
    "architecture": "(768 -> 128) x 2 -> 1, SCReLU, Chess768 inputs, QA 255, QB 64, SCALE 400",
    "datasets": datasets,
    "split": {k: v for k, v in split.items() if k != "val_workers"},
    "val_workers": split["val_workers"],
    "schedule": {"wdl": $wdl, "eval_scale": 400, "superbatches": $superbatches, "batches_per_superbatch": $batches,
                 "batch_size": 16384, "lr": 0.001, "lr_gamma": 0.1, "lr_step": max(1, $superbatches * 9 // 20),
                 "optimiser": "AdamW (bullet defaults, weights clipped to ±1.98)",
                 "loss": "sigmoid(output) squared error against λ·result + (1 − λ)·sigmoid(score / 400)"},
    "versions": {"engine_commit": "$sha", "bullet_rev": "$bullet_rev", "bulletformat": "1.8.0", "backend": "$backend"},
    "hardware": {"gpu": "$gpu", "nvidia_driver": "$driver", "cuda": "$cuda"},
    "losses": {"first": losses[0] if losses else None, "last": losses[-1] if losses else None, "points": len(losses)},
    "validation": json.load(open(f"{scratch}/out/validation.json")),
    "check": check[-1] if check else "",
    "check_passed": $check_code == 0,
    "seconds": {"to_trained": $trained},
}, open(out, "w"), indent=2)
PY
cp "$scratch/out/net.nnue" "$scratch/out/eval_check.json" "$scratch/out/val_losses.txt" "$scratch/data/split.json" "$OUT_DIR/"
cp "$scratch/out/$net_id-$superbatches/log.txt" "$OUT_DIR/losses.txt"
cp "$scratch/train.log" "$OUT_DIR/train.log"
if [ "$check_code" != 0 ]; then
  echo "train-task: the engine's evaluation disagrees with the trainer's beyond quantization error; not uploading"
  exit 1
fi

echo "-- uploading to $nets/$net_id/ ($(since)s)"
for f in net.nnue training.json eval_check.json val_losses.txt split.json losses.txt train.log; do
  aws s3 cp --only-show-errors "$OUT_DIR/$f" "$nets/$net_id/$f"
done
echo "train-task: $net_id trained and checked in $(since)s; net sha256 $net_sha"
