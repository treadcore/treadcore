# Third-party notices

treadcore itself is MIT (see /LICENSE). This file lists everything else
it uses, so the licence picture is in one place.

## Vendored (in this directory)

None yet. Each entry, when there is one:

| name | version | source | licence | why vendored |
| --- | --- | --- | --- | --- |

## Crates (fetched by Cargo, pinned by Cargo.lock)

Direct dependencies and their licences as published on crates.io.
Transitive dependencies are the usual Rust set (MIT or Apache-2.0 at
the developer's choice, with a few zlib, BSD and MPL-2.0 leaves in
Bevy's graphics stack); `cargo license` or `cargo deny` will enumerate
them from the lock file.

| crate | used by | licence |
| --- | --- | --- |
| serde, serde_derive | every crate | MIT OR Apache-2.0 |
| serde_json | brain, server | MIT OR Apache-2.0 |
| bincode 1.x | proto, net | MIT |
| log | every crate | MIT OR Apache-2.0 |
| env_logger | server | MIT OR Apache-2.0 |
| bevy 0.17 | client | MIT OR Apache-2.0 (its bundled default font, FiraMono, is OFL-1.1) |

## Ideas, not code

- Bolo (Stuart Cheshire, 1987/1989): the shared mutable world, the
  Brain interface, peers organising their own network. No code or
  assets from any Bolo implementation are used. The map logic in
  WinBolo (GPL) and PyBolo (all rights reserved, with an MIT-licensed
  WinBolo-derived component) was not consulted.
- Published reporting on vehicle-mounted counter-drone systems and
  indirect-fire tank employment, cited in documentation/RESEARCH.md.
  Facts, not expression.
