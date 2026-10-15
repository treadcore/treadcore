#!/usr/bin/env bash
# Four headless peers on this machine find each other over loopback
# broadcast, probe, elect a host, and play a short match. Each writes its
# own log; this script waits for all four and prints who hosted.
#
# INTRODUCER=1: no broadcast and no seed address; an introducer runs on
# this machine and is the only way the four can meet (roadmap step 7,
# as far as one machine proves it: the rendezvous, not the routers).
# REJOIN=1 (implies INTRODUCER=1): after REJOIN_AFTER seconds (20) the
# highest-numbered peer that is not hosting is killed outright and
# started again with the same identity file; it must find the host
# through the introducer, be welcomed back to its seat at a tick well
# past zero, and see the match end. (The first run of this killed peer 4
# blind, the election had made peer 4 the host, and the match died with
# it: a host that drops is step 8's problem, not this one's.)
# LATE=1 (implies INTRODUCER=1): after LATE_AFTER seconds (20) a fifth
# peer with an identity nobody has seen starts and asks in; the host
# must give it a bot's seat by order (roadmap step 8: a late arrival
# takes an empty bot seat), welcome it at a tick well past zero, and
# it must see the match end with the others, on the same world hash.
# LISTEN=near or LISTEN=far: a dedicated server (`--listen`, roadmap
# step 8) runs beside the four, and every peer is given its address.
# near: the server has the best links (the four have LAG, 30 ms by
# default, on every receive; the server none), so it must be elected,
# host all four as seats, write the match's trace and replay (which
# must replay with every hash matching), and answer its status page
# mid-match. far: the server has LISTEN_LAG (100 ms) on every receive
# and the four none, so it must lose, stay out of the match, and go
# back to listening; one of the four hosts, as without it.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
say "building headless (release)"
cargo build --release -p treadcore-server 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/release/treadcore-server"
[ "${REJOIN:-0}" = 1 ] && INTRODUCER=1
[ "${LATE:-0}" = 1 ] && INTRODUCER=1
LISTEN="${LISTEN:-}"
case "$LISTEN" in
    "") ;;
    # make exports LAG=0 when it is not given: near means 30 then.
    near) [ "${LAG:-0}" = 0 ] && LAG=30; SERVER_LAG=0 ;;
    far) SERVER_LAG="${LISTEN_LAG:-100}" ;;
    *) die "LISTEN is near or far, not '$LISTEN'" ;;
esac
INTRO_PID=
FIND=(--peer 127.0.0.1:47296)
if [ "${INTRODUCER:-0}" = 1 ]; then
    INTRO_PORT=47300
    INTRO_LOG="$LOGDIR/lobby-$STAMP-introducer.log"
    RUST_LOG=info "$BIN" --introduce --port "$INTRO_PORT" > "$INTRO_LOG" 2>&1 &
    INTRO_PID=$!
    echo "  introducer pid $INTRO_PID port $INTRO_PORT -> $INTRO_LOG"
    # No broadcast, no seed: the introducer or nothing.
    FIND=(--introducer "127.0.0.1:$INTRO_PORT" --no-beacon)
fi
SERVER_PID=
SERVER_LOG="$LOGDIR/lobby-$STAMP-server.log"
SERVER_DIR="$BUILDDIR/listen-$STAMP"
if [ -n "$LISTEN" ]; then
    SERVER_PORT=47305
    # shellcheck disable=SC2068
    TREADCORE_LAG_MS="$SERVER_LAG" RUST_LOG=info "$BIN" --listen "$SERVER_DIR" --name server --game "lobbytest-$STAMP" --port "$SERVER_PORT" --http "$SERVER_PORT" ${FIND[@]} --seconds "${LOBBY_SECONDS:-60}" --swarm 20 > "$SERVER_LOG" 2>&1 &
    SERVER_PID=$!
    echo "  server pid $SERVER_PID port $SERVER_PORT (udp and http), ${SERVER_LAG} ms lag -> $SERVER_LOG"
    # With an introducer, the server is found through it or not at all.
    [ "${INTRODUCER:-0}" = 1 ] || FIND+=(--peer "127.0.0.1:$SERVER_PORT")
