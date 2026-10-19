# Who Saw the Gun That Sank the Boat

**Check-in report CI-009.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0057; `LAB-REPORT-BRAIN.md` sections 31 and
32; CI-008.

---

## Abstract

CI-008 explained its null result by sight: a boat sees a gun only at
the gun's own range, too late to choose around it. It proposed pricing
the guns the team had seen (backlog 32). This step measured that
explanation before building anything. Over 16 seeds, the boat had seen
its killer in the 10 s before in 128 of 164 landfall deaths (78%); a
teammate alone had seen it once. The explanation is withdrawn, and
backlog 32 closes with no part to build. Boats land under guns they can
see. Backlog 33 proposes standing off a covered beach, by a weight
whose default Paul sets.

**Index terms:** partial observability, measurement before design,
correction.

---

## I. Introduction

### A. Intent

To read backlog 32's premise, as CI-008 planned, before building its
part.

### B. The prompt

> yes 0. commit. push. continue backlog

(The same message as CI-008; this is its continuation.)

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-008 D6: read first | Sample each trial seat's sight at 1 Hz; class each landfall death's killer | Building the remembered-gun price first |
| D2 | Cost | 1 Hz sampling under `CENSUS_ACTS` only | Every tick: 20 times the cost for a share bounded either way |
| D3 | F: the boat saw its killer in 78% | CI-008's explanation withdrawn, marked in LR 31; backlog 32 closed | Building 32 anyway: a teammate added a gun once in 164 |
| D4 | F: boats land under guns they see | Backlog 33, standing off, as a weight | Choosing its default: Paul's to give |

---

## III. Methods

In `brain_census` (fleet.rs), at each 1 Hz sample and for each living
trial seat, `w.observe(p).hostile_tanks()` records `(seat, owner) →
tick`. At a `TankDestroyed` of a trial seat whose last code was
`landfall` and whose `by` is set, the killer counts as seen by the boat
if `(owner, by)` is within 10 s, and by a teammate if any other trial
seat's is. Null run, `CENSUS_SEED0` 0, 6, 11 with `CENSUS_SEEDS` 6, 5,
5; naval.map, 900 s, respawns 3, the fair census.

---

## IV. Results

| Seen by | Deaths | Share |
| --- | --- | --- |
| both | 128 | 78.0% |
| neither | 35 | 21.3% |
| a teammate only | 1 | 0.6% |
| the boat only | 0 | 0 |

Null: 0.00 on every seed. Tests: brain 71 passed and 8 ignored; client
31; net 14; proto 3; sim 50.

---

## V. Discussion

Two explanations in two steps were wrong: LR 29's equal shares (CI-007)
and LR 31's sight (this report). Both were reasoned from the code
rather than measured, and both were caught by the next step's read.
They cost no code, because each read came before the part it would
have justified. The working rule is to measure the premise of a
backlog item before building it.

**Next:** Paul's default for `landfall_wait`; then backlog 33.
