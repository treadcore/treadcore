# Each Switch Alone, the Posture Read, and a Review of the Brain

**Check-in report CI-001.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 18 October 2026.
Companion records: journal 0049; `LAB-REPORT-BRAIN.md` section 24;
`BRAIN-MAP.md` (the function index).

---

## Abstract

The improved bot brain of step 12 had sixteen capabilities built,
tested and switched off, none promoted, because no census had measured
any of them alone at the twelve seeds the roadmap asks for. This
session measured all eleven that are off, one at a time, over the fair
census: 12 seeds of 900 s on `naval.map`, every team in the trial seat,
396 matches. It gave the noise a number, the standard error of the
paired difference over seeds. No switch was outside two standard
errors. Three never acted at all: `board`, `island_site` and
`escort`. The team board in particular was drawn once a second and read
by no decision. The session then made the board's posture lean the
Auto ladder through a blending weight, `w_posture`. The first match
read with it exposed a mis-calibrated posture: Defend in 94% of picks,
because a long-tailed threat field summed every hostile tank on the map.
After the fix, twelve seeds read +1.21 ± 1.31 a seat; 48 seeds read
-0.81 ± 0.65. The switch stays off. The report closes with a review of
the brain's architecture, a comparison of ways to tune its 77 weights
by mutation, how to playtest faster, and a proposed form for backlog
items.

**Index terms:** game AI, utility AI, influence maps, paired
comparison, standard error, statistical power, evolution strategies,
SPSA, CMA-ES, deterministic simulation.

---

## I. Introduction

### A. Intent

To finish and test the step-12 brain: measure each new part alone,
fairly enough to tell it from chance, and either promote it or say why
not. To keep a lab report for every change, recording what changed,
why, and for which goal (roadmap goal 4: *bots that think as a team*).

### B. The prompts

The session's first prompt, word for word:

> read backlog and continue working on on the roadmap. we were workign
> on brainz and its planning implimentation, it started as a lab
> report. resume writing, and finishing up. and testing this new brain
> structure, and keep writing lab reports about what you change and why
> for what goal.

Its second, at the first check-in:

> keep going, write the lab report when it's done. your doing awesome.
> i want a very deatiled breakdown of the methods used and IEEE like
> lab report at each time we checkin . we have intentions we write the
> prompt and how our converstaion causes decisions . we want a full
> architectural reivew of the brain system. compairons and ways to test
> and grow these genetic algorithems, or adjusting the scalars via
> mutations. is there a way to create description and try to atampt
> these via playtests faster. does it run at accelerated rate, can we
> write what we have done in detail each function and part of the brain
> explored. and how to improve placing new items intho the backlog.

### C. State at the start

Commit `a352fb6` (journal 0048, LR 23). The brain: 17 files, about
12,600 lines; 76 weights and 17 capabilities; brain tests 64 passed,
8 ignored. The roadmap's "next" line read: *the census at twelve seeds
or more for each capability alone, then `make tune` on the ones that
pay*. Backlog 18 named the switches. Standing rules (ROADMAP goal 4):
every part behind a `Caps` bit and off until its census says on; the
sim, the wire and the replay untouched; every either/or a scalar;
`make test` green. GitHub Actions are paused, so there are no tags.

---

## II. Decision Log

