# The Run for the Beach

**Check-in report CI-008.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0056; `LAB-REPORT-BRAIN.md` section 31;
CI-004, CI-007.

---

## Abstract

CI-004 found 20% of the trial team's deaths on `landfall`, the boat's
run for a beach, most of them sunk by direct fire. This step added a
weight, `landfall_shy` (default 0, Paul's choice), that prices the
tiles of the run inside a seen hostile gun's reach. At 1 over 48 seeds
it reads +0.65 ± 0.69 a seat, and the landfall deaths did not change
(169 against 165 on the same 16 seeds). The hypothesis is not
confirmed. Sight and direct range are both 10 tiles, so a boat sees
the gun that sinks it only once it is in its reach. Backlog 32 prices
the guns the team has seen instead.

**Index terms:** partial observability, utility scoring, boats,
weights.

---

## I. Introduction

### A. Intent

Backlog 30, as proposed at CI-007's close: a weight that turns a boat
off a gun's range.

### B. The prompts

> yes 0. commit. push. continue backlog

The answer to "I'd start it at 0 ... Is that the default you want?"

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Paul: "yes 0" | `landfall_shy` 0; play unchanged | |
| D2 | The either/or rule (memory, 18 October) | A weight blending the old price and the exposure | A switch: a scalar lets the tuner find the value |
| D3 | Reading the code: `danger` and `toward` already avoid a seen gun | Price what they miss, the run's exposure, and test it | Abandoning the step before measuring: approved, and cheap to measure |
| D4 | The weight placed among the dodge's | Moved after `dodge_hold` | |
| D5 | F: +0.65 ± 0.69; landfall deaths unchanged | Not confirmed; the weight stays 0; backlog 30 closed | Tuning `landfall_shy`: the deaths did not move at 1 |
| D6 | The duck's note: sight = direct range = 10 | Backlog 32: the team's remembered guns, read first | Building it now: one step at a time, and the read may show no gun was seen |

---

## III. Methods

### A. The part

`RuleBot::landfall_order` (lib.rs) scores each reachable shore tile `b`
as `dist(me, b) + danger(b) + toward(b) + 4 × shy × exposed(b)`.
`exposed(b)` cuts the line from the boat to `b` into ⌈dist⌉ steps and
counts, in tiles, the steps within `DIRECT_RANGE + 4` of a seen hostile
tank or `PILLBOX_RANGE + 2` of a hostile pillbox. It returns 0 at once
when `shy` is 0, so default play is unchanged by construction.
`Commander::set_settings` copies `W::LandfallShy` into the inner
RuleBot.

### B. The test

`a_shy_boat_runs_for_the_beach_clear_of_the_guns`: a beach 30 tiles
away whose run passes a gun 12 tiles off the line, and one 40 away with
a gun 15 beyond it. Scores at 0: 66 and 82, the near beach chosen. At
1, 15 exposed tiles add 60: 126 and 82, the far beach chosen.

### C. The census

The defaults dumped with `landfall_shy = 1`, 48 seeds in three ranges,
naval.map, 900 s, respawns 3, the fair census, `CENSUS_ACTS` on.

---

## IV. Results

| | d a seat | Deaths (seeds 0–15) | On landfall | Direct, sunk, landfall |
| --- | --- | --- | --- | --- |
| Null (CI-004) | 0 | 819 | 165 | 132 |
| `landfall_shy` 1 | +0.65 ± 0.69 | 821 | 169 | 133 |

All 48 seeds: 2,522 deaths, 548 on landfall (21.7%). Tests: brain 71
passed and 8 ignored; client 31; net 14; proto 3; sim 50; no warnings.
354 functions, 121 undocumented, 79 tests.

---

## V. Discussion

1. **What the code already does should come before the hypothesis.**
   The old score already steered boats off every gun they could see.
   The step's own first reading showed it, and the census confirmed
   it.
2. **The limit is the information, not the decision.** A gun is seen
   at its own range. Better choices need better knowledge: what a
   teammate saw, kept for a while. That is the team board's job (CI-001
   A1), and the first use for it that a death count supports.

**Next:** backlog 32's read, then its part.
