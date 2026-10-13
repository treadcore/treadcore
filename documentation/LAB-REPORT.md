# treadcore: lab report

A walk through the code as it stands on 2026-10-12: what each section
is, what it looks like, how it is used, and the Rust technique behind
it. Source: [github.com/treadcore/treadcore](https://github.com/treadcore/treadcore).
Site: [treadcore.org](https://treadcore.org).

## 1. Status, first

As of 13 October 2026 (the earlier tables are in the history; journal
0002 has the chronology, 0009 the backlog, 0011 prediction):

| crate | lines | compiled | tests |
| --- | --- | --- | --- |
| treadcore-sim | ~4700 | yes | 24 pass (determinism, prediction drives the tank as the host does, a team shares recon and spares its own, alliances form and break and teams end the match, proposals change sides under the three-way rule, hash follows state, a replay reproduces the world and a tampered one is caught, launch validation, shell flight, swarm vs bubble, a Move routed across the river, a pillbox kills a tank and routes bend round it, Pillboys capture a pillbox, the squad harvests, walls and mines, mines chain and shells clear them, retask moves a recon and recalls one drone, a forest block conceals a tank, game types set the start, sixteen seats, a world from a map file, map determinism, every start reaches the centre, a map survives the file round trip, bad map files are refused with a line) |
| treadcore-proto | ~330 | yes | 3 pass (round trip, snapshot split and reassembly, big packets chunk and reassemble) |
| treadcore-net | ~1570 | yes | 6 pass (4 election, 1 loopback session, 1 prediction under 200 ms of lag: the predicted tank moves 250 ms before the snapshot shows it, zero mismatches) |
| treadcore-brain | ~1150 | yes | 4 pass (sixteen bots fight to a result; `pillboys_pay`: over twenty seeds, bots that capture pillboxes outscore and outlive the same doctrine without the squad; `fortification_pays`: a fortified depot outlives a bare one against a scripted attack; `defenders_hold`: twelve Defenders against four attackers, ten seeds, every match to the time limit, the defenders outliving the attackers and keeping their depots) |


| treadcore-server | ~760 | yes; `make debug` and `make lobby-test` run (`LAG=200` for the lagged one) | none |
| treadcore-client | ~1320 | yes, against Bevy 0.17.3; three dead-code warnings | none |
| treadcore-warm | ~45 | yes; the window panics in this VM for want of a Vulkan driver (BUILD.md) | none |

The round of small fixes the morning predicted was three, and none of
them was a compile error: the compiler accepted every crate first
time. They were a UDP receive buffer too small for `Welcome` (found by
the loopback test), a straight-line `Move` that could not cross the
river (found by the first trace), and the Makefile's `SECONDS` colliding
with bash's own (found by reading the trace header). The rule bot was
rewritten after the first trace showed a twenty-second melee with no
indirect fire.


## 2. The shape: six crates, one direction

```
treadcore-client (Bevy)      treadcore-server (headless)
        \                        /
         treadcore-brain  treadcore-net
                   \        /
                treadcore-proto
                      |
                 treadcore-sim
```

Every arrow points down. The sim knows nothing of sockets, engines or
bots; the proto crate knows the sim's types and nothing else; net knows
packets and the sim's `World`; the two binaries know everything. That is
the whole architecture, and the Cargo workspace enforces it: a crate
cannot import something it does not list in its manifest, and a cycle
is a build error.

The practical payoff is in the server's `run_local` and the client's
solo mode: both drive the same `Host` with no socket. The server runs a
full match in a few seconds and writes the trace; the window plays the
same code at 20 ticks a second. There is no "simulation for the game"
and "simulation for the tests". There is one.

## 3. treadcore-sim: the world

### 3.1 Plain data, plain functions

```rust
pub struct World {
    pub tick: Tick,
    pub map: Map,
    pub players: Vec<Player>,
    pub tanks: Vec<Tank>,
    pub drones: Vec<Drone>,
    pub shells: Vec<Shell>,
    pub depots: Vec<Depot>,
    pub rng: Rng,
    ...
    #[serde(skip)]
    pub events: Vec<Event>,
}

impl World {
    pub fn step(&mut self, orders: &[Order]) { ... }
    pub fn observe(&self, p: PlayerId) -> Observation { ... }
}
```

Two methods matter. `step` advances one tick with the orders for that
tick. `observe` returns what one player is allowed to see. Everything
else is detail.

The technique is deliberate plainness. No entity-component system, no
trait objects, no `Rc<RefCell<>>`: entities are `Vec`s of structs with
an `id`, looked up by linear search. At four tanks, forty drones and
thirty shells a linear search is faster than a hash map and the code
reads as what it does. `#[serde(skip)]` on `events` says that events
are the output of a step, not part of the state; a snapshot of a world
does not carry them.

### 3.2 Determinism as a property, not a hope

Since journal 0003 the property has a number: `World::hash()` is FNV-1a
over the bincode bytes of the world, and a `Replay` (scenario, every
tick's orders, a hash every hundred ticks) plays back to the same
hashes or names the first tick where it does not. `make debug` records
one beside every trace; `make replay` checks it. That is the regression
test for every later change to `step`.


```rust
pub struct Rng(pub u64);          // xorshift64*
impl Rng {
    pub fn f32(&mut self) -> f32 { ... }
    pub fn disc(&mut self) -> V2 { ... }
}
```

The random source is the world's own, seeded from the scenario,
advanced only inside `step`. The host ticks orders sorted by player.
The test:

```rust
#[test]
fn deterministic() {
    let mut a = world(); let mut b = world();
    for i in 0..400 { let o = if i == 3 { orders.clone() } else { vec![] }; a.step(&o); b.step(&o); }
    assert_eq!(a.rng.0, b.rng.0);
    ...
}
```

Determinism is what makes host-authority honest (a client could, in
principle, run the same world and check), what makes a trace replayable,
and what makes a bot testable.

### 3.3 Commands are validated, events are the record

```rust
pub enum Command { Move { to: V2 }, FireIndirect { at: V2 }, Launch { kind, task }, SetBubble { radius }, ... }
pub struct Order { pub player: PlayerId, pub cmd: Command }
pub enum Event { Rejected { player, cmd, why: String }, ShellLaunched { .. }, Intercepted { .. }, TankDestroyed { .. }, ... }
```

`apply` is a long match over `Command`. Every arm checks before it acts
(`reloading`, `no shells`, `beyond maximum range`, `bay empty for that
kind`) and a failure is not a panic or an `Err` but an `Event::Rejected`
with the reason. The caller does not need a result type: the reason
goes to the player's HUD and the bot's log through the same channel as
everything else. Nothing a client sends can put the world in a bad
state; the worst it can do is be refused.

### 3.4 The step, and the borrow checker

`step_drones` is the densest function and shows the technique Rust
pushes you toward when many things move and interact in one tick:

```rust
// 3. Motion. Positions of everything, read before anything moves.
let drone_pos: Vec<(EntityId, V2)> = self.drones.iter().map(|d| (d.id, d.pos)).collect();
let tank_pos:  Vec<(EntityId, V2, bool)> = ...;
let mut lost = Vec::new(); let mut rams = Vec::new(); let mut strikes = Vec::new();
for d in self.drones.iter_mut() { ... push decisions into those vecs ... }
// 4. Resolve rams, strikes, recoveries, losses -- then remove.
```

Read a snapshot of positions, mutate each drone once with that snapshot,
collect the consequences into lists, then apply the consequences in a
second pass. The borrow checker will not let you hold `&mut` to one
drone while reading another, and the shape that satisfies it is also
the shape that gives order-independent results. The constraint is the
design.

### 3.5 The numbers live in one file

`rules.rs` is nothing but `pub const` with a sentence each:

```rust
/// Circular error, in tiles, blind and spotted. The whole reason to fly
/// recon: a blind shot is a suppression round, a spotted one is a kill.
pub const CEP_BLIND: f32 = 4.0;
pub const CEP_SPOTTED: f32 = 0.6;
```

A balance change is a one-line diff with its reason beside it. The HUD,
the bot and the docs all read the same constants, so "12 tiles" is never
typed twice.

### 3.6 The map

`map.rs`: a `Vec<u8>` of terrain, row-major, with `Terrain::from_u8`,
sized by the map (`w`, `h`): the generator makes 128 x 128, a map file
declares its own `size`, and the converted prior-art maps are 256 x 256. The
map is also a text file (`to_text`, `from_text`, documentation/MAPS.md):
one glyph per tile and then the places, which any text editor edits.

Since the evening of day one the world calls `route` itself when it
applies a `Move`, keeps the waypoints on the tank, brakes the tank to a
crawl for corners, and plans again from where it stands if water is in
front of it. The reason is in the first trace: a bot drove the straight
line from its start to the centre, met the river a tile from the
bridge, and sat there for the whole match. Neither a client nor a Brain
has the map in its observation, so the pathfinding has to be the
world's.
`u8` rather than `Vec<Terrain>` because the map crosses the wire once in
`Welcome` and 16 KiB is the right size for it. Generation is value noise
for forest, a wandering line for the river, and roads walked from each
start to the centre that become bridges over water. `route` is A* with
terrain cost and no diagonal corner-cutting, simplified to corners.

The one bug found by test so far was here: a road walked across the
river in two-thirds-tile steps could lay a bridge that was only
diagonally connected, which A* correctly refused to cross. The fix was
to lay a plus of tiles at each water crossing, and the test that found
it (`every_start_can_reach_the_centre`) stays.

## 4. treadcore-proto: the wire vocabulary

```rust
pub enum Packet {
    Beacon { peer, name, game },
    Hello { peer, name, game, known: Vec<(PeerId, String, String)> },
    Ping { peer, seq, sent_us }, Pong { .. },
    RttRow { peer, epoch, rtts: Vec<(PeerId, Option<u32>)> },
    Elect { peer, epoch, host, route: Vec<(PeerId, PeerId)> },
    Join, Welcome { player, scenario, tick, map_cells }, Refuse,
    Command { peer, seq, cmd }, CommandAck { seq },
    Snapshot { tick, part, parts, bytes }, Events { seq, tick, events }, EventsAck,
    Heartbeat, Leave,
    Relay { from, to, hops, inner: Vec<u8> },
}
```

One enum is everything that crosses a socket. `encode` prefixes four
magic bytes and a version; `decode` refuses at the first wrong byte.
A packet from a different build is dropped, never misread.

Two small techniques. `Relay` carries its payload as `Vec<u8>`, not
`Box<Packet>`: a forwarding peer does not decode what it forwards, so
a relay node can be on a different minor version of the inner message
and still be useful. And `SnapshotAssembler` reassembles an observation
split across datagrams by tick, keeping only the newest: UDP's
reordering and loss handled in thirty lines, with no retransmission,
because a snapshot that is late is worthless and the next one is
coming.

The Rust style here is the one the language is best at: a closed enum
with named fields, `serde` deriving everything, `bincode` for bytes.
The wire format is the type definition. There is no schema file to keep
in sync.

## 5. treadcore-net: finding each other, electing, relaying

### 5.1 The election is a pure function

```rust
pub fn elect(peers: &[PeerId], rows: &BTreeMap<PeerId, Row>) -> Option<Decision>
```

No sockets, no clocks: a latency matrix in, a host and a route out. For
each candidate, Dijkstra from it; candidates that cannot reach everyone
(even by relay) are out; the winner has the smallest worst-case path
latency, ties by total then by id. The route is the Dijkstra tree:
each peer's next hop toward the host is its parent.

Because it is pure, it has four tests that take microseconds (a central
peer hosts; a peer reachable only through a neighbour is routed through
it; a line of four picks the middle; alone, you host) and the lobby
can call it on every peer with the confidence that identical input
gives identical output. That is the entire trick of a serverless
election: do not vote, compute, and compare.

`BTreeMap` rather than `HashMap` everywhere in this file is not taste.
Iteration order is part of the result (ties, "the lowest id wins"), and
a `BTreeMap` iterates the same on every machine.

### 5.2 The lobby is a state machine with timers

```rust
pub enum Phase { Discover, Probe, Share, Decide, Done }
pub fn poll(&mut self, sock: &mut Socket) -> Option<LobbyResult>
```

`poll` drains the socket, dispatches each packet to `handle`, then runs
the current phase's timer logic. It is called from whoever owns the
loop: a `while` in the server, a Bevy system in the client. No threads.
The simulation's clock stays the only clock, and the lobby's behaviour
is the same in a 20 Hz loop and a 2000 Hz one.

`Hello` carrying the sender's known peers with their addresses is what
makes a lobby of four converge in one round when broadcast only reaches
some of them, and what lets `--peer` seed a lobby across a router.

### 5.3 Host and Client

```rust
pub struct Host { pub world: World, pub seats: Vec<Seat>, pub routes: RouteTable, sock: Option<Socket>, inbox: Vec<Order>, .. }
impl Host {
    pub fn poll(&mut self)                         // joins, orders, acks, heartbeats, relays
    pub fn tick(&mut self, local: Vec<Order>) -> &[Event]   // step, snapshot, events out
}
```

`sock: Option<Socket>` is the line that makes solo play and the debug
trace the same code as LAN play. With `None`, `poll` returns at once
and `tick` skips the send loop. There is no `trait Transport` with two
implementations; there is one struct and one `if let`.

Reliability is three small mechanisms, not a protocol library:

- orders: per-client sequence, resend every 120 ms until acked, host
  applies each once in order (a `BTreeMap<u32, Command>` buffers early
  arrivals);
- events: per-seat sequence, resend the head of a `VecDeque` until
  acked, client delivers once;
- snapshots: fire and forget, newest wins.

The `RouteTable` with `direct` and `via` maps is shared by both ends;
`send` wraps in `Relay` when `via` says so, and `relay` either unwraps
(for me) or forwards (not for me, hops < 8). Neither `Host` nor `Client`
knows which packets were relayed.

The `poll` method's first line, `let Some(mut sock) = self.sock.take()
else { return };`, and its last, `self.sock = Some(sock)`, are the
borrow checker again: taking the socket out of `self` lets the method
mutate seats and routes while holding it, then puts it back. It is a
common Rust idiom for "I need two mutable things from one struct", and
it costs nothing.

### 5.4 Prediction is a partial world and one function

```rust
pub struct Predictor { pub world: World, pub me: PlayerId, in_flight: Vec<(u32, Command)>, history: VecDeque<..>, .. }
impl Predictor {
    pub fn order(&mut self, seq: u32, cmd: &Command)           // applied now, remembered
    pub fn tick(&mut self)                                      // one tick, at the tick rate
    pub fn reconcile(&mut self, obs: &Observation, acked: u32)  // check, adopt, re-apply, re-drive
}
```

The copy is a real `World` built from the `Welcome`, with every list
but the map, the pillboxes, the known mines and the players left
empty or overwritten from each snapshot. It never calls `World::step`:
the sim exposes `World::predict`, which applies one player's hull
orders and calls the same `drive_tank` the host's `step_tanks` calls,
so the two cannot disagree by construction. The check that they do not
is `Tank::tick_state`, a tuple of float bits compared whenever nothing
of the client's is in flight; `prediction_hides_the_lag_and_matches_the_host`
runs host and client on loopback with `Socket::set_lag(100 ms)` each
way and asserts zero mismatches and a 250 ms lead. The transport's lag
queue (`TREADCORE_LAG_MS`) is what stands in for `tc netem` on a
machine without root.

## 6. treadcore-brain: the controller interface

```rust
pub trait Brain: Send {
    fn name(&self) -> &str;
    fn think(&mut self, obs: &Observation) -> Vec<Command>;
}
```

This is the prior art's Brain. Observation in, commands out, and the world
validates the commands exactly as a human's. A Brain cannot cheat
because it has no more access than a keyboard: `Observation` is what
the host already filtered.

`RuleBot` is a doctrine written as a function: keep the bubble up, keep
a recon out, shoot what the recon sees, move after firing, resupply
when low, strike a thin bubble. It thinks four times a second, by
comparing ticks, so four bots cost the host almost nothing.

`JsonBrain` is the same trait over a child process: one JSON observation
per line on stdin, one JSON array of commands per line on stdout, read
by a thread into an `mpsc` channel so a slow bot never stalls the tick.
`Observation` and `Command` serialise with `serde` as they are, so the
schema for an external bot is the Rust type, printed by
`treadcore-server --brain-schema`.

`Box<dyn Brain>` is the one trait object in the project, and it is
where one belongs: the set of controllers is open, and a seat does not
care which it has.

## 7. treadcore-server: the trace

Hand-parsed flags (the loop is forty lines and reads top to bottom),
`env_logger`, and `narrate`:

```rust
Event::ShellLaunched { owner, kind, spotted, eta_ticks, .. } =>
    log::info!("[{:>6} {:>6.1}s] p{} fires {:?} {} eta {:.1}s", t, s, owner, kind,
               if *spotted { "SPOTTED" } else { "blind" }, *eta_ticks as f32 * DT),
```

Every event at `info` as a sentence with tick and seconds in front;
every tank's state once a second at `debug`; `--trace` writes the same
events as JSON lines. `make debug` runs this at `RUST_LOG=trace` with
backtraces on and captures everything under `build/logs/`. The log is
meant to be read by a person, or handed to an AI with the source beside
it, which is why it is prose with numbers and not numbers alone.

The same binary is the LAN host (`--net`, elected) and a LAN bot seat
(`--net`, not elected), so `make lobby-test` runs four of them on one
machine and checks that exactly one says `hosting:`.

## 8. treadcore-client: the window

Bevy, four plugins, each a file:

| file | owns | pattern |
| --- | --- | --- |
| session.rs | `Session` enum (Lobby / Hosting / Client / Ended), `View` resource | `FixedUpdate` at 20 Hz steps the host, or the client's predicted tank; `Update` polls the net |
| render.rs | tile sprites, entity sprites keyed by id, the gizmo overlay | sprites synced from the observation; everything else is lines |
| hud.rs | four text panels | one `String` per panel rebuilt each frame |
| input.rs | keys and mouse to `Orders`, the camera | click to drive via the sim's own A* |

The engine never touches the world directly. `View` holds the last
`Observation` and the map; render and HUD read `View`; input writes
`Orders`; session moves orders to the host or the client and
observations back. Bevy's resources and systems give the plumbing;
the game stays in the sim. A client's own tank is the exception that
proves it: `View.predicted` is the tank from treadcore-net's
`Predictor`, run one tick ahead in `FixedUpdate`, and render draws it
in place of the snapshot's; the HUD's NET panel shows how far ahead,
what is in flight, and the mismatch count (section 5.4).

The overlay is the game's look and it is all `Gizmos`: the bubble as a
ring, the detection margin as a fainter ring, each drone's link as a
line coloured by `link_color(d.link)`, the recon's vision as a disc,
each shell as an arc with an impact circle growing with progress. No
assets. A tank is a rotated rectangle with a line for a barrel.

Pinned to Bevy 0.17 on purpose: it is the API this crate is written
against, and it moves with every minor. `Time::<Fixed>::from_hz(20.0)`
gives the host's tick; `AccumulatedMouseScroll` is zoom; `Camera2d`
with an `OrthographicProjection` scale is the camera.

## 9. The build system

`Makefile` is the front door and prints its own targets; `scripts/*.sh`
do the work; `cargo` is what they call. `configure.sh` checks the
toolchain and the display libraries and writes `build/config.env`;
every other script `need_config`s and refuses to run without it.
`build.sh` builds headless automatically when the display libraries are
missing. `debug.sh` is the trace. All output goes to the terminal and
to a stamped file under `build/logs/` both, so nothing lives only in
scrollback.

## 10. What the Rust style here is, in one paragraph

Data is plain structs and closed enums with `serde` derives; behaviour
is functions on them. The one trait object is where the set is open
(Brains). Mutation follows the shape the borrow checker wants: read a
snapshot, decide, apply. Determinism is a test, not an aspiration.
Failure of input is an event, not a panic; failure of the machine (a
bind error) is a log line and a fallback. Every tuning number has a
sentence. Every layer can be run without the layer above it, and the
workspace makes that a compile-time fact.

## 11. What to do next

1. The window: `doas apk add mesa-vulkan-swrast`, then
   `target/debug/treadcore-warm`, then `make run`. Or build the
   window on a machine with a GPU; the headless crates do not care.
2. `make warm` for the release profile, measured, so `make run` is a
   link and not an hour.
3. `make lobby-test` on two machines.
4. The backlog in ROADMAP.md: the interceptor-versus-interceptor
   decision, tank speed, drone retasking, the match report.
5. Then roadmap steps 1 and 2 (replay, hash), and the Pillboys
   (step 3) whose design is now written.

