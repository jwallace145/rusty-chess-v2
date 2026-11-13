#!/usr/bin/env bash
# Runs queued screens one at a time: lines of  NAME|NODES|ROUNDS|OPTS|BASEOPTS  in runs/queue.txt.
S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
touch $S/runs/queue.txt $S/runs/done.txt
while true; do
  [ -f $S/runs/STOP ] && exit 0
  line=$(grep -v -x -F -f $S/runs/done.txt $S/runs/queue.txt | grep -v '^#' | grep . | head -1 || true)
  if [ -z "$line" ]; then sleep 5; continue; fi
  IFS='|' read -r name nodes rounds opts baseopts <<< "$line"
  $S/scripts/screen.sh "$name" "$nodes" "$rounds" "$opts" "$baseopts" 12 >> $S/runs/queue.log 2>&1
  echo "$line" >> $S/runs/done.txt
done
