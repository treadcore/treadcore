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

## Maps (in `maps/bolo/`)

Five Macintosh Bolo community maps from the author's own collection,
converted to treadcore's text format by `scripts/bolo2map.py` on 12
October 2026. Their authors are not named in the files and their
licence is not established: Bolo maps were traded freely in the 1990s
without licences. They are included as starting maps at the author's
direction; any author who objects will have theirs removed on request,
and a map with a known author and licence gets a proper entry here.

| file | Bolo name | source | licence |
| --- | --- | --- | --- |
| `maps/bolo/constanine.map` | Constanine | author's collection | not established |
| `maps/bolo/fidos-airstrip.map` | Fido's Airstrip | author's collection | not established |
| `maps/bolo/fortressisles2.map` | FortressIsles2 | author's collection | not established |
| `maps/bolo/fortressislesx.map` | FortressIslesX | author's collection | not established |
| `maps/bolo/godzilla.map` | Godzilla | author's collection | not established |

The converter is treadcore's own code (MIT). The Bolo map file format
it reads was learned from the files themselves and the format's public
description; no Bolo or WinBolo code was consulted.

## Ideas, not code


- Bolo (Stuart Cheshire, 1987/1989): the shared mutable world, the
  Brain interface, peers organising their own network. No code or
  assets from any Bolo implementation are used. The map logic in
  WinBolo (GPL) and PyBolo (all rights reserved, with an MIT-licensed
  WinBolo-derived component) was not consulted.
- Published reporting on vehicle-mounted counter-drone systems and
  indirect-fire tank employment, cited in documentation/RESEARCH.md.
  Facts, not expression.
