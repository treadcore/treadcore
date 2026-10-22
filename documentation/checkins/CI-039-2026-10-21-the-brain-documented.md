# The Brain Documented

**Check-in report CI-039.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0096; `LAB-REPORT-BRAIN.md` section 70;
CI-001 A9; roadmap backlog 26.

---

## Abstract

CI-001 A9 found a third of the brain's functions undocumented; LR 60
took it to 99. This step finishes it. Of the 100 left, 29 were
methods of a trait's impl, which the trait documents; the brain map
now lists them as the trait's. The other 71 got doc comments, each
read against its code. The reading found the `Brain` trait's own two
required methods undocumented and `run_bots`'s doc on the wrong
function. `make brainmap`: 0 of 403 without a doc comment. Comments
only; every test passes as before.

**Index terms:** documentation, brain map, rustdoc, trait methods.

---

## I. Introduction

### A. Intent

After CI-038 closed the tuning lead, the next step in goal 4 that
gives a sure result: backlog 26's last part, a brain whose every
function says what it does.

### B. The prompt

> continue. upward and onward. My nobile campanion.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-038: the census cannot see small leads; tuning wants a sharper measure first | A step with a certain end: backlog 26 | Backlog 22 (6 h of census) or 25 (two maps): more noise-bound work straight after a null |
| D2 | 29 of 100 were trait impl methods (`Brain`, `Default`, `Drop`...) | The map lists them as the trait's and counts them apart | 29 comments repeating the trait's: rustdoc already shows the trait's |
| D3 | F: a test's `impl Brain` was credited to `Drop` | The scan reads an impl's trait by its indent | Leave it: a wrong row in a map is a wrong map |
| D4 | Each doc is a claim | Read each against its code; `rookie`, `defend_within`, `posture` corrected after a first draft | Docs from the names alone |
| D5 | F: `run_bots`'s doc on `retire_taken` | Moved to its function | — |
| D6 | Comments only | `make test` and `make families` as the proof that nothing moved | A census: nothing to measure |

---

## III. Methods

`scripts/brainmap.py`'s `scan` listed every function without a doc;
each impl's trait was read from the source. For each function the
code was read (and its callers where the doc names a use) and a
comment written in the file's own manner: a sentence or two, the
weights and LRs named where they apply. `make brainmap` counts them.

---

## IV. Results

| Measure | Before | After |
| --- | --- | --- |
| Functions without a doc comment | 100 | 0 |
| Trait methods, counted apart | (counted as undocumented) | 29 |
| Functions | 403 | 403 |

Tests: brain 86 passed and 12 ignored; client 32; net 14; proto 3; sim
50. The families as before.

---

## V. Discussion

1. **The trait was the gap.** The two methods every brain implements
   were the two the trait left unsaid.
2. **A doc read against its code is a small review.** Three first
   drafts said less than, or other than, the code did.
3. **Backlog 26 is done.**

**For Paul:** none blocking. Commits are local until you say push.
