# The Fleet Records, the Card Trains: Twenty Times the Data

**Check-in report CI-070.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 108;
`LAB-REPORT-FLEET.md` (the pipes, the ssh and the time);
`LEARNING-DESIGN.md` VIII and IX.

---

## Abstract

What was done:

- The fleet's spare CPUs (calber, the Mac, columbia; nasser joining)
  recorded the scripted brain in 1,200 matches in 14 minutes. Every
  shard was streamed to calber and checked by sha256 at both ends.
- calber's 1080 Ti trained `bc2` on all 47 million training samples in
  226 s, streaming them through the card.
- A batch sweep found the card fastest at batch 32,768; larger batches
  are slower.

What was found:

- Twenty times the data moved the drive head's held-out agreement from
  65.5% to 66.6%. Rule part 2 passed.
- In the census against bc1, paired on one binary: bc2 is 3.42 +/- 0.41 a seat better than bc1, paired on one binary (rule part 3 passed), and level with the official net map7 (+0.57 +/- 0.37, inside the noise).
- The gauntlet and the match reading show what cloning learns: to
  survive, not to go anywhere. The clones die once in 40 matches and
  pass none of the gauntlet's journeys.
- So the senses, not the amount of data, set the clone's ceiling.

---

## I. Introduction

### A. Intent

Use every spare CPU to record, train on CUDA, gather the results in one
place, plan how jobs are collected and orchestrated, and find the batch
size that fills the card.

### B. The prompts

> deploy everywhere, even on spare cpus. run on cuda. consolidate results. plan collection and orchestration of batches. increase batch size to fill up cuda capabilities.

> we also got 192.168.8.121

> also install copal-tm. unsure why its not.

> write a lab report explaining the pipe commands, the nested ssh connectivity between these orchestrated partners. how we collectively use our time and report findings back. in order to change the official listing of matrix scalars for the net (or however you store them).

> what is the hostname of 192.168.8.224 .

> cool. continue.

> continue. commit and push when done.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "even on spare cpus"; the load of each machine read first | calber, columbia, the Mac at `nice 19` on 5 threads; saskatchewan left out (load 23.8 of 24, `make converge`) | Every machine at full width: would slow the run on saskatchewan |
| D2 | Recording is CPU work and training GPU work (LEARNING-DESIGN VIII) | CPUs record, the card trains | Training on the CPUs: Burn's CPU backend is 8 times slower (LR 107) |
| D3 | LR 104 E's fleet design | Jobs claimed by `mv`, a manifest, a heartbeat, files as the record | A server or a message bus first: not on the network yet |
| D4 | Only the Mac holds keys to all | A star: the Mac relays columbia's shards through a pipe | Host-to-host keys or agent forwarding: trust Paul has not granted (LAB-REPORT-FLEET II) |
| D5 | Rehearse a script that touches four machines | A run of 6 seeds of 20 s first, on far seeds | The real run first |
| D6 | 50 million samples will not fit an 11 GB card as LR 107 laid them out | Compact samples, masks from tables, `gather`; streamed in chunks assembled on the CPU while the card trains | f16 senses: Pascal's f16 is slow, and storage alone does not solve 30 GB |
| D7 | Two refused allocations of exactly 1,024 bytes a row; one of 3.7 GB refused on an empty card | Senses uploaded flat; no upload over 2 million samples | Guessing at memory: the failure said what the card did |
| D8 | nasser's login shell is busybox `sh` | Every remote command under bash, sent as base64 | Requiring bash as every host's login shell: a change to each machine |
| D9 | Two edits to the orchestrator while bash was running it | Original text written back into the same inode; the new version run from a copy, then put in place after the run | Leaving it: bash would have read a fragment of the new file on exit |
| D10 | The rule (LR 108 B.3): pair on one binary, else read bc1 again | bc1 read again on bc2's binary (`f1c24a77`) | Pairing across binaries: CENSUS.md 5 |
| D11 | "unsure why its not" | Read first: installed by the Copal Store on columbia and saskatchewan, invisible to a non-login ssh; absent on nasser; installed there as the Store does it; all four on `8459277` | Running `copal-store install`: it asks for doas, which needs Paul's password |
| D12 | The gauntlet's list is fixed | `GAUNTLET_NETS=` adds named nets | Making the clones milestones: they are not |

---

## III. Methods

LR 108 A to D and LAB-REPORT-FLEET III: `scripts/fleet-record.sh plan
rec1200 9000060 1200 24`, `prepare`, `run`, `consolidate`;
`tools/bc/sweep.sh`; `treadcore-bc all nets/bc2.bin 12 32768 1e-2` with
`HELD=9000054-9000059,9001140-9001259`; `make census` on columbia with
`CENSUS_NET=` bc2 then bc1; `make gauntlet GAUNTLET_NETS=bc1,bc2` and
`make train CHECK=... MATCH=40` on the Mac.

---

## IV. Results

| | |
| --- | --- |
| Recording | 1,200 seeds, 49,807,225 samples, 29.2 GB in 14 min; every hash equal |
| The card's batch | 32,768: 3.15 million samples a second; both halving and doubling are slower |
| bc2 training | 47 million samples, 12 epochs in 226 s, 2.6 million a second streamed |
| Drive held out | bc2 66.6% +/- 0.1 against bc1 65.5% (bc1's seeds); 67.7% on 120 new seeds |
| Census, paired by seed | bc2 - bc1 +3.42 +/- 0.41 (one binary); bc2 - map7 +0.57 +/- 0.37; bc1 identical on both binaries, seed for seed |
| Gauntlet journeys (G5, G6, G7, G9) | 0 to 0.5% for both clones |
| Match reading | deaths 0.03 (bc1, bc2), 0.82 (map7); damage 4, 4, 27 |

Errors and false starts:

- an equivalence check bound written too tight in LR 107, widened before
  the run;
- `pkill -f "nvidia-smi dmon"` killed its own ssh session (bc2 was
  already written);
- the gauntlet run first needed the extra nets;
- the census's binary differed from bc1's because the fleet's worktrees
  shared columbia's `target/` (bc1 read again, as the rule said).

---

## V. Discussion

The fleet works and its time is accounted for. The card, made efficient,
reads all the data in four minutes.

The finding is about cloning. More examples of a choice the net cannot
see the reason for teach it the safe guess: stand still, hold fire,
survive. The drive label is the sector of a route the scripted brain
planned over the whole map. Two ways forward, Paul to choose:

1. Give the net the plan as senses: its route's next waypoint and its
   goal, as a sector and a distance.
2. Train on the census's own score (PPO from bc2), which needs no
   labels at all.

The official net stays `nets/map7.bin` until a candidate beats it in the
census (LAB-REPORT-FLEET VII).
