# Each Default Off Again, on the New Defaults

**Check-in report CI-046.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 81; CI-044 and
sections 76 and 77; roadmap backlog 63; `scripts/switches.sh`.

---

## Abstract

LR 77 changed the brain's defaults to turtle with team_rotate off. LR
76's census of each part switched off was read against the old
defaults, so it was run again at the new ones: twelve rows, 200 fresh
seeds, every map. No row is past two errors toward a change, so by the
rule set first no default changes. The dodge (-4.76 +/- 0.28 off), the
posture (-0.51 +/- 0.22) and priced_crossing (-0.47 +/- 0.15) pay.
team_rotate on reads -0.24 +/- 0.23, costing atlantic -4.13. The rest
sit inside the noise. While it ran, the twelve single scenarios were
read over 200 seeds each on the Mac: ten held on every seed, and the
two cut from the ready-gun family held 80%, that family's own rate.

**Index terms:** brain census, ablation, defaults, scenarios.

---

## I. Introduction

### A. Intent

To keep testing the new defaults: the parts they rest on, and the
scenarios beyond the one seed each is pinned to.

### B. The prompt

> keep testing.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | LR 76 read each part against the old defaults | LR 76's twelve rows again at 7c18851, seeds 100000 to 100199 | A new round robin: the styles were just ordered; the parts were not read on the new brain |
| D2 | team_rotate is now off | The row adds it instead of removing it | Drop the row: its third reading costs nothing extra |
| D3 | The machine was two commits behind | Pulled to 7c18851; the backlog 62 fix (a scenario only) not needed there | Wait for a commit: the census does not read scenarios |
| D4 | The rule before the run | `build/switches/NOTES-lr81.txt`, LR 76's rule | — |
| D5 | The Mac idle; the plain tests read each scenario on one seed | Each scenario over 200 seeds from 5000, read only, nothing changed | Make the two at 80% families: they are the ready-gun family already |
| D6 | F: no row past two errors toward a change | No default changes; backlog 63 done | — |

---

## III. Methods

On the machine: `SWITCHES="-forget -board team_rotate -priced_crossing
-column -cycle -island_site -escort -bridge_jobs -duck -posture -dodge"
SEEDS=200 SEED0=100000 SPLIT=2 JOBS=24 MAP=all make switches`, 900 s, 3
respawns, a frozen binary; each row's map split by `scripts/pool.sh
DIR/ROW`. On the Mac: `make scenario S=FILE RUNS=200 SEED0=5000` for
each file under `scenarios/`.

---

## IV. Results

In LR 81: the table and the map splits. Scenarios over 200 seeds: ten
at 200 of 200; a-dodging-tank-holds-its-fire and
a-tank-ducks-out-of-reach-in-time at 160 of 200 (80.0% +/- 2.8%),
against the ready-gun family's 83.4% and its 80% floor.

---

## V. Discussion

The new defaults rest on the same parts as the old: the dodge first by
far, then the posture and the priced crossing. The posture and the
board pay a little less on a turtle than on the old brain, though not
past two errors of the difference. Turtle's weights already hold the
base, which is the posture's own lean. Atlantic is the map where the
board, the posture and team_rotate are each worth the most, the same
as before.
