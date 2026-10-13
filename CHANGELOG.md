# Changelog

What changed, by date. For what happened, in order, and why, see the
creation journal in `journal/`: one entry per working session, in a
fixed form (`journal/TEMPLATE.txt`).

## 2026-10-13, later still: sub-cubes, damage that chips, cubes that roll off

Journal: [0017](journal/0017-2026-10-13-sub-cubes.txt).

- Changed: the models' cubes are a tenth of a tile (the things that
  move) and a quarter (the ground's blocks and pits), with hidden top
  and side faces culled: a tank is 160 cubes with treads, a
  superstructure, a turret and a barrel; a pillbox a four-step
  pyramid; a depot a plate with corner posts; a wall four by four by
  three on its tile; a crater four by four, two deep, its far wall in
  shadow. The hull is Bolo's olive, the turret the seat's colour.
- New: damage chips a model (`Model::chip`): the tank's turret as its
  armor goes, in tenths; the pillbox's gun and top steps as its
  health goes, in eighths.
- New: debris. Every impact throws loose cubes that fly, bounce,
  roll to a stop, spin and fade: earth from the ground, tan from a
  wall or pillbox, olive from a hull, the seat's colour from a
  destroyed tank (`View.impacts`, `render::debris`).
- Changed: the frame's boxes are bevelled, sunken for the insets and
  the map, raised for the order tiles; the bezel is lighter. Solo
  play's map now takes craters and walls from events, as a client's
  always did (it never showed a crater before). Fog dims less.
- Fixed: the newswire reported any visible tank's hit as "tank hit";
  it now says whose.

## 2026-10-13, late: Bolo's colours, and assets out of cubes

Journal: [0016](journal/0016-2026-10-13-cubes.txt).

- Changed: the ground is Bolo's palette (`render.rs`): dark green
  grass, bright checkered forest, grey road with its dashes, cyan
  water, tan bridges and walls, brown rubble.
- New: `voxel.rs`. Every thing on the map is a model of cubes on a
  grid of fifths of a tile, drawn in an oblique projection with
  sprites: a top face lit by its height, a shaded side face below.
  The tank (hull, tracks, turret, barrel), the pillbox (a tan pyramid
  with an orange gun, the gun gone under half health, a dark slab
  when dead), the depot (a tan plate with the owner's mark), the
  drones, shells, mines, the squad, and walls as tan blocks raised
  and dropped as the terrain changes. Models are rebuilt only when
  their key changes. No images; two rectangles a cube.
- New: two frames in the gallery (playtest-15, 16).

## 2026-10-13, night: the bezel: the frame in WinBolo's look

Journal: [0015](journal/0015-2026-10-13-the-bezel.txt).

- Changed: the frame wears the prior art's look (`INTERFACE.md`
  section 7), drawn with UI nodes and no images: a grey bezel of
  inset black boxes; a lamp per tank, per pillbox and per depot in
  the owner's colour (mine in my seat colour, green ally, red hostile,
  dim neutral, dark dead); shells as red squares, the bay by kind, the
  squad as four lamps that go amber when out, material as eight,
  armor and fuel as bars; a pictogram tile and a lamp beside every
  order, red when it can be given; kills and deaths; the newswire in
  a 22 px face across the bottom. The top strip holds two lines.
- New: `View.kills` and `View.deaths`, counted from `TankDestroyed`.
- Changed: the gallery's frame pictures are the bezel (playtest-10 to
  13), plus the scrollback (14).

## 2026-10-13, evening: the frame (the GUI redesign), and the playability review

Journal: [0014](journal/0014-2026-10-13-the-frame.txt).

