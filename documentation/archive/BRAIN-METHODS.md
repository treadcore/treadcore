# The brain's methods: archived, current and on trial

Step 12 rebuilt the bot AI (`treadcore-brain`) from the brain of 17
October 2026 into the improved prototype of `../LAB-REPORT-BRAIN.md`
section 20. This page is the register of what that changed, method by
method, so the old ways stay part of the development record and can be
found, read and played again.

Three states:

- **Archived.** Replaced as the default. Most old methods are still in
  the code, one switch away, so a census can play them against what
  replaced them; a few were plain inefficiencies, removed outright.
- **Current, challenger off.** The old method is still the default. An
  improved one is built, proven by its test and measured, but stays off
  until a census promotes it (ROADMAP goal 4's rule). Turn it on with
  `caps = baseline, NAME` in a weights file, on the coordination
  screen's weights page (`U`), or with `CENSUS_CAPS=NAME`.
- **Shown impossible.** Measured, and the physics rules it out.

Section numbers are the lab report's (LR); dates are 2026.

## Archived

| Old method | Replaced by | When | The old one now | Evidence |
| --- | --- | --- | --- | --- |
| Constants written into `Tactics` and through the brain | `weights::Settings`: a flat `Weights` array of named weights (76 now) and `Caps` bits (17), defaults equal to the old constants | 18 Oct, LR 11 | the defaults themselves | `default_weights_are_todays_constants` |
| The castle of steps 11b and 11h: three towers and two bastions, filled with every pillbox carried in | the capped castle: two pillboxes inside, one on its road, the rest to posts on the roads between bases, kept off junctions (`castle_cap`, default on) | 18 Oct, LR 12 | `caps = baseline, -castle_cap`; proofs `a_commander_fortifies_its_depot`, `a_commander_climbs_the_ladder` run it | fair census: old castle 8.36 a seat, capped 9.08 (LR 14) |
| The census's one match a seed, the trial team turned round | the fair census: every team in the trial seat on every seed (`CENSUS_FAIR`, default on) | 18 Oct, LR 14 | `CENSUS_FAIR=0` | a null run read 12.66 against 6.44 before, 13.93 against 13.93 after |
| The census's baseline teams always on the defaults | `CENSUS_BASE_CAPS`: the baseline teams' switches, for a part that needs another on | 18 Oct, LR 18 | leave it unset | used for `bridge_jobs` against `board` |
| `auto()` copying the whole fort plan and running `add_turrets` on the copy every tick (L3) | `turret_preview`, a `OnceLock` read | 18 Oct, LR 11 | removed | the meter: 2 to 4 plan copies a match |
| `Tactician` building a list of reasons for every candidate (L3) | reasons for the winner only | 18 Oct, LR 11 | removed | `value()`, `rate()` |
| A failed bridge search never cached (L4) | `MapFacts`' crossing cache, failures kept for 60 s | 18 Oct, LR 11 | removed | the meter: 438 crossings searched, 2,539 answered from the cache in six matches |
| `Commander::set_settings` rebuilding the tactician, and able only to turn four switches off | the tactician updated in place; a switch off and on again gives back the role's own setting (`CapHeld`) | 18 Oct, LR 17 | removed | `a_switch_comes_back_and_a_slider_keeps_the_target` |
| Logic and the census reading the status sentences back ("on guard", "called", "ducking" by substring) | codes beside the words (`status.rs`: `Act`, `HopAct`, `CycleAct`, `KiteStep`, `Why`, the tactician's `Tags`); the words kept for people | 18 Oct, LR 22 | removed | a census match identical before and after; `every_code_is_its_own` |
| Floats compared with `partial_cmp().unwrap()`, a panic on any NaN (74 in the brain, 19 in the window and the sim) | `total_cmp`, which orders every float and cannot fail | 18 Oct, LR 23 | removed | `a_nan_does_not_panic_the_brain` panics on the old code and passes on the new; a census match identical before and after |
| The window's pilot playing on no settings at all | the pilot on the weights page's settings, which start from the team file or `--weights` | 18 Oct, LR 17 | removed | `client::tune` |

## Current, challenger off

| Current method (the default) | Challenger | Switch | Measured | LR |
| --- | --- | --- | --- | --- |
| Given-up targets remembered for the match (L6) | memories that run out after `forget_for` | `forget` | not yet alone | 11 |
| Each decision taken at once, every think (L7) | desire vectors, a turret held until a challenger wins three picks, the Auto rung confirmed over two thinks, the stance from smoothed odds | `smooth` | 7.55 a seat against 9.47, worse | 11, 15 |
| Each tank on its own picture of the map (L5) | the team board: influence, front, defence points, jobs kept, posture | `board` | not yet alone | 11 |
| Each tank balancing only itself | a hurt tank at the hottest point swapped for a fresh one | `team_rotate` | not yet alone | 11 |
| Every gap of 20 water tiles or fewer bridged | boat or bridge by price: long bridges and scarce trees cost more | `priced_crossing` | not yet alone | 11 |
| The wedge kept on a bridge | the fleet in file on a bridge | `column` | not yet alone | 11 |
| Trees felled only when a build is short (L8) | after filling up: an item and an improvement for the base, then a road out | `cycle` | not yet alone | 11 |
| Any island a base | a small island with few trees a poor site | `island_site` | not yet alone | 11 |
| A leader fetching a far pillbox alone | the fleet gathered first, escorting it home along the road | `escort` | 7.70 against 7.70: never reached in 900 s | 16 |
| A bridge given up left half laid; a leader asking one wingman for its next tile | part-laid bridges on the team board, taken by the next teammate with material | `bridge_jobs` (needs `board`) | 7.31 against 8.01, inside the noise | 18 |
| Direct fire taken where it falls (only kiting, against the one enemy a tank fights, keeps out of reach) | the duck: out of a hostile gun's range or line, seen or remembered, when it is ready before one's own | `duck` | 7.01 against 7.52, inside the noise | 21 |

## Shown impossible

| Idea | Why not | Evidence | LR |
| --- | --- | --- | --- |
| Dodging a direct shot once it is fired | the shell flies five ticks at most; a tank at full speed turning its hardest moves about 0.4 tiles off the gunner's lead in that time, and the hit radius is 0.8 | `a_led_direct_shot_cannot_be_dodged`: hit from 4, 7 and 9.5 tiles whatever the target did | 19 |

## Still as they were

Not every finding of LR 2 has an answer yet: the observation is still
a full copy of the world per bot per tick (L2); the `doing` words are
still written every think, beside their codes, measured as costing
nothing (L3, LR 22); and the Auto ladder and the
wingman's priorities are still first-match orders (L1).
