#!/usr/bin/env python3
"""provingmap.py [OUT.map] -- the proving ground: one island that holds the maze and takeover tests as places.

Paul, 6 November: "build a map, with the test setup gauntlet of a the series of tests. A series of spiral
mazes, S shaped bases. and roads ... look at the map tiles, for ways to stitch togeather componets like
blocks to make rivers and roads styled with consistent tiles along with your own map creativity. Also the
map A Flame War Modified has a good walled section. Add bases around the island."

THE STITCHING follows the prior-art tile sheet Paul sent (Birdman's map tiles): the land is built in blocks
whose edges agree -- a road is one tile wide and leaves a block at the same offset it entered, a river is two
tiles wide, a road crosses a river on a bridge as wide as the river, a forest's edge is ragged a tile deep and
never touches a road. Every block here is laid by one function, so the joins match.

THE PLACES, a quadrant each, every one reached from the ring road:
  NW  the S: two bars staggered inside a walled square, a road down the S to a base and a pillbox at its end;
  NE  the @: three square rings, their gaps north, south, north, the base and its pillbox at the centre;
  SW  the 5: a top bar, a middle bar and the bowl's stroke, the base in the bowl;
  SE  the walled city (A Flame War Modified's, its blocks and lanes as they are, pond and all), and beside it
      a compound of wall and concrete with no gate, a base and a pillbox inside: the breach.
Eight bases round the coast, a start beside each; pillboxes at the bridges. The gauntlet's files G30 to G33
put a tank at a place's door and its pillbox at the place's end (scenarios/gauntlet).
"""
import math
import random
import sys

N = 128
G = [["." for _ in range(N)] for _ in range(N)]
rng = random.Random(0x9e0f)


def put(x, y, c):
    if 0 <= x < N and 0 <= y < N:
        G[y][x] = c


def at(x, y):
    return G[y][x] if 0 <= x < N and 0 <= y < N else "~"


def rect(x0, y0, x1, y1, c):
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            put(x, y, c)


def frame(x0, y0, x1, y1, c, gaps=()):
    """A ring of `c` round a box, open where `gaps` (sets of tiles) say."""
    for x in range(x0, x1 + 1):
        for y in (y0, y1):
            if (x, y) not in gaps:
                put(x, y, c)
    for y in range(y0, y1 + 1):
        for x in (x0, x1):
            if (x, y) not in gaps:
                put(x, y, c)


def road(path):
    """A road one tile wide through the corners given; over water it is a bridge."""
    for (ax, ay), (bx, by) in zip(path, path[1:]):
        n = max(abs(bx - ax), abs(by - ay))
        for i in range(n + 1):
            x, y = ax + (bx - ax) * i // max(n, 1), ay + (by - ay) * i // max(n, 1)
            put(x, y, "+" if at(x, y) == "~" else "=")


# ---- the island: sea round a square with its corners rounded, the coast a little ragged
for y in range(N):
    for x in range(N):
        e = min(x, y, N - 1 - x, N - 1 - y)
        cx, cy = min(max(x, 14), N - 15), min(max(y, 14), N - 15)
        corner = math.hypot(x - cx, y - cy) > 11
        if e < 3 or (e < 5 and rng.random() < 0.35) or corner:
            G[y][x] = "~"

# ---- the river: two tiles wide, north to south, a gentle bend (the tile sheet's river blocks)
for y in range(N):
    x = 63 + round(2.0 * math.sin(y / 11.0))
    if at(x, y) != "~" or y < 3 or y > N - 4:
        put(x, y, "~")
        put(x + 1, y, "~")

# ---- forest: blobs on open ground, ragged at their edges, and a belt along the river's banks
def blob(cx, cy, r):
    for y in range(cy - r - 1, cy + r + 2):
        for x in range(cx - r - 1, cx + r + 2):
            d = math.hypot(x - cx, y - cy)
            if at(x, y) == "." and (d < r - 0.5 or (d < r + 1 and rng.random() < 0.45)):
                put(x, y, "T")


