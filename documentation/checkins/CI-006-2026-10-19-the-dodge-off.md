# The Dodge Off

**Check-in report CI-006.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0054; `LAB-REPORT-BRAIN.md` section 29;
CI-004, CI-005.

---

## Abstract

The shell dodge was the code held last at 43% of deaths (CI-004), and
holding it longer cost 1.18 a seat (CI-005). This step turned it off
for the trial team over 48 seeds: **-2.45 ± 0.67 a seat**, outside the
noise. The dodge pays. A rough counterfactual from the meter shows it
nearly halving the dodged shell's hits ashore and changing nothing
afloat. Backlog 28 closes. Backlog 31, a weight for dodging afloat, is
the next part.

**Index terms:** reflexes, ablation, counterfactual, paired census.

---

## I. Introduction

### A. Intent

To finish backlog 28 with the ablation CI-004 set.

### B. The prompt

> commit. push. continue backlog

Read as: record the dodge-off run when it lands, commit and push it,
and set the next backlog item.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | F: -2.45 ± 0.67, 3.7 errors | The dodge stays on; backlog 28 done | Reading the 43% death share as a cost: the ablation says the opposite |
| D2 | F: the meter still counted near-null dodges with `-dodge` | Read the code: `order_with` meters before `dodges` drops the order; use it as a would-be count | Calling the run broken: the trial team's moves were dropped, as the score shows |
| D3 | The would-be counts are mixed with the baseline teams' | Estimate undodged rates as 3 × run − 2 × null, labelled as a direction | Reporting the mixed rates as the trial team's |
| D4 | F: afloat, undodged ≈ dodged | Backlog 31: `dodge_afloat`, a weight (1, today's), after the either/or rule | Turning the boat's dodge off outright: a scalar lets the tuner find the value |
| D5 | D3's estimate | Backlog 31 starts with the meter by team, would-be apart from taken | Building `dodge_afloat` on an estimate |

---

## III. Methods

`CENSUS_CAPS=-dodge` (the trial team's `dodges` false; `Commander::
set_settings`), the defaults for the baseline teams. `CENSUS_SEED0` 0,
16, 32 with `CENSUS_SEEDS=16`, side by side; naval.map, 900 s, respawns
3, the fair census, the ledger off.

The part: `Commander::think_on` calls `self.dodge.order_with(..)`
(`lib.rs`), which starts the watch and meters, and then filters the
order by `self.dodges`; `holding` is likewise gated by `dodges`. With
the switch off, the meter therefore records each dodge the trial team
would have made and its outcome undodged.

Estimate: with the trial team's share of shells taken as one third,
run rate r = r_u / 3 + 2 r_d / 3, so r_u = 3r − 2r_d, with r_d the null
run's dodged rate (CI-004).

---

## IV. Results

| | d a seat |
| --- | --- |
| `dodge_hold` 1 (CI-005) | -1.18 ± 0.55 |
| `-dodge` | **-2.45 ± 0.67** |

By range of 16 seeds: -4.11 ± 1.10, -1.39 ± 1.14, -1.84 ± 1.19.

| | Dodged (null) | Undodged, estimated |
| --- | --- | --- |
| Ashore, hit by the shell | 16.6% | ≈30% |
| Ashore, killed | 7.4% | ≈11% |
| Afloat, hit by the shell | 7.6% | ≈8% |
| Afloat, killed | 16.6% | ≈14.5% |

---

## V. Discussion

1. **A death census overstates a reflex.** The code most deaths passed
   through is the part worth most. Before reading any reflex's share of
   deaths, the ablation has to be run.
2. **Ashore and afloat are two parts.** The same dodge saves a tank and
   costs a boat its crossing for nothing. That is an either/or, so it
   becomes a weight.
3. **Instrument by team.** The meter's per-world counts forced an
   estimate; per-team and taken-or-not counts make the next reading
   exact.

**Next:** backlog 31, the meter first; then backlog 30.
