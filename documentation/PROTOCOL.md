# treadcore protocol

Version 13 (12 until roadmap step 10 gave the sim structures, the
longshot and two terrains). One UDP socket per process, default port 47296. Every datagram
is `TRDC`, a little-endian u16 version, then a bincode `Packet`
(`source/treadcore-proto`). Anything else is dropped at the first byte.

## Finding each other, and electing a host

No one sets up a server. Each process runs the same lobby state machine
(`source/treadcore-net/src/lobby.rs`):

1. **Discover.** Every second, shout `Beacon { peer, name, game, server }` to the
   LAN broadcast address and send `Hello` to any address given with
   `--peer`. Answer every Beacon or Hello with a Hello carrying every
   peer known so far and the address it was heard from, so the set
   converges in one round even where broadcast does not reach. Stop when
   `--want` seats are known (a server is not one; below) or after 8
   seconds. Alone: host, decided.
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

This is the prior art's ring, generalised: a ring forwards everything through
everyone; a tree forwards only for the peers that need it, and the host
is the peer the tree is shortest from.

## A dedicated server

Roadmap step 8: `treadcore-server --listen DIR` (`make listen`) is a
peer that runs the same lobby with `server` set in its `Beacon`, its
`Hello` (and in every `Hello`'s list of known peers, the fourth field),
and its `Register`. It is the "someone runs one for the group" option,
without making it the only option:

- **No seat.** It does not count toward anyone's `--want`, and no host
  seats it: a lobby's result drops every server but the elected host.
- **It follows.** It never hosts alone and never times out of
  Discover; it starts probing when the first seat's `Ping` arrives,
  which is when that seat has. It never joins a match it is told of.
- **The election prefers it** (`election::elect_among`). Each server is
  weighed over the seats plus itself, with `SERVER_PREFERENCE` (10 ms)
  taken off its worst path; each seat over the seats alone. The least
  key wins (worst path less any preference, then total, then id). So a
  server within 10 ms of the best seat hosts, and a server that loses
  is in no tree at all: never a relay, out of the match, back to
  listening. A seat can leave mid-match; a server runs nothing else.

When elected it hosts as any host does, fills the empty seats with
bots, and writes `match-NNNN.trace.jsonl` and `match-NNNN.replay.json`
to DIR. It serves a status page over plain HTTP on `--http` (TCP, the
UDP port's number by default): `/` for a person, `/status.json` for a
script, and the files of each match it has hosted, by name. With
`--introducer` it is listed there under its lobby name with `server`
set, and `hosting` set while a match runs, as any host.

One server can sit in several lobbies at once: `--game` again, each a
thread of the process with its own UDP port (`--port`, then the
ports after it) and its own peer in the lobby, under one peer id (the
introducer keys an entry by lobby and peer, so the entries do not
collide). A lobby still hosts one match at a time; each writes to
DIR/NAME, and the one status page lists every lobby (`games` in
`status.json`; with one lobby its fields also stand at the top, as
before).

## Across the Internet: the introducer

Nothing above reaches past the LAN: the beacon is a broadcast, and a
peer behind a home router has an address only its router knows. Roadmap
step 7 adds the smallest thing that fixes it, the **introducer**
(`source/treadcore-net/src/introducer.rs`; `treadcore-server
--introduce`): a UDP service that hosts no game, keeps no state past
thirty seconds, and tells the peers registered under one lobby name
each other's addresses. It is not a server anyone plays on; a group
runs one anywhere with a public address, or borrows one.

- `Register { peer, name, game, local, hosting, server }` from a peer to the
  introducer: every second while looking for a game, every five while
  hosting one. `local` is the address the peer sees for itself (its
  LAN address, guessed from the interface the default route uses); the
  introducer notes the address the datagram came from beside it, which
  is the peer's public address as its router rewrote it. `hosting` says
  a match is running here; `server`, that this is a dedicated server
  (above), listed under this lobby name match after match.
- `Introduce { game, peers }` from the introducer to every peer in
  `game`: every second, and at once when someone new registers. Each
  entry is a peer id, name, public address, local address, whether
  it is hosting, and whether it is a server. A registration lapses after thirty seconds of silence.

A lobby given `--introducer HOST:PORT` registers in its Discover phase
and, with `--no-beacon`, shouts nothing on the LAN. Every peer it is
introduced to is greeted at both addresses, every 250 ms in Discover
and with every probe in Probe, until a packet comes back from the peer
directly; whichever address it came from is the peer's address from
then on (the same rule already settled addresses behind a NAT that
rewrites ports). That is hole punching in its plain form: each side's
first packet out opens its own router for the other's packets in. The
local address is what works for two peers behind the same router. A
pair whose routers never pass a packet either way (symmetric NAT on
both sides) shows in the matrix as an unmeasured link, and the
election routes it through a peer that reaches both, exactly as on a
LAN with a broken segment. Nothing in the election, the relay, or the
session changed for this step.

**Rejoin** (SPEC GAM-12, backlog item 10). A seat belongs to a peer id,
and `--identity FILE` keeps a peer id across restarts (made the first
time, sixteen hex digits). A host keeps registering while it hosts,
with `hosting` set. A peer that drops and starts again registers under
the same lobby name, is told who is hosting, pings that host at both
its addresses until it answers (a host answers a `Ping` from any peer
that has a seat), and decides for it without an election. Its `Join`
carries a `session` number, random per process; the host sees the
same peer id with a new session, gives the seat back at the tick the
match has reached, and starts the seat's sequence numbers over, so the
newcomer's first order (seq 1 again) is applied. A repeated `Join`
from the same process (a lost Welcome) has the same session number and
resets nothing. On the way back the host also pings every address the
introducer gives for a seated peer that is not joined, which opens the
host's router for the returning `Join`. What was the player's is still
theirs: the seat never left the world, its tank drove on under no
orders, its pillboxes and depots kept their owner. There is no window:
the seat waits as long as the match.

**Late arrival** (roadmap step 8). A peer with no seat that finds a
running host the same way (the introducer says who is hosting) is
answered while a bot's seat is free: the host answers its `Ping`, and
on its `Join` gives it the lowest-numbered bot's seat whose tank is
alive. The world is told by order, `Command::TakeSeat { name }` in
that tick's orders, so a replay and the world hash carry it: the seat
is a person's from then, under their name, with everything the bot
held, and every seat hears `Event::SeatTaken`. Whoever runs the bots
retires that one (`treadcore_brain::retire_taken`). With no seat free
the host does not answer the `Ping`, so a late group elects among
itself; a `Join` that gets through anyway is refused ("no free seat:
every live seat is a person's").

## The session

Host-authoritative. The host owns the world; clients send orders and
receive their view.

- `Join { peer, name, session }` from a client, repeated every 500 ms
  until `Welcome { player, scenario, tick, map }` or `Refuse`.
  `session` is the client process's number (rejoin, above). `Welcome`
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
client was ahead again. A terrain event (a crater, a wall) for a tick
the client has already predicted past does the same from the last
snapshot's tank: the ticks up to the event's are driven over the old
map, the rest over the new one, because the host changes its map after
that tick's drive. Nothing predicted leaves the client; the host never
hears of it.

The check: whenever none of the client's orders is in flight across a
snapshot's span, and a snapshot has been adopted before (the first one
after a Welcome is the baseline: a seat that comes back mid-match has
a copy built from the scenario's start), the tank the host reports at
tick T must equal the tank predicted at T bit for bit (position,
heading, speed, waypoint, route length). A difference is logged as a mismatch with the distance,
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
