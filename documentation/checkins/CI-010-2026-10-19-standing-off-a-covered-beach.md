# Standing Off a Covered Beach

**Check-in report CI-010.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0058; `LAB-REPORT-BRAIN.md` section 33;
CI-009.

---

## Abstract

CI-009 found that boats on landfall die to guns they have seen. This
step added a weight, `landfall_wait` (default 0, Paul's choice): a boat
whose chosen beach a seen hostile tank covers waits out of that gun's
reach before landing. At 10 s over 48 seeds it reads -0.51 ± 0.63 a
seat. The landfall deaths did not fall, and the stand-off acted in
under 0.07% of the trial team's time. The hypothesis is not confirmed:
the guns that sink boats are rarely at the beach. Backlog 34 reads
where they are.

**Index terms:** boats, utility AI, activation rate, weights.

---

## I. Introduction

### A. Intent

Backlog 33, as proposed at CI-009's close.

### B. The prompts

> yes 0. commit. push. continue backlog

> commit. push. continue backlog

The first set `landfall_wait`'s default. The second came while the
census ran; the code was committed with its measurement minutes later,
as one step.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Paul: "yes 0" | `landfall_wait` 0; play unchanged | |
| D2 | A gun flickering in and out of sight | One wait a crossing, from the first stand-off, not restarted | A wait from each sighting: a boat could be held off for ever |
| D3 | The census must see the part act | A status code, `stand_off` | Inferring it from `landfall` time |
| D4 | The second prompt arrived mid-census | Commit code and measurement together, three minutes later | Committing unmeasured code |
| D5 | F: -0.51 ± 0.63; stand-off < 0.07% of time | Not confirmed; stays 0; backlog 34 to read where the gun stands | Tuning the wait: the part barely acts |
| D6 | `make brainmap`: `standing_off` took `landfall_order`'s doc | Moved above the doc block | |

---

## III. Methods

### A. The part

In `RuleBot::landfall_order` (lib.rs), after the beach `best` is
chosen and when `landfall_wait` > 0: the nearest seen hostile tank
within `DIRECT_RANGE + 4` of `best` is the cover. If the crossing's
stand-off (`standoff`, set on first use) is younger than the wait, the
boat moves to `gun + (me − gun).norm() × (DIRECT_RANGE + 6)`, clamped
to the map, or holds if already that far. A move is not re-sent while
the current goal is within 2 tiles of the target. `standing` marks the
think; `Commander::think_on` reads it (`standing_off`) for `stand_off`.
`standoff` resets ashore, at a respawn; `standing` also under a hop.

### B. The test

`a_boat_stands_off_a_covered_beach_for_its_wait`: one beach 10 tiles
off, a gun 8 tiles off beside it. At 0: a move to the beach. At 10: a
move to 16 tiles from the gun, `standing_off` true; 11 s later, a move
to the beach.

### C. The census

`landfall_wait = 10` as the trial weights, 48 seeds in three ranges,
naval.map, 900 s, respawns 3, the fair census, `CENSUS_ACTS`.

---

## IV. Results

| | d a seat | Deaths, seeds 0–15 | On landfall | Stand-off time |
| --- | --- | --- | --- | --- |
| Null (CI-004) | 0 | 819 | 165 | |
| `landfall_wait` 10 | -0.51 ± 0.63 | 798 | 184 | < 0.07% |

By range: -0.85 ± 1.23, -0.53 ± 1.05, -0.15 ± 1.03. Over 48 seeds,
573 of 2,460 deaths on landfall (23.3%). Tests: brain 72 passed and 8
ignored; client 31; net 14; proto 3; sim 50; no warnings. 355
functions, 121 undocumented, 80 tests.

---

## V. Discussion

1. **Read how often a part acts before its score.** A part that fires
   0.07% of the time cannot move a score, and its null result says
   nothing about the idea. The status codes made the activation rate a
   one-line read.
2. **Three parts, one wrong premise.** `landfall_shy`, the remembered
   guns, and the stand-off each assumed where the gun stands. Backlog
   34 measures that before a fourth part.

**Next:** backlog 34.
