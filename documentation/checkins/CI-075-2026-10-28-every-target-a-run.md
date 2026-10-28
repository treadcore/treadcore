# Every Make Target a Run of the Fleet

**Check-in report CI-075.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` sections 114, 115 and 116;
orrery's `tools/plans/treadcore.py`, `tools/fleet-queue.py`,
`src/training.rs`, `src/sessions.rs`.

---

## Abstract

This report covers steps 3 to 6 of the move from make to orrery.

- **Runs.** Each of the 13 training and measurement targets is now a
  run of orrery's fleet queue, asked for by a button or by
  `fleet-queue.py Q new KIND`.
  - Censuses are played a machine each.
  - spsa and tune advance a step or a generation at a time, by their
    scripts' rules.
  - The learning harness runs as one job.
  - Recordings are delivered to the store, hashed.
- **The test.** A tiny run of eight kinds worked end to end.
- **The Training view** is in four pages (Fleet, Runs, Found, Net), with
  the newest change in one line and Found Improvement leaderboards, after
  Paul's word on the first version.
- **Sessions** puts a resumable tmux session a machine in a grid, in
  6×10.
- **Promotion** is prepared on a branch, tested, its families read, and
  reaches main only on Paul's Approve.
- **The Makefile** has no training.

The first promotion stopped at a test. Every strong candidate trades
focus fire for shooting the near tank in range. That is Paul's decision.

---

## I. Introduction

### A. Intent

Finish moving training to orrery:
- every target a run;
- every activity a button;
- results brought into treadcore by a promotion Paul approves;
- the Makefile cleared.

### B. The prompts

> write an orrery user guide. this fleet organization. take pictures. show the concept of training treadcore, show samples of maps. show tables of example signal input, inner matrix and output, we can show that our contribed training of programming an AI manually in functions can accelerate training, although biased to perform like the function, we can then have a working generational set of coefficients for the neural net. we want to show our goal in associate with this specific training of tank pathing and fighting. by using a best practice time shart scripted AI, to be the metric which the neural net ai is judged. In general any sort of AI inteligence does not appear until it has lots of test data. Our test data is the simulation running in treadcore. We adjust the scalars that need adjusting in the scripted brainz ai . but we clearly need faster testing to get generational improvements in the net. Describe a way to keep track o making sure all matrix and variables are potential canidates for refinement. Also. Perhaps in the training mode showing us the transform information. Can you highlight, or allow the mouse to show more detail on hover of pixle on grid. Also a section, bar maybe (slim) . to show the newest coefficient to change. Stream them like tickers and marquee them show the variable name and modified value and its offset and which comptuer win the calculation. in fact a leader board of number of sucessfull scalar modified in the net or in the weights , a learder board of counts of the computer that submitted the results. "Found Improvment" . The user guide will be long, in html. It describes the projects of treadcore and how we need a CUDA and a CPU cluster to make good time. I expect it to be very long, IEEE master thesis. Describe writing a shell clustering program to destribute the values of the matrix to refine during neural net training, and training weights of rewards and its custom coded brinz. give convergence appromixations, considering we have our current run rate. annotate your uses. make sure all activities you ahve area available from an interface button, show user guide diagram of each button and when to use it. If it applies to our treadcore purpose, or if its just part of general orchesetration. Also, it would be nice to launch a text subshell perhaps starting with copal-tm . We ca run the subshells in a grid. Would be nice if we can have resumable sessions in tmux or screen. This changes the rust application layout, please consider UI and the progress of a user adding and stopping tasks. For example Orrery will have more types of training tasks, treadcore is just our first template. As we find more tasks, we will then make it more generic, right now consider the user guide and flow of technical explanation and live screen like a bunch of screen and sessions of copal. (also in tiny fony so they fit, like pt8).

> the scrolling marquee is too much. we need a way to show new items. less space, perahps just show the latest. take a look at the screen we ahve some text overflow. lets try to get your text to fit. add more screens or dropdown for information not neseaary all the time. Also the progress bars, I'm a bit confused what they do, are they CPU meters? Unsure.

The user guide is the step after the fleet management test.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "all orchestration coded in orrery"; the science stays treadcore's | Treadcore lists the censuses (`LIST=1`); orrery decides where they run | Copying the parts and switches lists into orrery |
| D2 | A census is one machine's | A run's censuses are a job each, on whichever machine is free; never split | Splitting one census's seeds across machines |
| D3 | spsa and tune results must read as before | Their rules carried over; tune's children side by side (recorded as the one difference) | A new optimiser |
| D4 | A run asked for should not wait behind the standing search | Priority for a run's jobs; a top-up as soon as a run is asked | First come, first served |
| D5 | A crash mid top-up left a run "running" with no jobs (the dry test) | Run state written only when the top-up succeeds | — |
| D6 | Runs waited for a new commit's builds | Runs advance without them; only new jobs wait | — |
| D7 | "every activity ... from an interface button" | A button a kind on the Runs page; Cancel, Approve, Sync now, Shut the fleet down | Command-line only |
| D8 | "the scrolling marquee is too much ... text to fit ... more screens" | One line, the newest change; four pages; checked by review frames at 1280 × 760 | A smaller font in the same crowded layout |
| D9 | "are they CPU meters?" | Bars labelled CPU and GPU with what they measure; the queue bar keyed (jobs done, running, queued) | Removing the bars |
| D10 | Full-screen programs need a terminal emulator | tmux draws (`capture-pane`), orrery shows and types (`send-keys`) | A VT100 emulator in Rust |
| D11 | "tiny font ... like pt8" | 6×10 from misc-fixed on a Copal node, padded to the table's 20 rows | Scaling 8×13 down |
| D12 | "Prepare, you approve" | Branch, tests, families, commit with the evidence, pushed as the branch; Approve pushes it to main as a fast-forward | Committing to main |
| D13 | Only the Mac can push | Promotion runs on the Mac (`host:mac`) | Any machine |
| D14 | The first promotion failed `default_weights_are_todays_constants` | `set-defaults.py` writes `Tactics::default()` too | Deleting the test |
| D15 | Every strong candidate fails the focus-fire test | Nothing promoted; the choice put to Paul | Changing the test without asking |
| D16 | "Remove the training from make" | The 13 targets and their help gone; the scripts stay, their usage lines name them | Deleting the scripts (one machine without the fleet still uses census.sh) |

---

## III. Methods

1. **treadcore:** `LIST=1` in `scripts/parts.sh`, `switches.sh` and
   `roundrobin.sh`; `scripts/set-defaults.py`; the Makefile; CENSUS.md
   sections 2, 4 and 4a; PROCESS.md.
2. **orrery `tools/plans/treadcore.py`:**
   - the runs: `run_jobs`, `census_run`, `spsa_run`, `tune_run`,
     `harness_run`, `record_run`, `promote_run`;
   - `approve`, `evidence` and `strongest`;
   - the ticker: `tick` and `ticker_scan`.
3. **orrery `tools/fleet-queue.py`:** `new`, `runs`, `cancel` and
   `approve`; priority; planning when `runs/` changes.
4. **orrery `src/`:**
   - `training.rs`: the pages, `draw_latest`, the matrix hover;
   - `sessions.rs`;
   - `font.rs`: F6X10;
   - `nav.rs`: Sessions;
   - `main.rs`: `--queue`, `--frame-view sessions`, `ORRERY_FRAME_PAGE`.
5. **The test runs:** r0001 to r0008 as in LR 114 C; the promotion
   r0009; the scratch worktree for the candidates' unit tests.

---

## IV. Results

| | |
| --- | --- |
| Run kinds tested | census, parts, switches, spsa, tune, gauntlet, record, roundrobin: all ran (LR 114 C) |
| Record | 2,812,816 bytes to calber, sha256 checked at both ends |
| orrery tests | 337 passed |
| Review frames | every Training page fits at 1280 × 760; Sessions shows six machines in 6×10 |
| Promotion r0009 | stopped at `default_weights_are_todays_constants` (fixed), then `focus_fire_picks_the_wounded_tank_a_teammate_is_on` |
| Candidates, held out | grove/w0003 +2.65; gen62/best +2.54; gen64/best +2.36; the best with `w_near` 1.0, gen58/best-less-w_near, +2.08 |
| Makefile | `make census`: "No rule to make target" |

**Errors:**
- The test runs were 1 to 4 seeds. Their numbers show that the machinery
  works, not facts about the brain.
- The held-out means are of two reads of 400 seeds, each with about
  ±0.28.

---

## V. Discussion

The fleet can now be driven from orrery alone:
- the standing search;
- every kind of run, by a button;
- the families;
- gathering, and shutting down;
- promotion.

The first promotion found what the census cannot. The search's best
weights change a behaviour the brain was built to have. The census, the
families and the unit tests each catch a different thing, and here the
unit test caught it. What happens to focus fire is Paul's choice.
