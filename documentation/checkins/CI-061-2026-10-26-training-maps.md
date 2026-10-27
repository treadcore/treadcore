# Training Maps: Scenario Files Teach the Learned Tank, a Guarded Depot Taken, and a Census That Asks for Survival

**Check-in report CI-061.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 99;
`AI-BRAINZ-PLAN.md` sections 7 to 9; `scenarios/training/`;
`build/train/` on the Mac; `~/code/treadcore-net/build/net4/` on the
census machine.

---

## Abstract

The guided-training plan of CI-060 is built. A scenario file now
carries a `[reward]` and a `kind = "net"` seat, so one file is read by
the scripted brain and by the trainer on the same seeds. The trainer
mixes courses, rehearsing old skills as it learns new ones. Three
training maps were written and read before training:

| Map | Scripted brain | Untrained net |
| --- | --- | --- |
| 4. the guarded depot | 100% | 0% |
| 5. under a shell | 99.8% | 79.3% |
| 6. an enemy that fires back | 20.3% | 0% |

The net learned map 4 (93.7% of 300 held-out seeds) and kept phases 1
and 3. It did not pass maps 5 and 6 (92.9%; no duel won), even after
800 more generations. A census of the map 4 net read -24.29 +/- 0.45.
The match logs show every net team dying 18 to 19 times of a possible
20 and killing about once, so the census cannot yet tell nets apart.
Survival is next; one net head inside the scripted brain is proposed
beside it.

**Index terms:** curriculum, rehearsal, scenario files, evolution
strategies, transfer.

---

## I. Introduction

### A. Intent

Implement the guided training of LR 98: training maps as scenario
files, maps 4 to 6, trained and read.

### B. The prompt

> ok. and for this dev version. lets continue to develop. impliment .

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | LR 98: one file a behaviour | `[reward]` and `kind = "net"` in the scenario format; `run_net`, `reward` | A second format for training: two situations would not compare |
| D2 | LR 97: a net forgets what it does not rehearse | A generation's episodes take the courses in turn | Train each map alone |
| D3 | Map 4 needs the guard killed | The gun head also takes a live hostile pillbox in reach | Leave pillboxes to the mortar: its minimum reach is 12, past the ring |
| D4 | The rule: a map whose untrained net reaches its floor is no test | Each map read on the scripted brain and the untrained net first; all under their floors | — |
| D5 | Map 6's scripted reading is 20% | Floor kept at the plan's 50% | Lower it to the scripted brain's: the plan asks for half the duels |
| D6 | Map 4 passed | `nets/map4.bin` kept; its census on LR 97's seeds, beside the run in a worktree | Stop the run: Paul left it running all day |
| D7 | Maps 5 and 6 plateaued; phase 1's enemy fell to 89% | The longer run not kept | Keep the newest net |
| D8 | Every net team dies nearly all its 20 times | Survival maps next, one at a time; a hybrid proposed for Paul | More attacking skill: the census says it does not count yet |

---

## III. Methods

On the Mac (M1, 8 threads): `make train` with
`GROUND=phase1,phase3,scenarios/training/04-the-guarded-depot.toml`,
`START=nets/phase3.bin`, `SEEDS=15`, 400 generations. Then all five
courses, `SEEDS=20`, 500 generations, then 800 more with maps 5 and 6
listed twice. Held out from seed 9,000,000. On the census machine at
5a10af0, in a worktree: `CENSUS_NET=nets/map4.bin SEEDS=110
SEED0=500000 MAP=all JOBS=24 make census`. Deaths, kills and held are
summed from the trial team's lines in every `g-*.log`, and the same
for LR 97's two censuses.

---

## IV. Results

| Read | Result |
| --- | --- |
| Map 4, trained | 93.7% +/- 1.4 (281 of 300); phase 1 99.1 to 100%, phase 3 99.7 to 100% |
| Maps 4 to 6 after 500 generations | 98.7%, 92.1%, 0% |
| After 800 more | 99.3%, 92.9%, 0%; phase 1 enemy 89.2% |
| Census, nets/map4.bin | -24.29 +/- 0.45 |
| Deaths, kills, held a match (random; phase 3; map 4) | 18.3, 0.8, 1.8; 19.0, 1.0, 1.9; 18.6, 0.9, 1.9 |
| Brain tests | 104 pass |

---

## V. Discussion

Training maps work as designed. The situation is placed predictably,
both brains are read on it, and a net learns what the map rewards and
keeps what it rehearses. What a map rewards is not yet what a match
rewards. A match is first of all about staying alive, and no map has
taught that.
