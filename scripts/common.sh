# shellcheck shell=bash
# Shared by every script: paths, logging, the config.env handshake.
# Sourced, not run.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILDDIR="${BUILDDIR:-$ROOT/build}"
case "$BUILDDIR" in /*) ;; *) BUILDDIR="$ROOT/$BUILDDIR" ;; esac
LOGDIR="${LOGDIR:-$BUILDDIR/logs}"
case "$LOGDIR" in /*) ;; *) LOGDIR="$ROOT/$LOGDIR" ;; esac
mkdir -p "$BUILDDIR" "$LOGDIR"
# shellcheck disable=SC2034  # used by every script that sources this
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

# census_maps SPEC: the census's CENSUS_MAP (roadmap backlog 25, LR 71)
# from MAP: a map file, or a comma-separated list of them; `gen` for a
# map generated from each seed (a new map every seed); `all` for every
# map under maps/ and `gen`. Files made absolute; printed comma-joined.
census_maps() {
    local out="" m
    for m in $(echo "$1" | tr ',' ' '); do
        case "$m" in
            gen) out="$out,gen" ;;
            all) out="$out,$(ls "$ROOT"/maps/*.map "$ROOT"/maps/*/*.map | grep -v '/\._' | paste -sd,),gen" ;;
            *) [ -f "$m" ] || die "no map file $m"
               out="$out,$(cd "$(dirname "$m")" && pwd)/$(basename "$m")" ;;
        esac
    done
    echo "${out#,}"
}

# census_bin DIR: build the brain's tests once and copy the test binary
# into DIR; print the copy's path. A census run from the copy is the code
# as it was when the run began, so the tree can change while it plays
# (roadmap backlog 41). Run it as: "$BIN" --ignored brain_census --nocapture
census_bin() {
    local exe
    exe="$(cd "$ROOT" && cargo test -q -p treadcore-brain --release --no-run --message-format=json 2>/dev/null \
        | python3 -I -c 'import json, sys
for line in sys.stdin:
    m = json.loads(line)
    if m.get("reason") == "compiler-artifact" and m["target"]["name"] == "treadcore_brain" and m["profile"]["test"] and m.get("executable"):
        print(m["executable"])' | head -1)"
    [ -n "$exe" ] && [ -x "$exe" ] || die "the brain does not build"
    cp "$exe" "$1/census-bin"
    echo "$1/census-bin"
}
