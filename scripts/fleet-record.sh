#!/bin/bash
# Behaviour cloning's recordings (LAB-REPORT-BRAIN.md sections 106 and 108),
# made on every spare CPU of the fleet and gathered on one store, where the
# GPU trains on them.
#
#   scripts/fleet-record.sh plan RUN SEED0 COUNT PER   queue COUNT seeds in jobs of PER
#   scripts/fleet-record.sh prepare RUN                each host: a worktree at the
#                                                      run's commit and a frozen binary
#   scripts/fleet-record.sh run RUN                    a lane a host until the queue
#                                                      is empty (run it in the background)
#   scripts/fleet-record.sh status RUN
#   scripts/fleet-record.sh consolidate RUN            check every shard on the store
#
# It runs on the coordinator, the one machine with ssh to all the others,
# and keeps the run's state in build/fleet/RUN/ there:
#
#   hosts         NAME TARGET THREADS NICE, a line a host; TARGET "-" is
#                 this machine
#   store         TARGET: where the shards are gathered, in ~/fleet/RUN/shards
#   commit        the commit every host builds (default: HEAD at plan time)
#   queue/        a file a job, named by its first seed; its text "SEED0 SEEDS"
#   claimed/NAME/ done/NAME/ failed/   where a job is now
#   lanes/NAME.log, lanes/NAME.beat    each lane's log, and its heartbeat
#   manifest.tsv  a line a shard: job host seed0 seeds samples bytes sha256
#                 binary_sha256 seconds
#
# A job is claimed with mv, which is atomic, so two lanes never take the
# same one; a job that fails goes to failed/ with its log and is not
# retried by itself (requeue it by moving it back). Each host plays on its
# own binary, built from the same commit: a recording is training data,
# not a census, so binaries need not match across machines, but each
# shard names the binary that made it.
#
# On each host the run lives in ~/fleet/RUN: wt/ (the worktree), census-bin,
# out/ (a shard while it is made). A shard is streamed to the store, its
# sha256 checked at both ends, then removed from the host.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SECS="${SECS:-600}"
SAMPLE=$((143 * 4 + 7 + 3 + 4))   # TCBC2: senses, labels, masks, seed

die() { printf '%s\n' "$*" >&2; exit 1; }
usage() { sed -n '6,13p' "$0" >&2; exit 2; }

[ $# -ge 2 ] || usage
CMD="$1" RUN="$2"
shift 2
D="$ROOT/build/fleet/$RUN"

# on TARGET CMD...: run a shell command on a host, or here for "-".
on() {
    local t="$1"
    shift
    if [ "$t" = - ]; then
        bash -c "$*"
    else
        # Under bash whatever the host's login shell (nasser's is busybox
        # ash, and common.sh needs BASH_SOURCE); sent as base64 so no quoting
        # is read twice.
        ssh -o BatchMode=yes -o ConnectTimeout=10 -o ServerAliveInterval=30 "$t" \
            "exec bash -c \"\$(echo $(printf '%s' "$*" | base64 | tr -d '\n') | base64 -d)\""
    fi
}

# The environment a remote command needs: cargo for a rustup install.
ENV='[ -f ~/.cargo/env ] && . ~/.cargo/env; export PATH="$HOME/.cargo/bin:$PATH";'

# ONLY=NAME: prepare or run that host alone (a host joining a run under way:
# add its line to hosts, then ONLY=NAME prepare, then ONLY=NAME run).
hosts() { grep -v '^#' "$D/hosts" | grep -v '^[[:space:]]*$' | awk -v o="${ONLY:-}" 'o == "" || $1 == o'; }
store() { cat "$D/store"; }

case "$CMD" in
plan)
    [ $# -eq 3 ] || usage
    SEED0="$1" COUNT="$2" PER="$3"
    [ ! -d "$D/queue" ] || die "$D exists: a run is planned once"
    mkdir -p "$D/queue" "$D/claimed" "$D/done" "$D/failed" "$D/lanes" "$D/bin"
    s="$SEED0"
    end=$((SEED0 + COUNT))
    while [ "$s" -lt "$end" ]; do
        n=$PER
        [ $((s + n)) -le "$end" ] || n=$((end - s))
        echo "$s $n" > "$D/queue/$s"
        s=$((s + n))
    done
    git -C "$ROOT" rev-parse HEAD > "$D/commit"
    [ -f "$D/hosts" ] || printf '# NAME TARGET THREADS NICE\n' > "$D/hosts"
    : > "$D/manifest.tsv"
    echo "planned $(ls "$D/queue" | wc -l | tr -d ' ') jobs of up to $PER seeds from $SEED0 at $(cut -c1-7 "$D/commit");"
    echo "now write $D/hosts and $D/store"
    ;;

prepare)
    C="$(cat "$D/commit")"
    S="$(store)"
    on "$S" "mkdir -p fleet/$RUN/shards"
    hosts | while read -r name target threads nice; do
        echo "== $name: worktree at ${C:0:7}, building"
        # The worktree is the run's own, so the host's checkout is left as it
        # was. Its build has a target directory of the fleet's own, shared by
        # runs: sharing the checkout's let columbia's next census pick up the
        # fleet's binary instead of building its own (LR 108 D; the same
        # source, so the same matches, but not the binary it named).
        on "$target" "$ENV set -e
            mkdir -p ~/fleet/$RUN/out
            cd ~/code/treadcore
            git fetch -q origin
            git worktree remove --force ~/fleet/$RUN/wt 2>/dev/null || true
            git worktree prune
            git worktree add -q --detach ~/fleet/$RUN/wt $C
            cd ~/fleet/$RUN/wt
            export CARGO_TARGET_DIR=\$HOME/fleet/target
            . scripts/common.sh
            census_bin ~/fleet/$RUN >/dev/null
            sha256sum ~/fleet/$RUN/census-bin | cut -c1-64" < /dev/null > "$D/bin/$name" \
            || die "$name: the worktree or the build failed"
        echo "   census-bin $(cut -c1-12 "$D/bin/$name")"
    done
    ;;

