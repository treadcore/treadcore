#!/usr/bin/env python3
"""floors.py [--give G] [--write] FAMILIES.txt [GAUNTLET.tsv] -- the floors of the families and of the
gauntlet, read against what a brain just scored: a small drop passes and moves its floor, a collapse stops.

Paul, 13 November 2026: "adapting the tests to just pass with current numbers. Then we can progress
forward." A floor was a wall: a brain better by three points a seat and a point under one file's floor was
refused whole. Now each file is one of three: on its floor; under it by no more than the give (a share,
0.05 unless said), which passes, and with --write its floor in the scenario file is moved down to what it
scored less a hundredth, so the next brain is held to this one; or under by more, which is a collapse
(a-raid-on-the-held-ground at 0 of 400) and exits 1 with nothing written.

FAMILIES.txt is `every_family_holds_its_floor`'s lines (`family NAME [set]: held H of R, P% ..., floor F%,
verdict`); GAUNTLET.tsv is the gauntlet's table (name, brain, ..., share in the fifth column), read for
the scripted brain against scenarios/gauntlet/*.toml. Floors only ever move down here: raising one is a
person's, in the file. LR 158."""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def set_floor(path, new):
    text = open(path).read()
    out = re.sub(r"^floor\s*=\s*[\d.]+", "floor = %.2f" % new, text, count=1, flags=re.M)
    if out != text:
        open(path, "w").write(out)


def main(a):
    give = float(a[a.index("--give") + 1]) if "--give" in a else 0.05
    write = "--write" in a
    files = [x for i, x in enumerate(a) if not x.startswith("--") and (i == 0 or a[i - 1] != "--give")]
    rows = []  # (what, name, file, share, floor)
    for line in open(files[0]).read().splitlines():
        m = re.match(r"^family (\S+) \[.*\]: held (\d+) of (\d+), .*floor ([\d.]+)%", line)
        if m:
            rows.append(("family", m.group(1), os.path.join(ROOT, "scenarios", "families", m.group(1)), int(m.group(2)) / max(1, int(m.group(3))), float(m.group(4)) / 100))
    if len(files) > 1 and os.path.exists(files[1]):
        floors = {}
        for p in glob.glob(os.path.join(ROOT, "scenarios", "gauntlet", "*.toml")):
            t = open(p).read()
            n, f = re.search(r'^name\s*=\s*"([^"]+)', t, re.M), re.search(r"^floor\s*=\s*([\d.]+)", t, re.M)
            if n and f:
                floors[n.group(1)] = (p, float(f.group(1)))
        for line in open(files[1]).read().splitlines()[1:]:
            c = line.split("\t")
            if len(c) >= 5 and c[1] == "scripted" and c[0] in floors:
                rows.append(("gauntlet", c[0], floors[c[0]][0], float(c[4]), floors[c[0]][1]))
    fell = [r for r in rows if r[3] < r[4] - give - 1e-9]
    drops = [r for r in rows if r[4] - give - 1e-9 <= r[3] < r[4] - 1e-9]
    for what, name, path, share, floor in fell:
        print("%s %s: %.1f%% is a collapse under its floor %.1f%% (the give is %.1f points)" % (what, name, 100 * share, 100 * floor, 100 * give))
    if fell:
        print("floors: %d collapsed: nothing written" % len(fell))
        return 1
    for what, name, path, share, floor in drops:
        new = max(0.0, int(share * 100 - 1 + 1e-9) / 100)
        print("%s %s: %.1f%% is a small drop under its floor %.1f%%: it passes%s" % (what, name, 100 * share, 100 * floor, ", and its floor moves to %.2f" % new if write else ""))
        if write:
            set_floor(path, new)
    print("floors: %d read, %d small drop(s)%s, none collapsed" % (len(rows), len(drops), " moved" if write and drops else ""))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
