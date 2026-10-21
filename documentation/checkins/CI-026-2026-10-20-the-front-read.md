# The Front Read, the Families Held to Floors, and the Crews Tested

**Check-in report CI-026.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journal 0083; `LAB-REPORT-BRAIN.md` sections 58 to
60; CI-025; roadmap backlog 26, 47, 49 to 52.

---

## Abstract

Four backlog items, in order. The team board's front, built every
second since LR 24 and read by nothing, now leans the Expand rung's
choice of depot (`w_front`, 0.5): a depot's cost is half its distance
to the base and half its distance to the front, and the pick is kept
until taken. The meter showed the front known at only 2.5 of Expand's
picks a match, since the board counts only hostiles in sight, so the
board now remembers hostile tanks for `front_memory` (30) seconds, for
the front alone: 3.2 picks a match with a front, 14% without. A
scenario holds with the part and fails at 0; the guard census reads
-0.14 +/- 0.42 a seat over 48 seeds. Each scenario family now carries a floor, and `make families`
runs all five at 1,000 seeds in 31 s; each falls under its floor with
its part broken. Allies' goals on the coordination screen were found
already shown by a hosting window and are now proven by a test; a
joining window needs the wire. The crews' hand-out and the hop's bridge
choice have tests of their own, and 22 functions got doc comments
(121 undocumented to 99).

**Index terms:** team board, influence front, sighting memory, guard
census, scenario floors, fleet command.

---

## I. Introduction

### A. Intent

To go on through the backlog after CI-025, each item proven by a test
that fails without it, and the testing made faster where it can be.

### B. The prompt

> read backlog and roadmap. and approparate documenttation to remind
> yourself of the code goals. continue working on backlg itmes. and
> continue using testing to verify improvements and additions work as
> directed, you can also run at faster spaeed.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "continue working on backlg itmes"; CI-025 named 49 next | Backlog 49 first, Expand's depot only; the guard posts split off as 51 | Both at once: one step at a time (Paul, 18 October) |
| D2 | Paul's rule of 20 October | `w_front` on at 0.5, the blend's middle; old method at 0 | A sweep first: tuning follows the features |
| D3 | Scenario Commanders had no board | `board = true`, `rungs`, `[[depot]] at`, `depot_first` in scenario.rs | A Rust test: scenarios are the form for new behaviours (LR 48) |
| D4 | F: the hostile 36 tiles off was never seen, so no front | A teammate placed to see it, as a team shares sight | Putting the hostile in the tank's own sight: the case is the team's picture |
| D5 | F: the tank turned back at 4% of a capture when its arrival erased the front | The pick kept until taken or given up, with `w_front` on | Hysteresis on the cost: one more number, and the flip is the front's, not the cost's |
| D6 | F: 2.5 picks a match with a front known, 60 matches in | The census stopped; `front_memory` added | Finishing it: it could only read zero |
| D7 | LR 35: the board forgets each second | Memory for the front only (`known`), the influence, threat, points, posture and rotation as before | Remembering in the influence: it moves five parts at once |
| D8 | "you can also run at faster spaeed" | Families at 1,000 seeds (6 ms each); the census frozen and in three jobs, other steps worked beside it | More census jobs: four cores |
| D9 | Backlog 47 | `runs`, `floor`, `make families`; floors 99% for the dodge families, 80% for the ready gun | Floors at today's rate: one seed's luck would fail them |
| D10 | Backlog 50: the hosting window already shows FLEET lines | A test of it; the joining half to 52 | A wire change: version 16 is fixed under goal 4 |
| D11 | Backlog 26 | Tests for `FleetCommand::think` and the hop's choice; doc comments for crew.rs, hop.rs, board.rs | All 121 doc comments: 22 now in the files touched |
| D12 | F: the hop's priced test sailed at 7 tiles on a bare map | Forest put behind the tank; the scarcity rule written into the test's comment | Changing the price: the rule is LR 6.2's, the test was wrong |

---

## III. Methods

**Parts explored.** board.rs: `TeamBoard::rebuild` (the influence, the
new `seen` and `known`), `front` (now on `known`), `front_dist`,
`sector_centre`. commander.rs: `Commander::expand` (the open depots,
the nearest, the kept target, the blended cost), the ladder's holds
(`auto`), `defend_base` and `post` (read, for 51). weights.rs: `Front`,
`FrontMemory`. meter.rs: `fronts`, `front`, `add_fronts`. scenario.rs:
`run_seeded` (boards per team, `rungs`, depots moved, `depot_owned`),
`check` (`depot_owned_by`, `depot_first`), `every_family_holds_its_floor`.
crew.rs: `FleetCommand::think`, `failed`, `job_tile`, `job_for`. hop.rs:
`Hop::send`, `think` (the bridge before a boat), `bridging`, and
facts.rs `crossing_prices`, `bridge_it`, `scarcity`. The client:
goal.rs `Pilot::screen`, session.rs `step_host` (the FLEET lines).

**Scenario.** `an-expanding-tank-takes-the-depot-at-the-front.toml`,
run by `make scenario S=... [SET="w_front=0"]`; in `make test` with
every file of `scenarios/`.

**Families.** `make families`, and `SET="dodge_plan=0 duck_plan=0"`
and `make scenario S=... RUNS=1000 SET="dodge_release=0"` for the
breaks.

**Guard.** `build/lr58/run.sh`: a frozen binary; the fair census on
naval.map, 900 s, 3 respawns, seeds 0 to 47 in three jobs of 16; trial
`caps = baseline` (the defaults), baseline the same with `w_front = 0`
and `front_memory = 0`. The stopped first run (no memory) is in
`build/lr58/no-memory`.

---

## IV. Results

| Step | Measure | Before | After |
| --- | --- | --- | --- |
| LR 58, `w_front` | the scenario | nearest depot first | the depot at the front first; fails at 0 |
| | fresh picks with a front, a trial match | 2.5 (no memory, 60 matches) | 3.2, 14% with none (memory, 144 matches) |
| | the guard, 48 seeds | | -0.14 +/- 0.42 a seat over 48 seeds |
| LR 59, floors | five families, 1,000 seeds | run by hand | 31 s, each under its floor broken |
| LR 59, allies' goals | client test | none | `the_screen_shows_an_allys_rung_and_target` |
| LR 60, tests and docs | brain tests; undocumented | 78; 121 | 81; 99 |

Tests: brain 81 passed and 11 ignored; client 32; net 14; proto 3; sim
50. 98 weights, 17 bits; ten scenario files, five families with floors.

---

## V. Discussion

1. **A front of what is in sight is a flicker.** The board reads one
   member's observation a second; LR 35 found it for the landfall, and
   the front needed the same memory.
2. **Count before you census.** Expand picks about four depots a
   match. The meter said in the first minutes of a 45-minute census
   that the part barely acted.
3. **Floors make families a regression suite.** Five families in 31 s
   now stand guard over the reflexes the census cannot resolve.

**For Paul:** none blocking. Next: backlog 51 (the guard posts toward
the front), then 20 (`island_site` where a naval start chooses).
