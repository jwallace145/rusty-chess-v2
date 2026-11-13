#!/usr/bin/env bash
# dispatch_match2.sh LABEL CANDIDATE BASELINE TC GAMES HASH CONFIDENCE NOTE
# A timed Match with an explicit hash, retried past the settings-job fault; waits for the run and downloads its report.
main() {
  local S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
  local label=$1 cand=$2 base=$3 tc=$4 games=$5 hash=$6 conf=$7 note=$8 attempt rid st
  cd /Users/jimmywallace/Projects/chess/rusty-chess-v2 || exit 1
  rid=""
  for attempt in 1 2 3; do
    gh workflow run Match --ref main -f candidate="$cand" -f baseline="$base" -f tc="$tc" -f games="$games" -f hash="$hash" -f confidence="$conf" -f note="$note (attempt $attempt)" > /dev/null 2>&1
    sleep 15
    rid=$(gh run list --workflow Match --limit 1 --json databaseId --jq '.[0].databaseId')
    st=""
    for _ in $(seq 1 40); do
      st=$(gh run view "$rid" --json jobs --jq '.jobs[] | select(.name | startswith("Resolve")) | .conclusion' 2>/dev/null)
      [ -n "$st" ] && break
      sleep 10
    done
    echo "$(date -u +%H:%M:%S) $label attempt $attempt run $rid settings=$st" >> "$S/runs/matches.log"
    [ "$st" = "success" ] && break
    rid=""
  done
  [ -n "$rid" ] || { echo "$(date -u +%H:%M:%S) $label GAVE UP" >> "$S/runs/matches.log"; return 1; }
  while [ "$(gh run view "$rid" --json status --jq .status 2>/dev/null)" != "completed" ]; do sleep 30; done
  st=$(gh run view "$rid" --json conclusion --jq .conclusion)
  mkdir -p "$S/artifacts/$label"
  gh run download "$rid" --repo jwallace145/rusty-chess-v2 --name match --dir "$S/artifacts/$label" > /dev/null 2>&1
  echo "$(date -u +%H:%M:%S) $label run $rid finished=$st $(grep -m1 -E '^\| [+−-][0-9]' "$S/artifacts/$label/match.md" 2>/dev/null)" >> "$S/runs/matches.log"
}
main "$@"
