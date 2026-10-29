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

Paul Richeson, 2026. MIT.
