#!/usr/bin/env bash
# Four headless peers on this machine find each other over loopback
# broadcast, probe, elect a host, and play a short match. Each writes its
# own log; this script waits for all four and prints who hosted.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
say "building headless (release)"
cargo build --release -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/release/treadcore-server"
PIDS=()
for i in 1 2 3 4; do
    LOG="$LOGDIR/lobby-$STAMP-peer$i.log"
    PORT=$((47296 + i - 1))
    # Every peer is told about peer 1's port, so discovery works even where
    # loopback broadcast does not.
    RUST_LOG=info "$BIN" --net --name "peer$i" --game "lobbytest-$STAMP" --want 4 --port "$PORT" --peer 127.0.0.1:47296 --seconds "${LOBBY_SECONDS:-60}" --swarm 20 > "$LOG" 2>&1 &
    PIDS+=($!)
    echo "  peer$i pid $! port $PORT -> $LOG"
done
# Real time, since the host ticks at 20 Hz: a minute by default
# (LOBBY_SECONDS=n to change), not GAME_SECONDS, which sizes the trace.
say "waiting for the match (${LOBBY_SECONDS:-60} game seconds, in real time)..."

FAIL=0
for p in "${PIDS[@]}"; do wait "$p" || FAIL=1; done
echo
for i in 1 2 3 4; do
    LOG="$LOGDIR/lobby-$STAMP-peer$i.log"
    grep -h 'lobby: decided' "$LOG" | sed "s/^/  peer$i: /" | sed 's/.*lobby: decided. //'
done
HOSTS="$(grep -l 'hosting:' "$LOGDIR"/lobby-"$STAMP"-peer*.log | wc -l)"
# A host alone is not a pass: the first run of this script "passed" while
# every client sat in Joining because Welcome never fit the receive
# buffer. Each of the three non-hosts must have been welcomed and seen
# the match end.
JOINED="$(grep -l 'welcomed as player' "$LOGDIR"/lobby-"$STAMP"-peer*.log | wc -l)"
ENDED="$(grep -l 'match over; winner' "$LOGDIR"/lobby-"$STAMP"-peer*.log | wc -l)"
# The hash: the host logs its world hash at tick 1000 and carries it in
# heartbeats; every client logs what it received. Four logs, one hash.
HASHES="$(grep -h 'world hash at tick 1000:' "$LOGDIR"/lobby-"$STAMP"-peer*.log | sed 's/.*tick 1000: //' | sort | uniq -c | awk '{print $1":"$2}' | tr '\n' ' ')"
HASH_LOGS="$(grep -l 'world hash at tick 1000:' "$LOGDIR"/lobby-"$STAMP"-peer*.log | wc -l)"
HASH_DISTINCT="$(grep -h 'world hash at tick 1000:' "$LOGDIR"/lobby-"$STAMP"-peer*.log | sed 's/.*tick 1000: //' | sort -u | wc -l)"
echo "  hosts $HOSTS, welcomed $JOINED, saw the end $ENDED, hash at tick 1000 in $HASH_LOGS logs: $HASHES"
if [ "$HOSTS" = 1 ] && [ "$JOINED" = 3 ] && [ "$ENDED" = 3 ] && [ "$HASH_LOGS" = 4 ] && [ "$HASH_DISTINCT" = 1 ] && [ "$FAIL" = 0 ]; then
    say "lobby-test: PASS (one host, three clients welcomed, all saw the match end, one world hash at tick 1000 in all four logs)"
else
    die "lobby-test: FAIL (hosts $HOSTS, welcomed $JOINED, ended $ENDED, hash logs $HASH_LOGS distinct $HASH_DISTINCT, exit status $FAIL); see $LOGDIR/lobby-$STAMP-peer*.log"
fi


