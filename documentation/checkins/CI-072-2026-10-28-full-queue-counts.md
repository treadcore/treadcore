# The Queue at Full Settings: What Four Hours Played

**Check-in report CI-072.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 111;
`scripts/fleet-count.py`; orrery's `docs/fleet-train.md`.

---

## Abstract

The fleet queue ran at full settings on calber, columbia, nasser and the
Mac. Its first 4 hours 10 minutes, counted from the logs it fetched back:

- 37,905 games of 15 minutes, which is 9,476 hours of play;
- 1.46 billion decisions (thinks), 10.4 billion seat-ticks and 381
  million planned moves;
- 416 million training sample-steps on calber's GPU, in two net rounds.

The results so far:

- 144 weight trials, 6 of them past two errors. None has passed both
  held-out reads.
- `castle_inside` 8 leads: ten 55-seed censuses all at or above zero,
  and 2 of 5 held-out reads passed.
- Both net rounds left bc2 the best.

---

## I. Introduction

### A. Intent

Keep the fleet's census running at the settings that can tell a weight
from luck, then say how much it has computed.

### B. The prompts

> continue running census on the fleet.

> also cafinate this mac

> hows it going.

> how many computations have been ran. over a million?

> add castle_inside = 8 to the converge candidates. and add lab report about these counts.

Paul's choices:
- "Replace with full queue", which stopped the sample.
- "Wait for the 3 reads": castle_inside 8 joins the candidates only with
  the verdict of its held-out reads.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The sample's 4-seed, 2-minute censuses cannot tell a weight from luck (CI-071 V) | Replace it with the full settings of `docs/fleet-train.md` | Leaving the sample running |
| D2 | `stop-now` kills jobs on three remote hosts; the permission check stopped it | Asked Paul; he chose to replace | Draining with `stop-adding` (slower; the sample's results matter little) |
| D3 | saskatchewan's load was about 20 of 24 on `make converge` | Left it out of the queue | Giving it fleet work (CLAUDE.md, memory) |
| D4 | Seeds must not overlap saskatchewan's run | Trials from 6,000,000, held out from 7,000,000; converge uses 2,000,000 and 3,000,000 | Sharing a seed range |
| D5 | "cafinate this mac" | `caffeinate -dimsu` in the background | Changing the Mac's power settings |
| D6 | "how many computations": a count must name its unit | Games, seat-ticks, thinks, moves and training sample-steps, each from the logs' own lines | Estimating from seat-ticks / 288,000 (net games have other sizes; the first reply's 32,000 games was that estimate) |
| D7 | A count must be repeatable | `scripts/fleet-count.py` reads only fetched results and `events.jsonl` | A one-off shell line |
| D8 | castle_inside 8 had passed 1 of 3 reads, not both of a pair | Asked Paul before adding it; he chose to wait | Adding it as a CANDIDATE, against LR 92's rule |
| D9 | The first report said three trials passed their re-checks | Corrected to Paul and recorded (LR 111 C) | Leaving it |

---

## III. Methods

1. `python3 ~/code/orrery/tools/fleet-queue.py build/queue/sample
   stop-now`.
2. `build/queue/full-1/config.json` is the sample's, with the planner's
   path changed. Its `plan.json`:
   ```json
   {"commit": "8e5df8827fb680aa28ad3aad00e4fbb5308acfe0",
    "census_host": "columbia",
    "weights": {"seeds": 55, "secs": 900, "map": "all", "seed0": 6000000,
                "held_seeds": 400, "held_seed0": 7000000},
    "net": {"rounds": [{"run": "rec2400", "lo": 9001260, "n": 1200},
                       {"run": "rec2400", "lo": 9002460, "n": 1200}],
            "train_args": "4 32768 3e-3",
            "census": {"seeds": 110, "seed0": 500000, "secs": 900, "map": "all"}}}
   ```
3. `fleet-queue.py build/queue/full-1 ensure`; `caffeinate -dimsu &`.
4. At 06:26:56: `python3 -I scripts/fleet-count.py build/queue/full-1`.
   The held-out reads come from `plan/ledger.tsv` and the job titles in
   `done/`.

---

## IV. Results

| Kind | Games | Seat-ticks | Thinks | Moves |
| --- | ---: | ---: | ---: | ---: |
| weight trials | 24,585 | 6,757,079,968 | 957,738,999 | 250,016,569 |
| held-out reads | 12,000 | 3,285,884,736 | 463,260,527 | 120,723,097 |
| net judging | 1,320 | 364,327,056 | 36,308,633 | 9,769,679 |
| all | 37,905 | 10,407,291,760 | 1,457,308,159 | 380,509,345 |

**Net training:**
- Round 1: 52,067,105 samples × 4 epochs, in 90.6 s.
- Round 2: 51,962,086 samples × 4 epochs, in 87.8 s.

**Hours busy:** calber 4.1, the Mac 4.1, columbia 3.8, nasser 3.3. No job
failed.

| Trial | Weights | Held-out read 1 | Read 2 |
| --- | --- | --- | --- |
| w0011 | advance_band 0.675, alpha 0.513, answer_cap 2.51 | +0.56 ± 0.24 | −0.11 ± 0.24 |
| w0095 | castle_inside 8 | −0.02 ± 0.16 | +0.56 ± 0.21 |
| w0113 | base_r 8, beta 0.6, bridge_keep 360, castle_inside 8 | +0.19 ± 0.19 | +0.40 ± 0.19 |
| w0116 | column_gap 1 | −0.02 ± 0.17 | −0.20 ± 0.12 |
| w0117 | castle_inside 8 | running | +0.28 ± 0.21 |
| w0128 | castle_inside 8 | +0.80 ± 0.21 | running |

**Net rounds,** candidate less bc2 over 110 seeds:
- Round 1: −0.77 ± 0.36.
- Round 2: +0.18 ± 0.35.

**Errors:**
- Each count is a lower bound at its moment: running jobs are left out.
- The per-host games come from the ledger, which lags the logs by a
  census or two.

---

## V. Discussion

- **Yes, over a million,** in every unit but whole games:
  - 1.46 billion decisions;
  - 10.4 billion seat-ticks;
  - 416 million training sample-steps.
  
  Whole games, at about 9,100 an hour, would reach a million in about
  4.6 days.
- **What the play bought so far:**
  - No weight trial yet passes the two-read rule. The defaults are near
    the top of most curves, as saskatchewan's run found (journal 0103).
  - `castle_inside` 8 is the one direction both runs point to.
- **19 trials changed no score at all.** On these maps those weights are
  inert. They are cheap to drop from the screen, or worth a scenario
  that exercises them.
- **Next:**
  - castle_inside 8's last two reads, then its line in saskatchewan's
    `candidates.txt`;
  - the queue goes on.
