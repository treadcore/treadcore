#!/usr/bin/env bash
# SPSA over the bots' weights (roadmap backlog 23; CI-001 section VI.C;
# documentation/LAB-REPORT-BRAIN.md section 40). Simultaneous perturbation
# stochastic approximation (Spall 1992), in log space, each tuned weight
# a positive number. Each step draws a sign for every tuned weight and
# plays one fair census with the trial team on theta * exp(+c delta) and
# the two baseline teams on theta * exp(-c delta) (`CENSUS_BASE_WEIGHTS`),
# so the census's paired difference a seat is f(+) - f(-) itself, seed by
# seed. The gradient is that difference over 2 c delta; the step moves
# log theta along it by a, capped at MAXSTEP. The seeds are fresh every
# step (SEED0 + k SEEDS), so no step is tuned to the last one's starts.
# The census's standard error is printed with each step.
#
#   make spsa                          8 steps, 12 seeds a step, naval.map
#   TUNE="dodge_horizon dodge_clear"   the weights to tune (positive)
#   START=file.toml                    start from a file, not the defaults
#   SET="landfall_memory=3"            values set on the start (a planted test)
#   STEPS=10 SEEDS=12 SEED0=1000 A=0.6 C=0.4 MAXSTEP=0.5 SECS=900 JOBS=3
#
# Output under build/spsa/STAMP: each step's plus and minus files and
# census logs, steps.tsv (k, the weights, d, its error), and theta.toml,
# the last weights. A result is promoted only after it beats the
# defaults on 48 seeds the search did not use (ROADMAP goal 4).

. "$(dirname "$0")/common.sh"
STEPS="${STEPS:-8}"
SEEDS="${SEEDS:-12}"
SEED0="${SEED0:-1000}"
SECS="${SECS:-900}"
A="${A:-0.6}"
C="${C:-0.4}"
MAXSTEP="${MAXSTEP:-0.5}"
JOBS="${JOBS:-3}"
MAP="${MAP:-maps/naval.map}"
RESPAWNS="${RESPAWNS:-3}"
TUNE="${TUNE:-dodge_horizon dodge_margin dodge_clear}"
OUT="$BUILDDIR/spsa/$STAMP"
mkdir -p "$OUT"
MAPFILE_="$(census_maps "$MAP")"

BIN="$(census_bin "$OUT")"
THETA="$OUT/theta.toml"
if [ -n "${START:-}" ]; then
    cp "$START" "$THETA"
else
    CENSUS_DUMP="$THETA" CENSUS_SEEDS=0 "$BIN" --ignored brain_census >/dev/null 2>&1 || die "could not write the default weights"
fi
for kv in ${SET:-}; do
    sed -i "s/^${kv%%=*} = [^ ]*/${kv%%=*} = ${kv#*=}/" "$THETA"
done

get() { # get FILE NAME
    sed -n "s/^$2 = \([^ ]*\).*/\1/p" "$1"
}
put() { # put FILE NAME VALUE
    sed -i "s/^$2 = [^ ]*/$2 = $3/" "$1"
}
for w in $TUNE; do
    v="$(get "$THETA" "$w")"
    [ -n "$v" ] || die "no weight $w"
    awk -v v="$v" 'BEGIN { exit !(v > 0) }' || die "$w is $v: SPSA here tunes positive weights only"
done

# One census in JOBS ranges side by side; prints the mean difference a
# seat and its standard error over all the seeds.
census() { # census PLUS MINUS FIRST LOGPREFIX
    local per=$(( (SEEDS + JOBS - 1) / JOBS )) i=0 s0 n
    while [ $((i * per)) -lt "$SEEDS" ]; do
        s0=$(( $3 + i * per ))
        n=$(( SEEDS - i * per < per ? SEEDS - i * per : per ))
        env CENSUS_MAP="$MAPFILE_" CENSUS_RESPAWNS="$RESPAWNS" CENSUS_SECS="$SECS" CENSUS_LEDGER=0 \
            CENSUS_SEEDS="$n" CENSUS_SEED0="$s0" CENSUS_WEIGHTS="$1" CENSUS_BASE_WEIGHTS="$2" \
            "$BIN" --ignored brain_census --nocapture >"$4-$i.log" 2>&1 &
        i=$((i + 1))
    done
    wait
    cat "$4"-*.log | sed -n 's/^brain_census: difference.*a seed: //p' | tr ' ' '\n' | awk 'NF {
        n++; s += $1; q += $1 * $1 } END {
        m = s / n; v = n > 1 ? (q - n * m * m) / (n - 1) : 0; if (v < 0) v = 0
        printf "%.4f %.4f\n", m, sqrt(v / n) }'
}

say "spsa: $STEPS steps over [$TUNE], $SEEDS fresh seeds a step from $SEED0, a $A c $C -> $OUT"
printf "k\t%s\td\tse\n" "$(echo $TUNE | tr ' ' '\t')" >"$OUT/steps.tsv"
for k in $(seq 1 "$STEPS"); do
    ak="$(awk -v a="$A" -v k="$k" 'BEGIN { printf "%.5f", a / (k + 1) ^ 0.602 }')"
    ck="$(awk -v c="$C" -v k="$k" 'BEGIN { printf "%.5f", c / k ^ 0.101 }')"
    PLUS="$OUT/k$k-plus.toml"
    MINUS="$OUT/k$k-minus.toml"
    cp "$THETA" "$PLUS"
    cp "$THETA" "$MINUS"
    declare -A SIGN=()
    for w in $TUNE; do
        SIGN[$w]=$(( RANDOM % 2 * 2 - 1 ))
        v="$(get "$THETA" "$w")"
        put "$PLUS" "$w" "$(awk -v v="$v" -v c="$ck" -v s="${SIGN[$w]}" 'BEGIN { printf "%.4f", v * exp(c * s) }')"
        put "$MINUS" "$w" "$(awk -v v="$v" -v c="$ck" -v s="${SIGN[$w]}" 'BEGIN { printf "%.4f", v * exp(-c * s) }')"
    done
    read -r D SE < <(census "$PLUS" "$MINUS" $(( SEED0 + (k - 1) * SEEDS )) "$OUT/k$k")
    [ -n "$D" ] || die "step $k: the census printed no difference (see $OUT/k$k-*.log)"
    row="$k"
    for w in $TUNE; do
        v="$(get "$THETA" "$w")"
        nv="$(awk -v v="$v" -v d="$D" -v a="$ak" -v c="$ck" -v s="${SIGN[$w]}" -v m="$MAXSTEP" 'BEGIN {
            step = a * d / (2 * c * s); if (step > m) step = m; if (step < -m) step = -m
            printf "%.4f", v * exp(step) }')"
        put "$THETA" "$w" "$nv"
        row="$row\t$nv"
    done
    printf "%b\t%s\t%s\n" "$row" "$D" "$SE" >>"$OUT/steps.tsv"
    echo "  step $k: d $D +/- $SE (a $ak, c $ck) -> $(for w in $TUNE; do printf '%s %s  ' "$w" "$(get "$THETA" "$w")"; done)"
done
say "spsa done: $THETA"
column -t "$OUT/steps.tsv"