- Changed: the window's HUD is the prior art's frame
  (`INTERFACE.md`, SPEC section 13): a top strip (who, when, NET), an
  orders column on the left with every key, its count or cost and a
  dash when it cannot be given, the map in the middle never covered,
  the roster with one lamp per pillbox and per depot on the right,
  and shells, bay, squad and material as lamps, armor and fuel bars,
  the air table and a two-line newswire along the bottom. The
  controls overlay is gone; Tab shows a forty-line events scrollback
  over the map. Lamps are ASCII: `#` lit, `.` dark; `#` mine, `+`
  ally, `o` neutral, `X` hostile, `.` dead.
- New: the lobby screen (the peers found with their measured round
  trips, how the host is chosen, the first minute in six lines) and
  the match-over screen (the roster by score, Esc to leave).
- New: the gun's envelope lights while the right button is held and
  the shot goes on release; the direct-fire ring with it.
- New: the pillbox view (SPEC UI-10 to UI-13, backlog 12): the
  apostrophe steps through the player's and allies' pillboxes, Return
  or F returns to the tank. The camera follows the tank into the
  middle of the map area, not the window; clicks on a panel are not
  orders.
- New: `--seconds N` on the window. Events after a rejoin are stamped
  with the last tick seen, not 0.
- New: `documentation/PLAYABILITY.md`, the review: fourteen items
  between this build and a playable game, ranked and costed; Bolo's
  interface elements checked off; what the frame still wants.
- New: four frames of the frame in the gallery (playtest-10 to 13).

## 2026-10-13, later: the first playtest build, with pictures

Journal: [0013](journal/0013-2026-10-13-first-playtest.txt). Report:
[LAB-REPORT-PLAYTEST.md](documentation/LAB-REPORT-PLAYTEST.md).

- New: the window runs in the VM. `mesa-vulkan-swrast` (lavapipe) is
  the driver; copal's full monty installs it now. `scripts/bigstack.c`,
  built by `make configure` on a musl machine and preloaded by `make
  run` and `make lan`, raises the default thread stack to 8 MiB:
  lavapipe compiles shaders with LLVM on its own thread, musl gives it
  128 KiB, and every Bevy binary segfaulted in
  `llvm::AsmPrinter::doFinalization` before its first frame.
  `configure` reports the Vulkan drivers it finds, and `PRELOAD` in
  config.env.
- Fixed: the HUD's rtt was heartbeat age (200 to 450 ms on a 60 ms
  link): the client echoed the host tick from the last heartbeat; it
  now echoes the newest snapshot tick, and reads 50 to 150 ms.
- New: nine frames in `reports/images/`, a gallery on the front page,
  the report link under Reports.
- New: `documentation/INTERFACE.md`, the layout study: Bolo's mode
  column, lamp grids, pillbox strip and newswire; the garage's
  between-screens; the HUD graphics' centred rings; and treadcore's
  layout drawn from them, in five steps.
- Found, not fixed: the two top HUD panels overlap below about
  700 px; the controls overlay covers the map until Tab; the event
  log stamps tick 0 after a rejoin until the first snapshot; a seam
  at the NET panel's edge under Xwayland at a guessed 1.083 scale; a
  standing human dies to fifteen bots in about thirty seconds.

## 2026-10-13, night: across the Internet (roadmap step 7)

Journal: [0012](journal/0012-2026-10-13-across-the-internet.txt).

- New: the introducer (`treadcore-net/src/introducer.rs`;
  `treadcore-server --introduce`, `make introduce`): a UDP rendezvous
  service that hosts nothing and keeps a registration thirty seconds.
  Peers in one lobby name register (`--introducer HOST:PORT`;
  `--no-beacon` keeps a peer off the LAN broadcast) and are told each
  other's public and local addresses; the lobby greets both until one
  answers (the hole punch) and takes the answering address. The
  election and the relay are untouched: a pair that never gets
  through is routed around, as on a LAN.
- New: `--identity FILE`, a peer id that survives a restart (made the
  first time; `build/identity` by default from `make lan|host|seat`).
