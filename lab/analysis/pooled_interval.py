"""The pooled timed result for experiment 5: the SPRT's timed leg and match M18,
from their pentanomial counts (match M18's table in the research log).

Verbatim from the session.
"""
import math
def elo(mu): return -400*math.log10(1/mu - 1)
def stats(p):
    n = sum(p); sc = [0, 0.25, 0.5, 0.75, 1.0]
    mu = sum(c*s for c, s in zip(p, sc))/n
    var = sum(c*(s-mu)**2 for c, s in zip(p, sc))/n
    se = math.sqrt(var/n)
    lo, hi = elo(mu-1.959964*se), elo(mu+1.959964*se)
    return n, mu, elo(mu), lo, hi, se
a = [105, 1071, 2512, 1201, 111]   # SPRT timed leg of #252 (run 43394229688)
b = [120, 1071, 2530, 1160, 119]   # match M18 (run 43402595891)
for name, p in (("SPRT timed leg", a), ("match M18", b), ("pooled counts", [x+y for x, y in zip(a, b)])):
    n, mu, e, lo, hi, se = stats(p)
    print(f"{name:15} pairs={n} score={mu:.5f} Elo={e:+.2f} 95% [{lo:+.2f}, {hi:+.2f}] half-width {(hi-lo)/2:.2f}")
# inverse-variance pooling of the two Elo estimates, as the plan worded it
ea, la, ha = stats(a)[2:5]; eb, lb, hb = stats(b)[2:5]
sa, sb = (ha-la)/2/1.959964, (hb-lb)/2/1.959964
w = 1/sa**2 + 1/sb**2
pe = (ea/sa**2 + eb/sb**2)/w; ps = math.sqrt(1/w)
print(f"inverse-variance pooled Elo={pe:+.2f} 95% [{pe-1.959964*ps:+.2f}, {pe+1.959964*ps:+.2f}]")
