# The Bottom of the Parts Table at 48 Seeds

**Check-in report CI-030.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0087; `LAB-REPORT-BRAIN.md` section 64;
CI-029; roadmap backlog 53, 54.

---

## Abstract

Backlog 53. LR 63's 12-seed table read three parts below zero, none
past two errors: `kite`, `duck` and `dodge_fire`. Each was turned off
from the all-on brain over the fair census at 48 seeds with `make
parts`. The kite off reads -0.94 +/- 0.77, the duck off +0.12 +/- 0.60,
the held shot off -0.19 +/- 0.58. None is past two errors, and the
defaults stand. The kite keeps a lean at 1.2 errors, but the meter
counts no kiting, so the next step counts it (backlog 54). No code
changed.

**Index terms:** tuning, ablation, census, kite, duck, dodge.

---

## I. Introduction

### A. Intent

To settle the three lowest rows of LR 63's table at the seed count
that can decide them.

### B. The prompt

> yes. kite , duck, dodge_fire . please try.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt | `kite`, `duck`, `dodge_fire` off at 48 seeds | `landfall_wait`: halved, LR 63 read it at -0.09 +/- 0.46 |
| D2 | Four cores, three parts | `make parts` with `SEEDS=48 JOBS=3`, one job a part | Three jobs of 16 a part as LR 63a: nine processes on four cores, no faster |
| D3 | F: none past two errors | The defaults kept; no family | Turning the kite off at 1.2 errors: LR 54's rule promotes past two |
| D4 | F: the meter counts no kiting | Backlog 54, the kite counted, before any tuning of its numbers | Tuning `kite_margin` now: a number for a part not known to act |

---

## III. Methods

`PARTS="-kite -duck dodge_fire=0" SEEDS=48 JOBS=3 make parts`
(`scripts/parts.sh`, CI-029): a frozen census binary; the trial seat
on the defaults (every part on), the baseline with the part off;
naval.map, 900 s, 3 respawns, seeds 0 to 47; `build/parts/20261021-123214`.
The difference a seat and its error are the census's own, over the 48
per-seed differences.

---

## IV. Results

| Part off | 48 seeds | Seeds above zero | 12 seeds (LR 63) |
| --- | --- | --- | --- |
| `-kite` | -0.94 +/- 0.77 | 22 of 48 | -1.76 +/- 1.04 |
| `-duck` | +0.12 +/- 0.60 | 22 of 48 | -1.57 +/- 0.96 |
| `dodge_fire=0` | -0.19 +/- 0.58 | 26 of 48 | -1.42 +/- 1.22 |

Tests and families as CI-029 (no code changed).

---

## V. Discussion

1. **Regression to the mean.** The three lowest of 31 rows at 12 seeds
   were picked for being low; at 48 two of them read near zero.
2. **The kite's lean** is the one left, and a count of it must come
   before a number for it.

**For Paul:** none blocking. Backlog 54, the kite counted, is next.
Commits are local; nothing was pushed.
