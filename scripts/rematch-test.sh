#!/usr/bin/env bash
# The window's rematch over the network (PLAYABILITY item 10, roadmap
# step 8): a headless host and the window as its client play a 20-second
# match; the host leaves, as a host does when its match ends; Enter on
# the window's match-over screen takes it back to the lobby on the same
# port, where a second headless host is waiting; it must be elected and
# the window seated and welcomed again, and the second match must end.
#
# The hosts take the smallest peer id and the window carries 30 ms of
# simulated lag, so a host wins the election. Needs the shoot machine
# (scripts/shoot.sh): an X display, xdotool, grim, hyprctl. The window
# is given the focus just before the Enter: the first run sent it while
# another window had the focus, and nothing happened.
. "$(dirname "$0")/common.sh"
need_config
for t in xdotool grim hyprctl python3; do command -v "$t" >/dev/null || die "rematch-test needs $t"; done
export DISPLAY="${DISPLAY:-:0}"
BIN="$ROOT/target/release/treadcore"; SRV="$ROOT/target/release/treadcore-server"
[ -x "$BIN" ] && [ -x "$SRV" ] || die "no release window or server: make build first"
S="$BUILDDIR/rematch"; rm -rf "$S"; mkdir -p "$S"
printf '0000000000000001' > "$S/host.id"
host() { "$SRV" --net --name "host$1" --game rematch --want 2 --players 4 --seconds 20 --port 47320 --peer 127.0.0.1:47296 --no-beacon --identity "$S/host.id" > "$S/host$1.log" 2>&1 & }
say "rematch-test: logs in $S"
host 1; H1=$!
WAYLAND_DISPLAY= LD_PRELOAD="${PRELOAD:-}" TREADCORE_LAG_MS=30 RUST_LOG=info "$BIN" --lan --name you --game rematch --want 2 --seconds 20 --peer 127.0.0.1:47320 --no-beacon --identity "$S/window.id" --keys "$S/keys" > "$S/window.log" 2>&1 &
WP=$!
trap 'kill $H1 ${H2:-} $WP 2>/dev/null || true' EXIT
for i in $(seq 1 100); do hyprctl clients -j | python3 -c "import json,sys; sys.exit(0 if any(c['pid']==$WP for c in json.load(sys.stdin)) else 1)" && break; sleep 0.3; done
hyprctl dispatch togglefloating "pid:$WP" >/dev/null
hyprctl dispatch resizewindowpixel "exact 1200 740,pid:$WP" >/dev/null
for i in $(seq 1 90); do grep -q 'match over' "$S/host1.log" && break; sleep 1; done
grep -q 'match over' "$S/host1.log" || die "round 1 never ended: see $S/host1.log"
grep -q 'welcomed as player' "$S/window.log" || die "the window never joined round 1: see $S/window.log"
sleep 4
host 2; H2=$!
hyprctl dispatch focuswindow "pid:$WP" >/dev/null; sleep 0.5
xdotool keydown Return; sleep 0.3; xdotool keyup Return
for i in $(seq 1 75); do grep -q 'match over' "$S/host2.log" && break; sleep 1; done
grep -q 'again: back to the lobby' "$S/window.log" || die "Enter did not take the window back to the lobby: see $S/window.log"
[ "$(grep -c 'welcomed as player' "$S/window.log")" -ge 2 ] || die "the window was not welcomed by the second host: see $S/window.log"
grep -q "seat 0 'you' joined" "$S/host2.log" || die "the second host never seated the window: see $S/host2.log"
grep -q 'match over' "$S/host2.log" || die "round 2 never ended: see $S/host2.log"
say "rematch-test passed: the window played round 1, went back to the lobby on Enter, and was seated for round 2"
