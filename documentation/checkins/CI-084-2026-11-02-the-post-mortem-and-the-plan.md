# The Post-Mortem and the Plan: Physical Ranges, the Wingman's Numbers, Eight Personas, and the Escort Gauntlets

**Check-in report CI-084.** Paul Richeson, treadcore.org; Claude Code
(Claude Fable 5.1) as the pair. 2 November 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 127; ROADMAP backlog 79;
the book `reports/2026-11-02-the-plan/` (sixteen chapters, from orrery's
`docs/book/`); the site's `reports/index.html`; orrery's trade guard
(`tools/plans/treadcore.py`, `gives_up`, orrery 84ce3b4) and book tooling
(`tools/report/chapters.py`, orrery 3d04507).

---

## Abstract

A week of automated training (25 October to 2 November) was read a
report at a time and written up as a book: the project, the goals
rewritten, Paul's directives in plain words, the week, how the line
works, what the numbers said, the lessons, a register of 380 open
entries, and a plan in seven chapters with 106 references in IEEE form.
The plan's first step was built the same day: every distance, time and
count of the brain has a range set by the simulator; the wingman's five
constants are weights; eight named personas join the roster; three escort
gauntlets measure a wingman's station. The scripted wingman reads 29 of
30 covering its leader, 30 of 30 coming back after a shell, and holds
eight tiles for 57 to 59% of a keep-station run. The brain's 151 tests
and the simulator's 59 pass; no default moved.

---

## I. Intent

Paul's prompt, word for word, as typed:

> we are on max fix mode. Where we read all the lab reports one section at a time. during the reading of the lab reports we write the super long mega chapter post mortum. We are done with about a week of training. We are adding playing ability of Brinz and AI Nets and want to make this game playable by a human and AI bots. The AI bots have toggles to switch between goals and personalities. We need a testing plan and ways to assign fleet messages and ally bot play. Read all the goals of Orrery and Treadcore. We have many past reports of how we did it. Look into unfinished tasks and incorporate them correctly with much consideration and GUI components to make all features of the application somehow have an interface element. Again we are on max effort and you longest thinking model, Please read the code, find the goals of the project. rewrite the entire goals and asiprations and how to linking back to past lab reports. Do not edit past lab reports unless necesary. Write code, adjsut parameters, write lab reports HTML with diagrams. We want less specifics, and more overview of the process. We want summaries of prompts and goals and directives, spell corrected and simplified. Analyize all the past logs and current data set, to setup tests and training to fix what needs to be fixed. We want to use the topography of local maxima to differentiate different named personas (perhpas use the civiziliations of CivV and make them like the historic cultures according to their strengths. Again. looking at the weights, I want a wingman trained, such that The wingman following my moves, like an escort mission. We need training gauntlets to help converge the data to all these abiliites. We might need to increase the size of the transform matrix, or to make it longer with multiple layers. Please only use proven generational developent technique citing offical sources, Prefer referencing IEEE reports and preset in IEEE format. Write final output in markdown, and convert it to chaptered html linked from index.html for treadcore. Also create a Lab-Reports section for treadcore. and in this we our defination of design, we have implimentatin of design, we training of Weights on Brainz Programmed, We have training of transforms matrix to become a neural net player. We then combine both training, and write a new program called Orrery to orchestrate the depdencies and the order of seeds, and the promotion and values and organizing the work to conerge into a well defined player. Right now, we are fighting optimzing training of values which were suspoed to be physical contraints, so we start this process by locking physical contraints into a range of values. Also, physical values, we have a range of artillerty, we ahve a range of direct fire, we have ranges of interceptor and long range drones, Can we set their values to be reasonable for a distance. And also the lag time in the air, would be for the scale we are at. Alrhough some maps are biger than other, we need to come up with a scale system so we can get ballistic arcs and drone speed . We also want to setup increase bonuses for economy like Refineries, and Smelter, and Wood collection harvesters. We want a funn zero sum economy, where all minerals (except growing trees) are limited in resource. We want to add a logisitic level, where each Base has a agent worker type which will do the work (remmemebr bases rechage empty supplies) . And same with pillbox. Except each time a pillbox gets hit. it looses its ammo supplies, they all go boom. And then the pillbox only has its newly created ammo. Please describe how citiez develop, and refiineries are built. Perhaps we can add more AI, to have one teammate stay back and constantly issue the city buildig commands (while defendig and fighting). We shoud have training data to be that like the map rule game of.

followed by a pasted description of a trade economy (ports with ports,
factories with factories, payouts by distance and alliance, embargoes on
attack). Later the same morning: "continue when the readers finish. move
to design.. what background process. can you work only in this agent. no
background agents. finish. i paused something." The work from then on
was done in one session, with no agents.

## II. Decision log

| Decision | Cause |
| --- | --- |
| The book is written at the altitude of the process, with the detail linked, not repeated; prompts are given in plain spell-corrected words with the originals cited | the prompt: "less specifics, and more overview ... summaries of prompts ... spell corrected" |
| The plan's first step is ranges, not decrees; three decree candidates are named for Paul (`tree_material` 4, `strikes_max` at the bay's 3, `coming_r`); `bio_period` is written as a decree because Paul said "that's perfect" on 1 November | CLAUDE.md: only Paul decrees; the prompt: "we start this process by locking physical constraints into a range of values" |
| The wingman's five constants become weights with their constants as defaults | nothing plays differently; a person, a persona and the search gain the dials at once |
| Personas take slots 8 to 15 and leave 0 to 7 as they are | a slot once given keeps its meaning (`bot_style` is a choice) |
| Each persona stands on a measured hill or a measured style where it can and says "unread" where it cannot; its census is its certificate | the terrain is thin (58 of 77 maxima unsettled on 1 November) and a persona with no read is a hypothesis |
| G10's bar is set where the scripted wingman stands (a half), with the 57% written in the file | a gauntlet nothing passes is no floor; a gauntlet the scripted brain fails is a finding, kept |
| The economy, the logistics, the scale header, the fleet messages and the game-side slot are designed with their first step named, not built | each is a rule or wire change (a `GAME_VERSION`, the evidence anew) or a day's work; the prompt asked for the design and the plan |
| The reading agents were stopped and the rest done in one session | Paul: "no background agents. finish." |
| References were found and verified by web search before any was cited; two themes came back thin (one source each) and the chapters say so | the prompt: "only use proven ... techniques citing official sources ... IEEE" |

