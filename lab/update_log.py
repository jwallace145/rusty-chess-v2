#!/usr/bin/env python3
"""Replace the screens table in the research log (between its markers) with a fresh render from the logs."""
import subprocess, sys
from pathlib import Path
S = Path("/private/tmp/claude-501/-Users-jimmywallace-Projects-chess-rusty-chess-v2/16b9d994-675d-459a-9093-5eca1604a183/scratchpad")
p = S / "wt" / "docs" / "docs" / "research-log.md"
s = p.read_text()
a, b = "<!-- screens:begin -->\n", "<!-- screens:end -->\n"
table = subprocess.run([sys.executable, str(S / "scripts" / "render_screens.py")], capture_output=True, text=True, check=True).stdout
i, j = s.index(a) + len(a), s.index(b)
p.write_text(s[:i] + table + s[j:])
print("screens table updated:", table.count("\n") - 2, "rows")
