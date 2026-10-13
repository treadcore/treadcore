# The interface: what the tank games put on the screen, and how treadcore's should be laid out

13 October 2026. A study of three kinds of tank-game interface, asked
for after the first playtest build, and a layout for treadcore drawn
from them. The playtest's findings 3, 4 and 7 (`LAB-REPORT-PLAYTEST.md`)
are what it has to fix. Everything here keeps the one rule the window
has always had: everything drawn is a rectangle or a line.

## 1. The references

Ten links, which resolve to four things.

**Bolo and WinBolo (1987, 1993; two screenshots at My Abandonware, one
at Reddit).** The prior art's own screen, and the one with the most to
teach, because its problem was treadcore's: one tank, a squad, built
things, sixteen players, and no room. The layout, from the WinBolo
frame:

```
 +----+-------------------------+--------+-------+
 | M  |                         | tank   | pill- |
 | O  |                         | status | box   |
 | D  |      map viewport       |  grids | and   |
 | E  |     (tank at centre)    |        | base  |
 | S  |                         | base   | lamps |
 |    |                         | status | gauges|
 +----+-------------------------+--------+-------+
 | newswire: one line of what just happened      |
 +-----------------------------------------------+
```

- A **mode column** on the left: six small tiles, one per thing the
  builder can be told to do (tree, road, building, pillbox, mine, and
  the tank), with a red lamp beside the active one. The whole command
  vocabulary, visible, and the current mode never in doubt.
- **Lamp grids** on the right for the tank's stock: shells as a grid of
  red squares, mines as a grid, armor as green dots, trees as a bar.
  No numbers. A glance says "half".
- A **pillbox strip** and a **base strip**: one lamp per pillbox on the
  map and one per base, coloured by owner. Territory as a row of lights
  that the eye reads at once, and in WinBolo the lamps are buttons: a
  click jumps the view to that pillbox (SPEC UI-10 to UI-13; backlog 12).
- **Kill and death counters** and, in WinBolo, a **range circle** for
  the gun.
- The **newswire** across the bottom: one line, the last thing that
  happened, in words ("Man_Eating_Cow captured a neutral pillbox").
- No overlay on the map at all. The map is the map.

**A garage screen (Behance, "Tank Game UI design": a free-to-play
tank game's hangar).** Between matches: a top bar (account, the
BATTLE button, currencies), the vehicle in the middle, a module grid
over it, a stats column on the right with the deltas each module
gives, a filmstrip of modules along the bottom, chat bottom-right. The
lesson is not the chrome but the fact of a *between* screen: where a
player chooses, reads numbers, talks, and presses one big button.
treadcore has no screen before the match: the lobby's state is one
status line.

**Military HUD motion graphics (three stock items from Envato and
VideoHive: "Advanced Military Tank HUD Interface", "Tank HUD", "HUD
Info Military Tank").** The vehicle at the centre, drawn in wireframe,
with concentric rings and arcs around it; data blocks in the four
corners with a label, a number and a tiny bar each; one accent colour
on a dark ground; readouts attached to the vehicle by leader lines.
They are decoration with nothing behind the numbers, and the designers
of them have never had to read one in a fight. What survives from them:
the vehicle is the centre and the rings are around *it*, which
treadcore already does with the bubble and detection rings; the corner
blocks; the leader line, which treadcore already draws from tank to
drone; and the restraint of one accent.

## 2. What treadcore has now

Two text panels anchored top-left (telemetry, AIR table, hostile
counts) and top-right (NET line, roster, depot strip), an EVENTS log
bottom-left, a controls overlay over the map until Tab, and the map
with its overlay (rings, links, arcs, vision discs). Every number is a
number. In a 1280-wide window it works; in a 626-wide tiled window
the two top panels overlap (finding 3), the overlay hides the start
(finding 4), and a newcomer who reads the controls instead of moving
is dead in thirty seconds (finding 7).

## 3. The layout

Bolo's frame, with the concepts of this game in its boxes, and the HUD
graphics' centre kept.

```
 +--------------------------------------------------------------------+
 | TREADCORE  alice  seat 1  t3   tick 820  41.0s   NET host rtt 50ms |
 |   predict +2  in flight 0  checked 174  mismatch 0  hash @1000 ..  |
 +------+-----------------------------------------------+-------------+
 |ORDERS|                                               | ROSTER      |
 | 1 I  |                                               | t0 p0 bot.. |
 | 2 R  |                                               |*t1 p1 alice |
 | 3 S  |                                               |  ...        |
 | R <  |            map viewport, tank at centre       | t15 p15 ..  |
 |      |            bubble ring, detection ring,       |             |
 | 4 cap|            links, arcs, vision discs          | PILLBOXES   |
 | 5 pb |            (unchanged)                        | o o x o . o |
 | 6 ret|                                               | . o o o x o |
 | 7 wal|                                               | o . o o o . |
 | 8 min|                                               | o o         |
 | 9 rd |                                               | DEPOTS      |
 | 0 har|                                               | o o . x o o |
 |      |                                               | o o o o o . |
 | [ ]  |                                               | o o o o o   |
 +------+-----------------------------------------------+-------------+
 | SHELLS ######## ######## ########   ARMOR ##########  FUEL ####### |
 | BAY  recon oo  intercept oooooo  strike ooo   SQUAD oooo  MAT oooo |
 | AIR  #182 I 73% 84s   #187 I 71% 85s   #70 R 68% 147s              |
 | 490  pillbox hit you -15      490  tank hit -15 (70 left) by swarm |
 +--------------------------------------------------------------------+
```

Top strip: who and when, and the NET line, one row each. It is the
only place a number that is not the tank's lives.

**ORDERS, the left column.** Bolo's mode column, for treadcore's
orders. The key, a glyph, a word: `1 I` launch interceptor, `2 R`
recon toward cursor, `3 S` strike at nearest hostile, `R <` recall;
then the squad's seven, `4` capture, `5` build pillbox, `6` return,
`7` wall, `8` mine, `9` road, `0` harvest; then `[ ]` bubble. The lamp
beside an order lights while it is the cursor's mode (the squad
orders take a tile; the lamp says which one the next click gives),
and dims when it cannot be given (no material: `5 7 8 9` dim; squad
deployed: `4 5 7 8 9 0` dim, `6` lit). This column *is* the help. The
controls overlay goes; Tab toggles the column's words off for a
player who knows them.

