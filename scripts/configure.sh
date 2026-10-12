#!/usr/bin/env bash
# Check the machine, write build/config.env. Idempotent; cheap; run by
# every other target so the answer is never stale.
. "$(dirname "$0")/common.sh"

say "treadcore configure"
command -v cargo >/dev/null || die "cargo not found: install Rust (https://rustup.rs)"
command -v rustc >/dev/null || die "rustc not found"
RUST_VER="$(rustc --version | awk '{print $2}')"
MIN_RUST="$(sed -n 's/^rust-version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
if [ "$(printf '%s\n%s\n' "$MIN_RUST" "$RUST_VER" | sort -V | head -1)" != "$MIN_RUST" ]; then
    die "rustc $RUST_VER is older than the $MIN_RUST this workspace needs"
fi
echo "  rustc        $RUST_VER (need >= $MIN_RUST)"
echo "  cargo        $(cargo --version | awk '{print $2}')"

# The window needs a display stack. Headless crates need nothing. Record
# what is here so build.sh can say why the window will not build rather
# than let the linker say it.
HAVE_WINDOW=1
if command -v pkg-config >/dev/null; then
    for lib in wayland-client xkbcommon alsa libudev; do
        if pkg-config --exists "$lib"; then
            printf '  %-13s%s\n' "$lib" "$(pkg-config --modversion "$lib")"
        else
            warn "  $lib: missing (the window will not build; headless crates will)"
            HAVE_WINDOW=0
        fi
    done
else
    warn "  pkg-config missing: cannot check display libraries"
    HAVE_WINDOW=0
fi
if [ -n "${WAYLAND_DISPLAY:-}" ]; then
    echo "  session      wayland ($WAYLAND_DISPLAY)"
elif [ -n "${DISPLAY:-}" ]; then
    echo "  session      x11 ($DISPLAY)"
else
    warn "  session      none: no WAYLAND_DISPLAY or DISPLAY; 'make run' will fail, 'make debug' will not"
fi
CPUS="$(nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 1)"
MEM_MB="$(awk '/MemTotal/ {printf "%d", $2/1024}' /proc/meminfo 2>/dev/null || echo 0)"
echo "  cpus         $CPUS"
echo "  memory       ${MEM_MB} MB"
# Bevy's link step wants memory. Below 3 GB, serialise the jobs.
JOBS="$CPUS"
if [ "$MEM_MB" -gt 0 ] && [ "$MEM_MB" -lt 3072 ]; then
    JOBS=1
    warn "  low memory: building with -j1"
fi

cat > "$BUILDDIR/config.env" <<CFG
# written by scripts/configure.sh on $STAMP -- do not edit, re-run make configure
RUST_VER=$RUST_VER
HAVE_WINDOW=$HAVE_WINDOW
CARGO_BUILD_JOBS=$JOBS
CFG
echo "  wrote        $BUILDDIR/config.env"
