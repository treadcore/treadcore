# Each Switch Alone, Without Running Out of Tanks

**Check-in report CI-020.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 20 October 2026.
Companion records: journal 0072; `LAB-REPORT-BRAIN.md` section 44;
CI-001, CI-016.

---

## Abstract

Backlog 29 asked whether the census, with half the seat-time dead,
hid the parts that pay late. Every capability off by default was
measured alone, at unlimited respawns and at 3, under today's
defaults, 12 seeds each. The base cycle and the escort read inside the
noise both ways: not confirmed. Three of 24 readings fell outside two
errors, where about one is expected by chance. Only `smooth`, negative
in all four readings on record, stands as a finding. `bridge_jobs`
(+1.68) and `duck` (-2.26) are leads for confirmation. On Paul's
choice, `smooth` becomes a weight with an insignificant default.

**Index terms:** multiple comparisons, attrition, ablation, census
design.

---

## I. Introduction

### A. Intent

Backlog 29, from CI-003: a census in which the late parts can act.

### B. The prompts

> commit. push. continue backlog. i love your analysis

> smooth sounds like we should remove it.

> ok. maybe not remove it. give it an insignificant coefficient

> is there an improved and faster way for you to test ?

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The defaults changed since LR 24 | Both passes under today's defaults | Reading unlimited against LR 24 alone: two changes at once |
| D2 | `RESPAWNS_UNLIMITED` is 255 | 255 | The roadmap's 254 |
| D3 | Paul: remove `smooth` | Shown it bundles six behaviours; offered split or removal | Removing unasked-about parts that may be neutral |
| D4 | Paul: "an insignificant coefficient" | Backlog 40, `w_smooth` 0.05; blend the numeric parts, threshold the yes-or-no ones at 0.5 | A switch kept off: Paul's rule makes an either/or a scalar |
| D5 | Paul: a faster way to test | Six ways, ranked; the frozen binary first (backlog 41) | More seeds alone: noise, not CPU, is the cost |
| D6 | 3 of 24 outside two errors | Only the consistent one (`smooth`) a finding; two leads to 48 fresh seeds (backlog 39) | Acting on single-pass readings |
| D7 | Brain edits wait for the runs | No source changed during either pass | |

---

## III. Methods

`RESPAWNS=255 JOBS=3 make switches`, then `RESPAWNS=3 JOBS=3 make
switches`: every capability off by default alone as the trial, the
defaults (the landfall's three on) for the baselines; `team_rotate`,
`bridge_jobs` and `posture` with `board` on for every team; 12 seeds
of 900 s on naval.map, the fair census.

---

## IV. Results

See LR 44's table. Outside two errors: `smooth` -3.32 ± 1.47
(unlimited), `bridge_jobs` +1.68 ± 0.70 (3), `duck` -2.26 ± 1.05 (3).
Never acting: `board` alone, `island_site`.

---

## V. Discussion

1. **The attrition hypothesis is not confirmed.** Given the full 900 s
   without running out, the late parts still read nothing.
2. **Read a table of tests as a table.** At two errors, one in twenty
   is outside by chance. Consistency across independent passes, or a
   confirmation on fresh seeds, separates a finding from a draw.
3. **Faster testing** (Paul's question): noise costs more than CPU.
   Freeze the binary so work goes on during a census; measure each
   part on its own counts; test single behaviours in scenarios
   (backlog 24); more cores; a cheaper match (finding L2); sequential
   rules set in advance.

**Next:** backlog 41, then 40, then 39.
