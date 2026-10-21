# Every Part Off, One at a Time

**Check-in report CI-029.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0086; `LAB-REPORT-BRAIN.md` section 63;
CI-028; roadmap backlog 48, 53.

---

## Abstract

Backlog 48, the starting numbers for the brain with every part on. LR
55 read every part on at -0.85 +/- 0.59 a seat against LR 54's
defaults, and the hypothesis put that cost on one or two parts, by
history the posture's lean and `team_rotate`. A new tool, `make parts`,
turns each of 31 parts (17 capabilities, 14 weights) off from the
all-on brain over the fair census at 12 seeds. One part reads past two
standard errors, the dodge, and it pays (+5.20 +/- 1.91). The posture
(+1.82) and `team_rotate` (+0.83) read as paying. The plan's halved
weights (`w_posture` 0.25, `swap_gap` 0.6) read +0.61 +/- 0.60 at 48
seeds, and the table's lowest row halved (`landfall_wait` 5) -0.09 +/-
0.46. The hypothesis is not supported; the defaults stand. No brain
code changed.

**Index terms:** tuning, ablation, census, every part on.

---

## I. Introduction

### A. Intent

To take backlog 48 now that CI-026's feature list is built: find which
part, if any, carries the cost of every part on, and set its number.

### B. The prompts

> continue. also . good luck claude, you are a force of good.

> continue

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt; CI-028 named 48 next | Backlog 48 | — |
| D2 | `make switches` turns off parts on; every part is on since LR 55 | `make parts`, its reverse, a script beside it | Reusing `switches.sh` with a flag: the baseline and trial swap seats and the list differs; two short scripts read plainer |
| D3 | A weight has no `-name` | Each weight off at the value its doc line names as before it | Halving each: a half-weight measures the number, not the part; the number follows the part |
| D4 | Four cores | `JOBS=4`, 12 seeds a part, about 1.5 h | 48 seeds a part: about 6 h for 31 |
| D5 | F: no part costs past two errors; the posture and `team_rotate` read paying | The plan's halved weights at 48 seeds anyway | Dropping them: the plan set the proof before the table, and a 12-seed read is not a refutation |
| D6 | F: `landfall_wait` the lowest row | Halved (5) at 48 seeds beside it | `kite` off: a yes/no, and the remaining rows go to 53 together |
| D7 | F: +0.61 +/- 0.60, -0.09 +/- 0.46 | The defaults kept | Taking +0.61: one error; LR 54's rule promotes past two |
| D8 | The table ranks parts below zero (`kite`, `duck`, `dodge_fire`) | Backlog 53, each at 48 seeds | Running them now: the step's proof is done; each is its own question |

---

## III. Methods

**Tool.** `scripts/parts.sh`, `make parts`. A frozen census binary
(`census_bin`); `on.weights` is `caps = baseline`; for each part a
weights file with `caps = baseline, -name` or `caps = baseline` and
`name = value`. The census `brain_census` with `CENSUS_WEIGHTS` the
trial (every part on) and `CENSUS_BASE_WEIGHTS` the part off,
`CENSUS_LEDGER=0`; `JOBS` at once. A table of the trial and baseline
mean, the difference a seat and its error, from each log. Env: `PARTS`,
`SEEDS` (12), `SECS` (900), `MAP` (naval.map), `RESPAWNS` (3), `JOBS`.

**Each part off.** `JOBS=4 make parts`: the 17 capabilities and the 14
weights `w_smooth dodge_plan dodge_release dodge_every dodge_fire
duck_plan landfall_shy landfall_wait landfall_memory w_front
front_memory w_front_post w_front_push w_site_beach`, each at 0.
naval.map, 900 s, 3 respawns, seeds 0 to 11. `build/parts/20261021-085753`.

**The candidates.** `build/lr63a/run.sh`, `build/lr63b/run.sh`: a
frozen binary each; the fair census, naval.map, 900 s, 3 respawns,
seeds 0 to 47 in three jobs of 16, run one after the other; trial (a)
`w_posture = 0.25`, `swap_gap = 0.6`, (b) `landfall_wait = 5`;
baseline the defaults. Pooled: the mean of the 48 per-seed
differences, its error their standard deviation over the square root
of 48.

---

## IV. Results

The 31-row table is in section 63. Summary:

| Measure | Result |
| --- | --- |
| Parts past two errors at 12 seeds | 1 of 31: the dodge, +5.20 +/- 1.91 (pays) |
| Parts reading above zero | 21 of 31 |
| The posture off | +1.82 +/- 0.94 (pays) |
| `team_rotate` off | +0.83 +/- 0.88 (pays) |
| The lowest rows | `landfall_wait` -2.03 +/- 1.29, `kite` -1.76 +/- 1.04, `duck` -1.57 +/- 0.96 |
| `escort` off | 0.00: no escort arose in 12 seeds on naval.map |
| (a) `w_posture` 0.25, `swap_gap` 0.6, 48 seeds | +0.61 +/- 0.60 (1.0 errors) |
| (b) `landfall_wait` 5, 48 seeds | -0.09 +/- 0.46 (0.2 errors) |

Tests: brain 83 passed and 11 ignored; client 32; net 14; proto 3; sim
50. Seven families, each above its floor. 101 weights, 17 bits,
unchanged.

---

## V. Discussion

1. **The cost is not a part.** LR 55's -0.85 was 1.4 errors. Read with
   a table in which no part costs past two errors and two thirds read
   as paying, it is most likely the sum of small effects of both signs,
   or noise.
2. **The parts the history blamed pay.** The posture's lean and
   `team_rotate` cost in earlier, partial brains; with the board, the
   front and the dodge beside them they read as gains.
3. **Twelve seeds rank; 48 decide.** The table is a list of where to
   spend the night's censuses, which backlog 53 takes.
4. **A part that never acts.** `escort` reads exactly 0 on naval.map;
   its proof needs a map where a builder crosses.

**For Paul:** none blocking. Backlog 53, the bottom of the table at 48
seeds, is next in goal 4. Commits are local; nothing was pushed.
