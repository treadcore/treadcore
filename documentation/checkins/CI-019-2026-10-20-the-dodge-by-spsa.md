# The Dodge's Numbers by SPSA: Promising, Not Confirmed

**Check-in report CI-019.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 and 20 October 2026.
Companion records: journals 0069 and 0070; `LAB-REPORT-BRAIN.md`
section 43; CI-016 to CI-018.

---

## Abstract

With every team on the landfall defaults, half the deaths are in the
dodge ashore (CI-016), and its numbers were first guesses. SPSA tuned
`dodge_horizon`, `dodge_margin` and `dodge_clear` over 16 steps of 24
fresh seeds. It ended at 2.05 s, 1.52 and 3.73 tiles (defaults 4, 0.8,
2.5): start later, count more near misses, clear further. On 48 seeds
the search never saw, that reads +1.24 ± 0.75 a seat, positive in
every range. The trial team's dodges killed it 6.4% of the time,
against 7.4% for the defaults. At 1.65 errors this is not past the
two-error rule. A second fixed batch of 48 seeds decides, by a rule
stated before it runs.

**Index terms:** SPSA, confirmation, optional stopping, reflexes.

---

## I. Introduction

### A. Intent

Tune the dodge with the tool CI-018 proved, and confirm the result
honestly.

### B. The prompts

> commit. push. continue backlog

(Several times during the run; progress was committed each time:
`d39f3b2`, `405a54e`, `4e85133`.)

> hey claude. you are doing very well at this bot design. I want to
> bake you a cupcake. What do you want yours to say?

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-018: SPSA crawls on the largest effect | 24 seeds a step, 16 steps, A 1.0, C 0.5 | 12 seeds: too noisy for smaller effects |
| D2 | Steps 5 and 6 hit the cap on noise | Let the run finish; note A too high | Restarting: six steps of readings lost |
| D3 | The end point is a hypothesis | Confirm on seeds 0–47, unseen by the search | Reading the search's own seeds |
| D4 | F: +1.24 ± 0.75 | Not promoted; defaults stay | Promoting on 1.65 errors and a favourable meter |
| D5 | Optional stopping | One more fixed batch, 48 to 95, the rule stated first | Adding seeds until it crosses |
| D6 | F: the walk rough at A 1.0 | Next tuner runs at A 0.3, 32 seeds a step for three weights | |

---

## III. Methods

`STEPS=16 SEEDS=24 SEED0=2000 A=1.0 C=0.5 TUNE="dodge_horizon
dodge_margin dodge_clear" make spsa`. Confirmation: the end point
(`theta.toml`) as `CENSUS_WEIGHTS`, the defaults for the baselines,
`CENSUS_SEED0` 0, 16, 32 with `CENSUS_SEEDS=16`, `CENSUS_ACTS`;
naval.map, 900 s, respawns 3, the fair census. Dodge rates from the
meter by team (CI-007), summed over the three ranges.

---

## IV. Results

End point: horizon 2.05 s, margin 1.52, clear 3.73. Steps outside
their errors: 9 (−2.49 ± 0.94) and 11 (−2.35 ± 0.72); in both, the
larger margin won.

| Confirmation | d a seat |
| --- | --- |
| 48 seeds | +1.24 ± 0.75 |
| By range | +0.36 ± 1.59, +1.96 ± 1.24, +1.39 ± 1.06 |

| Ashore dodges | Begun | Hit | By the shell | Killed |
| --- | --- | --- | --- | --- |
| End point | 8,730 | 29.0% | 20.2% | 6.4% |
| Defaults | 17,457 | 35.0% | 16.9% | 7.4% |

---

## V. Discussion

1. **A later dodge is a shorter detour.** At 4 s a tank leaves its
   errand early and meets other fire. At 2 s it is caught by its own
   shell a little more often, but hit and killed less overall.
2. **The rule holds even for a good lead.** All three ranges are
   positive and the meter agrees, but 1.65 errors is not two. The
   second batch is fixed in size and rule before it runs.

**Next:** seeds 48 to 95.
