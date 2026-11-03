# Games 7 and 8: Oil, the Workers, Supply, the Quartermaster, Trade, and the Logistics Gauntlets

**Check-in report CI-086.** Paul Richeson, treadcore.org; Claude Code
(Claude Fable 5.1) as the pair. 2 November 2026, night.
Companion records: `LAB-REPORT-BRAIN.md` sections 129 and 130; CHANGELOG,
games 7 and 8; the lab report of the economy on the site
(`reports/2026-11-02-the-economy.html`, from orrery's
`docs/economy-lab-report.md`); `HOW-TO-PLAY.md` section 6b; `MAPS.md`;
`scenarios/gauntlet/g13` to `g17`.

---

## I. Intent

Paul: "continue. stage 3 (workers in transit, the city's shapes, the
quartermaster, oil) and stage 4 (trade); map authors' own deposits; a
stocks page in the window." Then: "write lab report of the new economy.
create a series of gauntlets to train the AI (both). to reward trains
moving from oil refinery to factory, to port, to city. continue
implementation. commit. push. document." The plan's chapter 13, stages
3 and 4, after stage 2 (CI-085) the same evening.

## II. What was built

**Game 7.** Wells of oil as the deposits, pumped by the squad, unloaded
at the side's refinery; a refinery with oil halves the burn and is a
fuel stop, one without is neither; on a map with wells a depot's fuel
comes from a stocked refinery within thirty tiles, an oil a stop, else a
trickle. The workers: a smelter's shells and a depot's shells for its
pillboxes on a figure that walks the map's route, five at a time, one
leaving every ten seconds; a splash loses it, a hostile tank that drives
up takes it. Supply as PROPOSAL-UPGRADES 1.1 had it, spent as one side;
the depot's stores (`F6`, sixty): thirty of each kind, made twice a
second. The guard as quartermaster (`city.rs`, `city_plan`): a harvester
by the wood; the stores; the road to the smelter; the industrial zone
taken, the plant first; a refinery where there is oil; a data centre
and a workshop. The oil run beside the ore run. The top strip shows the
side's supply, shells, ore, oil and workers. The fleet's three maps carry
their own deposits and wells.

**Game 8.** Convoys of supply between a side's depots, one a depot a
minute to its nearest partner, by road or over the water between ports;
both ends paid by the distance; a convoy is a worker a raider takes. The
scripted brain covers a convoy of its own from a hostile near its road
and raids one of another side it sees.

**The gauntlets.** G13 ore to the smelter, G14 oil to the refinery, G15
the convoy comes through, G16 the raider takes the convoy, G17 the chain
to the city (deposit, factory, port, city). The scenario form places
deposits, wells and structures, knows `road`, and expects and rewards
`ore`, `smelted`, `oil`, `oil_in`, `delivered`, `convoys`, `taken` and
`lost`. Seventeen new senses for the learned tank; old nets and the
shipped transform widened.

## III. What was read

- Gauntlet, ten seeds: the scripted brain 10 of 10 on all five; the nets
  0 on the three that need the squad, 7 to 10 on the two that a gun or a
  wander passes. The table is LR 130 and the lab report's section VI.
- End to end, nine bots on the naval map for ten minutes, under game 8:
  3 smelters, 36 ore, 32 smelted, 63 workers left and 62 arrived, 2 stores
  bought, 7 city works, no tank at a dry depot past 5 s.
- Censuses of two seeds: supply 258 to 583 a team under game 7; under
  the first cut of trade 16,951 for one team, which is why trade is one
  convoy a depot a round at half the rate.
- Simulator 66 tests, brain 154, the workspace and its tests compile.

## IV. What is not built

The embargo; fuel fully zero-sum; the proposal's other upgrades; the
dock and the concrete walls; a stocks page of its own; a scenario's
`held` depot (G15 reads every brain alike without it); the nets' squad
head trained on the new gauntlets.

## V. Paul's decisions

Whether 120 ore and oil a deposit, a shell an ore, an oil a stop, sixty
for the stores, and the trade rates are the numbers or the search's;
whether the fuel trickle stays; whether `city_plan` works is the default
for every bots' team; the decrees of CI-084 still open.
