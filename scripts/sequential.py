#!/usr/bin/env python3
"""The sequential census (backlog 59, LR 78 and 79).

A census looks K times, at each K-th of its seeds, and stops at the first
look past the O'Brien-Fleming bound |z_k| > c sqrt(K / k), c found by
simulation so that, at a true difference of zero, the chance of stopping
at any look is P(|Z| > 2) = 0.0455: the false alarm rate of the fixed
census's "outside two errors". The jobs' seeds are interleaved by round,
job order by first seed, the order in which parallel jobs finish them.

An early stop is not a verdict (LR 78: LR 73's `-column` stopped out at
100 seeds and read inside at 200). It is read again on fresh seeds, a
second sequential census; the reading is outside only if the re-read is
outside the same way. A census that reaches its last look is the fixed
census, read as before.

    sequential.py replay [-k LOOKS] DIR...   finished censuses, one stage
                                             against the fixed (LR 78)
    sequential.py twostage [-k LOOKS] DIR... the two stages against the
                                             fixed: a simulation, and a
                                             replay, re-reads drawn from
                                             the census's own seeds
    sequential.py watch DIR BIN [-k LOOKS]   a census as it plays (census.sh
                                             LOOKS=): each look to
                                             DIR/sequential.txt; exit 3 at
                                             an early stop, 0 at the end
    sequential.py verdict DIR                the verdict of DIR and its
                                             re-read, DIR/reread

A census whose every seed reads 0 (the same brain on both sides) is
skipped in a replay.
"""
import argparse, glob, math, os, random, re, subprocess, sys, time

ALPHA = math.erfc(2 / math.sqrt(2))
OBF = lambda k: lambda j: math.sqrt(k / j)
POCOCK = lambda k: lambda j: 1.0


def bound_c(shape, k, sims=200000, rng=None):
    """The c for which P(max_j |z_j| / shape(j) > c) = ALPHA at mean 0."""
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


def stats(xs):
    """Mean, standard error and z of the seed differences."""
    n = len(xs)
    mean = sum(xs) / n
    sd = math.sqrt(sum((x - mean) ** 2 for x in xs) / (n - 1)) if n > 1 else 0.0
    se = sd / math.sqrt(n)
    return mean, se, mean / se if se else 0.0


def z(xs):
    return stats(xs)[2]


def looks(n, k):
    return [round(j * n / k) for j in range(1, k + 1)]


def run(xs, k, c, shape):
    """One stage over xs: (seeds spent, the look it stopped at or None, z there)."""
    s = q = 0.0; i = 0; zj = 0.0
    for j, m in enumerate(looks(len(xs), k), 1):
        for x in xs[i:m]:
            s += x; q += x * x
        i = m
        var = (q - s * s / m) / (m - 1) if m > 1 else 0.0
        zj = s / m / math.sqrt(var / m) if var > 0 else 0.0
        if abs(zj) > c * shape(j):
            return m, j, zj
    return len(xs), None, zj


def interleave(jobs):
    longest = max(map(len, jobs), default=0)
    return [j[r] for r in range(longest) for j in jobs if r < len(j)]


def logs(d):
    """DIR's job logs, in order of their first seed."""
    def first(f):
        m = re.search(r"(\d+)\.log$", os.path.basename(f))
        return int(m.group(1)) if m else 0
    return sorted(glob.glob(os.path.join(d, "*.log")), key=first)


def seeds(d):
    """A finished census's seed differences (its "a seed:" lists), interleaved."""
    jobs = []
    for f in logs(d):
        for line in open(f, errors="replace"):
            if line.startswith("brain_census: difference"):
                jobs.append([float(x) for x in line.split("a seed:")[1].split()])
    return interleave(jobs)


HEAD = re.compile(r"^seed ([0-9a-f]+) on ")
TEAM = re.compile(r"^  team (\d+)( \(trial\))?: score (-?[0-9.]+)")


