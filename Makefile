# SPDX-License-Identifier: MIT
# Copyright (c) 2026 Paul Richeson
#
# TREADCORE -- build, run, trace.
#
# The Makefile is the front door; the scripts under scripts/ do the work;
# cargo is what they call. Same convention as copal, radbeeper and the
# rest: `make` on its own prints this list.
#
#   make configure   check the toolchain and libraries, write build/config.env
#   make warm        build the engine alone (Bevy, ~370 crates) with a job
#                    count sized to RAM and at low priority; do this once,
#                    then every other target finds it built. Resumable.
#                    Prints every crate as [n/total]; VERBOSE=1 shows the
#                    dependency tree first, VERBOSE=2 the rustc commands.
#   make tree        the engine's dependency tree, each crate's deps under it
#   make why CRATE=x which crates pull x into the build (inverse tree)
#   make build       release build of every crate (after warm: just the game)
#   make headless    release build of everything except the window
#   make debug       bot-only match, RUST_LOG=trace, backtraces on, everything
#                    captured under build/logs/ -- the trace for AI-assisted
#                    development and debugging
#   make run         the window, solo against fifteen bots (PLAYERS=4 for three;
#                    SKILL=0.6 for weaker bots against you, 1 in full)
#   make lan         the window, LAN lobby: waits for peers, elects a host
#   make host        headless host on the LAN (plus bots for empty seats, PLAYERS)
#   make seat        headless bot that joins a LAN lobby (run four: a test)
#   make introduce   the introducer: tells peers across the Internet each
#                    other's addresses (INTRODUCER=host:port on lan, host,
#                    seat; BEACON=0 to stay off the LAN; PORT=n)
#   make listen      the server as a service: a dedicated host in lobby GAME,
#                    match after match, takes no seat, hosts when elected;
#                    traces and replays to LISTEN_DIR (build/matches), a
#                    status page on http://this-machine:PORT/; GAME="a b"
#                    serves both lobbies at once
#   make quad        three windows on this machine, networked, one a team:
#                    each its team's commander on the autopilot, bots to
#                    fifteen seats; a LAN lobby over loopback, a host
#                    elected among them, the windows tiled two by two
#                    (CLIENTS=n, PLAYERS=n, TEAMS=n, AUTOPILOT=0,
#                    RESPAWNS=n, MATCH_SECONDS=n); logs in build/logs/quad-*
#   make lobby-test  four headless seats on this machine negotiate a host
#   make rematch-test  the window plays a networked match, Enter, and a second
#                    (LAG=200 for lag; INTRODUCER=1 to meet through an
#                    introducer only; REJOIN=1 kills a seat and brings it
#                    back to the same seat; LISTEN=near|far adds a --listen
#                    server that must win or lose the election; with near,
#                    LOBBIES=2 has it host a second lobby at the same time)
#   make map         write the generated map for SEED to MAP (default
#                    maps/generated-SEED.map): the start of a hand-made one;
#                    MAP=maps/x.map make debug|run|host plays on it
#                    (INDUSTRY=1: with pads and neutral structures)
#   make worldmaps   draw maps/world/atlantic.map, europe.map and world.map from
#                    scripts/worldmap.py's coastlines (roadmap step 10g),
#                    and maps/naval.map, every start at sea, from
#                    scripts/navalmap.py
#   make convert FROM=file.map  convert a prior-art map into maps/prior-art/
#   make replay      play back the newest build/logs/replay-*.json (or
#                    REPLAY=file) and check every world hash it recorded;
#                    the regression test for a change to the rules
#   make test        cargo test, every crate
#   make tune        evolve the bots' weights over the brain census (scripts/tune.sh)
#   make scenario    run a scenario file (S=scenarios/x.toml) and print its
#                    trace; every file in scenarios/ is also a test.
#                    RUNS=n runs it over n seeds and gives how often it
#                    holds; SET="name=value" sets a weight on every brain
#   make families    every family in scenarios/families/ over its seeds,
#                    each held to its floor (SET="w=v" to break one).
#   make census      one brain against another over the fair census, its
#                    seeds split over JOBS, pooled overall and by map
#                    (scripts/census.sh; TRIAL, BASE, SEEDS, SEED0, MAP=all;
#                    RESUME=build/census/STAMP finishes a stopped one)
#   make parts       each part off from the brain with every part on, by
#                    the census (scripts/parts.sh; PARTS, SEEDS, JOBS)
#   make spsa        tune named weights by SPSA over the paired census
#                    (scripts/spsa.sh; TUNE, STEPS, SEEDS, SET, START)
#   make converge    the census as a learner, until stopped: each generation
#                    of trials suggested from every census so far, each
#                    read past two errors read twice on fresh seeds; no
#                    default changed (scripts/converge.sh; UNTIL, GEN,
#                    SEEDS, SEED0, HELD, RESUME)
#   make ledger      every census on this machine, one row each
#                    (scripts/ledger.py > build/ledger.tsv)
#   make train       train the learned tank by evolution strategies, a
#                    phase at a time (LR 97): PHASE=1 go to, PHASE=3
#                    shoot; GROUND=a,b,... courses trained together
#                    (phase1, phase3, scenarios/training/*.toml, LR 99);
#                    GENS, PAIRS, SEEDS, THREADS, START (a net to begin
#                    from), OUT (build/train), NAME; CHECK=net.bin reads a
#                    net on the held-out seeds only; BASELINE=1 reads the
#                    scripted brain first; with CHECK, MATCH=n reads the
#                    net in one seat of n dealt matches' first 3 minutes
#   make gauntlet    single-decision tests (scenarios/gauntlet/, LR 102):
#                    the scripted brain, the untrained net and every
#                    milestone net in nets/, pass share and reaction time
#                    (RUNS a test, 200; build/train/gauntlet.tsv)
#   make clips       a short GIF of each milestone net at work, and the
#                    newest in a real match (needs ffmpeg; build/train/
#                    clips/, LR 102)
#   make roundrobin  the census for every ordered pair of profiles/*.weights
#                    and the defaults: the payoff matrix (scripts/roundrobin.sh;
#                    PROFILES, SEEDS, SEED0, MAP)
#   make brainmap    write documentation/BRAIN-MAP.md: every brain file and
#                    function with its doc comment's first sentence
#   make switches    each capability that is off, alone, over the fair census:
#                    the difference a seat and its standard error
#                    (scripts/switches.sh; SWITCHES, SEEDS, SECS, JOBS)
#   make sounds      write every sound the window makes to build/sounds/*.wav
#                    (synthesised, no files in the tree): to listen to
#   make shoot       take the release frames: the window driven by a
#                    script, every picture the player's guide and the site
#                    show, cropped, into reports/images/guide-*.png with
#                    a manifest for the version
#   make release     the release procedure (RELEASE.md section 6): version
#                    and changelog agree, tests pass, the build has no
#                    warnings, the frames for this version are taken and
#                    every one is shown by the guide or the site, the
#                    documents name the guide, the crates package. Then
#                    says what to do by hand: commit, tag, publish, push
#   make builds      write builds.json and the build grid on index.html and
#                    download.html from the published release (three API
#                    calls at most; run on the day a release is published)
#   make hooks       turn on the repository's git hooks (.githooks/): a
#                    push of a release tag runs make release first, and
#                    is refused if it fails
#   make token       store or replace the crates.io API token for cargo
#                    (TOKEN=... or typed, echo off); it lives in
#                    ~/.cargo/credentials.toml, never in this tree
#   make publish     publish the five crates to crates.io in dependency
#                    order (sim, proto, net, brain, client); DRY=1 only
#                    packages and verifies sim. Needs make token, a clean
#                    tree (ALLOW_DIRTY=1 to override) and, by RELEASE.md,
#                    a tag v<version>
#   make mem         one line every 2 s of free/available/swap/rustc count,
#                    for a second terminal beside make warm (N=5 for 5 s)
#   make quiet       desktop effects off (blur/shadow/animation) while
#                    building: on a VM with no GPU they cost the CPU the
#                    compiler wants. make loud restores them. No root.
#   make sysprep     show the root-level memory changes for this VM
#                    (disk swapfile behind zram, zstd); apply with doas
#   make schema      print the JSON brain's observation and command examples
#   make deps        the system packages the window needs, for this
#                    distribution (Copal, Alpine, Arch, Omarchy, Debian,
#                    Ubuntu, Gentoo, Fedora); prints the command,
#                    APPLY=1 runs it through doas or sudo
#   make install     the release binaries into PREFIX/bin (~/.local; on
#                    macOS also ~/Applications/Treadcore.app, APP=0 not)
#   make uninstall   remove what make install put there
#   make clean       remove build output, keep the cargo cache
#   make distclean   remove build/ and target/ both

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help
.PHONY: help configure warm tree why build headless debug run lan host seat introduce listen quad lobby-test rematch-test replay map worldmaps convert test tune scenario families parts census spsa converge ledger train gauntlet clips roundrobin switches brainmap sounds shoot release builds hooks token publish mem quiet loud sysprep schema deps install uninstall clean distclean

