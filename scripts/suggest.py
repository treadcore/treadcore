#!/usr/bin/env python3
"""The next generation of trials, from the ledger (LR 92).

    scripts/suggest.py LEDGER DEFAULTS OUT [--since STAMP] [--n 12]
                       [--bound 2] [--seed 1] [--skip w1,w2]

LEDGER is `scripts/ledger.py`'s table; DEFAULTS the brain's weights
(`CENSUS_DUMP`); OUT a directory for the trial files and `plan.tsv`.

The model, a response surface: each census against the defaults since
STAMP (the defaults' last change) reads the score difference d with
its error se. A weight set to v is x = log2(v / default), from -1
(half) to +1 (double) at BOUND 2. d is fit as the sum, over weights,
of a x + b x^2, by least squares weighted by 1 / se^2 with a little
ridge, so each weight's curve is what the screens and searches read of
it, alone and in company. Weights the ledger never moved are unknown.

The generation, in this order, N files in all:
  1. best: every weight at its curve's top within the bound where the
     top is past its error above zero, the rest at their defaults;
  2. best less one: (1) with each of its moved weights back, the two
     most uncertain;
  2a. again: a trial past 1.5 errors above on fewer than 110 seeds,
     read once, read again on fresh seeds (successive halving);
  3. samples: weights drawn from the fit's uncertainty (Thompson), each
     at its drawn curve's top, four at a time, so the uncertain are
     tried;
  4. unexplored: weights never moved, at half and double, a screen;
     they keep up to half the generation.
None of it changes a default: a search suggests, a census reads, the
held-out rule decides, Paul promotes.
"""
import argparse
import csv
import math
import os
import random
import re


# Weights from 0 to 1 (a share, a blend): above the default they stop
# halfway to 1 rather than double past it.
UNIT = set('w_smooth alpha beta margin commit connect_bases w_posture w_duck advance_band swap_gap'.split())


def defaults(path):
    out = {}
    for l in open(path):
        l = l.split('#', 1)[0].strip()
        m = re.match(r'^(\w+)\s*=\s*(-?[\d.eE+-]+)$', l)
        if m:
            out[m.group(1)] = float(m.group(2))
    return out


def trial(text):
    out = {}
    for kv in text.split(';'):
        m = re.match(r'^(\w+)=(-?[\d.eE+-]+)$', kv.strip())
        if m:
            out[m.group(1)] = float(m.group(2))
    return out


def solve(a, b):
    """Gaussian elimination with partial pivoting: a x = b; also a's inverse."""
    n = len(a)
    m = [row[:] + [b[i]] + [1.0 if j == i else 0.0 for j in range(n)] for i, row in enumerate(a)]
    for c in range(n):
        p = max(range(c, n), key=lambda r: abs(m[r][c]))
        m[c], m[p] = m[p], m[c]
        if abs(m[c][c]) < 1e-12:
            continue
        piv = m[c][c]
        m[c] = [v / piv for v in m[c]]
        for r in range(n):
            if r != c and m[r][c] != 0.0:
                f = m[r][c]
                m[r] = [vr - f * vc for vr, vc in zip(m[r], m[c])]
    return [m[i][n] for i in range(n)], [m[i][n + 1:] for i in range(n)]


