# The Methods So Far, and the Census as a Learner

**Check-in report CI-052.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 87; roadmap backlog
69.

---

## Abstract

A review, no run. It does three things. It lists the methods the
census gained in LR 78 to 86: the sequential census, families as a
guard, parts taken apart, the squad and spending meters, `rungs.py`
and `traits.py`, rewards in the score, and worktrees on the machine.
It sets the gains beside how they were found. Continuous tuning by
SPSA read flat (LR 74: -0.00 +/- 0.14). Turtle, a hand-set style, read
+1.40. The cowboy, new behaviour, read +11.62. And it plans the census
as a learning loop: the paired score as the objective, fresh seeds as
batches, 400 held-out seeds as the test set, families as constraints,
the NOTES rule as preregistration. The steps, in order: make dead
weights act and screen every weight's slope; evaluate against a league
of fixed opponents; run MAP-Elites over the personalities with
`traits.py`'s signature as the descriptor; batch search on the 24
threads; weights by map features.

**Index terms:** methods, brain census, machine learning, SPSA,
quality-diversity, MAP-Elites, league training.

---

## I. Introduction

### A. Intent

To commit LR 85 and 86 on main, and write the report Paul asked for.

### B. The prompts

> looking good. the census just finished. can we get a visual playtest. I want to see these battles in a 16 bot map. show 2 of the screen.

> merge. only on main. i think we are ready. lab report about these new methods and looking forward into running census and using this like machine learning.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "merge. only on main" | LR 85 and 86 committed and pushed on main (`bd08f40`); no branch | A branch and a merge: the project works on main |
| D2 | "a visual playtest ... 16 bot ... 2 of the screen" | `CLIENTS=2 PLAYERS=16 TEAMS=2 scripts/quad.sh` on the machine's screen, from the worktree with the new brain | The machine's main tree: it was behind the cowboy then |
| D3 | The report looks forward | LR 87 a review and a plan, no run | A first learning run in the same step: one step at a time |
| D4 | LR 74 flat, LR 86 large | The plan puts new behaviour and dead weights before tuning | Start with a bigger optimiser: it would polish a ridge |
| D5 | Shelling pillboxes mostly paid by its reward | The old score reported beside the rewarded one, as a guard on every learner | Drop the rewards: Paul asked for them, and they are visible now |
| D6 | The defaults move | A league of fixed opponents | Climb against today's defaults alone: a cycle would go unseen |

---

## III. Methods

The review read LR 24 to 86, the ledger (`ledger.rs`), `scripts/
spsa.sh` (LR 40) and the census timings on the machine (a 110-seed
census about 7 to 9 minutes, 400 seeds 20 to 30, every map, 24 jobs).

---

## IV. Results

In LR 87: the methods table; the gains table; the mapping of learning
to the census; the plan and its guards.

---

## V. Discussion

The census is already most of a learning system: it has an objective,
fresh data each step, a held-out test, constraints and
preregistration. What it lacks is a search that proposes behaviour
rather than numbers, and opponents that stay put. The personalities
Paul asked for are the bridge. A quality-diversity search keeps one of
each, tuned, and plain to see by construction.
