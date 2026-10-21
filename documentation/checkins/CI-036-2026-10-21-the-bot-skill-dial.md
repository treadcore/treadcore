# The Bot Skill Dial

**Check-in report CI-036.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0093; `LAB-REPORT-BRAIN.md` section 67;
roadmap backlog 56; PLAYABILITY item 9. (CI-034 and CI-035 are on the
branch `proposal/upgrades`, not on main.)

---

## Abstract

Paul found the game hard. The only dial, `--rookies N`, was a yes or
no a bot, and it reached no bot in a teamed match, which `make run`
is; the player guide's advice did nothing. One weight, `skill` (1 in
full, 0 a rookie), now blends each rookie trait: aim scatter and lead,
drone delay, think rate, and under 0.5 no dodge, kite or storm. A
Commander spoils its own orders at one place after its think.
`--skill` and `SKILL=` deal it to the bots against the person; allies
play in full. In the duel family the win rate falls steadily from 63%
at 1 to 8% at 0.5, and every family's count is unchanged at 1. A match
at 0.6 was left open on Paul's screen.

**Index terms:** difficulty, skill, rookies, playability.

---

## I. Introduction

### A. Intent

Paul's choice: "bot skill dial". A dial a person can set that makes
the bots against them weaker by degrees.

### B. The prompt

> bot skill dial. continue.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | F: `--rookies` is read only for plain RuleBots; `make run` deals team Commanders | The dial reaches every bot, Commanders too; the guide corrected | — |
| D2 | Either/or is a scalar | `skill` in Settings; `RuleBot::rookie` a `skill` of 0 | A second flag |
| D3 | A Commander fires from many paths | Spoil its orders at one place after the think (`blunt`) | Editing every fire path |
| D4 | F: the doctrine's scatter and `blunt`'s doubled | The doctrine inside a Commander aims in full | — |
| D5 | F: a quarter or a tenth of the scatter for a direct shot read the same | A tenth | — |
| D6 | The person's allies | Full skill; only the bots against the person dialled | All bots: weak allies make it harder, not easier |
| D7 | At 1 the brain must be today's | No random draw, no blend at 1; families checked to the seed | — |
| D8 | F: under 0.5 the duel is 0% | The guide names 0.5 to 1 the useful range | Reshaping the thresholds: a playtest should say first |

---

## III. Methods

Code: `weights.rs` `Skill`; `lib.rs` `RuleBot::skill`, `is_rookie`,
`aim_skill`, the blends; `commander.rs` `set_settings`, the dodge gate,
`blunt`; client `--skill` (`main.rs`, `session.rs`); `scripts/run.sh`
`SKILL`; Makefile help and export. Proof: `make scenario
S=scenarios/families/a-duel-in-the-open.toml RUNS=400 SET="skill=v"`
for v from 1 to 0; `make families`; `make test`. The window launched
with `--skill 0.6`: no errors.

---

## IV. Results

As section 67: duels outscored 63.0% at 1, 52.0% at 0.75, 24.5% at
0.6, 7.8% at 0.5, 0 under 0.5. Families unchanged at 1. Tests: brain
85 passed and 12 ignored; client 32; net 14; proto 3; sim 50.

---

## V. Discussion

1. **A dial that did nothing.** The rookie flag was built for plain
   RuleBots before teams were the default; nobody had checked it
   since. The guide sent a newcomer to it.
2. **A useful half.** The dial is smooth from 1 to 0.5 and a wall
   below; a person's playtest decides whether 0.5 is easy enough.

**For Paul:** a match at skill 0.6 is open on your screen. Try it,
and say whether 0.6 feels right; `SKILL=0.5 make run` is the easiest
that still dodges. Commits are local until you say push.
