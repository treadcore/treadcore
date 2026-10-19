# Which Landfall Weight Carries the Gain

**Check-in report CI-014.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journals 0062 and 0063; `LAB-REPORT-BRAIN.md`
section 37; CI-013.

---

## Abstract

CI-013 found +2.31 ± 0.58 a seat with three landfall weights on
together: `landfall_memory` 30, `landfall_shy` 1, `landfall_wait` 10.
This step took each back to 0 in turn over 48 seeds. Without the
memory, -1.08 ± 0.53. Without the stand-off, +0.69 ± 0.64. Without the
run's price, +1.50 ± 0.67. The memory is the base: the other two cost
a point without it and add about 2.4 a seat between them with it. All
three together is the best setting measured. The defaults are Paul's.

**Index terms:** ablation, complementary features, memory, weights.

---

## I. Introduction

### A. Intent

Backlog 37: which of the three carries the gain, before Paul chooses
defaults.

### B. The prompts

> commit. push. continue backlog

Given twice while the runs went on. The first finished run (memory
off) was committed alone (journal 0062); this report covers all three.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-013 D6 | One weight back at 0 in each of three runs, 48 seeds, the same seeds | Each alone from the null: shy and wait alone are LR 31 and 33 already |
| D2 | A prompt mid-ablation | Commit the finished run alone; this report for the whole | Waiting 20 minutes |
| D3 | F: memory off -1.08; the others positive | The memory is the base | |
| D4 | F: all three best; the values hand-picked | Recommend all three on at these values; the tuner later | Choosing the defaults: Paul's |

---

## III. Methods

From CI-013's weights file (memory 30, shy 1, wait 10), three files
each with one of the three set to 0. Each run: `CENSUS_WEIGHTS`, 48
seeds as three ranges of 16 side by side, naval.map, 900 s, respawns 3,
the fair census, `CENSUS_ACTS`. The runs went one after another. Each
reads against the null (the defaults for every team) seed by seed.

---

## IV. Results

| Trial | d a seat | Landfall deaths, seeds 0–15 | 48 seeds | Seen near there earlier, 0–15 |
| --- | --- | --- | --- | --- |
| Null | 0 | 165 | | 76 |
| All three | **+2.31 ± 0.58** | 82 | 281 | 16 |
| Memory off | -1.08 ± 0.53 | 164 | 542 | 72 |
| Shy off | +1.50 ± 0.67 | 103 | 341 | 16 |
| Wait off | +0.69 ± 0.64 | 113 | 369 | 25 |

---

## V. Discussion

1. **Complementary parts.** The memory knows where a gun is; the run's
   price keeps the boat's course off it; the stand-off keeps the boat
   off a beach it covers. Each needs the memory, and each adds to it.
2. **Differences between trials are rough.** Each run is paired with
   the null, not with the others, so "the stand-off adds 1.6" carries
   the errors of two runs (about 0.9).
3. **The values are hand-picked.** 30 s, 1 and 10 s were the first
   tried. The tuner (backlog 23) can search them once they are on.

**Next:** Paul's defaults for the three.
