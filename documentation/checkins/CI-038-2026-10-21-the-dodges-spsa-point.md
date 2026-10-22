# The Dodge's SPSA Point on Today's Brain

**Check-in report CI-038.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0095; `LAB-REPORT-BRAIN.md` section 69;
LR 43; roadmap backlog 23, 12g.

---

## Abstract

12g's tuning took the strongest lead on record: LR 43's SPSA end
point for the dodge (horizon 2.05, margin 1.52, clear 3.73), +0.88
+/- 0.51 over 96 seeds and not promoted. Read again on today's brain
over 192 fresh seeds, with a rule set before the run (promote past
two errors), it reads +0.25 +/- 0.32, 0.79 errors. The first job of
48 reads +0.88, as LR 43 did; the other three read nothing. The meter
shows the point dodging twice as often and hit by the dodged shell
three times as often, for no score. The defaults stay; the lead is
closed.

**Index terms:** tuning, SPSA, the dodge, census, promotion rule.

---

## I. Introduction

### A. Intent

Paul's choice: tune (12g). The step settles the one lead the tuner
left open rather than starting a new walk.

### B. The prompt

> continue with 12g tuning

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Backlog 23: LR 43's point at 1.74 errors, unsettled | Read that point again before any new walk | A new SPSA walk: the old lead first, and a walk costs hours of noise |
| D2 | The brain changed since LR 43 (LR 53 on) | The census on today's brain, against today's defaults | Pooling with LR 43's 96 seeds: a different brain |
| D3 | CI-001 VI.B: the tuner selects noise | Fresh seeds 96 to 287, none read before; 192 of them | 48 seeds: the error near 0.6, too wide for a lead of this size |
| D4 | A rule chosen after the result is no rule | Promote only past two errors, set before the run | Promote on any positive mean |
| D5 | A census is a sum; a number can break a behaviour the sum hides | The five dodge families at the point first | The census alone |
| D6 | F: +0.25 +/- 0.32 | Not promoted; the defaults stay; the lead closed | A further 192 seeds: at 0.25 a seat the point is worth nothing worth having |

---

## III. Methods

**Families.** `make scenario S=... RUNS=n SET="dodge_horizon=2.05
dodge_margin=1.52 dodge_clear=3.73"` for each dodge family and the
ready gun.

**Census.** `build/lr69/run.sh`: the fair census (`brain_census`,
frozen binary), `CENSUS_WEIGHTS` the point, `CENSUS_BASE_WEIGHTS` the
defaults; naval.map, 900 s, 3 respawns; `CENSUS_SEED0` 96, 144, 192,
240, 48 seeds each, four at once. Pooled over the 192 per-seed
differences (mean and standard error).

---

## IV. Results

| Measure | Result |
| --- | --- |
| Dodge families at the point | 99.9 to 100%; the ready gun 83.5% (83.4% at the defaults) |
| Census, 192 seeds | +0.25 +/- 0.32 a seat (0.79 errors) |
| By job | +0.88, +0.56, -0.43, +0.00 (each +/- about 0.65) |
| Dodges ashore a team, point / defaults | 49,707 / 26,092 |
| Hit by the shell dodged, point / defaults | 997 / 316 |

No default moved; tests and families as CI-037.

---

## V. Discussion

1. **Chance, or a changed brain.** A 1.7-error reading comes about
   one time in twenty by chance; the brain has also changed since LR
   43. The census cannot say which; neither makes the point worth
   promoting.
2. **More dodging is not better dodging.** The point doubles the
   dodges and triples the hits by the shell dodged; the score is even (whether the extra dodges were of shells that
   would have missed is not measured).
   The dodge's numbers sit on a flat, as `landfall_memory` did (LR 41).
3. **What this says of 12g.** The census at 48 seeds cannot see a
   lead under about a point a seat; a tuner walking on it finds noise.
   The next tuning wants a measure nearer the part (a family's rate, the
   meter) or a part whose worth is larger.

**For Paul:** none blocking. Commits are local until you say push.