BUILDDIR ?= build
LOGDIR   ?= $(BUILDDIR)/logs
# Match length for `make debug` and `make lobby-test`, in game seconds. Short by
# default: a trace is for reading. Not SECONDS: that is bash's own counter of
# seconds since the shell started, and exporting it makes the match as long
# as the build took plus 180.
GAME_SECONDS ?= 180
SEED     ?= 0x7EADC0DE
NAME     ?= $(shell hostname -s 2>/dev/null || echo player)
# Parallel rustc jobs. Empty means configure.sh sizes it from RAM.
JOBS     ?=
# make warm chatter: 0 numbered crates, 1 tree + timings, 2 rustc commands.
VERBOSE  ?= 0
GAME     ?= treadcore
WANT     ?= 4
PEER     ?=
PLAYERS  ?= 16
# Of the bot seats, how many dig in (Defender) rather than attack (RuleBot),
# and how many of the rest play badly (a rookie, PLAYABILITY item 9).
DEFENDERS ?= 0
ROOKIES ?= 0
# Of the window's bots, the first N fly on the player's wing (step 11a).
WINGMEN ?= 0
# FLEETS=1: three base-building teams of five and one aggro bot (solo).
FLEETS ?= 0
# Simulated one-way lag, in ms, on every peer of `make lobby-test`: the
# stand-in for tc netem. 0 is the loopback as it is.
LAG      ?= 0
# An introducer, host:port, for lan/host/seat (roadmap step 7); for
# lobby-test, 1 runs one on this machine and meets through it alone.
INTRODUCER ?=
# 0 keeps lan/host/seat off the LAN broadcast (with an introducer).
BEACON   ?= 1
# The file that keeps this peer's id across restarts (build/identity).
IDENTITY ?=
# lobby-test: 1 kills peer 4 after REJOIN_AFTER seconds and restarts it.
REJOIN   ?= 0
REJOIN_AFTER ?= 20
# Where make listen writes each match's trace and replay.
LISTEN_DIR ?=
# lobby-test: near or far, a --listen server beside the four; far gives
# it LISTEN_LAG ms (100) on every receive, near gives the four LAG (30).
LISTEN   ?=
# lobby-test, with LISTEN=near: 2 has the server in a second lobby at
# the same time, with two seats of its own.
LOBBIES  ?= 1
# The introducer's port (make introduce), and the server's (make listen).
PORT     ?= 47296
# Deal the seats into N teams (0: everyone alone, allying in play).
# Empty: each target's own default, three teams for quad and run, none
# for lan, listen and debug. A 0 here made quad and run free-for-alls of
# plain RuleBots: the scripts' ${TEAMS:-3} keeps a 0.
TEAMS    ?=
# What a tank starts with: open, tournament or strict (SPEC section 11).
GAMETYPE ?= open
# A map file to play on instead of the generated one (documentation/MAPS.md).
MAP      ?=
# 1: give a map with no structures an economy (roadmap step 10f).
INDUSTRY ?= 0
REPLAY   ?=
export BUILDDIR LOGDIR GAME_SECONDS SEED NAME GAME WANT PEER PLAYERS DEFENDERS ROOKIES SKILL WINGMEN FLEETS TEAMS GAMETYPE MAP INDUSTRY JOBS VERBOSE LAG INTRODUCER BEACON IDENTITY REJOIN REJOIN_AFTER PORT LISTEN LISTEN_DIR LISTEN_LAG LOBBIES

