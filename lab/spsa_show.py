#!/usr/bin/env python3
"""Show how far the SPSA run has moved each parameter, in units of its final perturbation."""
import json
S = "/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad"
d = json.load(open(S + '/runs/spsa-s1/state.json'))
start = {"RFP_MARGIN":80,"RFP_DEPTH":6,"FP_BASE":100,"FP_MARGIN":100,"FP_DEPTH":6,"LMP_BASE":3,"LMP_DEPTH":4,"LMP_PCT":100,"SEE_QUIET":64,"SEE_DEPTH":8,"LMR_BASE":75,"LMR_DIV":225,"LMR_HIST_DIV":512,"ASP_DELTA":35,"NMP_A64":192,"NMP_B64":22,"NMP_EVAL_DIV":200}
cend = {"RFP_MARGIN":8,"RFP_DEPTH":0.5,"FP_BASE":14,"FP_MARGIN":10,"FP_DEPTH":0.5,"LMP_BASE":0.5,"LMP_DEPTH":0.5,"LMP_PCT":8,"SEE_QUIET":8,"SEE_DEPTH":0.5,"LMR_BASE":8,"LMR_DIV":12,"LMR_HIST_DIV":48,"ASP_DELTA":4,"NMP_A64":12,"NMP_B64":2,"NMP_EVAL_DIV":20}
print('iteration', d['iteration'], d['totals'])
for k, v in d['theta'].items():
    m = (v - start[k]) / cend[k]
    print(f"{k:13} {start[k]:5} -> {v:8.2f}  {m:+.2f} c_end {'  <<' if abs(m) >= 1 else ''}")
