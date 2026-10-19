# Roadmap

Where treadcore is going, set out as goals. Each goal says what the
game should be when it is met, what already meets it, and the steps
still between here and there; each step says what it changes, what it
must not change, and how it is proven. The constraint under all of it
is unchanged: the simulation stays headless and deterministic, and
every layer above it drives it through `step` and `observe`.

Rewritten 18 October 2026. Steps 0 to 11 as they ran, with every proof
and finding, are kept word for word in `archive/ROADMAP-STEPS-0-11.md`; the
journal and the changelog cite those step numbers, and they keep their
meaning. The brain work of goal 4 is designed in `LAB-REPORT-BRAIN.md`,
and in the shared document it was drafted in,
[Treadcore: Status Update & Brain AI Lab Report](https://claude.ai/code/artifact/97f399bf-9abe-4094-a060-6a9666f273bd);
section numbers below (LR 5, LR 6.2) are that report's.

Each check-in with Paul has an IEEE-form report in `checkins/`: the
prompts word for word, the decisions they caused, the methods and the
results (CI-001: each switch alone, the posture, a review of the
brain). `BRAIN-MAP.md` indexes every function of the brain
(`make brainmap`).

What is archived and what is current: `archive/README.md`. The brain's
methods, old and new, are registered in `archive/BRAIN-METHODS.md`:
archived (replaced as the default, most still one switch away),
current with an improved challenger still off, and shown impossible.

| Goal | What it means | State |
| --- | --- | --- |
| 1. One world, many machines | playable, editable, networked, programmable: the prior art's idea on today's network | met (steps 0 to 9) |
| 2. The battlefield the war made | indirect fire, drones over every tank, industry under drones | met (step 10) |
| 3. Command, not only driving | goal modes, fleets, castles, pillboxes that move, a coordination screen | half met (step 11) |
| 4. Bots that think as a team | one tunable decision pattern on shared memory, cheap enough to run sixteen | the prototype built (12a to 12h, LR 11 to 20); every switch measured alone at 12 seeds, none yet outside the noise (LR 24) |
| 5. Longer, truer matches | real numbers, upgrades, towns that see, matches past sixteen minutes | open (11f, 11g, 11j) |
| 6. In players' hands | packages, a release, two real routers | waiting (Actions paused since 13 October) |


## Goal 1. One world, many machines (met)

A world that is at once playable, editable, networked and
programmable, as the prior art's was, without its ring: a
deterministic simulation (step 1, the replay; step 2, the world hash),
sixteen seats (0b), the Pillboys (3) and what they build (6c), maps
and their editor (5), teams, alliances and proposals (6, 6b), the
defender brain (6d), prediction (4), play across the Internet (7), the
headless server (8), sound, art and packages (9).

Still open from it: 7's and 8's proof across two real routers and two
machines, and a listening session for the sound.


## Goal 2. The battlefield the war made (met)

The tank as artillery with a drone crew: indirect fire, the bubble,
recon, strike and longshot drones, jammers, the works and industry
under drones (step 10, 10a to 10k). Met on 14 October; balance
continues through goal 5.


## Goal 3. Command, not only driving (half met)

A person commands a tank as well as drives it, allies are a fleet, and
the map grows walled bases with gates under guard towers.

Met: 11a goal modes (the Commander), 11b castles, 11h pillboxes that
move, turrets, defend base and the Auto ladder; and on 17 October,
rotation, the yard, boats, bridges and island hops, fleet commanders
with Lanchester odds, down and revived (wire version 16).

Steps left:

- [~] 11c the coordination screen. Not yet: allies' goals, which are
  not in the Observation. Goal 4's team board (12e) gives them a home.
- [~] 11d pings and fleet orders. Pings, dodging, kiting and the
  leader's ping as the fleet's order shipped on 17 October
  (CHANGELOG); left: orders from the coordination screen.
- [ ] 11e shared drones and transfers; a starved plan asks its rangers
  for material (journal 0036). 12f's job board carries the transfer.
- [ ] 11i build patterns over an area, declared by ping; the fleet's
  build list. 12f's base cycle is the bots' half of it.


## Goal 4. Bots that think as a team (step 12, the prototype built)

Bots that decide together, on one picture of the map, by one weighted
pattern a person can read, switch off in parts and tune with sliders;
that hold to a decision until it is confirmed, except to dodge or
retreat; that build economies, roads, forts and front lines; and that
cost little enough for sixteen at 4x game speed. The case for it is
`LAB-REPORT-BRAIN.md`. In short (LR 2, archived as the baseline): the
brain of 17 October was a stack of first-match ladders with 22
weights, all in combat; each tank decided alone on a full copy of the
world twenty times a second; the team shared nothing but one-tick
events. By the evening of 18 October the improved prototype (LR 20)
has every number as a named weight with a slider, 16 switches, a team
board, a crossing cache, a ledger, a fair census and a tuner; its new
behaviours are built and proven, and wait for the census to promote
them.

Rules for every part of step 12:

- Each part lands behind a bit of `Caps`, off until its census says
  on, so the brain of 17 October stays the baseline and every balance
  proof reads the same until a part is promoted.
- *Must not change:* the sim's rules, the wire (version 16), the
  replay. The brain is above `observe`; where 12b touches `observe`
  itself, the world hash and every sim test must be unchanged.
- A part becomes the default only by beating the baseline over the
  fixed census (the ignored `quad_census`, `three_fleets_build`, the
  fleet match over seven seeds), as rotation was promoted on kills 249
  to 260, or by Paul's choice, as `castle_cap` was (18 October). The
  census puts every team in the trial seat on every seed (LR 14), since
  a trial team in one seat drew the better starts (LR 12: 12.66 against
  6.44 with every team the same); a fair null run reads exactly even.
- `make test` green at every commit.
- Every either/or is a scalar (Paul, 18 October: "should it x or y, we
  choose a scalar of how much x, and how much y is weighted"). Where a
  design question asks "x or y", the answer is a weight in `Settings`
  that blends them, its default the choice made today, and the ledger
  and the tuner (12g) say where it should sit. A question that cannot
  be blended (a count, a yes or no) is a weight rounded or thresholded,
  said so in its doc line.

### 12a. Measure (LR 5, "Measure first")

*Change:* `treadcore-brain::meter`: per-layer counters (thinks, Moves
issued, A* routes asked for, crossing searches, plan clones) gathered
by every brain and printed by the census tests; a microseconds-per-bot
figure from `std::time::Instant` in the ignored census only, never in
a test that asserts. *Proof:* the baseline census written into the
journal before 12b lands, and again after each part.

### 12b. Memory (LR 5; findings L2, L3, L4, L6)

*Change:*
- `MapFacts`: islands labelled once per map (a flood fill, `u16` per
  tile), area, forest count and shore length per island, refreshed
  when trees fall; the crossing cache keyed by from-tile and to-tile,
  failures included, so `Map::crossing` is asked once per pair a minute
  instead of four times a second.
- The plan answers `turret_slot` and `yard_left` without a clone.
- Given-up lists (`given_up`, `depots_given_up`, `revive_failed`,
  Hop's `bad_*` and `no_bridge`, the fleet's `fails`) expire, so a
  target is retried after a while: `brain::memory::Forget<T>`, a small
  ring with expiry ticks.
- The Tactician's reasons and the ladder's `why` built only when the
  screen asks (`Brain::status`), not every think.
- Later in the step: the Observation shared per team per tick instead
  of copied per bot (the one change that touches the sim; the hash
  must not move).

*Proof:* the counters of 12a fall (crossing searches per minute, plan
clones to zero); every existing brain test passes unchanged; the
census reads the same within noise.

### 12c. Weights and Caps (LR 4)

*Change:* `brain::weights`: `Weights`, a flat `[f32; N]` named by a
`W` enum, with defaults equal to today's constants; `Caps`, a `u64` of
capability bits; response curves (linear, quadratic, logistic) with
their parameters in `Weights`; the geometric-mean scorer with the
commitment bonus. `Tactics` reads its 22 weights from `Weights`.
Loaded from a TOML file (`--weights FILE`), shown and set from the
coordination screen. *Must not change:* behaviour, at the defaults.
*Proof:* `default_weights_are_todays_constants`; the census at the
defaults equal to 12b's.

### 12d. Intent and smoothing (LR 7)

*Change:* `brain::intent`: the held intent replayed between thinks;
desire vectors for the move and the advance (alpha 0.4, beta 0.3);
confirmation with hysteresis (beat by h = 0.15 for n = 2 thinks, 3 for
a turret target; commitment m = 0.1); the survival bypass for dodge
and retreat; one resupply threshold for the Commander and its
doctrine. *Proof:* `a_challenger_needs_two_thinks`,
`a_dodge_skips_confirmation`, `the_turret_holds_between_equal_targets`;
Moves issued per minute down in the census, kills not down.

### 12e. The team board and the team brain (LR 5, LR 9)

*Change:* `brain::board::TeamBoard`, one per team, rebuilt at 1 Hz:
16 x 16 sector grids (influence, territory, forest left, supply
distance), a job board of persistent jobs (replacing the one-tick
`Requested` as the record; the event stays as the signal), the
defence-point roster, allies' goals for 11c. Postures (Economy,
Fortify, Connect, Expand, Defend) scored at the team level; rotation
across the team (swap a hurt tank at a hot point for a fresh one at a
cold point when the gap is over 0.3). *Proof:* `the_front_is_where_
influence_crosses_zero`, `a_hurt_tank_swaps_out`, a missed `Requested`
event no longer loses its job.

### 12f. Behaviours (LR 6)

*Change:*
- 12f1 the base cycle: fill, one item, one improvement, connect by
  road, escort a pillbox home along it (LR 6.1).
- 12f2 islands: a small island's base site penalised by its forest
  and area; crossings priced, `C_bridge(L) = L (1 + s) + k (L - 6)^2`
  against `C_boat = 5 (1 + s) + 3`; bridges of 6 or fewer preferred,
  20 the cap; part-laid bridges finished as board jobs (LR 6.2).
- 12f3 column on a bridge: the leader straight at full speed, slots in
  file two tiles apart, the wedge back two tiles past the end (LR 6.3).

*Proof:* `a_small_island_takes_a_boat`, `a_short_gap_takes_a_bridge`,
`a_fleet_crosses_a_bridge_in_file`, `a_full_tank_gives_the_base_an_
item`, `a_road_reaches_the_next_base`; the census: bridges finished
over started up, starved-seconds down.

### 12g. The ledger and the tuner (LR 8)

*Change:* every match writes one JSON line per seat to
`reports/ledger/`: the genome (weights, caps, seed, role, parent) and
the result (won, kills, deaths, revives, territory-seconds, items
built, road tiles, bridges finished and started, pillboxes hauled,
time to the first castle, starved-seconds). `scripts/tune.sh`: a
(1 + lambda) evolution strategy over the census on fixed seeds, a
hall of fame of 16 per role rated by TrueSkill's mu - 3 sigma, the
promotion rule above. Role sets are genomes too, so the fair fleet
start (seat 0's team won five of seven, journal 0036) is measured.
*Proof:* a ledger line reproduces its match from the replay; one
tuning generation runs by hand.

### 12h. The answers as scalars, and pillboxes that connect (LR 10)

Paul's answers of 18 October to the report's open questions, each a
weight (the defaults are his answers):

| Question | Weight | Default | 0 means | 1 (or more) means |
| --- | --- | --- | --- | --- |
| The improvement slot before 11g: mend, mines or works? | `w_mend`, `w_mine`, `w_works`; `upgrade_vs_mend` | 1, 0.5, 0.5; 0.5 | that kind never | that kind as often as the others; `upgrade_vs_mend` 1 spends the slot on 11g's upgrades once they exist |
| A hauled pillbox: the castle or the road? | `castle_inside`, `castle_road` (counts); `post_spacing`, `posts_max` | 2, 1; 10 tiles, 4 | no pillbox inside / on the road | the castle holds this many inside and this many outside on its road, then pillboxes go to posts along the roads between bases |
| Posts at junctions? | `junction_clear` | 3 tiles | posts may sit on a junction | no post this near a road junction; posts sit beside the road, never on it |
| Road out to a pillbox or to a base? | `connect_bases` | 0.75 | the nearest of either | a base or castle preferred, its distance counted at a quarter |
| The score: elimination or holding ground? | `long_game` | 0.5 | the win and kills (short matches) | holdings over time (long matches) |
| The same roles for every team? | `assist` | 1 | a person's team dealt as a bots' team | a person's team gets a builder (thresholded at 0.5) |
| Sliders for players or the host? | (not a blend) | | | the host's `--weights` file is every bot's default; a player's sliders on the coordination screen change only their own team's bots |

The castle rule (Paul: "we in general want to move the pillboxes near
the castle, but we also stop filling the castle once it has ... 2 on
the inside, 1 on the outside along the road. They need to not stick at
junctions but connect bases and castles"), capability `castle_cap`:
the castle is drawn with `castle_inside` pillboxes inside its wall (the
gate's two towers first, then the rear) and `castle_road` slots outside
along its road; no bastions. Once those stand, the turrets rung carries
outlying pillboxes to road posts: every `post_spacing` tiles on the line
from the base to each other friendly base or castle, beside the road,
`junction_clear` tiles from any junction, at most `posts_max`, so the
roads between bases are covered end to end and no pillbox stands where
roads meet. *Proof:* `the_capped_castle_has_two_inside_one_on_the_road`,
`posts_line_the_road_and_keep_off_junctions`.

Order: 12a, 12b, 12c, then 12d and 12e (independent), then 12f, then
12g. 12a to 12d are one journal entry's work or two; 12e and 12f
two or three each.

Status, 18 October 2026 (journal 0037). Every part is built, tested and
off by default; none is promoted, because three seeds of the census
cannot yet tell a part from the dice (baseline against baseline: 13.95
to 8.93 a seat).

- [x] 12a measure: `meter`, printed by `brain_census` and `quad_census`;
  28 us a seat-tick on naval.map.
- [~] 12b memory: the plan copy gone (`Plan::turret_preview`), crossings
  cached with their failures (`facts::MapFacts`), reasons built for the
  winner only, given-up lists that can expire (`memory::Forget`,
  `forget`); the status as codes (LR 22, `status.rs`): logic, the
  census and the coordination screen read codes, the words kept for
  people; measured, the strings cost nothing, and a census match is
  identical before and after. Not yet: the Observation shared per
  team (the sim change).
- [x] 12c weights and caps: `weights::Settings` (56 weights and 13 bits, 68 and 14 after 12h, 70 and 15 by the sliders, 73 and 16 with the bridge jobs, 76 and 17 with the duck, 77 and 18 with the posture, 83 with its scores),
  `--weights FILE` in the window, `default_weights_are_todays_constants`.
  The sliders, 18 October (LR 17): the coordination screen's weights
  page (`U`) for the player's own team, saved to
  `~/.config/treadcore/weights`; `a_switch_comes_back_and_a_slider_keeps_the_target`,
  `the_page_moves_a_weight_and_puts_it_back`.
- [~] 12d intent and smoothing (`smooth`): the desire vector in `go`,
  the turret's confirmation, one resupply threshold;
  `a_challenger_needs_two_thinks`, `a_dodge_skips_confirmation`; then
  the ladder's rungs confirmed and the advance scalar for the stance
  (LR 15). Measured fairly it cost 1.9 a seat (7.55 against 9.47, six
  naval matches), so it stays off for the tuner. A dodge for direct
  fire measured first (LR 19): a led direct shot cannot be dodged once
  fired, at any range (`a_led_direct_shot_cannot_be_dodged`); in its
  place the duck (`duck`, LR 21; `a_tank_ducks_a_gun_ready_before_its_own`):
  out of a gun's range or line before it is ready, seen or remembered;
  measured fairly at 7.01 a seat against 7.52, inside the noise, so it
  stays off.
- [x] 12e the team board (`board`, `team_rotate`):
  `the_front_is_where_influence_crosses_zero`, `a_hurt_tank_swaps_out`,
  `a_missed_job_is_still_on_the_board`. Not yet: allies' goals shown on
  the coordination screen. Measured alone (LR 24), the board changed
  no decision: only `team_rotate` and `bridge_jobs` read it. The
  posture read, 18 October (`posture`, `w_posture`; LR 24): the Auto
  ladder leaned by the team's posture, `(1 - w) order + w lean`
  (`Rung::pick`, `the_posture_leans_the_ladder`); its first match found
  Defend in 94% of picks, a long-tailed threat field, fixed
  (`the_team_defends_a_threat_at_the_base_not_far_off`); measured at 48
  seeds, -0.81 a seat with an error of 0.65, so it stays off. The
  posture as scores, the same day (LR 25; `posture_scored`, the five
  `w_pos_*`): `each_posture_scores_its_own_case`; 48 seeds, -1.04 with
  an error of 0.58, no different from the ladder's (-0.23 +/- 0.74
  apart); the two together -0.93 +/- 0.49, so the lean, not the
  posture, is the suspect (backlog 27). Which rungs it moves (LR 26):
  Fortify to Expand, Turrets -88%; with Economy off Turrets recovers
  half and the score does not move. The ladder decides 5.6% of a
  team's seat-seconds; 46.7% are dead. Not yet: influence, the front
  and territory read by nothing.
- [~] 12f behaviours: the base cycle (`cycle`,
  `a_full_tank_gives_the_base_an_item`), priced crossings and the island
  site (`priced_crossing`, `island_site`, `a_small_island_takes_a_boat`),
  the column (`column`, `a_fleet_crosses_a_bridge_in_file`), the
  escort (`escort`, `a_far_fetch_gathers_the_fleet`: a far fetch
  gathers the fleet, which screens the carry home along the road; LR
  16; it never acted in a 900 s naval census, so it waits on longer
  matches); bridge jobs on the board (`bridge_jobs`, `bridge_wing`;
  LR 18; `a_teammate_finishes_a_part_laid_bridge`: a part-laid bridge
  is posted, taken by the next teammate with material, wingmen too,
  and laid; measured fairly at 7.31 a seat against 8.01 with `board`
  on for all, inside the noise, so it stays off).
- [x] 12g the ledger and the tuner: `brain_census`, `ledger_ratings`,
  `make tune`. The census's error (LR 24): the difference a seed, its
  standard error, "outside the noise" past two errors
  (`ledger::mean_se`); `make switches`, each switch alone;
  `CENSUS_SEED0` to split a census over the cores. Not yet: a tuner
  that can see what it selects (CI-001 VI: at two seeds it selects
  noise; backlog 23).
- [x] 12h the answers as scalars, the same day: the weights of the
  table above, `castle_cap` (`Plan::castle_capped`, road slots in place
  of bastions, `Plan::add_posts`, `Cycle::road_posts`), the improvement
  by weight (`Cycle::improvement`: mend, a mine on the approach, works),
  `connect_bases` in the road out, `assist` in the roles,
  `ScoreWeights::from_long_game`. Proof:
  `the_capped_castle_has_two_inside_one_on_the_road`,
  `posts_line_the_road_and_keep_off_junctions`,
  `the_improvement_goes_by_its_weights`,
  `a_capped_castle_fills_then_lines_the_road` (alone: the castle up with
  two inside, the road slot and a post toward the second depot filled
  by 371 s); the sliders (LR 17). Not yet: `upgrade_vs_mend` has no
  upgrades to weigh until 11g.
- [x] `castle_cap` the default, 18 October (Paul's choice; LR 12): the
  old castle one bit away (`-castle_cap`), its two proofs kept with the
  bit off. Paired against the null run, the old castle scored 0.96 a
  seat less over six naval seeds, inside the noise.

Done 18 October (LR 24, CI-001): every capability alone at twelve
seeds; none outside two errors; `board`, `island_site` and `escort`
never acted. The posture as scores (backlog 21, LR 25), no better. Which rungs the
lean moves (27, LR 26): the ladder decides 5.6% of the time, and half
of it is dead. What kills (28, LR 27): the dodge, 43% of deaths, and
the run for the beach, 20%. The dodge off reads -2.45 a seat (LR 29):
it pays ashore and, by the meter by team (LR 30), afloat a fifth of
the boats. Next, one at a time: the landfall (30), the dodge afloat
(31), a census that
lets late parts act (29); an honest tuner (23); `island_site` where a
naval start chooses (20).
Twelve seeds resolve a point a seat; a half point takes 10 to 300 seeds
by the part (CI-001 IV.A), one night's census for all of them.


## Goal 5. Longer, truer matches (open)

Matches end today in eight to sixteen minutes by elimination; Paul
asked for longer ones. Steps:

- [ ] 11f real numbers and tuned ranges. Open: replace the game's
  numbers, or ship them as `--game-type real`.
- [ ] 11g upgrades, designed 18 October (Paul: "think of more base
  upgrades and ways for users to purchase them"). In-match, bought for
  **supply**, a new count a side earns from what it holds (each
  pillbox and depot a little a minute, the ledger's `held_s` made a
  currency) and from kills, and installed by the squad at the base
  for material, as a structure is. A rules and wire change, so a
  proposal first (GOVERNANCE.md section 5), then a sim step of its
  own. The proposal is written: `PROPOSAL-UPGRADES.md` (18 October:
  supply's earning rates, sixteen upgrades with prices, `Command::Buy`
  and `PillboyTask::Install`, wire version 17, the proofs and the
  traces it needs); next, posted for its seven days, and the two traces
  of its section 6.

  | Upgrade | Levels | What it does | Bought |
  | --- | --- | --- | --- |
  | Hardened walls | 1 | the castle's wall tiles become concrete | supply + 1 material a tile |
  | Pillbox calibre | 2 | the base's pillboxes reach 9, then 10 tiles | supply |
  | Pillbox plating | 2 | the base's pillboxes 75, then 90 health | supply |
  | Watchtower | 1 | 11j's flag: the town sees 12 tiles round the pedestal | supply + material |
  | Radar mast | 1 | the base's alarm at 24 tiles, not 16; incoming shells shown | supply |
  | Gatehouse | 1 | the gate opens for the side and shuts for the rest | supply + material |
  | Depot stores | 2 | the depot refills faster and holds more material | supply |
  | Yard workshop | 1 | pillboxes and walls inside the wall mend slowly | supply + material (a workshop on the yard) |
  | Minefield | 1 | the squad lays the moat's mines for half price | supply |
  | Dock | 1 | boats at the base cost 3 material, not 5; a launch site by the wall | supply + material |
  | Bridge works | 1 | bridge tiles next to the side's roads cost half | supply |
  | Supply road | 1 | the side's tanks drive faster on its own roads | supply |
  | Tank: plating, bay, squad, hopper | 2 each | armour 100 to 120; one more drone of each; a fifth Pillboy; material 8 to 12 | supply, at a depot |

  Ways to buy: at any depot of the side, from the coordination screen's
  works tab (`Command::Buy`, validated by the world: cost, place, the
  level before); a fleet commander's bot buys for its team by the
  improvement slot (`upgrade_vs_mend`), by utility, the team's posture
  leaning it (Defend: plating and calibre; Connect: roads, bridge
  works, the dock; Economy: stores); a person's ally can pool supply
  with them (11e's transfers). Not carried between matches; a veteran's
  list across matches is a later question.
- [ ] 11j the flag and the town's vision (a pedestal that gives a
  walled town sight; the radius open).
- [ ] The match score of 12g weighted toward territory held over time,
  if longer matches are the goal.


## Goal 6. In players' hands (waiting)

0.1.0-rc.6 is the last candidate; it speaks wire version 12 and cannot
meet today's builds. GitHub Actions are paused since 13 October: no
`v*` tag and no workflow run until Paul lifts the pause. Then: rc.7 on
wire 16, tagged on day 0 and published on day 1 or later (`DEPLOY.md`);
PROTOCOL.md brought to wire version 16 first.


## Backlog

Proposals in waiting, each with the goal it serves. New items go at
the end with a date.

1. Fair fleet start: deal every team the same roles (journal 0036,
   seat 0's team won five of seven). Goal 4, measured by 12g.
2. A boat on naval.map at (61,102) keeps choosing (60,102) and sails
   in place (CHANGELOG, Unreleased). Goal 4, 12f2.
3. Most bridges left part-laid; the road on from a bridge about a tile
   a match. Goal 4, 12f2. Answered in part 18 October (LR 18): a
   part-laid bridge is a job on the team board (`bridge_jobs`, off);
   the road on is still about a tile a match.
4. Revives fell from 113 to 63 over a census (CHANGELOG). Goal 4.
5. Kiting even at best; plain RuleBot kiting tried and off. Goal 4,
   12c then 12g.
6. The RuleBot attacking a castle, unproved (journal 0036). Goal 3.
7. A wingman in a headless server's seats (journal 0034). Goal 3.
8. Direct fire is never dodged before it lands (LR 2). Goal 4, 12d.
   Measured 18 October (LR 19): it cannot be, once fired; backlog 17
   holds what could be done instead.
9. A Hunt target given up after twelve bearings is given up for good
   (LR 2). Goal 4, 12b.
10. Done 18 October (LR 23): `partial_cmp().unwrap()` in the brain
    panicked on a NaN; see 19. Goal 4, 12b.
11. PROTOCOL.md says wire version 14; the source is at 16. Goal 6.
12. `make test` not recorded since 17 October's commits (journal 0036
    has the last). Goal 4, 12a. Recorded again 18 October: brain 34
    passed, 6 ignored before step 12; journal 0037 has the run after.
13. The open questions of LR 10, answered 18 October as scalars (12h):
    the improvement slot (`w_mend`, `w_mine`, `w_works`,
    `upgrade_vs_mend`); the castle holds 2 inside and 1 on its road,
    then pillboxes go to posts along the roads between bases, never at
    junctions (`castle_cap`, `castle_inside`, `castle_road`,
    `post_spacing`, `posts_max`, `junction_clear`); road toward bases
    first (`connect_bases`); the score between elimination and holdings
    (`long_game`); the builder for a person's team (`assist`); sliders
    for the host's file and each player's own team.
14. Upgrades and the ways to buy them (11g), designed 18 October; the
    proposal written the same evening (`PROPOSAL-UPGRADES.md`). Left:
    post it for the seven days, run its two traces, then the sim step.
    Goal 5.
15. Done 18 October (LR 17): the sliders, the coordination screen's
    weights page (`U`), one row per capability and weight, for the
    player's own team, saved beside the keys file. Goal 4.
16. Done 18 October (LR 14): the census puts every team in the trial
    seat on every seed (`CENSUS_FAIR`); a null run reads exactly even,
    and the capped castle measured again at -0.72 a seat for the old
    one. Goal 4, 12g.
17. Done 18 October (LR 19, LR 21): direct fire is denied, not
    dodged. Paul chose the duck: `duck`, with `w_duck`, `duck_ready`
    and `duck_memory`; off, measured inside the noise. Goal 4, 12d.
18. Measured 18 October (LR 24): each switch alone at twelve seeds,
    none outside two errors; see 22 for the seeds a half point needs.
    As asked: the census at twelve seeds or more for each switch
    alone (`forget`, `board`, `team_rotate`, `priced_crossing`,
    `column`, `cycle`, `island_site`), none measured alone yet; and
    longer matches for the parts that pay late (`escort`,
    `bridge_jobs`, `cycle`); and `duck`, which acted little in 900 s.
    Goal 4, 12g; goal 5.
19. Done 18 October (LR 23): all 74 calls in the brain, and 19 in the
    window and the sim, are `total_cmp`;
    `a_nan_does_not_panic_the_brain` fails on the old code and passes
    on the new, and a census match is identical before and after.
    Goal 4, 12b.

From item 20 each item is written in the form of CI-001 section VIII:
cause, goal and step, hypothesis, proof (with the seeds a census needs,
(2 sigma / delta)^2), cost, state.

20. `island_site` where a naval start chooses. (18 October; cause: LR
    24, it never acted.) Goal 4, 12f2. Hypothesis: a fleet leader
    choosing its first landing by site score (forest, island size)
    builds more and starves less. Proof: a test where a small bare
    islet is nearer than a large forested island and the leader lands
    on the island; the census with `island_site` reading nonzero
    seat-seconds, then d and its error at 36 seeds (sigma unknown until
    it acts). Cost: 108 matches, 6 min. State: idea.
21. The posture as scores. (18 October; cause: LR 24, CI-001 A3; Paul's
    rule, every either/or a scalar.) Goal 4, 12e. Hypothesis: the
    board's posture, a first-match ladder on thresholds, scored as LR 9
    designed (each posture a 0..1 score, the blend picks), reads the
    match better and lets `posture` pay. Proof: a test per posture's
    score; the census of `posture` at 48 seeds against LR 24's -0.81 +/-
    0.65. Cost: 144 matches, 8 min. State: done 18 October (LR 25):
    scored, the ladder kept at `posture_scored` 0; -1.04 +/- 0.58,
    no different from the ladder's; see 27.
22. 18 October: the switch census at 200 seeds a switch, one night,
    so a half point a seat is outside two errors for every part (CI-001
    IV.A). Goal 4, 12g. Proof: `SEEDS=200 JOBS=3 make switches`. Cost:
    6,600 matches, about 6 h. State: idea; supersedes the seed count of
    18.
23. An honest tuner. (18 October; cause: CI-001 VI.B, the tuner selects
    noise; Paul: "ways to test and grow these genetic algorithms".)
    Goal 4, 12g. Hypothesis: re-evaluating the parent on fresh seeds and
    accepting a child only past two errors, then SPSA on the paired
    census, finds weights that hold on new seeds. Proof: the planted
    test (CI-001 VI.D): from `w_wounded` 0 the tuner returns toward
    1.2; a found genome confirmed on 48 unused seeds and a second map.
    Cost: about 600 matches a generation, 35 min. State: designed.
24. Scenario files: a playtest described in text and run headless at
    85x. (18 October; cause: Paul, "is there a way to create description
    and try to attempt these via playtests faster".) Goal 4; goal 1.
    Hypothesis: a TOML file (map, tanks, terrain, board, caps,
    weights, expectations; CI-001 VII.B) and `make scenario` replace a
    hand-written Rust test for each new behaviour. Proof: three of
    today's tests rewritten as scenarios pass and fail as they do.
    Cost: a step. State: designed.
25. The census on more than one map. (18 October; cause: CI-001 A8,
    every census since LR 11 is naval.map.) Goal 4, 12g. Hypothesis: a
    part's difference holds on a generated map as on naval.map.
    Proof: `CENSUS_MAP` over a list; promotion asks two maps. Cost:
    doubles a census. State: idea.
26. 120 of 337 brain functions have no doc comment (`BRAIN-MAP.md`;
    CI-001 A9); `crew.rs` and `hop.rs` have no tests of their own (A10).
    Goal 4. Proof: `make brainmap` reading fewer; a test each for the
    crew's job hand-out and a hop's bridge choice. Cost: a step. State:
    idea.
27. Which rungs the posture's lean moves. (18 October; cause: LR 25,
    leaning the ladder reads -0.93 +/- 0.49 in both forms of the
    posture.) Goal 4, 12e. Hypothesis: Economy, read 37% of the time
    from material spent, leans tanks from Fortify and Turrets to
    Expand, and that costs the point. Proof: the census counts the
    trial team's seat-seconds on each rung, with the lean and without;
    then the leanings as weights (a 5 x 6 table, 0 or 1 today) for the
    tuner. Cost: one 48-seed census, 10 min. State: done 18 October
    (LR 26): Fortify to Expand 84% of changed picks, Turrets -88%;
    Economy off restores half of Turrets and none of the score (+0.13
    +/- 0.47); not confirmed. The ladder is 5.6% of the time.
28. Survival: half the team's time is dead. (18 October; cause: LR 26,
    46.7% of the trial team's seat-seconds dead, 3.4 deaths a seat
    against 3 respawns.) Goal 4. Hypothesis: what a seat was doing when
    it went down, read by code, names the two or three situations that
    kill most, and a fix there moves the score more than any part so
    far. Proof: a census histogram of the code a seat held in the 5 s
    before each death (`CENSUS_ACTS`); then the part, with its test and
    48 seeds. Cost: one 16-seed census to look, 5 min. State: looked
    19 October (LR 27): the dodge is the code held last at 43% of
    deaths (4.5% of the time) and `landfall` at 20% (3.2%); afloat, a
    dodge ends in a kill one time in six. `dodge_hold` 1 (LR 28):
    -1.18 +/- 0.55 at 48 seeds, the dodged shell's hits unchanged; it
    stays 0. `-dodge` (LR 29): -2.45 +/- 0.67, so the dodge pays and
    stays on; ashore it nearly halves the dodged shell's hits, afloat
    it changes nothing. State: done; the dodge afloat is 31,
    `dodge_horizon` and `dodge_clear` go to the tuner (23).
29. A census in which late parts act. (18 October; cause: LR 26, the
    census measures attrition; the escort, the cycle, bridge jobs and
    the turrets pay late.) Goal 4, 12g; goal 5. Hypothesis: with
    unlimited respawns (or 1,800 s), the late parts read differently
    from LR 24. Proof: `CENSUS_RESPAWNS=254` (or `CENSUS_SECS=1800`)
    on `make switches`, beside LR 24's table. Cost: 396 matches, 25
    min (twice that at 1,800 s). State: idea.
30. The run for the beach. (19 October; cause: LR 27, 165 deaths of
    819 on `landfall`, 132 of them a boat sunk by direct fire.) Goal 4.
    Hypothesis: `landfall_order` prices a beach by the threats near it
    but not the boat's run past a gun in range; a boat in a tank's
    direct range turns away from it (a weight, `landfall_shy`, blending
    the nearest beach and the safest) and the deaths there fall by a
    third. Proof: a test of the turn; the landfall deaths and the score
    at 48 seeds. Cost: a step. State: idea.
31. The dodge afloat. (19 October; cause: LR 29, with the dodge off an
    undodged boat is hit by the shell about as often as a dodging one,
    8% against 7.6%, and killed no more often.) Goal 4. Hypothesis: a
    boat's dodge costs its crossing and saves nothing; a weight,
    `dodge_afloat` (1, today's), the share of a boat's shell dodges
    taken, at 0 keeps the boat sailing and gains a share of LR 27's 165
    landfall and 65 sailing deaths back as time ashore. First the meter
    counts would-be dodges apart from taken ones, by team. Proof: a
    test that a boat at 0 holds its course under a shell; 48 seeds.
    Cost: a step. State: the meter by team done 19 October (LR 30);
    measured exactly, a boat's dodge cuts its kills by a fifth (18.2%
    to 14.7%), so LR 29's premise is withdrawn and the gain, if any, is
    the crossing time traded for that fifth: after 30.


## Sequencing

```
goal 1 (0 to 9) ── goal 2 (10) ── goal 3 (11a 11b 11h; 11c 11d 11e 11i left)
                                      │
                                      └── goal 4: 12a ── 12b ── 12c ─┬─ 12d ─┬─ 12f ── 12g ── 12h
                                                                     └─ 12e ─┘
goal 5 (11f 11g 11j) after 12c, so the numbers are tuned with the weights
goal 6 when the Actions pause lifts
```
