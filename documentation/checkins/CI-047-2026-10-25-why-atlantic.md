# Why Atlantic: Where the Switches Move the Time

**Check-in report CI-047.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 82; CI-046 and
section 81; roadmap backlog 64; `scripts/rungs.py`.

---

## Abstract

In LR 81, atlantic is the map where the board, the posture and
team_rotate each count most. LR 81's logs already carry each match's
score parts and the trial team's seconds on each rung, so they were
read rather than a new run started. There are two mechanisms, not
three: on atlantic the board's whole worth is the posture's, which
needs it. An Expand second does not buy more ground on atlantic than
elsewhere (+0.69 held a 100 s, mid-table). team_rotate takes 48% of
atlantic's trial time off the ladder (elsewhere 3% to 22%) and costs
2.22 held. Without the posture, atlantic's trial team builds 20 more
things and 4.6 more bridges and holds 1.35 less. No default changes.

**Index terms:** brain census, meter, maps, ablation analysis.

---

## I. Introduction

### A. Intent

The first of the three next steps offered after LR 81: why atlantic
carries the board, the posture and team_rotate.

### B. The prompt

> 1

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | LR 81's logs print each match's rung seconds and score parts | Read them; no new run | A metered census on atlantic: 2 h for lines already printed |
| D2 | A reading, not a change | What it looks for set before reading; no default changes on it | — |
| D3 | `-forget` reads +0.04 +/- 0.06 off | It stands for the defaults | A defaults-against-defaults census: an hour for a row already near zero |
| D4 | F: the posture and team_rotate cut Expand seconds | Test whether atlantic pays an Expand second more: the slope a map | Stop at "less Expand": the slope says no, atlantic is mid-table |
| D5 | F: team_rotate's cut larger on atlantic | Its share of ladder time a map: atlantic -48%, the next -22% | — |
| D6 | Repeat the reading | `scripts/rungs.py`, checked to reproduce the figures | Leave it in a scratch file |

---

## III. Methods

`scripts/rungs.py build/switches/20261025-131009 -forget -board
-posture team_rotate`: for each match line block, the trial team's
`seat-seconds a rung` and its held ground, built and bridges less the
mean of the other two teams in the same match, averaged a row and a
map; then over every row pooled, the least-squares slope of held
ground on Expand seconds a map with its standard error. The ladder
share is the sum of the four rungs' seconds, `-forget` against
`team_rotate`.

---

## IV. Results

In LR 82: the atlantic table, the slopes a map, the ladder shares a
map.

---

## V. Discussion

A switch that counts on one map leaves a trace in what the trial team
spends its time on there. The census's own lines carried the answer
without a run. The meter cannot say when in the match the time goes,
or what a bridge costs in builders' time. That is the next line to
add if atlantic's bridges are to be priced.
