# Lab report: the Pillboy requirement, checked against Bolo

12 October 2026, evening. A requirement arrived in the form of two
documents: a beginner's tutorial for a "Tank Pillboy Game" (a Bolo
descendant with pillboxes, depots, momentum driving and boats), and a
technical audit of that tutorial against Stuart Cheshire's original
Macintosh Bolo manual. The audit's verdict: the tutorial matched
Bolo's core loop but got three things wrong, and could not, from a
manual alone, say anything about the game's code. This report takes
the audit's corrections as the requirement for treadcore's Pillboys,
checks each against what treadcore already says and does, records what
was decided, and says what is and is not done.

## 1. The requirement, as received

The audit corrected the tutorial on these points:

1. **Construction is builder-mediated.** In Bolo a click does not make
   a pillbox. It orders a builder, who leaves the tank, travels to the
   square, does the work if the material is there, and returns. The
   builder can be killed while out.
2. **Refuelling bases are not pillboxes.** Bases replenish shells, mines
   and armor when the tank drives onto them, and are captured by being
   driven onto. There is no "repair the base" action; the tutorial had
   invented one by confusing base maintenance with pillbox repair.
3. **Range control is a cursor, not a weapon setting.** The player can
   move an aim cursor nearer or farther; most shots do not need it
   because a shell hits the first obstacle; its real use is detonating
   mines at a distance.
4. **Movement has momentum and terrain-limited speed.** Acceleration
   and deceleration take time; swamp and water are slow, roads and
   boats fast; a tank can overshoot a road into swamp.
5. **Driving is keyboard; clicks are for the builder.** The tutorial's
   "click anywhere to control the tank" was wrong for Bolo.
6. **A manual cannot verify code.** Exact update order, whether terrain
   limits velocity or applies drag, and the boat-boarding rule are
   unknowable without the source.

## 2. Method

Each point was compared with three things: the brief as recorded
(`CONCEPT.md` sections 1 and 7), the design (`DESIGN.md`), and the
code as it ran this evening (`source/treadcore-sim/src/lib.rs`,
`map.rs`, `rules.rs`; the trace from `make debug`). Where treadcore
had no answer, one was decided and written into `DESIGN.md` before
any code, which is what `ROADMAP.md` step 3 requires.

## 3. Findings

| audit point | Bolo | treadcore before | treadcore after | verdict |
| --- | --- | --- | --- | --- |
| 1. builder | a unit that leaves the tank on a click, works, returns, can die | CONCEPT.md section 7 left the control model open: a unit the player moves, or an order the tank gives | decided: an order the tank gives (`Command::Deploy { task }`); the squad is state on the tank, walks the world's route to the task, is vulnerable to splash and strikes while out, returns | matches Bolo's model exactly, with the tank as the one thing driven |
| 2. bases | drive on to refuel; drive on to capture; no repair action | depots: a tank inside 2 tiles of a friendly one resupplies every half second; alone on another's for 10 s captures it; two tanks contest (`step_depots`) | unchanged; written down that no Pillboy task touches a depot | already Bolo's; the tutorial's error is kept out |
| 3. range cursor | an aim cursor nearer or farther; shells hit the first obstacle | the gun fires at a map coordinate (`FireIndirect { at }`, 12 to 60 tiles) and lands where aimed plus scatter; direct fire needs line of sight through no forest and hits a tank within 0.8 tiles of the aim point | unchanged | the coordinate aim is the cursor generalised; indirect fire over obstacles is the premise, so "hits the first obstacle" does not apply to it. Direct fire does not stop at the first tank in its path; noted, not changed |
| 4. momentum, terrain | accelerate, decelerate, speed limited by terrain | `TANK_ACCEL` 3 tiles/s^2 to a maximum of `TANK_SPEED / move_cost`: terrain limits the maximum velocity, not a drag; water is impassable, no swamp, no boats | plus a brake: a heading error over 0.6 rad drops the limit to a quarter, so the tank pivots at corners instead of swinging wide of a bridge | matches the documented behaviour; the audit's open question ("limit or drag?") has an answer here because the code is the specification |
| 5. click to drive | keyboard drives; clicks order the builder | left click orders a destination; the world routes it | unchanged; written down as a deliberate divergence | the premise differs: the tank is a mobile command and the player orders it, as the player orders the drones and will order the Pillboys. One interaction model for everything deployed, including the tank |
| 6. code verification | not possible from the manual | every rule is a `pub const` with a sentence in `rules.rs`; every step is `World::step`; `make debug` writes the match as events | unchanged | the audit's complaint about Bolo is a design goal met here: the rules are readable, and the trace shows them running |

## 4. What was done

- `DESIGN.md` gained "Pillboys and pillboxes: the decision": the
  control model, the squad's data (`strength`, `aboard`, `pos`, `task`,
  `progress`, `material`), the four tasks, what a deployed squad risks,
  what depots remain, what pillboxes are (Bolo's, plus a short-range
  gun against drones), and what this must not change.
- `ROADMAP.md` step 3 now points at that decision and at this report.
- Nothing in `treadcore-sim` was written for Pillboys. The roadmap
  orders the replay (step 1) and the world hash (step 2) first, so the
  change to the step function that Pillboys bring can be shown to do
  only what it meant to.

## 5. Conclusion

The requirement is accepted, with the three corrections the audit
made taken as given: Pillboys are Bolo's builder (an order, resolved
over time, under fire, by a squad that can be lost); depots stay
refuelling bases with no repair action; aiming is a coordinate, which
is the range cursor made general. Two divergences from Bolo are
deliberate and are now written down: the player orders a destination
rather than steering, and shells fly over what is between the gun and
the aim point because indirect fire is the point of the game.

The audit's last point is the one worth keeping in mind. It could not
tell, from Bolo's manual, how Bolo's movement really worked. treadcore
can answer the same questions about itself in one file of constants
and one `step` function, and a trace that shows them running: terrain
limits the maximum speed (`TANK_SPEED / move_cost`), the update order
is orders, tanks, drones, shells, depots, swarm, match, and there are
no boats. When the Pillboys are built, the same will be true of them.

## 6. Not done

The Pillboy, pillbox and build code: roadmap step 3, after steps 1
and 2. A playable test of the decision (does ordering a squad feel
like commanding, or like waiting?) needs the window, which this VM
cannot yet open (`BUILD.md`, "The window in this VM").

## References

- The tutorial and its audit, received 12 October 2026 (the audit
  cites the Bolo manual at dialup.party/~jolo/bolo/guides/bolomanual/,
  the PyBolo repository, and the Bolo2K control guide).
- `documentation/CONCEPT.md` section 7; `documentation/DESIGN.md`;
  `documentation/ROADMAP.md` step 3; `documentation/RESEARCH.md`, Bolo.
- `source/treadcore-sim/src/lib.rs` (`apply`, `step_tanks`,
  `step_depots`), `rules.rs`, `map.rs` (`route`).
