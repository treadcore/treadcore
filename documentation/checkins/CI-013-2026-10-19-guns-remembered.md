# Guns Remembered for the Landfall

**Check-in report CI-013.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0061; `LAB-REPORT-BRAIN.md` section 36;
CI-008 to CI-012.

---

## Abstract

CI-012 found that in 46% of the boats lost on the run for the beach,
the team had seen the killer near there in the minute before, and lost
it. This step gave the RuleBot a memory of hostile tanks out of sight,
`landfall_memory` (default 0, Paul's choice), read by the landfall's
pricing. With the memory at 30 s and the two earlier landfall weights
on (`landfall_shy` 1, `landfall_wait` 10), the trial team gained 2.31 ±
0.58 a seat over 48 seeds, and its landfall deaths halved. This is the
first landfall result outside the noise. Which of the three weights
carries it is the next step.

**Index terms:** memory, partial observability, utility scoring,
ablation.

---

## I. Introduction

### A. Intent

Backlog 36, as proposed at CI-012's close.

### B. The prompt

> yes 0. commit. push. continue backlog

The answer to "the default for `landfall_memory` ... I'd make it 0".

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Paul: "yes 0" | `landfall_memory` 0; play unchanged | |
| D2 | CI-012: the gun was seen, then lost | A per-bot memory of hostile tanks by owner, last position and tick | A memory on the team board: the board is off by default and rebuilt each second |
| D3 | A memory must not outlive what the team can see | Dropped when seen again, when its time is up, or when its spot is in a teammate's sight with no tank there | A pure timer: a boat would shy from a beach a teammate is standing on |
| D4 | The earlier parts read only seen guns | `known_guns` feeds danger, heading, exposure and cover alike | Memory for one of them: the census then tests the knowledge, not one use of it |
| D5 | Roadmap 36's proof: the three together | One 48-seed run, all three on | Each alone first: four null parts already; the joint run tests whether the knowledge helps at all |
| D6 | F: +2.31 ± 0.58 | Confirmed; the defaults stay 0 for Paul; backlog 37 ablates | Moving the defaults: Paul's to choose, and the ablation first |

---

## III. Methods

### A. The part

| File | Function or field | What it does |
| --- | --- | --- |
| `lib.rs` | `RuleBot.landfall_memory`, `remembered` | the weight; (owner, last position, tick) |
| `lib.rs` | `remember_guns` (new) | each tick from `think_map`: drops expired, re-seen, and contradicted memories (a teammate within 0.9 × `TANK_VISION` of the spot), then records every seen hostile tank |
| `lib.rs` | `known_guns` (new) | the seen hostile tanks, then the remembered not seen now |
| `lib.rs` | `landfall_order` | `threats`, `guns` and `cover` built from `known_guns` |
| `commander.rs` | `set_settings` | copies `W::LandfallMemory` |
| `weights.rs` | `landfall_memory` 0 | seconds |

### B. The test

`a_gun_seen_and_lost_still_prices_the_run`: the geometry of CI-008's
test. The gun beside the near beach's run is seen, then absent 5 s
later. With `landfall_shy` 1: at memory 0 the near beach; at 30 the
far; at 30 but 31 s later, the near again.

### C. The census

`landfall_memory = 30`, `landfall_shy = 1`, `landfall_wait = 10` as
the trial weights; 48 seeds in three ranges; naval.map, 900 s,
respawns 3, the fair census, `CENSUS_ACTS`.

---

## IV. Results

| | Null, seeds 0–15 | All three, seeds 0–15 | All three, 48 seeds |
| --- | --- | --- | --- |
| d a seat | 0 | +1.84 ± 1.13 | **+2.31 ± 0.58** |
| Deaths | 819 | 747 | 2,298 |
| On landfall | 165 | 82 | 281 |
| Killer seen near there 10–60 s before | 76 | 16 | 59 |
| Dead share | 46.5% | 42.2% | |
| Stand-off share | < 0.07% | 0.17% | 0.16% |

By range: +1.84 ± 1.13, +3.07 ± 0.91, +2.00 ± 0.97. Tests: brain 73
passed and 8 ignored; client 31; net 14; proto 3; sim 50; no warnings.
360 functions, 121 undocumented, 81 tests.

---

## V. Discussion

1. **The knowledge, not the rule.** `landfall_shy` and `landfall_wait`
   each read null alone (CI-008, CI-010). With memory they are two of
   three weights in the best part measured since the dodge. A rule
   over what the brain sees can be right and still fail for want of
   what it should know.
2. **A death count, followed to its cause.** CI-004 named the landfall,
   and four reads (CI-009, CI-011, CI-012) located the cause: guns
   seen and lost. The part that followed from them is the one that
   worked.

**Next:** backlog 37, each weight back at 0 in turn; then Paul's
defaults.
