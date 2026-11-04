# The Score Counts Building and the Base

**Check-in report CI-088.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 139; CHANGELOG; ROADMAP
backlog 81; journal 0115; in orrery, `game_at` (the score's version).

---

## Abstract

The fleet's search had learned to score by building less: on 4
November every one of its 60 current candidates was held by the trade
guard, the smallest trade +1.27 a seat for 1.48 fewer things built a
team. The score now counts building at half a kill and counts the base:
repairs, wood felled near it, its pillboxes' life and kills, and its
guns ready. The score has a version, and the fleet reads anew under it.
On the same 48 seeds the new score reads w8466 0.42 ± 0.15 a seat lower
than the old one did.

## I. Intent

orrery asked what should happen with the held line (promote one by
hand, raise the floor to 2, keep holding, or count building in the
score). Paul, word for word:

> Count Building the score. give well defended pillboxes average time
> duration and longest best turrent survivial and its killrate and for
> bases, repair rate and usefull rate. in fact, workers mining forrests
> around bases should receive reward, because now base fills faster.

Then the numbers, each asked and chosen: a thing built 0.5, half a
kill; all four pillbox measures; a base's useful rate as its guns
ready; "Start anew" for the old reads; a repair 0.25; a load of wood
0.2 within 16 tiles; guns ready +3 a team; a base's reach 16 tiles of a
depot.

## II. Decision log

| Decision | Cause |
| --- | --- |
| A thing built 0.5 | Paul's choice; at 0.1 the search sold building for points |
| The pillbox terms are the team's, each seat's as castles are | they are what a side holds, not what a seat did; the castle term set the form |
| A turret kill is a `PillboxFired` on a tank destroyed that tick by the pillbox's owner | the sim credits a pillbox's kill to its owner (`by`), so the shot is the only mark of the pillbox |
| The owner's seat keeps the kill among its own | its kills feed the kd guard and every report; the turret kill is the castle's credit beside it |
| The rate over at least ten pillbox-minutes | one pillbox that stood a minute and killed once is not ten times a castle that stood all match |
| A load of wood counts only when wood was gained | felling with a full squad fills nothing |
| Repairs and wood are the seat's | the seat's squad did them |
| `CENSUS_REWARDS=0` keeps a thing built at 0.1 | it is the score of LR 8 to 83, and stays that |
| `SCORE_VERSION`, read by orrery beside `GAME_VERSION` | Paul: "Start anew"; a read of the old score is no evidence about the new |
| The base's line in the census begins `base of team` | orrery's readers take lines from their start (`team N`, `rewards of team N`) |

## III. Method

`ledger.rs`: seven counts on `Outcome` (serde defaults, so old ledger
lines read), seven weights on `ScoreWeights`, the tally's turret kills,
pillbox seconds a team and guns ready a second; the team's numbers
written to each of its seats at `records`. `fleet.rs`: the base line.
A test of the new counts (a repair each, a load near and one far and one
empty, a turret kill, a minute of two guns one dry). 163 brain tests.

The rule, before the run: under the new score, w8466 against the
defaults reads under its old-score read by more than two errors of the
paired difference on the same seeds. Two censuses on this Mac, every
map (the BSD paste shim), 48 seeds, 900 s, respawns 3, `JOBS=8`, w8466's
own trial file; the old score by setting aside `ledger.rs` and
`fleet.rs` for one build; paired seed by seed.

## IV. Results

| w8466, a seat against the defaults | 48 seeds, every map |
| --- | --- |
| old score | +1.95 ± 1.20 |
| new score | +1.54 ± 1.20 |
| new less old, paired | **−0.42 ± 0.15** |

The new terms over the 432 teams of the new census, a team a match:
built 21.5, repairs 4.7, wood near a base 1.45, pillbox life 0.13 and
the longest 0.26 of the match, turret kills 0.05, guns ready 0.16.

## V. Discussion

The score now sees what the trades gave away: w8466 loses 0.42 a seat
by it, 2.8 errors. It does not see enough to make w8466 a loser, and it
should not have to: a score is what is worth winning, and the guard is
what a person will not trade. The floors stay; the fleet reads anew.

## VI. What is not built

The new counts as a scenario's reward terms; a change to the guard's
floors; any search over these weights, which are Paul's.

## VII. Paul's decisions

Whether to commit and push this (CLAUDE.md); after the fleet's first
reads under it, whether the guard's floors should move.