fi
peer_args() {
    local i="$1"
    local PORT=$((47296 + i - 1))
    # Every peer is told about peer 1's port, so discovery works even where
    # loopback broadcast does not (or, with an introducer, nothing is).
    echo --net --name "peer$i" --game "lobbytest-$STAMP" --want 4 --port "$PORT" "${FIND[@]}" --seconds "${LOBBY_SECONDS:-60}" --swarm 20 --identity "$BUILDDIR/lobby-$STAMP-peer$i.id"
}
PIDS=()
for i in 1 2 3 4; do
    LOG="$LOGDIR/lobby-$STAMP-peer$i.log"
    # shellcheck disable=SC2046
    TREADCORE_LAG_MS="${LAG:-0}" RUST_LOG=info "$BIN" $(peer_args "$i") > "$LOG" 2>&1 &
    PIDS+=($!)
    echo "  peer$i pid $! port $((47296 + i - 1)) -> $LOG"
done
# Real time, since the host ticks at 20 Hz: a minute by default
# (LOBBY_SECONDS=n to change), not GAME_SECONDS, which sizes the trace.
HOW=""
[ "${LAG:-0}" != 0 ] && HOW="$HOW, ${LAG} ms of lag on every receive"
[ "${INTRODUCER:-0}" = 1 ] && HOW="$HOW, through an introducer"
[ "${REJOIN:-0}" = 1 ] && HOW="$HOW, a seat killed at ${REJOIN_AFTER:-20} s and restarted"
[ -n "$LISTEN" ] && HOW="$HOW, a --listen server $LISTEN"
[ "${LATE:-0}" = 1 ] && HOW="$HOW, a fifth peer arriving at ${LATE_AFTER:-20} s"
say "waiting for the match (${LOBBY_SECONDS:-60} game seconds, in real time$HOW)..."

VICTIM=
if [ "${REJOIN:-0}" = 1 ]; then
    sleep "${REJOIN_AFTER:-20}"
    # The highest-numbered seat that is not the host dies without a
    # word (no Leave): the host still has its seat.
    for i in 4 3 2 1; do
        grep -q 'hosting:' "$LOGDIR/lobby-$STAMP-peer$i.log" 2>/dev/null && continue
        VICTIM=$i
        break
    done
    [ -n "$VICTIM" ] || die "rejoin: no peer to kill (nobody is hosting?)"
    V=$((VICTIM - 1))
    kill -9 "${PIDS[$V]}" 2>/dev/null || true
    wait "${PIDS[$V]}" 2>/dev/null || true
    mv "$LOGDIR/lobby-$STAMP-peer$VICTIM.log" "$LOGDIR/lobby-$STAMP-killed$VICTIM.log"
    say "peer$VICTIM killed after ${REJOIN_AFTER:-20} s; starting it again with the same identity"
    # shellcheck disable=SC2046
    TREADCORE_LAG_MS="${LAG:-0}" RUST_LOG=info "$BIN" $(peer_args "$VICTIM") > "$LOGDIR/lobby-$STAMP-peer$VICTIM.log" 2>&1 &
    PIDS[$V]=$!
    echo "  peer$VICTIM again, pid ${PIDS[$V]} -> $LOGDIR/lobby-$STAMP-peer$VICTIM.log"
fi

if [ "${LATE:-0}" = 1 ]; then
    sleep "${LATE_AFTER:-20}"
    LOG="$LOGDIR/lobby-$STAMP-peer5.log"
    # shellcheck disable=SC2046
    TREADCORE_LAG_MS="${LAG:-0}" RUST_LOG=info "$BIN" $(peer_args 5) > "$LOG" 2>&1 &
    PIDS+=($!)
    say "peer5 arrives late, pid $! -> $LOG"
fi

STATUS_JSON=
if [ -n "$LISTEN" ]; then
    # Mid-match: what the status page says.
    sleep "${STATUS_AFTER:-25}"
    STATUS_JSON="$(curl -s --max-time 5 "http://127.0.0.1:$SERVER_PORT/status.json" || true)"
    echo "  status.json: $STATUS_JSON" | cut -c1-300
