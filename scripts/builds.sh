#!/usr/bin/env bash
# builds.sh (make builds): what is built, tested and published, for the
# grid on index.html and the links on download.html.
#
# Asks GitHub and crates.io once, here, and writes the answer into the
# repository: builds.json, and the same data embedded in index.html and
# download.html between their BUILDS markers. The pages never call an
# API themselves, so a visitor costs GitHub nothing: GitHub allows 60
# unauthenticated calls an hour per address, and a busy page would
# spend them for everyone behind one NAT. This script makes at most
# three calls (the releases, the release commit's check runs, the crate)
# and prints what is left; GITHUB_TOKEN raises the limit to 5000.
#
# Run it on the day a release is published (RELEASE.md section 6),
# after `make publish`, and commit what it changed.
. "$(dirname "$0")/common.sh"
python3 - "$ROOT" <<'PY'
import json, os, re, sys, urllib.request, datetime
ROOT = sys.argv[1]
REPO = "treadcore/treadcore"
calls = []

def get(url):
    req = urllib.request.Request(url, headers={"User-Agent": "treadcore-builds (https://treadcore.org)", "Accept": "application/vnd.github+json"})
    tok = os.environ.get("GITHUB_TOKEN")
    if tok and "api.github.com" in url:
        req.add_header("Authorization", f"Bearer {tok}")
    with urllib.request.urlopen(req, timeout=30) as r:
        calls.append((url, r.headers.get("x-ratelimit-remaining")))
        return json.load(r)

# Every way in, with what it needs. "asset" is the release file's name
# after the version; "test" is the workflow job whose success means the
# package installed and played a match there.
TARGETS = [
  # system, arch, kind, asset, test job, runtime dependencies
  ("Debian 12, 13", "x86-64", ".deb", "linux-amd64.deb", "test-deb (debian:12, amd64)", "libwayland-client0, libxkbcommon0, libvulkan1, mesa-vulkan-drivers"),
  ("Debian 12, 13", "ARM64", ".deb", "linux-arm64.deb", "test-deb (debian:12, arm64)", "as above"),
  ("Ubuntu 22.04, 24.04", "x86-64", ".deb", "linux-amd64.deb", "test-deb (ubuntu:24.04, amd64)", "as Debian"),
  ("Linux Mint 22", "x86-64", ".deb", "linux-amd64.deb", "test-deb (linuxmintd/mint22-amd64, amd64)", "as Debian"),
  ("Fedora 41, 42", "x86-64", ".rpm", "linux-x86_64.rpm", "test-rpm (fedora:42, x86_64)", "libwayland-client, libxkbcommon, vulkan-loader, mesa-vulkan-drivers"),
  ("Fedora 41, 42", "AArch64", ".rpm", "linux-aarch64.rpm", "test-rpm (fedora:42, aarch64)", "as above"),
  ("Windows 10, 11", "x86-64", ".msi", "windows-x86_64.msi", "test-windows", "none (DirectX 12 or Vulkan from the GPU driver)"),
  ("Windows 10, 11", "x86-64", "zip", "windows-x86_64.zip", "test-windows", "none"),
  ("macOS 11+", "Apple silicon", "zip", "macos-arm64.zip", "test-macos", "none (Metal)"),
  ("NixOS, Nix", "x86-64, AArch64", "flake", None, "nix", "in the flake"),
  ("Copal, Alpine", "AArch64, x86-64", "make install", None, "hand", "make deps: apk"),
  ("Arch, Omarchy", "x86-64", "make install", None, None, "make deps: pacman"),
  ("Gentoo", "any", "make install", None, None, "make deps: emerge"),
  ("Any, with Rust", "any", "cargo install", None, None, "make deps for the system"),
]

rel = None
try:
    for r in get(f"https://api.github.com/repos/{REPO}/releases?per_page=10"):
        if not r["draft"] and any(re.search(r"\.(deb|rpm|msi|zip)$", a["name"]) for a in r["assets"]):
            rel = r; break
except Exception as e:
    print(f"  releases: {e}", file=sys.stderr)

checks = {}
if rel:
    try:
        runs = get(f"https://api.github.com/repos/{REPO}/commits/{rel['tag_name']}/check-runs?per_page=100")
        checks = {c["name"]: c["conclusion"] for c in runs.get("check_runs", [])}
    except Exception as e:
        print(f"  check runs: {e}", file=sys.stderr)

crate = None
try:
    c = get("https://crates.io/api/v1/crates/treadcore-client")["crate"]
    crate = {"version": c["max_version"], "date": c["updated_at"][:10]}
except Exception as e:
    print(f"  crates.io: {e}", file=sys.stderr)

version = rel["tag_name"].lstrip("v") if rel else None
rows = []
for system, arch, kind, asset, test, deps in TARGETS:
    url = None; built = False
    if asset and rel:
        name = f"treadcore-{version}-{asset}"
        a = next((a for a in rel["assets"] if a["name"] == name), None)
        if a: url, built = a["browser_download_url"], True
    if kind == "cargo install":
        built = crate is not None
        url = "https://crates.io/crates/treadcore-client"
    if kind in ("make install", "flake"):
        built = True
        url = "https://treadcore.org/download.html"
    tested = "hand" if test == "hand" else (checks.get(test) == "success") if test else None
    rows.append({"system": system, "arch": arch, "kind": kind, "deps": deps,
                 "built": built, "tested": tested, "published": bool(url and (rel or kind != ".deb")),
                 "url": url})

out = {
    "generated": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%d %H:%M UTC"),
    "release": {"tag": rel["tag_name"], "version": version, "date": rel["published_at"][:10],
                "url": rel["html_url"], "prerelease": rel["prerelease"]} if rel else None,
    "crate": crate,
    "rows": rows,
}
with open(os.path.join(ROOT, "builds.json"), "w") as f:
    json.dump(out, f, indent=1); f.write("\n")
blob = json.dumps(out, separators=(",", ":")).replace("</", "<\\/")
for page in ("index.html", "download.html"):
    p = os.path.join(ROOT, page); s = open(p).read()
    new = f'<script type="application/json" id="builds"><!-- BUILDS: written by make builds -->{blob}</script>'
    s2, n = re.subn(r'<script type="application/json" id="builds">.*?</script>', lambda m: new, s, flags=re.S)
    if n != 1:
        print(f"  {page}: no BUILDS block to fill", file=sys.stderr); continue
    open(p, "w").write(s2)
print(f"  release: {out['release']['tag'] + ' of ' + out['release']['date'] if rel else 'none with packages published yet'}")
print(f"  crate:   {crate['version'] + ' of ' + crate['date'] if crate else 'unknown'}")
for url, left in calls:
    print(f"  api call {url.split('?')[0]}  (GitHub calls left this hour: {left})" if left else f"  api call {url}")
PY
