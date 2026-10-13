#!/usr/bin/env bash
# Four windows on this machine, each a player, networked: they meet in a
# LAN lobby over loopback, elect a host among themselves, and play one
# match, the windows tiled two by two on the screen. The windowed twin of
# make lobby-test, for seeing what four seats see.
#
# CLIENTS=n for another count (2..4 tile; more stack in the last cell),
# PLAYERS=n seats in all (bots fill the rest), MATCH_SECONDS for the match
# length (unset: the game's own). Each client logs to
# build/logs/quad-STAMP-clientN.log and keeps its id in
# build/quad-clientN.id, so a client started again gets its seat back.
# Ctrl-C closes all of them.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
[ "$HAVE_WINDOW" = 1 ] || die "the window cannot build here (see make configure); try make lobby-test"
N="${CLIENTS:-4}"
[ "$N" -ge 2 ] 2>/dev/null || die "CLIENTS=$N: two or more"
say "building the window (release) -> $LOGDIR/build-$STAMP.log"
cargo build --release -p treadcore-client 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/release/treadcore"

# The screen, in logical pixels, and its scale: winit takes a window's
# size in logical pixels but its position in physical ones.
SW=1920 SH=1080 SCALE=1 TOP=0
if [ "$(uname -s)" = Darwin ]; then
    B="$(osascript -e 'tell application "Finder" to get bounds of window of desktop' 2>/dev/null | tr -d ' ')"
    [ -n "$B" ] && { SW="$(echo "$B" | cut -d, -f3)"; SH="$(echo "$B" | cut -d, -f4)"; }
    PHYS="$(system_profiler SPDisplaysDataType 2>/dev/null | sed -n 's/.*Resolution:.*(\([0-9]*\) x.*/\1/p' | head -1)"
    [ -n "$PHYS" ] && SCALE=$(( (PHYS + SW / 2) / SW ))
    [ "$SCALE" -ge 1 ] || SCALE=1
    TOP=25    # the menu bar
elif command -v xrandr >/dev/null && [ -n "${DISPLAY:-}" ]; then
    R="$(xrandr --current 2>/dev/null | sed -n 's/.*current \([0-9]*\) x \([0-9]*\).*/\1 \2/p' | head -1)"
    [ -n "$R" ] && read -r SW SH <<<"$R"
fi
TITLE=28      # the title bar the window manager adds above the content
CW=$(( SW / 2 ))
CH=$(( (SH - TOP) / 2 ))
echo "  screen       ${SW}x${SH} (scale $SCALE): ${N} windows of $((CW))x$((CH - TITLE))"

GAMEID="quad-$STAMP"
PIDS=()
trap 'kill "${PIDS[@]}" 2>/dev/null || true' INT TERM
for i in $(seq 1 "$N"); do
    c=$(( (i - 1) % 2 )); r=$(( (i - 1) / 2 )); [ "$r" -gt 1 ] && r=1
    X=$(( c * CW * SCALE )); Y=$(( (TOP + r * CH) * SCALE ))
    LOG="$LOGDIR/quad-$STAMP-client$i.log"
    ARGS=(--lan --name "client$i" --game "$GAMEID" --want "$N" --players "${PLAYERS:-$N}" --teams "${TEAMS:-0}"
          --seed "${SEED:-0x7EADC0DE}" --peer 127.0.0.1:47296 --identity "$BUILDDIR/quad-client$i.id"
          --window "$X,$Y,$CW,$((CH - TITLE))")
    [ -n "${MAP:-}" ] && ARGS+=(--map "$MAP")
    [ -n "${MATCH_SECONDS:-}" ] && ARGS+=(--seconds "$MATCH_SECONDS")
    RUST_LOG="${RUST_LOG:-info}" RUST_BACKTRACE=1 "$BIN" "${ARGS[@]}" > "$LOG" 2>&1 &
    PIDS+=($!)
    echo "  client$i pid $! -> $LOG"
    # One at a time, so the first holds the well-known port the rest seed from.
    sleep 1
done
say "$N windows up; the lobby elects a host once all $N have met. Close any window or Ctrl-C to end."
for p in "${PIDS[@]}"; do wait "$p" 2>/dev/null || true; done
trap - INT TERM
echo
for i in $(seq 1 "$N"); do
    LOG="$LOGDIR/quad-$STAMP-client$i.log"
    printf '  client%s: ' "$i"
    { grep -ahoE 'hosting:.*|hosting for [0-9]+ peer\(s\)|welcomed as player [0-9]+|prediction: .*' "$LOG" || echo 'no seat (lobby never decided?)'; } | sed 's/\x1b\[[0-9;]*m//g' | tr '\n' ' '
    echo
done
