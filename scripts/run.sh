#!/usr/bin/env bash
# run.sh MODE
#   solo   the window, three bots, no network
#   lan    the window, LAN lobby, elect a host among whoever answers
#   host   headless host on the LAN; bots in empty seats
#   seat   headless bot that joins the LAN lobby
# All of them log to build/logs/MODE-STAMP.log at RUST_LOG=debug.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
MODE="${1:-solo}"
LOG="$LOGDIR/$MODE-$STAMP.log"
NET=(--name "${NAME:-player}" --game "${GAME:-treadcore}" --want "${WANT:-4}")
[ -n "${PEER:-}" ] && NET+=(--peer "$PEER")
case "$MODE" in
    solo|lan)
        [ "$HAVE_WINDOW" = 1 ] || die "the window cannot build here (see make configure); try make debug"
        say "building the window (release) -> $LOGDIR/build-$STAMP.log"
        cargo build --release -p treadcore-client 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
        BIN="$ROOT/target/release/treadcore"
        if [ "$MODE" = solo ]; then ARGS=(--solo --seed "${SEED:-0x7EADC0DE}"); else ARGS=(--lan "${NET[@]}" --seed "${SEED:-0x7EADC0DE}"); fi
        ;;
    host|seat)
        say "building headless (release) -> $LOGDIR/build-$STAMP.log"
        cargo build --release -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
        BIN="$ROOT/target/release/treadcore-server"
        ARGS=(--net "${NET[@]}" --seed "${SEED:-0x7EADC0DE}")
        [ "$MODE" = seat ] && ARGS+=(--port 0)
        ;;
    *) die "unknown mode $MODE" ;;
esac
say "$BIN ${ARGS[*]}  (log: $LOG)"
RUST_LOG="${RUST_LOG:-debug}" RUST_BACKTRACE=1 "$BIN" "${ARGS[@]}" 2>&1 | tee "$LOG"
