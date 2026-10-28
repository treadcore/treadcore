#!/usr/bin/env python3
"""Set the brain's defaults from a weights file (LAB-REPORT-BRAIN.md 115).

    python3 -I scripts/set-defaults.py FILE.weights [--check]

Each `name = value` line of FILE sets that weight's default in
source/treadcore-brain/src/weights.rs, the number on its line in the
`weights!` table:

    PressOdds "press_odds" 2.0, "odds: above these, close in";

`caps` lines and comments are left out: a promotion moves numbers, not
switches. A name the table does not have is an error, and nothing is
written. --check prints what would change and writes nothing. Prints one
line a weight changed: name, the old default, the new.

This is how orrery's promotion (tools/plans/treadcore.py, kind "promote")
writes a candidate into the brain, on a branch, before the tests and the
families run on it and Paul approves it.
"""
import os
import re
import sys

TC = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RS = os.path.join(TC, "source", "treadcore-brain", "src", "weights.rs")
LINE = re.compile(r'^(\s*[A-Z][A-Za-z0-9]*\s+"([a-z_0-9]+)"\s+)(-?[0-9][0-9.eE+-]*)(\s*,)')


def number(v):
    """A Rust f64 literal: always with a point."""
    s = ("%.6f" % v).rstrip("0")
    return s + "0" if s.endswith(".") else s


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    want = {}
    for l in open(sys.argv[1]):
        l = l.split("#")[0].strip()
        m = re.match(r"^([a-z_][a-z0-9_]*)\s*=\s*(-?[0-9.eE+-]+)$", l)
        if m:
            want[m.group(1)] = float(m.group(2))
    lines = open(RS).read().split("\n")
    seen = set()
    for i, l in enumerate(lines):
        m = LINE.match(l)
        if m and m.group(2) in want:
            name, old = m.group(2), float(m.group(3))
            seen.add(name)
            new = want[name]
            if old != new:
                lines[i] = m.group(1) + number(new) + m.group(4) + l[m.end():]
                print("%s %s -> %s" % (name, number(old), number(new)))
    missing = sorted(set(want) - seen)
    if missing:
        sys.exit("no such weight in weights.rs: %s (nothing written)" % ", ".join(missing))
    if "--check" not in sys.argv:
        open(RS, "w").write("\n".join(lines))


if __name__ == "__main__":
    main()
