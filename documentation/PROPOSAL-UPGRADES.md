# Proposal: upgrades, and supply to buy them (roadmap 11g)

Status: drafted 18 October 2026 at Paul's request ("think of more base
upgrades and ways for users to purchase them ... write the upgrades
proposal"). Trace 1 of section 6 run 21 October (section 7); a post
for the seven days of discussion (GOVERNANCE.md section 5) drafted the
same day, to be posted by Paul. Accepted, it becomes roadmap step 11g's
build, in the sim, the wire, the window and the brain, in that order.

It changes the rules and the wire, so it is a proposal and not a pull
request. Every number in it is a first guess for the playtest and the
census to move. A number in `rules.rs` is accepted only with a trace or
a playtest showing its effect (section 5, item 3), and section 6 of
this proposal says which traces.


## 1. Change

### 1.1 Supply, a side's second count

A side earns **supply** from what it holds and from what it destroys.
Material is what a squad carries to build with; supply is what a side
has earned to spend on upgrades.

| Source | Supply | Note |
| --- | --- | --- |
| Each live pillbox the side owns | 1 every 10 s | the ledger's `held_s` made a currency |
| Each depot the side owns | 2 every 10 s | a depot is worth two pillboxes |
| Each structure of the side, powered | 1 every 20 s | industry pays a little |
| A hostile tank destroyed | 10 | to the side of the tank that fired the shot |
| A hostile pillbox captured | 5 | once per capture |

Supply is held by the side, not the tank, and an alliance pools
nothing: each side keeps its own. A side's supply is in the
Observation of every member (`Observation::supply`), so a window shows
it and a bot reads it. Over the six naval censuses of journal 0038 a
team held 2 to 6 pillboxes and depots on average, which is about 12 to 50
supply a minute (6 a pillbox, 12 a depot). The first upgrade below costs
60, so it comes into reach between the second and the fifth minute.

### 1.2 Upgrades

Each upgrade has a supply price for each level and, if it is built, a
material price. A base upgrade applies to one depot of the side and to
its castle, the pillboxes and walls within `BASE_R` 10 tiles of it. A
tank upgrade applies to the buyer's tank until it is destroyed; a
respawned tank starts plain, so death costs something.

| Upgrade | Kind | Levels | Effect | Supply per level | Material | Installed |
| --- | --- | --- | --- | --- | --- | --- |
| Hardened walls | base | 1 | the castle's wall tiles become concrete | 80 | 1 a tile | by the squad, a tile at a time |
| Pillbox calibre | base | 2 | the base's pillboxes reach 9, then 10 tiles (8 now) | 60, 120 | none | at once |
| Pillbox plating | base | 2 | the base's pillboxes 75, then 90 health (60 now) | 60, 120 | none | at once; damage taken keeps its share |
| Watchtower | base | 1 | 11j's flag: the base sees 12 tiles round its pedestal | 100 | 4 | by the squad, on a free tile inside the wall |
| Radar mast | base | 1 | the base's alarm at 24 tiles (16 now); shells in flight at it shown | 80 | 2 | by the squad |
| Gatehouse | base | 1 | the gate tiles pass the side's tanks and stop all others | 120 | 2 a gate tile | by the squad |
| Depot stores | base | 2 | the depot restocks twice as fast; then the squad's material to 12 there | 60, 120 | none | at once |
| Yard workshop | base | 1 | pillboxes and walls inside the wall mend 1 health a second | 100 | a workshop's | by the squad, on the yard |
| Minefield | base | 1 | mines within 10 tiles of the base cost nothing | 60 | none | at once |
| Dock | base | 1 | a boat built within 10 tiles of the base costs 3 material (5 now) | 60 | none | at once |
| Bridge works | side | 1 | a bridge tile beside the side's road costs half a material, rounded up a pair at a time | 80 | none | at once |
| Supply road | side | 1 | the side's tanks drive 25% faster on the side's own road tiles | 100 | none | at once |
| Plating | tank | 2 | armour 110, then 120 (100 now) | 50, 100 | none | at a depot of the side |
| Bay | tank | 2 | one more of each drone, then two | 50, 100 | none | at a depot of the side |
| Squad | tank | 1 | a fifth Pillboy (4 now) | 80 | none | at a depot of the side |
| Hopper | tank | 1 | the squad carries 12 material (8 now) | 60 | none | at a depot of the side |