fi
FAIL=0
for p in "${PIDS[@]}"; do wait "$p" || FAIL=1; done
if [ -n "$SERVER_PID" ]; then
    # The server goes on listening; give a hosted match time to be saved.
    for _ in $(seq 20); do
        grep -q -E 'match 1 done|listening again' "$SERVER_LOG" && break
        sleep 0.5
    done
    sleep 1
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
fi
[ -n "$INTRO_PID" ] && { kill "$INTRO_PID" 2>/dev/null || true; }
rm -f "$BUILDDIR"/lobby-"$STAMP"-peer*.id
echo
for i in 1 2 3 4; do
    LOG="$LOGDIR/lobby-$STAMP-peer$i.log"
    grep -h 'lobby: decided' "$LOG" | sed "s/^/  peer$i: /" | sed 's/.*lobby: decided. //'
done
HOSTS="$( (grep -l 'hosting:' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
# A host alone is not a pass: the first run of this script "passed" while
# every client sat in Joining because Welcome never fit the receive
# buffer. Each of the three non-hosts must have been welcomed and seen
# the match end.
JOINED="$( (grep -l 'welcomed as player' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
ENDED="$( (grep -l 'match over; winner' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
# The hash: the host logs its world hash at tick 1000 and carries it in
# heartbeats; every client logs what it received. Four logs, one hash.
HASHES="$( (grep -h 'world hash at tick 1000:' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | sed 's/.*tick 1000: //' | sort | uniq -c | awk '{print $1":"$2}' | tr '\n' ' ')"
HASH_LOGS="$( (grep -l 'world hash at tick 1000:' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
HASH_DISTINCT="$( (grep -h 'world hash at tick 1000:' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | sed 's/.*tick 1000: //' | sort -u | wc -l | tr -d " ")"
# Prediction (roadmap step 4): every client seat runs its own tank ahead
# of the snapshots and checks it against them whenever none of its
# orders is in flight. Each logs one line at the end; the mismatches
# must be zero in all three.
grep -h 'prediction:' "$LOGDIR"/lobby-"$STAMP"-peer*.log | sed 's/.*prediction:/  prediction:/' || true
PREDICTED="$( (grep -l 'prediction:' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
MISMATCHES="$( (grep -h 'prediction:' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | sed 's/.* \([0-9]*\) mismatches.*/\1/' | awk '{s+=$1} END {print s+0}')"
echo "  hosts $HOSTS, welcomed $JOINED, saw the end $ENDED, hash at tick 1000 in $HASH_LOGS logs: $HASHES; prediction lines $PREDICTED, mismatches $MISMATCHES"
# The introducer (roadmap step 7): with no beacon and no seed address,
# every peer must have been introduced to the other three, and the
# host's log must say it registered as hosting.
INTRO_OK=1
if [ "${INTRODUCER:-0}" = 1 ]; then
    INTRODUCED="$(cat "$LOGDIR"/lobby-"$STAMP"-peer*.log "$LOGDIR"/lobby-"$STAMP"-killed*.log 2>/dev/null | grep -c 'introduced to' || true)"
    REGISTERED_HOST="$( (grep -l 'registered as hosting' "$LOGDIR"/lobby-"$STAMP"-peer*.log ${SERVER_PID:+"$SERVER_LOG"} || true) | wc -l | tr -d " ")"
    echo "  introducer: $INTRODUCED introductions logged across the peers, host registered as hosting in $REGISTERED_HOST log"
    if [ -n "$LISTEN" ]; then
        # With a server, a seat may hear of another seat in the server's
        # Hello before the introducer's word: what must hold is that
        # every seat found the server through the introducer.
        TO_SERVER="$( (grep -l 'introduced to .*, a server' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
        echo "  introducer: $TO_SERVER seats introduced to the server"
        [ "$TO_SERVER" = $((4 + ${LATE:-0})) ] && [ "$REGISTERED_HOST" = 1 ] || INTRO_OK=0
    else
        [ "$INTRODUCED" -ge 12 ] && [ "$REGISTERED_HOST" = 1 ] || INTRO_OK=0
    fi
