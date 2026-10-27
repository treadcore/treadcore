# Survival Maps: Low Armour Means Home, the Missing Drones Found, and the First Net Off the Bottom of the Census

**Check-in report CI-062.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 100;
`AI-BRAINZ-PLAN.md` sections 7 and 9; `scenarios/training/`;
`build/train/NOTES-maps.txt` on the Mac; `build/net7/` on the census
machine.

---

## Abstract

Three survival maps were written and read before training: 7 (low and
far from home), 7a (low, go home), and 8 (outnumbered; amended first,
because the scripted brain lived only 5% of it as drawn). The net
learned map 7a (99.7%) but stayed at 0 on map 7. Probes showed the
net's drive actions can dodge every shell with exact look-ahead, so
the limit was the senses; a shell contact took the dodge from 90 to
93% up to 95.9%. A traced death then showed what was killing it: a
strike drone, not the mortar. The net had no drones up and no sense
of the air. With the drone head on and an air contact, map 7 went
from 0 to 44.3% (scripted 53.3%). One net seat in a real match died
half as often (0.82 deaths in 3 minutes, from 1.73). A team of the net
read -20.33 +/- 0.27 in the census: level with a random net, so the
rule fails, but +3.96 over the last net, with four times the kills.

**Index terms:** curriculum, sensor design, upper-bound probe, drone
defence, transfer.

---

## I. Introduction

### A. Intent

Teach the learned tank to survive, the census's first need (LR 99).

### B. The prompt

> survival maps. implimment

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Survival needs hurt, short tanks | `armor` and `shells` on a scenario tank; armour, death time and `alive_time`, `armor_gained`, `armor_at_least` | — |
| D2 | Map 8 as drawn: the scripted brain lived 5% | Amended before training (home 8, enemies 14) | Train on an unsurvivable map |
| D3 | Map 7: 0 of 300 after 400 generations | A step map, 7a (no enemy): low armour means home | More generations on 7 |
| D4 | Map 5 stalled at 90 to 93% | Probes: the actions by oracle (88.5%) and by simulation (100%) | Blame the learner, or change the actions |
| D5 | The actions suffice; the senses fold when and where | The shell contact, appended; older nets widened with zeros | A new net from scratch |
| D6 | The trace: deaths from a 35-damage hit | Read as `STRIKE_DAMAGE`: the drone head on, the air contact | Keep training against the mortar |
| D7 | Phase 1's enemy fell to 64.9% | Failures read first: 114 of 117 enemies destroyed by the net; both readings reported, the second marked as chosen after the result | Change the rule silently |
| D8 | Between maps and the census | The match reading: one seat of a dealt match, first 3 minutes, against its scripted brain | Only the census, which cannot tell nets apart yet |
| D9 | The census of the map 7 net | Beside the run in a worktree, LR 97's seeds, rule first | Stop the run |

---

## III. Methods

On the Mac: `make train` with the courses in turn, from `nets/map4.bin`
and then from each kept net, held-out seeds from 9,000,000; `PROBE=oracle`
and `PROBE=sim` (map 5's situation built in `learn.rs`, nine actions
played forward in a copy of the world); `MATCH=40` (seeds from
8,000,000). On the census machine at 814d601, in a worktree:
`CENSUS_NET=nets/map7.bin SEEDS=110 SEED0=500000 MAP=all JOBS=24 make
census`. Deaths, kills and held are summed from the trial team's lines
in every `g-*.log`.

---

## IV. Results

| Read | Result |
| --- | --- |
| Map 7a, trained | 99.7% (299 of 300); scripted 100%, untrained 0% |
| Map 5, oracle / simulated bound / trained with the shell contact | 88.5% / 100% / 95.9% |
| Map 7 with drones, trained | 44.3% +/- 2.9; scripted 53.3% |
| Map 8, untrained on it | 15.7% |
| Match, first 3 minutes (map 4; map 7a; map 5; map 7; scripted) | deaths 1.77; 3.08; 1.73; 0.82; 0.33 |
| Census, nets/map7.bin | -20.33 +/- 0.27 (random -20.35; map 4 -24.29) |
| A match: deaths, kills, held, score (map 7) | 16.6, 3.7, 2.1, 5.3 |
| Brain tests | pass (122) |

---

## V. Discussion

The step map, the probe and the trace each turned a wall into a
result. What a net needs in a match is now partly known: the bubble,
then going home, then the dodge. The census cannot yet tell the net
from a random one, but it is no longer at the bottom.