run)
    for h in $(hosts | cut -d' ' -f1); do [ -s "$D/bin/$h" ] || die "$h: prepare first"; done
    S="$(store)"
    lane() {
        local name="$1" target="$2" threads="$3" nice="$4" job s0 n t0 secs bytes sha sha2 log
        mkdir -p "$D/claimed/$name" "$D/done/$name"
        log="$D/lanes/$name.log"
        while :; do
            job=''
            for f in "$D"/queue/*; do
                [ -e "$f" ] || break
                if mv "$f" "$D/claimed/$name/" 2>/dev/null; then job="$(basename "$f")"; break; fi
            done
            [ -n "$job" ] || { echo "$(date +%T) queue empty: lane ends" >> "$log"; date +%s > "$D/lanes/$name.beat"; return 0; }
            read -r s0 n < "$D/claimed/$name/$job"
            echo "$(date +%s) $job recording" > "$D/lanes/$name.beat"
            echo "$(date +%T) $job: seeds $s0 +$n on $threads threads" >> "$log"
            t0=$(date +%s)
            if ! on "$target" "set -o pipefail; cd ~/fleet/$RUN/wt && RECORD=\$HOME/fleet/$RUN/out/$job.bin SEED0=$s0 SEEDS=$n SECS=$SECS THREADS=$threads \
                    nice -n $nice ~/fleet/$RUN/census-bin --ignored learn::harness::train --nocapture 2>&1 | tail -4 \
                    && [ \$(wc -c < ~/fleet/$RUN/out/$job.bin) -gt 16 ]" \
                    < /dev/null >> "$log" 2>&1; then
                echo "$(date +%T) $job: FAILED to record" >> "$log"
                mv "$D/claimed/$name/$job" "$D/failed/"
                continue
            fi
            secs=$(( $(date +%s) - t0 ))
            echo "$(date +%s) $job sending" > "$D/lanes/$name.beat"
            sha="$(on "$target" "sha256sum ~/fleet/$RUN/out/$job.bin | cut -c1-64; wc -c < ~/fleet/$RUN/out/$job.bin" < /dev/null)"
            bytes="$(echo "$sha" | sed -n 2p | tr -d ' ')"
            sha="$(echo "$sha" | sed -n 1p)"
            # To the store: in place if this host is the store, else streamed
            # (through this machine when neither end is it).
            if [ "$target" = "$S" ]; then
                on "$S" "mv ~/fleet/$RUN/out/$job.bin ~/fleet/$RUN/shards/$job.bin.part" < /dev/null
            elif [ "$target" = - ]; then
                cat "$HOME/fleet/$RUN/out/$job.bin" | on "$S" "cat > ~/fleet/$RUN/shards/$job.bin.part"
            else
                on "$target" "cat ~/fleet/$RUN/out/$job.bin" < /dev/null | on "$S" "cat > ~/fleet/$RUN/shards/$job.bin.part"
            fi
            sha2="$(on "$S" "sha256sum ~/fleet/$RUN/shards/$job.bin.part | cut -c1-64" < /dev/null)"
            if [ "$sha" != "$sha2" ]; then
                echo "$(date +%T) $job: FAILED, the store's copy differs ($sha2 against $sha)" >> "$log"
                mv "$D/claimed/$name/$job" "$D/failed/"
                continue
            fi
            on "$S" "mv ~/fleet/$RUN/shards/$job.bin.part ~/fleet/$RUN/shards/$job.bin" < /dev/null
            [ "$target" = "$S" ] || on "$target" "rm -f ~/fleet/$RUN/out/$job.bin" < /dev/null
            printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$job" "$name" "$s0" "$n" \
                $(( (bytes - 16) / SAMPLE )) "$bytes" "$sha" "$(cat "$D/bin/$name")" "$secs" >> "$D/manifest.tsv"
            mv "$D/claimed/$name/$job" "$D/done/$name/"
            echo "$(date +%T) $job: done, $bytes bytes, ${secs} s" >> "$log"
        done
    }
    # Read in this shell, not a pipe's, so wait sees every lane.
    while read -r name target threads nice; do
        lane "$name" "$target" "$threads" "$nice" &
    done < <(hosts)
    wait
    echo "run $RUN: $(ls "$D/queue" | wc -l | tr -d ' ') queued, $(cat "$D"/done/*/* 2>/dev/null | wc -l | tr -d ' ') done, $(ls "$D/failed" | wc -l | tr -d ' ') failed"
    ;;

