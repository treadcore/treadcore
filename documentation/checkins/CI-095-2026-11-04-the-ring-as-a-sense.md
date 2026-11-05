# The Ring as a Sense: Built, Proved Harmless, and Not Shown to Help

**Check-in report CI-095.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026, late.
Companion records: `LAB-REPORT-BRAIN.md` section 146; journal 0122.

---

## Abstract

A sense of the nearest live pillbox's ring was added to the nets'
senses and proved harmless to every shipped net. Nets imitating the rules
with it did no better on the two ring gauntlets than nets without it, and
worse on G7. By the rule written before the run it is not shipped; the
code is parked in a stash.

## I. Intent

Paul: "perhaps the sensor will indicate a ring, as it would indicate and
identify the foe."

## II. Decision log

| Decision | Cause |
| --- | --- |
| The block goes last, six numbers | every older net and patch widens with zeros and plays as it did |
| A pillbox "that fires on the tank" is a live enemy's or a neutral one | a neutral pillbox fires on everyone |
| Judged by imitation, five runs a side | one run of each differed by more than the effect sought |
| The no-ring nets' six input columns zeroed after training | weights on an input that never moved stay at their random start |
| Not shipped, parked in a stash | the rule was not met |

## III. Method

The gauntlet for the rules and every shipped net, with the block and at
LR 145's commit without it. `RECORD_COURSE` on G7, G8, G18 and G19, 2,000
seeds from 9,100,000; the same file with the block zeroed; `treadcore-bc`
with `BC_DEVICE=flex`, 30 epochs, batches of 512, rate 0.001; each net on
the gauntlet's 200 seeds.

## IV. Results

Identical to the seed for every shipped net. Imitation, five runs a side,
passed of 200: G7 42.1% with the ring, 52.0% without; G8 85.2% and 83.3%;
G18 100% and 100%; G19 40.7% and 44.5%, each run anywhere from 0 to 172.

## V. Discussion

The nets' failure on G18 was a lack of experience: imitating the rules
where the choice comes up teaches it without the new sense. G19 is the
open one, and its spread is the training's, not the senses'.

## VI. What is not built

Several training starts with the best kept, for G19.

## VII. Paul's decisions

Whether to drop the parked block or keep it for a later trial.
