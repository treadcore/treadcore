#!/usr/bin/env python3
"""Write the brain's map: every file of treadcore-brain, what it says it
is, and every function in it with the first sentence of its doc comment.

    python3 -I scripts/brainmap.py documentation/BRAIN-MAP.md

The index is the code's own words, read from its `//!` and `///`
comments, so it is as true as they are and is written again, never
edited by hand (`make brainmap`). Tests are listed apart from the
functions they prove. A function without a doc comment is listed with
none, which is itself worth seeing.
"""
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
SRC = os.path.join(ROOT, "source", "treadcore-brain", "src")
FN = re.compile(r"^(\s*)(pub(?:\([a-z]+\))?\s+)?(?:const\s+)?fn\s+([a-z_0-9]+)")
IMPL = re.compile(r"^\s*impl(?:<[^>]*>)?\s+(?:([A-Za-z_:<>]+)\s+for\s+)?([A-Z][A-Za-z0-9_]*)")


def first_sentence(lines):
    text = " ".join(l.strip() for l in lines).strip()
    # The first full stop after a word, a number, a code span or a
    # bracket: "e.g." and "1.5" do not end a sentence.
    m = re.match(r"(.+?[a-z0-9`)'\"]\.)(\s|$)", text)
    s = m.group(1) if m else text
    return s if len(s) <= 220 else s[:217].rstrip() + "..."


def scan(path):
    head, fns, tests = [], [], []
    doc, owner, test_next, in_tests = [], None, False, False
    with open(path, encoding="utf-8") as f:
        lines = f.read().split("\n")
    for i, line in enumerate(lines):
        s = line.strip()
        if s.startswith("//!") and not fns and not doc:
            head.append(s[3:].strip())
            continue
        if s.startswith("#[cfg(test)]"):
            in_tests = True
        if s.startswith("///"):
            doc.append(s[3:])
            continue
        if s.startswith("#[test]"):
            test_next = True
            continue
        if s.startswith("#["):
            continue
        m = IMPL.match(line)
        if m and not line.startswith(" " * 8):
            owner = m.group(2)
        m = FN.match(line)
        if m:
            name = m.group(3)
            indent = len(m.group(1))
            where = owner if indent >= 4 and owner and not in_tests else None
            entry = (i + 1, name, where, bool(m.group(2)), first_sentence(doc) if doc else "")
            (tests if test_next else fns).append(entry)
            test_next = False
        if s and not s.startswith("//"):
            doc = []
    return head, fns, tests


def main():
    out = sys.argv[1] if len(sys.argv) > 1 else os.path.join(ROOT, "documentation", "BRAIN-MAP.md")
    files = sorted(f for f in os.listdir(SRC) if f.endswith(".rs"))
    rows, body = [], []
    tf = tt = tu = 0
    for name in files:
        path = os.path.join(SRC, name)
        head, fns, tests = scan(path)
        n = sum(1 for _ in open(path, encoding="utf-8"))
        undoc = sum(1 for e in fns if not e[4])
        tf, tt, tu = tf + len(fns), tt + len(tests), tu + undoc
        rows.append(f"| [`{name}`](#{name.replace('.', '')}) | {n} | {len(fns)} | {undoc} | {len(tests)} | {first_sentence(head) if head else ''} |")
        body.append(f"\n## {name}\n")
        if head:
            body.append(" ".join(head).strip() + "\n")
        body.append("| Line | Function | Of | Pub | What it does (its doc comment) |")
        body.append("| --- | --- | --- | --- | --- |")
        for ln, fn, where, pub, d in fns:
            body.append(f"| {ln} | `{fn}` | {where or ''} | {'yes' if pub else ''} | {d.replace('|', '/')} |")
        if tests:
            body.append(f"\nTests ({len(tests)}):\n")
            for ln, fn, _, _, d in tests:
                body.append(f"- `{fn}` (line {ln}){': ' + d.replace('|', '/') if d else ''}")
    with open(out, "w", encoding="utf-8") as f:
        f.write("# The brain's map\n\n")
        f.write("Every file of `treadcore-brain` and every function in it, with the\n")
        f.write("first sentence of its own doc comment. Written by\n")
        f.write("`scripts/brainmap.py` (`make brainmap`); do not edit by hand. The\n")
        f.write("design and its review are `LAB-REPORT-BRAIN.md`; the review of the\n")
        f.write("structure as built is `checkins/`.\n\n")
        f.write(f"{len(files)} files, {tf} functions ({tu} with no doc comment), {tt} tests.\n\n")
        f.write("| File | Lines | Functions | Undocumented | Tests | What it is |\n| --- | --- | --- | --- | --- | --- |\n")
        f.write("\n".join(rows) + "\n")
        f.write("\n".join(body) + "\n")
    print(f"{out}: {len(files)} files, {tf} functions, {tu} undocumented, {tt} tests")


if __name__ == "__main__":
    main()
