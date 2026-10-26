# Big-Lizard and the Senses: A Route Forecast Found, Fixed and On, and a Learned Tank Designed in Rust

**Check-in report CI-057.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` sections 93, 94 and 95;
roadmap backlog 70 and 71; `build/route/` on the census machine; the
big-lizard page, https://claude.ai/artifact/Mias4vtFMwmMM6Fqsd6wsa.

---

## Abstract

`dodge_own_route` failed its first census on one map, big-lizard
(−6.48 ± 1.56 over 10 seeds). A probe compared the own-route forecast
with a fresh search at every ask: at sea they parted three times as
often as ashore and turned the dodge's verdict three times as often.
A test showed the own route is the truer forecast near guns: 0.00
tiles off against 7.85. Over 120 seeds on the Mac, big-lizard read
−0.97 ± 0.66. The fix is the own route ashore only. Three reads on
110 fresh seeds each passed their rule: every map −0.18 ± 0.37,
big-lizard +0.58 ± 0.81, and the generated maps +0.51 ± 0.43 (the one
map below in the first read). `make families` held. The fix is on by
default, and the run restarted on the new defaults. LR 94 is a primer
on the day's implementation, with lessons. LR 95 designs a learned
tank in Rust: senses as a compass of signal strengths, about 100
inputs, every action, rewards from driving toward a target upward,
nine gated phases, evolution on the CPUs first.

**Index terms:** route forecast, multiple comparisons, sensor
interface, evolution strategies, curriculum.

---

## I. Introduction

### A. Intent

Find why the own route cost on big-lizard and fix it. Explain the map.
Write the implementation out in full, with the lessons it needs. Then
design the neural-net tank Paul described, in Rust, with milestones.

### B. The prompts

> find why big-lizard costs and fix the route search. also give me a reading link to explain biglizard. also write out your implimentation in the lab report. be detailed, and also give me introductory information and teach the needed lessns.

> i want to be able to start converging the neural net as well. what kind of general sensors can we give to make the smallest reasonable neural net. Can we use thsee conclusions as inputs. Can we give a sensor like interface to give only reduced knowedge so we can minimze thenumebr of inputs in the transformer. I actually dont know all the AI terminology. I want a tank that has all the actions available to it. I know we need a million generaions , but can we conceptualize this in the lab report about what data is needed. perhaps direction indicators. signal striength. we want a sensor like interface so the tank can expieience its rewards and its decision. Do do this. Can we have a good test plan with milestones to move to the next phase. First the tanks need to be able to be reward to driving towards an enemy. or driving towards bases or pillboxes. Since we have a working Brainz, we also want a parallel system so we can have train the bots. We can baby step the training to with larger goals as we develop milestones of Tank Generational AI Neural Net. Is this a plan for some library. I read earlier I can convert a box to ubuntu. i can use an 1080TI or a 4060TI. I expect to install ubuntu 26.10 I heard its LTS. I also prefer Rust Neural Nets. I really hate python.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | A census says "costs", not why | A probe: both forecasts at every ask, where they part and where the verdict turns | Guess a cause from the map |
| D2 | The first big-lizard seeds the Mac played were not the census's | Worked out the census's big-lizard seeds (n ≡ 3 mod 11) and replayed the worst (170063) | — |
| D3 | The Mac read −13.86 where the machine read −16.48 | Reported as the two architectures' different matches, the direction the same | Call it a fault |
| D4 | Ten seeds of a wild map | 120 big-lizard seeds on the Mac, pooled within the Mac | Pool with the machine's 10: two machines |
| D5 | Afloat: three times the parting and the turned verdicts; few afloat asks | The own route ashore only | Drop the feature: the saving is ashore |
| D6 | "a more truthful forecast" is a claim | A test with a gun beside the way: own 0.00, searched 7.85 tiles off | — |
| D7 | The rule before the reads | NOTES2: every map and big-lizard alone on 110 fresh seeds; a map past two below read alone, that read deciding | Promote on the Mac's 120 seeds: not the census machine |
| D8 | A's generated maps −1.83 (10 seeds) | Read alone, 110 seeds: +0.51 ± 0.43; passes | Fail on 10 seeds |
| D9 | A route census and the run cannot share the machine | Each read queued between the run's censuses, the run resumed | — |
| D10 | Every rule passed, families held | On by default | — |
| D11 | The defaults moved under the run | A new run from this change (CENSUS.md 4a) | Keep the old rows in the fit: they read another brain |
| D12 | "a reading link to explain biglizard" | A page: the map drawn tile by tile from its file, guns' reach, the forecast diagram, the reads, four lessons | — |
| D13 | "Rust Neural Nets. I really hate python." | LR 95 all Rust: ES with std and threads; Burn for gradients | LR 90's PyO3 bridge and PyTorch |
| D14 | "a million generaions" feared; 24 threads now | Evolution strategies first, on the CPUs, with episode costs estimated (to measure in phase 0) | Wait for the GPU |
| D15 | "Ubuntu 26.10 ... LTS"; 1080 Ti | Checked: 26.10 is interim, 26.04 the LTS; 580 is the last driver for Pascal: the 4060 Ti | — |

---

## III. Methods

The probe (removed after): in `drive_path_by`, under `PROBE_OWN`, both
forecasts computed and compared (largest distance apart over the
ticks), and in `hold_fire` both verdicts; counters printed at the
census's end. Runs on the Mac: seed 170063 on the census's map list;
big-lizard alone, 40 seeds from 170000 and 80 from 170040, `make census`
with `JOBS=8`, the seeds' differences pooled from the job logs. On the
census machine, `build/route/run2.sh` and `run3.sh`: each read after the
run's census in hand ended, the run resumed after.

---

## IV. Results

| Read | Result |
| --- | --- |
| First census, own route everywhere | every map −0.53 ± 0.48; big-lizard −6.48 ± 1.56 (10 of 10 below) |
| Probe, big-lizard | ashore 14% apart > 1 tile, 0.6% verdicts turned; afloat 32%, 1.7% |
| Mac, big-lizard alone, 120 seeds | −0.97 ± 0.66 |
| Test round a gun | own 0.00 tiles off, searched 7.85 |
| A. ashore only, every map, 110 | −0.18 ± 0.37; generated −1.83 ± 0.87 |
| B. big-lizard alone, 110 | +0.58 ± 0.81 |
| C. generated alone, 110 | +0.51 ± 0.43 |
| make families at 1 | all nine hold |

Errors: a heredoc took `` `SINCE` `` for a command in the lab report's
first draft; restored.

---

## V. Discussion

The cost on big-lizard was mostly a crowded map's noise on ten seeds,
over a smaller real effect that came from the sea. The fix removed the
sea case and kept the saving. Two of the three "failures" in this
story were the tail of many comparisons, which is why the rule now
reads a failing map again before it decides. The learned tank is
designed to be judged the same way: each phase gated by a family, the
census the last word.
