#!/usr/bin/env bash
# release.sh: the release procedure, run before a candidate is tagged
# and before anything goes to crates.io (documentation/RELEASE.md
# section 6). Every step prints what it checked; the first failure
# stops it. It changes nothing but build output.
#
#   1. the version: one in Cargo.toml, the path dependencies agreeing,
#      CHANGELOG.md with a section headed by it
#   2. make test, and make build with no warnings
#   3. the pictures: scripts/shoot.sh has run for this version
#      (reports/images/MANIFEST-<version>.txt), every frame it lists
#      exists, is not empty, and is shown by the player's guide
#      (documentation/HOW-TO-PLAY.md) or the site (index.html); and no
#      frame is older than the last commit that touched the window
#   4. the documents: README.md and index.html name the version's
#      guide; every image either names is a file
#   5. make publish DRY=1: the manifests package for crates.io
#   6. what is left to do by hand: the tag, the publish
. "$(dirname "$0")/common.sh"
need_config
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
IMG="$ROOT/reports/images"
GUIDE="$ROOT/documentation/HOW-TO-PLAY.md"
fail=0
ok()   { printf '  \033[32mok\033[0m    %s\n' "$*"; }
bad()  { printf '  \033[31mFAIL\033[0m  %s\n' "$*"; fail=1; }

say "release check for treadcore $VERSION"

say "1. the version"
deps="$(grep -c "version = \"$VERSION\"" "$ROOT/Cargo.toml")"
[ "$deps" -ge 5 ] && ok "Cargo.toml: workspace and the four path dependencies at $VERSION" || bad "Cargo.toml: the path dependencies do not all say $VERSION"
grep -q "^## $VERSION" "$ROOT/CHANGELOG.md" && ok "CHANGELOG.md has a section for $VERSION" || bad "CHANGELOG.md has no '## $VERSION' section"
[ -n "$(git -C "$ROOT" status --porcelain)" ] && warn "  the tree has uncommitted changes (fine for a dry run; the tag wants a commit)"

say "2. tests and a clean build"
if "$ROOT/scripts/build.sh" test >/dev/null 2>&1; then ok "make test"; else bad "make test (see $(latest build))"; fi
if "$ROOT/scripts/build.sh" release all >/dev/null 2>&1; then
    n="$(grep -c 'warning' "$(latest build)" || true)"
    [ "$n" = 0 ] && ok "make build, no warnings" || bad "make build: $n warning lines (see $(latest build))"
else
    bad "make build (see $(latest build))"
fi

say "3. the pictures"
MAN="$IMG/MANIFEST-$VERSION.txt"
if [ -f "$MAN" ]; then
    ok "frames shot for $VERSION ($MAN)"
    # The window's last change: a frame older than the newest source file
    # of the window shows an old window. (Not the last commit: the release
    # commit itself touches the window, and the check runs after it too.)
    since="$(stat -c %Y "$ROOT"/source/treadcore-client/src/*.rs | sort -n | tail -1)"
    while read -r f; do
        case "$f" in ''|'#'*) continue ;; esac
        if [ ! -s "$IMG/$f" ]; then bad "$f missing or empty"; continue; fi
        if [ "$(stat -c %Y "$IMG/$f")" -lt "$since" ]; then bad "$f is older than the window's newest source: shoot again"; continue; fi
        if grep -q "$f" "$GUIDE" "$ROOT/index.html" 2>/dev/null; then ok "$f shown"; else bad "$f is taken but no document shows it"; fi
    done < "$MAN"
else
    bad "no $MAN: run make shoot"
fi

say "4. the documents"
[ -f "$GUIDE" ] && ok "the player's guide exists" || bad "no documentation/HOW-TO-PLAY.md"
grep -q "HOW-TO-PLAY" "$ROOT/README.md" && ok "README.md names the guide" || bad "README.md does not name HOW-TO-PLAY.md"
grep -q "HOW-TO-PLAY" "$ROOT/index.html" && ok "index.html names the guide" || bad "index.html does not name HOW-TO-PLAY.md"
for doc in "$GUIDE" "$ROOT/index.html"; do
    for img in $(grep -o '[A-Za-z0-9_./-]*reports/images/[A-Za-z0-9_.-]*\.png' "$doc" | sed 's#.*reports/images/##' | sort -u); do
        [ -s "$IMG/$img" ] || bad "$(basename "$doc") shows $img, which is missing or empty"
    done
done
ok "every image the guide and the site show is a file"

say "5. crates.io"
if "$ROOT/scripts/publish.sh" dry >/dev/null 2>&1; then ok "make publish DRY=1"; else bad "make publish DRY=1"; fi

if [ "$fail" = 0 ]; then
    say "release check passed for $VERSION. By hand, in order (RELEASE.md section 6):"
    echo "   git commit -s ...                       the release commit"
    echo "   git tag -s v$VERSION -m '...'           the candidate's tag"
    echo "   ... a day ...                          live with the tagged build; a fix is a new candidate"
    echo "   make publish                           the five crates to crates.io, no sooner than a day after the tag"
    echo "   git push && git push --tags"
else
    die "release check failed for $VERSION"
fi
