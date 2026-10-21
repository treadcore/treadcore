# Supply Traced: the Upgrades Proposal's Trace 1

**Check-in report CI-033.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0090; `PROPOSAL-UPGRADES.md` section 7;
CI-032; roadmap goal 5, 11g.

---

## Abstract

Goal 5, 11g. The upgrades proposal (18 October) needs two traces
before acceptance and seven days of public discussion. Trace 1, supply
counted with nothing bought, needs no rule change: a new ignored test
pays each team the proposal's rates from outside six bot-only matches
on naval.map. Teams earn 48.8 supply a minute, at the top of the
proposal's guess; every team has the first price of 60 by a median of
97 s; depots give 60% of it and kills 81% of the first two minutes; the
leading side's supply runs away. The proposal gains a section 7 with
the measurement and suggested changes for its author. The post for the
discussion is drafted for Paul to post; this machine has no GitHub
credentials beyond git over SSH, and posting is his. Trace 2 needs a
prototype.

**Index terms:** upgrades, supply, proposal, trace, governance.

---

## I. Introduction

### A. Intent

Paul chose the next step: "Goal 5: upgrades proposal" (post the
proposal for its seven days and write the traces of its section 6).

### B. The prompt

> ok. nice. the game is hard. continue.

Then, asked for the next step among a bot-skill scalar, the territory
read and the upgrades proposal: **Goal 5: upgrades proposal**.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "the game is hard"; no queued step | Asked Paul; offered a skill scalar, noting `ROOKIES=n` | Building a skill scalar unasked |
| D2 | Paul's choice | Goal 5, 11g | — |
| D3 | Trace 1 changes no rule | Supply paid from outside the world in an ignored test | A sim prototype first: trace 1 does not need one |
| D4 | The census's settings (LR 54 on) | The default brain in all sixteen seats, naval.map, 3 respawns, 30 min at most, six seeds, a thread each | No respawns: matches of 8 to 16 min would not show the late game |
| D5 | GOVERNANCE.md section 5: the author revises during discussion | Findings and *suggested* numbers in a new section 7; section 1.1's numbers left as Paul wrote them | Rewriting 1.1: the author's call |
| D6 | No `gh`, no token; posting is outward | The post drafted in `build/supply/discussion-post.md` for Paul | Posting by other means |
| D7 | Trace 2 needs buying | Named as needing a prototype behind `Scenario::upgrades` | Building it now: a rules change before the discussion |
| D8 | Three of my sentences overstated the data | Corrected before commit (15-minute total, "most matches", the halving's effect) | — |

---

## III. Methods

`fleet::tests::supply_trace` (ignored): for each of `SUPPLY_SEEDS` (6)
seeds a thread with a 256 MB stack; the census's world (`Scenario`
with the fleet's teams, naval.map, `SUPPLY_RESPAWNS` 3, no time limit)
run to `SUPPLY_SECS` (1,800) or the end; every seat `fleet::brain`.
Every 10 s each side earns 1 a live pillbox and 2 a depot; every 20 s 1
a powered structure; 10 for a `TankDestroyed` of another team's tank
by its own seat, 5 for a `PillboxCaptured` from another team. Printed:
by side, supply by source, the seconds at 60 and 120, every 5 minutes;
in all, supply a team-minute, the shares, the median second at 60, the
match length. Log: `build/supply/trace1.log`.

---

## IV. Results

As PROPOSAL-UPGRADES.md section 7: 48.8 supply a team-minute (7.3 to
92.5 by team); 60 by 18 of 18 teams, median 97 s; depots 60%,
pillboxes 21%, kills 19%, captures under 1%, structures 0%; matches
18.0 min on average (16 to 23). Tests: brain 84 passed and 12 ignored;
client 32; net 14; proto 3; sim 50.

---

## V. Discussion

1. **The economy is richer than guessed.** The first upgrade is in
   reach in a minute and a half, and a winning side could own the
   whole base list by the 20th minute.
2. **Supply follows the winner.** The open question of a catch-up
   price now has evidence.

**For Paul:** two things are yours. (1) Post `build/supply/discussion-
post.md` in GitHub Discussions under *Proposals*; the seven days start
then. (2) Section 7's suggested numbers (halved rates, a bounty of 5)
are suggestions; section 1.1 is unchanged until you say. Commits are
local; nothing was pushed.
