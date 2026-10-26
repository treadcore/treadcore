# The Morning Read: Families, and the Point Read Again

**Check-in report CI-054.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 88 (its morning
part); roadmap backlog 69; `build/night-am/` on the Mac,
`build/reread/` on the census machine.

---

## Abstract

The night's SPSA point passed its held-out rule by 0.002 of an error.
The rule's second guard, `make families`, passed: all nine hold, the
duel 5 points lower, inside the noise. Because the pass was so narrow
and the point lay far outside the range the screen read, one more
census read it on 400 fresh seeds, its rule written first: +0.09 +/-
0.27, inside the noise. The point is not promoted; no default changes.
The second read caught what the first held-out read let through.

**Index terms:** brain census, SPSA, held-out read, replication,
scenario families.

---

## I. Introduction

### A. Intent

Fetch the night's results from the census machine, score them, and
take the SPSA point through the rest of its rule.

### B. The prompts

> good morning. remember our last conversation. what do you say>

> yes. conenct and get reults.

> yes. start with families

> yes. i love the work you are doing. give yourself a tip.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | No session memory; the repo is the record | Read journal 0100, CI-053, LR 88 first | — |
| D2 | The machine's name is in no record; `~/.ssh/known_hosts` | Probe the LAN hosts read-only with `BatchMode`: the machine is `saskatchewan`, 192.168.8.235 | Ask Paul: two probes cheaper |
| D3 | z 2.002, a pass by 0.002 | Report it as at the bar, not as a clear pass | — |
| D4 | The night's rule: families in the morning with Paul | `make families SET=` the five on the Mac | — |
| D5 | Families at the point alone show no change | Families at the defaults beside it (30 s) | — |
| D6 | A pass at the bar from a point outside the screen's range; LR 74 | A second read, 400 fresh seeds from 400000, rule and expectation in `build/reread/NOTES.txt` before it ran | Promote on the rule's letter: LR 74's lesson unheeded |
| D7 | The second read inside the noise | Not promoted; defaults stay | — |
| D8 | No code or default changed | No CHANGELOG entry (LR 87 and 88 the precedent) | — |

---

## III. Methods

On the Mac at 7c97aa9:

    make families SET="strikes_max=46.6825 longshot_every=11.0390 \
      w_focus=0.4439 mass_for=52.3568 castle_inside=18.6594"
    make families

On the census machine at 7c97aa9, in tmux:

    TRIAL=build/reread/theta.toml SEEDS=400 SEED0=400000 MAP=all \
      JOBS=24 make census

The census in `build/census/20261026-073751`; every log ends in `test
result: ok`. z to four places from the logs' "a seed:" lists, as
`scripts/pool.sh` reads them.

---

## IV. Results

| Read | Every map | z |
| --- | --- | --- |
| held-out, 300000 (the night) | +0.597 +/- 0.298 | 2.002 |
| read again, 400000 | +0.088 +/- 0.267 | 0.33 |
| the two pooled, 800 seeds | +0.34 +/- 0.20 | 1.71 |

Families: all nine hold; the duel 73.5% +/- 2.2% at the point, 78.5%
at the defaults; the rest unchanged. Fortressislesx below both reads
(-1.33 +/- 0.66 pooled). Per map in LR 88.

Errors: none in the runs. The first draft of LR 88's morning part
miscounted the expectations (13 of 22; 12 of 19) and claimed a step
scaling for SPSA not checked; both corrected before commit.

---

## V. Discussion

The rule as written would have promoted a point worth nothing. A
single held-out read at two errors is right about 97 times in 100
when there is nothing to find, but SPSA points are chosen for reading
high, and the bar meets that bias at its weakest. Two proposals for
Paul (LR 88): promote an SPSA point only after two held-out reads, or
one of 800 seeds; and bound SPSA to the range the screen has read.
