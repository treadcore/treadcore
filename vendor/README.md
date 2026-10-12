# vendor

External code and binary blobs that treadcore ships with, copied here
by the author rather than fetched at build time, each in its own
directory with its licence beside it.

The rule: nothing lands here without three things.

1. Its licence, verbatim, in `vendor/<name>/LICENSE`.
2. A line in `THIRD-PARTY.md` saying what it is, where it came from,
   which version, and under what terms.
3. A reason it is vendored instead of pulled from a registry: the
   upstream is unmaintained, the blob is not on any registry, the build
   must work offline, or the author has patched it.

Rust crates from crates.io are not vendored here; Cargo fetches them
and `Cargo.lock` pins them. Their licences are listed in
`THIRD-PARTY.md` under "Crates" so the set is still visible in one
place. If the build ever needs to be fully offline, `cargo vendor
vendor/crates` puts them in this directory under the same rules.

Original Bolo artwork, sounds, maps and code are not vendored and will
not be: treadcore draws rectangles and lines, generates its maps from a
seed, and takes from Bolo only ideas. See documentation/RESEARCH.md.

Currently vendored: nothing.
