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

The prior art's original artwork, sounds and code are not vendored and will not
be: treadcore draws rectangles and lines and takes from the prior art only
ideas. The prior art *maps* are the one exception, made on 12 October 2026 at
the author's direction: five community maps, converted to treadcore's
own text format, live in `maps/prior-art/` with their provenance in
`THIRD-PARTY.md`, licence not established, removable on request. See
documentation/RESEARCH.md and documentation/MAPS.md.


Currently vendored: nothing.
