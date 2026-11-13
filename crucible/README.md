# crucible

A self-learning loop that never leaves the GPU. Tanks with sensors duel in
an arena. A population of small nets is scored, sorted, kept and mutated,
and its champions are kept in a hall of fame on the card. The host only
launches kernels and reads two numbers a generation.

The grove fleet's Pascal cards run it:

| Card | Tank-steps per second |
| --- | --- |
| GTX 1080 Ti | 581 M |
| GTX 1080 | 462 M |
| GTX 1070 Ti | 419 M |

The design, the sizing for an 8 GB card, the measurements and how it
scales: `docs/LAB-REPORT-GPU.md`.

```
make                                   # CUDA 12.x, sm_61 (CUDA 13 drops Pascal)
./crucible --pop 2048 --eps 128 --gens 400 --yard 10 --out runs/a.tsv --save a.state
./crucible --pop 2048 --eps 128 --gens 400 --load a.state --save b.state      # train on
./crucible --pop 2048 --gens 4 --bench                                        # throughput only
```

Options:

| Option | What it sets | Default |
| --- | --- | --- |
| `--pop` | individuals | 8192 |
| `--eps` | duels per individual a generation, at most 256 | 256 |
| `--steps` | steps per duel | 600 (1,200 with `WORLD=1`) |
| `--sigma` | mutation size | 0.03 |
| `--elite` | individuals kept unchanged | 16 |
| `--every` | generations between champions joining the hall | 5 |
| `--cap` | size of the hall | 4096 |
| `--spar` | share of games against scripted sparring bots | 0.5 |
| `--yard` | generations between yardstick reads (4,096 duels) | 10 |
| `--export` | file for the final champion's weights | none |
| `--import` | other islands' champions, into the hall first | none |
| `--migrate` | champions put into the population itself, in its last places: parents if they rank | none |
| `--ladder` | milestones, oldest first: opponents and a harder yardstick | none |
| `--lad` | share of games against a milestone | 0.2 |
| `--exam` | file for the final champion's score against each milestone | none |
| `--exam-eps` | duels against each milestone in the exam | 2048 |
| `--draw` | a drawn duel's worth in training (win +2, loss -2) | -1 |

**The ladder.** Once an island beats the scripted yardstick nearly every
time, the yardstick measures nothing. The fleet keeps milestones instead:
champions that each beat the one before. A champion becomes the next
milestone only when its exam was against the newest one and it won by
more than two standard errors (wins less losses), without losing to the
ladder on balance. Scores are wins less losses, from -1 to 1. Wins plus
half the draws would give a net that never fights 0.5, since a duel it
hides from is a draw. The coordinator, orrery's planner, keeps every
segment's champion and its line in `lineage.tsv`.

**The world.** `make WORLD=1` plays treadcore's rules instead of the
arena's own (Paul, 3 November: "the rules of this world, can we apply them
to the simulation expand the matrix dimensions. to the crucible"). One
arena unit is a tile of 25 m. The tank goes 4 tiles a second, reached at 4
and shed at 6 a second, and its hull turns 3 radians a second, easing off
for a corner and pivoting for a turn right round as treadcore's driver
does. It has 100 of armour, 24 shells, a gun of 10 tiles at 40 tiles a
second for 45 a hit with a 4-second reload, and eyes of 10 tiles. Four
neutral pillboxes stand in the middle band. Each shoots the nearest tank it
can see within 8 tiles for 15, every 2 seconds, and two shells kill it.
Under the world a duel is 1,200 steps, two minutes. Roads, water, the
indirect gun, drones and the economy are not in the arena; the forest and
walls are, with `FOREST=1`.

| Build | Inputs | Outputs | Weights at H=32 | At H=48 |
| --- | ---: | ---: | ---: | ---: |
| `make` | 32 | 3 | 2,211 | 4,083 |
| `make HYBRID=1` | 35 | 4 | 2,340 | 4,276 |
| `make WORLD=1` | 52 | 3 | 2,851 | 5,043 |
| `make WORLD=1 HYBRID=1` | 55 | 4 | 2,980 | 5,236 |
| `make WORLD=1 HYBRID=1 FOREST=1 NAV=1` | 64 | 6 | | 5,766 |

