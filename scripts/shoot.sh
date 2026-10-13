#!/usr/bin/env bash
# shoot.sh: the release frames. Runs the window solo on a prior-art
# map, drives it with xdotool, and takes the set of pictures the
# player's guide (documentation/HOW-TO-PLAY.md) and the site show,
# each cropped to the thing it shows. Writes reports/images/guide-*.png
# and reports/images/MANIFEST-<version>.txt listing them, which
# scripts/release.sh checks against the documents.
#
# The second half shoots the network: an introducer, a headless host
# and a bot seat on this machine with the window as a client.
#
# Needs the shoot machine: an X display (Xwayland under Hyprland here,
# DISPLAY=:0), xdotool, grim, hyprctl, python3 with Pillow. The window
# is run under X11 because xdotool drives X11 windows (journal 0013).
. "$(dirname "$0")/common.sh"
need_config
for t in xdotool grim hyprctl python3; do command -v "$t" >/dev/null || die "shoot needs $t"; done
python3 -c 'import PIL' 2>/dev/null || die "shoot needs python3 Pillow"
export DISPLAY="${DISPLAY:-:0}"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
OUT="$ROOT/reports/images"
MAP="${MAP:-$ROOT/maps/prior-art/constanine.map}"
W=1200; H=740; X=40; Y=40
S="$BUILDDIR/shoot"; mkdir -p "$S"
BIN="$ROOT/target/release/treadcore"
[ -x "$BIN" ] || die "no release window: make build first"

say "shoot $VERSION on $(basename "$MAP") -> $OUT/guide-*.png"
WAYLAND_DISPLAY= LD_PRELOAD="${PRELOAD:-}" RUST_LOG=info "$BIN" --solo --seed "${SEED:-0x7EADC0DE}" --players 8 --map "$MAP" > "$LOGDIR/shoot-$STAMP.log" 2>&1 &
PID=$!
trap 'kill $PID 2>/dev/null || true' EXIT
sleep 14
kill -0 "$PID" 2>/dev/null || die "the window died: see $LOGDIR/shoot-$STAMP.log"
hyprctl dispatch togglefloating "pid:$PID" >/dev/null
hyprctl dispatch resizewindowpixel "exact $W $H,pid:$PID" >/dev/null
hyprctl dispatch movewindowpixel "exact $X $Y,pid:$PID" >/dev/null
hyprctl dispatch focuswindow "pid:$PID" >/dev/null
sleep 5
frame() { grim -g "$X,$Y ${W}x$H" "$S/$1.png"; }
# A key held for a moment: a press and release inside one frame of a
# software rasteriser can be missed.
key() { xdotool keydown "$1"; sleep 0.2; xdotool keyup "$1"; }

xdotool mousemove 700 420
frame screen                                   # the whole frame, idle
key 7; sleep 0.6; xdotool mousemove 740 300; sleep 0.4
frame armed                                    # pillbox mode armed, the square at the cursor
xdotool mousemove 700 420; sleep 0.3; xdotool click 1; sleep 1.5
frame squad-out                                # the squad sent, on the grass by the depot
key 4; sleep 0.4; xdotool mousemove 660 440; sleep 0.3; xdotool click 1; sleep 1.0
frame refused                                  # farm on grass: the refusal held (the mode dropped with the click; Esc would quit)
xdotool mousemove 820 500; xdotool mousedown 3; sleep 0.6
frame aim                                      # the right button held: the gun's envelope
xdotool mouseup 3; sleep 2
key Tab; sleep 0.8
frame events                                   # the scrollback over the map
key Tab; sleep 0.3
xdotool mousemove 580 380; for i in 1 2 3 4 5 6; do xdotool click 4; sleep 0.1; done; sleep 2
frame close                                    # the models, close
kill "$PID" 2>/dev/null || die "the window died during the shoot: see $LOGDIR/shoot-$STAMP.log"; trap - EXIT

# The network, on this one machine, the way journal 0013 proved it: an
# introducer, a headless host whose identity is the smallest id (it wins
# any tie), the window, and a headless bot seat, no LAN broadcast. The
# window and the seat carry 30 ms of simulated lag each way, so the
# lagless host measures best and is elected. The lobby waits for three
# peers: the window comes up second, so its lobby screen is on while the
# seat has not yet arrived, and that is the frame.
SERVER="$ROOT/target/release/treadcore-server"
[ -x "$SERVER" ] || die "no release server: make build first"
NP=47310; NET="$S/net"; mkdir -p "$NET"
printf '0000000000000001' > "$NET/host.id"
"$SERVER" --introduce --port "$NP" > "$LOGDIR/shoot-introducer-$STAMP.log" 2>&1 &
P_INTRO=$!
"$SERVER" --net --want 3 --introducer "127.0.0.1:$NP" --no-beacon --identity "$NET/host.id" --players 16 --port 0 > "$LOGDIR/shoot-host-$STAMP.log" 2>&1 &
P_HOST=$!
trap 'kill $P_INTRO $P_HOST ${P_SEAT:-} ${PID:-} 2>/dev/null || true' EXIT
sleep 1
WAYLAND_DISPLAY= LD_PRELOAD="${PRELOAD:-}" TREADCORE_LAG_MS=30 RUST_LOG=info "$BIN" --lan --name you --want 3 --introducer "127.0.0.1:$NP" --no-beacon --identity "$NET/window.id" > "$LOGDIR/shoot-window-net-$STAMP.log" 2>&1 &
PID=$!
# The lobby decides within eight seconds of the process starting, and on
# a software rasteriser the window's first frame comes about as late:
# so the lobby is caught as the window appears, at whatever size the
# compositor gave it, in a burst, before it is floated and sized.
geom=""
for i in $(seq 1 100); do
    geom="$(hyprctl clients -j | python3 -c "import json,sys; c=[c for c in json.load(sys.stdin) if c['pid']==$PID]; print('%d,%d %dx%d' % (c[0]['at'][0], c[0]['at'][1], c[0]['size'][0], c[0]['size'][1]) if c else '')")"
    [ -n "$geom" ] && break
    sleep 0.2
