#!/usr/bin/env bash
# Local exploratory screen (NOT the record): a lab binary with options against
# the same binary with defaults (or BASEOPTS), fixed nodes per move, the CI
# SPRT's openings and adjudication.
# Usage: screen.sh NAME NODES MAX_ROUNDS "OPT=V OPT=V ..." [BASEOPTS] [CONCURRENCY] [BIN_A] [BIN_B]
# The body is one function, read whole before it runs, and this file is only
# ever replaced by `mv`: editing a running bash script in place corrupts the
# run (it cost the first s1-lmr-cut screen its log).
set -euo pipefail
main() {
  local S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
  local defbin; defbin=$S/bin/$(cat $S/runs/default-bin 2>/dev/null || echo lab)
  local name=$1 nodes=$2 rounds=$3 opts=${4:-} baseopts=${5:-} conc=${6:-12} bina=${7:-$defbin} binb=${8:-$defbin}
  local out=$S/runs/$name; mkdir -p "$out"
  local a=() b=() o
  for o in $opts; do a+=("option.L_$o"); done
  for o in $baseopts; do b+=("option.L_$o"); done
  local start; start=$(date +%s)
  cd "$out"
  "$S/fastchess-src/fastchess" \
    -engine cmd="$bina" name=cand ${a[@]+"${a[@]}"} \
    -engine cmd="$binb" name=base ${b[@]+"${b[@]}"} \
    -each tc=600+6 nodes="$nodes" \
    -openings file=/Users/jimmywallace/Projects/chess/rusty-chess-v2/tests/openings/openings.epd format=epd order=random \
    -games 2 -repeat -rounds "$rounds" -concurrency "$conc" -maxmoves 200 \
    -draw movenumber=40 movecount=8 score=10 -resign movecount=3 score=800 twosided=true \
    -sprt elo0=0 elo1=5 alpha=0.05 beta=0.05 -ratinginterval 100 -report penta=true -recover \
    > "fc-$start.log" 2>&1 || true
  cp "fc-$start.log" fc.log
  local secs=$(( $(date +%s) - start ))
  local elo los games penta llr
  elo=$(grep '^Elo:' fc.log | tail -1 | sed 's/Elo: //;s/, nElo.*//')
  los=$(grep '^LOS:' fc.log | tail -1 | sed 's/,.*//;s/LOS: //')
  games=$(grep '^Games:' fc.log | tail -1 | sed 's/Games: //;s/, Points.*//')
  penta=$(grep '^Ptnml' fc.log | tail -1 | sed 's/.*\[\(.*\)\].*/\1/')
  llr=$(grep '^LLR:' fc.log | tail -1 | sed 's/LLR: //;s/ (-2.94.*//')
  printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%ss\t[%s]\t[%s]\t%s\n' "$name" "$nodes" "$elo" "$los" "$games" "$penta" "$llr" "$secs" "$opts" "$baseopts" "$(basename "$bina")" | tee -a "$S/runs/summary.tsv"
}
main "$@"
exit 0
