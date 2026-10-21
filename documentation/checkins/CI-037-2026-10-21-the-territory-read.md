# The Territory Read: the Held Ground

**Check-in report CI-037.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0094; `LAB-REPORT-BRAIN.md` section 68;
CI-036; roadmap backlog 57, 12e.

---

## Abstract

Goal 4's last open item: the team board built territory every second
and nothing read where it was. LR 9's design gave it one job, Defend
when there is threat in our territory. The board now keeps the held
ground, the influence of the side's pillboxes and depots alone,
thresholded at one work's value at 14 tiles. A hostile on it beyond
the base's alarm is an incursion: it counts toward Defend at
`w_defend_ground` (1), and the guard goes to the raided outlying work
instead of rotating. In a family of a raid on an outlying pillbox
while the team is away expanding, the Commander meets the raider in
400 of 400 seeds, none at 0. The census guard reads -0.36 +/- 0.27 a
seat over 48 seeds, inside the noise; the read acts about 22 seconds
a match. One older family loses 3.7 points and holds its floor.

**Index terms:** territory, influence, team board, Defend, scenario
families, census guard.

---

## I. Introduction

### A. Intent

Paul's choice: the territory read. Make the board's territory act.

### B. The prompt

> territory read. continue.

> continue.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | F: the ladder reads the territory's share; nothing reads where | Read where, by LR 9's Defend | Expand by territory: Expand already reads the front (LR 58) |
| D2 | LR 24: the sector threat's long tail made Defend fire all match | The held ground of works alone, tanks left out | The board's territory with tanks: every tank's ground "held" |
| D3 | F: with no hostile works, every sector is "ours" | A threshold: one work's value at 14 tiles (`GROUND_R`) | Influence above 0 |
| D4 | F: a Defend posture alone keeps the tank home | The guard's post is the raided work (`raided`) | Posture only: the tank went home, not to the raid |
| D5 | Either/or is a scalar | `w_defend_ground` (1) blends the incursion into Defend and gates the post | Two weights: one read |
| D6 | F (three stagings): the gun's reach; a spotter out of sight; Guard held both ways | The team away at 25 s, the spotter toward the raider, a depot to expand to | Hits on a raider in gun range: the read is not needed there |
| D7 | F: the guarded-base raid fell 85.5 to 81.8%; a teammate's depot by the raider | Kept: an outlying work raided is defended, as designed | Excluding allies' works: their ground is the team's |
| D8 | F: at first the base's own depot became the post | Only an outlying work (past `BASE_R`) | — |
| D9 | The census binary predated D8 | Stopped and run again on the final code | Reporting the first run |
| D10 | F: -0.36 +/- 0.27 | On, by Paul's rule of 20 October | Off until tuned |

---

## III. Methods

**Code.** `board.rs`: `GROUND_R`; `TeamBoard::ground`, `incursion`,
`incursion_at`; in `rebuild`, the ground from pillboxes and depots,
the incursion beyond `BASE_ALARM`, Defend's threat plus
`w_defend_ground` times it; `meter::ground`. `commander.rs`:
`defend_base` takes `raided(obs, base)`. `weights.rs`: `DefendGround`.
`meter.rs`: `grounds`. `scenario.rs`: a pillbox's `toward`; a move
event's `toward` and `step`. Test:
`a_hostile_on_the_held_ground_is_an_incursion`.

**Family.** As section 68; `make scenario S=... RUNS=400
SET="w_defend_ground=v"` for v in 0, 0.25, 0.5, 1; the guarded-base
raid at 0 and 1.

**Guard.** `build/lr68/run.sh`, the fair census as LR 62, trial the
defaults, baseline `w_defend_ground = 0`; pooled over 48 per-seed
differences.

---

## IV. Results

| Measure | Result |
| --- | --- |
| The family | 0% at 0; 95.0% at 0.25; 100% at 0.5 and 1 |
| The guarded-base raid | 85.5% at 0; 81.8% at 1 (floor 75%) |
| Incursion seconds, trial / rest | 3,125 (2,694 defended for it alone) / 6,097 (none) |
| The guard, 48 seeds | -0.36 +/- 0.27 a seat |

Tests: brain 86 passed and 12 ignored; client 32; net 14; proto 3; sim
50. Nine families above their floors.

---

## V. Discussion

1. **Territory needs eyes.** A pillbox sees nothing; the held ground
   is defended only where a teammate or a drone sees it.
2. **Where, not how much.** The share of the map was already read;
   what the board lacked was the place, and the guard's post is where
   it acts.
3. **Every part of 12e is read.**

**For Paul:** none blocking. Commits are local until you say push.
