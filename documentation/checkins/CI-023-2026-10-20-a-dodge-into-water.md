# The Circle at the Edge of Sight, and a Dodge into Water

**Check-in report CI-023.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journals 0077 to 0079; `LAB-REPORT-BRAIN.md`
sections 49 to 52; CI-022.

---

## Abstract

Scenario files grew from three ported tests into a fast instrument,
and in four steps they found three things the census could not. First,
the mechanism of the landfall memory's gain: without memory a boat
circles at the edge of a gun's sight for 39 s and lands beside it, and
with any memory over 2 s it lands on the safe coast in 5. Second, a
Commander built directly ran the landfall's weights at 0; this is
fixed, and dealt play is unchanged to the hundredth. Third, a dodge
could choose a point in the water, be refused, and leave the tank
under the shell. The fix, measured before and after on the same 48
seeds, gives 7.1% fewer deaths. One claim made on the way was wrong
and is withdrawn: that a Commander at sea never prices a beach.

**Index terms:** scenario testing, partial observability, limit cycles,
reflexes, before-and-after measurement.

---

## I. Introduction

### A. Intent

To make the landfall testable in seconds (LR 48's next step), then to
run scenarios over many seeds.

### B. The prompt

> commit. push. continue backlog

(Five times across the four steps.)

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The landfall needs sea and coasts | `[[terrain]]`, `[[tank]]`, teams, weights, `lands_near` | Map files per scenario: carving in the file keeps it one page |
| D2 | Three versions held with memory on and off | Run each broken before trusting it; each failure traced | Accepting a passing test |
| D3 | A 5 s trace showed `fortify` throughout | Claimed "never prices a beach" (LR 49) | |
| D4 | A 1 s trace showed `landfall` from 1.9 s | D3 withdrawn; `trace_every`, positions in the trace | Leaving the claim |
| D5 | Identical runs: no `set_settings` | `Commander::new` applies the defaults; dealt play checked unchanged | Fixing only the runner: the trap is in the constructor |
| D6 | F: the circle | A scenario that fails without memory | |
| D7 | A pass or fail on one seed says little | `many`: a share over seeds with its error | |
| D8 | F: 97 of 100, "refused: water" | `Dodge::order_on` with the map, four candidate points | A retry on refusal: a tick late, and only after failing |
| D9 | A shared reflex cannot be paired | Before and after nulls, frozen binaries, the same seeds | A switch only to measure it: a bug fix has no "off" worth keeping |
| D10 | F: water-only made boats worse | Water or a beach afloat | Shipping the first fix |

---

## III. Methods

The scenario runner (`scenario.rs`): placements, `trace_every`,
`run_seeded`, `many`, `set_weights`, `scenario_many`, `scenario_one`
with `SEED` and `SCENARIO_SET`, brains thinking with the map. The
dodge (`lib.rs` `Dodge::order_on`; callers `RuleBot::think_map`,
`Commander::think_map`). Before and after: `git stash` the fix, build
a frozen census binary, `git stash pop`, build another; nulls on seeds
0 to 47 with `CENSUS_ACTS`, naval.map, 900 s, respawns 3, the fair
census.

---

## IV. Results

Circling, 100 seeds: memory 0 holds 0; memory 2 to 60 s holds 100.
Ashore at 6 s with memory, 40 s without (seed of the file). Dealt
play after the `Commander::new` fix: LR 46's 16 per-seed differences
identical. Shell scenario: 97 of 100, then 1,000 of 1,000.

| Same 48 seeds | Before | After |
| --- | --- | --- |
| Deaths | 2,472 | 2,297 |
| Dead share | 42.5% | 40.6% |
| Ashore: by own shell, killed | 15.7%, 6.5% | 10.4%, 5.1% |
| Afloat: killed | 14.8% | 14.0% |

Tests: brain 77 passed and 10 ignored; client 31; net 14; proto 3; sim
50; no warnings.

---

## V. Discussion

1. **A run that cannot fail tests nothing.** Three versions of the
   landfall scenario passed with the memory on and off. Breaking each
   found a real behaviour: the binding first choice, a drone seeing
   for its tank, and the circle.
2. **A trace must sample faster than what it watches.** A 5 s trace
   reported the state after a 5 s crossing and produced a wrong claim.
3. **Many seeds find what one seed hides.** The dodge's water bug sat
   under every census since LR 27, averaged into the meter. Three
   failing seeds in a hundred pointed straight at it.

**Next:** backlog 42 (`duck`), or the boat's dodge (31) with a
scenario.
