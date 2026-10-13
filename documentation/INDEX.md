# treadcore: code structure index

An index of the repository as of commit `0874ff5` (13 October 2026):
the tree, an outline of every file, a glossary, and a keyword
cross-reference. Line numbers are those of that commit; `grep -n` is
the authority when they drift. The walk-through with the reasoning is
LAB-REPORT.md; this is the map.

## 1. The tree

```
Cargo.toml            workspace: seven crates in source/, one profile set
Makefile              every entry point: configure, warm, build, debug, run,
                      lan, host, seat, lobby-test, replay, map, convert, test
README.md             what the game is, how to build and run it, the documents
CHANGELOG.md          what changed, by date, newest first
CONTRIBUTING.md       how to send a change
CONTRIBUTORS.md       who has
LICENSE               MIT
CNAME                 treadcore.org (the GitHub Pages domain)
index.html            the website's front page
reports/              published write-ups (2026-10-12-opening-day.html)
documentation/        the documents (section 4)
journal/              one entry per working session, RFC 822 header, IEEE
                      sections; TEMPLATE.txt and README.txt say the form
maps/                 island.map (the generated default, saved); prior-art/
                      five converted community maps
scripts/              the shell behind every make target, and the converter
source/               the crates, dependency order below
vendor/               README.md, THIRD-PARTY.md: what is taken from where
build/                configure.sh's config.env and every log (untracked)
target/               cargo's (untracked)
```

The crates, each depending only on those above it:

```
treadcore-sim      the world: map, tanks, drones, shells, squads, pillboxes,
                   depots, mines, teams, proposals; deterministic; no I/O
treadcore-proto    the wire vocabulary: one Packet enum, chunking, snapshots
treadcore-net      UDP transport, lobby and election, host and client
                   sessions, the relay route, client-side prediction
treadcore-brain    the Brain trait and three brains: RuleBot, Defender,
                   JsonBrain (an external program over stdin/stdout)
treadcore-server   headless binary: --local bot matches with a trace and a
                   match report, --net lobby then host or seat, --replay
treadcore-client   the window (Bevy): session, render, hud, input
treadcore-warm     a tiny Bevy program that exists to build the engine once
```

## 2. The source, file by file

Items are `name  line`. Private helpers are listed where they carry
a rule; tests are listed by name.

### 2.1 source/treadcore-sim/src/lib.rs (3 680 lines)

The world. Header comment: the premise, determinism, units.

