# The Squad's Time by Task: a Meter Line

**Check-in report CI-048.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 83; CI-047 and
section 82; roadmap backlog 65; `meter.rs`, `commander.rs`.

---

## Abstract

LR 82 left open what a bridge on atlantic costs in the builders' time.
The meter now counts, each second a seat is alive, whether its squad
is out and on what task: each build kind (a road over water is a
bridge), harvest, capture, repair, lift, the walk home, or out with no
task, with the trial's seats apart. The count decides nothing: the
same atlantic census before and after prints the same lines to the
digit. On a first four seeds the squad is out 37% of the time alive,
and most of that is the walk home (39%) and captures (28%); bridges
are 1.5%.

**Index terms:** meter, instrumentation, squads, building.

---

## I. Introduction

### A. Intent

To commit and push LR 82, then add the build-time meter it called for.

### B. The prompt

> commit and push. then add the build time meter

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "commit and push" | LR 82 committed and pushed on main (`a6c71f3`) | — |
| D2 | A census sees the bot's observation, and `obs.me` carries the squad's task | Count in `Commander::think_released`, a second a seat alive, beside the kiting count | Count in the sim: the meter is the brain's, and per seat and group there |
| D3 | `BuildKind::Road` builds a road on land and a bridge on water | A road whose site is water or bridge on the map is a bridge | One "road" kind: the bridge was the question |
| D4 | F: on the first look "other" was two-thirds of the time out | Split it: capture, repair, lift, home, idle | Attribute the walk home to the job before it: needs memory a seat, and a count that reads as what the squad is doing is simpler |
| D5 | A meter must not change play | The same census at a6c71f3 and after, 4 seeds on atlantic: the same to the digit | Trust it: a counter cannot steer, but the check was cheap |
| D6 | `cargo fmt --check` fails | Left as is: 972 diffs before the change, the file's long-line style kept | Format the crate: a thousand-line change in a meter step |
| D7 | The machine plays the pushed tree; this step is uncommitted | The atlantic reading waits for the commit | Copy the files to the machine: its tree would not be a commit |

---

## III. Methods

`meter::SquadTask` (13 tasks) and `meter::squad`, `Meter::squads`
(`[[u64; 14]; 2]`, the last seconds alive), summed by `add_fronts`
and printed by `Meter::line`. The check: `SEEDS=4 SEED0=500 SECS=300
MAP=maps/world/atlantic.map JOBS=4 make census` in a worktree at
a6c71f3 and in the working tree; `diff` of the sorted `seed` and
`team` lines. `cargo test -p treadcore-brain --release`.

---

## IV. Results

In LR 83: the same lines to the digit; 88 tests passed; the first look
(the squad out 37% of seconds alive; home 39%, capture 28%, pillbox
11%, wall 9%, harvest 5%, bridge 1.5%).

---

## V. Discussion

The question LR 82 asked was about bridges, but the first look says
the walking matters more: each job sends the squad out and back. If a
census on atlantic holds that, the posture saves its team the round
trips more than the building itself. That census is the next step.
