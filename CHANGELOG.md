# Changelog

What changed, by date. For what happened, in order, and why, see the
creation journal in `journal/`: one entry per working session, in a
fixed form (`journal/TEMPLATE.txt`).

## Unreleased

- Changed: the brain's defaults are grove/w0034's, promoted by orrery's
  auto mode after the brain's tests and every family held: `w_near` 2,
  `w_in_range` 1.4, `kite_in` 4.5, `w_pos_fortify` 6, `dodge_horizon`
  1.083, `castle_inside` 8, `longshot_every` 2.5. Auto mode also runs
  one of each kind of run in turn (LAB-REPORT-BRAIN.md section 119).

- Changed: the fleet's cycle closes itself (orrery's auto mode, on by
  default). A spsa or tune run's result is adopted with its two held-out
  reads; the strongest current candidate whose families hold is
  promoted, tested and pushed to main by orrery; each machine's build is
  checked for the promoted values. Candidates and other news are
  progress, not flags (LAB-REPORT-BRAIN.md section 118).

- Added: `focus_blend` (and the switch `focus_blend`), the target between
  the near tank in range (0) and the wounded one a teammate is on (1);
  0.5 plays as before, bit for bit. Fixed: a census in orrery's queue
  plays the binary its machine built for that commit (`CENSUS_BIN`),
  never one rebuilt in a shared target folder that could be another
  commit's (LAB-REPORT-BRAIN.md section 117).

- Changed: training is no longer started from make. The 13 training and
  measurement targets are gone from the Makefile; each is a run of
  orrery's fleet queue, a button in its Training view or
  `fleet-queue.py Q new KIND`. `scripts/parts.sh`, `switches.sh` and
  `roundrobin.sh` print their censuses with `LIST=1`;
  `scripts/set-defaults.py` writes a candidate into `weights.rs` and
  `Tactics::default()`. A promotion is prepared by orrery on a branch,
  tested and its families read, and reaches main only when Paul approves
  (LAB-REPORT-BRAIN.md sections 114 to 116).

- Changed: orrery holds all of the fleet's orchestration. The plan for
  treadcore, the Training view's document, the lanes and the counting
  moved to orrery's `tools/plans/` (treadcore's `scripts/fleet-view.py`,
  `fleet-jobs.py`, `fleet-count.py` and `fleet-plan.py` are gone). The
  fleet follows `origin/main`: each machine fetches, fast-forwards its own
  checkout and builds each commit. A sync gathers every machine's census
  ledger into `build/ledger/fleet.tsv` on its own clock. Shutdown Now:
  each machine finishes its job, reports and powers off, and a report
  says who finished what (LAB-REPORT-BRAIN.md section 113).

- Changed: families on the fleet. The queue runs `make families`'s test
  for every candidate (8 a job, side by side) and records each family's
  share in `plan/families.json`; all 42 candidates hold every floor.
  orrery's queue no longer fails jobs on a host that is down: its lane
  waits, and the job goes back on the queue (LAB-REPORT-BRAIN.md
  section 112; the first of six steps that move training from make to
  orrery).

- Measured: the fleet queue at full settings (55 seeds of 15 minutes a
  trial, 400 held out). In its first 4 hours 10 minutes, on four
  machines, it played 37,905 games, 9,476 hours of play: 1.46 billion
  decisions and 10.4 billion seat-ticks, and trained the net through
  416 million samples. No weight trial has yet passed both held-out
  reads; `castle_inside` 8 leads. bc2 stays the best net. New:
  `scripts/fleet-count.py` (LAB-REPORT-BRAIN.md section 111).

- Added: one queue for the whole fleet (orrery's `tools/fleet-queue.py`,
  treadcore's `scripts/fleet-plan.py`). Trials of the scripted brain's
  110 weights and rounds of the learned net take turns on every spare
  machine. A keeper keeps it running, and two controls stop it: stop
  adding, and collect results and stop now. orrery's Training view shows
  it and, for an operator, has the buttons. A sample run did 70 trials
  and a net round in five minutes. Also `LAB-REPORT-NET-PICTURES.md`
  (LAB-REPORT-BRAIN.md sections 109 and 110).

- Measured: twenty times the recording (1,200 matches, 50 million
  samples), made on every spare CPU of the fleet in 14 minutes and
  trained on calber's GPU in 4 minutes. The new clone, `bc2`, agrees
  with the scripted brain more often (drive 66.6% against 65.5%) and
  plays 3.42 +/- 0.41 a seat better than `bc1` in the census, level with
  `nets/map7.bin`, which stays the official net. The clones survive but
  go nowhere; the senses, not the data, set their ceiling. The card is
  fastest at batch 32,768 (LAB-REPORT-BRAIN.md section 108).

- Added: `scripts/fleet-record.sh`, recordings on every spare CPU of the
  fleet, claimed by `mv`, streamed to one store, checked by sha256, a
  manifest; `tools/bc` streams compact samples through the GPU;
  `tools/bc/sweep.sh`; `GAUNTLET_NETS=`; `LAB-REPORT-FLEET.md`, the
  pipes, the ssh and the time, and how a net becomes official.

- Changed: `tools/bc` trains five to eight times faster on the GPU: the
  recording loaded onto the card once and batches gathered there, every
  head's loss in one masked log-softmax, the loss read once an epoch. A
  settled epoch takes 0.815 s at batch 4,096 (was 4.3 s) and 0.643 s at
  32,768, against 4.5 and 5.0 s on the CPU; the held-out agreement is
  unchanged (LAB-REPORT-BRAIN.md section 107).

- Measured: behaviour cloning of the scripted brain into the learned
  tank's net (`nets/bc1.bin`), trained on calber's GTX 1080 Ti with
  Burn. It agrees with the scripted brain on held-out matches (drive
  65.5% against 36.5% for always the commonest label), and Rust's
  `Net::forward` gives the trainer's figures exactly; in the census it
  plays worse than `nets/map7.bin`, -2.85 +/- 0.44 paired over 110
  seeds. For this net the GPU is not faster than the CPU (6.4 ms a step
  against 5.6 ms); what would make it so, listed
  (LAB-REPORT-BRAIN.md section 106, LEARNING-DESIGN.md).

- Added: `RECORD=` (the scripted brain's choices through the learned
  tank's senses and masks, `TCBC2`), `AGREE=` (a net's held-out
  agreement in Rust), and `tools/bc`, the Burn trainer, its own Cargo
  project, on CUDA or the CPU (`BC_DEVICE=cuda|flex`), timed by part
  (LAB-REPORT-BRAIN.md section 106). `documentation/LEARNING-DESIGN.md`,
  the learning process.

- Measured: a second census machine, columbia, plays the census
  machine's job identically with the run's frozen binary
  (LAB-REPORT-BRAIN.md section 105).

- Documented: how the two loops' reports are read and what a read may
  adjust; the school's first eight rounds (maps 9 and 10 learned; open
  maps and the gauntlet slip, two guards proposed); the run's second
  candidate (dodge_horizon 1.08, w_in_range 1.4); a fleet of census
  machines, designed (LAB-REPORT-BRAIN.md section 104, CENSUS.md 5a).

- Added: `make school`, the learned tank's curriculum run until stopped
  beside `make converge`; training maps 9 (low armour, full shells) and
  10 (an unguarded depot) against the gauntlet's shortcuts; CENSUS.md
  section 5a, more than one census machine (LAB-REPORT-BRAIN.md
  section 103).

- Added: the gauntlet (`scenarios/gauntlet/`, `make gauntlet`), nine
  single-decision tests for every milestone net and the scripted
  brain; clips (`clip.rs`, `make clips`), a GIF of each generation at
  work, drawn in Rust; reactions, `[[drone]]`, `fires_within` and
  `launched` in scenario files; the match reading counts damage and
  kills. The report: `reports/2026-10-27-generations.html`
  (LAB-REPORT-BRAIN.md section 102).

- Added: survival maps (LAB-REPORT-BRAIN.md section 100): 7a (low, go
  home), 7 (low and far from home), 8 (outnumbered); scenario tanks
  that start hurt and short. The learned tank's shell contact and air
  contact, and its drone head (interceptors, recon, strikes,
  longshots). `make train CHECK=net MATCH=n`: a net in one seat of n
  dealt matches. `nets/map7.bin` dies half as often in a real match's
  first minutes, and a team of it reads level with a random net in
  the census, +3.96 over the last net, with four times the kills.

- Added: training maps from scenario files (LAB-REPORT-BRAIN.md
  section 99). A scenario's `[reward]` and `kind = "net"` brains;
  `make train GROUND=phase1,phase3,scenarios/training/...` trains on a
  mix, rehearsing what was learned; `BASELINE=1` reads the scripted
  brain on the same seeds. Maps 4 to 6 in `scenarios/training/`.
  `nets/map4.bin` takes a depot behind a hostile pillbox on 93.7% of
  held-out seeds. The learned tank's gun also takes a hostile
  pillbox.

- Added: `documentation/PROCESS.md`, how treadcore is made: the pair,
  from a prompt to a commit, where every conversation is kept, the
  brain designed by families and the census. The shared pages are now
  reports of the site: the weight ledger and the big-lizard map
  (LAB-REPORT-BRAIN.md section 98). The AI Brainz plan's section 9:
  guided training, one scenario file a behaviour read by the families
  and the trainer, and a series of ten training maps.

## 0.2.0 (2026-10-26): the first release: the brain on weights, and the learned tank

The next minor version after six candidates of 0.1.0, and the first
not marked a candidate. Wire version 16: it does not play with
0.1.0-rc.6 (wire 12). Since rc.6: rotation, the yard, boats, bridges
and island hops, down and revived, respawns, the Commander's ladder on
a team board with 110 census-tuned weights, a census 2.6 to 3 times
faster, `make converge`, and the learned tank. The release page:
`reports/2026-10-26-the-learned-tank.html`.

- Changed: the dodge starts out from under a shell 2 s before it lands
  (`dodge_horizon` 2, was 4). The run found it on its own and read it
  held out ten times on 400 seeds each, +1.09 to +2.66, each past two
  errors; `make families` holds (LAB-REPORT-BRAIN.md section 97).

- Added: the learned tank drives and shoots (LAB-REPORT-BRAIN.md
  section 97, `documentation/AI-BRAINZ-PLAN.md`). The senses gain
  instruments (place, heading, the nearest edge, a hit just taken), a
  periscope (the exact bearing and strength of the nearest enemy, with
  its closing speed and aspect; of the nearest thing to take; of our
  nearest depot; of a friend's call) and a Call channel: 133 senses.
  `calls.rs`: a team's shared queue of calls to friends (help, attack,
  take), each also a ping that scripted teammates answer. `learn.rs`
  and `make train`: training grounds and evolution strategies on every
  core. `nets/phase1.bin` drives to an enemy, a depot or a pillbox
  (999 of 999 held-out grounds); `nets/phase3.bin` also kills a still
  enemy with the direct gun (999 of 999). In the census a team of it
  reads -23.52 +/- 0.38, below a random net (-20.35 +/- 0.25): it
  charges what a match defends. Next: teach it to live.

- Changed: PROTOCOL.md describes wire version 16.

- Added: the learned tank's phase 0 (LAB-REPORT-BRAIN.md section 96):
  `senses.rs`, what a tank feels as 101 numbers (a compass of eight
  sectors by ten channels of signal strength, the body, the brain's
  hints); `net.rs`, a small net in plain Rust and `NetBrain`, which
  drives, aims and fires by it; `CENSUS_NET` seats a team of nets in
  the census.

- Changed: the dodge's forecast of a drive ashore follows the tank's
  own route (`dodge_own_route` 1, the default now), with no search;
  afloat it searches. The census is faster again, the play the same
  within the noise on every map (LAB-REPORT-BRAIN.md sections 93, 94).

- Added: the weight `dodge_own_route`, off by default: at 1 the dodge's
  forecast of a drive follows the tank's own route instead of a fresh
  search. Faster, but it read past two errors below on big-lizard, so
  it stays off (LAB-REPORT-BRAIN.md section 93).

- Added: `make converge`, the census as a learner until stopped: each
  generation fits every weight's curve to every census so far and
  plays the trials it suggests, reading any past two errors twice more
  on fresh seeds; `make ledger`, every census in one table; each
  census's `run.env` records its commit and host (LAB-REPORT-BRAIN.md
  section 92).

- Changed: the census runs 2.6 times faster with every match the same:
  each observation gathers its side's eyes, data centres, pads and
  banks once; forest and water are read outward in rings that stop
  early; the route search's heap compares integers
  (LAB-REPORT-BRAIN.md section 91).

- Changed: `make spsa` keeps each tuned weight within `BOUND` of where
  it started, half to double by default (`BOUND=0` the old unbounded
  walk; LAB-REPORT-BRAIN.md section 89).

- Changed: the brain spends what it carries (the cowboy, LR 86): a
  ready gun with no tank to shoot shells hostile pillboxes in reach, a
  longshot every 5 s at structures, else pillboxes, else the target
  tank, and three strike drones out at once. +11.64 +/- 0.43 a seat
  against the old defaults over 400 fresh seeds, every map above zero
  (+5.52 without the rewards). New weights `shell_boxes`,
  `longshot_every`, `longshot_any`, `strikes_max`; the old spending is
  `profiles/steady.weights`. A pillbox shot waits while a hostile
  shell comes down near the tank. The meter counts shots and launches
  (`spent`).

- Added: three personalities, `profiles/timid.weights`,
  `aggressive.weights` and `castle.weights`, and `scripts/traits.py`,
  which reads how a census's trial team plays against the others.
  Castle is plain to see; aggressive dies more without killing more;
  timid reads as the defaults (LAB-REPORT-BRAIN.md section 85).

- Added: rewards in the census score: direct hits on hostile tanks,
  damage to hostile pillboxes, neutral and hostile pillboxes taken,
  kills defending a pillbox, and a bonus for taking every neutral
  pillbox. `CENSUS_REWARDS=0` scores as before; each team line shows
  the rewards' part and counts (LAB-REPORT-BRAIN.md section 84).

- Added: the meter's "seconds the squad out" line: seconds alive with
  the squad out, by task (road, bridge, wall, pillbox, mine, structure,
  boat, harvest, capture, repair, lift, home, idle), the trial's apart.
  It decides nothing; a census reads the same with it
  (LAB-REPORT-BRAIN.md section 83).

- Added: `scripts/rungs.py`, a switch census read map by map: the trial
  team's seconds a rung and its held ground, built and bridges against
  the others. Why atlantic: rotation takes half its ladder time, and
  the posture keeps it from building (LAB-REPORT-BRAIN.md section 82).

- Added: each default off again on the new defaults: the dodge, the
  posture and priced_crossing pay, team_rotate on costs atlantic, no
  default changes (LAB-REPORT-BRAIN.md section 81).

- Fixed: the a-raid-on-a-guarded-base family under its floor since the
  front read went off (5899e8e). It sets `w_front_push` to its old
  0.75, as the front scenario sets `w_front`, so it still tests the
  push point: 81.5% against a 75% floor (LAB-REPORT-BRAIN.md section
  80).

