# The Sequential Census with a Re-read

**Check-in report CI-043.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 79; CI-042 and
section 78; roadmap backlog 59; `CENSUS.md`; `scripts/census.sh`,
`scripts/sequential.py`.

---

## Abstract

LR 78's sequential census was not adopted because one early stop
(LR 73's `-column`) was false and final. This check-in builds the form
LR 78 left to Paul: an early stop is read again, sequentially, on fresh
seeds, and counts only if the re-read agrees. `LOOKS=8 make census`
follows the census as it plays, from its match lines, which match the
census's own seed lists to 0.01 over 976 seeds. The two stages hold
the false alarm rate to 2.8%, below the fixed census's 4.8%, and
`-column` now reads as the fixed census does 81% of the time. They
cost seeds, though: 106% of the fixed census over LR 78's 44 censuses,
and less power at the effects the census mostly reads (z 2.5: 53%
against 68%). No re-read size beats the fixed census on both counts.
By the rule set first it is an option for large expected effects, and
the fixed census stays the default. A live test on the Mac exercised
both paths, the end and the early stop with its re-read.

**Index terms:** brain census, sequential testing, O'Brien-Fleming,
confirmation, statistical power, false alarm rate.

---

## I. Introduction

### A. Intent

To build what LR 78 proposed and test it against a rule written before
the build, while the profiles' round robin plays on the machine.

### B. The prompt

> commit and push. then build the re-read version

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "commit and push" | LR 78 committed and pushed on main (`732fde3`) | — |
| D2 | A stop must happen while the census plays; jobs print their seed lists only at the end | Rebuild each seed's difference from the match lines (`team t (trial): score`), checked against the finished lists | Change fleet.rs to print a line a seed: the round robin plays the machine's tree, and a census binary should not change mid-study |
| D3 | The round robin runs `scripts/census.sh` from the machine's tree | Change it on the Mac only; the machine pulls after the round robin and the team_rotate re-read | Pull now: bash reads a running script as it goes |
| D4 | "Either/or is a scalar" | `LOOKS` opt-in; unset, `make census` is as it was | Make it the default before its reading |
| D5 | A stop at the last look is the fixed census | Only a stop before the last look is re-read | Re-read every outside verdict: it would also re-read the fixed census's own |
| D6 | The rule before the build | (a) read as it plays, (b) false alarms at or below 4.55%, (c) LR 78's verdicts as often as not on fewer seeds | — |
| D7 | F: (c) fails, 106% of the seeds | Not the default; kept as an option for large effects | Shrink the re-read until it saves seeds: then it loses the close calls (the size table) |
| D8 | The re-read size explored after the verdict | Reported as exploration, no rule changed | Pick a size and call it adopted |
| D9 | A live test needs an early stop for sure | A rookie trial (`skill = 0`) at 120 s; a bare brain (`caps =`) read inside, so it tested the end path | A fake census binary: it would test the plumbing, not the census |

---

## III. Methods

**Reading as it plays.** `sequential.py watch DIR BIN` every 20 s:
each job's match lines, a seed finished when its three plays' nine
team lines are read; its difference the trial team's mean score a seat
less the baseline's (fleet.rs `brain_census`). The whole rounds across
jobs, interleaved by first seed, are the census so far. At each look
(round(j N / K)) the reading and the bound go to `sequential.txt`; past
the bound before the last look, exit 3. `census.sh` then marks the
directory `stopped`, kills the jobs by their frozen binary's path, and
plays a second stage into `reread/`, its `run.env` SEED0 + SEEDS.
`sequential.py verdict` reads both.

**The check of (a).** Four finished censuses from the machine (LR 72's
dodge on every map, LR 76 `-posture` and `-dodge`, the board's
`nofront`): the rebuilt differences against each job's "a seed:" list.

**(b) and (c).** `sequential.py twostage` over LR 78's 44 censuses:
the simulation (N = 200 a stage, unit spread, 4 000 censuses a row,
expected z 0 to 10) and the replay (stage one on the census's own
seeds in order; the re-read drawn from them with replacement, 200
times). The size table, a quarter, a half and the whole as the
re-read, 3 000 censuses a cell.

**The live test.** On the Mac, `LOOKS=4 SEEDS=32 SEED0=900 SECS=120
JOBS=8 make census`, naval.map, first `TRIAL` with every capability
off, then a rookie (`skill = 0`).

---

## IV. Results

In LR 79: (a) 976 seeds, the largest difference 0.01; (b) false alarms
2.8%; (c) 44 verdicts of 44 as often as not, 106% of the seeds; the
power and size tables; the live test.

---

## V. Discussion

Confirmation is not free. An early stop is cheap only when the effect
is large enough to stop at the first looks of both stages. For the
mid-sized effects most of this brain's parts have, the first stage
stops at look four to six and the re-read then costs a census more.
Each re-read also has a chance to miss a real effect, which is where
the power goes. LR 78's single stage saved seeds by trusting the stop;
LR 79's two stages are honest but spend the saving. The fixed census,
with its rule before the run, was already the efficient instrument
for the close calls this tuning makes. The sequential census earns its
place as a fast screen for a large effect.
