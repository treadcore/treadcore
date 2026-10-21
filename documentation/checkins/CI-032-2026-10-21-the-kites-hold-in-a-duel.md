# The Kite's Hold in a Duel

**Check-in report CI-032.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0089; `LAB-REPORT-BRAIN.md` section 66;
CI-031; roadmap backlog 55.

---

## Abstract

Backlog 55. The kite spends 62% of its time holding past the target's
reach (LR 65); the hypothesis was that a shorter wait scores more. The
meter, split by whom, puts 92% of the kite's seconds in rallies. A
scenario's Commander could not rally, so scenarios gained `pings =
true` and an `outscores` expectation. In a new family, a duel in the
open over 400 seeds, a Commander that rallies on a RuleBot outscores
it in 63.0% of seeds with the kite and 51.5% without. Twelve settings
of `kite_spare`, `kite_in` and `kite_margin` read within about an
error of the default, except `kite_in` 10, which is worse. The
hypothesis is not supported; the defaults stand; the family holds a
floor of 0.55. All six commits before this one were pushed at Paul's
word.

**Index terms:** kite, rally, duel, scenario families, tuning.

---

## I. Introduction

### A. Intent

To score the kite's hold where it can be scored, a duel, and move its
numbers if the duel says so; and to push the work and show Paul the
autopilot.

### B. The prompt

> nice . continue. commit. push. and let me see it run when you are ready to show me auto pilot

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt: "push" | The six local commits pushed first (`347b087..0c7e72f`) | — |
| D2 | F: a scenario Commander on Auto fortified and was shelled; it never kited | Find which kite the census counts | Building the duel blind |
| D3 | The kite runs in three places | The meter split by whom (`kite_kinds`); three seeds: rally 1,092, wing 68, intruder 27 | A 12-seed read: three seeds answered at 92% |
| D4 | F: a scenario's Commander has `pings` and `answers_pings` off | A `pings = true` key, as `fleet.rs` deals a leader | Making scenarios deal fleet roles: a larger change than the key |
| D5 | F: hits "by None" were a neutral pillbox | `pillboxes = false`, `depots = false`, grass, drones recalled | — |
| D6 | A duel is won or lost | `outscores`: hits given more than taken | Kills: a minute often ends without one |
| D7 | F: 63.0% on, 51.5% off; the numbers flat | The defaults kept; floor 0.55 | `kite_margin` 1 (65.0%): within an error |
| D8 | F: every tuning lead of LR 63 now read | Goal 4's tuning closed at every part on | — |

---

## III. Methods

**Code.** `meter.rs`: `kite(act)` takes the act; `kite_kinds` by group
(intruder, wingman, rally); the census line "seconds kiting by whom".
`scenario.rs`: a Commander's `pings = true` sets `pings` and
`answers_pings`; `outscores = { by, on }` holds when hits by `by` on
`on` outnumber those back.

**By whom.** `build/lr66/run.sh`: the fair census as LR 65, seeds 0 to
2, one seed a job.

**Family and sweep.** `scenarios/families/a-duel-in-the-open.toml`
(section 66). `make scenario S=... RUNS=400 SET="w=v"` for the
defaults, `kite` 0, `kite_spare` 0, 0.2, 0.8, 1.2, `kite_in` 7, 8, 10,
11, `kite_margin` 0.5, 1, 2.5, 3.5.

---

## IV. Results

| Measure | Result |
| --- | --- |
| Kiting by whom, 3 seeds | rally 1,092, wingman 68, intruder 27 |
| The duel, defaults | 63.0% +/- 2.4% |
| The duel, kite off | 51.5% +/- 2.5% |
| The best setting | `kite_margin` 1, 65.0% (within an error) |
| The worst setting | `kite_in` 10, 57.2% |

Tests: brain 84 passed and 11 ignored; client 32; net 14; proto 3; sim
50. Eight families, each above its floor.

---

## V. Discussion

1. **The hold is the win.** Holding past the target's reach until its
   gun reloads is how a duel is won; shortening the wait does not help.
2. **The census lean is not the duel's.** If the -0.94 at 48 seeds is
   real, it is in the team's fight, not in the kite's numbers.
3. **Tuning closed.** Every lead of the parts table is read; the brain
   with every part on is the starting numbers.

**For Paul:** none blocking. This commit is pushed with the rest; the
autopilot is shown next.
