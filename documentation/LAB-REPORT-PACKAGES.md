# Lab report: packages for every platform, from a git hook to a release

*A Cargo and Rust tutorial, written from treadcore's own release
pipeline. 13 October 2026, versions 0.1.0-rc.2 and rc.3.*

## Abstract

A Rust game built with Bevy should reach its players the way each of
their systems expects: a `.deb` for Debian, Ubuntu and Linux Mint, an
`.rpm` for Fedora, an `.msi` for Windows, an app for macOS, a flake for
Nix, `make install` where building from source is the custom, and
`cargo install` everywhere. This report builds that, step by step, as it
was built for treadcore, with the code that does each step. The chain
is: a **git hook** that refuses to push a release tag until the release
check passes; the **release check**, which holds the documentation to
the build (the pictures must be of this version, and every one must be
shown); a **GitHub Actions workflow** that builds every package on its
own platform, **installs each one on the systems it is for and plays a
match there**, and only then puts them on a draft release; a **one-day
limit** between the tag and anything public; and **API discipline**, so
that neither the pipeline nor the website spends rate limits that are
not ours. The first run found a broken `.rpm` before anyone downloaded
it; the gate held, and the next candidate fixed it.

## I. What you need

| piece | what it is | where in treadcore |
| --- | --- | --- |
| a Cargo workspace | the game's crates, one version | `Cargo.toml` |
| a Bevy dependency block | the engine, with only the features used | `Cargo.toml`, `[workspace.dependencies.bevy]` |
| package metadata | what cargo-deb and cargo-generate-rpm need | `source/treadcore-client/Cargo.toml` |
| a WiX source | the Windows installer | `packaging/windows/treadcore.wxs` |
| a desktop entry and icon | shared by every Linux route | `packaging/` |
| a release check | the procedure as a script | `scripts/release.sh`, `make release` |
| a git hook | runs the check before a tag leaves | `.githooks/pre-push`, `make hooks` |
| a workflow | builds, tests, drafts | `.github/workflows/packages.yml` |
| a publish script | crates.io, a day after the tag | `scripts/publish.sh`, `make publish` |
| a build table | what is published, for the site | `scripts/builds.sh`, `make builds` |

Rust from rustup or the distribution, either: nothing here pins a
toolchain beyond `rust-version` in the workspace.

## II. The Bevy template

Bevy's default features pull in a 3D renderer, audio, glTF, animation
and more. A 2D game that turns them off builds faster, links smaller
and has fewer system dependencies to package. Define Bevy **once**, in
the workspace, so every crate that uses it resolves to the same
features and the same artifacts:

```toml
# Cargo.toml (workspace root)
[workspace]
resolver = "2"
members = ["source/treadcore-sim", "source/treadcore-client", "..."]

[workspace.package]
version = "0.1.0-rc.3"          # the one version: crates, packages, tags
edition = "2021"
rust-version = "1.85"           # what the code needs; any newer toolchain builds it
license = "MIT"
repository = "https://github.com/treadcore/treadcore"
homepage = "https://treadcore.org"

[workspace.dependencies]
# Internal crates with a version as well as a path: crates.io refuses a
# package whose dependency has only a path.
treadcore-sim = { path = "source/treadcore-sim", version = "0.1.0-rc.3" }

[workspace.dependencies.bevy]
version = "0.17"
default-features = false
features = [
    "std", "multi_threaded", "bevy_log", "bevy_asset", "bevy_color", "bevy_state",
    "bevy_window", "bevy_winit", "wayland", "x11", "bevy_input_focus",
    "bevy_render", "bevy_core_pipeline", "bevy_sprite", "bevy_sprite_render",
    "bevy_text", "default_font", "bevy_ui", "bevy_ui_render", "bevy_gizmos",
]
```

A crate takes it with `bevy = { workspace = true }`. Two things about
this list matter for packaging:

