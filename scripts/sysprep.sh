#!/usr/bin/env bash
# sysprep.sh [--apply] -- the root-level memory settings for building the
# engine on this VM. Without --apply it only says what it would do.
#
# Measured on the 6 GB Alpine guest, 12 Oct 2026:
#   zram0: 3.1 GB device, lz4, holding 943 MB compressed to 620 MB of RAM
#          (1.5x). zstd typically gives 2.5-3x on this kind of data for a
#          little more CPU: the same 943 MB would cost ~350 MB.
#   disk:  ext4 root with 26 GB free. No disk swap at all, so when zram
#          fills (it did, with rust-analyzer resident) the next allocation
#          has nowhere to go but the OOM killer.
#
# Two changes, both reversible:
#   1. A 4 GB swapfile on the root disk at priority 10. zram stays at
#      priority 100, so it fills first (fast); the file is overflow only.
#      It costs 4 GB of disk and nothing else until it is needed.
#   2. zram compression lz4 -> zstd, by editing /etc/local.d/zram.start.
#      Takes effect at next boot (the device cannot be changed while it
#      holds pages). NOTE: that file is copal's; a copal redeploy may put
#      it back. Re-run this script after one.
#
# Not done here, because they are the host's decision: give the VM more
# memory in UTM (8 GB would end this whole class of problem), or more
# CPUs. The guest cannot change either.
set -euo pipefail
APPLY=0
[ "${1:-}" = "--apply" ] && APPLY=1
SWAPFILE=/swapfile
SWAPSIZE_MB=4096
ZRAM_START=/etc/local.d/zram.start

say() { printf '\033[1m%s\033[0m\n' "$*"; }
run() {
    if [ "$APPLY" = 1 ]; then
        echo "  + $*"; "$@"
    else
        echo "  would: $*"
    fi
}

if [ "$APPLY" = 1 ] && [ "$(id -u)" != 0 ]; then
    echo "sysprep: --apply needs root: doas $0 --apply" >&2
    exit 1
fi

say "1. disk swapfile, overflow behind zram"
if grep -q "^$SWAPFILE" /proc/swaps; then
    echo "  already active: $(grep "^$SWAPFILE" /proc/swaps)"
else
    run fallocate -l "${SWAPSIZE_MB}M" "$SWAPFILE"
    run chmod 600 "$SWAPFILE"
    run mkswap "$SWAPFILE"
    run swapon -p 10 "$SWAPFILE"
    if ! grep -q "^$SWAPFILE" /etc/fstab 2>/dev/null; then
        if [ "$APPLY" = 1 ]; then
            echo "$SWAPFILE none swap sw,pri=10 0 0" >> /etc/fstab
            echo "  + fstab line added"
        else
            echo "  would: append '$SWAPFILE none swap sw,pri=10 0 0' to /etc/fstab"
        fi
    fi
fi

say "2. zram compression zstd (at next boot)"
if [ -f "$ZRAM_START" ]; then
    if grep -q "echo zstd" "$ZRAM_START"; then
        echo "  already zstd in $ZRAM_START"
    else
        if [ "$APPLY" = 1 ]; then
            sed -i 's/^echo lz4 > /echo zstd > /' "$ZRAM_START"
            echo "  + $ZRAM_START: lz4 -> zstd (reboot to take effect)"
        else
            echo "  would: sed -i 's/^echo lz4 > /echo zstd > /' $ZRAM_START"
        fi
    fi
    echo "  current algorithm: $(cat /sys/block/zram0/comp_algorithm 2>/dev/null)"
else
    echo "  $ZRAM_START not found; zram is set up some other way here"
fi

say "3. the lever this script cannot pull"
echo "  UTM > this VM > System > Memory: 6144 -> 8192 MB. Needs the host."
[ "$APPLY" = 1 ] || echo; [ "$APPLY" = 1 ] || say "nothing changed. apply with: doas $0 --apply"
