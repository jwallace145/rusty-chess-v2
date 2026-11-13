#!/usr/bin/env python3
"""sprt_rows.py DIR: render an SPRT run's two legs from its artifacts.

DIR holds the run's downloaded artifacts, `sprt-timed/sprt.md` and
`sprt-nodes/sprt.md` (either may be missing). Prints one JSON object with, for
each leg, the figures as the report gives them and a ready table row, so the
record is rendered from the report and never typed.
"""

import json
import re
import sys
from pathlib import Path

LEGS = (("timed", "sprt-timed", "Timed, 5+0.05"), ("nodes", "sprt-nodes", "Fixed nodes, 200,000 per move"))


def parse(md: str) -> dict:
    row = re.search(r"^\|\s*([+\-−]?[0-9.]+) ± ([0-9.]+)\s*\|\s*([0-9.]+)%\s*\|\s*(\d+)\s*\|\s*(\d+) / (\d+) / (\d+)\s*\|\s*([+\-−]?[0-9.]+) \(", md, re.M)
    if row is None:
        raise SystemExit("no result row in the report")
    elo, err, los, games, w, l, d, llr = row.groups()
    penta = re.search(r"Pentanomial.*\):\s*(\d+) (\d+) (\d+) (\d+) (\d+)", md)
    head = md.splitlines()[2] if len(md.splitlines()) > 2 else ""
    if "H1" in head:
        verdict = "H1"
    elif "H0" in head:
        verdict = "H0"
    elif "inconclusive" in head.lower():
        verdict = "inconclusive"
    else:
        verdict = "unknown"
    return {
        "elo": elo.replace("-", "−"), "err": err, "los": los, "games": int(games),
        "w": int(w), "l": int(l), "d": int(d), "llr": llr.replace("-", "−"),
        "penta": [int(x) for x in penta.groups()] if penta else None,
        "verdict": verdict, "headline": head,
    }


def main() -> None:
    base = Path(sys.argv[1])
    out = {}
    for key, folder, title in LEGS:
        f = base / folder / "sprt.md"
        if not f.exists():
            out[key] = None
            continue
        r = parse(f.read_text())
        bold = r["verdict"] in ("H1", "H0")
        elo = f"{r['elo']} ± {r['err']}"
        result = {"H1": "H1", "H0": "H0", "inconclusive": "Inconclusive at the 5,000-round cap"}.get(r["verdict"], r["verdict"])
        penta = ", ".join(str(x) for x in r["penta"]) if r["penta"] else "not reported"
        r["row"] = (
            f"| {title} | {'**' + elo + '**' if bold else elo} | {r['los']}% | {r['games']} | "
            f"{r['w']} / {r['l']} / {r['d']} | {r['llr']} | {penta} | {result} |"
        )
        r["elo_pm"] = elo
        out[key] = r
    print(json.dumps(out, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    main()
