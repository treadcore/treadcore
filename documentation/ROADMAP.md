# The next architectural steps

Where the architecture goes from the opening day, in the order the
steps depend on each other. Each step says what it changes in which
crate, what it must not change, and how it is proven. The constraint
that shapes all of it: the simulation stays headless and deterministic,
and every layer above it keeps driving it through `step` and `observe`.

## 0. Prove what exists (before any of the below)

Status after the second session (12 October 2026, evening; journal
0002):

- [x] `make test`: every crate compiles, 16 tests pass. The dev-profile
  engine built as a side effect in 4 min 28 s on the 8 GB VM. The
  release-profile engine (`make warm`, `make run`) is not yet built or
  measured.
- [x] `make debug`, read: three bugs and two balance findings, below.
  Fixed: the UDP receive buffer (no client could ever join), `Move`
  across water (the sim now routes), the bash `SECONDS` collision in
  the Makefile. Rewritten: the rule bot's doctrine, so a trace
  exercises indirect fire. Still open: the balance items in the
  backlog.
- [x] `make lobby-test`, with a real assertion (three clients welcomed,
  all saw the match end). Two machines: not yet.
- [x] The window. `treadcore-warm` panicked in this VM with no Vulkan
  driver (`BUILD.md`, "The window in this VM"); with lavapipe in, the
  window has run since 13 October (journals 0013, 0014), and `make
  shoot` drives it for every release's frames.

Nothing below should start until step 0's open problems are closed, or
every later step is built on a guess.

## The reference specification

`SPEC-PRIOR-ART-1.14.md` is the clean-room functional specification of
the prior art, version 1.14, received 12 October 2026. Every step below that
touches the prior-art-derived layer (3, 5, 6, 6b, 6c, 6d) cites its
requirement IDs, and a proposal that changes one of those mechanics
says which IDs it keeps, adapts or drops. Its preface says which parts
treadcore adopts; its networking (section 12) is reference only, and
nothing of the prior art's LAN ring is in it or in treadcore.

## Backlog: what the first traces found

Carried from journal 0002. Each is a proposal in waiting
(`GOVERNANCE.md` section 5): a decision, with a trace to argue from.
Where an item has a counterpart in the reference specification, the ID
is given.

Worked through on 13 October 2026 (journal 0009): every item is done,
decided from a trace, or deferred to the step that owns it. The traces
are eight seeds of sixteen bots for 300 game seconds, read through the
match report (item 8), figures per match.

1. **Interceptors fight interceptors.** When two bubbles overlap
   (tanks inside 2 x bubble + margin of each other) both rings
   annihilate each other in two seconds, and the bots refill them from
   the bay until it is empty; the first trace shows 22 intercepts in 20
   seconds and three tanks dead to strikes through the stripped rings.
   Decision wanted: should an interceptor on station engage a hostile
   *interceptor* at all, or only strikes and recon? The real-world
   analogue says engage anything; the game may want otherwise.
   **Done: strikes only** (and a recon inside the bubble, as before).
   Engaging everything: 7.0 kills by shell to 5.9 by strike, 32
   interceptors shot down, 2.8 of 16 alive at the end. Strikes only:
   8.8 to 2.9, 6 shot down (by pillboxes), 4.1 alive. The bubble is a
   shield, not a sword; the analogue was rejected for the game's sake.
2. **Tank speed.** 4 tiles/s is 360 km/h; the map is crossed in
   fifteen seconds and a match of four bots ends in under two minutes.
   `rules.rs` already calls it a game speed. Try 2.5 and read the
   trace. **Decided: keep 4.** With item 1 in, 2.5 moved the kills
   from 8.8 : 2.9 to 9.8 : 2.2, inside the noise; and four bots on the
   sixteen-start map play out the time limit at either speed (first
   kill at about 200 s). The complaint was of the four-start map. One
   constant; a human playtest decides.
