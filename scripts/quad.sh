#!/usr/bin/env bash
# Windows on this machine, each a player, networked: they meet in a LAN
# lobby over loopback, elect a host among themselves, and play one
# match, the windows tiled two by two on the screen. The windowed twin of
# make lobby-test, for seeing what the seats see.
#
# By default three windows, one a team: each is its team's commander on
# the autopilot (the side's fleet commander, goal.rs), the teams filled
# with bots to fifteen seats (a guard, rangers and a wingman each), on
# the naval map, respawns without limit. Watch the three commanders play
# each other; the host's log has every seat's line every 30 s.
#
# CLIENTS=n for another count (2..4 tile two by two; more tile in a
# square-ish grid, 16 as four by four; under Hyprland, which tiles
# windows itself and ignores the position a window asks for, each is
# floated and placed with hyprctl once it maps),
# PLAYERS=n seats in all (bots fill the rest), TEAMS=n, AUTOPILOT=0 to
# drive yourselves, RESPAWNS=n|unlimited, MATCH_SECONDS for the match
# length (unset: the game's own). Each client logs to
# build/logs/quad-STAMP-clientN.log and keeps its id in
# build/quad-clientN.id, so a client started again gets its seat back.
# Ctrl-C closes all of them.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
[ "$HAVE_WINDOW" = 1 ] || die "the window cannot build here (see make configure); try make lobby-test"
N="${CLIENTS:-3}"
[ "$N" -ge 2 ] 2>/dev/null || die "CLIENTS=$N: two or more"
say "building the window (release) -> $LOGDIR/build-$STAMP.log"
cargo build --release -p treadcore-client 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"
BIN="$ROOT/target/release/treadcore"
# musl and lavapipe: the window's thread wants a bigger stack (configure.sh).
[ -n "${PRELOAD:-}" ] && export LD_PRELOAD="$PRELOAD"

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
elif [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] && command -v hyprctl >/dev/null; then
    # The focused monitor, in logical pixels: hyprctl gives its mode in
    # physical ones and its scale beside it.
    M="$(hyprctl monitors -j 2>/dev/null | python3 -c 'import json,sys
m=[m for m in json.load(sys.stdin) if m.get("focused")] or [None]
m=m[0]
if m: print(int(m["width"]/m["scale"]), int(m["height"]/m["scale"]), m["x"], m["y"])' 2>/dev/null)"
    [ -n "$M" ] && read -r SW SH MX MY <<<"$M"
    HYPR=1
elif command -v xrandr >/dev/null && [ -n "${DISPLAY:-}" ]; then
    R="$(xrandr --current 2>/dev/null | sed -n 's/.*current \([0-9]*\) x \([0-9]*\).*/\1 \2/p' | head -1)"
    [ -n "$R" ] && read -r SW SH <<<"$R"
fi
TITLE=28      # the title bar the window manager adds above the content
[ "${HYPR:-0}" = 1 ] && TITLE=0    # Hyprland draws none
# Two by two up to four; past that the smallest square-ish grid that holds
# them all, columns first: 16 is four by four, 9 three by three.
COLS=2 ROWS=2
if [ "$N" -gt 4 ]; then
    COLS=1; while [ $(( COLS * COLS )) -lt "$N" ]; do COLS=$(( COLS + 1 )); done
    ROWS=$(( (N + COLS - 1) / COLS ))
fi
CW=$(( SW / COLS ))
CH=$(( (SH - TOP) / ROWS ))
echo "  screen       ${SW}x${SH} (scale $SCALE): ${N} windows of $((CW))x$((CH - TITLE)), ${COLS}x${ROWS}"

# Hyprland: wait for the window to map, then float it into its cell.
# Positions here are logical layout pixels, not the physical ones winit
# was given. In the background, so a slow window holds up no other.
place() {
    for _ in $(seq 1 120); do
        hyprctl clients -j 2>/dev/null | python3 -c "import json,sys; sys.exit(0 if any(c['pid']==$1 for c in json.load(sys.stdin)) else 1)" && break
        sleep 0.5
    done
    hyprctl dispatch setfloating "pid:$1" >/dev/null
    hyprctl dispatch resizewindowpixel "exact $4 $5,pid:$1" >/dev/null
    hyprctl dispatch movewindowpixel "exact $2 $3,pid:$1" >/dev/null
}

GAMEID="quad-$STAMP"
PIDS=()
trap 'kill "${PIDS[@]}" 2>/dev/null || true' INT TERM
for i in $(seq 1 "$N"); do
    c=$(( (i - 1) % COLS )); r=$(( (i - 1) / COLS ))
    if [ "$N" -le 4 ] && [ "$r" -gt 1 ]; then r=1; fi
    X=$(( c * CW * SCALE )); Y=$(( (TOP + r * CH) * SCALE ))
    LOG="$LOGDIR/quad-$STAMP-client$i.log"
    ARGS=(--lan --name "client$i" --game "$GAMEID" --want "$N" --players "${PLAYERS:-15}" --teams "${TEAMS:-3}"
          --seed "${SEED:-0x7EADC0DE}" --peer 127.0.0.1:47296 --identity "$BUILDDIR/quad-client$i.id"
          --window "$X,$Y,$CW,$((CH - TITLE))")
    [ -n "${MAP:-}" ] && ARGS+=(--map "$MAP")
    [ -n "${MATCH_SECONDS:-}" ] && ARGS+=(--seconds "$MATCH_SECONDS")
    [ -n "${RESPAWNS:-}" ] && ARGS+=(--respawns "$RESPAWNS")
    [ "${AUTOPILOT:-1}" = 1 ] && ARGS+=(--autopilot)
    RUST_LOG="${RUST_LOG:-info}" RUST_BACKTRACE=1 "$BIN" "${ARGS[@]}" > "$LOG" 2>&1 &
    PIDS+=($!)
    echo "  client$i pid $! -> $LOG"
    [ "${HYPR:-0}" = 1 ] && place "$!" "$(( ${MX:-0} + c * CW ))" "$(( ${MY:-0} + TOP + r * CH ))" "$CW" "$CH" &
    # The first alone, so it holds the well-known port the rest seed from;
    # the rest close behind it. Each looks for peers for 8 s (lobby.rs,
    # discover) and then settles with whoever it found, so a second
    # apart was fine at four and left 13 to 16 each alone at sixteen.
    if [ "$i" = 1 ]; then sleep 1; else sleep 0.25; fi
done
say "$N windows up; the lobby elects a host once all $N have met. Close any window or Ctrl-C to end."
for p in "${PIDS[@]}"; do wait "$p" 2>/dev/null || true; done
trap - INT TERM
echo
for i in $(seq 1 "$N"); do
    LOG="$LOGDIR/quad-$STAMP-client$i.log"
    printf '  client%s: ' "$i"
    { grep -ahoE 'hosting:.*|hosting for [0-9]+ peer\(s\)|welcomed as player [0-9]+|prediction: .*' "$LOG" || echo 'no seat (lobby never decided?)'; } | sed 's/\x1b\[[0-9;]*m//g' | tr '\n' ' '
    grep -ahoE 'pilot at [0-9]+s:.*' "$LOG" | tail -1 | sed 's/\x1b\[[0-9;]*m//g'
    echo
done