- Changed: the defaults lean toward holding what the base has, the
  turtle style of the round robin: the posture at 0.95, Defend and
  Fortify worth 3, Expand 0.4, a castle of four inside and two on the
  road, six posts, home at 50 armour, closing in at odds of 2; and
  team_rotate off. Against the old defaults +1.75 +/- 0.19 a seat over
  400 fresh seeds, no map below zero. The old defaults are
  `profiles/classic.weights`; turtle.weights is gone
  (LAB-REPORT-BRAIN.md sections 76 and 77).

- Added: the play styles' round robin read: the styles are ordered,
  turtle first, with no cycle on any map (LAB-REPORT-BRAIN.md section
  77).

- Added: `LOOKS=8 make census`, the sequential census: read as it
  plays (`scripts/sequential.py watch`), stopped at the first look past
  the O'Brien-Fleming bound, an early stop read again on fresh seeds;
  `sequential.txt`, `reread/`, `verdict.txt`. False alarms 2.8% against
  the fixed census's 4.8%; it saves seeds only on large effects, so it
  is an option and the fixed census stays the default
  (LAB-REPORT-BRAIN.md section 79).

- Added: `scripts/sequential.py DIR...` replays finished censuses under
  a group-sequential test (O'Brien-Fleming and Pocock, eight looks, the
  false alarm rate of two errors). Over 44 censuses it reached 43 of
  their verdicts on 76% of the seeds; not adopted, one early stop the
  full census undid (LAB-REPORT-BRAIN.md section 78).

- Added: `RESUME=build/census/STAMP make census` finishes a stopped
  census on its own binary and settings (`run.env`); `JOBS` defaults to
  the machine's cores. `documentation/CENSUS.md`, the census's manual,
  and `CLAUDE.md`, the working rules for an agent on a new machine
  (LAB-REPORT-BRAIN.md section 72).

- Added: the census on every map. `make census` plays one brain
  against another (`TRIAL`, `BASE`) over the fair census, its seeds
  split over `JOBS`, pooled overall and by map; `MAP` takes a list of
  maps, `gen` or `all` (eleven), here and in `make parts`, `switches`,
  `spsa` and `tune`. The dodge on against off over every map: +5.94
  +/- 0.38 a seat, for the dodge on ten maps of eleven
  (LAB-REPORT-BRAIN.md section 71).

- Changed: every function of the brain has a doc comment
  (`BRAIN-MAP.md`: 0 of 403 without one, from 100). `make brainmap`
  lists a trait impl's method as the trait's and no longer counts it
  as undocumented (LAB-REPORT-BRAIN.md section 70).

- Added: the territory read (`w_defend_ground` 1, on with the board): a
  hostile on the side's held ground (the land its pillboxes and depots
  hold) beyond the base's alarm counts toward the Defend posture, and
  the guard goes to the raided outlying work. A family of a raid on an
  outlying pillbox while the team is away: met in 400 of 400 seeds,
  none at 0. The guard: -0.36 +/- 0.27 a seat over 48 seeds
  (LAB-REPORT-BRAIN.md section 68).
- Added: scenarios take a `[[pillbox]]`'s `toward`, and a move event's
  `toward` and `step`; the meter counts seconds with a hostile on the
  held ground.

- Added: the bot skill dial. `SKILL=0.6 make run` (`--skill 0..1` on the
  window) makes the bots against you play at that share of full skill;
  your allies play in full. Their shells land further off and lead you
  less and their drones go up later as it falls; under 0.5 they stop
  dodging and kiting. In a duel a full bot outscores its opponent 63%
  of the time, one at 0.75 52%, at 0.6 25%, at 0.5 8% (LAB-REPORT-BRAIN.md
  section 67).
- Fixed: the player guide said `ROOKIES=15 make run` makes the bots
  easier; with teams, `make run`'s default, it reached no bot.

- Added: `fleet::tests::supply_trace` (ignored), trace 1 of the upgrades
  proposal: the proposal's supply paid from outside today's matches.
  48.8 supply a team-minute on naval.map, the first 60 at a median of
  97 s (PROPOSAL-UPGRADES.md section 7).

- Added: a scenario family of a duel in the open
  (`a-duel-in-the-open.toml`, floor 0.55): a Commander that rallies on
  a RuleBot 16 tiles off outscores it in 63.0% of 400 seeds with the
  kite, 51.5% without. Scenarios take a Commander's `pings = true` and
  the `outscores` expectation; the meter counts seconds kiting by
  whom (an intruder's guard, a wingman, a rally) (LAB-REPORT-BRAIN.md
  section 66).

- Added: the meter counts seconds kiting by step (massing, closing,
  stepping in, stepping out, holding) out of seconds alive. The trial
  seat kites 4.7% of its seconds alive on naval.map, 62% of that
  holding past the target's reach (LAB-REPORT-BRAIN.md section 65).

- Added: `make parts` (`scripts/parts.sh`), the reverse of `make
  switches`: each part off from the brain with every part on, over the
  fair census (`PARTS`, `SEEDS`, `SECS`, `MAP`, `RESPAWNS`, `JOBS`). Of
  31 parts at 12 seeds only the dodge reads past two errors, and it
  pays; the halved posture and swap gap read +0.61 +/- 0.60 at 48
  seeds, so the defaults are kept (LAB-REPORT-BRAIN.md section 63).

- Added: `island_site` where a naval start chooses (`w_site_beach` 20):
  afloat while the side holds no depot, each beach in sight costs 20
  tiles of the run less per point of its island's site score (forest,
  size), so the first landing is on an island fit for a base. A family
  of a boat by a bare islet lands on the forested island in 400 of 400
  seeds, none at 0. The guard: +0.82 +/- 0.69 a seat over 48 seeds
  (LAB-REPORT-BRAIN.md section 62).
- Added: scenarios take `depots = false` and forest terrain.

- Added: the guard posts at the front (`w_front_push` 0.75,
  `w_front_post` 0.5, on with `board`): the rotation's push point moves
  that share of the way toward the board's front sector nearest it, and
  a guard's post toward the nearest front sector, 6 tiles at most. A
  family of a raid on a guarded base meets the raider beyond the base's
  ring in 85.5% of 400 seeds, 50.5% at 0. The guard: -0.27 +/- 0.64 a
  seat over 48 seeds (LAB-REPORT-BRAIN.md section 61).
- Added: scenarios take the anchor `"centre"`, a `[[tank]]`'s `toward`,
  and `met_beyond` (how far from a point a seat first hit another); the
  meter counts a guard's seconds leaned toward the front.

- Added: the front read (`w_front` 0.5, on with `board`): Expand's depot
  costs half its distance to the base and half its distance to the team
  board's front, and is kept until taken; the front is kept on the
  hostile tanks seen in the last `front_memory` (30) seconds, since with
  only what was in sight it was known at 2.5 picks a match. The guard:
  -0.14 +/- 0.42 a seat over 48 seeds (LAB-REPORT-BRAIN.md section 58).
- Added: scenarios take `board = true`, a Commander's `rungs`, a
  `[[depot]]` moved by `at` and `offset`, and `depot_owned_by` and
  `depot_first`; the meter counts Expand's picks by the front.
- Added: `make families`: every family in `scenarios/families/` over
  its `runs` seeds, held to its `floor`; each falls under it with its
  part broken (section 59).
- Added: tests of their own for the crews' hand-out and the hop's
  bridge choice; a client test that the coordination screen shows an
  allied bot's rung and target; doc comments for crew.rs, hop.rs and
  board.rs, 99 functions of 392 left without one (section 60).

- Added: the duck foreseen (`duck_plan`, on): the point is held while it
  stays safe, else the one `drive_to` has out of the gun's reach or line
  by the time it is ready; a family of a tank under a ready gun from
  76.6% to 83.4% (LAB-REPORT-BRAIN.md section 57).
- Added: a shot that would ruin a dodge waits (`dodge_fire`, on). A tank
  has no turret; firing turns the hull, and a dodging tank that fired
  back drove into the splash. With LR 56: dodges caught by their own
  shell ashore 6.1% to 1.6%, afloat 4.0% to 0.5%; the score +0.21 a seat,
  error 0.68.
- Added: scenario `[[tank]]` takes `around` and `reload` (a time or a
  range); traces show ground, speed, heading and each hit with its
  damage; `SET` gives a RuleBot only the weights it holds.
- Added: the dodge plans against every hostile shell landing within its
  horizon, each at its own landing (`dodge_every`, on): two shells 76.5%
  to 100%, four 56.6% to 99.7% over 1,000 seeds (LAB-REPORT-BRAIN.md
  section 56).
- Changed: every part of the brain is on by default: `forget`, `board`,
  `team_rotate`, `priced_crossing`, `column`, `cycle`, `island_site`,
  `escort`, `bridge_jobs`, `duck`, `posture`. Against the parts off,
  -0.85 a seat with an error of 0.59 over 48 seeds, about 30% more
  brain time (section 55). Each old method is `caps = baseline, -name`.
- Changed: the dodge plans by the tank's own motion by default
  (`dodge_plan` 1, `dodge_release` 1; Paul's choice, LAB-REPORT-BRAIN.md
  section 54). New behaviours now land on, with starting numbers, and
  the census guards that the brain still works.
- Added: the dodge planned by the tank's own motion, two weights
  (`dodge_plan`, `dodge_release`; LAB-REPORT-BRAIN.md section 53).
  `drive_to` foresees where a tank will be, by the world's route and
  the sim's rules of motion, exactly (96 drives, worst 0.000 tiles).
  With `dodge_plan` the dodge takes a point the tank will have reached
  clear of the splash when the shell lands; with `dodge_release` an
  errand that would drive it back into the splash before then waits.
  Together they take two scenario families of 1,000 seeds from 89.6%
  (rolling) and 96.2% (standing) to 100%; in the census, own-shell
  catches ashore 10.1% to 5.7%, and +0.81 a seat with an error of 0.40
  over 96 naval seeds, not confirmed on 48 fresh seeds alone.
- Added: scenario `[[tank]]` takes `speed`, `heading` (or `"any"`) and
  `drive`, so a tank starts rolling; `[[shell]]` takes `offset`,
  `scatter`, `from_any` and `lands_in = [lo, hi]`, drawn from the run's
  seed, so one file is a family of situations; `seed = n` pins a file
  to one of them. Traces show where each route ends and where each
  shell lands.
- Fixed: a dodge whose point was water (or a wall) was refused, and the
  tank sat under the shell. The dodge now takes the first reachable
  point: the near side, the far side, back and forward along the
  shell's line; a boat may run onto a beach. Over the same 48 seeds,
  7.1% fewer deaths and a third fewer dodges caught by their own shell
  ashore (LAB-REPORT-BRAIN.md section 52).
- Added: `make scenario S=file RUNS=n` runs a scenario over n seeds and
  gives how often it holds; `SET="name=value"` sets a weight on every
  brain; `SEED=n` traces one seed.
- Fixed: a Commander built directly (`Commander::new`: tests,
  scenarios, tools) applies the default Settings, not only holds them;
  it played the landfall's weights at 0. Dealt bots were not affected.
- Added: a scenario of a Commander's boat that, without the landfall's
  memory, circles at the edge of a gun's sight for 39 s before landing
  beside it; with the memory it lands on the safe coast at 6 s
  (LAB-REPORT-BRAIN.md section 50). Scenario traces show positions and
  take `trace_every`.
- Added: scenario files. A playtest written as a small TOML file
  (`scenarios/*.toml`): the seats that think and how, pillboxes,
  depots, shells and timed moves placed by anchor, and checks at the
  end. Every file is a test; `make scenario S=file` runs one and prints
  its status codes over time. Three hand-written tests have scenario
  forms that pass and fail as they do (LAB-REPORT-BRAIN.md section
  48). Scenarios can carve sea and coast, put a tank afloat, set teams
  and weights, recall drones, and check where a boat came ashore; the
  landfall's memory has one that fails with the memory off (section
  49).
- Changed: `smooth` is a weight, `w_smooth` (0 to 1, default 0.05;
  Paul's choice), no longer a capability. The move's desire vector and
  the retreat thresholds blend by it; the rung's and the turret's
  confirmation, the stance's advance scalar and resupply by the
  doctrine alone act above 0.5. A weights file naming the old
  capability still loads (as 1). At 0.05 play is within the noise of
  before (LAB-REPORT-BRAIN.md section 46).
- Changed: `make switches`, `make spsa` and `make tune` build the
  brain's census once and run a copy of the binary, kept with the
  run's logs. The source can change while a census runs
  (LAB-REPORT-BRAIN.md section 45).
- Added: `make spsa` (`scripts/spsa.sh`), a tuner by simultaneous
  perturbation (SPSA) over named weights: each step one fair census
  with the trial team on the weights nudged up and the baseline teams
  nudged down, on fresh seeds, and the weights moved along the
  difference. The census takes `CENSUS_BASE_WEIGHTS`, a weights file
  for the baseline teams (LAB-REPORT-BRAIN.md section 40).
- Changed: the landfall's three weights are on by default
  (`landfall_memory` 30, `landfall_shy` 1, `landfall_wait` 10; Paul's
  choice). Bots remember hostile tanks for 30 s when choosing a beach,
  price their run past guns, and stand off a covered beach up to 10 s.
  A team with them beat teams without them by 2.31 a seat over 48
  seeds; a team without them among teams with them read even (-0.08,
  error 0.45), so what they do when every team has them is still to be
  measured (LAB-REPORT-BRAIN.md section 38). With every team on them
  (section 39), boats lost on the run for the beach nearly halve, the
  tanks that land die ashore instead, and matches run 4.5% longer with
  13% more time alive.
- Added: `landfall_memory` (0, as before): the seconds a hostile tank
  out of sight is kept where it was last seen, for a boat's choice of
  beach, its run, and its stand-off. With `landfall_memory` 30,
  `landfall_shy` 1 and `landfall_wait` 10 together, over 48 seeds, the
  trial team scored 2.31 a seat more (error 0.58) and lost half as many
  boats on the run for the beach (LAB-REPORT-BRAIN.md section 36). Off
  by default until the defaults are chosen. Each taken away in turn
  (section 37): without the memory, -1.08; without the stand-off,
  +0.69; without the run's price, +1.50. All three together are best.
- Added: with `CENSUS_ACTS=1` the census says whether the team had seen
  each landfall killer 10 to 60 s before. In 46% of those deaths it
  had, near where the boat died, and lost it: nothing in the brain
  remembers a hostile tank for long (LAB-REPORT-BRAIN.md section 35).
- Added: `Brain::beach`, the beach a boat is running for; with
  `CENSUS_ACTS=1` the census says where the gun that sank each boat on
  landfall stood. A median 8 tiles from the boat, inside its range and
  at the edge of sight; two in five at the beach, three in five along
  the run (LAB-REPORT-BRAIN.md section 34).
- Added: `landfall_wait` (0, as before): the seconds of a crossing a
  boat stands off a beach a seen hostile tank covers, out of its reach,
  before it lands anyway; a new status code, `stand_off`. At 10, over
  48 seeds, -0.51 a seat with an error of 0.63: the stand-off fired
  under 0.07% of the time, since the guns that sink boats are rarely at
  the beach (LAB-REPORT-BRAIN.md section 33). Off by default.
- Added: with `CENSUS_ACTS=1` the census says, of each landfall death,
  whether the boat or a teammate had seen its killer in the 10 s
  before. The boat had, 78% of the time; a teammate alone, once in 164
  (LAB-REPORT-BRAIN.md section 32).
- Added: `landfall_shy` (0, as before): a boat running for a beach
  prices the tiles of its run in a seen hostile gun's reach. At 1, over
  48 seeds, +0.65 a seat with an error of 0.69 and no fewer landfall
  deaths: a boat sees a gun only once in its reach (LAB-REPORT-BRAIN.md
  section 31). Off by default.
- Added: the census shows what kills. With `CENSUS_ACTS=1` it counts,
  for each of the trial team's deaths, the status code held last, every
  code held in the 5 s before, and what took the tank (direct fire,
  indirect, a strike, a mine, a pillbox; sunk or not). The meter counts
  shell dodges ashore and afloat and those hit before the shell landed.
  On naval.map the dodge is the code held last at 43% of deaths and the
  run for the beach (a new code, `landfall`, read `hop.idle` before) at
  20%, in 7.7% of the time (LAB-REPORT-BRAIN.md section 27).
- Added: the dodge's numbers are weights (`dodge_horizon`,
  `dodge_margin`, `dodge_clear`), and a new one, `dodge_hold` (0, as
  before): the share of the shell's flight a tank stays at its dodge
  point once there. Default play is unchanged. At 1, over 48 seeds,
  the trial team scored 1.18 a seat less (error 0.55), with no fewer
  hits from the dodged shell, so it stays 0 (LAB-REPORT-BRAIN.md
  section 28). The dodge itself, off over 48 seeds, reads 2.45 a seat
  less (error 0.67): it pays (section 29). The meter now counts the
  dodges by team, taken apart from dropped: ashore the dodge halves the
  kills (15.0% to 6.6%), afloat it cuts them by a fifth (18.2% to
  14.7%) (section 30).