How the conversation and the findings caused each decision. "P1" and
"P2" are the two prompts above; "F" marks a finding of this session.

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | P1 "continue working on the roadmap", "testing this new brain structure"; ROADMAP's "next" line; backlog 18 | Measure every off switch alone at 12 seeds before building anything new | Building the next behaviour first: a part that is not measured cannot be promoted, and 16 were waiting |
| D2 | LR 21 to 23 call results "inside the noise" with no number for the noise | Print the paired difference a seed and its standard error; "outside" means beyond 2 errors | A t-test p-value: the same information, harder to read in a table; Elo over the ledger: it mixes genomes across runs |
| D3 | 11 switches x 36 matches is about 70 minutes one at a time | `scripts/switches.sh` with `JOBS=3`: three censuses side by side on the four cores | Running the matches inside one process on threads: the meter is per thread, and the census code would change |
| D4 | Parallel censuses would interleave the ledger's lines | The switch census runs with `CENSUS_LEDGER=0` | A lock on the ledger file: not needed for a measurement whose table is the record |
| D5 | F: `board` reads exactly 0.00 on every seed, and its matches equal the baseline's number for number | Look for what reads the board: only `team_rotate` and `bridge_jobs` do; influence, front and posture are read by nothing | Calling the board "even": an identical match is not a measurement, it is a part that never acted |
| D6 | F: `island_site` 0.00, identical matches | Trace it: it is only asked when a tank has no base, and in a fleet the leader's depot is always found first | Fixing it now: the right place (the naval leader's first landing) is a design question; backlog 20 |
| D7 | D5, goal 4 ("one picture of the map per team"), and Paul's rule "every either/or is a scalar" | Make the posture lean the Auto ladder by a blend, `(1 - w) order + w lean`, behind `posture`, with `w_posture` | Replacing the ladder with a full utility scorer: much larger, and it would change the baseline itself |
| D8 | F: first match with `posture`, 3.64 against 11.02; the new meter shows Defend in 94% of picks | Find why before measuring further: the threat field's long tail, and the readiness spread | Lowering `w_posture` until it stops hurting: that tunes around a bug |
| D9 | D8 | Defend becomes hostile worth within the base's alarm, over 0.5; the spread is left to `team_rotate` | Keeping the field with a higher threshold: the field's value at the base grows with the number of tanks on the map, so no single threshold works |
| D10 | Regression tests must fail where the bug was (LR 23's practice) | Run the new posture test against the old rule: it fails; restore | Trusting a test that has only ever passed |
| D11 | F: at `w_posture` 0.25 nothing changed; 0.5 and 1 gave identical matches | Measure 0.35, 0.5 and 1 at 12 seeds; set the default to 0.5 | Sweeping finely: the formula shows that only the rung gaps matter, so 0.2, 0.33 and 0.5 are the only distinct steps |
| D12 | F: 0.5 read +1.21 ± 1.31 over 12 seeds: promising, not proven | 36 more seeds (`CENSUS_SEED0`, three ranges side by side); over 48, -0.81 ± 0.65 | Promoting on 12 seeds: inside the noise by the rule of D2 |
| D13 | P2 "IEEE like lab report at each time we checkin" | This series, `documentation/checkins/`, and a standing memory of the rule | Folding it into the journal: the journal is the session; this ties the prompts to the decisions |
| D14 | P2 "each function and part of the brain explored" | `scripts/brainmap.py` writes `BRAIN-MAP.md` from the code's own doc comments; section V reviews the structure | Writing the index by hand: it would drift from the code the next day |
| D15 | P2 "genetic algorithms ... mutations" | Section VI measures what the tuner can and cannot see with the noise as now measured, and compares methods | Running `make tune` now: by VI.B its two seeds cannot see a change of under about 4 a seat |
| D16 | P2 "playtests faster", "accelerated rate" | Section VII: the measured speed; a proposal for scenario files | Building the scenario runner now: one step at a time; backlog 24 |
| D17 | P2 "placing new items into the backlog" | Section VIII: a form for every new item, used for items 20 to 26 | Rewriting items 1 to 19: their numbers are cited by the journal and the changelog |

---

## III. Methods

### A. Apparatus

| Item | Value |
| --- | --- |
| Machine | Alpine Linux guest (kernel 6.18.52-lts) under UTM/QEMU on a Mac; 4 virtual cores, 8 GB |
| Toolchain | rustc 1.98.1, cargo 1.98.1; `--release` for every census |
| Simulation | `treadcore-sim`, deterministic, 20 ticks a second; the world hash and replay tests unchanged by this session |
| Map | `maps/naval.map`: 160 x 160, every start at sea, five islands and eight islets |
| Seats | 16: three teams of five (leader, wingmen, builder, guard, ranger as `fleet.rs` deals them) and one RuleBot alone |
| Match | 900 s (18,000 ticks; 288,000 seat-ticks), 3 respawns a seat, no time limit inside the world |

### B. The fair census (`brain_census`, `fleet.rs`)

For seed index n, the world seed is `0x7EADC0DE ^ (n * 0x9E3779B9)`.
Each seed is played three times, once with each team on the trial
settings and the other two on the baseline (`CENSUS_FAIR`, LR 14), so a
start that favours one seat favours trial and baseline equally. A
seat's score (LR 8, `ledger::score`, `long_game` 0.5) is:

```
score = 10 won + kills - deaths + 2 held_s / seconds + 0.1 built + 0.05 (road + bridge tiles)
```

A team's score is its seats' mean. A seed's reading is the trial
team's mean over its three matches less the baseline teams' mean over
their six team-matches.

### C. The statistic (new: `ledger::mean_se`)

The seeds are the samples. A seed's three matches share its starts, so
the seed gives one paired difference d_s. Over S seeds:

```latex
\bar d = \frac{1}{S}\sum_s d_s, \qquad
SE = \frac{1}{\sqrt S}\sqrt{\frac{1}{S-1}\sum_s (d_s - \bar d)^2}
```

A difference is *outside the noise* when |d̄| > 2 SE, about 95% on a
normal approximation. A null run (trial = baseline) gives d_s = 0
exactly on every seed: the matches are deterministic, so the three
matches of a seed are one match. The spread then comes only from
what the trial changes. A change to a rule that acts in fights
perturbs the match chaotically, and that perturbation is the noise.

**Power.** With a seed-to-seed spread σ = SE √S, putting a true
difference δ outside 2 SE takes S ≈ (2σ / δ)² seeds. This is the
number every backlog item should carry (section VIII).

### D. Each switch alone (`scripts/switches.sh`, `make switches`)

For each capability off by default: `CENSUS_CAPS=<switch>`, and
`CENSUS_BASE_CAPS=board` for those that need the board (`team_rotate`,
`bridge_jobs`, `posture`), so the baseline differs from the trial by
that one bit. Run three at a time; scores do not depend on load. The
microseconds a seat-tick do, so they are not compared across this run.
The table is written to `build/switches/STAMP/table.txt`.

To tell "acted and did not help" from "never acted", the census also
reports, for the trial team, the seat-seconds spent in each new part's
status codes (`status.rs`), and the meter now counts the posture picks.

### E. The posture lean (`Rung::pick`, `Rung::leaned`)

Rungs in order: DefendBase, Fortify, Turrets, Hunt, Expand, Guard. For
the rungs that are switched on and hold:

```latex
\text{score}(r) = (1-w)\left(1 - \tfrac{i_r}{5}\right) + w \cdot \text{lean}(r, \text{posture})
```

A leaned rung at index i beats an unleaned one at j < i when
w / (1 - w) > (i - j) / 5, so it climbs fewer than 5w / (1 - w) rungs.
At w = 0.2 that is none; at 0.25, one; at 0.5, all five. Ties go to
the higher rung, and a holding DefendBase is never passed. With no
posture, or w = 0, the result is the old first-match pick. The test
checks this for all 4,096 combinations of six on-bits and six
hold-bits.

### F. Proving a fix

Following LR 23: a regression test is run on the old code and must fail
there. For the posture's Defend rule the old line was put back, the
test run (it panicked on its first assertion), and the new line
restored, all in the working tree.

### G. Splitting a census (`CENSUS_SEED0`)

The census takes a first seed index, so one long census runs as
several processes over disjoint seed ranges, each on its own core.
Their per-seed lists are concatenated, and the mean and error are
computed over all seeds together.

---

## IV. Results

### A. Each switch alone (12 seeds; full table in LR 24)

| Switch | Difference a seat | SE | Seed spread σ | Seeds for δ = 0.5 | Acted |
| --- | --- | --- | --- | --- | --- |
| `smooth` | -1.98 | 1.25 | 4.3 | 300 | always |
| `team_rotate` | -0.83 | 0.87 | 3.0 | 144 | 879 seat-s relieving |
| `forget` | -0.36 | 0.23 | 0.8 | 10 | uncounted |
| `column` | -0.06 | 0.41 | 1.4 | 32 | on bridges |
| `board` | 0.00 | 0.00 | 0 | | never |
| `island_site` | 0.00 | 0.00 | 0 | | never |
| `escort` | +0.01 | 0.01 | 0.03 | | one seed |
| `priced_crossing` | +0.11 | 0.56 | 1.9 | 60 | road posts 92 against 30 |
| `cycle` | +0.31 | 0.43 | 1.5 | 36 | 243 seat-s |
| `duck` | +0.39 | 1.04 | 3.6 | 207 | 911 seat-s |
| `bridge_jobs` | +0.44 | 0.91 | 3.2 | 159 | 4,438 seat-s |

None is outside 2 SE. The throughput was 396 matches in 22 minutes,
three at a time: about 1,080 matches an hour.

### B. The posture

| Run | Seeds | Postures read (econ / fort / conn / exp / def) | Picks changed | d̄ | SE |
| --- | --- | --- | --- | --- | --- |
| before the fix, w 0.25 | 1 | 3% / 0 / 3% / 0 / 94% | 8,720 | -7.39 | |
| fixed, w 0.25 | 1 | 72% / 0 / 26% / 0 / 2% | 0 | +0.08 | |
| fixed, w 0.35 | 12 | 9% / 48% / 17% / 26% / 0.5% | 5,122 | +0.39 | 0.36 |
| fixed, w 0.5 | 12 | 8% / 53% / 16% / 22% / 0.5% | 50,677 | +1.21 | 1.31 |
| fixed, w 0.5 | 48 | 16% / 47% / 19% / 18% / 0.4% | 164,606 | -0.81 | 0.65 |

The four 12-seed blocks at w 0.5 read +1.21, -1.30, -2.16 and -1.01.

### C. Tests

`make test`, every crate: brain 67 passed and 8 ignored (64 before;
new: `the_posture_leans_the_ladder`,
`the_team_defends_a_threat_at_the_base_not_far_off`,
`the_standard_error_of_a_sample`); client 31; net 14; proto 3; sim 50,
its replay and world-hash tests among them. No warnings.

---

## V. Architectural Review of the Brain

What follows is the structure as built on 18 October, read from the
code. `BRAIN-MAP.md` lists every function in it: 17 files, 337
functions (120 without a doc comment), 75 tests.

### A. Layers and data flow

```
World (sim, 20 Hz) --observe(p)--> Observation (a full copy, per seat, per tick)
        ^                                   |
        |  Vec<Order>                       v
   run_bots (lib.rs) ---- for each seat ---> Brain::think_on(obs, map)
                                              |
            +---------------------------------+----------------------------+
            |                                 |                            |
        RuleBot (lib.rs)               Commander (commander.rs)      FleetCommand (crew.rs)
        doctrine at 4 Hz:              20 Hz; wraps a RuleBot        a team's leader or builder:
        guns, drones, target,          ("inner"), whose doctrine     musters every 30 s, hands
        landfall, dodge                runs every tick, then a       out the castle plan as jobs
          |                            first-match chain of about
          +-- Tactician (tactics.rs)   30 branches decides which
          +-- Hop (hop.rs)             commands survive
            |
   shared and per-team state: Settings (weights.rs) via with_settings;
   TeamBoard (board.rs) via board_for(leader); MapFacts (facts.rs) per tank;
   Plan (fortify.rs) per base; Cycle (cycle.rs); Forget (memory.rs); meter, ledger
```

### B. The modules, by role

| Role | Files | What they hold | Read by |
| --- | --- | --- | --- |
| Interface | `lib.rs` (`Brain`, `run_bots`, `retire_taken`) | the controller contract: `think_on`, `status`, `act`, `settings` | the window, the server, the census |
| Doctrine | `lib.rs` (`RuleBot`), `tactics.rs` | fighting: target score (14 weights), odds and massing, stance and kiting, drones, dodge | every Commander, through `inner` |
| Command | `commander.rs` | goals (Auto and the six fixed), the Auto ladder, survival reflexes, fleet slots, revives, pings, the base | itself; the coordination screen |
| Team | `crew.rs`, `fleet.rs`, `board.rs`, `rotation.rs` | the fleet commander and jobs; dealing seats into teams; the team board; push and defence points | Commanders of a team |
| Building | `fortify.rs`, `cycle.rs` | the castle plan (gate, walls, turrets, capped slots, road posts); the base cycle after a fill | the Fortify and Turrets rungs; the cycle branch |
| Movement over water | `hop.rs`, `facts.rs` | islands, the crossing cache, bridges and boats, priced crossings, site scores | the doctrine's landfall and hop |
| Commitment | `intent.rs`, `memory.rs` | the desire vector and confirmation (`smooth`); lists that forget | `go()`, the ladder, given-up lists |
| Settings and evidence | `weights.rs`, `status.rs`, `meter.rs`, `ledger.rs` | 77 weights, 18 bits; status codes; counters; the ledger and its score | everything; the census and the tuner |

### C. Findings

Numbered A1 onward so later reports can cite them; L1 to L8 are LR 2's.

- **A1. The top of the Commander is still a first-match chain (L1).**
  `think_on` runs about 30 guarded branches in a fixed order, each
  ending in `return cmds` (afloat, hop, revive, dodge, duck, bridge
  help, fleet afloat, chain hauling, pings, the fort, leaderless,
  unseen fire, board orders, the cycle, then the goal). The weights
  tune inside the branches; nothing tunes their order. The ladder's
  first-match pick now has a blend (`Rung::pick`); the chain does not.
- **A2. The team board is mostly unread (L5, half answered).** It
  draws influence, threat, the front, territory, defence points and a
  posture once a second. Only `team_rotate` reads the points and
  `bridge_jobs` its bridge list; `posture` (this session) now reads the
  posture. Influence, the front and territory are read by nothing.
  Every new board field needs a reader and a census, or it is
  bookkeeping.
- **A3. The posture itself is a first-match ladder** (Defend > Economy >
  Expand > Connect > Fortify on thresholds), where LR 9 designed scores
  ("when it scores high"). By Paul's rule it should be five scores and
  a blend. Backlog 21.
- **A4. A full Observation copied per seat per tick (L2, open).**
  `world.observe(p)` builds each seat's copy, 16 times a tick. Sharing
  one per team is 12b's last item, and the only one that touches the
  sim.
- **A5. Global state by thread.** `with_settings`, `new_match`,
  `board_for` and the meter are thread-local or process-global. Replays
  are the same because bots run in seat order, and the board is rebuilt
  by the first member due. It holds, and it is why parallel runs must
  be processes, not threads, in one census. A census in threads would
  need these passed in.
- **A6. The doctrine runs every tick inside the Commander,** and the
  Commander throws away what it does not keep (`keep_inner`). That is
  simple and correct, but the doctrine's work is spent even when a
  branch above it decides. At 23 to 33 us a seat-tick it is affordable:
  sixteen seats cost about 0.5 ms a tick, and at 4x (80 ticks a
  second) about 4% of one core.
- **A7. Parts sit where they cannot act.** `island_site` is consulted
  only when a tank has no base, which in a fleet never happens. `escort`
  needs a far fetch, which a 900 s match has once. A part's trigger
  should be checked against the census map before its census: the
  status-code seat-seconds are that check, and now exist for every
  part.
- **A8. One map.** Every census since LR 11 is `naval.map`. A part
  promoted there may only be good there (the islands make bridges and
  boats matter more than on a generated map). The census needs a map
  list, and a promotion should hold on at least two maps.
- **A9. Documentation coverage.** 120 of 337 functions have no doc
  comment; `crew.rs` has 10 of 16 bare. `BRAIN-MAP.md` shows each.
- **A10. Tests by module.** `crew.rs` and `hop.rs` have no tests in
  their own files. They are proven through the Commander's tests and
  the census, which shows them in combination but cannot isolate them.

### D. What the structure does well

Every number is a named weight with a doc line, a slider and a place in
the ledger. Every new part is behind a bit, so the baseline is always
one setting away. The sim is deterministic, so a match is a function of
its seed and settings. That makes the paired census possible at all,
and a null run reads exactly zero. Logic reads codes, not sentences
(LR 22), and no comparison can panic on a NaN (LR 23).

---

## VI. Tuning the Weights: Mutation, Selection, and Noise

### A. The tuner as it is (`scripts/tune.sh`, `make tune`)

A (1 + λ) evolution strategy. The parent is the default weights. Each
generation makes λ = 4 children. Each child multiplies 15 named weights
(target selection, odds, exposure, `k_long`, `c_boat`, `swap_gap`) by
exp(σ N(0, 1)), with σ = 0.15. Each child is evaluated by the fair
census on 2 seeds (6 matches); its fitness is d̄, the trial team less
the baseline. A child replaces the parent if its fitness is higher.
Capabilities are fixed per run (`CAPS`).

### B. What it can see

Its fitness is d̄ over S = 2 seeds, so its error is σ_seed / √2, which
is 0.6 to 3 a seat by section IV.A. Three effects follow:

1. **The parent's fitness is exactly 0.** The default against itself
   is a null run. Any child whose noise draw is positive replaces it,
   which is about half of them, whatever its weights are worth.
2. **The winner's curse.** The best of λ = 4 noisy readings exceeds
   its true value by about 1.03 standard errors on average, 2 to 3 a
   seat for weights that act in fights. The new parent's fitness is
   that inflated figure, and later children are compared against it,
   so the run stalls on a lucky parent rather than climbing.
3. **The step cannot be seen.** σ = 0.15 moves a weight by about 15%.
   The effects measured so far, whole capabilities on or off, are under
   2 a seat. A 15% change in a weight will be smaller. The tuner,
   as set, is selecting noise.

### C. Ways to do better, compared

Costs are in matches, at the measured 1,080 an hour on three cores.

| Method | What changes | Cost a step | Strength | Weakness |
| --- | --- | --- | --- | --- |
| (1 + λ)-ES as now | | 6 λ = 24 | simple; already built | selects noise (VI.B) |
| (1 + λ)-ES, noise-aware | the parent is re-evaluated each generation on fresh seeds; a child must beat it by 2 SE; S chosen by the power formula | 3 S (λ + 1) | honest acceptance; uses what is built | slow: S = 40 is 600 matches a generation |
| Racing / successive halving | every child starts with a few seeds; the worse half is dropped, the rest get more seeds | about 3 S λ / 2 | spends seeds on the contenders only | needs the census to resume a child (`CENSUS_SEED0` makes it possible) |
| SPSA (Spall 1992) | all weights perturbed at once by +/- c Δ (Δ random signs); the gradient read from f(θ + cΔ) - f(θ - cΔ); a step along it | 3 S, whatever the number of weights | **the paired census measures f(+) - f(-) directly**: the + team against the - teams in the same matches | one gradient a step; needs a gain schedule |
| CMA-ES (Hansen) | a population whose covariance adapts to the landscape | 3 S x population | learns which weights move together | noise needs large populations or re-evaluation |
| Self-adaptive σ, the 1/5 rule | σ grows when over a fifth of children succeed and shrinks otherwise | none extra | steps sized to what the census can see | meaningless while success is noise |
| Bit-flip mutation of `Caps` | a capability is a gene too, flipped with a small probability | none extra | searches parts and weights together | the space doubles per bit; needs the larger S anyway |
| MAP-Elites (Mouret and Clune) | an archive of the best genome per behaviour cell (e.g. bridge tiles x kills) | per child, as the ES | keeps diverse play styles (roles) | needs behaviour descriptors from the ledger (they are there) |
| Hall of fame and co-evolution | children meet past champions, not only the default | as the ES | avoids tuning against one opponent | Elo over noisy matches needs many games |

**Recommendation.** First make the acceptance honest: noise-aware
(1 + λ) with re-evaluation and the 2-SE rule. That is a small change to
`tune.sh`. Then SPSA, because the fair census already measures a paired
difference. One step of SPSA is one census with the trial team on
θ + cΔ and the baseline teams on θ - cΔ, and its cost does not grow
with the number of weights. Racing on top of either spends seeds where
they matter. Every promotion is confirmed on seeds the search did not
use and on a second map (A8). Backlog 23.

### D. How to see a tuner work before spending a night on it

A *planted* test. Detune one weight on purpose (say `w_wounded` to 0)
and start the tuner from there; it must find its way back toward the
default. If it cannot recover a known large effect, its findings on
unknown small ones mean nothing.

---

## VII. Faster Playtests

### A. Does it run at an accelerated rate?

Yes. Headless, the world and the brains run as fast as the CPU allows.
One 900 s match with sixteen bots took 10 to 11 s on one core in this
session, about 85 times real time. Three censuses side by side ran
1,080 matches an hour, about 270 match-hours an hour. The window plays
at 1x, 2x or 4x for a person watching. Small scenario tests run faster
still: the brain's 67 tests, many of them several minutes of game
time, finish in about 60 s together.

### B. Describe it, then try it

Today a playtest of a behaviour is a Rust test written by hand: place
tanks, set a goal, run N seconds, assert. The tests of LR 16 to 24 are
all of this form (`a_teammate_finishes_a_part_laid_bridge`,
`a_far_fetch_gathers_the_fleet`). A small text format would let Paul
describe one and have it run headless at 85x:

```toml
# scenarios/bridge-handoff.toml
map = "generated:0x7EADC0DE"       # or a .map file
seconds = 240
caps = ["board", "bridge_jobs"]
weights = { bridge_reach = 40 }

[[tank]]  seat = 0  at = [40, 64]  goal = "auto"  material = "full"
[[tank]]  seat = 1  at = [44, 64]  team = 0  goal = "guard"

[[terrain]]  water = { x = [46, 49], y = "all" }
[[board]]    bridge = { by = 1, tiles = [[47, 64], [48, 64], [49, 64]] }

[expect]
by = 240
bridge_laid = [[47, 64], [48, 64], [49, 64]]
status_seen = ["team_bridge"]          # a status code, status.rs
```

A runner (`make scenario S=...`) would build the world, run it, check
`[expect]`, and print the status codes over time. A census-sized
version would run a scenario over many seeds and give its d̄ and SE.
Backlog 24.

### C. Shorter matches for early behaviours

Parts that act in the first minutes (the duck, the column) can be
measured on 300 s matches, three times as many seeds an hour. Parts
that pay late (escort, cycle, bridge jobs) need longer ones (goal 5).
The census's `CENSUS_SECS` already allows both, so each backlog item
should say which it needs.

---

## VIII. Placing New Items into the Backlog

### A. What went wrong with the form so far

Items 1 to 19 are sentences. Several say what to do and not how they
would be known done. None says what a measurement would cost: backlog
18 asked for "twelve seeds or more", and this session found that
twelve can only see a difference of 1 to 4 a seat. Items that are done
stay in the list with "Done" in front, so the open ones are hard to
find.

### B. The form proposed

Every new item carries six fields:

```
N. Title. (date; cause: a prompt quoted, or LR/journal/CI finding)
   Goal g, step s. Hypothesis: what should change, and in which number.
   Proof: the test that fails before and passes after, and/or the
   census (switch, map, seconds, seeds from (2 sigma / delta)^2).
   Cost: matches or hours. State: idea | designed | built (off) |
   measured | promoted | done | dropped.
```

Items keep their numbers forever, since the journal and the changelog
cite them. When an item is done, its state changes and a line in the
roadmap's goal says where it went; the open list stays short because
the roadmap's backlog section shows the open items first and done ones
after. Items 20 to 26 below are written this way.

---

## IX. Discussion and Conclusions

1. The census can now say how sure it is. At twelve seeds it can
   resolve about a point a seat for parts that act in many fights, and
   a tenth for parts that act rarely.
2. A part that never acted cannot be judged by its score. Identical
   matches, and the status codes' seat-seconds, are the check. Three
   parts failed it: `board`, `island_site` and `escort`.
3. Reading the board found the board's own bug at once. Each field
   drawn and never read is an untested field.
4. A reading of +1.2 ± 1.3 is not a result. The 48-seed rerun turned
   it to -0.8 ± 0.7. Confirmation on fresh seeds is part of the method,
   not an extra.
5. The tuner as set selects noise. Making it honest is the first tuning
   step, before any run long enough to promote anything.

**Next** (one at a time): backlog 21, the posture as scores; then 23,
the honest tuner; then 20, `island_site` where a naval start chooses.

---

## Appendix A. Reproducing this report

```
make switches                                    # IV.A: 11 switches, 12 seeds, 900 s, naval.map (JOBS=3 for three at a time)
CENSUS_DUMP=w.toml CENSUS_SEEDS=0 CENSUS_CAPS=board,posture \
  cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture
sed -i 's/^w_posture = .*/w_posture = 0.5/' w.toml
for k in 0 12 24 36; do CENSUS_SEED0=$k CENSUS_SEEDS=12 CENSUS_WEIGHTS=w.toml \
  CENSUS_BASE_CAPS=board CENSUS_MAP=$PWD/maps/naval.map CENSUS_RESPAWNS=3 \
  CENSUS_LEDGER=0 cargo test -q -p treadcore-brain --release -- --ignored brain_census --nocapture \
  > posture-$k.log 2>&1 & done; wait          # IV.B, 48 seeds
make brainmap                                    # BRAIN-MAP.md
```

## Appendix B. References

- Spall, J. C., "Multivariate stochastic approximation using a
  simultaneous perturbation gradient approximation," *IEEE Trans.
  Automatic Control*, 37(3), 1992. (From memory; not opened this
  session.)
- Hansen, N., "The CMA Evolution Strategy: A Tutorial," arXiv:1604.00772,
  2016. (From memory.)
- Mouret, J.-B., and Clune, J., "Illuminating search spaces by mapping
  elites," arXiv:1504.04909, 2015. (From memory.)
- Beyer, H.-G., and Schwefel, H.-P., "Evolution strategies: a
  comprehensive introduction," *Natural Computing*, 1, 2002, for the
  1/5 success rule. (From memory.)
- `LAB-REPORT-BRAIN.md` sections 2, 8, 9, 14, 20 to 24; journal 0038,
  0048, 0049.
