# The Dodge Planned by the Tank's Own Motion

**Check-in report CI-024.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journal 0080; `LAB-REPORT-BRAIN.md` section 53;
CI-023; roadmap backlog 44 to 47.

---

## Abstract

The dodge, the reflex behind 43% of deaths (LR 27), still let a tenth
of ashore dodges be caught by their own shell after LR 52's water fix.
Scenario families, single files whose geometry is drawn from the run's
seed, measured today's dodge at 89.6% (a tank rolling at full speed)
and 96.2% (at rest) over 1,000 seeds each, in three seconds. Traces
named two failures: a dodge point behind a rolling tank, reached by a
looping two-leg route, and a tank that arrived early and was sent back
under its shell by its errand. A motion model, `drive_to`, foresees the
world's route and the sim's rules of motion exactly. Two weights built
on it, `dodge_plan` and `dodge_release`, take both families to 100%. In
the fair census they cut own-shell catches ashore by 40% in every run,
and read +0.92 +/- 0.47 a seat on naval.map, +1.20 +/- 0.60 on a
generated map, and +0.70 +/- 0.66 on 48 fresh seeds: not confirmed
alone, +0.81 +/- 0.40 pooled. Both stay off, for Paul's choice.

**Index terms:** reflexes, motion models, foresight, scenario testing,
confirmation on fresh seeds.

---

## I. Introduction

### A. Intent

Continue goal 4 from where CI-023 left it, by the next lead with a
measured cause. CI-023 offered `duck` (backlog 42, four censuses) or
the boat's dodge (backlog 31). LR 52's own table offered a third with
a number on it: after the water fix, 10.4% of ashore dodges were still
caught by their own shell. The dodge is the reflex most deaths pass
through (LR 27), so a mend there could reach further than any part so
far. It was chosen.

### B. The prompt

