#!/bin/bash
# Filling the card (LAB-REPORT-BRAIN.md section 108, B.1): the trainer at
# each batch size on one recording kept on the device, with nvidia-smi
# logging the card's busy share and memory every second. Run on the GPU
# node:
#
#   tools/bc/sweep.sh RECORDING OUTDIR "32768 65536 ..." [EPOCHS=4] [GPU_GB=9]
#
# A line a batch in OUTDIR/sweep.tsv: batch, settled epoch (s), its standard
# deviation, samples a second, the card's mean busy share and its peak
# memory (MB) over the settled epochs, and the held-out drive agreement.
set -euo pipefail
REC="$1" OUT="$2" BATCHES="$3" EPOCHS="${4:-4}"
BIN="$(cd "$(dirname "$0")" && pwd)/target/release/treadcore-bc"
mkdir -p "$OUT"
printf 'batch\tepoch_s\tsd\tsamples_s\tbusy_pct\tmem_mb\tdrive\n' > "$OUT/sweep.tsv"
for b in $BATCHES; do
    nvidia-smi dmon -s um -d 1 -o T > "$OUT/dmon-$b.log" 2>/dev/null &
    mon=$!
    start=$(date +%s)
    # The rate is scaled with the batch (1e-3 at 4,096, as the square root),
    # so a large batch still learns something in a few epochs; the time does
    # not depend on it.
    lr=$(python3 -c "print(f'{1e-3 * ($b / 4096) ** 0.5:.5f}')")
    if ! GPU_GB="${GPU_GB:-9}" BC_DEVICE=cuda "$BIN" "$REC" "$OUT/net-$b.bin" "$EPOCHS" "$b" "$lr" > "$OUT/run-$b.log" 2>&1; then
        echo "batch $b: the trainer failed (out of memory?): $(grep -m1 -iE 'panicked|error|memory' "$OUT/run-$b.log" | cut -c1-160)"
    fi
    kill "$mon" 2>/dev/null || true
    wait "$mon" 2>/dev/null || true
    # The settled epochs are the run's tail: from the end of epoch 1 on.
    first=$(grep -m1 'epoch  1:' "$OUT/run-$b.log" | sed 's/.*(\([0-9.]*\) s;.*/\1/' || true)
    python3 - "$OUT" "$b" "$start" "${first:-0}" <<'EOF' >> "$OUT/sweep.tsv"
import re, sys
out, b, start, first = sys.argv[1], sys.argv[2], int(sys.argv[3]), float(sys.argv[4])
log = open(f"{out}/run-{b}.log").read()
m = re.search(r"at \d+: ([\d.]+) s \+/- ([\d.]+) an epoch \([\d.]+ ms a step, (\d+) samples/s", log)
d = re.search(r"^  drive\s+([\d.]+)%", log, re.M)
# dmon lines: time gpu sm mem ... fb; take those after the first epoch ended
# (the read and the first epoch excluded by skipping that many seconds plus
# the read's).
r = re.search(r"read in ([\d.]+) s", log)
skip = first + (float(r.group(1)) if r else 0) + 3
cols, busy, mem, t0 = None, [], [], None
for line in open(f"{out}/dmon-{b}.log"):
    if line.startswith("#"):
        if cols is None:
            cols = line[1:].split()
        continue
    f = line.split()
    if not cols or len(f) < len(cols):
        continue
    t0 = t0 or 0
    t0 += 1
    if t0 < skip:
        continue
    try:
        busy.append(float(f[cols.index("sm")]))
        mem.append(float(f[cols.index("fb")]))
    except ValueError:
        pass
if m:
    print(f"{b}\t{m.group(1)}\t{m.group(2)}\t{m.group(3)}\t{sum(busy)/len(busy) if busy else float('nan'):.0f}\t{max(mem) if mem else float('nan'):.0f}\t{d.group(1) if d else ''}")
else:
    print(f"{b}\t\t\t\t\t{max(mem) if mem else ''}\tfailed")
EOF
    tail -1 "$OUT/sweep.tsv"
done
