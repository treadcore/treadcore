# Deploy procedure

How a version of treadcore goes from a finished change to something
people can install: crates on crates.io, packages on a GitHub release,
and treadcore.org showing them. In order, with the command for each
step, what it checks, and what to do when it fails.

Why and when to release, and how candidates are confirmed by
playtesters, are in `RELEASE.md`. How the packages are made is in
`LAB-REPORT-PACKAGES.md`. This document is only the steps.

## The two days

| day | steps | can be undone? |
| --- | --- | --- |
| **Day 0: tag** | 1 to 7: prepare, check, commit, tag, push, watch the build | yes: nothing is public, a fix is the next candidate |
| **Day 1 or later: publish** | 8 to 12: inspect, publish the release, publish the crates, update the site, announce | **no**: a crate can be yanked, never removed |

Publishing waits **at least one day** after the tag. The tagged build
is lived with for that day; anything found in it makes a new
candidate, not a published version. `make publish` refuses a tag
younger than 24 hours.

## Once per machine

```sh
make configure      # toolchain, display libraries, build/config.env
make deps           # the window's system packages, for this system (APPLY=1 installs)
make hooks          # the pre-push hook: a v* tag push runs the release check first
make token          # the crates.io API token, into ~/.cargo/credentials.toml
```

The token never goes in the repository. `make hooks` must be run in
every checkout: git does not clone hooks.

## Day 0: tag

### 1. Set the version

One version for everything, in `Cargo.toml`: the workspace's and the
four internal path dependencies'.

```sh
sed -i 's/version = "0.1.0-rc.6"/version = "0.1.0-rc.7"/g' Cargo.toml
cargo update -w --offline
grep -c '0.1.0-rc.7' Cargo.toml          # 5
```

A candidate is `X.Y.Z-rc.N`; a release is `X.Y.Z` (`RELEASE.md`
section 2). A number is never reused: a tag that went wrong is
followed by the next number, not moved.

### 2. Write down what changed

- `CHANGELOG.md`: a section headed `## <version> (<date>): ...`. The
  release check looks for it.
- `README.md`: the "current release" line.
- `index.html`: the version under the screenshot, and
  `softwareVersion` in its JSON-LD block.
- A journal entry for the session (`journal/`, `TEMPLATE.txt`).

### 3. Take the pictures

```sh
make shoot
```

Drives the window through one scripted match and saves fifteen frames,
cropped, to `reports/images/guide-*.png`, listed in
`reports/images/MANIFEST-<version>.txt`. It captures the game window
itself (`grim -T`), whatever else is on screen, and stops if a frame is
not the game. Needs this machine's Hyprland, Xwayland and grim
(`LAB-REPORT-PACKAGES.md` section VII).

If a new feature has no frame, add one to `scripts/shoot.sh` and show
it in `documentation/HOW-TO-PLAY.md` first: a feature is not done until
the guide shows it.

### 4. Run the release check

```sh
make release
```

| it checks | if it fails |
| --- | --- |
| the version agrees in `Cargo.toml` and has a `CHANGELOG.md` section | step 1 or 2 |
| `make test` passes | fix the code |
| `make build` prints no warnings | fix the warning |
| every frame in this version's manifest exists, is not empty, is newer than the window's source, and is shown by the guide or the site | step 3; or show the frame |
| the guide exists and the README and site name it; every image they show is a file | the document it names |
| `cargo publish --dry-run` packages the first crate | the crate manifests |

It changes nothing but build output. Read its last lines: it ends
with "release check passed".

### 5. Commit and tag

```sh
git add -A
git commit -s -m "release: <version>: <what it is>"
git status --porcelain                      # must print nothing
git tag -s v<version> -m "treadcore <version>: ..."
[ "$(git rev-parse v<version>^{commit})" = "$(git rev-parse HEAD)" ] && echo "tag on HEAD"
```

Check the commit happened before tagging. A tag made after a commit
that silently did not happen names the previous version's code; that
is how `v0.1.0-rc.4` came to name rc.3 (journal 0021).

### 6. Push

```sh
git push origin main
git push origin v<version>
```

The tag push runs the hook (`.githooks/pre-push`). It refuses the push
if the tag is not the version in `Cargo.toml`, if the tag is not the
checked-out commit, if the working tree has uncommitted changes, or if
`make release` fails. `git push --no-verify` skips it; do not.

### 7. Watch the build

The tag starts `.github/workflows/packages.yml` on GitHub:

