#!/usr/bin/env bash
# Runs queued Match dispatches one at a time. runs/match-queue.txt holds lines
#   LABEL|CANDIDATE|BASELINE|nodes or tc|VALUE|GAMES|NOTE
# For each: wait until no Match run is in progress, dispatch (retrying the
# settings-job fault up to 3 times), wait for the run to finish, download its
# report to artifacts/<label>/, and log one line to runs/matches.log.
# Stops when runs/STOP_MATCHES exists. Only ever replaced by mv.
main() {
  local S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
  cd /Users/jimmywallace/Projects/chess/rusty-chess-v2 || exit 1
  touch "$S/runs/match-queue.txt" "$S/runs/match-done.txt"
  local line label cand base kind value games note attempt rid st busy flag
  while true; do
    [ -f "$S/runs/STOP_MATCHES" ] && exit 0
    line=$(grep -v -x -F -f "$S/runs/match-done.txt" "$S/runs/match-queue.txt" | grep -v '^#' | grep . | head -1 || true)
    if [ -z "$line" ]; then sleep 20; continue; fi
    busy=$(gh run list --workflow Match --limit 6 --json status --jq '[.[] | select(.status != "completed")] | length' 2>/dev/null)
    if [ "${busy:-1}" != "0" ]; then sleep 30; continue; fi
    IFS='|' read -r label cand base kind value games note <<< "$line"
    if [ "$kind" = "nodes" ]; then flag=(-f nodes="$value"); else flag=(-f tc="$value"); fi
    rid=""
    for attempt in 1 2 3; do
      gh workflow run Match --ref main -f candidate="$cand" -f baseline="$base" "${flag[@]}" -f games="$games" -f note="$note (attempt $attempt)" > /dev/null 2>&1
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
    if [ -n "$rid" ]; then
      while [ "$(gh run view "$rid" --json status --jq .status 2>/dev/null)" != "completed" ]; do sleep 30; done
      st=$(gh run view "$rid" --json conclusion --jq .conclusion)
      mkdir -p "$S/artifacts/$label"
      gh run download "$rid" --repo jwallace145/rusty-chess-v2 --name match --dir "$S/artifacts/$label" > /dev/null 2>&1
      echo "$(date -u +%H:%M:%S) $label run $rid finished=$st $(grep -m1 -E '^\| [+−-][0-9]' "$S/artifacts/$label/match.md" 2>/dev/null)" >> "$S/runs/matches.log"
    else
      echo "$(date -u +%H:%M:%S) $label GAVE UP after 3 settings failures" >> "$S/runs/matches.log"
    fi
    echo "$line" >> "$S/runs/match-done.txt"
  done
}
main "$@"
