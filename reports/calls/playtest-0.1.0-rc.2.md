# Call for playtest: treadcore 0.1.0-rc.2

*Draft for GitHub Discussions, category Playtests (RELEASE.md section
4, step 2). Not posted. It assumes 0.1.0-rc.2 is cut first, because
rc.1 lacks the first-minute grace and its bots shell a sitting
newcomer at thirty seconds. To call for rc.1 instead, change the
version and drop the first-minute line under "What to look at".*

---

treadcore 0.1.0-rc.2 is the second release candidate, and the first
offered for playtesting. It is a sixteen-player overhead tank game for
the drone battlefield: the gun fires indirect at what a recon drone
sees, interceptors hold a ring round the hull, a squad of four takes
the ground with roads, walls, mines and pillboxes, and the players
elect a host among themselves. No server.

**Install**

```
cargo install treadcore-client     # the window, as `treadcore`
treadcore --solo                   # you against fifteen bots
```

Linux, with a Vulkan driver (`mesa-vulkan-swrast` on a machine with
no GPU). From a clone, `make configure && make run` does the same and
gives you the traces.

**How to play** is
[documentation/HOW-TO-PLAY.md](https://github.com/treadcore/treadcore/blob/main/documentation/HOW-TO-PLAY.md),
with pictures of this build.

**What to play**, as much of it as you can:

1. Solo: `treadcore --solo`, at least two matches to the end.
2. A LAN of two: `treadcore --lan` on two machines on one network.
3. A LAN of four, if you have four machines.
4. Across the Internet: one of you runs `make introduce`, the others
   `INTRODUCER=host:port make lan`.
5. The debug trace: `make debug`, and a look at the report it ends
   with.

**What to look at**

- The candidate's proof: two or more people in one match, on separate
  machines, with no server, and a player who drops and starts again
  gets the same seat back.
- The first minute: sit still at the start. The bots should leave you
  alone for sixty seconds, and not after.
- The builder: press `4` to `8`, click a square, and see whether the
  squad does what the outlined square promised.
- Anything that felt wrong, however small. A refusal you could not
  understand is a finding.

**How to report**

Open an issue from the
[playtest template](https://github.com/treadcore/treadcore/issues/new?template=playtest.md):
your machine and platform, the mode, what happened, and the log from
`build/logs/` (or the terminal's output, from a `cargo install`). A
report is a contribution and the reporter is credited.

**Deadline**: Tuesday 27 October 2026, two weeks, as for any 0.x.0.

Confirmation needs three playtesters, one of them not a maintainer,
reporting the proof holding (RELEASE.md section 4).
