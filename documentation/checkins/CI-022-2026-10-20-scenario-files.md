# The Two Leads, and Scenario Files

**Check-in report CI-022.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journals 0075 and 0076; `LAB-REPORT-BRAIN.md`
sections 47 and 48; CI-020, CI-021.

---

## Abstract

Two steps. First, LR 44's two single-pass leads went to 48 fresh
seeds. `bridge_jobs` read +0.07 ± 0.41, so the lead was gone. `duck`
read +1.25 ± 0.52, past two errors but the opposite way from its lead.
Neither is confirmed, and both stay off. Second, backlog 24: playtests
as text. A scenario file sets up seats, pillboxes, depots, shells and
timed moves, runs headless and checks the end. Three hand-written tests
have scenario forms that agree with them, unmutated and under two
deliberate breaks of the code, and run in hundredths of a second.

**Index terms:** confirmation, regression to the mean, test design,
mutation testing.

---

## I. Introduction

### A. Intent

Backlog 39, then 24, as proposed at CI-021's close and in the answer
to Paul's "is there an improved and faster way for you to test ?".

### B. The prompt

> commit. push. continue backlog

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | LR 44: two single-pass leads | 48 fresh seeds each, the rule first | Acting on one pass |
| D2 | F: `duck` +1.25, the other way | Not confirmed; "duck helps" a new hypothesis (backlog 42) | Turning `duck` on: the rule asked for its lead's direction |
| D3 | No TOML crate in the tree | A subset read in the brain | A new dependency: a supply-chain choice for Paul |
| D4 | Backlog 24's proof | Three tests ported; both forms run unmutated and under two breaks | Ports only: a test that cannot fail proves nothing |
| D5 | Breaking source in a test | Backups and an EXIT trap restore on every path | The loop of CI-021 that left a file broken |
| D6 | The depot trace stopped at 60 s | The runner records the match's end and stops | Treating it as a bug: the Rust test does the same |
| D7 | Retire the ported Rust tests? | Kept; Paul's call | Deleting them unasked |

---

## III. Methods

`scenario.rs`: `parse` (the TOML subset), `run` (the world from
`Scenario::bots(players)` with no swarm and no time limit, the
placements, the brains, `seconds × 20` ticks, the events, and the
checks), `run_file`; tests `the_subset_reads_values_tables_and_lists`,
`every_scenario_holds`, and the ignored `scenario_one` (`make scenario
S=...`). Mutation A: `Dodge::order_with` returns None at once. Mutation
B: `Commander::go` returns at once. Each Rust test and scenario run by
name; the files restored by a trap.

---

## IV. Results

| | `bridge_jobs` | `duck` |
| --- | --- | --- |
| LR 44 lead | +1.68 ± 0.70 | −2.26 ± 1.05 |
| Seeds 96–143 | +0.07 ± 0.41 | +1.25 ± 0.52 |

| | Rust | Scenario |
| --- | --- | --- |
| Unmutated, three | pass | pass |
| A: shell | FAIL | FAIL |
| B: pillbox | FAIL | FAIL |
| B: depot | pass | pass |

Tests: brain 76 passed and 9 ignored; client 31; net 14; proto 3; sim
50; no warnings.

---

## V. Discussion

1. **Of four leads sent to fresh seeds, one held.** The census at 12
   seeds finds effects that a second look does not, about as often as
   chance predicts.
2. **Seconds, not minutes, for a behaviour.** A scenario answers "does
   it do the thing?" in hundredths of a second, so the census is spent
   only on "does it win?". This is the third of the six ways to test
   faster listed in CI-020 V.3.

**Next:** placements for the landfall's situations, and a census-sized
scenario mode; or backlog 42.
