# Alone, the Step In Stays Shallow: the Kite Redesigned for a Tank Without Backup

**Check-in report CI-094.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026, late.
Companion records: `LAB-REPORT-BRAIN.md` section 145; journal 0121;
CHANGELOG; ROADMAP 87.

---

## Abstract

The rules passed G21 (alone, kite the one that shot you) on 24.5% of
its seeds. Read hit by hit, G21 was lost on the direct gun: the kite
waited on the mortar's ring, the mortar tied the gun up 12 s, and the
step in's window was open a quarter second of each enemy reload. Of
three designs, `kite_in_alone` was kept: with no friend near the
target, the step in ends six tiles from it. G21 rises to 35.5%
(+8.3 ± 2.6 points paired over 600 seeds), G22 is unchanged, the census
reads −1.12 ± 0.92 a seat, inside the noise, and the families hold.

## I. Intent

Paul: "its risky, we will recover. i think it indicates faster
training." Then: "good, commit when the census holds, then the ring
sensor." And: "write a lab report about these redesigns."

## II. Decision log

| Decision | Cause |
| --- | --- |
| G21 read hit by hit before any change | a third of the losses could be the test's and not the rules' |
| `drones = false` added to the scenario form | to read the tanks' own duel; it held 1 seed in 200, and showed kiting needs a recon's eyes |
| G21 keeps its drones | without them no rule can kite: sight and reach are both 10 tiles |
| `duck_alone` dropped | no effect: a gun first seen inside its reach cannot be ducked |
| Holding the mortar dropped | G21 24.5% to 19.0%: its splash hits count |
| `kite_in_alone`, not `kite_in` moved | `kite_in` 7 gained G21 a third and cost G22 a third: the boundary |
| 6, from a sweep and 400 held-out seeds | best on both; paired +7.0 ± 3.2 on the held-out alone |
| The enemy in G21 and G22 pins 2.25 | with 6 on every brain G21 read 34.5% and G22 50.5%: a yardstick must not move with the rules |
| G21's floor 0.25 | today's success, 35.5% less two errors of a hundred seeds |

## III. Method

G21 and G22 for the rules (`make scenario`, a commander in seat 0
with the dial in its own table), 200 seeds from 9,500,000; G21 again on
400 from 9,700,000, read seed by seed for the paired difference. The
rule before the census: not worse than two errors on 48 seeds of every
map, the families holding, the other gauntlets the same. Census on this
Mac, `JOBS=8`, the paste shim under bash. The families with
`SCENARIO_SET="kite_in_alone=6"`. The gauntlet with `GAUNTLET_WEIGHTS`,
then as built with the default moved. 165 brain tests.

## IV. Results

| `kite_in_alone` | 2.25 | 5 | 6 | 7 | 8 |
| --- | --- | --- | --- | --- | --- |
| G21, 200 seeds | 24.5% | 30.5% | 35.5% | 32.5% | 29.5% |
| G21, 400 held out | 24.5% | 25.0% | 31.5% | 28.2% | |
| G22, 200 seeds | 52.5% | 52.5% | 52.5% | 52.5% | 52.5% |

Paired, 6 against 2.25 over 600 seeds: +8.3 ± 2.6 points; deaths
48.5% to 40.2%. Census −1.12 ± 0.92 a seat (europe −11.76 ± 4.78 on
four seeds, the one map outside the noise). Families hold. Every
gauntlet floor holds.

## V. Discussion

The kite was right in its idea and wrong in two numbers that the game
had moved under it: the mortar's reload and the step in's depth.
Split by whether a friend is near, the depth is the boundary Paul
asked for.

## VI. What is not built

G22 taught to the rules; the ring as a sense for the nets; whether
the waiting ring should stay at the mortar's minimum.

## VII. Paul's decisions

None outstanding.
