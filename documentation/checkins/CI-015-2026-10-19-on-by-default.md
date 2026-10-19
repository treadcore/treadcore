# The Landfall's Three on by Default, and the Reverse Reading

**Check-in report CI-015.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0064; `LAB-REPORT-BRAIN.md` sections 20 and
38; CI-013, CI-014.

---

## Abstract

On Paul's choice, `landfall_memory` 30, `landfall_shy` 1 and
`landfall_wait` 10 are the defaults. As proof, the census was run in
reverse: a team on the old defaults among two on the new. It read
-0.08 ± 0.45 a seat, where the forward +2.31 ± 0.58 would predict about
-2.3. The gain is relative to opponents that play the old way. What
the weights do when every team has them is unmeasured: the census
reads a team against the others. The defaults stay; backlog 38 takes
an absolute reading.

**Index terms:** promotion, relative evaluation, interaction effects.

---

## I. Introduction

### A. Intent

To set the defaults Paul chose and prove the switch.

### B. The prompt

> yes all three on. commit. push. continue backlog

The answer to: "I recommend turning all three on ... Which do you
want?"

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Paul: "yes all three on" | Defaults 30, 1, 10 in weights.rs | |
| D2 | A plain RuleBot has no Settings | It keeps 0, as with the dodge's constants | Changing `RuleBot::new`: the baseline bot would change too |
| D3 | A promotion needs a proof | The reverse census: old as trial, new as baselines | A null run: it reads 0 whatever the defaults |
| D4 | F: reverse -0.08 ± 0.45 | Report it with the commit; the defaults stay as Paul chose; backlog 38 | Reverting unasked: the forward gain is real, and the choice is Paul's |
| D5 | "look about brain overview" (CI-004) | LR 20 kept current: 90 weights, the landfall in the diagram, the defaults | |

---

## III. Methods

The new defaults dumped (`CENSUS_DUMP`), with the three set to 0 as the
trial's `CENSUS_WEIGHTS`; the baselines on the defaults. 48 seeds in
three ranges, naval.map, 900 s, respawns 3, the fair census.

---

## IV. Results

| | Trial | Baselines | d a seat |
| --- | --- | --- | --- |
| Forward (CI-013) | three on | old | +2.31 ± 0.58 |
| Reverse | old | three on | -0.08 ± 0.45 |

Tests: brain 73 passed and 8 ignored; client 31; net 14; proto 3; sim
50; no warnings.

---

## V. Discussion

A relative census answers "does a team with this beat teams without
it?", and here the answer is yes. It does not answer "is the game
better played when everyone has it?" For a default, which every bot
takes, the second question is the one that matters, and needs a
reading that is not relative. Every earlier promotion, the capped
castle included, was read in one direction only.

**Next:** backlog 38.
