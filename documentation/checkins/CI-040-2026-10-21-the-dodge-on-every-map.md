# The Dodge on Every Map

**Check-in report CI-040.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0097; `LAB-REPORT-BRAIN.md` section 71;
CI-001 A8; roadmap backlog 25.

---

## Abstract

Every census since LR 11 played naval.map alone (CI-001 A8). The
census now plays a list of maps, `MAP=all` being all eleven, and
reads a difference on each beside the pooled one. Its first reading
puts the dodge, on against off, over 123 seeds of the 176 planned:
+6.00 +/- 0.48 a seat, 12.5 errors. Ten maps of eleven are past two
errors for the dodge, and none against it. The dodge holds by the rule
set before the run. Naval.map is mid-table, so the one map did not
overstate the part.

**Index terms:** brain census, maps, paired difference, standard
error, dodge.

---

## I. Introduction

### A. Intent

Backlog 25: does a part's difference on naval.map hold on other maps?
The dodge, the largest part (LR 29, LR 63), is the first one asked.

### B. The prompts

The census was built and launched earlier in the session. Paul then
stopped some runs and asked:

> i just cancled some drive runs mid process. do you have record o the last run. does it give any insite?

The census in `build/census/20261021-193635` had not been stopped;
it was read part-way (D2). Then:

> ok. so update. commit. push. i like this.

> treadcore build census keeps running. whats its intention.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-001 A8: one map | `CENSUS_MAP` a list or `gen`; seed n on map n mod k, so jobs keep a seed's map | A map per job: a job's maps would differ with JOBS |
| D2 | Paul asked for insight before the census ended | Read part-way from the per-match team lines; the census's own `pooled.txt` stays the record | Wait an hour, or stop it: he asked for it now |
| D3 | "update. commit. push." | Write LR 71 on 123 seeds, marked part-way, and push | Hold the write-up for 176 seeds |
| D4 | F: MAPS.md's paragraph on how matches end came from one seed a map and is wrong on naval and the fortress isles | Corrected from the 372 matches | — |
| D5 | F: a comment named `scripts/maps.sh` | Named `scripts/pool.sh`, the script that pools | — |
| D6 | A cached `.pyc` in the tree | Removed; `__pycache__/` ignored | — |

---

## III. Methods

**The census.** `scripts/census.sh`: the fair census
(`brain_census`), trial weights `caps = baseline` (every part on),
baseline `caps = baseline, -dodge`; `MAP=all`, which is the five
prior-art maps, the two world maps, industry, island, naval and `gen`.
Seeds 1000 to 1175 (16 a map), 900 s, 3 respawns, four jobs of 44
seeds from a binary copied when the run began; ledger off.

**The part-way reading.** For each seed, all three of its matches
played (each team in the trial seat once): the trial team's mean
score a seat, averaged over the three matches, less the baseline
teams' mean. Its mean and standard error over seeds, overall and by
map; past two errors is outside the noise. The census weights by
seat (teams of 6, 5 and 5). This reading weights by team line, so it
may differ from `pooled.txt` in the second decimal. The meter's dodge
counts were summed over every match played.

---

## IV. Results

| | Dodge on less off, a seat |
| --- | --- |
| Every map, 123 seeds | +6.00 +/- 0.48 |
| Highest | island +11.90 +/- 1.74; generated +9.44 +/- 1.53; industry +9.11 +/- 0.95 |
| naval (12) | +4.56 +/- 1.64 |
| Lowest | atlantic +2.23 +/- 0.84; fortressisles2 +1.26 +/- 0.94 (inside) |

Ashore, the dodged shell hits 2.6% of the time with the dodge and
22.8% without. Afloat it hits 0.5% against 4.6%, but deaths after
the shell fall less (8.9% against 12.5%). The full table is in LR 71.

---

## V. Discussion

1. **The dodge holds everywhere.** It is the one part so far past two
   errors on a single map. On eleven maps it is past twelve errors.
2. **Where it pays least.** It pays least on the island-chain maps,
   where more of the fight is afloat. This matches LR 29: afloat, the
   dodge changes little (backlog 31).
3. **The method.** Ten maps cost no more than one at the same seeds.
   Promotions can now ask for more than one map.

**For Paul:** the census is still running at the time of writing. Its
`pooled.txt` is the final word; the section can be updated from it.