def top(a, b, lo, hi):
    """Where a x + b x^2 is highest on [lo, hi], and its height."""
    xs = [lo, hi, 0.0] + ([-a / (2 * b)] if b < 0 and lo < -a / (2 * b) < hi else [])
    return max(((a * x + b * x * x, x) for x in xs))[::-1]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('ledger')
    ap.add_argument('defaults')
    ap.add_argument('out')
    ap.add_argument('--since', default='20261025-181300')
    ap.add_argument('--n', type=int, default=12)
    ap.add_argument('--bound', type=float, default=2.0)
    ap.add_argument('--seed', type=int, default=1)
    # Not slopes: the skill dial, the score's own blend, people's roles,
    # weights for what does not exist yet or is off, and the switches
    # (thresholded at 0.5), read by the switch census instead.
    ap.add_argument('--skip', default='skill,long_game,assist,upgrade_vs_mend,w_front,w_front_post,w_front_push,front_memory,dodge_hold,'
                    'kite,posture_scored,dodge_every,dodge_fire,dodge_plan,dodge_release,landfall_shy,w_give_item,w_give_upgrade,'
                    'w_connect,bridge_wing,duck_plan,shell_boxes,longshot_any')
    o = ap.parse_args()
    # The choices (a weight whose doc in weights.rs says "(choice: ...)"):
    # a version, not a slope; the planner tries each against the defaults
    # (A/B), and a fit across versions would be a fit across functions.
    rs = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'source', 'treadcore-brain', 'src', 'weights.rs')
    choices = set(re.findall(r'^\s*[A-Z][A-Za-z0-9]*\s+"([a-z_0-9]+)"\s+[-0-9.]+,\s*"[^"]*\(choice: ', open(rs).read(), re.M)) if os.path.exists(rs) else set()
    rng = random.Random(o.seed)
    base = defaults(o.defaults)
    skip = set(filter(None, o.skip.split(','))) | choices
    span = math.log2(o.bound)

    rows, reads = [], []
    for r in csv.DictReader(open(o.ledger), delimiter='\t'):
        # Weights against the defaults only: not a caps census, not a net's (LR 96).
        if r['stamp'] < o.since or r['base'].replace(' ', '') != 'caps=baseline' or 'caps=' in r['trial'] or r['trial'].startswith('net='):
            continue
        t = trial(r['trial'])
        if choices & set(t):
            continue
        x = {w: math.log2(v / base[w]) for w, v in t.items() if w in base and base[w] > 0 and v > 0}
        if len(x) != len(t):
            continue  # a weight at 0 or unknown: not on the log scale
        rows.append((x, float(r['d']), max(float(r['se']), 0.05)))
        reads.append((r['trial'], x, float(r['z']), int(r['seeds'])))
    names = sorted({w for x, _, _ in rows for w in x} - skip)
    k = len(names)
    col = {w: i for i, w in enumerate(names)}
    # Weighted least squares with ridge: features (x, x^2) per weight.
    A = [[0.0] * (2 * k) for _ in range(2 * k)]
    B = [0.0] * (2 * k)
    for x, d, se in rows:
        f = [0.0] * (2 * k)
        for w, v in x.items():
            if w in col:
                f[2 * col[w]], f[2 * col[w] + 1] = v, v * v
        iw = 1.0 / (se * se)
        for i in range(2 * k):
            if f[i]:
                B[i] += iw * f[i] * d
                for j in range(2 * k):
                    if f[j]:
                        A[i][j] += iw * f[i] * f[j]
    for i in range(2 * k):
        A[i][i] += 1.0  # ridge: a coefficient of about +/-1 a seat is a prior's size
    beta, cov = solve(A, B) if k else ([], [])

    curve = {}
    for w in names:
        a, b = beta[2 * col[w]], beta[2 * col[w] + 1]
        x, gain = top(a, b, -span, span)
        # The error of the gain at x: f = (x, x^2), var = f' C f.
        i = 2 * col[w]
        var = x * x * cov[i][i] + 2 * x ** 3 * cov[i][i + 1] + x ** 4 * cov[i + 1][i + 1]
        curve[w] = (a, b, x, gain, math.sqrt(max(var, 0.0)))

    os.makedirs(o.out, exist_ok=True)
    plan = []
    seen = set()  # a trial once a generation

    def value(w, v):
        """A weight's value at x = v; a weight of 0 to 1 stops halfway
        from its default to 1 (as the day's screen, LR 89)."""
        y = base[w] * 2 ** v
        if w in UNIT and y > base[w]:
            y = min(y, base[w] + (1 - base[w]) / 2) if base[w] < 1 else base[w]
        return y

    def emit(name, why, x):
        if len(plan) >= o.n:
            return
        x = {w: v for w, v in x.items() if value(w, v) != base[w]}
        key = tuple(sorted((w, round(value(w, v), 6)) for w, v in x.items()))
        if not x or key in seen:
            return
        seen.add(key)
        path = os.path.join(o.out, name + '.weights')
        with open(path, 'w') as f:
            f.write(f'# suggest.py: {why}\n')
            for w, v in sorted(x.items()):
                f.write(f'{w} = {value(w, v):.6g}\n')
        pred = sum(curve[w][0] * v + curve[w][1] * v * v for w, v in x.items() if w in curve)
        plan.append((name, why, f'{pred:+.2f}', ' '.join(f'{w}={value(w, v):.4g}' for w, v in sorted(x.items()))))

    best = {w: c[2] for w, c in curve.items() if c[3] > 2 * c[4] and abs(c[2]) > 1e-9}
    if best:
        emit('best', 'every weight at its curve\'s top where the top is past two errors', best)
        for w in sorted(best, key=lambda w: -curve[w][4])[:2]:
            emit(f'best-less-{w}', f'best with {w} back at its default', {u: v for u, v in best.items() if u != w})
    read_before = {tuple(sorted((w, round(value(w, v), 6)) for w, v in x.items())) for _, x, _, _ in reads}
    # Read again: a trial past 1.5 errors above on fewer than 110 seeds,
    # read once (successive halving, LR 89): fresh seeds, pooled by the fit.
    times = {}
    for t, x, z, n in reads:
        times[t] = times.get(t, 0) + 1
    for t, x, z, n in sorted(reads, key=lambda r: -r[2]):
        if z >= 1.5 and n < 110 and times[t] == 1 and x:
            emit('again-' + '-'.join(f'{w}{"+" if v > 0 else "-"}' for w, v in sorted(x.items()))[:60], f'read again: z {z:+.2f} on {n} seeds', x)
            times[t] = 2
    # Each weight's two ends, half and double, read alone: the screen.
    # Ends never read keep half the generation, so every weight is read.
    ends = {(w, v > 0) for x, _, _ in rows if len(x) == 1 for w, v in x.items()}
    unexplored = [(w, hi) for w in sorted(set(base) - skip) if base[w] > 0 for hi in (False, True) if (w, hi) not in ends and value(w, span if hi else -span) != base[w]]
    room = o.n - len(plan) - min(len(unexplored), o.n // 2)
    s = 0
    while s < room and names:
        draw = {}
        for w in names:
            i = 2 * col[w]
            sa, sb = math.sqrt(max(cov[i][i], 0)), math.sqrt(max(cov[i + 1][i + 1], 0))
            a, b = rng.gauss(beta[i], sa), rng.gauss(beta[i + 1], sb)
            x, g = top(a, b, -span, span)
            if g > 0 and abs(x) > 1e-9:
                draw[w] = x
        # A few weights at a time, so a census can tell what it read; a
        # draw already read is not read again here (reads again are 2a's).
        pick = dict(rng.sample(sorted(draw.items()), min(4, len(draw)))) if draw else {}
        if pick and tuple(sorted((w, round(value(w, v), 6)) for w, v in pick.items())) not in read_before:
            emit(f'sample-{s}', 'a draw from the fit\'s uncertainty, its top', pick)
        s += 1
    for w, hi in unexplored:
        emit(f'screen-{w}.{"hi" if hi else "lo"}', f'{w} not yet read at {"double" if hi else "half"} alone', {w: span if hi else -span})

    with open(os.path.join(o.out, 'plan.tsv'), 'w') as f:
        f.write('name\twhy\tpredicted\tweights\n')
        for p in plan:
            f.write('\t'.join(p) + '\n')
    with open(os.path.join(o.out, 'curves.tsv'), 'w') as f:
        f.write('weight\ta\tb\ttop_x\ttop_value\tgain\tgain_se\n')
        for w in sorted(curve, key=lambda w: -curve[w][3]):
            a, b, x, g, e = curve[w]
            f.write(f'{w}\t{a:+.3f}\t{b:+.3f}\t{x:+.2f}\t{base[w] * 2 ** x:.4g}\t{g:+.2f}\t{e:.2f}\n')
    print(f'{len(rows)} censuses, {k} weights fit; {len(plan)} trials in {o.out}')


if __name__ == '__main__':
    main()
