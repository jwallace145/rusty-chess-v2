#!/usr/bin/env bash
# Latest lines of the two newest SPRT task streams (read-only).
export AWS_PROFILE=prog-strength-admin AWS_DEFAULT_REGION=us-east-1
aws logs describe-log-streams --log-group-name /jwall-engineering/rusty-chess/tasks --order-by LastEventTime --descending --limit ${1:-2} --query 'logStreams[].logStreamName' --output text | tr '\t' '\n' | grep stdout | while read -r n; do
  echo "== ${n: -62}"
  aws logs get-log-events --log-group-name /jwall-engineering/rusty-chess/tasks --log-stream-name "$n" --limit 400 --query 'events[].message' --output text | tr '\t' '\n' | grep -E "strength task|games/min|Elo:|Games:|LLR:|SPRT .* completed|H1 accepted|H0 accepted|inconclusive|mode=|games=|total_seconds" | tail -${2:-6}
done
