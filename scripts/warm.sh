#!/usr/bin/env bash
# Build the engine, and only the engine, gently -- and say what it is doing.
#
# treadcore-warm depends on Bevy exactly as the window does (one
# [workspace.dependencies.bevy] table feeds both), so its artifacts are
# the window's artifacts. Build it with the RAM-sized job count from
# configure, at low CPU and IO priority so the machine stays usable, in
# both profiles the game uses. Cargo resumes: a cancelled warm picks up
# from the last finished crate, nothing is lost.
#
#   make warm               every crate as it compiles, numbered [n/total]
#   VERBOSE=1 make warm     the full dependency tree first (who pulls in
#                           what), then the numbered lines with elapsed
#                           time per crate
#   VERBOSE=2 make warm     cargo -v: the rustc command line of each crate
#   PROFILE=release make warm
#   JOBS=1 make warm        for a machine that is swapping even at 2
#
# Everything cargo prints goes to the log regardless of VERBOSE.
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
export CARGO_TERM_COLOR=always
LOG="$LOGDIR/warm-$STAMP.log"
TREE="$LOGDIR/warm-$STAMP-tree.txt"
PROFILES="${PROFILE:-dev release}"
VERBOSE="${VERBOSE:-0}"
NICE="nice -n 15"
command -v ionice >/dev/null && NICE="ionice -c 3 $NICE"

# The dependency graph, once, to a file: the tree with indentation, so
# every crate's own dependencies are under it. The count of distinct
# crates is the denominator of the progress counter.
cargo tree -p treadcore-warm --offline -e normal,build --prefix indent > "$TREE" 2>/dev/null \
    || cargo tree -p treadcore-warm -e normal,build --prefix indent > "$TREE"
TOTAL=$(cargo tree -p treadcore-warm -e normal,build --prefix none 2>/dev/null | sort -u | wc -l)
DEPTH=$(awk '{match($0,/^[ │├└─]*/); d=RLENGTH/4; if (d>m) m=d} END{print m+0}' "$TREE")

say "warming the engine: profiles [$PROFILES], $CARGO_BUILD_JOBS job(s), low priority"
say "  $TOTAL crates, dependency tree $DEPTH deep -> $TREE"
say "  full cargo output -> $LOG"
say "  cancel any time with ctrl-c; it resumes where it stopped"
if [ "$VERBOSE" -ge 1 ]; then
    echo
    say "dependency tree (each crate's dependencies indented under it):"
    cat "$TREE"
    echo
fi

START=$(date +%s)
for p in $PROFILES; do
    case "$p" in
        dev) FLAG=() ;;
        release) FLAG=(--release) ;;
        *) die "unknown profile $p" ;;
    esac
    say "== profile $p  ($(date +%H:%M:%S))"
    CFLAG=()
    [ "$VERBOSE" -ge 2 ] && CFLAG=(-v)
    # Number every "Compiling" line against the total; with VERBOSE=1
    # also stamp each with seconds since the profile started, so the
    # slow crates are visible by eye and by grep.
    $NICE cargo build "${CFLAG[@]}" "${FLAG[@]}" -p treadcore-warm 2>&1 \
        | tee -a "$LOG" \
        | awk -v total="$TOTAL" -v t0="$(date +%s)" -v verbose="$VERBOSE" '
            /^\s*Compiling/ {
                n++; t = systime() - t0
                if (verbose >= 1) printf "[%3d/%d] %5ds  %s %s\n", n, total, t, $2, $3
                else              printf "[%3d/%d] %s %s\n", n, total, $2, $3
                fflush(); next
            }
            /^\s*(Finished|error|warning: unused)/ { print; fflush(); next }
            verbose >= 2 { print; fflush() }
            /^error/ { print; fflush() }
        '
    [ "${PIPESTATUS[0]}" = 0 ] || die "warm ($p) failed: see $LOG"
done
say "engine warm in $(( $(date +%s) - START ))s. 'make build' now compiles only the game."
say "  smoke test the window: target/release/treadcore-warm"
say "  why is a crate in the build?  make why CRATE=naga"