help:
	@sed -n '/^#   make /,/^$$/p' Makefile | sed 's/^#//'

configure:
	@scripts/configure.sh

warm: configure
	@scripts/warm.sh

tree:
	@cargo tree -p treadcore-warm -e normal,build --prefix indent
	@echo; echo "$$(cargo tree -p treadcore-warm -e normal,build --prefix none | sort -u | wc -l) distinct crates"

why:
	@test -n "$(CRATE)" || { echo 'usage: make why CRATE=naga'; exit 2; }
	@cargo tree -p treadcore-warm -e normal,build -i $(CRATE) --prefix indent

build: configure
	@scripts/build.sh release all

headless: configure
	@scripts/build.sh release headless

debug: configure
	@scripts/debug.sh

run: configure
	@scripts/run.sh solo

lan: configure
	@scripts/run.sh lan

host: configure
	@scripts/run.sh host

seat: configure
	@scripts/run.sh seat

introduce: configure
	@scripts/run.sh introduce

listen: configure
	@scripts/run.sh listen

quad: configure
	@scripts/quad.sh

lobby-test: configure
	@scripts/lobby-test.sh

rematch-test: configure
	@scripts/rematch-test.sh

replay: configure
	@scripts/replay.sh $(REPLAY)

convert:
	@test -n "$(FROM)" || { echo 'usage: make convert FROM=path/to/Some.map'; exit 2; }
	@mkdir -p maps/prior-art
	@python3 -I scripts/priorart2map.py "$(FROM)" "maps/prior-art/$$(basename "$(FROM)" .map | tr 'A-Z' 'a-z' | tr -d "'" | tr ' ' '-').map"

