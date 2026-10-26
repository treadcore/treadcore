# The Cowboy: the Defaults Spend

**Check-in report CI-051.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 25 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 86; CI-050 and
section 85; roadmap backlog 67; `weights.rs`, `tactics.rs`, `lib.rs`,
`meter.rs`, `profiles/steady.weights`.

---

## Abstract

The brain had no weight for spending: shells went at its target tank,
longshots every 30 s at structures only, one strike drone at a time.
Four weights let it spend, and a cowboy profile at loud values was
plain to see. It also won by a margin no change had come near: +11.54
a seat on its census, +11.62 +/- 0.45 read again over 400 seeds, every
map above. Taken apart, the longshots carry the fighting and shelling
pillboxes is mostly LR 84's reward for it. `retreat_shells` does
nothing at the defaults, because the retreat weights act through
`w_smooth` 0.05. By the rule the spending became the defaults. `make
families` then caught the dodge families at 91-96.5%: a pillbox shot
turned the hull into a shell's splash. A pillbox shot now waits under
an incoming shell, and the defaults as changed read +11.64 +/- 0.43
against the old.

**Index terms:** personalities, brain census, ablation, regression,
defaults.

---

## I. Introduction

### A. Intent

Personalities with code, read by the census: the cowboy first, as Paul
named it.

### B. The prompts

> more census. with the code changes. continue. resume

> nice!. looks like its ending. pick up the results.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "with the code changes" | The cowboy first: four spending weights, at the old behaviour by default | Timid and aggressive first: they needed a cause found (later: the retreat blend) |
| D2 | The machine plays committed trees; nothing committed | A worktree on the machine at 3e55c83 with the patch applied (`treadcore-lr86`, `-lr86b`) | Commit to run: Paul has not asked |
| D3 | A weight added must not change play at its default | The same atlantic census to the digit | — |
| D4 | F: +11.54, a style past two errors above | Read again on 400 fresh seeds, and each part alone, rules first | Adopt on 110 seeds |
| D5 | F: re-read +11.62, every map | The spending the defaults; `steady.weights` the old | — |
| D6 | F: retreat_shells exactly 0 | Left at 4; the blend through `w_smooth` found and reported | Keep it in the change: it changes nothing |
| D7 | `make families`: four dodge families under 99% | `shell_boxes` alone the cause; a pillbox shot waits under an incoming hostile shell | Lower the floors; drop shell_boxes: the fix kept its gain |
| D8 | The match test lost its pings on one seed | The test's teams on steady's spending, as LR 77 pinned mechanism tests | Change the seed: hides the cause |
| D9 | The fix changed the code after the census | The defaults as changed read against steady on 400 fresh seeds, rule first | Adopt on the earlier reading |

---

## III. Methods

The weights in `Tactics` (`from_weights`); `RuleBot`'s fight: the
pillbox shot after the gun and strikes, the longshot's target, the
strike cap. `meter::spend` on each think's commands. Censuses on the
machine, every map, 900 s, 24 jobs: the cowboy (110 from 120000), the
re-read (400 from 130000), the parts (110 from 140000), the pair (400
from 150000, `BASE=steady.weights`). `scripts/traits.py`; the old score
as score less rewards. On the Mac: `cargo test -p treadcore-brain
--release`, `make families` with `SET` for each part.

---

## IV. Results

In LR 86: the cowboy +11.54 +/- 0.89; re-read +11.62 +/- 0.45; the
parts (shell_boxes +7.19, longshots +5.59, strikes +1.46,
retreat_shells 0); the families; the pair +11.64 +/- 0.43, +5.52
without the rewards.

---

## V. Discussion

The brain carried drones and shells it did not use. Spending them is
worth more than any weighting of the ladder so far. Part of the
measured gain is the reward for pillbox damage that LR 84 added an
hour before, but even on the old score the change is four times
turtle's. The families earned their place again: a census cannot see
a tank shooting itself into a splash on 4% of seeds. The retreat
weights acting a twentieth of the way is a finding for every profile
that set them.