- Added: the census shows where the trial team's time goes. Seat-seconds
  on each Auto rung (`Brain::rung`), the posture's changed picks by
  rung (`Meter::moved`), and with `CENSUS_ACTS=1` every seat-second by
  status code. On naval.map the ladder decides 5.6% of a team's time,
  the island hop 19%, and 46.7% is spent dead (LAB-REPORT-BRAIN.md
  section 26). The posture's lean moves tanks from Fortify to Expand;
  turning Economy off restores half the turret time but none of the
  score.
- Changed: the team board's posture is five scores, as LR 9 designed:
  Defend from a threat at the base, Economy from the material spent or
  a starved plan, Fortify from the castle left to build, Connect from
  the pillbox slots left, Expand from the odds against what the team
  sees, each with its weight (`w_pos_*`). `posture_scored` blends them
  with the old threshold ladder (0 is the ladder). Under `posture`, 48
  seeds read -1.04 a seat with an error of 0.58, no different from the
  ladder's posture; `posture` stays off and default play is unchanged.
- Added: the census gives its noise as a number. It prints the
  difference a seat seed by seed and its standard error over the seeds;
  a part is outside the noise only past two errors (`ledger::mean_se`).
  `make switches` measures every switch that is off, alone, over the
  fair census (`scripts/switches.sh`; `JOBS` runs several at once), and
  `CENSUS_SEED0` splits one long census over the cores. At 12 seeds of
  900 s (396 matches) none of the eleven was outside two errors; `board`,
  `island_site` and `escort` never acted (LAB-REPORT-BRAIN.md section
  24).
- Added, off: the team's posture leans the Auto ladder (`posture`,
  needs `board`; `w_posture`, 0.5: the posture decides among the rungs
  that hold; 0 is the old ladder). Measured over 48 seeds: -0.81 a seat
  with an error of 0.65, so it stays off.
- Fixed: the team board's posture was Defend whenever a hostile tank
  was anywhere on the map, since its threat field adds every tank's
  pull to every sector, or whenever one teammate was low on shells.
  Defend is now a threat within the base's alarm (16 tiles). The board
  is off by default, so default play is unchanged.
- Added: `make brainmap` writes `documentation/BRAIN-MAP.md`, every
  brain file and function with its doc comment's first sentence (17
  files, 337 functions, 120 undocumented). Check-in reports in IEEE form
  start in `documentation/checkins/` (CI-001: the measurement, a review
  of the brain's architecture, the tuner and its noise, faster
  playtests, a form for backlog items).
- Fixed: a NaN in a position can no longer panic the bots, the window
  or the simulation (backlog 10 and 19). All 93 float comparisons that
  unwrapped `partial_cmp` (74 in the brain, 6 in the window, 13 in the
  sim) use `total_cmp`, which orders every float. A new test panics on
  the old code and passes on the new; a census match is identical
  before and after, and every crate's tests pass.
- Changed: the bots' status is written as codes beside the words
  (`status.rs`; roadmap 12b). The brain's own logic (when a guard is
  idle, when a focus call goes out), the census counts and the
  coordination screen's reasons read codes, so rewording a status can
  no longer change play or a measurement. The words are kept for
  people. Measured: the strings cost nothing (15 us a seat-tick either
  way), and a census match is identical before and after.
- Added, off: the duck (`duck`; Paul: "build the duck reflex"). A
  direct shot cannot be dodged once fired, so it is denied: a tank
  leaves a hostile gun's range or line when that gun is ready within
  `duck_ready` (1.5 s) and before its own. The gun can be in sight or
  remembered for `duck_memory` (5 s), from a sighting or a direct shell
  of its in flight, since a gunner just past sight fires unseen. It
  ducks only with its armour share at or under `w_duck` (1: always).
  In a duel the tank was hit 2 times ducking against 3 standing. Fair
  census: 7.01 a seat against 7.52, inside the noise.
- Changed: the documentation marks what the improved prototype
  replaced. `documentation/archive/` is the development record: the
  roadmap of steps 0 to 11 moved there, and `BRAIN-METHODS.md`
  registers the brain's methods: archived (replaced as the default,
  most still one switch away, such as the old castle and the old
  census), current with an improved challenger still off, and shown
  impossible. In place: the brain lab report's section 2 is marked as
  the archived baseline of 17 October, with section 20 for the brain
  now, and the code index's snapshot of 13 October as archived, with
  section 6 for the files since. The roadmap and its backlog are
  updated (items 17 to 19 new), and the README lists the brain report,
  the upgrades proposal and the archive.
- Found: a led direct shot cannot be dodged once fired, from any range
  (LR 19, `a_led_direct_shot_cannot_be_dodged`). The shell flies five
  ticks at most, and a tank moves about 0.4 tiles off the lead in that
  time against a hit radius of 0.8. What to do instead is backlog 17.
