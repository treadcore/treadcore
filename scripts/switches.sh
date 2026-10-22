#!/usr/bin/env bash
# Each switch alone (roadmap goal 4, backlog 18; LAB-REPORT-BRAIN.md
# section 24): the brain census (treadcore-brain fleet.rs `brain_census`)
# once for every capability that is off by default, one team with that
# capability added, two without, every team in the trial seat on every
# seed. A switch that needs another (team_rotate, bridge_jobs and
# posture need board) is tried with that one on for every team. Prints
# a table: the
# difference a seat, its standard error over the seeds, and whether it
# is outside two standard errors.
#
#   make switches                      12 seeds of 900 s on naval.map
#   SWITCHES="cycle duck" make switches
#   SEEDS=6 SECS=600 MAP=maps/x.map RESPAWNS=3 JOBS=3 make switches
#
# JOBS runs that many censuses at once; the scores are the same either
# way (the matches are deterministic), only the microseconds a seat-tick
# read high under load. The ledger is not written (CENSUS_LEDGER=0), so
# runs side by side do not interleave its lines. Logs and the table
# under build/switches/STAMP.

. "$(dirname "$0")/common.sh"
SWITCHES="${SWITCHES:-forget smooth board team_rotate priced_crossing column cycle island_site escort bridge_jobs duck posture}"
SEEDS="${SEEDS:-12}"
SECS="${SECS:-900}"
MAP="${MAP:-maps/naval.map}"
RESPAWNS="${RESPAWNS:-3}"
JOBS="${JOBS:-1}"
OUT="$BUILDDIR/switches/$STAMP"
mkdir -p "$OUT"
MAPFILE_="$(census_maps "$MAP")"

# What a switch needs on for every team, else nothing.
needs() {
    case "$1" in
        team_rotate|bridge_jobs|posture) echo board ;;
        *) echo "" ;;
    esac
}

say "each switch alone: $SWITCHES; $SEEDS seeds of $SECS s on $MAP, respawns $RESPAWNS, $JOBS at once -> $OUT"
BIN="$(census_bin "$OUT")"

census() { # census SWITCH
    env CENSUS_MAP="$MAPFILE_" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SEEDS="$SEEDS" CENSUS_SECS="$SECS" \
        CENSUS_LEDGER=0 CENSUS_CAPS="$1" CENSUS_BASE_CAPS="$(needs "$1")" \
        "$BIN" --ignored brain_census --nocapture >"$OUT/$1.log" 2>&1 \
        || warn "the census of $1 failed: see $OUT/$1.log"
    echo "  $1 done"
}

running=0
for s in $SWITCHES; do
    census "$s" &
    running=$((running + 1))
    if [ "$running" -ge "$JOBS" ]; then
        wait -n
        running=$((running - 1))
    fi
done
wait

{
    printf '%-16s %-8s %8s %8s %8s %6s  %s\n' switch with trial baseline diff se verdict
    for s in $SWITCHES; do
        read -r t b < <(sed -n 's/^brain_census: trial \(-\{0,1\}[0-9.]*\), baseline \(-\{0,1\}[0-9.]*\).*/\1 \2/p' "$OUT/$s.log") || true
        read -r d e v < <(sed -n 's/^brain_census: difference \([-+0-9.]*\) a seat, standard error \([0-9.]*\) over [0-9]* seeds (\(.*\)); .*/\1 \2 \3/p' "$OUT/$s.log") || true
        printf '%-16s %-8s %8s %8s %8s %6s  %s\n' "$s" "$(needs "$s" | sed 's/^$/-/')" "${t:-?}" "${b:-?}" "${d:-?}" "${e:-?}" "${v:-failed}"
    done
} | tee "$OUT/table.txt"
