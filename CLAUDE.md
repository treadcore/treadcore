# treadcore: notes for an agent

This file carries the working rules from Claude Code sessions, so a new
session on another machine can resume from a clone. The author is Paul
Richeson (he/him). Journals and commits are in his name.

## Where things are

- The brain (the bot AI): `source/treadcore-brain`. Its record is
  `documentation/LAB-REPORT-BRAIN.md`, a numbered section (LR n) for
  each step. Its map is `documentation/BRAIN-MAP.md`.
- The plan: `documentation/ROADMAP.md`, goals then a numbered backlog.
- Measurement: `documentation/CENSUS.md` explains how to run, read and
  resume the census. **To resume work, follow its resume prompt
  (section 4).**
- History: `journal/` (one entry a session, `journal/TEMPLATE.txt`),
  `documentation/checkins/` (one report a check-in), `CHANGELOG.md`.
- How it is made: `documentation/PROCESS.md`. The learned tank:
  `documentation/AI-BRAINZ-PLAN.md`; how it is trained, on CPUs and GPUs,
  and the parity between them: `documentation/LEARNING-DESIGN.md`. The recording
  fleet (pipes, ssh, time; how a net becomes official):
  `documentation/LAB-REPORT-FLEET.md`, `scripts/fleet-record.sh`. A page published as a shared
  link is also kept as a report of the site in `reports/` (MIT, with
  the code).

## How Paul works

- **One step at a time, synchronously.** No parallel agents or
  background fan-out for implementation. A census running in the
  background is fine.
- **Every step is recorded:** a LAB-REPORT section, ROADMAP and its
  backlog item, CHANGELOG, a journal entry, and an IEEE-style check-in
  report (`documentation/checkins/CI-NNN-...md`: abstract; intent;
  Paul's prompt quoted word for word; a decision log tracing each
  decision to its cause; methods in enough detail to repeat; results
  with errors; discussion). Follow the newest of each as the template.
- **Either/or is a scalar.** "Should it do x or y" becomes a weight in
  `weights.rs` that blends the two, defaulting to Paul's preference,
  plus a Caps switch. The census finds the value.
- **Features on by default** (since 20 October). A new behaviour ships
  on, with a starting number from a scenario family or a quick sweep.
  The census is the guard: it must not break anything and may cost
  only a little. Tuning comes after the features are in.
- **The rule before the run.** Write down what result would confirm a
  change before starting its census. Outside two standard errors is
  the bar.
- **A default change runs `make families` too.** The census and the
  brain's tests passed on 5899e8e while a family fell under its floor
  (CI-044, backlog 62).
- **The game is hard for a person.** `SKILL=0.6 make run` is the dial
  Paul liked (LR 67). Leave it unless he asks.

## Things not to do without asking

- Commit or push only when Paul asks. The remote is
  `git@github.com:treadcore/treadcore.git`.
- **A `v*` tag starts GitHub Actions** (the packages workflow). They
  were paused 13 to 26 October; Paul lifted the pause for 0.2.0 ("Tag
  it"). Push a tag or trigger a workflow only when he asks for that
  release. Commits to main start no run.
- **Publish to crates.io at least a day after a tag** (`make publish`
  enforces it). Hand the publish back to Paul.
- A census is one machine's (CENSUS.md section 5): never pool jobs from
  two machines into one result.
