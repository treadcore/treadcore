#!/usr/bin/env bash
# deps.sh: the system packages the window needs to build and run, for
# this machine's distribution. Prints the command; APPLY=1 runs it (as
# root, through doas or sudo). The headless crates need none of this,
# only Rust.
#
#   to build the window   a C compiler and pkg-config; wayland, xkbcommon,
#                         ALSA and libudev headers
#   to run it             a Vulkan loader and a driver: the GPU's, or on a
#                         machine without one, lavapipe (Vulkan on the CPU)
#
# Package names were checked against each distribution's package index;
# the Alpine and Copal lines are the ones this project builds on every
# day, the rest have been run by people, not by the scripts' author.
. "$(dirname "$0")/common.sh"
. "$(dirname "$0")/platform.sh"

case "$PLATFORM" in
    copal|alpine)
        CMD="apk add build-base pkgconf wayland-dev libxkbcommon-dev alsa-lib-dev eudev-dev vulkan-loader mesa-vulkan-swrast" ;;
    arch|omarchy)
        # vulkan-swrast is lavapipe; a real GPU wants vulkan-radeon,
        # vulkan-intel or nvidia-utils as well, and Omarchy has its own.
        CMD="pacman -S --needed base-devel pkgconf wayland libxkbcommon alsa-lib systemd-libs vulkan-icd-loader vulkan-swrast" ;;
    debian|ubuntu|mint)
        # mesa-vulkan-drivers carries lavapipe beside the radeon and intel drivers.
        CMD="apt-get install -y build-essential pkg-config libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev libvulkan1 mesa-vulkan-drivers" ;;
    gentoo)
        # Mesa's Vulkan drivers follow USE=vulkan and VIDEO_CARDS; lavapipe
        # is VIDEO_CARDS="lavapipe" (or add it to what is there).
        CMD="emerge --ask --noreplace dev-libs/wayland x11-libs/libxkbcommon media-libs/alsa-lib virtual/libudev media-libs/vulkan-loader media-libs/mesa" ;;
    fedora)
        CMD="dnf install -y gcc pkgconf-pkg-config wayland-devel libxkbcommon-devel alsa-lib-devel systemd-devel vulkan-loader mesa-vulkan-drivers" ;;
    macos)
        say "$PLATFORM_NAME: nothing to install. The window uses Metal and Cocoa, which come with the system; Rust from https://rustup.rs."
        exit 0 ;;
    windows)
        say "$PLATFORM_NAME: no system packages. The window uses DirectX 12 or Vulkan from the GPU's driver."
        echo "  Rust from https://rustup.rs, which asks for the Visual Studio C++ build tools (the MSVC toolchain) and installs them."
        echo "  Or the ready-made zip from https://treadcore.org/download.html"
        exit 0 ;;
    nixos)
        say "$PLATFORM_NAME: the flake brings everything, Rust included."
        echo "  nix run github:treadcore/treadcore          # play"
        echo "  nix develop                                 # a shell with the build's packages, in this checkout"
        exit 0 ;;
    *)
        warn "$PLATFORM_NAME: not one this script knows. The window needs, by any name: a C compiler, pkg-config, the wayland-client, xkbcommon, alsa and libudev development files, a Vulkan loader and a Vulkan driver."
        exit 1 ;;
esac

say "$PLATFORM_NAME"
echo "  $CMD"
if [ "$PLATFORM" = gentoo ]; then
    echo "  and for a machine with no GPU, VIDEO_CARDS=\"lavapipe\" with USE=vulkan on media-libs/mesa"
fi
if ! command -v cargo >/dev/null; then
    echo "  Rust: https://rustup.rs (the distribution's own rust package works too if it is $(awk -F'"' '/^rust-version *= /{print $2; exit}' "$ROOT/Cargo.toml") or newer)"
fi
if [ "${APPLY:-0}" = 1 ]; then
    [ -n "$SUDO" ] || [ "$(id -u)" = 0 ] || die "neither doas nor sudo, and not root: run the line above as root"
    say "running it"
    # shellcheck disable=SC2086
    $SUDO $CMD
else
    echo "  (APPLY=1 make deps runs it)"
fi