| job | does |
| --- | --- |
| version | refuses a tag that is not the commit's version |
| deb, rpm, windows, macos | build each package on its own system, x86-64 and ARM |
| test-deb, test-rpm, test-windows, test-macos | install each package on fresh Debian 12 and 13, Ubuntu 22.04 and 24.04, Linux Mint 22, Fedora 41 and 42, Windows and macOS; run the window's `--help`, check its libraries, play a 60-second bot match |
| nix | build the flake and play a match (not a gate) |
| release | only if every build and test passed: a **draft** release with all packages and `SHA256SUMS` |

A run takes about twenty minutes. Watch it at
https://github.com/treadcore/treadcore/actions, or from a terminal
without spending the API: one request every few minutes to

```sh
curl -s https://api.github.com/repos/treadcore/treadcore/commits/v<version>/check-runs
```

(GitHub allows 60 unauthenticated requests an hour per address.)

**If a job fails**, its reason is published as an annotation on the
job, readable without a login: on the run's page, or

```sh
curl -s https://api.github.com/repos/treadcore/treadcore/check-runs/<job id>/annotations
```

Fix it, and go back to step 1 with the **next** candidate number.
There is no draft, so nothing needs undoing. Do not delete or move the
failed tag; the changelog says what happened to it.

**When every job passes**, there is a draft release at
https://github.com/treadcore/treadcore/releases, visible only to
maintainers. Day 0 ends here.

## Day 1 or later: publish

At least 24 hours after the tag (`make publish` prints the earliest
time if asked too soon).

### 8. Inspect the draft

On the draft release page:

- every file is there: two `.deb`, two `.rpm`, the `.msi`, the Windows
  zip, the macOS zip, `SHA256SUMS`;
- the title and the pre-release mark are right (a candidate is a
  pre-release);
- anything found during the day? Then do not publish: fix it and go to
  step 1 with the next candidate.

### 9. Publish the release

On the draft: edit, add the notes (the CHANGELOG section, and for a
candidate the call for playtest's text), and **Publish release**.
This is the moment the packages become downloadable.

### 10. Publish the crates

```sh
make publish DRY=1      # package and verify, uploads nothing
make publish            # the five crates, in dependency order
```

`treadcore-sim`, `-proto`, `-net`, `-brain`, `-client`: crates.io
accepts a crate only when its dependencies are already in its index,
and cargo waits for each. It refuses a dirty tree, a missing tag, or a
tag younger than a day. Logs go to `build/logs/publish-*.log`. A crate
version cannot be removed afterwards, only yanked.

### 11. Update the site

```sh
make builds             # three API calls at most
git add builds.json index.html download.html
git commit -s -m "site: builds for <version>"
git push origin main
```

`make builds` asks GitHub and crates.io once what is published, and
writes it into `builds.json`, the build grid on `index.html` and the
download links on `download.html`. The pages never call an API
themselves. GitHub Pages republishes treadcore.org a minute after the
push. Check https://treadcore.org/download.html: the buttons point at
this release, and the grid's boxes are ticked.

### 12. Announce

For a candidate: the call for playtest in Discussions, Playtests
(`reports/calls/`, with the version updated; `RELEASE.md` section 4).
For a release: the release ceremony (`GOVERNANCE.md` section 10).
Either way, a journal entry saying it was published.

## Quick reference

```sh
# day 0
sed -i 's/version = "OLD"/version = "NEW"/g' Cargo.toml && cargo update -w --offline
$EDITOR CHANGELOG.md README.md index.html       # the version, what changed
make shoot
make release
git add -A && git commit -s -m "release: NEW" && test -z "$(git status --porcelain)"
git tag -s vNEW -m "treadcore NEW"
git push origin main && git push origin vNEW
# watch https://github.com/treadcore/treadcore/actions; a failure means NEW+1

# day 1 or later
# inspect and publish the draft on https://github.com/treadcore/treadcore/releases
make publish DRY=1 && make publish
make builds && git add builds.json index.html download.html && git commit -s -m "site: builds for NEW" && git push origin main
# announce
```

## Where things are

| what | where |
| --- | --- |
| this procedure | `documentation/DEPLOY.md` |
| why, versions, candidates, playtests | `documentation/RELEASE.md` |
| how the packages are made, with code | `documentation/LAB-REPORT-PACKAGES.md` |
| the release check | `scripts/release.sh` (`make release`) |
| the pictures | `scripts/shoot.sh` (`make shoot`) |
| the hook | `.githooks/pre-push` (`make hooks`) |
| the package build and tests | `.github/workflows/packages.yml` |
| crates.io | `scripts/publish.sh` (`make publish`, `make token`) |
| the site's build data | `scripts/builds.sh` (`make builds`), `builds.json` |
| the packages' runtime dependencies | `source/treadcore-client/Cargo.toml`, `flake.nix`, `packaging/` |