for y in range(6, N - 6):
    rx = 63 + round(2.0 * math.sin(y / 11.0))
    for x in list(range(rx - 4, rx - 1)) + list(range(rx + 3, rx + 6)):
        if at(x, y) == "." and rng.random() < 0.55:
            put(x, y, "T")
for (cx, cy, r) in [(30, 62, 3), (97, 62, 3), (8, 45, 2), (119, 80, 2), (45, 120, 2), (80, 8, 2), (60, 30, 2), (68, 98, 2)]:
    blob(cx, cy, r)

# ---- the NW place: the S
S0, S1 = (16, 16), (56, 56)
rect(S0[0], S0[1], S1[0], S1[1], ".")
frame(S0[0], S0[1], S1[0], S1[1], "W", gaps={(16, y) for y in range(20, 24)})
rect(16, 29, 48, 29, "W")                      # the first bar, its gap at the east end
rect(24, 42, 56, 42, "W")                      # the second, its gap at the west
road([(10, 21), (52, 21), (52, 35), (20, 35), (20, 48), (47, 48)])
rect(48, 50, 50, 52, "D")
S_BASE, S_PILL = (49.5, 51.5), (53.5, 52.5)

# ---- the NE place: the @
C = (91, 36)
for k, (r, side) in enumerate([(19, "n"), (13, "s"), (7, "n")]):
    gx = {(x, C[1] - r) for x in range(C[0] - 2, C[0] + 3)} if side == "n" else {(x, C[1] + r) for x in range(C[0] - 2, C[0] + 3)}
    if k == 0:
        rect(C[0] - r, C[1] - r, C[0] + r, C[1] + r, ".")
    frame(C[0] - r, C[1] - r, C[0] + r, C[1] + r, "W", gaps=gx)
road([(91, 10), (91, 17)])
rect(89, 38, 91, 40, "D")
AT_BASE, AT_PILL = (90.5, 39.5), (91.5, 35.5)

# ---- the SW place: the 5
F0, F1 = (16, 72), (56, 112)
rect(F0[0], F0[1], F1[0], F1[1], ".")
frame(F0[0], F0[1], F1[0], F1[1], "W", gaps={(x, 72) for x in range(50, 54)})
rect(24, 80, 56, 80, "W")                      # the top bar: round it at the west
rect(16, 92, 48, 92, "W")                      # the middle bar: round it at the east
rect(36, 93, 36, 104, "W")                     # the bowl's stroke
road([(52, 64), (52, 76), (20, 76), (20, 86), (52, 86), (52, 108), (40, 108)])
rect(22, 104, 24, 106, "D")
FIVE_BASE, FIVE_PILL = (23.5, 105.5), (27.5, 101.5)

# ---- the SE place: the walled city (A Flame War Modified's, as it is) and the breach
CITY = """\
.WWWWW=WWWWWTD=TT.~~
.==T=======T===.T.~~
WWWWWW=WTW=W=WTWT.~~
=====T=W=W=W=W..TT~~
=WTWWW=W=W=W=W.TT.~~
=W=======W=T==..~~~~
=WWWTWWW=W=W=..~~~~~
=======T=T===.~~~~~~
W=WWWTW=WWW=W=.~~~.~
T=T===T===W=W=T~~.T~
W=W=W=WWW=W=WWW~~.T~
T=W=T...T=W=======.~
WTW=~~~TD=W=W=W.~~~~
T===~~~===W=W=T..~~~
W=WT...=WWW=W=W..~~~
W=W===T===T=T=TT...~
W=W=W=WWW=W=WWW=WT..
=W===T===T=W=W===WTT
WTW=WWW=W=W=W=WWWTT.
==W=T===T=W======...
WTW=W=WTW=W=WWWTW...
W===W=T=====T.TT..TT
WWWTW=WWWWW......TT.
==========T..T.TTT.~
TTW=WTWWWWW.TT..T..~"""
for j, line in enumerate(CITY.splitlines()):
    for i, c in enumerate(line):
        put(72 + i, 72 + j, c)
