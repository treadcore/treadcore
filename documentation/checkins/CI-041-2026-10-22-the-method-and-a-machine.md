# The Method, a Machine for It, and Competitions

**Check-in report CI-041.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 22 October 2026.
Companion records: journal 0098; `LAB-REPORT-BRAIN.md` sections 71
(final) and 72; `CENSUS.md`; `CLAUDE.md`; roadmap backlog 58 to 61.

---

## Abstract

LR 71's census ended at +5.94 +/- 0.38 a seat over 176 seeds, and its
section now carries the final numbers. The census's cost was measured:
3.4 CPU-hours and 5 MB a job, with each job given only a third of a
core in the guest VM. The census needs cores and nothing else. This
check-in prepares it for a dedicated machine. It adds a manual
(`CENSUS.md`), a resume option (`RESUME=`, checked to replay a job
exactly), `JOBS` defaulting to every core, and `CLAUDE.md`, so a
session on a new clone has the working rules. LR 72 sets out the
method behind 71 sections (directed changes tested by paired Monte
Carlo censuses, drawn as a loop). It also sets out what a machine
opens: profiles of play, their round robin as a payoff matrix that can
show rock-paper-scissors, and rotation among them as a meta-strategy
(backlog 60, 61).

**Index terms:** brain census, Monte Carlo, paired difference,
compute, resumable runs, payoff matrix, mixed strategy.

---

## I. Introduction

### A. Intent

To finish LR 71's record, then turn the census from a test that runs
overnight in a VM into an instrument a dedicated machine can run and a
new session can resume. Then to write down the method and the next
question it makes affordable: no single style of play may be best.

### B. The prompts

> what other backlog itmes need. should we be testing this on the mac side. seems we are ram and cpu limted. what resources do you need. I can put you on a server. What would speed this up. I got a DDR2 server with 96 GB of ram. or perhaps  64GB DDR4 new pc. Is there an improvement that we can change our testing hardware?

> its a dual socket supermicro server board, . I think it has 48 threads maybe cores.

> update section 71 with the final numbers and push. WRite a lab writeup of using the cargo test -p treadcore-brain consensus. With options of resuming this test as a resume prompt . so write memory in a way that git can resume from it when I launch claude on the new machine. Talk about tuning the brain method. Talk about our approach, and this milestone of getting a dedicated compute to dial in the AI battles. So now we can have competitions .

> update section 71 with the final numbers and push. WRite a lab writeup of using the cargo test -p treadcore-brain consensus. With options of resuming this test as a resume prompt . so write memory in a way that git can resume from it when I launch claude on the new machine. Talk about tuning the brain method. Talk about our approach, and this milestone of getting a dedicated compute to dial in the AI battles. So now we can have competitions, and variations of game attitudes and game play style. We can create profiles in a rock paper scissors like game. Cause there is no single method that works. Perhaps a rotation to strategies is even a meta strategy. when done. draw a diagram on the lab report showing the process of brain improvement using directed changes and testing monty carlo (if thats what we are doing).

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "seems we are ram and cpu limted" | Measure first: `ps` and `free` during the run | Answer from the hardware list: the guess would have been RAM |
| D2 | F: 5 MB a job; 44 min of CPU in 2 h 22 min a job | The census is CPU-bound and the VM is starved; RAM rules out neither box | — |
| D3 | F: a DDR2 board cannot carry 48 threads on two sockets | Ask for `lscpu`; estimate both cases | Choose on "DDR2": the thread count matters more |
| D4 | "update section 71 with the final numbers" | `pooled.txt` replaces the part-way table; the meter recounted from per-match lines only | F: the jobs' end summaries had doubled the first count |
| D5 | "options of resuming this test" | `RESUME=DIR`: keep finished jobs, replay the rest whole on the frozen binary | Resume mid-job from the last whole seed: more code, and a job is under an hour on a machine |
| D6 | Resuming is only sound if a seed replays exactly | Checked: a resumed job's log matched the original line for line, timings apart | Assume it |
| D7 | "write memory in a way that git can resume from it" | `CLAUDE.md` in the tree with the working rules; the resume prompt in `CENSUS.md` | The agent's memory directory: it stays on this machine |
| D8 | "monty carlo (if thats what we are doing)" | Yes, for the testing half; the changes are directed, not random. Drawn as a loop in LR 72 | Call the whole loop Monte Carlo: the changes are hypotheses, not samples |
| D9 | "rock paper scissors ... rotation ... meta strategy" | The round robin's payoff matrix as the test; its solved mix as rotation; backlog 60, 61 | Build profiles now: Paul asked for the write-up, and one step at a time |
| D10 | x86 and ARM may round floats differently | "One census, one machine" in CENSUS.md and CLAUDE.md | Pool across machines for speed: the pairing would not hold |

---

## III. Methods

**The cost.** During LR 71, `ps` gave each `census-bin` job's CPU time
and elapsed time (44:06 of CPU in 2:22:30) and its resident memory
(4 to 5 MB). `free` gave the VM's 8 GB. The CPU-hours are the four
jobs' CPU time to the end. The machine estimates scale those hours by
assumed cores and per-core speed; they are estimates, marked so.

**The final reading.** `scripts/pool.sh` over the four jobs' logs
(`pooled.txt`). The meter's dodge counts were summed over the lines
beginning `  meter:` only. Each job's closing summary repeats the
totals, which doubled the first count.

**The resume check.** `SEEDS=2 SECS=20 JOBS=2 make census`; one job's
`test result` line removed; `RESUME=` on the directory. The job was
replayed, the old log kept as `.partial`, and the two compared with
timing lines excluded: identical.

---

## IV. Results

| | |
| --- | --- |
| LR 71, final | +5.94 +/- 0.38 a seat, 176 seeds; 10 of 11 maps past two errors; fortressisles2 +2.29 +/- 1.15 |
| Cost of LR 71 | 3.4 CPU-hours; 23 s of CPU a match; 5 MB a job; about 1.2 cores used of the VM's 4 |
| The dodge, read by seed count | +5.63 +/- 1.80 at 10 seeds of one job, +7.05 +/- 1.24 at 20 |
| Resume | a replayed job identical to the original |

---

## V. Discussion

1. **The limit was never RAM.** A census is cores. The 96 GB and the
   64 GB are equally beside the point. The server's 48 threads, if it
   has them, are what matter.
2. **The method is the loop in LR 72.** Directed changes, judged by
   paired Monte Carlo censuses against a rule written first.
3. **The machine changes which questions can be asked.** Seeds were
   rationed, so most parts were read at 12 to 48. The round robin of
   profiles is a dozen censuses: a night in the VM, a few hours on the
   server.

**For Paul:** the machine's `lscpu`, then CENSUS.md section 5 on it.
The first run there is LR 71's census again, to time it.
