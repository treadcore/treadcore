#!/usr/bin/env bash
# run.sh MODE
#   solo   the window, fifteen bots (PLAYERS=n), no network
#   lan    the window, LAN lobby, elect a host among whoever answers
#   host   headless host on the LAN; bots in empty seats
#   seat   headless bot that joins the LAN lobby
#   introduce  the introducer: a rendezvous service for peers across the
#          Internet (PORT=n); hosts nothing
#   listen the server as a service (roadmap step 8): a dedicated host in
#          lobby GAME, match after match, files to LISTEN_DIR
#          (build/matches), status page on http://this-machine:PORT/;
#          GAME="a b" sits in both lobbies at once, files in LISTEN_DIR/a
#          and LISTEN_DIR/b
# INTRODUCER=host:port registers lan, host and seat with an introducer;
# BEACON=0 keeps them off the LAN broadcast; IDENTITY=file keeps the
# peer id across restarts (the default, build/identity, does).
# WINGMEN=n makes the first n bots wingmen of the player: they follow,
# fight beside it and ask to join its side (solo, lan; step 11a).
# ROOKIES=n makes n of the bots rookies, for a newcomer (solo, lan,
# host, listen).
# All of them log to build/logs/MODE-STAMP.log at RUST_LOG=debug.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
MODE="${1:-solo}"
LOG="$LOGDIR/$MODE-$STAMP.log"
NET=(--name "${NAME:-player}" --game "${GAME:-treadcore}" --want "${WANT:-4}" --players "${PLAYERS:-16}" --defenders "${DEFENDERS:-0}" --teams "${TEAMS:-0}" --game-type "${GAMETYPE:-open}")
[ -n "${PEER:-}" ] && NET+=(--peer "$PEER")
[ -n "${INTRODUCER:-}" ] && NET+=(--introducer "$INTRODUCER")
[ "${BEACON:-1}" = 0 ] && NET+=(--no-beacon)
NET+=(--identity "${IDENTITY:-$BUILDDIR/identity}")
case "$MODE" in
    solo|lan)
        [ "$HAVE_WINDOW" = 1 ] || die "the window cannot build here (see make configure); try make debug"
        say "building the window (release) -> $LOGDIR/build-$STAMP.log"
        cargo build --release -p treadcore-client 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
        BIN="$ROOT/target/release/treadcore"
        if [ "$MODE" = solo ]; then ARGS=(--solo --seed "${SEED:-0x7EADC0DE}" --players "${PLAYERS:-16}" --teams "${TEAMS:-0}"); else ARGS=(--lan "${NET[@]}" --seed "${SEED:-0x7EADC0DE}"); fi
        ;;
    host|seat)
        say "building headless (release) -> $LOGDIR/build-$STAMP.log"
        cargo build --release -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
        BIN="$ROOT/target/release/treadcore-server"
        ARGS=(--net "${NET[@]}" --seed "${SEED:-0x7EADC0DE}")
        [ "$MODE" = seat ] && ARGS+=(--port 0)
        ;;
    introduce)
        say "building headless (release) -> $LOGDIR/build-$STAMP.log"
        cargo build --release -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
        BIN="$ROOT/target/release/treadcore-server"
        ARGS=(--introduce --port "${PORT:-47296}")
        ;;
    listen)
        say "building headless (release) -> $LOGDIR/build-$STAMP.log"
        cargo build --release -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
        BIN="$ROOT/target/release/treadcore-server"
        # Not NET: a server has no want, and its identity is its own.
        ARGS=(--listen "${LISTEN_DIR:-$BUILDDIR/matches}" --name "${NAME:-server}" --players "${PLAYERS:-16}" --defenders "${DEFENDERS:-0}" --teams "${TEAMS:-0}" --game-type "${GAMETYPE:-open}" --port "${PORT:-47296}" --seed "${SEED:-0x7EADC0DE}" --identity "${IDENTITY:-$BUILDDIR/server-identity}")
        [ -n "${PEER:-}" ] && ARGS+=(--peer "$PEER")
        [ -n "${INTRODUCER:-}" ] && ARGS+=(--introducer "$INTRODUCER")
        [ "${BEACON:-1}" = 0 ] && ARGS+=(--no-beacon)
        # GAME="north south": one lobby each, at once, on PORT and after.
        for g in ${GAME:-treadcore}; do ARGS+=(--game "$g"); done
        ;;
    *) die "unknown mode $MODE" ;;
esac
[ -n "${MAP:-}" ] && ARGS+=(--map "$MAP")
case "$MODE" in solo|lan|host|seat|listen) [ "${INDUSTRY:-0}" = 1 ] && ARGS+=(--industry) ;; esac
case "$MODE" in solo|lan|host|listen) [ "${ROOKIES:-0}" != 0 ] && ARGS+=(--rookies "$ROOKIES") ;; esac
case "$MODE" in solo|lan) [ "${WINGMEN:-0}" != 0 ] && ARGS+=(--wingmen "$WINGMEN") ;; esac
say "$BIN ${ARGS[*]}  (log: $LOG)"

# The window on a musl machine with lavapipe: build/bigstack.so from
# configure (scripts/bigstack.c says why). Headless modes need nothing.
case "$MODE" in solo|lan) [ -n "${PRELOAD:-}" ] && export LD_PRELOAD="$PRELOAD" ;; esac
RUST_LOG="${RUST_LOG:-debug}" RUST_BACKTRACE=1 "$BIN" "${ARGS[@]}" 2>&1 | tee "$LOG"
