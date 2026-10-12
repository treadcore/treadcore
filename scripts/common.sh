# Shared by every script: paths, logging, the config.env handshake.
# Sourced, not run.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILDDIR="${BUILDDIR:-$ROOT/build}"
case "$BUILDDIR" in /*) ;; *) BUILDDIR="$ROOT/$BUILDDIR" ;; esac
LOGDIR="${LOGDIR:-$BUILDDIR/logs}"
case "$LOGDIR" in /*) ;; *) LOGDIR="$ROOT/$LOGDIR" ;; esac
mkdir -p "$BUILDDIR" "$LOGDIR"
STAMP="$(date +%Y%m%d-%H%M%S)"

say()  { printf '\033[1m%s\033[0m\n' "$*"; }
warn() { printf '\033[33m%s\033[0m\n' "$*" >&2; }
die()  { printf '\033[31m%s\033[0m\n' "$*" >&2; exit 1; }

# configure.sh writes this; everything else reads it and refuses to run
# without it, so a build on a machine that has not been checked is not a
# thing that can happen.
need_config() {
    [ -f "$BUILDDIR/config.env" ] || die "not configured: run 'make configure' first"
    # shellcheck disable=SC1090
    . "$BUILDDIR/config.env"
}

# The newest log of a kind, for the "see ..." line at the end.
latest() { ls -t "$LOGDIR"/"$1"-*.log 2>/dev/null | head -1; }
