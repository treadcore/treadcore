# Phase Zero and a Candidate: The Learned Tank's Senses and Net, and dodge_horizon 2 Through Every Read

**Check-in report CI-058.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 96; roadmap backlog
69 and 70; `build/net0/` and `build/horizon/` on the census machine.

---

## Abstract

Phase 0 of the learned tank is built, tested and read. The senses are
101 numbers: a compass of 8 sectors by 10 channels of signal strength,
the body, and the brain's hints. A 14,068-weight net in plain Rust and
`NetBrain` drive, aim and fire by them, five decisions a second, every
choice masked to what can be done. `CENSUS_NET` seats a team of nets in
the census, and without it every match is the same as before. A
decision costs 7.8 µs on the Mac. A random team reads −19.30 ± 0.36
over 110 seeds, every map far below, as a net that knows nothing must.
Alongside, dodge_horizon 2 passed both held-out reads on today's
defaults (+1.82 ± 0.27 and +1.66 ± 0.29, 400 seeds each), with no map
below, and `make families` holds: a candidate for Paul.

**Index terms:** sensor interface, multilayer perceptron, action
masking, held-out replication, brain census.

---

## I. Introduction

### A. Intent

Build phase 0 of LR 95: the senses, a net, a brain that acts by it,
and a census of it. Carry the dodge_horizon candidate through its rule.

### B. The prompts

> very that you are progressing

> yes. please. start.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "very that you are progressing" | Shown with timestamps from the machine; a leftover census found to be a held-out read, left to finish | Kill it: it was the run's own rule at work |
| D2 | The run took "the newest census" | It reads its census's folder from census.sh's own line, so reads can run beside it | Keep stopping the run for each side read |
| D3 | The fit's draws re-read read ends | Draws skip a trial already read; a slip (819fa23) fixed (f8a1ec3) before the run called it | — |
| D4 | dodge_horizon 2 high three times | Two held-out reads on today's defaults, rule first | Promote on 55-seed reads |
| D5 | The senses need the brain's conclusions | Modules in treadcore-brain (`senses.rs`, `net.rs`), not yet a crate of their own | A crate now: nothing outside the brain uses them yet |
| D6 | A compass needs a direction convention | Sector k turns clockwise on the screen; a test at 8 headings, and the test broken on purpose | Assume it |
| D7 | Seeds 0 and 1 made one net | SplitMix64 before the world's `Rng` | — |
| D8 | A Move costs the world a route | A drive one way renewed every 2 s, not each decision | — |
| D9 | Drone, squad and signal need their phases | Scored, held | Map them now: untested actions in phase 0 |
| D10 | The census must not change without a net | HEAD against phase 0, 18 matches: the same | — |
| D11 | A failed remote command left the machine at the old commit | The census without `CENSUS_NET` support stopped, the tree pulled, the census started again, the net seated confirmed in its log | Read its result: it would have been the defaults against themselves |
| D12 | The rule's timing part shared the machine | Recorded as not readable from this census; the decision's own cost given instead | Call it passed |
| D13 | The candidate passed every step | Left for Paul | Change the default: the rule says with Paul |

---

## III. Methods

Tests: `cargo test --release -p treadcore-brain` (96, 6 new). Identity:
`git stash`, the 6-seed census, `git stash pop`, again, compared.
Speed: `net_decisions_a_second` (ignored test). Censuses on the census
machine: `CENSUS_NET=seed:1 SEEDS=110 SEED0=500000 MAP=all JOBS=24
make census` (`build/net0/run.sh`); `TRIAL=build/horizon/h2.weights
SEEDS=400 SEED0=3500000` and `3600000` (`build/horizon/run.sh`).
Families: `make families SET="dodge_horizon=2"` on the Mac.

---

## IV. Results

| Read | Result |
| --- | --- |
| The senses' and net's tests | all pass; the compass test fails with the compass reversed |
| Without a net | the same matches as HEAD |
| A random team, Mac, 6 seeds of 300 s | −8.18 ± 0.78; the same twice |
| A decision | 7.8 µs (Mac, one thread) |
| A random team, census machine, 110 seeds | −19.30 ± 0.36, every map below |
| dodge_horizon 2, from 3500000 | +1.82 ± 0.27 |
| dodge_horizon 2, from 3600000 | +1.66 ± 0.29 |
| families at dodge_horizon 2 | all nine hold |

Errors: an apostrophe broke a remote command and left the machine at
f8a1ec3; the census it started is the defaults against themselves,
stopped and set aside. A suggest.py slip (819fa23), fixed before use.

---

## V. Discussion

Phase 0 gives the learned tank a body and a census seat. Its senses are
tested one by one, and its net plays the same match twice. Phase 1
teaches it to go somewhere, by evolution on the CPUs. The candidate is
the run's first: found by the screen, the fit and the run each, and
then held out twice on 400 seeds with room to spare.
