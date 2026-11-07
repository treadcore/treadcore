# Two Client Tests Mended

**Check-in report CI-105.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 6 November 2026, night.
Companion records: `LAB-REPORT-BRAIN.md` section 156; journal 0133.

---

## Abstract

`keys::key_setup_shows_the_pick_and_the_wait` and
`tune::the_page_moves_a_weight_and_puts_it_back` failed on values later
changes moved. Both now read them from the game.

## I. Intent

Paul, asked which housekeeping to do: "Fix 2 old client tests".

## II. Decision log

| Decision | Cause |
| --- | --- |
| key-setup found by name | actions were added after it |
| alpha's expected value from its default and step | promotions move defaults |

## III. Method

`cargo test -p treadcore-client`.

## IV. Results

34 passed, 0 failed.

## V. Discussion

A test of the interface should not pin a number the fleet tunes.

## VI. What is not built

Nothing further.

## VII. Paul's decisions

Fix the two tests.