- New: rejoin (backlog item 10, SPEC GAM-12). A host keeps registering
  with `hosting` set; a returning peer with the same identity is told
  its host, pings it until it answers, joins without an election, and
  gets its seat back at the tick the match has reached, with its
  sequence numbers started over. `Join` carries a `session` number,
  random per process, so a restart is told from a repeated Join.
- Changed: wire version 11 (`Register`, `Introduce`, `Introduced`,
  `Join { session }`).
- Changed: `make lobby-test` takes `INTRODUCER=1` (no broadcast, no
  seed address: the four meet through an introducer alone) and
  `REJOIN=1` (a seat that is not hosting killed outright at
  `REJOIN_AFTER` seconds, 20, and started again with its identity
  file; must come back to the same seat at a later tick).
- Changed: the predictor's first snapshot after a Welcome is a
  baseline, not a check: a seat that comes back mid-match has a copy
  built from the scenario's start, and the host's tank is not there.
- Fixed: a terrain change (a shell's crater, a wall) landing under
  ticks the client had already predicted was a mismatch of a fifth of
  a tile, one in some 2 400 checks. The predictor now keeps the last
  snapshot's tank as an anchor and, on such an event, drives the ticks
  ahead again: over the old map up to the event's tick, the new one
  after. `Predictor::events` takes the events' tick.
- Proof: `introducer_brings_two_silent_peers_together`,
  `seat_comes_back_after_a_restart`, `identity_file_round_trips`;
  `make lobby-test` plain, `INTRODUCER=1`, `REJOIN=1` and `LAG=200
  REJOIN=1` all PASS with zero prediction mismatches, the killed seat
  back in its own seat at tick 371 and 376. Not proven: two peers
  behind two home routers, which needs two machines on two networks
  (ROADMAP step 7 says so).
  40 tests (sim 24, proto 3, net 9, brain 4).

## 2026-10-13, evening: client-side prediction (roadmap step 4)

Journal: [0011](journal/0011-2026-10-13-prediction.txt).

- New: a client runs its own tank ahead of the snapshots
  (`treadcore-net/src/predict.rs`). A partial `World` from the
  `Welcome`, kept up from every snapshot and terrain event; hull
  orders (`Move`, `Stop`, the two fire orders) applied the moment they
  are sent; one tick at the tick rate by `World::predict`, which calls
  the same `drive_tank` the host does; every snapshot replaces the
  tank, re-applies the orders after the snapshot's `acked`, and drives
  the ticks ahead again. Bit-for-bit check against the host whenever
  nothing is in flight; mismatches logged and counted.
- Changed: wire version 10. `Snapshot` carries `acked`, the last of
  the client's command seqs the host had applied by that tick.
- New: `TREADCORE_LAG_MS` holds every received datagram that long in
  the transport: the stand-in for `tc netem` where there is no root.
  `make lobby-test LAG=200`, which now also requires zero prediction
  mismatches in the three client seats.
- Changed: the window draws the predicted tank in place of the
  snapshot's own tank; the NET panel shows `predict +N ticks`, orders
  in flight, checks, mismatches, and CORRECTED for two seconds after
  a correction. The headless `--net` seat predicts too, and logs one
  `prediction:` line at the end.
- Proof: `prediction_hides_the_lag_and_matches_the_host` (100 ms each
  way: the predicted tank moves 250 ms before the snapshot shows it,
  53 checks, 0 mismatches); `LAG=200 make lobby-test` (1 602 checks,
  0 mismatches); the 6d replay still matches 60 of 60 after the
  `step_tanks` refactor. 37 tests.

## 2026-10-13, later: the Defender and the storming attacker (roadmap step 6d)

Journal: [0010](journal/0010-2026-10-13-the-defender.txt).

- Changed: `Fortifier` is now `Defender`, roadmap 6d's doctrine: at
  its depot for good; a recon on the approach, moved by `Retask` to
  the depot of whoever shot it last; counter-battery; a sidestep when
  hit; the squad in order: capture the spoke's pillbox, repair its
  own, the moat, a pillbox at the mouth of the walls, the road to it,
  mines across the approach, the walls. It proposes to recruit its
  nearest neighbour once a minute and accepts proposals from anyone
  who has never hurt it: the first bots to use step 6b.
- Changed: the `RuleBot` attacker keeps its recon ten tiles short of
  what it watches (one over a defended depot was intercepted in eight
  seconds), counts blind as low, recalls its squad and moves when hit,
  and storms a depot whose defender has been in view for twice its
  patience: the guns first, then the depot.
- New: `Scenario.advantage` and `--advantage` (the prior art's BRN-12): bot
  seats are told where every tank is.
- Proof, `defenders_hold`: ten seeds of sixteen bots, four attacking:
  every match to the time limit; attackers alive 2 of 40, defenders
  84 of 120; the defenders keep 101 of 120 depots. The kills are 65
  by shell, 7 by mine, 1 by pillbox: the roadmap's "most kills are
  pillbox kills" did not come true, and ROADMAP 6d says why.
  `fortification_pays` widens to 144 000 against 134 206 ticks.
- Tests: 35 pass (sim 23, proto 3, net 5, brain 4).

## 2026-10-13: the backlog, worked through

Journal: [0009](journal/0009-2026-10-13-the-backlog.txt).

- Changed: interceptors engage strikes only (and a recon inside the
  bubble), never other interceptors. Eight seeds of sixteen bots: kills
  by shell to kills by strike go from 7.0 : 5.9 to 8.8 : 2.9 a match,
  interceptors shot down from 32 to 6, survivors from 2.8 to 4.1
  (backlog 1, and 7 with it).
- New: `Command::Retask { drone, task }` (a recon's orbit, a strike's
  target, one drone's recall) and `DroneRetasked`; the rule bot moves
  its recon instead of doubling it (backlog 3).
- New: The prior art's forest concealment, VIS-02 and PIL-07: a tank in a 2 x 2
  block of forest is unseen and untargeted beyond `CONCEAL_RANGE` (3)
  (backlog 13).
- New: `Scenario.game_type`, the prior art's open / tournament / strict as what
  a tank starts with; `--game-type`, `GAMETYPE=` (backlog 11).
- New: the match report at the end of every match and replay: shells
  by kind and spotted, intercept rate, strikes through, kills by cause,
  first kill, pillbox shots, drones lost by reason, refused orders
  (backlog 8).
- Changed: `Heartbeat` carries the host's `rtt_ticks` back; the HUD's
  rtt is that. Wire `VERSION` 9 (backlog 5).
- Changed: the rule bot's led shot is aimed at the tank when the lead
  falls inside minimum range; the Fortifier waits with no material and
  no forest in sight (backlog 4).
- Decided, no change: tank speed stays 4 (backlog 2); depots stay
  time-gated, not shell-gated (backlog 9). Deferred: rejoin to steps 7
  and 8 (10); the pillbox view to the first working window (12);
  pillboxes that never fire to step 6d (14).
- Tests: 34 pass (sim 23, proto 3, net 5, brain 3).

## 2026-10-12, night: proposals and the three-way rule (roadmap step 6b)

Journal: [0008](journal/0008-2026-10-12-proposals.txt).

- New: `Command::Propose { to, terms }` with `Terms::Join | Recruit |
  Swap | Merge`, `Command::Answer { proposal, accept }`; a `Proposal`
  stands 30 s; the three-way rule (`MIN_SIDES` 3, no side over half
  the living tanks) refuses an outcome, with the reason, when proposed
  and when accepted; `Proposed` and `ProposalResolved` events, the
  latter to everyone. The prior art's alliance commands are sugar over this.
- Changed: `AllianceRequest` and its two events gone; the observation
  carries `proposals`. Wire `VERSION` 8.
- Proof: a recorded match in which two sides of eight refuse every
  proposal, a defection among three sides is carried out and shares
  recon, recruiting past half is refused and a swap is allowed. 31
  tests pass; replay and lobby-test pass.

## 2026-10-12, night: teams and alliances (roadmap step 6)


Journal: [0007](journal/0007-2026-10-12-teams-and-alliances.txt).

- New: `Player.team`; `Scenario.teams` and `with_teams(n)`; `--teams N`
  on server and window, `TEAMS=n make debug`. A team shares sensors
  (an ally's recon spots for your gun), depots, pillbox shells and
  the knowledge of its mines; friendly fire is off within a team for
  shells, strikes, splash and mines (`FriendlyFireSpared` in the
  trace; one's own shells still hurt); interceptors and pillboxes
  leave allies alone; a strike at an ally is refused. `Command::Chat {
  text, to: All | Team }`. The match ends when one team stands;
  `MatchOver { winner, team }`.
- New: The prior art's alliance protocol in play, `Command::Alliance(Request |
  Accept | Decline | Leave)`: a request stands 30 s, any member of the
  alliance asked may accept, leave at will; all events. The
  observation carries every player's team and the requests a player
  may answer; the HUD scoreboard is by team.
- Changed: the net host filters events by the world's rule
  (`World::event_visible`) instead of its own copy. Wire `VERSION` 7.
- Proof: a shell fired at what only an ally's recon sees is spotted; a
  shell landing on a teammate does no damage and says so; alliances
  form, admit, refuse, lapse and break; the match ends by team. 30
  tests; four teams of four traced (61 of 79 shells spotted).

## 2026-10-12, night: map files (roadmap step 5), five prior-art maps


Journal: [0006](journal/0006-2026-10-12-map-files.txt).

- New: the map as a text file (`documentation/MAPS.md`): a format
  line, `size`, `seed`, one glyph per tile, then `start`, `depot x y
  [seat|neutral]` and `pillbox x y [seat|neutral]` lines. `Map::to_text`
  and `Map::from_text` (refuses with the line; repairs an owner that is
  no seat); `World::map_as_played`. `Scenario.map: MapSource` (the
  scenario's seed, another seed, or a file's text carried whole).
  Server `--map`, `--save-map`, `--write-map`; `make map`; `MAP=file`
  on debug, run and host; the window's `--map`.
- New: the world is built at the size the map declares. The generator
  still makes 128 x 128 (bit-identical to day one); the prior art's maps are
  256 x 256. `Observation` carries `map_w`, `map_h`; swarm edges, the
  brains and the client take the size from the map.
- New: `Packet::Chunk` for anything too big for a datagram; `Welcome`
  (now the whole `Map`, up to 130 KiB with a map file) goes in pieces
  (backlog item 6 done). Wire `VERSION` 6.
- New: `scripts/priorart2map.py` and `make convert FROM=file` turn a
  prior-art map (its binary format, v1) into a treadcore map; five
  community maps converted into `maps/prior-art/` (provenance and the
  licence question in `vendor/THIRD-PARTY.md`); `maps/island.map` is
  the generated default written out.
- Changed: the RuleBot remembers what the world refused as
  unreachable (a squad's site, a tank's destination) instead of
  re-ordering it four times a second, which island maps exposed.
- Proof: a generated map round-trips through text to the bit;
  sixteen bots play a converted prior-art map, on which pillboxes fire in
  bot play for the first time. 28 tests pass; replay and lobby-test
  pass.

## 2026-10-12, night: material, walls, mines, roads (roadmap step 6c)


Journal: [0005](journal/0005-2026-10-12-material-walls-mines.txt).

- New: `Terrain::Wall` (impassable, blocks a flat shot and a pillbox's
  line, a splash knocks it to rubble); `Mine` entities (hidden until
  found within 1.5 tiles; set off by a tank or squad on the tile, a
  shell's splash or an orthogonal neighbour; 50 to the tank on it, 20
  within 1.5, a squad in it lost, the tile cratered; drones untouched;
  a tank's own mines never hurt it); `Build { Wall | Road | Mine |
  Pillbox }` and `Harvest { at }` (forest to grass, +4 material);
  roads become bridges over water; material cap 8; pillboxes hold 8
  shells for their owner's tank. `Event::TerrainChanged`; the
  Observation carries known mines and the eight nearest forest tiles.
  Routes avoid known hostile mines. Keys 7/8/9/0.
- New: `Fortifier`, the minimal defender (harvest, a gateless mine
  ring at radius 3 round the home depot, a wall ring at 4 with a gate
  toward the centre; learns from its own refusals). `--defenders N`,
  `DEFENDERS=n make debug`.
- Changed: wire `VERSION` 5.
- Proof (`fortification_pays`, twenty seeds, scripted attacker): the
  fortified defender survived to tick 137 664 in total against
  94 885 bare, lived 18:5, killed the attacker 20:12, built 431:0.
  24 tests pass; replay 36/36; lobby-test passes.

## 2026-10-12, night: the Pillboys (roadmap step 3)


Journal: [0004](journal/0004-2026-10-12-the-pillboys.txt).

- New: the squad. `Pillboys` on every tank (strength 4, material 2,
  aboard or out); `Command::Deploy { task }` with `Capture`, `Repair`,
  `Build { Pillbox, at }` and `Return`; the squad walks the world's
  route at 1.5 tiles/s, captures in 10 s, repairs in 6 for one
  material, builds in 15 for two, walks home; a shell's splash over it
  costs two men; depots restock men and material. HUD line, map figure,
  keys 4/5/6.
- New: pillboxes. Twenty neutral ones on the map (one per spoke, four
  round the centre); 60 health; a hitscan gun at hostile tanks within
  8 tiles with a clear line, 15 damage every 2 s; an interceptor's odds
  against hostile drones within 4; dead at zero, never removed;
  capturable when neutral or dead; all visible to everyone. Shells
  damage them; direct fire hits one when no tank is there. Range rings
  and capture progress drawn; narrated in the trace.
- Changed: a tank's route avoids the range of live hostile pillboxes
  (`route_for`, `PILLBOX_AVOID_COST`), because the spoke roads run
  straight through them. Wire `VERSION` 4. The rule bot sends its
  squad from a standing tank with no enemy in view and holds while it
  is out; `RuleBot::without_pillboys` is the control.
- Proof (`pillboys_pay`, twenty seeds, eight bots): squads win 18:2,
  score 20 945 to 13 655, survive 37 to 32, 248 captures. 24 tests
  pass; the replay of the new default match matches 36 checkpoints;
  lobby-test passes.
- Known: in the default bot trace no pillbox fires at a tank (the
  router keeps tanks out); backlog item 14.

## 2026-10-12, night: replay and world hash (roadmap steps 1 and 2)


Journal: [0003](journal/0003-2026-10-12-replay-and-hash.txt).

- New: `World::hash()` (FNV-1a over the serialised world) and
  `Replay` (scenario, every tick's orders, a hash checkpoint every
  100 ticks) in `treadcore-sim`, with `World::run` and
  `Replay::verify`. The host records on request; the server has
  `--record FILE` and `--replay FILE` (exit 1 on a mismatch). `make
  debug` writes `build/logs/replay-STAMP.json`; `make replay` checks
  the newest, or `REPLAY=file`. The host hashes every 1 000 ticks, logs
  it and carries it in `Heartbeat`; clients log it; the HUD shows it.
- Changed: wire `VERSION` 3 (`Heartbeat` gained `hash_tick`, `hash`).
  `make lobby-test` asserts one world hash at tick 1 000 in all four
  logs.
- Measured: a sixteen-bot, 180-second match records to 43 KB and
  replays to all 36 checkpoints in 0.2 s; with the indirect shells
  removed it fails at tick 100. 20 tests pass.

## 2026-10-12, evening: proven, fixed, sixteen


Journal: [0002](journal/0002-2026-10-12-proving-the-world.txt).

- Proven: every crate compiles (the client against Bevy 0.17.3 with
  three dead-code warnings); 16 tests pass; `make debug` traces a match
  with indirect fire, recon and the bubble all in it; `make lobby-test`
  passes with a real assertion (one host, three clients welcomed, all
  three saw the result). The window does not open in the development
  VM: no Vulkan driver (`BUILD.md`, "The window in this VM").
- Fixed: `net` UDP receive buffer 2 KiB -> 64 KiB (the 16.5 KiB `Welcome`
  was truncated and no client ever joined; the loopback test found it).
  `sim` `Move` now routes with the map's A*, the tank follows the
  waypoints, brakes to pivot at corners, and re-plans if blocked (a bot
  sat at the river bank for a whole match). `build` `SECONDS` renamed
  `GAME_SECONDS` (bash's own counter made the match as long as the
  build took, plus 180). `server` reads the final snapshot before
  judging a lost connection; the host waits a second before `Leave`.
- Changed: sixteen seats (`MAX_PLAYERS` 16): sixteen starts round the
  edge, a depot each, a ring road; `--players N` and `PLAYERS=n`;
  sixteen bots is the default. The wire is `VERSION` 2 (`Tank` gained
  `route`). The rule bot's doctrine rewritten: firing position short of
  the objective, recon over it from a standing tank, indirect fire with
  lead, patience drawn per bot, advance to capture.
- New: `GOVERNANCE.md` (roles, application, entry, proposal,
  agreement, review, removal, resignation, ceremony, recognition,
  assessment, award, annual processes; the organization on GitHub),
  `RELEASE.md` (the implementation cycle, versions, release monuments,
  candidates confirmed by playtest), `CONTRIBUTING.md`,
  `CONTRIBUTORS.md`, `.github/` templates and `CODEOWNERS`.
  `SPEC-PRIOR-ART-1.14.md`: the clean-room functional specification of
  the prior art, version 1.14 (256 requirements), with a preface on what treadcore
  adopts, adapts and leaves; the roadmap's prior-art-derived steps and
  backlog cite its IDs.
  `LAB-REPORT-PILLBOYS.md`: the Pillboy requirement checked against
  the prior art's builder; `DESIGN.md` gained the Pillboys decision; `CONCEPT.md`

  section 9: sixteen tanks, defenders, alliances, betrayal; `ROADMAP.md`
  gained a backlog and steps 0b, 6b, 6c, 6d.
- Known: the release-profile engine (`make warm`, `make run`) not yet
  built; the window unproven here; two-machine LAN untested; the
  backlog's balance items (interceptors fighting interceptors, tank
  speed, drone retasking) open.

## 2026-10-12: opening day


Journal: [0001](journal/0001-2026-10-12-opening-day.txt).

- New: six-crate Rust workspace. `treadcore-sim` (deterministic world;
  compiled, 6 tests pass), `treadcore-proto`, `treadcore-net` (LAN
  lobby, host election, relay, Host/Client), `treadcore-brain`
  (RuleBot, JsonBrain), `treadcore-server` (headless host, traced bot
  matches), `treadcore-client` (Bevy 0.17 window with telemetry
  overlay), `treadcore-warm` (engine prerequisites).
- New: Makefile over `scripts/` over cargo; `make configure`, `warm`,
  `build`, `headless`, `debug`, `run`, `lan`, `host`, `seat`,
  `lobby-test`, `test`, `schema`.
- New: documentation (concept, design, protocol, research, lab
  report, build), vendor policy and third-party notices, site at the
  root for GitHub Pages, first report.
- Changed: build sized for a 6 GB machine after the engine build ran
  out of memory on bevy_ui (jobs from RAM, trimmed Bevy features, no
  dependency debug info, no release LTO).
- Known: crates above the simulation not yet compiled; `make warm` not
  yet run to completion.

Commits: 17c71d3, 3a26432, 49cd8f9.
