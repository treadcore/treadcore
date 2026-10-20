# The Census Binary Frozen, and `smooth` as a Weight

**Check-in report CI-021.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journals 0073 and 0074; `LAB-REPORT-BRAIN.md`
sections 20, 45 and 46; CI-020.

---

## Abstract

Two items from CI-020's conversation. First, the census scripts now
run a frozen copy of the test binary, so brain code can change while a
census plays. This was proved by breaking the source mid-run, and the
run finished with an identical reading. Second, on Paul's choice, the
capability `smooth` is now a weight, `w_smooth`, with an insignificant
default of 0.05. Its numeric parts blend by the weight; its yes-or-no
parts act above 0.5. The old play against the new default reads +0.47
± 0.60 a seat over 48 seeds.

**Index terms:** tooling, reproducibility, weights, either-or rule.

---

## I. Introduction

### A. Intent

Backlog 41 and 40, set at CI-020's close.

### B. The prompts

> is there an improved and faster way for you to test ?

> ok. maybe not remove it. give it an insignificant coefficient

> commit. push. continue backlog

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Each census rebuilt the brain | `census_bin`: build once, copy the binary beside the logs | A second target directory: the copy also records which binary made a result |
| D2 | No new dependencies | Parse cargo's JSON with python3 | jq |
| D3 | Proof of D1 | Break the source mid-run; compare with a known reading | Trusting it |
| D4 | The proof's wait loop matched itself | Stopped; the source restored from a copy, the tree checked clean | |
| D5 | Paul: "an insignificant coefficient" | `w_smooth` 0.05 | 0: indistinguishable from removal |
| D6 | The stance's smoothing at small shares is another rule | Thresholded with the counts | Blending it: a small share would change play more, not less |
| D7 | Old files and `make switches` name `smooth` | Read as the weight at 1 or 0 | Breaking them |
| D8 | Proof of D5 | The old play as trial against the default, 48 seeds | |

---

## III. Methods

`census_bin DIR` (scripts/common.sh). The `smooth` change in weights.rs
(`SmoothShare`, `smooth`, `smooth_alpha`, `smooth_on`, the parser),
commander.rs (`set_settings`, `go`, `needs_home`, `auto`) and fleet.rs
(the census's capability reader). Census: `w_smooth = 0` as the trial
weights, the defaults for the baselines, 48 seeds in three ranges,
naval.map, 900 s, respawns 3, the fair census, from
`build/lr46/census-bin`.

---

## IV. Results

Frozen binary: the broken-tree run read d −0.4567 ± 0.4054, as LR 40's
first plumbing run. `w_smooth` 0 against 0.05: +0.47 ± 0.60. Tests:
brain 74 passed and 8 ignored; client 31; net 14; proto 3; sim 50; no
warnings.

---

## V. Discussion

Paul's rule turned a removal into a dial: the six behaviours stay,
reachable by one number, at a setting that does not show in play. The
frozen binary let this step's documentation be written while its
census ran, which is what Paul's question was after.

**Next:** backlog 39.
