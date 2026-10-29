# orrery in treadcore's Brushed Metal

**Check-in report CI-081.** Paul Richeson, treadcore.org; Claude Code
(Claude Opus 5.5) as the pair. 28 October 2026.
Companion record: `LAB-REPORT-BRAIN.md` section 122; orrery `src/draw.rs`,
`src/ui.rs`, `src/nav.rs`.

---

## Abstract

orrery's window now wears treadcore's look:
- brushed steel plates, from the same recipe as treadcore-client's
  `metal.rs`;
- metal keys with lamps and digits;
- black readout wells for the content.

All five views share one header. The look is the default; the flat
palettes remain behind `--light` and `--dark`. The story of the views,
and crucible as grove's second job, are the next steps.

---

## I. Introduction

### A. Intent

Make orrery look like treadcore, first, as the ground the redesigned
progression will stand on.

### B. The prompts

> we need to consider the interface from treadcore . the styling. thr rows and columns of brushed metal. Can we redesign all tabs all sub tabs and rethink about the interface progression. tell a story, help us orchestrate and add comptuers to configure in the orchestration. For example, we want to create a second project to run in cuda. [...]

> Amazing. Do a backflip! [with the report's Next list: the interface redesign; islands across the three cards; the headroom in VII.B]

> and incorporate orrery and crucible into its second job. we want to complement and use our fleet to run to improve the canidates and allow fleet processing of the queue in the grove.

---

## II. Decision Log

| # | Cause | Decision | Not chosen |
| --- | --- | --- | --- |
| D1 | One step at a time | The look first; the story and crucible's queue work next | All three in one change |
| D2 | orrery draws in software, with no textures | treadcore's recipe drawn at the size asked, cached | Slicing a stored texture |
| D3 | "rows and columns of brushed metal" | Steel frame and columns; the content in black readouts | Steel under the text |
| D4 | Text on steel and text in a readout need opposite inks | `engrave` (dark, for steel) beside `ink` (light, for readouts) | One ink |
| D5 | Five views each drew their own header | `nav::header`, shared | Restyling five copies |
| D6 | Features on by default | `METAL` is the default; `--light`, `--dark` kept | An opt-in flag |
| D7 | A cache bounded by entries could hold gigabytes after a resize | Bounded at 12 M pixels | 512 entries |

---

## III. Methods

1. **`draw.rs`:**
   - `Theme.metal` and `Theme.engrave`;
   - `METAL`;
   - `Finish` (Plate, Button, Pressed), `finish_pixels`, the cache;
   - `Canvas::metal` and `Canvas::well`.
2. **`ui.rs`:** in metal, `panel` draws a plate, `button` a capsule
   (pressed while held), and `rail_item` a key; `well` is new.
3. **`nav.rs`:** `header` (frame, name, keys, status strip, body well),
   `width_for`, `HEADER_H` 40.
4. **The views:** Training, Machines, Sessions and Media use
   `nav::header`. Lab's header, rail, bar and status are drawn on the
   steel. Training's pages are keys.
5. **`main.rs`:** `--light`; `METAL` otherwise.
6. **Checks:** `cargo test` (344 pass); frames of every view; installed
   and relaunched; memory read twice, 20 s apart.

---

## IV. Results

| | |
| --- | --- |
| Tests | 344 passed, 1 ignored |
| Orrery.app memory | 389 MB, steady (368 MB before) |
| Frames | all five views; see `reports/images/orrery-metal-training.png`, `orrery-metal-sessions.png` |

**Errors.**
- The first build left the strip's width computed before the theme was
  known, and Lab's fleet name drew over the Sessions key. Fixed:
  `width_for(theme)`.
- Plates drawn twice made a seam in Lab's rail. Fixed: the rail is the
  frame's own steel.
- Sessions' tile titles were pale on steel. Fixed: engraved.

---

## V. Discussion

The look carries over without touching what each page says. The pages
already drew their rows as tiles on a base, and those became readouts
on steel. What the step does not yet do is the story: the views are
still in the order they were built.

**Next:**
1. The views in the order the work goes, with a Projects view.
2. crucible as a second job in grove's queue.
