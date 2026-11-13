"""Search depth by phase of the game, PR against base, in both SPRT legs
(experiment 2's table in the research log).

Run in a directory holding `timed/sprt.pgn` and `nodes/sprt.pgn`, the
`sprt-timed` and `sprt-nodes` artifacts of SPRT run 43381561111. Verbatim from
the session.
"""
import re, statistics
def analyse(path, label):
    txt = open(path).read()
    games = re.split(r'\n(?=\[Event )', txt)
    mv = re.compile(r'\{([+-]?(?:M?\d+(?:\.\d+)?))/(\d+) ([\d.]+)s[^}]*\}')
    agg = {}  # (engine, bucket) -> [depths], [times]
    for g in games:
        w = re.search(r'\[White "([^"]+)"\]', g); b = re.search(r'\[Black "([^"]+)"\]', g)
        if not w or not b or '\n\n' not in g: continue
        body = g.split('\n\n', 1)[1]
        # which side moves first: from FEN
        fen = re.search(r'\[FEN "([^"]+)"\]', g)
        stm_white = (fen.group(1).split()[1] == 'w') if fen else True
        moves = mv.findall(body)
        for i, (score, depth, t) in enumerate(moves):
            white_to_move = stm_white if i % 2 == 0 else not stm_white
            eng = w.group(1) if white_to_move else b.group(1)
            own = i // 2 + 1
            bucket = '01-10' if own <= 10 else '11-20' if own <= 20 else '21-30' if own <= 30 else '31-45' if own <= 45 else '46+'
            d = agg.setdefault((eng, bucket), ([], []))
            d[0].append(int(depth)); d[1].append(float(t))
    print(label)
    print("own moves | engine | n | mean depth | mean time ms")
    for bucket in ['01-10', '11-20', '21-30', '31-45', '46+']:
        for eng in ['pr', 'base']:
            d = agg.get((eng, bucket))
            if d: print(f"{bucket:6} | {eng:4} | {len(d[0]):6} | {statistics.mean(d[0]):6.2f} | {1000*statistics.mean(d[1]):6.1f}")
analyse('timed/sprt.pgn', '== #248 timed leg (5+0.05)')
analyse('nodes/sprt.pgn', '== #248 fixed-node leg (200,000)')
