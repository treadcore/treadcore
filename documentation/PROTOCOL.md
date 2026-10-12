# treadcore protocol

Version 1. One UDP socket per process, default port 47296. Every datagram
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
  `Welcome { player, scenario, tick, map_cells }` or `Refuse`.
- `Command { seq, cmd }`: an order. Per-client monotonic `seq`; resent
  every 120 ms until `CommandAck { seq }`. The host applies each seq
  once, in order, buffering any that arrive early.
- `Snapshot { tick, part, parts, bytes }`: the player's observation,
  every other tick (10 Hz), bincode, split into datagrams under 1200
  bytes. Unreliable: a lost one is replaced by the next. The client
  reassembles by tick and keeps only the newest.
- `Events { seq, tick, events }`: what happened that the player may know
  about. Resent every 120 ms until `EventsAck { seq }`; a kill is never
  lost. Delivered once, in order.
- `Heartbeat { tick }` both ways every 500 ms; six seconds of silence
  and the other side is gone. A client's heartbeat echoes the host tick
  it last saw, which the host turns into a latency estimate.
- `Leave`.

Orders from every seat for a tick are sorted by player before the step,
so arrival order never changes the result.

## Relay

A packet for a peer the sender cannot reach directly is wrapped:
`Relay { from, to, hops, inner }` and sent to the next hop from the
route table. A peer that receives a Relay not addressed to it forwards
along its own table, incrementing `hops`; at 8 it is dropped. Both the
host and the client wrap and unwrap the same way; the session code does
not know which packets were relayed.

## What a client sees

`Observation` (`source/treadcore-sim`): the tick, the player's own tank
in full, every tank, drone and shell inside the player's sensor discs,
every depot, every player's name, score and life, the events for this
tick the player is allowed to know about, and whether the match is over.
The map is not in it: it went in `Welcome`, and craters arrive as
impact events, which the client applies to its copy.

The same `Observation` is what a Brain gets. There is no second channel.
