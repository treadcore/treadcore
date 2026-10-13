# Making treadcore playable: a review

13 October 2026, after the first playtest build (`LAB-REPORT-PLAYTEST.md`)
and the frame that came of it (`INTERFACE.md`). What stands between
this build and a game a person sits down to and plays, in the order it
stands there, with what each item costs. "Playable" here means: a
newcomer survives the first minute, knows what to do in the second,
and wants a third match.

## 1. The verdict

The simulation is a game. The bots fight a real artillery duel with
air cover, the squad takes and fortifies ground, alliances form and
break under a rule that keeps three sides in play, and sixteen seats
run at 20 Hz on a CPU rasteriser. The network is further along than
most hobby games ever get: no server, an election, prediction that
matches bit for bit, a seat that waits for a dropped player. What is
not there yet is the human: the window had its first run today, the
first minute kills the newcomer, and there is no sound, no feel of a
hit, and no way to talk. The list below is that gap. None of it is in
the sim.

## 2. The ranking

| rank | what | why it blocks play | cost |
| --- | --- | --- | --- |
| 1 | **The first minute.** A tank at its start is found by indirect fire inside thirty seconds, three times of three. | The newcomer dies reading the help. | Sim: none. Brain: a bot opening that does not fire at a tank that has not moved or fired for the first 60 s (`RuleBot`, one condition). Scenario: `grace: Tick`, with `--grace`. Half a day with the trace. |
| 2 | **Reading the screen.** Done today: the frame (orders, roster, lamps, newswire, the lobby and match-over screens). | A player who cannot see the shells left does not know when to go home. | Done; a day's polish left (section 4). |
| 3 | **A hit you can feel.** A shell landing on you is a line in the newswire and a number. | No fear, no urgency; the bubble never feels like a shield. | Window: a flash of the frame's edge on `TankDamaged`, a shake of the camera for a quarter second, the `UNDER FIRE` strip in red. A day. Sound is step 9 and stays there, but these three cost nothing. |
| 4 | **Aiming.** Fixed today: hold right to see the gun's reach, release to fire. The recon's disc is what you can hit; a blind shot is told in the newswire. | A shot refused "inside minimum range" was the first thing the script did. | Done. Left: the cursor's tile under the pointer as a square, so `0 harvest` lands on the tree you meant (finding 8). An hour. |
| 5 | **The squad's orders take a tile.** Seven keys, each a tile under the cursor, no confirmation, no preview. | The builder was Bolo's second game; here it is a key you press at a guess. | Window: a mode. Press `7`, the wall tile follows the cursor as a ghost, click to place, Esc to drop; the orders column lamp says which mode. A day. The sim is unchanged. |
| 6 | **Nothing to say.** `Command::Chat` exists; the window has no box. | Sixteen players and alliances with no words. | Window: Enter opens a line at the newswire, Enter sends to all, Shift-Enter to the team; messages in the newswire tagged by name (SPEC UI-22, UI-23). Half a day. |
| 7 | **Proposals are invisible.** The three-way rule, Join, Recruit, Swap, Merge: the best idea in the design, and the window shows a proposal as one roster line and gives no key to answer it. | The social game is bot-only. | Window: `Y`/`N` answer the newest proposal; a `P` menu over the roster to make one (pick a seat, pick terms). A day. |
| 8 | **The pillbox view.** Done today (`'`, Return). | Territory you cannot look at is not yours. | Done. |
| 9 | **Bots for a newcomer.** Every bot is a RuleBot with full doctrine or a Defender. | The first match is against the best bot there is. | Brain: a `Rookie` that launches late, fires blind, never storms; `--rookies N`. Half a day. The `advantage` flag already goes the other way. |
| 10 | **The second match.** Esc quits the process. | No "again". | Window: the match-over screen's `Enter` restarts the scenario with the same seats (solo) or sends the lobby round again (step 8's problem across the network). A day solo. |
| 11 | **The camera at the start.** The map is 128 tiles and the window shows 60; the newcomer does not know where the centre is or where the others are. | Lost before the first shot. | Window: a minimap, 128 x 128 pixels, in the right column under the depots: fog, depots, pillboxes, the tank. Half a day; the data is in the observation. |
| 12 | **Smoothing the correction.** A prediction mismatch snaps the tank (0011's open problem). | Not seen yet: 784 checks, 0 mismatches under lag. | Window: lerp the drawn tank to the corrected one over 100 ms. Leave until it is seen. |
| 13 | **Two windows on one machine.** Untested; lavapipe takes 90 % of four cores for one. | Only matters for the shoot. | Measure. Nothing to build. |
| 14 | **Sound.** Step 9. | A shell you cannot see has no sound. | Last, on purpose. |

Items 1, 3, 4, 5 and 6 are the week. After them a stranger can be
handed the window.

## 3. Bolo's interface elements, checked off

The prior art's screen (SPEC section 13; the WinBolo frames in
`INTERFACE.md`), element by element, against the window before and
after today.

