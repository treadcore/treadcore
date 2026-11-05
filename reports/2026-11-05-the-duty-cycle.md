# Ninety-Eight Percent Busy: Where the Fleet's Waiting Goes, a Round Table to See It, and a Background Job That Asks for Nothing

*Paul Richeson · Lab Report — IEEE Format · 5 November 2026* · posted from orrery (`docs/duty-cycle-lab-report.md`, https://github.com/vonglurt/orrery)

Copyright (c) 2026 Paul Richeson. MIT licensed, as orrery.

Companions: `tools/plans/treadcore.py` (the planner: `blame_view`, `handoffs`, `timeline_view`, `values_view`); `tools/fleet-queue.py` (the keeper: its low-water trigger and starved lanes); `src/pipeline.rs` (the Pipeline's sixth page, Blame); `tools/plans/line.json` (its six answers); `tools/plans/board-check.py` (the round table's checks); the queue's `events.jsonl` and `done/`, 29 October to 5 November. Report of 4 November: *Fifty-Four Idle Minutes* [1].

## Abstract

Paul looked at the Pipeline's four screens on the morning of 5 November and asked whether the fleet's waiting is a backlog of CUDA work that will clean up with time, or whether it will always be there, and, if it will, whether the waiting time can be put to use: "a constant low priority background job that can be worked on piecemeal". **The machines do not wait.** Over the last twelve hours the eight CPU lanes were busy 97.7% of the time (96.2% to 98.5% each) and the three GPU lanes 99.8%; no job anywhere waited on another machine. **The work waits instead,** in three places: the GPU islands' fifteen queued segments, a buffer the planner keeps on purpose (six segments ahead a card, one running and five queued) that will never clear and should not; the chains of the line, whose next step is asked only at the planner's next pass, five minutes apart, so the transform's thirty steps take 3.8 hours for 54 minutes of work, and a trial's path from first read to held-out verdict spends 15 of its 36 minutes between its own jobs; and the CPU lanes' last two percent, idle about a minute at a time, because the keeper's low-water trigger counts the GPU buffer and so never sees the CPU queue run dry. **The bottleneck is latency on a few chains, not capacity,** and each part has a remedy with an estimate: the low water counted by kind of lane (about two percent more CPU work); chains advanced when they return, not at the next pass (the transform from 3.8 to about 1.6 hours, a trial's path from 36 to about 25 minutes); the held-out read in four pooled pieces (about 17 minutes off each generation's path); and the background job Paul asked for, designed so that every piece is stamped, pooled only with its own kind, weighed by the generation it read, and depended on by nothing. A new Pipeline page, **Blame**, draws the machines round a table, the queue in the middle, and every wait as an arrow: today it shows no arrow between two machines, three loops of GPU segments waiting their turn, one run handed back to the planner, and every machine's help going back to the queue. Built and checked here: the page and its data. Designed and estimated, not built: the four improvements.

*Index Terms*: duty cycle, utilization, queueing, Little's law, polling server, critical path, work-conserving schedule, speculative execution, pooled estimates.

## I. The question

Paul, with four screenshots of the Pipeline (the Board, the Timeline, the Flow and the Queued list): "describe the hold up on waiting. is this caused by the backlog of cuda computations. this one run is going very long ... describe if we can add other steps to improve the utalization of all GPUs and CPUs by scheduling approprate intervals or search depth ... the question mainly is will this clean up with time. is it because we had an unnatural backlog, or will this waiting always be happening. If so, is there a way to use the time, the wait time, as a backup research for something." And then: "also create a blame tab ... the computers point at each other saying who they are waiting on. On their other hand, shows their backup help, where it can reach out back to the queue to assist in small jobs of improvement, piecemeal in returnable incriments."

Then, for this report: "If we can't improve the bottleneck, can we use the extra compute time on something usefull if we guess right. Like a constant low priory background job that can be worked on piecemeal. Each of these additions needs to allow discrepencies of to consolidate their start times and generation weights. We want to prevent creating circular depdencies. We want to analyze the overall work duty cycle."

Four questions, then: **how busy is the fleet** (the duty cycle); **what is the waiting** (and whether it clears); **can it be shortened** (the bottleneck); and **can the rest be used** (the background job), without double counting, without stale evidence weighing as fresh, and without a cycle in the queue's graph.

## II. Background: the fleet as a queueing system

The fleet is eleven lanes. Eight are CPU machines (saskatchewan, boburner, columbia, calber, whitewolf, macmini, jaguaribe, bosporus) that run treadcore's census: a **first read** of a trial (55 seeds, about 2.5 minutes), **read-ons** that add seeds until the trial is decided (27 to 42 seeds a step), **held-out reads** of 400 unseen seeds for a trial past the read-on bar, the **families** and the promotion's **tests**, the **line's proof** against main, the **transform's** SPSA steps, and the **builds** of each new commit. Three are GPU lanes (boburner-cuda, calber-cuda, whitewolf-cuda), one card each, that run crucible's **islands**: an evolving population that lives on its card, advanced a segment at a time (100 generations, about twelve minutes). The **keeper** (`fleet-queue.py`) hands each free lane the first queued job it can run. The **planner** (`treadcore.py`) decides what is queued, once a pass.

Four results from queueing theory frame what follows [2]-[5].

1. **The utilization law,** U = X·S: a lane's busy share is its throughput times the work a job takes. A schedule is *work-conserving* when no lane idles while a job it could run waits. At U near 1 there is nothing left to harvest; only the order of the work can change.
2. **Little's law,** L = λW: the jobs waiting (L) equal the rate they arrive (λ) times how long each waits (W), for any stable queue, whatever the order. A queue that holds the same length all day is not a backlog; it is a buffer, and its W is a choice.
3. **The polling server:** a server that looks at its inputs every P seconds makes an input that arrives at a random moment wait P/2 on average, plus the time the look takes. The planner is such a server, with P = `plan_s` = 300 s.
4. **The critical path:** a chain of N steps, each of which can begin only when the last has come back, takes N·(h + q + w): the handoff h to ask for the next step, its wait q in the queue, and its work w. Its floor is N·w, however many machines there are; this is Amdahl's law for a chain [5].

## III. Method

Everything is read from the queue's own records, nothing sampled by hand. A job's work is its span from `started` to `done`, `failed`, `stopped` or `requeued` in `events.jsonl`; a lane's busy share in a window is the sum of its spans clipped to the window. A start with no ending (one, left by the keeper that went down on 31 October) is not counted. A job's **asked** time is its queue file's name, the keeper's clock in milliseconds when the planner's job was added (a high-priority job's name is 10^11 ms earlier, so it sorts first; the method adds it back). A **handoff** in a chain is the minutes from one job's return to the next one's asking; its **queue wait** the minutes from asking to starting. Chains are the transform's steps (`rNNNN-kNN`, consecutive) and a trial's reads (its first read `wNNNN`, then the read-ons and held-out reads whose titles name it). Two jobs asked together (a held-out read's two halves) are not a handoff. Stages are the planner's own (`stage_of`). Windows: twelve hours, a day and a week to 10:00 on 5 November.

## IV. Results

### A. The duty cycle

*Table I. Busy share by day: the mean of the lanes that ran that day, and the lane-hours lost to jobs sent back, failed or stopped.*

| Day | CPU busy | CPU lanes | GPU busy | GPU lanes | Lost, lane-hours |
|---|---|---|---|---|---|
| Thu 29 Oct | 72.7% | 6 | 85.8% | 3 | 7.2 |
| Fri 30 Oct | 53.2% | 6 | 52.8% | 3 | 11.1 |
| Sat 31 Oct | 31.7% | 7 | 0.0% | 0 | 13.1 |
| Sun 1 Nov | 95.7% | 7 | 44.6% | 3 | 3.6 |
| Mon 2 Nov | 97.6% | 7 | 83.2% | 3 | 0.1 |
| Tue 3 Nov | 87.1% | 8 | 97.2% | 3 | 4.2 |
| Wed 4 Nov | 93.1% | 8 | 99.0% | 3 | 4.1 |
| Thu 5 Nov, to 10:00 | 97.6% | 8 | 99.8% | 3 | 0.0 |

**The week had an unnatural part, and it is over.** The 29th to the 31st were outages: machines going down, ssh failing, a fleet shut off for four hours on the 31st, and 31 lane-hours of work run again. From the 1st the CPU lanes run 87% to 98%, the dips being the evening of the 3rd (a promotion's worktree blocking its bundle) and the 54 idle minutes of the 4th [1]. Since the handover of 4 November [1] nothing has been sent back.

*Table II. The last twelve hours, a day and a week.*

| Window | CPU lane-hours busy | CPU busy | GPU busy | Lost, lane-hours |
|---|---|---|---|---|
| 12 h | 94 of 96 | **97.7%** (96.2 to 98.5 a lane) | **99.8%** | 0.0 |
| 24 h | 179 of 192 | 93.1% | 99.4% | 2.7 |
| 7 days | 936 of 1,344 | 69.6% | 65.8% | 43.4 |

*Table III. Where the CPU's busy time went, last twelve hours, by stage.*

| First read | Read-on | Held-out | Proof | Transform | Asked for | Build | Families | Tests |
|---|---|---|---|---|---|---|---|---|
| 46.1% | 37.9% | 7.8% | 3.1% | 2.9% | 1.7% | 0.3% | 0.1% | 0.1% |

**Five sixths of the CPU is the search: first reads and read-ons.** A day's jobs: 1,519 first reads, 1,505 read-ons, 378 GPU segments, 177 transform steps, 131 builds, 120 held-out reads, 49 runs asked for, 26 families, 24 proofs and 3 promotions. The funnel: 1,520 first reads; 652 trials read on (42.9%); 75 reach a held-out read (11.5% of those); three become generations. Twenty-seven generations landed in the week, a median 2.2 hours apart.

### B. What waits, and on what

![The Pipeline's Blame page: eleven machines round a table, the queue in the middle](https://vonglurt.github.io/orrery/docs/site/shots/pipe-blame.png)

*Fig. 1. The round table, live at 10:05 (the Pipeline's Blame page): the machines round the edge, the queue in the middle. A green arrow is a machine's help, the small reads it takes from the queue; the amber one is r0187 handed back to the planner; the numbered loops are segments waiting their turn on their own card. There is no arrow between two machines.*

Through the morning the queue held thirteen to eighteen jobs, and every wait was one of three kinds:

1. **A GPU segment waiting its turn on its own card:** fifteen, five an island. Crucible keeps `ahead: 6` segments asked for each island, one running and five queued, each after the one before it, because a population lives on its card and its next segment can run nowhere else. The planner tops them up as they go. By Little's law, L = 5 and λ = one segment every twelve minutes give W = 60 minutes for the last of them: **a buffer, by design, that will look exactly like this every day.** It covers a keeper restart or a slow planner pass with an hour of work in hand, and it does not touch a CPU lane.
2. **A build waiting its turn:** when main moves, every machine builds the new commit, a job a machine ahead of its others. Today this was the 07:46 gap: main moved at 07:42 and every CPU lane stood 2.5 to 3.3 minutes while the line was replayed and the builds ran (20 to 74 seconds each).
3. **A run handed back to the planner:** a chain whose last job came back and whose next is not asked yet. At 10:05 that was r0187, the transform, between its steps 16 and 17.

**No job waited on a job another machine was running.** The red arrow of Fig. 1 has nothing to draw.

![The Pipeline's Timeline: six hours of each machine, measured, and its forecast](https://vonglurt.github.io/orrery/docs/site/shots/pipe-timeline.png)

*Fig. 2. Six hours of each machine at 10:05 (the Pipeline's Timeline): every CPU row full of the search's first reads (blue) and read-ons (green), held-out reads (orange) and the proof (pink); the GPU rows one unbroken bar of island segments. The thin dark lines are the minute-long gaps of IV-D.*

### C. The chains: where the minutes go

*Table IV. The handoffs, medians of the day.*

| Chain | Handoffs | Planner (h) | Queue (q) | Work (w) | Waiting share |
|---|---|---|---|---|---|
| The transform's steps | 170 | 3.2 min | 2.6 min | 1.8 min | 76% |
| A trial's reads | 1,575 | 3.1 min | 1.1 min | 2.5 min | 63% |

**The planner's handoff is the polling server's P/2 plus its pass:** 2.5 minutes of waiting for the next pass and about 0.7 for the pass itself predicts 3.2; the measurement is 3.1 and 3.2. A trial's reads already go to the front of the queue (they are high priority), so their queue wait is about a minute; the transform's steps are the rotation's and are not, so theirs is 2.6.

**"This one run is going very long": the transform.** r0187, asked at 07:56, is thirty SPSA steps over the 942 numbers of a learned override, each step one census of 24 seeds on one lane. Its steps compute for 0.8 to 3.5 minutes and start about ten minutes apart (07:59, 08:10, 08:17, 08:28, ...). By the critical path, T = 30·(3.2 + 2.6 + 1.8) = 228 minutes, 3.8 hours; r0184 took 3.4 hours (04:26 to 07:52) and r0187 is on course for 3.9. **Three quarters of the transform's time is waiting between its own steps,** while every machine is busy with something else.

A trial's path, from its first read to its last held-out read, takes a median 36 minutes, of which 15 are spent between its own jobs (100 trials in the day reached a held-out read).

### D. The CPU's last two percent

In twelve hours each CPU lane was idle 8 to 27 minutes, in gaps of about a minute (24 gaps over a minute; one shared gap of three minutes at the 07:46 build). The cause is the keeper's trigger. It calls the planner when the queue holds fewer than `low_water` jobs (7), every `plan_s` (300 s), or when a lane has stood idle with nothing it can start for `starve_idle_s` (30 s) and then `starve_s` (60 s). **The queue's count includes the fifteen GPU segments, so it is never under seven,** and the CPU queue, two jobs a machine (`queue_per_machine`, which Paul set to 2), drains in about six minutes of three-minute reads, before the next five-minute pass. The lane that finds it empty waits for the starved path: about a minute.

## V. The round table (built)

The page Paul asked for is the Pipeline's sixth, **Blame**. The machines sit round the edge of the page as at a round table, CPU and GPU alike, each with what it runs now, its busy share over six hours and its **help**: the share of that time spent on the small returnable reads (first reads, read-ons, held-out pieces, the proof, the transform's steps, the GPU segments), which is what a free lane takes back from the queue when its own line has nothing for it. The queue and its planner are the table. Four marks:

- a **red arrow** from one machine to another: queued work dealt to the first that waits on a job the second is running, the only real blame;
- an **amber arrow** to the table: a run handed back to the planner, with the minutes it has waited;
- a **loop** beside a machine, with a count: jobs waiting their turn on it (amber when it is its build);
- a **green arrow** to the table: the machine's help, the hand it reaches back with.

A queued job belongs to the machine the Timeline's forecast deals it to, so the page and the Timeline cannot disagree; the planner writes `blame` into `plan/pipeline.json` at each pass (`blame_view`, a quarter of a second) and the page counts nothing. Two lines above the table answer the question before the picture does: the busy shares, and the handoffs of Table IV. `board-check.py` holds the four kinds of wait, the hand-back and the shares to a fixed queue; the six answers of its "About this screen" are in `line.json` with every other page's.

## VI. Improving the bottleneck: four changes, estimated

*Table V. Today and the estimate, each change alone and all four.*

| Measure | Today | A: low water by kind | B: chains advance on return | C: held-out in pieces | A to C together |
|---|---|---|---|---|---|
| CPU busy, 12 h | 97.7% | ~99.5% | 97.7% | 97.7% | ~99.5% |
| Transform run, 30 steps | 3.8 h | 3.8 h | ~1.6 h | 3.8 h | ~1.6 h |
| A trial, first read to held-out verdict | 36 min | 36 min | ~25 min | ~28 min | ~17 min |
| Held-out read, wall clock | ~12 min | ~12 min | ~12 min | ~3.5 min | ~3.5 min |
| Generations a day (search-limited) | 3.9 (27 in 7 days) | ~3.9 | ~4.0 | ~4.0 | ~4.2 |

**A. The low water counted by kind (keeper; a few lines; no risk).** Count only the queued jobs a CPU lane can run against the CPU lanes, and the GPU buffer against the GPU lanes, so a CPU queue running low calls the planner at once. *Theory:* a work-conserving schedule leaves no lane idle while work could be asked; the gaps of IV-D are the only CPU idle there is. *Estimate:* most of the 2.3% recovered, about 4 lane-hours and 100 first reads a day. *Cost:* at most a few more plan passes an hour.

**B. Chains advance when they return (keeper and planner; medium).** When a job of a chain comes back (a transform step, a read-on, a held-out read, a proof), the keeper asks the planner within about fifteen seconds instead of at the next pass, and a chain's next step goes ahead of new first reads. *Theory:* the polling server's P/2 becomes the debounce; the queue wait becomes about one job's remainder. *Estimate:* h from 3.2 to about 0.9 minutes, q from 2.6 to about 0.5: the transform 30·(0.9 + 0.5 + 1.8) = 96 minutes, 1.6 hours (2.4 times faster); a trial's path about 11 minutes shorter. Its floor is the work, 54 minutes for the transform. *Cost:* plan passes rise from twelve an hour to thirty or forty with a one-minute debounce, each about forty seconds on the Mac's one core; **triggering on the transform's steps and the held-out reads alone** (about ten an hour) takes most of the gain for a tenth of the passes, and is the version to build first.

**C. The held-out read in four pieces (planner; small).** A held-out read is one job of 400 seeds, about twelve minutes on one lane, and a passing trial needs two. Four jobs of 100 seeds on four lanes, pooled seed by seed, give the same estimate (the paired difference's mean and error over the same 400 seeds) in about 3.5 minutes. *Estimate:* about 17 minutes off each generation's path; the same CPU.

**D. Generations a day.** A generation is limited by the search's luck against the bar (held-out z 1.55 today, easing after four dry hours), not by the machines. The week's 27 generations came a mean 6.2 hours apart (the median, 2.2 hours, is short because they come in bursts); A to C take about 30 minutes of latency off each, and A adds about 2% more trials. **About 8% more generations,** 4.2 a day for 3.9, with the CPU used no differently: the gain is mostly in how soon a found improvement reaches the game, not in how many are found.

**E. The GPUs: nothing to take.** At 99.8% and with a population on each card, the islands have no slack to harvest and no other work their cards could do between segments. The look-ahead of six could be three without losing a minute (a 36-minute buffer still covers a restart), which would only shorten the Queued list.

## VII. The background job: using the time "if we guess right"

Paul's idea: a constant, lowest-priority job, worked piecemeal, that soaks up whatever time the line leaves. **At 97.7% busy there is little time left to soak up,** about 4.4 lane-hours a day, and A takes most of it. But a background job can also take a small fixed share from the first reads, the fleet's least valuable work at the margin, if what it computes is worth more. The design, so that it can be built safely:

**What it computes: held-out reads ahead of time.** A trial that has had three read-ons reaches a held-out read 28% of the time (74 of 269 in the day). A background piece reads 25 unseen seeds of such a trial's held-out set, the strongest by its read-on z first. **If we guess right,** the trial's held-out read is already part done when it is asked for, and the generation's path is shorter by the pieces in hand. **If we guess wrong,** nothing is lost but the CPU: the seeds are the trial's own held-out evidence, kept and pooled if it is ever asked for later. Each piece is returnable: it stands alone and adds to every other.

**Consolidating start times and generation weights.** Pieces start and end at any time, across promotions, so each is stamped and pooled only with its own kind:

1. **The stamp.** Every piece carries its question (the trial's weights, hashed), the defaults it played against (the commit's defaults, as `same_defaults` compares them), the `GAME_VERSION`, the `SCORE_VERSION`, the census binary's hash and its seed block. The stamp is taken when the piece is asked for, from the build it runs, never when it ends: a piece that starts under one generation and ends under the next belongs to the first.
2. **The seeds.** Each piece takes the next block of a trial's held-out range (60,000,000 up, 25 seeds a block, a counter in the planner's state), so no seed is counted twice and the foreground read takes the blocks after them.
3. **The pool.** Pieces pool with each other and with the foreground read only when their stamps match, by the seed: the paired difference's mean and standard error over every seed in hand, so order and start time do not matter.
4. **The generation weight.** A piece whose defaults are today's weighs fully; one whose defaults have moved weighs nothing toward today's question and is archived with its own (the same rule the proof's pool follows [6]). There is no partial weight: a trial read against other defaults is a different question.

**No circular dependencies.** A background piece's only dependency is its machine's build of its commit (`after_host`). No foreground job may name a background job in its `after`; the planner reads the background pool as data when it decides, which is a read, not a wait. A lint would prove it at every pass: the queue's `after` graph sorts topologically, and no edge enters a background job. Because a piece is small (about 2.5 minutes) and sorts last, a foreground job never waits more than one piece for a lane.

**Its budget.** One piece queued for each CPU lane at most, never more; or, once A has taken the idle, a fixed share of the CPU (5% is about 230 pieces a day) taken from the first reads. Whether it pays is measurable: the minutes it takes off each generation's path against the first reads it displaces (about 230 a day at 5%). **Build it after B and C, and as an experiment with a stated rule:** it stays if the median path from a trial's last read-on to its held-out verdict falls by more than two of its standard errors over a week.

Other background work considered and set aside: more seeds for the line's proof (the line reads +0.02 ± 0.08 against main over 5,600 seeds: a tie, which more seeds only confirm); the families read again (minutes a day); a drift sentinel reading main against the last generation (already the proof's job).

## VIII. Discussion

**Is the waiting caused by the backlog of CUDA computations?** No. The fifteen waiting GPU segments wait only for their own cards, which never stop, and no CPU work waits on a GPU. **Will it clean up with time?** The GPU list will look like this forever, by design; the CPU queue, two jobs a machine, is never long. **Was it an unnatural backlog?** The week before the 1st of November was; this week is steady. **Will waiting always happen?** Yes, and it should: a queue with nothing in it is a lane about to idle. What can change is not how full the machines are but how long the few chains that make a generation wait between their own steps.

The answer to "can we use the wait time" is therefore mostly about time, not machines: the machines' spare time is two percent, and the chains' spare time is three quarters of the transform and two fifths of a trial's path. A, B and C take that back; the background job is a way to spend a little capacity to buy a little more latency, safely, once the cheaper changes have been made.

## IX. What Paul decides

Which of A to C to build, and in what order (the recommendation: A, then B on the transform and the held-out reads only, then C); and whether to run the background job of VII as an experiment once they are in. The round table is built and live. Paul chose the same morning: "build A, then B" (section XI). C and the background job wait.

## X. Addendum: the Values page had stopped

Reading the pipeline document for this report found the **Values** page empty since 4 November: a value added that day (`size_warn_gb`, read by growth.py, not the planner) has no attribute name, and the planner asked for it by name, which failed and dropped the whole table from `pipeline.json`. It now reads such a value from `tuning.json` or its default, and all 37 values are back.

## XI. Addendum, the same morning: A and B built

Paul: "build A, then B. commit push.."

**A, in the keeper.** The low water counts only the queued jobs a CPU machine could take (`cpu_queued`); the GPU islands' buffer no longer holds it up. Twenty seconds at least between two passes it starts (`low_min_s`), so a planner with nothing to add is not asked each poll.

**B, in the keeper and the planner.** The planner marks a transform's step and a held-out read `replan` (`chain_marks`), and a transform's step goes ahead of new trials, as a held-out read already did. When a job marked so comes back, the keeper asks the planner fifteen seconds later (`replan_after_s`), the returns of those seconds gathered into one pass. A trial's read-ons are left to the regular pass: one pass each for their 1,500 returns a day would take most of the Mac's core.

**Checked.** A new check, `tools/plan-check.sh`, runs a real keeper with a stub planner that writes down each time it is asked. With eight GPU jobs queued and the CPU's queue empty, the keeper asked four times in twelve seconds, each `low_min_s` at most; the keeper before this asked none. A step marked `replan` came back and the planner was asked two seconds later (`replan_after_s` 2, `plan_s` an hour); the keeper before this did not ask. `board-check.py` holds which jobs carry the mark. The keeper's other checks pass: the queue's buttons, the breaker, the handover and the idle flag.

**Live, from 10:26.** The keeper took the new code at 10:26:23 and handed over its eleven running jobs; none ran again.

*Table VI. Before and after, measured.*

| Measure | Before (09:45 to 10:25) | After (10:27 on) |
|---|---|---|
| CPU busy | 95.0% | **99.3%** |
| CPU gaps over 30 s | 6, 8.7 lane-minutes | **0** |
| A transform step: planner, queue, work | 3.2 + 2.6 + 1.8 min | **1.2 + 0.2 + 0.8 min** (medians of seven) |
| A transform step, end to end | 7.6 min | **about 2.3 min** |

**What the estimate missed: the pass.** Section VI took a pass to be about 40 seconds, from the handoff's 3.2 minutes less the polling server's 2.5. The keeper now writes each pass's length into its events, and a pass took 80 to 106 seconds. Asked sooner, the planner was running most of the time and a step's handoff waited on a pass already under way. A profile of one pass (`plan/profile-next` makes the next pass write `plan/profile.txt`) found half of it in the file system: 27,000 fetched censuses globbed sixteen times a pass (44 seconds), and every finished job's record loaded five and seven times over by two functions (25 seconds). The planner is a fresh process each pass and a fetched census does not change, so each is now read once a pass (`census_dirs`, `done_titles`, and each census's commit and one-map verdict); the outputs are the same to the byte, checked against the code before on the live queue, and a commit's game version is kept in the state between passes. **A pass went from 105 seconds to 43.** At that length a step's handoff is about a minute, half of it the pass itself; the thirty steps of a transform 70 to 90 minutes (as their work runs 0.8 to 1.8 minutes a step), against 3.8 hours.

**What stays open.** C (the held-out read in pieces) and the background job of VII. The planner now passes about once every one and a half minutes, about half the Mac's one core; a pass under 43 seconds would shorten every handoff again, and its remaining cost is spread over many small reads and 450 git calls.

## References

[1] orrery, *Fifty-Four Idle Minutes: the Line Replayed Twice, Three Guards, and Restarting Orrery Without Losing Work*, lab report, 4 November 2026, sections X and XI.

[2] J. D. C. Little, "A proof for the queuing formula L = λW", *Operations Research*, vol. 9, no. 3, pp. 383–387, 1961.

[3] L. Kleinrock, *Queueing Systems, Volume I: Theory*. New York: Wiley, 1975.

[4] H. Takagi, *Analysis of Polling Systems*. Cambridge, MA: MIT Press, 1986.

[5] G. M. Amdahl, "Validity of the single processor approach to achieving large scale computing capabilities", in *Proc. AFIPS Spring Joint Computer Conference*, 1967, pp. 483–485.

[6] orrery commit `228c30c`, "The line's proofs pooled", 4 November 2026.

[7] The queue's `events.jsonl`, `done/` and `plan/pipeline.json`, 29 October to 5 November 2026.
