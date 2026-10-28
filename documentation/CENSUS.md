# The census: how the brain is measured and tuned

The census is the instrument every brain change since 18 October has
been judged by. This file is its manual: what it runs, how to run it,
how to read it, how to resume one that stopped, and how to move it to a
dedicated machine. The record of what it has found is
`LAB-REPORT-BRAIN.md` (section 14 the fair census, 24 the noise, 71 the
census on every map, 72 this method and the machine for it).

## 1. What it is

The census is a test in the brain crate, ignored by `make test`
because one run takes hours:

    cargo test -p treadcore-brain --release -- --ignored brain_census --nocapture

One **seed** is one starting world. It is played three times, each team
taking the **trial** seat once (the fair census, LR 14). The trial team
plays one brain (a weights file) and the other two teams play the
**baseline** brain. Every bot is a full brain and there are no people.
A seed's reading is the trial team's mean score a seat less the
baseline's. Both brains meet the same starts, so most of the map's
luck cancels in the difference (a paired reading).

Over many seeds this is a Monte Carlo estimate of one number: how much
more a seat scores with the trial brain than with the baseline, with
its standard error. Outside two errors of zero is **outside the
noise**. That is the bar for every claim.

The sim is deterministic on one binary: a seed played twice by the
same binary plays the same match (checked again for `RESUME`, LR 72).

### Its settings (environment)

| Variable | Default | What it sets |
| --- | --- | --- |
| `CENSUS_WEIGHTS` | the defaults | the trial brain's weights file |
| `CENSUS_BASE_WEIGHTS` | the defaults | the baseline brain's weights file |
| `CENSUS_SEEDS`, `CENSUS_SEED0` | 3, 0 | how many seeds, from which index |
| `CENSUS_MAP` | `gen` | a map file, a comma list of them, `gen` (a map from each seed); seed n plays map n mod k |
| `CENSUS_SECS` | 900 | the match's time limit |
| `CENSUS_RESPAWNS` | 0 | lives after the first (the scripts use 3) |
| `CENSUS_FAIR` | 1 | 0: the old census, one trial seat a seed |
| `CENSUS_LEDGER` | 1 | 0: write no ledger lines to `reports/ledger` |
| `CENSUS_ACTS` | 0 | 1: where the trial team's time goes, and what it was doing when it died |
| `CENSUS_LONG_GAME` | the weight's | the score's weight on the long game |
| `CENSUS_DUMP` | none | write the trial brain's full weights to this file |

A **weights file** is `name = value` lines and a `caps` line of
switches (`weights.rs`):

    # the dodge off, everything else the defaults
    caps = baseline, -dodge
    dodge_horizon = 4.0

## 2. Running it: orrery's queue, on the fleet

Since 28 October (LAB-REPORT-BRAIN.md 112 to 116) every census, and
everything made of censuses, runs on the fleet from orrery
(`~/code/orrery`, its guide `docs/fleet-train.md`). The standing queue
is `build/queue/grove`:
- a weight search runs in it day and night (what `make converge` did);
- a run is a button in orrery's Training view, or a command.

```sh
F="python3 $HOME/code/orrery/tools/fleet-queue.py $HOME/code/treadcore/build/queue/grove"
$F new census trial="castle_inside = 8" seeds=110   # is brain A better than brain B (base= for B)
$F new parts                                        # what is each part worth: each off, against all on
$F new switches                                     # what is each switch that is off worth alone
$F new roundrobin                                   # every ordered pair of profiles/*.weights
$F new spsa tune="dodge_horizon dodge_clear"        # values for these weights, a step a census
$F new tune                                         # the older evolutionary tuner (LR 40)
$F new families                                     # every candidate against the families' floors again
$F new promote candidate=@strongest                 # a candidate into weights.rs on a branch; Paul approves
$F runs; $F cancel RID; $F approve RID
```

**What a run does:**
- **Each census is played by one machine**, with every core, by
  `scripts/census.sh`, on the newest commit of `origin/main`.
