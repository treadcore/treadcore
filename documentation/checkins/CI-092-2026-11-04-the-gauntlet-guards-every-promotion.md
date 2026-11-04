# The Gauntlet's Floors Guard Every Promotion

**Check-in report CI-092.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026, late.
Companion records: `LAB-REPORT-BRAIN.md` section 143; CHANGELOG; ROADMAP
backlog 85; journal 0119; orrery `guard_jobs`.

---

## Abstract

A candidate promotes only if the rules with its dials stand on every
gauntlet floor, and every floor is today's success, so a generation can
only go forward.

## I. Intent

Paul, word for word: "adjust the floors to be what we have now as
sucess, so we can create generations from here forwards", and, asked
whether the floors should guard every promotion: "Yes, a guard on
promotions".

## II. Decision log

| Decision | Cause |
| --- | --- |
| The rules alone, with the candidate's dials | a promotion changes the rules' defaults; the nets are not promoted |
| Read before auto promotion, and again in the promotion job | the first spares a promotion that would fail; the second reads the batch, riders and all |
| No margin at the guard | each floor is already the rules' share less two errors |
| G3 0.65, G8 0.30 | the rules stood under 0.95 and 0.9; a guard no brain passes stops the line |

## III. Method

`learn.rs`: `GAUNTLET_BRAINS=scripted`, `with_dials`. The rules' shares
of section 142 (200 seeds) against every floor; the promotion job's
check run on that read (holds) and on the read with `w_shooter = 0`
(G20 51%, under 0.60: held).

## IV. Results

Today's rules hold all twenty-two floors. A candidate that undid
section 142 would be held at G20.

## V. Discussion

The floors were the nets' targets (section 102) and the boundary's
(section 141); they are now the line's ratchet, each raised as the rules
earn it.

## VI. What is not built

The nets' and styles' own floors; the floors raised by themselves.

## VII. Paul's decisions

None outstanding for this step.
