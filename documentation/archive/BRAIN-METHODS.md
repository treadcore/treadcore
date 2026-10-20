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
| Constants written into `Tactics` and through the brain | `weights::Settings`: a flat `Weights` array of named weights (96 now) and `Caps` bits (17), defaults equal to the old constants | 18 Oct, LR 11 | the defaults themselves | `default_weights_are_todays_constants` |
| The castle of steps 11b and 11h: three towers and two bastions, filled with every pillbox carried in | the capped castle: two pillboxes inside, one on its road, the rest to posts on the roads between bases, kept off junctions (`castle_cap`, default on) | 18 Oct, LR 12 | `caps = baseline, -castle_cap`; proofs `a_commander_fortifies_its_depot`, `a_commander_climbs_the_ladder` run it | fair census: old castle 8.36 a seat, capped 9.08 (LR 14) |
| The census's one match a seed, the trial team turned round | the fair census: every team in the trial seat on every seed (`CENSUS_FAIR`, default on) | 18 Oct, LR 14 | `CENSUS_FAIR=0` | a null run read 12.66 against 6.44 before, 13.93 against 13.93 after |
| The census's baseline teams always on the defaults | `CENSUS_BASE_CAPS`: the baseline teams' switches, for a part that needs another on | 18 Oct, LR 18 | leave it unset | used for `bridge_jobs` against `board` |
| `auto()` copying the whole fort plan and running `add_turrets` on the copy every tick (L3) | `turret_preview`, a `OnceLock` read | 18 Oct, LR 11 | removed | the meter: 2 to 4 plan copies a match |
| `Tactician` building a list of reasons for every candidate (L3) | reasons for the winner only | 18 Oct, LR 11 | removed | `value()`, `rate()` |
| A failed bridge search never cached (L4) | `MapFacts`' crossing cache, failures kept for 60 s | 18 Oct, LR 11 | removed | the meter: 438 crossings searched, 2,539 answered from the cache in six matches |
| `Commander::set_settings` rebuilding the tactician, and able only to turn four switches off | the tactician updated in place; a switch off and on again gives back the role's own setting (`CapHeld`) | 18 Oct, LR 17 | removed | `a_switch_comes_back_and_a_slider_keeps_the_target` |
| Logic and the census reading the status sentences back ("on guard", "called", "ducking" by substring) | codes beside the words (`status.rs`: `Act`, `HopAct`, `CycleAct`, `KiteStep`, `Why`, the tactician's `Tags`); the words kept for people | 18 Oct, LR 22 | removed | a census match identical before and after; `every_code_is_its_own` |
| Floats compared with `partial_cmp().unwrap()`, a panic on any NaN (74 in the brain, 19 in the window and the sim) | `total_cmp`, which orders every float and cannot fail | 18 Oct, LR 23 | removed | `a_nan_does_not_panic_the_brain` panics on the old code and passes on the new; a census match identical before and after |
| The posture's Defend on the sector threat at the base over 0.5, or a readiness spread over 0.6: Defend in 94% of picks | hostile worth within the base's alarm (16 tiles) over 0.5; the spread left to `team_rotate` | 18 Oct, LR 24 | removed | `the_team_defends_a_threat_at_the_base_not_far_off` fails on the old rule |
| A census judged "inside the noise" by eye | the difference a seed and its standard error, outside past two errors (`ledger::mean_se`) | 18 Oct, LR 24 | the means are still printed | a null run 0 with an error of 0; `the_standard_error_of_a_sample` |
| The board's posture as a threshold ladder (Defend > Economy > Expand > Connect > Fortify) | five scores, each weighted, blended with the ladder's pick by `posture_scored` (1) | 18 Oct, LR 25 | `posture_scored = 0` | `each_posture_scores_its_own_case`; 48 seeds under `posture`: -0.23 +/- 0.74 against the ladder's |
| The dodge's point across the shell's line, on the tank's side (the first reachable since LR 52) | the point `drive_to` foresees clear of the splash at the landing (`dodge_plan` 1) | 20 Oct, LR 54 (Paul's choice) | `dodge_plan = 0` | families 89.6% to 100% with the release; 96 naval seeds +0.81, error 0.40 (LR 53) |
| At its point before the shell lands, the tank free to any errand | an errand foreseen in the splash at the landing waits (`dodge_release` 1) | 20 Oct, LR 54 (Paul's choice) | `dodge_release = 0` | families 96.2% to 100% with the plan (LR 53) |
| Given-up targets remembered for the match (L6) | memories that run out after `forget_for` | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -forget` | alone, 12 seeds: -0.36, error 0.23 (LR 24) (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| Each decision taken at once, every think (L7) | desire vectors, a turret held until a challenger wins three picks, the Auto rung confirmed over two thinks, the stance from smoothed odds | 20 Oct, LR 46 (`smooth` a weight) | `w_smooth = 0` (its default 0.05 blends the parts that blend) | 7.55 a seat against 9.47, worse; 12 seeds: -1.98, error 1.25 (LR 11, 15, 24); the old play against 0.05, +0.47, error 0.60 (LR 46) |
| Each tank on its own picture of the map (L5) | the team board: influence, front, defence points, jobs kept, posture | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -board` | 12 seeds: identical matches, it changes no decision alone (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| Each tank balancing only itself | a hurt tank at the hottest point swapped for a fresh one | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -team_rotate` | 12 seeds: -0.83, error 0.87 (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| Every gap of 20 water tiles or fewer bridged | boat or bridge by price: long bridges and scarce trees cost more | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -priced_crossing` | 12 seeds: +0.11, error 0.56 (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| The wedge kept on a bridge | the fleet in file on a bridge | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -column` | 12 seeds: -0.06, error 0.41 (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| Trees felled only when a build is short (L8) | after filling up: an item and an improvement for the base, then a road out | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -cycle` | 12 seeds: +0.31, error 0.43 (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| Any island a base | a small island with few trees a poor site | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -island_site` | 12 seeds: identical matches; never asked in a fleet (backlog 20) (LR 11, 24); every part on together -0.85, error 0.59 (LR 55) |
| A leader fetching a far pillbox alone | the fleet gathered first, escorting it home along the road | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -escort` | 7.70 against 7.70: never reached in 900 s; 12 seeds: +0.01 (LR 16, 24); every part on together -0.85, error 0.59 (LR 55) |
| A bridge given up left half laid; a leader asking one wingman for its next tile | part-laid bridges on the team board, taken by the next teammate with material | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -bridge_jobs` | 7.31 against 8.01, inside the noise; 12 seeds: +0.44, error 0.91 (LR 18, 24); every part on together -0.85, error 0.59 (LR 55) |
| Direct fire taken where it falls (only kiting, against the one enemy a tank fights, keeps out of reach) | the duck: out of a hostile gun's range or line, seen or remembered, when it is ready before one's own | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -duck` | 7.01 against 7.52, inside the noise; 12 seeds: +0.39, error 1.04 (LR 21, 24); every part on together -0.85, error 0.59 (LR 55) |
| The Auto ladder's first rung that holds (L1) | the team's posture leaning the pick, `(1 - w) order + w lean` | 20 Oct, LR 55 (every part on, Paul's rule of LR 54) | `caps = baseline, -posture` | 48 seeds: -0.81, error 0.65; scored (LR 25) -1.04, error 0.58 (LR 24, 25); every part on together -0.85, error 0.59 (LR 55) |
| The dodge planned and released against the one shell dodged | every hostile shell landing within the horizon, each at its own landing (`dodge_every`) | 20 Oct, LR 56 | `dodge_every = 0` | two-shell families 76.5% and 79.9% to 100%; with LR 57, the guard +0.21, error 0.68 |
| The guns free while a shell is in flight (a shot turns the hull) | a shot that would carry the tank into a splash waits (`dodge_fire`) | 20 Oct, LR 57 | `dodge_fire = 0` | `a-dodging-tank-holds-its-fire.toml`; the guard as above |
| The duck's point picked anew each think, nearest the errand, reached or not | held while safe, else foreseen out of reach or line by the gun's ready tick (`duck_plan`) | 20 Oct, LR 57 | `duck_plan = 0` | ready-gun family 76.6% to 83.4%; the guard as above |
| The window's pilot playing on no settings at all | the pilot on the weights page's settings, which start from the team file or `--weights` | 18 Oct, LR 17 | removed | `client::tune` |

## Current, challenger off

None since 20 October: by Paul's rule (LR 54) a part lands on, and the
parts built off went on together in LR 55 (archived above).

## Shown impossible

| Idea | Why not | Evidence | LR |
| --- | --- | --- | --- |
| Dodging a direct shot once it is fired | the shell flies five ticks at most; a tank at full speed turning its hardest moves about 0.4 tiles off the gunner's lead in that time, and the hit radius is 0.8 | `a_led_direct_shot_cannot_be_dodged`: hit from 4, 7 and 9.5 tiles whatever the target did | 19 |

## Still as they were

Not every finding of LR 2 has an answer yet: the observation is still
a full copy of the world per bot per tick (L2); the `doing` words are
still written every think, beside their codes, measured as costing
nothing (L3, LR 22); and the Commander's chain of branches, the
wingman's priorities are still first-match orders (L1; the Auto
ladder has a blend under `posture`, LR 24, and the board's posture is
scored, LR 25).
