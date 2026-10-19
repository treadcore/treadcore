# The Meter by Team, and LR 29 Corrected

**Check-in report CI-007.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0055; `LAB-REPORT-BRAIN.md` sections 29 and
30; CI-006.

---

## Abstract

CI-006 estimated, from a meter that mixed the teams, that a boat's
dodge saves nothing. This step split the meter by team and by whether
the dodge was taken, with no change to play, and read it over 16 seeds
of the dodge off. Ashore the dodge halves the kills (15.0% to 6.6%);
afloat it cuts them by a fifth (18.2% to 14.7%). The estimate failed
because the team without the dodge drew fewer shells. CI-006's afloat
finding is withdrawn, and backlog 31 is weakened and moved after 30.

**Index terms:** instrumentation, counterfactual, measurement error,
reflexes.

---

## I. Introduction

### A. Intent

To start backlog 31 with the instrument CI-006 D5 called for, so the
dodge afloat could be read exactly before any part was built on it.

### B. The prompt

> commit. push. continue backlog

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-006 D5: the estimate needs equal shares | Group by seat (`meter::trial`, `meter::seat(p)`) and by taken (`Dodge::dropped`) | A meter per brain: the census reads one thread-local, and a group bit is enough |
| D2 | Proving play unchanged | Re-run `-dodge` on LR 29's first 16 seeds; compare per-seed differences | A null run: the same seeds against a known reading prove it and give the counts at once |
| D3 | F: afloat 18.2% dropped against 14.7% taken | Withdraw LR 29's afloat reading, marked in place | Deleting it: the record keeps what was believed and why it changed |
| D4 | F: the boat's dodge saves a fifth | Backlog 31 stays as a trade for the tuner, after 30 | Dropping 31: the crossing time the dodge costs is still unmeasured |

---

## III. Methods

### A. The parts, function by function

| File | Function or field | What it does |
| --- | --- | --- |
| `meter.rs` | `trial(mask)`, `SEATS`, `group()` | a thread-local mask of trial seats and the seat being run; a count's group |
| `meter.rs` | `seat(p)` | names the seat (and counts it, as before) |
| `meter.rs` | `Dodges`, `Meter::dodges[group][dropped][afloat]`, `add_dodges`, `dodge_line` | the counts, their sum for a census, and the meter line, a phrase for each kind that happened |
| `meter.rs` | `dodge(afloat, dropped)`, `dodge_hit(afloat, dropped, killed, shell)` | the bumps, into the seat's group |
| `lib.rs` | `Dodge::dropped`, `watch` | carried from the dodge's start to its outcome |
| `lib.rs` | `run_bots` | `meter::seat(*p)` |
| `commander.rs` | `think_on`, the survival branch | `dodge.dropped = !dodges` before `order_with` |
| `fleet.rs` | `brain_census` | `meter::trial` of the trial team's seats each match; `add_dodges` to the total |

### B. The run

`CENSUS_CAPS=-dodge`, `CENSUS_SEED0` 0, 6, 11 with `CENSUS_SEEDS` 6, 5,
5, side by side; naval.map, 900 s, respawns 3, the fair census.

---

## IV. Results

Per-seed differences: equal to LR 29's seeds 0 to 15.

| | Taken (the rest) | Dropped (trial) |
| --- | --- | --- |
| Ashore: begun, by the shell, killed | 4,982; 15.5%; 6.6% | 1,899; 34.0%; 15.0% |
| Afloat: begun, by the shell, killed | 2,148; 6.9%; 14.7% | 1,029; 10.1%; 18.2% |

Tests: brain 70 passed and 8 ignored; client 31; net 14; proto 3; sim
50. 354 functions, 121 undocumented, 78 tests.

---

## V. Discussion

An ablation changes how much of the world a team sees: the team without
the dodge is dead more, so it meets fewer shells. Any estimate that
holds exposure fixed across an ablation is biased by the ablation
itself. Per-group counts remove the assumption.

**Next:** backlog 30, the run for the beach.