The twenty world senses come after the old ones: a ray each for
pillboxes, the nearest pillbox's bearing, nearness, readiness, health and
whether it has the tank in its sights, and the tank's own shells and
reload. `gens.tsv` gains four columns after its twelve: the pillbox damage
the learner took a duel in hits, the pillboxes it destroyed, the shells it
fired, and the share of those that struck.

**Widening.** A state or a champion of a smaller net loads into a larger
build. Every new weight starts at zero, so the new inputs reach nothing and
the new hidden units add nothing, and the old net plays exactly as it did.
Mutation then wakes the new weights. A hybrid island's state goes on as
`make WORLD=1 HYBRID=1 H=48` with `--load`, and its generations are kept. A
larger net never loads into a smaller build. A world build saves its state
as v3, which records the shape. The other builds still save v2, which the
program before this one can read.

**The forest and walls.** `make WORLD=1 FOREST=1` adds treadcore's forest:
six groves that slow a tank, stop a flat shot and hide a tank deep in them.
It adds walls too, three bars in a training arena and up to twelve in a
gauntlet file. A wall takes the shells that stop on it, and at 90 gone (two
hits) a gap of three tiles opens where they struck. Treadcore's walls fall
the same way since its game 9: two flat shots aimed at a wall tile open a gap
two tiles wide, for a tank a tile and a half wide where crucible's is two.
A pillbox shot to rubble is taken by the tank that drives onto it. A forest
arena's fitness adds the game's score: a kill, a capture, ground held.

**The gauntlet and its tasks.** `--gauntlet DIR` plays treadcore's gauntlet
files (their words: tanks, pillboxes, terrain, brains, expectations) on the
card. Each file is played `--gauntlet-runs` times on its own dice; with two
champions the runs are paired (McNemar's count of runs only one passed).
`--tasks DIR --task-p P` gives a share P of training's warps one of those
files as a task instead of a duel, the same task for every net in a
generation, paid by passing it and by its progress toward its goal.

**The maze.** `make ... FOREST=1 NAV=1` gives the net nine more inputs and
two more outputs:

- a route sense: the bearing and length of the way to its goal, by a
  breadth-first wave over a 32 by 32 grid made only when the line of sight
  is shut, and whether the goal is walled off;
- three arcs of its own motion over 1.5 s;
- two numbers it writes and reads back at the next step.

Under `HYBRID=1` its rules run an errand along the route, as treadcore's
Commander does: round a wall to an enemy, to a pillbox's gun range to shell
it, onto rubble or a point (`ERRAND=0` builds the old script). The gauntlet
files N0 to N3 ask for the route alone (`ends_near`), and M0 to M3, B1 and
the takeovers T1 to T3 for more.

**The camp.** A gauntlet or task file may be a chain of `[[stage]]`s, each
one of three goals: reach a point, take a pillbox, or destroy one. A pillbox
marked `dormant` rises only when the stage that spawns it begins. A stage
pays its reward, twice it when done at once, falling to the reward itself
at its `quick` seconds. The camps are K1 (a beacon, then two castles), K2
(corridors with many ways, two beacons, a castle) and K3 (a corner, then a
walled castle).

**The last chance.** A camp file's `last_chance = p` gives the learner a
second life: killed before the last stage, it rises once, whole, where that
stage began, with its memory kept and the castles left as they stand, and
every stage it does from then pays `p` of what it would have. The three
camps say 0.5. A pass is still every stage done on the first life, so the
gauntlet asks the same question with or without it; the last chance is what
training learns from a fall, where before a run that died taught nothing
past the stage it died in.

**Traces.** `--trace OUT --champions C` writes duels step by step for
orrery's montage. `--trace-scen FILE` writes runs of a gauntlet file instead,
for its maze films and footage. A camp's castle is written where it will
rise, and as not risen (-999) until it does.

Paul Richeson, 2026. MIT.
