# The First Auto Promotion, the Rotation, and a Pass That Failed

**Check-in report CI-078.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion record: `LAB-REPORT-BRAIN.md` section 119; orrery
`tools/plans/treadcore.py`, `tools/fleet-queue.py`, `src/training.rs`,
`docs/fleet-train.md`.

---

## Abstract

Auto mode promoted its first candidate. grove/w0034 passed the brain's
109 tests and all nine families, and is main as dc3df01, its seven
defaults built next on every machine. The pass that pushed it failed
just after. It left an unsaved state and two runs, which were fixed and
made impossible. A rotation now asks one run of each kind in turn, in
two lanes, with fresh seeds. The Runs page describes each button as it
works in the cycle.

---

## I. Introduction

### A. Intent

See the first promotion through, give auto mode a default share of
every kind of run, and make the Runs page fit the cycle.

### B. The prompt

> we need to update the buttons and the concepts for "Ask for a run" do all these still apply. Make sure they apply to our current context and the full process. Is there a do one of each on rotation method that is the default choice? Can we have that.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "do one of each on rotation" | Two lanes (brain, net), a run active in each, one of each kind in turn | One lane (train and school take days and would stall it) |
| D2 | tune replayed seeds 0 every run; spsa 1000 | `seed0` from 500,000,000, a million a turn | Repeating runs on the same seeds |
| D3 | parts, switches, roundrobin, gauntlet say nothing new on the same defaults | Once per set of defaults | Every turn |
| D4 | A census of `@best` measured and went nowhere | Passing, adopted with its held-out reads | — |
| D5 | Rotation runs at high priority would crowd out the standing search | Normal priority; asked runs first | — |
| D6 | The approving pass failed on a `Tactics::default()` line | `changed_vars()` reads `name old -> new` only | — |
| D7 | The failed pass left two runs and unsaved counters | New runs written by `keep()`, with the state; counters set to match | Deleting r0020, r0021 (they are valid rotation runs) |
| D8 | A prepared promotion waited behind a full queue | A plan after every sync; every 5 min from the next keeper start | Restarting the keeper (it requeues running jobs) |

---

## III. Methods

1. **orrery `tools/plans/treadcore.py`:**
   - `rotate()`, `ROTATION`, `ONCE_PER_DEFAULTS`;
   - `tune_run` takes `seed0`;
   - `adopt_runs` takes a passing census;
   - `run_jobs` keeps rotation runs at normal priority;
   - `changed_vars()`;
   - `new_run` deferred to `keep()`;
   - `sync` ends with a file made and removed in `runs/`.
2. **orrery `tools/fleet-queue.py`:** `rotation on|off`; status carries
   `rotation`; the keeper plans every `plan_s`.
3. **orrery `src/training.rs`:** the Runs page rewritten (the rotation
   and its switch, the buttons' descriptions, r/a markers).
4. **Checks:**
   - the planner's passes in `events.jsonl` (two failed at 14:24, one
     clean at 14:28);
   - r0019's log;
   - `changed_vars` on the failing line;
   - a machine's `defaults.toml` parsed to the promoted names.

---

## IV. Results

| | |
| --- | --- |
| r0019 tests | 109 passed, 0 failed (66 s with the families) |
| r0019 families | all nine hold (table in LR 119 A) |
| main | dc3df01: w0034's seven defaults |
| Builds | dc3df01 queued on all six machines; confirmation pending |
| Rotation | r0020 spsa (seed0 500,000,000), r0021 train |
| Failed passes | two, 14:24:14 and 14:24:31; one clean after the fix |

**Errors.** w0034's held-out reads are two reads of 400 seeds against
the old defaults. The promotion's families read nine scenario families
of 400 to 1,000 runs each.

---

## V. Discussion

The bar held, and the promotion is a large one: seven defaults, two of
them doubled. That is what the search had been pointing at since LR
116, and the first promotion stopped at the test LR 117 made into a
scalar.

The failed pass is the cost of a planner that pushes. Its push could
not be undone by the failure, and its state was never saved, and it
left runs a correct pass would not have. Both are fixed. The rule from
it: a pass writes nothing outside its state until it keeps it, the
push being the one deliberate exception.
