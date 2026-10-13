#!/usr/bin/env bash
# install.sh [install | uninstall]
# The release binaries into PREFIX/bin (default ~/.local; PREFIX=/usr/local
# wants sudo): treadcore, the window, if it built here, and
# treadcore-server always. Nothing else: the game needs no files beside
# itself, and a map is a path given with --map.
# On macOS the window also goes into APPDIR (default ~/Applications) as
# Treadcore.app, so it opens from Finder, Launchpad and Spotlight; APP=0
# skips it.
# On Linux (Copal, Alpine, Arch, Omarchy, Debian, Ubuntu, Gentoo, Fedora)
# the window gets a desktop entry and an icon under PREFIX/share, which
# every XDG launcher reads: Copal's Super+Space menu, Omarchy's, GNOME's,
# KDE's. On a musl system (Copal, Alpine) the window is installed behind
# a wrapper that preloads bigstack.so, because musl's 128 KiB thread
# stacks crash lavapipe's shader compiler (scripts/bigstack.c); the real
# binary and the preload go in PREFIX/lib/treadcore.
. "$(dirname "$0")/common.sh"
. "$(dirname "$0")/platform.sh"
need_config
export CARGO_BUILD_JOBS
PREFIX="${PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
APPDIR="${APPDIR:-$HOME/Applications}"
APPBUNDLE="$APPDIR/Treadcore.app"
LIBDIR="$PREFIX/lib/treadcore"
DESKTOP="$PREFIX/share/applications/org.treadcore.treadcore.desktop"
ICON="$PREFIX/share/icons/hicolor/scalable/apps/treadcore.svg"
DARWIN=0; [ "$(uname -s)" = Darwin ] && DARWIN=1
BINS=(treadcore-server)
[ "$HAVE_WINDOW" = 1 ] && BINS=(treadcore "${BINS[@]}")

# Tell the launchers. Copal's menu notices the directory change by
# itself; the rest read the caches these two tools write.
refresh_menus() {
    command -v update-desktop-database >/dev/null && update-desktop-database "$PREFIX/share/applications" >/dev/null 2>&1 || true
    command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$PREFIX/share/icons/hicolor" >/dev/null 2>&1 || true
}

case "${1:-install}" in
    uninstall)
        for b in treadcore treadcore-server; do
            [ -e "$BINDIR/$b" ] && { rm -f "$BINDIR/$b"; echo "  removed $BINDIR/$b"; }
        done
        [ -d "$APPBUNDLE" ] && { rm -rf "$APPBUNDLE"; echo "  removed $APPBUNDLE"; }
        [ -d "$LIBDIR" ] && { rm -rf "$LIBDIR"; echo "  removed $LIBDIR"; }
        for f in "$DESKTOP" "$ICON"; do
            [ -e "$f" ] && { rm -f "$f"; echo "  removed $f"; }
        done
        refresh_menus
        exit 0
        ;;
    install) ;;
    *) die "unknown action $1" ;;
esac

if [ "$HAVE_WINDOW" != 1 ]; then
    warn "the window cannot build here (its system packages are missing): installing the headless server only."
    warn "to install the game itself, first:"
    "$ROOT/scripts/deps.sh" | sed 's/^/  /' || true
fi
say "building ${BINS[*]} (release) -> $LOGDIR/build-$STAMP.log"
PKGS=(); for b in "${BINS[@]}"; do [ "$b" = treadcore ] && PKGS+=(-p treadcore-client) || PKGS+=(-p "$b"); done
cargo build --release "${PKGS[@]}" 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"

say "installing for $PLATFORM_NAME into $PREFIX"
mkdir -p "$BINDIR"
for b in "${BINS[@]}"; do
    if [ "$b" = treadcore ] && [ "$DARWIN" = 0 ] && [ -n "${PRELOAD:-}" ]; then
        # musl: the window behind its preload.
        mkdir -p "$LIBDIR"
        install -m 755 "$ROOT/target/release/treadcore" "$LIBDIR/treadcore"
        install -m 644 "$PRELOAD" "$LIBDIR/bigstack.so"
        cat > "$BINDIR/treadcore" <<WRAP
