#!/usr/bin/env bash
# The profiles' round robin (roadmap backlog 60): the fair census
# (scripts/census.sh) for every ordered pair of play styles, one team of
# the first against two of the second, on the same seeds, so the payoff
# matrix reads whether the styles are ordered or go round in a cycle.
# The defaults are one of the styles. Above zero the row's style scores
# more a seat than the column's.
#
#   scripts/roundrobin.sh                       profiles/*.weights and the defaults
#   PROFILES="profiles/navy.weights defaults" SEEDS=44 scripts/roundrobin.sh
#
# SEEDS, SEED0, SECS, MAP (default all), RESPAWNS and JOBS go to each
# census. Each pair's census is under build/census/ as usual; the
# matrix, every map and each map alone, under build/roundrobin/STAMP.
#
# LIST=1 prints the censuses this would play, a line each, and plays none:
# the pair, the trial's weights file and the base's, tab-separated, a
# file's lines joined by ";" (orrery's fleet queue plays them, a census a
# machine; LAB-REPORT-BRAIN.md 114). A first line "#settings ..." gives
# SEEDS, SEED0, SECS, MAP and RESPAWNS.

. "$(dirname "$0")/common.sh"
PROFILES="${PROFILES:-defaults $(ls "$ROOT"/profiles/*.weights)}"
export SEEDS="${SEEDS:-110}" SEED0="${SEED0:-60000}" MAP="${MAP:-all}"
name() { basename "$1" .weights; }
file() { [ "$1" = defaults ] || echo "$1"; }
if [ -n "${LIST:-}" ]; then
    echo "#settings SEEDS=$SEEDS SEED0=$SEED0 SECS=${SECS:-900} MAP=$MAP RESPAWNS=${RESPAWNS:-3}"
    text() { if [ "$1" = defaults ]; then echo "caps = baseline"; else grep -v '^[[:space:]]*\(#\|$\)' "$1" | paste -sd';' -; fi; }
    for a in $PROFILES; do
        for b in $PROFILES; do
            [ "$a" = "$b" ] && continue
            printf '%s-vs-%s\t%s\t%s\n' "$(name "$a")" "$(name "$b")" "$(text "$a")" "$(text "$b")"
        done
    done
    exit 0
fi
OUT="$BUILDDIR/roundrobin/$STAMP"
mkdir -p "$OUT"

say "round robin: $(for p in $PROFILES; do printf '%s ' "$(name "$p")"; done)over $SEEDS seeds from $SEED0 on $MAP -> $OUT"
for a in $PROFILES; do
    for b in $PROFILES; do
        [ "$a" = "$b" ] && continue
        pair="$(name "$a")-vs-$(name "$b")"
        TRIAL="$(file "$a")" BASE="$(file "$b")" "$ROOT/scripts/census.sh" >"$OUT/$pair.log" 2>&1 \
            || warn "the census of $pair failed: see $OUT/$pair.log"
        grep -a '^[a-z]' "$OUT/$pair.log" | grep -a ' over ' >"$OUT/$pair.txt" || true
        echo "  $pair: $(head -1 "$OUT/$pair.txt" | sed 's/^every map *//')"
        sleep 1 # each census its own STAMP
    done
done

# The matrix for every map, then for each map alone.
matrix() { # matrix LABEL
    printf '%-10s' "$1"; for b in $PROFILES; do printf ' %15s' "$(name "$b")"; done; echo
    for a in $PROFILES; do
        printf '%-10s' "$(name "$a")"
        for b in $PROFILES; do
            if [ "$a" = "$b" ]; then printf ' %15s' "."; continue; fi
            v="$(grep -a "^$1 " "$OUT/$(name "$a")-vs-$(name "$b").txt" | awk '{print $(NF-8), $(NF-6), ($0 ~ /outside/) ? "*" : ""}')"
            printf ' %15s' "$(echo "$v" | awk '{printf "%s+/-%s%s", $1, $2, $3}')"
        done
        echo
    done
    echo
}
{
    echo "Each cell: the row's style less the column's, a seat (* past two errors)."
    echo
    matrix "every map"
    first="$(ls "$OUT"/*.txt | head -1)"
    for m in $(tail -n +2 "$first" | awk '{print $1}'); do matrix "$m"; done
} | tee "$OUT/matrix.txt"
