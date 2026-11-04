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
Under the world a duel is 1,200 steps, two minutes. Forest, roads, water,
the indirect gun, drones and the economy are not in the arena.

| Build | Inputs | Outputs | Weights at H=32 | At H=48 |
| --- | ---: | ---: | ---: | ---: |
| `make` | 32 | 3 | 2,211 | 4,083 |
| `make HYBRID=1` | 35 | 4 | 2,340 | 4,276 |
| `make WORLD=1` | 52 | 3 | 2,851 | 5,043 |
| `make WORLD=1 HYBRID=1` | 55 | 4 | 2,980 | 5,236 |

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

Paul Richeson, 2026. MIT.
