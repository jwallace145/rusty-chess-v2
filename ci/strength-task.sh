#!/usr/bin/env bash
# One leg of the SPRT on a dedicated instance, run by jwall-engineering-infra's
# remote-task workflow (docs/remote-tasks.md there). It starts in the
# repository root at the PR's head commit, with OUT_DIR set; everything written
# to OUT_DIR comes back as the job's artifact, also when the script fails or
# spot interrupts it.
#
#   ci/strength-task.sh --mode timed|nodes --pr SHA --base SHA [--quick true|false]
#
# Builds the PR and its merge base, plays them with fastchess at as many games
# at once as the box has physical cores less four, and writes sprt.log,
# sprt.pgn, sprt.md and timing.txt to OUT_DIR. Exits like the old Verdict
# step: the timed leg fails unless H1 was accepted (a smoke test may also be
# inconclusive); the fixed-node leg passes once its games are played. Never
# exits 75, which the runner reserves for spot interruptions.

set -euo pipefail

mode="" pr="" base="" quick="false"
while [ $# -gt 0 ]; do
  case "$1" in
    --mode) mode="$2"; shift 2 ;;
    --pr) pr="$2"; shift 2 ;;
    --base) base="$2"; shift 2 ;;
    --quick) quick="$2"; shift 2 ;;
    *) echo "strength-task: unknown argument $1" >&2; exit 2 ;;
  esac
done
case "$mode" in timed) flag="--timed" ;; nodes) flag="" ;; *) echo "strength-task: --mode must be timed or nodes" >&2; exit 2 ;; esac
[ -n "$pr" ] && [ -n "$base" ] || { echo "strength-task: --pr and --base are required" >&2; exit 2; }
: "${OUT_DIR:?OUT_DIR is not set}"

FASTCHESS_VERSION=v1.8.2-alpha
REPO_URL=https://github.com/jwallace145/rusty-chess-v2
export RUSTFLAGS="-C target-cpu=x86-64-v3" CARGO_TERM_COLOR=never
work=$(mktemp -d /tmp/strength.XXXXXX)
t0=$(date +%s)
since() { echo $(( $(date +%s) - t0 )); }

head=$(git rev-parse HEAD)
[ "$head" = "$pr" ] || echo "strength-task: note: checked out $head, asked for $pr"
cores=$(nproc)
concurrency=$(( cores - 4 ))
[ "$concurrency" -ge 1 ] || concurrency=1
echo "== strength task: $mode leg, PR ${pr:0:7} vs base ${base:0:7}, $cores cores, $concurrency games at once"

# Both engines build at once; on this many cores each takes well under a minute.
echo "-- building the PR engine and the tools, and the base engine ($(since)s)"
git fetch --quiet --depth 1 "$REPO_URL" "$base"
git worktree add --quiet --detach "$work/base" FETCH_HEAD
( cargo build --release --locked --quiet \
    && cargo build --release --locked --quiet -p rusty-chess-tools --bin sprt-report ) > "$work/build-pr.log" 2>&1 &
pr_build=$!
cargo build --release --locked --quiet --manifest-path "$work/base/Cargo.toml" --target-dir "$work/base-target" > "$work/build-base.log" 2>&1 &
base_build=$!
ok=0
wait "$pr_build" || { echo "PR build failed:"; tail -40 "$work/build-pr.log"; ok=1; }
wait "$base_build" || { echo "base build failed:"; tail -40 "$work/build-base.log"; ok=1; }
[ "$ok" = 0 ] || exit 1
mkdir -p "$work/engines"
cp target/release/rusty-chess "$work/engines/pr"
cp "$work/base-target/release/rusty-chess" "$work/engines/base"
signature() { "$1" bench | awk -F: '/Nodes searched/ {gsub(/ /, "", $2); print $2}'; }
echo "PR engine: $("$work/engines/pr" --version), bench $(signature "$work/engines/pr")"
echo "Base engine at ${base:0:7}: $("$work/engines/base" --version), bench $(signature "$work/engines/base")"
built=$(since)

