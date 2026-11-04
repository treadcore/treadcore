# The Duelist's Gate a Dial, a Clone Judged Beside the Rules, and crucible's Partners in the Game's Styles

**Check-in report CI-089.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 4 November 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 140; CHANGELOG; ROADMAP
backlog 82; journal 0116; crucible `cab1adf`; orrery `7b3bf3c`.

---

## Abstract

The Gladiator read 2.5 a seat under the game's defaults, and the clone
stage's first round could not tell a clone from the Scholar as a whole
team of nets. The Duelist's gate is now a dial the census can read
(`duel_gate`); the clone is judged on one seat beside the rules
(`SCHOLAR=file`, `bot_mix = one`); and crucible's world islands spar
against the game's styles. The gate swept at 0, 0.3 and 0.6 leaves the
default at 0: 0.6 leans better by +1.61 ± 1.50, inside two errors.

## I. Intent

orrery asked what should close the Gladiator's gap and how a clone
should be judged. Paul chose, word for word: "Islands meet the game's
styles", "The gate as a dial", and "One seat beside the rules".

## II. Decision log

| Decision | Cause |
| --- | --- |
| `duel_gate`, default 0, range −1 to 1 | CLAUDE.md, "either/or is a scalar"; 0 is the gate as trained; the confidence is a tanh |
| The gate read from the rules' settings | every dial of a seat lives there; the Duelist wraps the rules |
| The gate for both styles 7 and 17 | one bridge, one rule |
| `SCHOLAR=file` for slot 6 | a census of one seat beside the rules needs the style, not `CENSUS_NET`'s whole team |
| The gate's default not moved | the rule before the run: no gate past two errors |
| crucible's styles only under WORLD, style 0 the script | the yardstick and the hybrid's answer must stay what they were measured on |

## III. Method

weights.rs `DuelGate`; RANGES; styles.rs `Duelist::gate`,
`scholar_file`, a test that a confidence of 0.54 passes gates 0 and 0.5
and not 0.6. 164 brain tests. Three censuses on this Mac, every map
(the BSD paste shim), 48 seeds, 900 s, respawns 3, `JOBS=8`, the trial
`bot_style = 17` with `duel_gate` 0, 0.3, 0.6; paired seed by seed.

## IV. Results

| Gate | Read, a seat against the defaults | Less gate 0, paired |
| --- | --- | --- |
| 0 | −2.58 ± 1.27 | |
| 0.3 | −2.50 ± 1.16 | +0.08 ± 1.37 |
| 0.6 | −0.97 ± 1.52 | +1.61 ± 1.50 |

## V. Discussion

A champion kept to the fights it is surest of costs its team less: at
0.6 the Gladiator reads inside the noise of the rules. One error is not
two, and 48 seeds are a tenth of a held-out read; the fleet's 400 will
say whether 0.6 is the line.

## VI. What is not built

The gate's read on the fleet; the clone's first one-seat judgement
(it waits for this commit); champions trained against the styles,
which the bridge reads as they come.

## VII. Paul's decisions

Whether to commit and push this; whether the fleet reads the gate at
0.6 on 400 seeds.
