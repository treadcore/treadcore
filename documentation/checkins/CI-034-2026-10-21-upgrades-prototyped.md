# Upgrades Prototyped: Trace 2

**Check-in report CI-034.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0091; `PROPOSAL-UPGRADES.md` section 8;
CI-033; roadmap goal 5, 11g. Branch `proposal/upgrades`.

---

## Abstract

Goal 5, 11g. The upgrades proposal's trace 2, bots buying, needs a
build. A prototype of four of its sixteen upgrades (pillbox calibre
and plating, depot stores, tank plating), with supply, `Command::Buy`
and a buying rule for the bots, was built on a branch, off unless the
scenario asks, and kept outside the world's hash so no default match,
replay or wire message changes; a test holds the hash equal tick for
tick with the economy on and nothing bought. Over the same six naval
matches as trace 1, the bots buying lengthen matches from 18.0 to 21.8
minutes, one of six running to the 30-minute limit. Of 195 purchases,
three in four are a tank's plating, bought again after every death;
the winning side bought the most in every decided match. The proposal
gains section 8. Main is unchanged.

**Index terms:** upgrades, supply, prototype, trace, governance.

---

## I. Introduction

### A. Intent

To run the proposal's trace 2, the one that needs buying, without
changing the rules on main before the discussion.

### B. The prompt

> continue

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-033: trace 2 next | A prototype | Waiting for acceptance: the proposal asks for the trace before it |
| D2 | A rules change before the discussion | On a branch, `proposal/upgrades`; main untouched | On main behind a flag: a rules change merged before acceptance |
| D3 | F: the hash is bincode over `World`; a new field moves every replay | The economy and `Scenario::upgrades` `serde(skip)`, beside the world | In the world: wire 17, the proposal's build, not the prototype's |
| D4 | `Command::Buy`, `Event::Bought` | Appended last to their enums, so earlier variants encode as before | — |
| D5 | Sixteen upgrades | Four applied at once: calibre, pillbox plating, stores, tank plating | The installed ones: the squad's tasks, terrain; a step each |
| D6 | F: an event from `damage` outside a step is cleared before earning reads it | A tank's upgrades go whenever it is not alive | Only on `TankDestroyed`: misses a kill made between steps |
| D7 | F: the workshop's mend capped armour at 100 | It no longer lowers armour above that | — |
| D8 | Either/or is a scalar | `upgrade_vs_mend` (0.5, already in Settings): buy when supply reaches the price over it | A yes/no to buy |
| D9 | Posture leans the improvement slot (proposal 1.3) | The base's pick by the team's posture | `w_up_*` weights: after the discussion |
| D10 | Trace 1's log has no deaths | Trace 1 run again on the branch (hash-equal matches) beside trace 2 | — |

---

## III. Methods

**Sim** (`treadcore-sim`): `upgrades.rs` (`Upgrade`, prices and
levels, `Economy`, `SUPPLY_SCALE` 1.0, `UPGRADE_BASE_R` 10,
`UPGRADE_BUY_REACH`); `Scenario::upgrades` and `World::economy`
(`serde(skip)`); `Command::Buy`, `Event::Bought`; `World::buy` (the
checks of section 1.3; refusals named in `chair.rs`); `step_economy`
at the end of `step` (section 1.1's earning; a tank's upgrades dropped
when it is not alive); `pillbox_range_of`, `pillbox_health_of`,
`tank_armor_of` (today's constants without the economy) read at the
pillbox's fire, repair and build, and the depot's mend; stores halve
the depot's resupply period; `Observation::supply` and `upgrades`
(`serde(skip)`). Tests: `supply_is_earned_from_what_a_side_holds`,
`an_upgrade_is_refused_without_the_supply`, `calibre_reaches_further`,
`a_tank_upgrade_is_lost_with_the_tank`,
`the_economy_changes_nothing_until_a_side_buys`,
`levels_and_prices_follow_the_proposal`. The server and the window
narrate `Bought`.

**Brain**: `Commander::buy`, once a second after the think (section
8's rule).

**Trace**: `fleet::tests::supply_trace` with `SUPPLY_BUY` 0 and 1, six
seeds, naval.map, 3 respawns, 30 minutes at most; logs
`build/supply/trace-buy0.log`, `trace-buy1.log`.

---

## IV. Results

As PROPOSAL-UPGRADES.md section 8: matches 18.0 to 21.8 min; deaths
317 to 343 (2.9 to 2.6 a minute); 195 purchases, 146 a tank's plating;
the winner the biggest buyer in 5 of 5 decided matches. Tests: brain 84
passed and 12 ignored; client 32; net 14; proto 3; sim 56. Eight
families above their floors.

---

## V. Discussion

1. **The aim is met in the trace**: longer matches.
2. **The tank's plating dominates**, because death takes it; the
   discussion should decide whether that is the cost of death the
   proposal meant or a drain.
3. **Supply follows the leader**, now in purchases too.

**For Paul:** the branch is local and not pushed; main has none of
it. Merging waits for the proposal's acceptance. The post draft in
`build/supply/discussion-post.md` now carries trace 2.