- Added, off: bridge jobs on the team board (`bridge_jobs`, needs
  `board`; LR 6.2's plan). A bridge a teammate gave up half laid stays
  on the board. The next teammate with material within `bridge_reach`
  (40 tiles) takes it and lays the rest, wingmen too with `bridge_wing`
  (1). It comes off the board once laid, or after `bridge_keep` (180 s)
  untouched. Fairly measured with `board` on for every team: 7.31 a
  seat against 8.01 over six naval matches, inside the noise.
- Added: `CENSUS_BASE_CAPS`, the census's baseline teams'
  capabilities, for a part that needs another on.
- Added: the sliders (backlog 15; Paul: "a slider for each coefficient
  weights"). `U` on the coordination screen opens the weights page: a
  switch for each of the 15 capabilities and a slider for each of the
  70 weights, with its default, its step and what it does. It is the
  player's own team's: their tank on auto and, when the window hosts,
  their side's bots, at once and at each new match. Enter saves it to
  `~/.config/treadcore/weights`. `--weights FILE` stays every bot's.
  A bot keeps its target through a change, and a switch turned off and
  on again gives back what the bot's role had.
- Changed: the brain census is fair (backlog 16): each seed is played
  three times, every team in the trial seat once, so the starts'
  unevenness cancels and a null run reads exactly even (13.93 against
  13.93). The capped castle measured again: the old castle 8.36 a seat
  against 9.08 over nine naval matches. `make tune` defaults to two
  seeds.
- Added, off: the rest of the smoothing (`smooth`): the Auto ladder
  holds a rung until a new one has won two thinks (a threat to the
  base still changes it at once), and the stance comes from the odds
  averaged a think at a time, pressing above `advance_band` and falling
  back below it. Over six fair naval matches it scored 7.55 a seat
  against 9.47, so it stays off.
- Added, off: the escort (`escort`, Paul: "go out on a mission with the
  team to start moving the pillbox along the newly created road"): a
  fleet leader's fetch trip past `fetch_gather` (20 tiles) gathers its
  wingmen at the base first, and the fleet screens the carry home, the
  router preferring the road. Proven in a match alone with one wingman
  (lifted by 324 s); a 900 s naval census never reached a far fetch.
- Added: the brain's step 12 groundwork (Paul, 18 October: "a
  flowchart logic system with weights that can be adjusted and
  componentized ... first step is memory and passing of parameters";
  `documentation/LAB-REPORT-BRAIN.md`, ROADMAP goal 4). Every new part
  is behind a capability bit and off by default, so the bots play as
  they did on 17 October until a census promotes a part.
  - Measure (12a): `treadcore-brain::meter` counts seat-ticks, thinks,
    Moves, crossing searches (and those the cache answered), plan
    copies and time in `run_bots`; the census tests print it.
  - Memory (12b): the Auto ladder no longer copies the whole castle
    plan every tick to ask whether a turret slot is left
    (`Plan::turret_preview`: 2 to 5 copies a 900 s match, where it
    was one per Auto tank per tick); failed and found crossings are
    kept a minute (`facts::MapFacts`: 1,368 answered from the cache
    against 207 searched over three naval matches); the target
    picker builds its reasons for the winner only; given-up lists are
    `memory::Forget`, which can run out (`forget`).
  - Weights and capabilities (12c): `weights::Settings`, one flat
    array of 56 named weights (the 22 of `Tactics` and its other
    settings at their old values, and the new parts' numbers) and a
    `Caps` bit per part; a `name = value` file, `--weights FILE` in
    the window. The geometric-mean utility and response curves for
    the components to come.
  - Intent and smoothing (12d, `smooth`): a moving goal followed by a
    desire vector, the turret holding its target until a challenger
    has won three picks by 15%, one resupply threshold for the
    Commander and its doctrine.
  - The team board (12e, `board`, `team_rotate`): one shared picture
    per team, rebuilt at 1 Hz: influence (ours less theirs) on 16 x 16
    sectors, the front, defence points, the jobs given out (a crew
    that missed its `Requested` finds it there), a posture; a hurt
    tank at the hottest point swapped for a fresh one, confirmed over
    two rebuilds.
  - Behaviours (12f): the base cycle after a fill (`cycle`: one plan
    item, a mend, road toward the next friendly pillbox or depot);
    crossings priced boat against bridge, a bridge past 6 tiles
    costing the square of the extra and every tile dearer on an
    island short of trees (`priced_crossing`); a small bare island a
    poor base (`island_site`); a fleet in file on a bridge, the leader
    not stopping to gather on one (`column`).
  - The ledger and the tuner (12g): `brain_census` (one team on the
    trial settings, two on the baseline, rotated over the seeds)
    writes a JSON line per seat to `reports/ledger/ledger.jsonl`;
    `ledger_ratings` prints the genomes by Elo; `make tune` runs a
    (1 + lambda) evolution strategy over it.
  - Measured on naval.map, three seeds of 900 s: baseline against
    baseline scored 13.95 to 8.93 a seat, so three seeds cannot tell
    a part from the dice; every new part on scored 10.29 against
    11.53. Nothing is promoted; the census needs more seeds first.
    Bots cost 28 us a seat-tick.
  - Paul's answers as scalars (12h; "should it x or y, we choose a
    scalar of how much x, and how much y"): `w_mend`, `w_mine`,
    `w_works` and `upgrade_vs_mend` for the base's improvement;
    `connect_bases` for the road out; `long_game` for the score;
    `assist` for a person's team's builder. And his castle,
    `castle_cap`: two pillboxes inside the wall and one outside beside
    its road, then road posts every 10 tiles toward the side's other
    bases, beside the road and never within 3 tiles of a junction.
  - Changed: the capped castle is the default (Paul, 18 October:
    "make castle_cap the default"). The castle of 11b and 11h is
    `-castle_cap`. Over six naval seeds it scored 0.96 a seat less
    than the capped one, read against a null run on the same seeds;
    the null run itself showed the census's trial team draws better
    seats (12.66 against 6.44 with every team the same).
  - Upgrades and the ways to buy them designed (ROADMAP 11g): supply
    earned from what a side holds, twelve base upgrades and four for
    the tank, bought at a depot. The proposal is written
    (`documentation/PROPOSAL-UPGRADES.md`); it waits on its seven days
    and two traces before the sim step (wire version 17).
- Added: game speed (Paul, 17 October: "a 4x speed adjuster. it will
  speed up (the server and all clients), if we skip frames then reduce
  speed"; client `speed.rs`). **`** (Key Setup: game-speed) steps the
  host through 1x, 2x and 4x; `--speed N` or `SPEED=N` starts there,
  in the window and in `treadcore-server`. The host's FixedUpdate
  timestep is `1 / (TICK_HZ * speed)`; a joined window measures the
  host's tick rate and steps its prediction to match, so there is no
  wire change. The governor steps down a notch when the host's ticks
  fall under 85% of the speed (none in the first 10 s after a start
  or a change), and back up after 30 s of keeping up; the frame rate
  is logged, not judged (the VM's three windows draw 4 fps at 1x and
  4x alike, and kept 3.76x of 4x). The dedicated server steps down
  when it falls a second behind. The title bar shows `x2` or `x4`.
- Added: rotation, a push point and a defense point for every bot
  (Paul, 17 October: "there was too much waiting ... the tank does
  not wait around ... alternate between the adjustable push point and
  defense point"; `treadcore-brain::rotation`). The defense point is
  the base's post; the push point is halfway from the base to the
  side's objective, moved onto ground the tank can drive to from the
  base (halfway to an island is on the water), refitted every 10 s.
  A side's members work it out alike: the fleet's push point; a tank
  can have its own instead. A layer over the ladder and the fleet,
  not a change to them: where a tank used to wait (defend base on
  guard with nothing in the leash, the turrets rung with nothing to
  fetch or standing) it drives out, looks round 6 s, drives home
  (staying up to 25 s to refill), looks round, and out again. In the
  window, a right click on the coordination screen sets the tank's
  own push point; the screen and the map show both points.
  Over six 600 s quad censuses: kills 226 to 249, revives 44 to 51;
  "on guard" (15 to 18% of all seat time) gone. Moving the fleet's
  gathering to the push point too was tried and dropped (kills 191:
  a leader past it drove back to it).
  - Wingmen rotate too (Paul: "rotate the idle wingmen too"): with
    the leader parked 8 s, a wingman goes between its slot and a
    point toward the objective, 9 tiles from the leader at most and
    on ground all the way there (inside the gathering ring), and
    closes up the moment the leader moves. Over the same six
    censuses: kills 249 to 260, "in formation" 9.4% to 4.4% of
    wingman time, stuck samples 12 to 0; revives 51 to 44.
  - Open: a boat on naval.map at (61,102) keeps choosing (60,102),
    water on the map as drawn (a bridge, likely), to land on, and
    sails in place; it was there before rotation.
- Added: the yard (Paul, 17 October: "a base, with roads in the
  centre, walls on the outside, and pillboxes and depots to
  recharge"). Once the castle and its turrets are down, the plan
  closes the curtain wall right round the depot (the gate left open)
  and lays a ring road inside it, 3 tiles out, between the depot and
  the towers. The turrets rung builds the yard while there is no
  outlying pillbox to bring in, and crews take its tiles as they take
  the castle's. A headless three-fleets match: the first team to
  finish had 25 wall tiles, 21 road tiles and 6 pillboxes (2 carried
  in) round its depot by 300 s. `three_fleets_build` prints each
  base's walls and roads.
- Fixed: three autopilot leaders of a windowed quad stuck most of
  the match, seen in its screenshots.
  - The road on from a bridge ran its straight line on past more
    water; the drive up to the next tile was refused ("water") and
    the leader sat at its depot 440 s. The line now ends at the
    shore, and a road on with no tile laid in 20 s is given up.
  - A bridge waiting on the fleet (or on material) counted the wait
    as progress, so it never stalled out: a leader waited 200 s on
    a part-laid bridge. Only a tile laid is progress now; with no
    material the stall is 80 s, not 40.
  - The castle's post, 1.5 tiles behind the depot, was on the water
    at an island's shore depot, and the leader hopped at it under
    fortify. The plan's post is now the nearest land within 3 tiles.
  `the_default_match_plays_as_teams` passes again.
- Changed: no text over drones on the battlefield (Paul, 17 October:
  "the drones have too much text"). Each drone of yours has a small
  bar under it instead: battery left, green, amber or red by its link
  (white for a longshot, which has none). The air line at the bottom
  left still has the figures. The old label turned with the drone.
- Added: `TREADCORE_ZOOM` starts the window's camera at that scale
  (0.7 by default, smaller is closer): for a playtest's screenshots.

- Fixed: bots stuck for the rest of a match, found by a headless quad
  (`fleet::quad_census`, ignored: fifteen seats, three autopilot
  leaders, a stuck seat is one on its way somewhere that has moved
  under 3 tiles in 90 s). Over six seeds, stuck samples 112 to 0,
  kills 189 to 260, revives 31 to 51.
  - A guard whose post was over the water, once the island hop gave
    the post up ("no forest for a boat"), drove at it, refused, for
    the rest of the match. A refused drive is now not asked again
    until the refusal runs out (60 s); the guard holds where it is
    and says so. A target the hop gave up on is tried again after
    60 s.
  - The hop gets a boat's material at a depot of the side within 40
    tiles (one a half second) before felling, and with no forest in
    sight drives along looking for 40 s before giving up.
  - A moored boat that cannot be reached (refused, or not boarded in
    30 s) is given up for another.
  - A hop pauses for a hostile tank within 1.5 direct ranges, not one
    merely in sight (sight grew; a far boat held every hop up).
  - A RuleBot whose every depot is over the water resupplies by sea
    instead of driving at the nearest, refused, for good.
  - Sim: a boat's route no longer cuts a landing past a bridge's
    corner (a boat cannot go onto a bridge; it was blocked and routed
    the same way every tick), and a blocked tank planned the same
    way again goes to its tile's centre first.

- Added: drone marks (`rules::drone_spec`, `Drone.mark`,
  `Event::DronesUpgraded`). Every kind comes in three marks after the
  systems in RESEARCH.md; the match moves to Mk II at 4 minutes and Mk
  III at 8, a drone keeping the mark it was launched as. Recon:
  quadcopter, VTOL, high-altitude VTOL (longer, further sight).
  Interceptor: kinetic rammer, radar-cued, high-speed autonomous
  (better on its own camera, faster). Strike: FPV, fibre-optic FPV
  (flies through jammers), loitering munition (longer, heavier).
  Longshot: faster, then heavier. The air panel shows the mark.

- Added: bridges before boats (Paul, 17 October: "the tanks should not
  try to cross water ... build roads where it would drive ... prefer
  long straight roads ... after a bridge is made, continuing that road
  to a pillbox or a depot"). `Map::crossing` plans the way to a target
  for a side that will bridge it: four-way steps, ground at its move
  cost, water at 8 a tile and 4 more a turn on the water, so a bridge
  is short and straight. An island hop with the map bridges a crossing
  of up to 20 water tiles: the tank at the end so far, its squad lays
  the next tile (1 material, 3 s), felling or fetching material as it
  runs out; no tile in 40 s or three refusals and it goes by boat. A
  bridge done, it lays a straight road on to the target, 12 tiles at
  most. Brains get the map (`Brain::think_on`, the host's world map, a
  window's copy for the autopilot): every window draws it already.
  Four quad seeds: 44 to 70 bridge tiles a match, boats boarded 577 to
  122, boats sunk 45 to 2, but kills 161 to 108: the fleets spend the
  opening building and meet late. Bridging only up to 8 tiles: kills
  184, boats 462, sunk 33.
  - Spans (`BRIDGE_SPAN`, a sim rule; Paul, 17 October): a squad that
    lays a bridge tile lays on straight out from where it stands, up to
    3 tiles in the trip, a material each; it stops at ground, a tank or
    a boat, a depot's ground or the material's end. Four quad seeds:
    kills 98 to 129, bridge time 28% to 20% of bot time.
  - The fleet lays it: a leader out of material hands the next tile to
    a wingman near it (a crew's job, `Job::Build`) and waits at the end
    rather than leaving for more; a wingman takes its leader's jobs,
    takes any job before anything else of its can return (a job is one
    tick's event, and a wingman dodging that tick lost it), and starts
    no bridge of its own (two of a fleet began the same one). Wingmen
    laid three bridge tiles in four.
  - No bridge through water beside a depot, where nothing is built:
    bridges to a depot on the shore were refused at the last tile.
  - Open: most bridges are still left part-laid (the hop changes its
    target or stalls), the road on from a bridge is down to a tile a
    match, and `the_default_match_plays_as_teams` fails: in its eight
    minutes the teams barely meet.
  - Fleet: a wingman's slot on the water (the leader at a bridge's end)
    moves to the nearest ground; one beside its leader is in formation.
    A wingman afloat steers at its leader only while the leader is
    afloat too.
  - `quad_census` counts bridge and road tiles laid.

- Changed: driving (Paul, 17 October: "more momentum, more consistent
  top speed ... something that still moves forward during a turn").
  Acceleration 4 tiles/s/s (was 3), braking 6 (was instant), hull turn
  3 rad/s (was 2.5). A corner past 34 degrees eases the tank down with
  the angle to 35% for a turn right round (was 25% for any corner); it
  brakes before a waypoint for the corner after it, and pivots where it
  stands when the way on is more than 115 degrees off (rolling on, a
  tank facing a wall drove into it every tick).
  - The Defender dodges a spotted shell inside its moat: the attackers
    kept their speed through a turn and dodged better; `defenders_hold`
    was 35 defenders of 120 alive against 18 attackers of 40, and is
    66 against 13 (48 against 11 before the change).

- Fixed: `make quad` and `make run` were free-for-alls of plain
  RuleBots: the Makefile exported `TEAMS ?= 0`, which the scripts'
  `${TEAMS:-3}` kept. It is empty now, each script's own default.

- Added: fleets (Paul, 17 October: "the allies dont seem to group up.
  can we have fleets. i want to see the tanks prioritise grouping and
  traveling in formation. and then using shared orders and attack
  togeather"). A bots' team is a leader, a guard and three wingmen
  (the two rangers, which roamed alone, are wingmen now and still crew
  for the castle); a person's team keeps its builder, guard and two
  wingmen (`fleet::fleet_of`).
  - The wedge: each wingman has a slot, row k / 2 + 1 behind the
    leader, left and right in turn, 4 tiles a row, where it will be
    in a second at the leader's speed.
  - Grouping first: a leader about to hunt, expand or launch holds
    until half its wingmen are within 10 tiles, at most 30 s, then
    not again for 30 s; it says "fleet: form up on me" in team chat.
  - Crossing together: wingmen within 20 tiles launch with a leader
    that launches, sail in the wedge round its boat, and land beside
    it; one left ashore waits for the landing and follows by sea.
  - Shared orders: a leader's ping is its fleet's order. A wingman
    answers its own leader's calls and nobody else's while the leader
    is up, past the "enough of us" cap, and with two of the fleet at
    the target presses whatever its own odds say. The leader's call
    says "fleet: attack together, n of us".
  - `quad_census` prints the fleet's cohesion: wingmen within 12 tiles
    of their leader, 31% to 60% of samples over four seeds, no seat
    stuck.

- Added: kiting (`tactics.rs` `kite`; Paul, 17 October: "more directed
  kiting, where we estimate the range of the opponent and avoid
  getting hit while aggressing closer in a group"). A bot closing on a
  hostile tank, answering a ping or as a wingman covering its leader,
  waits at 12.5 tiles, past the target's direct reach and inside its
  own indirect gun. It steps in to 9 tiles, fires, and steps out only
  while the target's gun is reloading for longer than the trip (its
  reload is in the observation), no other loaded gun or live pillbox
  covers the spot, and its own gun is loaded. Pressing, it also steps
  in while a teammate is already inside the target's reach; massing,
  never. With the target out of the side's sight for under 6 s it
  holds the ring round the last spot instead of driving onto it.
  Its own gun counts as loaded if it will be by the time it is in:
  the indirect gun at the ring fires every reload, and a gun loaded
  now was rarely so when the target's window opened. `kiting_pays`
  (ignored; one team of three kiting, turned round the seats), six
  seeds, a team's mean: kills 12.7 against 10.7, direct hits taken 3.5
  against 4.6, deaths 13.0 against 13.0, depots 3.8 against 4.4.
  - Defenders (guards, defend and hold) kite an intruder too, from
    the side of what they defend and on their leash. No measurable
    effect over eight quad seeds: guards rarely meet a tank.

- Changed: the forks the brain map showed missing, found by where the
  quad's bots died (`quad_census` now prints what each seat was doing
  when it went down, and by what: of 233 downs over four seeds, 108 to
  indirect shells, 69 boats sunk, 30 strike drones, 20 direct shells,
  and three in five during island hops). Eight seeds, against the
  brain before them: downs 489 to 404, downs not revived 376 to 341,
  indirect-shell downs 243 to 143.
  - A Commander's dodge held for one tick: the next, its errand
    re-ordered the tank back under the shell. A shell's dodge now
    holds until it lands, and comes before a revive and a hop (a
    reviver parked by a wreck never dodged). A jink stays one order:
    held, it ran tanks out of their own bubble (strike downs 46 to
    100) and off wrecks (revives 113 to 62). A tank whose squad is
    mending a wreck does not jink.
  - A covered crossing (`hop.rs` `covered`): a hop holds its launch
    while a hostile tank with shells is within 40 tiles, a shell is
    coming down within 8, or a live hostile pillbox is on the bank;
    at most 20 s, then it goes. Without it: downs 429, not revived
    383 (against 404 and 341). 1.8% of bot time is spent holding.
  - Air in the odds (`Tactics::strike_worth`): a hostile strike drone
    over the fight counts 0.35 of a tank. Without it: strike downs 92
    against 64, downs 435 against 404.
  - Revives fell (113 to 63 over the eight seeds): fewer downs, and a
    reviver now leaves for a shell. Open; `quad_census` now prints
    how each revive attempt ended.
  - Afloat, a hop lands on the beach in sight within 25 tiles of its
    target that is least covered by hostile guns (the doctrine's
    landfall scale), not the world's beach nearest the target.
  - A kited target past reach for 15 s with no window is a standoff:
    the rally ends and no ping is answered for 20 s (the kiting team
    held fewer depots). Over twelve `kiting_pays` seeds kiting is
    even at best now: deaths 14.1 against 13.1 with this, 13.8
    against 12.7 without; depots 4.0 against 4.4, 3.8 against 4.5.
  - Tried and off: a plain RuleBot kiting (`RuleBot::kites`). Ten
    seeds of `rulebot_kiting_pays`: kills 20 against 15, deaths 26
    against 18, direct hits taken 31 against 15.
  - Bots lead a direct shot: a shell flies 10 tiles in 0.25 s and
    hits within 0.8 of where it lands, so a tank at full speed was
    missed. Rookies still aim at where the tank is.

- Changed: the HUD again (Paul, 17 October). The right column is a
  quarter narrower (148 px, was 196), the top strip a quarter shorter
  (44, was 58) and the bottom strip half as tall (64, was 128). The
  stock (shells, the bay, squad and material, armor and fuel) moved
  from the bottom strip to the top of the right column, under the
  autopilot; the roster moved below the lamps, in 20-character lines,
  and the pillbox, depot, tank and works lamps take only the rows the
  match fills. The minimap is at the foot of the right column, as wide
  as it. The bottom strip is the air table and the newswire.
  The guide's screenshots of the columns and the bottom strip are of
  the old layout.

- Changed: a launch from the shore (**E**) takes the squad's material
  up to five and launches with none (Paul, 17 October: a launch for
  every tank). A bot with a bank in sight boards a moored boat only
  within 6 tiles (was 30) and launches from the bank otherwise. Six
  quad seeds: walking to a moored boat 8.5% of bot time to 4.6%, kills
  349 to 344, revives 73 to 69, boats sunk 102 to 111, time dead 12.7%
  to 12.9%.
  - Sim: a tank on its way to a moored boat slows to turn inside the
    bank's waypoint, as a boat already did (an autopilot leader went
    round the bank, 0.4 tiles off, for 90 s). Doing it for every tank
    ashore changed how bots fight: rookies won 4 of 10.

- Changed: a boat a tank lands from stays moored on the water beside
  the beach, for anyone to board (TNK-17, the prior art's way; it was
  spent). Over the same six seeds, after the change below: finding
  material 12.4% of bot time to 0%, boats boarded 569 to 864, kills
  263 to 349, revives 45 to 73; time dead 9.1% to 12.7%, boats sunk
  71 to 102, walking to a moored boat 8.5%.
  - Sim: a tank driving onto a moored boat that is blocked plans
    again by land to the bank beside it (it planned by land to the
    water, found nothing and stopped a tile off), and pivots to face
    the boat rather than going round it at a crawl.
  - Sim: two tanks drawing a pillbox's last shell in one tick took
    it twice (an overflow, found by the default match's test).

- Changed: a bot on an island hop with a boat's material drives to the
  bank and launches (**E**) instead of sending its squad to build a
  boat there; the squad still builds if the bank is refused or not
  reached in 20 s. Over six quad seeds (`quad_census`, which now also
  counts boats and boats sunk) building boats fell from 13.8% of bot
  time to 3.7%, but finding material rose from 7.2% to 12.4%: sea
  errands 37.3% to 34.5%, time dead 11.0% to 9.1%, boats sunk 81 to
  71, kills 289 to 263, revives 64 to 45.

- Added: launching from the shore. `Command::Embark` (**E**): a tank
  on the beach, squad aboard with five material, puts a boat on the
  water beside it and is aboard at once. Bots on an island hop use it
  at the water's edge.

- Changed: sight. A tank sees 10 tiles (was 7: three short of its
  direct gun), a recon 20 (was 12), an interceptor or strike 8 (was 4).

- Changed: the HUD. The side columns are narrower (the orders column
  52 px, the right 196 px) and the field wider. The orders column is
  one column of icons, no words, each key in a round badge on its tile
  that goes red while that mode is armed. A bearing ring round your
  tank: a thin arc toward every tank in sight, red hostile, green ally,
  longer and brighter the nearer it is.

- Changed: the window's bezel is brushed metal. The four gray panels
  are metal plates with rounded, outlined corners, a chrome gradient,
  horizontal brush streaks, a lit top rim and a shaded bottom; the
  autopilot button is a metal capsule, pushed in (the gradient turned
  over) while held or on, green-cast when on. The textures are drawn
  at startup (`client/src/metal.rs`), no image files, and every frame
  wears one nine-sliced, so the corners and outline keep their size
  whatever the frame's. Words on the metal are etched dark, the order
  keys oxide red; the black displays are rounder.

- Added: down and revived. A destroyed tank is down for twenty
  seconds (`DOWNED_TICKS`) before it is dead; an ally's squad sent to
  it (`PillboyTask::Repair { target: tank }`; in the window a click on
  the fallen tank in any builder mode) works four seconds for two
  material and brings it back where it fell at half armour
  (`Event::Revived`). Allies see their side's downed tanks. Bots
  revive the ally they are best placed for and ping for help when
  down; the autopilot does both. A side with a tank down and one
  standing is still in the match. Respawn opens when the time down
  ends (`RESPAWN_TICKS` is now `DOWNED_TICKS`). Wire version 16.

- Changed: the window's default match has respawns without limit (it
  ends on its fifteen minutes); three each ran the quad out of tanks.

- Added: fleet tactics (`treadcore-brain/src/tactics.rs`), weights
  with reasons for a team's Commanders and the autopilot. Targets by
  score: near, wounded, one shot from dead, afloat, called by a ping,
  a teammate on it (to two; past that, overkill), in range, a threat
  to a teammate or the base, the held target kept. Focus calls in team
  chat. A rally reckons Lanchester's square-law odds at the fight:
  masses out of range for teammates coming when behind, presses when
  ahead, and stands on the firing point with fewest other guns on it.
  Three teammates nearer a ping are enough; a crew on a job answers
  within 40 tiles. The status line and the logs say the target and
  why. `fleet::commander_as`, `Commander::set_tactics`; the ignored
  test `tactics_pay` plays a team with them against two without.

- Changed: `make quad` is three windows, one a team, each its team's
  commander on the autopilot, bots to fifteen seats (`CLIENTS`,
  `TEAMS`, `PLAYERS`, `AUTOPILOT=0`, `RESPAWNS`); it now loads the
  musl stack preload as `make run` does. On `--autopilot` the pilot
  is the side's fleet commander and the person's team is dealt as a
  bots' team. The host's log has every seat's line every 30 s; each
  window's log its commander's.

- Fixed: a Commander whose drive was refused as over the water (home
  to resupply, a guard's post, a leader across the sea) drove at it
  again every second for the rest of the match; now it goes by boat
  (`Hop::send`), and resupply tries the nearest depot not refused. A
  rally to a fight over the water is dropped.

- Added: teams play as fleets. In the window's dealt teams (the
  default three) a person's team gets a castle builder, a guard and
  wingmen who stay with the person; a bots' team a leader, guard,
  rangers and wingmen (`fleet::team_brain`). `--plain-bots` for the
  old RuleBots.

- Added: the chain. `Command::Tow { target }`, `Event::Chained`; **C**
  in the window. A tank hitches a chain to a pillbox or depot of its
  side within 2.5 tiles; it moves only with two tanks of the side on
  it (Paul: "this action requires an ally"), dragged at a tenth of
  their speed over land, a depot's tile with it; one tank alone is held
  at the chain's length. A pillbox on a chain is silent.

- Added: hauls. `Job::Haul { target, to }`: once the castle stands, a
  fleet commander gives two crews the same job on an ally's outlying
  pillbox within 40 tiles; they chain it, drag it to a turret slot
  (the lower seat left of the line, the other right) and let go. Chain
  help stands aside for a crew on a haul of its own.

- Added: the autopilot respawns its tank as bots do. A bot or the
  autopilot back in a new tank drops the old one's errands (resupply,
  rally, chain help, crew job, island hop).

- Added: pings on the minimap, as rings spreading from the point in
  the pinger's colour for four seconds.

- Added: fleet commanders (treadcore-brain `crew`). One Commander a
  side, a bots' team's leader or a person's team's builder, keeps the
  castle plan and organises it: `Command::Request { to, job }` with
  `Job::Muster`, `Ready`, `Build`, `Fetch` (lift your pillbox into a
  turret slot) and `Release`, `Event::Requested` to the side. Crews
  (rangers, wingmen while their leader is at the fort) take a job,
  fell for it, build it and report ready; a late job goes out again,
  and a tile that fails twice leaves the plan. The plan counts a
  crew's builds on its tiles.

- Added: chain help, first of a bot's priorities after a dodge. A tank
  of the side that chains on alone is answered by the nearest free bot
  of the side, which says so, hitches on and hauls beside it to where
  it drives, and lets go with it.

- Added: bot talk. A fleet bot that calls an enemy says so in team
  chat with a plan for the allies in reach (alone, focus fire, or a
  pincer); each that answers says its flank and distance and comes in
  from it, left, centre or right, sixty degrees apart.

- Added: pings. `Command::Ping`, `Event::Pinged` (to the pinger's side;
  one a second). **Q** or the middle button pings the cursor; a ring
  in the pinger's colour marks it. Fleet bots ping hostile tanks they
  see, and answer a teammate's ping within 90 tiles by converging on it
  and fighting what is there (not the builder or the guard).

- Added: dodging. A bot (not a rookie) or the autopilot drives out from
  under a hostile indirect shell that will land on it, and jinks across
  its heading when hit.

- Fixed: a Commander that started at sea took the open water as its
  base for the match; it now waits for its side's first depot, and
  builds no castle until then.

- Added: island hopping (treadcore-brain `hop`). A RuleBot or the
  autopilot idle for 20 seconds, with nothing hostile in sight and
  something untaken on the map, goes for the nearest depot or neutral
  or dead pillbox; over water, it boards a moored boat within 30 tiles
  or has its squad fell forest and build one at the bank facing the
  target, then sails for it. Landing clears what the bot had found
  unreachable. The observation carries `launch`, the water beside the
  bank near the tank. The first naval match had tanks sit on their
  first island for the rest of the match.

- Changed: the window's default match is the naval one: `maps/naval.map`
  built in, three teams (`--teams 3`; the map's starts are now three
  sectors, one a team), three respawns each. `--map island`,
  `--teams 0` and `--respawns 0` give the old default; `make run` takes
  `TEAMS`, `MAP=island` and `RESPAWNS`. The server's defaults are
  unchanged.

- Added: a big AUTOPILOT button at the head of the right column, green
  while the bot drives; a click is **O**.

- Added: respawns. `--respawns N` (or `unlimited`) on the window or
  the server lets a destroyed tank come back N times, five seconds
  after, at its start with full stocks (on a landing craft at a sea
  start, SPEC TNK-08); **V** in play, and bots come back on their own.
  A side with a respawn left is not out. Off by default: elimination
  as before. New `Command::Respawn`, `Event::Respawned`; wire
  version 15.

- Added: boats, after the prior art (SPEC TNK-08, TNK-11 to TNK-14,
  TNK-17). A start on water puts its tank on a landing craft; afloat
  it crosses water fast, cannot pass a bridge, and lands by driving
  onto a beach. One hit sinks a boat and the tank with it, the kill to
  the shooter. A squad builds a boat from five material on water
  beside its bank (road mode with Shift); any tank boards a moored boat
  by driving onto it. New map `maps/naval.map` (scripts/navalmap.py),
  every start at sea. Wire version 15.

- Added: bots run for the beach. A RuleBot (and a Commander, on any
  goal) afloat does nothing but make for the nearest beach that is
  away from every hostile tank and pillbox it knows of.

- Added: bots watch their own progress. Every 30 seconds a RuleBot
  reads its form (score gained, depots gained or lost, armour lost,
  contact or none) and plays to it: hurt for nothing, it waits longer,
  stands further off and goes home for repairs sooner; stalled, or
  gaining, it presses. A `--local` match logs a progress line a seat
  every 30 seconds with what each bot says of its form.

- Added: tank styles and a livelier battlefield, after the sprite kit.
  Three chassis, bulwark, strider and rattler: `--style NAME`, or **T**
  in play, picks your side's; other sides take the other two. Turrets
  turn apart from the hull toward what they shoot at, with a flash
  and a kick; treads roll in four frames; tanks cast shadows and leave
  track marks; shells, mines and wrecks leave scorch. Tank labels stay
  upright. Cosmetic only: nothing new on the wire.

- Added: the autopilot. **O** (or `--autopilot`) puts your tank on
  auto, the Commander's ladder, without the coordination screen; **O**
  again or a click to drive takes the wheel back.

- Added: the ladder's **expand** rung (sixth on the coordination
  screen): take depots not yours, their guard pillboxes first, then the
  far pillboxes. A side on auto used to hold its first three depots all
  match. Allied tanks on one depot no longer contest it (and both
  resupply). A fleet bot whose leader dies takes command. Under unseen
  fire a tank under a goal scoots, sends its recon to where the shells
  came from, and fires back.

- Added: fleets. `--fleets` (`FLEETS=1 make run`): sixteen seats as
  three base-building teams of five and one aggressive bot alone. Your
  team assists you: a builder fortifies your depot, a guard defends it,
  two wingmen follow you. The coordination screen lists your fleet and
  what each bot is doing. Allied squads on one pillbox now take it
  together instead of contesting it.

- Added: pillboxes move (roadmap step 11h; wire version 14). In
  pillbox mode, Shift-click a pillbox of yours: the squad lifts it and
  carries it, and the next pillbox you build is that one set down, at
  its health, for no material. Lost with the squad or the tank, it
  falls there, dead.

- Added: defend base and auto. `G` now starts at **auto**, a ladder the
  tank climbs by itself: defend the base when something comes for it,
  else fortify, else (once the castle stands and the base has been
  quiet twenty seconds) bring outlying pillboxes of yours home into two
  turrets at the wall's ends, else hunt pillboxes near the base, else
  guard. **Defend base** is also a goal of its own.

- Changed: **Space** opens the coordination screen and hands the tank to
  auto; Space again gives you the wheel. Stop is now **H**. On the
  screen `1` to `5` turn the ladder's rungs on and off, `G` picks one
  goal, a click holds the tank there, Tab folds it away. A keys file
  saved before this keeps Stop on Space and puts the coordination
  screen on H; Key Setup (`K`) swaps them back.

- Added: fortify (roadmap step 11b), the fifth goal on `G`. The squad
  builds a castle's gate round the nearest depot of your side: two
  guard towers flanking the gate and a pillbox behind the depot, each
  covering the others; a curtain wall across the threat's side with a
  one-tile gate; a road out through it. A tank from the front is
  routed through the gate.

- Added: goals (roadmap step 11a). `G` steps the tank's goal: hunt
  pillboxes (shoot a live enemy one dead from between 8 and 10 tiles,
  walking round a blocked line, then the squad takes it; a neutral one
  the squad takes live), defend depot, defend pillbox, hold, escort;
  a click to drive or Space takes the wheel back. The top strip says
  what the tank is doing for its goal. `--wingmen N` (`WINGMEN=n make
  run`) makes the first N bots wingmen: they follow you, ask to join
  your side, cover you, hit what is ahead of you, take the pillboxes
  and depots near you, and leave the side only when it holds every
  pillbox and depot. No change to the world or the wire.

- Added: one `--listen` server for several lobbies at once (roadmap
  step 8, its last item). Give `--game` more than once (`GAME="a b"
  make listen`): each lobby has its own thread and UDP port (`--port`
  and the ports after it) and its files in DIR/NAME; one status page
  lists every lobby, and `status.json` gains `games`, with one lobby's
  fields still at the top as before. `LISTEN=near LOBBIES=2 make
  lobby-test` proves it: two matches hosted at once, both replays
  matching.

- Added: a late arrival takes a bot's seat (roadmap step 8). A peer
  that finds a running match through the introducer, while a bot's
  seat with a live tank is free, is answered, seated in it under its
  own name and welcomed mid-match; the world is told by order
  (`Command::TakeSeat`), so replays and the world hash carry it, and
  everyone hears it on the newswire. With no seat free a late group
  elects its own host as before. `LATE=1 make lobby-test` proves it,
  with a `--listen` server too (`LISTEN=near LATE=1`), its replay
  matching.

- Added: the rematch over a network. After a networked match, or a
  lost connection, Enter takes the window back to the lobby on the
  same port; it looks for peers for twenty seconds, so the others meet
  it there as they press Enter too. The rounds-won tally counts
  networked rounds. `make rematch-test` proves it with the window and
  two headless hosts.
- Fixed: a finished networked match kept its MATCH OVER heading
  instead of "connection lost" when the host left; tank speak's lines
  read right with the player's own name, "you".

- Added: Key Setup (SPEC UI-18 to UI-21). K opens it over the map;
  every key the window gives can be rebound, one key an action
  (swapped when taken), Alt, Print Screen and Pause never; the
  bindings are kept in `~/.config/treadcore/keys` or `--keys FILE`,
  by key position so they hold on any layout. The orders column and
  its hints show the keys as bound.

- Added: chat to the tanks nearby and to chosen players (the rest of
  SPEC UI-22). Tab steps all, team, nearby (fifteen tiles of your
  tank); a line starting `@name` (or `@p3`) goes to those players
  only; the prompt says who will hear it and how many. `ChatTo` gains
  `Nearby` and `Players(mask)` (wire version 13, still unreleased).
- Added: a bot with the scrap for what its structures want drives to
  them to build (roadmap step 10k), and holds there while its squad
  works; a structure with no room beside it is remembered.

- Added: the second match (PLAYABILITY item 10). On a solo match-over
  screen Enter plays again: the same island and seats, the bots
  reseeded each round; the screen tallies rounds won.
- Added: bots that run an economy (roadmap step 10j): a power plant for
  a dark structure of theirs, then the jammer once hit, then a
  workshop; scrap felled for any of them and for mending a wreck; a
  working workshop is a resupply stop for armour and the bay.

- Added: tank speak (`treadcore_sim::lingo`). The players are geekers
  (the roster's header, the window's default name); the newswire says
  kills, launches, lost flies, links lost, jamming, intercepts, shots
  and the end of a match several ways, in a cyberpunk, l33t voice with
  FPV pilots' words for the drones (whoop, toothpick, send it, punch
  out, jello, failsafed, pancaked) and some real tank crews' (Power!,
  bite the dust, pigiron); a refusal's tag gets a quip; bots storming
  a depot call it in tank speak. Picked by the event's own number,
  never the world's Rng: matches and the balance proofs read the same.
  The lexicon is in HOW-TO-PLAY 6c. Slang aimed at people is left out.

- Added: the helpdesk's names for a problem the player can fix. Every
  refused order is tagged on the newswire (60 CM for aiming, PICNIC
  for the world working as made, PEBKAC for what the lamps already
  said, LOOSE NUT for the wrong target, KB-TO-CHAIR for talk and
  treaties), spelled out in the scrollback; a command-line mistake on
  the server or the window is an ID-10-T that says nothing is broken;
  the window warns of a flag it does not know instead of ignoring it.
  The tag is the joke and the line after it says what is wrong with
  the order, never who. A structure's power back after a DARK alert,
  and a dropped seat back in the host's log, are told "We're back up."
  and the like.

- Added: industry (roadmap step 10). Pads (`_`), the foundations a
  structure is built on, and five structures: the power plant, which
  powers its side's structures within 18 tiles; the refinery (half
  the fuel for the side, a fuel and shell stop beside it); the data
  centre (a blind shell near it scatters half as far, and hostile
  drones within 30 tiles are seen); the workshop (repair at twice a
  depot's rate, the bay restocked longshots first, pillboxes and
  structures near it mended); the jammer (no drone within 7 tiles
  keeps its link, its own side's included). Taken, mended and built by
  the squad; 120 health, a wreck at zero; every standing structure
  scores its owner two points every ten seconds.
- Added: the longshot, a long-range one-way drone: two in the bay, sent
  at any point on the map with `X`, three tiles a second, no recall,
  80 damage to a structure where it lands, 25 to a tank. Interceptors
  and pillboxes shoot it down; a jammer pushes it off its point.
- Added: concrete (`H`), a wall a splash only cracks: Shift-6, or
  Shift-click in wall mode, for two material.
- Added: the commander's picture. A minimap in the bottom strip (click
  or drag to move the view); alerts on the newswire, with the alarm,
  when a structure, pillbox or depot of your side is hit, wrecked,
  taken or goes dark; `J` jumps to the newest. The works mode (`0`,
  `B` for the kind) builds, takes and mends structures. Structures are
  drawn each its own shape, labelled with health, DARK without power.
- Added: `--industry` (`INDUSTRY=1`) gives a map with no structures an
  economy; `maps/industry.map`; `structure x y kind [owner]` lines in
  map files.
- Added: world maps, drawn by `scripts/worldmap.py` (`make
  worldmaps`): `maps/world/atlantic.map` (the Americas to Arabia, the
  Pan-American Highway from Prudhoe Bay to Ushuaia with the Darien Gap
  in it, Europe's trunk roads, sixteen cities, industry where it is)
  and `maps/world/europe.map`.
- Added: bots that know an economy: a RuleBot takes structures in
  reach and sends a longshot at the nearest hostile structure, a power
  plant first, every half minute; a Defender takes and mends its own.
  On the generated island nothing changes: the balance proofs read the
  same to the digit.
- Added: a RuleBot whose side has had a structure hit builds a jammer
  on a free pad beside a structure of its own, felling forest for the
  material; it launches nothing inside its own side's jammer and drives
  out of one when it is free to. Observations carry `pads`, the free
  pad tiles near the tank, for a brain with no map.
- Added: rookies (PLAYABILITY item 9). `--rookies N` on the window and
  the server, `ROOKIES=n` on make: the first n bots play the RuleBot's
  doctrine badly (no drones for 90 seconds, one interceptor after, a
  loose aim with no lead, no storming), for a newcomer's first match.
- Changed: wire version 13. The match report gains an economy line.
- Fixed: a drone launched with a task that does not suit it goes back
  in the bay.
- Added: proposals in the window (PLAYABILITY item 7). P opens a list
  of everyone not on your side: pick a player and Join, Recruit, Swap
  or Merge, see beforehand whether the world would refuse it and why,
  Enter proposes, L leaves your side. A proposal to you or your side
  stands on the newswire with its seconds: Y accepts, N declines. The
  newswire tells proposals and their ends by name.
- Added: a RuleBot answers a person's proposal, yes unless that person
  has hurt it; bot-only matches are unchanged.
- Changed: the three-way rule is a pair of public functions in the sim
  (`proposal_outcome`, `three_way`), used by the world and by
  `Observation::would_refuse`.
- Fixed: a key typed into a chat line in the frame its Esc closed it
  no longer reaches the proposal keys; the key hint fits a 740 px
  window.
- Added: chat (PLAYABILITY item 6). Enter opens a line on the newswire
  to everyone, Shift-Enter to your team, Tab switches while typing,
  Enter sends, Esc puts it away; the keys are the line's while it is
  open. A message stays on the wire eight seconds under the sender's
  name, with a tone. The world takes one message a second from each
  player (SPEC UI-23), cuts it to 120 characters and strips control
  characters.
- Changed: Return no longer returns the view to the tank (Enter is
  chat's); F does, as before.
- Changed: the sound is FM. The weapons and the metal are two-operator
  FM: the gun's direct fire is a gunshot (an inharmonic crack, its
  index falling in milliseconds, over a punch from 1 kHz to the bass),
  the indirect gun a cannon blast, a direct hit on a tank a struck bell
  of armour, a strike drone a missile's rising whoosh. Thirty-two
  cues, three takes of each. New: strike, ricochet, rocket, rocket hit,
  crash, link static, a pillbox set down, a notice; and the machine
  around the player: the breech after each of its shots (doo, cha,
  klank), the ammunition loader at a depot, a clock while it takes a
  depot or a pillbox, BEEEEEP when it fires with no shells, a beating
  alarm when its armour falls under a third, and its tracks squealing
  and rumbling as it moves.
- Changed: the sound engine runs at 88.2 kHz and is band-limited to
  44.1 kHz, and every FM index is held under Carson's bound, so no
  sideband folds back as noise; the mix has 6 dB of headroom and a
  crowded mixer takes each new voice a little quieter.
- Changed: every sound has a place and the whole map is heard. On
  screen is full strength, panned across the screen; off screen fades
  with the distance past the edge to a quiet floor, never silence, and
  far sounds are low-passed. A sound the fog hides the place of plays
  as background from nowhere. The interface tones of a capture, a loss
  and the swarm are placed where they happened.
- Added: sound (roadmap step 9). Eighteen cues synthesised in the
  window, no sound files: the gun, shells landing, drones, hits, a tank
  destroyed, pillboxes, mines, building, and short interface tones.
  Each is placed where its event happened and panned from the middle of
  the view; a hostile shell whistles where it will land. `M` mutes,
  `--mute` starts muted, `make sounds` writes them all as WAV files.
  The window now links ALSA through Bevy's audio on Linux.
- Added: the server as a service (roadmap step 8). `treadcore-server
  --listen DIR`, `make listen`: a dedicated host in one lobby, match
  after match. It takes no seat, the election prefers it within 10 ms
  of the best seat and leaves it out altogether otherwise, and it
  writes every hosted match's trace and replay to DIR. A status page on
  plain HTTP (`/`, `/status.json`). With an introducer it is listed as a
  server. `LISTEN=near|far make lobby-test` proves the election both
  ways.
- Changed: wire version 12. `Beacon`, `Hello`, `Register` and the
  introducer's `Introduced` carry a `server` flag; version 11 peers and
  version 12 peers ignore each other.
- Fixed: `make lobby-test`'s counts survive a zero: a grep that finds
  nothing no longer ends the script under pipefail before its verdict.

## 0.1.0-rc.6 (2026-10-13): the sixth candidate

- Fixed: the MSI. rc.5's run reported WiX's error in public (WIX0367: two
  unversioned files in one component with a generated id); the guide
  and the licence are now a component each. Everything else in rc.5's
  run passed: seven .deb systems, three .rpm, macOS, Nix.

## 0.1.0-rc.5 (2026-10-13): the fifth candidate, tagged, not published


What rc.4 was meant to be. The tag `v0.1.0-rc.4` was pushed from a tree
whose release commit had not been made, so it names 0.1.0-rc.3's code;
the workflow's version check refused it and nothing was built. The
pre-push hook now refuses a tag pushed from a tree with uncommitted
changes, and every script reads the version with one awk that stops at
the first match: the old `sed | head -1` could die of SIGPIPE under
pipefail, silently, which is how the release commit was skipped. rc.5
carries:


- Fixed: the Windows build, which failed in rc.3's new MSI step: WiX is
  called by its installed path, and a build error is published as an
  annotation.
- Fixed: the release frames are captured from the game window itself
  (grim -T), not a region of the screen; rc.3's first frames showed a
  terminal that sat over the window. Any frame that is not the window
  stops the shoot.
- Changed: index.html, short: what it is, a card per system with its
  command and what it installs, four pictures, how to build (with the
  links to the README and BUILD.md the site lacked), the build grid.
  The paper is now paper.html.

## 0.1.0-rc.3 (2026-10-13): the third candidate, tagged, not published

Its run: every .deb and .rpm installed and played on its systems; the
Windows build failed; no draft.

- New: a Windows installer, `treadcore-<version>-windows-x86_64.msi`
  (WiX, `packaging/windows/treadcore.wxs`): Program Files, a Start Menu
  shortcut, the folder on PATH; tested in the workflow by a silent
  install, a bot match from the installed copy, and an uninstall.
- Fixed: the .rpm, which Fedora 41 and 42 both refused to install in
  0.1.0-rc.2's tests (rc.2 was tagged, never published). It is now made
  with RPM's own dependency generator (rpm-build in the build
  container, `auto-req = "auto"`).
- New: every package test in the workflow prints its failing output as
  a public annotation, so a failure can be read without a login; the
  ARM64 .deb (Debian 12, Ubuntu 24.04) and AArch64 .rpm (Fedora 42) are
  tested on ARM machines.
- New: `make builds` (`scripts/builds.sh`) writes `builds.json` and the
  build grid on index.html, and the links on download.html, from the
  published release: three API calls at most, by the maintainer, never
  by a visitor. download.html no longer calls GitHub's API.

## 0.1.0-rc.2 (2026-10-13): the second candidate, tagged, not published

Its packages' tests found the .rpm broken on Fedora; the release gate
held, no draft was made, and 0.1.0-rc.3 carries the fix.

Journal: [0019](journal/0019-2026-10-13-the-first-minute.txt).

- Changed (brain): the first minute (PLAYABILITY.md item 1). For 60
  seconds the rule bot and the Defender hold fire, and hold strikes,
  at a hostile tank that has never moved (its fuel is still full) and
  never fired. A newcomer sitting at the start is not shelled; bots,
  which drive off their starts at once, are not spared. New test
  `a_newcomer_gets_a_first_minute`: before, the sitting tank was first
  hit at 29.5 s; after, never inside the minute. The squad proof is
  unchanged over its twenty seeds.
- Changed: the lobby's first-minute help says so.
- New: `make shoot` shoots the network on one machine (an introducer,
  a headless host, a bot seat, the window as a client with 30 ms of
  lag): the screen before the match, a networked frame, and the top
  strip's network line. The player's guide shows them in section 10.
- New: builds and runs on macOS (Apple silicon): `make configure`
  knows Darwin (Cocoa, Metal, CoreAudio, sysctl for memory) and
  `make lobby-test` reads BSD `wc`.
- New: `make install` and `make uninstall` (`scripts/install.sh`): the
  release binaries into `PREFIX/bin`, `~/.local` by default; on macOS
  also an ad-hoc signed `~/Applications/Treadcore.app`.
- New: `make quad` (`scripts/quad.sh`): four windows on one machine,
  networked through the LAN lobby over loopback, tiled two by two;
  `CLIENTS=n`, `MATCH_SECONDS=n`. The window takes `--window X,Y,W,H`.

- New: `make deps` (`scripts/deps.sh`, `scripts/platform.sh`): the
  window's system packages for Copal, Alpine, Arch, Omarchy, Debian,
  Ubuntu, Gentoo and Fedora, printed, or run with `APPLY=1`.
  `make configure` and `make install` print the same line when
  something is missing.
- Changed: `make install` on Linux adds a desktop entry and an icon
  (drawn in rectangles), so Treadcore is in the application menu;
  on musl (Copal, Alpine) the window goes behind a wrapper that
  preloads bigstack.so. Tested on Copal: in the Super+Space menu,
  launched from it, uninstalled cleanly.
- Changed: the window's app id is `org.treadcore.treadcore`, matching
  the desktop entry, so taskbars give it its name and icon.
- New: packages. `.github/workflows/packages.yml` builds, on a release
  tag, a .deb (Debian, Ubuntu, Linux Mint; x86-64 and ARM64), a .rpm
  (Fedora; x86-64 and AArch64), a Windows zip and a macOS zip, onto a
  draft GitHub release with SHA256SUMS. The packages carry the window's
  runtime dependencies (Wayland, xkbcommon, the Vulkan loader, Mesa's
  Vulkan drivers; the linked ones read from the binary); Rust is never
  one of them. Metadata in `source/treadcore-client/Cargo.toml`; the
  desktop entry and icon in `packaging/`, shared with `make install`.
- New: `make hooks` turns on `.githooks/pre-push`: pushing a `v*` tag
  runs `make release` first and is refused if it fails or the tag is
  not the version.
- New: a Nix flake (`nix run github:treadcore/treadcore`); untested.
- New: `download.html`, a system selector with a badge and a panel for
  each of Debian, Ubuntu, Linux Mint, Fedora, NixOS, Windows, macOS,
  Copal, Alpine, Arch, Omarchy, Gentoo and cargo, its links read from
  the newest published release; and the same platforms on index.html.
- New: `make deps` knows Linux Mint, NixOS and Windows (Git Bash or
  MSYS2); the default player name falls back to `USERNAME` on Windows.

## 0.1.0-rc.1 (2026-10-13): the first candidate

Journal: [0018](journal/0018-2026-10-13-first-candidate.txt). Offered
for playtesting under `documentation/RELEASE.md` section 4; the
steward's own playtest on one machine stands in for the three
playtesters until there are three.

- New: the release procedure, `make release` (`scripts/release.sh`,
  RELEASE.md section 6): the version agreed everywhere and in this
  file, tests green, a build with no warnings, the release frames
  taken for the version and every one shown by the guide or the site,
  the documents naming the guide, the crates packaging. `make shoot`
  (`scripts/shoot.sh`) takes the frames: the window driven through
  the same match every time, cropped to what each shows,
  `reports/images/guide-*.png` with a manifest.
- New: the player's guide, `documentation/HOW-TO-PLAY.md`, with the
  frames: the screen, driving and firing, the drones, the builder's
  five modes, pillboxes and depots, the stock, the events, every key,
  the network.
- New: the five crates on crates.io (`treadcore-sim`, `-proto`,
  `-net`, `-brain`, `-client`, the window installed as `treadcore`);
  `cargo install treadcore-client`. README and the site say so and
  point at treadcore.org.
- New: treadcore.org made over: an overview in the style of the
  author's other project pages, with the guide's frames, and below
  it a paper in the IEEE form: the rules with their numbers, a
  balance review from eight traced matches on the candidate, the
  network's test record, the game against the prior art, what is
  real and what is not, fourteen references.
- Everything under 2026-10-13 below is in this candidate.

## 2026-10-13, later still: sub-cubes, damage that chips, cubes that roll off

Journal: [0017](journal/0017-2026-10-13-sub-cubes.txt).

- Changed: the models' cubes are a tenth of a tile (the things that
  move) and a quarter (the ground's blocks and pits), with hidden top
  and side faces culled: a tank is 160 cubes with treads, a
  superstructure, a turret and a barrel; a pillbox a four-step
  pyramid; a depot a plate with corner posts; a wall four by four by
  three on its tile; a crater four by four, two deep, its far wall in
  shadow. The hull is Bolo's olive, the turret the seat's colour.
- New: damage chips a model (`Model::chip`): the tank's turret as its
  armor goes, in tenths; the pillbox's gun and top steps as its
  health goes, in eighths.
- New: debris. Every impact throws loose cubes that fly, bounce,
  roll to a stop, spin and fade: earth from the ground, tan from a
  wall or pillbox, olive from a hull, the seat's colour from a
  destroyed tank (`View.impacts`, `render::debris`).
- Changed: the frame's boxes are bevelled, sunken for the insets and
  the map, raised for the order tiles; the bezel is lighter. Solo
  play's map now takes craters and walls from events, as a client's
  always did (it never showed a crater before). Fog dims less.
- Fixed: the newswire reported any visible tank's hit as "tank hit";
  it now says whose.
- Changed: no checker. Every tile is one solid block of one colour,
  sampled from WinBolo's sprite for that terrain (grass, forest,
  road, water, the building's tan and brown). Craters are brown, not
  black.
- Changed: the orders column is the prior art's builder column (SPEC
  BLD-01, UI-02): five modes in the spec's order (4 farm, 5 road, 6
  wall, 7 pillbox, 8 mine), each a picture tile of the ground with
  the thing on it, a lamp beside it lit red for the one armed; the
  key or a click on the tile arms it, the next click on the map gives
  it, and the mode drops as the squad goes. In pillbox mode a click
  on a pillbox repairs it, or captures a dead or neutral one; open
  ground builds one. 9 brings the squad home. The tank's and drones'
  rows have no lamp; a key dims when its order cannot be given.
- Changed (sim): any pillbox can be repaired, whoever owns it, and
  repair never changes the owner (SPEC BLD-05, BLD-06, as adopted);
  "not our pillbox" is no longer a refusal.
- Changed: the drones' rows wear the same picture tiles as the
  builder's, so the column is one thing; the mouse's drive and fire
  have no row, the foot of the column names them. While a mode is
  armed the square under the cursor is outlined in the colour of
  what the squad would make there; Esc drops the mode (and quits only
  with none armed).
- New: a hit you can feel (PLAYABILITY.md item 3): the map's frame
  flashes red and the camera shakes for a third of a second when
  your tank is hit; the top strip is red while UNDER FIRE.
- Changed: a refused order stays on the newswire for four seconds or
  until the next order, instead of passing in a frame.
- New: a click on a pillbox lamp jumps the view to that pillbox
  (SPEC UI-10); in pillbox view the arrow keys jump to the nearest
  pillbox in that direction (UI-11).
- Changed: seat 0's colour is a violet blue, off the river's cyan.
- New: `make token` stores the crates.io API token for cargo
  (`scripts/publish.sh`); `make publish` publishes the five crates in
  dependency order, `DRY=1` packages and verifies only. The manifests
  carry what crates.io wants (versions on the path dependencies,
  readme, keywords, categories; treadcore-warm never published).
- Fixed: `make build` is warning-free (two unread fields and an
  unused session state, removed).

## 2026-10-13, late: Bolo's colours, and assets out of cubes

Journal: [0016](journal/0016-2026-10-13-cubes.txt).

- Changed: the ground is Bolo's palette (`render.rs`): dark green
  grass, bright checkered forest, grey road with its dashes, cyan
  water, tan bridges and walls, brown rubble.
- New: `voxel.rs`. Every thing on the map is a model of cubes on a
  grid of fifths of a tile, drawn in an oblique projection with
  sprites: a top face lit by its height, a shaded side face below.
  The tank (hull, tracks, turret, barrel), the pillbox (a tan pyramid
  with an orange gun, the gun gone under half health, a dark slab
  when dead), the depot (a tan plate with the owner's mark), the
  drones, shells, mines, the squad, and walls as tan blocks raised
  and dropped as the terrain changes. Models are rebuilt only when
  their key changes. No images; two rectangles a cube.
- New: two frames in the gallery (playtest-15, 16).

## 2026-10-13, night: the bezel: the frame in WinBolo's look

Journal: [0015](journal/0015-2026-10-13-the-bezel.txt).

- Changed: the frame wears the prior art's look (`INTERFACE.md`
  section 7), drawn with UI nodes and no images: a grey bezel of
  inset black boxes; a lamp per tank, per pillbox and per depot in
  the owner's colour (mine in my seat colour, green ally, red hostile,
  dim neutral, dark dead); shells as red squares, the bay by kind, the
  squad as four lamps that go amber when out, material as eight,
  armor and fuel as bars; a pictogram tile and a lamp beside every
  order, red when it can be given; kills and deaths; the newswire in
  a 22 px face across the bottom. The top strip holds two lines.
- New: `View.kills` and `View.deaths`, counted from `TankDestroyed`.
- Changed: the gallery's frame pictures are the bezel (playtest-10 to
  13), plus the scrollback (14).