| item | line | what |
| --- | --- | --- |
| `TICK_HZ`, `DT` | 37 | 20 ticks a second |
| `MAX_PLAYERS` | 41 | 16 seats |
| `EntityId`, `PlayerId`, `Tick` | 44 | u32, u8, u64 |
| `V2` | 51 | tile-space vector: `len`, `dist`, `norm`, `scale`, `lerp`, `heading`, `from_heading`, `tile` |
| `Rng` | 110 | xorshift: `next_u64`, `f32`, `range`, `below`, `disc` |
| `DroneKind` | 145 | Recon, Interceptor, Strike |
| `DroneTask` | 160 | Orbit, Intercept, Strike, Return |
| `Drone` | 172 | id, owner, kind, pos, heading, task, battery, link, tether |
| `ShellKind`, `Shell` | 192 | Indirect or Direct; `progress`, `pos_at` |
| `Bay` | 228 | counts per kind: `get`, `take`, `put` |
| `BuildKind` | 269 | Pillbox, Wall, Mine, Road: `cost`, `ticks`, `standoff` |
| `Mine` | 311 | owner, pos |
| `PillboyTask` | 323 | Capture, Repair, Build, Harvest, Return: `site`, `reach` |
| `Pillboys` | 358 | the squad: strength, material, aboard, task, route; `fresh`, `deployed` |
| `Pillbox` | 388 | owner, health, shells, reload: `alive`, `shooter`, `hostile_to` |
| `Tank` | 417 | pos, heading, speed, armor, shells, fuel, bay, goal, route, reload, bubble, last_hit_tick, alive, pillboys |
| `Tank::tick_state` | 450 | the bits prediction compares |
| `Depot` | 456 | pos, owner, capture, capturer |
| `TeamId`, `Player` | 468 | id, name, tank, alive, score, is_bot, team |
| `ChatTo`, `Terms`, `AllianceCmd`, `Proposal` | 483 | the proposal protocol's words |
| `Command` | 532 | Move, Stop, FireIndirect, FireDirect, Launch, Recall, Retask, SetBubble, Deploy, Alliance, Propose, Answer, Chat |
| `Command::predicted` | 561 | the four hull orders a client predicts |
| `Order` | 569 | player + command |
| `Event` | 578 | everything that happens in a tick (the HUD, log, bot, trace and replay read these) |
| `LossReason` | 637 | why a drone is gone |
| `MapSource` | 653 | FromScenarioSeed or Text: `build` |
| `GameType` | 672 | Open, Tournament, Strict: `shells`, `material`, `parse` |
| `Scenario` | 714 | seed, map, game_type, advantage, players, teams, swarm, time_limit; `bots`, `with_teams`, `seat_name` |
| `World` | 782 | the state; `events` is the last tick's |
| `World::new` | 809 | from a scenario: starts, depots, pillboxes, tanks |
| `map_as_played` | 891 | the map with craters and walls |
| `tank`, `tank_of`, `drone`, `pillbox` | 897 | lookups |
| `route_for` | 916 | the map's A* avoiding hostile guns and known mines |
| `mine_known_to` | 933 | own, allied, or within MINE_REVEAL |
| `team_of`, `allied`, `allied_opt`, `team_members` | 968 | sides |
| `visible_to`, `spotted_by`, `concealed`, `tank_visible_to` | 1001 | fog, spotting, forest cover |
| `step` | 1062 | one tick: proposals lapse, orders apply, then tanks, pillboys, drones, pillboxes, shells, depots, swarm, match |
| `apply` | 1092 | validates and applies one order, or rejects it with a reason |
| `apply_alliance`, `outcome`, `three_way_check`, `propose`, `answer` | 1404 | teams and the three-way rule |
| `step_pillboys` | 1547 | the squad walks, captures, repairs, builds, harvests |
| `explode_mine` | 1811 | damage, chain orthogonally, crater |
| `step_pillboxes` | 1856 | guns: tanks first, drones second |
| `step_tanks` | 1993 | every tank through `drive_tank`; mines set off |
| `predict` | 2022 | one player's tank alone (prediction) |
| `step_drones` | 2040 | orbits, links, intercepts, strikes, recovery |
| `step_shells` | 2255 | flight and impact; splash; mines cleared |
| `damage` | 2338 | armor, kills, score |
| `step_depots` | 2385 | capture, resupply, repair; contested does nothing |
| `step_swarm` | 2467 | neutral strikes from the edge |
| `step_match` | 2511 | last side standing, or the time limit |
| `hash` | 2548 | FNV-1a over the bincode of the world |
| `run` | 2555 | a world from a replay |
| `observe` | 2571 | what player p may see: `Observation` |
| `event_visible` | 2603 | which events p may know |
| `drive_tank` | 2623 | one tank's motion for one tick (host and client both) |
| `fnv1a64` | 2696 | the hash function |
| `CHECKPOINT_EVERY`, `Replay` | 2708 | scenario, orders per tick, hashes every 100: `record`, `orders_for`, `final_hash`, `verify` |
| `Observation`, `PlayerView` | 2786 | the view; `my_team`, `allied`, `my_drones`, `hostile_tanks`, `hostile_drones`, `allied_tanks`, `centre`, `clamp` |
| `wrap_angle` | 2868 | to (-pi, pi] |
| tests | 2880 | deterministic, interceptor_takes_station_and_launch_is_validated, indirect_shell_flies_and_lands, move_is_routed_across_the_river, prediction_drives_the_tank_as_the_host_does, hash_follows_state, replay_reproduces_the_world, pillbox_kills_a_tank_and_routes_bend_round_it, pillboys_capture_a_pillbox, squad_harvests_walls_and_mines, mines_chain_and_shells_clear_them, world_from_a_map_file, a_team_shares_recon_and_spares_its_own, alliances_form_and_break_and_teams_end_the_match, proposals_change_sides_under_the_three_way_rule, retask_moves_a_recon_and_recalls_one_drone, forest_block_conceals_a_tank, game_types_set_the_start, sixteen_seats, swarm_meets_bubble |

### 2.2 source/treadcore-sim/src/map.rs (790 lines)

