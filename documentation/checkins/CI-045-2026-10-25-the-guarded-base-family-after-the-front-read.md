# The Guarded-Base Family after the Front Read

**Check-in report CI-045.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 80; CI-044 and
sections 75 and 76; roadmap backlog 62;
`scenarios/families/a-raid-on-a-guarded-base.toml`.

---

## Abstract

CI-044 found the a-raid-on-a-guarded-base family under its floor
(70.2% against 75%) and bisected the drop to 5899e8e, which set the
front read's three weights to 0 by default. That commit gave the
front scenario its old `w_front` back so the scenario still tested
the front read. It missed this family, whose header names
`w_front_push` as its mechanism. By a rule written before the run, the
family now sets `w_front_push` to its old 0.75 and reads 81.5% +/-
1.9% (326 of 400), close to the 81.8% it read before 5899e8e. Every
family holds and the brain's 87 tests pass. The defaults are
unchanged.

**Index terms:** scenario families, regression, test fixtures,
defaults.

---

## I. Introduction

### A. Intent

To commit and push LR 77 once the turtle census had landed, then fix
backlog 62.

### B. The prompt

> commit and push when turtle lands. then fix backlog 62

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "when turtle lands": turtle's census had finished (`pooled.txt`, 08:18) and was written up in LR 77 | LR 77 committed and pushed on main (`7c18851`) | Wait for another run: none was playing |
| D2 | The family's header: the push point does it, 85.5% against 50.5% with `w_front_push` at 0 | Set `w_front_push` alone in the family's brain | All three front weights at their old values: the family tests one of them |
| D3 | 5899e8e's precedent: the front scenario sets `w_front = 0.5` | The weight in the family's toml, with a comment why | Lower the floor: the family then tests nothing it was written for. Turn the push back on by default: LR 75's census found it costs in play |
| D4 | The rule before the run | Written to `build/b62/NOTES.txt`: every family at or above its floor; if not, stop, no second weight | — |
| D5 | F: 81.5%, every family holds | Backlog 62 done | — |

---

## III. Methods

`scenarios/families/a-raid-on-a-guarded-base.toml`: `weights = {
w_front_push = 0.75 }` under its `[[brain]]`, and a comment in the
header. `make families` (400 seeds for this family, its own seeds) and
`cargo test -p treadcore-brain --release`, on the Mac at 7c18851.

---

## IV. Results

In LR 80: the family at 326 of 400, 81.5% +/- 1.9%, floor 75%
(281 of 400, 70.2%, without the weight); every other family as
before; 87 tests passed.

---

## V. Discussion

A scenario that tests a mechanism has to keep the mechanism on when a
default turns it off. 5899e8e did that for the scenario it knew of and
missed the family. The rule CI-044 added to CLAUDE.md (`make families`
at each default change) would have caught it on that commit. The fix
is in the test, not the brain: the push point is still off for play.
