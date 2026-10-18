# The brain: lab report and design

A lab report on the bot AI (`treadcore-brain`) as it stood on 18
October 2026, the research it was reviewed against, and the design it
is being rebuilt to: one weighted, tunable decision pattern on shared
team memory, memory and compute first. The same report, with its two
diagrams drawn, is the shared document
[Treadcore: Status Update & Brain AI Lab Report](https://claude.ai/code/artifact/97f399bf-9abe-4094-a060-6a9666f273bd);
this file is the copy that lives with the code. The work it plans is
step 12 of `ROADMAP.md`.

**How to read it now.** Sections 1 to 10 are the review and the design
of 17 and 18 October. Section 2, the brain as it stood then, is
**archived in place**: it is the baseline every later number is
measured against, and the improved prototype has replaced it. Sections
11 to 19 are the record of building the design, a step and a census
at a time. Section 20 is the brain as it stands now. Which old methods
were archived, and which still run with an improved challenger
switched off, is the register `archive/BRAIN-METHODS.md`.

Five conclusions, numbered so the rest can cite them:

- **C1.** The brain is a stack of first-match ladders with 22 tunable
  weights, all in combat; each tank decides alone, on a full copy of
  the world, twenty times a second (section 2, findings L1 to L8).
- **C2.** Step one is memory: static map facts, one shared team board
  per team at 1 Hz, a small fixed mind per tank; parameters as one
  flat `Weights` array and `Caps` bits; thinks staggered by layer
  (section 5). Nothing else lands before this and a measured
  baseline.
- **C3.** One scoring pattern for every decision: features, response
  curves, weighted products, a confirming selector, a held intent.
  Behaviours plug in as components that can be switched off and
  tuned (section 4), after the utility, Killzone 3 and Spring designs
  of section 3.
- **C4.** The behaviours asked for are components (sections 6, 7, 9):
  fill up, give the base one item and one improvement, road out, and
  escort pillboxes home along the road; a small island is a bad base
  and a priced crossing picks boat or bridge (6 tiles or fewer
  preferred, 20 at most); column on a bridge; desire vectors that need
  several thinks of confirmation, except retreat and dodge; forts,
  fronts and rotation across the team.
- **C5.** Every match records each tank's weights, capabilities and
  seed beside its result; ratings and an offline evolution strategy
  find better weights, and fair fleet roles, by measurement (section 8).


## 1. Method

Read: `source/treadcore-brain/src` (8,458 lines on 18 October:
commander.rs 2,966, lib.rs 2,150, hop.rs 766, fleet.rs 719, tactics.rs
562, fortify.rs 538, crew.rs 486, rotation.rs 271) and the sim's
`observe` and routing. The key claims were checked against the source.
Nothing was run for the report: every number in sections 2 and 6 to 9
is a code constant or a starting guess, never a measurement. The
measurement is step 12a.


## 2. The brain as it stood on 17 October (archived)

> **Archived 18 October 2026: the baseline.** This is the brain step 12
> started from, kept as the development record and as the baseline the
> census measures against. The brain now is section 20; what replaced
> each method here, and what is still current, is
> `archive/BRAIN-METHODS.md`. Findings L1 to L8 keep their numbers,
> since the rest of the report cites them.

### Who runs, how often

| Level | Code | Rate | Output |
| --- | --- | --- | --- |
| Fleet commander | `crew::FleetCommand` in the Leader's or Builder's Commander | 20 Hz; musters every 30 s | `Request{Muster, Build, Fetch, Haul}` events, pings |
| Commander, one per tank | `commander.rs` `think_map` | 20 Hz | `Vec<Command>` |
| Auto ladder | `auto()`: DefendBase > Fortify > Turrets > Hunt > Expand > Guard | 20 Hz | a goal |
| Wingman | `fly_wing`: Defend > Ahead > Pillbox > Depot > Help > Regroup | 20 Hz, 3 s commit | a Move to a task or wedge slot |
| Doctrine, the inner RuleBot | `lib.rs` `think_map` | 4 Hz (rookies 2 Hz) | drones, bubble, gun; its Moves dropped by `keep_inner` |
| Tactician | `tactics.rs` | each doctrine think | target score, Lanchester odds (ours squared over theirs squared), Mass/Engage/Press, kite |
| Hop | `hop.rs` | each doctrine think | bridge (at most `BRIDGE_MAX` 20 water tiles), boat, landfall |
| Plan, Rotation, Crew | `fortify.rs`, `rotation.rs`, `crew.rs` | 20 Hz | next build item, push and defence points, the job |
| Sim router | `Map::route_by`, A* on 128 x 128 | per Move | a route kept until arrival or Stop |

```
Fleet commander (Leader or Builder)
   |  one-tick events: Request, Pinged                        (L5)
   v
Commander, each tank, 20 Hz, ~17 branches, first match wins   (L1)
   |  <-- observation: a full world copy per bot per tick --   (L2)
   |  --> commands --> sim router, A* per Move
   +-- Auto ladder (6 rungs)
   +-- Wingman (3 s commit)                                   (L7)
   +-- Crew job (Build / Fetch / Haul)
   +-- Rotation, fortify plan
   +-- Doctrine, 4 Hz (its Moves dropped)
          +-- Tactician (22 weights)
          +-- Hop (bridge <= 20, failures not cached)         (L4)
          +-- Dodge (indirect shells only)
```

The Commander's branches, in order, first return wins: downed, dodge,
revive, bridge help, shore gather, afloat, chain help, pings and
rally, fleet, crew job, leader takeover, scoot, `needs_home`, goal
dispatch, doctrine guns and drones, alliances.

### Findings

- **L1. Decisions are first-match ladders.** Weights exist only in
  `Tactics` (22 coefficients); the Auto ladder and the wingman's
  priorities are fixed orders with on/off rungs.
- **L2. Every bot gets a fresh copy of the world every tick.**
  `World::observe` clones every depot, pillbox, structure, player name
  and visible tank (route `Vec` and all) per bot, 20 times a second,
  and runs two 41 x 41 terrain scans with sorts; RuleBot and Defender
  think at 4 Hz.
- **L3. Repeated work.** `auto()` cloned the whole `Plan` and ran
  `add_turrets` on the copy every tick for two yes/no answers;
  `Tactician` built a `Vec<String>` of reasons per candidate; six
  `why` strings and a `doing` string were formatted every tick.
- **L4. A failed bridge search was never cached.** `Map::crossing` is
  a Dijkstra with no heuristic over 65,536 states and 768 KB of
  buffers per call, rerun at 4 Hz while a crossing is over 20 tiles or
  impossible.
- **L5. No shared team memory.** Push points, odds and assignments are
  recomputed per tank; coordination rides on one-tick events, and a
  missed event is a missed job.
- **L6. Lists that never forget.** `given_up`, `depots_given_up`,
  `revive_failed`, Hop's `bad_*` and `no_bridge`, the fleet's `fails`,
  the Defender's `*_tries` grow for the match and are searched
  linearly; what they hold is never retried.
- **L7. Uneven commitment.** `go()` re-issues a Move only on a 1-tile
  shift, 4 s or a stop, but targets that follow a moving thing (wedge
  slots, rallies, kite step-ins, chain help) re-route nearly every
  tick; hysteresis exists in patches (`WING_COMMIT` 3 s, `w_stick`
  0.4, landfall 6 points over 2 s, `ALARM_HOLD` 6 s) with no common
  rule.
- **L8. The economy is reactive.** Trees are felled only when a build
  is short, the squad holds 8; nothing fills up ahead of need, so a
  starved plan turns Fortify off. Rotation balances one tank (refill
  to 90%, at most 25 s), never the team.

Also: two resupply latches that disagree (doctrine under 6 shells,
Commander at 4 shells or 35 armour) and the doctrine's can take an
Auto tank over through `hop.send`; direct fire is never dodged before
it lands; a Hunt target given up after 12 bearings is given up for
good; `partial_cmp().unwrap()` would panic on a NaN.


## 3. Research review

Every source here was opened and read (the page, the paper or the
source code); what could not be confirmed is marked unverified.

| Source | Takeaway | For us |
| --- | --- | --- |
| [Baritone ActionCosts](https://raw.githubusercontent.com/cabaletta/baritone/master/src/api/java/baritone/api/pathing/movement/ActionCosts.java), [Settings](https://raw.githubusercontent.com/cabaletta/baritone/master/src/api/java/baritone/api/Settings.java) | Costs in ticks; placing a block costs `blockPlacementPenalty` 20 (about 4 blocks of walking); re-used path costs half, so routes don't flip-flop | bridge or detour as one priced search (6.2); discount the old path (7) |
| [mineflayer-pathfinder](https://github.com/PrismarineJS/mineflayer-pathfinder) | `remainingBlocks` in each search node; a place-move impossible at 0; pluggable cost adders; 40 ms per tick | carry material in the crossing search |
| [Voyager](https://voyager.minedojo.org/) (2023) | LLM-written skills, curriculum, skill library | too heavy at runtime; the curriculum suits offline tuning |
| [Terraria Fighter AI](https://terraria.wiki.gg/wiki/Fighter_AI) | ~132 hand-written styles, no planning; stuck means back up and retry | only a stuck detector, which route refusals give us |
| [Robertson & Watson, A Review of RTS Game AI](https://www.cs.auckland.ac.nz/research/gameai/publications/Robertson_Watson_AIMag14.pdf) (2014) | multi-scale control is the open problem; fixes are a blackboard and a commander-manager-unit chain | the TeamBoard and one arbiter for trees (L5) |
| [Straatman et al., Killzone 3 multiplayer bots](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter29_Hierarchical_AI_for_Multiplayer_Bots_in_Killzone_3.pdf) | commander, squad, individual over an area graph; a plan replaced only by a higher one; influence smoothed in space and time | the template for fleets; islands as areas (9) |
| [Lewis, Choosing Effective Utility-Based Considerations](http://www.gameaipro.com/GameAIPro3/GameAIPro3_Chapter13_Choosing_Effective_Utility-Based_Considerations.pdf) | the Infinite Axis Utility System: normalised inputs, curves, multiplied, early exit on zero; commitment bonus, runtime and cooldown against oscillation | the core of section 4 |
| [utilityworlds](https://uintel-ecs.utilityworlds.com/Documentation/TipsAndTricks/Decisions/) | nine considerations at 0.9 multiply to 0.39, so compensate; momentum 1.1 to 1.25 (exact compensation formula unverified) | the geometric mean in 4; m in 7 |
| [Graham, Introduction to Utility Theory](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter09_An_Introduction_to_Utility_Theory.pdf) | curve families; priority buckets | survival above combat above economy |
| [Humphreys, Exploring HTN Planners](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter12_Exploring_HTN_Planners_through_Example.pdf) | compound tasks; re-plan only for a higher method | multi-step jobs (6.1) |
| [Orkin, Three States and a Plan](https://www.gamedevs.org/uploads/three-states-plan-ai-of-fear.pdf) (2006) | costed actions, A* planning; squads re-clustered by nearness | re-cluster fleets; full GOAP is more than we need |
| [Hagelbäck & Johansson, Multi-agent Potential Fields](https://aamas.csc.liv.ac.uk/Proceedings/aamas08/proceedings/pdf/paper/AAMAS08_0269.pdf) (2008) | tank battles; a field peaks at the attacker's maximum shooting distance; own units repel | local stand-off points, as our kite ring |
| [E323AI](https://github.com/Error323/E323AI) (Spring) | income and use as EMAs; stall under 10% of storage, request under 50%, exceed over 90%; toggles at 30/70; build spot = distance + 1000 x threat; threat per domain | trees as a flow; 30/70 bands; land and water threat apart |
| [FAF Supreme Commander AI](https://github.com/FAForever/fa) | base and platoon templates; builders with priority and conditions, re-checked every 13 s | a mission launches when its platoon is full |
| [Shard](https://shard.tomjn.com/behaviours/) | each behaviour reports a priority, the highest wins | the component pattern |
| [Pottinger, Coordinated Unit Movement](https://www.gamedeveloper.com/programming/coordinated-unit-movement), [Implementing](https://www.gamedeveloper.com/programming/implementing-coordinated-movement) (1999) | unit priorities, the lower yields; break, pass and re-form past an obstacle | column on a bridge (6.3); the explicit column is game practice, not named there |
| [Emerson, Flow Field Tiles](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter23_Crowd_Pathfinding_and_Steering_Using_Flow_Field_Tiles.pdf) | A* over sector portals, a flow field per sector | sector A* for the strategy layer; flow fields not needed at 16 tanks |
| [Churchill, Saffidine & Buro](https://cdn.aaai.org/ojs/12527/12527-52-16049-1-2-20201228.pdf) (2012) | NOK-AV focus fire; LTD2 = sum of sqrt(hp) x dpf | focus fire; spread damage is worth more, the case for team rotation (9) |
| [Uriarte & Ontañón, Combat Models](https://arxiv.org/pdf/1605.05305) | Lanchester's square law as a cheap evaluation | confirms our odds |
| [UAlbertaBot Squad.cpp](https://raw.githubusercontent.com/davechurchill/ualbertabot/master/UAlbertaBot/Source/Squad.cpp) | simulate, retreat if losing, 100 frames before re-attacking | confirmation for the squad (7) |
| [Liu, Louis & Ballinger, Evolving Effective Micro](https://www.cse.unr.edu/~simingl/papers/publish/CIG2014IMPFMicro.pdf) (2014) | 14 parameters, GA population 20 over 30 generations beat UAlbertaBot at kiting | a small weight vector evolves within a census budget (8) |
| [Avery & Louis, Coevolving Team Tactics](https://www.cse.unr.edu/~sushil/pubs/newestPapers/2010/cec/PID1296853.pdf) (2010) | naval capture-the-flag on islands; the team as one chromosome | fleet role sets as one genome |
| [Lucas et al., Planet Wars tuning](https://arxiv.org/abs/1901.00723) | win/loss tuning is very noisy | fixed seed sets, many matches |
| [TrueSkill](https://www.microsoft.com/en-us/research/project/trueskill-ranking-system/) | mu 25, sigma 25/3, rank by mu - 3 sigma | rate genomes in leagues |

Not opened: the Ontañón et al. 2013 survey (blocked, so its time-scale
split is unverified), the Spring wiki AI pages, KAIK, BA AI, the
Killzone 2 GDC paper, CLOP, MineRL.


## 4. Design: a weighted decision graph of components

One scoring pattern at the team and unit levels:

```
Team brain, 1 Hz (posture, jobs, defence points)       Weights and Caps
        | posture, threat                              (sliders, bits)
        v                                                    |
Features, per tank --> Considerations --> Action scores, 4 Hz <-+
(32 values, 0..1)      (curves C(x))      U = W x prod C, masked by Caps
                                                 |
                       Intent (held)  <------ Selector (beat by 15%
                             |                 for 2 thinks)
                             v
Survival bypass ====> Reflex, 20 Hz -------> Commands to the sim
(dodge, retreat)      (desire vectors,              | match result
                       fire gate)                   v
                                              Ledger and tuner --> better weights
```

The score of an action a:

```
U(a) = W[a] * cap[a] * P[posture][a] * (prod_i C_i(x_i))^(1/n) * (1 + m * [a is current])
```

W[a] is the action's slider (0 to 2, default 1); cap[a] is 0 or 1
from `Caps`; P is the team posture's multiplier for the action's
component (section 9); the geometric mean keeps a five-consideration
action comparable with a two-consideration one (compensation in its
simplest form); m is the commitment bonus of section 7. A curve C(x)
is linear, quadratic or logistic with four parameters, and those are
sliders too, so one flat `Weights` array holds every number a tuner
may touch.

A component is a trait object that does not allocate:

```rust
trait Component {
    const CAP: Cap;
    fn actions(&self, f: &Features, board: &TeamBoard, out: &mut ArrayVec<Action, 16>);
    fn score(&self, a: &Action, f: &Features, w: &Weights) -> f32;
    fn emit(&self, a: &Action, mind: &mut TankMind, out: &mut Vec<Command>);
}
```

| Component | Caps bit | Actions | Main considerations | From |
| --- | --- | --- | --- | --- |
| Combat | `KITE`, `FOCUS` | attack, kite, mass, press | odds, target score, range, shells | `Tactics`, its 22 weights as they are |
| Survival | always | dodge, retreat, revive | eta, armour, shells | `Dodge`, `needs_home` |
| Economy | `FILL` | fill, give item, give improvement | material, plan gap, forest near | `Plan::next`, fell |
| Roads and crossings | `ROADS`, `BRIDGES`, `BOATS` | lay road, bridge, boat | priced crossing (6.2), supply distance | `hop.rs` |
| Fortify | `FORTIFY`, `TURRETS` | castle item, turret slot, yard | plan state, quiet | `fortify.rs` |
| Haul | `HAULS` | fetch, tow, escort | road to pillbox, crews free | `crew.rs`, `help_chain` |
| Formation | `WING` | wedge slot, column, wing task | leader distance, bridge ahead | `fly_wing` |
| Rotation | `ROTATE` | push point, defence point, swap | team armour spread, point threat | `rotation.rs` |

The team level uses the same pattern with postures for actions. The
reflex layer is not scored: fixed rules at 20 Hz are cheaper, and
survival should not hang on a slider.


## 5. Step one: memory, parameters, compute

Work each fact out once per team per think, keep it in a fixed-size
board, and lend it to every tank by reference.

| Memory | Lives | Holds | Size |
| --- | --- | --- | --- |
| `MapFacts` | the match; updated when a tile changes | island id per tile (`u16`); the island table (area, forest, shore, pads); the crossing cache `(from, to) -> Option<tiles>`, failures included (L4) | ~40 KB |
| `TeamBoard` | per team, rebuilt at 1 Hz | 16 x 16 sector grids of 8 x 8 tiles (threat, strength, territory, forest left); the job board (persistent jobs, L5); the defence-point roster; the road graph | ~8 KB |
| `TankMind` | per tank | the intent and its score, desire vectors, confirmation counters, recent memories with expiry ticks (L6) | under 1 KB |

Parameters: `Weights([f32; N])` named by a `W` enum and lent as
`&Weights`; `Caps(u64)`, one bit per capability, a capability off is
never scored and costs nothing; `Features([f32; 32])` per tank per
think, read by pure scorers `fn(&Features, &Weights) -> f32`. The
Observation stops being a per-bot copy of the world: one fog-filtered
snapshot per team per tick, or a borrowed view with a vision mask.

Scratch: each bot reuses its buffers (A*, crossing); status strings
become codes formatted only when the HUD asks; the plan answers
`turret_slot` and `yard_left` without a clone (L3).

| Layer | Rate | Staggered by | Work |
| --- | --- | --- | --- |
| Reflex | 20 Hz | none | dodge, retreat trigger, fire gate, desire vectors, hold the command |
| Unit | 4 Hz | `tick % 5 == seat % 5` | score ~12 actions, choose the intent, at most one Move |
| Team | 1 Hz | `tick % 20 == team * 6 % 20` | rebuild the board, post jobs, assign roles and points, pick a posture |
| Strategy | 0.2 Hz | one team a second | sites, road plans, the front, crossings (cached) |

Measure first: per-layer timers and counters in the census runs
(thinks, Moves, A* calls, crossing calls, allocations), reported as
microseconds per bot per tick, before and after each change.


## 6. Behaviours to add

### 6.1 The base cycle (L8)

1. **Fill.** Shells and armour at the depot; the squad to its 8
   material from the forest nearest the base, from the board's forest
   grid.
2. **Give one item.** One plan item at the base, chosen by utility,
   so the plan never starves while the fleet is out.
3. **Give one improvement.** Mend the worst pillbox, a mine on the
   moat, works on a pad; 11g's upgrades fill this slot when decided.
4. **Connect.** Road toward the nearest established friendly base or
   pillbox not yet joined to ours, planned once by the strategy layer
   (grass 1, forest 1 and a fell, water a bridge's price), kept on the
   board as road jobs any tank can lay.
5. **Mission.** When a road reaches an outlying pillbox, an escort job:
   two haulers on the `Tow` chain, the rest in column screening the
   threat side; the pillbox comes home along the road and is set in
   the castle while it has fewer than its two inside and one on the
   road, else at a post along a road between bases, never at a
   junction (section 10, the castle rule).

Weights `w_fill`, `w_give_item`, `w_give_upgrade`, `w_connect`,
`w_escort`. A threat at base skips the cycle, as today.

### 6.2 Islands, boats and bridges

A base site's score:

```
S = w_forest * min(1, F_island / F_need) + w_pads * P + w_choke * K
    - w_small * max(0, 1 - A_island / A_min)
```

F_need is what the plan will ask for (castle, turrets, yard, roads
out) in trees at 4 material each; A_min a tunable area. A small island
with few trees scores low.

A crossing is priced both ways, with scarcity s = F_need / (F_reachable + 1):

```
C_bridge(L) = L * (1 + s) + k_long * max(0, L - 6)^2
C_boat      = c_boat * (1 + s) + t_launch
```

Up to 6 tiles the long term is zero and a short bridge wins; from 7 to
20 the square grows fast, and a long bridge wins only for a road the
network will use; `BRIDGE_MAX` stays 20. On a small island the high s
makes the boat (`BOAT_COST` 5) beat a long bridge. Starting guesses,
for the tuner: k_long 0.5, c_boat 5, t_launch 3.

A part-laid bridge is a job on the board, picked up by the next tank
with material, dropped only when the strategy layer drops the crossing.

### 6.3 Column on a bridge

When the leader is on a bridge or narrow tile, or its next 3 route
tiles are: the leader drives straight at full speed (no gathering, no
formation); slot k is k x 2 tiles behind along the bridge axis, no side
offsets; wingmen still on land wait at the bridge head in slot order;
the wedge returns when the last tank is 2 tiles past the far end.


## 7. Decision smoothing

A tank keeps doing its last command until a new one is confirmed over
several thoughts; only the two survival reflexes break in at once.

1. **Hold the last command.** The reflex layer replays the intent
   every tick between thinks; a think that finds nothing better emits
   nothing; a new Move only when the intent changes or its target moves
   more than a tile.
2. **Desire vectors.** D = (1 - alpha) D + alpha d, A = (1 - beta) A +
   beta a, with alpha 0.4, beta 0.3 at 4 Hz: a full swing in 3 or 4
   thinks.
3. **Confirmation with hysteresis.** A challenger b replaces a when
   U(b) > U(a) (1 + h) for n thinks in a row, with U(a) raised by
   (1 + m) while held: h 0.15, n 2 (3 for a turret target), m 0.1.
   Advance above A = +0.35, fall back below -0.35, hold between. The
   aim point is led, then smoothed (EMA 0.5).
4. **Survival bypass.** Dodge (a shell with eta 4 s or less landing
   within 3.3 tiles) and retreat (one threshold pair,
   `retreat_armor` and `retreat_shells`) skip smoothing and
   confirmation at full speed; the old intent comes back after, unchanged.
5. **Brain power per system.** The cadences of section 5 decide how
   often a system thinks; the weights decide how much it counts; both
   tunable, both measured by section 8.


## 8. Weight seeds and the ledger

One record per seat per match, JSON lines in `reports/ledger/`:

| Field | Note |
| --- | --- |
| `genome` | a hash of the fields below, the key |
| `weights` | every slider: posture, actions, the Tactics 22, the smoothing constants |
| `caps` | the components that were on |
| `seed` | the per-seat seed |
| `role` | Leader, Guard, Ranger, Wing, Builder |
| `parent` | lineage |
| result | won, kills, deaths, revives, territory-seconds, plan items built, road tiles, bridges finished and started, pillboxes hauled, time to first castle, starved-seconds |

```
score = 10 win + k_kill kills - k_death deaths + k_terr territory_s / T
        + k_build items + k_road tiles - k_starve starved_s
```

Offline only, on the ignored census tests, headless at 4x: a hall of
fame of 16 per role ranked by a rating (Elo or TrueSkill, mu - 3
sigma); a regression of score on each weight's offset, to show which
sliders matter; a (1 + lambda) evolution strategy, CMA-ES past about
20 weights, on fixed seed sets; a genome becomes the default only by
beating the default over a fixed census with a confidence interval
clear of zero. Role sets are genomes too, so the fair fleet start is
measured.


## 9. Territory, fronts, logistics, forts, rotation

| Map (16 x 16 sectors, 1 Hz) | Built from | Used for |
| --- | --- | --- |
| Influence = ours - theirs | pillbox coverage and tank worth, decayed per sector | the front where it crosses zero; territory where positive |
| Supply distance | road distance to a friendly depot | logistics cost |
| Forest left | trees per sector | felling; site scarcity |
| Favourites | forest, pads, chokes, distance to the front | where forts go |

Forts are the castle plan and the yard on a favourite site; a second
fort when the front passes a sector beyond the first's guns. Roads
between forts are the attack lines. Defence points are the chokes of
the road network (bridge heads, gates, junctions near the front), two
to four of them.

Rotation across the team: each second rank the points by threat and
the tanks by readiness (armour x shells, time to supply); swap when a
tank at a hot point is more than 0.3 below one at a cold point,
confirmed over two team thinks; each tank keeps `DWELL` 6 s and the
90% refill as a floor.

| Posture | When it scores high |
| --- | --- |
| Economy | low material, a starved plan, no threat near |
| Fortify | an unfinished fort on a favourite site |
| Connect | road gaps between forts or to pillboxes |
| Expand / Assault | front odds at least `press_odds` 1.5 and an attack line |
| Defend / Rotate | threat in our territory, or a wide armour spread |


## 10. The plan

Step 12 of `ROADMAP.md`, in this order, each part behind a `Caps` bit
so the brain of 17 October stays the baseline, each judged by a census
before it is the default: 12a measure, 12b memory, 12c weights and
caps, 12d intent and smoothing, 12e the team board and team brain, 12f
behaviours, 12g ledger and tuner.

Paul's answers, 18 October: every either/or is a scalar ("should it x
or y, we choose a scalar of how much x, and how much y is weighted").
Each question became a weight whose default is his answer and whose
best value the ledger and the tuner find (ROADMAP 12h):

| Question | Answer | Weights |
| --- | --- | --- |
| The improvement slot before 11g | mend, mines and works by weight; upgrades take a share once 11g exists | `w_mend` 1, `w_mine` 0.5, `w_works` 0.5, `upgrade_vs_mend` 0.5 |
| Sliders for players or the host | the host's file is every bot's default; a player's sliders move only their own team | (not a blend) |
| A hauled pillbox: castle or junction | the castle until it holds 2 inside and 1 outside on its road; then posts along the roads between bases and castles, beside the road, never at a junction | `castle_inside` 2, `castle_road` 1, `post_spacing` 10, `posts_max` 4, `junction_clear` 3, capability `castle_cap` |
| Road out to what | bases and castles before lone pillboxes | `connect_bases` 0.75 |
| The score | halfway between elimination and holding ground | `long_game` 0.5 |
| The same roles for every team | a person's team keeps its builder for now | `assist` 1 (0 deals it as a bots' team) |

Upgrades and the ways to buy them, which 11g left open, are designed in
ROADMAP goal 5: in-match, for supply earned from what a side holds,
installed by the squad at the base; twelve base upgrades and four for
the tank; bought at a depot from the coordination screen, or by a
fleet commander's bot through the improvement slot, its team's posture
leaning the choice.


## 11. Implementation and first measurement, 18 October

Built the same day, in `treadcore-brain`, each part behind its bit of
`Caps` and off by default (ROADMAP goal 4, the status list):

| Part | Module | Bit | Proof |
| --- | --- | --- | --- |
| 12a measure | `meter.rs` | always | `the_meter_counts_and_resets`; the census line |
| 12b memory | `facts.rs`, `memory.rs`, `fortify.rs` `turret_preview`, `tactics.rs` `value` | `forget` for expiry; the rest always | `the_islands_are_counted`, `a_memory_runs_out_unless_kept_for_ever` |
| 12c weights | `weights.rs` | the existing parts' bits on | `default_weights_are_todays_constants`, `a_weights_file_round_trips_and_names_its_mistakes`, `the_geometric_mean_compensates_and_vetoes` |
| 12d smoothing | `intent.rs`, `commander.rs` `go`, `tactics.rs` `pick` | `smooth` | `a_challenger_needs_two_thinks`, `a_dodge_skips_confirmation`, `the_advance_holds_between_its_bands` |
| 12e the board | `board.rs`, `crew.rs` `adopt`, `fleet.rs` `board_for` | `board`, `team_rotate` | `the_front_is_where_influence_crosses_zero`, `a_hurt_tank_swaps_out`, `a_missed_job_is_still_on_the_board` |
| 12f behaviours | `cycle.rs`, `facts.rs` prices, `hop.rs`, `commander.rs` `slot_point` | `cycle`, `priced_crossing`, `island_site`, `column` | `a_full_tank_gives_the_base_an_item`, `a_small_island_takes_a_boat`, `a_fleet_crosses_a_bridge_in_file` |
| 12g ledger | `ledger.rs`, `fleet.rs` `brain_census`, `scripts/tune.sh` | n/a | `a_ledger_line_round_trips_and_rates` |

Two departures from the design above. The ledger's "territory-seconds"
is `held_s`, the pillboxes and depots a team held, summed each second:
the board's territory needs the board on, and the census compares teams
with it off. And the turret's confirmation needs three picks by the
margin, the move's desire vector applies only to shifts under `jitter`
(3 tiles), and the commitment bonus is in `utility` for the components
still to be ported.

The first census, naval.map, three seeds of 900 s, respawns 3:

| Trial | Trial team, mean score a seat | Baseline teams | Crossings searched / cached | Plan copies | Cost |
| --- | --- | --- | --- | --- | --- |
| the baseline itself | 13.95 | 8.93 | 213 / 956 | 10 | 28.0 us a seat-tick |
| every new bit on | 10.29 | 11.53 | 207 / 1,368 | 11 | 28.6 us a seat-tick |

The baseline against itself differs by five points a seat, so three
seeds measure the dice, not the parts: nothing is promoted. Over the
same matches the trial team spent 0 to 44 seat-seconds a match in the
base cycle and 2 to 36 relieving a teammate, and laid fewer bridge
tiles, 4.7 a match against the baseline teams' 9.3 (0 to 22), which is
what the crossing price asks, though not yet more than the dice allow.

Later the same day, Paul's answers (section 10) went in as step 12h:
twelve more weights (68 in all) and the `castle_cap` bit (14 bits). The capped castle draws
`castle_inside` pillboxes inside its wall and, once it stands, puts
`castle_road` slots beside its road where the bastions were; past that,
the Commander adds road posts every `post_spacing` tiles on the line to
each other depot of the side, beside the road and `junction_clear`
tiles from any junction, and the turrets rung carries outlying
pillboxes out to them. In a match alone with three outlying pillboxes
and a second depot, the castle stood with two inside, and the road
slot and the first post were filled by 371 s
(`a_capped_castle_fills_then_lines_the_road`). Brain tests: 55 passed,
8 ignored.


## 12. The capped castle made the default, 18 October

Paul made `castle_cap` the default the same evening ("make castle_cap
the default"). The bit's default turned on in `weights.rs`, so every
Commander, a fleet's or a person's autopilot, draws the castle of 12h:
two pillboxes inside, one outside beside its road, then road posts.
The castle of 11b and 11h (three towers, two bastions) stays one bit
away: `caps = baseline, -castle_cap` in a weights file, or
`CENSUS_CAPS=-castle_cap` in the census. The two proofs written for it,
`a_commander_fortifies_its_depot` and `a_commander_climbs_the_ladder`,
now set that bit off and prove the old castle still works;
`a_capped_castle_fills_then_lines_the_road` proves the default, and
`default_weights_are_todays_constants` says the cap is on. Brain tests:
55 passed, 8 ignored.

### The measurement, and what it showed about the census

Naval.map, six seeds of 900 s, respawns 3, the trial team rotated over
the three starts:

| Run | Trial team | Baseline teams | Trial a seat, by seed |
| --- | --- | --- | --- |
| the default against itself (the null run) | 12.66 | 6.44 | 6.39, 13.35, 12.51, 21.01, 11.54, 11.15 |
| the old castle (`-castle_cap`) against the default | 11.70 | 7.67 | 6.96, 11.84, 10.48, 14.87, 21.58, 4.45 |

The null run plays the same settings on every team, yet its trial team
scored six points more a seat than the rest: with the seeds fixed, the
seat the trial team draws on each seed is a good one more often than
not. So the census's trial-against-baseline figure is biased, and the
only fair reading is paired, the trial run against the null run on the
same seeds. Read that way, the old castle scored 0.96 a seat less than
the capped one, inside the spread of either (4.45 to 21.58). The capped
castle costs nothing measurable, and is the default by Paul's choice,
not by the census. The first seed's castles were not finished in 900 s
under either rule, and that seed's numbers barely moved.

What this changes in the method (ROADMAP goal 4): a part is judged
against a null run on the same seeds, not against the baseline teams of
its own matches; the tuner's fitness (trial less baseline, over fixed
seeds) still ranks children fairly against one another, since every
child meets the same bias.


## 13. The upgrades proposal, 18 October

Paul asked for the proposal the same evening, and it is
`PROPOSAL-UPGRADES.md`. It is in GOVERNANCE.md's form, because it
changes the rules and the wire: Change, Why, Must not change, Proof,
the questions left for discussion, and the traces it needs before it
can be accepted. In short:

- **Supply.** A side earns a second count from what it holds: 1 every
  10 s a pillbox, 2 a depot, 1 every 20 s a powered structure, 10 a
  kill, 5 a capture. Over the naval censuses (section 12) that is about
  12 to 50 a minute.
- **The upgrades.** Twelve for a base or the whole side, and four for
  the tank, each with its levels, supply price, material and how it is
  installed. They are hardened walls, pillbox calibre and plating, a
  watchtower (11j's flag), a radar mast, a gatehouse, depot stores, a
  yard workshop, a minefield, a dock, bridge works and a supply road;
  and for the tank, plating, bay, squad and hopper.
- **Buying.**
    - The world checks every purchase made with `Command::Buy`.
    - Upgrades that need building are installed by the squad
      (`PillboyTask::Install`).
    - A works tab on the coordination screen lists them.
    - A fleet commander's bot buys through the base cycle's
      improvement slot, by `upgrade_vs_mend`, with its team's posture
      leaning the choice and a `w_up_<name>` weight for each upgrade.
    - Allies can hand each other supply (11e).
- **Must not change.** The world stays deterministic and hashed. Old
  replays check under `Scenario::upgrades` false. A match where nobody
  buys plays as today. Nothing is carried between matches.

For this report, the proposal ties the brain to the game's economy.
The improvement slot of 12h, and the weight `upgrade_vs_mend` that
Paul's answer created, are where the bots will spend supply. The
team's posture (12e) chooses what they buy. Each upgrade's worth is a
weight in the flat array (12c), so the tuner (12g) can learn the bots'
buying as it learns their fighting. Until the sim step lands,
`upgrade_vs_mend` has nothing to weigh, and the brain buys nothing.


## 14. The fair census, 18 October

Section 12 found the census biased: the trial team's seat on each fixed
seed is a good one more often than not. The cure, backlog item 16, is
in `brain_census`: each seed is played three times, every team in the
trial seat once (`CENSUS_FAIR`, on by default; `CENSUS_FAIR=0` keeps
the old rotation, a third of the matches). With every team the same,
the three matches of a seed are the same match, so a null run reads
exactly even; one seed of 120 s read 13.93 against 13.93.

The capped castle measured again, fairly, on naval.map, three seeds of
900 s, nine matches:

| Run | Old castle (trial) | Capped castle (baseline) | Difference a seat |
| --- | --- | --- | --- |
| paired against a null run, six seeds (section 12) | 11.70 | 12.66 | -0.96 |
| fair, three seeds | 8.36 | 9.08 | -0.72 |

The two methods agree: the old castle is a little behind and inside the
noise, and the capped castle stays the default. A fair census costs
three times the matches, so `make tune` now defaults to two seeds (six
matches a child).


## 15. Smoothing finished: the ladder and the advance, 18 October

The rest of 12d, under `smooth`:

- **The Auto ladder confirms a change of rung.** A new rung must win
  `confirm` thinks in a row while the old one still holds; a think is
  a quarter second. A threat to the base (DefendBase), or the old rung
  failing its own condition, changes the rung at once
  (`Commander::rung_challenger`).
- **The decision to close is an averaged advance scalar.** The odds are
  read as -1 (at `engage_odds` or under) to +1 (at `press_odds` or
  over). The reading is averaged once a think by `beta` and held
  between thinks. The tank presses only above `advance_band` (0.35, a
  new weight), falls back to mass only below minus it, and engages in
  between (`Tactician::smooth`). Proof: in
  `the_square_law`, one think of two-to-one odds is not yet a press,
  and two are.

The fair census, naval.map, two seeds of 900 s, six matches:

| Trial | Trial team | Baseline teams | Moves a match, all seats |
| --- | --- | --- | --- |
| `smooth` | 7.55 | 9.47 | 4,661 |

Smoothing cost 1.9 a seat over six matches; the spread of a team's
score across them was 3.28 to 14.29. It did not cut the Moves asked of
the world either (the old castle's nine matches asked 4,391 a match).
So `smooth` stays off. The likely reasons to look at next: holding a
rung for half a second delays leaving a rung whose work is done, and a
smoothed stance arrives at a fight a think or two late. Its weights
(`confirm`, `beta`, `advance_band`, `margin`) are the tuner's to try
before the bit is judged again.


## 16. The escort, 18 October

Paul's first message asked that the tanks "go out on a mission with the
team to start moving the pillbox along the newly created road leading
back to the base". The pieces existed: the turrets rung fetches an
outlying pillbox, the wingmen fly the leader's wedge, and the world's
router already prefers road (a road tile costs 0.6 against grass's
1.0), so a pillbox carried home comes along the road where there is
one. What was missing was the leaving together. Under the new bit
`escort`, a fleet leader at its base whose fetch trip runs farther than
`fetch_gather` (20 tiles, a new weight) gathers its wingmen first, with
the same gathering the hunt and the expand use. The fleet then goes out
in its wedge and screens the carry home. Proof:
`a_far_fetch_gathers_the_fleet`. With the wingman 45 tiles off and the
pillboxes 26 and 30 out, the leader gathered for the fetch and its
squad lifted the first pillbox by 324 s. Brain tests: 56 passed.

The fair census (two naval seeds, six matches of 900 s) read 7.70
against 7.70 exactly. No team reached a far fetch in 900 s, so the bit
never acted and the matches were the null run's. `escort` stays off
until a longer census (goal 5's longer matches) can see it work.

## 17. The sliders, 18 October

Paul's first message asked for "a slider for each coefficient
weights". The weights page is the coordination screen's second page:
`U` opens it (a new rebindable action, `tune`). It has a row for each
of the 15 capabilities, a switch, and a row for each of the 70
weights, a slider (`client::tune`).
- Up and down pick a row; left and right move it one step.
- Delete puts the row back to its default; Enter saves; `U` or Esc goes
  back to the ladder.
- Each row shows its value and a bar from nothing to twice the default,
  the default at the middle. A `*` marks a row that differs from the
  default. Under the rows is the doc line of the picked row, with its
  default and its step.

A step is one for a rounded weight (the doc says "(rounded)"; five docs
gained the word: `focus_cap`, `answer_cap`, `kite`, `confirm`,
`confirm_turret`). Otherwise it is a tenth of the default, taken to
1, 2 or 5 of its power of ten: `alpha` 0.05, `near_scale` 2,
`forget_for` 10 (`W::step`). `Settings::nudge` never goes under nothing
and keeps four places, so three steps up and three down is the number
it was.

Whose bots, the table of section 10 decided: the host's `--weights`
file stays every bot's, and a player's sliders are their own team's.
The page gives its settings to the tank under the goal at once, and,
when the window hosts, to the bots of the player's side, again at each
new match (`Brain::set_settings`, new, a no-op for a brain without
weights). A remote player's page reaches only their own tank, and it
says so. Enter keeps the team's settings in
`~/.config/treadcore/weights`, beside the keys file, and the next start
reads them for the team only. Without that file the page starts from
`--weights` or the defaults.

Two changes in the brain made a mid-match change safe:
- `Commander::set_settings` updates the tactician in place, keeping
  its target and the team's calls.
- The four switches it used to turn off for good (dodge, pings, rotate,
  bridges) now put away the role's own setting when turned off and give
  it back when turned on again (`CapHeld`).

Before this, no settings ever reached the window's pilot. It now has
the page's, so `--weights` or the team file steer the person's own
tank on auto too.

Some switches are read only when a match is dealt: `board` gives a team
its board in `fleet::equip`. Set on the page, such a switch takes hold
at the next match.

Proof:
- `a_slider_steps_and_steps_back`;
  `a_switch_comes_back_and_a_slider_keeps_the_target`;
- `the_page_moves_a_weight_and_puts_it_back`,
  `the_page_switches_a_capability_and_wraps`, `a_saved_page_reads_back`,
  `the_bar_marks_the_default_in_the_middle`.
- Brain tests: 58 passed; client: 31.
- The window, driven with `hyprctl dispatch sendshortcut` and shot with
  `grim` (journal 0043): Space, then `U`, then a switch and a slider
  moved, Delete, then Esc back to the ladder with the fleet listed.

The page measures nothing by itself, so it has no census. What it
changes is who can tune: a player can now try a weight in the match in
front of them, and a saved file is a genome the ledger can read.

## 18. Bridge jobs on the team board, 18 October

Section 6.2 planned it: "A part-laid bridge is a job on the board,
picked up by the next tank with material, dropped only when the
strategy layer drops the crossing." Before this, a bridge given up
(stalled 40 s, no material in reach, the layer dead) stayed in the
water half laid. Its layer went on by boat and wrote the target off.

Under the new bit `bridge_jobs`, which needs `board`:
- Every member laying a bridge posts it to its team board each think:
  the water tiles still to lay, in order from the laid end, and what it
  crosses to (`Hop::bridge_job`, `TeamBoard::post_bridge`). The same
  bridge is known by a tile or the target in common.
- Tiles laid by anyone come off; a bridge laid, or untouched for
  `bridge_keep` (180 s, new), comes off the board
  (`prune_bridges`). That is the "strategy layer drops the crossing"
  of the plan, as a time.
- A bridge whose layer has not posted for 3 s is free. A teammate free
  of more pressing work takes the nearest whose next tile is within
  `bridge_reach` (40 tiles, new): the squad aboard with material, not
  resupplying, no trip under way, a bridge it has not given up itself
  (`take_bridge`, `Hop::bridge_refused`).
- The taker's hop lays the rest from the laid end and stops there
  (`Hop::adopt_bridge`). The crossing was the other tank's errand, so a
  bridge finished or given up ends the trip; a given-up one stays on
  the board for another.

Who is free of more pressing work was the step's finding. First it was
a tank on auto whose rung was expand or guard. A temporary counter in
one census match showed no such tank: the free bridges were the
leader's, given up as it sailed on, and the only teammates near them
with material were its wingmen. So who takes became a scalar, per the
either/or rule: `bridge_wing` (rounded, default 1) lets a wingman with
material take a free bridge, leaving the wedge to do it.

The census gained `CENSUS_BASE_CAPS`, the baseline teams'
capabilities, so a part that needs another is tried against that one
on: here `board` for every team and `bridge_jobs` for the trial team.
It also counts the trial team's seat-seconds "for the team".

Proof:
- `a_part_laid_bridge_waits_for_the_next_tank`: posted, not free while
  its layer lays it, not taken from too far or by a tank that gave it
  up, taken by the nearest, laid tiles struck off, dropped untouched
  and when laid.
- `a_teammate_finishes_a_part_laid_bridge`: a channel four tiles wide
  across the map, its first tile laid; a tank on auto with material
  took the other three and laid them by 9 s, and the bridge came off
  the board. With the bit off nobody touched it in 240 s.
- Brain tests: 60 passed.

The fair census (naval.map, two seeds, six matches of 900 s, respawns
3) read 7.31 a seat for the trial team against 8.01. The bit acted in
five of the six matches, 69 to 783 seat-seconds. One trial team laid
36 bridge tiles against 15 to 20 in the others. The 0.70 a seat it
cost is about the noise of six matches; `bridge_jobs` stays off, and
`bridge_wing`, `bridge_reach` and `bridge_keep` are for the tuner.

## 19. A dodge for direct fire, measured first, 18 October

Backlog 8 and finding "direct fire is never dodged before it lands"
asked for a dodge of direct fire beside the dodge of indirect shells.
Before designing one, the step measured whether a tank that sees a
direct shell launched can get out of its way at all.

The numbers are against it:
- Direct range is 10 tiles and a direct shell flies at 40 tiles a
  second, so it is in the air five ticks at most (0.25 s).
- It hits if the tank is within `DIRECT_HIT_RADIUS` (0.8 tiles) of
  where it was aimed when it lands, and a bot aims where the tank will
  be at the speed and heading it has now (`lead`).
- In 0.25 s a standing tank accelerating at 4 tiles a second squared
  covers 0.125 tiles. A tank at full speed turning at its 3 radians a
  second strays about v w t^2 / 2 = 4 x 3 x 0.0625 / 2, roughly 0.4
  tiles, off the line the gunner led; braking gains about 0.125.

Measured (`a_led_direct_shot_cannot_be_dodged`, `lib.rs`): a tank
crossing at full speed, a gunner square to it at 4, 7 and 9.5 tiles
firing a led shot, and the target answering the launch five ways (carry
on, reverse, turn away, turn toward, stop). Every one was hit at every
range. The test asserts it, so a change of the rules that made a dodge
possible would show.

A weave before the shot is no better against a bot, because it leads
from the speed and heading at the moment it fires, and only what
changes during the flight counts.

So the only defence against direct fire is denying the shot: not being
inside an enemy gun's range, in its line, when that gun is ready
before one's own. Every tank's reload is in the observation. Kiting
already does this against the one enemy a tank is fighting; a builder,
a hauler or a bridge-layer shot by a tank it is not fighting has
nothing. Three ways on, for Paul to choose (ROADMAP backlog 17):
1. a "duck" reflex behind its own switch: out of range or line when a
   hostile gun ready before one's own covers the tank and the exchange
   is not one it wins, with a weight for how far it outranks the
   errand; measured by the fair census;
2. close the item as impossible, as this section records;
3. change the rules (a slower direct shell, a smaller hit radius) so a
   dodge is possible, which changes how every match plays.

## 20. The brain now: the improved prototype, 18 October

What runs after step 12. Every row of section 2's tables is still
there, and around it are the parts step 12 added; those marked (off)
are built, proven and measured but wait for a census to promote them.

| Level | Code | Rate | Output |
| --- | --- | --- | --- |
| Settings | `weights.rs`: 76 weights, 17 capabilities; `--weights FILE` for every bot, the weights page (`U`) for a player's team | read at the deal and on a change | the numbers every part below reads |
| Team board (off) | `board.rs`, one per team, shared | rebuilt 1 Hz by the first member due | influence, front, defence points, posture, kept jobs, swaps, part-laid bridges |
| Fleet commander | `crew::FleetCommand` | 20 Hz; musters every 30 s | jobs, as before |
| Commander, one per tank | `commander.rs` | 20 Hz | `Vec<Command>` |
| Auto ladder | DefendBase > Fortify > Turrets > Hunt > Expand > Guard, the rung confirmed over two thinks under `smooth` (off) | 20 Hz | a goal |
| Base cycle (off) | `cycle.rs` | after a fill | an item, an improvement by weight (mend, a mine, works), a road out |
| Doctrine and tactician | `lib.rs`, `tactics.rs`, its weights from `Settings` | 4 Hz | guns, drones, target, odds, stance (smoothed under `smooth`) |
| Hop | `hop.rs`, `facts.rs`: the crossing cache, priced crossings (off), adopted bridges (off) | each doctrine think | bridge, boat, landfall |
| Memory | `memory.rs` `Forget` (expiry off by default), `intent.rs` | per use | what was given up, smoothed goals |
| Ledger and meter | `ledger.rs`, `meter.rs`, `brain_census`, `make tune` | per match | a line per tank: genome, seed, score; the cost a seat-tick |

```
Settings (weights.rs: 76 weights, 17 switches)  <-- --weights FILE, weights page (U)
   |
Team board, one per team, 1 Hz (board)                         (L5)
   |  influence, front, points, posture, jobs kept,
   |  swaps (team_rotate), part-laid bridges (bridge_jobs)
   v
Commander, each tank, 20 Hz, first match wins                  (L1)
   +-- survival: downed, dodge (indirect shells), duck (direct
   |      fire denied before it is fired: duck, LR 21), revive, home
   +-- bridges: help the leader's; take a part-laid one (bridge_jobs)
   +-- afloat, shore gather, column on a bridge (column)
   +-- board: post, rebuild, adopt a missed job, relieve or fall back
   +-- road posts once the capped castle stands (castle_cap, on)
   +-- pings, rally, fleet, crew job
   +-- Auto ladder (6 rungs; confirmed under smooth)
   |      Fortify: the capped castle; Turrets: the fetch, gathered
   |      first when far (escort)
   +-- base cycle after a fill (cycle)
   +-- go(): desire vector under smooth                        (L7)
   +-- Doctrine, 4 Hz
          +-- Tactician (its 22 weights from Settings; smoothed
          |   stance under smooth)
          +-- Hop (crossing cache with failures; priced under
              priced_crossing; island sites under island_site)  (L4)
```

The findings of section 2, now:

| Finding | Now |
| --- | --- |
| L1 first-match ladders | every number is a named weight with a slider; the ladder and the wingman's priorities are still fixed orders |
| L2 a full world copy per bot per tick | unchanged: the Observation shared per team is 12b's remainder |
| L3 repeated work | the plan copy and the reasons per candidate gone; the `why` and `doing` strings still formatted each tick |
| L4 failed searches not cached | answered: the crossing cache keeps failures 60 s |
| L5 no shared team memory | the team board (off) |
| L6 lists that never forget | `Forget` with `forget_for` (off) |
| L7 uneven commitment | desire vectors and confirmation (`smooth`, off: measured worse) |
| L8 a reactive economy | the base cycle (`cycle`, off) |

The numbers, 18 October evening:
- 12,107 lines in 16 files (8,458 in 8 on 18 October morning).
- Brain tests: 62 passed, 8 ignored; client: 31.
- 76 weights and 17 switches, with the duck (LR 21).
- 23 to 33 us a seat-tick in the census.
- On by default: kite, pings, dodge, bridges, rotate, and since 18
  October `castle_cap`.
- Measured fairly: the capped castle against the old, +0.72 a seat;
  `smooth` -1.92; `escort` even (never reached); `bridge_jobs` -0.70
  with `board` on for every team; `duck` -0.51 (LR 21).

What the prototype has not shown yet: that any of its new parts makes
the bots win more. The fair census can now tell a real difference from
the starts, but six matches of 900 s are not enough to see the parts
that pay late (the escort, bridges, the base cycle), and none has had
the twelve seeds the roadmap asks for. That is the next measurement,
and goal 5's longer matches the one after.

## 21. The duck: direct fire denied, 18 October

Paul chose the first of section 19's three ways: "build the duck
reflex". A shot that cannot be dodged once fired can still be denied,
by not being in reach when the gun is ready.

The reflex (`Commander::duck`, the bit `duck`, off):
- **Which guns.** A hostile tank with shells, not afloat, within 12
  tiles (direct range plus 2, since its lead reaches past its range by
  the target's own motion) and in line. Its gun is ready within
  `duck_ready` (1.5 s), and before mine, or I have no shells.
- **Seen or remembered.** Sight and direct range are both 10 tiles, so
  a gunner just past sight leads its shot into it unseen. The first
  duel showed exactly that: the gun that fired was not in the
  observation. So a gun is remembered for `duck_memory` (5 s) where it
  was last seen, its reload counted down. A direct shell in flight
  refreshes the memory, seen or not, because it carries its gun's
  position (`from`) and says the gun has just reloaded.
- **When.** Only with the tank's armour share at or under `w_duck`.
  This is the scalar between safety and the errand: 1 always ducks, 0
  never.
- **Where to.** It looks at sixteen points around it, at the distance
  that clears the gun with a margin (13 tiles out, 3 to 8 tiles away).
  Of those out of its reach or line, it takes the one nearest the
  errand (the last Move's goal); if none, it backs straight away.
- It holds the point a second at a time. The guns stay free while it
  ducks, as in a dodge. It never ducks afloat or while its squad mends
  a wreck. Once the gun is no longer a threat, the errand plans again.

The census counts the trial team's seat-seconds "ducking".

Proof: `a_tank_ducks_a_gun_ready_before_its_own`.
- **The rules.** It ducks out of reach when the gun will fire first,
  and not when its own gun is ready first, not with `w_duck` 0, and not
  with the bit off.
- **The duel.** A gunner standing 7 tiles off fires a led shot whenever
  it can; the tank holds its post with its gun held reloading, never
  able to answer. Kiting is off in both arms, since kiting already
  holds a fighting tank past the gun's reach; the duck is for the tank
  that does not kite. Over 40 s: hit 2 times ducking, 3 standing. The
  hit that remains comes when the memory lapses and the post, 7 tiles
  from the gun, draws the tank back into its fire. Holding a post under
  a waiting gun is the trade `w_duck` is there to weigh.
- Brain tests: 62 passed, 8 ignored.

Building it took four duels, each of which found something before the
reflex did:
1. Three direct hits down a tank, so counting hits needed a fair cap.
2. A tank whose gun is ready does not duck, by the rule, and a duel
   that silenced its gun hid that.
3. Return fire and strike drones ended the match in four seconds.
4. The unseen gun at 10.3 tiles, the real finding.

Fair census (naval.map, two seeds, six matches of 900 s, respawns 3):
7.01 a seat for the trial team against 7.52. It ducked in all six
matches, 2 to 16 seat-seconds in five and 567 in one. The 0.51 is
inside the noise; `duck` stays off, and its four weights are for the
tuner.
