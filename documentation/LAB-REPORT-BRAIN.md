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
| Settings | `weights.rs`: 91 weights (the dodge's four since LR 27, the landfall's three since LR 31 to 36, `w_smooth` since LR 46), 17 capabilities; `--weights FILE` for every bot, the weights page (`U`) for a player's team | read at the deal and on a change | the numbers every part below reads |
| Team board (off) | `board.rs`, one per team, shared | rebuilt 1 Hz by the first member due | influence, front, defence points, posture (five scores, LR 25), kept jobs, swaps, part-laid bridges |
| Fleet commander | `crew::FleetCommand` | 20 Hz; musters every 30 s | jobs, as before |
| Commander, one per tank | `commander.rs` | 20 Hz | `Vec<Command>` |
| Auto ladder | DefendBase > Fortify > Turrets > Hunt > Expand > Guard, the rung confirmed over two thinks above `w_smooth` 0.5 (0.05 by default, LR 46), leaned by the team's posture under `posture` (off, LR 24) | 20 Hz | a goal |
| Base cycle (off) | `cycle.rs` | after a fill | an item, an improvement by weight (mend, a mine, works), a road out |
| Doctrine and tactician | `lib.rs`, `tactics.rs`, its weights from `Settings` | 4 Hz | guns, drones, target, odds, stance (smoothed above `w_smooth` 0.5) |
| Hop | `hop.rs`, `facts.rs`: the crossing cache, priced crossings (off), adopted bridges (off) | each doctrine think | bridge, boat, landfall |
| Memory | `memory.rs` `Forget` (expiry off by default), `intent.rs` | per use | what was given up, smoothed goals |
| Ledger and meter | `ledger.rs`, `meter.rs`, `brain_census`, `make tune` | per match | a line per tank: genome, seed, score; the cost a seat-tick |

```
Settings (weights.rs: 91 weights, 17 switches)  <-- --weights FILE, weights page (U)
   |
Team board, one per team, 1 Hz (board)                         (L5)
   |  influence, front, points, posture, jobs kept,
   |  swaps (team_rotate), part-laid bridges (bridge_jobs)
   v
Commander, each tank, 20 Hz, first match wins                  (L1)
   +-- survival: downed, dodge (indirect shells; its numbers
   |      weights and `dodge_hold`, LR 27), duck (direct
   |      fire denied before it is fired: duck, LR 21), revive, home
   +-- bridges: help the leader's; take a part-laid one (bridge_jobs)
   +-- afloat: the landfall, over the guns seen and remembered
   |      (landfall_memory 30 s), pricing the run (landfall_shy)
   |      and standing off a covered beach (landfall_wait); on
   |      since LR 38. Shore gather, column on a bridge (column)
   +-- board: post, rebuild, adopt a missed job, relieve or fall back
   +-- road posts once the capped castle stands (castle_cap, on)
   +-- pings, rally, fleet, crew job
   +-- Auto ladder (6 rungs; confirmed above w_smooth 0.5)
   |      Fortify: the capped castle; Turrets: the fetch, gathered
   |      first when far (escort)
   +-- base cycle after a fill (cycle)
   +-- go(): desire vector, by w_smooth (0.05)                 (L7)
   +-- Doctrine, 4 Hz
          +-- Tactician (its 22 weights from Settings; smoothed
          |   stance above w_smooth 0.5)
          +-- Hop (crossing cache with failures; priced under
              priced_crossing; island sites under island_site)  (L4)
```

The findings of section 2, now:

| Finding | Now |
| --- | --- |
| L1 first-match ladders | every number is a named weight with a slider; the ladder and the wingman's priorities are still fixed orders |
| L2 a full world copy per bot per tick | unchanged: the Observation shared per team is 12b's remainder |
| L3 repeated work | the plan copy and the reasons per candidate gone; the status as codes (LR 22): the `why` column formatted only when the screen shows it, logic and census reading codes; the `doing` words still written, measured as costing nothing |
| L4 failed searches not cached | answered: the crossing cache keeps failures 60 s |
| L5 no shared team memory | the team board (off); read by `team_rotate`, `bridge_jobs` and, since LR 24, `posture`; its influence, front and territory still read by nothing (LR 24) |
| L6 lists that never forget | `Forget` with `forget_for` (off) |
| L7 uneven commitment | desire vectors and confirmation, as `smooth` measured worse in four readings, and since LR 46 a weight, `w_smooth`, 0.05 by default |
| L8 a reactive economy | the base cycle (`cycle`, off) |
| `partial_cmp().unwrap()` on a NaN | answered (LR 23): every float comparison a `total_cmp`, in the brain, the client and the sim |

The numbers, 18 October evening:
- 12,566 lines in 17 files (8,458 in 8 on 18 October morning).
- Brain tests: 64 passed, 8 ignored; client: 31; sim: 50.
- 76 weights and 17 switches, with the duck (LR 21); 77 and 18 with
  the posture (LR 24); 83 with its scores (LR 25).
- 23 to 33 us a seat-tick in the census.
- On by default: kite, pings, dodge, bridges, rotate, and since 18
  October `castle_cap`; since 19 October the landfall's three weights
  (`landfall_memory` 30, `landfall_shy` 1, `landfall_wait` 10, LR 38).
- Measured fairly: the capped castle against the old, +0.72 a seat;
  `smooth` -1.92; `escort` even (never reached); `bridge_jobs` -0.70
  with `board` on for every team; `duck` -0.51 (LR 21). At twelve seeds
  each, with the error (LR 24): none outside two errors.
- 19 October (LR 27): 13,193 lines; 349 functions, 122 undocumented;
  brain tests 69 passed, 8 ignored; 87 weights; 24.8 us a seat-tick.
  Where it dies: the dodge (43% of deaths in 4.5% of the time) and the
  run for the beach, `landfall` (20% in 3.2%); the ladder, rotation and
  the fleet a small share.
- 19 October evening (LR 38): 13,566 lines; 360 functions, 121
  undocumented; brain tests 73 passed, 8 ignored; 90 weights. The
  first part since the dodge measured outside the noise: guns
  remembered for the landfall, +2.31 +/- 0.58 a seat over 48 seeds
  with its two companions, landfall deaths halved; on by default.
  The dodge itself measured: off, -2.45 +/- 0.67 (LR 29).

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

## 22. The status as codes, 18 October

The last of 12b's string work. Finding L3 counted "six `why` strings
and a `doing` string formatted every tick" as repeated work. The step
measured the cost first, and found the case for codes was not speed.

**The cost.** One census match (naval.map, 300 s), as it was and with
the `why` formatting stubbed out: 14.9 and 15.0 us a seat-tick as it
was, 15.3 and 15.7 without. The difference is noise. `why` is built
only for tanks on auto. A `doing` string is one or two formats a
think, about 0.1 us against a think of 15.

**The case.** The brain's own logic and its measurements read the
sentences back:
- a guard counted as idle if its words began "on guard" or "to the
  post", and the rotation looked for exactly "on guard";
- a focus call went out if the target's reason said "called", "on it"
  or "one shot";
- the census counted its parts by substring, the quad census its kiting
  steps by phrase and its revives by "REVIVE:";
- the tests read the words.

Rewording any of those sentences would have changed play or a count,
silently.

**The change** (`status.rs`, new):
- **`Act`.** What a Commander is doing, 48 codes, three of them
  carrying a `KiteStep` and two a step of the hop or the base cycle.
  It is set beside the words at all 81 places the Commander writes
  them: 76 through `set_doing`, and 5 that hold a borrow of the fort
  plan set the two fields directly.
- **`HopAct`.** The island hop's 23 steps, set at its 28 writes
  (`Hop::say`).
- **`CycleAct`.** The base cycle's steps; `Cycle::improvement` returns
  the code, not words.
- **`KiteStep`.** The tactician's eight kiting steps. `Kite::word`
  became `Kite::step`, and its words come from `step.word()`.
- **`Tags`.** The tactician keeps bits beside its reason (one shot,
  called, on it), and the focus call reads them.
- **`Why`.** The ladder's reasons, a code and its numbers; the
  coordination screen puts them in words with `why_text`.
- **`Brain::act`.** The census and the quad census count by code, the
  ledger can carry one, and `code()` is a stable dotted name
  (`kite.step_in.reloading`, `hop.bridging`).
- The words stay, exactly as they were, for people: the status line,
  the coordination screen, the logs.

The quad census's diagnostic histograms (the status shapes, what a
seat was doing when it went down, the stuck detector) still read the
words. They want detail the codes do not carry: one hop code covers
"bridging, the squad is laying it" and "bridging, fetching material".

Proof:
- `every_code_is_its_own`: no two codes the same.
- The tests that checked logic by words now check codes: the duck, the
  escort, the base under threat, and the tactician's three kiting
  tests.
- **The equivalence.** The codes must change nothing a tank does. One
  census match (600 s, `board, cycle, escort, duck, bridge_jobs` on
  for the trial team, `board` for all) before and after: identical, in
  score, kills, deaths, thinks, moves and crossings.
- Brain tests: 63 passed, 8 ignored; client 31.

What is left of 12b is the Observation shared per team, a sim change
that must keep the world hash.

## 23. No panic on a NaN, 18 October

Backlog 10 and 19, from the review's "also" (section 2):
`partial_cmp().unwrap()` would panic on a NaN. Comparing two floats
with `partial_cmp` gives `None` when either is NaN, and `.unwrap()` on
that ends the process. In the brain that was 74 calls: picking the
nearest pillbox, the closest depot, the least hurt tank, the best
landing. One NaN in a position, from a bug anywhere or a malformed
observation, would have stopped every bot of a host at once.

**The change.** Every one is now a `total_cmp`, which orders every
float, NaN included, and never fails:
- 74 in the brain, in commander.rs (26), lib.rs (25), hop.rs (10),
  fortify.rs (8), crew.rs (4) and rotation.rs (1);
- 19 more in the window's code (input.rs 5, goal.rs 1) and the
  simulation (lib.rs 11, map.rs 2), where a NaN panic is as bad.

A one-off script rewrote each `X.partial_cmp(Y).unwrap()` as
`X.total_cmp(Y)`, matching parentheses. One comparison was of a tuple
(material short, distance), which has no `total_cmp`; it now compares
the parts in turn. Two in the map's route search already fell back to
"equal" and could not panic; they stay as they were.

`total_cmp` orders the real numbers exactly as `partial_cmp` did. It
differs only by putting -0.0 before +0.0, and the values compared here
are distances, health and scores, never a signed zero.

Proof:
- **The bug, shown.** `a_nan_does_not_panic_the_brain` puts a hostile
  tank, a pillbox and a depot at NaN in an observation and lets a
  Commander think forty times under each of seven goals. Put into the
  code of the commit before, it panics on the first goal (Auto) at a
  `partial_cmp().unwrap()`. On the fixed code it passes.
- **Nothing else changed.** One census match (600 s, five new switches
  on) before and after: identical, in score, kills, deaths, thinks,
  moves and crossings, and the same numbers as LR 22's run. Every
  crate's tests pass: brain 64, client 31, net 14, proto 3, sim 50,
  the sim's replay and world-hash tests among them.

One wrong turn on the way. The first run of the new test sat for ten
minutes and looked like a NaN driving something into a loop. It was
the build: changing the sim rebuilds every crate above it. Run by
itself under a hard timeout, the test passed in an instant.

## 24. Each switch alone, twelve seeds; the posture read, 18 October

Backlog 18 and the roadmap's "next": every capability that is off,
measured alone over the fair census at twelve seeds. Until now a part
was judged "inside the noise" by eye, from two or three seeds, with no
number for the noise.

**The noise as a number.** The census now prints the difference a
seat seed by seed, and its standard error over the seeds
(`ledger::mean_se`, `the_standard_error_of_a_sample`). A seed's three
matches share its starts, so a seed is one paired reading; within two
standard errors of zero is "inside the noise". A null run reads 0 with
an error of 0 on every seed, as it must. `make switches`
(`scripts/switches.sh`) runs the census once for each switch and prints
the table below; a switch that needs the board is tried with the board
on for every team.

**The measurement** (naval.map, respawns 3, 900 s, 12 seeds, every team
in the trial seat: 36 matches a switch, 396 in all; run three at a
time, so the microseconds read high):

| Switch | With | Trial | Baseline | Difference a seat | Error | Acted (trial team, seat-seconds) |
| --- | --- | --- | --- | --- | --- | --- |
| `smooth` | | 7.98 | 9.96 | -1.98 | 1.25 | always (it smooths every move) |
| `team_rotate` | board | 8.25 | 9.09 | -0.83 | 0.87 | relieving 879 |
| `forget` | | 8.54 | 8.90 | -0.36 | 0.23 | (retries are not counted) |
| `column` | | 8.98 | 9.05 | -0.06 | 0.41 | on bridges |
| `board` | | 8.75 | 8.75 | 0.00 | 0.00 | never: the same matches as the baseline |
| `island_site` | | 8.75 | 8.75 | 0.00 | 0.00 | never |
| `escort` | | 8.76 | 8.75 | +0.01 | 0.01 | far fetches, one seed |
| `priced_crossing` | | 9.20 | 9.09 | +0.11 | 0.56 | cheaper by boat 0; road posts 92 against 30 |
| `cycle` | | 9.08 | 8.77 | +0.31 | 0.43 | base cycle 243 |
| `duck` | | 8.93 | 8.54 | +0.39 | 1.04 | ducking 911 |
| `bridge_jobs` | board | 9.11 | 8.67 | +0.44 | 0.91 | for the team 4,438 |

None is outside two standard errors. `smooth` comes nearest (1.6 errors
below), where LR 15 put it with six matches (-1.92). `duck` and
`bridge_jobs`, slightly behind at two seeds, are slightly ahead at
twelve: the sign of a difference under one error is the dice's.

What twelve seeds can and cannot tell. A seed's difference spreads by
0.8 to 4.3 a seat (`smooth` 4.3, `duck` 3.6, `team_rotate` 3.0, `forget`
0.8). To put a real half point a seat outside two errors takes (2 x
spread / 0.5)^2 seeds: 10 for `forget`, 140 for `team_rotate`, 200 for
`duck`; a whole point, a quarter of that. The 396 matches took 22
minutes three at a time, about 1,000 an hour, so 200 seeds (600
matches) is a 35-minute run: every switch at a half point is one
night's census. The tuner's two seeds (`make tune`) can only move
weights whose effect is several points.

**Three switches never acted,** and the census could not have found
that by score alone, only by the identical matches:
- `island_site` chooses among the side's depots only when a tank has
  no base yet. In a fleet every tank's base is its leader's depot,
  found first, so the island rule is never asked. It needs to stand
  where a naval start actually chooses: which neutral depot the leader
  takes first.
- `escort` acts on a far fetch, and a 900 s naval match has about one
  (LR 16 said so).
- `board` alone changes no decision. It draws influence, the front, the
  defence points and a posture, and only `team_rotate` and
  `bridge_jobs` read any of it; its own act, adopting a job a crew
  missed, happened in none of 36 matches. The team's picture was drawn
  and nobody looked at it.

The last is the brain's structure, not a tuning: goal 4's team board
was to be the one picture the team decides on (LR 5, LR 9). The step
that follows makes the posture read.

**The posture read** (`posture`, off; needs `board`). The board's
posture is what the team should mostly be doing; the Auto ladder is
what a tank does, six rungs taken first-match, each a yes or no (L1).
The ladder now reads the posture by a blend, as every either/or is a
scalar (`Rung::pick`):

```
score(rung) = (1 - w) order(rung) + w lean(rung, posture)
order: 1 for DefendBase down to 0 for Guard, in steps of 0.2
lean:  1 for a rung the posture leans toward, else 0
```

The rung taken is the best of those switched on that hold. `w` is
`w_posture`; at 0 the pick is the first that holds, as the ladder
always was, and a leaned rung climbs past fewer than 5 w / (1 - w)
rungs above it, so from 0.5 the posture decides among the rungs that
hold. DefendBase, holding, is never passed. The leanings
(`Rung::leaned`): Defend to Guard; Fortify to Fortify and Turrets;
Connect to Turrets and Expand; Expand to Expand and Hunt; Economy, short
of material, to Expand (depots).

**What the first match found.** On the first census match the trial
team scored 3.64 against 11.02. The meter (new: the postures read and
the picks they changed) said why: the posture was Defend in 94% of the
picks, and the lean sent the tanks home to Guard. The posture had never
been read, so it had never been checked. Two faults:
- Defend's test was the sector threat at the base over 0.5. Each
  hostile tank adds 1 / (1 + d / 8) to every sector of the map, so
  eight tanks 80 tiles off add 0.73 between them. The base was always
  under threat.
- Defend was also taken when the members' readiness spread past 0.6:
  any team with one tank low on shells.

The fix: Defend is hostile worth within the base's alarm (`BASE_ALARM`,
16 tiles), a full tank 1, over 0.5; the spread is left to rotation
(`team_rotate`), which is where it belongs.
`the_team_defends_a_threat_at_the_base_not_far_off` puts eight hostile
tanks 80 tiles off and then one at the gate. On the old rule it fails;
on the new it passes. After the fix, over 48 seeds, the posture read
Fortify 47%, Connect 19%, Expand 18%, Economy 16% and Defend 0.4%.

**The measurement** (as above, with `board` on for every team):

| `w_posture` | Seeds | Picks changed | Difference a seat | Error |
| --- | --- | --- | --- | --- |
| 0.25 | 1 | 0 | +0.08 | (one seed) |
| 0.35 | 12 | 5,122 | +0.39 | 0.36 |
| 0.5 (= 1) | 12 | 50,677 | +1.21 | 1.31 |
| 0.5 | 48 | 164,606 | -0.81 | 0.65 |

At 0.25 a lean climbs one rung, and the posture's leanings sit two
below the rung that holds, so nothing changed. At 0.5 and at 1 the
matches are identical, as the formula says they must be. The first
twelve seeds read +1.21, which is promising. Thirty-six more seeds, run
three ranges at a time (`CENSUS_SEED0`), read -1.30, -2.16 and -1.01;
over all 48 the difference is -0.81 with an error of 0.65, 21 seeds
ahead and 27 behind. The twelve were the dice. `posture` stays off,
and `w_posture`'s default is 0.5, the value measured.

What the step shows about the method. A part that is never read is
never tested; the first match it is read in finds what was wrong. A
reading of +1.2 on twelve seeds with an error of 1.3 is no reading. A
part is promising only outside two errors, and confirmed only on seeds
it was not chosen on.

