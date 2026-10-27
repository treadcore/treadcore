# The Learned Tank Drives and Shoots: Periscope Senses, Calls to Friends, Evolution in Rust, the Census's Verdict, and Release 0.2.0

**Check-in report CI-059.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 97;
`AI-BRAINZ-PLAN.md`; roadmap backlog 69 and 70 and goal 6;
`build/train/` on the Mac; `build/net3/` and
`build/converge/20261026-181011/` on the census machine; the page
`reports/2026-10-26-the-learned-tank.html`
(https://claude.ai/artifact/2vzTNKcmB71GYLodeZLTjS).

---

## Abstract

The learned tank was taught, by evolution strategies in Rust on eight
cores, to drive to an enemy, a depot or a pillbox (999 of 999 held-out
grounds) and to kill a still enemy with the direct gun while still
going to the other two (999 of 999). Its senses grew from 101 to 133:
instruments, a periscope of four exact contacts, and a channel for a
friend's call; a team's shared queue of calls (help, attack, take)
lets learned tanks ask one another for help, and speaks to scripted
teammates as pings. With the periscope, phase 1 passed by generation
67 where the compass alone needed 500. Two of phase 3's rules were
found empty by reading the untrained net first, and amended before
training; a shoot-only net forgot how to go to a depot, so phase 3
trains all three grounds. In the census a team of the trained net
reads -23.52 +/- 0.38, below a random net (-20.35 +/- 0.25): the rule
fails, and says what to teach next. dodge_horizon 2 became the
default after ten more held-out reads. Release 0.2.0 was prepared.

**Index terms:** evolution strategies, sensor interface, curriculum,
catastrophic forgetting, multi-agent communication, release.

---

## I. Introduction

### A. Intent

Teach the network tank to drive and shoot at an enemy, a depot and a
pillbox; give it tank-like sensors and a way to call friends; write
the AI Brainz plan with diagrams and a page to show it; accept the new
defaults; release the next minor version.

### B. The prompts

> write a lab report. and lest go to the next . teach to network tank to drive and shoot, and enemy a depot a pillbox.

> can you create documentation and show the html about the new design of the sensors. add more sensors if approporate. give tank like sensors. like pariscope tank controls. almost like a submarine, what kind of general positioning and relative sensors can we provide the neural net to use as input data, so we can still be performant. We want to train.  Thanks. Write documentation describing the current model of brainz, and how we are using a sensor network to compare. Also give general request commands to the training to it can cally friends through shared requests queues. Make diagrams and update the graphics for the entire treadcore. in fact during this it seems we want to write a full relaese version. Fully tag. It runs so well. This is the next minor version. Tag it. Marge for deploy. Do all the make targets for release. I want to show off the html and the new ai learning. Our current methods and proposed upgrades. write them out in the AI Brainz Plan,. Also grab the results from the copal census runs. Accept the new defaults.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "teach to network tank to drive and shoot" | Phase 1 (go to) first, then phase 3 (aim) from it; evolution strategies in `learn.rs`, an ignored test as the census is, `make train` | A new crate: the census's own pattern does it with none |
| D2 | A tank sees 10 tiles; targets 10 to 20 off | The ground's enemy felt at any range (`advantage`): phase 1 is going to, not searching | Targets inside sight only: too near to learn steering |
| D3 | A one-seat world ran past its end (hung test) | A guard on `over` in `play` | — |
| D4 | "pariscope ... almost like a submarine" | A periscope of four contacts with exact bearings (sine, cosine), the enemy's closing and aspect; instruments; a Call channel | A picture of the map: a convolution and a GPU, and a different size each map |
| D5 | "call friends through shared requests queues" | `Calls`, a team's queue (help, attack, take), each also a `Ping`; pings heard come back as calls | A new wire command: a wire change for a brain's idea |
| D6 | The net's shape changed (133 in, 46 out) | Phase 1 retrained from scratch on the new senses; the first-senses run kept as the comparison | Graft the old weights: a different input layer |
| D7 | Phase 3's first rule met by an untrained net (99.8%, the mortar) | Amended before training: the direct gun only | Train against an empty rule |
| D8 | RuleBot fires nothing on the grounds; a still tank is never missed | Hits a shot moved to phase 3b (a mover); 3a's bar a direct hit on 95% | Compare with a baseline of zero |
| D9 | The shoot-only net forgot the going (depot 9.3%) | Phase 3 trains all three grounds in turn | Keep a net per task: Paul asked for one tank that does both |
| D10 | "grab the results from the copal census runs" | Read the run: dodge_horizon 2 best in five generations, ten held-out reads | — |
| D11 | "Accept the new defaults" | dodge_horizon 2 only: the three other passing trials were it plus random draws | Take the draws too: never read alone |
| D12 | A default changed under the run | The run stopped, families read, restarted on the new defaults after the net censuses | — |
| D13 | Is the trained net a better tank? | A census of it and of a random net of its shape, same seeds, rule first | Report the grounds only |
| D14 | "the next minor version. Tag it." | 0.2.0; PROTOCOL.md to wire 16 first (ROADMAP goal 6); CLAUDE.md's pause lifted for this release | 0.1.0: Paul said the next minor |
| D15 | `make shoot` needs Hyprland, xdotool, grim | Run on the census machine, in a worktree, so the run's tree is untouched | The Mac: no Hyprland |
| D16 | "show off the html" | A page in the site's own look, as the release's report page and as a shared page | A Markdown page: nothing to drag |
| D17 | Publish to crates.io a day after the tag | Handed to Paul | — |

---

## III. Methods

On the Mac (M1, 8 threads): `make train` (ES: 48 pairs, 12 grounds a
generation, sigma 0.05, Adam 0.02, decay 0.0005); held out: 999
grounds from seed 9,000,000. Rules in `build/train/NOTES.txt` before
each run, amendments dated in place. On the census machine at
7d0af54: `build/net3/run.sh`, two censuses (110 seeds from 500,000,
every map, 24 jobs), then `make converge` with `SINCE` now. `make
families` and `make test` on the Mac before the default commit.

---

## IV. Results

| Read | Result |
| --- | --- |
| dodge_horizon 2, the run's held-out reads | +2.66, +1.71, +1.61, +1.44, +1.09, +1.98, +2.44, +1.84, +2.00, +1.73 (each +/- 0.26 to 0.30, 400 seeds) |
| make families at 2 | all nine hold |
| Phase 1, first senses, 500 generations | enemy 99.7%, depot 100%, pillbox 95.2% +/- 1.2 |
| Phase 1, untrained, 133 senses | 13.2%, 1.8%, 0.3% |
| Phase 1, 133 senses, 500 generations (130 s) | 999 of 999 |
| Phase 3, shoot only (83 s) | a direct hit 999 of 999; then depot 9.3%, pillbox 23.7% |
| Phase 3, drive and shoot (98 s) | enemy killed 333 of 333, depot 333 of 333, pillbox 333 of 333 |
| Census, nets/phase3.bin | -23.52 +/- 0.38 (island -28.91, fortressislesx -18.02) |
| Census, random net of its shape | -20.35 +/- 0.25 |
| make test | passes |

---

## V. Discussion

The grounds taught what they held: an exact bearing makes going to a
point easy, and a curriculum keeps what it rehearses. They held
nothing that shoots back, and the census measured the cost of that
precisely: a net that charges is worse in a match than one that
wanders. The next phase is the one the census asks for: live.
