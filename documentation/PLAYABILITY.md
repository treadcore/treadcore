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
| 1 | **The first minute.** Done (journal 0019): the bots hold fire for 60 s at a tank that has never moved or fired (`GRACE_SECONDS`, brain only). The proof, `a_newcomer_gets_a_first_minute`, sits seat 0 still among fifteen bots on three seeds: first hit at 29.5 s before, none inside the minute after. | The newcomer died reading the help. | Done. Bot against bot is unchanged: the squad proof's twenty seeds read the same to the digit. No `--grace` flag: a scenario knob was not needed. |
| 2 | **Reading the screen.** Done today: the frame (orders, roster, lamps, newswire, the lobby and match-over screens). | A player who cannot see the shells left does not know when to go home. | Done; a day's polish left (section 4). |
| 3 | **A hit you can feel.** Done (journal 0017): the map's frame flashes red and the camera shakes for a third of a second on `TankDamaged`, and the top strip goes red while `UNDER FIRE`. | No fear, no urgency; the bubble never feels like a shield. | Done. Sound is step 9 and stays there. |
| 4 | **Aiming.** Fixed today: hold right to see the gun's reach, release to fire. The recon's disc is what you can hit; a blind shot is told in the newswire. | A shot refused "inside minimum range" was the first thing the script did. | Done, and the cursor's tile is outlined while a builder mode is armed (journal 0017), so farm lands on the tree you meant. A refusal now stays on the newswire for four seconds or until the next order. |
| 5 | **The squad's orders take a tile.** Done (journal 0017): the prior art's five modes. Press `6`, the wall's square follows the cursor as an outline, click to place, Esc to drop; the column's one red lamp says which mode. | The builder was Bolo's second game; here it was a key you pressed at a guess. | Done. The sim is unchanged but for BLD-05. |
| 6 | **Nothing to say.** `Command::Chat` exists; the window has no box. | Sixteen players and alliances with no words. | Done (journal 0025): Enter opens a line on the newswire to all, Shift-Enter to the team, Tab switches; messages held on the wire eight seconds by name, with a tone; the world cuts a message to 120 characters and takes one a second (UI-23). The tank view is F alone now (UI-13). Nearby and chosen recipients (the rest of UI-22) not done. |
| 7 | **Proposals are invisible.** The three-way rule, Join, Recruit, Swap, Merge: the best idea in the design, and the window shows a proposal as one roster line and gives no key to answer it. | The social game is bot-only. | Done (journal 0026): `Y`/`N` answer the newest proposal to you or your side, which stands on the newswire with its seconds; `P` opens a list over the map (up/down who, left/right Join, Recruit, Swap, Merge, Enter proposes, `L` leaves your side) that says beforehand whether the world would refuse it (`Observation::would_refuse`, the world's own three-way rule). A RuleBot answers a person: yes unless shot by them. |
| 8 | **The pillbox view.** Done today (`'`, Return). | Territory you cannot look at is not yours. | Done. |
| 9 | **Bots for a newcomer.** Every bot is a RuleBot with full doctrine or a Defender. | The first match is against the best bot there is. | Brain: a `Rookie` that launches late, fires blind, never storms; `--rookies N`. Half a day. The `advantage` flag already goes the other way. |
| 10 | **The second match.** Esc quits the process. | No "again". | Window: the match-over screen's `Enter` restarts the scenario with the same seats (solo) or sends the lobby round again (step 8's problem across the network). A day solo. |
| 11 | **The camera at the start.** The map is 128 tiles and the window shows 60; the newcomer does not know where the centre is or where the others are. | Lost before the first shot. | Window: a minimap, 128 x 128 pixels, in the right column under the depots: fog, depots, pillboxes, the tank. Half a day; the data is in the observation. |
| 12 | **Smoothing the correction.** A prediction mismatch snaps the tank (0011's open problem). | Not seen yet: 784 checks, 0 mismatches under lag. | Window: lerp the drawn tank to the corrected one over 100 ms. Leave until it is seen. |
| 13 | **Two windows on one machine.** Untested; lavapipe takes 90 % of four cores for one. | Only matters for the shoot. | Measure. Nothing to build. |
| 14 | **Sound.** Step 9. | A shell you cannot see has no sound. | Done (journals 0023, 0024): 32 synthesised cues, FM for the weapons, the metal and the machine; full strength on screen, the whole map heard fading to a floor off it; a hostile shell whistles where it will land; M mutes. Wants a human ear. |

Items 1, 3, 4, 5 and 6 are the week. After them a stranger can be
handed the window.

## 3. Bolo's interface elements, checked off

The prior art's screen (SPEC section 13; the WinBolo frames in
`INTERFACE.md`), element by element, against the window before and
after today.

| Bolo element | SPEC | before today | after today |
| --- | --- | --- | --- |
| map in the centre, never covered | UI-01 | covered by the controls overlay | the map is the middle of the frame; the scrollback and screens are the only things over it, on request |
| builder mode column, left | UI-02 | a key list in the overlay | the orders column: the five builder modes in the spec's order, a picture tile and a lamp each, one lamp lit for the armed mode, a click on the map gives it (journal 0017) |
| one status icon per tank, pillbox, base | UI-03 | the roster; depots as `p0 p1 ..` | the roster, 20 pillbox lamps, 17 depot lamps, by owner |
| bars for the nearest friendly base's stock | UI-04 | none | none: a depot's stock is not a thing in this game (resupply is time-gated) |
| four bars for the tank's own stock | UI-05 | armor and fuel bars, numbers for the rest | shells as 24 lamps, the bay as 11, the squad as 4, material as 8, armor and fuel bars |
| a message pane that scrolls | UI-06 | ten lines over the map | the two-line newswire; the forty-line scrollback behind Tab |
| menu bar | UI-07 | none | none; the orders column is the menu |
| colour code: red hostile, green friendly | §2.9.11 | seat colours on the map; hostile air red | the same; in the lamps `X` hostile, `#` mine, `+` ally, `o` neutral, `.` dead, because the frame is one colour |
| tank view follows the tank | UI-08 | F | F or Return, and the tank sits in the map's middle, not the window's |
| scroll keys suspend following | UI-09 | WASD | the same |
| pillbox view, next on each press | UI-10 | none | the apostrophe, mine and my allies'; a click on a pillbox lamp jumps to that one (journal 0017) |
| arrows to the neighbouring pillbox | UI-11 | none | in pillbox view the arrows jump to the nearest pillbox in that direction (journal 0017); outside it they scroll |
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

- Colour in the text. Done the same evening for everything that is
  a lamp or a bar (`INTERFACE.md` section 7): the lamps are nodes,
  coloured by state. The text panels are still one ink, except that
  the top strip goes red while `UNDER FIRE` (journal 0017).
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
