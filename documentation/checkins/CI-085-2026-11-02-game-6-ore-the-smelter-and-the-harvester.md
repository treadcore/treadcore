# Game 6: Ore, the Smelter and the Harvester

**Check-in report CI-085.** Paul Richeson, treadcore.org; Claude Code
(Claude Fable 5.1) as the pair. 2 November 2026, evening.
Companion records: `LAB-REPORT-BRAIN.md` section 128; CHANGELOG, game 6;
the plan's chapter 13 (`reports/2026-11-02-the-plan/13-economy-and-logistics.html`),
stage 2; `HOW-TO-PLAY.md` section 6b, the ore; `MAPS.md`, the `deposit` line.

---

## I. Intent

Paul: "continue. implement the economy. commit and push." The plan of
2 November named four stages of a zero-sum economy and built the first
in the afternoon (game 5: a hit pillbox loses its stock and refills from
its depot). This check-in is the second stage, the large one: ore as a
finite thing on the map, a works that turns it into shells, a depot that
no longer makes shells from nothing, and a brain that runs the supply.

## II. What was built

**The simulator (game 6).** `Deposit { id, pos, stock }` on the world
and in the observation; `deposit x y [stock]` in a map file, and
`Map::seed_deposits` for a map that places none (one for every second
start, six tenths of the way to the centre, on open ground, 120 ore).
`Scenario.ore`, on unless said otherwise; `treadcore-server --no-ore`.
On an ore map `DepotStock::make_from(true)` passes over shells and the
refinery's stop gives fuel alone. `PillboyTask::Harvest` on a deposit's
tile mines it (six seconds, four ore, eight aboard, `Event::Mined`
with what is left). A tank within three tiles of a working smelter of
its side unloads (`Event::OreUnloaded`); a smelter makes a shell a
second into the nearest depot of its side within thirty tiles that has
room (`Event::Smelted`). `StructureKind::Smelter` and `::Harvester`
stand at their resource on open ground (`at_resource`,
`works_site_refused`), need no plant, and are built with the works
order like the rest. A harvest within eight tiles of a working
harvester yields six and the tree returns in a minute. The chair
knows the new refusals. `GAME_VERSION` is 6.

**The brain.** `ore_low` (10 shells) and `ore_reach` (40 tiles) in
RANGES; `Commander::ore_run` with `Act::OreRun` (to the deposit, the
squad mines, to the smelter, unload; or first the side's nearest tank
with the material builds the smelter beside the ore,
`Cycle::smelter_site`); the base's guard runs whenever the depot is
short, every other tank once after each fill-up; the cycle's
improvement offers the smelter at `w_works`. A tank at a depot with
nothing to give leaves after ten seconds and does not come home for
shells for a minute (`FILL_PATIENCE`, `DRY_FOR`, `needs_home` takes the
tick). The census ledger counts ore mined and shells smelted a seat
and writes them on the composition line; a scenario's `reward` may name
`ore` and `smelted`.

**The window.** The deposit as a heap that lowers as it is worked out,
labelled `ore N`; the smelter and the harvester as their own voxel
shapes with names on the wire; the ORE lamps beside MATERIAL; a ring
round a working smelter of your side; the wire says what was mined
and unloaded.

**Records.** CHANGELOG; LR 128; HOW-TO-PLAY (the farm and works rows,
the industry section's ore paragraph); MAPS.md; the book's chapter 13
marks stage 2 built and says what was read; orrery reads `ore` and
`smelted` from the composition line (not yet a floor).

## III. What was read

- `the_bots_build_a_smelter_mine_and_smelt` (brain, new): nine seats
  in three teams on the naval map for ten minutes, the fleet's team
  brains: 5 smelters built, 40 ore mined, 9 unloads, 36 shells
  smelted, the longest wait at a dry depot 8 s.
- A two-seed census of 300 s on the generated island: two teams mined
  and smelted 12 and 20; the team that did neither scored between
  them. The economy is not yet the score; that is the trade guard's
  floor to add once the fleet has read game 6.
- Tests: simulator 64 (four new), brain 153 (one new), the workspace
  and its tests compile. Game 5 under `ore: false` still holds.
- An old fault, exposed by the new rule and fixed: the hunter lobbed
  one blind shell at a neutral pillbox its squad was about to take
  (`shell_boxes`), and the depot's free shell replaced it before the
  test looked. A live target keeps the gun now.

## IV. What is not built

Workers as stock in transit, the city's four shapes and the
quartermaster (stage 3); oil, the well and the refinery's source;
trade (stage 4). Map authors' own deposits in `maps/*.map` (the rule
seeds them). A page of the window for the side's stocks. The
thirty-minute shape and the halving of shells a match are the fleet's
censuses to read; every read before this is of the old game.

## V. Paul's decisions

The decrees named in CI-084 stand open (`tree_material` 4,
`strikes_max` 3, `coming_r`). New today: whether a deposit's 120 ore,
a shell an ore, and one smelter a base are the numbers; whether
`ore_low` and `ore_reach` should be decrees or the search's.