fi
# Rejoin: the restarted peer was welcomed back to its old seat, at a
# tick past the kill, and the host says so.
REJOIN_OK=1
if [ "${REJOIN:-0}" = 1 ]; then
    KILLED="$LOGDIR/lobby-$STAMP-killed$VICTIM.log"
    AGAIN="$LOGDIR/lobby-$STAMP-peer$VICTIM.log"
    SEAT_BEFORE="$( (grep -h 'welcomed as player' "$KILLED" || true) | sed 's/.*welcomed as player \([0-9]*\) at tick \([0-9]*\).*/\1/' | head -1)"
    SEAT_AFTER="$( (grep -h 'welcomed as player' "$AGAIN" || true) | sed 's/.*welcomed as player \([0-9]*\) at tick \([0-9]*\).*/\1/' | head -1)"
    TICK_AFTER="$( (grep -h 'welcomed as player' "$AGAIN" || true) | sed 's/.*welcomed as player \([0-9]*\) at tick \([0-9]*\).*/\2/' | head -1)"
    GIVEN_BACK="$( (grep -h 'is back' "$LOGDIR"/lobby-"$STAMP"-peer*.log || true) | wc -l | tr -d " ")"
    NO_ELECTION="$(grep -c 'is hosting and answers; joining it' "$AGAIN" || true)"
    echo "  rejoin: peer$VICTIM had seat ${SEAT_BEFORE:-none}, back in seat ${SEAT_AFTER:-none} at tick ${TICK_AFTER:-none}; host gave a seat back $GIVEN_BACK time(s); joined without an election: $NO_ELECTION"
    [ -n "$SEAT_AFTER" ] && [ "$SEAT_AFTER" = "$SEAT_BEFORE" ] && [ "${TICK_AFTER:-0}" -gt 200 ] && [ "$GIVEN_BACK" = 1 ] && [ "$NO_ELECTION" = 1 ] || REJOIN_OK=0
fi
# A late arrival: peer5 joined the host without an election, into a
# bot's seat (3 and up: a headless host seats the three remote peers as
# 0 to 2, and its own seat is a bot's), well past tick zero, and the
# host said so.
LATE_OK=1
LATE_SEATS=0
if [ "${LATE:-0}" = 1 ]; then
    LATE_SEATS=1
    L5="$LOGDIR/lobby-$STAMP-peer5.log"
    SEAT5="$( (grep -h 'welcomed as player' "$L5" || true) | sed 's/.*welcomed as player \([0-9]*\) at tick \([0-9]*\).*/\1/' | head -1)"
    TICK5="$( (grep -h 'welcomed as player' "$L5" || true) | sed 's/.*welcomed as player \([0-9]*\) at tick \([0-9]*\).*/\2/' | head -1)"
    NO_ELECTION5="$(grep -c 'is hosting and answers; joining it' "$L5" || true)"
    TAKEN="$( (grep -h 'SEAT TAKEN' "$LOGDIR"/lobby-"$STAMP"-peer[1-4].log ${SERVER_PID:+"$SERVER_LOG"} || true) | wc -l | tr -d " ")"
    echo "  late: peer5 welcomed as player ${SEAT5:-none} at tick ${TICK5:-none}; joined without an election: $NO_ELECTION5; the host logged $TAKEN seat taken"
    [ -n "$SEAT5" ] && [ "$SEAT5" -ge 3 ] && [ "${TICK5:-0}" -gt 200 ] && [ "$NO_ELECTION5" = 1 ] && [ "$TAKEN" = 1 ] || LATE_OK=0