| item | line | what |
| --- | --- | --- |
| `MAP_W`, `MAP_H` | 16 | 128 x 128 generated |
| `Terrain` | 21 | Grass, Forest, Road, Water, Bridge, Rubble, Depot, Wall: `from_u8`, `move_cost`, `passable`, `blocks_fire`, `glyph`, `from_glyph` |
| `DepotOwner`, `MapDepot`, `MapPillbox` | 71 | what a map file declares |
| `Map` | 92 | w, h, cells, starts, depots, pillboxes |
| `MapError` | 109 | line and message |
| `Map::generate` | 155 | island from a seed: noise, river, bridge, ring road, spokes, starts, depots, pillboxes |
| `crater` | 320 | a shell's mark |
| `in_tile`, `in_bounds`, `at`, `at_tile`, `move_cost`, `center` | 334 | cell access |
| `line_clear` | 360 | direct fire's line of sight |
| `set`, `tile_centre`, `forest_near` | 373 | edits and queries |
| `route`, `route_with` | 416 | A* over tiles with a danger cost, simplified to corners |
| `MAP_FORMAT`, `to_text`, `from_text` | 526 | the text file format (MAPS.md) |
| `Noise` | 694 | value noise for the generator |
| tests | 717 | generation_is_deterministic_and_starts_are_dry, map_survives_the_file_round_trip, bad_map_files_are_refused_with_a_line, every_start_can_reach_the_centre |

### 2.3 source/treadcore-sim/src/rules.rs (222 lines)

Every number of the game, grouped: the tank (10), the gun (37), the
drone crew (61), pillboys and pillboxes (98), mines (132), depots
(190), alliances (199), scoring (209), `SWARM_OWNER` 255 and `NEUTRAL`
254 (186, 214), `drone_speed` (216). A change here is a proposal under
GOVERNANCE.md section 5 and wants a trace.

### 2.4 source/treadcore-proto/src/lib.rs (329 lines)

