#!/usr/bin/env bash
# replay.sh FILE -- play a recorded match back and check its hashes.
#
# A replay is what `make debug` writes beside its trace: the scenario,
# every tick's orders, and the world hash every hundred ticks. Playing
# it back after a change to treadcore-sim says whether the change did
# only what it meant to: every checkpoint matches, or the first that
# does not is named with both hashes. Exit status 1 on a mismatch, so
# a directory of replays is a regression suite:
#
#   for r in build/replays/*.json; do scripts/replay.sh "$r" || exit 1; done
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
FILE="${1:-}"
[ -n "$FILE" ] || FILE="$(ls -t "$LOGDIR"/replay-*.json 2>/dev/null | head -1)"
[ -n "$FILE" ] && [ -f "$FILE" ] || die "usage: make replay REPLAY=build/logs/replay-STAMP.json (none found)"
LOG="$LOGDIR/replay-$STAMP.log"
say "building headless (debug profile)"
cargo build -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/debug/treadcore-server"
say "replaying $FILE -> $LOG"
set +e
RUST_LOG="${RUST_LOG:-info}" "$BIN" --replay "$FILE" 2>&1 | tee "$LOG" | grep -E 'replay:|MISMATCH|---- match|<- winner' 
STATUS="${PIPESTATUS[0]}"
set -e
if [ "$STATUS" = 0 ]; then say "replay: PASS"; else die "replay: FAIL (exit $STATUS); see $LOG"; fi
