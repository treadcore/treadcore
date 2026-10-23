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
# SPLIT=n plays each switch's seeds as n censuses over consecutive seed
# ranges (CENSUS_SEED0), pooled by scripts/pool.sh into the switch's row:
# twelve switches are only twelve processes, and a machine with more
# threads than that sat half idle. SEEDS=200 SPLIT=2 JOBS=24 is backlog
# 22 on a 24-thread machine. SEED0 moves the whole range.
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
SPLIT="${SPLIT:-1}"
SEED0="${SEED0:-0}"
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

say "each switch alone: $SWITCHES; $SEEDS seeds from $SEED0 of $SECS s on $MAP, respawns $RESPAWNS, split $SPLIT, $JOBS at once -> $OUT"
BIN="$(census_bin "$OUT")"

census() { # census SWITCH [PART]: PART k of SPLIT plays the kth range
    local log="$OUT/$1.log" n="$SEEDS" from="$SEED0"
    if [ "$SPLIT" -gt 1 ]; then
        local per=$(( (SEEDS + SPLIT - 1) / SPLIT ))
        from=$(( SEED0 + $2 * per ))
        n=$(( SEED0 + SEEDS - from )); [ "$n" -gt "$per" ] && n=$per
        [ "$n" -gt 0 ] || return 0
        mkdir -p "$OUT/$1"; log="$OUT/$1/p$2.log"
    fi
    env CENSUS_MAP="$MAPFILE_" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SEEDS="$n" CENSUS_SEED0="$from" CENSUS_SECS="$SECS" \
        CENSUS_LEDGER=0 CENSUS_CAPS="$1" CENSUS_BASE_CAPS="$(needs "$1")" \
        "$BIN" --ignored brain_census --nocapture >"$log" 2>&1 \
        || warn "the census of $1 failed: see $log"
    echo "  $1${2:+ part $2} done"
}

running=0
for s in $SWITCHES; do
    for k in $(seq 0 $(( SPLIT - 1 ))); do
        if [ "$SPLIT" -gt 1 ]; then census "$s" "$k" & else census "$s" & fi
        running=$((running + 1))
        if [ "$running" -ge "$JOBS" ]; then
            wait -n
            running=$((running - 1))
        fi
    done
done
wait

{
    printf '%-16s %-8s %8s %8s %8s %6s  %s\n' switch with trial baseline diff se verdict
    for s in $SWITCHES; do
        if [ "$SPLIT" -gt 1 ]; then
            # The parts pooled: pool.sh's overall line, "every map +d +/- e over n seeds (verdict)".
            read -r d e v < <(scripts/pool.sh "$OUT/$s" 2>/dev/null | sed -n 's/^every map *\([-+0-9.]*\) +\/- \([0-9.]*\) over .*(\(.*\))$/\1 \2 \3/p') || true
            printf '%-16s %-8s %8s %8s %8s %6s  %s\n' "$s" "$(needs "$s" | sed 's/^$/-/')" - - "${d:-?}" "${e:-?}" "${v:-failed}"
            continue
        fi
        read -r t b < <(sed -n 's/^brain_census: trial \(-\{0,1\}[0-9.]*\), baseline \(-\{0,1\}[0-9.]*\).*/\1 \2/p' "$OUT/$s.log") || true
        read -r d e v < <(sed -n 's/^brain_census: difference \([-+0-9.]*\) a seat, standard error \([0-9.]*\) over [0-9]* seeds (\(.*\)); .*/\1 \2 \3/p' "$OUT/$s.log") || true
        printf '%-16s %-8s %8s %8s %8s %6s  %s\n' "$s" "$(needs "$s" | sed 's/^$/-/')" "${t:-?}" "${b:-?}" "${d:-?}" "${e:-?}" "${v:-failed}"
    done
} | tee "$OUT/table.txt"