| Bolo element | SPEC | before today | after today |
| --- | --- | --- | --- |
| map in the centre, never covered | UI-01 | covered by the controls overlay | the map is the middle of the frame; the scrollback and screens are the only things over it, on request |
| builder mode column, left | UI-02 | a key list in the overlay | the orders column: key, word, count or cost, a dash when it cannot be given |
| one status icon per tank, pillbox, base | UI-03 | the roster; depots as `p0 p1 ..` | the roster, 20 pillbox lamps, 17 depot lamps, by owner |
| bars for the nearest friendly base's stock | UI-04 | none | none: a depot's stock is not a thing in this game (resupply is time-gated) |
| four bars for the tank's own stock | UI-05 | armor and fuel bars, numbers for the rest | shells as 24 lamps, the bay as 11, the squad as 4, material as 8, armor and fuel bars |
| a message pane that scrolls | UI-06 | ten lines over the map | the two-line newswire; the forty-line scrollback behind Tab |
| menu bar | UI-07 | none | none; the orders column is the menu |
| colour code: red hostile, green friendly | §2.9.11 | seat colours on the map; hostile air red | the same; in the lamps `X` hostile, `#` mine, `+` ally, `o` neutral, `.` dead, because the frame is one colour |
| tank view follows the tank | UI-08 | F | F or Return, and the tank sits in the map's middle, not the window's |
| scroll keys suspend following | UI-09 | WASD | the same |
| pillbox view, next on each press | UI-10 | none | the apostrophe, mine and my allies' |
| arrows to the neighbouring pillbox | UI-11 | none | none; next in id order |
| re-entering starts from the last viewed | UI-12 | none | yes |
| tank view key returns | UI-13 | F | Return or F |
| window sizes, redraw rate | UI-14, 15 | wheel zoom | the same |
| name labels on tanks | UI-16 | yes, with armor | the same |
| rebindable keys | UI-18 to 21 | none | none |
| messages to all, allies, nearby, selected | UI-22 | the command exists; no box | item 6 above |
| the newswire as a message class | §3.2.3 | the log | the newswire |
| gun range key, longer and shorter | §2.9.10 | the gun takes a point | the envelope lit while aiming; a point is better than a range for an indirect gun with a spotter |

Three of Bolo's elements do not transfer and are not missed: the base
stock bars (depots hold no stock here), the menu bar (the column is
the menu), and the gun range keys (the gun is aimed at a point the
recon sees, which is the design). Two are open: the chat box and the
neighbouring-pillbox arrows.

## 4. The frame, after a day

What the frame still wants, found in its own first frames:

- Colour in the text. The frame is one ink. `UNDER FIRE`, a dead
  lamp, a hostile `X` want red; a lit lamp wants the seat's colour.
  Bevy's `TextSpan` does it per run; the panels become a few spans
  instead of one string. A morning.
- The roster's columns do not line up with their header. Minutes.
- Below about 900 px the bottom strip's lines wrap. Fold the AIR line
  under the lamps and let the newswire be one line. An hour.
- The map's clear colour shows through the frame at the corners of a
  resized window for a frame. Cosmetic.

## 5. What this review does not ask for

A tutorial. The lobby screen's six lines and a sixty-second grace are
the tutorial; a game whose first minute needs a lesson has the wrong
first minute. A minimap before the pillbox view and the lamps have
been played with. Any change to `rules.rs`: the numbers have traces
behind them (GOVERNANCE.md section 5), and nothing in this review was
found by playing against them. A second art pass: the rectangles and
lines are the look, and the frames in the gallery sell it as it is.
