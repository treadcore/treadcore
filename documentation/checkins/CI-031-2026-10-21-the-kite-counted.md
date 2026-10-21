# The Kite Counted

**Check-in report CI-031.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0088; `LAB-REPORT-BRAIN.md` section 65;
CI-030; roadmap backlog 54, 55.

---

## Abstract

Backlog 54. The kite was the one part left leaning below zero (LR 64,
-0.94 +/- 0.77 at 48 seeds), and the meter counted no kiting. A new
meter counts each live seat's seconds by kite step. Over 12 seeds of
the fair census the trial seat kites 4.7% of its seconds alive, about
114 s a match. Of that, 62% is spent holding past the target's reach,
18% massing, 11% stepping out and 9% stepping in. The kite acts, and
mostly waits. The meter changes no play (the per-seed differences are
LR 63's). The kite stays on; its hold goes to a duel family, backlog
55.

**Index terms:** kite, meter, census, tuning.

---

## I. Introduction

### A. Intent

To learn whether the kite acts on naval.map, and what it does when it
does, before any number of it is tuned.

### B. The prompt

> continue

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt; CI-030 named 54 next | Backlog 54 | — |
| D2 | `Act::kite()` already names the step for all three kites (`Kite`, `WingKite`, `RallyKite`) | Count the act once a second after the think (`think_released`) | Counting inside `Tactician::kite`: it is also asked by plain RuleBots and in what-ifs, which do not move the tank |
| D3 | The question is "how much of a life" | Seconds by step, out of seconds alive, by group | Kills won and lost while kiting: needs kill attribution the meter lacks; the time answers the hypothesis's first branch |
| D4 | Steps are 8 | Five words in the line (stepping in and holding summed) | Eight: the reasons inside a hold are for 55 |
| D5 | F: 4.7% of the life, 62% holding | The kite stays on; its hold to a duel family (55) | Tuning `kite_spare` on the census: a number moved by noise; a family scores a duel |

---

## III. Methods

**Code.** `meter.rs`: `Meter::kites: [[u64; 9]; 2]`, by group the 8
steps in `KiteStep::ALL`'s order and seconds alive; `kite(step)`;
summed in `add_fronts`; the census line. `commander.rs`:
`think_released`, for a live seat on a whole second,
`meter::kite(self.act.kite())`. Test: `a_second_alive_counts_its_kite_step`.

**Read.** `build/lr65/run.sh`: a frozen binary; the fair census,
naval.map, 900 s, 3 respawns, seeds 0 to 11 in three jobs of 4; trial
the defaults, baseline `caps = baseline, -kite`. The meter totals from
each job's census line, summed.

---

## IV. Results

| Measure | Result |
| --- | --- |
| Trial seconds alive | 88,122 |
| Kiting | 4,111 (4.7%), about 114 a trial match |
| Holding / massing / stepping out / stepping in / closing | 2,549 / 746 / 433 / 356 / 27 |
| The rest kiting (kite off) | 0 of 199,437 |
| Difference a seat, 12 seeds | -1.76 +/- 1.04 (LR 63's row) |

Tests: brain 84 passed and 11 ignored; client 32; net 14; proto 3;
sim 50.

---

## V. Discussion

1. **The kite acts, and waits.** Two minutes a match of a part is not
   seldom; six tenths of it is a tank past its target's reach,
   waiting for a window.
2. **Time is not cost.** Whether those minutes are lost depends on what
   the tank would do instead, which a census cannot isolate and a duel
   can.

**For Paul:** none blocking. Backlog 55, the kite's hold in a duel, is
next. Commits are local; nothing was pushed.