- **`wayland` and `x11` are both on.** The window runs native on
  Wayland and under X11 alike. On Windows and macOS these two features
  compile to nothing, so the same list builds everywhere: the Windows
  and macOS jobs of the workflow use it unchanged.
- **No `bevy_audio`.** So no ALSA library is linked into the window,
  and no package has to require one. (The build still wants ALSA's
  headers on Linux, through a dependency's build script; the packages
  do not.)

Profiles sized for the machine the game is developed on, and one for
release binaries:

```toml
[profile.release]
opt-level = 3
lto = false
codegen-units = 16      # the link stays under a gigabyte on a 6 GB machine

[profile.dist]          # for a binary to hand to someone, when time allows
inherits = "release"
lto = "thin"
codegen-units = 1
```

The window's `name` is the Wayland app id and the X11 window class. It
must match the desktop entry's file name, or launchers and taskbars
cannot give the window its name and icon:

```rust
// source/treadcore-client/src/main.rs
let mut window = Window {
    title: "treadcore".into(),
    name: Some("org.treadcore.treadcore".into()),   // = org.treadcore.treadcore.desktop
    present_mode: PresentMode::AutoVsync,
    ..default()
};
```

## III. The multi-target build

**Native runners, not cross-compilation.** A Bevy window links system
libraries (Wayland, xkbcommon, the GPU stack) whose headers and
`.so` files must be the target's. Cross-compiling that is possible and
fragile; GitHub gives free native runners for x86-64 and ARM64 Linux,
Windows and Apple-silicon macOS, so each package is built where it
runs:

| target triple | runner | packages |
| --- | --- | --- |
| `x86_64-unknown-linux-gnu` | `ubuntu-22.04` | `.deb` amd64 |
| `aarch64-unknown-linux-gnu` | `ubuntu-22.04-arm` | `.deb` arm64 |
| `x86_64-unknown-linux-gnu` | `fedora:41` container | `.rpm` x86_64 |
| `aarch64-unknown-linux-gnu` | `fedora:41` on `ubuntu-22.04-arm` | `.rpm` aarch64 |
| `x86_64-pc-windows-msvc` | `windows-latest` | `.msi`, zip |
| `aarch64-apple-darwin` | `macos-14` | zip of `Treadcore.app` |
| `x86_64-linux`, `aarch64-linux` (Nix) | `ubuntu-latest` + Nix | the flake |
| `aarch64-unknown-linux-musl` | Copal (by hand) | `make install` |

**Build where the glibc is oldest you want to support.** A binary
linked on Ubuntu 22.04 needs glibc 2.35 or newer, so one `.deb` serves
Debian 12, Ubuntu 22.04 and 24.04 and Mint 21 and 22. Built on 24.04 it
would refuse Debian 12. The `.rpm` is built in a Fedora container for
the same reason, the other way round: Fedora's own libraries and its own
dependency generator.

The build job, for one row of the table:

```yaml
deb:
  strategy:
    matrix:
      include:
        - { runner: ubuntu-22.04, arch: amd64 }
        - { runner: ubuntu-22.04-arm, arch: arm64 }
  runs-on: ${{ matrix.runner }}
  steps:
    - uses: actions/checkout@v4
    - name: system packages (scripts/deps.sh, Debian line)
      run: sudo apt-get update && sudo apt-get install -y build-essential pkg-config
           libwayland-dev libxkbcommon-dev libasound2-dev libudev-dev libvulkan1 mesa-vulkan-drivers
    - uses: dtolnay/rust-toolchain@stable
    - run: cargo build --release --locked -p treadcore-client -p treadcore-server
    - run: cargo install --locked cargo-deb
    - run: cargo deb -p treadcore-client --no-build -o "treadcore-$VERSION-linux-${{ matrix.arch }}.deb"
```

`--locked` builds what `Cargo.lock` says, so the package is the code
that was tested. The system packages are the same list `make deps`
prints for a user on that distribution, from `scripts/deps.sh`: one
source for "what does this need", used by people and by CI alike.

## IV. Packages that bring what they need