3. **Retasking a drone.** The only way to move a recon is `Recall`,
   which brings every drone home. A `Command::Retask { drone, task }`
   (recon orbit centre; a single drone's recall) is small, and both the
   bot and a human need it. Interceptors stay the world's to task.
   **Done.** A recon's orbit, a strike's target, or any one drone's
   recall; `DroneRetasked` to allies. The rule bot moves its recon to
   an enemy it cannot see instead of launching a second. The window
   has no key for it yet.
4. **Rejected orders repeat.** A bot whose `Move` is refused (a point in
   water) re-issues it four times a second. Harmless; noisy in the
   trace. The bot should remember a refusal for a few seconds.
   **Done.** The rule bot already remembered what was unreachable
   (step 5); its one repeat left was a led shot falling inside minimum
   range, 8 a match, now aimed at the tank instead. The Fortifier
   ordered a build with no material and no forest in sight; it waits.
   The default trace has 0 refused orders; twelve defenders have 46,
   one per forest tile in a ring, which is how a Fortifier with no map
   learns where the trees are.
5. **The HUD's rtt is not an rtt.** The client shows snapshot tick
   minus last-heartbeat tick, which is mostly heartbeat age. Use the
   host's `rtt_ticks`, sent back in `Heartbeat`. **Done**; wire
   version 9.
6. **`Welcome` is one 16.5 KiB datagram.** Done with step 5: any packet
   too big for a datagram goes as `Chunk`s (PROTOCOL.md).

7. **The strike is the kill.** In every trace so far the tanks die to
   strike drones (35 x 3 in the bay) after the rings are stripped, not
   to shells. If the premise is that the gun is the weapon, either
   strikes need a longer approach (battery) or interceptors need to
   survive their own melee (item 1). **Decided by item 1.** Final
   trace: 9.2 kills by shell to 2.5 by strike; 10 % of strikes get
   through (24 % before). Battery and damage unchanged.
8. **A match report.** `make debug` should end with the numbers a
   balance reader wants (shells by kind and spotted, intercept rate,
   kills by cause, time to first kill) instead of a reader counting
   JSON lines. **Done**: six lines after the scoreboard, in a live
   match and a replay alike.
9. **Depot armour as the gate.** The spec's bases cannot be driven
   onto by an enemy while they hold armour; shooting them opens them
   (SPEC BAS-07 to BAS-09). treadcore's depots are taken by standing
   on them for ten seconds. Decide whether the shell should be the key.
   **Decided: not adopted.** Two objects, two keys: the pillbox is
   shell-gated (shoot it dead, then the squad takes it), the depot is
   time-gated. A gun with a 12-tile minimum would have to bombard the
   thing it wants intact, and standing on a depot for ten seconds is
   what makes the capture a risk. Revisit with a playtest.
10. **Rejoin.** A player who drops and returns within a window keeps
    their seat and what was theirs (SPEC GAM-12, OWN-04). Needs a
    peer identity that survives a process restart. **Deferred to steps
    7 and 8**, where identity across processes is designed once.
    **Done in step 7** (journal 0012): `--identity FILE`, the host's
    standing registration with the introducer, a `session` number on
    `Join`. The seat waits as long as the match, not five minutes, and
    what was the player's never changed hands.
11. **Game types.** Open, Tournament and Strict Tournament (SPEC
    section 11) as `Scenario` presets once pillboxes and material exist.
    **Done**: `Scenario.game_type`, what a tank starts with (open: 24
    shells, 2 material; tournament: 12, 0; strict: 0, 0); `--game-type`,
    `GAMETYPE=strict make debug`.
12. **Pillbox view.** Jump the camera between owned pillboxes (SPEC
    UI-10 to UI-13); the client's one the prior art feature worth having early.
    **Deferred** to the first session with a working window (step 0's
    open problem); a client change that cannot be seen is a guess.
    **Done 13 October 2026** (journal 0014), the first day with a
    window: the apostrophe steps through the player's and allies'
    pillboxes, Return or F returns to the tank (UI-10, UI-12, UI-13);
    the arrows to a neighbouring pillbox (UI-11) are not taken.
13. **Forest concealment as the prior art had it.** Hidden to everything beyond
    3 tiles when inside a 2 x 2 block of forest (SPEC VIS-02), against
    treadcore's halved vision radius. A proposal with a trace. **Done**,
    as written, pillboxes included (PIL-07): `CONCEAL_RANGE` 3, beside
    the halved radius in single trees. No bot stands in forest yet, so
    the proof is a test; a brain that uses cover is step 6d's.
14. **Pillboxes never fire in a bot match.** With the router keeping
    tanks out of hostile rings, the default sixteen-bot trace has zero
    `PillboxFired` events in three minutes: the gun is proven by test,
    not by play. The proof of step 3 therefore rests on wins (18:2) and
    survival (37:32), and its score margin is mostly capture points.
    Step 6d's attackers need a reason to go into a ring (a depot or a
    dead pillbox inside one), and a defender needs to put its pillbox
    where the road has to pass. Design work, with traces. **Done in
    6d as far as it goes:** the attacker storms a depot it cannot
    shell clear, and the defender's pillbox sits at the mouth and on
    the spoke; and still 0 to 4 pillbox shots a match, because the
    artillery duel is decided before anyone drives up. The pillbox's
    job in this game is to make a depot untakeable by a tank, not to
    kill; the trace says that is what it does (101 of 120 depots
    held). Closed with that reading; a playtest may reopen it.



## 0b. Sixteen seats (done the same evening)

`MAX_PLAYERS` 16; sixteen starts round the map's edge (the four sides
first, then the corners, then the eight between), a depot each and the
centre, a ring road through every start and spokes from the four sides;
`--players N` on the server and the window, `PLAYERS=n` on `make`;
seats named for their starts. Proof: `sixteen_seats` and
`every_start_can_reach_the_centre` pass; `make debug` runs sixteen
bots for 180 game seconds in under a second of wall time with 62
shells, 128 intercepts, 10 kills and 8 captures on the default seed.

## 1. The replay: traces that run

**Done 12 October 2026 (journal 0003).** `Replay { scenario, ticks,
orders: Vec<(Tick, Order)>, checkpoints: Vec<(Tick, u64)> }` in
`treadcore-sim`, serde; `World::run(&replay)` and `Replay::verify()`
(the first checkpoint that differs, with both hashes). The host records
when asked (`Host::record`); the server has `--record FILE` and
`--replay FILE`; `make debug` records beside its trace and `make
replay` plays the newest back, exit 1 on a mismatch. Proof so far: a
recorded sixteen-bot match replays to the same hash on this machine,
and a replay with one shell removed is caught at the first checkpoint
after it. A different machine is still to do.

**Change.** `treadcore-sim` gains `Replay { scenario, orders: Vec<(Tick,
Order)> }`, serialisable; `World::run(replay) -> World`. The server
records one (`--record FILE`) and plays one back (`--replay FILE`),
asserting the final world hash matches the recorded one.

**Why first.** Every later step (prediction, teams, Pillboys) changes
the step function. A replay corpus from the current rules is the
regression test that says a change did only what it meant to. It is
also the debugging tool for "the bot did something odd at tick 3 412".

**Must not change.** The `Order` type's wire form; a replay is a list
of the same packets' payloads.

**Proof.** A recorded bot match replays to the same world hash on a
different machine.

## 2. World hash and desync detection

**Done 12 October 2026 (journal 0003).** `World::hash()`: FNV-1a over
the bincode serialisation of the world, so every field is in it the
day it is added and floats hash by their bits. The host hashes every
`HASH_EVERY` (1 000) ticks, logs it, and carries it in `Heartbeat`
(wire version 3); clients log what they receive. `make lobby-test`
asserts one hash at tick 1 000 in all four logs.

