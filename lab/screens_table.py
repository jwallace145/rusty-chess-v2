#!/usr/bin/env python3
"""Render screens-table rows from each run's fastchess log (the primary source). Usage: screens_table.py NAME [NAME...]"""
import re, sys
from pathlib import Path
S = Path("/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad")
def last(pattern, text):
    m = None
    for m in re.finditer(pattern, text, re.M):
        pass
    return m
for name in sys.argv[1:]:
    log = (S / "runs" / name / "fc.log").read_text()
    elo = last(r"^Elo: (-?[\d.]+) \+/- ([\d.]+)", log)
    los = last(r"^LOS: ([\d.]+) %", log)
    games = last(r"^Games: (\d+), Wins: (\d+), Losses: (\d+), Draws: (\d+)", log)
    penta = last(r"^Ptnml\(0-2\): \[([\d, ]+)\]", log)
    llr = last(r"^LLR: (-?[\d.]+) ", log)
    nodes = last(r"nodes=(\d+)", (S / "runs" / name / "config.json").read_text()) if False else None
    stop = "H1 (early stop)" if "H1 was accepted" in log else "H0 (early stop)" if "H0 was accepted" in log else "Game cap"
    e = float(elo.group(1)); sign = "+" if e >= 0 else "−"
    print(f"| `{name}` | {int(games.group(1)):,} | {sign}{abs(e):.1f} ± {float(elo.group(2)):.1f} | {float(los.group(1)):.1f}% | {games.group(2)} / {games.group(3)} / {games.group(4)} | {penta.group(1)} | {float(llr.group(1)):.2f} | {stop} |".replace("-0.", "−0.").replace("| -", "| −"))
