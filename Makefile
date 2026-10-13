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
#   make run         the window, solo against three bots
#   make lan         the window, LAN lobby: waits for peers, elects a host
#   make host        headless host on the LAN (plus bots for empty seats)
#   make seat        headless bot that joins a LAN lobby (run four: a test)
#   make lobby-test  four headless seats on this machine negotiate a host
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
.PHONY: help configure warm tree why build headless debug run lan host seat lobby-test test mem quiet loud sysprep schema clean distclean

BUILDDIR ?= build
LOGDIR   ?= $(BUILDDIR)/logs
# Match length for `make debug`, in game seconds. Short by default: a trace
# is for reading.
SECONDS  ?= 180
SEED     ?= 0x7EADC0DE
NAME     ?= $(shell hostname -s 2>/dev/null || echo player)
# Parallel rustc jobs. Empty means configure.sh sizes it from RAM.
JOBS     ?=
# make warm chatter: 0 numbered crates, 1 tree + timings, 2 rustc commands.
VERBOSE  ?= 0
GAME     ?= treadcore
WANT     ?= 4
PEER     ?=
export BUILDDIR LOGDIR SECONDS SEED NAME GAME WANT PEER JOBS VERBOSE

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

lobby-test: configure
	@scripts/lobby-test.sh

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
