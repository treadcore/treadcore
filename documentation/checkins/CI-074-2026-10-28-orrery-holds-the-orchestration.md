# Orrery Holds the Orchestration

**Check-in report CI-074.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 113; orrery's
`docs/fleet-train.md`, `tools/plans/treadcore.py` and
`tools/fleet-queue.py`.

---

## Abstract

The fleet's orchestration moved from treadcore to orrery:
- **the plan** (what runs where), the Training view's document, the
  lanes and the counting are now in `orrery/tools/plans/`;
- **the fleet follows `origin/main`:** each machine fetches,
  fast-forwards its own checkout, and builds each commit once;
- **the search reads the fleet's history** where `weights.rs` is
  unchanged: 1,153 censuses;
- **a sync on the keeper's clock** gathers every machine's ledger: 3,622
  censuses in 13.7 s;
- **Shutdown Now:** each machine finishes, reports and powers off, and a
  report says who finished what. Tested on a scratch queue.

saskatchewan's converge run stopped after generation 64 and the machine
joined the standing queue, grove. On the new commit, all 40 candidates
hold every family floor.

---

## I. Introduction

### A. Intent

Make orrery the only place the fleet's training is decided, started,
gathered and stopped, with every machine on the newest pushed commit.

### B. The prompts

> can we stop saskatchewan after this run. and then connect it to the fleet.

> can we stop saskatchewan after this run. and then connect it to the fleet. . make sure to get its data.

> to confirm. that all training of make, make census, make family, and other fleet orchanization is now done, scheduled, and collected by orrery. we want all orchestration coded in orrery, so we don't need to manually collect using manual shell commands, we want a sync updater. we also want the fleet to be able to accept Shutdown Now commands for the fleet . Where we finished, report back, then shut down as they complete.

> yes, go ahead in that order. and make sure at each step. each of the comptuers in the fleet get the updated values for training. i'm sure its already done, but we need to be git pushing and git pulling and using these numbers to always work with the newest. The computer running orrery would be pushing.

> we then test orrery in fleet management mode.

> i unplugged ethernet. sorry fix.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "all orchestration coded in orrery" | orrery holds the plan, view, jobs and count; treadcore keeps the work (census, tests, ledger, fit, trainer) | Moving `ledger.py` and `suggest.py` too: they are the science, also used by a census on one machine |
| D2 | "always work with the newest" | `commit: origin/main`, read after a fetch at each top-up; a build a commit a machine (`c/SHORT/`) | A pinned commit changed by hand |
| D3 | A running job must not change binary under it | A job carries the build it was planned with | Rebuilding in place |
| D4 | "git pulling" on the machines | Each machine fast-forwards its own checkout, only if clean and on main | `git reset --hard` (would lose a person's changes) |
| D5 | Defaults change with a promotion | History rows only where `weights.rs`'s blob is today's, 55+ seeds, 900 s, every map | All rows (mixing defaults); only the queue's own (losing 64 generations) |
| D6 | "a sync updater" | `PLANNER sync` on the keeper's clock (30 min), and Sync now | A cron job outside the queue |
| D7 | A row keeps its binary's hash | Each machine's ledger read on that machine | Copying 4 GB of binaries to the Mac |
| D8 | The Mac's census rows had three host names | `aliases` in config.json, applied at sync | Rewriting run.env files |
| D9 | "Shutdown Now ... report back, then shut down as they complete" | Lane: finish, fetch, sync that host, power off; keeper: report when none is left | Powering everything off at once (results lost) |
| D10 | No machine has passwordless root | `tools/fleet-poweroff-setup.sh`: one rule a machine for only the power-off command, run by Paul | Asking for the password; a broader rule |
| D11 | Two keepers must not share machines | full-1 drained (9 unstarted trials set aside); grove started on the two idle machines, the rest join as full-1 ends | Two queues on one machine |
| D12 | `pkill -f census-bin` killed its own ssh session | `pkill -x census-bin` (process name) | — |
| D13 | Shutdown powers machines off | The button asks twice | One press |

---

## III. Methods

1. **orrery `tools/plans/treadcore.py`.** It starts from `fleet-plan.py`
   and adds:
   - `sha()`, `prep_id()` and `cdir()`;
   - `same_defaults()` and `ledger()` (history);
   - `sync(only)`.
2. **orrery `tools/fleet-queue.py`:**
   - `power_off()` and `shutdown_report()`;
   - the keeper's sync thread, and its `SHUTDOWN` branch;
   - the commands `shutdown-now` and `sync`;
   - `status` with `shutdown` and `sync`.
3. **orrery `src/main.rs`:** `--queue`. **`src/training.rs`:** the Sync
   now and Shut the fleet down buttons, and the sync and shutdown lines.
4. **The Shutdown test.** A scratch queue: `{here: "-"}` and
   `{boburner, poweroff: "true"}`, six `sleep 8` jobs, `shutdown-now`
   after 4 s.
5. **The grove queue.** `build/queue/grove/config.json` and `plan.json`,
   as in orrery's guide section 1. `ensure` was run at 09:35.
6. **saskatchewan.** A watcher waited for `gen65/` to appear, then:
   - stopped converge, then the census by process name;
   - set saskatchewan's caps in grove to `cpu`.

---

## IV. Results

| | |
| --- | --- |
| orrery tests | 335 passed |
| Sync | 13.7 s; 3,622 censuses (saskatchewan 1,139, mac 1,435, columbia 450, calber 438, nasser 151, boburner 9) |
| Builds of ea336a6 | saskatchewan 18.9 s, boburner 16.6 s; saskatchewan's checkout 7d0af54 to ea336a6 |
| History in grove's first fit | 1,153 rows |
| Families on ea336a6 | 40 of 40 hold every floor |
| Shutdown test | `here: t00, left running; boburner: t01, poweroff failed; 4 jobs never started` |
| converge | stopped after generation 64, 09:31:21 |

**Errors and limits:**
- No machine has a power-off rule yet, so a real shutdown would leave
  every machine running and report so.
- Every machine's census binary has a different hash (each toolchain
  builds its own). That is why a census stays one machine's.

---

## V. Discussion

The fleet can now be run without a shell:
- **start it, watch it, gather it, stop it, or shut it down**, all from
  orrery;
- **train on the newest commit** this Mac has pushed;
- **search from everything the fleet has measured** under the same
  defaults.

What is left is the rest of the make targets (step 3), starting a run of
each from orrery (step 4), promotion (step 5), and the Makefile itself
(step 6).
