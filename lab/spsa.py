#!/usr/bin/env python3
"""Local SPSA tuning of the lab build's parameters (exploratory, not the record).

Each iteration perturbs every parameter by +/- c_k in a random direction,
plays a small fixed-node match between the two perturbed engines with
fastchess (the SPRT's openings and adjudication), and moves each parameter
along its perturbation in proportion to the match result. The schedule is
the one engine testing frameworks commonly use (Spall's SPSA with
a_k = a / (A + k)^alpha, c_k = c / k^gamma).

Usage: spsa.py NAME BIN NODES ITERATIONS PAIRS CONCURRENCY [--fixed "OPT=V ..."] [--seed N]
State is written to runs/NAME/state.json after every iteration, so a run can
be read while it plays and resumed if it stops.
"""

import json
import random
import re
import subprocess
import sys
import time
from pathlib import Path

S = Path("/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad")
OPENINGS = "/Users/jimmywallace/Projects/chess/rusty-chess-v2/tests/openings/openings.epd"
FASTCHESS = str(S / "fastchess-src" / "fastchess")

# name: (start, min, max, c_end, r_end). c_end is the perturbation at the last
# iteration (about a twentieth of the plausible range, at least half a unit
# for an integer); r_end the learning rate there.
PARAMS = {
    "RFP_MARGIN": (80, 40, 150, 8.0, 0.003),
    "RFP_DEPTH": (6, 3, 10, 0.5, 0.003),
    "FP_BASE": (100, 0, 260, 14.0, 0.003),
    "FP_MARGIN": (100, 40, 200, 10.0, 0.003),
    "FP_DEPTH": (6, 3, 10, 0.5, 0.003),
    "LMP_BASE": (3, 1, 8, 0.5, 0.003),
    "LMP_DEPTH": (4, 2, 9, 0.5, 0.003),
    "LMP_PCT": (100, 40, 170, 8.0, 0.003),
    "SEE_QUIET": (64, 20, 140, 8.0, 0.003),
    "SEE_DEPTH": (8, 4, 12, 0.5, 0.003),
    "LMR_BASE": (75, 20, 150, 8.0, 0.003),
    "LMR_DIV": (225, 150, 330, 12.0, 0.003),
    "LMR_HIST_DIV": (512, 128, 2048, 48.0, 0.003),
    "ASP_DELTA": (35, 8, 80, 4.0, 0.003),
    "NMP_A64": (192, 96, 330, 12.0, 0.003),
    "NMP_B64": (22, 8, 44, 2.0, 0.003),
    "NMP_EVAL_DIV": (200, 80, 400, 20.0, 0.003),
}
ALPHA, GAMMA = 0.602, 0.101


def play(bin_path, nodes, pairs, conc, plus, minus, fixed, seed):
    def opts(values):
        out = [f"option.L_{k}={v}" for k, v in values.items()]
        out += [f"option.L_{o}" for o in fixed]
        return out

    cmd = [
        FASTCHESS,
        "-engine", f"cmd={bin_path}", "name=plus", *opts(plus),
        "-engine", f"cmd={bin_path}", "name=minus", *opts(minus),
        "-each", "tc=600+6", f"nodes={nodes}",
        "-openings", f"file={OPENINGS}", "format=epd", "order=random",
        "-srand", str(seed),
        "-games", "2", "-repeat", "-rounds", str(pairs), "-concurrency", str(conc),
        "-maxmoves", "200",
        "-draw", "movenumber=40", "movecount=8", "score=10",
        "-resign", "movecount=3", "score=800", "twosided=true",
        "-recover",
    ]
    out = subprocess.run(cmd, capture_output=True, text=True, timeout=1800).stdout
    m = None
    for m in re.finditer(r"Games: (\d+), Wins: (\d+), Losses: (\d+), Draws: (\d+)", out):
        pass
    if m is None:
        return None
    games, wins, losses, draws = (int(x) for x in m.groups())
    return games, wins, losses, draws


def main():
    args = sys.argv[1:]
    name, bin_path, nodes, iters, pairs, conc = args[0], args[1], int(args[2]), int(args[3]), int(args[4]), int(args[5])
    fixed, seed = [], 1
    if "--fixed" in args:
        fixed = args[args.index("--fixed") + 1].split()
    if "--seed" in args:
        seed = int(args[args.index("--seed") + 1])
    out_dir = S / "runs" / name
    out_dir.mkdir(parents=True, exist_ok=True)
    state_file = out_dir / "state.json"
    rng = random.Random(seed)
    big_a = 0.1 * iters
    if state_file.exists():
        state = json.loads(state_file.read_text())
        theta, k0, total = state["theta"], state["iteration"], state["totals"]
        # Replay the generator so a resumed run draws what a straight one would.
        for _ in range(k0 * (len(PARAMS) + 1)):
            rng.random()
    else:
        theta = {k: float(v[0]) for k, v in PARAMS.items()}
        k0, total = 0, {"games": 0, "wins": 0, "losses": 0, "draws": 0}
    log = open(out_dir / "spsa.log", "a")
    start = time.time()
    for k in range(k0, iters):
        if (S / "runs" / "STOP_SPSA").exists():
            break
        plus, minus, step = {}, {}, {}
        for p, (_, lo, hi, c_end, r_end) in PARAMS.items():
            c_k = c_end * (iters ** GAMMA) / ((k + 1) ** GAMMA)
            a_end = r_end * c_end * c_end
            a_k = a_end * ((big_a + iters) ** ALPHA) / ((big_a + k + 1) ** ALPHA)
            flip = 1 if rng.random() < 0.5 else -1
            plus[p] = int(round(min(hi, max(lo, theta[p] + c_k * flip))))
            minus[p] = int(round(min(hi, max(lo, theta[p] - c_k * flip))))
            step[p] = (a_k / (c_k * c_k)) * c_k * flip
        game_seed = int(rng.random() * 2**31)
        result = play(bin_path, nodes, pairs, conc, plus, minus, fixed, game_seed)
        if result is None:
            log.write(f"{k} failed\n")
            log.flush()
            continue
        games, wins, losses, draws = result
        for p, (_, lo, hi, _, _) in PARAMS.items():
            theta[p] = min(hi, max(lo, theta[p] + step[p] * (wins - losses)))
        total["games"] += games
        total["wins"] += wins
        total["losses"] += losses
        total["draws"] += draws
        state_file.write_text(json.dumps({"iteration": k + 1, "theta": theta, "totals": total, "nodes": nodes, "pairs": pairs, "fixed": fixed, "seed": seed, "iterations": iters}, indent=1))
        if k % 10 == 0 or k == iters - 1:
            line = f"{k + 1}/{iters} {int(time.time() - start)}s games={total['games']} " + " ".join(f"{p}={theta[p]:.1f}" for p in PARAMS)
            log.write(line + "\n")
            log.flush()
    log.close()


if __name__ == "__main__":
    main()
