# The next architectural steps

Where the architecture goes from the opening day, in the order the
steps depend on each other. Each step says what it changes in which
crate, what it must not change, and how it is proven. The constraint
that shapes all of it: the simulation stays headless and deterministic,
and every layer above it keeps driving it through `step` and `observe`.

## 0. Prove what exists (before any of the below)

- `make warm`, measured: wall time, peak memory, swap. Journal 0002.
- `make test`: the compiler's verdict on proto, net, brain, server.
- `cargo build -p treadcore-client`; `target/release/treadcore-warm`
  to see whether wgpu finds an adapter in this VM. If it does not, the
  fallback is Bevy's software path or developing the window on the
  Mac host; the headless crates do not care either way.
- `make debug` and read the trace for balance.
- `make lobby-test`, then two machines.

Nothing below should start until step 0's open problems are closed, or
every later step is built on a guess.

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

**Decision first.** Written into DESIGN.md before code: Pillboys are an
*order the tank gives*, not a unit the player drives. The tank is the
one thing the player drives; that is the premise. `Command::Deploy {
task: PillboyTask }` where the task is `Capture { pillbox }`, `Repair {
target }`, `Build { what, at }`, `Return`. The squad is a state on the
tank (strength 0..N, deployed/aboard, position when deployed, task,
progress) that the HUD shows beside the drones.

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

**Change.** `Player.team: u8`. `visible_to` unions a team's sensors;
`event_for` extends to teammates; interceptors do not engage team
drones; scores are per team. `Command::Chat` gains `to: Team | All`.
Two-versus-two is the first scenario.

**Must not change.** The four-seat limit; the election; the wire
format beyond one field.

**Proof.** Two bots on a team share recon: a shell fired by one at a
target only the other's drone sees is `spotted`.

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
0 prove ─┬─ 1 replay ── 2 hash ─┬─ 4 prediction
         │                      └─ 7 internet ── 8 service
         └─ 3 pillboys ── 5 maps ── 6 teams
                                           └─ 9 sound/art/dist
```

Two tracks after step 0, independent until step 6, which wants both
the hash (for team-shared prediction) and the maps (for a balanced
two-versus-two layout). Each step is one journal entry's work, or two.
