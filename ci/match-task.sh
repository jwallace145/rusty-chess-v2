#!/usr/bin/env bash
# A head-to-head match between any two commits, run by the Match workflow on a
# dedicated instance through jwall-engineering-infra's remote-task workflow
# (or on a GitHub-hosted runner as the fallback). It starts in the repository
# root at the workflow's commit, with OUT_DIR set; everything written there
# comes back as the job's artifact.
#
#   ci/match-task.sh --candidate SHA --baseline SHA [--candidate-label REF] [--baseline-label REF]
#                    [--tc 10+0.1 | --nodes N] --games G --hash MB --confidence 95|99|99.9
#                    [--note TEXT] [--concurrency N]
#
# Both commits are fetched anonymously by full SHA and built at once; the
# report tool comes from the workflow's commit. Games at once default to the
# physical cores less four (60 on a c8a.16xlarge). Writes match.log,
# match.pgn, match.md and timing.txt to OUT_DIR, and exits 0 once the games
# are played: a match is a measurement, not a gate. Never exits 75, which the
# runner reserves for spot interruptions.

set -euo pipefail

candidate="" baseline="" cand_label="" base_label="" tc="10+0.1" nodes="" games="" hash="64" confidence="99" note="" concurrency=""
while [ $# -gt 0 ]; do
  case "$1" in
    --candidate) candidate="$2"; shift 2 ;;
    --baseline) baseline="$2"; shift 2 ;;
    --candidate-label) cand_label="$2"; shift 2 ;;
    --baseline-label) base_label="$2"; shift 2 ;;
    --tc) tc="$2"; shift 2 ;;
    --nodes) nodes="$2"; shift 2 ;;
    --games) games="$2"; shift 2 ;;
    --hash) hash="$2"; shift 2 ;;
    --confidence) confidence="$2"; shift 2 ;;
    --note) note="$2"; shift 2 ;;
    --concurrency) concurrency="$2"; shift 2 ;;
    *) echo "match-task: unknown argument $1" >&2; exit 2 ;;
  esac
done
[ -n "$candidate" ] && [ -n "$baseline" ] && [ -n "$games" ] || { echo "match-task: --candidate, --baseline and --games are required" >&2; exit 2; }
: "${OUT_DIR:?OUT_DIR is not set}"
cand_label=${cand_label:-${candidate:0:7}}
base_label=${base_label:-${baseline:0:7}}

FASTCHESS_VERSION=v1.8.2-alpha
REPO_URL=https://github.com/jwallace145/rusty-chess-v2
OPENINGS=tests/openings/openings.epd
export RUSTFLAGS="-C target-cpu=x86-64-v3" CARGO_TERM_COLOR=never
work=$(mktemp -d /tmp/match.XXXXXX)
t0=$(date +%s)
since() { echo $(( $(date +%s) - t0 )); }

cores=$(nproc)
if [ -z "$concurrency" ]; then
  concurrency=$(( cores - 4 ))
  [ "$concurrency" -ge 1 ] || concurrency=1
fi
cpu=$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs)
echo "== match: $cand_label (${candidate:0:7}) vs $base_label (${baseline:0:7}), ${nodes:+$nodes nodes per move}${nodes:-$tc}, $games games, $confidence% confidence"
echo "   $cpu, $cores cores, $concurrency games at once${note:+; note: $note}"

# Validate the settings before spending minutes on builds.
cargo build --release --locked --quiet -p rusty-chess-tools --bin match-report
settings=(--tc "$tc" --nodes "$nodes" --games "$games" --concurrency "$concurrency" --hash "$hash" --openings "$OPENINGS" --confidence "$confidence")
read -ra args <<< "$(target/release/match-report args "${settings[@]}")"

echo "-- fetching and building both engines ($(since)s)"
for side in candidate baseline; do
  sha=${!side}
  git fetch --quiet --depth 1 "$REPO_URL" "$sha"
  git worktree add --quiet --detach "$work/$side" "$sha"
