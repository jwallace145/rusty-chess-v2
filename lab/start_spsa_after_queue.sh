#!/usr/bin/env bash
# Wait for the last queued screen to finish, stop the queue runner, then start SPSA run S1.
main() {
  local S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
  while ! grep -q '^r1-capfut200|' "$S/runs/done.txt"; do sleep 5; done
  touch "$S/runs/STOP"
  sleep 8
  date > "$S/runs/spsa-s1-started.txt"
  exec python3 "$S/scripts/spsa.py" spsa-s1 "$S/bin/lab6" 100000 1500 12 14 --fixed "NMP_EVAL_MAX=3" --seed 20261113
}
main "$@"