| item | line | what |
| --- | --- | --- |
| `MAGIC`, `VERSION` | 27 | `TRDC`, 10 |
| `PORT`, `MAX_DATAGRAM` | 35 | 47296, 1200 bytes |
| `PeerId` | 43 | u64 for the life of a process |
| `Packet` | 46 | Beacon, Hello, Ping, Pong, RttRow, Elect; Join, Welcome, Refuse, Command, CommandAck, Snapshot (with `acked`), Events, EventsAck, Leave, Heartbeat, Chunk; Relay |
| `DecodeError` | 129 | TooShort, BadMagic, Version, Body |
| `encode`, `decode`, `sender` | 148 | framing |
| `chunk_packets`, `ChunkAssembler` | 191 | a big packet in pieces (Welcome's map) |
| `snapshot_packets`, `SnapshotAssembler` | 233 | an observation in pieces, newest tick wins |
| tests | 279 | round_trip, big_packets_chunk_and_reassemble, snapshot_splits_and_reassembles |

### 2.5 source/treadcore-net/src/

**lib.rs (49)**: the crate's five parts in a comment; `new_peer_id` 42.

**transport.rs (131)**: `LAG_ENV` 15 (`TREADCORE_LAG_MS`); `Socket` 17
with `bind` 31, `set_lag` 53, `send` 57, `broadcast` 70, `recv` 85
(the 64 KiB buffer; the lag queue), `local_with_loopback` 125.

**lobby.rs (391)**: `LobbyConfig` 27, `Phase` 59 (Discover, Probe,
Share, Decide, Done), `Peer` 68, `LobbyResult` 88 (`i_host`,
`addr_of`), `Lobby` 106 with `poll` 198 (the state machine), `compute`
299 (calls the election), `handle` 331 (every negotiation packet).

**election.rs (145)**: `Row` 16, `Decision` 19 (host, route), `link`
30, `dijkstra` 40, `elect` 64 (the pure function); tests
central_peer_hosts, unreachable_peer_routes_through_neighbour,
relay_needed_when_host_cannot_see_peer, solo.

**session.rs (664)**: constants 25 (RESEND 120 ms, `HASH_EVERY` 1000,
HEARTBEAT 500 ms, TIMEOUT 6 s, SNAPSHOT_EVERY 2, EVENT_BACKLOG 64);
`RouteTable` 37 (`resolve`, `send`, `relay`); `Seat` 81; `Host` 96
with `new` 118, `record` 145, `poll` 158 (joins, orders, acks,
heartbeats, timeouts), `tick` 251 (step, hash, snapshots with `acked`,
events), `shutdown` 294; `ClientState` 309; `Client` 316 with `new`
352, `send` 382 (also to the predictor), `predict_tick` 394,
`predicted_tank` 404, `take_observation` 408, `drain_events` 412,
`poll` 424 (Welcome builds the predictor; snapshots reconcile it;
events keep the maps up), `leave` 541 (logs the prediction summary);
tests loopback_session 557,
prediction_hides_the_lag_and_matches_the_host 599.

**predict.rs (195)**: the header comment is the design; `HISTORY` 64
ticks, `MAX_AHEAD` 40; `Predictor` 36 with `new` 61 (from a Welcome),
`tank` 69, `order` 75, `tick` 87, `map_keeps_up` 108 (craters and
terrain events, shared with the window), `reconcile` 126 (check,
adopt, re-apply, re-drive), `in_flight` 187, `summary` 192.

### 2.6 source/treadcore-brain/src/lib.rs (1 158 lines)

| item | line | what |
| --- | --- | --- |
| `Brain` | 21 | `name`, `think(&Observation) -> Vec<Command>` |
| `run_bots` | 27 | every bot's orders for a tick |
| `RuleBot` | 78 | the attacker: recon, counter-battery, scoot, resupply, squad, storm |
| `PILLBOY_REACH`, `STANDOFF`, `PATIENCE` | 113 | its numbers |
| `RuleBot::new`, `without_pillboys` | 124 | seeded |
| `RuleBot::think` | 152 | the doctrine, in order |
| `Defender` | 476 | at its depot: recon on the approach, dodge, moat, mouth pillbox, road, approach mines, walls, proposals |
| `MOAT_RADIUS`, `MOUTH`, `WATCH`, `APPROACH`, `PROPOSE_EVERY` | 504 | its numbers |
| `Defender::new`, `ring`, `mouth_sites`, `approach_sites` | 518 | the build sites |
| `Defender::think` | 590 | the doctrine, in order |
| `JsonBrain` | 839 | an external program: one JSON observation out per think, one JSON command list in; `spawn`, `schema_examples` 897 |
| tests | 921 | pillboys_pay, fortification_pays, defenders_hold, bots_fight_to_a_result |

### 2.7 source/treadcore-server/src/main.rs (758 lines)

`Args` 24 and `parse` 58 (`HELP` 136 lists every flag); `main` 195
dispatches to `run_replay` 233, `run_local` 582, `run_net` 616.
`make_brain`, `make_brain_kind` 272 (RuleBot, Defender, or an external
`--brain`); `Report` 304 with `tally` 331 and `print` 394 (the match
report: shells, drones, kills by cause, pillboxes, depots, proposals);
`Trace` 445 (one JSON event per line); `narrate` 464 (the log's
words for every event); `summary` 527; `save_replay` 222, `save_map`
554; `scenario_from` 563. In `run_net`, the host loop ticks at 20 Hz
and the seat loop predicts at 20 Hz.

### 2.8 source/treadcore-client/src/

**main.rs (160)**: `TILE` 31 (8 px), `Args` 34 and `parse_args` 50,
`main` 106 (the plugins), `to_screen`, `to_tile` 134, `player_color`
145.

**session.rs (374)**: `SessionPlugin` 22; `Orders` 36; `View` 40 (obs,
map, me, log, status, net, received, predicted); `NetStats` 55;
`Session` 73 (Lobby, Hosting, Client, Ended); `start` 83; `scenario`
105; `host_local` 122; `step_host` 143 (FixedUpdate: bots, the local
player, tick); `step_client` 178 (FixedUpdate: predict); `poll` 186
(Update: the lobby's phases, the client's packets, the host's socket);
`push_events`, `describe` 298 (the event log's words).

**render.rs (407)**: `Sprites` 40, `Fog` 49, `terrain_color` 65,
`build_map` 78, `update_fog` 120, `sync_entities` 154 (tanks, the
predicted own tank, drones, shells, depots, pillboxes, mines, squads),
`link_color` 275, `overlay` 287 (the gizmos: bubble, links, vision,
arcs).

**hud.rs (204)**: `HudState` 19, `Panel` 30, `HELP` 85 (the controls),
`telemetry` 104, `scoreboard` 161 (NET line, hash, prediction line,
seats, proposals, depots), `log` 198.

**input.rs (180)**: `Cam` 22, `cursor_tile` 33, `camera` 39, `orders`
84 (every key and click to a Command).

### 2.9 source/treadcore-warm/src/main.rs (45 lines)

`main`, `setup`, `spin`, `quit`: a spinner, so `make warm` builds Bevy
with no game code in the way.

### 2.10 scripts/ and the Makefile

| file | what |
| --- | --- |
| common.sh | paths, `say`/`warn`/`die`, `need_config` (every script sources it) |
| configure.sh | checks the machine, writes build/config.env (`make configure`) |
| sysprep.sh | root-level memory settings for the engine build |
| warm.sh | builds the engine alone, gently (`make warm`) |
| buildmode.sh | makes the desktop cheap while the engine builds |
| build.sh | `build.sh PROFILE WHAT` (`make build`, `make headless`) |
| debug.sh | the bot-only trace: log, trace.jsonl, replay.json (`make debug`; knobs GAME_SECONDS, SEED, PLAYERS, DEFENDERS, TEAMS, GAMETYPE, MAP, BRAIN) |
| replay.sh | plays a replay back and checks every checkpoint (`make replay`) |
| run.sh | the window or headless in a mode: run, lan, host, seat |
| lobby-test.sh | four headless peers on loopback; asserts one host, three welcomed, all ended, one hash, zero prediction mismatches (`make lobby-test`, `LAG=200`) |
| memwatch.sh | memory every N seconds (`make mem`) |
| priorart2map.py | converts a prior-art binary map to a treadcore map (`make convert FROM=`) |

The Makefile's knobs (line 50 on): BUILDDIR, LOGDIR, GAME_SECONDS,
SEED, NAME, JOBS, VERBOSE, GAME, WANT, PEER, PLAYERS, DEFENDERS, LAG,
TEAMS, GAMETYPE, MAP, BRAIN.

## 3. Glossary

- **acked**: on a `Snapshot`, the last of a client's command seqs the
  host had applied by that tick; a predicting client re-applies
  everything after it.
- **advantage**: `Scenario.advantage`; a bot seat is told where every
  tank is (`--advantage`).
- **Bay**: the drones aboard a tank, by kind; `BAY_FULL` is 2/6/3.
- **blind / spotted**: an indirect shell at a point no sensor of the
  firer's side sees lands with `CEP_BLIND` scatter; a spotted one with
  `CEP_SPOTTED`.
- **Brain**: anything with `think`; a bot. `JsonBrain` runs one as a
  program.
- **bubble**: the radius the interceptors hold station at
  (`SetBubble`); strikes entering it are engaged; `DETECT_MARGIN`
  beyond it is the warning.
- **checkpoint**: a replay's hash every `CHECKPOINT_EVERY` (100) ticks.
- **concealed**: a tank inside a 2 x 2 block of forest is seen only
  within `CONCEAL_RANGE`.
- **contested depot**: two sides on one depot: nothing happens there.
- **Defender**: the brain that stays at its depot and fortifies it.
- **depot**: a capturable point; resupply and repair for its owner.
- **election**: the lobby's choice of host from the latency matrix:
  the peer whose worst path to anyone is best.
- **Event**: a thing that happened in a tick; filtered per player by
  `event_visible`.
- **fog**: a player sees within `TANK_VISION` of its tank and its
  drones' vision; the window dims the rest.
- **game type**: Open, Tournament, Strict: what a tank starts with.
- **hash**: `World::hash`, FNV-1a over the bincode of the world;
  carried in heartbeats; the replay's checkpoints.
- **in flight**: an order sent and not yet seen applied in a snapshot.
- **interceptor**: the drone that orbits the tank and kills strikes.
- **lag queue**: `TREADCORE_LAG_MS`; the transport holds received
  datagrams that long.
- **link**: a drone's radio quality by distance from its tank; below
  `LINK_FLOOR` it comes home.
- **lobby**: discovery, probing, sharing rows, deciding; then a host
  and a route.
- **material**: what the squad builds with; harvested from forest.
- **mismatch**: a snapshot's tank differing from the predicted one
  with nothing in flight.
- **moat**: the Defender's first ring of mines round its depot.
- **mouth**: the gap in the Defender's wall ring, with a pillbox at it.
- **Observation**: what one player is told each tick.
- **Order**: a Command with its player, stamped by the host.
- **Pillboys**: the four-man squad aboard every tank; captures,
  repairs, builds, harvests.
- **pillbox**: a stationary gun; neutral ones on every spoke; shoots
  tanks within `PILLBOX_RANGE` and drones within `PILLBOX_AIR_RANGE`.
- **prediction**: the client's own tank run ahead of the snapshots.
- **prior art**: the 1987/1989 tank game this game descends from, and
  its later ports; SPEC-PRIOR-ART-1.14.md restates its manual.
- **proposal**: an offer of alliance terms (`Terms`), answered within
  `PROPOSAL_TICKS`, refused if it would break the three-way rule.
- **recon**: the drone that sees `RECON_VISION` and spots for the gun.
- **reconcile**: on a snapshot, replace the predicted tank with the
  host's, re-apply what is in flight, re-drive the ticks ahead.
- **relay**: a packet for a peer the sender cannot reach, through one
  it can (`Packet::Relay`, `RouteTable`).
- **Replay**: scenario, orders per tick, checkpoints; `verify` runs it.
- **RuleBot**: the attacking brain.
- **scenario**: everything that makes a match: seed, map, game type,
  players, teams, swarm, time limit.
- **scoot**: the RuleBot moving after it fires.
- **seat**: a player slot, 0..16, named for its start.
- **snapshot**: an observation on the wire, every other tick.
- **spoke**: a road from a side start to the centre; a neutral pillbox
  sits on each.
- **storm**: the RuleBot driving onto a depot it cannot shell clear.
- **strike**: the drone that hunts a tank; the swarm's kind.
- **swarm**: neutral strikes from the map edge every
  `swarm_interval`; owner `SWARM_OWNER`.
- **three-way rule**: `MIN_SIDES` 3; no alliance may leave fewer sides.
- **tick**: one step of the world, 1/20 s.
- **trace**: `build/logs/trace-STAMP.jsonl`, one event per line.
- **Welcome**: the host's reply to a Join: player, scenario, tick, map.

## 4. The documents

| file | what |
| --- | --- |
| documentation/CONCEPT.md | the brief, line by line; the prior art as the base; what the war changed |
| documentation/DESIGN.md | the rules and the numbers, and why; "what is not here yet" |
| documentation/PROTOCOL.md | the wire protocol, the election, the relay, prediction |
| documentation/ROADMAP.md | the steps in dependency order with Change / Must not change / Proof; the backlog |
| documentation/RESEARCH.md | what the game is modelled on, with sources |
| documentation/BUILD.md | the build on a small machine; `make warm` |
| documentation/MAPS.md | the map file format; converting prior-art maps |
| documentation/LAB-REPORT.md | the code, crate by crate, and the Rust technique in it |
| documentation/LAB-REPORT-PILLBOYS.md | the Pillboy requirement audited against the prior art's builder |
| documentation/SPEC-PRIOR-ART-1.14.md | the clean-room functional specification of the prior art, with the preface on what treadcore takes |
| documentation/RELEASE.md | the cycle, versions, releases, playtesting |
| documentation/GOVERNANCE.md | roles, joining, proposals (section 5: a change to rules.rs wants a trace), review, leaving |
| documentation/INDEX.md | this |
| journal/0001 to 0011 | opening day; proving the world; replay and hash; the Pillboys; material, walls, mines; map files; teams and alliances; proposals; the backlog; the Defender; prediction |

## 5. Keyword cross-reference

Where a name is defined and where it is used, as `grep -c -w` counts
per file at this commit (`treadcore-sim/lib.rs` means
`source/treadcore-sim/src/lib.rs`). The definition is first.

| keyword | files (count) |
| --- | --- |
| `World` | treadcore-sim/lib.rs (31), treadcore-brain/lib.rs (6), treadcore-net/predict.rs (4), treadcore-net/session.rs (4), treadcore-proto/lib.rs (2), treadcore-server/main.rs (4) |
| `Observation` | treadcore-sim/lib.rs (4), treadcore-brain/lib.rs (11), treadcore-net/session.rs (3), treadcore-proto/lib.rs (3), treadcore-net/predict.rs (2), treadcore-client/session.rs (1), treadcore-server/main.rs (1) |
| `Scenario` | treadcore-sim/lib.rs (25), treadcore-brain/lib.rs (5), treadcore-net/session.rs (5), treadcore-proto/lib.rs (4), treadcore-client/session.rs (2), treadcore-net/predict.rs (2), treadcore-server/main.rs (2) |
| `Command` | treadcore-sim/lib.rs (80), treadcore-brain/lib.rs (64), treadcore-client/input.rs (16), treadcore-net/session.rs (9), treadcore-net/predict.rs (5), treadcore-proto/lib.rs (5), treadcore-client/session.rs (3) |
| `Order` | treadcore-sim/lib.rs (70), treadcore-net/session.rs (4), treadcore-brain/lib.rs (2), treadcore-client/session.rs (1), treadcore-server/main.rs (1) |
| `Event` | treadcore-sim/lib.rs (112), treadcore-server/main.rs (61), treadcore-client/session.rs (45), treadcore-brain/lib.rs (14), treadcore-net/session.rs (7), treadcore-net/predict.rs (5), treadcore-proto/lib.rs (2) |
| `Tank` | treadcore-sim/lib.rs (13), treadcore-brain/lib.rs (2), treadcore-net/predict.rs (2), treadcore-net/session.rs (2), treadcore-client/session.rs (1) |
| `Drone` | treadcore-sim/lib.rs (10), treadcore-brain/lib.rs (1), treadcore-client/hud.rs (1) |
| `Shell` | treadcore-sim/lib.rs (7) |
| `Pillboys` | treadcore-sim/lib.rs (8), treadcore-client/hud.rs (5), treadcore-client/session.rs (4), treadcore-brain/lib.rs (1), treadcore-sim/rules.rs (1) |
| `Pillbox` | treadcore-sim/lib.rs (16), treadcore-brain/lib.rs (5), treadcore-client/input.rs (1), treadcore-client/render.rs (1) |
| `Depot` | treadcore-sim/lib.rs (5), treadcore-sim/map.rs (7), treadcore-brain/lib.rs (2), treadcore-client/render.rs (2) |
| `Mine` | treadcore-sim/lib.rs (18), treadcore-brain/lib.rs (2), treadcore-client/input.rs (1), treadcore-client/render.rs (1) |
| `Map` | treadcore-sim/map.rs (18), treadcore-sim/lib.rs (20), treadcore-brain/lib.rs (5), treadcore-net/predict.rs (3), treadcore-net/session.rs (2), treadcore-proto/lib.rs (2), treadcore-server/main.rs (2), treadcore-client/main.rs (1), treadcore-client/render.rs (1), treadcore-client/session.rs (1) |
| `Replay` | treadcore-sim/lib.rs (11), treadcore-net/session.rs (3), treadcore-server/main.rs (1) |
| `Packet` | treadcore-proto/lib.rs (29), treadcore-net/session.rs (39), treadcore-net/lobby.rs (15), treadcore-net/transport.rs (7) |
| `Socket` | treadcore-net/transport.rs (6), treadcore-net/session.rs (13), treadcore-client/session.rs (4), treadcore-net/lobby.rs (3), treadcore-server/main.rs (2), treadcore-net/lib.rs (1) |
| `Lobby` | treadcore-net/lobby.rs (4), treadcore-client/session.rs (4), treadcore-server/main.rs (2), treadcore-net/lib.rs (1) |
| `Host` | treadcore-net/session.rs (11), treadcore-server/main.rs (4), treadcore-client/session.rs (3), treadcore-net/lib.rs (2), treadcore-net/election.rs (1) |
| `Client` | treadcore-net/session.rs (6), treadcore-client/session.rs (6), treadcore-net/lib.rs (2), treadcore-server/main.rs (2), treadcore-net/predict.rs (1), treadcore-client/render.rs (1), treadcore-sim/lib.rs (1) |
| `Predictor` | treadcore-net/predict.rs (5), treadcore-net/session.rs (4), treadcore-client/session.rs (2), treadcore-client/main.rs (1), treadcore-net/lib.rs (1), treadcore-sim/lib.rs (1) |
| `Brain` | treadcore-brain/lib.rs (16), treadcore-sim/lib.rs (6), treadcore-server/main.rs (5), treadcore-client/session.rs (3), treadcore-sim/map.rs (1), treadcore-sim/rules.rs (1) |
| `RuleBot` | treadcore-brain/lib.rs (11), treadcore-server/main.rs (3), treadcore-client/session.rs (2), Makefile (1) |
| `Defender` | treadcore-brain/lib.rs (7), treadcore-server/main.rs (2), Makefile (1) |
| `JsonBrain` | treadcore-brain/lib.rs (8), treadcore-server/main.rs (2) |
| `Report`, `Trace` | treadcore-server/main.rs (6, 7) |
| `View` | treadcore-client/session.rs (10), treadcore-client/hud.rs (5), treadcore-client/render.rs (5), treadcore-client/input.rs (3) |
| `hash` | treadcore-sim/lib.rs (17), treadcore-net/session.rs (12), scripts/lobby-test.sh (8), treadcore-client/session.rs (4), treadcore-proto/lib.rs (4), treadcore-server/main.rs (4), treadcore-net/lib.rs (3), treadcore-client/hud.rs (2), scripts/replay.sh (1), Makefile (1) |
| `predict` | treadcore-sim/lib.rs (6), treadcore-net/predict.rs (5), treadcore-net/lib.rs (3), treadcore-client/hud.rs (2), treadcore-client/session.rs (2), treadcore-net/session.rs (1) |
| `drive_tank`, `route_for` | treadcore-sim/lib.rs (3, 3) |
| `observe` | treadcore-sim/lib.rs (14), treadcore-brain/lib.rs (2), treadcore-client/session.rs (1), treadcore-net/session.rs (1), treadcore-proto/lib.rs (1) |
| `event_visible` | treadcore-sim/lib.rs (2), treadcore-net/session.rs (2) |
| `snapshot` | treadcore-net/predict.rs (10), treadcore-net/session.rs (7), treadcore-proto/lib.rs (4), treadcore-client/session.rs (3), treadcore-sim/lib.rs (3), treadcore-client/render.rs (2), treadcore-server/main.rs (2), treadcore-sim/map.rs (1) |
| `Heartbeat` | treadcore-proto/lib.rs (2), treadcore-net/session.rs (4) |
| `Welcome` | treadcore-proto/lib.rs (8), treadcore-net/session.rs (4), treadcore-net/predict.rs (2), treadcore-sim/lib.rs (2), treadcore-net/transport.rs (1), scripts/lobby-test.sh (1) |
| `acked` | treadcore-proto/lib.rs (7), treadcore-net/predict.rs (4), treadcore-net/session.rs (2), treadcore-server/main.rs (1) |
| `bubble` | treadcore-sim/lib.rs (21), treadcore-brain/lib.rs (11), treadcore-client/render.rs (5), treadcore-client/hud.rs (4), treadcore-client/input.rs (2), treadcore-server/main.rs (2), treadcore-sim/rules.rs (1) |
| `swarm` | treadcore-server/main.rs (13), treadcore-sim/lib.rs (9), treadcore-client/main.rs (5), treadcore-client/session.rs (4), treadcore-brain/lib.rs (2), treadcore-net/predict.rs (1), treadcore-sim/rules.rs (1), scripts/lobby-test.sh (1) |
| `spotted` | treadcore-sim/lib.rs (18), treadcore-server/main.rs (4), treadcore-brain/lib.rs (3), treadcore-client/hud.rs (2), treadcore-sim/rules.rs (2), treadcore-client/session.rs (1) |
| `Propose`, `Terms` | treadcore-sim/lib.rs (9, 21), treadcore-brain/lib.rs (2, 2) |
| `GameType` | treadcore-sim/lib.rs (18), treadcore-server/main.rs (3) |
| `advantage` | treadcore-sim/lib.rs (5), treadcore-server/main.rs (7) |
| `TICK_HZ` | treadcore-sim/lib.rs (26), treadcore-sim/rules.rs (14), treadcore-brain/lib.rs (12), treadcore-server/main.rs (5), treadcore-client/main.rs (4), treadcore-client/hud.rs (2), treadcore-client/session.rs (2) |
| `LAG` | Makefile (2), scripts/lobby-test.sh (2), treadcore-net/transport.rs (1), treadcore-server/main.rs (1) |

To refresh the table: `grep -c -w NAME` over `source/*/src/*.rs`,
`scripts/*`, `Makefile`.