A package must install everything the program needs at run time, and
nothing it does not. Rust itself is **never** a dependency of a binary
package; users who build from source bring their own toolchain, from
rustup or their distribution.

Two kinds of runtime dependency, handled two ways:

- **Linked** libraries appear in the binary's `NEEDED` list. Let the
  tools read them: cargo-deb's `$auto` runs `dpkg-shlibdeps`, and
  cargo-generate-rpm's `auto-req` runs RPM's own generator.
- **Loaded at run time** (`dlopen`) libraries are invisible to those
  tools. winit loads Wayland and xkbcommon this way, and wgpu loads the
  Vulkan loader, which loads a driver. Name them by hand.

```toml
# source/treadcore-client/Cargo.toml
[package.metadata.deb]
name = "treadcore"
section = "games"
depends = "$auto, libwayland-client0, libwayland-cursor0, libxkbcommon0, libvulkan1, mesa-vulkan-drivers"
recommends = "libx11-6, libx11-xcb1, libxcursor1, libxi6, libxrandr2, libxkbcommon-x11-0"
assets = [
    ["target/release/treadcore", "usr/bin/", "755"],
    ["target/release/treadcore-server", "usr/bin/", "755"],
    ["../../packaging/org.treadcore.treadcore.desktop", "usr/share/applications/", "644"],
    ["../../packaging/treadcore.svg", "usr/share/icons/hicolor/scalable/apps/", "644"],
]

[package.metadata.generate-rpm]
name = "treadcore"
auto-req = "auto"     # RPM's own generator where rpm-build is installed
assets = [
    { source = "target/release/treadcore", dest = "/usr/bin/treadcore", mode = "755" },
    # ...
]
[package.metadata.generate-rpm.requires]
libwayland-client = "*"
libxkbcommon = "*"
vulkan-loader = "*"
mesa-vulkan-drivers = "*"
```

`mesa-vulkan-drivers` carries **lavapipe**, Vulkan on the CPU, so a
machine with no GPU still opens the window. X11 libraries are
*recommended*, not required: they matter only in an X11 session.

**Version strings differ by format.** Cargo's `0.1.0-rc.3` is a
pre-release; Debian spells that `0.1.0~rc.3` (cargo-deb converts it),
and RPM refuses a `-` in a version outright:

```sh
cargo generate-rpm -p source/treadcore-client \
    --set-metadata 'version = "0.1.0~rc.3"'      # ~ sorts before the release
```

An MSI version is three numbers, so a candidate installs as its
release's number and `AllowSameVersionUpgrades` lets the next candidate
replace it:

```xml
<!-- packaging/windows/treadcore.wxs, built with WiX 5 -->
<Package Name="Treadcore" Manufacturer="Paul Richeson" Version="$(var.Version)"
         UpgradeCode="e85be08f-f494-5999-bac3-8b89ec9ef3cb" Scope="perMachine">
  <MajorUpgrade AllowSameVersionUpgrades="yes" DowngradeErrorMessage="A newer Treadcore is already installed." />
  <StandardDirectory Id="ProgramFiles64Folder"><Directory Id="INSTALLFOLDER" Name="Treadcore" /></StandardDirectory>
  <StandardDirectory Id="ProgramMenuFolder" />
  <Feature Id="Main">
    <Component Directory="INSTALLFOLDER">
      <File Id="treadcore.exe" Source="$(var.BinDir)\treadcore.exe" KeyPath="yes" />
      <Shortcut Id="StartMenuShortcut" Directory="ProgramMenuFolder" Name="Treadcore"
                Target="[#treadcore.exe]" Arguments="--solo" WorkingDirectory="INSTALLFOLDER" />
      <Environment Id="PathEntry" Name="PATH" Value="[INSTALLFOLDER]" Part="last" Action="set" System="yes" />
    </Component>
  </Feature>
</Package>
```