map: configure

	@mkdir -p maps
	@cargo run -q -p treadcore-server -- --seed $(SEED) --write-map $(or $(MAP),maps/generated-$(SEED).map) $(if $(filter 1,$(INDUSTRY)),--industry)



worldmaps:
	@mkdir -p maps/world
	@python3 -I scripts/worldmap.py atlantic maps/world/atlantic.map
	@python3 -I scripts/worldmap.py europe maps/world/europe.map
	@python3 -I scripts/worldmap.py world maps/world/world.map
	@python3 -I scripts/navalmap.py maps/naval.map

test:
	@scripts/build.sh test

tune:
	@scripts/tune.sh

switches:
	@scripts/switches.sh

parts:
	@scripts/parts.sh

census:
	@scripts/census.sh

spsa:
	@scripts/spsa.sh

converge:
	@scripts/converge.sh

ledger:
	@python3 -I scripts/ledger.py build/census > build/ledger.tsv && echo "build/ledger.tsv: $$(($$(wc -l < build/ledger.tsv) - 1)) censuses"

train:
	@PHASE="$(or $(PHASE),1)" OUT="$(abspath $(or $(OUT),build/train))" START="$(if $(START),$(abspath $(START)))" CHECK="$(if $(CHECK),$(abspath $(CHECK)))" GROUND="$(GROUND)" PROBE="$(PROBE)" MATCH="$(MATCH)" NAME="$(NAME)" HELD="$(HELD)" GENS="$(GENS)" PAIRS="$(PAIRS)" SEEDS="$(SEEDS)" THREADS="$(THREADS)" BASELINE="$(BASELINE)" \
		cargo test -q -p treadcore-brain --release -- --ignored learn::harness::train --nocapture

gauntlet:
	@GAUNTLET=1 OUT="$(abspath $(or $(OUT),build/train))" GAUNTLET_RUNS="$(RUNS)" THREADS="$(THREADS)" \
		cargo test -q -p treadcore-brain --release -- --ignored learn::harness::train --nocapture

clips:
	@CLIPS=1 OUT="$(abspath $(or $(OUT),build/train))" THREADS="$(THREADS)" \
		cargo test -q -p treadcore-brain --release -- --ignored learn::harness::train --nocapture

roundrobin:
	@scripts/roundrobin.sh

scenario:
	@test -n "$(S)" || { echo "make scenario S=scenarios/name.toml"; exit 1; }
	@if [ -n "$(RUNS)" ]; then \
		SCENARIO="$(abspath $(S))" RUNS="$(RUNS)" SEED0="$(SEED0)" SCENARIO_SET="$(SET)" cargo test -q -p treadcore-brain --release -- --ignored scenario_many --nocapture; \
	else \
		SCENARIO="$(abspath $(S))" SEED="$(SEED)" SCENARIO_SET="$(SET)" cargo test -q -p treadcore-brain --release -- --ignored scenario_one --nocapture; \
	fi

families:
	@SCENARIO_SET="$(SET)" cargo test -q -p treadcore-brain --release -- --ignored every_family_holds_its_floor --nocapture

brainmap:
	@python3 -I scripts/brainmap.py documentation/BRAIN-MAP.md

sounds: configure
	@cargo run -q --release -p treadcore-client -- --write-sounds "$(BUILDDIR)/sounds"

shoot: configure
	@scripts/shoot.sh

release: configure
	@scripts/release.sh

builds:
	@scripts/builds.sh

hooks:
	@git config core.hooksPath .githooks && echo "  hooks on: a push of a v* tag runs make release first (.githooks/pre-push)"

token:
	@scripts/publish.sh token

publish:
	@scripts/publish.sh $(if $(DRY),dry,all)

mem:
	@scripts/memwatch.sh $(or $(N),2)

quiet:
	@scripts/buildmode.sh on

loud:
	@scripts/buildmode.sh off

sysprep:
	@scripts/sysprep.sh

schema:
	@cargo run -q -p treadcore-server -- --brain-schema

deps:
	@scripts/deps.sh

install: configure
	@scripts/install.sh install

uninstall:
	@scripts/install.sh uninstall

clean:
	rm -rf $(BUILDDIR)/logs $(BUILDDIR)/config.env $(BUILDDIR)/*.log
	cargo clean -q 2>/dev/null || true

distclean: clean
	rm -rf $(BUILDDIR) target
