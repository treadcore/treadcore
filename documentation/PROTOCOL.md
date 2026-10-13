# treadcore protocol

Version 10. One UDP socket per process, default port 47296. Every datagram
is `TRDC`, a little-endian u16 version, then a bincode `Packet`
(`source/treadcore-proto`). Anything else is dropped at the first byte.

## Finding each other, and electing a host

No one sets up a server. Each process runs the same lobby state machine
(`source/treadcore-net/src/lobby.rs`):

1. **Discover.** Every second, shout `Beacon { peer, name, game }` to the
   LAN broadcast address and send `Hello` to any address given with
   `--peer`. Answer every Beacon or Hello with a Hello carrying every
   peer known so far and the address it was heard from, so the set
   converges in one round even where broadcast does not reach. Stop when
   `--want` peers are known or after 8 seconds. Alone: host, decided.
2. **Probe.** For 1.5 s, `Ping` every peer ten times a second and keep
   the round-trip of every `Pong`. Clocks are never compared across
   machines: the sender's timestamp goes out and comes back untouched.
3. **Share.** Publish `RttRow`: this peer's median round trip to every
   peer, or none where probes went unanswered. Repeat until every peer's
   row is in, or 1.5 s.
4. **Decide.** Run the election on the matrix. Announce `Elect { host,
   route }`. Collect everyone else's. The announcement from the lowest
   peer id is final; it is normally identical to everyone's, and when
   rows were lost it is the tie-break everyone agrees on.

The election (`election.rs`) is a pure function of the matrix. Link cost
between two peers is the mean of the two measured directions, or the one
that was measured, or absent. For each candidate, Dijkstra from it;
candidates that cannot reach every peer, even by relay, are out. The
winner has the smallest worst-case path latency to any peer; ties go to
the smallest total, then the smallest id. The Dijkstra tree from the
winner is the **connectivity route**: each peer's next hop toward the
host is its parent in that tree, the host itself when the link is
direct, a neighbour when it is not.

This is Bolo's ring, generalised: a ring forwards everything through
everyone; a tree forwards only for the peers that need it, and the host
is the peer the tree is shortest from.

## The session

Host-authoritative. The host owns the world; clients send orders and
receive their view.

- `Join { peer, name }` from a client, repeated every 500 ms until
  `Welcome { player, scenario, tick, map }` or `Refuse`. `Welcome`
  carries the whole `Map` as it is at that tick (cells, starts, depots,
  pillboxes), not just the cells, because a map from a file is not
  something a client can regenerate from a seed. Encoded, it is 17 KiB
  for a generated map and over 130 KiB for a 256 x 256 map file (the
  map, and the scenario's copy of the file's text), so it does not go
  as one datagram: the host sends it as `Chunk { id, part, parts,
  bytes }` pieces under `MAX_DATAGRAM`, a fresh `id` per Welcome, and
  the client reassembles by id and decodes the whole. Any packet too big
  for a datagram may travel this way. The pieces are unreliable like
  snapshots: a lost one loses the Welcome, and the client's next `Join`
  draws a fresh one. The old note stays true of the fragments of


  an Ethernet LAN and arrives; across the Internet, where fragments
  are often dropped, it will need splitting the way snapshots are
  (roadmap step 7). The receive buffer is 64 KiB for this reason: a
  2 KiB buffer truncated it silently and no client ever joined.

- `Command { seq, cmd }`: an order. Per-client monotonic `seq`; resent
  every 120 ms until `CommandAck { seq }`. The host applies each seq
  once, in order, buffering any that arrive early.
- `Snapshot { tick, part, parts, acked, bytes }`: the player's
  observation, every other tick (10 Hz), bincode, split into datagrams
  under 1200 bytes. Unreliable: a lost one is replaced by the next. The
  client reassembles by tick and keeps only the newest. `acked` is the
  last of that client's command seqs the host had applied by `tick`:
  a predicting client re-applies everything after it (below).
- `Events { seq, tick, events }`: what happened that the player may know
  about. Resent every 120 ms until `EventsAck { seq }`; a kill is never
  lost. Delivered once, in order.
- `Heartbeat { tick, hash_tick, hash, rtt_ticks }` both ways every
  500 ms; six seconds of silence and the other side is gone. A client's
  heartbeat echoes the host tick it last saw, which the host turns into
  a latency estimate (its tick minus the echoed one: a snapshot out and
  a heartbeat back) and sends back as `rtt_ticks`; that is the HUD's
  rtt. The host's heartbeat also carries the world hash as of the last
  multiple of 1 000 ticks (`HASH_EVERY`); every client logs it, and
  four peers' logs with one hash at one tick is the lobby test's
  proof. A client sends 0 for the hash (it runs its own tank, not the
  world: below) and 0 for `rtt_ticks` always.

- `Leave`.

Orders from every seat for a tick are sorted by player before the step,
so arrival order never changes the result.

## Prediction

The host is authoritative; a client is told the world at 10 Hz, half a
round trip late, and its own orders take a whole round trip to show.
So a client runs its own tank itself (`treadcore-net/src/predict.rs`,
roadmap step 4): a partial copy of the world built from `Welcome` (the
scenario, the map as the host has it) and kept up from every snapshot
(every pillbox, the mines it knows, the teams) and every terrain event
(craters, walls, roads, felled forest). The orders that move the hull,
`Move`, `Stop` and the two fire orders (firing turns the tank), are
applied to the copy the moment they are sent; the copy's tank is driven
one tick at the tick rate by the same function the host runs
(`World::predict`, which calls the host's `drive_tank`). Every
snapshot replaces the copy's tank with the host's, re-applies the
orders with seq after the snapshot's `acked`, and drives the ticks the
client was ahead again. Nothing predicted leaves the client; the host
never hears of it.

The check: whenever none of the client's orders is in flight across a
snapshot's span, the tank the host reports at tick T must equal the
tank predicted at T bit for bit (position, heading, speed, waypoint,
route length). A difference is logged as a mismatch with the distance,
counted, and shown on the HUD. `make lobby-test LAG=200` holds every
received datagram 200 ms (`TREADCORE_LAG_MS`, the transport's stand-in
for `tc netem`, which needs root) and requires zero mismatches across
the three client seats. A client's copy is of its own tank only, which
is why its heartbeat still carries no world hash: it cannot run what
it is not shown.

## Relay

A packet for a peer the sender cannot reach directly is wrapped:
`Relay { from, to, hops, inner }` and sent to the next hop from the
route table. A peer that receives a Relay not addressed to it forwards
along its own table, incrementing `hops`; at 8 it is dropped. Both the
host and the client wrap and unwrap the same way; the session code does
not know which packets were relayed.

## What a client sees

`Observation` (`source/treadcore-sim`): the tick, the player's own tank
in full (its squad included), every tank, drone and shell inside the
player's or an ally's sensor discs, every depot and every pillbox (territory is
public), the mines the player knows about (its own, and hostile ones
within a tile and a half of its tank or squad), the eight nearest
forest tiles (a Brain has no map), every player's name, score and
life, the events for this tick the player is allowed to know about, and
whether the match is over, and which team won; every player's team;
the proposals (to join, recruit, swap or merge) the player made or may
answer.
 Terrain changes (walls built, forest

felled, mine craters) arrive as `TerrainChanged` events; shell craters
as `ShellImpact`; the client applies both to its copy of the map.


The map is not in it: it went in `Welcome`, and craters arrive as
impact events, which the client applies to its copy.

The same `Observation` is what a Brain gets. There is no second channel.