road([(64, 77), (72, 77)])                     # the city's west gate from the cross road's bank
# The breach: a compound of wall with concrete corners, no gate; a road runs up to its wall and stops.
B0, B1 = (97, 96), (107, 106)
rect(B0[0], B0[1], B1[0], B1[1], ".")
frame(B0[0], B0[1], B1[0], B1[1], "W")
for (x, y) in [B0, (B1[0], B0[1]), (B0[0], B1[1]), B1]:
    put(x, y, "H")
rect(100, 100, 102, 102, "D")
road([(117, 101), (108, 101)])
BREACH_BASE, BREACH_PILL = (101.5, 101.5), (104.5, 99.5)

# ---- the ring road and the cross road, laid last so they cross the river on bridges
road([(10, 10), (117, 10), (117, 117), (10, 117), (10, 10)])
road([(10, 64), (117, 64)])
road([(64, 10), (64, 4)])

# ---- the coast's bases, a start beside each (seat order clockwise from the north)
COAST = [(30, 6), (98, 6), (121, 30), (121, 98), (98, 121), (30, 121), (6, 98), (6, 30)]
starts, depots = [], []
for (x, y) in COAST:
    for dy in range(-1, 2):
        for dx in range(-1, 2):
            put(x + dx, y + dy, "D")
    sx, sy = x + (3 if x < 64 else -3), y + (3 if y < 64 else -3)
    for (px, py) in [(sx, sy), (sx + 1, sy), (sx, sy + 1)]:
        if at(px, py) in "~WHT":
            put(px, py, ".")
    starts.append((sx + 0.5, sy + 0.5))
    depots.append((x + 0.5, y + 0.5))

# The coast's ground: a base and its start never stand on water, a wall or forest.
for (x, y) in [(int(a), int(b)) for a, b in depots + starts]:
    for dy in range(-2, 3):
        for dx in range(-2, 3):
            if at(x + dx, y + dy) in "~TWH" and 3 <= x + dx < N - 3 and 3 <= y + dy < N - 3:
                put(x + dx, y + dy, ".")

PILLS = [(S_PILL, "neutral"), (AT_PILL, "neutral"), (FIVE_PILL, "neutral"), (BREACH_PILL, "neutral"),
         ((61.5, 12.5), "neutral"), ((67.5, 115.5), "neutral"), ((61.5, 66.5), "neutral"), ((67.5, 62.5), "neutral")]

out = sys.argv[1] if len(sys.argv) > 1 else "maps/proving.map"
with open(out, "w") as f:
    f.write("treadcore map 1\nsize %d %d\nseed 0x9e0f\n" % (N, N))
    f.write("# the proving ground (scripts/provingmap.py): the S, the @, the 5, a walled city and a breach, a quadrant each,\n"
            "# off a ring road over a river; eight bases round the coast. The walled city is A Flame War Modified's.\n")
    f.write("# . grass  T forest  = road  ~ water  + bridge  % rubble  D depot  W wall  H concrete\n")
    for row in G:
        f.write("".join(row) + "\n")
    f.write("# places: the coast's starts and their bases (seat order), then the places' bases, neutral\n")
    for x, y in starts:
        f.write("start %g %g\n" % (x, y))
    for x, y in depots:
        f.write("depot %g %g\n" % (x, y))
    for (x, y) in [S_BASE, AT_BASE, FIVE_BASE, BREACH_BASE]:
        f.write("depot %g %g neutral\n" % (x, y))
    for (x, y), who in PILLS:
        f.write("pillbox %g %g %s\n" % (x, y, who))
print("wrote %s: %dx%d, %d starts, %d bases, %d pillboxes" % (out, N, N, len(starts), len(depots) + 4, len(PILLS)))