> we are working on AI Brins. Read Roadmap and Backlog. Read the code
> section of AI and the newest labreport declarations. Continue.
> Improve. Impliment. Plan. Backlog.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt: "Continue. Improve."; LR 52's 10.4% | Take the dodge's remaining failures before `duck` | `duck` (42): four censuses on a part whose sign has flipped with the seeds; the dodge had a measured, concrete residue |
| D2 | `Dodge::order_on` reads the shell's line, not the tank's heading or speed; the sim draws scatter at launch | Hypothesis: the line carries nothing once fired; the tank's motion decides | Changing the across-line rule outright: today's point is kept wherever it works, so the change is measurable |
| D3 | The scenario form could not start a tank rolling | `[[tank]]` speed, heading, drive; `[[shell]]` offset, scatter, from_any, a lands_in range | A Rust test per geometry: one family file samples them all |
| D4 | The fixed case held on `manual` with the act `idle` | Commanders on `auto` in the families | Trusting a pass (LR 49's lesson) |
| D5 | One case, held narrowly at 1.9 s of 2.0 | Measure rates over 1,000 seeds, not a case | |
| D6 | Traces: F1 a point behind the tank; F2 an errand back under the shell | Two parts, each its own weight | One weight for both: the census could not say which carries it |
| D7 | Paul's rule (18 October): every either/or a scalar | `dodge_plan`, `dodge_release` in Settings, thresholded at 0.5, default 0 | Defaults at 1: a part is promoted by its census or by Paul |
| D8 | F: the plan on a straight-line model changed nothing | `drive_to` follows `Map::route_by` | Picking candidates on the eight compass lines only: it would still miss the corner braking |
| D9 | The foresight test read 0.000 with tanks that seemed parked | The test checked against two deliberate breaks | Accepting 0.000 |
| D10 | `dodge_hold` 1 cost 1.18 a seat (LR 28) | The release strikes only a Move foreseen inside the splash | Holding every errand, as `dodge_hold` does |
| D11 | The family's last failure: straight on 14 tiles, into a pillbox's reach | Straight on stops at the 5-tile circle | |
| D12 | A family's failing seed is worth keeping | `seed = n` in a file; two tests that fail with their weight at 0 | |
| D13 | F: the first census on the defaults' generated map | Kept as a second map (backlog 25); the standard run after | Discarding it |
| D14 | 1.94 errors, the rule of LR 47 | 48 fresh seeds (240 to 287), past two errors alone to confirm | Calling 1.94 a pass; pooling before the fresh run |
| D15 | Fresh: +0.70 +/- 0.66 | Not confirmed; both stay 0; on is Paul's choice | Promoting on the pooled 2.00 |
| D16 | The prompt: "Plan. Backlog." | Backlog 44 (done), 45 two shells, 46 the duck by `drive_to`, 47 families with floors | |

---

## III. Methods

### A. The brain's parts explored

- `Dodge` (lib.rs): fields `dodged`, `until`, `watch`, `tune` (now six
  numbers: horizon, margin, clear, hold, plan, release), `began`, and
  the new `landing` (where and when the shell dodged last lands).
- `Dodge::order_on`: the trigger (an indirect hostile shell landing
  within the splash and `dodge_margin` of the tank or of where it will
  be at its speed, between 2 ticks and `dodge_horizon` out); the four
  points of LR 52, each `SPLASH_RADIUS + dodge_clear` from the impact;
  the reachability test `fits`. With `dodge_plan` at 0.5 or more it
  hands them to `planned`.
- `Dodge::planned` (new): candidates in order (today's four; straight
  on, where the heading leaves the circle of `reach` round the impact,
  if it meets it; twelve round the impact every 30 degrees), each that
  `fits` driven by `drive_to` for the shell's flight; the first to end
  at least `SPLASH_RADIUS + dodge_margin` from the impact is taken; if
  none does, the one ending farthest off.
- `drive_to` (new, free function): the route by `Map::route_by(pos,
  to, boat, no danger)`; per tick, as `drive_tank` in the sim: the
  ground's cost (`BOAT_MOVE_COST` afloat on water), the heading error,
  easing past `TURN_BRAKE_ANGLE` to `TURN_CRAWL`, the pivot past
  `TURN_PIVOT`, braking before the next waypoint's corner, a boat's
  tighter turn, momentum at `TANK_ACCEL` and `TANK_BRAKE`, the hull
  turned at most `TANK_TURN` a tick, and a stop where the next step is
  not open. It leaves out the world's price on pillboxes' reach and
  known mines, and a re-plan after a block.
- `Dodge::release` (new): with `dodge_release` at 0.5 or more and the
  dodged shell still in flight, every `Command::Move` in the think's
  output whose `drive_to` end at the landing tick is inside the splash
  and the margin is struck.
- `Commander::think_released` (new): `think_map`, then `release` unless
  this tick's act is the dodge itself; when one is struck, `last_move`
  is cleared so the ladder asks again. Both `think` and `think_on` go
  through it.
- `Commander::think_map`'s dodge branch (read, unchanged): the dodge
  and `holding` before every rung; `holding` ends on arrival at
  `dodge_hold` 0, the opening F2 used.
- `Commander::set_settings`: passes the six numbers to `Dodge::tune`.
- The sim, read: `World::step`'s `Command::Move` (the world routes),
  `Map::route_by` (eight neighbours; a waypoint at each change of
  direction, at tile centres; the last replaced by the target),
  `drive_tank`, and the shell's `impact` drawn at launch.
- `meter.rs`: the dodge counts by group, trial and the rest, disjoint.

### B. The scenario form (scenario.rs)

`[[tank]]`: `speed`, `heading` in degrees (0 east, 90 south) or
`"any"`, `drive` (the rolling goal's distance, 20). `[[shell]]`:
`offset`, `scatter`, `from_any`, `lands_in = [lo, hi]`. The dice:
`Rng::new(seed ^ 0x5ce4a210)`, the run's seed (0 for a file run alone).
`seed = n` at the top pins a file to one of `many`'s seeds. The trace:
each tank's route end and legs, each indirect shell's landing and time.

The families (`scenarios/families/a-rolling-tank-under-a-shell.toml`, `a-standing-tank-under-a-shell.toml`):
two seats, 4.5 s; seat 0 a Commander on Auto at full speed on any
heading, or at rest; a shell of seat 1 from 30 tiles in any direction,
landing within 2.5 tiles of seat 0's tank in 1.5 to 3.5 s. A run holds
if the tank is never hit. `make scenario S=file RUNS=1000 SET="..."`,
seeds 1 to 1,000.

### C. The census

`build/lr53/census-bin`, frozen after `make test`. The trial team's
weights `build/lr53/both.weights` (`dodge_plan = 1`, `dodge_release =
1`) against the defaults; the fair census (every team in the trial seat
on every seed); `CENSUS_ACTS=1`, `CENSUS_LEDGER=0`; three jobs of 16
seeds. Standard: `CENSUS_MAP=maps/naval.map CENSUS_RESPAWNS=3
CENSUS_SECS=900`, seeds 0 to 47, then 240 to 287. The generated map:
the same without the map and respawns, seeds 0 to 47. Pooled by
`build/lr53/pool.py`: the mean of the per-seed differences and its
standard error; the meter's dodge counts summed by group.

---

## IV. Results

| 1,000 seeds each | Rolling | Standing |
| --- | --- | --- |
| Today's dodge | 89.6% +/- 1.0% | 96.2% +/- 0.6% |
| `dodge_plan` 1 | 94.5% +/- 0.7% | 97.0% +/- 0.5% |
| `dodge_release` 1 | 95.0% +/- 0.7% | 99.2% +/- 0.3% |
| both | 100.0% | 100.0% |

`drive_to_foresees_the_world`: 96 drives, 4.0 tiles on average, worst
0.000 tiles off; without corner braking 1.745, a straight line 4.691.

| Census, 48 seeds | Difference a seat | Ashore, own shell: trial / rest | Afloat, killed: trial / rest |
| --- | --- | --- | --- |
| naval, 0 to 47 | +0.92 +/- 0.47 | 5.7% / 10.1% | 11.6% / 14.7% |
| naval, 240 to 287 (fresh) | +0.70 +/- 0.66 | 6.3% / 10.2% | 11.7% / 14.4% |
| naval, the 96 pooled | +0.81 +/- 0.40 | | |
| generated, 0 to 47 | +1.20 +/- 0.60 | 6.2% / 10.6% | (9 dodges) |

Tests: brain 78 passed and 10 ignored; client 31; net 14; proto 3; sim
50; no warnings. 382 functions, 121 undocumented, 88 tests.

---

## V. Discussion

1. **The impact is known, so the line is not.** Every dodge since step
   10 chose its side by the shell's line; once fired, the only facts
   that matter are where the shell lands, when, and how this tank
   moves.
2. **Foresee the world, not a picture of it.** A plan on a straight
   line changed nothing, because the world routes. The model that
   worked copies the sim's rules, and its test proves it exact.
3. **A hold should know why it holds.** `dodge_hold` held every
   errand and cost 1.18 a seat; `dodge_release` holds only the ones
   foreseen under the shell.
4. **The mechanism replicates where the score does not yet.** Own-shell
   catches fell by 40% on every set of seeds. The score moved +0.7 to
   +1.2 each time, below what 48 seeds resolve alone. About 95 fresh
   seeds would settle it.
5. **Checks that cannot fail.** A goal that runs no brain, an assertion
   that tested the wrong thing, a pin one runner skipped, a meter's
   groups misread: each read as a pass or a number until run broken
   or questioned.

**For Paul:** whether `dodge_plan` and `dodge_release` go on by default.
**Next:** backlog 45 (two shells), 46 (the duck by `drive_to`), 47
(families with floors), 42 (`duck`).