def played(f, plays=3):
    """A job's seeds finished so far, from its match lines (fleet.rs
    `brain_census`): each seed's trial team's mean score a seat less the
    baseline's, over its plays. The scores are printed to 2 places, so a
    difference may be off the census's own by 0.01."""
    out, cur, acc = [], None, None
    try:
        lines = open(f, errors="replace")
    except FileNotFoundError:
        return out
    for line in lines:
        m = HEAD.match(line)
        if m:
            if m[1] != cur:
                cur, acc = m[1], [0.0, 0, 0.0, 0]
            continue
        m = TEAM.match(line)
        if m and acc is not None:
            if m[2]:
                acc[0] += float(m[3]); acc[1] += 1
            else:
                acc[2] += float(m[3]); acc[3] += 1
            if acc[1] == plays and acc[3] == 2 * plays:
                out.append(acc[0] / acc[1] - acc[2] / acc[3])
                acc = None
    return out


def runenv(d):
    env = {}
    for line in open(os.path.join(d, "run.env")):
        k, _, v = line.strip().partition("=")
        env[k] = v.strip("'\"")
    return env


def watch(a):
    """Follow a census as it plays; write each look; stop early or at the end."""
    env = runenv(a.dir)
    n, s0, jobs = int(env["SEEDS"]), int(env["SEED0"]), int(env["JOBS"])
    per = -(-n // jobs)
    starts = list(range(s0, s0 + n, per))
    plan = [min(per, s0 + n - s) for s in starts]
    k, shape = a.k, OBF(a.k)
    c = bound_c(shape, k)
    at = looks(n, k)
    out = open(os.path.join(a.dir, "sequential.txt"), "a")
    def say(line):
        print(line, file=out, flush=True)
        print(line, flush=True)
    say(f"sequential: {n} seeds, {k} looks at {at}; O'Brien-Fleming c {c:.3f}")
    j, began = 0, time.time()
    while True:
        done = [played(os.path.join(a.dir, f"g-{s}.log"))[:p] for s, p in zip(starts, plan)]
        # The whole rounds: seed r of every job that has an r-th to play.
        r = 0
        while any(p > r for p in plan) and all(len(d) > r for d, p in zip(done, plan) if p > r):
            r += 1
        xs = interleave([d[:r] for d in done])
        while j < k and len(xs) >= at[j]:
            mean, se, zj = stats(xs[:at[j]])
            bound = c * shape(j + 1)
            past = abs(zj) > bound
            say(f"look {j + 1} of {k}: {at[j]} seeds, {mean:+.2f} +/- {se:.2f} a seat, z {zj:+.2f}, bound {bound:.2f}{', past it' if past else ''}")
            j += 1
            if past and j < k:
                say(f"stop: past the bound early, z {zj:+.2f} at {at[j - 1]} seeds")
                return 3
        if j == k:
            mean, se, zj = stats(xs)
            say(f"end: z {zj:+.2f} over {len(xs)} seeds, {mean:+.2f} +/- {se:.2f} a seat")
            return 0
        alive = time.time() - began < 30 or subprocess.run(["pgrep", "-f", a.bin], stdout=subprocess.DEVNULL).returncode == 0
        if not alive:
            say(f"end: the jobs ended with {len(xs)} whole rounds of {n} seeds; read pooled.txt")
            return 0
        time.sleep(a.poll)


def last(d):
    """(stopped early, z) from DIR/sequential.txt's last stop or end line."""
    p = os.path.join(d, "sequential.txt")
    if not os.path.exists(p):
        return None
    for line in reversed(open(p).read().splitlines()):
        m = re.match(r"(stop|end): .*?z ([-+][0-9.]+)", line)
        if m:
            return m[1] == "stop", float(m[2])
    return None


def verdict(a):
    one, two = last(a.dir), last(os.path.join(a.dir, "reread"))
    if one is None:
        print("verdict: no sequential reading in", a.dir); return 1
    stopped, z1 = one
    if not stopped:
        print(f"verdict: {'outside' if abs(z1) > 2 else 'inside'} the noise, the census to its end (z {z1:+.2f})")
        return 0
    if two is None:
        print(f"verdict: none yet; stopped early at z {z1:+.2f}, the re-read not read")
        return 1
    _, z2 = two
    same = (z2 > 0) == (z1 > 0)
    out = same and (two[0] or abs(z2) > 2)
    print(f"verdict: {'outside' if out else 'inside'} the noise: stopped early at z {z1:+.2f}, re-read on fresh seeds z {z2:+.2f}{'' if same else ' (the other way)'}")
    return 0


def twostage_once(xs, rest, k, c, shape):
    """Two stages: seeds spent and the verdict (+1, -1 or 0). `rest()` gives
    the re-read's seeds."""
    m, j, z1 = run(xs, k, c, shape)
    if j is None or j == k:
        return m, (1 if z1 > 2 else -1 if z1 < -2 else 0)
    ys = rest()
    m2, j2, z2 = run(ys, k, c, shape)
    out = (z2 > 0) == (z1 > 0) and (j2 is not None or abs(z2) > 2)
    return m + m2, ((1 if z1 > 0 else -1) if out else 0)


def twostage(a):
    k, shape = a.k, OBF(a.k)
    c = bound_c(shape, k)
    rng = random.Random(79)
    print(f"{k} looks, O'Brien-Fleming c {c:.3f}; the fixed census outside past z 2\n")
    # The simulation: N seeds of a true difference `d` errors a seed (d sqrt(N)
    # the fixed census's expected z), unit spread.
    N, sims = 200, 4000
    print(f"simulated, {N} seeds a stage, {sims} censuses a row:")
    print(f"{'expected z':>10} {'fixed out':>10} {'two-stage out':>14} {'one stage out':>14} {'seeds, two-stage':>17}")
    for ez in (0, 1, 2, 2.5, 3, 4, 6, 10):
        d = ez / math.sqrt(N)
        fixed = two = one = spent = 0
        for _ in range(sims):
            draw = lambda: [rng.gauss(d, 1) for _ in range(N)]
            xs = draw()
            fixed += abs(z(xs)) > 2
            one += run(xs, k, c, shape)[1] is not None
            m, v = twostage_once(xs, draw, k, c, shape)
            two += v != 0; spent += m
        print(f"{ez:10.1f} {fixed / sims:10.1%} {two / sims:14.1%} {one / sims:14.1%} {spent / sims / N:16.0%}")
    # The replay: the census's own seeds for stage one; a re-read drawn
    # from them with replacement, 200 times.
    print(f"\n{'census':48} {'seeds':>5} {'fixed z':>8} {'stop':>5} {'agree':>6} {'seeds spent':>11}")
    tot_fixed = tot_two = 0.0; differ = []
    for d in a.dirs:
        xs = seeds(d)
        n = len(xs)
        if n < 2 * k or not any(xs):
            continue
        zf = z(xs)
        vf = 1 if zf > 2 else -1 if zf < -2 else 0
        m1, j, _ = run(xs, k, c, shape)
        boots = [twostage_once(xs, lambda: [rng.choice(xs) for _ in range(n)], k, c, shape) for _ in range(200)]
        agree = sum(v == vf for _, v in boots) / len(boots)
        spent = sum(m for m, _ in boots) / len(boots)
        tot_fixed += n; tot_two += spent
        if agree < 0.5:
            differ.append(f"{d} (fixed z {zf:+.2f}, agrees {agree:.0%})")
        print(f"{d[-48:]:48} {n:5d} {zf:+8.2f}{'*' if vf else ' '} {('-' if j is None else m1):>5} {agree:6.0%} {spent:11.0f}")
    print(f"\nseeds: fixed {tot_fixed:.0f}, two-stage {tot_two:.0f} ({tot_two / max(tot_fixed, 1):.0%})")
    print(f"verdicts the two stages miss more often than not: {len(differ)}")
    for line in differ:
        print(f"    {line}")


def replay(a):
    k = a.k
    tests = {"OBF": OBF(k), "Pocock": POCOCK(k)}
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
            stop, j, zs = run(xs, k, cs[t], shape)
            out = j is not None
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


def main():
    argv = sys.argv[1:]
    if argv and argv[0] not in ("replay", "twostage", "watch", "verdict", "-h", "--help"):
        argv = ["replay"] + argv  # LR 78's form: sequential.py [-k LOOKS] DIR...
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("replay", "twostage"):
        p = sub.add_parser(name)
        p.add_argument("-k", type=int, default=8, help="looks (default 8)")
        p.add_argument("dirs", nargs="+")
    p = sub.add_parser("watch")
    p.add_argument("dir"); p.add_argument("bin")
    p.add_argument("-k", type=int, default=8)
    p.add_argument("--poll", type=float, default=20)
    p = sub.add_parser("verdict")
    p.add_argument("dir")
    a = ap.parse_args(argv)
    sys.exit({"replay": replay, "twostage": twostage, "watch": watch, "verdict": verdict}[a.cmd](a) or 0)


if __name__ == "__main__":
    main()
