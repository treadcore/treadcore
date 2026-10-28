# shellcheck shell=bash
# treadcore's rounds for orrery's fleet-train (LAB-REPORT-BRAIN.md section 109):
#
#   ~/code/orrery/tools/fleet-train.sh run    scripts/fleet-train-stages.sh [ROUNDS]
#   ~/code/orrery/tools/fleet-train.sh status scripts/fleet-train-stages.sh [--json]
#   ~/code/orrery/tools/fleet-train.sh stop   scripts/fleet-train-stages.sh
#
# Run on the coordinator (the Mac). Each round:
#
#   record  new matches of the scripted brain on every spare CPU in the hosts
#           file, SEEDS_A_ROUND of them from the loop's next seed
#           (scripts/fleet-record.sh, its own run ftNNN)
#   train   on the GPU node: the loop's best net, continued (START=) on this
#           round's matches, with the fixed held-out seeds kept out
#   judge   on the census machine: a census of the candidate, paired by seed
#           with the best's on the same binary (the best is read again when
#           the binary differs); the candidate becomes the loop's best only
#           if it is above it outside two standard errors
#
# The loop's best is the loop's, never the official net: nets/map7.bin
# changes only when Paul says (LAB-REPORT-FLEET VII). A round that finds a
# better net says so in its summary and in the event stream.
#
# Settings (the environment overrides each):
TC="${TC:-$HOME/code/treadcore}"
FT_DIR="${FT_DIR:-$TC/build/fleet-train}"
FT_STAGES="record train judge"
STORE="${STORE:-user@192.168.8.181}"            # the GPU node, where the shards are gathered
CENSUS_HOST="${CENSUS_HOST:-user@192.168.8.224}" # every census of the loop on this one machine
HOSTS="${HOSTS:-$TC/build/fleet/rec2400/hosts}" # the recording hosts, copied into each round's run
SEEDS_A_ROUND="${SEEDS_A_ROUND:-1200}"
PER="${PER:-12}"
FIRST_SEED="${FIRST_SEED:-9003660}"             # after rec60, rec1200 and rec2400
HELD="${HELD:-9000054-9000059,9001140-9001259}" # bc1's held-out seeds and rec1200's last 120
TRAIN_ARGS="${TRAIN_ARGS:-4 32768 3e-3}"
# Recordings already made, used by the first rounds instead of recording:
# RUN:FIRST_SEED:COUNT, a round each, in order. rec2400 is two rounds, since
# calber's memory holds about 1,500 seeds of samples at once.
PREMADE="${PREMADE:-rec2400:9001260:1200 rec2400:9002460:1200}"        # epochs, batch, rate: a continuation, gentler than from scratch
CENSUS_ENV="${CENSUS_ENV:-SEEDS=110 SEED0=500000 SECS=900 MAP=all RESPAWNS=3 JOBS=8}"
BC="${BC:-\$HOME/code/treadcore/tools/bc/target/release/treadcore-bc}"

ssh_() { ssh -o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=30 "$@"; }

# The loop's state, created on first use: the best is bc2 (LR 108), its
# census the one read on columbia at 20261027-200745.
ft_init() {
    [ -f "$FT_DIR/next_seed" ] || echo "$FIRST_SEED" > "$FT_DIR/next_seed"
    if [ ! -f "$FT_DIR/best" ]; then
        echo "bc2 (LR 108): census -19.76 +/- 0.39 against the defaults" > "$FT_DIR/best"
        echo "nets/bc2.bin" > "$FT_DIR/best.net"           # on the GPU node and the census machine
        echo "build/census/20261027-200745" > "$FT_DIR/best.census"
    fi
}

stage_record() {
    local R="$1" RD="$2" s0 run
    ft_init
    local pre
    pre="$(echo "$PREMADE" | tr ' ' '\n' | grep -v '^$' | sed -n "${R}p")"
    if [ -n "$pre" ]; then
        echo "$pre" | tr ':' ' ' > "$RD/run"
        "$TC/scripts/fleet-record.sh" consolidate "${pre%%:*}" | tee "$RD/record.txt"
        echo "round $R trains on $pre, recorded before the loop began"
        return 0
    fi
    s0=$(cat "$FT_DIR/next_seed")
    run="ft$(printf '%03d' "$R")"
    echo "$run $s0 $SEEDS_A_ROUND" > "$RD/run"
    [ -d "$TC/build/fleet/$run" ] || "$TC/scripts/fleet-record.sh" plan "$run" "$s0" "$SEEDS_A_ROUND" "$PER"
    cp "$HOSTS" "$TC/build/fleet/$run/hosts"
    echo "$STORE" > "$TC/build/fleet/$run/store"
    "$TC/scripts/fleet-record.sh" prepare "$run"
    "$TC/scripts/fleet-record.sh" run "$run"
    "$TC/scripts/fleet-record.sh" consolidate "$run" | tee "$RD/record.txt"
    echo $(( s0 + SEEDS_A_ROUND )) > "$FT_DIR/next_seed"
    echo "recorded seeds $s0 to $(( s0 + SEEDS_A_ROUND - 1 )) as run $run"
}

