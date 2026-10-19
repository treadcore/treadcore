# SPSA on the Paired Census, and Its First Planted Test

**Check-in report CI-017.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journals 0066 and 0067; `LAB-REPORT-BRAIN.md`
sections 40 and 41; CI-001 VI.

---

## Abstract

Backlog 23, the honest tuner. The census now takes a weights file for
its baseline teams. A new script, `make spsa`, tunes named weights by
simultaneous perturbation: one paired census a step, plus side against
minus side, on fresh seeds. Its planted test detuned `landfall_memory`
to 5 s and asked SPSA to bring it back toward 30. After eight steps it
stood at 3.69 s, with every step's reading inside its error. The test
most likely sat on a flat: memory acts on sightings 10 to 60 s old,
and the perturbations reached only 3 to 7.5 s. It runs again with
wider perturbations.

**Index terms:** stochastic approximation, SPSA, planted test, noisy
optimisation.

---

## I. Introduction

### A. Intent

To make the tuner honest (CI-001 VI) and prove it on a planted weight
before it tunes the dodge.

### B. The prompts

> commit. push. continue backlog

> i want to see it run. how much backlog  items before this is testable.

The second was answered with no backlog items: the defaults are on
`main`, and `make quad` or `FLEETS=1 make run` shows them.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-001 VI.C: SPSA reads the paired census directly | SPSA, in log space, Spall's gain schedule | Noise-aware (1 + λ): 600 matches a generation |
| D2 | SPSA needs both sides' weights | `CENSUS_BASE_WEIGHTS` | Two censuses a step against the defaults: twice the cost and unpaired |
| D3 | One noisy reading must not throw a weight | Each step capped at `MAXSTEP` 0.5 in log | |
| D4 | Fresh seeds each step | `SEED0 + (k − 1) S` | Fixed seeds: the search would fit their starts |
| D5 | `w_wounded` (CI-001's plant) never measured | Plant `landfall_memory`, measured at about 3.4 a seat | |
| D6 | The first plumbing log showed only the trial's weights | The census header prints the baseline's too | |
| D7 | Paul's prompt, four steps in | Commit the tool with the run marked running | |
| D8 | F: the walk is noise; the plant on a flat | Run again at C 1.0 | Declaring SPSA broken: the test gave it no slope |

---

## III. Methods

`scripts/spsa.sh`: for k = 1..K, a_k = A/(k + 1)^0.602, c_k =
C/k^0.101; δ_i = ±1 at random; the plus file θ_i e^(c_k δ_i) is
`CENSUS_WEIGHTS` and the minus file θ_i e^(−c_k δ_i) is
`CENSUS_BASE_WEIGHTS`. The census runs in `JOBS` ranges side by side
and its per-seed differences are pooled to d̄ and its error. log θ_i
moves by min(MAXSTEP, max(−MAXSTEP, a_k d̄ / (2 c_k δ_i))). Planted:
`SET="landfall_memory=5" TUNE=landfall_memory STEPS=8 SEEDS=12`,
naval.map, 900 s, respawns 3, seeds 1,000 to 1,095.

---

## IV. Results

| k | d̄ | ± | memory |
| --- | --- | --- | --- |
| 1 | -0.13 | 1.80 | 4.69 |
| 2 | -0.87 | 1.24 | 3.27 |
| 3 | +0.55 | 1.33 | 3.99 |
| 4 | +1.21 | 1.43 | 2.68 |
| 5 | -0.59 | 0.88 | 3.21 |
| 6 | -0.84 | 1.09 | 4.05 |
| 7 | +0.36 | 1.57 | 4.45 |
| 8 | -0.77 | 1.14 | 3.69 |

Tests: brain 73 passed and 8 ignored; no warnings.

---

## V. Discussion

A planted test has to put the weight where the slope is, not only away
from the optimum. The census resolves about 1.2 a seat at 12 seeds,
and the gradient between 3 and 7.5 s of memory is far below that.

**Next:** the planted test at C 1.0.