- **The machine builds that commit itself.**
- **The results come back** to `build/queue/grove/results/`.
- **Each run's table** is in `plan/runs/RID/`.
- **The list of what to measure stays here.** A run's censuses are what
  the script prints with `LIST=1`: `scripts/parts.sh`,
  `scripts/switches.sh` and `scripts/roundrobin.sh`.

**On one machine, without the fleet,** the scripts still run directly:
`TRIAL=a.weights BASE=b.weights scripts/census.sh`, or
`S=scenarios/x.toml make scenario` for one described situation.

`scripts/census.sh` writes into `build/census/STAMP/`:
- `run.env` (its settings);
- `trial.weights` and `base.weights`;
- `census-bin`;
- one log per job (`g-SEED0.log`);
- at the end, `pooled.txt`.

## 3. Reading it

`pooled.txt`, from LR 71:

    every map         +5.94 +/- 0.38 over 176 seeds (outside the noise)
    island           +10.86 +/- 1.48 over  16 seeds (outside the noise)
    fortressisles2    +2.29 +/- 1.15 over  16 seeds (inside the noise)

- The first number is trial less baseline, a seat. A match scores a
  seat about 8 to 16.
- Half a point needs about 200 seeds to show (LR 24). A part worth
  five points shows in 30.
- Write the rule down before the run ("holds if pooled > 2 SE and no
  map past 2 SE the other way"), then read the result against it.
- The per-match lines in the logs carry the **meter**: dodges, kites,
  postures, the squad's time by task (LR 83), the cost of a seat-tick.
  `scripts/rungs.py` reads a switch census's rung seconds by map.
- The score includes **rewards** since LR 84 (direct hits, pillbox
  damage and captures, defences). `CENSUS_REWARDS=0` scores as before,
  for a census to compare with one before LR 84. Use them to explain the number,
  not to replace it.

**The sequential census** (`LOOKS=8`, LR 78 and 79) writes each look
to `sequential.txt` as it plays and its verdict to `verdict.txt`. An
early stop is read again on SEEDS fresh seeds from SEED0 + SEEDS, into
`reread/`, and is outside only if the re-read is outside the same way.
It reads a large effect (z past 5) in a third to a half of the seeds;
on a close call it spends more than the fixed census and reads it
outside less often, so the fixed census stays the default. It does not
resume. `scripts/sequential.py replay DIR...` and `twostage DIR...`
replay finished censuses under it.

A census still running prints no pooled line until it ends. Its
per-match `team N (trial): score` lines can be read early (LR 71 was
read so at 123 seeds). The pooled file is the record.

## 4. Resuming

**In the queue, nothing is resumed by hand.**
- A census lost with its machine goes back on the queue and is played
  again in full on another machine.
- A stopped queue starts again with **Start the queue**, or with
  `fleet-queue.py Q ensure`.
- Every machine's censuses are gathered into `build/ledger/fleet.tsv`
  by the queue's sync, every 30 minutes, or with **Sync now**.

**A census run directly by `scripts/census.sh`** can be finished on the
machine that started it:

    RESUME=build/census/STAMP scripts/census.sh

It reads `run.env` and keeps every job whose log ends in `test result:
ok`. Any other log is set aside as `g-SEED.log.partial`, and that job is
played again in full on the same frozen binary and weights.

### The resume prompt

To pick the work up in a new Claude Code session, on this machine or a
new one, from a fresh clone, paste:

> Read CLAUDE.md and documentation/CENSUS.md, then the newest section
> of documentation/LAB-REPORT-BRAIN.md and the newest journal entry.
> Check the fleet's queue (`python3 ~/code/orrery/tools/fleet-queue.py
> build/queue/grove status` and `... runs`): if its keeper is stopped,
> start it (`... ensure`); read what finished against the rule written
> before it, and any promotion waiting for Paul. Then tell me where the
> roadmap's backlog stands and what you would ask the fleet for next,
> with its cost in matches and hours.

`CLAUDE.md` at the top of the tree carries the working rules a new
session needs, so the clone is enough to resume from.

## 4a. The search: the census as a learner, on the fleet

The search `make converge` ran on one machine (LAB-REPORT-BRAIN.md
section 92) is now the standing work of orrery's queue
(`tools/plans/treadcore.py`, kind "weights"). It works the same way:
1. **The ledger.** Each top-up reads the ledger: the queue's censuses,
   and the fleet's history with the same `weights.rs`.
2. **The fit.** It fits each weight's curve to the ledger
   (`scripts/suggest.py`).
3. **The trials.** It plays the trials the fit suggests:
   - the best point, where a curve's top is past two errors;
   - reads again of trials past 1.5 errors;
   - each weight's half and double not yet read;
   - draws from what is uncertain.
4. **The held-out reads.** A trial past two errors, with no map past two
   below, is read twice more on 400 seeds no step saw.
5. **The families.** A trial that passes both reads is a candidate, and
   its families run next.

It changes no default. A candidate becomes the default only by a
promotion that Paul approves.

`scripts/converge.sh` is kept for one machine without the fleet.
saskatchewan's run (`build/converge/20261026-181011`, 64 generations)
ended there on 28 October. Its records are mirrored into
`build/fleet-sync/saskatchewan/`, and its candidates are among the
queue's families.

## 5. A dedicated machine

A census is many single-threaded matches side by side, each using
about 5 MB. It needs cores and nothing else: no GPU, little memory,
little disk. LR 71 took 3.4 CPU-hours (23 s of CPU a 900 s match, all
eleven maps; naval alone is several times cheaper).

On a fresh Linux machine:

    # a C toolchain and git (Debian: apt-get install build-essential git make python3)
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    git clone git@github.com:treadcore/treadcore.git && cd treadcore
    SEEDS=8 SECS=60 scripts/census.sh       # a short first run: it builds and plays

To join the fleet instead, add the machine to orrery's queue
(`build/queue/grove/config.json`, orrery `docs/fleet-train.md` section 5):
it builds the newest commit itself and takes work.

The brain crate should need only the C toolchain. The game's window
needs the full `make deps`. `JOBS` defaults to `nproc`. Long runs
belong in `tmux` or under `nohup`, so a closed SSH session does not end
them. If one ends anyway, `RESUME` finishes it.

Three rules:

1. **One census, one machine.** x86 and ARM may round floats
   differently, so one seed can play two different matches on the two.
   A paired reading needs both sides played on one binary. Never pool
   jobs from two machines into one result.
2. **Compare within a machine.** Two censuses compared to each other
   (LR 69 against LR 43, say) must both come from one machine, or both
   be read only against their own baselines.
3. **Check the CPU is really yours.** The guest VM gave each job a
   third of a core during LR 71. In `top`, `%st` (steal) above a few
   percent means the host is holding the cores back.

## 5a. More than one machine (planned, LR 103; the fleet's design, LR 104)

A census stays one machine's. The run scales across machines by whole
censuses:

- **One machine, more cores:** nothing to do (`JOBS` is `nproc`).
- **Several machines, one run:**
  - The run writes each generation's trials into a queue folder. A
    worker on each machine (`scripts/census-worker.sh`, to build when
    a second machine arrives) takes the next trial, runs `make census`
    on it with its own frozen binary, and copies the census folder
    back. (Done, as orrery's fleet queue: LAB-REPORT-BRAIN.md 110 to
    116.)
  - `run.env` names the `HOST`, and `ledger.py --merge` joins every
    machine's rows without pooling them.
  - Each held-out read is one census on one machine. A trial's two
    held-out reads may fall on two machines; each is still a complete
    paired reading.
  - Compare within a machine (rule 2 above) when two censuses are read
    against each other rather than against their own baselines.
- **The learned tank:** a school on each machine on its own seeds,
  the best net carried to the others each night (an island model).
  Splitting one generation over a network costs more than it gains.
- **The fleet** (LR 104, section E): saskatchewan coordinates;
  each Copal worker pulls a trial from `build/fleet/<run>/queue/` with
  an atomic `mv` over ssh, plays it on the coordinator's frozen binary
  (copied by hash), returns it to `done/<host>/`, and beats a
  heartbeat; a stale claim goes back to the queue. A machine joins
  only after a reference census matches saskatchewan's to the
  character (`make fleet-join HOST=`).
