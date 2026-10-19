# Where the Gun That Sinks the Boat Stands

**Check-in report CI-011.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0059; `LAB-REPORT-BRAIN.md` sections 32 and
34; CI-008 to CI-010.

---

## Abstract

Three landfall parts (CI-008 to CI-010) each assumed where the gun
that sinks a boat stands, and none moved the deaths. This step
measured it, with no change to play. Over 164 landfall deaths in 16
seeds, the killer stood a median 8 tiles from the boat, inside its
10-tile direct range and at the edge of the 10-tile sight. 38% stood
within reach of the boat's chosen beach, a median 20 tiles off and out
of sight; 60% stood nearer the boat than its beach, along the run; 9%
were afloat. A boat sees its killer only once the killer can fire.
CI-009's "boats are not blind" is withdrawn. The knowledge the team
does not use is the team board's hostile influence, which backlog 35
proposes to price.

**Index terms:** partial observability, sensing range, team knowledge,
measurement.

---

## I. Introduction

### A. Intent

To read where the gun stands, as CI-010 set, before a fourth landfall
part.

### B. The prompt

> commit. push. continue backlog

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-010 V.2: three parts, one premise | Measure the killer's place at each landfall death | A fourth part |
| D2 | The census cannot see the beach a brain chose | `Brain::beach`, default None, answered by RuleBot and Commander | Recomputing the beach in the census: it would not be the one chosen |
| D3 | F: killer to boat a median 8 tiles | CI-009's "not blind" withdrawn, marked in LR 32 | Leaving LR 32 unmarked |
| D4 | F: at the beach 38%, along the run 60%, both out of sight | Backlog 35: price the landfall by the team board's hostile influence, with a read first | Raising a boat's sight: a rules change, and a proposal (GOVERNANCE.md) |
| D5 | The either/or rule | `landfall_board` as a weight, its default Paul's | |

---

## III. Methods

### A. The parts, function by function

| File | Function | What it does |
| --- | --- | --- |
| `lib.rs` | `Brain::beach` (new, default None) | the beach a boat is running for |
| `lib.rs` | `impl Brain for RuleBot::beach` | `landfall`'s chosen beach |
| `commander.rs` | `impl Brain for Commander::beach` | the inner RuleBot's |
| `fleet.rs` | `brain_census`: `gun_at`, the summary line | at each landfall death with a killer: killer to boat, killer to beach, boat to beach, killer afloat; quartiles, and the shares within `DIRECT_RANGE + 4` of the beach and nearer the boat than the beach |

The killer's position is its tank's after the world's step on the
tick of the death; the boat's is the destroyed tank's.

### B. The run

Null run, `CENSUS_SEED0` 0, 6, 11 with `CENSUS_SEEDS` 6, 5, 5;
naval.map, 900 s, respawns 3, the fair census, `CENSUS_ACTS`.

---

## IV. Results

| | Median | Quartiles |
| --- | --- | --- |
| Killer to boat | ≈8.0 | ≈7.0–9.6 |
| Killer to beach | ≈29 | ≈4–37 |
| Boat to beach | ≈20 | ≈7–33 |

At the beach (within 14 tiles): 62 of 164 (37.8%). Nearer the boat than
the beach: 98 (59.8%). Afloat: 15 (9.1%). Null 0.00 on every seed.
Tests: brain 72 passed and 8 ignored; client 31; net 14; proto 3; sim
50. 358 functions, 121 undocumented, 80 tests.

---

## V. Discussion

1. **A sighting is worth something only from beyond the gun's range.**
   With sight equal to range, a boat's own eyes never give warning, so
   no rule over what it sees can save it.
2. **The team already builds the knowledge.** The board's hostile
   influence spreads each seen tank's pull over sectors and keeps it.
   It has been built and unread since CI-001. The landfall is the first
   decision a death count says it should feed.

**Next:** Paul's default for `landfall_board`; then backlog 35's read.
