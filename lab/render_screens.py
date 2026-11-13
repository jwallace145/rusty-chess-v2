#!/usr/bin/env python3
"""Render the research log's screens table from each run's fastchess log (the primary source).
runs/screens-meta.json is an ordered list of {"name", "options", "nodes", "note"} or {"raw": "<a full row>"}."""
import json, re
from pathlib import Path
S = Path("/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad")
def last(pattern, text):
    m = None
    for m in re.finditer(pattern, text, re.M):
        pass
    return m
def num(x):
    return f"{x:+.1f}".replace("-", "−")
print("| Screen | Options (against the defaults unless noted) | Nodes/move | Games | Elo ± 95% | LOS | W / L / D | Pentanomial (LL, LD, WL+DD, WD, WW) | LLR | How it ended, and the decision |")
print("|:--|:--|--:|--:|--:|--:|:-:|:--|--:|:--|")
for e in json.loads((S / "runs" / "screens-meta.json").read_text()):
    if "raw" in e:
        print(e["raw"]); continue
    log = (S / "runs" / e["name"] / "fc.log").read_text()
    elo = last(r"^Elo: (-?[\d.]+) \+/- ([\d.]+)", log)
    los = last(r"^LOS: ([\d.]+) %", log)
    g = last(r"^Games: (\d+), Wins: (\d+), Losses: (\d+), Draws: (\d+)", log)
    penta = last(r"^Ptnml\(0-2\): \[([\d, ]+)\]", log)
    llr = last(r"^LLR: (-?[\d.]+) ", log)
    stop = "H1 by early stop" if "H1 was accepted" in log else "H0 by early stop" if "H0 was accepted" in log else "Game cap"
    estr = f"{num(float(elo.group(1)))} ± {float(elo.group(2)):.1f}"
    if e.get("bold"): estr = f"**{estr}**"
    print(f"| `{e['name']}` | {e['options']} | {e['nodes']:,} | {int(g.group(1)):,} | {estr} | {float(los.group(1)):.1f}% | {g.group(2)} / {g.group(3)} / {g.group(4)} | {penta.group(1)} | {float(llr.group(1)):.2f}".replace("| -", "| −") + f" | {stop}. {e['note']} |")