```powershell
dotnet tool install --global wix --version 5.0.2
wix build packaging/windows/treadcore.wxs -arch x64 -d Version=0.1.0 -d BinDir=target/release -o treadcore.msi
```

The `UpgradeCode` never changes; it is how Windows knows a newer
treadcore replaces an older one. treadcore's is a UUID derived from the
project's domain, so it can be regenerated and checked:
`uuid.uuid5(uuid.NAMESPACE_DNS, "upgrade.treadcore.org")`.

**Nix** builds from `Cargo.lock` with no extra hashes, because there
are no git dependencies, and wraps the window with the libraries it
loads at run time, since NixOS has no global library path:

```nix
default = pkgs.rustPlatform.buildRustPackage {
  pname = "treadcore";
  version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
  cargoBuildFlags = [ "-p" "treadcore-client" "-p" "treadcore-server" ];
  nativeBuildInputs = with pkgs; [ pkg-config makeWrapper ];
  buildInputs = with pkgs; [ alsa-lib udev wayland libxkbcommon vulkan-loader ];
  postFixup = ''
    wrapProgram $out/bin/treadcore --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (runtimeLibs pkgs)}
  '';
};
```

## V. Test every package where its users are

A package that builds is not a package that installs. Each one is
installed on a fresh copy of the system it is for, with that system's
own package manager resolving the dependencies, and then run:

```yaml
test-deb:
  needs: [version, deb]
  strategy:
    fail-fast: false
    matrix:
      include:
        - { image: "debian:12", arch: amd64, runner: ubuntu-latest }
        - { image: "linuxmintd/mint22-amd64", arch: amd64, runner: ubuntu-latest }
        - { image: "debian:12", arch: arm64, runner: ubuntu-22.04-arm }
        # ...
  runs-on: ${{ matrix.runner }}
  container: ${{ matrix.image }}
  steps:
    - uses: actions/download-artifact@v4
      with: { name: "deb-${{ matrix.arch }}" }
    - shell: bash
      run: |
        check() {   # run a test; if it fails, its output becomes a public annotation
          if "$@" > out.log 2>&1; then rc=0; else rc=$?; fi; tail -5 out.log
          if [ $rc -ne 0 ]; then echo "::error title=$1 failed (exit $rc)::$(tail -c 1500 out.log | tr '\n\r' '  ')"; exit 1; fi
        }
        check apt-get update
        check env DEBIAN_FRONTEND=noninteractive apt-get install -y ./treadcore-*-linux-${{ matrix.arch }}.deb
        check treadcore --help
        check sh -c '! ldd /usr/bin/treadcore | grep "not found"'
        check test -f /usr/share/applications/org.treadcore.treadcore.desktop
        check sh -c 'treadcore-server --local --seconds 60 --swarm 0 2>&1 | grep "match over"'
```

The window cannot open without a display on a runner, so three things
stand in for it: `--help` proves the binary loads, `ldd` proves every
linked library was brought by the package, and a sixty-second bot
match from the installed server proves the game itself runs.

Two details are earned, not obvious:

- **`if "$@"; then ... else rc=$?`.** GitHub runs bash with `-e`, which
  exits on the first failed command, before a plain `"$@"; rc=$?` could
  record anything.
- **The failure as an annotation.** A job's log needs a login to read,
  even on a public repository; its annotations do not. The first run's
  Fedora failure said only "exit code 1" in public. Now it says why.

**The gate.** The release job needs every test:

```yaml
release:
  needs: [version, deb, rpm, windows, macos, test-deb, test-rpm, test-windows, test-macos]
  steps:
    - uses: actions/download-artifact@v4
      with: { path: out, merge-multiple: true }
    - run: cd out && sha256sum * > SHA256SUMS
    - uses: softprops/action-gh-release@v2
      with:
        draft: true
        prerelease: ${{ contains(needs.version.outputs.version, '-') }}
        files: out/*
```

If one package fails anywhere, there is no draft at all. The Nix build
runs beside them but is not in `needs`: it is the newest route, and a
failure there should not hold the others back.