## 2026-10-13, evening: the frame (the GUI redesign), and the playability review

Journal: [0014](journal/0014-2026-10-13-the-frame.txt).

- Changed: the window's HUD is the prior art's frame
  (`INTERFACE.md`, SPEC section 13): a top strip (who, when, NET), an
  orders column on the left with every key, its count or cost and a
  dash when it cannot be given, the map in the middle never covered,
  the roster with one lamp per pillbox and per depot on the right,
  and shells, bay, squad and material as lamps, armor and fuel bars,
  the air table and a two-line newswire along the bottom. The
  controls overlay is gone; Tab shows a forty-line events scrollback
  over the map. Lamps are ASCII: `#` lit, `.` dark; `#` mine, `+`
  ally, `o` neutral, `X` hostile, `.` dead.
- New: the lobby screen (the peers found with their measured round
  trips, how the host is chosen, the first minute in six lines) and
  the match-over screen (the roster by score, Esc to leave).
- New: the gun's envelope lights while the right button is held and
  the shot goes on release; the direct-fire ring with it.
- New: the pillbox view (SPEC UI-10 to UI-13, backlog 12): the
  apostrophe steps through the player's and allies' pillboxes, Return
  or F returns to the tank. The camera follows the tank into the
  middle of the map area, not the window; clicks on a panel are not
  orders.
