# Every Default a Number: The Wingman's Five, and the Candidate They Held Back

**Check-in report CI-098.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 5 November 2026, midday.
Companion records: `LAB-REPORT-BRAIN.md` section 149; journal 0126.

---

## Abstract

The day's strongest candidate, grove/w10839 (held out +0.96), could not
be promoted: one of its moves, `wing_leash`, had a default written in the
weights table as the name of a constant, which the promotion's writer
read as no default at all. The five wingman weights' defaults are now
numbers in the table, a test holds every default to that, and the
writer names the cause if it ever meets another. With the change,
w10839's moves pass all 168 brain tests.

## I. Intent

Paul: "fix both. commit push.. and restart server, attempting to allow
work to resume," after a look at the queue found w10839's promotion
stopped at 11:09 with "no such weight in weights.rs: wing_leash".

## II. Decision log

| Decision | Cause |
| --- | --- |
| The five defaults written as numbers, the constants removed | set-defaults.py, orrery's planner and the ledger read a weight's default as the number on its line |
| Not: teaching set-defaults.py to edit the constants | orrery caches the defaults by weights.rs's blob; a promotion that moved only commander.rs would read to it as unchanged defaults |
| The leash test reads the pinned brain's leash | it read the constant, which a promotion of `wing_leash` would have moved (LR 147's rule) |
| A test that every default in the table is a number | the fault was silent for three days and cost a candidate |
| set-defaults.py says when a default is not a number | "no such weight" sent the reader looking for a weight that was there |

## III. Method

`cargo test -p treadcore-brain --release --lib`; the new test given a
default written as an expression, then restored; `set-defaults.py
--check` on w10839's trial file; a worktree of main with the change,
w10839's numbers set by `set-defaults.py`, the tests run.

## IV. Results

All 168 brain tests pass (167 and the new one). The new test fails on a
default written as an expression, naming its line. `set-defaults.py`
writes w10839's four moves, `wing_leash 20.0 -> 10.0` among them; with
them set, the 168 tests pass.

## V. Discussion

A weight that cannot be written is a weight the fleet can search but
never keep: the fit proposed `wing_leash` 10, the reads held it, and the
last step had no hand to take it. The test makes the table's form a rule
rather than a habit.

## VI. What is not built

Nothing of this step. Whether w10839 holds its families and gauntlet is
the promotion job's to read.

## VII. Paul's decisions

None outstanding from this step.
