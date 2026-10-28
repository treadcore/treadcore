# Behaviour Cloning on a GPU, and Whether the GPU Was Faster

**Check-in report CI-068.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 27 and 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 106;
`LEARNING-DESIGN.md` VIII and X; orrery's `docs/cuda-node-lab-report.md`
and `docs/pascal-versions-lab-report.md`.

---

## Abstract

The scripted brain's choices were recorded through the learned tank's own
senses and masks (2,490,248 samples from 60 matches) and cloned into the
learned tank's net with Burn on calber's GTX 1080 Ti. On held-out matches
the clone, `nets/bc1.bin`, agrees with the scripted brain far above the
commonest-label baseline (drive 65.5% against 36.5%, gun 64.2% against
37.9%), and Rust's `Net::forward` reproduces the trainer's figures
exactly. In the census on columbia it plays worse than the net before it,
`nets/map7.bin`: -2.85 +/- 0.44 paired over 110 seeds. Rule part 1
passed; rule part 2 failed. Timed part by part, the GPU is not faster than
the CPU for this net: 6.4 ms a step against 5.6 ms, with the card's
arithmetic about half a percent in use.

---

## I. Introduction

### A. Intent

Start the learned tank's gradient training (backlog 70, upgrade 7) with
behaviour cloning, judge it by a rule written before the run, and answer
with measured time whether the GPU is the faster place to train.

### B. The prompts

> start the burn training on calber.

> start the behaviour cloning on calber.

> keep track of all this progress if the lab-report we want a time based analysis is this gpu really faster, what improvemnts to we need for me effieienct processing.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | AI-BRAINZ-PLAN 8: cloning before PPO | Clone the scripted brain into the existing net shape (143-64-64-47), so NetBrain, the census and the gauntlet judge it unchanged | A new shape: every reading would need new code first |
| D2 | A scripted tank follows a route set seconds before and need not say `Move` again | The drive label from the tank's state (its next waypoint's sector, or stop) | From its commands: most samples would read "no command" |
| D3 | A first recording showed a direct shot possible in 0.17% of samples | Record NetBrain's own masks and train a masked softmax | No masks: the gun would learn always to hold |
| D4 | The census machines must never build Burn | `tools/bc` its own Cargo project (`[workspace]`) | A crate in the workspace: Burn and CUDA in every census build |
| D5 | Held-out samples must be other matches, not other ticks of the same match | Split by seed: the last tenth held out | A random split: neighbouring ticks leak between the halves |
| D6 | The rule before the run | Written in LR 106 C: agreement above the commonest label; then bc1 above map7 in a census on one machine, outside two errors | A rule after seeing the agreement |
| D7 | CENSUS.md 5: a census is one machine's | Both censuses on columbia, one binary; paired by seed | bc1 on columbia against map7's saskatchewan reading |
| D8 | Parity, LEARNING-DESIGN VII | `AGREE=`: the written net run by Rust on the held-out samples | Trust the export: the transpose is where it would go wrong |
| D9 | Paul asked for time, not impressions | Time each part (read, first step, batches, steps) on CUDA and on Burn's CPU backend, the same trainer | GPU utilisation from `nvidia-smi`: a first reading (1%, "42 s an epoch") was taken during the load and was wrong |

---

## III. Methods

LR 106 A to F. Recording: `RECORD=` on calber, seeds 9,000,000 to
9,000,059, 600 s, 8 threads. Training: `tools/bc` (Burn 0.22.1), 12 epochs,
batch 4,096, Adam 1e-3, `BC_DEVICE=cuda` and `flex`. Agreement: the
trainer's held-out figures and `AGREE=` in Rust. Census: `CENSUS_NET=`,
110 seeds from 500,000, every map, 900 s, respawns 3, on columbia at
`742037d`; the pairing from each job's per-seed line
(`brain_census: difference ... a seed: ...`).

---

## IV. Results

| | |
| --- | --- |
| Recording | 2,490,248 samples in 97 s, 1.46 GB |
| Held out, drive / gun / mortar / drone | 65.5% / 64.2% / 89.3% / 96.7% (baselines 36.5 / 37.9 / 87.7 / 94.0) |
| Parity check 1 | exact |
| Census, bc1 / map7 | -23.18 +/- 0.49 / -20.33 +/- 0.27 |
| bc1 less map7, paired | **-2.85 +/- 0.44**; ahead on 29 seeds of 110 |
| A settled step, CUDA / CPU | 6.4 ms / 5.6 ms |
| Warm-up on CUDA | 3.1 s and 23.2 s on two runs |

Errors: the first epoch-time claim (42 s an epoch) was wrong and was
corrected in `742037d`; the `#[test]` attribute was once placed on the
wrong function and moved back before the commit.

---

## V. Discussion

Rule part 1 passed and part 2 failed, the case the rule named in advance:
imitation measured a decision at a time does not carry into a match,
where a third of the decisions being someone else's compounds along a
route. The next step is Paul's to choose (LR 106 F): PPO from the clone,
a look at whether the senses can carry the drive label at all, or DAgger.

The GPU is waiting, not computing: a net of 15,936 multiply-adds a sample
leaves the card's time to kernel launches, uploads and a wait for the loss
every step. LR 106 E lists the changes in order and writes the rule for
the next timing: under 1.0 s a settled CUDA epoch at batch 32,768 with the
data on the card, and faster than the CPU on all its cores, or BC stays on
the CPU until PPO.