done
[ -n "$geom" ] || die "the networked window never appeared: see $LOGDIR/shoot-window-net-$STAMP.log"
for i in $(seq 1 16); do grim -g "$geom" "$S/lobby-$i.png"; sleep 0.5; done
kill -0 "$PID" 2>/dev/null || die "the networked window died: see $LOGDIR/shoot-window-net-$STAMP.log"
hyprctl dispatch togglefloating "pid:$PID" >/dev/null
hyprctl dispatch resizewindowpixel "exact $W $H,pid:$PID" >/dev/null
hyprctl dispatch movewindowpixel "exact $X $Y,pid:$PID" >/dev/null
hyprctl dispatch focuswindow "pid:$PID" >/dev/null
sleep 4
TREADCORE_LAG_MS=30 "$SERVER" --net --want 3 --port 0 --introducer "127.0.0.1:$NP" --no-beacon > "$LOGDIR/shoot-seat-$STAMP.log" 2>&1 &
P_SEAT=$!
sleep 12
frame networked                                # playing as a client: the NET line, rtt and prediction
kill "$PID" "$P_SEAT" "$P_HOST" "$P_INTRO" 2>/dev/null || true; trap - EXIT

python3 - "$S" "$OUT" "$VERSION" <<'PY'
import sys, os
from PIL import Image
S, OUT, VERSION = sys.argv[1:4]
def crop(src, box, name, scale=1):
    im = Image.open(f"{S}/{src}.png").crop(box)
    if scale != 1:
        im = im.resize((im.width * scale, im.height * scale), Image.NEAREST)
    im.save(f"{OUT}/guide-{name}.png")
    return f"guide-{name}.png"
made = []
made.append(crop("screen", (0, 0, 1200, 740), "screen"))
made.append(crop("screen", (0, 60, 170, 604), "column"))
made.append(crop("armed", (0, 60, 170, 604), "column-armed"))
made.append(crop("armed", (620, 200, 860, 400), "ghost", 2))
made.append(crop("squad-out", (0, 604, 1200, 740), "bottom"))
made.append(crop("squad-out", (590, 604, 830, 684), "squad", 2))
made.append(crop("refused", (0, 684, 1200, 740), "refused"))
made.append(crop("screen", (915, 60, 1200, 604), "right"))
made.append(crop("screen", (0, 0, 1200, 60), "top"))
made.append(crop("aim", (300, 120, 900, 560), "aim"))
made.append(crop("events", (170, 300, 700, 604), "events"))
made.append(crop("close", (300, 150, 900, 550), "close"))
# The lobby: the last frame of the burst on which the lobby screen
# stands (TREADCORE at the top of a box over an empty map, no tiles yet:
# the map's grass is the tell that the match has begun).
def grassy(path):
    im = Image.open(path).convert("RGB"); w, h = im.size
    px = list(im.resize((w // 8, h // 8)).get_flattened_data())
    return sum(1 for r, g, b in px if g > 90 and g > r + 30 and g > b + 30) / len(px)
lobby = None
for i in range(1, 17):
    f = f"{S}/lobby-{i}.png"
    if os.path.exists(f) and os.path.getsize(f) > 0 and grassy(f) < 0.05:
        lobby = f
if lobby is None:
    sys.exit("no frame of the burst shows the lobby: the match began before the window drew")
Image.open(lobby).save(f"{OUT}/guide-lobby.png"); made.append("guide-lobby.png")
made.append(crop("networked", (0, 0, 1200, 740), "networked"))
made.append(crop("networked", (0, 0, 1200, 60), "net-top", 1))
with open(f"{OUT}/MANIFEST-{VERSION}.txt", "w") as f:
    f.write(f"# frames taken by scripts/shoot.sh for {VERSION}; release.sh checks each is shown\n")
    for m in made:
        f.write(m + "\n")
print("wrote", len(made), "frames")
PY
say "frames in $OUT, listed in $OUT/MANIFEST-$VERSION.txt"