echo "-- fetching fastchess $FASTCHESS_VERSION ($(since)s)"
curl -sSfL -o "$work/fastchess.tar" "https://github.com/Disservin/fastchess/releases/download/${FASTCHESS_VERSION}/fastchess-linux-x86-64.tar"
mkdir -p "$work/fc" && tar -xf "$work/fastchess.tar" -C "$work/fc"
fc=$(find "$work/fc" -type f -name 'fastchess*' | head -1)
chmod +x "$fc"

override="$flag --concurrency $concurrency"
smoke=""
if [ "$quick" = "true" ]; then
  override="$override --max-rounds 50"
  smoke="--smoke --max-rounds 50"
  echo "sprt-quick: capped at 50 pairs, a smoke test"
fi
# shellcheck disable=SC2086
read -ra args <<< "$(target/release/sprt-report args --config perf-targets.toml $override)"

# Only fastchess's running result is printed, every 30 seconds; the full log
# and the games are in OUT_DIR.
echo "-- playing ($(since)s): ${args[*]}"
play_start=$(date +%s)
log="$OUT_DIR/sprt.log"
"$fc" -engine cmd="$work/engines/pr" name=pr -engine cmd="$work/engines/base" name=base "${args[@]}" \
  -pgnout file="$OUT_DIR/sprt.pgn" > "$log" 2>&1 &
fc_pid=$!
# On SIGTERM (a spot interruption) stop the games; the log so far is already in
# OUT_DIR, for reading only.
trap 'kill "$fc_pid" 2>/dev/null; echo "strength-task: stopped by SIGTERM"; exit 143' TERM
shown=0
while kill -0 "$fc_pid" 2>/dev/null; do
  # In the background, so a SIGTERM runs the trap at once rather than after the sleep.
  sleep 30 & wait $! || true
  n=$(grep -c '^Results of' "$log" || true)
  if [ "$n" -gt "$shown" ]; then
    shown=$n
    games=$(grep -c '^Finished game' "$log" || true)
    mins=$(( ($(date +%s) - play_start + 59) / 60 ))
    echo "  [$(( $(date +%s) - play_start ))s, $games games, $(( games / mins )) games/min]"
    awk '/^Results of/ {b=""} /^(Elo|LOS|Games|LLR):/ {b=b $0 "\n"} END {printf "%s", b}' "$log" | sed 's/^/  /'
  fi
done
set +e
wait "$fc_pid"; code=$?
set -e
trap - TERM
play_secs=$(( $(date +%s) - play_start ))
games=$(grep -c '^Finished game' "$log" || true)
grep -E "SPRT .* completed|Tournament finished" "$log" | tail -2 || true
echo "fastchess exit $code"

{
  echo "mode=$mode"
  echo "cores=$cores"
  echo "concurrency=$concurrency"
  echo "setup_seconds=$built"
  echo "first_game_seconds=$(( play_start - t0 ))"
  echo "play_seconds=$play_secs"
  echo "games=$games"
  awk -v g="$games" -v s="$play_secs" 'BEGIN { if (s > 0) printf "games_per_minute=%.1f\n", g * 60 / s }'
  echo "total_seconds=$(since)"
} | tee "$OUT_DIR/timing.txt"
if [ "$code" != 0 ]; then exit 1; fi

# The verdict, as the GitHub-hosted Verdict step gives it. The PR comment is
# combined from both legs' logs by the workflow's comment job.
set +e
# shellcheck disable=SC2086
target/release/sprt-report render --config perf-targets.toml $flag $smoke --log "$log" \
  --head "$pr" --base "main" --fastchess "$FASTCHESS_VERSION" \
  --openings "$(grep -c . tests/openings/openings.epd)" > "$OUT_DIR/sprt.md"
status=$?
set -e
grep -m1 -E "H1 accepted|H0 accepted|inconclusive" "$OUT_DIR/sprt.md" || true
if [ "$mode" = "timed" ] && [ "$status" != 0 ]; then
  echo "The timed SPRT did not accept H1: only an accepted H1 passes the gate. The owner may merge over this for a reason stated in the PR."
  exit 1
fi
exit 0
