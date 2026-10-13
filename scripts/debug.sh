#!/usr/bin/env bash
# The debug trace: a bot-only match, every log level on, backtraces on,
# the event stream as JSON lines beside it. Reads top to bottom as the
# story of a fight; greps as data. Both land under build/logs.
#
#   GAME_SECONDS=600 make debug   a longer match
#   SEED=0x1234 make debug      a different map and dice
#   BRAIN='python3 bot.py' make debug   an external JSON brain in seat 0
#   PLAYERS=4 make debug        four bots instead of sixteen
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
LOG="$LOGDIR/debug-$STAMP.log"
TRACE="$LOGDIR/trace-$STAMP.jsonl"
say "building headless (debug profile) -> $LOGDIR/build-$STAMP.log"
cargo build -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/debug/treadcore-server"
ARGS=(--local --seed "${SEED:-0x7EADC0DE}" --seconds "${GAME_SECONDS:-180}" --players "${PLAYERS:-16}" --trace "$TRACE")
[ -n "${BRAIN:-}" ] && ARGS+=(--brain "$BRAIN")
say "tracing: $BIN ${ARGS[*]}"
say "  log   $LOG"
say "  trace $TRACE"
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
