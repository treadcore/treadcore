# What Kills Half the Team

**Check-in report CI-004.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 19 October 2026.
Companion records: journal 0052; `LAB-REPORT-BRAIN.md` sections 20 and
27; CI-003.

---

## Abstract

CI-003 found 46.7% of the trial team's seat-time spent dead. This step
finished and measured the instruments that name what a seat was doing
when it went down, with no change to play. Over 16 seeds and 819
deaths, two situations account for 63% of deaths in 7.7% of the time:
the shell dodge (43%) and the boat's run for the beach, `landfall`
(20%). Afloat, one dodge in six ends in a kill. Because a tank dodges
only when a shell is on it, the count cannot say whether the dodge
helps. The causal test (the dodge off, and `dodge_hold` 1, each at 48
seeds) is running and belongs to the next check-in.

**Index terms:** attrition, reflexes, instrumentation, survival
analysis, experimental design.

---

## I. Introduction

### A. Intent

To finish the work left in the tree for backlog 28, run its look, bring
the brain overview up to date, and commit.

### B. The prompts

> we are working on the backlog and the roadmap and writing lab reports
> and researching and implimenting best ai structured brain. look about
> brain overview in this latest rewrite. finish. commit continue.

> commit. push. continue backlog

"Brain overview" was read as LR 20, "The brain now", and `BRAIN-MAP.md`;
"this latest rewrite" as the uncommitted dodge and death-census work.
"Finish" was read as build, test, measure and document it.

### C. State at the start

Commit `a7a6f16` (CI-003, journal 0051), no tag (the Actions pause).
Six brain files changed in the tree, not built or measured.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "finish": work in the tree, untested | Build and test it first; read the diff whole | Committing it as found |
| D2 | `make brainmap`: two new functions undocumented (backlog 26) | A doc line each | Leaving the count to rise |
| D3 | Backlog 28's proof: a 16-seed census | The null run with `CENSUS_ACTS=1`; play unchanged, so it also checks the instruments read 0 | A trial run: nothing to try until the look names a part |
| D4 | F: the dodge last held at 43% of deaths | A causal test before any change: `-dodge` and `dodge_hold` 1 at 48 seeds | Changing the dodge on the count alone: a reflex that answers fire precedes deaths whether or not it helps |
| D5 | F: 132 deaths a boat sunk by direct fire on landfall | Backlog 30, a weight that turns a boat off a gun's range | Fixing it in this step: one step at a time |
| D6 | "look about brain overview" | LR 20 updated in place: 87 weights, the dodge's weights in the diagram, the 19 October numbers and where it dies | A new overview section: LR 20 is the one the others point to |
| D7 | "commit. push." with the causal runs unfinished | Commit the look as its own step; the runs are the next check-in | Waiting 25 minutes to fold them in |

---

## III. Methods

### A. Census

naval.map, 900 s, respawns 3, the fair census (every team in the trial
seat on each seed), 16 seeds from 0, the ledger off, `CENSUS_ACTS=1`.
The trial and baseline settings are the defaults (a null run).

### B. The parts explored, function by function

| File | Function or field | What it does |
| --- | --- | --- |
| `fleet.rs` | `killed_by` (new, test helper) | from a tick's events, the weapon whose hit names the tank (pillbox, mine, strike, longshot, direct, indirect), whether it sank, and whether the shooter was the lone RuleBot or no one |
| `fleet.rs` | `brain_census`: `window`, `died_last`, `died_window`, `died_by`, `died_both` | each trial seat's last five codes, one a second while alive; at each `TankDestroyed` of a trial seat, the four tables |
| `lib.rs` | `Brain::dead` (new, default nothing), `run_bots` | called each tick a seat is dead, in place of a think |
| `lib.rs` | `Dodge::watch` (new), `Dodge.watch`, `metered`, `tune`, `began` | the dodge under way from its start to its shell's landing; a hit there counted, killed or not, by the dodged shell or not |
| `lib.rs` | `Dodge::order_with` | the horizon, margin and clearance from `tune` (the constants when None) |
| `lib.rs` | `Dodge::holding` | held while driving, and by `dodge_hold` that share of the flight once arrived |
| `commander.rs` | `set_settings`, `impl Brain::dead`, the afloat branch of `think_on` | the weights into the dodge; the dead seat's watch; `Act::Landfall` when afloat with no hop under way |
| `meter.rs` | `Meter.dodges`, `dodge_hits`, `dodge_kills`, `dodge_shell`; `dodge`, `dodge_hit` | the counts, ashore and afloat, in the meter's line |
| `status.rs` | `Act::Landfall`, code `landfall` | the run for the beach; unique among codes (test) |
| `weights.rs` | `dodge_horizon` 4, `dodge_margin` 0.8, `dodge_clear` 2.5, `dodge_hold` 0 | the dodge's numbers, after Paul's rule that an either/or is a scalar |

### C. Reading the tables

A code's share of deaths over its share of time is how much more
likely a seat is to die while holding it. A reflex raises this ratio by
being where danger is, so the ratio is a lead for a causal test, not a
cause.

---

## IV. Results

### A. Deaths (819, 17.1 a match)

| Code held last | Share of deaths | Share of time | Ratio |
| --- | --- | --- | --- |
| dodge | 43.2% | 4.5% | 9.6 |
| landfall | 20.1% | 3.2% | 6.3 |
| hop.sailing | 7.9% | 4.7% | 1.7 |
| resupply | 3.9% | 0.9% | 4.6 |
| hop.holding_launch | 3.4% | 1.1% | 3.1 |

By weapon (791 in the twelve commonest causes): indirect 275, direct
253, strike 246, pillbox 17; sunk 399. The commonest pair: direct fire
on a boat at landfall, 132.

### B. The dodge

Ashore 7,911 dodges: 34.0% hit before the shell landed, 16.6% by the
shell dodged, 7.4% killed. Afloat 3,408: 18.2% hit, 7.6% by the shell,
16.6% killed.

### C. Null and tests

Trial 8.38, baseline 8.38, 0.00 on every seed. 24.8 us a seat-tick.
Brain 69 passed and 8 ignored; client 31; net 14; proto 3; sim 50. No
warnings.

---

## V. Discussion

1. **Two situations, not ten.** The hypothesis named two or three, and
   two make 63% of deaths. The ladder, rotation and the fleet, the
   object of most steps so far, are under 3% of deaths each.
2. **The dodge is where danger is.** Its ratio of 9.6 is partly that a
   tank dodges only when a shell is on it. The meter adds that a sixth
   of ashore dodges are still caught by their own shell, which the
   clearance (`dodge_clear`) and the hold (`dodge_hold`) speak to. The
   switch off says whether the dodge saves more than it loses.
3. **Afloat, a hit is a kill** (567 of 621). Anything that keeps a
   boat out of range pays more than anything that makes it hard to
   hit, which points at the landfall (backlog 30) and the crossings.

**Next:** read `dodge_hold` 1 and `-dodge` against the null (48 seeds
each, running); then backlog 30.

---

## Appendix A. Reproducing this report

```
CENSUS_ACTS=1 CENSUS_SEEDS=16 CENSUS_MAP=$PWD/maps/naval.map CENSUS_RESPAWNS=3 \
  CENSUS_SECS=900 CENSUS_LEDGER=0 \
  cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture
```

The causal pair: `CENSUS_WEIGHTS` a dump of the defaults
(`CENSUS_DUMP`) with `dodge_hold = 1`, and `CENSUS_CAPS=-dodge`, each
over `CENSUS_SEED0` 0, 16 and 32 with `CENSUS_SEEDS=16`.
