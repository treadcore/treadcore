#!/usr/bin/env bash
# install.sh [install | uninstall]
# The release binaries into PREFIX/bin (default ~/.local; PREFIX=/usr/local
# wants sudo): treadcore, the window, if it built here, and
# treadcore-server always. Nothing else: the game needs no files beside
# itself, and a map is a path given with --map.
# On macOS the window also goes into APPDIR (default ~/Applications) as
# Treadcore.app, so it opens from Finder, Launchpad and Spotlight; APP=0
# skips it.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
PREFIX="${PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
APPDIR="${APPDIR:-$HOME/Applications}"
APPBUNDLE="$APPDIR/Treadcore.app"
DARWIN=0; [ "$(uname -s)" = Darwin ] && DARWIN=1
BINS=(treadcore-server)
[ "$HAVE_WINDOW" = 1 ] && BINS=(treadcore "${BINS[@]}")

case "${1:-install}" in
    uninstall)
        for b in treadcore treadcore-server; do
            [ -e "$BINDIR/$b" ] && { rm -f "$BINDIR/$b"; echo "  removed $BINDIR/$b"; }
        done
        [ -d "$APPBUNDLE" ] && { rm -rf "$APPBUNDLE"; echo "  removed $APPBUNDLE"; }
        exit 0
        ;;
    install) ;;
    *) die "unknown action $1" ;;
esac

say "building ${BINS[*]} (release) -> $LOGDIR/build-$STAMP.log"
PKGS=(); for b in "${BINS[@]}"; do [ "$b" = treadcore ] && PKGS+=(-p treadcore-client) || PKGS+=(-p "$b"); done
cargo build --release "${PKGS[@]}" 2>&1 | tee "$LOGDIR/build-$STAMP.log" >/dev/null || die "build failed: see $LOGDIR/build-$STAMP.log"

mkdir -p "$BINDIR"
for b in "${BINS[@]}"; do
    install -m 755 "$ROOT/target/release/$b" "$BINDIR/$b"
    echo "  $BINDIR/$b"
done

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
