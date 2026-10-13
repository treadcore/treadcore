# Changelog

What changed, by date. For what happened, in order, and why, see the
creation journal in `journal/`: one entry per working session, in a
fixed form (`journal/TEMPLATE.txt`).

## 2026-10-12, night: proposals and the three-way rule (roadmap step 6b)

Journal: [0008](journal/0008-2026-10-12-proposals.txt).

- New: `Command::Propose { to, terms }` with `Terms::Join | Recruit |
  Swap | Merge`, `Command::Answer { proposal, accept }`; a `Proposal`
  stands 30 s; the three-way rule (`MIN_SIDES` 3, no side over half
  the living tanks) refuses an outcome, with the reason, when proposed
  and when accepted; `Proposed` and `ProposalResolved` events, the
  latter to everyone. Bolo's alliance commands are sugar over this.
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
- New: Bolo's alliance protocol in play, `Command::Alliance(Request |
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

## 2026-10-12, night: map files (roadmap step 5), five Bolo maps


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
  still makes 128 x 128 (bit-identical to day one); Bolo's maps are
  256 x 256. `Observation` carries `map_w`, `map_h`; swarm edges, the
  brains and the client take the size from the map.
- New: `Packet::Chunk` for anything too big for a datagram; `Welcome`
  (now the whole `Map`, up to 130 KiB with a map file) goes in pieces
  (backlog item 6 done). Wire `VERSION` 6.
- New: `scripts/bolo2map.py` and `make convert BOLO=file` turn a
  Macintosh Bolo map (`BMAPBOLO` v1) into a treadcore map; five
  community maps converted into `maps/bolo/` (provenance and the
  licence question in `vendor/THIRD-PARTY.md`); `maps/island.map` is
  the generated default written out.
- Changed: the RuleBot remembers what the world refused as
  unreachable (a squad's site, a tank's destination) instead of
  re-ordering it four times a second, which island maps exposed.
- Proof: a generated map round-trips through text to the bit;
  sixteen bots play a converted Bolo map, on which pillboxes fire in
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
  `SPEC-WINBOLO-1.14.md`: the clean-room functional specification of
  WinBolo 1.14 (256 requirements), with a preface on what treadcore
  adopts, adapts and leaves; the roadmap's Bolo-derived steps and
  backlog cite its IDs.
  `LAB-REPORT-PILLBOYS.md`: the Pillboy requirement checked against
  Bolo's builder; `DESIGN.md` gained the Pillboys decision; `CONCEPT.md`

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
