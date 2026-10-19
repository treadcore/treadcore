# Holding Out of the Splash

**Check-in report CI-005.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0053; `LAB-REPORT-BRAIN.md` section 28;
CI-004.

---

## Abstract

CI-004 found the shell dodge last held at 43% of the trial team's
deaths, and that a sixth of ashore dodges are caught by the shell they
dodged. This step tested one cause: tanks dropping the dodge on arrival
and driving back under the shell. Holding them at the dodge point until
the shell lands (`dodge_hold` 1) costs 1.18 ± 0.55 a seat over 48 seeds
and does not change the dodged shell's hit rate (16.7% against 16.6%).
The hypothesis is rejected; the weight stays 0.

**Index terms:** reflexes, causal test, paired census, weights.

---

## I. Introduction

### A. Intent

To run the first of the two causal tests CI-004 set, and record it as
its own step.

### B. The prompt

> commit. push. continue backlog

Given after CI-004 was pushed, with the causal runs under way. "Commit.
push." was read as recording the finished run now rather than waiting
for the second.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-004 D4: a reflex's death share is not a cause | Test the weight that targets the measured failure (the dodged shell's hits) | Tuning all four dodge weights at once: one cause at a time |
| D2 | The prompt, with the hold run done and `-dodge` not | Commit the hold result alone | Waiting about 20 minutes for both |
| D3 | F: -1.18 ± 0.55, outside two errors; by-the-shell rate unchanged | `dodge_hold` stays 0; hypothesis rejected | Keeping it for afloat only: the gain afloat is diluted and unmeasured for the trial team alone |
| D4 | F: the hits come during the drive | The next dodge levers are `dodge_horizon` and `dodge_clear`, for the tuner | Writing a new dodge before `-dodge` says whether the dodge pays at all |

---

## III. Methods

`CENSUS_DUMP` writes the default settings; `dodge_hold = 0` changed to
`1` is the trial's `CENSUS_WEIGHTS`. Three runs side by side,
`CENSUS_SEED0` 0, 16 and 32, `CENSUS_SEEDS=16`, naval.map, 900 s,
respawns 3, the fair census, `CENSUS_LEDGER=0`. The 48 per-seed
differences give the mean and its standard error. The meter's dodge
counts sum every Commander's dodges in each run.

The part under test is `Dodge::holding` (lib.rs): with `dodge_hold` h,
a dodge is held while the tank drives to its point, and also while the
tick is before `began + h * (until - began)`; at 1, until the shell
lands.

---

## IV. Results

| | d a seat | Ashore: hit, by the shell, killed | Afloat: hit, by the shell, killed |
| --- | --- | --- | --- |
| Null (CI-004) | 0 | 34.0%, 16.6%, 7.4% | 18.2%, 7.6%, 16.6% |
| `dodge_hold` 1 | -1.18 ± 0.55 | 34.2%, 16.7%, 7.3% | 15.5%, 6.0%, 13.9% |

By range of 16 seeds: -2.02 ± 1.05, -0.38 ± 1.09, -1.14 ± 0.68.

---

## V. Discussion

The meter was built to test exactly this, and it did: the dodged
shell's hits did not move, so tanks are not caught after arriving. The
cost in score is the errand time the hold gives up. What is left for
the dodge is when it starts and where it goes, and first whether it
should happen at all.

**Next:** `-dodge` at 48 seeds (running); then backlog 30.
