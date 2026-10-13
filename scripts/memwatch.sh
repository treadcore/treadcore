#!/usr/bin/env bash
# One line every N seconds: what the machine's memory is doing while a
# build runs. Run it in a second terminal beside `make warm`.
#
#   make mem            every 2 s
#   make mem N=5        every 5 s
#   scripts/memwatch.sh 2 > build/logs/mem.log   to keep it
#
# Columns: time, free, available (free + reclaimable cache: the number
# that matters), swap used, swap in/out rate (KB/s; sustained non-zero
# means the build is thrashing), rustc count, biggest process.
N="${1:-2}"
printf '%-8s %7s %7s %7s %6s %6s %5s  %s\n' time free avail swap si so rustc biggest
prev=$(awk '/pswpin|pswpout/ {printf "%s ", $2}' /proc/vmstat)
while true; do
    cur=$(awk '/pswpin|pswpout/ {printf "%s ", $2}' /proc/vmstat)
    read -r pi0 po0 <<<"$prev"; read -r pi1 po1 <<<"$cur"; prev="$cur"
    si=$(( (pi1 - pi0) * 4 / N )); so=$(( (po1 - po0) * 4 / N ))
    read -r free avail swapu <<<"$(free -m | awk 'NR==2{f=$4; a=$7} NR==3{s=$3} END{print f, a, s}')"
    rc=$(pgrep -xc rustc)
    big=$(ps -eo rss,comm --sort=-rss | awk 'NR==2{printf "%d MB %s", $1/1024, $2}')
    printf '%-8s %6dM %6dM %6dM %6d %6d %5d  %s\n' "$(date +%H:%M:%S)" "$free" "$avail" "$swapu" "$si" "$so" "$rc" "$big"
    sleep "$N"
done
