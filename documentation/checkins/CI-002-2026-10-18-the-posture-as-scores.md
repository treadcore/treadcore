# The Posture as Scores

**Check-in report CI-002.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 18 October 2026.
Companion records: journal 0050; `LAB-REPORT-BRAIN.md` section 25;
CI-001 (the step before).

---

## Abstract

CI-001 found that the team board's posture, which steers the Auto
ladder under the `posture` capability, was a first-match ladder on
thresholds, where the design (LR 9) called for scores. This step
replaced it with five scores, each from 0 to 1 and each with a weight.
A new scalar, `posture_scored`, blends them with the old ladder, so
the old posture stays exactly one setting away. A unit test shows each
posture winning on its own case. In three of the five cases the old
ladder would have said Fortify. Measured over the same 48 seeds as
CI-001, the scored posture read -1.04 ± 0.58 a seat, against -0.81 ±
0.65 for the ladder's: -0.23 ± 0.74 apart, no difference. Together the
two read -0.93 ± 0.49. That moves the suspicion from the posture to the
lean that overrides the ladder's order. `posture` stays off.

**Index terms:** utility AI, team posture, influence map, paired
comparison, standard error.

---

## I. Introduction

### A. Intent

To finish backlog 21, the next item CI-001 set out: the posture scored
as LR 9 designed it, measured against the ladder it replaces on the
same seeds.

### B. The prompt

> commit and push, then start backlog 21

### C. State at the start

