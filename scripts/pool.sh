#!/usr/bin/env bash
# Pool a census split over jobs (roadmap backlog 25, LR 71): every
# `brain_census` log in DIR (*.log), its seeds' differences read from
# the "a seed:" lists, the mean a seat and its standard error over all
# of them, then each map's alone (the "on MAP:" lines of a census over
# more than one map). Outside two errors is "outside the noise".
#
#   scripts/pool.sh build/census/STAMP
set -euo pipefail
DIR="${1:?usage: pool.sh DIR}"
stat() { # stat LABEL: the differences on stdin, one a line
    awk -v l="$1" '{x[NR]=$1; s+=$1} END {
        if (NR == 0) { printf "%-16s no seeds\n", l; exit }
        m = s / NR; v = 0; for (i = 1; i <= NR; i++) v += (x[i] - m) ^ 2
        se = NR > 1 ? sqrt(v / (NR - 1)) / sqrt(NR) : 0
        printf "%-16s %+6.2f +/- %4.2f over %3d seeds (%s)\n", l, m, se, NR, (se > 0 && (m > 2 * se || m < -2 * se)) ? "outside the noise" : "inside the noise" }'
}
seeds() { sed 's/.*a seed: //' | tr ' ' '\n' | grep . || true; }
grep -h '^brain_census: difference' "$DIR"/*.log | seeds | stat "every map"
for m in $(grep -ho '^brain_census: on [^:]*:' "$DIR"/*.log | sed 's/^brain_census: on //; s/:$//' | awk '!s[$0]++'); do
    grep -h "^brain_census: on $m:" "$DIR"/*.log | seeds | stat "$m"
done
