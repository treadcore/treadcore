# The Boundary as a Test: Gauntlets G18 to G22

**Check-in report CI-090.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 141; CHANGELOG; ROADMAP
backlog 83; journal 0117; `scenarios/gauntlet/g18` to `g22`.

---

## Abstract

Five single-decision tests mark the boundary of smart play around
pillboxes and attackers: shoot the hostile pillbox and spare the neutral
one; take two pillboxes one at a time from outside their rings; answer
the tank that shot you before a nearer one that did not; kite an
attacker when alone; stand and fight it with friends near. No brain
meets them on 50 seeds, the rules included (24, 0, 48, 34, 56%).

## I. Intent

Paul, word for word:

> we need to setup a goal where it would be smart to attack some
> pillboxes while avoiding others. like enemy preference , last attacked
> enemy we would kite and dance, unless we have backup with fleet allies
> nearby. we should solve it by creating a coded set of tests that
> should result into that being the boudray of your question.

## II. Decision log

| Decision | Cause |
| --- | --- |
| Gauntlet files, not a new harness | the gauntlet is the single-decision test already, read for every brain with a floor |
| Hostile: shoot; neutral: spare | the sim: a live hostile pillbox must be shot dead before it is taken, a live neutral one is taken whole and fires on whoever enters its ring |
| Every pillbox test passable from outside the rings | a gun of ten against a ring of eight; a test the game cannot pass is not a boundary |
| G20's attacker fires first | the tank's gun reloads its first two seconds |
| G21 and G22 as a pair | "kite and dance, unless we have backup": the same fight alone and with friends |
| G22's friends hold their ground | escorting friends made staying with them their choice, not the tank's (an untrained net passed 44%) |
| G22 asks the tank to outscore the attacker | two hits were most of a kill friends share (the rules 14%) |
| Floors above every brain | the floor is the boundary; a brain earns it |

## III. Method

`scenario.rs`: `pillbox_struck` (`PillboxDamaged` on a placed pillbox),
`first_hit` (the first tank a seat hit), `pillboxes_standing`. Five
files. The gauntlet on this Mac, 50 seeds from 9,500,000, the rules,
the untrained net, the six milestones, bc2 and logistics
(`GAUNTLET_NETS`); each file played by the rules on 20 seeds for the
failures in words (`make scenario`). 164 brain tests.

## IV. Results

| | the rules | untrained | map7 | map5 | bc2 | logistics |
| --- | --- | --- | --- | --- | --- | --- |
| G18 (floor 0.8) | 24% | 0% | 0% | 0% | 0% | 0% |
| G19 (0.7) | 0% | 0% | 6% | 0% | 0% | 0% |
| G20 (0.7) | 48% | 0% | 8% | 4% | 0% | 2% |
| G21 (0.6) | 34% | 2% | 0% | 26% | 0% | 6% |
| G22 (0.7) | 56% | 30% | 20% | 40% | 16% | 8% |

The rules' failures, in the harness's words: the neutral pillbox struck
for 55 to 65 and 45% of the armour gone (G18); "1 placed pillboxes stand
at the end" and 55% armour (G19); "seat 0 hit seat 1 first, seat 2
expected" (G20); dead, or hit as often as hitting (G21).

## V. Discussion

The rules play these decisions about half as well as they should, and
the nets worse: the boundary is real and nobody is at it. G22 cannot
tell a passive tank beside its friends from an active one as well as
the others tell theirs apart (an untrained net 30%), and is read with
G21: a brain that passes both has learned when to kite and when to
stand.

## VI. What is not built

The styles (Duelist, Gladiator) in the gauntlet; the rules taught the
five; crucible's arena given a hostile and a neutral pillbox.

## VII. Paul's decisions

Whether to commit and push this; which of the next three comes first.
