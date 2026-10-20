# Every Part On, a Dodge Against Every Shell, and the Duck Foreseen

**Check-in report CI-025.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journals 0081 and 0082; `LAB-REPORT-BRAIN.md`
sections 54 to 57; CI-024; roadmap backlog 45 to 50.

---

## Abstract

Paul changed goal 4's rule: new behaviours land on, with sensible
starting numbers, the census guards that the brain still works, and
tuning comes after the features. The planned dodge went on (LR 54);
then the eleven parts built and still off went on together, -0.85 +/-
0.59 a seat against them off, inside the noise, at about 30% more brain
time (LR 55). Three new parts followed, each found by a scenario family
and proved by a test that fails without it: the dodge planned against
every shell in flight (two shells 76.5% to 100%, four 56.6% to 99.7%;
LR 56), the duck foreseen to the gun's ready tick (76.6% to 83.4%), and
a shot held when its turn of the hull would carry the tank into a
splash, since a tank has no turret (LR 57). The guard for the three:
+0.21 +/- 0.68; dodges caught by their own shell ashore 6.1% to 1.6%.

**Index terms:** defaults policy, guard census, motion foresight,
reflexes, scenario families.

---

## I. Introduction

### A. Intent

To finish the brain's features before tuning: turn on what was built,
keep adding behaviours with starting numbers, and keep the brain
working.

### B. The prompt

> yes. keep all the new behavior. keep adding. we need to finish
> writing the brain. but ofcourse it still must work. even if slightly
> less. We will continue to adjsut scalars after all features are
> added. but we need to add them wile finding a sensible starting
> numbers.

It answered CI-024's question (the planned dodge on by default, and
commit and push).

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "yes" to CI-024's two questions | `dodge_plan`, `dodge_release` 1; committed and pushed (61c368b) | |
| D2 | "keep all the new behavior ... it still must work. even if slightly less" | A new rule: parts land on; the census guards; saved as a memory and in ROADMAP goal 4 | Keeping the promotion gate: Paul replaced it |
| D3 | "all the new behavior" | The eleven parts built and off turned on together | One at a time: eleven censuses for parts already measured alone near zero |
| D4 | Two tests read `Caps::BASELINE` as "off" | Their off cases name the part off | Weakening the tests |
| D5 | The guard: -0.85 +/- 0.59 | Accepted as "slightly less"; the likely carriers (posture, team_rotate) recorded for the tuning phase (backlog 48) | Hunting them now: tuning follows the features |
| D6 | "keep adding"; backlog 45 | `dodge_every`, `drive_path` | |
| D7 | Backlog 46 | A ready-gun family; RuleBot gunner | |
| D8 | F: the hull turned 123 degrees in a tick | Read the sim: a shot turns the hull; `dodge_fire` | Calling it a steering bug in `drive_to` |
| D9 | F: the duck re-picks each think | `duck_plan` holds its point, and foresees the ready tick | |
| D10 | The family said 0.4 to 1.4 s and set 0.9 | `reload = [lo, hi]` and the file corrected | Leaving the comment wrong |
| D11 | The seed-4 test held broken after `duck_plan` | Searched for a seed that fails broken (261) | Keeping a test that cannot fail |
| D12 | "add them while finding a sensible starting numbers" | The new weights are thresholded switches at 1, the families their evidence; continuous numbers (`dodge_margin`, `duck_ready`) kept | A sweep per weight now: after the features |

---

## III. Methods

**Parts explored.** `weights.rs`: `caps!` (every bit on), the new
`DodgeEvery`, `DodgeFire`, `DuckPlan`. lib.rs: `Dodge::threats` (every
hostile indirect shell in flight landing within `dodge_horizon`, beside
the one dodged), `Dodge::clearance` (the least, over the landings, of
the distance at each landing tick less the splash and `dodge_margin`),
`Dodge::planned` (now over the threats), `Dodge::release` (over the
threats), `Dodge::hold_fire` (each `FireDirect` or `FireIndirect`
foreseen with the hull turned to it), `drive_path` (the per-tick form
of `drive_to`). commander.rs: `think_released` (release unless the act
is the dodge; hold_fire always), `Commander::duck` (the guns seen and
remembered, the readiness test, the ring of sixteen; with `duck_plan`
the held point and the foreseen choice), the duck's call site (re-sent
when the point moves a tile or a second passes). The sim, read:
`FireIndirect` and `FireDirect` set the hull's heading; `drive_tank`.
scenario.rs: `[[tank]] around`, `reload`; trace fields; hits with
damage and drones; `set_weights` and RuleBots.

**Families** (`scenarios/families/`), 1,000 seeds each, `make scenario
S=file RUNS=1000 SET=...`: one shell and two shells, rolling and
standing; a tank under a ready gun (a RuleBot 9 tiles off, ready in 0.4
to 1.4 s; the tank's own gun at 2.5 s). Three and four shells from
copies in `build/lr55`.

**Guards.** Frozen binaries; the fair census on naval.map, 900 s, 3
respawns, seeds 0 to 47, `CENSUS_ACTS`; LR 55: `caps = baseline`
against the eleven off (`build/lr55`); LR 56 and 57: the defaults
against `dodge_every`, `dodge_fire`, `duck_plan` 0 (`build/lr57`).
Pooled by `build/lr53/pool.py`.

---

## IV. Results

| Step | Family or census | Before | After |
| --- | --- | --- | --- |
| LR 55, every part on | census, 48 seeds | | -0.85 +/- 0.59 a seat; 38 us a seat-tick (27 to 30) |
| LR 56, `dodge_every` | two shells, rolling / standing | 76.5% / 79.9% | 100% / 100% |
| | three / four shells, rolling | 67.7% / 56.6% | 100% / 99.7% |
| LR 57, `duck_plan` and `dodge_fire` | ready gun | 76.6% (no duck 63.7%) | 83.4% |
| LR 56 and 57 | census, 48 seeds | | +0.21 +/- 0.68; own shell ashore 6.1% to 1.6%, afloat 4.0% to 0.5% |

Tests: brain 78 passed and 10 ignored; client 31; net 14; proto 3; sim
50; no warnings. 386 functions, 121 undocumented; 96 weights, 17 bits;
nine scenario files and five families.

---

## V. Discussion

1. **On by default changes what the census is for.** At 48 seeds it
   resolves about a point a seat; most parts move less. As a guard it
   answers the question it can: did the brain break.
2. **The hull is the gun.** Treating the guns as free while dodging was
   a turret's habit in a game without turrets.
3. **Three questions, one model.** Where to dodge, when to go back,
   whether to shoot: `drive_to` answered each, and each family went to
   or near 100%.

**For Paul:** none blocking. The next features are backlog 49 (the
front read: Expand and the guard posts toward the contested sectors)
and 50 (allies' goals on the coordination screen).
