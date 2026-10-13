# treadcore

A sixteen-player overhead tank game for the battlefield the tank was just
handed: the gun fires indirect like artillery, and the tank keeps itself
alive the way a carrier does, by owning the airspace around it with a
drone crew flown off its own hull.

Rust, Bevy for the window, plain `std` UDP for the network. The
clients find each other on the LAN, measure the links between them, and
elect a host by themselves: the one whose worst link is best, with
anyone the host cannot reach routed through a neighbour that can.

Website: [treadcore.org](https://treadcore.org). Repository:
[github.com/treadcore/treadcore](https://github.com/treadcore/treadcore).

## The game

Up to sixteen players, one tank each, on a 128 x 128 tile island (one
tile is 25 m); bots fill the seats people do not. The tank carries a
bay of drones:

| drone | job | the number that matters |
| --- | --- | --- |
| recon | orbits a point, sees 12 tiles, feeds telemetry back over a 40-tile link | a shell at a target it can see lands within 0.6 tiles; blind, within 4 |
| interceptor | holds station on a ring around the tank (the *bubble*), rams anything that flies in | kills 90 % of the time on a good link, 50 % on a dead one; one shot each |
| strike | the FPV: flies to an enemy tank and dives | 35 of 100 armor, if it gets through the bubble |

The gun lobs high-calibre rounds at map coordinates between 12 and 60
tiles out. Six seconds of flight at full range. Seventy damage in a
2.5-tile splash, and the splash takes down every drone over it: the
bubble does nothing to a shell, and a shell does plenty to the bubble.
There is also a direct-fire mode for anything that drives into line of
sight inside 10 tiles.

Depots resupply and repair. Twenty neutral pillboxes start on the map
and shoot anyone within 8 tiles; the tank's Pillboys, a squad of four
that rides with it, capture them (10 seconds standing on one that is
neutral or dead), repair them and build new ones, and a pillbox you own
shoots tanks and drones for you and holds shells for your tank. The
squad also fells forest for material and builds walls, roads, bridges
and mines with it: a mine is hidden until found, set off by a tank, a
shell or a neighbouring mine, and ignored by drones. Neutral FPV swarms come in off the map
edge every 45 seconds, so the bubble matters even alone. Last tank
standing wins; at the time limit, the highest score.


The player's one dial is the bubble radius: wide catches strikes early
but thins the ring and stretches every link.

## Building

```
make configure   # toolchain, display libraries, a RAM-sized job count; writes build/config.env
make warm        # the engine alone, once, gently (see documentation/BUILD.md)
make build       # release, every crate; after warm, only the game
make headless    # everything except the window
make test        # cargo test, every crate
```

Needs Rust 1.85+. The window needs `wayland-client`, `xkbcommon`, `alsa`
and `libudev` dev packages (on Alpine: `wayland-dev libxkbcommon-dev
alsa-lib-dev eudev-dev`). Without them the headless crates still build.

## Running

```
make run         # the window, solo against fifteen bots (PLAYERS=4 for three)
make lan         # the window, LAN lobby; WANT=2 to start with two
make host        # headless host on the LAN, bots in empty seats
make seat        # headless bot that joins a LAN lobby
make lobby-test  # four headless peers on this machine elect a host
```

Controls are on screen (Tab hides them). Left click drives (the world
routes around water, over bridges and round hostile pillboxes), right
click fires, 1/2/3 launch interceptor / recon / strike, R recalls, 4
sends the Pillboys to capture or repair the pillbox under the cursor,
5 to build one there, 6 brings them home, 7/8/9 wall, mine and road
(a bridge on water) at the cursor, 0 harvests the forest there, [ ]
set the bubble.


## The debug trace

```
make debug                      # 3 game minutes of sixteen bots, flat out
GAME_SECONDS=600 SEED=0x1234 PLAYERS=4 make debug
DEFENDERS=12 make debug         # twelve dig in at their depots, four attack
TEAMS=4 make debug              # four teams of four; allies share recon, spare each other

MAP=maps/quarters.map make debug   # on a map file; make map writes one to edit


BRAIN='python3 mybot.py' make debug   # an external brain in seat 0
```

Writes `build/logs/debug-STAMP.log` (every log level, every event,
per-second state of every tank), `build/logs/trace-STAMP.jsonl` (one
JSON event per line) and `build/logs/replay-STAMP.json` (the scenario,
every tick's orders, the world hash every hundred ticks). The log is
written to be read, by a person or by an AI with the source beside it:
`[tick seconds] who did what`.

```
make replay                     # play the newest replay back; every hash must match
make replay REPLAY=build/logs/replay-STAMP.json
```

A replay is the regression test for a change to the rules: it ends
with every checkpoint matched, or names the first tick where the world
differs and exits 1.


## Brains

A Brain is anything that takes an observation and returns commands. The
observation is the player's view, filtered by the host; a Brain cannot
see or do anything a keyboard cannot. `RuleBot` fills empty seats.
`JsonBrain` runs an external program: one JSON observation per line on
its stdin, one JSON array of commands per line on its stdout. `make
schema` prints an example of each. See `source/treadcore-brain`.

## Layout

```
source/treadcore-sim      the world: map, tanks, drones, shells. Headless, deterministic.
source/treadcore-proto    the packets: commands, snapshots, events, negotiation.
source/treadcore-net      UDP, LAN discovery, RTT matrix, host election, relay, Host/Client.
source/treadcore-brain    the Brain trait, RuleBot, JsonBrain.
source/treadcore-server   headless: bot matches, LAN host, LAN bot seat.
source/treadcore-client   the Bevy window.
scripts/                  what the Makefile calls.
maps/                     map files (documentation/MAPS.md); make map writes a new one;
                          maps/bolo/ holds five converted Macintosh Bolo maps.


vendor/                   external code and blobs vendored by the author, with their licences.
documentation/            concept, design, protocol, research notes, the lab report.
index.html, CNAME         the site at treadcore.org (GitHub Pages, served from the root).
reports/                  dated web reports, served by the site.
journal/                  the creation journal: one plain-text entry per session (see CHANGELOG.md).
```

## Documents

- [documentation/CONCEPT.md](documentation/CONCEPT.md): the brief, reviewed; how the design improves on the base concept.
- [documentation/DESIGN.md](documentation/DESIGN.md): the rules, the numbers, and why.
- [documentation/PROTOCOL.md](documentation/PROTOCOL.md): the wire protocol and the host election.
- [documentation/RESEARCH.md](documentation/RESEARCH.md): what the game is modelled on, with sources.
- [documentation/ROADMAP.md](documentation/ROADMAP.md): the next architectural steps, in dependency order.
- [documentation/BUILD.md](documentation/BUILD.md): how the build is designed for a small machine; `make warm`.
- [documentation/LAB-REPORT.md](documentation/LAB-REPORT.md): a walk through the code, crate by crate, and the Rust technique in it.
- [documentation/LAB-REPORT-PILLBOYS.md](documentation/LAB-REPORT-PILLBOYS.md): the Pillboy requirement checked against Bolo's builder, and what was decided.
- [documentation/SPEC-WINBOLO-1.14.md](documentation/SPEC-WINBOLO-1.14.md): the clean-room functional specification of WinBolo 1.14 that the Bolo-derived steps cite, with a preface on what treadcore adopts.
- [documentation/MAPS.md](documentation/MAPS.md): the map file format; any text editor is the editor.

- [documentation/RELEASE.md](documentation/RELEASE.md): the implementation cycle, versions, release monuments, candidates and playtesting.
- [documentation/GOVERNANCE.md](documentation/GOVERNANCE.md): how the organization is run: roles, joining, proposals, review, leaving, recognition.
- [CONTRIBUTING.md](CONTRIBUTING.md): the short version for a first pull request.

## License

MIT. See [LICENSE](LICENSE). No third-party game assets; everything drawn
is a rectangle or a line.
