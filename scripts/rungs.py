#!/usr/bin/env python3
"""Where a switch moves the trial team's time, map by map (LR 82).

Reads the match lines of a `make switches` directory (or any census
logs): for each row and map, the trial team's seat-seconds on the
Expand, Turrets, Hunt and Fortify rungs, and its held ground, built and
bridges less the mean of the other two teams in the same match. Then,
over every row pooled, the slope of held ground on Expand seconds a map.

    scripts/rungs.py build/switches/STAMP -forget -posture team_rotate
"""
import glob
import re
import sys
from math import sqrt

HEAD = re.compile(r'^seed (\S+) on (\S+): trial team (\d)')
TEAM = re.compile(r'^  team (\d)( \(trial\))?: score (-?[\d.]+), kills \d+, deaths \d+, '
                  r'revives \d+, built (\d+), road \d+, bridge (\d+), held ([\d.]+)')
RUNG = re.compile(r'seat-seconds a rung: (.*)')
KEYS = ['expand', 'turrets', 'hunt', 'fortify']


def matches(paths):
    """Each match: (map, rung seconds, [held, built, bridge] less the others)."""
    for log in paths:
        cur = None
        for line in open(log, errors='replace'):
            h = HEAD.match(line)
            if h:
                cur = {'map': h.group(2), 't': int(h.group(3)), 'v': {}}
                continue
            if cur is None:
                continue
            t = TEAM.match(line)
            if t:
                cur['v'][int(t.group(1))] = (float(t.group(6)), int(t.group(4)), int(t.group(5)))
                continue
            r = RUNG.search(line)
            if r and len(cur['v']) == 3:
                rg = {k.strip().rsplit(' ', 1)[0]: int(k.strip().rsplit(' ', 1)[1]) for k in r.group(1).split(',')}
                o = [v for k, v in cur['v'].items() if k != cur['t']]
                me = cur['v'][cur['t']]
                yield cur['map'], rg, [me[i] - (o[0][i] + o[1][i]) / 2 for i in range(3)]


def main():
    d, rows = sys.argv[1], sys.argv[2:]
    res, pts = {}, {}
    for row in rows:
        for m, rg, diff in matches(glob.glob(f'{d}/{row}/*.log')):
            res.setdefault((m, row), []).append([rg.get(k, 0) for k in KEYS] + diff)
    for p in glob.glob(f'{d}/*/*.log'):
        for m, rg, diff in matches([p]):
            pts.setdefault(m, []).append((rg.get('expand', 0), diff[0]))
    print('trial seat-seconds a rung; held, built, bridge less the other two teams')
    for m in sorted({m for m, _ in res}):
        print(m)
        for row in rows:
            x = res.get((m, row), [])
            if not x:
                continue
            a = [sum(c[i] for c in x) / len(x) for i in range(7)]
            print(f'  {row:16s} expand {a[0]:5.0f} turrets {a[1]:4.0f} hunt {a[2]:4.0f} fortify {a[3]:4.0f}'
                  f' | held {a[4]:+.2f} built {a[5]:+5.1f} bridge {a[6]:+4.1f}  ({len(x)} matches)')
    print('\nheld ground a 100 Expand seconds, every row pooled')
    for m, p in sorted(pts.items()):
        n = len(p)
        mx, my = sum(x for x, _ in p) / n, sum(y for _, y in p) / n
        sxx = sum((x - mx) ** 2 for x, _ in p)
        if n < 3 or sxx == 0:
            continue
        b = sum((x - mx) * (y - my) for x, y in p) / sxx
        se = sqrt(sum((y - my - b * (x - mx)) ** 2 for x, y in p) / (n - 2) / sxx)
        print(f'  {m:16s} {100 * b:+.2f} +/- {100 * se:.2f} over {n} matches, {mx:.0f} s mean')


if __name__ == '__main__':
    main()
