# The Gladiator Reads a Maze Net: nav_senses, the Two Kept Numbers, and the Rules Round a Wall

**Check-in report CI-101.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, evening.
Companion records: `LAB-REPORT-BRAIN.md` section 152; journal 0129.

---

## Abstract

Crucible's maze build gives a GPU net nine inputs more and two outputs
it reads back. The game's Gladiator now reads such a net: the way to
its enemy by the map's own route, the walled flag, three arcs of its
own motion and the two kept numbers, with the rules' answer turned
round a wall as the Commander turns. A maze champion widened from a
world one answers as it did; the trained one plays as the Gladiator.

## I. Intent

Paul: "How would we give a sensor to the AI net, and Brinz such that it
has enough information to pass the maze."

Asked which treadcore change to build, Paul chose: "nav_senses for the
Gladiator, w_breach intrusion dial".

## II. Decision log

| Decision | Cause |
| --- | --- |
| The way by `route_kept`, not a grid | the rules' own planner; the game has the map |
| A clear line is the way itself | crucible's `route_from` does the same |
| An empty route reads walled | the map's planner says so; crucible's wave gives 255 |
| The rules' answer turns round a wall | crucible's errand; the Commander routes there |
| The kept numbers cleared when no enemy is in reach | each of crucible's runs begins at nothing |
| A four-output champion's last two are nothing | one `think` for every champion |

## III. Method

`cargo test -p treadcore-brain`; a census of eight seeds of 300 s with
`GLADIATOR=` the trained maze champion and `bot_style = 17`.

## IV. Results

| Check | Result |
| --- | --- |
| treadcore-brain tests | 174 passed (six new) |
| Widened champion's four outputs | equal to the world one's within 1e-5 |
| The maze champion as the Gladiator, 8 seeds | plays every game; -2.61 +/- 1.05 on this Mac's one map |

## V. Discussion

The score is not a read: eight seeds on a Mac's single map. The read is
orrery's bridge, 192 seeds on every map, once the maze home is on the
fleet.

## VI. What is not built

The Gladiator's net has its say only in a duel; the mazes away from an
enemy remain the Commander's. The kept numbers move at each decision,
five a second, where crucible's move every step.

## VII. Paul's decisions

Build nav_senses and the w_breach dial, each with its own LR; keep the
GPU's trade cap and train on; cut crucible's maze build's cost before it
goes to the fleet.
