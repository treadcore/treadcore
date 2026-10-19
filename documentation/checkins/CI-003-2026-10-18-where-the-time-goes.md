# Which Rungs the Lean Moves, and Where the Time Goes

**Check-in report CI-003.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 18 October 2026.
Companion records: journal 0051; `LAB-REPORT-BRAIN.md` section 26;
CI-001, CI-002.

---

## Abstract

CI-002 found that leaning the Auto ladder by the team's posture costs
about a point a seat, whichever way the posture is computed. This step
instrumented the brain to show which rungs the lean moves and where a
team's time goes, with no change to play. The lean moves tanks from
Fortify to Expand (84% of changed picks), and turret time falls 88%.
Economy's score feeds back on building: the castle spends material, and
spent material reads as Economy. Turning Economy off restores half the
turret time but none of the score (+0.13 ± 0.47), so the hypothesis is
not confirmed. The larger finding: on naval.map the ladder decides only
5.6% of a team's seat-seconds; the island hop takes 19%, and 46.7% is
spent dead. The census as set measures attrition, and the levers worth
pulling are survival and the crossings, not the ladder.

**Index terms:** instrumentation, time budget, utility AI, attrition,
experimental design.

---

## I. Introduction

### A. Intent

To finish backlog 27: show which rungs the lean moves, test the
hypothesis that Economy's lean costs the point, and decide what the
leanings should become.

### B. The prompt

> yes. commit. push. start backlog

"Start backlog" was read as the next item CI-002 named, backlog 27, as
CI-002 section V and its "Next" line set out.

### C. State at the start

Commit `61d0c41` (CI-002, journal 0050), pushed, no tag. `posture` off;
scored posture -1.04 ± 0.58, ladder's posture -0.81 ± 0.65, on the same
48 seeds.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Backlog 27's proof: "seat-seconds on each rung, with the lean and without" | `Brain::rung` and a per-rung count in the census, plus the meter's from-to matrix of changed picks | Inferring rungs from status words: LR 22 made codes the only thing logic and measurement read |
| D2 | A rung kept from an earlier think would be counted when the ladder was not reached | `rung_now`, cleared at the start of every think, set only when the ladder picks | Reading `Commander::rung`: it is the last rung taken, kept across reflexes and jobs |
| D3 | The first launcher passed its label to `env` as a command, and nothing ran | A small script taking the label apart from the variables | Re-reading results from a run that never happened |
| D4 | The summary line printed no moves: the census total did not add the new matrix | Add it to the total; sum this run's per-match lines for the report | Re-running 144 matches for a figure the per-match lines already held |
| D5 | F: Fortify to Expand 84%, Turrets -88%, Economy causing two thirds | A causal test with no code: `w_pos_economy` 0 in a weights file, the same 48 seeds | Changing Economy's score first: the weight answers whether Economy matters at all, for free |
| D6 | F: Economy off restores half the turret time and none of the score | Hypothesis not confirmed; the leanings stay as they are | Tuning the 5 x 6 leanings: the census cannot see the point they might be worth |
| D7 | F: the ladder held under 7% of seat-seconds by the per-rung count | Count every seat-second by status code (`CENSUS_ACTS`) | Assuming the rest was fleet slots: measuring was cheaper than guessing |
| D8 | F: 46.7% dead, 19% hop, 5.6% ladder | Backlog 28 (survival) and 29 (a census where late parts act) ahead of the tuner | More ladder work: it can move at most 5.6% of the time |

---

## III. Methods

### A. Census

As in CI-001 III and CI-002 III.A: naval.map, 900 s, respawns 3, the
fair census, 48 seeds in three ranges of 16 (`CENSUS_SEED0` 0, 16, 32)
side by side, `board` on for every team. The three runs:

| Run | Trial | Settings file |
| --- | --- | --- |
| no lean | `board` (a null run) | none, `CENSUS_CAPS=` |
| lean | `board, posture`, `w_posture` 0.5, scored | `scored.toml` (CI-002) |
| lean, Economy off | as lean, `w_pos_economy` 0 | `noecon.toml` |

The time-share run was a null run with `CENSUS_ACTS=1`, 16 seeds in
ranges of 6, 6 and 4.

### B. The instruments, function by function

