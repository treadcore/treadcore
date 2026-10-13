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
#   make run         the window, solo against fifteen bots (PLAYERS=4 for three)
#   make lan         the window, LAN lobby: waits for peers, elects a host
#   make host        headless host on the LAN (plus bots for empty seats, PLAYERS)
#   make seat        headless bot that joins a LAN lobby (run four: a test)
#   make introduce   the introducer: tells peers across the Internet each
#                    other's addresses (INTRODUCER=host:port on lan, host,
#                    seat; BEACON=0 to stay off the LAN; PORT=n)
#   make lobby-test  four headless seats on this machine negotiate a host
#                    (LAG=200 for lag; INTRODUCER=1 to meet through an
#                    introducer only; REJOIN=1 kills a seat and brings it
#                    back to the same seat)
#   make map         write the generated map for SEED to MAP (default
#                    maps/generated-SEED.map): the start of a hand-made one;
#                    MAP=maps/x.map make debug|run|host plays on it
#   make convert FROM=file.map  convert a prior-art map into maps/prior-art/
#   make replay      play back the newest build/logs/replay-*.json (or
#                    REPLAY=file) and check every world hash it recorded;
#                    the regression test for a change to the rules
#   make test        cargo test, every crate
#   make mem         one line every 2 s of free/available/swap/rustc count,
#                    for a second terminal beside make warm (N=5 for 5 s)
#   make quiet       desktop effects off (blur/shadow/animation) while
#                    building: on a VM with no GPU they cost the CPU the
#                    compiler wants. make loud restores them. No root.
#   make sysprep     show the root-level memory changes for this VM
#                    (disk swapfile behind zram, zstd); apply with doas
#   make schema      print the JSON brain's observation and command examples
#   make clean       remove build output, keep the cargo cache
#   make distclean   remove build/ and target/ both

SHELL := /usr/bin/env bash
.DEFAULT_GOAL := help
.PHONY: help configure warm tree why build headless debug run lan host seat introduce lobby-test replay map convert test mem quiet loud sysprep schema clean distclean

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
# Of the bot seats, how many dig in (Defender) rather than attack (RuleBot).
DEFENDERS ?= 0
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
# The introducer's port (make introduce).
PORT     ?= 47296
# Deal the seats into N teams (0: everyone alone, allying in play).
TEAMS    ?= 0
# What a tank starts with: open, tournament or strict (SPEC section 11).
GAMETYPE ?= open
# A map file to play on instead of the generated one (documentation/MAPS.md).
MAP      ?=
REPLAY   ?=
export BUILDDIR LOGDIR GAME_SECONDS SEED NAME GAME WANT PEER PLAYERS DEFENDERS TEAMS GAMETYPE MAP JOBS VERBOSE LAG INTRODUCER BEACON IDENTITY REJOIN REJOIN_AFTER PORT

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

lobby-test: configure
	@scripts/lobby-test.sh

replay: configure
	@scripts/replay.sh $(REPLAY)

convert:
	@test -n "$(FROM)" || { echo 'usage: make convert FROM=path/to/Some.map'; exit 2; }
	@mkdir -p maps/prior-art
	@python3 -I scripts/priorart2map.py "$(FROM)" "maps/prior-art/$$(basename "$(FROM)" .map | tr 'A-Z' 'a-z' | tr -d "'" | tr ' ' '-').map"

map: configure

	@mkdir -p maps
	@cargo run -q -p treadcore-server -- --seed $(SEED) --write-map $(or $(MAP),maps/generated-$(SEED).map)



test:
	@scripts/build.sh test

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

clean:
	rm -rf $(BUILDDIR)/logs $(BUILDDIR)/config.env $(BUILDDIR)/*.log
	cargo clean -q 2>/dev/null || true

distclean: clean
	rm -rf $(BUILDDIR) target
