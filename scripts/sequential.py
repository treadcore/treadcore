#!/usr/bin/env python3
"""Replay finished censuses under a group-sequential test (backlog 59, LR 78).

Each DIR is a census (`build/census/STAMP`, `build/switches/STAMP/NAME`,
...): its `*.log` files' "a seed:" lists, as scripts/pool.sh reads them.
The jobs are interleaved by round, job order by first seed, the order in
which parallel jobs finish their seeds. The test looks K times, at each
K-th of the seeds, and stops at the first look past its bound:

    O'Brien-Fleming  |z_k| > c sqrt(K / k)
    Pocock           |z_k| > c

c is found by simulation so that, at a true difference of zero, the
chance of stopping at any look is P(|Z| > 2) = 0.0455: the false alarm
rate of the fixed census's "outside two errors". A census whose every
seed reads 0 (the same brain on both sides) is skipped.

    scripts/sequential.py [-k LOOKS] DIR...
"""
import argparse, glob, math, os, random, re

ALPHA = math.erfc(2 / math.sqrt(2))


def bound_c(shape, k, sims=200000, rng=None):
    """The c for which P(max_k |z_k| / shape(k) > c) = ALPHA at mean 0."""
    rng = rng or random.Random(59)
    worst = []
    for _ in range(sims):
        s, w = 0.0, 0.0
        for j in range(1, k + 1):
            s += rng.gauss(0, 1)
            w = max(w, abs(s / math.sqrt(j)) / shape(j))
        worst.append(w)
    worst.sort()
    return worst[int((1 - ALPHA) * sims)]


def z(xs):
    n = len(xs)
    mean = sum(xs) / n
    sd = math.sqrt(sum((x - mean) ** 2 for x in xs) / (n - 1))
    return mean / (sd / math.sqrt(n)) if sd else 0.0


def seeds(d):
    """The census's seed differences, interleaved across its jobs."""
    jobs = []
    for f in glob.glob(os.path.join(d, "*.log")):
        m = re.search(r"(\d+)\.log$", os.path.basename(f))
        for line in open(f, errors="replace"):
            if line.startswith("brain_census: difference"):
                jobs.append((int(m.group(1)) if m else 0,
                             [float(x) for x in line.split("a seed:")[1].split()]))
    jobs = [xs for _, xs in sorted(jobs, key=lambda j: j[0])]
    longest = max(map(len, jobs), default=0)
    return [j[r] for r in range(longest) for j in jobs if r < len(j)]


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("-k", type=int, default=8, help="looks (default 8)")
    ap.add_argument("dirs", nargs="+")
    a = ap.parse_args()
    k = a.k
    tests = {"OBF": lambda j: math.sqrt(k / j), "Pocock": lambda j: 1.0}
    cs = {t: bound_c(s, k) for t, s in tests.items()}
    print(f"{k} looks, false alarm {ALPHA:.4f}: O'Brien-Fleming c {cs['OBF']:.3f} "
          f"(first look {cs['OBF'] * tests['OBF'](1):.2f}), Pocock c {cs['Pocock']:.3f}\n")
    print(f"{'census':48} {'seeds':>5} {'fixed z':>8} {'OBF':>9} {'Pocock':>9}")
    total = dict.fromkeys(["fixed", *tests], 0)
    differ = {t: [] for t in tests}
    for d in a.dirs:
        xs = seeds(d)
        n = len(xs)
        if n < 2 * k or not any(xs):
            continue
        zf = z(xs)
        fixed = abs(zf) > 2
        total["fixed"] += n
        cells = []
        for t, shape in tests.items():
            stop, out, zs = n, False, zf
            for j in range(1, k + 1):
                m = round(j * n / k)
                zj = z(xs[:m])
                if abs(zj) > cs[t] * shape(j):
                    stop, out, zs = m, True, zj
                    break
            total[t] += stop
            if out != fixed or (out and (zs > 0) != (zf > 0)):
                differ[t].append(f"{d} (fixed z {zf:+.2f}, stopped at {stop})")
            cells.append(f"{stop:4d} {'out' if out else 'in'}")
        print(f"{d[-48:]:48} {n:5d} {zf:+8.2f}{'*' if fixed else ' '} {cells[0]:>9} {cells[1]:>9}")
    print(f"\nseeds: fixed {total['fixed']}" + "".join(
        f", {t} {total[t]} ({total[t] / max(total['fixed'], 1):.0%})" for t in tests))
    for t in tests:
        print(f"{t}, verdicts unlike the fixed census: {len(differ[t])}")
        for line in differ[t]:
            print(f"    {line}")


if __name__ == "__main__":
    main()