Proof:
- `the_posture_leans_the_ladder`: at weight 0 or with no posture, the
  pick is the first rung that holds for all 4,096 combinations of rungs
  on and holding; DefendBase is never passed; 0.25 climbs one rung and
  0.5 every rung; a switched-off rung is never picked.
- `the_team_defends_a_threat_at_the_base_not_far_off`, failing on the
  old rule.
- `the_standard_error_of_a_sample`.
- `make test`: brain 67 passed and 8 ignored; client 31; net 14; proto 3;
  sim 50. No warnings.

## 25. The posture as scores, 18 October

Backlog 21, from LR 24 and CI-001 finding A3: the board's posture was
itself a first-match ladder on thresholds, where LR 9 designed five
scores ("when it scores high"). By Paul's rule, every either/or is a
scalar.

**The change** (`board.rs`, `fortify.rs`, `weights.rs`). Each posture
has a score from 0 to 1, from what the board and the rebuilding member
know (`Home`: the plan starved, `Plan::castle_left`,
`Plan::slots_left`; each member now posts its squad's material):

| Posture | Score | LR 9's "when it scores high" |
| --- | --- | --- |
| Defend | hostile worth within the base's alarm, half a tank full | threat in our territory |
| Economy | 1 if the plan is starved, else the share of the team's material spent | low material, a starved plan |
| Fortify | the share of the castle not yet built | an unfinished fort |
| Connect | the share of pillbox slots and road posts still empty | road gaps between forts or to pillboxes |
| Expand | the team's odds against the hostiles it sees: 0 at even, 1 at `press_odds`; none seen is no front, even | front odds at least `press_odds` |

Each score has its weight (`w_pos_defend`, `w_pos_economy`,
`w_pos_fortify`, `w_pos_connect`, `w_pos_expand`, all 1). The ladder of
12e is kept, and the posture is the blend:

```
value(p) = (1 - k) [p is the ladder's pick] + k w_p score_p
```

The highest value wins; ties go in the ladder's order (Defend, Economy,
Expand, Connect, Fortify); if nothing scores, Fortify, as the ladder
had it. `k` is `posture_scored`, 1 by default: at 0 the posture is the
ladder's, exactly. The board keeps both picks and the scores, for the
screen and the tests. The weights now number 83.

Proof: `each_posture_scores_its_own_case` builds the five cases. A
castle just planned at even odds is Fortify; the castle up with half
its slots empty is Connect; the squads spent is Economy; a starved plan
is Economy; eight of ours against two of theirs in sight is Expand; a
hostile at the gate is Defend over a planned castle. At
`posture_scored` 0 the pick is the ladder's in every case. The ladder
alone would have said Fortify in three of the cases, since it has no
term for empty slots, spent material or odds.

**The measurement** (the fair census as in LR 24, `posture` at
`w_posture` 0.5, `board` on for every team, 48 seeds in three ranges of
16, the same seeds as LR 24's):

| Posture | d a seat | Error | Postures read (econ / fort / conn / exp / def) | Picks changed |
| --- | --- | --- | --- | --- |
| the ladder's (LR 24) | -0.81 | 0.65 | 16% / 47% / 19% / 18% / 0.4% | 164,606 |
| scored (`posture_scored` 1) | -1.04 | 0.58 | 37% / 43% / 6% / 15% / 0.5% | 131,278 |
| scored less the ladder's, seed by seed | -0.23 | 0.74 | | |
| the two averaged, seed by seed | -0.93 | 0.49 | | |

The ranges read +0.40, -1.46 and -2.07. The last 16 seeds alone were
outside two errors, and the 48 are not. That is one more case of a
range reading its own luck.

**What it shows.** How the posture is worked out makes no measurable
difference: scored and ladder's are -0.23 ± 0.74 apart. What both have
in common is that they override the ladder's own order, and together
they read about a point a seat behind (-0.93 ± 0.49, 1.9 errors). The
lean is the suspect, not the posture. The likely mechanism is in the
leanings. Economy is now read 37% of the time, from material spent on
building, and it leans to Expand, so tanks leave Fortify and Turrets
for depots. That is a hypothesis; the census does not yet count which
rungs gained. Backlog 27 adds that count, and the leanings themselves
are weights for the tuner once it can see a point (backlog 23).

`posture` stays off. The scored posture is the board's default, since
it is LR 9's design and measures no worse; the ladder is
`posture_scored = 0`.

- `make test`: brain 68 passed and 8 ignored; client 31; net 14; proto 3;
  sim 50. No warnings.

## 26. Which rungs the lean moves, and where the time goes, 18 October

Backlog 27, from LR 25: leaning the Auto ladder by the team's posture
reads about a point a seat behind in both forms of the posture. The
hypothesis: Economy, read 37% of the time from material spent, leans
tanks off Fortify and Turrets.

**The instruments** (no change to play):
- `Brain::rung`: the Auto rung a think took, or None if the think did
  not reach the ladder (a reflex, a fleet slot, a job, another goal).
  The Commander keeps `rung_now`, cleared at each think.
- The census counts the trial team's seat-seconds on each rung.
- The meter counts each pick the posture changed, by the rung that held
  first and the rung taken (`Meter::moved`, 6 x 6).
- `CENSUS_ACTS=1`: the trial team's every seat-second by status code
  (`Act::code`), dead seats apart, ladder thinks marked.

**Which rungs** (48 seeds, the same as LR 24 and 25; trial team
seat-seconds; "no lean" is the null run, `board` on for all):

| Rung | No lean | Lean | Lean, Economy off (`w_pos_economy` 0) |
| --- | --- | --- | --- |
| DefendBase | 609 | 381 | 422 |
| Fortify | 14,262 | 9,967 | 10,609 |
| Turrets | 8,427 | 1,026 | 3,961 |
| Hunt | 14 | 245 | 151 |
| Expand | 9,197 | 11,034 | 8,928 |
| Guard | 0 | 31 | 34 |
| Picks changed | 0 | 131,278 | 44,975 |
| d a seat | 0 | -1.04 +/- 0.58 | -0.92 +/- 0.54 |

With the lean, 84% of the changed picks were Fortify to Expand and 12%
Turrets to Expand. The castle stands later, so the Turrets rung, which
fills it with pillboxes, nearly vanishes (-88%). Economy caused two
thirds of the changed picks: building spends material, spent material
reads as Economy, and Economy leans away from building, a feedback.
With Economy off, Turrets recovers about half. **The score does not
move**: Economy off against Economy on, seed by seed, is +0.13 +/-
0.47. The hypothesis is not confirmed. The lean does move tanks off the
castle, but the census cannot see that cost, if it is one.

**Where the time goes** (`CENSUS_ACTS`, 16 seeds, null run, the trial
team's 205,537 seat-seconds, the 40 commonest codes):

| Share | What |
| --- | --- |
| 46.7% | dead (waiting to respawn, or out of respawns) |
| 18.9% | the island hop: sailing 5.5%, bridging 4.4%, by sea 2.1%, idle 1.5%, to the boat 1.5%, holding the launch 1.2%, and the rest |
| 7.3% | rotation (push and defence points) |
| 5.6% | the Auto ladder: fortify 2.1%, turrets 1.2%, take a depot 0.9%, no way to the post 0.7%, gather 0.6% |
| 4.5% | dodging |
| 4.2% | the fleet: wing 2.7%, wing at sea 1.5% |
| 3.9% | rallies and kiting toward a ping |
| 2.4% | taking depots and pillboxes outside the ladder |
| 1.7% | crew jobs |
| 0.7% to 1.9% each | gather 1.9%, revive 1.0%, resupply 0.9%, escort 0.7% |

Three things follow:
1. **The ladder is a small lever.** It decides 5.6% of a team's
   seat-seconds, about 11% of its living time. A change to the
   ladder's pick can only move that share. The posture's lean changed
   most ladder picks and moved the score by a point at most.
2. **Half the time is dead.** A seat dies 3.4 times a match on average
   with 3 respawns; a respawn waits 20 s, so most of the dead time is
   seats out of respawns. The census, as set, measures attrition. A
   part that pays late (the castle's turrets, the escort, bridge jobs,
   the cycle) is cut off before it can.
3. **The island hop is the largest living share** (about 36% of living
   time), on naval.map. Speeding a crossing is worth more than any
   rung.

What this changes. Backlog 28: survival, since what kills half the
team is the largest lever there is. The quad census's "what a seat was
doing when it went down" can now be read by code. Backlog 29: a census
with unlimited respawns, or longer matches (goal 5), so the late parts
are measured at all. The leanings (a 5 x 6 table of 0s and 1s) stay as
they are until the census can see a point.

Proof: no behaviour changed. The lean run of LR 25 run again gives the
same per-seed differences to the hundredth, and the null run reads 0 on
every seed. `make test`: brain 68 passed and 8 ignored; client 31; net 14; proto 3; sim 50. No
warnings.

## 27. What kills half the team, 19 October

Backlog 28, from LR 26: 46.7% of the trial team's seat-seconds are
dead. The hypothesis: what a seat was doing when it went down, read by
code, names the two or three situations that kill most.

**The instruments** (no change to play):
- `CENSUS_ACTS` now also keeps each trial seat's last five codes, one a
  second, and at each `TankDestroyed` counts the code held last, every
  code held in the 5 s before (once a death), and what took the tank
  (`killed_by`: the weapon whose hit names it on that tick, whether it
  sank, and whether the shooter was the lone RuleBot or no one).
- The meter counts the Commander's shell dodges, ashore and afloat
  (`Dodge::metered`), and those hit, those hit by the dodged shell
  itself (on its landing tick), and those killed, between the dodge's
  start and its shell's landing (`Dodge::watch`, `meter::dodge`,
  `meter::dodge_hit`). A dead seat still reads its dodge
  (`Brain::dead`, called each tick in place of a think).
- `Act::Landfall`: afloat with no hop under way, the doctrine's run for
  the beach (`RuleBot::landfall_order`). It read `hop.idle` before.
- The dodge's four constants are weights (`dodge_horizon` 4 s,
  `dodge_margin` 0.8, `dodge_clear` 2.5 tiles) and one new one,
  `dodge_hold` (0): the share of the shell's flight a tank stays at its
  dodge point once there. At 0 it goes back to its errand on arrival,
  as before; at 1 it stays out until the shell lands. The default
  numbers pick the same point as the constants
  (`a_dodge_holds_out_of_the_splash_by_its_weight`).

