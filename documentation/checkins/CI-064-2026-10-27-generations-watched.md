# Generations Watched: Clips of Each Learned Tank, a Gauntlet of Single Decisions, and What the AI Costs

**Check-in report CI-064.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 27 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 102; journal 0103;
`reports/2026-10-27-generations.html`
(https://claude.ai/artifact/Vg35Adn3MmqibS7k69HLW7);
`scenarios/gauntlet/`.

---

## Abstract

Each of the seven milestone nets was recorded at work on its defining
test and drawn into a GIF in plain Rust (`clip.rs`, `make clips`): ten
clips, one of them a duel and one a real match. A gauntlet of nine
single-decision tests on bare ground (`make gauntlet`) read every net
and the scripted brain on the same 200 seeds, with reaction times. It
found two shortcuts the training maps hid: the map 7a net learned
"few shells, go home", and the map 4 net learned the guarded scene,
not the depot. It also found the scripted brain's blind spot: it
fights inside a pillbox's ring. The match reading, now counting damage
dealt, shows living and fighting pulling apart. On one M1 thread a
decision costs 10.6 us, and a 16-seat match of nets runs at 197 times
real time. The report is an HTML page with every clip.

**Index terms:** visualisation, behavioural testing, shortcut learning,
reaction time, computation cost.

---

## I. Introduction

### A. Intent

Watch every generation, measure reactions and progress, write the
report as a page, and build gauntlets of single decisions.

### B. The prompt

> Can I get a short GIF of every generation of the tank AI battling. Show reaction measurements and progress on census. For each generation (or major milestone), show the capability of the tank AI during its action. Write a lab report in html. Describe out methods of using training based of recently designed brainz, which we are using as a basis to help develop the sandbox for testing and training. Descibe the quality of improvements. Describe the computation time of using the tank with its AI and only its AI. We want to watch, short clips of action. Please create a set gauntlets of test with maps for testing, and single decision with little distractions for testing.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "a short GIF of every generation" | A recorder in the scenario format and the built grounds; frames drawn in Rust; ffmpeg packs them | A screen capture of the window: the nets play headless, and a recording replays exactly |
| D2 | "show the capability" | Each net on its own test, on the first held-out seed it passes; the untrained on its first | A random seed: a failure shows nothing of what was learned |
| D3 | "battling" | A duel clip and a 30 s clip of a real match beside the milestones | — |
| D4 | "single decision with little distractions" | Nine tests on bare grass, one decision each, a timed reaction | The training maps: they mix several decisions |
| D5 | "with maps" | One test on terrain (G9, water) beside the eight on grass | — |
| D6 | The match reading favoured a net that does nothing | Damage dealt and kills added, every net read again on today's code | Report survival alone |
| D7 | "computation time ... only its AI" | A decision, an episode, and a 16-seat match all nets against all scripted, on one thread | — |
| D8 | Captions ran off the narrow clips | Shorter captions, the clips remade | — |

---

## III. Methods

`make gauntlet` (200 seeds a test from 9,500,000; every net widened to
today's senses; the scripted brain is a Commander in the net's seat).
`make clips` (a frame every 2 ticks, 15 s at most, the match clip
30 s). `make train CHECK=net MATCH=40` for the untrained net and each
milestone net. `TIMING=1` for the cost. The page is built from
`build/train/gauntlet.tsv`, the match readings, and the census reads
of LR 97 to 100.

---

## IV. Results

LAB-REPORT-BRAIN.md section 102, tables B to D.

---

## V. Discussion

The training maps say what a net can do in the situation it trained
on. The gauntlet says what it learned. The two were not always the
same, and the gauntlet is cheap enough (a minute for every net) to run
after every training run from now on.
