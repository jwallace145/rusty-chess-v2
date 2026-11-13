#!/usr/bin/env bash
# gate.sh "LAB_A=1 LAB_B=2" [BIN] [STATS_BIN]  -> signature, fail-high-first, EBF, selected counters of a lab variant (local, depth-10 bench)
S=/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad
bin=${2:-$S/bin/lab}; sbin=${3:-$S/bin/lab-stats}
t=$(mktemp -d)
env $1 "$bin" bench --out $t/bench.jsonl > $t/bench.log
env $1 "$sbin" bench --out $t/stats.jsonl > $t/stats.log
sig=$(sed -n 's/^Nodes searched  : //p' $t/bench.log); ms=$(sed -n 's/^Total time (ms) : //p' $t/bench.log)
gate=$($S/bin/perf-gate --targets /Users/jimmywallace/Projects/chess/rusty-chess-v2/perf-targets.toml --env local $t/bench.jsonl $t/stats.jsonl 2>&1 | grep -E "fail_high_first|max_ebf" | awk -F'|' '{gsub(/ /,"",$3); gsub(/ /,"",$5); printf "%s(%s) ", $3, $5}')
cnt=$(tail -1 $t/stats.jsonl | python3 -c "
import sys,json
s=json.loads(sys.stdin.read())['stats']
print('rfp=%d nmp=%d lmr=%d/%d fut=%d lmp=%d qn=%d stage=%s' % (s['rfp_cutoffs'],s['null_move_cutoffs'],s['lmr_reductions'],s['lmr_researches'],s['futility_skips'],s['lmp_skips'],s['qnodes'],s['cutoffs_by_stage']))")
printf '%-58s sig=%s %sms fhf/ebf=%s %s\n' "[$1]" "$sig" "$ms" "$gate" "$cnt"
rm -rf $t
