# Faster, and the Run: The Census 3 Times Quicker With the Same Matches, and a Learner That Runs Until Stopped

**Check-in report CI-056.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` sections 91, 92 and 93;
roadmap backlog 69 and 71; `build/converge/`, `build/route/` and
`build/day/` on the census machine.

---

## Abstract

A profile of the census found one function, `World::visible_to`,
taking 58% of its time. Four changes make the census 2.6 times faster
with every match the same: the old and new binaries were compared
match for match on the Mac and on the census machine, and with
`make families`. They are deployed, mid-day, to the census machine.
The day's queue and a planned night were folded into one run,
`make converge`. Each generation it reads every census into a ledger,
fits each weight's curve, and plays the trials the fit suggests. A
trial past two errors is read twice more on fresh seeds. A faster
forecast for the dodge (`dodge_own_route`) saved 1.2 times more but read
−6.48 ± 1.56 on big-lizard, so it ships off. CUDA would not speed the
census; the GPU is for training a network later. Paul set that aside.

**Index terms:** profiling, determinism, response surface, successive
halving, brain census.

---

## I. Introduction

### A. Intent

Make the census faster where it can be done without changing a match.
Turn the census into a learner that runs day and night and suggests
its own next trials. Test a route-search speed-up that does change
matches with a census. Answer the CUDA question.

### B. The prompts

> we ahve lots of unexplored variables. Is there a way to write the make census write lots that give suggestions that can be picked up and used to adjsut the next generation of runs. We essentially want a method to just start computing at night time which runs concenus to converge the values. We have CPU . but we can also install some CUDA on Copal and perhaps adjust the brain to simulate much faster. If it won't be faster, then just keep with CPU, and create a more swarm based analysis that we can collect all the logs and merge them into better data, to converge the bets values.  Right now we got the remove PC with GPU. I think you should commit. We have a working version. I want to consider making this fater, and what we can do. Eventually I want a parallel training of a neural network. For a tank game that arms its gun, gets ammo, in this universe.

> nice findings and reductions. Continue to refactor. I like these speedups.

> yes. all your askss. 1,2,3. make night. consus. faster route search with consusus.

> look at the logs. import all good analysis. recompile. build. push. commit. all the above. jget thsese new speed ups computing.

> what commands shall I input in the copal on with the nvidia with cuda. I want to target the dev newest version, even if we have to build it, within reason. i have a 4060ti. so it must support that.

> ok. so night and day need to be consolidated. we just want to have a run. a way for it to choose eventually from all of the variables . doing your methods of analyzing them . i also have a question about why sthese consus are going very quick and some are ending right after the batch starts. Why would a simulation go so fast. is this our improvement?

> start the run and do the route search. thanks for the explanation of CUDA. I'll side that for now.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "I think you should commit" | fcbce9e: the morning's and the day's records in one commit (their edits share files) | Two commits: needs hunk staging, which this session cannot do |
| D2 | "simulate much faster"; a GPU on the census machine | Profile first (macOS `sample`), on the Mac | Guess at the cost |
| D3 | `visible_to` 58%, each check searching the players | Gather a side's eyes once an observation | — |
| D4 | The census is the instrument | A speed-up must keep every match the same: 18 matches compared line for line, the timer apart | Accept small drifts: every comparison since LR 71 would be in doubt |
| D5 | Regions joined in place, danger marked once: no gain measured | Taken out | Keep them: code without a gain |
| D6 | The ring read's exactness rests on a bound | A test against the old scan over 3000 points, and the bound broken on purpose to see the test fail | — |
| D7 | The census machine runs x86, the Mac ARM | A row of the day played on both binaries there: the same | Trust the Mac's check alone |
| D8 | "push. commit." | Pushed d4181b2 and later; the census machine pulled between censuses | Pull mid-build: a half-updated tree |
| D9 | GitHub Actions paused | Pushes to main only; no tag | — |
| D10 | "suggestions ... next generation of runs" | A ledger (one row a census) and a fit (each weight's curve, a response surface) | SPSA alone: one direction a step, blind to what the screens already read |
| D11 | "night and day need to be consolidated" | One run, `make converge`, until stopped; the day's queue stopped after stage 1's half ends | Keep both: two queues racing for one machine |
| D12 | The day's stage 2 and SPSA | Folded into the run: reads again of z past 1.5, the double ends, Thompson draws | — |
| D13 | Many censuses read 0.00 | Checked: the smoothing weights act above `w_smooth` 0.5 (LR 86); proven identical binaries, not a fault | Assume |
| D14 | The run died at its first census (exit 141) | `ls … \| head` under `pipefail` on many folders; fixed with `sed -n 1p`, resumed | — |
| D15 | A route change moves the defaults under the run | `dodge_own_route` ships at 0 until the census reads it | Ship on (the 20 October habit): the run's table would mix two brains |
| D16 | The route census and the run would both take "the newest census" | Queued one after the other on the machine | Run both at once: the run could read the wrong census |
| D17 | Big-lizard −6.48 ± 1.56 | Off, by the rule written first | Promote on the whole's reading |
| D18 | CUDA on Alpine (musl, NVK) | Ubuntu on the spare disk recommended; the census does not need a GPU | Install a driver on the census machine unasked |
| D19 | The sim's route search is the same for every team | Not changed: the census cannot pair it | — |

---

## III. Methods

**The profile** (Mac): a census run in the background, sampled with
`sample PID 20`, its call tree attributed by caller. A build with the
linker's symbol merging off did not name the merged symbol. Its caller
in the tree (`route_kept`) did.

**The identity check**, after each change:

    CENSUS_MAP=<eleven maps>,gen CENSUS_RESPAWNS=3 CENSUS_SEED0=170000 \
      CENSUS_SEEDS=6 CENSUS_SECS=300 cargo test -q -p treadcore-brain \
      --release -- --ignored brain_census --nocapture

stderr compared with `diff`, the "us a seat-tick" figures and the
harness's "running for over 60 seconds" lines taken out. On the census
machine: the census binaries of 20261026-102652 (before) and
20261026-104153 (after), `CENSUS_WEIGHTS` column_past=1, 4 seeds of 300 s.

**The run** on the census machine: `nohup setsid make converge`;
resumed after the route census with `RESUME=build/converge/STAMP`.
**The route census**: `TRIAL=build/route/own.weights SEEDS=110
SEED0=170000 MAP=all JOBS=24 make census`, its rule in
`build/route/NOTES.txt` first.

---

## IV. Results

| Read | Result |
| --- | --- |
| Before (the test census) | 125.8 s; bots 64.5 µs a seat-tick |
| The eyes, rings, heap, sites and radar | 48.7 s; bots 20.2 µs; every match the same (Mac and census machine); families the same counts |
| On the census machine | a 55-seed census 3:55 → 1:50 |
| `dodge_own_route` 1, every team | 38.9 s; bots 15.8 µs |
| `dodge_own_route` 1 against the defaults, 110 seeds | −0.53 ± 0.48; big-lizard −6.48 ± 1.56: **off** |
| The arms (LR 89) | w_exposure 0.15 −0.37 ± 0.36; w_mine 2 +0.04 ± 0.14; w_flank 1.5 +0.17 ± 0.41: all act, all inside the noise |
| The day's screen, stage 1 | dodge_horizon half +2.71 ± 0.82 (z 3.3, 55 seeds), being read again by the run |

Errors: two tries with no gain, taken out. A false alarm on x86 (the
harness's timing line), caught before any claim was made. The run died
once (exit 141) and was resumed.

---

## V. Discussion

Every change was measured against a census that matches to the
character, and that check is what made a 2.6 times speed-up safe on a
machine running a day of results. The two tries that bought nothing
show the profile alone was not enough; each change had to be timed.
The route change is the reverse case: a real saving that alters play,
and the census found what a forecast that looks more faithful costs on
one map. The run now does on its own, generation after generation,
what the last three days did by hand.
