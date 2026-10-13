#!/usr/bin/env bash
# publish.sh token|dry|all
#   token  store (or replace) the crates.io API token for cargo, from
#          TOKEN=... or typed at a prompt with echo off. It goes to
#          ~/.cargo/credentials.toml, cargo's own place, never into
#          this tree.
#   dry    package and verify the sim crate against crates.io without
#          uploading: the check that the manifests are publishable.
#   all    publish the five crates in dependency order (sim, proto,
#          net, brain, client); cargo waits for each to reach the index
#          before the next. Refuses a dirty tree unless ALLOW_DIRTY=1,
#          and a version that is already a tag unless it is.
. "$(dirname "$0")/common.sh"
MODE="${1:-dry}"
CREDS="${CARGO_HOME:-$HOME/.cargo}/credentials.toml"
CRATES=(treadcore-sim treadcore-proto treadcore-net treadcore-brain treadcore-client)
VERSION="$(awk -F'"' '/^version *= /{print $2; exit}' "$ROOT/Cargo.toml")"

case "$MODE" in
    token)
        if [ -z "${TOKEN:-}" ]; then
            printf 'crates.io API token (from https://crates.io/settings/tokens, echo off): '
            read -rs TOKEN; echo
        fi
        [ -n "$TOKEN" ] || die "no token given"
        # cargo login reads the token on stdin; it writes credentials.toml
        # with mode 600 itself.
        printf '%s\n' "$TOKEN" | cargo login >/dev/null
        say "token stored in $CREDS$( [ -f "$CREDS" ] && echo " (replaced)" )"
        ;;
    dry)
        say "dry run: packaging treadcore-sim $VERSION"
        cargo publish --dry-run --allow-dirty -p treadcore-sim
        say "ok: the manifests publish. The other four can only be verified once sim is on crates.io (each depends on the one before)."
        ;;
    all)
        [ -f "$CREDS" ] || die "no crates.io token: run 'make token' first"
        if [ -n "$(git -C "$ROOT" status --porcelain)" ] && [ "${ALLOW_DIRTY:-0}" != 1 ]; then
            die "the tree has uncommitted changes; commit them (RELEASE.md section 1) or ALLOW_DIRTY=1"
        fi
        # A day between the tag and the publish (RELEASE.md section 6):
        # the tagged tree is lived with for a day, and anything found in
        # it is a new candidate, not a version on crates.io.
        git -C "$ROOT" rev-parse -q --verify "refs/tags/v$VERSION" >/dev/null || die "no tag v$VERSION: tag the release, then publish a day later"
        tagged="$(git -C "$ROOT" for-each-ref --format='%(taggerdate:unix)%(committerdate:unix)' "refs/tags/v$VERSION" | cut -c1-10)"
        age=$(( $(date +%s) - tagged ))
        if [ "$age" -lt 86400 ]; then
            die "v$VERSION was tagged $((age / 3600)) h ago; publish no sooner than $(date -d "@$((tagged + 86400))" '+%a %d %b %H:%M' 2>/dev/null || echo 'a day after the tag')"
        fi
        say "publishing treadcore $VERSION: ${CRATES[*]}"
        for c in "${CRATES[@]}"; do
            say "== $c"
            cargo publish -p "$c" 2>&1 | tee "$LOGDIR/publish-$c-$STAMP.log" | grep -v '^\s*Compiling' || die "publish failed: see $LOGDIR/publish-$c-$STAMP.log"
        done
        say "published $VERSION; logs under $LOGDIR/publish-*-$STAMP.log"
        ;;
    *) die "usage: publish.sh token|dry|all" ;;
esac
