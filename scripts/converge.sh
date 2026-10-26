#!/usr/bin/env bash
# The run (LR 92, backlog 69): the census as a learner, day and night,
# until stopped. Each generation reads every census on this machine,
# fits each weight's curve to them, and plays the trials it suggests;
# a trial that reads past two errors is read twice more on seeds no
# step saw. Nothing is promoted: candidates wait for families and Paul.
#
#   make converge                    until stopped (pkill -f converge.sh)
#   UNTIL=06:30                      or until that hour
#   GEN=12 SEEDS=55 SEED0=2000000 HELD=400 MAP=all JOBS=24
#   SINCE=20261025-181300            the ledger's rows read against
#                                    today's defaults (their last change)
#   RESUME=build/converge/STAMP      go on with a run, its generations kept
#
# A generation: `scripts/ledger.py` reads build/census into a table;
# `scripts/suggest.py` fits it and writes the trials (the best point
# where a curve's top is past two errors, the best less one, reads
# again of trials past 1.5 errors on 55 seeds, each weight's half and
# double not yet read alone, draws from the fit's uncertainty); each is
# a census of SEEDS fresh seeds (generation g from SEED0 + g * 10000).
#
# Output under build/converge/STAMP: NOTES.txt (the rule, written before
# anything plays), defaults.toml, gen*/ (the trials, plan.tsv,
# curves.tsv), ledger.tsv, censuses.txt, candidates.txt, progress.txt.

. "$(dirname "$0")/common.sh"
UNTIL="${UNTIL:-}"
GEN="${GEN:-12}"
SEEDS="${SEEDS:-55}"
SEED0="${SEED0:-2000000}"
HELD="${HELD:-400}"
MAP="${MAP:-all}"
JOBS="${JOBS:-$(nproc 2>/dev/null || sysctl -n hw.ncpu)}"
SINCE="${SINCE:-20261025-181300}"
# RESUME is the run's, not its censuses': census.sh reads it too.
if [ -n "${RESUME:-}" ]; then OUT="$RESUME"; else OUT="$BUILDDIR/converge/$STAMP"; fi
unset RESUME
mkdir -p "$OUT"
log() { echo "$(date +%H:%M:%S) $*" | tee -a "$OUT/progress.txt"; }

