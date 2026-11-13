"""How the clock is spent at 5+0.05 (the research log's table of that name).

Run in a directory holding `sprt.pgn`, the `sprt-timed` artifact of SPRT run
43378687011 (experiment 1). Verbatim from the session, 2026-11-12 23:56 MST.
"""
import re, statistics
txt = open('sprt.pgn').read()
games = re.split(r'\n(?=\[Event )', txt)
mv = re.compile(r'\{(?:book|([+-]?(?:M?\d+(?:\.\d+)?))/(\d+) ([\d.]+)s[^}]*)\}')
by_ply = {}   # ply index of engine move (1-based own move number) -> list of (time, depth)
left = []     # clock left at game end per side (s)
plies = []
used_frac = []
for g in games:
    if '[White ' not in g: continue
    body = g.split('\n\n', 1)[1] if '\n\n' in g else ''
    moves = mv.findall(body)
    clock = [5.0, 5.0]
    n = [0, 0]
    for i, (score, depth, t) in enumerate(moves):
        if not t: continue
        side = i % 2
        t = float(t); n[side] += 1
        clock[side] += 0.05 - t
        by_ply.setdefault(n[side], []).append((t, int(depth), clock[side] + t - 0.05))
    plies.append(len(moves))
    for s in (0, 1):
        if n[s] > 0:
            left.append(clock[s])
print("games", len(plies), "median plies", statistics.median(plies))
print("clock left at the end of the game, per side: mean %.2fs median %.2fs p10 %.2fs p90 %.2fs" % (statistics.mean(left), statistics.median(left), sorted(left)[len(left)//10], sorted(left)[len(left)*9//10]))
print("own move | n | mean time ms | median | mean depth | mean clock before move (s) | time/clock")
for k in [1,2,3,5,8,10,15,20,25,30,40,50,60,80,100]:
    v = by_ply.get(k)
    if not v or len(v) < 30: continue
    ts = [x[0] for x in v]; ds = [x[1] for x in v]; cs = [x[2] for x in v]
    print("%4d | %5d | %6.0f | %6.0f | %5.1f | %5.2f | %.3f" % (k, len(v), 1000*statistics.mean(ts), 1000*statistics.median(ts), statistics.mean(ds), statistics.mean(cs), statistics.mean(ts)/statistics.mean(cs)))
