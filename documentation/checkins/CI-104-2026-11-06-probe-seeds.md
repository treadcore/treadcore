# The Census's Probe Seeds: the Same Eight for Every Candidate, on the Maps That Tell Soonest

**Check-in report CI-104.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, night.
Companion records: `LAB-REPORT-BRAIN.md` section 155; journal 0132.

---

## Abstract

No seed was ever played by two candidates, so none could be ranked.
Now every census on every map plays eight probe seeds first, the same
numbers for every candidate, each on a map that tells a verdict soon.

## I. Intent

Paul, asked whether to order the census's maps and to give every census
the same first seeds: "Map order first, Probe seeds". Of the two maps
LR 151 added: "how much worse is it to do them all. and keep a history
and a passrate."

## II. Decision log

| Decision | Cause |
| --- | --- |
| Probes are part of SEEDS | a census costs what it did, near enough |
| Each probe chosen on a telling map | the map order and the probes in one change |
| Probes first, a seed a job | a sequential census's first look reads them |
| Only with MAP=all | the probe maps are all there; a one-map census is as before |
| The two new maps kept | Paul: keep them, with a history and a pass rate |

## III. Method

`MAP=all PROBE=3 SEEDS=6 SEED0=5 scripts/census.sh` on this Mac.

## IV. Results

| Seed | Map |
| --- | --- |
| 13000000002 (probe) | naval |
| 13000000017 (probe) | constanine |
| 13000000027 (probe) | island |
| 5, 6, 7 | fidos-airstrip, flame-war-modified, fortressisles2 |

## V. Discussion

Keeping all thirteen maps costs 18% more seeds for the same verdict;
their history (orrery's plan/maps-history.jsonl) says when that changes.

## VI. What is not built

The read of each probe seed as an exam item, until enough candidates
have played them.

## VII. Paul's decisions

Map order and probe seeds; all thirteen maps kept, with a history and a
pass rate.
