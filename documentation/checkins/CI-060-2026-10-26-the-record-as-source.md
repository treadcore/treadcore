# The Record as Source: How treadcore Is Made, the Pages Kept in the Repository, and Guided Training Planned

**Check-in report CI-060.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 26 October 2026.
Companion records: `LAB-REPORT-BRAIN.md` section 98; `PROCESS.md`;
`AI-BRAINZ-PLAN.md` section 9; roadmap backlog 70; `reports/`.

---

## Abstract

The making of treadcore is now documented as part of its MIT source.
`PROCESS.md` describes the pair, how a prompt becomes a commit, where
every conversation is kept, how the brain is designed with weights,
scenario families, the census and held-out reads, the fifteen days,
and what the process caught. The three pages that had existed only as
shared links are reports of the site: the weight ledger (rebuilt as a
static page from its data, with the day's two default changes), the
big-lizard map and the learned tank. Guided training is planned: one
scenario file a behaviour, read by the families for the scripted brain
and by the trainer for the net on the same seeds, and a series of ten
training maps that place situations predictably. The run stays on the
census machine all day.

**Index terms:** open-source process, reproducibility, scenario
families, curriculum, documentation.

---

## I. Introduction

### A. Intent

Make every artifact a report in the repository; describe the process
and the interactions; document the design by census and families;
plan guided training with training maps.

### B. The prompt

> ok. all these artifacts make sure they are reports in the documentation and the process of project createion is part of the MIT open source expieience of developing this. . Describe the process, our interactions. Document our converstaions and deveopment of design using census testing families . We have our first draft going out. I'm making sure all documentation is good. I'm going to leave the census machine running all day. Tomorrow we might get some results. But we need to always be testing on and imporving those coefficients. We want to have our gruided training to reward being training along with our design of our functions. We might need to create a series of training maps to assit and place certain situations in a predictable occurance to test predictabily.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "all these artifacts ... reports in the documentation" | Each shared page a report of the site in `reports/`, linked from `index.html`, `INDEX.md`, `README.md` and `PROCESS.md` | Links only: a link is not under the repository's licence |
| D2 | The ledger dashboard runs on a framework that is not ours to ship | Rebuilt as a static page from the same data, in the site's look; the day's two default changes added | Copy its source: it does not run alone |
| D3 | "Describe the process, our interactions" | `PROCESS.md`: roles, the rules and why, prompt to commit with an example, the record, the method, a timeline from the journal, what the process caught | A transcript: the prompts are already verbatim in the check-ins, and their consequences are what a reader needs |
| D4 | "guided training ... along with our design of our functions" | One scenario file a behaviour: `[expect]` for the families, `[reward]` for the trainer, held-out seeds for both | Separate files for brain and net: two situations would not compare |
| D5 | "a series of training maps ... predictable" | Ten maps in curriculum order, each fixed where it matters and drawn from the seed in stated ranges; maps 4 to 6 next, as LR 97's census asks | Build them now: Paul is reviewing documentation for the release; the plan comes first |
| D6 | "always be testing on and imporving those coefficients" | The run left on all day; the morning read in PROCESS.md section 8 | — |

---

## III. Methods

The ledger page: `weights_history`, `default_changes`, `screen` and
`censuses` from the dashboard's datasets of 26 October (11:02), plus
the git history of `weights.rs` for c61ad61 and 7fc7809; rendered with
inline SVG, no library. Each page checked: its scripts parse; the
ledger page screenshot once (a date label collision fixed). The
timeline in PROCESS.md from the journal's titles; the counts from the
repository (197 commits, 102 journal entries, 57 check-ins, 97 lab
report sections before this one, 9 families).

---

## IV. Results

| Item | Result |
| --- | --- |
| Reports in the repository | 4 pages (opening day, the weight ledger, big-lizard, the learned tank) |
| The ledger page | 110 weights (68 on 18 October), 18 defaults changed, 56 screen reads, 98 censuses |
| PROCESS.md | 10 sections, 2 diagrams |
| AI-BRAINZ-PLAN.md section 9 | the one-file design, 10 training maps |

---

## V. Discussion

The record was already kept; it was not yet one thing a stranger could
pick up. With the process written and every page in the repository,
the release carries its own history: why each number is what it is,
and how to check it.