- New: `--seconds N` on the window. Events after a rejoin are stamped
  with the last tick seen, not 0.
- New: `documentation/PLAYABILITY.md`, the review: fourteen items
  between this build and a playable game, ranked and costed; Bolo's
  interface elements checked off; what the frame still wants.
- New: four frames of the frame in the gallery (playtest-10 to 13).

## 2026-10-13, later: the first playtest build, with pictures

Journal: [0013](journal/0013-2026-10-13-first-playtest.txt). Report:
[LAB-REPORT-PLAYTEST.md](documentation/LAB-REPORT-PLAYTEST.md).

- New: the window runs in the VM. `mesa-vulkan-swrast` (lavapipe) is
  the driver; copal's full monty installs it now. `scripts/bigstack.c`,
  built by `make configure` on a musl machine and preloaded by `make
  run` and `make lan`, raises the default thread stack to 8 MiB:
  lavapipe compiles shaders with LLVM on its own thread, musl gives it
  128 KiB, and every Bevy binary segfaulted in
  `llvm::AsmPrinter::doFinalization` before its first frame.
  `configure` reports the Vulkan drivers it finds, and `PRELOAD` in
  config.env.
- Fixed: the HUD's rtt was heartbeat age (200 to 450 ms on a 60 ms
  link): the client echoed the host tick from the last heartbeat; it
  now echoes the newest snapshot tick, and reads 50 to 150 ms.
