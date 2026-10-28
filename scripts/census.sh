#!/usr/bin/env bash
# The fair census of one brain against another (roadmap backlog 25, LR
# 71): treadcore-brain fleet.rs `brain_census`, the trial weights file
# for one team and the baseline for two, every team in the trial seat
# on every seed; the seeds split over JOBS at once from a frozen binary,
# then pooled overall and by map (scripts/pool.sh). Above zero the trial
# scores more a seat.
#
#   TRIAL=a.weights scripts/census.sh                  against the defaults
#   TRIAL=a.weights BASE=b.weights SEEDS=96 SEED0=96 JOBS=4 scripts/census.sh
#   MAP=all SEEDS=88 scripts/census.sh                 every map and a new one
#                                                generated each seed
#
# MAP is a file, a comma-separated list, `gen` or `all` (common.sh
# `census_maps`); seed n plays the (n mod k)th map of k. Logs, the
# weights files, run.env and pooled.txt under build/census/STAMP.
# JOBS defaults to the machine's cores.
#
#   RESUME=build/census/STAMP scripts/census.sh        finish a stopped census
#
# A resumed census reads its settings from the directory's run.env and
# plays on its frozen binary and weights, so it must resume on the
# machine that began it. A job whose log ends in "test result: ok" is
# kept; any other is set aside as g-SEED.log.partial and played again
# whole (the sim is deterministic on one binary, so a seed replays as
# it played). See documentation/CENSUS.md.
#
#   LOOKS=8 TRIAL=a.weights scripts/census.sh          the sequential census
#
# LOOKS (backlog 59, LR 79) reads the census as it plays at each LOOKS-th
# of its seeds (scripts/sequential.py watch) and stops at the first look
# past the O'Brien-Fleming bound. An early stop is no verdict: the jobs
# are stopped and the census is read again, sequentially, on SEEDS fresh
# seeds from SEED0 + SEEDS (DIR/reread); outside only if the re-read is
# outside the same way. A census that reaches its last look is read as
# without LOOKS. The verdict is DIR/verdict.txt. It saves seeds only on
# large effects (z past 5 or so); on the close calls it costs more, and
# reads fewer of them outside (LR 79).

. "$(dirname "$0")/common.sh"
if [ -n "${RESUME:-}" ]; then
    OUT="$RESUME"
    [ -x "$OUT/census-bin" ] || die "no census to resume in $OUT"
    # run.env is written at a census's start; one begun before it
    # existed resumes on the settings given (they must be the same).
    if [ -f "$OUT/run.env" ]; then . "$OUT/run.env"; else warn "no $OUT/run.env: resuming on SEEDS, SEED0, SECS, MAP, RESPAWNS and JOBS as given"; fi
fi
SEEDS="${SEEDS:-48}"
SEED0="${SEED0:-0}"
SECS="${SECS:-900}"
MAP="${MAP:-maps/naval.map}"
RESPAWNS="${RESPAWNS:-3}"
JOBS="${JOBS:-$(nproc)}"
if [ -z "${RESUME:-}" ]; then
    OUT="$BUILDDIR/census/$STAMP"
    mkdir -p "$OUT"
    # Each side the defaults unless named: an empty TRIAL is the
    # defaults, not the baseline's file.
    for side in trial base; do
        if [ "$side" = trial ]; then f="${TRIAL:-}"; else f="${BASE:-}"; fi
        if [ -n "$f" ]; then cp "$f" "$OUT/$side.weights"; else printf 'caps = baseline\n' >"$OUT/$side.weights"; fi
    done
    printf 'SEEDS=%q\nSEED0=%q\nSECS=%q\nMAP=%q\nRESPAWNS=%q\nJOBS=%q\n' "$SEEDS" "$SEED0" "$SECS" "$MAP" "$RESPAWNS" "$JOBS" >"$OUT/run.env"
    # What it was built from and where, for the ledger (LR 92).
    # A team of learned tanks in the trial seat (LR 96), if any.
    printf 'NET=%q\n' "${CENSUS_NET:-}" >>"$OUT/run.env"
    printf 'COMMIT=%q\nHOST=%q\n' "$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null)$(git -C "$ROOT" diff --quiet HEAD -- source 2>/dev/null || echo +)" "$(hostname)" >>"$OUT/run.env"
