# The Proving Ground: A Map of the Maze and the Takeover, and the Series G30 to G33

**Check-in report CI-100.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, afternoon.
Companion records: `LAB-REPORT-BRAIN.md` section 151; journal 0128.

---

## Abstract

The maze and takeover tests became places on one map. The proving
ground is an island with the S, the @ and the 5 as walled places, A
Flame War Modified's walled city, and a gateless compound to breach,
reached by a ring road that crosses a river on bridges, with eight
bases round the coast. The rules take each place's pillbox from its
door: the S, the @ and the 5 every time, the breach 95.5% of the time.

## I. Intent

Paul: "build a map, with the test setup gauntlet of a the series of
tests. A series of spiral mazes, S shaped bases. and roads. in fact,
/Users/user/Downloads/Birdman_s\ map\ tiles.map
/Users/user/Downloads/A\ Flame\ War\ Modified.map  look at the map
times, for ways to stitch togeather componets like blocks to make rivers
and roads styled with consistent tiles along with your own map
creativity. Also the map A Flame War Modified has a good walled section.
Add bases around the island if they are missing, but I think its all
good."

## II. Decision log

| Decision | Cause |
| --- | --- |
| Both files converted by priorart2map.py, read as data | they are binary prior-art maps from outside |
| Roads one tile wide, rivers two, bridges as wide as the river | the tile sheet's blocks, whose edges agree |
| The walled city copied as it is | Paul named it the good walled section |
| A gateless compound beside it | the breach test needs a wall that must come down |
| Map pillboxes and bases cleared in G30 to G33 | one decision a file, as the gauntlet's |
| Flame War's bases kept as they are | eleven on the island; Paul: "I think its all good" |

## III. Method

`python3 -I scripts/provingmap.py maps/proving.map`; the gauntlet read
by the rules at 200 runs as in CI-099; the loader checks the map on
every run.

## IV. Results

| File | The rules, 200 runs | Floor |
| --- | --- | --- |
| G30 the S | 1.000 | 0.95 |
| G31 the @ | 1.000 | 0.95 |
| G32 the 5 | 1.000 | 0.95 |
| G33 the breach | 0.955 | 0.90 |

## V. Discussion

On the proving ground the 5 is easy for the rules, where on the open
field (G24) it was 66.5%: the place's corridors are wider and its road
leads the way. The breach is the rules' mortar: aimed at the pillbox
inside, its splash brings the wall down, and the tank drives in.

## VI. What is not built

Crucible's card cannot load a map file; its twins of these tests are
its own (M0 to M3, B1).

## VII. Paul's decisions

None asked yet.
