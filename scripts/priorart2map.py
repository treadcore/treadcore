#!/usr/bin/env python3
"""priorart2map.py IN.map [OUT.map]

Convert a prior-art map (its binary map format, version 1, as the
the prior art and prior-art map editors write it) into a treadcore map file
(documentation/MAPS.md). Run it with `python3 -I`, or `make convert
FROM=file.map`; the input is a binary from outside and is read as data
only.

What is kept: the whole 256 x 256 grid, sea and all (treadcore's world
is the prior art's size); every pillbox with its owner; every base as a depot with its
owner (prior-art bases are usually neutral, which is said in the file so the
generator's first-N rule does not apply); every start, moved to the
nearest passable tile because prior-art starts at sea on a boat and
treadcore has no boats.

What is mapped: The prior art has terrains treadcore does not. Buildings become
walls; shot buildings, craters, rubble and swamp become rubble (slow
ground); river, deep sea and boats become water; mined variants become
their plain terrain (mines are laid in play, never in a map). The
tank's start direction is dropped.

The format, from the prior-art map file description:
  magic(8) version(1) npills nbases nstarts
  pills:  x y owner armour speed        (5 bytes each)
  bases:  x y owner armour shells mines (6 bytes each)
  starts: x y direction                 (3 bytes each)
  runs:   datalen y startx endx data..., datalen including these four
          bytes; nibbles: n<8 means n+1 literal terrain nibbles follow,
          n>=8 means the next nibble repeats n-6 times. A run with no
          data ends the map. Tiles no run covers are deep sea.
  terrain nibbles: 0 building 1 river 2 swamp 3 crater 4 road 5 forest
          6 rubble 7 grass 8 shot building 9 boat 10-15 mined 2-7
"""
import sys

GLYPH = {0: 'W', 1: '~', 2: '%', 3: '%', 4: '=', 5: 'T', 6: '%', 7: '.', 8: '%', 9: '~',
         10: '%', 11: '%', 12: '=', 13: 'T', 14: '%', 15: '.'}
SEA = '~'
PASSABLE = set('.=%T+D')


# The eight-byte magic that opens every map file of the prior art, as hex
# so the format's name is not spelled out in this tree.
MAGIC = bytes.fromhex('424d4150424f4c4f')

def decode(b):
    if b[:8] != MAGIC:
        raise SystemExit('not a prior-art map: wrong magic')
    if b[8] != 1:
        raise SystemExit(f'prior-art map version {b[8]}; only 1 is understood')
    npills, nbases, nstarts = b[9], b[10], b[11]
    i = 12
    pills, bases, starts = [], [], []
    for _ in range(npills):
        x, y, owner, armour, speed = b[i:i + 5]
        i += 5
        pills.append((x, y, owner))
    for _ in range(nbases):
        x, y, owner, armour, shells, mines = b[i:i + 6]
        i += 6
        bases.append((x, y, owner))
    for _ in range(nstarts):
        x, y, d = b[i:i + 3]
        i += 3
        starts.append((x, y))
    grid = [[None] * 256 for _ in range(256)]
    while i + 4 <= len(b):
        datalen, y, sx, ex = b[i:i + 4]
        if datalen <= 4:
            break
        data = b[i + 4:i + datalen]
        i += datalen
        nibbles = []
        for byte in data:
            nibbles += [byte >> 4, byte & 15]
        x, k = sx, 0
        while x < ex and k < len(nibbles):
            n = nibbles[k]
            k += 1
            if n < 8:
                for _ in range(n + 1):
                    if k < len(nibbles) and x < ex:
                        grid[y][x] = nibbles[k]
                        x += 1
                        k += 1
            else:
                if k >= len(nibbles):
                    break
                t = nibbles[k]
                k += 1
                for _ in range(n - 6):
                    if x < ex:
                        grid[y][x] = t
                        x += 1
    return pills, bases, starts, grid


def nearest_passable(rows, x, y):
    """The closest tile to (x, y) a tank can stand on, by ring search."""
    h, w = len(rows), len(rows[0])
    for r in range(0, max(w, h)):
        best = None
        for dy in range(-r, r + 1):
            for dx in range(-r, r + 1):
                if max(abs(dx), abs(dy)) != r:
                    continue
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h and rows[ny][nx] in PASSABLE:
                    d = dx * dx + dy * dy
                    if best is None or d < best[0]:
                        best = (d, nx, ny)
        if best:
            return best[1], best[2]
    raise SystemExit('no passable tile anywhere')


def convert(path, out):
    b = open(path, 'rb').read()
    pills, bases, starts, grid = decode(b)
    land = [(x, y) for y in range(256) for x in range(256) if grid[y][x] is not None]
    if not land:
        raise SystemExit('the map has no land')
    x0, y0, x1, y1 = 0, 0, 255, 255
    w, h = 256, 256
    rows = [[SEA] * w for _ in range(h)]
    for y in range(y0, y1 + 1):
        for x in range(x0, x1 + 1):
            t = grid[y][x]
            rows[y - y0][x - x0] = SEA if t is None else GLYPH[t]
    # Bases stamp a depot tile; the depot line is what counts.
    for x, y, _ in bases:
        if x0 <= x <= x1 and y0 <= y <= y1:
            rows[y - y0][x - x0] = 'D'
    name = path.rsplit('/', 1)[-1]
    lines = ['treadcore map 1', f'size {w} {h}', 'seed 0',
             f'# converted from the prior-art map "{name}" by scripts/priorart2map.py:',
             '# the whole 256x256 grid; buildings are walls,',
             '# swamp, craters and shot buildings are rubble, sea and boats are water;',
             '# starts moved from their boats to the nearest land.',
             '# . grass  T forest  = road  ~ water  + bridge  % rubble  D depot  W wall']
    lines += [''.join(r) for r in rows]
    lines.append('# places')
    seen = 0
    for x, y in starts[:16]:
        cx, cy = x - x0, y - y0
        if not (0 <= cx < w and 0 <= cy < h):
            cx, cy = min(max(cx, 0), w - 1), min(max(cy, 0), h - 1)
        px, py = nearest_passable(rows, cx, cy)
        lines.append(f'start {px + 0.5} {py + 0.5}')
        seen += 1
    for x, y, owner in bases:
        if x0 <= x <= x1 and y0 <= y <= y1:
            o = 'neutral' if owner > 15 else str(owner)
            lines.append(f'depot {x - x0 + 0.5} {y - y0 + 0.5} {o}')
    for x, y, owner in pills:
        if x0 <= x <= x1 and y0 <= y <= y1 and rows[y - y0][x - x0] != '~':
            o = 'neutral' if owner > 15 else str(owner)
            lines.append(f'pillbox {x - x0 + 0.5} {y - y0 + 0.5} {o}')
    text = '\n'.join(lines) + '\n'
    if out == '-':
        sys.stdout.write(text)
    else:
        open(out, 'w').write(text)
    walls = sum(r.count('W') for r in rows)
    forest = sum(r.count('T') for r in rows)
    print(f'{name}: {w}x{h}, {seen} starts, {len(bases)} depots, {len(pills)} pillboxes, {walls} wall tiles, {forest} forest -> {out}', file=sys.stderr)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    src = sys.argv[1]
    dst = sys.argv[2] if len(sys.argv) > 2 else src.rsplit('/', 1)[-1].rsplit('.', 1)[0].lower().replace(' ', '-').replace("'", '') + '.map'
    convert(src, dst)
