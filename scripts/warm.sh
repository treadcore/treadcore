#!/usr/bin/env bash
# Build the engine, and only the engine, gently.
#
# treadcore-warm depends on Bevy exactly as the window does (one
# [workspace.dependencies.bevy] table feeds both), so its artifacts are
# the window's artifacts. Build it with the RAM-sized job count from
# configure, at low CPU and IO priority so the machine stays usable, in
# both profiles the game uses. Cargo resumes: a cancelled warm picks up
# from the last finished crate, nothing is lost.
#
#   make warm             both profiles (dev first, release second)
#   PROFILE=release make warm
#   JOBS=1 make warm      for a machine that is swapping even at 2
. "$(dirname "$0")/common.sh"
need_config
export CARGO_BUILD_JOBS
export CARGO_TERM_COLOR=always
LOG="$LOGDIR/warm-$STAMP.log"
PROFILES="${PROFILE:-dev release}"
NICE="nice -n 15"
command -v ionice >/dev/null && NICE="ionice -c 3 $NICE"
say "warming the engine: profiles [$PROFILES], $CARGO_BUILD_JOBS job(s), low priority -> $LOG"
say "  cancel any time with ctrl-c; it resumes where it stopped"
START=$(date +%s)
for p in $PROFILES; do
    case "$p" in
        dev) FLAG=() ;;
        release) FLAG=(--release) ;;
        *) die "unknown profile $p" ;;
    esac
    say "== $p"
    $NICE cargo build "${FLAG[@]}" -p treadcore-warm 2>&1 | tee -a "$LOG" | grep --line-buffered -E "Compiling (bevy|wgpu|naga)|Finished|error" || true
    [ "${PIPESTATUS[0]}" = 0 ] || die "warm ($p) failed: see $LOG"
done
say "engine warm in $(( $(date +%s) - START ))s. 'make build' now compiles only the game."
say "  smoke test the window: target/release/treadcore-warm"
