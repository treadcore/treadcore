# The Landfall's Three When Every Team Has Them

**Check-in report CI-016.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0065; `LAB-REPORT-BRAIN.md` section 39;
CI-015.

---

## Abstract

CI-015's reverse census read even where the forward read +2.31. This
step took an absolute reading: every team on the new defaults against
every team on the old (LR 27's null), on the same 16 seeds. Boats lost
on the run for the beach nearly halve (165 to 89), but deaths in a
dodge rise by about as many (354 to 434). The death rate is unchanged,
the matches run 4.5% longer, and the team spends 13% more time alive.
The deaths move ashore. This explains the asymmetry, and points the
next step at the dodge's untuned numbers.

**Index terms:** absolute evaluation, attrition, displacement,
defaults.

---

## I. Introduction

### A. Intent

Backlog 38: the three when every team has them.

### B. The prompt

> yes all three on. commit. push. continue backlog

(The same message as CI-015; this is its continuation.)

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The census sets only the trial team's weights | LR 27's null as the all-old reading; a null now as the all-new | A base-weights option in the census: not needed while the old defaults are a recorded null |
| D2 | Play at the old defaults unchanged since LR 27 | The two runs comparable on seeds 0–15 | |
| D3 | F: landfall down, dodge up, rate the same | The deaths move ashore; the defaults stay | Reverting: the matches are longer, as goal 5 asks, and no worse in deaths |
| D4 | F: dodge deaths 51% of the held-last codes | Next: the dodge's numbers to an honest tuner (23) | Another landfall part: its share is down to 10% |

---

## III. Methods

`CENSUS_ACTS=1`, every team on the defaults, `CENSUS_SEED0` 0, 6, 11
with `CENSUS_SEEDS` 6, 5, 5; naval.map, 900 s, respawns 3, the fair
census. Against LR 27's log (`build/lr27/null-acts.log`), the same
configuration at the old defaults. Absolute counts summed over the
three ranges. No pairing by seed, so no standard error.

---

## IV. Results

| Seeds 0–15, every team | Old | New |
| --- | --- | --- |
| Deaths | 819 | 848 |
| On landfall | 165 | 89 |
| In a dodge | 354 | 434 |
| Seat-seconds | 206,685 | 216,000 |
| Deaths per 1,000 seat-seconds | 3.96 | 3.93 |
| Dead share | 46.5% | 41.9% |
| Living seat-seconds | 110,615 | 125,471 |
| Mean score a seat | 8.38 | 7.95 |

---

## V. Discussion

1. **Saved tanks find the next danger.** The landfall's parts move
   deaths from the water to the shore, and the rate stays where it
   was. A relative census shows that as a gain only while the
   opponents still lose their boats.
2. **Absolute readings belong in every promotion.** Deaths, living
   time and match length sit beside the relative score from now on.
3. **The dodge is now half the deaths.** It pays (CI-006), and its four
   numbers were first-guess constants. That is a tuning problem, and
   the tuner (backlog 23) has to be made honest, against the fair
   census and its error, before it is trusted with them.

**Next:** backlog 23 on the dodge's numbers.
