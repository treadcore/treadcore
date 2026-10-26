# The Second Day: Two Inert Weights Explained, Every Weight Screened, a Learned Brain Studied

**Check-in report CI-055.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` sections 89 and 90; roadmap
backlog 69 and 70; `build/day/` on the census machine; the weight
ledger, https://claude.ai/artifact/KdMP63zpmJVWgm4ChwgBpf.

---

## Abstract

w_exposure and w_mine read exactly zero at half and double in the
night's screen. Neither is a bug. w_exposure ties: above w_flank (0.3)
every value orders the firing points the same. w_mine rarely decides:
a probe over 165 improvement steps found half changed no pick and
double six. The second day was queued on the census machine. It has
three arms where those weights can act and a two-stage screen of the
68 scalar weights not yet screened. It ends with SPSA, now bounded to
half and double, and two held-out reads of SPSA's point, the rule
Paul chose. A dashboard, the weight ledger, shows the weights over
time. A study (LR 90) lays out a neural-network brain trained on the
census's rewards. The sim's CPU time is the cost, not the network, so
more CPUs are the lever. The day's results are pending.

**Index terms:** brain census, slope screen, successive halving, SPSA,
reinforcement learning.

---

## I. Introduction

### A. Intent

Explain the two inert weights, run the next census Paul approved, and
carry backlog 69 on through the day: screen what is left, search
within bounds, read the result twice. Draw the weights' history.
Study a learned brain.

### B. The prompts

> continue with census family.

> 1. yes. yes. I want you nest ideas to improve in best industry practices. Start again. We got another day. Can you create a lab report showing a bar chart of the range and the timeline measurement values so we can see how these decision priorities have changed and grown. also we have been adding more variables. Please start more analysis and converge into good values. Start on remote.

> If we were to use Cuda, or some sort of Mac or PC based hardware acceleration. Is there a way to define this such that we start training a normal AI neural network given our current reward system. Perhaps we need some simple low effort rewards, like moving to the center of the map. and shooting ammo with hits. It needs rewards in order to find how to aim. Write a lab report about this. Perhaps we can consider training this faster on a GPU.

> I can slo spin up some more CPUs.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "continue with census family"; LR 88's next: why the two weights never act | Read the code first; no census until the cause was known | A census at new values blind: it might read zero again |
| D2 | The stand's cost: `guns * w_exposure + (\|k\|/3) * w_flank` | w_exposure tied above 0.3; w_flank tied below w_exposure, predicted zero | — |
| D3 | `Cycle::improvement` is only reached in play | An env-gated probe pricing each improvement at half, default, double; 24 census games on the Mac; probe removed | A census per value: hours for what a probe shows in ten minutes |
| D4 | Paul: "1. yes. yes." | The arms (w_exposure 0.15, w_mine 2, and w_flank 1.5 added by D2) and LR 88's two rules adopted | — |
| D5 | "converge into good values"; 68 scalar weights unscreened; 110 seeds a census is 6.6 min | A two-stage screen: 55 seeds each end, 110 for any end past 1.5 errors (successive halving) | 110 seeds for all: 14 h, against about 10 |
| D6 | Switches, the skill dial, `long_game` and weights for absent features are not slopes | Left out of the screen, each named in NOTES | — |
| D7 | Paul chose the bound | `BOUND` in `scripts/spsa.sh`, default 2; copied to the machine before the run | Bound in the day's script only: every later SPSA would forget it |
| D8 | Paul chose two held-out reads | Two reads of 400 (from 900000 and 1000000) | One of 800: the same seeds, but two reads show whether they agree |
| D9 | The machine's tree is at 7c97aa9; the brain's code unchanged since | Run there; only `scripts/spsa.sh` differs, noted | Pull first: Paul has not asked for a commit |
| D10 | "a lab report showing a bar chart of the range and the timeline" | A dashboard of five datasets: defaults by commit (git), default changes, the screen, every census on the machine, every value tried | A static chart in the lab report: Paul asked to see change, and the day's rows will be added |
| D11 | "next ideas ... best industry practices"; LR 87 lists SPSA, CMA-ES, BO, MAP-Elites, a league | Eight more in LR 89: FDR and shrinkage, SPRT, variance reduction, a results ledger, a canary census, screening designs, more machines, a learned brain | — |
| D12 | "start training a normal AI neural network"; "train ... faster on a GPU" | A study (LR 90), backlog 70; nothing built | Start building: a new crate is a step Paul chooses |
| D13 | 625 ticks a second a thread measured from the night | The sim, not the network, is the cost; say so plainly, and that more CPUs help | — |
| D14 | "I can slo spin up some more CPUs" | A census stays on one machine; a queue of censuses can be dealt to several (LR 89, idea 7) | Pool jobs across machines: CENSUS.md section 5 forbids it |

---

## III. Methods

The probe (Mac, removed after; `git diff` clean on `cycle.rs`): at the
improve stage, `Cycle::improvement` called with `w_mine` times 0.5, 1
and 2, each pick printed under `PROBE_MINE=1`:

    PROBE_MINE=1 CENSUS_MAP=<the eleven maps>,gen CENSUS_RESPAWNS=1 \
      CENSUS_SEED0=500000 CENSUS_SEEDS=24 CENSUS_SECS=900 \
      cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture

On the census machine at 7c97aa9, `build/day/run.sh` under `nohup
setsid`, started 09:21; its files (`NOTES.txt`, `pick.py`, 138 weight
files in `screen/`) written on the Mac and copied. Stage 1: 135
censuses of 55 seeds from 170000. Stage 2: `pick.py DAY stage2`.
SPSA: `TUNE="$(pick.py DAY tune | head -1)" BOUND=2 STEPS=24 SEEDS=48
SEED0=800000 MAP=all JOBS=24 make spsa`. Reads: `TRIAL=theta.toml
SEEDS=400 SEED0=900000` and `1000000`.

The ledger's data: `git log` of `weights.rs` (27 commits), each
default parsed from the `Name "key" value` lines; `build/census/*/`
`pooled.txt` and `trial.weights` on the machine (70 censuses, 638
values tried); the night's screen from `build/night/tune.txt`.

---

## IV. Results

| Read | Result |
| --- | --- |
| Probe, 165 improvement steps | 159 the same pick at all three values; 6 a mine at double only; 0 changed at half; works never offered |
| Arm w_exposure 0.15 | -0.37 +/- 0.36 over 110 seeds: acts, inside the noise, below as expected |
| The rest of the day | running; to be added here when read |

No errors in the run so far. The weight ledger's own check had not
reported when this was written.

---

## V. Discussion

A screen at half and double reads a weight only where the weight has
something to decide. Both inert weights sit behind a comparison with
another number (w_flank; a mend's hurt), so their range of action is
set by that other number, not by their own default. The day's arms
test exactly that. The screen of the remaining 68 will show how many
more are like them; dead weights then leave every search.

The study's main finding is about cost: a network for this game is
small and the sim is not, so a GPU would mostly wait for CPUs. A
learned brain is feasible with what treadcore has. Its first step,
the bridge, is a day of code. Behaviour cloning from the scripted
brain answers Paul's aiming question better than shaped rewards alone.
