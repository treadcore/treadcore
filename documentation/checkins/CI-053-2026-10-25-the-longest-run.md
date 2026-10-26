# The Longest Run: a Night's Plan, and What We Expect

**Check-in report CI-053.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 88; roadmap backlog
69; `build/night/` on the census machine.

---

## Abstract

The first night of backlog 69: about ten hours on the census machine,
queued at 18:40. Block 1 reads eight profiles against the new
defaults. Block 2 screens sixteen weights at half and double. Block 3
runs SPSA over the five weights the screen ranks first, chosen by a
script written before the screen, then reads its point on 400
held-out seeds. Each block's rule and our expectations were written
before it played, so the morning can score the predictions as well as
the brain. Nothing changes a default overnight.

**Index terms:** brain census, slope screen, SPSA, preregistration,
league.

---

## I. Introduction

### A. Intent

A night's census, planned to about ten hours, with the expectations
written first.

### B. The prompts

> so good. can you plan about 10 hours a census.

> yes. please. and write about our expectations. the longest run.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | LR 87's plan, step 1 the slope screen and step 2 a league | Both tonight with existing code; the retreat fix (code) waits | Write the retreat fix first: a night lost to review |
| D2 | A 110-seed census about 8 min, 400 about 25 | 8 + 32 censuses of 110, SPSA 24 x 48, one of 400: about 8 h 40 min with slack | More weights: past ten hours |
| D3 | SPSA's weights must not be picked by eye after seeing the screen | `pick.py`, written before: the five largest gaps | Pick in the morning: a night idle |
| D4 | The playtest held the machine's screen | Closed before the run | Run beside it: both slow |
| D5 | "write about our expectations" | Predictions per run, in NOTES and LR 88, before | — |
| D6 | One step at a time; Paul's word for defaults | No default changes overnight; candidates read again later | — |

---

## III. Methods

`build/night/run.sh` on the machine at 7c97aa9: `make census` with
`TRIAL` each profile (110 from 160000), each screen file (110 from
170000), `pick.py`, `make spsa` (`TUNE` the five, `STEPS=24 SEEDS=48
SEED0=200000`), the held-out census of `theta.toml` (400 from 300000).
Each census's directory in `build/night/NAME.dir`; a line a census in
`build/night/progress.txt`.

---

## IV. Results

To come: LR 88's morning part.

---

## V. Discussion

Writing expectations down turns the night into two tests: one of the
brain, and one of what we think we know about it. LR 74's flat ridge
and LR 86's cowboy are the two priors in tension tonight.
