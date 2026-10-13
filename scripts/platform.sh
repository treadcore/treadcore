# shellcheck shell=bash
# platform.sh: which system this is, for deps.sh and install.sh. Sourced.
# Sets PLATFORM (copal, alpine, omarchy, arch, debian, ubuntu, gentoo,
# fedora, macos, or unknown), PLATFORM_NAME (for messages) and SUDO
# (doas or sudo, or nothing when already root).
PLATFORM=unknown
PLATFORM_NAME="$(uname -s)"
if [ "$(uname -s)" = Darwin ]; then
    PLATFORM=macos; PLATFORM_NAME="macOS $(sw_vers -productVersion 2>/dev/null)"
elif [ -r /etc/os-release ]; then
    # shellcheck disable=SC1091
    . /etc/os-release
    PLATFORM_NAME="${PRETTY_NAME:-$ID}"
    # ID first, then ID_LIKE, so a derivative (Manjaro, EndeavourOS, Mint,
    # Pop!_OS) lands on its parent's packages.
    for id in ${ID:-} ${ID_LIKE:-}; do
        case "$id" in
            alpine) PLATFORM=alpine ;;
            arch) PLATFORM=arch ;;
            ubuntu) PLATFORM=ubuntu ;;
            debian) PLATFORM=debian ;;
            gentoo) PLATFORM=gentoo ;;
            fedora|rhel) PLATFORM=fedora ;;
            *) continue ;;
        esac
        break
    done
    # Copal is Alpine and says so in os-release; its own mark is /etc/copal.
    [ "$PLATFORM" = alpine ] && [ -d /etc/copal ] && { PLATFORM=copal; PLATFORM_NAME="Copal Linux ($PLATFORM_NAME)"; }
    # Omarchy is Arch with Hyprland and its own tree in the user's home.
    [ "$PLATFORM" = arch ] && [ -d "$HOME/.local/share/omarchy/bin" ] && { PLATFORM=omarchy; PLATFORM_NAME="Omarchy ($PLATFORM_NAME)"; }
fi
SUDO=
if [ "$(id -u)" != 0 ]; then
    if command -v doas >/dev/null; then SUDO=doas; elif command -v sudo >/dev/null; then SUDO=sudo; fi
fi
