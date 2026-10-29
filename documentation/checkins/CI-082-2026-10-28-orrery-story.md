# orrery's Story: Five Chapters and a Projects View

**Check-in report CI-082.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion record: `LAB-REPORT-BRAIN.md` section 123; orrery `src/nav.rs`,
`src/projects.rs`, `src/training.rs`, `src/commands.rs`.

---

## Abstract

orrery's views are now five chapters in the order the work goes:
Machines, Projects, Queue, Results, Watch. Each chapter's pages are
metal keys in a steel column, with the chapter's sentence at its foot.

Training is split into Queue (what runs) and Results (what it found).
A new Projects view shows treadcore and crucible side by side, and
which machine runs which, from the queue's status and crucible's run
files.

---

## I. Introduction

### A. Intent

Make the interface's order tell the story Paul asked for. Give the
fleet's second project a place in it.

### B. The prompts

> Can we redesign all tabs all sub tabs and rethink about the interface progression. tell a story, help us orchestrate and add comptuers to configure in the orchestration.

> and incorporate orrery and crucible into its second job. we want to complement and use our fleet to run to improve the canidates and allow fleet processing of the queue in the grove.

Paul's choice, asked with three previews: "Build → Run → Result".

---

## II. Decision Log

| # | Cause | Decision | Not chosen |
| --- | --- | --- | --- |
| D1 | The order changes Paul's daily path | Asked, with three previews | Choosing alone |
| D2 | Paul: Build, Run, Result | Machines, Projects, Queue, Results, Watch | Projects first; a sixth tab |
| D3 | Pages were scattered (a row in Training, Lab's rail) | A steel column of page keys on every view; Lab's rail carries them at its top | A second header row |
| D4 | "tell a story" | Each chapter's sentence cut into the column's foot; the chapters numbered | Help text |
| D5 | Training mixed controls and findings | Queue (band and controls) and Results (findings) | One view |
| D6 | The window opened on the Lab | It opens on the Queue, what is happening now | Chapter 1 |
| D7 | crucible is not yet queued | Projects says "ready, not queued" on the GPU machines | A lamp it has not earned |
| D8 | Real data only | treadcore from the queue's status; crucible from `runs/*.tsv` | Figures typed into the page |
| D9 | A seam halfway down the steel | treadcore's rim blended down the sides | Leaving it |

---

## III. Methods

1. **`nav.rs`:**
   - `View` (Machines, Media, Projects, Queue, Results, Lab, Sessions);
   - `Chapter` with `views()` and `story()`;
   - `column` and `Key`;
   - `header` returning the body, a view and a page key;
   - chords by chapter.
2. **`training.rs`:**
   - `draw(.., view)`;
   - `QUEUE_PAGES`, `RESULTS_PAGES` and `PAGES`;
   - `results_headline`;
   - the page row replaced by the column.
3. **`projects.rs`:** `Crucible` (its runs read every 15 s),
   `parse_run`, the two cards and the matrix.
4. **`lab.rs`:** the rail as wide as the column, with the Watch keys at
   its top.
5. **`main.rs`:** routing, and `--frame-view projects|queue|results`.
   **`commands.rs`:** the View menu by chapter.
6. **`draw.rs`:** the rim blended.
7. **Checks:** 348 tests; frames of all six pages; installed and
   relaunched.

---

## IV. Results

| | |
| --- | --- |
| Tests | 348 passed (4 new: the story fits, a view in one chapter, a run file read, a card's name) |
| Projects, as drawn | treadcore: 74 queued, 7 running, 721 done, 1 failed; 37 candidates, 0 current. crucible: three runs, fastest 478 M tank-steps a second in training (581 M in the bare benchmark) |
| Orrery.app | 398 MB after relaunch |

**Errors.** The column's sentences were clipped at 132 pixels. The
column is now 150 pixels, and a test holds each line to it.

---

## V. Discussion

The window now reads left to right as the work goes. The Projects page
is where a second job shows up. It also shows, honestly, that crucible
is built but not yet scheduled.

**Next:** that scheduling. crucible as a run kind on the GPU machines,
islands exchanging champions, sharing the cards with treadcore's net
runs.
