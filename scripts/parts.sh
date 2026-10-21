#!/usr/bin/env bash
# Each part off from the brain with every part on (roadmap backlog 48;
# LAB-REPORT-BRAIN.md section 63): the reverse of `make switches`. The
# brain census (treadcore-brain fleet.rs `brain_census`) once for every
# part, one team on the defaults (every part on), two with that part
# off, every team in the trial seat on every seed. A capability is off
# as `-name`; a weight is off at the value its doc line names as before
# it (`name=value`). The difference is what the part is worth: above
# zero it pays. Prints a table: the difference a seat, its standard
# error over the seeds, and whether it is outside two standard errors.
#
#   make parts                         every part, 12 seeds of 900 s
#   PARTS="-posture w_front=0" make parts
#   SEEDS=6 SECS=600 MAP=maps/x.map RESPAWNS=3 JOBS=4 make parts
#
# JOBS runs that many censuses at once; the scores are the same either
# way (the matches are deterministic). The ledger is not written. Logs,
# the weights files and the table under build/parts/STAMP.

. "$(dirname "$0")/common.sh"
CAPS_OFF="-kite -pings -dodge -bridges -rotate -forget -board -team_rotate -priced_crossing -column -cycle -island_site -escort -bridge_jobs -duck -posture -castle_cap"
WEIGHTS_OFF="w_smooth=0 dodge_plan=0 dodge_release=0 dodge_every=0 dodge_fire=0 duck_plan=0 landfall_shy=0 landfall_wait=0 landfall_memory=0 w_front=0 front_memory=0 w_front_post=0 w_front_push=0 w_site_beach=0"
PARTS="${PARTS:-$CAPS_OFF $WEIGHTS_OFF}"
SEEDS="${SEEDS:-12}"
SECS="${SECS:-900}"
MAP="${MAP:-maps/naval.map}"
RESPAWNS="${RESPAWNS:-3}"
JOBS="${JOBS:-1}"
OUT="$BUILDDIR/parts/$STAMP"
mkdir -p "$OUT"
MAPFILE_="$(cd "$(dirname "$MAP")" && pwd)/$(basename "$MAP")"

say "each part off: $PARTS; $SEEDS seeds of $SECS s on $MAP, respawns $RESPAWNS, $JOBS at once -> $OUT"
BIN="$(census_bin "$OUT")"
printf 'caps = baseline\n' >"$OUT/on.weights"

# A part's name as a file name: `-duck` is duck, `w_front=0` is w_front.
key() { echo "$1" | sed 's/^-//; s/=.*//'; }

census() { # census PART
    local k; k="$(key "$1")"
    case "$1" in
        -*) printf 'caps = baseline, %s\n' "$1" ;;
        *=*) printf 'caps = baseline\n%s = %s\n' "${1%%=*}" "${1#*=}" ;;
    esac >"$OUT/$k.weights"
    env CENSUS_MAP="$MAPFILE_" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SEEDS="$SEEDS" CENSUS_SECS="$SECS" \
        CENSUS_LEDGER=0 CENSUS_WEIGHTS="$OUT/on.weights" CENSUS_BASE_WEIGHTS="$OUT/$k.weights" \
        "$BIN" --ignored brain_census --nocapture >"$OUT/$k.log" 2>&1 \
        || warn "the census of $1 failed: see $OUT/$k.log"
    echo "  $1 done"
}

running=0
for p in $PARTS; do
    census "$p" &
    running=$((running + 1))
    if [ "$running" -ge "$JOBS" ]; then
        wait -n
        running=$((running - 1))
    fi
done
wait

{
    printf '%-20s %8s %8s %8s %6s  %s\n' "part off" "on" "off" diff se verdict
    for p in $PARTS; do
        k="$(key "$p")"
        read -r t b < <(sed -n 's/^brain_census: trial \(-\{0,1\}[0-9.]*\), baseline \(-\{0,1\}[0-9.]*\).*/\1 \2/p' "$OUT/$k.log") || true
        read -r d e v < <(sed -n 's/^brain_census: difference \([-+0-9.]*\) a seat, standard error \([0-9.]*\) over [0-9]* seeds (\(.*\)); .*/\1 \2 \3/p' "$OUT/$k.log") || true
        printf '%-20s %8s %8s %8s %6s  %s\n' "$p" "${t:-?}" "${b:-?}" "${d:-?}" "${e:-?}" "${v:-failed}"
    done
} | tee "$OUT/table.txt"