fi
# The server (roadmap step 8). near: it hosted, and the four are all
# seats; far: it was not elected and one of the four hosted.
SEATS=$((3 + LATE_SEATS))
HASH_WANT=$((4 + LATE_SEATS))
SERVER_OK=1
if [ "$LISTEN" = near ]; then
    SEATS=$((4 + LATE_SEATS))
    HASH_WANT=$((5 + LATE_SEATS))
    HOSTS="$(grep -c 'hosting:' "$SERVER_LOG" || true)"
    grep -h 'world hash at tick 1000:' "$SERVER_LOG" >/dev/null || SERVER_OK=0
    HASH_LOGS=$((HASH_LOGS + $(grep -c 'world hash at tick 1000:' "$SERVER_LOG" || true)))
    HASH_DISTINCT="$( (cat "$LOGDIR"/lobby-"$STAMP"-peer*.log "$SERVER_LOG" | grep -h 'world hash at tick 1000:' || true) | sed 's/.*tick 1000: //' | sort -u | wc -l | tr -d " ")"
    REPLAY_OK=0
    if [ -f "$SERVER_DIR/match-0001.replay.json" ] && [ -s "$SERVER_DIR/match-0001.trace.jsonl" ]; then
        "$BIN" --replay "$SERVER_DIR/match-0001.replay.json" > "$LOGDIR/lobby-$STAMP-server-replay.log" 2>&1 && REPLAY_OK=1
    fi
    LIVE="$(printf '%s' "$STATUS_JSON" | grep -c '"state":"hosting"' || true)"
    echo "  server: hosted $HOSTS, replay of match-0001 matched $REPLAY_OK, status page said hosting $LIVE"
    [ "$HOSTS" = 1 ] && [ "$REPLAY_OK" = 1 ] && [ "$LIVE" = 1 ] || SERVER_OK=0
elif [ "$LISTEN" = far ]; then
    LOST="$(grep -c 'not elected:' "$SERVER_LOG" || true)"
    SERVED="$(grep -c 'hosting:' "$SERVER_LOG" || true)"
    LISTENING="$(printf '%s' "$STATUS_JSON" | grep -c '"elected_elsewhere":1' || true)"
    echo "  server: not elected $LOST, hosted $SERVED, status page counted it $LISTENING"
    [ "$LOST" = 1 ] && [ "$SERVED" = 0 ] && [ "$LISTENING" = 1 ] || SERVER_OK=0
fi
if [ "$HOSTS" = 1 ] && [ "$JOINED" = "$SEATS" ] && [ "$ENDED" = "$SEATS" ] && [ "$HASH_LOGS" = "$HASH_WANT" ] && [ "$HASH_DISTINCT" = 1 ] && [ "$PREDICTED" = "$SEATS" ] && [ "$MISMATCHES" = 0 ] && [ "$FAIL" = 0 ] && [ "$INTRO_OK" = 1 ] && [ "$REJOIN_OK" = 1 ] && [ "$SERVER_OK" = 1 ] && [ "$LATE_OK" = 1 ]; then
    PASSED=""
    [ "${INTRODUCER:-0}" = 1 ] && PASSED="$PASSED, met through the introducer alone"
    [ "${REJOIN:-0}" = 1 ] && PASSED="$PASSED, peer$VICTIM back in its seat"
    [ "${LATE:-0}" = 1 ] && PASSED="$PASSED, peer5 arrived late and took bot seat $SEAT5"
    [ "$LISTEN" = near ] && PASSED="$PASSED; the --listen server elected, hosting all four, its replay matched"
    [ "$LISTEN" = far ] && PASSED="$PASSED; the far --listen server not elected"
    say "lobby-test: PASS (one host, $SEATS clients welcomed, all saw the match end, one world hash at tick 1000 in all $HASH_WANT logs, no prediction mismatches$PASSED)"
else
    die "lobby-test: FAIL (hosts $HOSTS, welcomed $JOINED, ended $ENDED, hash logs $HASH_LOGS distinct $HASH_DISTINCT, prediction lines $PREDICTED mismatches $MISMATCHES, introducer ok $INTRO_OK, rejoin ok $REJOIN_OK, server ok $SERVER_OK, late ok $LATE_OK, exit status $FAIL); see $LOGDIR/lobby-$STAMP-*.log"
fi


