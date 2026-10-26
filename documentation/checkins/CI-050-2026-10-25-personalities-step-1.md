# Personalities, Step 1: Timid, Aggressive, Castle

**Check-in report CI-050.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 85; CI-049; roadmap
backlog 67; `profiles/`, `scripts/traits.py`.

---

## Abstract

Three personalities from existing weights, each tested against a rule
written before its census: it must be plain to see in its own
signature, past two errors against the default teams in the same
matches. Castle builds 4.01 more (6.7 errors) and passes. Aggressive
dies 1.15 more but kills no more, so it half passes. It is plain in
everything else (12.7 fewer things built, more neutral pillboxes
taken, more pillbox damage) and loses 2.10 a seat. Timid reads as the
defaults, which LR 77 already made turtle. Timid and aggressive need
behaviour, not louder values; that is the next step's code.

**Index terms:** personalities, profiles, brain census, behaviour
signatures.

---

## I. Introduction

### A. Intent

The personalities Paul asked for (CI-049's first prompt), begun with
those existing weights can carry, after the rewards.

### B. The prompt

> (to commit LR 84 and then start the personalities) Commit and push

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | The cowboy, the collector and the front line need code; timid, aggressive and castle have weights | Step 1 the three with weights | All at once: one step at a time |
| D2 | "very vissible" | The rule: a signature past two errors its own way (deaths and kills, built) | Judge by the score: a personality need not win |
| D3 | The defaults' rung seconds are not printed for the baseline teams | The signature from the team lines, paired; rung seconds reported | A defaults census for the rungs: unpaired, and not the question |
| D4 | The profiles are uncommitted | Copied to the machine's `build/personalities/` with `traits.py`; its tree at 3e55c83 | Commit first: Paul has not asked |
| D5 | F: timid and aggressive fail their signatures | Made stronger in a later step, with code | Keep them: the rule says not; push their values further: they are near their ends |

---

## III. Methods

For each of timid, aggressive and castle: `TRIAL=build/personalities/
P.weights SEEDS=110 SEED0=110000 MAP=all JOBS=24 make census` on the
machine, one after another (`build/personalities/run.sh`, 16:24 to
16:42). `scripts/traits.py DIR`: for each seed, the trial team's
counts less the mean of the other two teams', averaged over the seed's
three matches, then the mean and its standard error over 110 seeds.

---

## IV. Results

In LR 85: the table; castle passes, aggressive half, timid not.

---

## V. Discussion

A personality is a change from the defaults, so it can only be as
visible as the defaults leave room for. After LR 77 the defaults are a
turtle, and a timid profile has nowhere left to go on the same
weights. Aggressive shows that visible is not the same as good: it
dies more, takes more neutral pillboxes and damages more hostile ones,
but trades a tank for less than a kill and loses. Both want new
behaviour: timid a kiting ring farther out or a guard inside the
castle; aggressive a way to make its extra fights pay.
