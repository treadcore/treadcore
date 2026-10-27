# Reading the Reports, What a Read May Adjust, and a Fleet of Census Machines

**Check-in report CI-066.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 27 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 104; `CENSUS.md`
section 5a; `build/school/` on the Mac;
`build/converge/20261026-181011/` on the census machine.

---

## Abstract

The two training loops' reports are now read by a written procedure:
alive, what it decided, whether that holds, what it costs. The
school's first eight rounds learned maps 9 and 10 in a round each and
kept every passed course at its floor, but open maps slip between
their turns (map 8 from 78% to 55%) and the gauntlet, unguarded,
swings (G8 from 93% to 32.5%). Two guards are proposed for Paul: a
high-water mark for each open map and the gauntlet as a guard. The
read of the census machine found the run's candidate of generation
36 (dodge_horizon 1.08, w_in_range 1.4; held out +1.20 +/- 0.30 and
+1.34 +/- 0.27), which goes to families and then to Paul. A fleet of
Copal machines is designed: workers pull whole censuses from a queue
of files on saskatchewan over ssh, join only after an identity check,
and the learned tank grows from islands to one evolution over the
fleet.

---

## I. Introduction

### A. Intent

Record how the reports are read and what a read may change; say how
a fleet of Copal machines would be orchestrated.

### B. The prompts

> check the school results

> write a lab report describing what you read and adjsut when reading these reports. if I was to install copal on more CPUs, Can I get a fleet of these working tandom. How do we orchestrate this.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "what you read" | A fixed order for each loop: alive, decided, true, cost; the files named | A free read: misses the slippage between a map's rounds |
| D2 | "and adjust" | A table of what a read may and may not change; the keep rule never changes mid-run without Paul | Applying the two guards now: changes the rule after the run started |
| D3 | Map 8 78% to 55%, map 6 16% to 2% | Propose a high-water mark for each open map | Train one map until it passes: the passed maps' guard would still hold, but the queue would stall on map 6 |
| D4 | G8 93% to 32.5%; G4, G6 swing | Propose the gauntlet as a guard before the keep; G9 a test only | Train on the gauntlet's files: it would stop measuring (CI-065, D2) |
| D5 | Map 5 at 95.2% against 99% | Floor unchanged | Lowering it after seeing the result |
| D6 | `candidates.txt` on this read | Report the candidate; families, then Paul | Adopt it: a default is Paul's |
| D7 | "a fleet ... working tandom" | Whole censuses from a queue of files over ssh, pulled by workers, an atomic `mv` to claim, heartbeats, the coordinator's frozen binary by hash | Jobs of one census over machines: breaks CENSUS.md rule 1; Slurm, Nomad, Kubernetes: schedulers that know nothing of the census's rules, for a few machines; Ray: Python |
| D8 | Determinism across machines | A reference census must match saskatchewan's to the character before a machine joins; one Copal release across the fleet | Trust the binary: musl links dynamically, so libm comes from each machine |
| D9 | The learned tank on a fleet | Islands, then specialists, then one shared-seed evolution, each when the one before plateaus | One evolution first: needs bit-identical updates and a long-lived connection |
| D10 | No second machine yet | Design written, code built when the first machine is on the network | Building it untested |

---

## III. Methods

The school: `rounds.tsv`, `state.txt`, `log.txt`, `run.log` and every
`gauntlet-round*.tsv` in `build/school/` on the Mac at 11:03, the
gauntlets tabulated side by side. The run: `progress.txt`,
`candidates.txt`, the candidate's `.weights` and `defaults.toml`
over ssh on saskatchewan at 11:05. The build flags read in
`.cargo/config.toml` (no `target-cpu=native`; `-crt-static` on musl).

---

## IV. Results

| Round | Map | Before | After +/- error | Kept |
| --- | --- | --- | --- | --- |
| 0 | 09 | 11.3% | 100.0% +/- 0.0 | passed |
| 1 | 05 | 93.7% | 95.2% +/- 0.7 | no |
| 2 | 07 | 15.3% | 51.3% +/- 2.9 | yes |
| 3 | 08 | 56.7% | 78.3% +/- 2.4 | yes |
| 4 | 06 | 0.0% | 16.0% +/- 2.1 | yes |
| 5 | 10 | 10.7% | 97.7% +/- 0.9 | passed |
| 6 | 08 | 54.7% | 77.0% +/- 2.4 | yes |
| 7 | 06 | 2.0% | 29.7% +/- 2.6 | yes |

Gauntlet, round 0 to round 7: G1 100 to 86.5; G4 95 to 97.5 (low 34);
G6 70 to 70 (low 0); G7 43 to 27; G8 93 to 32.5; G9 7 to 0 (%, 200
seeds each). Rounds took 460 to 909 seconds.

The run: generation 39, about 540 censuses, no failure. Candidate
gen36/again-dodge_horizon--w_in_range+: +1.92 +/- 0.71 (55 seeds),
+1.20 +/- 0.30 and +1.34 +/- 0.27 (400 each); one weight out at a
time, both inside the noise.

---

## V. Discussion

The school's rule protects what is passed and nothing else, and the
reports show the cost of that exactly where it was not looked for:
open maps and single decisions. The guards are cheap; whether to
change the rule mid-run is Paul's. The run's candidate is its first
since dodge_horizon 2, and partly undoes it; families will say
whether the scenarios agree. A fleet needs no new infrastructure,
only a worker script and the run reading a queue, because the census
was already built of whole, independent, machine-labelled readings.