**The map.** Unchanged, and never covered. The tank at the centre
when following; the bubble ring and detection ring around it, as the
HUD graphics put rings around the vehicle; the links as leader lines
to each drone with its label; shell arcs; vision discs. One addition
from WinBolo's range circle: while the right button is held, the
gun's annulus, 12 to 60 tiles, so a shot inside minimum range is
never ordered (the first solo frame's `order refused: inside minimum
range` was that).

**ROSTER, PILLBOXES, DEPOTS, the right column.** The roster as now,
sixteen rows, mine starred. Below it Bolo's strips as lamp grids: one
lamp per pillbox (20) and per depot (17), coloured by owner, hollow
for neutral, crossed for dead; a click on a pillbox lamp jumps the
camera to it and `F` brings it back to the tank (backlog 12, in one
stroke). The depot strip the NET panel already draws as `p0 p1 ..`
becomes this.

**The bottom strip: lamps, air, newswire.** Bolo's grids, laid flat:
shells as 24 lamps in three rows of eight, armor and fuel as bars, the
bay as lamps by kind (2, 6, 3), the squad as four man-lamps, material
as eight. The AIR table on one line per drone, as now but across. The
last line is the newswire: the two most recent events, in the words
the log already uses; the full EVENTS log is a scrollback behind Tab,
not a panel on the map.

Widths: the columns are fixed in characters (the orders column is
eight wide, the roster twenty-six), the map takes the rest, and below
about 80 columns the right column folds under the roster and the
bottom strip wraps. The overlap of finding 3 cannot happen because
nothing is anchored to the same corner twice.

## 4. The screens around the match

The garage's lesson: a screen before, and one after.

**The lobby screen.** Where the status line is now: the lobby's name,
the peers found with their names and the median rtt to each as the
probe measures it, the matrix once shared, the election's answer as
it is announced, and the one big line: `playing as seat 1 in 2 s`.
Below it, the first minute's instructions in six lines, which is
finding 7's cheapest fix: *you are at your depot; launch an
interceptor (1) and a recon (2) before you move; drive (left click)
when the recon sees something; a standing tank is a target*. Chat
goes here too (`Command::Chat` exists; the window has no box for it).

**The match-over screen.** The server's match report already has the
numbers (shells by kind and spotted, intercepts, kills by cause,
pillboxes and depots held, proposals); the window shows a status line.
The report, as a table, over the dimmed map, with the roster's final
scores beside it, and the one line `Esc to leave`.

## 5. What this does not change

The observation. Every panel above draws from `Observation` as the
HUD does now; no new field is needed for the lamps, the strips or the
newswire, and a Brain sees exactly what the player sees. The sim and
the wire are untouched. The work is in `treadcore-client`: `hud.rs`
for the columns and strips, `input.rs` for the order lamps and the
pillbox jump, `session.rs` for the lobby and match-over screens.

## 6. In order

1. The columns and the bottom strip; the controls overlay removed.
   Fixes findings 3 and 4.
2. The order lamps with their dimming rules, and the gun's annulus.
3. The pillbox and depot lamp strips, with the jump (backlog 12).
4. The lobby screen with the first-minute lines (finding 7); the
   match-over screen.
5. The newswire and the scrollback.

Each step is a window change, and now that the window opens here,
each can be seen.

**Built the same day** (journal 0014; `hud.rs`, `input.rs`): all five,
with these departures from the drawing above. The orders are a dash in
the margin when they cannot be given, not a lamp, because the frame is
one ink (colour per run is PLAYABILITY.md section 4's first item). The
pillbox jump is the apostrophe, Bolo's own key, and Return or F is the
tank view. The gun's annulus lights while the right button is held and
the shot goes on release. The newswire is two lines; the scrollback
holds forty. The frame's widths are 132, 268, 44 and 84 pixels. The
frames are in the gallery, and what the frame still wants is in
PLAYABILITY.md section 4.

## 7. The look: WinBolo's bezel, the same evening

A second look at the prior art, this time WinBolo's 2026 screen at
full size (an AppleInsider screenshot of the Mac port) and the Bolo
cover with the title spelled in wall tiles. What is going on in that
frame, element by element: a grey bezel of inset black boxes; a lamp
for every thing in the game (a row of circles for the tanks, hollow
for one's own, green friendly, red hostile; a grid of red squares for
the pillboxes, chequered when neutral; a grid for the bases), each
lit in the colour of its state; a tile with a tiny picture beside each
builder mode and a red lamp against the active one; four green bars
with a pictogram each for shells, mines, armour and trees; a skull and
a gravestone with the kills and deaths; the Players menu with every
player's ping; and the newswire in a large face across the whole
bottom, scrolling.

The frame of section 3 now wears it (`hud.rs`, journal 0015). Every
lamp is a UI node whose background is its state, so the one-ink limit
of the first frame is gone: pillbox and depot lamps are the owner's
seat colour for mine, green for an ally's, red for hostile, dim tan
for neutral, dark for dead; a row of sixteen tank lamps the same way;
shells as red squares; the bay by kind (recon sky, interceptor green,
strike red); the squad as four green lamps that go amber when it is
out; material as eight green squares; armor and fuel as green bars.
Each order has a pictogram tile (the hull, a round, the interceptor's
ring, the recon's disc, the strike's dart, a hollow pillbox for
capture and a filled one for build, a wall, a mine, a road, a tree
tile) and a lamp beside it, red when the order can be given and dim
when not. Kills and deaths count in a box of their own. The newswire
is one line in a 22 px face across the bottom. No images: every
element is a rectangle, rounded or not. The pings are the one thing
not taken, because a seat is told only its own round trip; the lobby
screen shows the medians while they are measured.

## 8. The map and the things on it: Bolo's colours, assets out of cubes

The last of the look, the same night (journal 0016): the ground in
Bolo's palette and the things on it built from cubes.

**The palette** (`render.rs`, `terrain_color`): dark green grass,
forest in a bright green checker at one sprite a tile (Bolo's leaf
pattern, as far as a flat tile goes), grey road with a lighter checker
for its dashes, cyan water, tan planks for bridges, tan blocks for
walls, brown rubble. The five converted community maps look like the
screenshots they were converted from.

**The cubes** (`voxel.rs`): a `Model` is a list of cubes on a grid of
fifths of a tile, and the window has no third dimension: a cube is
drawn in an oblique projection as its top face, lifted and lightened
by its height, and a shaded side face hanging below it where no cube
stands in front. Two rectangles a cube; the rule holds. The models
are Bolo's things in Bolo's colours: the tank a hull eight by five
with dark tracks down the sides, a turret in the seat's colour on
top and a steel barrel forward; the pillbox a tan pyramid, three
levels, the gun on top in orange while it has more than half its
health and a flat dark slab when dead; the depot a tan plate with the
owner's mark raised in the middle; the recon a cross, the interceptor
a block, the strike a dart, the swarm's in red; mines, shells, the
squad as two men; a wall a tan block the size of its tile, raised when
the tile becomes one and taken down when a shell knocks it to rubble.
A model's cubes are rebuilt only when its key changes (owner, alive,
health bucket); the parent moves and turns.

What this is not: a 3D renderer. The engine's PBR crates are not in
the build, by choice (BUILD.md), and a camera that tilts would be a
different game. The cubes stand up from the map the way Bolo's
pillboxes stood up from theirs: by their drawing.
