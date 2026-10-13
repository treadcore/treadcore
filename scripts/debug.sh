#!/usr/bin/env bash
# The debug trace: a bot-only match, every log level on, backtraces on,
# the event stream as JSON lines beside it. Reads top to bottom as the
# story of a fight; greps as data. Both land under build/logs.
#
#   GAME_SECONDS=600 make debug   a longer match
#   SEED=0x1234 make debug      a different map and dice
#   BRAIN='python3 bot.py' make debug   an external JSON brain in seat 0
#   PLAYERS=4 make debug        four bots instead of sixteen
#   DEFENDERS=12 make debug     twelve of them dig in; four attack
#   MAP=maps/x.map make debug   on a map file (documentation/MAPS.md)

. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
LOG="$LOGDIR/debug-$STAMP.log"
TRACE="$LOGDIR/trace-$STAMP.jsonl"
REPLAY="$LOGDIR/replay-$STAMP.json"
say "building headless (debug profile) -> $LOGDIR/build-$STAMP.log"
cargo build -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/debug/treadcore-server"
ARGS=(--local --seed "${SEED:-0x7EADC0DE}" --seconds "${GAME_SECONDS:-180}" --players "${PLAYERS:-16}" --defenders "${DEFENDERS:-0}" --trace "$TRACE" --record "$REPLAY")
[ -n "${BRAIN:-}" ] && ARGS+=(--brain "$BRAIN")
[ -n "${MAP:-}" ] && ARGS+=(--map "$MAP")
say "tracing: $BIN ${ARGS[*]}"
say "  log   $LOG"
say "  trace $TRACE"
say "  replay $REPLAY   (play it back: make replay REPLAY=$REPLAY)"
{
    echo "# treadcore debug trace $STAMP"
    echo "# $BIN ${ARGS[*]}"
    echo "# rustc $RUST_VER, $(uname -srm)"
    echo
} > "$LOG"
RUST_LOG="${RUST_LOG:-trace}" RUST_BACKTRACE="${RUST_BACKTRACE:-1}" \
    "$BIN" "${ARGS[@]}" 2>&1 | tee -a "$LOG" | grep -E ' (WARN|ERROR) |^---- |^  p[0-9]' || true
echo
say "done. $(wc -l < "$LOG") log lines, $(wc -l < "$TRACE") events."
say "  less $LOG"
say "  grep -c Intercept $TRACE"
say "  make replay REPLAY=$REPLAY"