# Seconds left before UNTIL (the next one, tomorrow's if today's is
# past); a large number when there is none.
END=0
if [ -n "$UNTIL" ]; then
    END=$(( $(date +%s) + $(python3 -I -c 'import datetime as d, sys
h, m = map(int, sys.argv[1].split(":"))
n = d.datetime.now(); t = n.replace(hour=h, minute=m, second=0, microsecond=0)
t += d.timedelta(days=1) if t <= n else d.timedelta(0)
print(int((t - n).total_seconds()))' "$UNTIL") ))
fi
remaining() { if [ "$END" -gt 0 ]; then echo $(( END - $(date +%s) )); else echo 999999999; fi; }

BIN="$(census_bin "$OUT")"
CENSUS_DUMP="$OUT/defaults.toml" CENSUS_SEEDS=0 "$BIN" --ignored brain_census >/dev/null 2>&1 || die "could not write the defaults"
touch "$OUT/censuses.txt" "$OUT/candidates.txt"

if [ ! -f "$OUT/NOTES.txt" ]; then
    cat >"$OUT/NOTES.txt" <<EOF
The run of $(date +%d\ %B\ %Y) (LR 92), on $(hostname) at
$(git -C "$ROOT" log --oneline -1). Written before it plays.

${UNTIL:+Until $UNTIL. }Each generation: ledger.py over build/census (rows since
$SINCE, against today's defaults); suggest.py, $GEN trials; a census
each of $SEEDS fresh seeds, generation g from $SEED0 + g * 10000, every
map ($MAP), 900 s, 3 respawns, $JOBS jobs, rewards on.

RULE. A trial whose census reads past two errors above the defaults
with no map past two errors below is read twice more, on $HELD seeds
each from 3000000 + 200000 k and 3100000 + 200000 k (k its turn
among the run's held-out pairs). It is a candidate for Paul only if
both reads are past two errors above with no map past two below, and
then make families holds. The run changes no default.

EXPECTED. The defaults sit at or near the top of each weight read so
far (LR 88, 89): few trials past two errors, most of them chance;
of those, few or none through both held-out reads.
EOF
fi

# The census's line and its directory, for a trial file and seeds.
census() { # census FILE SEEDS SEED0 -> prints the census dir
    TRIAL="$1" SEEDS="$2" SEED0="$3" MAP="$MAP" JOBS="$JOBS" "$ROOT/scripts/census.sh" >>"$OUT/census.log" 2>&1 || log "$(basename "$1"): census failed"
    ls -td "$BUILDDIR"/census/*/ | sed -n 1p  # sed reads it all: no SIGPIPE under pipefail
}
# Past two errors above, with no map past two below.
passes() { # passes DIR
    python3 -I -c 'import re, sys
ok, below = False, False
for l in open(sys.argv[1] + "/pooled.txt"):
    m = re.match(r"^(\S+(?: map)?)\s+([-+][\d.]+) \+/- ([\d.]+)", l)
    if not m:
        continue
    d, se = float(m.group(2)), float(m.group(3))
    if m.group(1) == "every map":
        ok = se > 0 and d > 2 * se
    elif se > 0 and d < -2 * se:
        below = True
sys.exit(0 if ok and not below else 1)' "$1" 2>/dev/null
}

held=$(wc -l <"$OUT/candidates.txt" | tr -d " ")
last=0
failed=0  # censuses failed in a row: three stop the run
g=$(find "$OUT" -maxdepth 1 -name "gen*" -type d | wc -l | tr -d " ")
log "run ${UNTIL:+until $UNTIL }from generation $g, $GEN trials of $SEEDS seeds -> $OUT"
while :; do
    python3 -I "$ROOT/scripts/ledger.py" "$BUILDDIR/census" >"$OUT/ledger.tsv"
    python3 -I "$ROOT/scripts/suggest.py" "$OUT/ledger.tsv" "$OUT/defaults.toml" "$OUT/gen$g" --n "$GEN" --seed "$g" --since "$SINCE" | tee -a "$OUT/progress.txt"
    ran=0
    for f in "$OUT/gen$g"/*.weights; do
        [ -e "$f" ] || continue
        name="gen$g/$(basename "$f" .weights)"
        if [ "$(remaining)" -lt "$last" ]; then break 2; fi
        t0=$(date +%s)
        d="$(census "$f" "$SEEDS" $(( SEED0 + g * 10000 )))"
        last=$(( $(date +%s) - t0 ))
        if [ ! -s "$d/pooled.txt" ] || [ "$(stat -c %Y "$d" 2>/dev/null || stat -f %m "$d")" -lt "$t0" ]; then
            failed=$((failed + 1))
            log "$name: no census read ($failed in a row)"
            [ "$failed" -lt 3 ] || { log "three censuses failed in a row: the run stops (see census.log)"; break 2; }
            continue
        fi
        failed=0
        echo "$name $d" >>"$OUT/censuses.txt"
        log "$name $(head -1 "$d/pooled.txt" 2>/dev/null)"
        ran=$((ran + 1))
        if passes "$d"; then
            reads=""
            for s0 in $(( 3000000 + 200000 * held )) $(( 3100000 + 200000 * held )); do
                h="$(census "$f" "$HELD" "$s0")"
                log "  held out from $s0: $(head -1 "$h/pooled.txt" 2>/dev/null)"
                if passes "$h"; then reads="$reads pass"; else reads="$reads fail"; fi
            done
            held=$((held + 1))
            verdict=$([ "$reads" = " pass pass" ] && echo "CANDIDATE: families, then Paul" || echo "not a candidate")
            echo "$name$reads: $verdict ($f)" >>"$OUT/candidates.txt"
            log "  $name:$reads, $verdict"
        fi
    done
    [ "$ran" -gt 0 ] || { log "suggest.py had nothing to read"; break; }
    g=$((g + 1))
done
log "run stopped at generation $g"
