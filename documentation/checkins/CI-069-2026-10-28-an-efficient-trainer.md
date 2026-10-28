# An Efficient Trainer: the GPU Five to Eight Times Faster than the CPU

**Check-in report CI-069.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 107 (and 106 E);
`LEARNING-DESIGN.md` VIII and X.

---

## Abstract

LR 106 E found that the behaviour-cloning trainer ran no faster on the GTX
1080 Ti than on the CPU, the card waiting on kernel launches, uploads and
a read of the loss every step. Three changes were made: the recording loaded
onto the card once and batches gathered there; every head's loss in one
masked log-softmax; the loss read once an epoch. At the old settings the
held-out agreement is unchanged (drive 65.5%, gun 64.2%), and a settled
epoch fell from 4.3 s to 0.815 s; at batch 32,768 it is 0.643 s, against
5.0 s for the same trainer on the CPU. The rule written before the runs
passed.

---

## I. Introduction

### A. Intent

Do the list LR 106 E wrote, and judge it by the rule written there.

### B. The prompt

Paul's message was the list, pasted from the previous answer:

> To make it efficient, in order:
>   1. Much bigger batches.
>   2. Read the loss once per epoch instead of every step.
>   3. Load the recording onto the card once.
>   4. Combine the per-head loss calculations.
>   5. Let the CPU version use all 8 cores (it used fewer than 2).

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Item 3; 2.49 million samples, 11 GB card | Senses, targets and masks on the device once (2.82 GB); batches by `select` | Prefetching CPU-built batches on a thread: still an upload a step |
| D2 | Item 4 | Targets and masks laid out [N, 7 x 10]; the 47 scores gathered into it; one log-softmax | Burn's fusion feature: a new build configuration, and the launches were mostly the seven heads' own |
| D3 | Item 2 | The detached loss summed on the device, read at the epoch's end | Reading every tenth step: still waits |
| D4 | Item 5; flex's source | Reported as having no switch: flex threads only matrix products above 192^3 operations | Patching Burn, or a second backend (CubeCL's CPU, LLVM): a build of its own for one item |
| D5 | The rule before the run | LR 106 E's rule restated in LR 107 B before any run, with an equivalence check of the rewrite | Judging the speed alone: a fast trainer that learns differently would pass |
| D6 | Two runs of the old trainer gave drive 65.5 and 65.3 | The equivalence check widened from two errors (0.2 points) to 0.5 points on drive, before any run | Two errors: would fail on the random start alone |
| D7 | Item 1 changes the steps eight-fold | Three learning rates tried at 32,768 (1e-3, 3e-3, 1e-2) | One rate: 1e-3 under-trains in 69 steps an epoch |
| D8 | `bc1.bin` was made at 4,096 and 1e-3 | Defaults unchanged; 32,768 at 1e-2 named as the fast setting | New defaults: a later run would not repeat LR 106 |

---

## III. Methods

LR 107 A. On calber, the recording of LR 106 D (`~/bc/rec60.bin`), 12
epochs each: `BC_DEVICE=cuda` and `flex`, batch 4,096 at 1e-3 and 32,768
at 1e-3, 3e-3 and 1e-2; flex under `/usr/bin/time`. A settled epoch is the
mean of epochs 2 to 12, each ending at its one read of the loss.

---

## IV. Results

| | Settled epoch | Drive held out |
| --- | --- | --- |
| Before, CUDA, 4,096 | 4.3 s | 65.3% / 65.5% |
| After, CUDA, 4,096 | 0.815 +/- 0.019 s | 65.5% |
| After, flex, 4,096 | 4.507 +/- 0.013 s | 65.4% |
| After, CUDA, 32,768, 1e-2 | **0.643 +/- 0.010 s** | **65.1%** |
| After, flex, 32,768, 1e-2 | 5.035 +/- 0.025 s | 64.9% |

The first CUDA epoch took 7.4 to 18.4 s (compiling and tuning). flex used
190 to 225% of a CPU. Equivalence check passed; rule passed.

---

## V. Discussion

The card's time had been spent waiting, and removing the waits is what
made it fast: most of the gain came at the old batch size. The larger batch
added a fifth more. The CPU backend got no faster from the same changes and
has no setting for its other cores, so the CPU's part in training is
several runs at once rather than one run on more cores. For behaviour
cloning and for PPO's update step the GPU is now the place to train.
