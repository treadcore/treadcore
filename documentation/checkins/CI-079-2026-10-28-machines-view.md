# The Machines View, and whitewolf Joins

**Check-in report CI-079.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion record: `LAB-REPORT-BRAIN.md` section 120; orrery
`tools/machines.py`, `tools/fleet-node.sh`, `src/machines.rs`,
`src/sessions.rs`.

---

## Abstract

orrery takes a fresh Ubuntu install to a fleet node from its window:
- probe it;
- apply its setup recipes, each behind a screen of details and a typed
  name, with root typed by Paul in the machine's Sessions tile;
- join it.

whitewolf (GTX 1080) was set up by the new one-run script and joined.
This Mac is in the view by its hostname.

---

## I. Introduction

### A. Intent

The steps a new machine went through by hand become orrery's process,
safe to repeat and hard to misapply.

### B. The prompts

> i did it. fixed. booted. i also created openssh server at ssh user@whitewolf . i already ssh-copy-id. please apply or help me apply the make cuda script and the orrery , and the cuda install, and the copal-tm with tmux . there is a script. andor add it to orrery. which gets it setup in a similear manner like columbia. This should also have a gpu. Please also add the motd with cpu and gpu snapshots.

> these steps should be made into the process of orrery. like after the media. we need to spruce up the general media createion tool of copal and use orrery as the interface of media creation. in this. we applied an install, following the instructions of the Ubuntu 24.04 with OpenSSH server and user authentication. Which the Orrery then applies scripts and allows idempotent running of the config scripts. Make then difficult to press (with confirm with details), we dont wont to run the wrong scripts. And also, in general the Orrery, do we have the ability to schedule an arbitrary bash job to any of the computers. Would be nice to be able to universally apply, or apply by Apline / Ubuntu year.

> whitewolf is back up, probe it and join it. also before that. launch the new version of orrery.

> in the machines. it should implicitly have This Mac, or whatever it is by hostname.

Paul's answers:
- "Type it in a Sessions tile";
- "As the user; root only in a tile";
- "Machines page: probe, recipes, join".

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | Five scripts run by hand for each machine | `fleet-node.sh` runs them in order, each checking first | A runbook |
| D2 | The motd with CPU and GPU | topnotch, already in `fleet-login.sh`, installed by it | A second banner |
| D3 | Root needs a password | Typed by Paul in the machine's Sessions tile, in a window of its own | A sudoers rule for the scripts (Paul chose the tile) |
| D4 | "difficult to press (with confirm with details)" | A screen with the machine, what it does, files and sha256, command; lit by typing the name | A plain confirm button |
| D5 | Files could differ from what was shown | Sums read back on the machine after copying | Trusting scp |
| D6 | Verbs are an allow-list for the web wall | Recipes and commands only in the operator's window, from the queue's tools | Free text through the verbs |
| D7 | `join` and the power-off rule disagreed | The probe records the exact allowed command; join writes it | `sudo -n poweroff` |
| D8 | "This Mac, or whatever it is by hostname" | The coordinator probed locally, named with its hostname | Leaving it out |

---

## III. Methods

1. **orrery `tools/fleet-node.sh`:**
   - build tools;
   - rustup for `SUDO_USER`;
   - treadcore over HTTPS;
   - `cuda-node.sh` or tmux and copal-tm;
   - `fleet-login.sh --role`;
   - `fleet-poweroff-setup.sh`;
   - the coordinator's two lines.
2. **orrery `tools/cuda-node.sh`:** modprobe, else a reboot note, before
   the check.
3. **orrery `tools/machines.py`:** the registry, `PROBE` and
   `LOCAL_PROBE`, `RECIPES`, `applies` and `applied`, `stage` with sums
   read back, and `join`.
4. **orrery `src/machines.rs`:** the view and its confirmation;
   `sessions.rs` `run()` (Ask::Window); `nav.rs` View::Machines;
   `main.rs`: the status thread, asks on threads, the hand-off,
   `--frame-view machines`.
5. **Checks:**
   - 340 tests;
   - review frames of the page and the confirmation;
   - every machine probed;
   - whitewolf's host key over its address compared with its key by
     name before trusting it.

---

## IV. Results

| | |
| --- | --- |
| whitewolf | Ubuntu 24.04.5, i7-6700K, 62 GB, GTX 1080, driver 580.178.04, CUDA 12.9; every recipe applied |
| CUDA check | compute 6.1, 20 SMs, 7.9 GB; a 1,048,576-float add correct |
| Joined | caps cpu and gpu; power-off `sudo -n /usr/bin/systemctl poweroff`; first build started 15:10 |
| The fleet as probed | calber and boburner without the power-off rule; boburner without tmux; the Alpine machines without the power-off rule |

**Errors.** whitewolf dropped off the network once during its setup run
(the run had finished; it rebooted). Its two ports share one subnet, and
traffic can leave by either.

---

## V. Discussion

A new machine is now three presses in one window, each behind a
confirmation that shows what it will do. The probe also turned up
what the older machines never finished. The command line remains for
anything the window does not do.
