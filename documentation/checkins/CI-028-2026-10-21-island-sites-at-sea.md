# Island Sites Where a Naval Start Chooses

**Check-in report CI-028.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0085; `LAB-REPORT-BRAIN.md` section 62;
CI-027; roadmap backlog 20, 48.

---

## Abstract

Backlog 20. The island site score (forest, size; 12f2) had never acted:
it was read only when a tank had no base, and a fleet's base is its
leader's depot, so LR 24 and LR 44 found it idle. In a naval start the
first island is chosen by the beach a boat lands on, and the beach was
priced by its run and its danger alone. A new weight, `w_site_beach`
(20), takes the island's site score, in tiles of the run, off each
beach's cost while the side holds no depot. In a scenario family where
a bare islet is always the nearest beach and a large forested island
lies 15 to 33 tiles off, the boat lands on the island in 400 of 400
seeds, and in none at 0. The part acts in every census match, about 89
trial seconds a match. The guard census reads +0.82 +/- 0.69 a seat
over 48 seeds, inside the noise. Built counts do not differ.

**Index terms:** naval start, landfall, island site score, scenario
families, census guard.

---

## I. Introduction

### A. Intent

To take the next backlog item CI-027 named, backlog 20: make the
island site score act where a naval start actually chooses.

### B. The prompt

> continue with backlog 20

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt | Backlog 20 | — |
| D2 | F: the first island is the beach's choice (`landfall_order`); ashore Expand takes the depot nearest | The site read at the beach | At Expand's depot pick: by then the tank is on the islet, and a better island means a second crossing |
| D3 | Either/or is a scalar (Paul, 18 October) | `w_site_beach`, tiles of run per point of site score, 0 the old price | A yes/no: the trade is a run against a site, a scalar by nature |
| D4 | RuleBot has no map facts; Commander does | Commander hands the doctrine each beach's worth (`beach_worth`) each think | Moving `MapFacts` into the RuleBot: a larger change than the part |
| D5 | Only a naval start was the hypothesis | Only while the side holds no depot | Every landing: a tank with a base wants the beach near it, not a new island |
| D6 | F: the side held a generated depot; generated depots sat in the new water | `depots = false` in scenarios | Neutralising each depot by hand: the file depends on the map's draw |
| D7 | Scenario terrain knew water and grass | `forest` terrain | A map file: a scenario is text |
| D8 | F: 0, 40, 75.5, 100% at 0, 5, 10, 12.5 | 20, above the step with a margin; floor 0.95 | 12.5: the family's edge, no margin for danger terms |
| D9 | LR 58: count before you census | Meter of seconds afloat priced, read first (54 to 169 a match) | — |
| D10 | Paul's rule of 20 October | On by default; the census as guard | A gate on a gain: tuning follows the features (48) |

---

## III. Methods

**Code.** `weights.rs`: `SiteBeach` (`w_site_beach`). `commander.rs`:
`beach_worth(obs)`: empty unless alive afloat, `w_site_beach` nonzero,
`Caps::ISLAND_SITE`, map facts present and no depot of the side or an
ally; else `(beach, w * site_score(beach))` for each of `obs.shore`;
set on `inner.beach_worth` in `think_map` after the facts refresh.
`lib.rs`: `RuleBot::beach_worth`; `landfall_order`'s score less the
worth of the beach's tile. `meter.rs`: `beach_site()` once a second.
`scenario.rs`: `depots = false`; terrain `forest`.

**Scenario and family.** As section 62. Sweeps: `make scenario
S=scenarios/families/a-naval-start-by-a-bare-islet.toml RUNS=400
SET="w_site_beach=v"` for v in 0, 5, 10, 12.5, 15, 20, 30.

**Guard.** `build/lr62/run.sh`: a frozen binary; the fair census on
naval.map, 900 s, 3 respawns, seeds 0 to 47 in three jobs of 16; trial
the defaults, baseline `w_site_beach = 0`. Pooled difference: the mean
of the 48 per-seed differences, its error their standard deviation over
the square root of 48. Built: the census's per-match team lines, trial
against the rest.

---

## IV. Results

| Measure | Before | After |
| --- | --- | --- |
| The scenario | lands on the islet; refused building; sails on | lands on the island; a castle begun by 20 s |
| The family, 400 seeds | 0.0% | 100.0% |
| Seconds afloat with beaches priced, census | | 12,779 (about 89 a trial match) |
| The guard, 48 seeds | | +0.82 +/- 0.69 a seat |
| Built a team-match | 16.17 +/- 0.57 | 16.72 +/- 0.84 |

Tests: brain 83 passed and 11 ignored; client 32; net 14; proto 3; sim
50. 101 weights, 17 bits; twelve scenario files and seven families with
floors.

---

## V. Discussion

1. **A rule acts where the choice is made.** The site score was right
   and idle for three days because it sat behind a choice the fleet
   had already made; the choice of island is the boat's.
2. **A choice the map seldom poses.** naval.map gives each side islands
   fit for a base, so the census reads a small lean, not a gain.

**For Paul:** none blocking. Backlog 48, the starting numbers, is next
now that the feature list in CI-026 is built. Commits are local;
nothing was pushed.
