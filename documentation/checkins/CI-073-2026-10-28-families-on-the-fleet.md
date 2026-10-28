# Families on the Fleet

**Check-in report CI-073.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 112; orrery's
`tools/fleet-queue.py`.

---

## Abstract

This is step 1 of six that move treadcore's training from make to
orrery's fleet queue.

- **Families is a job of the queue.** `make families`'s test runs for 8
  candidates a job, side by side on a host's cores, and each family's
  share is recorded.
- **42 candidates ran on the fleet in 3 minutes:** the defaults, the
  queue's two of `castle_inside` 8, and saskatchewan's 39. All hold
  every floor.
- **gen64/best is the strongest:**
  - census +2.97 ± 0.71 over 55 seeds;
  - held out +2.40 ± 0.28 and +2.32 ± 0.28 over 400 seeds each;
  - the duel family from 76.2% to 84.0%.
- **A fault, fixed:** a host that dropped off the network failed eleven
  jobs and stopped the queue. Lanes now wait for a lost host, and
  requeue its job.

---

## I. Introduction

### A. Intent

Begin moving every training and measurement command from treadcore's
Makefile into orrery's fleet queue. Families comes first, because 38
candidates were waiting on it.

### B. The prompts

> commit the orrery lab report. and what needs to be done with make families.  we want orrery to take over all make training commands. We want to use orrery to orchestrate training as a fleet. We no longer start training from a make command. Remove the training from make, and insert them all into orrery. Make sure all aspects are accomplished, including collecting the data and incorporating the updates into treadcore itself.

Paul's choices:
- "All 13": every training and measurement target leaves the Makefile.
- "Prepare, you approve": a candidate is prepared for treadcore, and it
  lands only on his word.
- "Newest first, all 38": families reads every candidate.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The move is large; Paul works a step at a time | Six steps, each recorded; families first | One change for all 13 targets |
| D2 | saskatchewan's candidates came from 7d0af54, the queue builds 8e5df88 | Compared the dumped defaults: 111 weights, identical; families runs at the queue's commit | Building 7d0af54 on every host |
| D3 | The families test is one thread; a host has 4 to 24 | A job is a batch of 8 candidates side by side | A job a candidate (a 16-thread host would run one core) |
| D4 | The test fails when a family is under its floor, and the breaker counts failures | The exit status becomes a verdict in the output; the job succeeds | Letting a weak candidate trip the breaker |
| D5 | The command is new, and five failures stop the queue | Tested once on the Mac (defaults: 55 s, holds) before switching it on | Trying it on the live queue first |
| D6 | nasser went down; its lane failed 11 jobs in 31 s; the breaker stopped the queue | Lanes wait for an unreachable host; a job lost to ssh's 255 goes back on the queue; a host removed from the config ends its lane | Raising the breaker's count; a retry count per job |
| D7 | The running keeper has the old code | nasser parked (its build record set aside); the fix applies at the keeper's next start | Restarting the keeper now (running jobs would run twice) |
| D8 | castle_inside 8 passed both reads twice (w0170, w0182) | Added to saskatchewan's candidates.txt as Paul chose (LR 111 D) | — |

---

## III. Methods

1. **`scripts/fleet-plan.py`:**
   - `family_candidates()`, `family_jobs()` and `families()`;
   - in `plan()`, families take up to half of each top-up;
   - `collect` includes them.
2. **`plan.json`:**
   `"families": {"converge": ["build/saskatchewan/converge/20261026-181011"], "queue": true, "batch": 8}`.
3. **The local test:** the rendered job command, with only the defaults,
   run on the Mac against `~/fleet/q-full-1/census-bin`.
4. **Reading the results:** `python3 -I scripts/fleet-plan.py
   build/queue/full-1 families` writes `plan/families.json`.
5. **orrery's `tools/fleet-queue.py`:**
   - `reachable()`;
   - in `lane()`, the removed-host and unreachable-host checks;
   - in `work()`, the requeue on 255.
   
   Tested on a scratch queue `{here: "-", gone: nasser}` with six
   3-second jobs, then with `gone` removed from its config.

---

## IV. Results

| | |
| --- | --- |
| Local test, defaults | holds every floor, 55 s |
| Families on the fleet | 6 jobs (Mac 4, boburner 2), 42 candidates, 09:12 to 09:16, 38 to 46 s a candidate |
| Verdicts | 42 of 42 hold every floor |
| Tightest | a tank under a ready gun: 82.5% at worst (gen53/best), floor 80% |
| Widest spread | a duel in the open: 75.2% to 84.0%, defaults 76.2% |
| Scratch queue, host down | 6 of 6 jobs done on the other host, 0 failed; `host unreachable` |
| Scratch queue, host removed | `lane left` within a probe |

**Errors:** a family's share has the error the test prints, for example
±1.8% on the duel's 400 seeds. Every candidate's margin over its floor
is larger than three of its errors, except the ready gun, which is 2.5
points over at worst, with an error of ±1.2%.

---

## V. Discussion

`make families` was a gate a person ran by hand on one machine. In the
queue it costs three minutes of the fleet, and it reads every candidate
as soon as it exists. The families never stopped a candidate here.

What decides now is the census, and Paul. gen64/best is the strongest
candidate, and it is what step 5 (promotion) will be built and tried
on.

The nasser fault shows the queue must treat a machine's absence as an
event, not as a failure. It now does.

**Next, step 2:** one ledger of every census, saskatchewan's included.
