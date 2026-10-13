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
- [ ] The window. `treadcore-warm` panics in this VM: no Vulkan driver
  is installed (`BUILD.md`, "The window in this VM"). One package,
  `mesa-vulkan-swrast`, needs root. Until it is in, the window is
  unproven here.

Nothing below should start until step 0's open problems are closed, or
every later step is built on a guess.

## The reference specification

`SPEC-WINBOLO-1.14.md` is the clean-room functional specification of
WinBolo/LinBolo 1.14, received 12 October 2026. Every step below that
touches the Bolo-derived layer (3, 5, 6, 6b, 6c, 6d) cites its
requirement IDs, and a proposal that changes one of those mechanics
says which IDs it keeps, adapts or drops. Its preface says which parts
treadcore adopts; its networking (section 12) is reference only, and
nothing of Macintosh Bolo's AppleTalk ring is in it or in treadcore.

## Backlog: what the first traces found

Carried from journal 0002. Each is a proposal in waiting
(`GOVERNANCE.md` section 5): a decision, with a trace to argue from.
Where an item has a counterpart in the reference specification, the ID
is given.

1. **Interceptors fight interceptors.** When two bubbles overlap
   (tanks inside 2 x bubble + margin of each other) both rings
   annihilate each other in two seconds, and the bots refill them from
   the bay until it is empty; the first trace shows 22 intercepts in 20
   seconds and three tanks dead to strikes through the stripped rings.
   Decision wanted: should an interceptor on station engage a hostile
   *interceptor* at all, or only strikes and recon? The real-world
   analogue says engage anything; the game may want otherwise.
2. **Tank speed.** 4 tiles/s is 360 km/h; the map is crossed in
   fifteen seconds and a match of four bots ends in under two minutes.
   `rules.rs` already calls it a game speed. Try 2.5 and read the
   trace.
3. **Retasking a drone.** The only way to move a recon is `Recall`,
   which brings every drone home. A `Command::Retask { drone, task }`
   (recon orbit centre; a single drone's recall) is small, and both the
   bot and a human need it. Interceptors stay the world's to task.
4. **Rejected orders repeat.** A bot whose `Move` is refused (a point in
   water) re-issues it four times a second. Harmless; noisy in the
   trace. The bot should remember a refusal for a few seconds.
5. **The HUD's rtt is not an rtt.** The client shows snapshot tick
   minus last-heartbeat tick, which is mostly heartbeat age. Use the
   host's `rtt_ticks`, sent back in `Heartbeat`.
6. **`Welcome` is one 16.5 KiB datagram.** Fine on a LAN, fragile on
   the Internet; split it like snapshots before step 7.
7. **The strike is the kill.** In every trace so far the tanks die to
   strike drones (35 x 3 in the bay) after the rings are stripped, not
   to shells. If the premise is that the gun is the weapon, either
   strikes need a longer approach (battery) or interceptors need to
   survive their own melee (item 1).
8. **A match report.** `make debug` should end with the numbers a
   balance reader wants (shells by kind and spotted, intercept rate,
   kills by cause, time to first kill) instead of a reader counting
   JSON lines.
9. **Depot armour as the gate.** The spec's bases cannot be driven
   onto by an enemy while they hold armour; shooting them opens them
   (SPEC BAS-07 to BAS-09). treadcore's depots are taken by standing
   on them for ten seconds. Decide whether the shell should be the key.
10. **Rejoin.** A player who drops and returns within a window keeps
    their seat and what was theirs (SPEC GAM-12, OWN-04). Needs a
    peer identity that survives a process restart.
11. **Game types.** Open, Tournament and Strict Tournament (SPEC
    section 11) as `Scenario` presets once pillboxes and material exist.
12. **Pillbox view.** Jump the camera between owned pillboxes (SPEC
    UI-10 to UI-13); the client's one Bolo feature worth having early.
13. **Forest concealment as Bolo had it.** Hidden to everything beyond
    3 tiles when inside a 2 x 2 block of forest (SPEC VIS-02), against
    treadcore's halved vision radius. A proposal with a trace.



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

**Change.** `World::hash() -> u64` over the state that matters (tick,
rng, every entity's fields, map cells). The host sends it in
`Heartbeat`; a client that also runs the world (step 4) compares.

**Why.** Before prediction exists, the hash makes replay testable
(step 1) and gives `make lobby-test` a real assertion: every peer
observed the same hash sequence.

**Proof.** Four lobby-test peers log identical hashes at tick 1 000.

## 3. Pillboys: the control model, then the unit

**Decision first.** Written into DESIGN.md on 12 October 2026 ("Pillboys
and pillboxes: the decision"), checked against Bolo's builder and
refuelling bases (`LAB-REPORT-PILLBOYS.md`): Pillboys are an *order the
tank gives*, not a unit the player drives. The tank is the one thing
the player drives; that is the premise. `Command::Deploy { task:
PillboyTask }` where the task is `Capture { pillbox }`, `Repair {
target }`, `Build { what, at }`, `Return`. The squad is a state on the
tank (strength 0..N, deployed/aboard, position when deployed, task,
progress, material) that the HUD shows beside the drones. Depots are
not a Pillboy's business: they are refuelling bases, taken and used by
driving onto them, as now.


**Reference.** SPEC-WINBOLO-1.14.md section 6 (BLD-01 to BLD-03,
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

**Reference.** SPEC MAP-03 to MAP-06, MAP-09, MAP-10: what a map
defines, what a loader repairs, that a running map can be saved.

**Change.** `treadcore-sim::map` gains `to_file` / `from_file`: a
header line, then 128 rows of 128 characters (one per terrain), then
`start x y`, `depot x y`, `pillbox x y owner` lines. Human-editable in
any text editor, which is the editor for the first release (Bolo's
BMAPEdit came later; its ASCII import/export came with it). `Scenario`
takes `map: MapSource::Seed(u64) | File(path)`.

**Why here.** Pillboxes (step 3) need placing by hand to be balanced;
a seed cannot do that.

**Proof.** `generate(seed).to_file().from_file() == generate(seed)`.

## 6. Teams and alliances

**Reference.** SPEC ALL-01 to ALL-10 (request, accept, any member
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

**Reference.** SPEC ALL-02 to ALL-05 are the request and answer; the
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

**Reference.** SPEC section 6 (the mode table: costs in trees), section
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


## 7. Across the Internet

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

**Change.** `treadcore-server --listen`: hosts match after match,
accepts joins from any lobby that elected it (a peer can be told to
prefer a listed address as host), writes traces and replays per match
to a directory, serves a small HTTP status page. This is the "someone
runs one for the group" option without making it the only option.

**Proof.** `make lobby-test` against a running `--listen` server
elects it when it has the best links and ignores it when it does not.

## 9. Sound, art, and a release profile

Last, on purpose. The overlay is the look; sound is the one thing the
HUD cannot convey (a shell launch you cannot see). `dist` profile with
thin LTO for binaries; `cargo dist` or a plain tarball per platform.

## Sequencing

```
0 prove ── 0b sixteen ─┬─ 1 replay ── 2 hash ─┬─ 4 prediction
                       │                      └─ 7 internet ── 8 service
                       └─ 3 pillboys ── 6c material ── 5 maps ── 6 teams ── 6b proposals ── 6d defender
                                                                                              └─ 9 sound/art/dist
```

Two tracks after step 0, independent until step 6, which wants both
the hash (for team-shared prediction) and the maps (for a balanced
layout of sixteen). The defender brain comes last on its track because
it needs everything it defends with. Each step is one journal entry's
work, or two.

