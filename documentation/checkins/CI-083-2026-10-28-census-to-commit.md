# Census to Commit: the Loop, Micro Versions, and crucible's Islands

**Check-in report CI-083.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 124; the lab report
`reports/census-to-commit/` (shared page
https://claude.ai/artifact/Q3UkM5y7rhpw8AyWse6KrD); orrery 6878a26
(crucible), 7ead828, d959a10, 3f9d773, a0f854e, 0a9ae4b; the tag
`brain-r0019`.

---

## Abstract

The path from a discovered coefficient to the brain in a fresh clone is
drawn with real commits and times. It is completed:
- **Micro versions:** each promotion is tagged `brain-RID`.
- **Do it now:** a button that syncs, promotes and pushes.
- **Re-reads:** older candidates are read again on today's defaults, so
  the loop does not stop at "0 current".

crucible became grove's second job: a GPU lane per card, the three
cards as islands, from a button on Projects.

On Results, the candidates are filtered and scrolled, and the Net page
draws every sense and score as a bar, with an explanation on hover.

---

## I. Introduction

### A. Intent

- Show how the brains are stored and how a change reaches main.
- Make the loop complete and visible.
- Put the GPUs to work.

### B. The prompts

> continue. improve . i want to see a button to add more gpu CUDA to the mix, add crucible. ALso. describe in a lab report, with graphics, and tutorial of . Create a learning loop and describe how the coefficients get commited back to treadcore, so the it picks it up in the AI. For example, show me commit versions, and timestamps. Actaully, show me hos the brainz and the net data are stored, show me the matrix for the AI. show me its composition, draw diagrams. we want to see the process of discovering a change, and promoting it, so push it on to the next download of treadcore. essentially creating micro versions of this git submission. its part of the program. it has a do it now button to sync and push the values, of course we have two templates of projects now, but its really just treadcore. . write lab report. impliment . also describe the results tab. why do we have failed tasks in family. why are some older, when it says promoted. describe the layout. of what happends. perhaps add a scrollbar and a filter by type. [image] . in the scroll bars show each of them. for example on teh Results Canidates, Net. we se the scripted brains chaoices, unseen matches, can we show each variable with a bar. we seem to have a small sampling, expand this for all values. provide more details on the hover. explain how it transforms, where it relates to in the other images.

---

## II. Decision Log

| # | Cause | Decision | Not chosen |
| --- | --- | --- | --- |
| D1 | One lane per host; crucible on calber's lane would stop its censuses | A second lane per GPU machine, `NAME-cuda`, caps `cuda` | Sharing the CPU lane |
| D2 | "keep its generations within itself", across cards | Islands: state on the machine, champions exchanged through the coordinator | One population moved between cards |
| D3 | The keeper plans only when its queue is low; a restart would requeue a three-hour school run | Six segments queued ahead, chained by `after` | Restarting the keeper |
| D4 | "micro versions of this git submission" | An annotated `brain-RID` tag on each approved promotion, pushed | A v* tag, which starts the release workflow |
| D5 | "a do it now button" | Sync, then promote the strongest current candidate, or re-read the older ones; two presses | One press that pushes |
| D6 | 0 current candidates: nothing could promote | Auto mode re-reads the three strongest older ones on each new set of defaults | Promoting on stale evidence |
| D7 | "why do we have failed tasks in family" | The failure names its step; an older failure marked superseded | The message "the tests or a family failed" |
| D8 | "a scrollbar and a filter by type" | `ui.scroll_list`; filter keys by standing with counts | Paging |
| D9 | "each variable with a bar ... all values ... explain how it transforms" | Every sense (143) and score (47) a bar; hover names it, says how it is transformed, marks its matrix line | More heads in the accuracy block (all five were already there) |

---

## III. Methods

1. **crucible:** `--export`, `--import`; the state keeps the hall's next
   place; built and tested on all three nodes.
2. **The planner (`treadcore.py`):**
   - `crucible_jobs` and `crucible_view`;
   - `reread_older`;
   - `now`, `micro_tag`, `promote_failure`;
   - the commands `now`, `tag`, `why`.
3. **`fleet-queue.py now`; `machines.py cuda-lanes`.**
4. **orrery:**
   - Projects: the button and the islands;
   - Results: Do it now, filter, scroll, superseded;
   - Net: the bars and hover;
   - `ui.scroll_list`;
   - menu Fleet → Do It Now (armed).
5. **Run:**
   - `cuda-lanes add all` gave calber-cuda, whitewolf-cuda and
     boburner-cuda;
   - `tag r0019`;
   - `why r0009`;
   - a sync, so the keeper planned.
6. **The report:** read from git, the queue's run files, `nets/bc2.bin`
   and `weights.rs`; screenshots from `--frame-view`.

---

## IV. Results

| | |
| --- | --- |
| Tests | 348 pass |
| crucible | first segments 50 generations in 21 s on calber; three islands running, six segments queued each |
| Re-reads | w0045, w0003, w0034 (control) queued at 19:11 |
| Tag | brain-r0019 → dc3df01, signed, on GitHub; no workflow started |
| r0009 | "the brain's tests failed (weights::tests::default_weights_are_todays_constants)" |

**Errors.**
- A sync under `timeout` (absent on macOS) never ran.
- w0034 was re-read against itself; promoted candidates are now left
  out.
- The tag for r0019 came five hours after its promotion.

---

## V. Discussion

The loop has no manual step left. Its weak point was the shelf life of
evidence, and the re-reads address that. crucible keeps the cards busy
but does not yet help treadcore's candidates. That needs a GPU arena of
treadcore's own rules, which is Paul's choice to make.
