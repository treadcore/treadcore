# Trace 2 at Half the Rates

**Check-in report CI-035.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 21 October 2026.
Companion records: journal 0092; `PROPOSAL-UPGRADES.md` section 9;
CI-034. Branch `proposal/upgrades`; main pushed (`5bc0faf..0240b1a`).

---

## Abstract

Section 7 of the upgrades proposal suggested halving supply's earning
rates; nothing had traced it. The prototype's earning scale became a
field of the economy, set by the trace. Over the same six naval
matches, halved rates give matches of 20.6 minutes (18.0 without
buying, 21.8 at full rates), 108 purchases instead of 195, and the
first purchase near the fourth minute instead of the second; but base
upgrades fall from 49 to 16, and a tank's plating takes 85% of the
spending. Halving the rates is sound only with the plating changed.

**Index terms:** upgrades, supply, trace.

---

## I. Introduction

### A. Intent

Trace section 7's halving, and push main.

### B. The prompt

> yes. run halved trace. push main.

---

## II. Decision Log

| # | Cause | Decision | Not taken, and why |
| --- | --- | --- | --- |
| D1 | "push main" | Pushed `5bc0faf..0240b1a`; the branch not pushed | Pushing the branch: not asked |
| D2 | The scale was a constant | `Economy::scale` (default `SUPPLY_SCALE` 1.0), set by `SUPPLY_SCALE` in the trace | A second constant: the trace could not choose |
| D3 | F: base upgrades 49 to 16 | Sections 7 and 8's questions joined in section 9 and the post | Recommending a number: the discussion's |

---

## III. Methods

`SUPPLY_BUY=1 SUPPLY_SCALE=0.5 cargo test -p treadcore-brain --release
-- --ignored supply_trace --nocapture` on the branch; log
`build/supply/trace-buy1-half.log`. The first purchase by seed and the
winner's share from the purchase lines.

---

## IV. Results

As section 9. Sim tests 56 (the upgrades module's with the new field).

---

## V. Discussion

Halving pushes the first purchase to the fourth minute and keeps
matches longer than today, but money then goes to what is cheap and
lost often. The plating's rule decides whether the base upgrades are
bought at all.

**For Paul:** main is pushed; the branch is local. The post draft
carries section 9.
