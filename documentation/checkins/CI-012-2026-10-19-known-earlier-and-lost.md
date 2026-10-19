# Known Earlier, and Lost

**Check-in report CI-012.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0060; `LAB-REPORT-BRAIN.md` sections 32 and
35; CI-011.

---

## Abstract

CI-011 proposed pricing the landfall by the team board's hostile
influence. Reading the code first showed two things. Sight is shared
by the whole team, so CI-009's boat-or-teammate split was vacuous. And
the board clears itself every second, so it cannot know a gun out of
sight. Backlog 35 closes without a part. Measured instead: in 76 of
164 landfall deaths (46%), the team had seen the killer 10 to 60 s
earlier, near where the boat died, and lost it. Backlog 36 remembers
hostile tanks for the landfall, by a weight whose default Paul sets.

**Index terms:** memory, partial observability, team knowledge,
measurement.

---

## I. Introduction

### A. Intent

Backlog 35's read, before its part.

### B. The prompt

> commit. push. continue backlog

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | CI-011 D4: a read before the part | Read `board.rs` and `World::visible_to` before measuring | Measuring the board's threat at killers' positions: the code answers it |
| D2 | F (code): shared sight | CI-009's split marked vacuous in LR 32 | |
| D3 | F (code): the board clears each second | Backlog 35 closed | A board with memory: a larger change than the question needs |
| D4 | The question left: known earlier? | Log team sightings by position at 1 Hz; class each landfall killer | |
| D5 | F: 46% seen near there 10 to 60 s before | Backlog 36, `landfall_memory`, default Paul's | Reusing `duck_memory`: a 5 s memory for a different decision |
| D6 | The compiler warned of an unused total | A shadowed name fixed; the printed numbers were unaffected | |

---

## III. Methods

In `brain_census` (fleet.rs), at each 1 Hz sample, every hostile tank
in a living trial seat's observation is appended to `sightings[owner]`
with the tick and its position, once a tick. At a landfall death with
killer `k`, the sightings of `k` whose age is 10 to 60 s are taken. If
any lies within `DIRECT_RANGE + 4` of the boat's position at death,
the death is "seen then, within reach"; else if any exists, "seen
then, elsewhere"; else "not seen". Null run, 16 seeds in ranges of 6,
5 and 5; naval.map, 900 s, respawns 3, the fair census.

---

## IV. Results

| The killer | Deaths | Share |
| --- | --- | --- |
| seen 10–60 s before, within reach of the death | 76 | 46.3% |
| seen 10–60 s before, elsewhere | 26 | 15.9% |
| not seen 10–60 s before | 62 | 37.8% |

Null 0.00 on every seed. Tests: brain 72 passed and 8 ignored; client
31; net 14; proto 3; sim 50; no warnings.

---

## V. Discussion

The bots forget a hostile tank the second it leaves sight. The duck
remembers for 5 s, the board for 1 s. Nearly half the boats that die
on landfall die to a gun the team had seen there within the minute.
Memory is the cheapest knowledge beyond sight the brain can have, and
the first part a death count supports that the earlier landfall parts
(`landfall_shy`, `landfall_wait`) can read at once.

**Next:** Paul's default for `landfall_memory`; then backlog 36.
