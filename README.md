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
| `--steps` | steps per duel | 600 |
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

Paul Richeson, 2026. MIT.
