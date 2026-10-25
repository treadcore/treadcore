#!/usr/bin/env bash
# The fair census of one brain against another (roadmap backlog 25, LR
# 71): treadcore-brain fleet.rs `brain_census`, the trial weights file
# for one team and the baseline for two, every team in the trial seat
# on every seed; the seeds split over JOBS at once from a frozen binary,
# then pooled overall and by map (scripts/pool.sh). Above zero the trial
# scores more a seat.
#
#   TRIAL=a.weights make census                  against the defaults
#   TRIAL=a.weights BASE=b.weights SEEDS=96 SEED0=96 JOBS=4 make census
#   MAP=all SEEDS=88 make census                 every map and a new one
#                                                generated each seed
#
# MAP is a file, a comma-separated list, `gen` or `all` (common.sh
# `census_maps`); seed n plays the (n mod k)th map of k. Logs, the
# weights files, run.env and pooled.txt under build/census/STAMP.
# JOBS defaults to the machine's cores.
#
#   RESUME=build/census/STAMP make census        finish a stopped census
#
# A resumed census reads its settings from the directory's run.env and
# plays on its frozen binary and weights, so it must resume on the
# machine that began it. A job whose log ends in "test result: ok" is
# kept; any other is set aside as g-SEED.log.partial and played again
# whole (the sim is deterministic on one binary, so a seed replays as
# it played). See documentation/CENSUS.md.

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
fi
MAPS="$(census_maps "$MAP")"
[ -n "$MAPS" ] || die "no map in MAP=$MAP"

say "census: $SEEDS seeds from $SEED0 of $SECS s on $MAP, respawns $RESPAWNS, $JOBS at once -> $OUT${RESUME:+ (resumed)}"
if [ -n "${RESUME:-}" ]; then BIN="$OUT/census-bin"; else BIN="$(census_bin "$OUT")"; fi
per=$(( (SEEDS + JOBS - 1) / JOBS ))
s=$SEED0
end=$(( SEED0 + SEEDS ))
while [ "$s" -lt "$end" ]; do
    n=$(( end - s < per ? end - s : per ))
    log="$OUT/g-$s.log"
    if [ -n "${RESUME:-}" ] && grep -q '^test result: ok' "$log" 2>/dev/null; then
        s=$(( s + n )); continue
    fi
    [ -f "$log" ] && mv "$log" "$log.partial"
    env CENSUS_MAP="$MAPS" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SEED0="$s" CENSUS_SEEDS="$n" CENSUS_SECS="$SECS" \
        CENSUS_LEDGER=0 CENSUS_WEIGHTS="$OUT/trial.weights" CENSUS_BASE_WEIGHTS="$OUT/base.weights" \
        "$BIN" --ignored brain_census --nocapture >"$log" 2>&1 \
        || warn "the census of seeds $s on failed: see $log" &
    s=$(( s + n ))
done
wait
"$ROOT/scripts/pool.sh" "$OUT" | tee "$OUT/pooled.txt"
