# A Wall Takes the Gun: Two Direct Hits Open a Gap, in the Game and on the Card

**Check-in report CI-103.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, night.
Companion records: `LAB-REPORT-BRAIN.md` section 154; journal 0131.

---

## Abstract

Walls fell to the mortar in the game and to the gun on the card. Now
both: a direct shot aimed at a wall takes 45 of a tile's 90 and its
neighbour's, so two hits open a gap a tank can drive, and crucible's
walls open the same way. Game 9.

## I. Intent

Paul: "continue unifiying the motor and the types of direct fire to take down a wall. Treadcore's rules should match crucible. Crucible is made for Cude GPU equivalanet to train and compare with existing gladdiators and ai brinz."

Asked which way to unify the two games' walls, Paul chose: "Treadcore
takes direct fire".

## II. Decision log

| Decision | Cause |
| --- | --- |
| Only a shot aimed at the wall is taken | shots at tanks behind walls keep their refusal; the brain's fire unchanged |
| The tile and its neighbour along the wall | two hits must open a way a tank can drive |
| Damage kept per tile, forgotten when it changes | a rebuilt wall starts whole |
| Concrete not hurt by the gun | it takes the mortar to crack it |
| Crucible's gap 3 tiles | its tank is 2 wide where this one is 1.5 |
| GAME_VERSION 9 | a rule change: game 8's census must not count |

## III. Method

`cargo test --workspace`; crucible's four builds compiled and the CUDA
gauntlet read again under the rule.

## IV. Results

See LAB-REPORT-BRAIN.md section 154 for the tests; the card's gauntlet
under the rule is recorded in orrery's report Round the Corner.

## V. Discussion

Crucible's shells stop on any wall they meet; treadcore's gun is
refused unless aimed at one. Crucible's tank fires along its hull with
no aim point, so the card cannot ask what a shot was aimed at.

## VI. What is not built

The mortar in crucible (its nets have no output for it).

## VII. Paul's decisions

Treadcore takes direct fire; crucible's walls all breakable.
