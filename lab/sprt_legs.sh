#!/usr/bin/env bash
# sprt_legs.sh RUN_ID : latest lines of both legs of an SPRT run, found by the run id tag on its instances' log streams.
main() {
  export AWS_PROFILE=prog-strength-admin AWS_DEFAULT_REGION=us-east-1
  local run=$1 ids n
  ids=$(aws ec2 describe-instances --filters "Name=tag:RunId,Values=${run}-*" --query 'Reservations[].Instances[].InstanceId' --output text)
  for n in $(aws logs describe-log-streams --log-group-name /jwall-engineering/rusty-chess/tasks --order-by LastEventTime --descending --limit 12 --query 'logStreams[].logStreamName' --output text | tr '\t' '\n' | grep stdout); do
    for i in $ids; do
      case "$n" in *"$i"*)
        aws logs get-log-events --log-group-name /jwall-engineering/rusty-chess/tasks --log-stream-name "$n" --limit 200 --query 'events[].message' --output text | tr '\t' '\n' | grep -E "strength task|Elo:|Games:|LLR:|completed|accepted|inconclusive" | awk '/strength task/ {h=$0} {a[NR]=$0} END {print h; for (i=NR-2;i<=NR;i++) if (i>0 && a[i] != h) print a[i]}' | cut -c1-150;;
      esac
    done
  done
}
main "$@"
