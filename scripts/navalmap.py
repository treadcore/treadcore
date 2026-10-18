#!/usr/bin/env python3
"""Draw the naval map: an archipelago with every start at sea.

    python3 -I scripts/navalmap.py maps/naval.map

A 160 x 160 sea with a large island in the middle, four smaller ones
around it, and a ring of islets. Every tank starts on a landing craft
(SPEC TNK-08) between the islands and the edge, so the first thing a
match asks is where to land. Coasts are circles roughened by a fixed
sum of sines, so the output is the same every run. Forest on every
island, for boats and walls; a depot on each of the five islands and
on each islet; neutral pillboxes on the big island's beaches.
"""
import math
import sys

W = H = 160
C = (80.0, 80.0)

# (x, y, radius, forest share)
ISLANDS = [(80, 80, 26, 0.35)]
for i in range(4):
    a = math.pi / 4 + i * math.pi / 2
    ISLANDS.append((80 + 44 * math.cos(a), 80 + 44 * math.sin(a), 12, 0.5))
for i in range(8):
    a = i * math.pi / 4
    ISLANDS.append((80 + 52 * math.cos(a), 80 + 52 * math.sin(a), 5, 0.4))


def rough(x, y, k):
    """A fixed wobble for coastline k at angle of (x, y)."""
    return 2.2 * math.sin(3 * x * 0.11 + k) + 1.6 * math.sin(y * 0.17 - 2 * k) + 1.1 * math.sin((x + y) * 0.23 + k * 0.7)


def forest_noise(x, y):
    return math.sin(x * 0.31) * math.cos(y * 0.27) + 0.5 * math.sin((x - y) * 0.19)


def main(out):
    grid = [['~'] * W for _ in range(H)]
    for k, (ix, iy, r, share) in enumerate(ISLANDS):
        for y in range(H):
            for x in range(W):
                d = math.hypot(x + 0.5 - ix, y + 0.5 - iy)
                if d <= r + rough(x, y, k) * (r / 26.0 + 0.3):
                    # Forest inland and in clumps; a grass beach all round.
                    inland = d < r - 3
                    grid[y][x] = 'T' if inland and forest_noise(x, y) > 1.0 - 2 * share else '.'

    starts, depots, boxes = [], [], []
    # Sixteen starts at sea, in three sectors of the compass, one for
    # each team as the window deals them (seat i is on team i % 3): seat
    # i in sector i % 3, the team's first seats nearest the sector's
    # middle and the rest fanned out either side, on two rings, so a
    # team starts together and lands on the same shores.
    for i in range(16):
        team, k = i % 3, i // 3
        fan = [0, 1, -1, 2, -2, 3][k]
        a = team * 2 * math.pi / 3 - math.pi / 2 + fan * math.radians(15)
        rad = 70 if k % 2 == 0 else 62
        x, y = C[0] + rad * math.cos(a), C[1] + rad * math.sin(a)
        x, y = min(max(x, 3), W - 4), min(max(y, 3), H - 4)
        # Nudge out to open water.
        while grid[int(y)][int(x)] != '~':
            x += math.cos(a)
            y += math.sin(a)
        starts.append((int(x) + 0.5, int(y) + 0.5))

    def land_at(px, py):
        return 0 <= px < W and 0 <= py < H and grid[int(py)][int(px)] in '.T'

    # A depot on each island and islet, a few tiles in from its seaward
    # coast; the big island's in its middle.
    for k, (ix, iy, r, _) in enumerate(ISLANDS):
        if k == 0:
            px, py = ix, iy
        else:
            a = math.atan2(iy - C[1], ix - C[0])
            px, py = ix + (r * 0.4) * math.cos(a), iy + (r * 0.4) * math.sin(a)
        if not land_at(px, py):
            px, py = ix, iy
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                if land_at(px + dx, py + dy):
                    grid[int(py + dy)][int(px + dx)] = '.'
        grid[int(py)][int(px)] = 'D'
        depots.append((int(px) + 0.5, int(py) + 0.5))
    # Pillboxes on the big island, guarding its beaches.
    for i in range(6):
        a = i * math.pi / 3
        px, py = C[0] + 16 * math.cos(a), C[1] + 16 * math.sin(a)
        if land_at(px, py):
            grid[int(py)][int(px)] = '.'
            boxes.append((int(px) + 0.5, int(py) + 0.5))

    lines = [
        'treadcore map 1',
        f'size {W} {H}',
        'seed 0x5ea5',
        '# naval, drawn by scripts/navalmap.py: an archipelago, every start at sea on',
        '# a landing craft (SPEC TNK-08); forest on every island for boats and walls',
        '# rows: . grass  T forest  = road  ~ water  + bridge  % rubble  D depot  W wall',
        '# places: start x y (seat order); depot x y [seat|neutral]; pillbox x y [seat|neutral]',
    ]
    lines += [''.join(r) for r in grid]
    lines += [f'start {x} {y}' for x, y in starts]
    lines += [f'depot {x} {y} neutral' for x, y in depots]
    lines += [f'pillbox {x} {y} neutral' for x, y in boxes]
    with open(out, 'w') as f:
        f.write('\n'.join(lines) + '\n')
    print(f'naval: {W}x{H}, {len(starts)} starts at sea, {len(depots)} depots, {len(boxes)} pillboxes -> {out}', file=sys.stderr)


if __name__ == '__main__':
    main(sys.argv[1] if len(sys.argv) > 1 else 'maps/naval.map')