There are twelve upgrades for a base or the side, and four for the
tank. The side upgrades (bridge works, supply road) are bought once
and apply wherever the side has roads.

### 1.3 Buying

- **The command:** `Command::Buy { upgrade: Upgrade, at: Option<EntityId> }`, where `at` is the depot for a base upgrade.
- **What the world checks:**
  - the side has the supply;
  - the level below is held;
  - the buyer is alive;
  - for a base upgrade, the depot is the side's and the buyer is within `DEPOT_RADIUS` + 6 of it;
  - for a tank upgrade, the buyer is on a depot of the side.
- **Refusals:** a refusal names a chair tag like any other (`every_refusal_has_a_name`). For example, "PICNIC: supply 40 of 60".
- **Events:**
  - a purchase that needs no installing applies on the tick it is bought (`Event::Bought`);
  - one that is installed adds a site to the squad's work (`PillboyTask::Install { upgrade, at }`) and applies when the squad finishes (`Event::Installed`);
  - an install left half done keeps its progress, as a build does.
- **The window:** the coordination screen gets a works tab: the side's supply, each upgrade with its level, price and where it can be bought, a key to buy. The HUD shows supply beside material.
- **Bots:**
  - A fleet commander's Commander buys for its side through the base cycle's improvement slot (`Cycle::improvement`). The slot weighs upgrades against mending, mines and works by `upgrade_vs_mend`.
  - Among upgrades, each is scored by the team's posture (board.rs): Defend favours calibre, plating and the radar mast; Connect favours the supply road, bridge works and the dock; Economy favours depot stores and the hopper; Fortify favours hardened walls, the gatehouse and the watchtower.
  - Each upgrade's worth is a weight (`w_up_<name>`), so the tuner can learn the bots' buying.
- **Allies:** a person's ally can hand supply to the person (11e's transfers, `Command::Give { supply }`), so a fleet can fund its leader.

### 1.4 What changes where

| Crate | Change |
| --- | --- |
| treadcore-sim | `rules.rs`: the prices and effects above. `lib.rs`: `Upgrade`, `Command::Buy`, `PillboyTask::Install`, a side's supply and upgrades in `World`, earning in `step`, the effects in range, armour, speed, restock and cost, `Event::Bought` and `Event::Installed`, the chair tags. |
| treadcore-proto | Wire version 17: the new command, task, events and observation fields. |
| treadcore-net | Nothing but the version. |
| treadcore-client | The works tab, supply on the HUD, sounds for a purchase. |
| treadcore-brain | `Cycle::improvement` weighs upgrades; `w_up_*` weights; the posture's leaning. |
| treadcore-server | `--no-upgrades` for a match of the old rules, and the match report's purchases. |
| documentation | DESIGN.md (the numbers and why), HOW-TO-PLAY.md (supply and the works tab), PROTOCOL.md (wire 17), the lab report of the step. |


## 2. Why

- **The matches are short.** They end in 8 to 16 minutes by elimination (journal 0036, item 7), and Paul asked for longer ones. Today a side that holds ground earns nothing for it beyond the ground itself. Supply makes holding pay over time, and upgrades make an older base stronger than a new one, so a defended base lasts and a match runs longer.
- **The base cycle needs something to spend on.** It has an improvement slot (roadmap 12f1, 12h), and today only mending, mines and works fill it.
- **11g was open.** It waited on one question, in-match or carried between matches. This proposal answers in-match; a veteran's list across matches is left for later.
- **Players asked for something to buy.** Paul asked for "base upgrades and ways for users to purchase them". The window has nothing to buy today.


## 3. Must not change

- **The simulation stays deterministic.** Supply and upgrades are world state: hashed, in the replay, and the same on every machine.
- **Old replays still check.** A replay recorded on wire 16 checks under the old rules. `Scenario::upgrades` is false when absent, as `industry` is, so an old scenario plays as it did.
- **Nothing changes until a side buys.** A match where nobody buys plays as it does today, number for number. The earning itself changes nothing else.
- **No pay-to-win across matches.** Nothing is carried from one match to the next.
- **The bots' baseline holds.** The brain of 18 October, with `upgrade_vs_mend` at 0, buys nothing, so every census before this step can be run again.


