# The Maze and the Takeover as Tests Both Machines Read: Walls in a Scenario, and G23 to G28

**Check-in report CI-099.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, morning.
Companion records: `LAB-REPORT-BRAIN.md` section 150; journal 0127;
orrery's lab report *One Rule, Two Machines*.

---

## Abstract

The fleet's two machines now read the same single-decision tests for
navigation and takeover. A scenario's terrain may be a wall, and six
gauntlet files put a maze shaped S, 5 or @ between a tank and a pillbox
it must take, or a pillbox and a base to take or to build. The rules,
on 200 runs each, pass the S 99.5%, the 5 66.5%, the @ 100%, take a
pillbox 99.5%, take a defended base 3.5% and build a base 88.0%; their
floors are today's success less two errors. Crucible plays the same
files' words on the GPU.

## I. Intent

Paul: "Specifically the goal is to naviate a simple maze, Like a maze of
S shaped and 5 Shaped, and @ (ampersat at-sign) shaped mazes on test
gauntlet maps of player vs environment, where we add takeover takss like
Pillbox capture, Base capture, Building a base capture. A series of
rewards need to be placed into the GPU and CPU such they can simulate
the same score, or at least correlate their fitness." And: "Code the
implimentation of more tests, coded in Cuda, for GPU and CPU." Then,
asked what to do with the work: "Commit and push both."

## II. Decision log

| Decision | Cause |
| --- | --- |
| A wall as a terrain kind, nothing else in the loader | the gauntlet's files already lay rectangles of terrain; a maze is rectangles of wall |
| The mazes take a pillbox at their end | the net's senses point at the nearest pillbox, so a GPU net can be asked the same task; the rules navigate by the map |
| `pillbox_owned_by` as the pass | treadcore's word for a takeover; crucible reads it as its own capture |
| Floors at today's success less two errors at 200 runs, G25 at 0.95 | LR 141's rule; a 100% share has no error, and a floor of 1.00 fails a promotion on one run |
| The same coordinates in both gauntlets | so a share on the card and a share on the CPU are shares of the same maze |

## III. Method

`GAUNTLET=1 GAUNTLET_BRAINS=scripted GAUNTLET_RUNS=200 cargo test -p
treadcore-brain --release --lib -- --ignored learn::harness::train`, on
the Mac, seeds 9,500,000 onward as every gauntlet read; the brain's unit
tests, `cargo test -p treadcore-brain --release --lib`.

## IV. Results

| File | The rules, 200 runs | Floor |
| --- | --- | --- |
| G23 the S maze | 0.995 | 0.95 |
| G24 the 5 maze | 0.665 | 0.55 |
| G25 the @ maze | 1.000 | 0.95 |
| G26 take a pillbox | 0.995 | 0.95 |
| G27 take the base | 0.035 | 0.00 |
| G28 build a base | 0.880 | 0.80 |

The brain's 168 tests pass; the whole gauntlet, the six included, reads
in 1.8 s at 20 runs.

## V. Discussion

The 5 is the hard maze for the rules: its bowl turns back on itself,
and a third of the runs end short of the pillbox. A defended base is
nearly never taken: the enemy's tank and pillbox together outgun one
tank, and G27's floor is zero until the rules learn it. On the card the
GPU's nets passed none of the mazes on their first day, and one of them
built a base 56 times in 256 after two segments of training on the
tasks.

## VI. What is not built

The mazes are open fields with walls, not the fleet's maps; the
takeover's squad and carrying are the game's own and not tested here.

## VII. Paul's decisions

Commit and push (6 November).