**Change.** `World::hash() -> u64` over the state that matters (tick,
rng, every entity's fields, map cells). The host sends it in
`Heartbeat`; a client that also runs the world (step 4) compares.


**Why.** Before prediction exists, the hash makes replay testable
(step 1) and gives `make lobby-test` a real assertion: every peer
observed the same hash sequence.

**Proof.** Four lobby-test peers log identical hashes at tick 1 000.

## 3. Pillboys: the control model, then the unit

**Done 12 October 2026 (journal 0004).** `Pillboys` on `Tank`
(strength, aboard, pos, task, progress, material, route);
`Command::Deploy { task }` with `Capture`, `Repair`, `Build { Pillbox,
at }`, `Return`; `step_pillboys` walks the world's route at 1.5 tiles/s,
works for the capture, repair or build time, walks home; a splash over a
deployed squad costs it two men, zero is no squad until a depot. `Pillbox`
entity: health, owner or neutral, a hitscan gun at hostile tanks within
8 tiles with a clear line (15 damage every 2 s), an interceptor's odds
against hostile drones within 4; dead at zero, never removed; captured
by a squad standing on it for 10 s when neutral or dead; repaired and
built for material. Twenty neutral pillboxes on the map: one on each
spoke and four round the centre. A tank's route avoids the range of
live hostile pillboxes. Wire version 4. Proof: `pillboys_pay`, twenty
seeds of eight bots, the four with squads against the four without;
the numbers are in journal 0004 and in the test's output.

**Decision first.**
 Written into DESIGN.md on 12 October 2026 ("Pillboys
and pillboxes: the decision"), checked against the prior art's builder and
refuelling bases (`LAB-REPORT-PILLBOYS.md`): Pillboys are an *order the
tank gives*, not a unit the player drives. The tank is the one thing
the player drives; that is the premise. `Command::Deploy { task:
PillboyTask }` where the task is `Capture { pillbox }`, `Repair {
target }`, `Build { what, at }`, `Return`. The squad is a state on the
tank (strength 0..N, deployed/aboard, position when deployed, task,
progress, material) that the HUD shows beside the drones. Depots are
not a Pillboy's business: they are refuelling bases, taken and used by
driving onto them, as now.


**Reference.** SPEC-PRIOR-ART-1.14.md section 6 (BLD-01 to BLD-03,
BLD-05, BLD-06, BLD-09, BLD-10, BLD-13) for the squad; section 8
(PIL-01 to PIL-05, PIL-07 to PIL-17) for the pillbox; OWN-01, OWN-02
for ownership. BLD-11's parachute becomes resupply at a depot.

**Change.** `treadcore-sim`: `Pillboy` squad struct on `Tank`; a
`step_pillboys` pass; squads are vulnerable to splash and to strikes
while deployed, so deploying under a thin bubble is a decision.
`Pillbox` entity: position, owner, health, a gun that engages tanks
within range and drones within a shorter range; captured by a squad
that stays on it for `CAPTURE_TICKS` uncontested. Depots keep their
role (resupply); pillboxes become territory.

**Must not change.** Drone and shell mechanics. Pillboxes shoot
drones with the same intercept-probability shape (a link to the owner
tank's radar if in range, else their own sensor), so one rule serves
both.

**Proof.** A bot that captures the two nearest pillboxes beats one
that does not, over twenty seeds, by a margin the trace can show.

## 4. Client-side prediction of the player's own tank

**Done 13 October 2026 (journal 0011).** As written, with one
narrowing and one widening. The client's `World` is partial: built
from `Welcome`, it holds the map (kept up from terrain events), every
pillbox, the mines the player knows, the teams, and the player's own
tank; nothing else, because a client is shown nothing else, so it
never calls `World::step`. The sim exposes `World::predict`, which
applies one player's hull orders and drives the tank by the same
`drive_tank` the host's `step_tanks` calls (`treadcore-net/src/
predict.rs`, wire version 10: `Snapshot` carries `acked`, the last
seq applied). The widening: the hull orders are `Move`, `Stop` and
both fire orders, because firing turns the tank; the first lagged
lobby test found that one as fourteen mismatches at zero tiles off.
The check is bit-for-bit (`Tank::tick_state`) whenever nothing of the
client's is in flight. Proof, as it came out: there is no `tc` and no
root in this VM, so the transport grew a lag queue (`TREADCORE_LAG_MS`;
`make lobby-test LAG=200`) that holds every received datagram.
`prediction_hides_the_lag_and_matches_the_host`: 100 ms each way,
the predicted tank moves 250 ms before the snapshot shows it, 53
checks, 0 mismatches. `LAG=200 make lobby-test`: three client seats,
598 snapshots each, 1 602 checks, 0 mismatches; the test now requires
that. The window draws the predicted tank in place of the snapshot's
and the NET panel shows ticks ahead, orders in flight, checks and
mismatches; it builds, and is unproven in this VM as before. A client
still sends no world hash: it cannot run what it is not shown.

**Change.** `treadcore-client` runs its own `World` from `Welcome`
(seed + map + scenario), applies its own orders immediately, and
reconciles on each snapshot: replace own-tank state with the host's,
re-apply unacked orders. Everything that is not the player's tank is
drawn from the snapshot as now.

**Why after 1 and 2.** Prediction without a hash to check against is
guesswork; with step 2 the client logs every mismatch.

**Must not change.** The host stays authoritative; a predicted state
never leaves the client.

**Proof.** With `tc netem delay 200ms` on loopback, driving feels
immediate and the mismatch log is empty on a straight road.

## 5. Map files and the editor

**Done 12 October 2026 (journal 0006).** `Map::to_text` and
`Map::from_text` (`documentation/MAPS.md`): a format line, `size`,
`seed`, 128 rows of glyphs, then `start`, `depot` and `pillbox` lines;
the loader refuses with the line and repairs an owner that is no seat.
`Scenario.map: MapSource::Seed(u64) | Text(String)`, so a replay and a
`Welcome` carry the map whole (wire version 6; `Welcome` sends the
`Map`, not its cells). `World::map_as_played`; the server's `--map`,
`--save-map`, `--write-map`; `make map`; `MAP=file` on debug, run and
host. The world is built at the size the file declares (the generator
makes 128, the prior art's maps are 256): swarm edges, the brains' clamps and
the client's camera take it from the map, and `Welcome` is sent in
`Chunk`s when the map does not fit a datagram (backlog item 6, done).
`scripts/priorart2map.py` and `make convert` turn a prior-art map
(its binary format, v1) into a treadcore map; five converted community maps
ship in `maps/prior-art/` (provenance in `vendor/THIRD-PARTY.md`). Proof:
`generate(seed).to_text()` reads back equal for two seeds and writes
back identical; a world built from a four-start file seats four of
sixteen; sixteen bots play a 256 x 256 prior-art map through `make debug`.


**Reference.**
 SPEC MAP-03 to MAP-06, MAP-09, MAP-10: what a map
defines, what a loader repairs, that a running map can be saved.

**Change.** `treadcore-sim::map` gains `to_file` / `from_file`: a
header line, then 128 rows of 128 characters (one per terrain), then
`start x y`, `depot x y`, `pillbox x y owner` lines. Human-editable in
any text editor, which is the editor for the first release (the prior art's
BMAPEdit came later; its ASCII import/export came with it). `Scenario`
takes `map: MapSource::Seed(u64) | File(path)`.

**Why here.** Pillboxes (step 3) need placing by hand to be balanced;
a seed cannot do that.

**Proof.** `generate(seed).to_file().from_file() == generate(seed)`.

## 6. Teams and alliances

**Done 12 October 2026 (journal 0007).** `Player.team`; a scenario deals
seats into teams (`with_teams(n)`, `--teams N`, `TEAMS=n make debug`) or
leaves everyone alone, the prior art's start. Sensors are the team's: `visible_to`
and `spotted_by` take any ally's tank or drone, so one tank's recon is
another's gun. Friendly fire is off within a team for shells, strikes,
mines and splash (`FriendlyFireSpared` in the trace); one's own shells
still hurt. Interceptors and pillboxes do not engage an ally's drones or
tanks; a strike at an ally is refused; allies share depots and pillbox
shells and see each other's mines; an ally's pillbox cannot be captured.
`Command::Chat { text, to: All | Team }`. Scores stay per player; the
match ends when one team stands, and the winning team's top scorer is
the winner (`MatchOver { winner, team }`). The prior art's alliance protocol in
play: `Command::Alliance(Request | Accept | Decline | Leave)` with
thirty-second requests, any member accepting, leaving at will, all of it
events (ALL-01 to ALL-06, ALL-08 to ALL-10). The RuleBot and Fortifier
read allies as friends. Wire version 7. Proof: `a_team_shares_recon_
and_spares_its_own` and `alliances_form_and_break_and_teams_end_the_
match`; a four-team sixteen-bot trace.

**Reference.**
 SPEC ALL-01 to ALL-10 (request, accept, any member
admits, leave at will, what leaves with the leaver), OWN-03.

**Change.** `Player.team: u8`. `visible_to` unions a team's sensors;
`event_for` extends to teammates; interceptors do not engage team
drones; shells and strikes do no damage within a team (friendly fire
off, so an alliance can stampede together, CONCEPT.md section 9);
scores are per team. `Command::Chat` gains `to: Team | All`. Four
teams of four is the first scenario; two-versus-two the smallest.

**Must not change.** The sixteen-seat limit; the election; the wire
format beyond one field.

**Proof.** Two bots on a team share recon: a shell fired by one at a
target only the other's drone sees is `spotted`; a shell landing on a
teammate does no damage and says so in the trace.

## 6b. Betrayal and recruitment: the proposal protocol

**Done 12 October 2026 (journal 0008).** `Command::Propose { to, terms }`
with `Terms::Join` (I join you: an alliance request, or a defection),
`Recruit` (you join me), `Swap` (we exchange teams) and `Merge` (my team
joins yours); `Command::Answer { proposal, accept }` by the one asked or
any member of their team; a `Proposal` stands `PROPOSAL_TICKS` (30 s).
**The three-way rule** (`MIN_SIDES` 3): the world refuses, with the
reason, any proposal whose outcome would leave fewer than three sides
with a living tank or give one side more than half the living tanks,
once when proposed and again when accepted. Every resolution is a
public event (`ProposalResolved`: accepted, declined, refused with why,
lapsed, or overtaken by a change of sides). The prior art's alliance commands are
sugar over this (a request is a Join proposal and keeps the prior art's "leave
first"). Wire version 8. Proof: `proposals_change_sides_under_the_
three_way_rule`, a recorded match: two sides of eight refuse every
proposal; of three sides a defection is accepted and the newcomer's
recon spots for its new side; recruiting to more than half is refused;
a swap is allowed at any size; the match replays.

**Reference.**
 SPEC ALL-02 to ALL-05 are the request and answer; the
swap, the recruit and the three-way rule are treadcore's additions.

**Change.** `treadcore-sim`: `Command::Propose { to: PlayerId, terms:
Terms }`, `Command::Answer { proposal, accept: bool }`; `Terms::Join {
team }` (recruit, or defect to), `Terms::Swap` (two tanks exchange
teams). A proposal is an entity with a deadline (`PROPOSAL_TICKS`);
the world resolves it when answered or expired and emits
`Event::ProposalResolved` to everyone, so a betrayal is public the
moment it happens. **The three-way rule**: the world refuses, with the
reason, any proposal whose outcome would leave fewer than three teams
with a living tank, or give one team more than half the living tanks.

**Why.** Sixteen players with alliances and no way to change them is a
fixed bracket; with unconstrained changes it collapses to one side.
The rule keeps three sides in play, which is the smallest number in
which an alliance is a choice.

**Must not change.** Validation stays in `apply`; a Brain cannot
propose anything a human cannot.

**Proof.** A scripted replay: two teams of eight, a proposal that would
merge them is refused; one that moves one tank to a third team is
accepted and the trace shows the team change and the shared recon.

## 6c. Material, walls, mines, roads: what a Pillboy builds

**Done 12 October 2026 (journal 0005).** `Terrain::Wall` (impassable,
blocks a flat shot and a pillbox's line, knocked to rubble by a
splash); `Mine` entities (hidden from all but the layer until a tank or
squad comes within 1.5 tiles; set off by a tank or squad on the tile,
a shell's splash, or an orthogonal neighbour; 50 to the tank on it, 20
within 1.5, a squad in it lost, the tile cratered; drones untouched);
`Build { Wall | Road | Mine | Pillbox }` and `Harvest { at }` (a forest
tile to grass, +4 material, 4 s); roads become bridges over water,
built from the bank; material capped at 8. Pillboxes hold 8 shells for
their owner's tank and regrow one every 10 s. `Event::TerrainChanged`
keeps the client's map current. The Observation carries the mines a
player knows and the eight nearest forest tiles. A tank's route avoids
the hostile mines its driver knows about. `Fortifier`, the minimal
defender (harvest, a gateless ring of mines at radius 3 round the home
depot, a ring of walls at 4 with a gate toward the centre); `--defenders
N` on the server, `DEFENDERS=n` on make. Wire version 5. Proof:
`fortification_pays`, twenty seeds, a scripted attacker against a
fortified and a bare depot; numbers in journal 0005.

**Reference.**
 SPEC section 6 (the mode table: costs in trees), section
7 (MIN-01 to MIN-13: who sees a mine, what sets it off, the crater),
TER-01 to TER-03 (walls take hits to rubble; forest farmed and
regrowing), BAS-01 (stocks held by a structure). Not taken: boats, deep
sea, swamp, the edge minefield (MAP-08).

**Change.** `treadcore-sim`: `Terrain::Wall` (impassable, blocks direct
fire, cratered by indirect) and `Terrain::Mine` (hidden to others;
destroys a tank or squad that enters it; cleared by a shell's splash;
ignored by drones, so a moat is no defence against the air).
`PillboyTask::Build { what: Wall | Road | Mine | Pillbox, at }` costs
`material`; `PillboyTask::Harvest { at }` turns a forest tile to grass
and adds material. Pillboxes carry shells: a tank inside `DEPOT_RADIUS`
of its own pillbox resupplies shells (not drones) from it, so ammo
comes from pillboxes and depots, as the brief says.

**Must not change.** Drone mechanics; the depot rules; the step order.

**Proof.** A bot that harvests, walls and mines a square round its
depot survives a scripted attack longer than one that does not, over
twenty seeds, by a margin the trace shows.

## 6d. The defender brain

**Reference.** SPEC section 16 for what a brain is told (BRN-08 to
BRN-12); the "advantage" option becomes a scenario flag.

**Change.** `treadcore-brain`:
 `Defender`, the simplest doctrine that
plays the brief's defensive game: harvest the nearest forest, wall a
maze round the home depot and a pillbox it builds at the maze's mouth,
mine the approaches, lay a road from depot to pillbox so the squad can
reach it to repair, keep two interceptors up, never leave. A scenario
option `attackers: u8` seats that many `RuleBot`s (the attackers, a
quarter by default) and `Defender`s in the rest.

**Proof.** Sixteen bots, four attacking: the match lasts to the time
limit more often than not, and most kills are pillbox kills.

**Done, 13 October 2026 (journal 0010).** `Defender` replaces the
Fortifier: the doctrine above with a recon on the approach (moved, by
`Retask`, to the depot of whoever shot it last: counter-battery by
deduction), a sidestep when hit, the squad in the order moat, mouth
pillbox, road, approach mines, walls (the moat first: built later, the
6c proof turned the other way, the attacker walking through the gap
onto the depot, where a contested depot repairs nobody), and the
spoke's neutral pillbox captured first of all. The "maze" is the 6c
ring with a gate. A Defender proposes to recruit its nearest neighbour
once a minute and accepts any proposal from a player who has never
hurt it: the first bots to propose and answer. The attacker got its
half of the step: a recon kept ten tiles short of what it watches, a
squad recalled under fire, and a storm (shell the guns, then drive onto
the depot) when a defender has been in view for twice its patience.
The `attackers` option is `--defenders N` from 6c, the other way up.
BRN-12 is `Scenario.advantage`, `--advantage`.

The proof, `defenders_hold`, ten seeds of five minutes: every match to
the time limit; attackers alive 2 of 40, defenders 84 of 120; the
defenders keep 101 of their 120 depots. Its second clause failed and
the test says so: the kills are 65 by shell, 7 by mine, 1 by pillbox,
1 by strike. The gun out-ranges the pillbox 60 tiles to 8, both sides
have eyes, and an attacker that reaches a prepared depot is one that
has already lost the duel; the four attackers are dead by the third
minute. Pillbox kills need a storm, and the storm needs the attacker
alive at twice its patience. Eight seeds of the same match through the
report: 7.5 kills a match (6.1 shell, 0.9 mine, 0.4 strike, 0.1
pillbox), 8.5 of 16 alive at five minutes and the same at ten.
`fortification_pays` widened to 144 000 against 134 206 ticks.


## 7. Across the Internet

**Done as far as one machine proves it, 13 October 2026 (journal
0012).** `treadcore-net/src/introducer.rs`: the introducer (`Introducer`,
`treadcore-server --introduce`, `make introduce`), a peer's
`Registration` with it, the LAN-address guess, and `identity_from_file`
(`--identity FILE`): a peer id that survives a restart. `Register` and
`Introduce` on the wire (version 11, with `Join { session }`). The
lobby registers in Discover (`--introducer HOST:PORT`; `--no-beacon`
keeps it off the LAN), greets every introduced peer at its public and
its local address every 250 ms and with every probe until the peer
answers, and takes the answering address: the hole punch in its plain
form. The election and the relay are untouched: a pair that never gets
through is an unmeasured link, routed around. Rejoin (backlog item 10,
SPEC GAM-12) came with it, because it is the introducer's business: a
host keeps registering with `hosting` set; a returning peer with the
same identity is told its host, pings it until it answers, joins it
with no election, and the host gives the seat back with its sequence
numbers started over, telling a restart from a repeated Join by the
session number. Proof so far: `introducer_brings_two_silent_peers_
together` (no beacon, no seed, two lobbies and an introducer in one
thread); `seat_comes_back_after_a_restart` (a client dropped mid-match,
a new one with the same peer id welcomed to the same seat at a later
tick, its first order applied); `INTRODUCER=1 make lobby-test` (four
peers, no broadcast, no seed address, met through the introducer
alone) and `REJOIN=1 make lobby-test` (a seat that is not hosting
killed outright at twenty seconds, started again, back in its seat at
the tick the match has reached, sees the match end), plain and under
`LAG=200`. Two findings on the way: the first run killed the host (the
election had picked peer 4) and the match died with it, which is step
8's problem and not this one's, so the script now kills a seat that is
not hosting; and the returning seat's predictor logged one mismatch at
zero tiles two ticks after its Welcome, because its copy was the
scenario's start and the host's tank had turned to fire in the
meantime, so the first snapshot after a Welcome is now a baseline, not
a check. A third, older than this step: a shell cratering the road two
ticks ahead of a client was a mismatch of a fifth of a tile, once in
some 2 400 checks, the kind journal 0011 said would show first here;
the predictor now re-drives from the last snapshot's tank when a
terrain event lands under ticks it has predicted. **Not proven:** two
peers behind two home routers. That is
the roadmap's proof clause and it needs two machines on two networks;
the code for it is in, and the first such run is the next journal's.

**Change.** `treadcore-net`: a rendezvous step before the lobby. Not a
server that hosts games; a tiny UDP service that tells peers each
other's public address and port (the "introducer"). The lobby's
`Hello` already carries addresses; the introducer fills in the ones a
NAT hides. Hole punching follows the usual shape: both sides send to
each other's public address on a timer until a packet gets through.
The election and the relay route need no change: a pair that cannot
punch through is routed around.

**Must not change.** The lobby on a LAN works with no introducer, as
now.

**Proof.** Two peers behind two home routers reach Playing.

## 8. The headless server as a service

**Done as far as one machine proves it, 13 October 2026 (journal
0022).** `treadcore-server --listen DIR` (`make listen`;
`treadcore-server/src/listen.rs`): a dedicated host that runs the
lobby with `server` set (wire version 12: on `Beacon`, `Hello` and its
known list, `Register`, `Introduced`), takes no seat, never hosts
alone, starts probing when the first seat does, and goes back to the
lobby after every match or lost election. The preference is
`election::elect_among`: a server is weighed over the seats plus
itself with `SERVER_PREFERENCE` (10 ms) off its worst path, every seat
over the seats alone; a server that loses is in no tree, so never a
relay. Each hosted match writes `match-NNNN.trace.jsonl` and
`match-NNNN.replay.json` to DIR, numbered on from what DIR holds; a
match no seat joins within 15 s is given up. The status page is plain
HTTP on `--http` (`/`, `/status.json`, and the hosted matches' files).
With `--introducer` it is listed under its lobby name as a server.
Proof: `LISTEN=near make lobby-test` (the four seats with 30 ms of lag
on every receive, the server none: it is elected, hosts all four as
seats, one world hash at tick 1000 in five logs, no prediction
mismatches, its replay plays back with every checkpoint matching, its
status page says `hosting` mid-match) and `LISTEN=far make lobby-test`
(the server with 100 ms: not elected, hosts nothing, the four elect
among themselves and pass as without it, the status page counts the
lost election); `INTRODUCER=1 LISTEN=near`, met through the introducer
alone. Tests: `server_with_best_links_hosts`,
`far_server_is_ignored_and_never_relays`, `of_two_servers_one_hosts`,
`server_in_the_lobby_hosts_and_takes_no_seat`. **Not proven:** a
server on a public address with players behind home routers (step 7's
two-router run, with a server in it). **Not done:** one server for
several lobbies at once (it serves one `--game`); a player who arrives
while the server hosts is refused until the next match, as with any
host (one lobby name is one match). **Done (journal 0032):** the
window's rematch, Enter on a networked match-over screen back to the
lobby on the same port, proven by `make rematch-test`. **Done
(journal 0033):** a late arrival takes a bot's seat by order, proven by
`LATE=1 make lobby-test` and `LISTEN=near LATE=1`, the server's replay
matching. Left: one server for several lobbies.

**Change.** `treadcore-server --listen`: hosts match after match,
accepts joins from any lobby that elected it (a peer can be told to
prefer a listed address as host), writes traces and replays per match
to a directory, serves a small HTTP status page. This is the "someone
runs one for the group" option without making it the only option.
The introducer (step 7) is the natural place for the listing: a
`--listen` server registers as hosting under a lobby name, and the
"prefer a listed address" is a weight in the election for the peer
that registered so.

**Proof.** `make lobby-test` against a running `--listen` server
elects it when it has the best links and ignores it when it does not.

## 9. Sound, art, and a release profile

Last, on purpose. The overlay is the look; sound is the one thing the
HUD cannot convey (a shell launch you cannot see). `dist` profile with
thin LTO for binaries; `cargo dist` or a plain tarball per platform.

**Done, 13 October 2026 (journal 0023).** The art came first, as the
playtest asked for it (journals 0014 to 0017: the frame, the bezel,
cubes); the `dist` profile and the packages per platform came with the
release pipeline (journals 0018 to 0021: .deb, .rpm, MSI, zips, the
Nix flake). Sound is `treadcore-client/src/sound.rs`: eighteen cues,
synthesised at start-up from swept sines, filtered noise and
inharmonic partials (no sound files in the tree), each made in five
pan positions as WAV in memory and played through Bevy's audio (the
`bevy_audio` and `wav` features; ALSA was already a dependency). A
sound is placed at its event, heard from the view's centre: half
strength at 25 tiles, silent past 120, panned fully at 50 tiles off
centre; at most 20 at once, the loudest kept, one cue at most every
60 ms. A hostile shell whistles at its impact point the first time a
snapshot shows it: the launch a player cannot see. `M` mutes, `--mute`
starts muted, `make sounds` writes every cue to `build/sounds/` to
listen to. Proof: tests (every cue sounds, does not clip, does not
click at the end; the WAV is well formed; distance and pan), a level
check of every cue (blasts loudest, the interface at half), and a
60-second solo match that played the swarm, the player's hits, a gun,
its boom, a wreck and a refusal with no audio error. **Not proven:**
that it sounds right. That takes an ear, and this machine has none.

**Then FM and the whole map (journal 0024).** The weapons and metal
are two-operator FM after Paul's notes (a gunshot, a cannon blast, a
direct hit as struck armour, a missile; then the machine noises: the
breech's doo-cha-klank, the loader, a clock, the out-of-ammunition
beep, a beating alarm, the tracks' squeal), 32 cues in three takes
each, synthesised at 88.2 kHz under Carson's bound and band-limited
to 44.1 kHz.
Every sound has a place: full strength anywhere on screen and panned
across it, fading past the screen's edge to a floor of 6 % and never
to silence, low-passed past fifteen tiles out; a sound whose place
the fog hides plays as background from nowhere.

## 10. The economy under drones

Planned 13 October 2026, from Paul's notes and a talk he passed on
(Peter Zeihan, October 2026, on the Houthi strike on Riyadh's airport
and Ukraine's on Russian data centres). What the talk says, as a game
designer reads it:

1. **Infrastructure is the target now.** Not the army: the airport,
   the refinery, the data centre. They are large, they cannot move,
   and a hole in the roof stops them (a data centre without cooling is
   a room of melting chips).
2. **The drones that do it are cheap and dumb.** No jam-proof
   guidance, no exotic parts: long range, one way, a fixed point. The
   clever drones (memory navigation through jamming) are a later
   generation and are coming anyway.
3. **Electronic warfare protects a point, not an economy.** It
   scrambles the link and the drone crashes, hovers or flies off; but
   it scrambles everything, its owner's too (an airport under EW cannot
   land planes), so it can ring a base and never a country.
4. **So everything within range is either defended or written off.**
   The economy is too big to protect; the defender chooses what to
   lose.
5. **Systems depend on systems.** Hit the power and the data centre
   goes dark without being touched.

treadcore already has the half of this that is about tanks: the
bubble, the strike, the recon, the gun. What it has not got is
anything worth defending that is not a tank, a depot or a pillbox. This
step gives the map an economy, a cheap drone that crosses the map to
hit it, the jammer that protects a point at a price, and the window a
commander's picture of all of it. The game stays a tank fighter: the
tank is still the one thing the player drives, the squad builds, and a
structure is a reason to fight over ground, not a city builder.

### The backlog of step 10

Each is a proposal in waiting in the roadmap's form, in the order they
depend on each other.

10a. **Industrial ground and a second wall.** `Terrain::Pad` (`_`):
     foundations, passable as a road; the only ground a structure goes
     on, so the map decides where an economy can be. `Terrain::Concrete`
     (`H`): a hardened wall, two material, impassable, blocks fire; a
     splash cracks it to a plain wall instead of rubble, so it takes
     two. *Must not change:* the generated island (no pads unless asked
     for), so every proof so far reads the same. *Proof:* the map
     format round-trips both glyphs; a splash takes concrete to wall to
     rubble.

10b. **Structures.** A `Structure` entity on a pad: kind, owner or
     neutral, health (`STRUCTURE_HEALTH` 120, twice a pillbox: large,
     hard to miss, slow to kill), capture and repair as a pillbox's (a
     squad on a neutral or wrecked one for ten seconds; material to
     mend). Built by the squad on a pad for material. Five kinds:

     | kind | cost | what it gives its owner's side | needs power |
     | --- | --- | --- | --- |
     | power plant | 4 | powers every friendly structure within 18 tiles | no |
     | refinery | 4 | allied tanks burn half the fuel; within 3 tiles, a fuel and shell stop | yes |
     | data centre | 6 | fire control: a blind shell aimed inside 40 tiles of it scatters half as far; the air picture: hostile drones inside 30 tiles are seen | yes |
     | workshop | 3 | within 3 tiles a tank is repaired at twice a depot's rate and its bay restocked; within 8, friendly pillboxes and structures mend slowly | yes |
     | jammer | 5 | inside 7 tiles every linked drone, *its owner's too*, loses its link: recon and strikes turn for home, a strike's last dive excepted; a long-range drone's satellite fix drifts | yes |

     A structure held scores `SCORE_HOLD` every ten seconds: holding
     ground pays while it is held, not only when taken, which makes the
     time-limit winner the side with the economy. *Must not change:*
     the pillbox, the depot, the bubble's rules. *Proof:* a test per
     row of the table; power cut, the benefit stops.

10c. **The long-range drone.** `DroneKind::Longshot`: launched at a
     point anywhere on the map, slow (3 tiles/s), a battery for 150 s,
     autonomous from launch (no link, no recall, no retask: the
     pre-programmed one-way drone of the talk). It dives at the point:
     heavy damage to structures (80), light to tanks (25), because a
     fixed point is where a structure is and a tank is not. Two in the
     bay, restocked last at a depot, first at a workshop. Interceptors
     on station and pillboxes engage it as a strike, so a ring of
     pillboxes and a parked tank defend a refinery; nothing defends all
     of them. *Must not change:* the strike, recon and interceptor.
     *Proof:* a longshot crosses a 256-tile map and wrecks an
     undefended refinery; one into a tank's bubble is intercepted; one
     through a jammer lands off its point.

10d. **The commander's picture.** The window: a minimap in the right
     column (PLAYABILITY item 11): fog, depots, pillboxes, structures,
     tanks seen, the player's own; a click moves the view there.
     Alerts: a structure of my side hit, wrecked or taken goes on the
     newswire in red with where, with the alarm cue, and blinks on the
     minimap; `J` jumps the view to the newest. The ledger: my side's
     structures as lamps with health, powered or dark. Orders: `0` arms
     the works mode (a click on a pad builds the selected structure, on
     a structure repairs or captures it), `B` cycles the kind,
     Shift-click in wall mode pours concrete, `X` sends a longshot at
     the cursor. *Proof:* the window driven by xdotool, frames read.

10e. **Bots that know an economy.** The RuleBot sends its squad to a
     neutral or wrecked structure in reach as it does to a pillbox, and
     a longshot at the nearest hostile structure every half minute; the
     Defender repairs its own. *Must not change:* a bot-only match on
     the generated island: no structures there, so no new behaviour,
     and the balance proofs read as they did. *Proof:* sixteen bots on
     an industrial map, through the match report: structures taken,
     longshots launched, intercepted and landed, structures wrecked.

10f. **Industrial maps.** `structure x y kind [seat|neutral]` lines
     in the map format; `Map::industrialise`, which lays a pad zone
     between each start and the centre with a neutral power plant and
     refinery on it and a data centre at the middle, behind
     `--industry` and `INDUSTRY=1`; `maps/industry.map`.

10g. **A world map.** `scripts/worldmap.py` draws a map from coarse
     coastlines in longitude and latitude: `maps/world/atlantic.map`
     (the Americas, the Atlantic, Europe, North Africa and Arabia,
     512 x 256) with the Pan-American Highway as a road from Prudhoe
     Bay to Ushuaia, broken at the Darién Gap as the real one is, and
     Europe's trunk roads from Lisbon to Moscow; the Channel, the
     Bosphorus and the Strait of Gibraltar crossed by bridges, the
     Atlantic by nothing. Sixteen starts in sixteen cities, industry
     where industry is (the Gulf Coast, the Ruhr, the North Sea,
     Arabia's oilfields). `maps/world/europe.map` is the same script at
     Europe's scale. *Proof:* the loader reads them; every start
     reaches the others on its continent; sixteen bots play one.

10i. **A bot that builds a jammer** (journal 0027's next step 2).
     Once a structure of its side has been hit, the RuleBot builds a
     jammer on a free pad within half a jammer's reach of a structure
     of its own that no jammer of its side covers, powered, felling
     forest for the material; it launches nothing inside its side's
     jammer (the bubble counts: it flies that far out) and drives out
     of one when nothing is in view. `Observation::pads`, the free pad
     tiles near the tank, because a Brain has no map. *Must not
     change:* the generated island (no pads, so `pads` is empty and
     the rule never fires). *Proof:* `a_rulebot_jams_after_a_hit`, and
     the Atlantic trace's jams, own against hostile.

10j. **Bots that run an economy** (journal 0028's next step 2). The
     squad's works, in order: a power plant beside a structure of the
     bot's that stands dark with no plant of its side in reach; the
     jammer of 10i; a workshop where none of the side covers the bot's
     structures. Scrap felled for any of them, and for mending a wreck.
     A working workshop of the side is a resupply stop while armour or
     the bay is what is short (it has no shells). *Must not change:*
     the generated island. *Proof:* `a_rulebot_powers_its_zone_then_
     builds_a_workshop`, and the traces' structures built by kind.

10k. **The works trip** (journal 0029's next step 2). A RuleBot with
     the scrap for what a structure of its own within `WORKS_TRIP` (45
     tiles) wants drives to it and holds there while the squad works;
     a structure with no free pad beside it for that kind is
     remembered and not visited again. *Proof:* `a_rulebot_drives_to_
     its_zone_to_build`, and the traces.

10h. **Not now.** Memory-guided drones that ignore jamming (the talk's
     next generation): the longshot's drift is already the first half;
     a second kind waits for a playtest of the first. Airports and
     ports as structures. A strategic layer above the match.

### Status

Built 13 October 2026, the same evening (journal 0027): 10a to 10g
as written, with the numbers in `rules.rs` beside their reasons. Wire
version 13. Proof so far: seven sim tests, one a row of the table each
and one per mechanism (`concrete_takes_two_splashes`,
`structures_need_power_and_serve_their_side`,
`squad_builds_takes_and_mends_structures`,
`longshots_cross_the_map_and_bubbles_stop_them`,
`a_jammer_jams_everyone_and_drifts_a_longshot`,
`a_data_centre_computes_fire_and_sees_the_air`,
`an_industrial_map_round_trips`), the world maps' connectivity test,
an alert test in the window, and the brain's balance proofs reading
the same to the digit as before the step (no structures on the
generated island, so no new behaviour there). Sixteen bots, five
minutes, through the match report: on the industrial island 13
structures taken, 27 longshots, 21 landed, 9 structures wrecked; on
the Atlantic map 36 taken, 78 longshots, 43 landed, 20 wrecked, and 5
of the 12 kills by pillboxes at the chokepoints. The window was driven
with xdotool on both. **Not proven:** that any of it is fun; the
numbers are first guesses with reasons, and a playtest moves them.

- [x] 10a industrial ground and concrete
- [x] 10b structures
- [x] 10c the longshot
- [x] 10d the commander's picture. Finished 14 October 2026 (journal
  0028): the first window run with a structure of the player's under
  longshots showed no alert, because the wire shows its newest line and
  a longshot's line overwrote the alert five ticks later, and no
  ledger, which had not been built. The alert now holds the wire five
  seconds in red; the WORKS row is the ledger; over eight seats the
  right column folds the tank lamps away and sets the roster smaller,
  where at sixteen it had pushed the depots off the bottom. Seen in
  the window (`guide-alert.png`, `make shoot`).
- [x] 10e bots
- [x] 10f industrial maps
- [x] 10g the world map
- [x] 10k the works trip (14 October 2026, journal 0030): Atlantic,
  sixteen bots, five minutes, 12 structures built (6 before: 1 power,
  4 jammers, 7 workshops). The industrial island still none: probed,
  the bots get there and the build is ordered, but a structure takes
  twenty seconds and on 128 tiles a bot is under fire long before
  that, and the squad comes home. The island is too hot to build on.
- [x] 10j bots that run an economy (14 October 2026, journal 0029):
  Atlantic, sixteen bots, five minutes: 2 power plants, 2 jammers, 2
  workshops built; Europe 1 workshop; the industrial island none (its
  numbers read as before: the bots are seldom settled at their zone
  with scrap in reach). The balance proofs read the same to the digit.
- [x] 10i a bot that builds a jammer (14 October 2026, journal 0028):
  the first trace jammed 610 of the builder's side's drones to 21 of
  the enemy's, because the bot launched into its own jammer; with the
  rule against it, 29 to 71, and 14 hostile longshots pushed off their
  point. Atlantic, sixteen bots, five minutes: 5 jammers built.

## Sequencing

```
0 prove ── 0b sixteen ─┬─ 1 replay ── 2 hash ─┬─ 4 prediction
                       │                      └─ 7 internet ── 8 service
                       └─ 3 pillboys ── 6c material ── 5 maps ── 6 teams ── 6b proposals ── 6d defender
                                                                                              └─ 9 sound/art/dist

Done through 6b on 12 October 2026 (journals 0002 to 0008); the backlog
worked through, 6d and 4 done on 13 October (journals 0009 to 0011);
7 built and proven on one machine the same day (journal 0012); 8
likewise (journal 0022); 9 the same evening (journal 0023); 10,
the economy under drones, that night (journal 0027); 10i and
PLAYABILITY item 9 the next day (journal 0028). Next: 7's
and 8's proof across two routers when a second machine is on the
table, and a listening session for the sound.

```

Two tracks after step 0, independent until step 6, which wants both
the hash (for team-shared prediction) and the maps (for a balanced
layout of sixteen). The defender brain comes last on its track because
it needs everything it defends with. Each step is one journal entry's
work, or two.