| File | Function or field | What it does |
| --- | --- | --- |
| `lib.rs` | `Brain::rung` (new, default None) | the Auto rung this think took, if it reached the ladder |
| `commander.rs` | `Commander.rung_now` (new), `think_on`, `auto`, `impl Brain::rung` | cleared at each think, set when the ladder picks, returned to the census |
| `meter.rs` | `Meter.moved` (new), `meter::posture(p, first, taken)` | each changed pick by the rung that held first and the rung taken; printed most common first |
| `fleet.rs` | `brain_census`: `rungs`, `rungs_total`, `rung_line`; `CENSUS_ACTS` | the trial team's seat-seconds a rung, per match and in total; every seat-second by `Act::code`, dead seats apart, ladder thinks marked `[ladder]` |

Sampling is once a second of game time, for each trial seat, after the
world steps. A seat whose tank is not alive counts as dead whatever its
last code.

### C. The causal test (D5)

If Economy's feedback on building caused the point, then removing
Economy, which changes only which posture is read, must recover it. The
test compares Economy on and off seed by seed, on the same seeds,
against the same baseline.

---

## IV. Results

### A. Rungs (48 seeds, trial team seat-seconds)

| Rung | No lean | Lean | Lean, Economy off |
| --- | --- | --- | --- |
| DefendBase | 609 | 381 | 422 |
| Fortify | 14,262 | 9,967 | 10,609 |
| Turrets | 8,427 | 1,026 | 3,961 |
| Hunt | 14 | 245 | 151 |
| Expand | 9,197 | 11,034 | 8,928 |
| Guard | 0 | 31 | 34 |
| Picks changed | 0 | 131,278 | 44,975 |
| d̄ a seat | 0 | -1.04 ± 0.58 | -0.92 ± 0.54 |

Changed picks under the lean: Fortify to Expand 84.4%, Turrets to Expand
11.6%, Fortify to Hunt 3.6%, Fortify to Guard 0.3%. Economy off against
on, seed by seed: +0.13 ± 0.47.

### B. Time (16 seeds, null run, 205,537 trial seat-seconds)

Dead 46.7%; the island hop 18.9%; rotation 7.3%; the ladder 5.6%;
dodging 4.5%; the fleet's wedge 4.2%; rallies and kiting 3.9%; taking
depots and pillboxes outside the ladder 2.4%; crew jobs 1.7%. The full
table is LR 26. The trial team died 17.1 times a match, 3.4 a seat,
against 3 respawns and a 20 s respawn wait.

### C. Tests

`make test`, every crate: brain 68 passed and 8 ignored; client 31; net 14; proto 3; sim 50. No
warnings.

---

## V. Discussion

1. **The mechanism was found; the cost was not.** The lean takes tanks
   off the castle, and Economy's feedback on building is a real design
   fault. But removing it does not recover the point. Either the point
   is noise shared by runs on the same seeds, or it comes from the
   Fortify-to-Expand moves that the Expand posture still makes.
   Neither can be told apart at 48 seeds.
2. **Measure the budget before tuning a part.** The ladder was the
   object of three steps of work (LR 24 to 26), and it holds 5.6% of
   the time. A time budget by status code should be the first
   measurement of any part, and is now one switch away.
3. **The census measures attrition.** With half the seat-time dead,
   parts that pay late are cut off, and every census since LR 11 has
   been about who dies first. Backlog 29 tests this directly.
4. **On the architecture (CI-001 A1).** Over 90% of a team's living
   time is decided by the chain of branches above the ladder: the hop,
   rotation, the dodge, the wedge, rallies. Those are where weights,
   blends and censuses will pay, and where the first-match order of the
   chain is itself a design question.

**Next:** backlog 28, what a seat was doing when it went down; then
29, the census with unlimited respawns.

---

## Appendix A. Reproducing this report

```
# lean, Economy off, and the null: r27.sh LABEL VARS..., three ranges of 16 seeds
./r27.sh null CENSUS_CAPS=
./r27.sh lean CENSUS_WEIGHTS=scored.toml
sed 's/^w_pos_economy = 1/w_pos_economy = 0/' scored.toml > noecon.toml
./r27.sh noecon CENSUS_WEIGHTS=noecon.toml
# the time budget
CENSUS_ACTS=1 CENSUS_SEEDS=16 CENSUS_BASE_CAPS=board CENSUS_MAP=$PWD/maps/naval.map \
  CENSUS_RESPAWNS=3 CENSUS_LEDGER=0 cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture
```

`r27.sh` runs `brain_census` three times side by side with
`CENSUS_SEED0` 0, 16 and 32, `CENSUS_SEEDS=16`, `CENSUS_BASE_CAPS=board`,
naval.map, respawns 3, 900 s, the ledger off.