fi
MAPS="$(census_maps "$MAP")"
[ -n "$MAPS" ] || die "no map in MAP=$MAP"

[ -z "${LOOKS:-}" ] || [ -z "${RESUME:-}" ] || die "a sequential census (LOOKS) does not resume"
say "census: $SEEDS seeds from $SEED0 of $SECS s on $MAP, respawns $RESPAWNS, $JOBS at once -> $OUT${RESUME:+ (resumed)}"
if [ -n "${RESUME:-}" ]; then BIN="$OUT/census-bin"; else BIN="$(census_bin "$OUT")"; fi
per=$(( (SEEDS + JOBS - 1) / JOBS ))
# play DIR SEED0: the jobs over SEEDS seeds from SEED0 into DIR, in the
# background.
play() {
    local dir="$1" s="$2" end=$(( $2 + SEEDS )) n log
    while [ "$s" -lt "$end" ]; do
        n=$(( end - s < per ? end - s : per ))
        log="$dir/g-$s.log"
        if [ -n "${RESUME:-}" ] && grep -q '^test result: ok' "$log" 2>/dev/null; then
            s=$(( s + n )); continue
        fi
        [ -f "$log" ] && mv "$log" "$log.partial"
        # Its stderr to nowhere, the warning to fd 3: a job stopped early
        # (LOOKS) is otherwise reported "Terminated" by its subshell.
        { env CENSUS_MAP="$MAPS" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SEED0="$s" CENSUS_SEEDS="$n" CENSUS_SECS="$SECS" \
            CENSUS_LEDGER=0 CENSUS_WEIGHTS="$OUT/trial.weights" CENSUS_BASE_WEIGHTS="$OUT/base.weights" \
            "$BIN" --ignored brain_census --nocapture >"$log" 2>&1 \
            || [ -f "$dir/stopped" ] || warn "the census of seeds $s on failed: see $log" 2>&3; } 3>&2 2>/dev/null &
        s=$(( s + n ))
    done
}
if [ -z "${LOOKS:-}" ]; then
    play "$OUT" "$SEED0"
    wait
    "$ROOT/scripts/pool.sh" "$OUT" | tee "$OUT/pooled.txt"
    exit
fi
# stage DIR SEED0: one sequential census; status 3 at an early stop.
stage() {
    local dir="$1" rc=0
    mkdir -p "$dir"
    [ "$dir" = "$OUT" ] || sed "s/^SEED0=.*/SEED0=$2/" "$OUT/run.env" >"$dir/run.env"
    play "$dir" "$2"
    "$ROOT/scripts/sequential.py" watch "$dir" "$BIN" -k "$LOOKS" || rc=$?
    if [ "$rc" = 3 ]; then touch "$dir/stopped"; pkill -f -- "$BIN" || true; fi
    wait
    "$ROOT/scripts/pool.sh" "$dir" >"$dir/pooled.txt" || true
    return "$rc"
}
rc=0; stage "$OUT" "$SEED0" || rc=$?
[ "$rc" = 0 ] || [ "$rc" = 3 ] || die "the sequential census of $OUT failed ($rc): see its sequential.txt"
if [ "$rc" = 3 ]; then
    say "census: stopped early; read again on $SEEDS fresh seeds from $(( SEED0 + SEEDS ))"
    stage "$OUT/reread" $(( SEED0 + SEEDS )) || true
fi
"$ROOT/scripts/sequential.py" verdict "$OUT" | tee "$OUT/verdict.txt"
