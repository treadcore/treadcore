# The Tests Play a Pinned Brain: A Promotion Moves the Game and Not the Tests

**Check-in report CI-096.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026, night.
Companion records: `LAB-REPORT-BRAIN.md` section 147; journal 0123.

---

## Abstract

From 1 to 4 November 13 of 36 promotions failed, every one on a unit
test that read the brain's defaults, none on a family. The unit tests
now play a pinned brain, the defaults of 4 November written down once
in `pinned.weights`. The families and the gauntlet still play the
defaults they judge. A lint holds every unit test to the rule. With the
change, the failed promotions' moves and a move of every searchable
weight pass every brain test.

## I. Intent

Paul: "commit push. then 1 and 2." Item 1 was the unit tests that lock
today's defaults, found when Paul asked to "look into making blockages
minized".

## II. Decision log

| Decision | Cause |
| --- | --- |
| A pinned brain, not a test-by-test fix | the first rule (name the weights you test) still read every other weight from the defaults |
| `pinned.weights` names every weight and the capabilities | a weight the file leaves out reads its default by the back door |
| Its writer adds missing names and never changes a line | to move the pinned brain is a lab report's decision |
| The scenario runner's base is a thread's setting (`with_base`) | the families and the gauntlet use the same runner and must keep the defaults |
| Ignored tests stay on the defaults | the censuses, families, ladders and recorders measure the brain as it ships |
| `weights.rs`'s tests stay on the defaults | they are about the defaults themselves |
| The lint reads a module item by item and follows helpers | splitting at `#[test]` folded every helper into the test before it |
| `Caps::BASELINE` counts as a default | the fleet promotes switches too (29 October) |
| `connect_idle`'s default line removed | it recorded LR 77; the dial has been the fleet's to move since |

## III. Method

Two worktrees, main at 43f9f67 and main with the change. Each move is
written by `scripts/set-defaults.py` as a promotion writes it, then
`cargo test -p treadcore-brain --release --lib` runs. The moves: those of
all 13 failed promotions (ten distinct) and a stress move. The stress move shifts every
searchable weight by a fifth of its range, from its default toward the
farther end (109 weights).

## IV. Results

| The defaults moved to | main, 43f9f67 | with the pinned brain |
| --- | --- | --- |
| today's (no move) | 165 pass | 166 pass |
| r0177 (and r0176) | the hop test fails | 166 pass |
| r0171 | 165 pass | 166 pass |
| r0112 | `a_fleet_match_plays_out` fails | 166 pass |
| r0117 | the hop test fails | 166 pass |
| r0166, r0170 | the smelter test fails | 166 pass |
| r0071, r0072, r0114 (to r0116), r0125 | 165 pass | 166 pass |
| every searchable weight, a fifth of its range | 7 fail | 166 pass |

The lint, given the defaults back in the hop test and in the helper
`hears`, fails and names both, and the five tests that call `hears`.

## V. Discussion

A promotion can no longer fail on a test of how a part works. The
families and the gauntlet carry the whole judgement of the defaults, as
they were meant to. The cost is the thirteen scenario files at the top
of `scenarios/`. Under the stress move two of them failed (the dodge).
They now play the pinned brain, so a promotion that broke the dodge
would pass them; only a family or a gauntlet file would stop it. None
of the 13 failures of 1 to 4 November was one of these files.

## VI. What is not built

A reading of the top-level scenario files at the candidate's defaults,
as a gate with a floor.

## VII. Paul's decisions

Whether the top-level scenario files should also be read at the
candidate's defaults, as the gauntlet's files are.
