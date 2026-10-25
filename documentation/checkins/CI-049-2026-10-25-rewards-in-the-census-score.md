# Rewards in the Census Score

**Check-in report CI-049.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 84; roadmap backlog
66, 67 and 68; `ledger.rs`, `fleet.rs`.

---

## Abstract

The census score now pays for pillbox fighting and gunnery. It counts
a seat's direct hits on hostile tanks, its damage to hostile
pillboxes, the neutral and hostile pillboxes its squad takes, its kills
near a pillbox of its team, and a bonus when its team took every
neutral pillbox. Each is a weight in `ScoreWeights`, and
`CENSUS_REWARDS=0` scores as before. A test checks each count; with
the rewards off, an atlantic census reads as LR 83's to the digit. As
first set they were 0.2 to 0.7 a seat against a score of about 17;
Paul chose about a third of the score, so each was tripled. The
personalities and the pillbox mechanics Paul described are logged as
backlog 67 and 68.

**Index terms:** census, scoring, rewards, pillboxes.

---

## I. Introduction

### A. Intent

The first of Paul's three asks: a reward system in the census. The
personalities and the pillbox mechanics come after it, in their own
steps.

### B. The prompts

> ok. so some alterations for different personalities. And I think We have personalities that prefer to collect pillboxes, personalities to want to get tight groupings of pillboxes, and bases. Right now we are a seeming to bave a balanced personality. I want to create some personalities that want to get their goal accomplished a bit more than normal. Anohter personality is to have coyboy tankers who always spend the expensive ammo with long shots and using all the resources as fast lauching like artillerty. We might have new functions to place at the end of the brainz, but I would like to add them all and make sure these ambitions of personalities are obvious. Such that timid or agressive would be very vissible . We also need to create a better scoring system for census. Add rewards of score when pillboxes are taken, defended. Pillboxes shoot and assit tank right. They shoot ammo rounds. when pillboxes take a shot they get damage. pillboxes that are damaged fire slower. pillboxes then need repair with tree resource. Pillboxes dont die all the way, they be come highly damaged and still functional. Damaged pillboxes can be dragged. If a pill box is dragged through wood (orest) it repairs itself. we want to improve Census with a reward system . the reward system can help each personality and its coefficients .  Big points for capturing all neautral pillboxes . Good points for taking enemy pillbox. Good points for doing damage to enemy pillbox. Getting a direct hit with the main tank weapon is a very good amount of points for reward. A defensive front line personality which would be the defense of an own base or pillbox near an enemy pillbox. Pillboxes can fight against eachother.

> ok. start with the reward system

> ok go

> (to the question of the rewards' size) About a third (Recommended); (to commit) Commit and push

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "ok" to the question before | LR 83 committed and pushed (`4c85d1b`) | — |
| D2 | The prompt mixes rewards, personalities and sim mechanics | The rewards first, as Paul said; the rest backlog 67 and 68 | All at once: one step at a time |
| D3 | The sim's mechanics differ from Paul's description in places | Read and reported (LR 84), nothing changed | Change the sim to match in this step: a second change, and Paul's to choose |
| D4 | "Either/or is a scalar" | Each reward a weight in `ScoreWeights`; `CENSUS_REWARDS=0` the old score | Replace the score: no way back to compare with LR 8 to 83 |
| D5 | The sim's events carry what each reward needs | Counted in `Tally::hear` | New sim events: the existing ones suffice |
| D6 | A pillbox lifted and set down is another id | "Every neutral" counted from captures, not ids | Track ids: a fetched pillbox would never count |
| D7 | "Defended" | A kill within 16 tiles (twice a pillbox's reach) of a live pillbox of the killer's team | Damage taken near one: noisier, and a kill is what ends the threat |
| D8 | "Very good" and "big" points, with no numbers | First a direct hit 0.5, a pillbox's health of damage 0.5, neutral 1, hostile 2, a defence 0.5, every neutral 5 | — |
| D10 | F: 0.2 to 0.7 a seat against about 17; Paul asked: "About a third" | Each tripled: 1.5, 1.5, 3, 6, 1.5, 15 | Keep as set, or lead the score: Paul's choice |
| D9 | The team line is read by scripts by its start | The rewards appended at the line's end | A new line: more for the scripts to pair |

---

## III. Methods

`ledger.rs`: `Outcome` gains the counts (serde defaults, so old ledger
lines read); `ScoreWeights` the six weights, `without_rewards`;
`rewards` the part; `score` adds it. `Tally` keeps the shells in flight
by owner and the neutral pillboxes' count and each team's taken.
`fleet.rs`: `CENSUS_REWARDS`, the line. The checks:
`cargo test -p treadcore-brain --release`; `CENSUS_REWARDS=0 SEEDS=4
SEED0=500 SECS=300 MAP=maps/world/atlantic.map JOBS=4 make census`
against LR 83's census of the same seeds (the team lines up to
"rewards", sorted, `diff`); the same with the rewards on.

---

## IV. Results

In LR 84: the test; the old score to the digit; the rewards as first
set 0.2 to 0.7 a seat on four atlantic seeds of 300 s, against a score
of about 17; tripled at Paul's choice.

---

## V. Discussion

What the rewards are worth decides what the census prefers. As first
set, held ground led the score and the rewards were a nudge; tripled,
pillbox fighting and gunnery are about a third of it. The census reads a difference
between teams, so the term that matters is the one that varies, and a
full census with the counts will show which of these do.