## 4. Proof

**Sim tests:**
- `supply_is_earned_from_what_a_side_holds`: 60 s with 2 pillboxes and a depot gives 24 supply.
- `an_upgrade_is_refused_without_the_supply` (named, PICNIC).
- `calibre_reaches_further`: a tank at 9.5 tiles is shot after one level, and not before.
- `plating_keeps_the_share_of_damage`.
- `the_gatehouse_lets_the_side_through_and_no_one_else`.
- `a_tank_upgrade_is_lost_with_the_tank`.
- `a_replay_with_purchases_checks`.

**Brain:**
- `a_fleet_commander_buys_by_its_posture`.

**Censuses:**
- the brain census with `upgrade_vs_mend` at 0 and at 0.5, read paired against a null run (journal 0038);
- match length and supply spent, over twelve seeds.

**Playtest:** two windows on the naval map, one side buying and one not, with the match report's purchases read.


## 5. Open for the discussion

- Earning from structures: in or out? It favours the industry map.
- Kills paying supply: does it reward the side already winning? A smaller bounty (5), or none?
- Prices are the same for every side. Should a side that is behind pay less, as a catch-up?
- The gatehouse needs walls a side's tanks can cross. That is a new kind of tile (`Terrain::Gate`). Is it worth its own small step first?


## 6. The traces this needs before acceptance

1. A bot-only match of 16 seats on naval.map, 30 minutes, with supply counted and nothing bought. This shows the earning rate and when the first upgrade comes into reach.
2. The same match with bots buying at `upgrade_vs_mend` 0.5. This compares match length, purchases by kind, and kills and deaths against the first.
3. A playtest of the works tab, for whether the prices feel right.


## 7. Trace 1, measured (21 October 2026)

The proposal's earning, paid from outside the world, in today's
matches (`fleet::tests::supply_trace`, ignored): the default brain in
all sixteen seats, naval.map, 3 respawns, 30 minutes at most, six
seeds. Nothing in the world reads the count, so these are today's
matches, number for number (section 3).

| Measure | Proposal's guess (1.1) | Measured |
| --- | --- | --- |
| Supply a team-minute | 12 to 50 | 48.8 on average; 7.3 to 92.5 by team |
| The first 60 | between the second and fifth minute | every team (18 of 18), median 97 s |
| By 5 minutes | | 98 to 284 a team |
| By the match's end | | the winner 1,040 to 1,770; a side out early 134 to 196 |
| Match length | 8 to 16 min | 18.0 min on average (16 to 23); none reached 30 |

| Source | Share of all supply |
| --- | --- |
| Depots (2 every 10 s) | 60% |
| Pillboxes (1 every 10 s) | 21% |
| Kills (10) | 19% (81% in the first two minutes) |
| Hostile pillboxes captured (5) | under 1% |
| Powered structures (1 every 20 s) | 0% (naval.map has no industry) |

**What it says, for the discussion:**

1. **Supply comes too early and too much.** Every base and side
   upgrade at every level costs 1,320; a winning side has 870 to 1,300
   by the 15th minute, and the one match that ran past the 20th
   passed it (1,442). Suggested: halve every earning rate (a pillbox 1 every 20
   s, a depot 1 every 10 s), which should put the first 60 near the third
   minute and the full list out of reach in an ordinary match.
2. **Kills are the opening's income** (open question 2): 81% of the
   first two minutes. A bounty of 5 keeps a kill worth something
   without paying the first upgrade for the first fight.
3. **The leader's supply runs away** (open question 3): in three of
   six matches one side earned under a sixth of the winner's, and a
   side reduced to its last tank earns nothing. Untested, but likely:
   upgrades bought with it would harden the leader most. A catch-up price
   (cheaper for the side with the least supply) has evidence for it.
4. **Structures pay nothing on the naval map** (open question 1): the
   question is the industry map's alone.

Trace 2 (bots buying at `upgrade_vs_mend` 0.5) needs the build: a
prototype behind `Scenario::upgrades`, off by default, is the way to
run it before acceptance without changing a default match.