- New: nine frames in `reports/images/`, a gallery on the front page,
  the report link under Reports.
- New: `documentation/INTERFACE.md`, the layout study: Bolo's mode
  column, lamp grids, pillbox strip and newswire; the garage's
  between-screens; the HUD graphics' centred rings; and treadcore's
  layout drawn from them, in five steps.
- Found, not fixed: the two top HUD panels overlap below about
  700 px; the controls overlay covers the map until Tab; the event
  log stamps tick 0 after a rejoin until the first snapshot; a seam
  at the NET panel's edge under Xwayland at a guessed 1.083 scale; a
  standing human dies to fifteen bots in about thirty seconds.

## 2026-10-13, night: across the Internet (roadmap step 7)

Journal: [0012](journal/0012-2026-10-13-across-the-internet.txt).

- New: the introducer (`treadcore-net/src/introducer.rs`;
  `treadcore-server --introduce`, `make introduce`): a UDP rendezvous
  service that hosts nothing and keeps a registration thirty seconds.
  Peers in one lobby name register (`--introducer HOST:PORT`;
  `--no-beacon` keeps a peer off the LAN broadcast) and are told each
  other's public and local addresses; the lobby greets both until one
  answers (the hole punch) and takes the answering address. The
  election and the relay are untouched: a pair that never gets
  through is routed around, as on a LAN.
- New: `--identity FILE`, a peer id that survives a restart (made the
  first time; `build/identity` by default from `make lan|host|seat`).
- New: rejoin (backlog item 10, SPEC GAM-12). A host keeps registering
  with `hosting` set; a returning peer with the same identity is told
  its host, pings it until it answers, joins without an election, and
  gets its seat back at the tick the match has reached, with its
  sequence numbers started over. `Join` carries a `session` number,
  random per process, so a restart is told from a repeated Join.
- Changed: wire version 11 (`Register`, `Introduce`, `Introduced`,
  `Join { session }`).
- Changed: `make lobby-test` takes `INTRODUCER=1` (no broadcast, no
  seed address: the four meet through an introducer alone) and
  `REJOIN=1` (a seat that is not hosting killed outright at
  `REJOIN_AFTER` seconds, 20, and started again with its identity
  file; must come back to the same seat at a later tick).
- Changed: the predictor's first snapshot after a Welcome is a
  baseline, not a check: a seat that comes back mid-match has a copy
  built from the scenario's start, and the host's tank is not there.
