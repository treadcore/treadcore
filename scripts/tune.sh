#!/usr/bin/env bash
# The tuner (roadmap step 12g, documentation/LAB-REPORT-BRAIN.md section
# 8): a (1 + lambda) evolution strategy over the bots' weights. Each
# generation makes LAMBDA children of the best weights so far, each a
# copy with the tuned weights multiplied by exp(SIGMA x a normal draw),
# and plays each in the brain census (treadcore-brain fleet.rs
# `brain_census`): one team on the child, two on the baseline, the
# child's team turned round the three starts over the same SEEDS every
# time, so a difference is the weights' and not the dice's. A child's
# fitness is its team's mean score a seat less the baseline teams'; the
# best child replaces the parent only by beating it. Every match goes
# into reports/ledger/ledger.jsonl with its genome, so any line can be
# played again and the ratings (`ledger_ratings`) read across runs.
#
# A weights file is promoted to the default only by the rule in
# ROADMAP.md goal 4: it beats the baseline over the fixed census.
#
#   make tune                         5 generations of 4, 3 seeds, 900 s
#   GENERATIONS=10 LAMBDA=6 make tune
#   START=build/tune/x/best.toml      start from a file, not the defaults
#   TUNE="w_wounded press_odds k_long" the weights to tune (default below)
#   CAPS="smooth,priced_crossing"     the capabilities every child has
#   MAP=maps/naval.map SECS=600 SEEDS=6 SIGMA=0.2
#
# Output under build/tune/STAMP: each child's file and census log, and
# best.toml.

. "$(dirname "$0")/common.sh"
GENERATIONS="${GENERATIONS:-5}"
LAMBDA="${LAMBDA:-4}"
SIGMA="${SIGMA:-0.15}"
SEEDS="${SEEDS:-3}"
SECS="${SECS:-900}"
TUNE="${TUNE:-w_near w_wounded w_finish w_focus w_overkill w_in_range w_threat w_stick engage_odds press_odds w_exposure w_flank k_long c_boat swap_gap}"
OUT="$BUILDDIR/tune/$STAMP"
mkdir -p "$OUT"
MAPARG=()
[ -n "${MAP:-}" ] && MAPARG=(CENSUS_MAP="$(cd "$(dirname "$MAP")" && pwd)/$(basename "$MAP")" CENSUS_RESPAWNS="${RESPAWNS:-3}")

census() { # census FILE LOG -> prints the fitness
    env "${MAPARG[@]}" CENSUS_WEIGHTS="$1" CENSUS_SEEDS="$SEEDS" CENSUS_SECS="$SECS" \
        cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture >"$2" 2>&1 || die "the census failed: see $2"
    sed -n 's/^brain_census: trial \(-\{0,1\}[0-9.]*\), baseline \(-\{0,1\}[0-9.]*\).*/\1 \2/p' "$2" | awk '{ printf "%.4f\n", $1 - $2 }'
}

# The parent: START, or every weight at its default (the census writes
# them out when asked, with no match played).
BEST="$OUT/best.toml"
if [ -n "${START:-}" ]; then
    cp "$START" "$BEST"
else
    CENSUS_DUMP="$BEST" CENSUS_SEEDS=0 cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture >/dev/null 2>&1 || die "could not write the default weights"
fi
[ -n "${CAPS:-}" ] && sed -i "s/^caps = .*/caps = baseline, ${CAPS}/" "$BEST"
say "tuning: $GENERATIONS generations of $LAMBDA, sigma $SIGMA, $SEEDS seeds of $SECS s -> $OUT"
BESTFIT="$(census "$BEST" "$OUT/parent.log")"
echo "  parent fitness $BESTFIT"

for g in $(seq 1 "$GENERATIONS"); do
    for k in $(seq 1 "$LAMBDA"); do
        CHILD="$OUT/g$g-c$k.toml"
        awk -v names=" $TUNE " -v sigma="$SIGMA" -v seed="$RANDOM$g$k" '
            BEGIN { srand(seed) }
            function normal() { return sqrt(-2 * log(1 - rand())) * cos(6.283185307 * rand()) }
            /^[a-z_]+ = / && index(names, " " $1 " ") > 0 {
                v = $3 * exp(sigma * normal())
                sub(/= [^ ]+/, "= " sprintf("%.4f", v))
            }
            { print }' "$BEST" >"$CHILD"
        FIT="$(census "$CHILD" "$OUT/g$g-c$k.log")"
        echo "  generation $g child $k: fitness $FIT"
        if awk -v a="$FIT" -v b="$BESTFIT" 'BEGIN { exit !(a > b) }'; then
            cp "$CHILD" "$BEST"
            BESTFIT="$FIT"
            echo "    the new parent"
        fi
    done
done
say "best fitness $BESTFIT: $BEST"
grep -vE '^#' "$BEST" | diff <(CENSUS_DUMP=/dev/stdout CENSUS_SEEDS=0 cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture 2>/dev/null | grep -E '^[a-z_]+ = ') - | grep '^>' || echo "  (the defaults won)"