done
pids=()
for side in candidate baseline; do
  cargo build --release --locked --quiet --manifest-path "$work/$side/Cargo.toml" --target-dir "$work/$side-target" > "$work/build-$side.log" 2>&1 &
  pids+=($!)
done
ok=0
for i in 0 1; do
  side=$([ "$i" = 0 ] && echo candidate || echo baseline)
  wait "${pids[$i]}" || { echo "$side build failed:"; tail -40 "$work/build-$side.log"; ok=1; }
done
[ "$ok" = 0 ] || exit 1
mkdir -p "$work/engines"
declare -A bench
for side in candidate baseline; do
  cp "$work/$side-target/release/rusty-chess" "$work/engines/$side"
  out=$("$work/engines/$side" bench)
  bench[$side]=$(awk -F: '/Nodes searched/ {gsub(/ /, "", $2); print $2}' <<< "$out")
  echo "$side: $("$work/engines/$side" --version), bench signature ${bench[$side]}"
done
built=$(since)

echo "-- fetching fastchess $FASTCHESS_VERSION ($(since)s)"
curl -sSfL -o "$work/fastchess.tar" "https://github.com/Disservin/fastchess/releases/download/${FASTCHESS_VERSION}/fastchess-linux-x86-64.tar"
mkdir -p "$work/fc" && tar -xf "$work/fastchess.tar" -C "$work/fc"
fc=$(find "$work/fc" -type f -name 'fastchess*' | head -1)
chmod +x "$fc"

echo "-- playing ($(since)s)"
play_start=$(date +%s)
log="$OUT_DIR/match.log"
"$fc" -engine cmd="$work/engines/candidate" name=candidate -engine cmd="$work/engines/baseline" name=baseline \
  "${args[@]}" -pgnout file="$OUT_DIR/match.pgn" > "$log" 2>&1 &
fc_pid=$!
trap 'kill "$fc_pid" 2>/dev/null; echo "match-task: stopped by SIGTERM"; exit 143' TERM
shown=0
while kill -0 "$fc_pid" 2>/dev/null; do
  # In the background, so a SIGTERM runs the trap at once rather than after the sleep.
  sleep 30 & wait $! || true
  n=$(grep -c '^Results of' "$log" || true)
  if [ "$n" -gt "$shown" ]; then
    shown=$n
    played=$(grep -c '^Finished game' "$log" || true)
    secs=$(( $(date +%s) - play_start ))
    echo "  [${secs}s, $played of $games games, $(( played * 60 / (secs > 0 ? secs : 1) )) games/min]"
    awk '/^Results of/ {b=""} /^(Elo|LOS|Games):/ {b=b $0 "\n"} END {printf "%s", b}' "$log" | sed 's/^/  /'
  fi
done
set +e
wait "$fc_pid"; code=$?
set -e
trap - TERM
play_secs=$(( $(date +%s) - play_start ))
played=$(grep -c '^Finished game' "$log" || true)
echo "fastchess exit $code"
{
  echo "cores=$cores"
  echo "concurrency=$concurrency"
  echo "setup_seconds=$built"
  echo "play_seconds=$play_secs"
  echo "games=$played"
  awk -v g="$played" -v s="$play_secs" 'BEGIN { if (s > 0) printf "games_per_minute=%.1f\n", g * 60 / s }'
  echo "total_seconds=$(since)"
} | tee "$OUT_DIR/timing.txt"
[ "$code" = 0 ] || exit 1

target/release/match-report render "${settings[@]}" --log "$log" \
  --candidate "$cand_label" --candidate-sha "$candidate" --candidate-bench "${bench[candidate]}" \
  --baseline "$base_label" --baseline-sha "$baseline" --baseline-bench "${bench[baseline]}" \
  --cpu "$cpu" --fastchess "$FASTCHESS_VERSION" --note "$note" > "$OUT_DIR/match.md"
cat "$OUT_DIR/match.md"
