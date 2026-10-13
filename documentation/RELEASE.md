# The implementation cycle, versions and releases

How a change gets from an idea to a numbered release that people have
played. The cycle is small on purpose: `main` always builds and
always has a journal entry for every session that changed it.

## 1. The cycle

    issue -> branch -> commits -> pull request -> review -> main -> journal -> push

1. **Issue.** Work starts as an issue (bug, task, or an accepted
   proposal; see `GOVERNANCE.md`), assigned to a milestone.
2. **Branch.** `topic/short-name` from `main`. No branch lives longer
   than a milestone; long work is split so that each piece lands.
3. **Commits.** Small, each one building and passing `make test`.
   Message in the repository's form: `area: what changed, in a
   sentence`, body with why, `Signed-off-by:` (`git commit -s`), signed
   with the author's key. Areas are the crate or directory: `sim:`,
   `net:`, `brain:`, `client:`, `server:`, `build:`, `docs:`,
   `journal:`, `site:`.
4. **Pull request.** Describes the change in the roadmap's four lines
   (change, why, must not change, proof) and, for a sim change, what
   the default trace shows differently. The template asks for exactly
   this.
5. **Review.** `GOVERNANCE.md` section 7. The reviewer runs `make
   test`; for a sim change, `make debug` and a read of the trace; for
   a net change, `make lobby-test`.
6. **Main.** Merged by a maintainer, linear history. `main` is never
   broken; a break is fixed or reverted the same day.
7. **Journal.** Every working session that changed the repository ends
   with a journal entry (`journal/`), a `CHANGELOG.md` line, and a push.
   Nothing stays local overnight.

While the project is one person, the cycle is the same with the
review step being the author's own read of the diff and the trace
the next morning; the pull request is still opened, so the record
has the four lines.

## 2. Versions

`0.MINOR.PATCH` until the first release monument that a stranger can
play across the Internet; `1.0.0` then, and semantic versioning after.

- **PATCH**: fixes; no change to rules numbers, the wire, or the map
  generator's output for a given seed.
- **MINOR**: a roadmap step landed, or a rules change, or a wire change.
  Any change to `treadcore-proto::VERSION` is at least a MINOR bump, and
  the release notes say the two versions do not play together.
- **MAJOR**: after 1.0, a wire or replay-format break.

The crate version lives once, in `[workspace.package] version`, and
the server and client print it at start. A build from a tag is
reproducible from `Cargo.lock`.

## 3. Release monuments

A monument is a release that marks a roadmap step proven. Each has a
name, a number, and the proof from `ROADMAP.md` as its definition of
done. Nothing is a monument until its proof has been run and written
into the release notes.

| version | monument | roadmap steps | proof |
| --- | --- | --- | --- |
| 0.1 | Opening Day | 0 | every crate compiles; tests pass; `make debug` traces a match in which indirect fire, recon and the bubble all occur; `make lobby-test` passes with every client welcomed; the window opens on one machine |
| 0.2 | Replay | 1, 2 | a recorded bot match replays to the same world hash on a different machine; four lobby-test peers log identical hashes |
| 0.3 | Pillboys | 3 | a bot that captures the two nearest pillboxes beats one that does not, over twenty seeds |
| 0.4 | Maps | 5 | `generate(seed).to_file().from_file() == generate(seed)`; one hand-authored map ships |
| 0.5 | Prediction | 4 | driving feels immediate under 200 ms of injected delay and the mismatch log is empty on a straight road |
| 0.6 | Teams | 6 | two bots on a team share recon: a shell fired by one at a target only the other's drone sees is spotted |
| 0.7 | Across the Internet | 7, 8 | two peers behind two home routers reach Playing; a `--listen` server is elected when its links are best and ignored when they are not |
| 1.0 | Release | 9 | sound; a dist build per platform; the 0.7 proof repeated by playtesters who are not members |

Monuments between these (a 0.3.1 with a balance change, say) are
ordinary releases and follow the same checklist.

## 4. Release candidates and user testing

A monument is reached only when people who did not build it have
played it. Release candidates exist for that.

1. **Tag** `vX.Y.0-rc.1` from `main` when the maintainers believe the
   proof holds. Build with the `dist` profile (`cargo build --profile
   dist`), one tarball per platform the members can build on, with a
   `SHA256SUMS` file.
2. **Call for playtest** in Discussions under *Playtests*, naming the
   candidate, what to play (solo, LAN of two, LAN of four, the debug
   trace), what to look at (the monument's proof, and anything that
   felt wrong), and the deadline (one week; two for a 0.x.0).
3. **Playtest reports** come in as issues from the `playtest.md`
   template: machine, platform, mode, what happened, the log file
   from `build/logs/` attached. A report is a contribution; the
   reporter is credited.
4. **Confirmation.** A candidate is confirmed when
   - at least three playtesters, at least one not a maintainer, report
     the monument's proof holding;
   - every `bug` opened against the candidate is fixed in a new
     candidate or explicitly deferred to the next milestone in the
     issue, with the reason;
   - `make test`, `make debug` and `make lobby-test` pass on every
     platform a tarball was built for.
5. **Promote**: tag `vX.Y.0` on the same commit as the confirmed
   candidate (a fix means a new candidate, not a different commit).
   Then the release ceremony (`GOVERNANCE.md` section 10): GitHub
   Release with notes, checksums and tarballs; journal entry; report
   page; the site updated.

Until there are three playtesters, the steward's own playtest on two
machines confirms a candidate, and the release notes say so.

## 5. Checklists

**Before tagging a candidate**

- [ ] `make test` green on `main`
- [ ] `make debug` default seed; trace read; numbers in the notes
- [ ] `make lobby-test` passes
- [ ] `CHANGELOG.md` section for the version; `CONTRIBUTORS.md` current
- [ ] `[workspace.package] version` bumped; `treadcore-proto::VERSION`
      bumped if the wire changed, and `PROTOCOL.md` says so
- [ ] documents that describe changed behaviour updated in the same
      release (`DESIGN.md`, `PROTOCOL.md`, `BUILD.md`, `README.md`)

**Tagging**

- [ ] signed, annotated tag: `git tag -s vX.Y.Z-rc.N -m "..."`
- [ ] `cargo build --profile dist` per platform; tarball named
      `treadcore-vX.Y.Z-rc.N-<os>-<arch>.tar.gz`; `SHA256SUMS`
- [ ] GitHub pre-release with the call-for-playtest text

**After promoting**

- [ ] GitHub Release (not pre-release) with notes, tarballs, checksums
- [ ] journal entry; report page under `reports/`; `index.html` links it
- [ ] milestone closed; next milestone's issues triaged
- [ ] the release ceremony's acts done (`GOVERNANCE.md` section 10)

## 6. Hotfixes

A fix to a released version that cannot wait: branch from the tag,
fix, `make test`, tag `vX.Y.(Z+1)`, release with a one-paragraph
note, then merge forward to `main`. No candidate for a patch unless
it touches `treadcore-sim`, in which case one candidate and one
maintainer's playtest.
