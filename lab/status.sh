#!/usr/bin/env bash
# One-screen status of everything the session has running.
main() {
  local S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
  cd /Users/jimmywallace/Projects/chess/rusty-chess-v2 || exit 1
  date
  echo "--- in-progress runs"
  gh run list --limit 120 --json databaseId,workflowName,status,displayTitle,createdAt --jq '.[] | select(.status != "completed") | "\(.databaseId) \(.workflowName) \(.createdAt[11:19]) \(.displayTitle[0:70])"'
  echo "--- matches.log (tail)"
  tail -${1:-4} "$S/runs/matches.log" | cut -c1-200
  echo "--- spsa"
  python3 -c "
import json; d=json.load(open('$S/runs/spsa-s1/state.json')); print('iteration', d['iteration'], d['totals'])"
  echo "--- instances"
  AWS_PROFILE=prog-strength-admin AWS_DEFAULT_REGION=us-east-1 aws ec2 describe-instances --filters "Name=instance-state-name,Values=pending,running,shutting-down" "Name=instance-type,Values=c8a.16xlarge,g6.xlarge" --query 'Reservations[].Instances[].[InstanceId,InstanceType,State.Name,LaunchTime,Tags[?Key==`Workflow`]|[0].Value]' --output text
}
main "$@"
