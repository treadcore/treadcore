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
# JOBS FROM MEMORY, NOT FROM CPUS. A rustc on one of Bevy's big crates
# (bevy_ui, bevy_render, wgpu-core, naga) wants 1 to 1.5 GB. Four of them
# at once on a 6 GB machine is a swap storm, and a swap storm is slower
# than one job. Budget 1.5 GB per job after a gigabyte for everything
# else, capped at the CPU count, floored at one. JOBS=n on the make line
# overrides.
if [ -n "${JOBS:-}" ]; then
    JOBS_FROM_USER=1
elif [ "$MEM_MB" -gt 0 ]; then
    JOBS=$(( (MEM_MB - 1024) / 1536 ))
    [ "$JOBS" -lt 1 ] && JOBS=1
    [ "$JOBS" -gt "$CPUS" ] && JOBS="$CPUS"
else
    JOBS="$CPUS"
fi
echo "  jobs         $JOBS (from ${MEM_MB} MB; JOBS=n overrides)"
SWAP_MB="$(awk '/SwapTotal/ {printf "%d", $2/1024}' /proc/meminfo 2>/dev/null || echo 0)"
SWAP_FREE_MB="$(awk '/SwapFree/ {printf "%d", $2/1024}' /proc/meminfo 2>/dev/null || echo 0)"
if [ "$SWAP_MB" -lt 1024 ]; then
    warn "  swap         ${SWAP_MB} MB: a Bevy link with no swap can be killed outright; 2 GB of swap is cheap insurance"
else
    echo "  swap         ${SWAP_MB} MB, ${SWAP_FREE_MB} MB free"
fi
# A machine whose swap is already full is over-committed before the
# build starts: something else (an editor's rust-analyzer, usually) is
# holding what the build needs. One job until it clears. JOBS=n still
# overrides, for someone who knows better.
if [ -z "${JOBS_FROM_USER:-}" ] && [ "$SWAP_MB" -gt 0 ] && [ $((SWAP_FREE_MB * 10)) -lt "$SWAP_MB" ] && [ "$JOBS" -gt 1 ]; then
    warn "  swap is over 90% full: building with 1 job until it clears (pkill -x rust-analyzer usually clears it)"
    JOBS=1
fi

# A Vulkan driver, for the window. wgpu asks the loader for a device
# and panics without one; on a VM with no GPU the answer is lavapipe
# (mesa-vulkan-swrast), Vulkan on the CPU.
PRELOAD=
if ls /usr/share/vulkan/icd.d/*.json >/dev/null 2>&1; then
    ICDS="$(ls /usr/share/vulkan/icd.d/ | sed 's/_icd.*//' | tr '\n' ' ')"
    echo "  vulkan       $ICDS"
else
    warn "  vulkan       no driver under /usr/share/vulkan/icd.d/: the window will panic 'unable to find a GPU' (Alpine: apk add mesa-vulkan-swrast)"
fi
# musl gives a thread 128 KiB of stack; lavapipe's shader JIT needs more
# and segfaults on its own thread. scripts/bigstack.c raises the default
# for every thread; built here, preloaded by run.sh. Nothing to do on
# glibc, where the default is 8 MiB.
if ls /lib/ld-musl-*.so.1 >/dev/null 2>&1; then
    if command -v cc >/dev/null; then
        if cc -shared -fPIC -O2 -o "$BUILDDIR/bigstack.so" "$ROOT/scripts/bigstack.c" 2>/dev/null; then
            PRELOAD="$BUILDDIR/bigstack.so"
            echo "  libc         musl: built $PRELOAD (8 MiB thread stacks for lavapipe's JIT)"
        else
            warn "  libc         musl, and scripts/bigstack.c did not build: the window may segfault on lavapipe"
        fi
    else
        warn "  libc         musl, and no C compiler to build scripts/bigstack.c: the window may segfault on lavapipe (apk add gcc)"
    fi
fi

cat > "$BUILDDIR/config.env" <<CFG
# written by scripts/configure.sh on $STAMP -- do not edit, re-run make configure
RUST_VER=$RUST_VER
HAVE_WINDOW=$HAVE_WINDOW
CARGO_BUILD_JOBS=$JOBS
PRELOAD=$PRELOAD
CFG
echo "  wrote        $BUILDDIR/config.env"
