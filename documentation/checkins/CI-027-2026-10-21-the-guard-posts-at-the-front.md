# The Guard Posts at the Front

**Check-in report CI-027.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0084; `LAB-REPORT-BRAIN.md` section 61;
CI-026; roadmap backlog 48, 49, 51.

---

## Abstract

The session first committed CI-026's finished step, after checking its
tests (brain 81, client 32). It then took backlog 51, the rest of
LR 58's hypothesis: a guard should meet an attack before it reaches the
base. Two weights lean the Guard rung toward the team board's front.
`w_front_push` (0.75) moves the rotation's push point that share of
the way toward the front sector nearest it. `w_front_post` (0.5) moves
the guard's post toward the nearest front sector, 6 tiles at most. A new
family, a raid on a guarded base from any side, meets the raider
beyond the base's ring in 85.5% +/- 1.8% of 400 seeds, against 50.5%
+/- 2.5% with the parts at 0. All of that gain comes from the push
point. The family shows the first form of a raid, one that the guard's
drones and a teammate could see, being met 20 tiles out whatever the
post. The guard census reads -0.27 +/- 0.64 a seat over 48 seeds,
inside the noise, with the push point acting in 79% of a rotating
guard's seconds.

**Index terms:** team board, influence front, rotation, guard post,
scenario families, census guard.

---

## I. Introduction

### A. Intent

To pick up where the last session stopped, and go on through the
backlog in the order CI-026 named: 51, then 20, then 48.

### B. The prompt

> resume.

The working tree held CI-026's step whole and uncommitted (journal
0083 said "this entry's commit follows"). It was committed locally as
`c61ea8a` after its tests were run again, and not pushed.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "resume."; CI-026 finished but not committed | Its tests run again, then committed locally, not pushed | Pushing: AGENTS.md, commit or push only when asked; a push is outward-facing |
| D2 | CI-026, next steps: 51 | Backlog 51 | 20 first: CI-026 put 51 ahead |
| D3 | Either/or is a scalar (Paul, 18 October); the post and the push point move by different amounts | Two weights, `w_front_post` and `w_front_push`, rather than the one the backlog named | One weight: the tuner could not tell them apart |
| D4 | The guard must still cover the base | The post moves at most `BASE_ALARM - BASE_R`, 6 tiles | An unbounded lean: the base's ring would leave the leash |
| D5 | The rotation's defence end is where the tank refills | The defence point stays the base's post; only the guard's stand moves | Moving `Commander::post` itself: it moves the refill point, the window's points and the pillbox carry home |
| D6 | F: a brainless raider was not a raid (the RuleBot went for a far depot) | A tank headed at the base (`toward`, new) | A scripted Commander raider: more parts in play than the one under test |
| D7 | F: raids off the map's edge | The base at the map's centre (`"centre"` without a number) | Clamping positions: changes the geometry silently |
| D8 | F: the parts on and at 0 gave the same counts at every bar; the raid met 20 tiles out by shells and drones | The teammate's sight and the drones taken away after half a second; the raid from 40 tiles | Reporting the first family: it measured the drones, not the post |
| D9 | F: the push point does all of it; 0.75 is within two errors of 1 | `w_front_push` 0.75 | 1: the push point on the front itself, farthest from the base |
| D10 | F: the post did nothing in the family | Kept on at 0.5, its use metered | Off: Paul's rule of 20 October, features on, tuning after (48) |
| D11 | LR 58: count before you census | A meter of seconds leaned, read in the first matches (404 to 537 a match) before the census was let run | The census blind |

---

## III. Methods

**Code.** `weights.rs`: `FrontPost` (`w_front_post`), `FrontPush`
(`w_front_push`). `commander.rs`: `front_points` (the front's sector
centres with `Caps::BOARD`, now also used by `expand`); `guard_post`
(the post moved `min(w * d, FRONT_POST_REACH)` toward the nearest front
centre, back to the post if the world refused that tile); `defend_base`
uses it outside an alarm; `think_map` hands the rotation `front` and
`front_w` before `fit`. `rotation.rs`: `fleet_push` a method, the raw
point `base + PUSH_FRAC (objective - base)`, moved `front_w` of the way
to the front centre nearest it. `meter.rs`: `lean(post, moved)` once a
second; `leans` by group in the census line.

**Scenario form.** `"centre"` alone as an anchor; `[[tank]] toward`;
`met_beyond = { by, on, at, more_than }` over a new `Outcome::hit_at`.

**Family.** `scenarios/families/a-raid-on-a-guarded-base.toml`, 400
runs, floor 0.75. Sweeps: `make scenario S=... RUNS=400
SET="w_front_push=0"` and the others in section 61's table; the bar
swept over 8 to 25 tiles at 200 seeds before 10 (`BASE_R`) was chosen.

**Guard.** `build/lr61/run.sh`: a frozen binary; the fair census on
naval.map, 900 s, 3 respawns, seeds 0 to 47 in three jobs of 16; trial
`caps = baseline` (the defaults), baseline the same with both weights
0. The pooled difference is the mean of the 48 per-seed differences,
its error their standard deviation over the square root of 48.

---

## IV. Results

| Measure | Before | After |
| --- | --- | --- |
| The family, met beyond 10 tiles, 400 seeds | 50.5% +/- 2.5% | 85.5% +/- 1.8% |
| The first family (sight and drones), any bar from 12 to 25 | same with or without | same |
| Push point leaned, census seconds | | 23,950 of 30,216 (79%) |
| Guard post leaned, census seconds | | 1,039 of 1,130 (92%) |
| The guard, 48 seeds | | -0.27 +/- 0.64 a seat |
| Ashore dodges hit | 15.8% | 15.7% |

Tests: brain 82 passed and 11 ignored; client 32; net 14; proto 3; sim
50. 100 weights, 17 bits; ten scenario files, six families with floors.

---

## V. Discussion

1. **A raid seen is met far out already.** The guard's shells and
   drones reach past where any post could stand; the post matters only
   when the guard is blind, and a scenario has to take the sight away
   to show it.
2. **The guard is seldom at its post.** Since 17 October an idle guard
   rotates, so the push point is where its time goes, and the push
   point is the part that pays. The post leans for about 8 s a match.
3. **A part that acts and reads zero.** Unlike LR 58's front read, the
   push point acts most of a rotating guard's time; the census still
   reads no gain or loss a point wide. Whether leaning helps a whole
   match is for the tuning phase.

**For Paul:** none blocking. Next: backlog 20 (`island_site` where a
naval start chooses), then 48 (the starting numbers). Commits are
local; nothing was pushed.
