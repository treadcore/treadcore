#!/usr/bin/env python3
"""How a census's trial team plays, against the others (LR 85).

For each seed, the trial team's counts less the mean of the other two
teams in the same match, averaged over the seed's three matches (each
team in the trial seat once); then the mean over the seeds and its
standard error. The counts: the score and its rewards, kills, deaths,
built, held ground, direct hits, pillbox damage, pillboxes taken, and
the trial team's seconds on each rung (those against the seed's other
two trial matches are the same brain, so a rung reads as the trial's
seconds alone). Outside two errors is marked with a *.

    scripts/traits.py build/census/STAMP [...]
"""
import glob
import re
import sys
from math import sqrt

HEAD = re.compile(r'^seed (\S+) on (\S+): trial team (\d)')
TEAM = re.compile(r'^  team (\d)( \(trial\))?: score (-?[\d.]+), kills (\d+), deaths (\d+), revives \d+, '
                  r'built (\d+), road \d+, bridge \d+, held ([\d.]+)(?:, rewards (-?[\d.]+) \(direct hits (\d+), '
                  r'pillbox damage ([\d.]+), pillboxes taken (\d+) neutral (\d+) hostile, defends (\d+))?')
RUNG = re.compile(r'seat-seconds a rung: (.*)')
SPENT = re.compile(r', spent( \(trial\))?: direct (\d+), indirect (\d+), recon (\d+), interceptor (\d+), strike (\d+), longshot (\d+)')
SPENDS = ['direct shells', 'indirect shells', 'recon', 'interceptor', 'strike', 'longshot']
FIELDS = ['score', 'kills', 'deaths', 'built', 'held', 'rewards', 'direct hits', 'pillbox damage',
          'neutral taken', 'hostile taken', 'defends'] + SPENDS
RUNGS = ['defend base', 'fortify', 'turrets', 'hunt', 'expand', 'guard']


def seeds(d):
    """seed -> list of matches: (the trial less the others' counts, the trial's rung seconds)."""
    out = {}
    for log in glob.glob(f'{d}/*.log') + glob.glob(f'{d}/*/*.log'):
        cur = None
        for line in open(log, errors='replace'):
            h = HEAD.match(line)
            if h:
                cur = {'seed': h.group(1), 't': int(h.group(3)), 'v': {}}
                continue
            if cur is None:
                continue
            t = TEAM.match(line)
            if t:
                g = [t.group(k) for k in (3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13)]
                cur['v'][int(t.group(1))] = [float(x) if x is not None else 0.0 for x in g]
                continue
            if line.startswith('  meter:'):
                # Spent (LR 86): the trial team's, and the other two's
                # halved, as a team's line would read.
                sp = {bool(m.group(1)): [int(m.group(k)) for k in range(2, 8)] for m in SPENT.finditer(line)}
                if sp and 'diff' in cur:
                    me, rest = sp.get(True, [0] * 6), sp.get(False, [0] * 6)
                    out[cur['seed']][-1][11:17] = [me[i] - rest[i] / 2 for i in range(6)]
                cur = None
                continue
            r = RUNG.search(line)
            if r and len(cur['v']) == 3:
                rg = {k.strip().rsplit(' ', 1)[0]: int(k.strip().rsplit(' ', 1)[1]) for k in r.group(1).split(',')}
                me = cur['v'][cur['t']]
                o = [v for k, v in cur['v'].items() if k != cur['t']]
                diff = [me[i] - (o[0][i] + o[1][i]) / 2 for i in range(11)] + [0.0] * len(SPENDS)
                out.setdefault(cur['seed'], []).append(diff + [rg.get(k, 0) for k in RUNGS])
                cur['diff'] = True
    return out


def main():
    for d in sys.argv[1:]:
        s = seeds(d)
        rows = [[sum(m[i] for m in ms) / len(ms) for i in range(len(FIELDS) + len(RUNGS))] for ms in s.values()]
        n = len(rows)
        print(f'{d}: {n} seeds, the trial team less the others a match (rungs: the trial\'s seconds)')
        if n < 2:
            continue
        for i, name in enumerate(FIELDS + [f'{r} s' for r in RUNGS]):
            x = [r[i] for r in rows]
            m = sum(x) / n
            se = sqrt(sum((v - m) ** 2 for v in x) / (n - 1) / n)
            star = '*' if i < len(FIELDS) and se > 1e-6 and abs(m) > 2 * se else ' '
            print(f'  {name:16s} {m:+9.2f} +/- {se:6.2f} {star}')


if __name__ == '__main__':
    main()
