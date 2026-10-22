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
# weights files and pooled.txt under build/census/STAMP.

. "$(dirname "$0")/common.sh"
SEEDS="${SEEDS:-48}"
SEED0="${SEED0:-0}"
SECS="${SECS:-900}"
MAP="${MAP:-maps/naval.map}"
RESPAWNS="${RESPAWNS:-3}"
JOBS="${JOBS:-4}"
OUT="$BUILDDIR/census/$STAMP"
mkdir -p "$OUT"
# Each side the defaults unless named: an empty TRIAL is the defaults,
# not the baseline's file.
for side in trial base; do
    f="$(eval echo "\${${side^^}:-}")"
    if [ -n "$f" ]; then cp "$f" "$OUT/$side.weights"; else printf 'caps = baseline\n' >"$OUT/$side.weights"; fi
done
MAPS="$(census_maps "$MAP")"
[ -n "$MAPS" ] || die "no map in MAP=$MAP"

say "census: $SEEDS seeds from $SEED0 of $SECS s on $MAP, respawns $RESPAWNS, $JOBS at once -> $OUT"
BIN="$(census_bin "$OUT")"
per=$(( (SEEDS + JOBS - 1) / JOBS ))
s=$SEED0
end=$(( SEED0 + SEEDS ))
while [ "$s" -lt "$end" ]; do
    n=$(( end - s < per ? end - s : per ))
    env CENSUS_MAP="$MAPS" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SEED0="$s" CENSUS_SEEDS="$n" CENSUS_SECS="$SECS" \
        CENSUS_LEDGER=0 CENSUS_WEIGHTS="$OUT/trial.weights" CENSUS_BASE_WEIGHTS="$OUT/base.weights" \
        "$BIN" --ignored brain_census --nocapture >"$OUT/g-$s.log" 2>&1 \
        || warn "the census of seeds $s on failed: see $OUT/g-$s.log" &
    s=$(( s + n ))
done
wait
"$ROOT/scripts/pool.sh" "$OUT" | tee "$OUT/pooled.txt"
