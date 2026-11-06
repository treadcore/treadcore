# The Intrusion Dial: w_breach, a Wall Shelled by Choice, the Press Temper Moving It

**Check-in report CI-102.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, evening.
Companion records: `LAB-REPORT-BRAIN.md` section 153; journal 0130.

---

## Abstract

The rules breached a wall only by shelling a pillbox behind it. A dial
now lets a storming tank break into a base by choice: when the way in
is shut, or longer than `w_breach` times the straight line, it shells
the first wall on that line and goes through. The press row of the
temperament matrix moves the bar. Off by default.

## I. Intent

Paul: "We need to ammend the brainz coding to consider intrusion given its weights (and maybe temperment of bio)."

Asked which treadcore change to build, Paul chose: "nav_senses for the
Gladiator, w_breach intrusion dial".

## II. Decision log

| Decision | Cause |
| --- | --- |
| Only in a storm, with no gun left to shell | the guns come first; a stormed base is the target |
| Only a wall | a splash brings a wall down; concrete cracks, water stays |
| The bar is a ratio of the way round to the straight line | a long detour is worth a shell; a near gate is not |
| The press temper scales it | "maybe temperment of bio"; press_odds is the bar it already moves |
| 0 by default | the brain as it was until a census promotes a value |

## III. Method

`cargo test -p treadcore-brain`; the pinned brain written again by
`write_the_pinned_brain` to name the new dial.

## IV. Results

| Check | Result |
| --- | --- |
| treadcore-brain tests | 176 passed (two new) |
| A shut base | breached at the first wall on the line |
| A gate on the line | driven through |
| A gate round the far side | breached at 1.5, driven round at 4 |

## V. Discussion

What the dial is worth is the census's to say: a storm begins after
long contact with an enemy, which no single scenario gives.

## VI. What is not built

A breach to a target other than a stormed base, and a breach of
concrete (two shells).

## VII. Paul's decisions

Build the dial (with nav_senses); each with its own LR.