#!/bin/sh
# treadcore, installed by make install on $PLATFORM_NAME. musl gives a
# thread 128 KiB of stack and lavapipe's shader compiler needs more, so
# the window runs with bigstack.so preloaded (scripts/bigstack.c).
export LD_PRELOAD="$LIBDIR/bigstack.so\${LD_PRELOAD:+:\$LD_PRELOAD}"
exec "$LIBDIR/treadcore" "\$@"
WRAP
        chmod 755 "$BINDIR/treadcore"
        echo "  $BINDIR/treadcore (runs $LIBDIR/treadcore with bigstack.so preloaded)"
    else
        # A glibc install over an old musl one: the wrapper's tree goes.
        [ "$b" = treadcore ] && rm -rf "$LIBDIR"
        install -m 755 "$ROOT/target/release/$b" "$BINDIR/$b"
        echo "  $BINDIR/$b"
    fi
done

if [ "$DARWIN" = 0 ] && [ "$HAVE_WINDOW" = 1 ]; then
    mkdir -p "$(dirname "$DESKTOP")" "$(dirname "$ICON")"
    # The icon is drawn the way the game is: rectangles. Grass, a tan
    # pillbox in three steps with its gun, an olive tank in front.
    cat > "$ICON" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
  <rect width="64" height="64" rx="10" fill="#007020"/>
  <rect x="6" y="6" width="22" height="22" fill="#1fb010"/>
  <rect x="30" y="8" width="26" height="26" fill="#806040"/>
  <rect x="30" y="8" width="26" height="22" fill="#c0a06f"/>
  <rect x="34" y="11" width="18" height="15" fill="#cbad7c"/>
  <rect x="38" y="14" width="10" height="8" fill="#d6ba8a"/>
  <rect x="41" y="16" width="5" height="4" fill="#f28c33"/>
  <rect x="8" y="36" width="36" height="20" fill="#33331f"/>
  <rect x="8" y="36" width="36" height="16" fill="#6b6b42"/>
  <rect x="8" y="36" width="36" height="3" fill="#333028"/>
  <rect x="8" y="49" width="36" height="3" fill="#333028"/>
  <rect x="18" y="40" width="14" height="8" fill="#8c94ff"/>
  <rect x="32" y="43" width="22" height="3" fill="#9ea39e"/>
</svg>
SVG
    cat > "$DESKTOP" <<DESK
[Desktop Entry]
Type=Application
Name=Treadcore
GenericName=Tank game
Comment=Sixteen tanks, one island, the airspace round each one
Exec=$BINDIR/treadcore --solo
Icon=treadcore
Terminal=false
Categories=Game;StrategyGame;
StartupWMClass=org.treadcore.treadcore
Keywords=tank;drone;bolo;multiplayer;strategy;
Actions=lan;

[Desktop Action lan]
Name=Treadcore on the LAN
Exec=$BINDIR/treadcore --lan
DESK
    refresh_menus
    echo "  $DESKTOP"
    echo "  $ICON"
    case "$PLATFORM" in
        copal) echo "  Copal: Treadcore is in the Super+Space menu under Games once the menu notices (it rebuilds when ~/.local/share/applications changes)" ;;
        omarchy) echo "  Omarchy: Treadcore is in the app launcher (Super+Space)" ;;
    esac
fi

if [ "$DARWIN" = 1 ] && [ "${APP:-1}" = 1 ] && [ "$HAVE_WINDOW" = 1 ]; then
    VERSION="$(sed -n 's/^version *= *"\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
    rm -rf "$APPBUNDLE"
    mkdir -p "$APPBUNDLE/Contents/MacOS"
    install -m 755 "$ROOT/target/release/treadcore" "$APPBUNDLE/Contents/MacOS/treadcore"
    cat > "$APPBUNDLE/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Treadcore</string>
    <key>CFBundleDisplayName</key><string>Treadcore</string>
    <key>CFBundleIdentifier</key><string>org.treadcore.treadcore</string>
    <key>CFBundleExecutable</key><string>treadcore</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleVersion</key><string>${VERSION%%-*}</string>
    <key>CFBundleShortVersionString</key><string>${VERSION:-0}</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSLocalNetworkUsageDescription</key><string>Treadcore finds other players on the local network.</string>
</dict>
</plist>
EOF
    # Ad-hoc signed, so Apple silicon runs it and the firewall can
    # remember the answer to "accept incoming connections?".
    codesign --force --sign - "$APPBUNDLE" >/dev/null 2>&1 || warn "  codesign failed: the app may need allowing in System Settings"
    echo "  $APPBUNDLE (solo; for the LAN lobby: treadcore --lan)"
fi

case ":$PATH:" in
    *":$BINDIR:"*) ;;
    *) warn "  $BINDIR is not on PATH: add  export PATH=\"$BINDIR:\$PATH\"  to your shell profile" ;;
esac
