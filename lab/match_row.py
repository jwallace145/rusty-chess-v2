#!/usr/bin/env python3
"""Print a results-table row for a Match from its downloaded report (artifacts/<dir>/match.md, timing.txt)."""
import re, sys
from pathlib import Path
S = Path("/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad")
for d in sys.argv[1:]:
    md = (S / "artifacts" / d / "match.md").read_text()
    row = re.search(r"^\| ([+−-][\d.]+) \| ([^|]+) \| ([\d.]+%) \| (\d+) \| ([\d /]+) \| ([\d.]+%) \|", md, re.M)
    penta = re.search(r"Pentanomial \(pairs: LL, LD, WL\+DD, WD, WW\): ([\d ]+)", md)
    cand = re.search(r"^\| Candidate \| `([^`]+)` \| `([^`]+)` \| ([\d,]+) \|", md, re.M)
    base = re.search(r"^\| Baseline \| `([^`]+)` \| `([^`]+)` \| ([\d,]+) \|", md, re.M)
    timing = dict(l.split("=") for l in (S / "artifacts" / d / "timing.txt").read_text().split())
    verdict = re.search(r"^[🟢🔴⚪] \*\*(.+?)\*\*", md, re.M)
    print(f"{d}: | {row.group(1)} | {row.group(2).strip()} | {row.group(3)} | {row.group(4)} | {row.group(5).strip()} | {', '.join(penta.group(1).split())} |")
    print(f"   candidate {cand.group(1)} {cand.group(2)} bench {cand.group(3)}; baseline {base.group(1)} {base.group(2)} bench {base.group(3)}; play {timing.get('play_seconds')} s, total {timing.get('total_seconds')} s; verdict: {verdict.group(1) if verdict else '?'}")