## VI. The git hook

Nothing above runs unless a tag reaches GitHub, so the last check
belongs before the tag leaves the machine. A `pre-push` hook reads the
refs being pushed on standard input:

```bash
#!/usr/bin/env bash
# .githooks/pre-push
ROOT="$(git rev-parse --show-toplevel)"
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)"
status=0
while read -r local_ref local_sha remote_ref remote_sha; do
    case "$remote_ref" in refs/tags/v*) ;; *) continue ;; esac   # branches pass
    tag="${remote_ref#refs/tags/}"
    if [ "$tag" != "v$VERSION" ]; then
        echo "pre-push: tag $tag is not v$VERSION, the version in Cargo.toml" >&2
        status=1; continue
    fi
    if [ "$(git rev-parse "$local_sha^{commit}")" != "$(git rev-parse HEAD)" ]; then
        echo "pre-push: check out $tag first, so the check runs on what it names" >&2
        status=1; continue
    fi
    "$ROOT/scripts/release.sh" >&2 || { echo "pre-push: release check failed" >&2; status=1; }
done
exit "$status"
```

Hooks are not cloned with a repository; each checkout turns them on:

```make
hooks:
	@git config core.hooksPath .githooks
```

`git push --no-verify` skips a hook, which is the point: skipping it is
deliberate and visible, not an accident.

## VII. The release check: documentation is part of the build

`scripts/release.sh` (`make release`) is the procedure as code. It
changes nothing but build output, and stops at the first failure:

1. the version agrees in `Cargo.toml`, the path dependencies and a
   `CHANGELOG.md` heading;
2. `make test` passes and `make build` prints **no warnings**;
3. **the pictures are of this version**: `make shoot` drives the window
   through a fixed match and writes `reports/images/MANIFEST-<version>.txt`;
   every listed frame must exist, be non-empty, be newer than the
   window's newest source file, and be shown by the player's guide or
   the site;
4. the documents name the guide, and every image they show is a file;
5. `cargo publish --dry-run` packages the first crate.

The picture rule is the one that pays. The first candidate's gallery
held three zero-byte frames for half a day; a frame older than the code
shows a window that no longer exists. Both now stop a release:

```bash
since="$(stat -c %Y "$ROOT"/source/treadcore-client/src/*.rs | sort -n | tail -1)"
while read -r f; do
    [ -s "$IMG/$f" ] || { bad "$f missing or empty"; continue; }
    [ "$(stat -c %Y "$IMG/$f")" -ge "$since" ] || { bad "$f is older than the window: shoot again"; continue; }
    grep -q "$f" "$GUIDE" "$ROOT/index.html" || bad "$f is taken but no document shows it"
done < "$MAN"
```

A feature is not done until the guide shows it: its frame is added to
`scripts/shoot.sh`, the guide's section is written against the frame,
and from then on the check holds the two together.

## VIII. The publishing limit

Two things cannot be undone: a version on crates.io (it can be yanked,
never removed) and a published GitHub release that people have
downloaded. Both wait **at least a day after the tag**. The tagged
build is lived with for a day; anything found in it makes a new
candidate, not a published version. The script enforces it:

```bash
# scripts/publish.sh (make publish)
tagged="$(git for-each-ref --format='%(taggerdate:unix)%(committerdate:unix)' "refs/tags/v$VERSION" | cut -c1-10)"
age=$(( $(date +%s) - tagged ))
if [ "$age" -lt 86400 ]; then
    die "v$VERSION was tagged $((age / 3600)) h ago; publish no sooner than $(date -d "@$((tagged + 86400))")"
fi
for c in treadcore-sim treadcore-proto treadcore-net treadcore-brain treadcore-client; do
    cargo publish -p "$c"      # in dependency order; cargo waits for each to reach the index
done
```

The crates go in dependency order because crates.io will not accept a
crate whose dependency is not yet in its index. `make token` stores the
crates.io token through `cargo login`, in `~/.cargo/credentials.toml`,
never in the repository.

