#!/usr/bin/env bash
# dispatch_match.sh LABEL CANDIDATE BASELINE NODES GAMES NOTE
# Dispatches the Match workflow from main and retries (up to 3 times) when the
# settings job dies before an instance is launched (the SIGILL cache fault).
# Appends "LABEL attempt RUN_ID outcome" lines to runs/matches.log.
main() {
  local S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
  local label=$1 cand=$2 base=$3 nodes=$4 games=$5 note=$6 attempt rid st
  cd /Users/jimmywallace/Projects/chess/rusty-chess-v2 || exit 1
  for attempt in 1 2 3; do
    gh workflow run Match --ref main -f candidate="$cand" -f baseline="$base" -f nodes="$nodes" -f games="$games" -f note="$note (attempt $attempt)" > /dev/null 2>&1
    sleep 12
    rid=$(gh run list --workflow Match --limit 1 --json databaseId --jq '.[0].databaseId')
    # wait for the settings job to finish
    for _ in $(seq 1 30); do
      st=$(gh run view "$rid" --json jobs --jq '.jobs[] | select(.name | startswith("Resolve")) | .conclusion' 2>/dev/null)
      [ -n "$st" ] && break
      sleep 10
    done
    echo "$(date -u +%H:%M:%S) $label attempt $attempt run $rid settings=$st" >> "$S/runs/matches.log"
    [ "$st" = "success" ] && { echo "$rid"; return 0; }
  done
  echo "FAILED"; return 1
}
main "$@"
