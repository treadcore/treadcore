#!/usr/bin/env python3
"""Set the brain's defaults from a weights file (LAB-REPORT-BRAIN.md 115).

    python3 -I scripts/set-defaults.py FILE.weights [--check]

Each `name = value` line of FILE sets that weight's default in
source/treadcore-brain/src/weights.rs, the number on its line in the
`weights!` table, and the same number in `Tactics::default()`
(tactics.rs) where a field is named for the weight:

    PressOdds "press_odds" 2.0, "odds: above these, close in";

A `caps = baseline, name, -name` line sets those capabilities on or off
in the `caps!` table (since 29 October: the fleet tries each switch
against the defaults, A/B, and one that passes is promoted as a number
is); a caps line without `baseline` is a set, not a change, and is an
error. A choice (a weight whose doc says `(choice: ...)`) is a number
like any other. A name the tables do not have is an error, and nothing is
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
# The same numbers written as constants: `Tactics::default()`, which the
# brain's test `default_weights_are_todays_constants` holds the table to.
TACTICS = os.path.join(TC, "source", "treadcore-brain", "src", "tactics.rs")
LINE = re.compile(r'^(\s*[A-Z][A-Za-z0-9]*\s+"([a-z_0-9]+)"\s+)(-?[0-9][0-9.eE+-]*)(\s*,)')
CAP = re.compile(r'^(\s*[A-Z][A-Z0-9_]*\s+[0-9]+\s+"([a-z_0-9]+)"\s+)(true|false)(\s*,)')


def number(v):
    """A Rust f64 literal: always with a point."""
    s = ("%.6f" % v).rstrip("0")
    return s + "0" if s.endswith(".") else s


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    want, caps = {}, {}
    for l in open(sys.argv[1]):
        l = l.split("#")[0].strip()
        m = re.match(r"^([a-z_][a-z0-9_]*)\s*=\s*(-?[0-9.eE+-]+)$", l)
        if m:
            want[m.group(1)] = float(m.group(2))
        m = re.match(r"^caps\s*=\s*(.*)$", l)
        if m:
            names = [c.strip() for c in m.group(1).split(",") if c.strip()]
            if "baseline" not in names:
                sys.exit("caps = %s: a promotion changes the baseline, so the line must begin from it (nothing written)" % m.group(1))
            for c in names:
                if c != "baseline":
                    caps[c.lstrip("-")] = not c.startswith("-")
    lines = open(RS).read().split("\n")
    seen = set()
    for i, l in enumerate(lines):
        m = CAP.match(l)
        if m and m.group(2) in caps:
            name, old = m.group(2), m.group(3) == "true"
            seen.add("caps:" + name)
            if old != caps[name]:
                lines[i] = m.group(1) + ("true" if caps[name] else "false") + m.group(4) + l[m.end():]
                print("caps %s %s -> %s" % (name, "on" if old else "off", "on" if caps[name] else "off"))
            continue
        m = LINE.match(l)
        if m and m.group(2) in want:
            name, old = m.group(2), float(m.group(3))
            seen.add(name)
            new = want[name]
            if old != new:
                lines[i] = m.group(1) + number(new) + m.group(4) + l[m.end():]
                print("%s %s -> %s" % (name, number(old), number(new)))
    missing = sorted((set(want) | {"caps:" + c for c in caps}) - seen)
    if missing:
        sys.exit("no such weight in weights.rs: %s (nothing written)" % ", ".join(missing))
    # Tactics::default(): a field named for a changed weight gets its number
    # (an integer field stays an integer: from_weights rounds it).
    tl = open(TACTICS).read().split("\n")
    start = next((i for i, l in enumerate(tl) if l.startswith("impl Default for Tactics")), None)
    if start is not None:
        for i in range(start, len(tl)):
            if tl[i].startswith("}"):
                break
            m = re.match(r"^(\s+)([a-z_][a-z0-9_]*): (.+),$", tl[i])
            if m and m.group(2) in want and m.group(3) not in ("true", "false"):
                v = want[m.group(2)]
                lit = str(int(round(v))) if re.fullmatch(r"[0-9]+", m.group(3)) else number(v)
                if lit != m.group(3):
                    tl[i] = "%s%s: %s," % (m.group(1), m.group(2), lit)
                    print("  Tactics::default() %s: %s -> %s" % (m.group(2), m.group(3), lit))
    if "--check" not in sys.argv:
        open(RS, "w").write("\n".join(lines))
        open(TACTICS, "w").write("\n".join(tl))


if __name__ == "__main__":
    main()
