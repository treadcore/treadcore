# A Test's Text Reads Over the Pinned Brain: The Night's Best Candidate Held Back by a Parse

**Check-in report CI-097.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 5 November 2026, morning.
Companion records: `LAB-REPORT-BRAIN.md` section 148; journal 0124.

---

## Abstract

The night's best candidate, grove/w10040, moved `ping_style` from 0 to
2 and failed one brain test. That test's helper read its weights text
over the defaults. LR 147 had pinned the brain the unit tests play, but
its lint let a parse over the defaults through. Its stress moved the
numbers and never a choice or a switch. A weights text now reads over
the pinned brain, the fleet's role tests ask under it, and a scenario's
switches read over its own brain. `set-defaults.py` now writes the
tactics' true/false fields and what a switch makes of them. A promotion
of any of five weights or switches would have failed on those fields.
Stressed over every promotable choice, every switch and every
true/false weight, the line failed six tests before. After the change,
everything passed except two assertions that the kite and castle
switches are on by default. Paul had them dropped.

## I. Intent

Paul: "fix the test." The test was `commander::tests::with_pings_on_as_built_a_bot_comes_to_any_ping`,
on which r0185, the promotion of w10040, failed at 05:44 on bosporus.

## II. Decision log

| Decision | Cause |
| --- | --- |
| `Settings::parse_over(brain, text)`, with `parse` as `parse_over` the defaults | `parse` reads as it always did everywhere outside the tests |
| `Settings::pinned_with(text)` for a test's text | `hears` read "caps = baseline, pings" over the defaults, and played warn once the default moved |
| `baseline` in a caps line is the brain's own capabilities | `Caps::BASELINE` is what a promotion of a switch moves |
| The fleet's role tests under `with_settings(Settings::pinned())` | `role` and `role_by_rank` read `fleet::settings()`, the defaults when none is set |
| A scenario's caps read over the scenario's base | under `with_base` a unit test's scenario read `baseline` as the code's; the families set no base and read as before |
| `set-defaults.py` writes true/false fields and switch-made values | `default_weights_are_todays_constants` failed when `kite`, `shell_boxes`, `longshot_any` or the kite or focus_blend switch moved |
| That test stays on the defaults | it is about the defaults themselves; the writer was wrong, not the test |
| The lint forbids `Settings::parse(` and follows `role`, `role_by_rank`, `team_role`, `equip` | the two ways the stress found that the lint could see |
| The stresses move choices, switches and true/false weights | `RANGES` holds none of the 15 choices or 18 switches, so LR 147's stress moved none |
| The assertions of kite on, castle on and smooth off dropped | Paul: "drop assertions."; no decree held them, and the fleet tries each the other way |
| The round-trip test picks its numbers off the defaults | its `w_smooth` 1 would have been the default after a promotion to 1 |
| A stress at every range's min and max | the search may carry a number to the end of its range |
| `Commander::pinned_wingman`, and the lint forbids `Commander::wingman` | `wingman` is made by `new`, on the defaults; at every weight's max a pillbox shot one |
| The slider test's step at a default of 0 is 0.1 | a default of 0 has no tenth; the step rule says 0.1 |
| `bot_style` and `patch` left out of the choice stress | the fleet does not promote them: they change the brain's whole form and wait for a person |

## III. Method

Two states of a worktree of the line the fleet runs, `f04f157` (main
`aebeb1d` with r0179's `bio` 0 to 3): as it is, and with the change.
Each move is written by that worktree's `scripts/set-defaults.py` as a
promotion writes it, then `cargo test -p treadcore-brain --release --lib`
runs. The moves:

- w10040's `ping_style = 2`;
- the 13 choices the fleet promotes, each moved one, two, three and four versions on from its default (modulo its count);
- every one of the 18 switches flipped;
- `kite`, `shell_boxes` and `longshot_any` set to 0;
- LR 147's moves: its stress and the ten distinct moves of the 13 failed promotions.

The scripts and files are in the session's scratchpad (`lr148/replay.sh`, `*.weights`).

## IV. Results

| The defaults moved to | the line, `f04f157` | the line with the change |
| --- | --- | --- |
| none | not run bare (r0185 ran its 166 there) | 167 pass |
| w10040: `ping_style` 0 to 2 | `with_pings_on_as_built_a_bot_comes_to_any_ping` fails | 167 pass |
| every promotable choice one version on | 3 fail: `at_style_0_with_pings_off_a_bot_answers_no_bot`, `a_bots_team_takes_its_roles_from_its_settings`, `three_teams_of_five_and_one_alone` | 167 pass |
| two versions on; three versions on | 2 fail each: the pings test as built, the roles test | 167 pass, both |
| four versions on | `at_style_0_with_pings_off_a_bot_answers_no_bot` fails | 167 pass |
| every true/false weight of the tactics flipped | `default_weights_are_todays_constants` fails | 167 pass |
| every switch flipped | 3 fail: `at_style_0_with_pings_off_a_bot_answers_no_bot`, `a_switch_is_set_on_every_commander`, `default_weights_are_todays_constants` | `default_weights_are_todays_constants` failed on its assertion that the kite switch is on; with the assertions dropped, 167 pass |
| every switch flipped but `kite` and `castle_cap` | not run | 167 pass |
| `w_smooth` 1 | not run | the round-trip test failed; mended, 167 pass |
| every searchable weight at its min | not run | the slider test failed; mended, 167 pass |
| every searchable weight at its max | not run | `a_wingman_takes_the_pillbox_by_its_leader` failed; wingmen pinned, 167 pass |
| LR 147's stress, and the ten distinct moves of its 13 failures | not run (LR 147: pass with the pinned brain) | 167 pass, all eleven |

The lint, before the tests moved, named the seven helpers and tests of
the first way, with `hears`'s five callers; before the fleet tests
moved, it named `three_teams_of_five_and_one_alone`.

The families, with the change on main (`every_family_holds_its_floor`): all nine hold, from a duel in the open at 87.2% ± 1.7% (floor 55%) to a tank under a ready gun at 82.1% ± 1.2% (floor 80%). They set no base, so the scenario reader's change reads their switches as before.

## V. Discussion

The pinned brain held for everything that built a brain by name and
let by everything that built one from a text or a thread's setting. A
promotion is the only thing that moves the defaults, so every way a
test reads them shows only when the fleet moves the right number. The
stresses are the check that does not wait for the fleet: they now move
every kind of thing a promotion moves. One failure was not a test
reading the defaults. It was a writer that did not write what it
promised, and its test was right to stop it.

## VI. What is not built

The lint reads a test's body whole. A test that sets `with_settings` for
one call and not another passes it; only the stress finds that.

## VII. Paul's decisions

The kite and castle switches: `default_weights_are_todays_constants`
asserted both on by default, with no decree in `RANGES` to hold them.
Paul: "drop assertions." Both went, and so did the assertion on the
same line that the bot does not smooth. The round-trip test that a
smooth promoted to 1 would have failed now picks values off the
defaults. Every searchable weight at its min and at its max then found the
slider test's zero step and five tests' wingmen on the defaults; both
are mended.

Whether to commit and push this. On main it lets w10040 be tried again,
since orrery retries a candidate whose tests failed once main moves.
