#!/usr/bin/env bash
# buildmode.sh on|off|status -- make the desktop cheap while the engine builds.
#
# What it does not do: save RAM. Hyprland is ~50 MB and nothing here
# changes that. What it does: on a VM with no GPU (virtio, llvmpipe), blur,
# shadows and animations are drawn by the CPU, the same CPU that is
# running three rustc. Turning them off during a build gives the compiler
# the cores back. Every change is a runtime `hyprctl keyword`: nothing is
# written to the config, and `off` (or a Hyprland restart) restores it.
#
# Needs no root. Safe to run at any time.
set -euo pipefail
case "${1:-status}" in
    on)
        hyprctl --batch "keyword decoration:blur:enabled 0; keyword decoration:shadow:enabled 0; keyword animations:enabled 0; keyword misc:vfr 1" >/dev/null
        echo "buildmode on: blur, shadows, animations off; variable frame rate on"
        ;;
    off)
        hyprctl reload >/dev/null
        echo "buildmode off: hyprland.conf reloaded, effects back"
        ;;
    status)
        for k in decoration:blur:enabled decoration:shadow:enabled animations:enabled misc:vfr; do
            printf '  %-28s %s\n' "$k" "$(hyprctl getoption "$k" | grep -oE 'int: [0-9]+' | cut -d' ' -f2)"
        done
        ;;
    *) echo "usage: buildmode.sh on|off|status" >&2; exit 2 ;;
esac