## III. Methods

1. Twenty-eight orrery reports and design documents read section by section into structured summaries (question, prompts, built, measured, decisions, open, lessons); treadcore's roadmap, brain report, check-ins, journal, code and the week's data read directly.
2. The weights classified by kind (physical 57, share 31, worth 33, choice or switch 14) and each physical one given a range from the simulator's constants; every default checked against its range by the brain's test `every_range_reads_and_holds_its_own_default`.
3. `fly_wing` reads the five numbers from `Settings`; the constants stay as the defaults.
4. Personas as `profiles/persona-*.weights`, `styles.rs` ROSTER 8 to 16, the `bot_style` choice list extended; the roster's tests widened.
5. `scenario.rs`: `goal = "escort:N"`; `Outcome.station`; `[expect] station` and `[reward] station`; `learn::REWARDS` extended; three gauntlet files.
6. `rules::physical()` and its test; the Lab Reports page and the book built by `tools/report/chapters.py`; every relative link checked.
7. The scripted wingman read on G10 to G12 by `scenario_many`, thirty seeds from 1, the net seat made a Commander.

## IV. Results

- Tests: treadcore-sim 59 of 59; treadcore-brain 151 of 151 (15 ignored), including the roster, RANGES and course tests.
- Gauntlets, scripted wingman, 30 seeds: G10 0 of 30 at a 0.7 bar (within 8 tiles 57 to 59% of the run on every seed), 30 of 30 at 0.5; G11 29 of 30; G12 30 of 30.
- Ranges added: 57 physical, 24 shares, 1 decree (`bio_period`); defaults all inside.
- Personas: 8, read at 400 seeds the same afternoon (score a seat; built a team): Rome −5.13 ± 0.58 (+5.96 built), China −5.17 ± 0.67 (+4.53), Mongolia −0.30 ± 0.65 (+3.53), Shoshone −7.15 ± 0.63 (kills less deaths −0.49 ± 0.07), Venice +0.39 ± 0.54, Japan −0.63 ± 0.61, Polynesia −0.36 ± 0.45, Assyria −0.96 ± 0.63 (+5.32 built). The builders build and the score, which pays for the walk, punishes it; Shoshone's pair of dials loses fights. Read again under game 5.
- The book: 16 chapters, 106 references (36 IEEE), 29 reports on the Lab Reports page.
- From the same morning, in orrery (84ce3b4): the trade guard; replayed over the sixteen candidates since the evening before, it would have held eleven; `cycle` off read +0.33 ± 0.49 on score and −5.03 ± 0.39 built, −4.14 ± 0.27 road, −0.66 ± 0.07 kills less deaths a seat at 400 seeds.

## V. Discussion

The week's numbers set the plan's order. The search had moved numbers that were constants in all but name; ranges stop that at no cost to play. The wingman asked for exists in the scripted brain and nobody could touch it; five weights fix that today, and the gauntlets say where it stands: it covers and it returns, and it is slow to close because it is a follower, not a formation. That is the design of chapter 11, and the trained wingman's first number to beat. The personas are honest about the terrain: three hills on six dials. The economy is the large thing and is staged behind game versions, because every version starts the evidence anew. What is still Paul's: the three decree candidates, the reward for going forward, the persona names, and whether a person's ping is always a call.

## VI. What is left

In the book's order: the economy's stage 1 (a hit pillbox loses its stock); the game-side slot and the `patch` row's refusal; fleet messages and a wingman stance as elements; the shape watch; the pinned-test lint; the persona reads and their rows on the Lab Reports page; a playtest afternoon.