**The reading** (16 seeds, null run, naval.map, 900 s, respawns 3; the
trial team's 819 deaths, 17.1 a match):

| Code held last | Deaths | Share of deaths | Share of time | Held in the 5 s before |
| --- | --- | --- | --- | --- |
| dodge | 354 | 43.2% | 4.5% | 67.3% |
| landfall | 165 | 20.1% | 3.2% | 30.2% |
| hop.sailing | 65 | 7.9% | 4.7% | 15.4% |
| resupply | 32 | 3.9% | 0.9% | 6.3% |
| hop.holding_launch | 28 | 3.4% | 1.1% | 8.1% |
| hop.bridging | 24 | 2.9% | 4.3% | 4.3% |
| hop.by_sea | 22 | 2.7% | 2.0% | 4.9% |
| everything else living | 129 | 15.8% | 32.9% | |

What took them (the twelve commonest causes, 791 of the 819): direct
fire 253, indirect 275, strikes 246, pillboxes 17; 399 of the 791 sank
in a boat. The commonest pairs: direct fire on a boat at landfall 132
(16.1%), indirect fire on a boat in a dodge 91, indirect fire ashore in
a dodge 87, a strike in a dodge 78.

The dodge's own meter (the same run):

| | Dodges | Hit before the shell landed | By the dodged shell | Killed |
| --- | --- | --- | --- | --- |
| Ashore | 7,911 | 2,691 (34.0%) | 1,314 (16.6%) | 582 (7.4%) |
| Afloat | 3,408 | 621 (18.2%) | 258 (7.6%) | 567 (16.6%) |

Three things follow:
1. **Two situations make 63% of the deaths**: the dodge and the run
   for the beach, in 7.7% of the time. A seat dies once every 26 s it
   is dodging and once every 40 s on landfall, against once every 135
   s of living time overall. The hypothesis is confirmed in its form:
   two situations, not ten.
2. **A dodge is a symptom before it is a cause.** A tank dodges because
   a shell is on it; under fire is where tanks die. That a dodge
   precedes most deaths does not say the dodge kills. Ashore, a sixth
   of dodges are still hit by the shell they dodged, and a third by
   something. Afloat, a hit is a kill 91% of the time: a boat sinks.
   The test is causal: the dodge off (`-dodge`) and `dodge_hold` 1,
   each at 48 seeds against the null run (next step).
3. **Landfall is the second killer, and it is not a dodge.** 132 deaths
   are a boat sunk by direct fire while running for a beach, though
   `landfall_order` prices each beach by the threats near it. A boat
   in range of a tank's gun is under fire it cannot answer.

What this changes. Backlog 28's look is done; its part is the dodge,
measured causally first, then the landfall (backlog 30). The ladder,
rotation and the fleet are a small share of deaths.

Proof: no behaviour changed. The null run reads 0 on every seed (trial
8.38, baseline 8.38). `make test`: brain 69 passed and 8 ignored;
client 31; net 14; proto 3; sim 50. No warnings.

## 28. Holding out of the splash, 19 October

Backlog 28, the first causal reading after LR 27. A sixth of ashore
dodges are caught by the shell they dodged. One reading: the tank
reaches its dodge point early, drops the dodge (`Dodge::holding` holds
only while it drives) and goes back to its errand under the shell. If
so, `dodge_hold` 1, which keeps the tank at its point until the shell
lands, should cut the hits by the dodged shell and save tanks.

**The run**: the defaults dumped (`CENSUS_DUMP`) with `dodge_hold = 1`,
as the trial team's weights; 48 seeds in three ranges of 16
(`CENSUS_SEED0` 0, 16, 32), naval.map, 900 s, respawns 3, the fair
census, the ledger off. Against the null run (the defaults for all).

| | d a seat | Ashore: hit, by the shell, killed | Afloat: hit, by the shell, killed |
| --- | --- | --- | --- |
| Null (LR 27, 16 seeds) | 0 | 34.0%, 16.6%, 7.4% of 7,911 | 18.2%, 7.6%, 16.6% of 3,408 |
| `dodge_hold` 1 (48 seeds) | **-1.18 +/- 0.55** | 34.2%, 16.7%, 7.3% of 23,099 | 15.5%, 6.0%, 13.9% of 10,002 |

The meter counts every Commander's dodges, the two baseline teams' as
well as the trial team's, so its rates move a third as far as the trial
team's own.

**Not confirmed, and worse.** Holding at the dodge point does not cut
the hits by the dodged shell ashore (16.7% against 16.6%). The tank
that is caught is caught before it gets there, not after. The trial
team loses 1.18 a seat, just outside two errors (1.10): a tank that
stands out the whole flight leaves its errand for nothing. Afloat the
rates fall a little, but that difference is diluted by the baseline
teams and the score says the trade is a loss. `dodge_hold` stays 0.

What it says about the dodge: the hits come in the drive, so the
levers are when it starts (`dodge_horizon`) and where it goes
(`dodge_clear`), and whether to dodge at all, which the switch off
(`-dodge`, running) answers next.

Proof: no code changed. The weights file differs from the defaults in
the one line.

## 29. The dodge off, 19 October

Backlog 28, the second causal reading. LR 27 found the dodge last held
at 43% of deaths; LR 28 that holding it longer costs. Does the dodge
save tanks at all, or does it walk them into fire?

**The run**: `CENSUS_CAPS=-dodge` for the trial team, the default for
the others; 48 seeds as in LR 28.

**The dodge pays: -2.45 +/- 0.67 a seat** with it off, outside the
noise at 3.7 errors (by range: -4.11 +/- 1.10, -1.39 +/- 1.14, -1.84
+/- 1.19). It is the largest single part measured since the capped
castle, and it was on all along. The 43% of deaths it precedes are
where the fire is, as LR 27 suspected; without it there would be more.

**Ashore and afloat apart.** With the dodge off, `order_with` still runs
and still meters (`commander.rs`, the order is dropped by `dodges`
after it), so the meter counts the shells the trial team would have
dodged and did not, mixed with the baseline teams' dodges. Taking each
team to draw a third of the shells, the undodged rate is three times
the run's rate less twice the null's:

| | Null (dodged) | `-dodge` run (mixed) | Undodged, estimated |
| --- | --- | --- | --- |
| Ashore, hit by the shell | 16.6% | 21.1% of 20,783 | about 30% |
| Ashore, killed | 7.4% | 8.6% | about 11% |
| Afloat, hit by the shell | 7.6% | 7.8% of 9,109 | about 8% |
| Afloat, killed | 16.6% | 15.9% | about 14.5% |

The estimate leans on equal shares and should be read as a direction.
(LR 30 measured it by team: the shares were not equal, and afloat the
dodge cuts the kills by a fifth. The afloat reading below is
withdrawn.) Ashore, the dodge nearly halves the dodged shell's hits.
**Afloat it does nothing**: a boat is slow to turn, its dodge point is often water
it cannot reach in time, and it is hit at the same rate dodging or not.
What it gives up afloat is its crossing.

What this changes. The dodge stays on. The next dodge part is the
either/or this table names: dodge afloat or keep sailing. A weight,
`dodge_afloat` (1, today's), the share of a boat's dodges taken
(backlog 31). The meter should count a would-be dodge apart from a
taken one, per team, so the next reading needs no estimate.

Proof: no code changed.

## 30. The meter by team, and LR 29 corrected, 19 October

Backlog 31's first part. LR 29 estimated the undodged rates from a
meter that mixed the teams, on the assumption that each team draws a
third of the shells. The meter now counts by the seat's group and
whether the dodge was taken.

**The instrument** (no change to play):
- `meter::trial(mask)`: the census marks the trial team's seats; the
  meter keeps them apart from the rest. `meter::seat(p)`, called by
  `run_bots` for each seat, names the seat the counts after it belong
  to.
- `Dodge::dropped`: set by the Commander from `dodges` before each
  `order_with`. A dodge the brain drops (the dodge off) counts as
  would-be.
- `Meter::dodges` is `[group][taken or dropped][ashore or afloat]` of
  `Dodges { begun, hit, shell, killed }`; `Meter::add_dodges` sums it
  for the census total. Proof:
  `a_dodge_counts_by_its_seat_and_whether_taken`.

**The reading**: `-dodge`, seeds 0 to 15 (LR 29's first range). The 16
per-seed differences are LR 29's to the hundredth, so play did not
change. In the same matches, the trial team's would-be dodges against
the other two teams' taken ones:

| | Taken (the rest) | Dropped (the trial team) |
| --- | --- | --- |
| Ashore, begun | 4,982 | 1,899 |
| Ashore, hit by the shell | 15.5% | 34.0% |
| Ashore, killed | 6.6% | 15.0% |
| Afloat, begun | 2,148 | 1,029 |
| Afloat, hit by the shell | 6.9% | 10.1% |
| Afloat, killed | 14.7% | 18.2% |

Ashore, the dodge more than halves the dodged shell's hits and the
kills, as LR 29 estimated. **Afloat it is not nothing**: it cuts the
shell's hits by a third and the kills by a fifth. LR 29's estimate was
wrong because its assumption was: the trial team without the dodge
drew fewer dodgeable shells (1,899 ashore against about 2,500 for each
other team), being dead more often. Taking the shares as equal hid the
effect afloat.

What this changes. LR 29's "afloat it changes nothing" is withdrawn.
Backlog 31's hypothesis is weakened: a boat's dodge saves a fifth of
the boats it is taken for. `dodge_afloat` can still be worth less than
1 if the crossing time it costs is worth more, but the case is now a
trade for the tuner, not a free gain. Backlog 31 stays, its expected
gain smaller, below 30 (the landfall).

Proof: play unchanged (the per-seed differences). `make test`: brain
70 passed and 8 ignored; client 31; net 14; proto 3; sim 50. No
warnings.

## 31. The run for the beach, 19 October

Backlog 30, from LR 27: 165 of 819 deaths on `landfall`, 132 of them a
boat sunk by direct fire. The hypothesis: `landfall_order` prices a
beach by the threats near it and the run's heading, not the run's
passage through a gun's reach, so boats sail past guns.

**The part**: `landfall_shy` (weights.rs, 0 by Paul's choice, so play
is unchanged; `RuleBot::landfall_shy`, set by the Commander). The run
to each beach is cut into tile steps; each step within a hostile
tank's direct range plus 4, or a pillbox's range plus 2, is exposed.
The beach's score adds `4 x landfall_shy x` the exposed tiles: at 1, a
tile in a gun's reach costs as much as four tiles more sailing. Proof:
`a_shy_boat_runs_for_the_beach_clear_of_the_guns` (a near beach whose
run passes a gun, and a farther one clear: at 0 the near, at 1 the
far).

**The reading**: `landfall_shy = 1` for the trial team, 48 seeds,
`CENSUS_ACTS` on.

| | d a seat | Deaths (seeds 0 to 15) | On landfall | Direct fire, sunk, on landfall |
| --- | --- | --- | --- | --- |
| Null (LR 27) | 0 | 819 | 165 | 132 |
| `landfall_shy` 1 | +0.65 +/- 0.69 | 821 | 169 | 133 |

**Not confirmed.** The score is inside the noise and the deaths it was
aimed at did not move. (LR 32 measured the explanation below and
withdrew it: the boat had seen its killer in the 10 s before in 78% of
landfall deaths.) Over all 48 seeds the landfall deaths were 548
of 2,522 (21.7%), as in the null's 20.1%. Choosing a beach clear of
the guns the boat sees does not save it. The cause has to be the guns
it does not see: sight and direct range are both 10 tiles (the duck's
note in weights.rs), so a gun the boat could choose around is seen
only once the boat is already in its reach. A price on what is seen
cannot help against what is not.

What this changes. `landfall_shy` stays 0. Backlog 30 closes, not
confirmed. Backlog 32: the run for the beach priced by the guns the
team has seen (the duck's remembered guns, `duck_memory`, or the
board's sightings), so a boat knows the beach a teammate saw covered.

Proof: play unchanged at 0 by construction (the exposure returns 0
before any work, and a score plus 0.0 is the same float; no null run
was repeated). `make test`: brain 71
passed and 8 ignored; client 31; net 14; proto 3; sim 50. No warnings.

## 32. Who saw the gun that sank the boat, 19 October

Backlog 32's read, from LR 31: if the boats on landfall die to guns
they never saw, the guns a teammate saw would help.

**The instrument** (no change to play): under `CENSUS_ACTS`, once a
second, each living trial seat's observation records the hostile tanks
it sees. At each trial death whose code held last was `landfall` and
whose killer is named, the killer is classed as seen in the 10 s
before by the boat, by a teammate, by both, or by neither.

**The reading** (16 seeds, null run, in ranges of 6, 5 and 5; 0.00 on
every seed):

| Seen in the 10 s before by | Landfall deaths | Share |
| --- | --- | --- |
| both the boat and a teammate | 128 | 78.0% |
| neither | 35 | 21.3% |
| a teammate only | 1 | 0.6% |
| the boat only | 0 | 0 |
| All with a killer | 164 | |

(LR 35: sight is shared by the whole team, so the split between the
boat and a teammate below differs only by when each was sampled, and
says nothing.)

(LR 34: the killer stood a median 8 tiles from the boat at its death,
inside its 10-tile range, so "seen" here means seen once the gun could
already fire. The conclusion that boats are not blind is withdrawn.)

**LR 31's explanation is withdrawn.** The boat had seen the gun that
sank it in nearly four landfall deaths of five, and a teammate had seen
it too: they cross together. A teammate's sighting adds a gun the boat
had not seen once in 164. Backlog 32's part, pricing the guns the team
has seen, has nothing to work with. The "neither" fifth is an upper
bound, since sight is sampled once a second.

What, then? The boat sees the gun, prices the beaches (with or without
`landfall_shy`), and dies anyway. It sinks at one hit and cannot fire
back. A beach is chosen once; a change waits 2 s and needs 6 better
(`landfall_order`'s `change`). Once a gun is in sight, the boat's
choice is not which beach but whether to land at all, against standing
off out of the gun's range until the beach is clear or the gun moves.
That is an either/or, so a weight: backlog 33.

Proof: play unchanged (0.00 on every seed). `make test`: brain 71
passed and 8 ignored; client 31; net 14; proto 3; sim 50.

## 33. Standing off a covered beach, 19 October

Backlog 33, from LR 32: boats on landfall die to guns they have seen.
The hypothesis: the gun covers the beach, and a boat that waits out of
its reach until it leaves lives.

**The part**: `landfall_wait` (weights.rs; 0 by Paul's choice, so play
is unchanged; `RuleBot::landfall_wait`, set by the Commander). In
`landfall_order`, once the beach is chosen: if a seen hostile tank is
within its direct range plus 4 of that beach, the boat moves to 16
tiles from the gun, on the line from the gun through the boat, or holds
where it is if already that far. It does so until `landfall_wait`
seconds of the crossing are spent, counted from the first stand-off and
not restarted (a gun flickering in and out of sight cannot hold a boat
off for ever), then lands. The wait resets ashore, at a respawn, and
under a hop. `RuleBot::standing_off` and a new code, `stand_off`
(status.rs), let the census count it. Proof:
`a_boat_stands_off_a_covered_beach_for_its_wait` (at 0 the boat lands;
at 10 it backs off to 16 tiles and lands once the 10 s are spent).

**The reading**: `landfall_wait = 10` for the trial team, 48 seeds,
`CENSUS_ACTS` on.

| | d a seat | Deaths (seeds 0 to 15) | On landfall | Stand-off time |
| --- | --- | --- | --- | --- |
| Null (LR 27) | 0 | 819 | 165 | |
| `landfall_wait` 10 | -0.51 +/- 0.63 | 798 | 184 | under 0.07% |

**Not confirmed.** The score is inside the noise, and the landfall
deaths did not fall (573 of 2,460 over 48 seeds, 23.3%). The stand-off
hardly ever fired: under 0.07% of the trial team's seat-seconds (below
the 40 commonest codes in every range), and it was the code held last
at about 13 deaths. A gun the boat sees is rarely within reach of the
beach it chose. **The guns that sink boats on landfall are not at the
landing.** They are near the boat somewhere along its run.

Three parts on the landfall (LR 31 to 33) have now found nothing to
price. Each assumed where the gun is. The next step reads it: at each
landfall death, the killer's distance to the chosen beach and to the
boat, and whether the killer is ashore (backlog 34).

`landfall_wait` stays 0.

Proof: play unchanged at 0 by construction (`landfall_wait` 0 skips the
block). `make test`: brain 72 passed and 8 ignored; client 31; net 14;
proto 3; sim 50. No warnings.

## 34. Where the gun that sinks the boat stands, 19 October

Backlog 34, from LR 31 to 33: three landfall parts each assumed where
the gun is, and none moved the deaths.

**The instrument** (no change to play): `Brain::beach`, the beach a
boat is running for (`RuleBot::landfall`; the Commander's inner
RuleBot's). Under `CENSUS_ACTS`, at each trial death on `landfall` with
a killer, the census records the killer's distance to the boat and to
that beach, the boat's distance to its beach, and whether the killer
was afloat.

**The reading** (16 seeds, null run, in ranges of 6, 5 and 5; 0.00 on
every seed; 164 landfall deaths with a killer, every one with a beach
chosen):

| | Range 0 to 5 | Range 6 to 10 | Range 11 to 15 | All |
| --- | --- | --- | --- | --- |
| Killer to boat, median (quartiles) | 8.1 (7.4, 8.7) | 8.0 (7.5, 9.6) | 8.0 (7.0, 9.4) | about 8 |
| Killer to beach, median (quartiles) | 28.6 (4.1, 33.6) | 28.6 (5.0, 36.8) | 31.8 (6.8, 34.9) | about 29 |
| Boat to beach, median (quartiles) | 24.1 (7.3, 28.4) | 18.3 (6.9, 32.8) | 17.6 (8.1, 30.7) | about 20 |
| Killer within its range + 4 of the beach | 34.0% | 43.3% | 34.1% | 62 of 164, 37.8% |
| Killer nearer the boat than the beach | 60.4% | 53.7% | 68.2% | 98 of 164, 59.8% |
| Killer afloat | 9.4% | 11.9% | 4.5% | 15 of 164, 9.1% |

Three things follow:
1. **The gun is seen when it can already fire.** The killer stood a
   median 8 tiles from the boat, inside its 10-tile direct range, and
   sight is also 10 tiles. LR 32's "the boat had seen its killer in
   the 10 s before" is true, but the sighting came at or inside the
   gun's range, too late to act on. LR 32's reading stands; its
   conclusion that the boats are not blind is withdrawn in this sense:
   they are blind until it is too late.
2. **Two kinds of death.** About two in five killers stood at the beach
   (within 14 tiles of it, the first quartile 4 to 7 tiles), a beach
   chosen a median 20 tiles off and so out of sight. That is why the
   stand-off (LR 33) found nothing to see. About three in five stood
   along the run, nearer the boat than its beach: guns on the coast
   the boat sailed past. Under one in ten were afloat.
3. **The limit is knowledge, not choice.** A boat's own sight cannot
   price either kind in time, and teammates crossing together see the
   same. What the team does not use is what it knows of the coast: the
   team board's hostile influence (`board.rs`, built under `board`,
   read by nothing since LR 24). A beach and a run priced by that
   field could see a held coast before the boat is in range.

What this changes. Backlog 34 closes: the gun stands either at the
beach or along the coast of the run, out of sight until in range.
Backlog 35: price the landfall by the team board's hostile influence,
at the beach and along the run (needs `board`). The parts of LR 31 and
33 (`landfall_shy`, `landfall_wait`) stay at 0; either can be fed the
board's knowledge instead of the boat's sight.

Proof: play unchanged (0.00 on every seed). `make test`: brain 72
passed and 8 ignored; client 31; net 14; proto 3; sim 50. No warnings.

## 35. Known earlier, and lost, 19 October

Backlog 35's read, from LR 34: a boat sees its killer only once it is
in range, so could the team board's hostile influence price the coast
before then?

**What the code says first.** Two facts, read before measuring:
- Sight is shared by the whole team (`World::visible_to`: any allied
  tank or drone within its vision). A seat's observation is the team's.
  LR 32's split between "the boat" and "a teammate" could only differ
  by when each was sampled; that split is vacuous and is marked so.
- The board keeps no memory (`TeamBoard::rebuild` clears its influence
  and threat each second and fills them from what the team sees now).
  It cannot know a gun the team does not see at that moment, which is
  the case LR 34 found. Backlog 35, as written, cannot work.

The question that remains: had the team seen the killer earlier, and
lost it?

**The instrument** (no change to play): under `CENSUS_ACTS`, each
hostile tank in the trial team's sight is logged with its position
once a second. At each landfall death with a killer, the killer is
classed: seen 10 to 60 s before, then within its direct range plus 4
of where the boat died; seen 10 to 60 s before, elsewhere; or not seen
then.

**The reading** (16 seeds, null run; 0.00 on every seed):

| The killer | Deaths | Share |
| --- | --- | --- |
| seen 10 to 60 s before, within reach of where the boat died | 76 | 46.3% |
| seen 10 to 60 s before, elsewhere | 26 | 15.9% |
| not seen 10 to 60 s before | 62 | 37.8% |
| All | 164 | |

**Nearly half the landfall deaths were known earlier.** The team had
seen the gun standing near the place the boat would die, 10 s to a
minute before, then lost sight of it (it moved behind the sight line,
or the team's eyes moved on), and the boat sailed into it. Nothing in
the brain keeps a hostile tank's last position for long. The duck
keeps a seen gun for `duck_memory` (5 s) and only for ducking; the
board for 1 s.

What this changes. Backlog 35 closes: the board forgets. Backlog 36:
the landfall's guns (its `danger`, `toward`, and the `landfall_shy`
and `landfall_wait` checks) include each hostile tank where it was
last seen, for `landfall_memory` seconds (a weight, Paul's default to
ask; 0 is today's). It reaches up to the 46% of landfall deaths whose
killer was seen near there earlier.

Proof: play unchanged (0.00 on every seed). `make test`: brain 72
passed and 8 ignored; client 31; net 14; proto 3; sim 50. No warnings.

## 36. Guns remembered for the landfall, 19 October

Backlog 36, from LR 35: in 46% of landfall deaths the team had seen the
killer near there 10 to 60 s before, and lost it.

**The part**: `landfall_memory` (weights.rs; 0 by Paul's choice, so
play is unchanged; `RuleBot::landfall_memory`, set by the Commander).
`RuleBot::remember_guns`, each tick in `think_map`, keeps each hostile
tank the team sees where it is, and a tank out of sight where it was
last seen, until the memory's seconds are up, the tank is seen again,
or its spot is in a teammate's sight (within 0.9 of `TANK_VISION`)
with no tank there. `RuleBot::known_guns`, the seen and the
remembered, is what `landfall_order` prices: the beach's `danger`, the
run's heading (`toward`), its exposure (`landfall_shy`) and the
stand-off's cover (`landfall_wait`). At 0 the list is the seen tanks in
the same order as before. Proof:
`a_gun_seen_and_lost_still_prices_the_run` (a gun beside the near
beach's run, seen and then lost: at memory 0 the boat takes the near
beach, at 30 the far one, and the near one again after 31 s).

**The reading**: the three landfall weights on together for the trial
team (`landfall_memory` 30, `landfall_shy` 1, `landfall_wait` 10), 48
seeds, `CENSUS_ACTS` on.

| | Null (LR 27, seeds 0 to 15) | All three (seeds 0 to 15) | All three (48 seeds) |
| --- | --- | --- | --- |
| d a seat | 0 | +1.84 +/- 1.13 | **+2.31 +/- 0.58** |
| Deaths | 819 | 747 | 2,298 |
| On landfall (code held last) | 165 | 82 | 281 |
| Killer seen near there 10 to 60 s before | 76 | 16 | 59 |
| Dead share of seat-seconds | 46.5% | 42.2% | |
| Stand-off share of seat-seconds | under 0.07% (LR 33) | 0.17% | 0.16% |

By range: +1.84 +/- 1.13, +3.07 +/- 0.91, +2.00 +/- 0.97.

**Confirmed.** The trial team scores 2.31 a seat more, four standard
errors out, and every range is positive. Landfall deaths halve (165 to
82 on the same seeds), and the deaths the memory was aimed at, a gun
seen near there earlier, fall from 76 to 16. The stand-off now acts,
because remembered guns give it a cover to see. Of the earlier
landfall parts, `landfall_shy` (LR 31) and `landfall_wait` (LR 33)
read nothing useful on their own; with memory they have something to
price.

What is not yet known: which of the three carries the gain. Memory
alone also feeds the beach's `danger` and the run's heading, which
were already on. Backlog 37 takes each away in turn. The defaults are
Paul's: all three stay 0 until he moves them.

Proof: play unchanged at 0 (`known_guns` is the seen tanks in order;
`remember_guns` keeps nothing). `make test`: brain 73 passed and 8
ignored; client 31; net 14; proto 3; sim 50. No warnings.

## 37. Which landfall weight carries the gain, 19 October

Backlog 37, from LR 36: `landfall_memory` 30, `landfall_shy` 1 and
`landfall_wait` 10 together read +2.31 +/- 0.58. Each is taken back to
0 in turn, 48 seeds each, against the null run (the defaults for all),
`CENSUS_ACTS` on.

**Memory off** (`landfall_shy` 1, `landfall_wait` 10,
`landfall_memory` 0):

| | d a seat (48 seeds) | Landfall deaths, seeds 0 to 15 | Killer seen near there earlier |
| --- | --- | --- | --- |
| Null (LR 27, 35) | 0 | 165 | 76 |
| All three (LR 36) | +2.31 +/- 0.58 | 82 | 16 |
| Memory off | **-1.08 +/- 0.53** | 164 | 72 |

By range: -0.82 +/- 0.86, -0.71 +/- 1.06, -1.70 +/- 0.82.

**The memory carries the gain.** Without it, the two weights over the
guns a boat sees cost about a point, just outside two errors, and the
landfall deaths are the null's. That matches LR 31 and 33, each alone,
with the same sign. The difference between this run and LR 36, about
3.4 a seat, is what remembering the guns is worth to the other two.

**Shy off and wait off** (each with the memory on):

| Trial | d a seat (48 seeds) | Landfall deaths, seeds 0 to 15 | All 48 seeds | Killer seen near there earlier (0 to 15) |
| --- | --- | --- | --- | --- |
| Null | 0 | 165 | | 76 |
| All three (LR 36) | **+2.31 +/- 0.58** | 82 | 281 | 16 |
| Memory off | -1.08 +/- 0.53 | 164 | 542 | 72 |
| Shy off (memory, wait) | +1.50 +/- 0.67 | 103 | 341 | 16 |
| Wait off (memory, shy) | +0.69 +/- 0.64 | 113 | 369 | 25 |

By range, shy off: +1.55 +/- 1.04, +1.62 +/- 1.35, +1.33 +/- 1.16;
wait off: +0.26 +/- 1.11, +1.23 +/- 1.07, +0.59 +/- 1.20.

**All three carry part of it, and the memory is the base.** Without the
memory the other two cost a point. With it, the stand-off is worth
about 1.6 a seat (all three against wait off) and the run's exposure
about 0.8 (all three against shy off). Each of those differences is
read across two runs against the null, not paired with each other, so
they are rougher than the table's errors. Shy off alone is outside two
errors (+1.50 +/- 0.67, 2.2 errors); wait off is not (1.1). The
landfall deaths order the runs the same way as the score: all three
fewest, then shy off, wait off, and memory off with the null.

The best measured setting is all three at once. Its numbers (30 s,
1, 10 s) were picked by hand and are not tuned. Moving any default is
Paul's choice.

## 38. The landfall's three on by default, and the reverse reading, 19 October

Paul, after LR 37: "yes all three on". `landfall_memory` 30,
`landfall_shy` 1 and `landfall_wait` 10 are the defaults (weights.rs).
A Commander takes them from its Settings; a plain RuleBot (the lone
aggro bot, and any RuleBot seat) keeps 0, as it keeps the dodge's
constants. Every test passes with the new defaults.

**The reverse reading.** As proof of the switch, the census was run
the other way round: the old defaults (all three 0) as the trial team,
against two teams on the new, 48 seeds.

| | Trial | Baselines | d a seat (48 seeds) |
| --- | --- | --- | --- |
| LR 36 (forward) | all three on | the old defaults | **+2.31 +/- 0.58** |
| Reverse | the old defaults | all three on | **-0.08 +/- 0.45** |

By range, reverse: -0.35 +/- 0.82, -0.77 +/- 0.62, +0.86 +/- 0.86.

**The two do not mirror.** A team with the memory beats teams without
it by 2.3 a seat; a team without it among teams with it loses nothing
that 48 seeds can see. The gain is real against opponents that play
the old way and not shown against opponents that play the new. One
reading, untested: the stand-off and the run's price change where
boats land and which coasts are contested for every team. A memory
team's edge then depends on its opponents landing where the guns are,
and a no-memory team among memory teams finds the coasts less
contested. What the switch does to play when every team has it is
measured by neither census: the census reads a team against others,
not a team against itself.

The defaults stay as Paul chose. Backlog 38: what all three do when
every team has them. That needs a fixed reference the census does not
have: a team of the old defaults kept for comparison, or a score that
is not relative (survival time, landfall deaths a seat, matches
finished).

Proof: `make test`: brain 73 passed and 8 ignored; client 31; net 14;
proto 3; sim 50. No warnings. 360 functions, 121 undocumented.

## 39. The landfall's three when every team has them, 19 October

Backlog 38, from LR 38: the relative census reads +2.31 forward and
-0.08 in reverse. What do the three do to the game when every team has
them?

**The method.** The census sets only the trial team's weights, but a
null run is every team on the defaults. LR 27's null run (seeds 0 to
15) was every team on the old defaults, and play at the old defaults
has not changed since (each new weight at 0 adds nothing). A null run
now, on the same seeds, is every team on the new. Both with
`CENSUS_ACTS`. These are absolute readings, not paired by seed, so they
carry no standard error.

| Seeds 0 to 15, every team | Old defaults (LR 27) | New defaults |
| --- | --- | --- |
| Trial team's deaths | 819 | 848 |
| On landfall (code held last) | 165 | 89 |
| In a dodge (code held last) | 354 | 434 |
| Seat-seconds (the matches' length) | 206,685 | 216,000 (+4.5%) |
| Deaths per 1,000 seat-seconds | 3.96 | 3.93 |
| Dead share | 46.5% | 41.9% |
| Living seat-seconds | 110,615 | 125,471 (+13%) |
| Mean score a seat | 8.38 | 7.95 |

**The deaths move; they do not fall.** With every team on the new
defaults, the boats lost on the run for the beach nearly halve (165 to
89), but the deaths in a dodge rise by about as many (354 to 434), and
the death rate is the same. The tanks that used to sink on the way in
now land, and die ashore under indirect fire. The matches run 4.5%
longer and the team spends 13% more time alive, which is what goal 5
asks for. The mean score a seat falls (8.38 to 7.95). It is a weighted
sum that counts kills and holdings, and it is relative to how the
other teams play, so without per-seed pairing this change cannot be
told from noise.

This explains LR 38's asymmetry. A team with the memory, among teams
without it, saves its boats while its opponents' boats still sink: it
gains. A team without it, among teams with it, still loses boats, but
its opponents' saved tanks die ashore instead, so the relative score
evens out.

What this changes. The defaults stay as Paul chose: the game is longer
and as deadly, with the deaths moved off the water. The next lever is
where the deaths moved to: the dodge ashore, now 51% of the deaths
held last. LR 29 showed the dodge pays; its numbers (`dodge_horizon`,
`dodge_clear`) have never been tuned (backlog 28's close), and the
tuner (backlog 23) is the way to tune them.

Proof: no code changed. `make test` as in LR 38.

## 40. SPSA on the paired census, 19 October

Backlog 23, from CI-001 VI: the (1 + lambda) tuner selects noise. The
recommended replacement is SPSA (Spall 1992), because the fair census
already measures a paired difference.

**The parts**:
- `CENSUS_BASE_WEIGHTS` (fleet.rs `brain_census`): a weights file for
  the two baseline teams, as `CENSUS_WEIGHTS` is for the trial. The
  header line prints both sides' changed weights. With it, one census
  plays theta + c delta against theta - c delta, and every team on
  some old defaults against the new is one run (LR 39 needed two).
- `scripts/spsa.sh` (`make spsa`): SPSA in log space over the weights
  named in `TUNE`, each positive. Each step k draws a sign delta_i for
  every tuned weight and plays one fair census on `SEEDS` fresh seeds
  (`SEED0 + (k - 1) SEEDS`), the trial team on theta_i exp(+c_k
  delta_i), the baselines on theta_i exp(-c_k delta_i). The census's
  difference a seat d is f(+) - f(-); each log theta_i moves by a_k d
  / (2 c_k delta_i), capped at `MAXSTEP` (0.5). The gains follow
  Spall: a_k = A / (k + 1)^0.602, c_k = C / k^0.101 (A 0.6, C 0.4).
  `SET` plants values on the start; `steps.tsv` keeps every step with
  its difference and standard error. A result is promoted only after
  beating the defaults on 48 seeds the search did not use.

**The planted test** (CI-001 VI.D): a weight with a known large effect,
detuned, which a working tuner must move back. `w_wounded`, named in
CI-001, has never been measured, so a failure there would say nothing.
`landfall_memory` has: removing it cost about 3.4 a seat (LR 37). It
is set to 5 s (default 30) and tuned alone: 8 steps of 12 seeds.
The first four steps, as committed with LR 40 (the whole run in LR
41):

| k | d a seat | Its error | landfall_memory after |
| --- | --- | --- | --- |
| 1 | -0.13 | 1.80 | 4.69 |
| 2 | -0.87 | 1.24 | 3.27 |
| 3 | +0.55 | 1.33 | 3.99 |
| 4 | +1.21 | 1.43 | 2.68 |

Every step's difference is inside its error: at 12 seeds a step reads
to about 1.4 a seat, and between memory of 2 s and 5 s the true
difference is likely far smaller. The walk so far is noise. LR 41
gives the whole run.

Proof: `make test`: brain 73 passed and 8 ignored. A plumbing run (1
step, 3 seeds of 120 s) wrote both sides' files, and the header shows
the trial on 4.48 s and the baselines on 2.01 s.

## 41. The planted test, first run: a flat slope, 19 October

LR 40's planted test, whole: `landfall_memory` from 5 s, tuned alone,
8 steps of 12 fresh seeds (1,000 to 1,095), A 0.6, C 0.4.

| k | d a seat | Its error | landfall_memory after |
| --- | --- | --- | --- |
| 1 | -0.13 | 1.80 | 4.69 |
| 2 | -0.87 | 1.24 | 3.27 |
| 3 | +0.55 | 1.33 | 3.99 |
| 4 | +1.21 | 1.43 | 2.68 |
| 5 | -0.59 | 0.88 | 3.21 |
| 6 | -0.84 | 1.09 | 4.05 |
| 7 | +0.36 | 1.57 | 4.45 |
| 8 | -0.77 | 1.14 | 3.69 |

**Not passed.** The weight ended at 3.69 s, below where it started,
after a walk in which no step's difference was outside its error. The
mean of the eight differences, each signed by its draw, is no
different from zero.

**Why, most likely: the plant sat on a flat.** With C 0.4 the two
sides played memories of about 3.4 and 7.5 s around 5 s. The deaths
the memory prevents were sightings 10 to 60 s old (LR 35). Below
about 10 s, memory barely acts, so the slope there is near zero and
each step reads only noise. SPSA was not shown to fail; the test did
not give it a slope. A planted test has to sit where the effect is.

What this changes. The test runs again with C 1.0: the two sides then
play about 13.6 s and 1.8 s, so every step straddles the range where
memory acts. If SPSA climbs from there, the tool works when the slope
is visible. If it does not, 12 seeds a step is too few for this
effect, and the next try is 24.

Proof: no code changed.

## 42. The planted test, second run: it climbs, slowly, 19 October

LR 41's planted test again with C 1.0, so each step's two sides
straddle the range where memory acts. `landfall_memory` from 5 s,
tuned alone, 8 steps of 12 fresh seeds (the same seeds, 1,000 to
1,095), A 0.6.

| k | Plus side (s) | Minus side (s) | d a seat | Its error | The longer side better by | Memory after |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 1.8 | 13.6 | +1.98 | 1.52 | -1.98 | 3.38 |
| 2 | 8.6 | 1.3 | +2.75 | 1.99 | +2.75 | 5.34 |
| 3 | 13.1 | 2.2 | +1.33 | 1.56 | +1.33 | 6.48 |
| 4 | 2.7 | 15.5 | -0.33 | 1.64 | +0.33 | 6.76 |
| 5 | 2.9 | 15.8 | -1.62 | 0.93 | +1.62 | 8.21 |
| 6 | 18.9 | 3.6 | +1.18 | 1.63 | +1.18 | 9.37 |
| 7 | 4.1 | 21.3 | -3.13 | 0.79 | +3.13 | 12.99 |
| 8 | 29.2 | 5.8 | -0.99 | 1.19 | -0.99 | 11.79 |

**Partly passed.** The weight climbed from 5 to 11.8 s, toward the
known better value. Six steps of eight read the longer memory better,
by +0.92 +/- 0.62 a seat on average (1.5 errors), and step 7 alone was
outside its error. With the slope in view, SPSA moves the right way.
It is slow: after 96 seeds it is less than halfway to 30 s in log
terms (ln 11.8 - ln 5 = 0.86 of 1.79). With Spall's decaying gains it
would take about as many steps again.

What it says for the dodge. The planted effect was about 3.4 a seat
between 0 and 30 s (LR 37), the largest part measured, and SPSA at 12
seeds a step only crawls toward it. The dodge's numbers act on
something smaller. Tuning them is a long run: 24 seeds a step (errors
about 0.8), A 1.0, and 16 or more steps, about 2.5 hours. The result
then has to beat the defaults on 48 seeds the search did not use.

Proof: no code changed.

## 43. The dodge's numbers by SPSA, 19 October

LR 42's long run: `dodge_horizon`, `dodge_margin` and `dodge_clear`
from their defaults (4 s, 0.8, 2.5 tiles), 16 steps of 24 fresh seeds
(2,000 on, used by no earlier run), A 1.0, C 0.5. The sixteen steps:

| k | Plus side (horizon, margin, clear) | Minus side | d a seat | Its error | After |
| --- | --- | --- | --- | --- | --- |
| 1 | 2.43, 1.32, 1.52 | 6.59, 0.49, 4.12 | +0.24 | 0.77 | 3.41, 0.94, 2.13 |
| 2 | 5.43, 1.50, 3.39 | 2.14, 0.59, 1.34 | -0.48 | 0.86 | 2.61, 0.72, 1.63 |
| 3 | 1.67, 1.13, 1.04 | 4.08, 0.46, 2.55 | -0.34 | 0.74 | 3.07, 0.61, 1.92 |
| 4 | 4.74, 0.94, 2.97 | 1.99, 0.40, 1.24 | +0.47 | 0.94 | 3.77, 0.75, 2.36 |
| 5 | 5.77, 1.15, 1.54 | 2.47, 0.49, 3.61 | -0.99 | 0.76 | 2.54, 0.50, 3.51 |
| 6 | 1.67, 0.77, 5.32 | 3.85, 0.33, 2.31 | +1.72 | 1.18 | 1.54, 0.83, 5.78 |
| 7 | 2.32, 1.26, 8.72 | 1.02, 0.55, 3.83 | +0.66 | 0.74 | 1.94, 1.05, 7.28 |
| 8 | 1.29, 0.70, 4.86 | 2.91, 1.57, 10.92 | +0.66 | 1.10 | 1.56, 0.84, 5.87 |
| 9 | 1.05, 0.57, 8.76 | 2.33, 1.26, 3.93 | **-2.49** | 0.94 | 2.58, 1.39, 3.56 |
| 10 | 1.73, 0.94, 5.29 | 3.83, 2.07, 2.39 | +0.24 | 0.88 | 2.40, 1.30, 3.82 |
| 11 | 3.55, 0.88, 2.58 | 1.62, 1.92, 5.66 | **-2.35** | 0.72 | 1.46, 2.14, 6.30 |
| 12 | 0.99, 3.16, 4.27 | 2.15, 1.45, 9.29 | +0.00 | 0.90 | 1.45, 2.14, 6.30 |
| 13 | 0.99, 3.15, 9.26 | 2.14, 1.46, 4.28 | -1.64 | 1.12 | 2.24, 1.39, 4.08 |
| 14 | 1.53, 2.03, 2.78 | 3.29, 0.95, 5.98 | +0.67 | 1.06 | 1.89, 1.65, 3.44 |
| 15 | 2.77, 1.12, 5.03 | 1.29, 2.41, 2.35 | -0.38 | 0.89 | 1.72, 1.81, 3.13 |
| 16 | 1.18, 2.64, 2.15 | 2.51, 1.24, 4.57 | -0.73 | 0.79 | 2.05, 1.52, 3.73 |

Step 9 is the first outside its error (2.7 errors): a horizon of about
1 s, with the narrower margin and the wider clearance, lost by 2.5 a
seat to one of about 2.3 s. The early noisy steps had walked the
horizon down to where a tank starts too late to get out from under a
shell, and SPSA walked it back. Before step 9, no step was outside
its error. The gain is too high for this noise: at
A 1.0, readings inside their errors move every weight by up to the cap
(steps 5 and 6 hit it). `dodge_clear` is the most consistent mover.
Signed toward the side with the larger clearance, the first seven
readings average +0.49 +/- 0.28 a seat (1.7 errors); with step 8,
which read the smaller clearance better, +0.35 +/- 0.28 (1.3 errors), though each also carries
the other two weights' perturbations. The run goes on; if it ends far
from the defaults on noise-sized readings, it runs again at A 0.3, and
any end point is confirmed against the defaults on 48 unused seeds
before it can be a default.

**Two steps of sixteen** were outside their errors, 9 and 11. In both,
the side with the larger margin won (1.26 over 0.57; 1.92 over 0.88).
On the horizon they disagree (step 9: about 1 s lost to 2.3 s; step
11: 3.6 s lost to 1.6 s), and step 13 (inside its error) had about 1
s lose again. The end point: **`dodge_horizon` 2.05 s, `dodge_margin`
1.52, `dodge_clear` 3.73 tiles**, against the defaults of 4, 0.8 and
2.5. Start later, count more near misses, and clear further.

**The confirmation**: the end point as the trial team's weights, the
defaults for the baselines, the 48 standard seeds (0 to 47, which the
search, on 2,000 to 2,383, never saw), `CENSUS_ACTS` on.

| | d a seat (48 seeds) | By range |
| --- | --- | --- |
| End point against the defaults | **+1.24 +/- 0.75** | +0.36 +/- 1.59, +1.96 +/- 1.24, +1.39 +/- 1.06 |

| Dodges taken ashore, 48 seeds | Begun | Hit before the shell landed | By the dodged shell | Killed |
| --- | --- | --- | --- | --- |
| The trial team (end point) | 8,730 | 29.0% | 20.2% | **6.4%** |
| The two baseline teams (defaults) | 17,457 | 35.0% | 16.9% | 7.4% |

**Promising, not confirmed.** +1.24 is 1.65 errors from zero, inside
the two-error rule, though all three ranges are positive. The meter
agrees in direction: with the end point a dodging tank is caught by
its own shell more often (it starts later), but hit by anything less
often and killed less often (6.4% against 7.4%). Starting a dodge 4 s
out takes a tank off its errand and into other fire; starting at 2 s
keeps it where it was until the shell is close.

The defaults stay. The decision rule, set before the next reading: a
second fixed batch of 48 seeds (48 to 95); the end point is promoted
if, and only if, the 96 seeds pooled read past two errors. No further
seeds are added after that.

On the tuner. SPSA moved three noisy weights to a point that reads
+1.24 on unseen seeds, from readings of which only two were outside
their errors. With A 1.0 the walk was rough, and steps 5, 6 and 11 hit
the cap. The next run should use A 0.3 and, for three weights, 32
seeds a step.

**The second batch** (seeds 48 to 95, the rule above set before it
ran):

| | d a seat | Errors from zero |
| --- | --- | --- |
| Seeds 0 to 47 | +1.24 +/- 0.75 | 1.65 |
| Seeds 48 to 95 | +0.53 +/- 0.69 | 0.76 |
| **All 96, pooled** | **+0.88 +/- 0.51** | **1.74** |

By range, second batch: +0.30 +/- 1.30, +1.89 +/- 1.18, -0.62 +/- 1.07.

**Not promoted.** By the rule set before it ran, the pooled 96 seeds
had to pass two errors, and they read 1.74. The defaults stay (4, 0.8,
2.5), and this end point gets no more seeds. If the dodge's numbers
are worth something, it is under a point a seat, smaller than the
census resolves at 96 seeds. A new search, at A 0.3 and 32 seeds a
step, would start a new hypothesis with its own confirmation. Backlog
23 closes: the tool is built, it found a lead, and the lead did not
hold.

## 44. Each switch alone, without running out of tanks, 20 October

Backlog 29, from LR 26: with 3 respawns, 46.7% of seat-time is dead,
and a part that pays late (the base cycle, the escort, bridge jobs,
the turrets) is cut off before it can. The census as set measures
attrition. The hypothesis: with unlimited respawns the late parts read
differently from LR 24.

**The run**: `RESPAWNS=255 JOBS=3 make switches`. That is every
capability off by default, alone, against today's defaults (the
landfall's three on, LR 38), 12 seeds of 900 s on naval.map,
`RESPAWNS_UNLIMITED` (255; the roadmap's 254 was a slip). Every match
runs the full 900 s. `team_rotate`, `bridge_jobs` and `posture` are
tried with `board` on for every team.

| Switch | Unlimited respawns: d a seat | Its error | Verdict | LR 24 (3 respawns, old defaults) |
| --- | --- | --- | --- | --- |
| `smooth` | **-3.32** | 1.47 | **outside: worse** | inside |
| `posture` (board) | -1.67 | 1.24 | inside | -0.81 +/- 0.65 (48 seeds) |
| `priced_crossing` | +1.37 | 1.23 | inside | inside |
| `team_rotate` (board) | +1.09 | 1.20 | inside | inside |
| `bridge_jobs` (board) | +1.06 | 0.91 | inside | inside |
| `forget` | +0.60 | 0.33 | inside (1.8) | inside |
| `cycle` | -0.60 | 0.46 | inside | inside |
| `duck` | -0.51 | 1.59 | inside | inside |
| `escort` | -0.34 | 0.34 | inside | never acted |
| `column` | +0.11 | 1.31 | inside | inside |
| `board` | 0.00 | 0.00 | never acts alone | never acted |
| `island_site` | 0.00 | 0.00 | never acts | never acted |

What it shows so far:
1. **The late parts are no better with time.** The base cycle reads
   -0.60, the escort -0.34, bridge jobs +1.06, all inside the noise.
   The escort now acts (LR 24: never), and does not pay.
2. **`smooth` is worse, outside the noise** (2.3 errors). It was off
   already; this is the first clear reading of its cost.
3. **Two parts still never act**: `board` alone (nothing reads it
   unless a part that uses it is on) and `island_site`.

**The side-by-side**: the same run at 3 respawns under today's
defaults (`RESPAWNS=3 JOBS=3 make switches`), and LR 24's figures (3
respawns, the defaults before the landfall's three), all 12 seeds.

| Switch | LR 24 | 3 respawns, today | Unlimited, today |
| --- | --- | --- | --- |
| `bridge_jobs` (board) | +0.44 +/- 0.91 | **+1.68 +/- 0.70** | +1.06 +/- 0.91 |
| `duck` | +0.39 +/- 1.04 | **-2.26 +/- 1.05** | -0.51 +/- 1.59 |
| `smooth` | -1.98 +/- 1.25 | -1.14 +/- 0.93 | **-3.32 +/- 1.47** |
| `team_rotate` (board) | -0.83 +/- 0.87 | +0.91 +/- 0.92 | +1.09 +/- 1.20 |
| `posture` (board) | (-0.81 +/- 0.65, 48 seeds) | -0.76 +/- 1.37 | -1.67 +/- 1.24 |
| `column` | -0.06 +/- 0.41 | +0.63 +/- 0.61 | +0.11 +/- 1.31 |
| `priced_crossing` | +0.11 +/- 0.56 | +0.36 +/- 0.67 | +1.37 +/- 1.23 |
| `cycle` | +0.31 +/- 0.43 | +0.02 +/- 0.19 | -0.60 +/- 0.46 |
| `escort` | +0.01 +/- 0.01 | +0.02 +/- 0.05 | -0.34 +/- 0.34 |
| `forget` | -0.36 +/- 0.23 | -0.00 +/- 0.15 | +0.60 +/- 0.33 |
| `board`, `island_site` | 0 | 0 | 0 |

Bold is outside two errors.

**Many tests, few findings.** Two passes of twelve are 24 tests at
the two-error line, where about one reading in twenty falls outside by
chance: about one false finding is expected, and three appeared.
Only `smooth` has the consistency to stand on: it is negative in all
four readings on record (-1.92 in LR 20's list, -1.98 in LR 24, and
both passes here). `bridge_jobs` (+1.68) and `duck` (-2.26) are each
outside in one pass and inside in the other. They are leads, to be
confirmed on 48 fresh seeds before anything changes (backlog 39).

**Backlog 29 answered.** The late parts do not read differently given
time: the base cycle +0.02 and -0.60, the escort +0.02 and -0.34, all
inside the noise. The hypothesis that the census hid them by
attrition is not confirmed. With today's landfall defaults the dead
share is already lower (LR 39), and unlimited respawns did not
uncover them either.

`smooth`: Paul, on reading the first pass, "smooth sounds like we
should remove it", then "maybe not remove it. give it an insignificant
coefficient". It becomes a weight (LR 45).

## 45. A frozen census binary, 20 October

Backlog 41, from Paul: "is there an improved and faster way for you to
test ?" Every census ran `cargo test`, which rebuilds the brain if the
source has changed. So no brain code could be edited while a census
ran, and every long run was idle time for the work.

**The part**: `census_bin DIR` (scripts/common.sh) builds the brain's
tests once, finds the test binary in cargo's JSON output, copies it to
`DIR/census-bin`, and prints the path. `switches.sh`, `spsa.sh` and
`tune.sh` run the copy (`"$BIN" --ignored brain_census --nocapture`).
A run then plays the code as it was when it began. The copy sits with
the run's logs, so the binary behind any result is kept beside it.

**The proof**: a 1-step SPSA plumbing run (`SET="landfall_memory=3"`,
3 seeds of 120 s) started, and 20 s in, a line that is not Rust was
appended to `fleet.rs`. The run finished. Its reading, d -0.4567 +/-
0.4054 with memory moved to 2.3940, is identical to the first plumbing
run of LR 40, which drew the same sign on the same seeds and code.
`fleet.rs` was then restored from a copy; `git diff` shows the tree
clean.

The other ways to test faster (LR 44's companion, CI-020 V.3) stay on
the list: metrics of the part's own counts, scenario files (backlog
24), more cores, a cheaper match (finding L2), sequential rules set in
advance.

## 46. `smooth` as a weight, 20 October

Backlog 40, from Paul on LR 44's table: "smooth sounds like we should
remove it.", then "ok. maybe not remove it. give it an insignificant
coefficient". `smooth` was one switch over six behaviours, and it read
negative in all four readings on record.

**The part**: the capability `smooth` is gone (its bit, 9, retired),
and a weight, `w_smooth`, 0 to 1, default **0.05**, takes its place.
Read through `Settings::smooth` (clamped) and `Settings::smooth_on`
(above 0.5).

| Behaviour | Where | How `w_smooth` acts |
| --- | --- | --- |
| The move's desire vector | `Commander::go` | the share of the new pull taken blends from 1 (at 0) to `alpha` (at 1): `smooth_alpha`; 0.97 at the default |
| The retreat thresholds | `Commander::needs_home` | blend from the constants (4 shells, 35 armour) to `retreat_shells` and `retreat_armor`; equal today, so no change until those are tuned |
| The rung's confirmation | `Commander::auto` | above 0.5 (a count) |
| The turret's confirmation | `Tactician.confirm` | above 0.5 (a count) |
| The stance's advance scalar | `Tactician.smooth` | above 0.5: at any share above 0 it is a different rule (press above the band's middle), not a softer one, so it is yes or no |
| Resupply by the doctrine alone | `RuleBot.own_resupply` | above 0.5 (yes or no) |

A file or census that names `smooth` as a capability reads it as
`w_smooth` 1, and `-smooth` as 0, so old weights files and `make
switches` still work. Proof: `smooth_is_a_weight_that_blends_and_thresholds`
(0.05 near off, the alpha blend in a line, the threshold strictly
above 0.5, clamped at 1); `a_weights_file_round_trips_and_names_its_mistakes`
(the old `caps = ..., smooth` reads as the weight).

**The reading**: today's play before the change was `smooth` off,
`w_smooth` 0. As the trial team, against two teams on the new default
0.05, 48 seeds, from a frozen binary (LR 45):

| Trial | Baselines | d a seat (48 seeds) | By range |
| --- | --- | --- | --- |
| `w_smooth` 0 (the old play) | 0.05 (the default) | +0.47 +/- 0.60 | -0.96 +/- 1.21, +0.46 +/- 0.89, +1.91 +/- 0.92 |

**Insignificant, as asked.** At 0.05 only the desire vector acts, a
move taking 97% of each new pull rather than all of it, and the census
cannot tell it from the old play. The behaviours stay in the brain,
reachable by one number, for the tuner or a slider.

Proof: `make test`: brain 74 passed and 8 ignored; client 31; net 14;
proto 3; sim 50. No warnings. 363 functions, 121 undocumented.

## 47. The two leads on fresh seeds, 20 October

Backlog 39, from LR 44: `bridge_jobs` (+1.68 +/- 0.70) and `duck`
(-2.26 +/- 1.05) were each outside two errors in one pass of 24 tests.
The rule, set before the run: each is confirmed only past two errors,
in the same direction, on 48 fresh seeds (96 to 143) alone. Each was
run as its trial team against today's defaults (`bridge_jobs` with
`board` on for every team), from a frozen binary.

| Lead | LR 44 (one pass) | Seeds 96 to 143 | Verdict |
| --- | --- | --- | --- |
| `bridge_jobs` helps | +1.68 +/- 0.70 | +0.07 +/- 0.41 | not confirmed |
| `duck` hurts | -2.26 +/- 1.05 | **+1.25 +/- 0.52** | not confirmed: past two errors the other way |

**Neither lead holds.** `bridge_jobs` reads nothing on fresh seeds: its
LR 44 reading was the chance finding the table predicted. `duck` reads
past two errors with the opposite sign. Its readings on record:
-0.51 (LR 21), +0.39 (LR 24), -0.51 and -2.26 (LR 44), +1.25 here. A
part whose sign changes with the seeds has an effect smaller than the
census resolves at 12 seeds, and possibly one that depends on the
map's starts. "`duck` helps" is a new hypothesis, set by this reading,
and needs its own fresh seeds before `duck` could be turned on
(backlog 42). Both stay off.

The lesson repeats LR 43's: a reading from a table of tests, or a
tuner's best point, regresses on fresh seeds. Of four leads confirmed
this way since LR 36, one held (the landfall's memory, consistent in
every run), and three did not.

Proof: no code changed.

## 48. Scenario files, 20 October

Backlog 24, from Paul (18 October): "is there a way to create
description and try to attempt these via playtests faster", and his
question of 20 October, "is there an improved and faster way for you
to test ?". A census answers "does it win?" in 25 minutes. A single
behaviour, such as "does the boat steer clear?", should be answerable in
seconds, written as text rather than as a Rust test.

**The part**: `scenario.rs`, and `scenarios/*.toml`.
- **The form**, a part of TOML read in the brain (no new crate):
  `key = value`, numbers, "strings", true and false, `[lists]`,
  `{ inline = tables }`, `[expect]`, and `[[brain]]`, `[[pillbox]]`,
  `[[depot]]`, `[[shell]]`, `[[event]]`. Positions are anchors with
  offsets: `"seat:0"`, `"depot:0"`, `"pillbox:0"`.
- **The setup**: `players`, `seconds`, `pillboxes = false`. A
  `[[brain]]` is a `rulebot` or a `commander` with a goal
  (`hunt_pillboxes`, `defend_depot:0`, `auto`, ...); a seat without
  one sits still, as in the Rust tests. A `[[pillbox]]` is placed as
  `one_pillbox` places it; a `[[depot]]` is the nearest, given an
  owner; a `[[shell]]` lands on an anchor after `lands_in` seconds; an
  `[[event]]` moves a seat's tank at a second.
- **The checks** (`[expect]`): `hits_on`, `hits`, `moved`,
  `shells_spent`, `pillbox_owned_by` (with `by`), `pillbox_damage`,
  `pillbox_alive`. A run reports every failed check in words, not the
  first.
- **The runners**: `every_scenario_holds` runs every file under
  `scenarios/` in `cargo test`; `make scenario S=file` runs one and
  prints each thinking seat's status code every 5 s, and when the
  match ends (the world stops then, as in the Rust tests; the trace now
  says so).

**The proof** (backlog 24: "three of today's tests rewritten as
scenarios pass and fail as they do"). Three tests rewritten:
`a_bot_drives_out_from_under_a_shell` (lib.rs),
`a_commander_takes_a_neutral_pillbox_without_a_shot` and
`a_commander_defends_its_depot` (commander.rs). Each Rust test and its
scenario was run unchanged, then under two deliberate breaks of the
code, the files restored by a trap on every path:

| | Rust test | Scenario |
| --- | --- | --- |
| Unmutated: the three | pass | pass |
| A, the dodge never acts: shell | FAIL | FAIL |
| B, the Commander never moves: neutral pillbox | FAIL | FAIL |
| B, the Commander never moves: depot | pass | pass |

They agree in every case. (Under B the defender still hits an intruder
placed 5 tiles from its depot without moving, in both.) The three run
in 0.02 s together: two tanks a world, so a step costs microseconds.

What this changes. A new behaviour can be specified as a scenario
before it is built, run in a second, and kept as a test, and the
census kept for whether it wins. The Rust tests stay for now; whether
to retire the ported three is Paul's call. Next for the form: a
`[[gun]]` or `[[tank]]` placement for the landfall's situations, and
a census-sized mode, a scenario over many seeds with its rate and
error.

Proof: `make test`: brain 76 passed and 9 ignored; client 31; net 14;
proto 3; sim 50. No warnings. 18 files, 374 functions, 121
undocumented.

## 49. The landfall as a scenario, 20 October

LR 48's next step: scenario placements for the landfall, so its
situations can be tested in seconds rather than by census.

**The form grows** (`scenario.rs`):
- `teams = [0, 1, 0]`: a team for each seat.
- `[[terrain]]`: a rectangle of tiles around an anchor, `x` and `y` in
  tiles from it, set to water or grass. A sea and its coasts are carved
  this way.
- `[[tank]]`: a seat's tank put at an anchor and offset at the start,
  afloat with `boat = true`.
- `"centre:0"`: the map's centre, as an anchor.
- `[[brain]] weights = { name = value }`: a Commander's Settings with
  those changed; on a RuleBot, which reads no Settings, the landfall's
  three, its only weights held as fields.
- `[[event]]` gains `boat = true` (put to sea) and `drones = "recall"`.
- `[expect]` gains `ends_near` (where a seat finishes) and
  `lands_near` (where it first comes ashore).
- The outcome keeps every order the world refused a thinking seat, and
  the trace shows a RuleBot's status text beside its code.

**The scenario**, `a-boat-remembers-the-gun-on-the-near-beach.toml`: a
RuleBot ashore beside a hostile tank sees it; at 1 s it is put to sea
between a near coast (12 tiles east, by the gun, now out of sight) and
a far one (22 west); its drones are recalled. With the landfall's
defaults it must come ashore within 10 tiles of the far coast.

| `landfall_memory` | Came ashore at | Verdict |
| --- | --- | --- |
| 30 (the default) | (42.8, 64.5), the far coast | holds |
| 0 | (76.2, 56.3), the near coast by the gun | fails: 35.1 tiles from the point |

**Three false starts, each a finding.** The first two versions held
with and without memory, so they tested nothing:
1. ~~A Commander on Auto that starts at sea never prices a beach.~~
   **Withdrawn (LR 50):** it does. The trace sampled every 5 s, and the
   boat had landed before the first sample, so every sample read the
   rung it took ashore. What was true: a Commander built by
   `Commander::new` ran the landfall's weights at 0 while its Settings
   said the defaults (LR 50).
2. **The first choice binds.** With the gun seen at t = 0 through a
   teammate, the boat chose the far coast at once. The landfall changes
   beach only after 2 s and for one 6 tiles better, and by then the
   distances had closed. Memory can only matter when the gun is seen
   and lost before the boat chooses.
3. **A drone sees for its tank.** A tank moved to sea by hand left its
   recon drone ashore, still seeing the gun, so even at memory 0 the
   boat knew it. Recalling the drones made the test honest.

One oddity is kept, not chased: at memory 0 the boat first headed west
and later turned east. The landing separates the two cases cleanly.

What this changes. The landfall's memory now has a test that runs in a
second and fails when the memory is off, beside its census result
(LR 36). The three false starts are why a scenario must be run broken
before it is trusted, and the first of them (withdrawn in LR 50) is
why a trace must sample faster than the thing it watches.

Proof: `make test`: brain 76 passed and 9 ignored. No warnings.

## 50. The circle at the edge of sight, 20 October

Backlog 43, from LR 49: "a Commander on Auto at sea never prices a
beach". The scenario runner gained `trace_every` (seconds between trace
lines) and each seat's position in the trace. At one line a second the
claim fell at once: the Commander's afloat branch runs before the
ladder, and the boat had landed in under 5 s, before the first 5 s
sample. LR 49's finding 1 and backlog 43 are withdrawn.

**What the fine trace found instead.**

1. **`Commander::new` held the default Settings but did not apply
   them.** Its inner RuleBot kept the landfall's weights at 0 while
   `settings` said the defaults. Dealt bots were not affected: `equip`
   (fleet.rs) and the window's tuning page (`tune.rs`) call
   `set_settings`. Tests, scenarios and any Commander built directly
   were. Fixed: `Commander::new` ends with
   `set_settings(Settings::default())`. The brain's tests all pass. A
   census re-run of LR 46's first range checks dealt play is unchanged.

2. **Without memory, a boat circles at the edge of sight.** A
   Commander on Auto at sea between a coast 12 tiles east, by a
   hostile tank it saw and lost, and one 22 tiles west, traced every
   second at `landfall_memory` 0:

   | s | Boat at | Making for |
   | --- | --- | --- |
   | 1.9 | (68.5, 63.9) | the near beach (76, 64) |
   | 3.0 | (74.2, 64.3) | (76, 64) |
   | 4.0 | (73.8, 67.1) | the far beach (42, 64): the gun seen at about 8 tiles |
   | 5.0 | (68.1, 66.7) | (42, 64) |
   | 6.0 | (68.6, 63.9) | (76, 66): the gun out of sight, forgotten |
   | 8.0 | (72.7, 68.5) | (42, 66) |
   | ... | ... | the same, every 4 s, drifting south |

   It sails toward the near beach, sees the gun, turns away, loses it
   at 10 tiles, forgets it, and turns back. It is ashore at **40 s**,
   beside the gun. With the default memory the same boat keeps the gun
   in mind, sails straight for the far coast, and is ashore at **6 s**.
   This is the mechanism behind LR 36's gain and LR 35's finding that
   the killer had been seen near there earlier: without memory, boats
   circled where the guns could reach them.

**The test**: `a-commander-boat-does-not-circle.toml`, the Commander
form of LR 49's scenario. It holds at the defaults, and with
`landfall_memory = 0` it fails ("came ashore 35.8 tiles from the
point").

The check of dealt play (LR 46's seeds 0 to 15, the same weights, the
new binary) is LR 51.

Proof: `make test`: brain 76 passed and 9 ignored. No warnings.

## 51. Dealt play unchanged by the Settings fix, 20 October

LR 50 changed `Commander::new` to apply its default Settings. A dealt
bot already had them applied (`equip`), so the census should play
exactly as before. The check: LR 46's first range (seeds 0 to 15,
`w_smooth` 0 as the trial against the defaults) again, from a binary
built after the fix.

| Run | The 16 per-seed differences |
| --- | --- |
| LR 46, before the fix | -1.68 -0.58 -0.18 +1.76 -3.31 +1.45 -0.59 -5.96 -0.60 +2.52 -4.61 +4.94 +2.74 -0.95 +6.65 +5.81 |
| After the fix | -1.68 -0.58 -0.18 +1.76 -3.31 +1.45 -0.59 -5.96 -0.60 +2.52 -4.61 +4.94 +2.74 -0.95 +6.65 +5.81 |

Identical to the hundredth: the census, and every measurement in this
report, played Commanders with their Settings applied all along. Only
Commanders built outside a deal were affected.

## 52. Scenarios over many seeds, and a dodge into water, 20 October

**The mode.** `scenario::many` runs a scenario over many seeds: the
world's seed (`Scenario::seed`, so the map outside what the file
carves) and every brain's (its seed xor the run's). It reports how
often it held and the binomial error, sqrt(p (1 - p) / n).
`SCENARIO_SET="name=value"` sets a weight on every brain
(`set_weights`), so two settings run on the same seeds. `make scenario
S=file RUNS=100 SET=...`; `SEED=n` traces one seed. Proof:
`a_weight_is_set_on_every_brain`. Brains in a scenario now think with
the map (`think_on`), as `run_bots` gives it in a match. That changed
no result.

**Every scenario over 100 seeds** (each takes 0.2 to 2.5 s):

| Scenario | Held |
| --- | --- |
| a boat remembers the gun on the near beach | 100 |
| a bot drives out from under a shell | 97 (before the fix below) |
| a commander's boat does not circle | 100; with `landfall_memory` 0, **0** |
| a commander defends its depot | 99 |
| a commander takes a neutral pillbox | 100 |

**The memory's curve**, the circling scenario at each memory, 100 seeds:
0 s holds 0 times; 2, 4, 6, 8, 10, 15, 20, 30 and 60 s each hold 100
times. In this geometry it is a step, not a slope: any memory longer
than the landfall's two-second commitment breaks the circle. That is
also why LR 41's planted test found nothing to climb between 3 and 7
s. The census's view (LR 35: killers seen 10 to 60 s before) still
argues for a long memory in matches, where guns move and boats cross
far.

**The dodge into water.** On 3 of 100 seeds the shell scenario failed.
`SEED=8` traced at a quarter second: the dodge's point, across the
shell's line on the near side, was water. The world refused the move
("refused: water"), the shell was already marked dodged, and the tank
sat under it for 3 s. `Dodge::order_on` (lib.rs) now takes the map,
which both callers hold. It tries the near side, the far side, back
along the line, and forward, and takes the first point the tank can
reach: passable ground ashore; afloat, water or a beach (a boat that
runs ashore lands). Without a map it acts as before (`order_with`).
The shell scenario then held on **1,000 of 1,000** seeds.

**Its effect in matches.** A change to a reflex every team has cannot
be paired against itself in the census, so the same 48 seeds were run
as a null before and after, from two frozen binaries (the tree without
the fix by `git stash`, then with it), `CENSUS_ACTS` on. The first
version required water afloat; it made boats worse, so it was
corrected to water or a beach.

| Same 48 seeds, every team | Before | Water only afloat | Water or a beach afloat |
| --- | --- | --- | --- |
| Deaths (the trial seat's team) | 2,472 | 2,390 | **2,297 (-7.1%)** |
| Dead share of seat-time | 42.5% | 40.9% | **40.6%** |
| Ashore dodges: hit / by the dodged shell / killed | 32.3% / 15.7% / 6.5% | 24.0% / 10.6% / 4.1% | **25.5% / 10.4% / 5.1%** |
| Afloat dodges: hit / by the dodged shell / killed | 16.3% / 7.1% / 14.8% | 17.4% / 7.9% / 16.6% | **15.4% / 6.6% / 14.0%** |

Ashore, a third fewer dodges are caught by their own shell. Afloat,
the beach restores and slightly betters the old rate. 175 fewer deaths
on the same seeds. These are absolute counts from two unpaired runs:
no error is given, and the mean score a seat (8.70, 8.81) is relative
to how every team plays, so it says nothing here.

What this changes. The census needed an hour to show the memory's
gain; a scenario over 100 seeds shows its mechanism in under two
seconds, and its first use found a bug that had been in the dodge
since it was written. A fix to a reflex every team shares is read by
before-and-after nulls on the same seeds, with absolute counts.

Proof: `make test`: brain 77 passed and 10 ignored; client 31; net 14;
proto 3; sim 50. No warnings. 378 functions, 121 undocumented.

## 53. The dodge planned by the tank's own motion, and a hold that lets go, 20 October

LR 52 left ashore dodges caught by their own shell at 10.4% after the
water fix. The dodge picks its point from the shell's line alone: 5
tiles from the impact, across the line, on the side the tank is on. It
reads neither the tank's heading nor its speed. Since the scatter is
drawn at launch (treadcore-sim `impact: *at + scatter`), the impact a
bot reads is exactly where the shell lands, and the line it came in on
says nothing once it is fired.

**The instrument.** Two scenario families, each one file that draws
its situation from the run's seed (`scenario.rs`): `[[tank]]` gains
`speed`, `heading` (degrees, or `"any"`) and a goal `drive` tiles on,
so it starts rolling; `[[shell]]` gains `offset`, `scatter` (up to that
many tiles off, by the dice), `from_any` (the direction by the dice)
and `lands_in = [lo, hi]`. A Commander on Auto, a shell from 30 tiles
landing within 2.5 tiles of its tank in 1.5 to 3.5 s, rolling at full
speed on any heading or standing (`scenarios/families/`). The
trace now shows where each tank's route ends and where each shell lands.

| 1,000 seeds each | Rolling | Standing |
| --- | --- | --- |
| Today's dodge | 89.6% +/- 1.0% | 96.2% +/- 0.6% |

**Two failures, traced at a quarter second.**
- **F1, the turnaround** (rolling, seed 2): the point across the line
  was behind a tank rolling the other way. The world routes a Move by
  tiles in eight directions with a waypoint at each turn (`Map::route_by`),
  so the point was two legs off, the first beside the tank; it looped
  round it and was 0.9 tiles from the impact when the shell landed.
- **F2, the release** (standing, seed 5): the tank reached its point at
  1.7 s with 1.65 s of flight left. At `dodge_hold` 0 the hold ends on
  arrival; the Auto ladder's fortify errand was routed back across the
  impact, and the shell landed on it. LR 28's blunt answer, holding for
  the whole flight (`dodge_hold` 1), cost 1.18 a seat.

**The parts.**
- `drive_to` (lib.rs): where a tank will be after n ticks sent to a
  point, by the sim's rules of motion: the route the world will plan
  (`route_by`, without the pillbox and mine prices the brain cannot
  read), then turn rate, easing for a corner and braking before the
  next, the pivot past 2 rad, momentum, the ground's cost, stopping
  where the way is shut. `drive_to_foresees_the_world`: 96 drives (at
  rest and rolling, four headings, twelve points 5 tiles round) end
  exactly where the world puts them, worst 0.000 tiles; broken two
  ways it fails (no corner braking: 1.7 tiles off; a straight line in
  place of the route: 4.7).
- `dodge_plan` (weights.rs, thresholded at 0.5, default 0): the dodge's
  candidates (today's four, then straight on to where the heading
  leaves the 5-tile circle, then twelve round the impact) are each
  driven by `drive_to` for the shell's flight; the first that ends past
  the splash and the margin is taken, so today's point stands wherever
  it works; failing all, the one ending farthest off.
- `dodge_release` (default 0): after the Commander's think, while a
  dodged shell is in flight and this tick was not the dodge's own
  order, a Move that `drive_to` says has the tank inside the splash and
  the margin at the landing is struck, and asked for again next think
  (`Commander::think_released`, `Dodge::release`). An errand that keeps
  clear goes at once, unlike `dodge_hold`.

Two false starts, each a finding. `dodge_plan` driving a straight line
moved nothing (89.6% to 89.7%): the plan kept today's point because it
foresaw a straight drive, and the world drove a route. And the first
"straight on" point, the tank's speed times the flight plus 5 tiles,
ran one tank 14 tiles out into a neutral pillbox's reach (the family's
one failure left with both parts on); straight on now stops at the
5-tile circle.

| 1,000 seeds each | Rolling | Standing |
| --- | --- | --- |
| Today's dodge | 89.6% +/- 1.0% | 96.2% +/- 0.6% |
| `dodge_plan` 1 | 94.5% +/- 0.7% | 97.0% +/- 0.5% |
| `dodge_release` 1 | 95.0% +/- 0.7% | 99.2% +/- 0.3% |
| both | **100.0%** (1,000) | **100.0%** (1,000) |

Each part mends about half the failures, and together they mend all of
them. The traced case of F2 with both on: the tank waits at its point
from 1.2 s, the errand is let go at 2.2 s when `drive_to` finds it
still short of the splash at the landing, and the shell lands 3.4 tiles
off.

**The tests.** `seed = n` in a scenario file pins it to one of
`many`'s seeds, so a family's case is kept as a test.
`a-dodge-does-not-turn-back-under-its-shell.toml` (rolling, seed 2,
`dodge_plan` 1) and `a-dodge-waits-out-its-shell.toml` (standing, seed
5, `dodge_release` 1) each hold, and fail with their weight at 0.

**The census.** The trial team on `dodge_plan` 1 and `dodge_release` 1
against today's defaults, the fair census from a frozen binary,
`CENSUS_ACTS` on. The first run left out `CENSUS_MAP` and
`CENSUS_RESPAWNS` and so played the census's own defaults, a generated
map without respawns; it is kept as a reading on a second map (backlog
25), and the standard run followed. The meter's dodge counts are the
trial team's against the rest of the seats (disjoint groups, `meter.rs`).

| 48 seeds | naval.map, 900 s, 3 respawns (0 to 47) | generated map, no respawns (0 to 47) |
| --- | --- | --- |
| Difference a seat | **+0.92 +/- 0.47** (1.94 errors) | **+1.20 +/- 0.60** (2.00 errors) |
| Ashore, caught by its own shell: trial / rest | 5.7% / 10.1% | 6.2% / 10.6% |
| Ashore, killed: trial / rest | 3.8% / 4.9% | 4.2% / 4.8% |
| Afloat, caught by its own shell: trial / rest | 4.2% / 6.9% | (9 dodges in all) |
| Afloat, killed: trial / rest | 11.6% / 14.7% | |

**Fresh seeds.** The rule, set before the run (LR 47): confirmed only
past two errors, the same way, on 48 seeds not used before (240 to
287; 144 to 239 are kept for `duck`, backlog 42), alone.

| naval.map | Seeds | Difference a seat | Ashore, own shell: trial / rest | Afloat, killed: trial / rest |
| --- | --- | --- | --- | --- |
| First | 0 to 47 | +0.92 +/- 0.47 | 5.7% / 10.1% | 11.6% / 14.7% |
| Fresh | 240 to 287 | +0.70 +/- 0.66 (1.06 errors) | 6.3% / 10.2% | 11.7% / 14.4% |
| Pooled | 96 | +0.81 +/- 0.40 (2.00 errors) | | |

**On the score, not confirmed.** The mechanism repeats to the tenth on
the fresh seeds: own-shell catches ashore cut by 40%, boats killed in
a dodge by a fifth. All three score readings are positive (+0.92,
+0.70, +1.20), and pooled they reach two errors, but alone the fresh
seeds do not. A seed's spread is 3.9 a seat, so an effect of 0.8 needs
about 95 fresh seeds, (2 x 3.9 / 0.8)^2, to pass two errors by itself.
Both weights stay 0, today's. Whether to turn them on, as the
landfall's three were (LR 38), is Paul's choice; what argues for it is
that the dodge is a reflex every team has, measured here by its own
counts as well as the score, and that the scenarios show no case it
makes worse.

What this changes. The dodge's last two failures in the scenarios have
a cause and a mend, each proved in seconds, and the census shows them
acting in matches as the scenarios predicted. `drive_to` gives the
brain a way to ask where a tank will be, exactly, which the duck and
the landfall choose points without (backlog 46). A scenario family, a
situation drawn from the seed, measured a part to a tenth of a percent
in three seconds.

Proof: `make test`: brain 78 passed and 10 ignored; client 31; net 14;
proto 3; sim 50. No warnings. 382 functions, 121 undocumented, 88
tests.

## 54. The planned dodge on by default, and a new rule, 20 October

Paul, on LR 53's question: "yes. keep all the new behavior. keep
adding. we need to finish writing the brain. but ofcourse it still must
work. even if slightly less. We will continue to adjsut scalars after
all features are added. but we need to add them wile finding a sensible
starting numbers."

`dodge_plan` and `dodge_release` default to 1. The old dodge is one
weight away (`dodge_plan = 0`, `dodge_release = 0`). With the defaults,
both scenario families of LR 53 hold on 1,000 of 1,000 seeds.

**The rule changes.** Since LR 11 a part landed off and waited for a
census to promote it; of the eighteen parts built that way, three were
promoted (`castle_cap`, the landfall's three, now the dodge's two) and
the rest wait at the noise floor that 48 seeds cannot see past (LR 24,
47, 53). From here a part lands on, with numbers started where a
scenario family or a short sweep puts them; the census guards that the
brain still works, at a small cost at most; the tuner follows once the
brain is written. The parts already built and off are next (LR 55).

Proof: `make test`: brain 78 passed and 10 ignored; client 31; net 14;
proto 3; sim 50. No warnings.

## 55. Every part on, and the guard, 20 October

By LR 54's rule the eleven parts built and still off go on: `forget`,
`board`, `team_rotate`, `priced_crossing`, `column`, `cycle`,
`island_site`, `escort`, `bridge_jobs`, `duck`, `posture`. Every bit of
`Caps` is now on by default; each old method is one switch away
(`caps = baseline, -name`).

Two tests compared a part on against `Caps::BASELINE` as the part off
(`a_tank_ducks_a_gun_ready_before_its_own`, its duel too, and
`a_teammate_finishes_a_part_laid_bridge`); their off cases now name the
part off. No other test moved.

**The guard.** The trial team on the new defaults against the defaults
of LR 54 (`caps = baseline` minus the eleven), the fair census on
naval.map, 900 s, 3 respawns, seeds 0 to 47, a frozen binary.

| 48 seeds | Every part on, against LR 54's defaults |
| --- | --- |
| Difference a seat | -0.85 +/- 0.59 (1.44 errors, inside the noise) |
| The three jobs' means, trial / baseline | 7.33 / 9.26, 7.30 / 9.11, 9.32 / 8.12 |
| Brain time | 37.8 to 38.9 us a seat-tick (26.5 to 29.6 in LR 47) |
| The trial team's rung seat-seconds | fortify 3,780 / 3,630 / 3,694; expand 4,241 / 3,262 / 2,861; turrets 455 / 820 / 444 |

It still works, a little less on the score, inside the noise; by
history the posture's lean (LR 24 to 26: -0.81, -1.04) and
`team_rotate` (LR 24: -0.83) are the likely costs, and the tuner's
phase will set their numbers (backlog 48). The brain costs about 30%
more time; sixteen seats at 4x speed would take about 5% of a core.

## 56. A dodge against every shell in flight, 20 October

Backlog 45. LR 53's plan drove each candidate against the one shell it
dodged, and the release checked only the last shell dodged. Two guns
on one tank, or one gun's salvo, are common in the quad.

**The families**: `scenarios/families/a-rolling-tank-under-two-shells.toml`
and `a-standing-tank-under-two-shells.toml`, two shells from any sides,
each landing within 3 tiles of a Commander's tank in 1.5 to 3.5 s.

**The part**: `dodge_every` (weights.rs, thresholded at 0.5, on by the
rule of LR 54). `Dodge::threats`: every hostile indirect shell in
flight landing within `dodge_horizon`, beside the one dodged.
`drive_path`: `drive_to`'s positions after every tick (`drive_to` is
now its last; `drive_to_foresees_the_world` still reads 0.000).
`Dodge::clearance`: the least, over the landings, of the tank's
distance from each impact at its own landing tick, less the splash and
the margin. `planned` takes the first candidate clear of all of them;
`release` strikes an errand foreseen inside any of them, dodged or not.

| 1,000 seeds each | Plan and release on the one shell (LR 53) | On every shell |
| --- | --- | --- |
| Rolling, two shells within 3 tiles | 76.5% +/- 1.3% | **100.0%** |
| Standing, two shells within 3 tiles | 79.9% +/- 1.3% | **100.0%** |
| Rolling, three within 2.5 tiles | 67.7% +/- 1.5% | **100.0%** |
| Rolling, four within 2.5 tiles | 56.6% +/- 1.6% | **99.7% +/- 0.2%** |
| One shell, rolling and standing (LR 53) | 100.0% | 100.0% |

(Without LR 53's two parts the two-shell families read 67.0% rolling and
75.9% standing.)

**The test**: `a-dodge-clears-two-shells.toml`, seed 2 of the rolling
family; it holds, and with `dodge_every` 0 the tank is hit.

## 57. The duck foreseen, and a shot that waits, 20 October

Backlog 46. The duck (LR 21, on since LR 55) takes the nearest point to
the errand among sixteen round the tank that are out of the gun's
reach or line, and never asks whether the tank gets there before the
gun is ready.

**The family**: `scenarios/families/a-tank-under-a-ready-gun.toml`. A
Commander on Auto at rest, any heading, its gun reloading for 2.5 s; a
hostile RuleBot 9 tiles off in any direction (`[[tank]] around = true`),
its gun ready in 0.4 to 1.4 s (`reload = [lo, hi]`). Held: never hit in
3 s, by the direct shot or by the shell it lobs once the tank is past
direct range. The trace now shows the ground, speed and heading of each
tank, and every hit with its damage and the drones in the air; the
RuleBot takes only the weights it holds from `SET`.

**What the traces found.**
1. **The duck jitters.** Each think it picks again round the tank's
   new position; the order is re-sent when the point moves a tile, so
   the tank pivots toward each new route and, on seed 9, sat a full
   second at its start.
2. **A tank has no turret.** On seed 4 the duck and then the dodge
   worked until the tank's own gun came ready at 2.5 s; "the guns stay
   free", so it lobbed a shell back. Firing turns the hull to the shot
   (treadcore-sim `FireIndirect`: `t.heading = (at - pos).heading()`):
   89 degrees to 212 in one tick, and at 3.9 tiles a second the tank ran
   on that way, into the splash of the shell it was leaving.
3. Of the first 21 failures in 150 seeds, 13 were a direct shot at 1.25
   to 1.3 s: the gun ready at 0.9 s, the shell 0.3 s in flight. From
   rest, 0.9 s carries a tank about 1.6 tiles: out of reach in time only
   by the right way at once, or out of the gun's line.

**The parts** (both on, LR 54's rule):
- `duck_plan` (weights.rs, thresholded at 0.5): the point is held while
  it stays safe; otherwise each of the sixteen is driven by `drive_to`
  to the tick the gun is ready, and the nearest to the errand of those
  out of reach (10.8 tiles, the range and the hit radius) or out of line
  by then is taken; failing that, the one farthest out of reach then.
- `dodge_fire`: with a hostile shell landing within the horizon, each
  shot is foreseen by `drive_path` twice, the hull as it is and turned
  to the shot, toward where the tank is going; a shot that takes a clear
  drive into a splash is struck (`Dodge::hold_fire`, from
  `Commander::think_released`, dodging or not).

| Ready-gun family, 1,000 seeds | Held |
| --- | --- |
| `w_duck` 0 (no duck) | 63.7% +/- 1.5% |
| the duck of LR 21 (`duck_plan` 0, `dodge_fire` 0) | 76.6% +/- 1.3% |
| with `dodge_fire` | 76.7% +/- 1.3% |
| with `duck_plan` too (the defaults) | **83.4% +/- 1.2%** |

`duck_plan` takes 29% of the duck's failures; `dodge_fire` saves the
tank alone on 2 seeds of 400 here (a tank in this family dodges with its
gun ready only late), and seed 4 of the fixed-reload form. Every other
family holds as before (100%).

**The tests**: `a-tank-ducks-out-of-reach-in-time.toml` (seed 19,
`duck_plan`) and `a-dodging-tank-holds-its-fire.toml` (seed 261,
`dodge_fire`); each holds, and fails with its weight at 0. The first
form of the second, seed 4 at a fixed reload, held with `dodge_fire` 0
once `duck_plan` had changed the duck: a test kept only after it fails
broken.

**The guard** (LR 54's rule): the trial team on the new defaults against
the same with `dodge_every`, `dodge_fire` and `duck_plan` at 0 (LR 56
and 57 together), the fair census, naval.map, 900 s, 3 respawns, seeds
0 to 47, a frozen binary.

| 48 seeds | Trial (LR 56, 57 on) | The rest (off) |
| --- | --- | --- |
| Difference a seat | +0.21 +/- 0.68 (inside the noise) | |
| Ashore dodges: hit / by their own shell / killed | 16.0% / **1.6%** / 4.0% | 20.7% / 6.1% / 4.3% |
| Afloat dodges: hit / by their own shell / killed | 8.9% / **0.5%** / 8.0% | 12.4% / 4.0% / 11.4% |
| Brain time | 40.2 to 42.2 us a seat-tick (both sides) | |

The score does not move past the noise; the reflexes do what the
families say: a dodge caught by its own shell is now rare ashore and
afloat, and boats in a dodge die 30% less. Since LR 52 the own-shell
share ashore has gone 15.7%, 10.4%, 5.7%, 1.6%.

Proof: `make test`: brain 78 passed and 10 ignored; client 31; net 14;
proto 3; sim 50. No warnings.

## 58. The front read, and a front remembered, 20 October

Backlog 49. Since LR 24 the board has built its influence, front and
territory every second and nothing has read them. The Expand rung took
the depot not of the side nearest the base, wherever the two sides
met.

**The part** (on, LR 54's rule): `w_front` (weights.rs, 0.5, needs
`board`). With the board's front known, `Commander::expand` costs each
open depot `(1 - w)` its distance to the base plus `w` its distance to
the nearest front sector's centre (`TeamBoard::front`, our sectors
beside theirs), and takes the cheapest. A depot so chosen is kept until
it is taken or given up (`depot_target`): the front changes each
second, and a tank whose own arrival tipped the balance found the front
gone and turned back for the nearest (the scenario's first trace, 4% of
the way into the capture). At 0, or with the board off, the nearest
depot to the base as before.

**The scenario**: `an-expanding-tank-takes-the-depot-at-the-front.toml`.
A Commander with only the Expand and Guard rungs on and a team board
(`board = true`, `rungs`, new); a teammate 30 tiles east sees a still
hostile tank 8 tiles past it (a team shares its sight); two neutral
depots placed by `[[depot]] at`, one 12 tiles south (the nearest), one
18 tiles east toward the front. Expected `depot_first` (new, with
`depot_owned_by`): the eastern one. It holds; at `w_front` 0 it fails
("took depot 1 first, at 13.1 s").

**First reading: the front is seldom known.** The meter (new: Expand's
fresh picks with `w_front` on, by group, with and without a front, and
those moved off the nearest depot) over the first 60 matches of the
guard census: 151 fresh picks with a front known, 2.5 a trial match, 69
of them moved (1.2 a match). The board reads one member's observation once a second,
and a hostile tank counts only while in sight (LR 35's finding for the
landfall, again). The census was stopped there: a part that acts once
a match cannot be read by it.

**The memory**: `front_memory` (30 s, as `landfall_memory`). Each
hostile tank in sight at a rebuild is kept where it is
(`TeamBoard::seen`); one seen dead, or out of sight for `front_memory`,
is dropped; the rest still count where they were, in a second
influence (`TeamBoard::known`) that the front is read from. The
influence itself, the threat, the defence points, the posture and the
rotation read only what is in sight, as before. Proof:
`a_front_seen_is_remembered` (the front unchanged a rebuild after the
hostiles leave sight, gone after 31 s, gone at once at 0).

Expand picks few depots, about four a match, since each is kept until
taken.

**The guard**: the trial team on the defaults against the same with
`w_front` and `front_memory` 0; the fair census, naval.map, 900 s, 3
respawns, seeds 0 to 47, a frozen binary (`build/lr58`).

| 48 seeds | Trial (`w_front` 0.5, `front_memory` 30) | The rest (both 0) |
| --- | --- | --- |
| Difference a seat | -0.14 +/- 0.42 (inside the noise) | |
| Mean score a seat, by job of 16 seeds | 8.27, 8.57, 8.13 | 8.30, 8.47, 8.60 |
| Expand's fresh picks with a front known, a trial match | 3.2 (459 of 536; 14% with none) | |
| of those, off the nearest depot | 176, 1.2 a match | |
| Ashore dodges: hit / by their own shell / killed | 16.5% / 1.7% / 4.3% | 15.9% / 1.6% / 4.0% |
| Brain time | 67.7 to 69.3 us a seat-tick (both sides; the machine was building and testing beside it, LR 57 read 40 to 42 unloaded) | |

The brain still works with the part: the score does not move past the
noise, as a part that changes about one decision a match should not.
The memory raised the picks with a front known from 2.5 to 3.2 a match
and left the ones moved where they were (1.2 and 1.2): when the front
is known, it is mostly known from what is in sight. The front read is
in; whether it pays waits for the tuning phase (backlog 48), and its
larger use, the guard posts (backlog 51).

Proof: `make test`: brain 81 passed and 11 ignored; client 32; net 14;
proto 3; sim 50.

## 59. The families held to floors, and allies' goals on the screen, 20 October

**Backlog 47, the floors.** A family (`scenarios/families/`) now says
how many seeds it runs (`runs`, 200 if unset) and the share it must
hold (`floor`); `make families` (scenario.rs
`every_family_holds_its_floor`, ignored) runs every family from seed 1
and fails if one is under its floor; `SET="name=value"` breaks a part
on every family at once. A seed takes about 6 ms, so all five families
at 1,000 seeds take 31 s on one core.

The floors are set under today's rates, more than two errors below
the ready gun's and at 99% for the four dodge families, which hold
every seed:

| Family, 1,000 seeds | Today | Floor | `dodge_plan` and `duck_plan` 0 | `dodge_release` 0 |
| --- | --- | --- | --- | --- |
| a rolling tank under a shell | 100.0% | 99% | **95.0%** | |
| a rolling tank under two shells | 100.0% | 99% | **76.4%** | |
| a standing tank under a shell | 100.0% | 99% | 99.2% | **97.0%** |
| a standing tank under two shells | 100.0% | 99% | **84.2%** | |
| a tank under a ready gun | 83.4% +/- 1.2% | 80% | **74.9%** | |

Each family falls under its floor with the part it proves broken (in
bold); the standing tank under one shell is the release's family (LR
53), not the plan's, and falls with `dodge_release` 0.

**Backlog 50, allies' goals (11c).** Read first: a window that hosts
already lists every allied bot under FLEET on the coordination screen
with the bot's own status, its rung and what it is going for ("auto >
expand: to the depot at (82,10), 20 tiles"), since 15 October
(session.rs `step_host`, b257eec). What is missing is the same for a
window that joins over the network: the bots think on the host, and
their goals are not in the Observation, so that half needs the wire
(version 16, which goal 4 must not change) and waits on goal 6. The
hosting half is now proven: client `the_screen_shows_an_allys_rung_and_target`
runs an allied Commander on Auto for 3 s and finds its line, rung
named, on the screen.

## 60. The crews and the hop tested, and the map documented, 20 October

Backlog 26 (CI-001 A9, A10). `crew.rs` and `hop.rs` had no tests of
their own; 121 of the brain's functions had no doc comment.

**`each_crew_gets_its_own_item_and_a_late_one_is_given_up`** (crew.rs).
`FleetCommand::think` on a castle plan, with two crews answering Ready
on the commander's observation: a muster and one Build each, never the
same tile, both kept off the commander's own squad (`plan.skip`); the
next tick, both busy, nothing. Late by `JOB_FOR`, each crew is dropped
and its item counted failed; Ready again, the same two items go out;
late a second time (`FAILS`), the plan skips them and the next two
items go out instead. Broken (`n > FAILS` for `n >= FAILS`) it fails.

**`a_hop_bridges_a_narrow_gap_and_sails_a_wide_one`** (hop.rs). A tank
sent (`Hop::send`) across a channel on an all-grass map, 200 tiles of
forest behind it: 4 tiles of water are bridged from the near bank, the
whole crossing a job of 4 tiles; `BRIDGE_MAX` + 5 is not bridged.
Priced (`priced_crossing`, LR 6.2), 7 tiles bridge (7.9 against a
boat's 8.3) and 8 sail (10.5). The first form, with no forest, sailed
at 7: scarcity is the trees of the tank's own island, 12 / (forest +
1), and a bare island makes every bridge dear, which the test now says.
Broken (the price's verdict inverted) it fails.

**Doc comments** for the 21 undocumented functions of `crew.rs`,
`hop.rs` and `board.rs`, and the new tests' helpers; `take_jobs`'s own comment had slipped above
`adopt` and is back on it. `make brainmap`: 392 functions, 99 with no
doc comment (121 before), 92 tests.

## 61. The guard posts at the front, 21 October

Backlog 51, the rest of 49's hypothesis: the Guard rung's post and the
rotation's push point, leaned toward the board's front, meet attacks
before they reach the base. Before, the post was the base's (behind
the castle's gate, else beside the depot) and the push point halfway
from the base to the side's objective (the nearest depot not of the
side, rotation.rs), wherever the two sides met.

**The parts** (on, LR 54's rule; each needs `board`, and is off at 0):

- `w_front_push` (0.75): `Rotation::fleet_push` moves the push point
  that share of the way toward the front sector nearest it. The
  Commander hands the rotation the front's sector centres each think
  (`Rotation::front`, `front_w`), before the push point is fitted to
  ground the tank can reach. Test: `the_push_point_leans_to_the_front`.
- `w_front_post` (0.5): `Commander::guard_post` moves the base's post
  that share of the way toward the nearest front sector, at most
  `FRONT_POST_REACH` (`BASE_ALARM - BASE_R`, 6 tiles) so the base's
  ring stays inside the guard's leash; back to the post if the world
  has refused a drive there. Used by `defend_base` when the guard is
  not on rotation.
- `Commander::front_points`, the front's sector centres with the board
  on, now shared with Expand's front read (LR 58).

**The family**: `scenarios/families/a-raid-on-a-guarded-base.toml`. A
Commander with only the Guard rung and a team board, its base in the
map's centre (`"centre"`, an anchor that now takes no number). A
brainless hostile tank 40 tiles off, any side, drives at the base
(`toward`, new: a tank headed at a place). A teammate 8 tiles from the
raider sees it for half a second and comes home, and the guard's
drones are recalled then, so the front on the raid's side is only
remembered (`front_memory`). Held: the guard's first hit on the raider
falls more than 10 tiles from the base, the base's ring (`met_beyond`,
new; the outcome now keeps where each hit tank was).

The first form of the family, 30 tiles, the teammate staying and the
drones flying, held at every bar with the parts on and at 0 alike: the
guard's shells, spotted by the teammate and its recon drone, and a
strike drone met the raider 20 to 23 tiles out before the guard had
moved. A raid seen far out is met far out whatever the post. With the
sight taken away:

| 400 seeds, met beyond 10 tiles | held |
| --- | --- |
| both 0 (before) | 50.5% +/- 2.5% |
| `w_front_push` 0.25 | 50.5% +/- 2.5% |
| `w_front_push` 0.5, the post at 0 | 69.8% +/- 2.3% |
| `w_front_push` 0.75 (the default) | 85.5% +/- 1.8% |
| `w_front_push` 1 | 89.5% +/- 1.5% |
| `w_front_post` 0.25, 0.75 or 1, the push at 0.5 | 68.2% +/- 2.3% |
| `w_front_post` 1, the push at 0.25 | 50.5% +/- 2.5% |

The push point does it all; the post nothing here. A guard with no
raider in its leash is on rotation and goes where the rotation sends
it, and once the raider is within 16 tiles it is met wherever the
leash is centred. 0.75 was taken over 1: within two errors of it, and
a push point on the front itself takes the tank farthest from its
base. Floor 0.75; at `w_front_push` 0 the family reads 50.5%.

**The meter** (new: a guard's seconds with the push point or the post
worked out, and of those, moved toward the front), read before the
census was let run (LR 58's lesson): in the first matches the push
point leaned in 404 to 537 seconds a match.

**The guard**: the trial team on the defaults against the same with
both weights 0; the fair census, naval.map, 900 s, 3 respawns, seeds 0
to 47, a frozen binary (`build/lr61`).

| 48 seeds | Trial (`w_front_post` 0.5, `w_front_push` 0.75) | The rest (both 0) |
| --- | --- | --- |
| Difference a seat | -0.27 +/- 0.64 (inside the noise) | |
| By job of 16 seeds | +1.79 +/- 1.26, -1.70 +/- 1.05, -0.89 +/- 0.88 | |
| Mean score a seat, by job | 9.31, 7.34, 7.53 | 7.51, 9.05, 8.42 |
| Push point worked out, moved toward the front | 23,950 of 30,216 s (79%) | |
| Guard's post worked out (a raider at the base), moved | 1,039 of 1,130 s (92%) | |
| Ashore dodges: hit | 15.7% (2,086 of 13,316) | 15.8% (3,931 of 24,846) |
| Brain time | 58.7 to 60.7 us a seat-tick, both sides | |

The brain still works with the parts: the score does not move past the
noise. Unlike the front read, the push point acts most of the time a
guard rotates, so the census reads a part that acts and finds no gain
or loss a point wide. The push point is worked out about 210 s a trial
match (144 of them), the post about 8: a raid on a guarded base is
rare on naval.map. Both stay on with their
starting numbers for the tuning phase (backlog 48).

Proof: `make test`: brain 82 passed and 11 ignored; client 32; net 14;
proto 3; sim 50. `make families`: six families, each above its floor.

## 62. Island sites where a naval start chooses, 21 October

Backlog 20. `island_site` (12f2) scores a base site: trees enough
(`w_site_forest`, against `forest_need`) less a small island
(`w_site_small`, under `site_area`). It was read only by
`Commander::base_of` among the side's depots when a tank had no base,
and in a fleet every tank's base is its leader's depot, found first:
LR 24 and LR 44 found it never acted. In a naval start, with every
tank afloat and every depot neutral, the first island is chosen by the
beach: `RuleBot::landfall_order` priced each beach in sight by its
distance, its danger, the heading toward a gun and the run in a gun's
reach, and landed on the cheapest. Ashore, Expand takes the depot
nearest. The beach is where the site has to be read.

**The part** (on, LR 54's rule; needs `island_site`):
`w_site_beach` (20). While the tank is afloat and its side holds no
depot, `Commander::beach_worth` gives the doctrine each beach in sight
(`obs.shore`) with `w_site_beach` times its island's `site_score` (-1
for a small bare islet to 1 for a large forested island), and
`landfall_order` takes that worth off the beach's cost
(`RuleBot::beach_worth`). With the side holding a depot, or at 0, the
beach as before. Test: `a_boat_lands_where_the_island_is_worth_the_run`
(the nearer beach with no worth; the farther worth more than the
difference in the run; the nearer again for a small worth).

**The scenario**: `a-naval-start-lands-on-the-forested-island.toml`.
Open water in the map's middle; a 4 by 4 islet of grass 10 tiles east,
a 31 by 31 island with a forest of 10 by 10 14 tiles west; a neutral
depot on each and every other depot gone (`depots = false`, new;
`forest` terrain, new). Held: the boat comes ashore within 8 tiles of
the island's near shore. With the part it lands there, takes the depot
and the squad begins a castle by 20 s. At 0 it lands on the islet,
takes its depot, is refused building there ("cannot build there", 12.6
s on) and sails on for the island.

**The family**: `scenarios/families/a-naval-start-by-a-bare-islet.toml`:
the boat 9 tiles from the islet's centre, any side, so the islet is
always the nearest beach; the island 18 tiles west of the islet, its
beach 15 to 33 tiles from the boat. Held: ashore on the island.

| 400 seeds | landed on the island |
| --- | --- |
| `w_site_beach` 0 (before) | 0.0% |
| 5 | 40.2% +/- 2.5% |
| 10 | 75.5% +/- 2.2% |
| 12.5 | 100.0% |
| 15, 20 (the default), 30 | 100.0% |

The island is worth 1.96 points more than the islet, so at 20 a run up
to 39 tiles longer, 7 s for a boat at 5.7 tiles a second. 20 was kept
over 12.5 for the margin: the family's longest extra run is 26 tiles
less the danger terms. Floor 0.95.

**The meter** (new: seconds afloat with beaches priced, by group), read
in the census's first matches: 54 to 169 trial seat-seconds a match,
in every match.

**The guard**: the trial team on the defaults against the same with
`w_site_beach` 0; the fair census, naval.map, 900 s, 3 respawns, seeds
0 to 47, a frozen binary (`build/lr62`).

| 48 seeds | Trial (`w_site_beach` 20) | The rest (0) |
| --- | --- | --- |
| Difference a seat | +0.82 +/- 0.69 (inside the noise, 1.2 errors) | |
| By job of 16 seeds | +0.72 +/- 1.37, +1.41 +/- 1.10, +0.34 +/- 1.15 | |
| Mean score a seat, by job | 8.54, 9.01, 8.98 | 8.20, 7.60, 8.26 |
| Seconds afloat with beaches priced | 12,779, about 89 a trial match | |
| Built a team-match | 16.72 +/- 0.84 (144) | 16.17 +/- 0.57 (288) |
| Afloat dodges: hit | 8.3% (374 of 4,524) | 8.4% (846 of 10,105) |
| Brain time | 60.8 to 62.7 us a seat-tick, both sides | |

The brain still works with the part, and the score leans its way
without leaving the noise. The hypothesis's "builds more" is not shown:
LR 61's census, with nothing that builds under test, read the trial
seat 0.4 higher on the same count, as this one does by 0.55. naval.map
starts every side near islands fit for a base, so the choice the part
makes is seldom a bad one avoided.

Proof: `make test`: brain 83 passed and 11 ignored; client 32; net 14;
proto 3; sim 50. `make families`: seven families, each above its
floor.

## 63. Every part off, one at a time, 21 October

Backlog 48, the starting numbers for the brain with every part on.
LR 55 read every part on at -0.85 +/- 0.59 against LR 54's defaults.
The hypothesis: one or two parts carry that cost (by history the
posture's lean and `team_rotate`), and their weight halved
(`w_posture` 0.25, `swap_gap` 0.6) gives it back.

**The tool**: `make parts` (`scripts/parts.sh`), the reverse of `make
switches`. The fair census once for each part, the trial seat on the
defaults (every part on), the baseline with that part off: a
capability as `-name`, a weight at the value its doc line names as
before it. The difference is what the part is worth; above zero it
pays. 31 parts: 17 capabilities and 14 weights. Logs, weights files
and the table under `build/parts/STAMP`.

**Each part off**: naval.map, 900 s, 3 respawns, 12 seeds, a frozen
binary, four at once (`JOBS=4 make parts`, about 1.5 h).

| Part off | On | Off | Difference | Error |
| --- | --- | --- | --- | --- |
| `-dodge` | 12.04 | 6.84 | **+5.20** | 1.91 (outside) |
| `landfall_memory=0` | 10.32 | 7.56 | +2.76 | 1.76 |
| `-posture` | 9.77 | 7.96 | +1.82 | 0.94 |
| `-rotate` | 9.71 | 8.11 | +1.60 | 1.27 |
| `landfall_shy=0` | 9.60 | 8.15 | +1.45 | 1.27 |
| `-cycle` | 9.49 | 8.05 | +1.44 | 0.90 |
| `dodge_plan=0` | 9.33 | 7.93 | +1.40 | 1.54 |
| `w_site_beach=0` | 8.92 | 7.54 | +1.38 | 1.45 |
| `w_front_post=0` | 9.03 | 7.94 | +1.09 | 1.01 |
| `-island_site` | 8.57 | 7.55 | +1.02 | 1.28 |
| `front_memory=0` | 9.39 | 8.43 | +0.96 | 1.32 |
| `-team_rotate` | 8.94 | 8.12 | +0.83 | 0.88 |
| `-priced_crossing` | 8.60 | 7.76 | +0.83 | 1.09 |
| `-board` | 9.57 | 9.01 | +0.56 | 1.20 |
| `-bridges` | 8.90 | 8.40 | +0.50 | 0.91 |
| `w_smooth=0` | 8.40 | 8.04 | +0.36 | 0.88 |
| `duck_plan=0` | 8.67 | 8.49 | +0.18 | 1.26 |
| `-bridge_jobs` | 8.66 | 8.50 | +0.16 | 0.52 |
| `-escort` | 8.21 | 8.21 | 0.00 | 0.00 |
| `-column` | 8.41 | 8.52 | -0.11 | 0.36 |
| `-forget` | 8.11 | 8.31 | -0.19 | 0.49 |
| `dodge_release=0` | 8.18 | 8.65 | -0.46 | 0.80 |
| `w_front=0` | 7.80 | 8.56 | -0.76 | 0.82 |
| `-castle_cap` | 8.28 | 9.09 | -0.81 | 0.62 |
| `-pings` | 8.57 | 9.61 | -1.05 | 1.13 |
| `dodge_every=0` | 7.88 | 9.06 | -1.18 | 1.36 |
| `w_front_push=0` | 7.78 | 9.09 | -1.30 | 1.23 |
| `dodge_fire=0` | 7.63 | 9.04 | -1.42 | 1.22 |
| `-duck` | 7.33 | 8.89 | -1.57 | 0.96 |
| `-kite` | 7.58 | 9.34 | -1.76 | 1.04 |
| `landfall_wait=0` | 6.79 | 8.82 | -2.03 | 1.29 |

One part is outside two errors, and it pays: the dodge. None costs
past two errors. The two the hypothesis named read on the paying side,
the posture +1.82 and `team_rotate` +0.83. Of 31 tests at two errors
about 1.5 would cross by chance; 21 of 31 read above zero.
`-escort` reads exactly 0: no escort job arose on naval.map in 12
seeds.

**The halved weights**, the plan's proof, and the worst row halved:
the trial seat on each against the defaults; the fair census at 48
seeds (0 to 47), three jobs of 16, frozen binaries (`build/lr63a`,
`build/lr63b`).

| 48 seeds | Difference a seat | By job of 16 |
| --- | --- | --- |
| a: `w_posture` 0.25, `swap_gap` 0.6 | +0.61 +/- 0.60 (1.0 errors) | -0.75 +/- 1.00, +1.64 +/- 1.24, +0.92 +/- 0.83 |
| b: `landfall_wait` 5 | -0.09 +/- 0.46 (0.2 errors) | +0.25 +/- 0.93, -0.75 +/- 0.65, +0.22 +/- 0.83 |

Neither is a gain. **The defaults stand.** The hypothesis is not
supported: LR 55's cost is not one or two parts. Read with this table
it is most likely the sum of many small effects of both signs, or no
cost at all (LR 55 was 1.4 errors). A half point a seat, the size
asked of it, takes the 48 seeds per part that one night of `make
parts` cannot give; the table ranks where to look first (`kite`,
`landfall_wait`, `duck` below; the dodge's three numbers among them).

Proof: `make test`: brain 83 passed and 11 ignored; client 32; net 14;
proto 3; sim 50. `make families`: seven families, each above its
floor. No code in the brain changed.

## 64. The bottom of the parts table at 48 seeds, 21 October

Backlog 53. LR 63's table read three parts below zero at 12 seeds,
none past two errors: `kite` -1.76 +/- 1.04, `duck` -1.57 +/- 0.96 and
the dodge's `dodge_fire` -1.42 +/- 1.22. The hypothesis: one of them
costs a half point a seat on naval.map.

**The run**: `PARTS="-kite -duck dodge_fire=0" SEEDS=48 JOBS=3 make
parts`; the fair census, naval.map, 900 s, 3 respawns, seeds 0 to 47
in one job a part, three at once, a frozen binary, about 50 min
(`build/parts/20261021-123214`).

| Part off | 48 seeds | Seeds above zero | 12 seeds (LR 63) |
| --- | --- | --- | --- |
| `-kite` | -0.94 +/- 0.77 (1.2 errors) | 22 of 48 | -1.76 +/- 1.04 |
| `-duck` | +0.12 +/- 0.60 (0.2 errors) | 22 of 48 | -1.57 +/- 0.96 |
| `dodge_fire=0` | -0.19 +/- 0.58 (0.3 errors) | 26 of 48 | -1.42 +/- 1.22 |

None is past two errors. The duck and the dodge's held shot fall to
nothing; the 12-seed reads were the tail of 31 tests, as LR 63 warned.
The kite keeps a lean below zero at 1.2 errors. **The defaults stand**;
no part goes to a family.

The kite is the one left to look at, and it cannot be read yet: the
meter counts no kiting, so how often it acts, and whether it costs
when it acts or only shifts the play, are unknown. LR 58's rule
(count before the census) puts the count first: backlog 54.

Proof: no code changed; `make test` and `make families` as LR 63.

## 65. The kite counted, 21 October

Backlog 54. LR 64 read the kite off at -0.94 +/- 0.77 at 48 seeds, the
one lean left in the parts table, and the meter counted no kiting. LR
58's rule: count before the census. The hypothesis: the kite acts
seldom on naval.map and its lean is noise; if it acts often, the
seconds it holds a tank out of the fight are what it costs.

**The meter** (new): `meter::kite(step)`, once a second for each live
Commander seat after its think (`think_released`), with the kite step
of its act (`Act::kite()`: `Kite`, `WingKite`, `RallyKite`) or none,
by the seat's group. The census line: "seconds kiting: N of M alive
(massing, closing, stepping in, stepping out, holding)". Test:
`a_second_alive_counts_its_kite_step`. No decision reads it: the 12
per-seed differences below are LR 63's to the hundredth.

**The read**: the defaults (trial) against the kite off; the fair
census, naval.map, 900 s, 3 respawns, seeds 0 to 11 in three jobs of
4, a frozen binary (`build/lr65`).

| 12 seeds, the trial seat | Seconds | Share |
| --- | --- | --- |
| Alive | 88,122 | |
| Kiting | 4,111 | 4.7% of alive; about 114 a trial match |
| Holding past its reach (reloading, covered, loaded) | 2,549 | 62% of kiting |
| Massing out of its reach | 746 | 18% |
| Stepping out to reload | 433 | 11% |
| Stepping in | 356 | 9% |
| Closing on a tank with no shells | 27 | 1% |
| The rest (kite off) | 0 of 199,437 | |

Difference a seat: -1.76 +/- 1.04 (the pooled 12; LR 63's row).

The kite acts, about two minutes of a trial match, and is not the
seldom part the hypothesis's first branch guessed. What it does is
mostly wait: of every 10 s kiting, 6 are spent holding past the
target's reach and 2 massing, and under 1 stepping in to fire. That
fits the second branch, a tank held out of the fight, without proving
it: the score's lean is still 1.2 errors at 48 seeds, and the meter
does not say what the held tank would have done instead.

The kite stays on (the defaults stand). The next question is the
hold's numbers in a duel, where a tank's minute held can be scored:
backlog 55.

Proof: `make test`: brain 84 passed and 11 ignored; client 32; net 14;
proto 3; sim 50.

## 66. The kite's hold in a duel, 21 October

Backlog 55. LR 65: the kite acts about 114 s a trial match, 62% of it
holding past the target's reach. The hypothesis: the hold waits for a
window that seldom opens, and a shorter wait (a smaller `kite_spare`,
a nearer `kite_in`) scores more in a duel.

**By whom** (the meter split, new: seconds kiting by an intruder's
guard, a wingman, a rally), the trial seat over three seeds of the
census (`build/lr66`): a rally 1,092, a wingman 68, an intruder 27.
The kite is the rally's: a tank that called or answered a ping and
fights what is there. A duel has to be a rally.

**The scenario keys** (new): a Commander's `pings = true` calls what
it sees and answers calls, as `fleet.rs` deals a leader (a scenario's
Commander had neither, so it never rallied: in a first duel it went to
Fortify and was shelled from out of sight). `outscores = { by, on }`
holds when `by` hit `on` more often than it was hit back.

**The family**: `scenarios/families/a-duel-in-the-open.toml`. Open
grass, no pillboxes, no depots, drones recalled at 0.5 s; a Commander
on Auto with pings against a RuleBot (which kites too, as a plain
RuleBot does) 16 tiles off in any direction; 60 s. Held: the Commander
outscores it. In seed 1 it rallies, steps in while the RuleBot's gun
reloads, holds past its reach, and kills it at 28.6 s, 4 hits to 0.

| 400 seeds | Held (outscored) |
| --- | --- |
| The defaults (`kite_spare` 0.4, `kite_in` 9, `kite_margin` 1.5) | 63.0% +/- 2.4% |
| `kite` 0 | 51.5% +/- 2.5% |
| `kite_spare` 0, 0.2, 0.8, 1.2 | 64.0, 63.2, 64.0, 61.0% |
| `kite_in` 7, 8, 10, 11 | 63.0, 64.0, 57.2, 60.5% |
| `kite_margin` 0.5, 1, 2.5, 3.5 | 64.8, 65.0, 63.0, 64.5% |

The kite wins a duel 11.5 points more often than no kite (3.3 errors).
Its numbers sit on a flat top: every one within an error or so of the
default but `kite_in` 10, which is worse. The hypothesis is not
supported: a shorter wait (`kite_spare` 0) scores the same. The hold
is not waste in a duel; it is how the duel is won. **The defaults
stand**; floor 0.55, above the kite off.

The census's lean (-0.94 +/- 0.77 at 48 seeds, the kite off) is then
not the duel's. If it is real it is in the team's fight, the 18% spent
massing and the pressing that the fleet's order forces; nothing here
says it is real.

Proof: `make test`: brain 84 passed and 11 ignored; client 32; net 14;
proto 3; sim 50. `make families`: eight families, each above its
floor.

## 67. The bot skill dial, 21 October

Backlog 56. Paul, after watching the autopilot: "the game is hard".
The only dial was `--rookies N`, a yes or no a bot, and it was read
only where plain RuleBots are dealt: in a teamed match, which `make
run` is (`--teams 3`), every bot is a team's Commander and `--rookies`
reached none. The player guide's "start here" did nothing.

**The weight** (goal 4's rule, every either/or a scalar): `skill` (1,
today's bots). `RuleBot::rookie: bool` became `RuleBot::skill: f32`
(`RuleBot::rookie` is skill 0), each of a rookie's traits by it:

| Trait | At skill s | At 1 | At 0 (a rookie, as before) |
| --- | --- | --- | --- |
| A plain RuleBot's think | every quarter second plus (1 - s) of a quarter | 4 Hz | 2 Hz |
| Drones | none before (1 - s) of `ROOKIE_LATE` (90 s) | at once | 90 s |
| An indirect shell's aim | lead times s; off by up to (1 - s) of `ROOKIE_SCATTER` (4 tiles) | led | at the tank, 4 tiles |
| A direct shot's aim | lead times s | led | at the tank |
| Dodge, kite, storm; interceptors on station | off under 0.5; one under 0.5 | on; 2 to 4 | off; one |

A Commander takes `skill` in `set_settings` (its doctrine's, its
tactician's kite under 0.5, its dodge under 0.5) and spoils what it
says at one place, after its think (`Commander::blunt`): every
`FireIndirect` off by up to (1 - s) of 4 tiles, every `FireDirect` by
a tenth of that, and every `Launch` held before (1 - s) of 90 s. Its
doctrine aims in full (`RuleBot::aim_skill`), so a shot is spoiled
once. At 1 nothing is touched: no random draw, no blend computed.
Test: `skill_spoils_the_aim_and_holds_the_drones`.

**The dial for a person**: `--skill` on the window, `SKILL=` on make,
dealt to every bot not on the person's team (a teamed match), not of
the person's fleet (`--fleets`), not a wingman; allies play in full.

**The proof**: the duel family (LR 66) with the Commander's `skill`
swept, 400 seeds each.

| skill | 1 | 0.95 | 0.9 | 0.85 | 0.75 | 0.7 | 0.65 | 0.6 | 0.55 | 0.5 | 0.4, 0.25, 0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Duels outscored | 63.0% | 70.8 | 62.0 | 67.8 | 52.0 | 43.2 | 38.0 | 24.5 | 14.2 | 7.8 | 0 |

Flat within the noise from 1 to 0.85, then falling steadily to 0.5;
under 0.5, with no dodge and no kite, a Commander never outscores a
full RuleBot one on one, as `rookies_are_beatable` found of rookies
against RuleBots. The useful range is 0.5 to 1. Two shapes tried on
the way: a direct shot off by a quarter of the indirect scatter read
the same at 0.75 (54.5%) as a tenth, so the tenth was kept (a direct
shot at short range is the precise one); the doctrine's shot spoiled
twice (its own scatter, then the Commander's) read 7.2% at 0.5, once
7.8%, within the noise; once was kept, as the plainer rule.

**Nothing changed at 1**: every family's count is the same to the seed
(duel 252, islet 400, raid 342, the four dodges 1,000 each, the ready
gun 834 of 1,000). Whether a person finds 0.6 right is a playtest's:
a match at `--skill 0.6` was left open for Paul.

Proof: `make test`: brain 85 passed and 12 ignored; client 32; net 14;
proto 3; sim 50. `make families`: eight, each above its floor.