Commit `45641fc` (CI-001, journal 0049), pushed to `main`, with no tag,
so no workflow ran. `posture` off and measured at -0.81 ± 0.65 with the
ladder's posture. The standing rules of CI-001 I.C hold.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The prompt; backlog 21's form (hypothesis, proof, cost) | Score the five postures from LR 9's table; measure at 48 seeds against LR 24's run | A new design of postures: LR 9's table already said what each should score on |
| D2 | Paul's rule, every either/or a scalar | `posture_scored` blends the ladder's pick with the scores; each score gets a weight | Replacing the ladder outright: the register keeps old methods one switch away |
| D3 | LR 9 Fortify: "an unfinished fort"; Connect: "road gaps" | `Plan::castle_left` and `Plan::slots_left`, passed to the board in `Home` by the member that rebuilds it | Reading the plan from the board: the plan belongs to the commander, and the board must not hold a borrow of it |
| D4 | LR 9 Economy: "low material" | Each member posts its squad's material; Economy is the share spent, or 1 when starved | Material counted from the rebuilding member's `obs.tanks`: posting is how the board already gathers readiness, so material takes the same path (whether an ally's squad is in every observation was not checked) |
| D5 | First draft: no hostile seen gave odds of `press_odds`, so Expand scored 1 at every quiet start and beat a planned castle | No hostile seen is no front: even odds, Expand 0 (LR 9: "front odds") | Keeping it and lowering `w_pos_expand`: that tunes around a wrong input |
| D6 | All seven board tests passed on the first run | Print each case's scores and the ladder's pick to see that each wins for its reason | Accepting a first-run pass unread |
| D7 | Two measurements on the same seeds | Compare them seed by seed, as well as each against the baseline | Comparing the two means alone: the pairing removes the seeds' share of the noise |
| D8 | The forms are no different, and both are about a point behind | Keep the scored posture as the default (LR 9's design, no worse) and `posture` off; backlog 27 counts which rungs the lean moves | Tuning `w_posture` down: the formula and LR 24 show it only stops the lean from acting |

---

## III. Methods

### A. Apparatus and census

As in CI-001 III.A to III.C: naval.map, 900 s, respawns 3, 16 seats,
the fair census, and the paired difference over seeds with its
standard error. Three ranges of 16 seeds (`CENSUS_SEED0` 0, 16, 32)
ran side by side. The trial was `board, posture` with `w_posture` 0.5
and `posture_scored` 1; the baseline was `board`.

### B. The parts touched, function by function

| File | Function or type | What it does now |
| --- | --- | --- |
| `board.rs` | `Home` (new) | what the rebuilding member knows of the base: `starved`, `castle_left`, `slots_left` |
| `board.rs` | `PostureScores` (new), `PostureScores::of` | the five scores; one by posture |
| `board.rs` | `Posture::ALL` (new) | the five in the ladder's order, the order ties go by |
| `board.rs` | `Member.material` (new field), `TeamBoard::post` | each member posts its squad's material as a share of full |
| `board.rs` | `TeamBoard::rebuild` | takes `Home` in place of `starved`; works out the ladder's pick as before, then the scores, then the blend |
| `board.rs` | `TeamBoard.scores`, `TeamBoard.ladder` (new fields) | the scores and the ladder's pick, kept for the screen, the census and the tests |
| `fortify.rs` | `Plan::castle_left` (new) | the share of the castle's items not done or skipped |
| `fortify.rs` | `Plan::slots_left` (new) | the share of pillbox slots (turrets, the road slot, posts) still empty |
| `commander.rs` | the board block of `think_on` | builds `Home` from its plan and base and passes it to `rebuild` |
| `weights.rs` | `posture_scored`, `w_pos_defend`, `w_pos_economy`, `w_pos_fortify`, `w_pos_connect`, `w_pos_expand` | the blend and the five weights, all 1 |

### C. The scores

With `T` the hostile worth within the base's 16-tile alarm (a full
tank 1), `m` the members' mean material share, and `o` the ratio of
allied to hostile tank worth in sight (1 if no hostile is seen):

```latex
s_D = \min(1, T / 0.5), \quad
s_E = \begin{cases}1 & \text{starved}\\ 1 - m & \text{else}\end{cases}, \quad
s_F = \text{castle\_left}, \quad
s_C = \text{slots\_left}, \quad
s_X = \operatorname{clamp}\!\left(\frac{o - 1}{\text{press\_odds} - 1}, 0, 1\right)
```

```latex
\text{value}(p) = (1 - k)\,[p = \text{ladder}] + k\, w_p\, s_p
```

The best value wins, ties in the order Defend, Economy, Expand,
Connect, Fortify; Fortify if every value is 0.

### D. The proof

`each_posture_scores_its_own_case`: sixteen seats in two teams, ours by
the base and theirs 80 tiles off, with full vision (`advantage`). Five
cases are built from the same world, each varying one input, and a
sixth puts a hostile tank at the gate. Each case must pick its
posture at `posture_scored` 1 and the ladder's pick at 0. The test
prints each case's scores (D6).

---

## IV. Results

### A. The test

| Case | Picked | The ladder's | Scores (D / E / F / C / X) |
| --- | --- | --- | --- |
| a castle just planned, even odds | Fortify | Fortify | 0 / 0 / 1 / 0 / 0 |
| the castle up, its slots half empty | Connect | Fortify | 0 / 0 / 0 / 0.5 / 0 |
| the castle up, the squads spent | Economy | Fortify | 0 / 0.8 / 0 / 0 / 0 |
| a starved plan | Economy | Economy | 0 / 1 / 1 / 0 / 0 |
| eight of ours against two in sight | Expand | Fortify | 0 / 0 / 0 / 0 / 1 |
| a hostile at the gate | Defend | Defend | |

### B. The census (48 seeds)

| Posture | d̄ a seat | SE | Ranges | Postures read (E / F / C / X / D) |
| --- | --- | --- | --- | --- |
| the ladder's (CI-001) | -0.81 | 0.65 | +1.21, -1.30, -2.16, -1.01 (12 each) | 16 / 47 / 19 / 18 / 0.4% |
| scored | -1.04 | 0.58 | +0.40, -1.46, -2.07 (16 each) | 37 / 43 / 6 / 15 / 0.5% |
| scored less the ladder's | -0.23 | 0.74 | | |
| both, averaged a seed | -0.93 | 0.49 | | |

The seed spread was 4.0 a seat. The two runs' per-seed differences
correlate at 0.28: a seed's start explains little of either.

### C. Tests

`make test`, every crate: brain 68 passed and 8 ignored (new:
`each_posture_scores_its_own_case`); client 31; net 14; proto 3; sim
50. No warnings.

---

## V. Discussion

1. **The posture was not the problem.** Two different ways of working
   it out give the same result within the noise. Finding A3 of CI-001
   was a real design fault, and fixing it changed which postures are
   read (Economy 16% to 37%, Connect 19% to 6%), but not the score.
2. **The lean is the suspect.** Both forms override the ladder's own
   order, and both are about a point behind. The ladder's order
   (DefendBase, Fortify, Turrets, Hunt, Expand, Guard) is the brain of
   17 October, tuned by play; a lean that lifts Expand over Fortify
   whenever material is low may simply be worse play. Backlog 27
   measures which rungs gain.
3. **Ranges read their own luck again.** The last 16 seeds alone were
   outside two errors (-2.07 ± 0.99); the 48 are not. Only the whole
   sample counts, and only on seeds chosen before the run.
4. **On the architecture.** The step added a `Home` read-out because
   the board cannot see the plan. That is finding A5 of CI-001 again:
   the brain's state is spread across the commander, the plan and the
   board, joined by whoever is first due. A team-level owner of the
   plan (the fleet commander, `crew.rs`) would be the natural place for
   these inputs. That is a larger change, for when the lean pays.

**Next:** backlog 27, which rungs the lean moves; then 23, the honest
tuner.

---

## Appendix A. Reproducing this report

```
CENSUS_DUMP=scored.toml CENSUS_SEEDS=0 CENSUS_CAPS=board,posture \
  cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture
for k in 0 16 32; do CENSUS_SEED0=$k CENSUS_SEEDS=16 CENSUS_WEIGHTS=scored.toml \
  CENSUS_BASE_CAPS=board CENSUS_MAP=$PWD/maps/naval.map CENSUS_RESPAWNS=3 CENSUS_LEDGER=0 \
  cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture > scored-$k.log 2>&1 & done; wait
cargo test -q -p treadcore-brain --release -- each_posture --nocapture
```
