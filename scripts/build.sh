#!/usr/bin/env bash
# build.sh PROFILE WHAT
#   PROFILE  release | debug | test
#   WHAT     all | headless
# Output goes to the terminal and to build/logs/build-STAMP.log both, so a
# failed build is a file that can be read later, not scrollback.
. "$(dirname "$0")/common.sh"
need_config
PROFILE="${1:-release}"
WHAT="${2:-all}"
LOG="$LOGDIR/build-$STAMP.log"
export CARGO_BUILD_JOBS
export CARGO_TERM_COLOR=always

HEADLESS=(-p treadcore-sim -p treadcore-proto -p treadcore-net -p treadcore-brain -p treadcore-server)
case "$PROFILE" in
    test)
        say "cargo test (all crates) -> $LOG"
        cargo test --workspace 2>&1 | tee "$LOG"
        exit "${PIPESTATUS[0]}"
        ;;
    release) FLAG=(--release) ;;
    debug)   FLAG=() ;;
    *) die "unknown profile $PROFILE" ;;
esac
case "$WHAT" in
    headless)
        say "cargo build $PROFILE headless -> $LOG"
        cargo build "${FLAG[@]}" "${HEADLESS[@]}" 2>&1 | tee "$LOG"
        exit "${PIPESTATUS[0]}"
        ;;
    all)
        if [ "$HAVE_WINDOW" != 1 ]; then
            warn "display libraries missing (see make configure): building headless crates only"
            exec "$0" "$PROFILE" headless
        fi
        say "cargo build $PROFILE all -> $LOG"
        cargo build "${FLAG[@]}" --workspace 2>&1 | tee "$LOG"
        exit "${PIPESTATUS[0]}"
        ;;
    *) die "unknown target $WHAT" ;;
esac
