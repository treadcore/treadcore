# Changelog

What changed, by date. For what happened, in order, and why, see the
creation journal in `journal/`: one entry per working session, in a
fixed form (`journal/TEMPLATE.txt`).

## 2026-10-12, evening: proven, fixed, sixteen

Journal: [0002](journal/0002-2026-10-12-proving-the-world.txt).

- Proven: every crate compiles (the client against Bevy 0.17.3 with
  three dead-code warnings); 16 tests pass; `make debug` traces a match
  with indirect fire, recon and the bubble all in it; `make lobby-test`
  passes with a real assertion (one host, three clients welcomed, all
  three saw the result). The window does not open in the development
  VM: no Vulkan driver (`BUILD.md`, "The window in this VM").
- Fixed: `net` UDP receive buffer 2 KiB -> 64 KiB (the 16.5 KiB `Welcome`
  was truncated and no client ever joined; the loopback test found it).
  `sim` `Move` now routes with the map's A*, the tank follows the
  waypoints, brakes to pivot at corners, and re-plans if blocked (a bot
  sat at the river bank for a whole match). `build` `SECONDS` renamed
  `GAME_SECONDS` (bash's own counter made the match as long as the
  build took, plus 180). `server` reads the final snapshot before
  judging a lost connection; the host waits a second before `Leave`.
- Changed: sixteen seats (`MAX_PLAYERS` 16): sixteen starts round the
  edge, a depot each, a ring road; `--players N` and `PLAYERS=n`;
  sixteen bots is the default. The wire is `VERSION` 2 (`Tank` gained
  `route`). The rule bot's doctrine rewritten: firing position short of
  the objective, recon over it from a standing tank, indirect fire with
  lead, patience drawn per bot, advance to capture.
- New: `GOVERNANCE.md` (roles, application, entry, proposal,
  agreement, review, removal, resignation, ceremony, recognition,
  assessment, award, annual processes; the organization on GitHub),
  `RELEASE.md` (the implementation cycle, versions, release monuments,
  candidates confirmed by playtest), `CONTRIBUTING.md`,
  `CONTRIBUTORS.md`, `.github/` templates and `CODEOWNERS`.
  `SPEC-WINBOLO-1.14.md`: the clean-room functional specification of
  WinBolo 1.14 (256 requirements), with a preface on what treadcore
  adopts, adapts and leaves; the roadmap's Bolo-derived steps and
  backlog cite its IDs.
  `LAB-REPORT-PILLBOYS.md`: the Pillboy requirement checked against
  Bolo's builder; `DESIGN.md` gained the Pillboys decision; `CONCEPT.md`

  section 9: sixteen tanks, defenders, alliances, betrayal; `ROADMAP.md`
  gained a backlog and steps 0b, 6b, 6c, 6d.
- Known: the release-profile engine (`make warm`, `make run`) not yet
  built; the window unproven here; two-machine LAN untested; the
  backlog's balance items (interceptors fighting interceptors, tank
  speed, drone retasking) open.

## 2026-10-12: opening day


Journal: [0001](journal/0001-2026-10-12-opening-day.txt).

- New: six-crate Rust workspace. `treadcore-sim` (deterministic world;
  compiled, 6 tests pass), `treadcore-proto`, `treadcore-net` (LAN
  lobby, host election, relay, Host/Client), `treadcore-brain`
  (RuleBot, JsonBrain), `treadcore-server` (headless host, traced bot
  matches), `treadcore-client` (Bevy 0.17 window with telemetry
  overlay), `treadcore-warm` (engine prerequisites).
- New: Makefile over `scripts/` over cargo; `make configure`, `warm`,
  `build`, `headless`, `debug`, `run`, `lan`, `host`, `seat`,
  `lobby-test`, `test`, `schema`.
- New: documentation (concept, design, protocol, research, lab
  report, build), vendor policy and third-party notices, site at the
  root for GitHub Pages, first report.
- Changed: build sized for a 6 GB machine after the engine build ran
  out of memory on bevy_ui (jobs from RAM, trimmed Bevy features, no
  dependency debug info, no release LTO).
- Known: crates above the simulation not yet compiled; `make warm` not
  yet run to completion.

Commits: 17c71d3, 3a26432, 49cd8f9.