stage_train() {
    local R="$1" RD="$2" run net
    local lo n
    read -r run lo n < "$RD/run"
    net="$(cat "$FT_DIR/best.net")"
    # The round's data: its own shards, and the shards holding the held-out
    # seeds (rec60's file and rec1200's last five), so every round is judged
    # on the same matches.
    ssh_ "$STORE" "set -e; d=\$HOME/bc/ft$(printf '%03d' "$R"); rm -rf \$d; mkdir -p \$d
        for f in \$HOME/fleet/$run/shards/*.bin; do s=\$(basename \$f .bin); [ \$s -ge $lo ] && [ \$s -lt $(( lo + n )) ] && ln -s \$f \$d/; done; true
        ln -s \$HOME/bc/rec60.bin \$d/9000000.bin
        for s in 9001140 9001164 9001188 9001212 9001236; do ln -s \$HOME/fleet/rec1200/shards/\$s.bin \$d/; done
        cd \$d && HELD=$HELD START=\$HOME/code/treadcore/$net BC_DEVICE=cuda $BC \$d \$d/cand.bin $TRAIN_ARGS > \$d/train.log 2>&1
        grep -v 'DEINIT\|^thread\|^note\|^\$' \$d/train.log" | tee "$RD/train.txt"
    awk '/seeds 9000054-9000059/ {f=1} f && /^  drive/ {print "candidate drive on bc1'"'"'s held-out seeds: " $2; exit}' "$RD/train.txt"
}

# The per-seed differences of a census folder on the census machine, as
# "SEED VALUE" lines.
census_seeds() {
    ssh_ "$CENSUS_HOST" "cd ~/code/treadcore/$1 && for f in g-*.log; do s=\${f#g-}; s=\${s%.log}; echo \"\$s \$(sed 's/\x1b\[[0-9;]*m//g' \$f | grep '^brain_census: difference' | sed 's/.*a seed: //')\"; done" \
        | awk '{for (i = 2; i <= NF; i++) print $1 + i - 2, $i}'
}
census_bin_hash() { ssh_ "$CENSUS_HOST" "sha256sum ~/code/treadcore/$1/census-bin | cut -c1-64"; }

# A census of NET (a path under the census machine's checkout); prints its folder.
census_of() {
    ssh_ "$CENSUS_HOST" "cd ~/code/treadcore && $CENSUS_ENV CENSUS_NET=\$HOME/code/treadcore/$1 make census > build/ft-census.log 2>&1; ls -td build/census/*/ | head -1 | sed 's:/\$::'"
}

stage_judge() {
    local R="$1" RD="$2" run cand best_dir cand_dir verdict
    run="ft$(printf '%03d' "$R")"   # the candidate's name, whatever run it trained on
    cand="nets/ft/$run.bin"
    ssh_ "$CENSUS_HOST" "mkdir -p ~/code/treadcore/nets/ft"
    ssh_ "$STORE" "cat ~/bc/$run/cand.bin" | ssh_ "$CENSUS_HOST" "cat > ~/code/treadcore/$cand"
    ssh_ "$STORE" "mkdir -p ~/code/treadcore/nets/ft && cp ~/bc/$run/cand.bin ~/code/treadcore/$cand"
    cand_dir="$(census_of "$cand")"
    echo "candidate's census: $cand_dir"
    best_dir="$(cat "$FT_DIR/best.census")"
    if [ "$(census_bin_hash "$cand_dir")" != "$(census_bin_hash "$best_dir")" ]; then
        echo "the best was read on another binary: reading it again on this one"
        best_dir="$(census_of "$(cat "$FT_DIR/best.net")")"
        echo "$best_dir" > "$FT_DIR/best.census"
    fi
    census_seeds "$cand_dir" > "$RD/cand.seeds"
    census_seeds "$best_dir" > "$RD/best.seeds"
    verdict="$(python3 - "$RD/cand.seeds" "$RD/best.seeds" <<'EOF'
import statistics as st, sys
a = dict(l.split() for l in open(sys.argv[1])); b = dict(l.split() for l in open(sys.argv[2]))
k = sorted(set(a) & set(b), key=int)
d = [float(a[s]) - float(b[s]) for s in k]
ca = [float(a[s]) for s in k]
m, se = st.mean(d), st.stdev(d) / len(d) ** 0.5
print(f"{'better' if m > 2 * se else 'not better'} {st.mean(ca):.2f} {st.stdev(ca) / len(ca) ** 0.5:.2f} {m:+.2f} {se:.2f} {len(k)}")
EOF
)"
    local v1 cm cse dm dse n
    read -r v1 cm cse dm dse n <<< "$(echo "$verdict" | sed 's/^not better/not_better/; s/^better/better/')"
    echo "census: candidate $cm +/- $cse against the defaults; paired with the best over $n seeds: $dm +/- $dse"
    if [ "$v1" = better ]; then
        echo "$run (round $R): census $cm +/- $cse against the defaults, $dm +/- $dse over the previous best" > "$FT_DIR/best"
        echo "$cand" > "$FT_DIR/best.net"
        echo "$cand_dir" > "$FT_DIR/best.census"
        echo "round $R: $run is the loop's new best ($dm +/- $dse over the last); official is still Paul's to decide" > "$RD/summary.txt"
    else
        echo "round $R: $run not better than the best ($dm +/- $dse); the best stays $(cut -d' ' -f1 "$FT_DIR/best")" > "$RD/summary.txt"
    fi
    cat "$RD/summary.txt"
}

# For `status --json` (orrery's Jobs view): the lanes of the round in hand's
# recording run, or of RUNS (space-separated run names) when set, so a run
# started by hand can be watched the same way.
ft_detail() {
    local r runs="${RUNS:-}"
    if [ -z "$runs" ]; then
        r=$(cat "$FT_DIR/round" 2>/dev/null || echo 0)
        [ "$r" -gt 0 ] && runs="$(cut -d' ' -f1 "$FT_DIR/rounds/$(printf '%03d' "$r")/run" 2>/dev/null || echo "ft$(printf '%03d' "$r")")"
    fi
    local dirs=""
    for x in $runs; do [ -d "$TC/build/fleet/$x" ] && dirs="$dirs $TC/build/fleet/$x"; done
    if [ -n "$dirs" ]; then python3 -I "$TC/scripts/fleet-jobs.py" $dirs; else echo '{"runs": []}'; fi
}
