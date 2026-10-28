# Focus Fire or the Near Tank: a Scalar

**Check-in report CI-076.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion record: `LAB-REPORT-BRAIN.md` section 117.

---

## Abstract

Every strong candidate of the search traded focus fire for shooting the
near tank in range (LR 116 D).

- **The scalar.** By Paul's rule that choice is now a scalar,
  `focus_blend`. At 0 the tank shoots the near tank in range; at 1 the
  wounded one a teammate is on. 0.5, the default, plays as before, bit
  for bit, behind a Caps switch.
- **The test** holds the two ends.
- **A fault, found and closed.** Checking that the default plays as
  before found that a shared cargo target folder can hand a census
  another commit's binary. The census now plays the binary its machine
  built for that commit.

---

## I. Introduction

### A. Intent

Turn an either/or the census found into a weight the census can set,
without changing today's play.

### B. The prompt

Paul's answer to the question "What should the brain do?":

> Make it a scalar

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "Either/or is a scalar" (CLAUDE.md) | `focus_blend`, 0 near, 1 focus, plus the Caps switch FOCUS_BLEND | A new boolean |
| D2 | The default must play as before | Today's score kept as accumulated; the blend added as (2b − 1)(team − near), skipped at 0.5 | Rewriting the score as a weighted sum (its float order would change) |
| D3 | The test pinned the default's choice | It holds the ends, on today's weights and on the search's | Deleting it |
| D4 | The first comparison differed | Bisected, a part at a time, in a scratch worktree | Accepting a difference |
| D5 | Two worktrees in one CARGO_TARGET_DIR share a binary's name | `CENSUS_BIN`: census jobs copy their commit's prepared binary; a promotion builds in its own target | A target folder a commit (a full build each) |
| D6 | Old commits' rows feed the search | Defaults compared by number, weight by weight (a weight added at a neutral default keeps the history) | weights.rs's blob (any added weight would drop 1,153 rows) |

---

## III. Methods

1. **weights.rs:** `FocusBlend` in `weights!`; `FOCUS_BLEND 21` in
   `caps!`; `from_weights` (the blend at 0.5 when off).
2. **tactics.rs:** the `focus_blend` field, `Tactics::default()`,
   `rate()`, and the test.
3. **The determinism check:** `brain_census`, the defaults, 2 seeds of
   120 s on naval.map, `CENSUS_LEDGER=0`. The old binary was 0e257c8's
   census-bin on the Mac; the new, a fresh build of the tree. The
   comparison is the `team` lines.
4. **The bisection:** a scratch worktree at 49f2f3b with each part added
   in turn (the table line, the field and mapping, the Caps line, the
   scoring), each built and compared.
5. **census.sh:** `CENSUS_BIN`. **orrery `tools/plans/treadcore.py`:**
   `CENSUS_BIN=$C/census-bin` on every census; a promotion's
   `CARGO_TARGET_DIR=$W/target`; `same_defaults` by the table's numbers
   (`defaults_at`).

---

## IV. Results

| | |
| --- | --- |
| The brain's tests | 109 passed, on the tree and on the tree with gen62/best's defaults |
| Old binary against itself | identical |
| The tree without the change, a fresh target | identical to the old binary |
| Each part of the change, alone, and all of them (scratch worktree) | identical |
| The tree with the change, a fresh target | **identical to the old binary** |
| The first comparison's "new" binary | gen62/best's defaults: the scratch promotion worktree's build, under the tree's fingerprint |

---

## V. Discussion

`focus_blend` costs nothing at its default and gives the census a dial.
The search will read its half and double without being asked.

The fault matters more than the feature. A census is only as true as
the binary it names, and a shared build folder could have mislabelled
one silently. Nothing measured so far was touched: every commit the
fleet built had the same brain source. From this commit on, the brain
changes, and the census now plays exactly the binary its commit made.
