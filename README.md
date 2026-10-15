# treadcore

A sixteen-player overhead tank game for the battlefield the tank was just
handed: the gun fires indirect like artillery, and the tank keeps itself
alive the way a carrier does, by owning the airspace around it with a
drone crew flown off its own hull.

Rust, Bevy for the window, plain `std` UDP for the network. The
clients find each other on the LAN, measure the links between them, and
elect a host by themselves: the one whose worst link is best, with
anyone the host cannot reach routed through a neighbour that can.
Across the Internet a tiny introducer tells them each other's
addresses and they punch through; nobody runs a game server.

Website: [treadcore.org](https://treadcore.org). Repository:
[github.com/treadcore/treadcore](https://github.com/treadcore/treadcore).
Crates: [treadcore-client](https://crates.io/crates/treadcore-client)
(the window, installed as `treadcore`), [treadcore-server](https://crates.io/crates/treadcore-server),
and the libraries they are built from, `treadcore-sim`, `treadcore-proto`,
`treadcore-net`, `treadcore-brain`. The current release is 0.1.0-rc.6,
the sixth candidate (`documentation/RELEASE.md`), the first with
packages; it is offered for playtesting, and the player's guide is
[documentation/HOW-TO-PLAY.md](documentation/HOW-TO-PLAY.md).

```
cargo install treadcore-client   # the window, as `treadcore`
treadcore --solo                 # you against fifteen bots
```

For your system: [treadcore.org/download.html](https://treadcore.org/download.html).
Packages for Debian, Ubuntu and Linux Mint (.deb) and Fedora (.rpm), a
zip for Windows and for macOS, all on each
[GitHub release](https://github.com/treadcore/treadcore/releases); a
Nix flake (`nix run github:treadcore/treadcore`); and `make install`
from source for Copal, Alpine, Arch, Omarchy and Gentoo. The packages
carry the window's runtime dependencies; none of them needs Rust,
which you bring yourself only to build from source.

## The game

Up to sixteen players, one tank each, on a 128 x 128 tile island (one
tile is 25 m); bots fill the seats people do not. The tank carries a
bay of drones:

| drone | job | the number that matters |
| --- | --- | --- |
| recon | orbits a point, sees 12 tiles, feeds telemetry back over a 40-tile link | a shell at a target it can see lands within 0.6 tiles; blind, within 4 |
| interceptor | holds station on a ring around the tank (the *bubble*), rams any strike that flies in | kills 90 % of the time on a good link, 50 % on a dead one; one shot each |
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
make shoot       # the release frames: the window driven by a script, cropped, for the guide and the site
make release     # the release procedure: version, tests, no warnings, frames taken and shown, crates package
make hooks       # turn on .githooks/: pushing a v* tag runs make release first
make token       # store the crates.io API token for cargo (TOKEN=... or typed)
make publish     # publish the five crates in dependency order; DRY=1 to verify only
```

Needs Rust 1.85+. The window needs `wayland-client`, `xkbcommon`, `alsa`
and `libudev` dev packages and a Vulkan driver (on a machine with no
GPU, lavapipe). `make deps` prints the command that installs them for
this distribution, and `APPLY=1 make deps` runs it through doas or
sudo: Copal, Alpine, Arch, Omarchy, Debian, Ubuntu, Gentoo and Fedora
are known. `make configure` prints the same line when something is
missing. Without them the headless crates still build. On a musl machine `make configure` also builds a tiny preload
that gives lavapipe's shader compiler a stack it can live on
(`documentation/BUILD.md`). On macOS nothing extra is needed: the
window uses Cocoa, Metal and CoreAudio from the system.

`make install` puts `treadcore` and `treadcore-server` in
`~/.local/bin` (`PREFIX=/usr/local` for elsewhere). On Linux it adds a
desktop entry and an icon, so Treadcore is in the application menu:
Copal's Super+Space, Omarchy's launcher, GNOME's, KDE's. On Copal and
Alpine the window is installed behind a small wrapper that preloads
the stack fix musl needs. On macOS it adds `Treadcore.app` in
`~/Applications`. `make uninstall` takes all of it out.

## Running

```
make run         # the window, solo against fifteen bots (PLAYERS=4 for three)
make lan         # the window, LAN lobby; WANT=2 to start with two
make host        # headless host on the LAN, bots in empty seats
make seat        # headless bot that joins a LAN lobby
make quad        # four windows on this machine, networked, tiled two by two
make lobby-test  # four headless peers on this machine elect a host
LAG=200 make lobby-test   # the same with 200 ms of lag on every receive: prediction's proof
INTRODUCER=1 make lobby-test   # no broadcast: the four meet through an introducer alone
REJOIN=1 make lobby-test       # a seat killed outright at 20 s comes back to the same seat
LATE=1 make lobby-test         # a fifth peer arriving at 20 s takes a bot's seat
make sounds      # every sound the window makes, as WAV files in build/sounds
make listen      # a dedicated server: hosts match after match when elected, takes no seat
LISTEN=near make lobby-test    # the four, plus a server with better links: it must host
LISTEN=far make lobby-test     # plus a server 100 ms away: it must be left out
```

Across the Internet (`documentation/PROTOCOL.md`, "the introducer"):
someone with a public address runs `make introduce` (it hosts nothing),
and everyone else plays with `INTRODUCER=that.host:47296 BEACON=0 make
lan` (or `host`, `seat`). The peer id lives in `build/identity`, so a
player whose game drops gets the same seat back by starting again.
A group with a machine that is always on can run `make listen` there
(with `INTRODUCER=` to be listed): it hosts whenever it is within 10 ms
of the best-placed player, keeps every match's trace and replay in
`build/matches`, and has a status page on `http://that.host:47296/`.

How to play, with pictures: [documentation/HOW-TO-PLAY.md](documentation/HOW-TO-PLAY.md).
In short: the orders are the column on the left of the screen, with
what each costs and whether it can be given now. Left click drives (the world
routes around water, over bridges and round hostile pillboxes); hold
the right button to see the gun's reach and release to fire; 1/2/3
launch interceptor / recon / strike, R recalls, [ ] set the bubble.
The Pillboys take the prior art's builder modes: 4 farm, 5 road (a
bridge on water), 6 wall, 7 pillbox, 8 mine arm a mode (the key or a
click on its tile; the red lamp shows which), and the next left click
on the map sends them to that square; in pillbox mode a click on a
pillbox repairs it, or captures it if it is dead or nobody's; the
square it would take is outlined under the cursor; Esc drops the
mode. 9 brings them home.
F follows the tank, Enter talks (Shift-Enter to your team), P proposes a change of sides and Y/N answer one, the apostrophe steps through your
pillboxes, Tab shows the events. The screen is laid out as the prior
art's was (`documentation/INTERFACE.md`); what still stands between
the build and a game a stranger can play is `documentation/PLAYABILITY.md`.


## The debug trace

```
make debug                      # 3 game minutes of sixteen bots, flat out
GAME_SECONDS=600 SEED=0x1234 PLAYERS=4 make debug
DEFENDERS=12 make debug         # twelve Defenders dig in at their depots, four RuleBots attack
ROOKIES=8 make debug            # eight rookies: drones late, aim loose, never storm
TEAMS=4 make debug              # four teams of four; allies share recon, spare each other
GAMETYPE=strict make debug      # the prior art's game types: open, tournament, strict (what a tank starts with)

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
source/treadcore-net      UDP, LAN discovery, RTT matrix, host election, relay, Host/Client,
                          prediction, the introducer (Internet rendezvous) and rejoin.
source/treadcore-brain    the Brain trait, RuleBot, JsonBrain.
source/treadcore-server   headless: bot matches, LAN host, LAN bot seat.
source/treadcore-client   the Bevy window.
scripts/                  what the Makefile calls.
maps/                     map files (documentation/MAPS.md); make map writes a new one;
                          maps/prior-art/ holds five converted prior-art maps.


vendor/                   external code and blobs vendored by the author, with their licences.
documentation/            concept, design, protocol, research notes, the lab report.
index.html, CNAME         the site at treadcore.org (GitHub Pages, served from the root).
reports/                  dated web reports, served by the site.
journal/                  the creation journal: one plain-text entry per session (see CHANGELOG.md).
```

## Documents

- [paper.html](https://treadcore.org/paper.html): the paper, in the IEEE form: the rules, a balance review from traced matches, the network's test record.
- [documentation/HOW-TO-PLAY.md](documentation/HOW-TO-PLAY.md): the player's guide, with pictures of the build it ships with: the screen, driving and firing, the drones, the builder's five modes, pillboxes and depots, every key.
- [documentation/CONCEPT.md](documentation/CONCEPT.md): the brief, reviewed; how the design improves on the base concept.
- [documentation/DESIGN.md](documentation/DESIGN.md): the rules, the numbers, and why.
- [documentation/PROTOCOL.md](documentation/PROTOCOL.md): the wire protocol and the host election.
- [documentation/RESEARCH.md](documentation/RESEARCH.md): what the game is modelled on, with sources.
- [documentation/ROADMAP.md](documentation/ROADMAP.md): the next architectural steps, in dependency order.
- [documentation/BUILD.md](documentation/BUILD.md): how the build is designed for a small machine; `make warm`.
- [documentation/LAB-REPORT.md](documentation/LAB-REPORT.md): a walk through the code, crate by crate, and the Rust technique in it.
- [documentation/LAB-REPORT-PILLBOYS.md](documentation/LAB-REPORT-PILLBOYS.md): the Pillboy requirement checked against the prior art's builder, and what was decided.
- [documentation/LAB-REPORT-PLAYTEST.md](documentation/LAB-REPORT-PLAYTEST.md): the first playtest build, with pictures: the window on a CPU rasteriser, a three-client match through the introducer, the multiclient explained, nine findings.
- [documentation/INTERFACE.md](documentation/INTERFACE.md): what Bolo, a garage screen and the military HUD graphics put on the screen, and treadcore's layout drawn from them; built 13 October.
- [documentation/PLAYABILITY.md](documentation/PLAYABILITY.md): the review of what stands between the build and a playable game, ranked and costed, with Bolo's interface elements checked off one by one.
- [documentation/SPEC-PRIOR-ART-1.14.md](documentation/SPEC-PRIOR-ART-1.14.md): the clean-room functional specification of the prior art, version 1.14 that the prior-art-derived steps cite, with a preface on what treadcore adopts.
- [documentation/MAPS.md](documentation/MAPS.md): the map file format; any text editor is the editor.

- [documentation/LAB-REPORT-PACKAGES.md](documentation/LAB-REPORT-PACKAGES.md): packages for every platform as a Cargo tutorial: the Bevy template, the multi-target build, .deb, .rpm, .msi and the flake, testing each where its users are, the git hook, the one-day limit, and API discipline.
- [documentation/DEPLOY.md](documentation/DEPLOY.md): the deploy procedure, step by step: tag on day 0, publish on day 1 or later, with the command for each step and what to do when one fails.
- [documentation/RELEASE.md](documentation/RELEASE.md): the implementation cycle, versions, release monuments, candidates and playtesting, and the release procedure `make release` runs.
- [documentation/INDEX.md](documentation/INDEX.md): the code structure index: the tree, every file outlined, a glossary, a keyword cross-reference.
- [documentation/GOVERNANCE.md](documentation/GOVERNANCE.md): how the organization is run: roles, joining, proposals, review, leaving, recognition.
- [CONTRIBUTING.md](CONTRIBUTING.md): the short version for a first pull request.

## License

MIT. See [LICENSE](LICENSE). No third-party game assets; everything drawn
is a rectangle or a line.
