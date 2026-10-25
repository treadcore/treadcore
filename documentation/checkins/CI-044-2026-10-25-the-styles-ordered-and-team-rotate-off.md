# The Styles Ordered: Turtle the Defaults, team_rotate Off

**Check-in report CI-044.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` sections 76 (team_rotate read
again) and 77; roadmap backlog 60, 61 and 62; `weights.rs`.

---

## Abstract

The night's two queued runs on the machine ended. The play styles'
round robin (20 censuses, 110 seeds each, every map) found no rock
paper scissors: the styles are ordered, turtle first, past two errors
over every other style and the defaults (+1.34 +/- 0.39 a seat), with
no cycle over every map or on any one. Backlog 61, rotation between
styles, is shelved. Turtle was read again on 400 fresh seeds, as its
rule asked: +1.40 +/- 0.18, every map above zero. With team_rotate off as well, the new defaults read +1.75 +/- 0.19 against the old, so both changes went in; the old defaults are kept as `profiles/classic.weights`. team_rotate off, read again on 400 fresh
seeds, reads +0.55 +/- 0.15, no map past two errors the other way, so
by its rule it is off by default. `make families` then showed one family
under its floor, a-raid-on-a-guarded-base, 70.2% against 75%. Bisected,
it fell at 5899e8e (the front read off) and has nothing to do with
team_rotate. It is backlog 62.

**Index terms:** brain census, payoff matrix, play styles, intransitive
games, defaults, regression bisection.

---

## I. Introduction

### A. Intent

To write up the round robin (LR 77) and team_rotate's re-read (LR 76),
and act on each by the rule written before its run.

### B. The prompt

> do it. start. yeah. start .

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | `build/rr/NOTES.txt`: a cycle on every map or on any one | Cycles of three to five sought among the relations past two errors, over every map and map by map: none | Read the matrix by eye: a cycle of four or five is easy to miss |
| D2 | The same notes: a style past two errors above the defaults, no map the other way, read again on 400 fresh seeds | Turtle (+1.34 +/- 0.39; naval -0.30 the lowest map) read again, seeds 80000 to 80399, the rule written to `build/turtle/NOTES.txt` first | Make it the default on 110 seeds: the rule says read again |
| D3 | The machine idle; its tree two commits behind (docs and scripts only) | Pulled before the turtle census | Run on the old tree: the census code was the same either way |
| D4 | team_rotate's rule (`build/rotate/NOTES.txt`) met: +0.55 +/- 0.15, no map past two errors the other way | `TEAM_ROTATE` default false; the switch and its code kept | Remove the code: a switch off is a census away from on again |
| D5 | Turtle's census reads the brain with team_rotate on | The team_rotate change made after the turtle census began, on the Mac only | Change both at once: two changes in one reading |
| D6 | F: a family under its floor after the change | The same family at HEAD without the change: the same 281 of 400 | Blame the change |
| D7 | 30 s a family run, the Mac idle during the turtle census | Bisected in a worktree: 81.8% at fa8ba05, 3f19029 and 0828d4a; 70.2% at 5899e8e | Leave the cause for later: it was cheap now |
| D8 | One step at a time | The family's fix is backlog 62, not this step | Fix it here: a second change in one check-in |
| D9 | F: no cycle; turtle first | Backlog 61 shelved | Build rotation anyway: a mix of ordered styles is no better than its best |
| D10 | F: turtle +1.40 +/- 0.18, its rule met; it was read with team_rotate on | One census more, the pair against the old defaults, its rule first: +1.75 +/- 0.19, both in | Adopt both on their separate readings: the pair was not read |
| D11 | Five tests broke on the new values | `Tactics::default` follows the weights; three tests of a mechanism pin the old values they were written for | Change their expected outcomes to the new values: they test the mechanism, not the tuning |
| D12 | Turtle's profile is now the defaults; Paul chose "old defaults as a profile" | `profiles/classic.weights`, turtle.weights removed | Keep turtle with the test relaxed; three styles only |

---

## III. Methods

**The round robin.** `make roundrobin` (`scripts/roundrobin.sh`): the
fair census for each ordered pair of {defaults, expander, navy,
raider, turtle}, one team of the row's style against two of the
column's, seeds 60000 to 60109, `MAP=all` (10 seeds a map), 900 s, 3
respawns, 24 jobs, a frozen binary each. `matrix.txt` from the 20
`pooled.txt`. The cycle search: for each map and over every map, the
directed relations "row past two errors above zero against column",
then every ordered three, four and five of the styles checked for a
closed ring.

**team_rotate.** `TRIAL=norotate.weights` (`caps = baseline,
-team_rotate`) against the defaults, seeds 70000 to 70399, `MAP=all`,
24 jobs.

**Turtle.** `TRIAL=profiles/turtle.weights` against the defaults,
seeds 80000 to 80399, `MAP=all`, 24 jobs, at 8062fb7.

**The pair.** `TRIAL=build/newdefaults/new.weights` (`caps = baseline,
-team_rotate` and turtle's ten weights) against the old defaults,
seeds 90000 to 90399, `MAP=all`, 24 jobs, at 8062fb7.

**The change and its checks.** `weights.rs` `TEAM_ROTATE` default
false; `cargo test -p treadcore-brain --release`; `make families`.
The bisection: `git worktree` at each commit, `make families` with its
own target directory.

---

## IV. Results

In LR 76 and 77: the matrix; no cycle on any map; team_rotate off
+0.55 +/- 0.15 over 400 seeds; turtle +1.40 +/- 0.18 over 400 seeds; the two changes together +1.75 +/- 0.19; the brain's tests, 87
passed; the families, all but a-raid-on-a-guarded-base (70.2%, floor
75%), whose drop is at 5899e8e.

---

## V. Discussion

Paul's intuition was that no single style works. On these four styles
and eleven maps one does: holding what the base has beats going out
to fight, whichever style it meets. That is a finding about these
profiles, not about all styles. A style built to beat turtle (one that
starves a castle, or out-expands it) could close a ring, and backlog
61 can open again then. The family regression is the cost of a
default changed without every family read on the commit: the census
and the brain's tests passed on 5899e8e, and `make families` was not
run. It now runs at each default change.