## IX. API usage

Two rate limits shape the pipeline:

- **GitHub's REST API allows 60 unauthenticated requests an hour per IP
  address** (5,000 with a token). Watching a workflow from a script by
  polling every few seconds would spend that in minutes; the polling in
  this work was once a minute, against one endpoint, and stopped when
  the run finished. When one endpoint returned 502 for a while, the
  same results were read from another (`/commits/<tag>/check-runs`)
  rather than retried in a loop.
- **A web page that calls the API spends the visitor's quota**, and
  every visitor behind one office NAT shares it. treadcore's download
  page first did exactly that. It no longer calls any API.

Instead, the maintainer asks once, at publish time:

```bash
make builds      # scripts/builds.sh: at most three requests
```

It reads the newest published release and its assets, the check runs
of the release commit (which package passed its test where), and the
crate's version from crates.io, then writes `builds.json` and embeds
the same data in `index.html` and `download.html`:

```html
<script type="application/json" id="builds"><!-- BUILDS: written by make builds -->{...}</script>
```

The pages render their links and the build grid from that block. A
visitor costs no API request; the data is as fresh as the last release,
which is exactly as fresh as it can usefully be. The script prints the
remaining quota after each call, and honours `GITHUB_TOKEN` if set.

## X. The build grid

`index.html`, under "Where it is built, and for what", shows one row
per target: the system, the architecture, the form (`.deb`, `.rpm`,
`.msi`, zip, flake, `make install`, `cargo install`), checkboxes for
**built**, **tested there** and **published**, and the runtime
dependencies, with the release, its build date and the crate's version
underneath. Each box is read from the data `make builds` wrote: built
if the release has the file, tested if that system's job in the
workflow passed, published if the release is public. Source routes show
"by hand" where the author tests them on his own machine (Copal, Alpine) and a
dash where nobody has yet.

## XI. Results

**0.1.0-rc.2**, the first run of the workflow:

| job | result |
| --- | --- |
| build: .deb amd64, .deb arm64, .rpm x86_64, .rpm aarch64, Windows, macOS | all passed |
| .deb installed and played: Debian 12, Debian 13, Ubuntu 22.04, Ubuntu 24.04, Linux Mint 22 | all passed |
| .rpm installed and played: Fedora 41, Fedora 42 | **both failed** |
| Windows zip run, macOS app verified and run | passed |
| Nix flake built and run | passed |
| draft release | **not made**: the gate held |

The `.rpm` failed on both Fedoras, so the package, not one release of
Fedora, was at fault. Its logs needed a login; its annotations said
only "exit code 1". Inspecting a locally built `.rpm` with `rpm -qp
--requires` showed the builtin dependency reader's output, faithful to
that (musl) binary but not something to trust blind. rc.3 makes the
`.rpm` with RPM's own generator inside the Fedora build container, and
every test now publishes its failing output.

**0.1.0-rc.3**: see the addendum below, written when its run finished.

## XII. What to take from this

1. Build each target natively, on the oldest system you support.
2. Let the tools read linked libraries; name what is loaded at run time.
3. Test the package, not the binary: install it where its users are,
   with their package manager, and run it.
4. Gate the release on those tests, and make failures readable in
   public.
5. Put the check before the tag leaves the machine: a hook.
6. Treat the documentation as part of the build: pictures of this
   version, every one shown.
7. Wait a day before anything that cannot be undone.
8. Spend API calls once, at release time, never per page view.

## References

1. cargo-deb, https://github.com/kornelski/cargo-deb
2. cargo-generate-rpm, https://github.com/cat-in-136/cargo-generate-rpm
3. WiX Toolset, https://wixtoolset.org
4. GitHub REST API rate limits, https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api
5. GitHub Actions workflow commands (annotations), https://docs.github.com/en/actions/reference/workflow-commands-for-github-actions
6. treadcore, `documentation/RELEASE.md` sections 4 and 6; journals 0018 to 0021.