- Fixed: a terrain change (a shell's crater, a wall) landing under
  ticks the client had already predicted was a mismatch of a fifth of
  a tile, one in some 2 400 checks. The predictor now keeps the last
  snapshot's tank as an anchor and, on such an event, drives the ticks
  ahead again: over the old map up to the event's tick, the new one
  after. `Predictor::events` takes the events' tick.
- Proof: `introducer_brings_two_silent_peers_together`,
  `seat_comes_back_after_a_restart`, `identity_file_round_trips`;
  `make lobby-test` plain, `INTRODUCER=1`, `REJOIN=1` and `LAG=200
  REJOIN=1` all PASS with zero prediction mismatches, the killed seat
  back in its own seat at tick 371 and 376. Not proven: two peers
  behind two home routers, which needs two machines on two networks
  (ROADMAP step 7 says so).
  40 tests (sim 24, proto 3, net 9, brain 4).

## 2026-10-13, evening: client-side prediction (roadmap step 4)

Journal: [0011](journal/0011-2026-10-13-prediction.txt).

- New: a client runs its own tank ahead of the snapshots
  (`treadcore-net/src/predict.rs`). A partial `World` from the
  `Welcome`, kept up from every snapshot and terrain event; hull
  orders (`Move`, `Stop`, the two fire orders) applied the moment they
  are sent; one tick at the tick rate by `World::predict`, which calls
  the same `drive_tank` the host does; every snapshot replaces the
  tank, re-applies the orders after the snapshot's `acked`, and drives
  the ticks ahead again. Bit-for-bit check against the host whenever
  nothing is in flight; mismatches logged and counted.
- Changed: wire version 10. `Snapshot` carries `acked`, the last of
  the client's command seqs the host had applied by that tick.
- New: `TREADCORE_LAG_MS` holds every received datagram that long in
  the transport: the stand-in for `tc netem` where there is no root.
  `make lobby-test LAG=200`, which now also requires zero prediction
  mismatches in the three client seats.
- Changed: the window draws the predicted tank in place of the
  snapshot's own tank; the NET panel shows `predict +N ticks`, orders
  in flight, checks, mismatches, and CORRECTED for two seconds after
  a correction. The headless `--net` seat predicts too, and logs one
  `prediction:` line at the end.
- Proof: `prediction_hides_the_lag_and_matches_the_host` (100 ms each
  way: the predicted tank moves 250 ms before the snapshot shows it,
  53 checks, 0 mismatches); `LAG=200 make lobby-test` (1 602 checks,
  0 mismatches); the 6d replay still matches 60 of 60 after the
  `step_tanks` refactor. 37 tests.

## 2026-10-13, later: the Defender and the storming attacker (roadmap step 6d)

Journal: [0010](journal/0010-2026-10-13-the-defender.txt).

- Changed: `Fortifier` is now `Defender`, roadmap 6d's doctrine: at
  its depot for good; a recon on the approach, moved by `Retask` to
  the depot of whoever shot it last; counter-battery; a sidestep when
  hit; the squad in order: capture the spoke's pillbox, repair its
  own, the moat, a pillbox at the mouth of the walls, the road to it,
  mines across the approach, the walls. It proposes to recruit its
  nearest neighbour once a minute and accepts proposals from anyone
  who has never hurt it: the first bots to use step 6b.
- Changed: the `RuleBot` attacker keeps its recon ten tiles short of
  what it watches (one over a defended depot was intercepted in eight
  seconds), counts blind as low, recalls its squad and moves when hit,
  and storms a depot whose defender has been in view for twice its
  patience: the guns first, then the depot.
- New: `Scenario.advantage` and `--advantage` (the prior art's BRN-12): bot
  seats are told where every tank is.
- Proof, `defenders_hold`: ten seeds of sixteen bots, four attacking:
  every match to the time limit; attackers alive 2 of 40, defenders
  84 of 120; the defenders keep 101 of 120 depots. The kills are 65
  by shell, 7 by mine, 1 by pillbox: the roadmap's "most kills are
  pillbox kills" did not come true, and ROADMAP 6d says why.
  `fortification_pays` widens to 144 000 against 134 206 ticks.
- Tests: 35 pass (sim 23, proto 3, net 5, brain 4).

## 2026-10-13: the backlog, worked through

Journal: [0009](journal/0009-2026-10-13-the-backlog.txt).

- Changed: interceptors engage strikes only (and a recon inside the
  bubble), never other interceptors. Eight seeds of sixteen bots: kills
  by shell to kills by strike go from 7.0 : 5.9 to 8.8 : 2.9 a match,
  interceptors shot down from 32 to 6, survivors from 2.8 to 4.1
  (backlog 1, and 7 with it).
- New: `Command::Retask { drone, task }` (a recon's orbit, a strike's
  target, one drone's recall) and `DroneRetasked`; the rule bot moves
  its recon instead of doubling it (backlog 3).
- New: The prior art's forest concealment, VIS-02 and PIL-07: a tank in a 2 x 2
  block of forest is unseen and untargeted beyond `CONCEAL_RANGE` (3)
  (backlog 13).
- New: `Scenario.game_type`, the prior art's open / tournament / strict as what
  a tank starts with; `--game-type`, `GAMETYPE=` (backlog 11).
- New: the match report at the end of every match and replay: shells
  by kind and spotted, intercept rate, strikes through, kills by cause,
  first kill, pillbox shots, drones lost by reason, refused orders
  (backlog 8).
- Changed: `Heartbeat` carries the host's `rtt_ticks` back; the HUD's
  rtt is that. Wire `VERSION` 9 (backlog 5).
- Changed: the rule bot's led shot is aimed at the tank when the lead
  falls inside minimum range; the Fortifier waits with no material and
  no forest in sight (backlog 4).
- Decided, no change: tank speed stays 4 (backlog 2); depots stay
  time-gated, not shell-gated (backlog 9). Deferred: rejoin to steps 7
  and 8 (10); the pillbox view to the first working window (12);
  pillboxes that never fire to step 6d (14).
- Tests: 34 pass (sim 23, proto 3, net 5, brain 3).

## 2026-10-12, night: proposals and the three-way rule (roadmap step 6b)

Journal: [0008](journal/0008-2026-10-12-proposals.txt).

- New: `Command::Propose { to, terms }` with `Terms::Join | Recruit |
  Swap | Merge`, `Command::Answer { proposal, accept }`; a `Proposal`
  stands 30 s; the three-way rule (`MIN_SIDES` 3, no side over half
  the living tanks) refuses an outcome, with the reason, when proposed
  and when accepted; `Proposed` and `ProposalResolved` events, the
  latter to everyone. The prior art's alliance commands are sugar over this.
- Changed: `AllianceRequest` and its two events gone; the observation
  carries `proposals`. Wire `VERSION` 8.
- Proof: a recorded match in which two sides of eight refuse every
  proposal, a defection among three sides is carried out and shares
  recon, recruiting past half is refused and a swap is allowed. 31
  tests pass; replay and lobby-test pass.

## 2026-10-12, night: teams and alliances (roadmap step 6)


Journal: [0007](journal/0007-2026-10-12-teams-and-alliances.txt).

- New: `Player.team`; `Scenario.teams` and `with_teams(n)`; `--teams N`
  on server and window, `TEAMS=n make debug`. A team shares sensors
  (an ally's recon spots for your gun), depots, pillbox shells and
  the knowledge of its mines; friendly fire is off within a team for
  shells, strikes, splash and mines (`FriendlyFireSpared` in the
  trace; one's own shells still hurt); interceptors and pillboxes
  leave allies alone; a strike at an ally is refused. `Command::Chat {
  text, to: All | Team }`. The match ends when one team stands;
  `MatchOver { winner, team }`.
- New: The prior art's alliance protocol in play, `Command::Alliance(Request |
  Accept | Decline | Leave)`: a request stands 30 s, any member of the
  alliance asked may accept, leave at will; all events. The
  observation carries every player's team and the requests a player
  may answer; the HUD scoreboard is by team.
- Changed: the net host filters events by the world's rule
  (`World::event_visible`) instead of its own copy. Wire `VERSION` 7.
- Proof: a shell fired at what only an ally's recon sees is spotted; a
  shell landing on a teammate does no damage and says so; alliances
  form, admit, refuse, lapse and break; the match ends by team. 30
  tests; four teams of four traced (61 of 79 shells spotted).

## 2026-10-12, night: map files (roadmap step 5), five prior-art maps


Journal: [0006](journal/0006-2026-10-12-map-files.txt).

- New: the map as a text file (`documentation/MAPS.md`): a format
  line, `size`, `seed`, one glyph per tile, then `start`, `depot x y
  [seat|neutral]` and `pillbox x y [seat|neutral]` lines. `Map::to_text`
  and `Map::from_text` (refuses with the line; repairs an owner that is
  no seat); `World::map_as_played`. `Scenario.map: MapSource` (the
  scenario's seed, another seed, or a file's text carried whole).
  Server `--map`, `--save-map`, `--write-map`; `make map`; `MAP=file`
  on debug, run and host; the window's `--map`.
- New: the world is built at the size the map declares. The generator
  still makes 128 x 128 (bit-identical to day one); the prior art's maps are
  256 x 256. `Observation` carries `map_w`, `map_h`; swarm edges, the
  brains and the client take the size from the map.
- New: `Packet::Chunk` for anything too big for a datagram; `Welcome`
  (now the whole `Map`, up to 130 KiB with a map file) goes in pieces
  (backlog item 6 done). Wire `VERSION` 6.
- New: `scripts/priorart2map.py` and `make convert FROM=file` turn a
  prior-art map (its binary format, v1) into a treadcore map; five
  community maps converted into `maps/prior-art/` (provenance and the
  licence question in `vendor/THIRD-PARTY.md`); `maps/island.map` is
  the generated default written out.
- Changed: the RuleBot remembers what the world refused as
  unreachable (a squad's site, a tank's destination) instead of
  re-ordering it four times a second, which island maps exposed.
- Proof: a generated map round-trips through text to the bit;
  sixteen bots play a converted prior-art map, on which pillboxes fire in
  bot play for the first time. 28 tests pass; replay and lobby-test
  pass.

## 2026-10-12, night: material, walls, mines, roads (roadmap step 6c)


Journal: [0005](journal/0005-2026-10-12-material-walls-mines.txt).

- New: `Terrain::Wall` (impassable, blocks a flat shot and a pillbox's
  line, a splash knocks it to rubble); `Mine` entities (hidden until
  found within 1.5 tiles; set off by a tank or squad on the tile, a
  shell's splash or an orthogonal neighbour; 50 to the tank on it, 20
  within 1.5, a squad in it lost, the tile cratered; drones untouched;
  a tank's own mines never hurt it); `Build { Wall | Road | Mine |
  Pillbox }` and `Harvest { at }` (forest to grass, +4 material);
  roads become bridges over water; material cap 8; pillboxes hold 8
  shells for their owner's tank. `Event::TerrainChanged`; the
  Observation carries known mines and the eight nearest forest tiles.
  Routes avoid known hostile mines. Keys 7/8/9/0.
- New: `Fortifier`, the minimal defender (harvest, a gateless mine
  ring at radius 3 round the home depot, a wall ring at 4 with a gate
  toward the centre; learns from its own refusals). `--defenders N`,
  `DEFENDERS=n make debug`.
- Changed: wire `VERSION` 5.
- Proof (`fortification_pays`, twenty seeds, scripted attacker): the
  fortified defender survived to tick 137 664 in total against
  94 885 bare, lived 18:5, killed the attacker 20:12, built 431:0.
  24 tests pass; replay 36/36; lobby-test passes.

## 2026-10-12, night: the Pillboys (roadmap step 3)


Journal: [0004](journal/0004-2026-10-12-the-pillboys.txt).

- New: the squad. `Pillboys` on every tank (strength 4, material 2,
  aboard or out); `Command::Deploy { task }` with `Capture`, `Repair`,
  `Build { Pillbox, at }` and `Return`; the squad walks the world's
  route at 1.5 tiles/s, captures in 10 s, repairs in 6 for one
  material, builds in 15 for two, walks home; a shell's splash over it
  costs two men; depots restock men and material. HUD line, map figure,
  keys 4/5/6.
- New: pillboxes. Twenty neutral ones on the map (one per spoke, four
  round the centre); 60 health; a hitscan gun at hostile tanks within
  8 tiles with a clear line, 15 damage every 2 s; an interceptor's odds
  against hostile drones within 4; dead at zero, never removed;
  capturable when neutral or dead; all visible to everyone. Shells
  damage them; direct fire hits one when no tank is there. Range rings
  and capture progress drawn; narrated in the trace.
- Changed: a tank's route avoids the range of live hostile pillboxes
  (`route_for`, `PILLBOX_AVOID_COST`), because the spoke roads run
  straight through them. Wire `VERSION` 4. The rule bot sends its
  squad from a standing tank with no enemy in view and holds while it
  is out; `RuleBot::without_pillboys` is the control.
- Proof (`pillboys_pay`, twenty seeds, eight bots): squads win 18:2,
  score 20 945 to 13 655, survive 37 to 32, 248 captures. 24 tests
  pass; the replay of the new default match matches 36 checkpoints;
  lobby-test passes.
- Known: in the default bot trace no pillbox fires at a tank (the
  router keeps tanks out); backlog item 14.

## 2026-10-12, night: replay and world hash (roadmap steps 1 and 2)


Journal: [0003](journal/0003-2026-10-12-replay-and-hash.txt).

- New: `World::hash()` (FNV-1a over the serialised world) and
  `Replay` (scenario, every tick's orders, a hash checkpoint every
  100 ticks) in `treadcore-sim`, with `World::run` and
  `Replay::verify`. The host records on request; the server has
  `--record FILE` and `--replay FILE` (exit 1 on a mismatch). `make
  debug` writes `build/logs/replay-STAMP.json`; `make replay` checks
  the newest, or `REPLAY=file`. The host hashes every 1 000 ticks, logs
  it and carries it in `Heartbeat`; clients log it; the HUD shows it.
- Changed: wire `VERSION` 3 (`Heartbeat` gained `hash_tick`, `hash`).
  `make lobby-test` asserts one world hash at tick 1 000 in all four
  logs.
- Measured: a sixteen-bot, 180-second match records to 43 KB and
  replays to all 36 checkpoints in 0.2 s; with the indirect shells
  removed it fails at tick 100. 20 tests pass.

## 2026-10-12, evening: proven, fixed, sixteen


Journal: [0002](journal/0002-2026-10-12-proving-the-world.txt).

- Proven: every crate compiles (the client against Bevy 0.17.3 with
  three dead-code warnings); 16 tests pass; `make debug` traces a match
  with indirect fire, recon and the bubble all in it; `make lobby-test`
  passes with a real assertion (one host, three clients welcomed, all
  three saw the result). The window does not open in the development
  VM: no Vulkan driver (`BUILD.md`, "The window in this VM").
- Fixed: `net` UDP receive buffer 2 KiB -> 64 KiB (the 16.5 KiB `Welcome`
  was truncated and no client ever joined; the loopback test found it).
  `sim` `Move` now routes with the map's A*, the tank follows the
  waypoints, brakes to pivot at corners, and re-plans if blocked (a bot
  sat at the river bank for a whole match). `build` `SECONDS` renamed
  `GAME_SECONDS` (bash's own counter made the match as long as the
  build took, plus 180). `server` reads the final snapshot before
  judging a lost connection; the host waits a second before `Leave`.
- Changed: sixteen seats (`MAX_PLAYERS` 16): sixteen starts round the
  edge, a depot each, a ring road; `--players N` and `PLAYERS=n`;
  sixteen bots is the default. The wire is `VERSION` 2 (`Tank` gained
  `route`). The rule bot's doctrine rewritten: firing position short of
  the objective, recon over it from a standing tank, indirect fire with
  lead, patience drawn per bot, advance to capture.
- New: `GOVERNANCE.md` (roles, application, entry, proposal,
  agreement, review, removal, resignation, ceremony, recognition,
  assessment, award, annual processes; the organization on GitHub),
  `RELEASE.md` (the implementation cycle, versions, release monuments,
  candidates confirmed by playtest), `CONTRIBUTING.md`,
  `CONTRIBUTORS.md`, `.github/` templates and `CODEOWNERS`.
  `SPEC-PRIOR-ART-1.14.md`: the clean-room functional specification of
  the prior art, version 1.14 (256 requirements), with a preface on what treadcore
  adopts, adapts and leaves; the roadmap's prior-art-derived steps and
  backlog cite its IDs.
  `LAB-REPORT-PILLBOYS.md`: the Pillboy requirement checked against
  the prior art's builder; `DESIGN.md` gained the Pillboys decision; `CONCEPT.md`

  section 9: sixteen tanks, defenders, alliances, betrayal; `ROADMAP.md`
  gained a backlog and steps 0b, 6b, 6c, 6d.
- Known: the release-profile engine (`make warm`, `make run`) not yet
  built; the window unproven here; two-machine LAN untested; the
  backlog's balance items (interceptors fighting interceptors, tank
  speed, drone retasking) open.

## 2026-10-12: opening day


Journal: [0001](journal/0001-2026-10-12-opening-day.txt).

- New: six-crate Rust workspace. `treadcore-sim` (deterministic world;
  compiled, 6 tests pass), `treadcore-proto`, `treadcore-net` (LAN
  lobby, host election, relay, Host/Client), `treadcore-brain`
  (RuleBot, JsonBrain), `treadcore-server` (headless host, traced bot
  matches), `treadcore-client` (Bevy 0.17 window with telemetry
  overlay), `treadcore-warm` (engine prerequisites).
- New: Makefile over `scripts/` over cargo; `make configure`, `warm`,
  `build`, `headless`, `debug`, `run`, `lan`, `host`, `seat`,
  `lobby-test`, `test`, `schema`.
- New: documentation (concept, design, protocol, research, lab
  report, build), vendor policy and third-party notices, site at the
  root for GitHub Pages, first report.
- Changed: build sized for a 6 GB machine after the engine build ran
  out of memory on bevy_ui (jobs from RAM, trimmed Bevy features, no
  dependency debug info, no release LTO).
- Known: crates above the simulation not yet compiled; `make warm` not
  yet run to completion.

Commits: 17c71d3, 3a26432, 49cd8f9.
