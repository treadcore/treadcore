# Read Sooner: A Sequential Census, Replayed

**Check-in report CI-042.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 24 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 78; roadmap backlog
59; `scripts/sequential.py`.

---

## Abstract

A census of fixed length spends as many seeds on a clear call as on a
close one. LR 72 found the dodge settled by seed 20 of 176. This
check-in replays every finished census on the census machine (44 of
them, 11 304 seeds, LR 72 to 76) under two group-sequential tests with
eight looks, each bound set by simulation to the fixed census's false
alarm rate (P(|Z| > 2) = 0.0455). O'Brien-Fleming reached 43 of the 44
verdicts on 76% of the seeds; Pocock spent 69% and lost five. The one
O'Brien-Fleming miss, LR 73's `-column`, stopped outside at 100 seeds
(z -3.08) and read -1.02 at 200: a false early stop, at about the rate
the design allows. By the rule set before the replay the test is not
adopted. An early stop as a mark for a re-read is left to Paul.

**Index terms:** brain census, sequential testing, O'Brien-Fleming,
Pocock, group-sequential design, false alarm rate.

---

## I. Introduction

### A. Intent

To take the next backlog item that needs no cores while the profiles'
round robin (backlog 60, LR 77) runs on the machine: backlog 59, a
census that stops when its reading is clear.

### B. The prompts

> hi. we just started a run of treadcore. and you were working on next items while it computes. also. ...

> its going well. thanks. continue. resume.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "working on next items while it computes" | Backlog 59: it replays logs on the Mac and leaves the machine's 24 threads to the round robin | 61 (rotation): it needs 60's matrix; profiling the world map: the machine's CPU is busy |
| D2 | F: LR 63 and LR 71's logs were on the Mac on 21 October and are gone | Replay every finished census on the machine instead (LR 72 has LR 71's census again) | Rerun LR 63: hours of the machine for a replay |
| D3 | The census reads a continuous paired difference, not wins and losses | A group-sequential z-test on the seed differences | Fishtest's SPRT on a pentanomial: made for game results, and needs an effect size to test against |
| D4 | "The rule set first stays honest" (backlog 59) | Each bound's c by simulation, so the false alarm rate over all looks is the fixed census's 0.0455 | A bound of 2 at each look: its false alarm rate over eight looks is several times larger |
| D5 | The two classic shapes trade early stops for late power | Both: O'Brien-Fleming (strict early, 2.12 last) and Pocock (2.56 throughout) | Choose one beforehand: the replay is cheap |
| D6 | Parallel jobs finish their seeds together | Jobs interleaved by round, job order by first seed | Seeds in index order: a census never sees them so |
| D7 | The rule before the run | Written before the replay: every verdict the same, or a flip only within 0.1 errors of the bar | — |
| D8 | F: twelve LR 73 censuses read 0.00 on every seed (switches already on) | Skipped: no information | Count them: they would pad the saving |
| D9 | F: `-column` flipped, far from the bar | Not adopted, by D7's rule; the form that keeps the saving (an early stop marks a re-read) left to Paul | Loosen the rule after the fact |
| D10 | LR 77 was named in the round robin's notes before it started | This step is LR 78; 77 goes above it when the matrix is in | Renumber: the notes are a record from before the run |

---

## III. Methods

**The data.** On the census machine, every `brain_census: difference`
line (each job's seed differences) under `build/census`, `switches`,
`board`, `front`, `front2` and `sitepoor`, copied to the Mac. Left out:
`build/spsa` (each step a census of a few seeds) and the round robin
under way (`build/census/20261024-211723`).

**The test.** `scripts/sequential.py DIR...`. A census's seeds, its
jobs interleaved by round; looks at n_k = round(k N / 8), k = 1 to 8;
z_k the mean of the first n_k over its standard error. It stops at the
first k with |z_k| > c s(k): O'Brien-Fleming s(k) = sqrt(8 / k),
Pocock s(k) = 1. c is the 0.9545 quantile of max_k |S_k / sqrt(k)| /
s(k) over 200 000 Gaussian random walks (seed 59): 2.117 and 2.557.
The verdict is "out" at a stop, "in" at the end without one; it is
compared with the fixed census's |z_N| > 2 and its sign.

**To repeat.** On the machine:

    scripts/sequential.py build/census/20261022-200008 \
        build/census/20261024-053657 build/switches/*/* \
        build/board/*/* build/front/*/* build/front2/*/* build/sitepoor/*/*

---

## IV. Results

| | Seeds | Share | Verdicts like the fixed census |
| --- | --- | --- | --- |
| Fixed census | 11 304 | 100% | 44 of 44 |
| O'Brien-Fleming | 8 547 | 76% | 43 of 44 |
| Pocock | 7 800 | 69% | 39 of 44 |

The dodge is settled at seed 25 of 200 by either. Pocock's misses are
four narrow readings past two errors (z 2.16 to 2.49), which its last
bound of 2.56 cannot reach, and `-column`. O'Brien-Fleming's miss,
`-column` (LR 73): z -1.41, -1.98, -2.45, -3.08 at 25, 50, 75 and 100
seeds (bound 2.99 at the fourth look), then -2.40, -1.93, -1.65, -1.02.
The table for each census is in LR 78.

---

## V. Discussion

The saving is real and comes from the large effects, as LR 72
expected. The miss is the price of looking: at a 4.55% false alarm
rate over 22 censuses inside two errors, one false stop is the expected
count. The fixed census pays the same rate, but at the end. What a
sequential census gives up is the 100 more seeds in which a run of
early luck regresses. The rule set first says not to adopt it. One form
keeps both: stop early, but treat an early "out" as the reason for a
re-read on fresh seeds before a default changes, as LR 76 already did
for team_rotate. That changes how a verdict is reached, which is
Paul's call.