status)
    now=$(date +%s)
    echo "queued $(ls "$D/queue" | wc -l | tr -d ' '), failed $(ls "$D/failed" | wc -l | tr -d ' ')"
    hosts | while read -r name target threads nice; do
        b="$(cat "$D/lanes/$name.beat" 2>/dev/null || echo "$now -")"
        printf '%-14s claimed %s, done %3s; %s, %s s ago\n' "$name" \
            "$(ls "$D/claimed/$name" 2>/dev/null | tr '\n' ' ')" \
            "$(ls "$D/done/$name" 2>/dev/null | wc -l | tr -d ' ')" \
            "$(echo "$b" | cut -d' ' -f2-)" $(( now - $(echo "$b" | cut -d' ' -f1) ))
    done
    awk -F'\t' '{s+=$5; b+=$6; n+=$4} END {printf "shards %d: %d seeds, %d samples, %.1f GB\n", NR, n, s, b/1e9}' "$D/manifest.tsv"
    ;;

consolidate)
    S="$(store)"
    # The manifest goes beside the shards, and every shard's hash is read
    # again there.
    on "$S" "cat > ~/fleet/$RUN/manifest.tsv" < "$D/manifest.tsv"
    on "$S" "cd ~/fleet/$RUN/shards && sha256sum *.bin" < /dev/null | awk '{f=$2; sub(/\.bin$/, "", f); print f "\t" $1}' | sort > "$D/store.sha"
    cut -f1,7 "$D/manifest.tsv" | sort > "$D/manifest.sha"
    if cmp -s "$D/store.sha" "$D/manifest.sha"; then
        awk -F'\t' '{s+=$5; b+=$6; n+=$4; h[$2]+=$4} END {printf "consolidated: %d shards, %d seeds, %d samples, %.1f GB; seeds by host:", NR, n, s, b/1e9; for (k in h) printf " %s %d", k, h[k]; print ""}' "$D/manifest.tsv"
    else
        diff "$D/manifest.sha" "$D/store.sha" | head
        die "the store's shards and the manifest differ"
    fi
    ;;

*) usage ;;
esac
