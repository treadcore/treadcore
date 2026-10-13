# WinBolo / LinBolo 1.14: clean-room functional specification

Received 12 October 2026 (author: @Micro) and kept here as the
reference specification for the Bolo-derived layer of treadcore:
builder, pillboxes, refuelling bases, mines, terrain, alliances,
visibility and game lifecycle. It restates the behaviour of WinBolo /
LinBolo 1.14 as observable from its user manual, in new words, so that
a team that has never seen the program or its code can build a
compatible game. treadcore does not build a compatible game; it builds
a descendant. The preface says which parts it takes.

## How treadcore reads this document

| spec section | treadcore | where |
| --- | --- | --- |
| 4 World, map and terrain | adapted: grass, forest, road, water, bridge, rubble, depot exist; building, swamp, deep sea, boats, craters-that-flood do not; walls and mines come with step 6c | DESIGN.md, ROADMAP 5, 6c |
| 5 Tank: movement | adapted: momentum and terrain-limited speed as in TNK-03 to TNK-06; steering replaced by a routed destination (DESIGN.md, LAB-REPORT-PILLBOYS.md) | DESIGN.md |
| 5 Tank: shooting | diverges by design: the gun fires indirect at a coordinate; TNK-18's "first obstacle" applies to direct fire only, and not yet | DESIGN.md |
| 5 Tank: stocks, boats | stocks adapted (shells, armor, fuel, trees-as-material in 6c; no mines stock until 6c); boats not adopted | ROADMAP 6c |
| 6 Builder | **adopted** as the Pillboy: BLD-01 to BLD-03, BLD-05, BLD-06, BLD-09, BLD-10, BLD-13; the parachute (BLD-11) becomes resupply at a depot | DESIGN.md "Pillboys and pillboxes", ROADMAP 3, 6c |
| 7 Mines | **adopted** (step 6c, done): MIN-01 as the squad's lay, MIN-02 as material, MIN-03, MIN-05, MIN-06, MIN-09 to MIN-13; one rule added: drones ignore mines; quick mines (MIN-04) and ally sharing (MIN-08) wait for alliances | ROADMAP 6c |

| 8 Pillboxes | **adopted**: PIL-01 to PIL-05, PIL-07 to PIL-17; added: pillboxes engage drones at short range; pillboxes hold shells (BAS-01 shape) | ROADMAP 3, 6c |
| 9 Refuelling bases | **adopted** as depots, already in: BAS-02, BAS-04 to BAS-06, BAS-11; BAS-07 to BAS-09 (armour as the gate) open for a proposal | DESIGN.md "Depots" |
| 10 Visibility | adapted: forest halves vision (VIS-02 in spirit); drones add sensors Bolo did not have | DESIGN.md "The world" |
| 10 Alliances | **adopted** (steps 6 and 6b, done): ALL-01 to ALL-06, ALL-08 to ALL-10 as written; ALL-07 falls out of per-player ownership; treadcore adds proposals to defect, recruit, swap and merge, under a three-way rule Bolo did not have | ROADMAP 6, 6b |


| 10 Ownership | adopted: OWN-01 to OWN-03 | ROADMAP 3, 6 |
| 11 Game setup and lifecycle | adapted: game types, start delay, time limit, player limit 16 (GAM-16) are the shape of `Scenario`; tutorial not planned; rejoin (GAM-12) is a backlog item | ROADMAP backlog |
| 12 Networking | **not adopted**. treadcore has no dedicated server as the only option, no tracker, no statistics site, and no TCP; peers elect a host from a measured latency matrix over UDP (PROTOCOL.md). NET-02 and NET-03 (authoritative server, client prediction) are treadcore's too, by other means (ROADMAP 4). Recorded for reference only. The Macintosh Bolo AppleTalk ring is not in this specification and not in treadcore. | PROTOCOL.md |
| 13 to 15 Window, dialogs, menus | not adopted; treadcore's HUD is the telemetry overlay. UI-10 (pillbox view) and the colour code (red hostile, green friendly) are worth taking | client |
| 16 Brains | adopted in principle: treadcore's Brain is the client's own interface (observation in, commands out); BRN-05, BRN-12 ("advantage") map to scenario options | treadcore-brain |
| 17 Dedicated server | not adopted as specified; `treadcore-server --listen` (ROADMAP 8) is the analogue; console commands lock/unlock/savemap/say/info are a good list | ROADMAP 8 |
| 18 Customisation | not adopted (no assets by design); map files are ROADMAP 5 | ROADMAP 5, 9 |
| 19 Errors | the categories are a checklist for treadcore's own messages | backlog |
| 20, 21 Contradictions, unspecified | every number in section 21 that treadcore needs is a `rules.rs` constant with a sentence, which is this project's answer to that section | rules.rs |

Nothing below this line has been altered from the received text
except its formatting into tables and the removal of nothing.

---

## 1. About this spec

This spec restates the behaviour of WinBolo/LinBolo 1.14 that is
observable from its user manual, so a team that has never seen the
program or its code can build a compatible game.

- **Single source.** Everything here comes from the Lin/WinBolo
  Manual, version 1.14 (John Morrison, 49 pages), including its
  screenshots and version history. No binary, source code, sample code
  or other Bolo document was consulted.
- **Original wording.** Behaviour is described in new words. Only
  interface literals are kept exact: file names, command-line flags,
  menu and dialog labels, and default values.
- **No guessing.** Where the manual is silent, the gap is listed in
  section 21 instead of being filled. Where it contradicts itself,
  section 20 records both statements and the reading adopted.
- **One-way flow.** The implementing team should work from this
  document alone and route questions back through the spec owner.

Every requirement carries an ID, a source and a basis code.

| basis | meaning |
| --- | --- |
| S | Stated in the manual's body text |
| F | Read from a figure or screenshot; the screenshots are small, so treat as lower confidence |
| H | Stated only in the version history (manual section 5), usually as a change or fix note |
| I | Inferred because other statements only make sense if it is true; confirm before relying on it |

Sources are cited as manual section numbers (§2.9.3) or
version-history entries (v1.07). Section numbers follow the manual's
body, not its table of contents, which omits the Skins section.

## 2. Scope and system overview

The product is a real-time tank game for up to 16 networked players,
delivered as a graphical client and a headless server that
interoperate between Windows and Linux (§2.1, §2.2). Clients also find
games on a local network by broadcast, without the tracker. A brain
runs inside one client and controls only that client's tank.

| component | role | source |
| --- | --- | --- |
| Game client (WinBolo.exe, Linbolo) | Draws the game, takes input and hosts every dialog; can also start a server inside itself | §2.6, §2.7, v1.14 |
| Dedicated server (WinBoloDS.exe, Linbolods) | Hosts a game from the command line with no graphics and holds the master game state | §3.3, §3.6 |
| Internet tracker | Outside directory that hosts notify and clients query for running games | §3.1.7, §3.1.8 |
| WinBolo.net | Outside statistics site that receives game results when players opt in | §4 |
| Brains | Plug-in AI modules the client loads to drive or assist a tank | §2.10 |
| Sounds file (BoloSounds.bsd) | Holds the built-in sound effects | §2.6 |

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| SYS-01 | A game supports up to 16 players, each on a separate computer with a private view of the world. | §2.1 | S |
| SYS-02 | Windows and Linux builds of the client and server join the same games. | §2.2, §2.8 | S |
| SYS-03 | Network compatibility with Macintosh Bolo is not provided. | §2.8 | S |
| SYS-04 | Play is real time with all players acting at once. There is no pause function. | §2.10, §6.4 | S |
| SYS-05 | The client can host a game itself without launching a separate server program. | v1.14 | H |
| SYS-06 | The client needs a display of at least 16-bit colour and refuses to run in a palette mode. | §2.3, §3.5.1 | S |

In scope: game rules; client windows, menus and controls; network
behaviour a user can observe; the server command line; customisation
file layouts; error conditions.

Out of scope: shareware pricing, registration and licence terms
(§2.4); project history and credits (§2.5, §6.2); design inspiration
(§6.4); the 2004-era prerequisites such as DirectX 5 and GTK 1.2
(§2.3); the Register.exe helper (§2.6).

### Platform parity

| capability | Windows | Linux | source |
| --- | --- | --- | --- |
| Play and host games across platforms | Yes | Yes | §2.2, §2.8 |
| Language sets | Yes | No | §3.1.3, §3.4 |
| Background Sound option | Yes | No | §3.2.3, §3.4 |
| ISA Sound Card option | Yes | No | §3.2.3 |
| Map preview when choosing a map file | Yes | No | v1.06, §3.4 |
| Double and quad window sizes | Yes | No | §3.4, v1.04 |
| Customised sounds folder | Yes | No | §3.8, §3.4 |
| Skins | Yes | Not stated; §3.12 lists it as planned | §3.9, §3.12 |
| Server lock and unlock by signal | No | Yes | §3.3 |

## 3. Glossary

| term | meaning |
| --- | --- |
| Tank | The single vehicle each player controls. |
| Builder | The figure carried by each tank that leaves it to farm, build and lay mines. The manual also calls it "the man" and, in the version history, "LGM" (never expanded). |
| Pillbox | A fixed automatic gun that fires at enemy tanks. Also "pill". |
| Dead pillbox | A pillbox shot down to zero strength: inactive and able to be picked up. |
| Refuelling base | A map square that resupplies tanks with shells, mines and armour. Also "base". |
| Neutral | Owned by no player. Neutral pillboxes attack everyone; neutral bases serve anyone. |
| Alliance | A declared group of players whose tanks, pillboxes and bases treat each other as friendly. |
| Trees | The unit of building material a tank carries, gained by farming forest. |
| Quick mine | A mine dropped directly under the tank by key press. |
| Hidden mine | A builder-laid mine that non-allied players cannot see until close. |
| Angry | The manual's word for a pillbox that has been provoked into firing faster. |
| Map square | One cell of the map grid. Skin graphics are 16 by 16 pixels per square at normal size. |
| Deep sea | Open water that destroys any tank not on a boat and never changes. |
| Brain | A plug-in that controls or assists a tank automatically. Also "computer tank". |
| Tracker | The Internet service that lists running games. |
| Lock | The state in which a game accepts no new players. |
| Rejoin | Re-entering a game recently left and reclaiming former possessions. |
| Open, Tournament, Strict Tournament | The three game types, which differ in what a new tank starts with. |
| Everard Island | The default map built into client and server. |
| Newswire | Automatic messages reporting game events to players. |
| Gunsight | The on-screen marker showing where a shell will land. |

## 4. World, map and terrain

The world is a grid of map squares, each holding one terrain type that
sets a tank's top speed and can change during play.

### Map

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| MAP-01 | The world is a two-dimensional island map made of squares and shown from above. | §2.1, §2.9.3 | S |
| MAP-02 | The map "Everard Island" is built into both client and server and needs no file. | v1.03, §3.3 | S |
| MAP-03 | Other maps load from files, chosen with Choose Map in Game Setup or -map on the server. A set of maps ships in a Maps folder. | §2.9.14, §3.1.6, §3.3 | S |
| MAP-04 | A map defines terrain, pillboxes, bases, their initial owners and the squares where tanks may start. | v1.02, v1.09 | I |
| MAP-05 | On load, any pillbox or base with invalid ownership data becomes neutral. | v1.02 | H |
| MAP-06 | On load, any mine lying under a base is removed. | v1.07 | H |
| MAP-07 | On load, the map is centred. The manual does not say whether this means within the world grid or in the view. | v1.05 | H |
| MAP-08 | A minefield surrounds the edge of the playable world. | v1.05, v1.14 | H |
| MAP-09 | The map in its current state can be saved to a file from any client and from the server. | §3.2.1, §3.3 | S |
| MAP-10 | A file that is not a valid map is rejected with an error and the game does not start. | §3.5.2 | S |

### Terrain types

Terrain speed uses five classes: fast, medium, slow, very slow and
impassable. The manual gives no numbers. All rows are from §2.9.3
unless noted.

| terrain | top speed | behaviour |
| --- | --- | --- |
| Grass | Medium | Default land cover. |
| Road | Fast | Built by the builder. |
| Bridge | Fast | Built over river water. Blocks boats until shot away (§2.9.2). |
| Moored boat | Fast | A tank drives onto it to board, then travels fast over water. |
| Forest | Slow | Hides a tank inside it, supplies trees and regrows over time. Cannot be built on until cleared (§2.9.6). |
| Swamp | Very slow | Drawn to resemble grass. Usually found near water. |
| Crater | Very slow | Left by a mine explosion. Floods when next to water. |
| Rubble | Very slow | What a building becomes after repeated shell hits. |
| Water | Very slow without a boat | Drains a boatless tank's shells and mines the longer it stays. |
| Deep sea | Fatal without a boat | Sinks and destroys a boatless tank at once. Nothing can be built on it and it never changes. |
| Building | Impassable | Blocks tanks and gives cover from fire. |
| Damaged building | Impassable | A building after a shell hit. |
| Pillbox, live or damaged | Impassable | See section 8. |
| Dead pillbox | Medium | Driven over to pick it up. |
| Refuelling base | Medium | Closed to enemies while it has armour (§2.9.5); see section 9. |

A mine is an overlay on a square, not a terrain type (section 7).

### How terrain changes

The manual does not name the terrain left when forest is cleared. A
pillbox keeps its owner through a repair and takes a new one only when
a tank picks it up.

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| TER-01 | A shell hit turns a building into a damaged building. Further hits turn it into rubble. | §2.9.3 | S |
| TER-02 | Forest is removed from a square by farming it or by shooting it. | §2.9.6, §2.9.13 | S |
| TER-03 | Forest regrows continuously, so farmed areas refill slowly. | §2.9.6 | S |
| TER-04 | A shell hit destroys a bridge, reopening the water to boats. | §2.9.2 | S |
| TER-05 | A mine explosion replaces its square with a crater. | §2.9.3 | S |
| TER-06 | A crater touching river or sea water fills and becomes water. A line of craters that reaches water becomes a channel boats can use. | §2.9.3, §2.9.8 | S |
| TER-07 | Deep sea is never altered by any action. | §2.9.3 | S |
| TER-08 | The builder creates roads, bridges, buildings, boats and pillboxes on squares (section 6). | §2.9.6 | S |

## 5. Tank

Each player drives one tank whose speed is capped by the terrain
beneath it and whose four stocks (shells, mines, armour, trees) decide
what it can do.

### Movement

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| TNK-01 | Each player controls exactly one tank. | §2.1 | S |
| TNK-02 | The tank has six basic commands: accelerate, decelerate, turn anticlockwise, turn clockwise, shoot and lay mine. | §2.9.1 | S |
| TNK-03 | Movement is smooth and continuous, not square by square. | §6.4 | S |
| TNK-04 | The terrain under the tank sets its top speed. A tank still accelerating as it reaches faster terrain speeds up at once. | §2.9.3, §2.9.13 | S |
| TNK-05 | Speed is held when no speed key is pressed. | §2.9.13, v1.01 | I |
| TNK-06 | With the Auto Slowdown option on, releasing the accelerate key brings the tank gently to a halt. | §3.1.14, v1.01 | S |
| TNK-07 | A tank that has lost synchronisation with the server cannot move until resynchronised. The main window shows an error message meanwhile. | v1.13 | H |

### Entering the game

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| TNK-08 | A new tank starts out at sea on a boat. | §2.9.2 | S |
| TNK-09 | A new tank always has full armour. Its shells, mines and trees depend on the game type (section 11). | §3.1.6 | S |
| TNK-10 | The start location is picked from the map's start squares so as to avoid other tanks and enemy pillboxes. When choices are few, a square near a friendly pillbox is preferred over one near a hostile pillbox. | v1.09 | H |

### Boats and water

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| TNK-11 | A tank on a boat moves fast over water and deep sea and can sail up rivers. | §2.9.2, §2.9.3 | S |
| TNK-12 | A tank leaves its boat by driving onto land, which may need some speed. | §2.9.2 | S |
| TNK-13 | A boat cannot pass a bridge. | §2.9.2 | S |
| TNK-14 | One shell hit sinks a boat and leaves its tank in the water. In deep sea the tank then sinks and is destroyed. | §2.9.2 | S |
| TNK-15 | A player whose shot sinks a boat in deep sea is credited with the kill. | v1.03 | H |
| TNK-16 | A boatless tank in water steadily loses shells and mines, with an audible cue. | §2.9.3, §3.8 | S |
| TNK-17 | A tank boards a moored boat by driving onto it. | §2.9.3 | S |

### Shooting

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| TNK-18 | A shell travels in the direction the tank faces and stops at the first obstacle or at the set gun range, whichever is nearer. | §2.9.10 | S |
| TNK-19 | Gun range is adjusted with two keys, shorter and longer. A gunsight marks where the shell will land. | §2.9.10, §3.2.2 | S |
| TNK-20 | With the automatic gunsight option on, shortening the range shows the gunsight and lengthening it past maximum hides it. | §3.1.14 | S |
| TNK-21 | A shell that lands on a mine detonates it. Firing while changing range clears a path ahead. | §2.9.10 | S |

| shell target | effect | source | basis |
| --- | --- | --- | --- |
| Tank | Reduces its armour | §2.9.4, §3.8 | I |
| Boat carrying a tank | Sinks in one hit | §2.9.2 | S |
| Builder outside its tank | Killed | §2.9.6 | S |
| Pillbox | Loses strength; dead at zero | §2.9.3, §2.9.4 | S |
| Base | Loses armour stock | §2.9.5 | S |
| Building | Damaged, then rubble | §2.9.3 | S |
| Forest | Cleared | §2.9.6 | S |
| Bridge | Destroyed | §2.9.2 | S |
| Mine | Detonated | §2.9.10 | S |

### Stocks and destruction

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| TNK-22 | The tank carries four stocks: shells, mines, armour and trees. | §2.9.11 | S |
| TNK-23 | Shells, mines and armour are replenished at refuelling bases. Trees are gained by farming. | §2.9.5, §2.9.6 | S |
| TNK-24 | Driving over a dead pillbox picks it up. The tank carries it until the builder places it. | §2.9.4 | S |
| TNK-25 | Driving over a mine detonates it and damages the tank. | §2.9.3 | S |
| TNK-26 | A tank that enters deep sea without a boat is destroyed immediately. | §2.9.3 | S |
| TNK-27 | A tank is destroyed when shell or mine damage exhausts its armour. | §2.9.4, §2.9.5 | I |
| TNK-28 | After destruction the player receives a new tank under TNK-08 to TNK-10. | §3.1.6, v1.09 | I |
| TNK-29 | Kills are counted per player. | v1.03 | I |

## 6. Builder, farming and construction

All construction is done by the tank's single builder, who is ordered
by picking one of five modes and clicking a map square, and who spends
trees from the tank's stock.

| mode button, top to bottom | action on the clicked square | cost in trees |
| --- | --- | --- |
| Farm trees | Harvests forest into the tank's tree stock | Gains trees |
| Build road or bridge | Lays a road or a bridge | 0.5 |
| Build building or boat | Raises a building or a boat | 0.5 for a building; 5 for a boat |
| Place or repair pillbox | Places a carried pillbox, or repairs the pillbox already on the square | 1 to place; up to 1 to repair, scaled by damage |
| Lay mine | Buries a mine taken from the tank's mine stock; no tree cost is stated | None |

Costs and modes are from §2.9.6. Which of each pair is built (road or
bridge, building or boat) presumably follows whether the square is land
or water; the manual does not say so outright (basis I).

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| BLD-01 | Each tank has one builder. An order is given by selecting a mode from five buttons at the left of the window and then clicking a map square. | §2.9.6 | S |
| BLD-02 | The builder leaves the tank, runs to the square, does the work and returns. He re-enters at the tank's edge. | §2.9.6, v1.07 | S |
| BLD-03 | An order that costs more trees than the tank holds is not carried out. | §2.9.6 | S |
| BLD-04 | Nothing can be built on deep sea, on a moored boat or on forest. | §2.9.6 | S |
| BLD-05 | In pillbox mode, a click on an existing pillbox repairs it instead of placing another. Any pillbox can be repaired, damaged or dead, whoever owns it. | §2.9.6, §2.9.13 | S |
| BLD-06 | Repair by the builder never changes a pillbox's owner. | §2.9.13 | S |
| BLD-07 | A pillbox standing among forest can still be repaired. | v1.13 | H |
| BLD-08 | Farming works only on forest. A base square cannot be farmed. | v1.13 | H |
| BLD-09 | The builder can be killed outside the tank by a shot or by an explosion within range of him. | §2.9.6, v1.07 | S |
| BLD-10 | While the builder is dead the tank cannot build, farm or lay builder mines. | §2.9.6 | S |
| BLD-11 | A replacement builder arrives by parachute after a delay that can run to several minutes. The parachute is drawn above tanks. | §2.9.6, v1.02 | S |
| BLD-12 | If the builder dies carrying a pillbox, the pillbox drops onto the map but never onto another pillbox, a base or a wall. | v1.14 | H |
| BLD-13 | An order queued for the builder is forgotten if he dies. | v1.09 | H |
| BLD-14 | A builder sent to repair a pillbox that has since been moved is not killed by a visible mine now on that square. | v1.09 | H |
| BLD-15 | The target square follows the mouse pointer and changes only when the mouse moves, not when the view scrolls. | v1.05 | H |
| BLD-16 | The builder reports to his player through assistant messages, which the player can switch off. | §3.2.3 | S |
| BLD-17 | Farming, building and builder death each play a sound, with near and far variants for other players. | §3.8 | S |

## 7. Mines

A mine sits on a map square, explodes when driven over, shot or caught
by a neighbouring blast, and leaves a crater.

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| MIN-01 | A mine is laid in one of two ways: dropped under the tank by key press (quick mine), or buried on a chosen square by the builder. | §2.9.9, §2.9.6 | S |
| MIN-02 | Each mine laid comes out of the tank's mine stock. | §2.9.5, §2.9.11 | I |
| MIN-03 | A mine can lie on any terrain except buildings, boats and water. | §2.9.3 | S |
| MIN-04 | A quick mine is seen by every tank near the layer when it is dropped. | §2.9.9 | S |
| MIN-05 | With hidden mines allowed, a builder-laid mine is visible only to its layer and to players allied with the layer when it was laid. | §3.1.6, §2.9.6 | S |
| MIN-06 | A hidden mine becomes visible to any other tank that comes within one square of it. | §3.1.6 | S |
| MIN-07 | With hidden mines disallowed, every mine is visible to every player however it was laid. | §3.1.6 | S |
| MIN-08 | Mines laid during an alliance are marked on allies' maps and stay marked after the alliance ends. Mines laid before it formed or after it broke are not shared. | §2.9.6, §2.9.7 | S |
| MIN-09 | A mine explodes when a tank drives over it, when a shell lands on it, or when a mine on an adjacent square explodes. | §2.9.3, §2.9.10, §2.9.8 | S |
| MIN-10 | Mines on diagonal neighbours do not set each other off, so a chequerboard layout does not chain. | §2.9.8 | I |
| MIN-11 | An explosion damages a tank on the square and also does damage to the surrounding squares. | §2.9.3, v1.01 | S |
| MIN-12 | An explosion can kill a builder, judged by his distance from the blast, not by which square he is on. | v1.07 | H |
| MIN-13 | An explosion leaves a crater (TER-05, TER-06). | §2.9.3 | S |
| MIN-14 | Dropping a quick mine plays a sound. Explosions play near and far sounds. | v1.02, §3.8 | S |
| MIN-15 | An order to lay a mine where one already lies is handled without error. | v1.02 | H |

## 8. Pillboxes

A pillbox is an automatic gun that beats a tank in a straight fight,
can never be removed from the game, and changes sides only by being
shot dead and picked up.

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| PIL-01 | A pillbox fires at any enemy tank that comes within its range. A neutral pillbox treats every tank as an enemy. | §2.9.3, §2.9.4, §2.9.11 | S |
| PIL-02 | A pillbox does not fire at its owner or at its owner's allies. | §2.9.4, §2.9.7 | S |
| PIL-03 | A pillbox waits a short time before its first shot at a tank that has just entered range. | v1.04 | H |
| PIL-04 | A pillbox aims accurately. Once provoked it becomes angry and fires faster, up to a maximum rate, then cools down over time. | §2.9.4, v1.07, v1.09 | S |
| PIL-05 | In a sustained exchange at full rate, a pillbox defeats a tank. | §2.9.4 | S |
| PIL-06 | A live pillbox also becomes angry when allied bases around it are attacked. A dead pillbox does not; the live case is inferred from that note. | v1.09 | H |
| PIL-07 | A pillbox cannot see or target a tank concealed in forest (VIS-02). | §2.9.3 | S |
| PIL-08 | Pillbox shots hit whatever lies in their path, including walls and other pillboxes. | §2.9.8, §3.6 | S |
| PIL-09 | Each tank shell that hits a pillbox lowers its strength. Its drawing shows the damage level. | §2.9.3, §2.9.11 | S |
| PIL-10 | At zero strength the pillbox is dead: it stops firing and tanks can drive over it. It is never removed from the game. | §2.9.3, §2.9.4 | S |
| PIL-11 | A tank that drives over a dead pillbox picks it up. The pillbox is repaired and now belongs to that player. | §2.9.4 | S |
| PIL-12 | A carried pillbox keeps its angry state and cools at the normal rate. | v1.07 | H |
| PIL-13 | The builder places a carried pillbox on a chosen square for 1 tree. It then defends for its owner and the owner's allies. | §2.9.4, §2.9.6 | S |
| PIL-14 | A pillbox is never placed or dropped onto a wall, a base, another pillbox or the edge minefield. | v1.05, v1.14 | H |
| PIL-15 | Pillboxes carried by a tank that is destroyed or leaves the game are dropped back onto the map under PIL-14. | v1.05, v1.06 | I |
| PIL-16 | The builder repairs a damaged or dead pillbox in place for up to 1 tree, without changing its owner (BLD-05, BLD-06). | §2.9.6, §2.9.13 | S |
| PIL-17 | A live pillbox is impassable to every tank. | §2.9.3 | S |

## 9. Refuelling bases

A base resupplies shells, mines and armour to tanks allowed to use it,
and capturing every base is the object of the game.

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| BAS-01 | A base holds three stocks: shells, mines and armour. | §2.9.5, §2.9.11 | S |
| BAS-02 | A tank sitting on a base it may use is refuelled: its stocks rise as the base's fall. | §2.9.5 | S |
| BAS-03 | Refuelling begins a short time after the tank arrives on the base. | v1.04 | H |
| BAS-04 | A base slowly replenishes its own stocks over time, in single-player as well as network games. | §2.9.5, v1.01 | S |
| BAS-05 | A neutral base serves any tank. Driving onto it captures it for that player and the player's allies. | §2.9.5, §2.9.11 | S |
| BAS-06 | An owned base serves only its owner and the owner's allies. | §2.9.5 | S |
| BAS-07 | An enemy tank cannot drive onto an owned base while the base has armour. | §2.9.5 | S |
| BAS-08 | Shooting a base depletes its armour stock. | §2.9.5 | S |
| BAS-09 | Once a base's armour is gone, an enemy tank can drive onto it and capture it. | §2.9.5 | S |
| BAS-10 | A tank is never refuelled by a base it is not entitled to use, even if pushed onto it. | v1.01 | H |
| BAS-11 | The object of the game is to capture all refuelling bases. | §2.9.5 | S |
| BAS-12 | The player's display shows the stocks of the nearest friendly base when one is nearby. Where bases are grouped, it shows the closest. | §2.9.5, §2.9.11, v1.04 | S |

## 10. Visibility, alliances and ownership

Players see only part of the world, and every pillbox and base is
hostile to a player unless that player or an ally owns it.

### Visibility

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| VIS-01 | Each player has a private view. No player sees another's screen or orders. | §2.1 | S |
| VIS-02 | A tank standing wholly inside forest is invisible to other players and to pillboxes. The version history sets the cover needed at a 2 by 2 block of forest and says a hidden tank is still seen by anything within 3 map squares. | §2.9.3, v1.01 | S |
| VIS-03 | Mine visibility follows MIN-04 to MIN-08. | §3.1.6 | S |
| VIS-04 | Knowledge of distant objects is limited by a visual range. The brain advantage option lifts that limit for bases, pillboxes and tanks. | §3.1.6, §6.4 | I |
| VIS-05 | For brains, a map square is reported as unknown terrain until the tank first encounters it. | v1.09 | H |

### Alliances

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| ALL-01 | Players start unallied. Everything a lone player owns is hostile to all others. | §2.9.7 | S |
| ALL-02 | A player asks to join an alliance by selecting one or more players on the Players menu and choosing Request Alliance. The command is unavailable while the requester is already in an alliance. | §2.9.7, §3.2.3, §3.6 | S |
| ALL-03 | The player asked sees a pop-up naming the requester, with accept and deny choices. The pop-up stays on top of other windows. | §2.9.7, §3.6, v1.04 | S |
| ALL-04 | On acceptance the requester is allied with every member of the accepting player's alliance. Any member can admit a newcomer. | §2.9.7, §3.6 | S |
| ALL-05 | A player can refuse all requests by unchecking Allow Alliance Request. Requester and target are then both told the request was ignored. | §3.2.3 | S |
| ALL-06 | A player can leave an alliance at any time with Leave Alliance, which is unavailable when not in one. | §2.9.7, §3.2.3 | S |
| ALL-07 | On leaving, the pillboxes and bases the leaver personally captured go with the leaver. Those captured by remaining members stay with the alliance. | §3.6 | S |
| ALL-08 | Allies' pillboxes do not fire on each other's tanks, allies share bases, and allies see the mines each other lay while allied. | §2.9.5, §2.9.7 | S |
| ALL-09 | Pending alliance requests are cleared when a new game starts. | v1.09 | H |
| ALL-10 | An alliance change is announced once to each player. | v1.03 | H |

### Ownership

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| OWN-01 | Every pillbox and base has one owner: a player, or nobody (neutral). | §2.9.11, v1.02 | S |
| OWN-02 | A pillbox changes owner only when a tank picks it up dead (PIL-11). A base changes owner only when a tank drives onto it (BAS-05, BAS-09). | §2.9.4, §2.9.5, §2.9.13 | S |
| OWN-03 | When a player leaves the game, that player's pillboxes and bases become neutral. | v1.04 | I |
| OWN-04 | A player who rejoins within the rejoin window regains former pillboxes and bases that are still neutral (GAM-12). | v1.04 | H |

## 11. Game setup and lifecycle

A game is defined by a map, one of three game types and five options,
all fixed by whoever starts it.

### Game types

The type decides what each new tank starts with (§3.1.6, v1.01).
Labels are read from the Game Setup screenshot (basis F).

| game type | label in Game Setup | shells | mines | trees | armour |
| --- | --- | --- | --- | --- | --- |
| Open | Open Game (each new tank comes pre-armed) | Full | Full | Full | Full |
| Tournament | Tournament (free ammunition early in the game) | Some; less as fewer bases remain neutral | None | None | Full |
| Strict Tournament | Strict Tournament (no free ammunition at all) | None | None (implied) | None (implied) | Full |

### Options

| ID | option | effect | server default | source |
| --- | --- | --- | --- | --- |
| GAM-01 | Map | The built-in Everard Island or a chosen map file. | None; one must be given | §3.1.6, §3.3 |
| GAM-02 | Allow Hidden Mines | On: MIN-05 and MIN-06 apply. Off: MIN-07 applies. | On | §3.1.6, §3.3 |
| GAM-03 | Allow Computer Tanks | Lets players run brains. A sub-option gives brains the location of every base, pillbox and tank regardless of range. | Off | §3.1.6, §3.3 |
| GAM-04 | Game Password | Players must supply it to join. Unused in single-player. | None | §3.1.6 |
| GAM-05 | Game Start Delay, in seconds | Play does not begin until the delay has elapsed, so players can join and receive the game state first. | None | §3.1.6 |
| GAM-06 | Game Time Limit, in minutes | The game ends after this long. The Game Information window shows the time left. | None | §3.1.6, §3.1.11 |

### Lifecycle

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| GAM-07 | The client offers five ways to start: Tutorial, Single Player, TCP/IP, Local Network and Internet. | §3.1.2 | S |
| GAM-08 | Tutorial is a guided game for first-time players, driven by on-screen prompts. Its content is fixed; users cannot supply their own. | §1.1, §3.1.2, v1.04 | S |
| GAM-09 | Single Player runs a game with no network and uses the same Game Setup options. | §1.2, §3.1.2 | S |
| GAM-10 | Players may join a running game at any time unless it is full, locked, or they fail the password or name check. | §3.5.1, §3.2.3 | S |
| GAM-11 | Player names are unique within a game. A player can rename during play; a name already in use is refused with an error. | §3.1.16, §3.5.1 | S |
| GAM-12 | A player who left can rejoin within 5 minutes under the same player name and regains former pillboxes and bases that are still neutral. | v1.04 | H |
| GAM-13 | File > New leaves the current game, closes its child windows and returns to the start window. | §3.2.1, v1.06 | S |
| GAM-14 | The game ends when the time limit expires. | §3.1.6 | S |
| GAM-15 | The stated objective is to own every refuelling base (BAS-11). The manual does not say what happens when a side achieves it. | §2.9.5 | S |
| GAM-16 | The player limit is 16 unless the host sets a lower maximum. | §2.1, §3.3 | S |

## 12. Networking

Games run client/server: one server holds the master state, and every
client predicts locally and yields to the server's corrections. The
manual describes no packet formats, so the wire protocol is open
(section 21). *(treadcore note: this section is kept for reference and
is not adopted; see the preface.)*

### Model

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| NET-01 | One server per game holds the master copy of the map and game state. Clients exchange game state with the server, not with each other. | §3.6 | S |
| NET-02 | The server is authoritative and checks what clients report, to resist cheating. | v1.09, v1.14 | H |
| NET-03 | Clients predict the results of their own actions to hide latency. When a prediction disagrees with the server, the client adopts the server's version, even if that visibly rewrites recent events. | §3.6 | S |
| NET-04 | A client found out of sync is frozen in place until resynchronised (TNK-07). | v1.13 | H |
| NET-05 | When the server stops, every player is dropped. The server tells players it is exiting. | §3.6, v1.03 | S |
| NET-06 | The server keeps running with no players connected, however long. | v1.06 | H |
| NET-07 | The server binds a UDP socket and a TCP listening socket. A client needs both UDP reachability and a TCP connection to the server. | §3.5.1, §3.5.2 | S |
| NET-08 | The default game port is 27500. The client's own UDP port is configurable and also defaults to 27500. | §3.1.9, §3.3 | F |
| NET-09 | The game tolerates a router or proxy changing a client's source port mid-game. | v1.02 | H |

### Joining

1. The player gives an address and port, or picks a game in a finder.
   The address may be the server's or that of any player in the game.
2. The client contacts that machine over UDP. A player's machine
   answers with the address it uses to reach the server.
3. The client connects to the server.
4. The server admits the player or refuses with a reason.
5. The client downloads the map and game state, then enters play.

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| NET-10 | A join may be addressed to any machine in the game. A player's machine refers the newcomer to the server. | §3.5.1 | S |
| NET-11 | A player who reached the server through 127.0.0.1 is advertised to others at the server's own address. | v1.03 | H |
| NET-12 | A client joins only games running its own version. The finder blocks the attempt; a direct attempt gets no response. | v1.03, §3.5.1 | S |
| NET-13 | The server refuses a join when the game is full, the password is wrong, the player name is taken or the game is locked. The client shows the reason. | §3.5.1, §3.2.3 | S |
| NET-14 | If two players try to join at the same moment with the same name, only one succeeds. | v1.02 | H |
| NET-15 | A joining client downloads the map and game state before play. | §3.1.6, v1.01 | S |
| NET-16 | A password-protected game prompts for the password. A wrong one returns the player to the join screen. | §3.1.10 | S |
| NET-17 | A winbolo:// URL launches the client and joins the game it names. | v1.11 | H |
| NET-18 | The game looks up each player's host name by reverse DNS and shows it when available. | v1.09 | H |

### Locking

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| NET-19 | A game becomes locked when every player in it has unchecked Allow New Players (client lock). | §3.2.3, v1.03 | S |
| NET-20 | The server operator can lock and unlock the game (server lock). | §3.3 | S |
| NET-21 | A game accepts joins only when neither lock is in force. The server lock holds whatever clients choose, and the server cannot lift a client lock. | §3.3 | S |
| NET-22 | Lock and unlock are announced to players as network status messages. | v1.03 | H |

### Finding games

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| NET-23 | The LAN finder discovers games on the local network with broadcast packets. | v1.04 | H |
| NET-24 | A game started from the LAN finder never notifies the tracker. | §3.1.8 | S |
| NET-25 | With Use Tracker on, a host notifies the tracker periodically so the game appears in the Internet finder. | §3.1.8 | S |
| NET-26 | The Internet finder queries the configured tracker for the game list even when Use Tracker is off. | §3.1.8 | S |
| NET-27 | The default tracker is tracker.winbolo.com, port 50000. | §3.1.8, §3.3 | F |
| NET-28 | For each listed game the finder shows server address, server port, version, map name, number of players, game type, free bases, free pillboxes, hidden mines, password, and brains (no, yes, or yes with advantage). | §3.1.7 | F |
| NET-29 | The tracker supplies a message of the day that the finder can display. | §3.1.7 | S |
| NET-30 | A tracker reply is awaited for up to 10 seconds. | v1.03 | H |
| NET-31 | A tracker host name is resolved before the game starts. If the lookup fails, a warning is shown and tracker notification is disabled for that game. | §3.5.1, §3.6 | S |
| NET-32 | In a game-information packet, a server address of 0.0.0.0 means the packet came from the server itself, and the receiver uses the packet's source address. | v1.01 | H |

### WinBolo.net

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| NET-33 | Reporting to WinBolo.net is optional per player and per game. The player sets a WinBolo.net password and an on/off switch before entering a game. | §3.1.5, §4 | S |
| NET-34 | A server can disable WinBolo.net reporting for its game. | §3.3 | S |
| NET-35 | A participating game reports its statistics, lock status and version. | §3.3, v1.14 | S |
| NET-36 | Participating players are marked with an asterisk beside their names. | v1.13 | H |

### Diagnostics

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| NET-37 | The Network Information window shows server address, this client's address, server ping in milliseconds, packets per second per player, network status and network error count. | §3.1.13 | F |
| NET-38 | Ping is never reported as negative. Corrupt packets are detected and reported. | v1.01 | H |

## 13. Main window, views and controls

The main window puts the map in the centre, builder mode buttons on
the left and status displays on the right, and uses one colour code
throughout: red for hostile, green for friendly.

### Layout

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| UI-01 | The centre of the main window is a top-down view of part of the map. | §2.9.11 | S |
| UI-02 | Five builder mode buttons sit in a column at the left, in the order given in section 6. | §2.9.6 | S |
| UI-03 | Info boxes at the right show one status icon per tank, per pillbox and per base in the game. | §2.9.11 | S |
| UI-04 | Below the base icons, three bars show the shells, mines and armour held by the nearest friendly base, when one is nearby. | §2.9.11 | S |
| UI-05 | Beside them, four bars show the tank's own shells, mines, armour and trees. | §2.9.11 | S |
| UI-06 | A message pane shows incoming messages and scrolls. | §3.2.2, v1.09 | S |
| UI-07 | A menu bar offers File, Edit, WinBolo, Players, Brains and Help (section 15). | §3.2 | S |

### Colour and symbol code

All rows are from §2.9.11 unless noted.

| where | symbol | meaning |
| --- | --- | --- |
| Info boxes | Red square | Hostile tank, pillbox or base |
| Info boxes | Green circle | Friendly tank, pillbox or base |
| Info boxes | Hollow circle | The player's own tank |
| Info boxes | Chequered red square | Neutral pillbox |
| Info boxes | Chequered green circle | Neutral base |
| Info boxes | Small version of an icon | A pillbox being carried by a hostile tank, a friendly tank or the player. One carried by an ally is a filled circle, not hollow (v1.03). |
| Map | Tank turret red, green or black | Hostile, friendly, or the player's own |
| Map | Pillbox guns red or green | Hostile or friendly. The drawing also shows damage. |
| Map | Base marking dotted, green or red | Neutral, friendly or hostile |

### Views

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| UI-08 | Tank view is the normal view. With Automatic Scrolling on, it keeps the player's tank at the centre and follows nothing else. | §3.2.2, §3.6 | S |
| UI-09 | The scroll keys move the view by hand. This suspends automatic scrolling until the view reaches the map edge or the tank view key is pressed. | v1.09 | H |
| UI-10 | Pillbox view shows the surroundings of a pillbox owned by the player or the player's alliance. Each press of its key moves to the next such pillbox. | §2.9.12 | S |
| UI-11 | In pillbox view the arrow keys jump to the neighbouring pillbox in that direction. | §2.9.12, v1.09 | S |
| UI-12 | Re-entering pillbox view starts from the pillbox last viewed. | v1.07 | H |
| UI-13 | The tank view key returns to the normal view. | §2.9.12 | S |
| UI-14 | The window can be shown at normal, double or quad size on Windows. Skin graphics are 16 pixels per square at normal size and 32 at double. | §3.2.2, §3.9, v1.04 | S |
| UI-15 | The redraw rate is adjustable by the player. | §3.2.2 | S |
| UI-16 | Tanks on screen can carry a name label of selectable length. Pillboxes and bases can carry labels, intended for brain authors. | §3.2.2 | S |
| UI-17 | The main map view can be hidden to save processing when several copies run on one machine. | §3.7 | S |

### Default keys

| action | default key | source | note |
| --- | --- | --- | --- |
| Accelerate | Q | §2.9.1 | |
| Decelerate | A | §2.9.1 | |
| Turn anticlockwise | O | §2.9.1 | |
| Turn clockwise | P | §2.9.1 | |
| Shoot | Space | §2.9.1 | |
| Lay mine | Tab | §2.9.1, §2.9.9 | The Key Setup screenshot appears to show Shift; see section 20 |
| Gun range longer | + | §3.1.14 screenshot | The text of §2.9.10 is garbled; see section 20 |
| Gun range shorter | - | §3.1.14 screenshot | As above |
| Tank view | Return | §2.9.12 | |
| Pillbox view | The apostrophe and quotation-mark key | §2.9.12 | |
| Scroll up, down, left, right | Arrow keys | §3.1.14 screenshot | |

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| UI-18 | Every action above can be rebound in Key Setup. Each action takes exactly one key. | §2.9.1, §3.6 | S |
| UI-19 | Alt, Print Screen and Pause cannot be bound. | §3.1.14 | F |
| UI-20 | Key bindings and menu options are saved between sessions. | v1.05 | H |
| UI-21 | Key handling works on international keyboard layouts. | v1.06 | H |

### Messages

| message class | content | player control | source |
| --- | --- | --- | --- |
| Player messages | Text typed by other players, tagged with the sender's name | Sender label length | §3.1.15, §3.2.2 |
| Newswire | Game events, such as a player capturing a base | On or off | §3.2.3 |
| Assistant | Reports from the player's builder | On or off | §3.2.3 |
| AI brain | Output from the player's active brain | On or off | §3.2.3 |
| Network status | Server notices such as lock, unlock and shutdown | On or off | §3.2.3, v1.03 |
| Network debug | Diagnostic detail for error reports | On or off | §3.2.3 |

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| UI-22 | A player sends text to all players, all allies, all nearby tanks, or the players selected on the Players menu. | §3.1.15 | S |
| UI-23 | After a message is sent, sending is blocked briefly to prevent flooding: at least 1 second. | §3.1.15, v1.03 | S |

## 14. Dialogs

Every secondary window of the client is listed here with its controls
and the rule behind it. Control names come from the screenshots (basis
F); behaviour comes from the text.

| ID | window | controls | behaviour | source |
| --- | --- | --- | --- | --- |
| DLG-01 | Network Selection | One choice of Tutorial, Practice (single player), TCP/IP, Local Network (broadcast search) or Internet (tracker search). Buttons WinBolo.net, Skins, Languages, Help, OK, Quit. A "Skip this Dialog Next Time" tick box. | Shown at start-up and after File > New. OK goes on: Tutorial starts play, Practice opens Game Setup, TCP/IP opens UDP Setup, the other two open the game finder. Help opens the manual. Quit exits. | §3.1.2 |
| DLG-02 | Language Select | List of language sets. Panel with the selected set's name, author and notes. OK, Cancel. | Lists English (default) plus each language file found. Text missing from a set falls back to English. | §3.1.3 |
| DLG-03 | Skin Selection | List of skins headed by "No Skin (Default)". Panel with name, author and notes. OK, Cancel. | Items missing from a skin fall back to the built-in graphics and sounds. | §3.1.4 |
| DLG-04 | WinBolo.net Settings | Password. Participation on or off. | Applies to games entered afterwards (NET-33). | §3.1.5 |
| DLG-05 | Game Setup | Choose Map button with the current map's name. Three game-type choices. Allow Hidden Mines. Allow Computer Tanks, with "and give them an advantage". Game Password and its field. Game start delay in seconds. Game time limit in minutes. OK, Cancel. | Shown when starting any new game. Options as in section 11. | §3.1.6 |
| DLG-06 | Open Map | File chooser with a preview of the selected map (Windows only). | Very long paths and file names are accepted. | v1.06, v1.03 |
| DLG-07 | Game Finder, LAN and Internet | List of games. Selected Game Information panel (fields in NET-28). Status line. Buttons Tracker Setup, Join By Address, Join, Set Player Name, Refresh, Rejoin, Message of the Day, New, Cancel. | Refresh starts the search and the status line reports progress. Selecting a game fills the panel. Join By Address opens UDP Setup and carries the tracker details across. | §3.1.7, v1.06 |
| DLG-08 | Tracker Config | Tracker Address. Tracker Port. Use Tracker. OK, Cancel. | NET-24 to NET-27. | §3.1.8 |
| DLG-09 | UDP Setup | Machine name or IP address (shown as localhost). UDP port of the server (27500). UDP port for this machine (27500). Player name. Remember player name. Tracker Setup. New, Join, Rejoin, Cancel. | New starts a game, Join joins the addressed game, Rejoin reclaims a place in it. Leading spaces are stripped from a pasted address:port. | §3.1.9, v1.09 |
| DLG-10 | Password | Text field. OK. | Shown on joining a protected game (NET-16). | §3.1.10 |
| DLG-11 | Alliance request | Requester's name. Accept and deny. | ALL-03. Leaving it open for a long time must be safe. | §3.6, v1.03 |
| DLG-12 | Game Information | Map name. Number of players. Game type. Hidden mines. Computer tanks allowed. Game time remaining. | Read-only. | §3.1.11 |
| DLG-13 | System Information | Processor use as percentages for simulation, communications, graphics, AI control and total. Graphics frames per second. | Counts only the client's own processing time. | §3.1.12, §3.6 |
| DLG-14 | Network Information | Fields in NET-37. | Read-only. | §3.1.13 |
| DLG-15 | Key Setup | One entry per action, grouped as Drive Tank, Rotate Tank, Gun Range, Weapons, Change Views and Scroll. Auto Slowdown. Automatic show and hide of gunsight. OK, Cancel. | Select an action; the next key pressed is bound to it (UI-18, UI-19). | §3.1.14 |
| DLG-16 | Compose Message | Four recipient choices (UI-22). A line stating how many players will receive the message. Text field. Send. | Return also sends. The text field has focus on opening and focus returns to the main window on closing. Send is disabled briefly after each message. | §3.1.15, v1.01 |
| DLG-17 | Set Player Name | Text field holding the current name, pre-selected. OK. | Refused with an error if the name is in use. | §3.1.16, v1.10 |

## 15. Menus

The menu bar has six menus, and every item in the manual's reference
(§3.2) is listed here with its effect.

| menu | item | kind | effect |
| --- | --- | --- | --- |
| File | New | Command | Leaves the game and returns to Network Selection (GAM-13). |
| File | Save Map | Command | Saves the map in its current state to a file. |
| File | Game Information | Command | Opens DLG-12. |
| File | System Information | Command | Opens DLG-13. |
| File | Network Information | Command | Opens DLG-14. |
| File | Exit | Command | Quits the game and the program. |
| Edit | Frame Rate | Choice | Sets how many times a second the screen is redrawn. |
| Edit | Window Size | Choice | Sets window scaling (UI-14). |
| Edit | Automatic Scrolling | Toggle | Keeps the tank centred (UI-08). |
| Edit | Show Gunsight | Toggle | Shows or hides the gunsight. |
| Edit | Message Sender Names | Choice | Sets the length of sender labels in the message pane. |
| Edit | Tank Labels | Choice | Sets the length of name labels beside tanks. |
| Edit | Pillbox Labels | Toggle | Draws labels on pillboxes. |
| Edit | Base Labels | Toggle | Draws labels on bases. |
| WinBolo | Allow New Players | Toggle | This player's vote on the client lock (NET-19). |
| WinBolo | Allow Alliance Request | Toggle | Accepts or ignores alliance requests (ALL-05). |
| WinBolo | Set Keys | Command | Opens DLG-15. |
| WinBolo | Change Player Name | Command | Opens DLG-17. |
| WinBolo | Sound Effects | Toggle | Turns sound effects on or off. |
| WinBolo | Background Sound | Toggle | Plays sound while the program is not the active application. Windows only. |
| WinBolo | ISA Sound Card | Toggle | Performance aid for old sound hardware. Windows only, on by default, unavailable when sound could not start (§3.6). |
| WinBolo | Newswire Messages | Toggle | Shows or hides game event messages. |
| WinBolo | Assistant Messages | Toggle | Shows or hides builder messages. |
| WinBolo | AI Brain Messages | Toggle | Shows or hides messages from the active brain. |
| WinBolo | Network Status Messages | Toggle | Shows or hides server notices. On by default (v1.03). |
| WinBolo | Network Debug Messages | Toggle | Shows or hides network diagnostics. |
| WinBolo | Request Alliance | Command | Sends a request to the players marked on the Players menu (ALL-02). Unavailable while in an alliance. |
| WinBolo | Leave Alliance | Command | Leaves the current alliance (ALL-06). Unavailable while not in one. |
| Players | Send Message | Command | Opens DLG-16. |
| Players | Select All | Command | Marks every player. |
| Players | Select None | Command | Clears all marks. |
| Players | Select Allies | Command | Marks the player's allies. |
| Players | Select Nearby Tanks | Command | Marks players whose tanks are close by. |
| Players | One entry per occupied player slot | Toggle | Shows that player's name and marks or unmarks the player. |
| Brains | Manual | Choice | Stops any running brain. |
| Brains | One entry per brain found | Choice | Starts that brain. A brain may add menus of its own. |
| Help | Help | Command | Opens the help file. |
| Help | About | Command | Shows copyright and shareware information. |

The manual also mentions a "Hide Main view" command (§3.7) without
placing it on any menu. Brains is a top-level menu, not a submenu
(v1.01).

## 16. Brains

A brain is a plug-in the client loads to drive a tank or assist its
player. The manual says what brains may do but not how they are
programmed; it defers that to sample code, which is outside this spec's
source.

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| BRN-01 | Brains are found at start-up in the Brains folder beside the program and listed on the Brains menu. On Linux a brains folder inside .linbolo in the user's home directory is searched as well. | §2.10, §3.4 | S |
| BRN-02 | A missing Brains folder produces a non-fatal warning and no brains are loaded. | §3.5.1 | S |
| BRN-03 | One client runs at most one brain at a time. | §2.10 | S |
| BRN-04 | A brain can be started only in a game that allows computer tanks. | §3.1.6 | S |
| BRN-05 | A brain can take full control of the tank (as opponent, ally or stand-in while the player is away) or share control with the player. | §2.10 | S |
| BRN-06 | A brain can aim and fire, clear mines, direct building and farming, draw information over the game view and add its own menus. | §2.10, §3.2.5 | S |
| BRN-07 | Choosing Manual on the Brains menu returns full control to the player. | §3.2.5 | S |
| BRN-08 | A brain receives all messages sent to its player, player names, pillbox strengths, tank headings and whether the tank is obstructed. | v1.10, v1.09, v1.06, v1.05 | H |
| BRN-09 | Pillbox and base positions are given to brains as the centre of the square. | v1.06 | H |
| BRN-10 | Terrain the tank has not yet encountered is reported to the brain as unknown (TERRAIN_UNKNOWN). | v1.09 | H |
| BRN-11 | A brain is not told the armour of an enemy base. The direction field of the base's ObjectInfo record carries 0 instead, or 1 if the base cannot be captured. | v1.09 | H |
| BRN-12 | With the advantage option, a brain is given the position of every base, pillbox and tank on the map regardless of range. | §3.1.6 | S |
| BRN-13 | A brain can post messages to its player. | §3.2.3 | S |
| BRN-14 | The standard distribution ships with no working brains. | §2.10 | S |

## 17. Dedicated server

The dedicated server is a console program configured entirely by
command-line flags, then steered by typed commands and, on Linux, by
signals (§3.3).

### Command line

| flag | argument | required | default | meaning |
| --- | --- | --- | --- | --- |
| -map | Path to a map file | One of -map or -inbuilt | | Map to load. Paths containing spaces go in quotation marks (§3.5.2). |
| -inbuilt | None | As above | | Use the built-in Everard Island map. |
| -port | Port number | Yes | | Port the server runs on. |
| -gametype | Open, Tournament or Strict | Yes | | Game type. |
| -mines | yes or no | No | yes | Allow hidden mines. |
| -ai | no, yes or yesAdv | No | no | Allow brains. yesAdv also gives them the advantage. |
| -delay | Seconds | No | No delay | Start delay. |
| -limit | Minutes, or -1 for none | No | No limit | Game time limit. |
| -password | Text | No | No password | Game password. |
| -tracker | address:port | No | Not stated | Tracker to notify. |
| -addr | IP address | No | Not stated | Address to use when the host has more than one. |
| -maxplayers | Number | No | Not stated | Most players allowed. Joins beyond it are refused with an error. |
| -logfile | File name | No | Screen | Writes server output to the file instead of the screen. |
| -quiet | None | No | Off | No screen output and no keyboard input. |
| -noinput | None | No | Off | No keyboard input; output unchanged. |
| -nowinbolonet | None | No | Off | Does not report the game to WinBolo.net. |

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| SRV-01 | Run with no arguments, the server prints its list of valid options. | §3.5.2 | S |
| SRV-02 | Given a bad parameter, the server names it where it can and prints the list of valid options. | §3.5.2 | S |
| SRV-03 | Flag names and values are matched without regard to letter case. | §3.3 | I |
| SRV-04 | On shutdown the server notifies all players (NET-05). On Linux it releases its listening ports promptly. | v1.03, v1.09 | H |

### Console commands

| command | effect |
| --- | --- |
| lock | Stops new players joining (NET-20). |
| unlock | Lifts the server lock. |
| savemap | Saves the map in its current state. |
| say | Sends a message to every player in the game. |
| info | Prints details of the game and the current player list (v1.14). |
| help | Shows help. |
| quit | Shuts the server down. |

### Linux signals

| signal | effect |
| --- | --- |
| SIGINT | Graceful shutdown |
| SIGUSR1 | Lock the game |
| SIGUSR2 | Unlock the game |

## 18. Customisation and files

Sounds, graphics and interface text can each be replaced by putting
files in named places, and anything missing falls back to the built-in
version.

### Installed layout

From §2.6, §2.7 and §3.4.

| item | Windows | Linux | purpose |
| --- | --- | --- | --- |
| Client | WinBolo.exe | Linbolo | Game program |
| Server | WinBoloDS.exe | Linbolods | Dedicated server |
| Sounds file | BoloSounds.bsd | BoloSounds.bsd | Built-in sound effects |
| Brains folder | Brains | brains | Plug-in brains |
| Languages folder | Languages | None | Language sets |
| Maps folder | Maps | maps | Map files |
| Sound override folder | Sounds | None | Replacement sound files |
| Manual folder | Manual | manual | The manual as a PDF, opened by Help |
| Sample code folder | Sample Code | sample_code | Material for brain and tool authors |
| Per-user folder | None | .linbolo in the home directory | Preferences and a second brains folder |
| Display font | System font Courier | cour.ttf in the program folder | Required; the program will not run without it |

### Sound events

There are 24 sound effects. Each file name below takes the extension
.wav (§3.8).

| game event | variants |
| --- | --- |
| Tank shooting | tank_shoot_self, tank_shoot_near, tank_shoot_far |
| Tank hit | tank_hit_self, tank_hit_near, tank_hit_far |
| Tank sinking | tank_sink_near, tank_sink_far |
| Explosion | explosion_near, explosion_far |
| Mine exploding | mine_explosion_near, mine_explosion_far |
| Mine being laid | mine_near |
| Building shot | shoot_wall_near, shoot_wall_far |
| Tree shot | shoot_tree_near, shoot_tree_far |
| Builder farming | farm_near, farm_far |
| Builder building | build_near, build_far |
| Builder dying | man_dying_near, man_dying_far |
| Tank in water losing shells and mines | bubbles |

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| CUS-01 | Each event plays the variant that fits: the player's own action, one nearby, or one far away. | §3.8 | S |
| CUS-02 | On Windows, a .wav file with a listed name in a Sounds folder beside the program replaces that effect. A missing file leaves the built-in effect in place. | §3.8 | S |
| CUS-03 | If the sounds file is missing, or the sound device is busy or absent, the game says so and runs silently. | §3.5.1, v1.01 | S |

### Skins

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| CUS-04 | A skin is an ordinary zip archive named with the extension .wsf or .zip. | §3.9 | S |
| CUS-05 | Skin files must sit at the top level of the archive. Anything inside a folder is ignored. | §3.9 | S |
| CUS-06 | Every skin file is optional. A missing one falls back to the built-in default. | §3.9, §3.1.4 | S |
| CUS-07 | A skin can replace the game graphics, the window background and any of the 24 sounds. | §3.9 | S |

| file in the archive | content |
| --- | --- |
| skin.bmp | Windows bitmap of the in-game graphics, 16 by 16 pixels per image |
| skin32.bmp | The same sheet at double size, 32 by 32 pixels per image. Used for the double-size window and never rescaled for other sizes. |
| screen.bmp | Windows bitmap of the window background |
| Any of the 24 .wav names above | Replacement sound |
| skin.ini | A [Skin] section with Name=, Author= and Notes= lines, shown in Skin Selection |

### Language sets

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| CUS-08 | At start-up the Windows client reads language set files, extension .wbl, from the Languages folder. | §3.1.3 | S |
| CUS-09 | A language set translates window text and in-game messages and carries a name, an author and notes. | §3.1.3 | S |
| CUS-10 | Any text a set omits is shown in English. Removing the folder leaves English only. | §3.1.3 | S |
| CUS-11 | English is built in. The screenshot also lists Dutch, Swedish, French and German sets. | §3.1.3 | F |

### Preferences

| ID | requirement | source | basis |
| --- | --- | --- | --- |
| CUS-12 | Key bindings, options and menu settings persist in an ini-format preferences file. | v1.05 | H |
| CUS-13 | On Linux, preferences live in .linbolo in the user's home directory, created on first run. | §3.4 | S |
| CUS-14 | An unreadable preferences file is replaced with defaults and the player is told. | v1.01 | H |

## 19. Error and warning conditions

The implementation must tell these failures apart and report each to
the user. The wording is free, and conditions the manual ties to
particular Windows libraries are restated by their general cause (§3.5
unless noted).

| ID | condition | when | severity | required behaviour |
| --- | --- | --- | --- | --- |
| ERR-01 | Brains folder missing | Client start | Warning | Continue without brains. |
| ERR-02 | Display font unavailable | Client start | Fatal | Explain and stop. |
| ERR-03 | Display colour depth below 16-bit | Client start | Fatal | Ask the user to raise the colour depth. |
| ERR-04 | Window, graphics or input set-up fails for another reason | Client start | Fatal | Report an internal set-up failure. |
| ERR-05 | Sounds file not found | Client start | Warning | Continue without sound. |
| ERR-06 | Sound device absent or in use by another program | Client start | Warning | Continue without sound. |
| ERR-07 | Networking cannot start, or a socket cannot be created | Client host or join; server start | Fatal for that attempt | Report the failure. |
| ERR-08 | Port already in use | Client or server | Fatal for that attempt | Tell the user to choose another port. |
| ERR-09 | No reply from the addressed machine | Join | Join fails | Name the three likely causes: wrong address or port, a firewall blocking UDP, a version mismatch. |
| ERR-10 | Game full | Join | Join fails | Say the game is full. |
| ERR-11 | The addressed machine replied but the server could not be reached | Join | Join fails | Suggest another machine or a retry after a few seconds. |
| ERR-12 | Wrong password | Join | Join fails | Say so and return to the join screen. |
| ERR-13 | Player name already in use | Join or rename | Refused | Ask for another name. |
| ERR-14 | Connected but not admitted | Join | Join fails | Say the join was refused, most likely over the player name. |
| ERR-15 | Tracker host name does not resolve | Client or server | Warning | Disable tracker notification and continue. |
| ERR-16 | Map file invalid or unreadable | Server start | Fatal | Report that the simulation could not start. |
| ERR-17 | Server threads cannot start, or a socket cannot be set to listen | Server start | Fatal | Report the failure. |
| ERR-18 | Bad command-line parameter | Server start | Fatal | Name it and print the valid options (SRV-02). |
| ERR-19 | Client out of sync with the server (v1.13) | During play | Recoverable | Show an error in the main window and freeze the tank until resynchronised. |
| ERR-20 | Selected game runs a different version (v1.03) | Game finder | Refused | Do not attempt the join. |

## 20. Contradictions in the source

The manual contradicts itself or is garbled in 17 places that matter
to an implementer. Each row gives both statements and the reading this
spec adopts.

| no. | topic | the manual says | it also says | reading adopted |
| --- | --- | --- | --- | --- |
| 1 | Default key for laying a mine | Tab (§2.9.1, §2.9.9) | The Key Setup screenshot appears to show Shift (§3.1.14) | Tab. The screenshot is hard to read and may show a customised set-up. |
| 2 | Gun range keys | "+" shortens the range and "9" lengthens it, in a sentence that looks damaged (§2.9.10) | The Key Setup screenshot shows "+" for increase and "-" for decrease (§3.1.14) | "+" longer, "-" shorter. |
| 3 | Number of start modes | Six options (§3.1.2) | Five are listed, and five appear in the screenshot | Five (GAM-07). |
| 4 | Extra buttons in Network Selection | "Two other buttons" (§3.1.2) | Four are then named: Language, WinBolo.net, Skins, Help | Four, as in DLG-01. |
| 5 | Server example, mines and brains | The example passes -ai yes -mines no (§3.3) | Its explanation says hidden mines are allowed and brains are not | The flag definitions win: brains allowed, hidden mines off. |
| 6 | Server example, tracker host | tracker.winbolo.com:50000 in the command (§3.3) | "bolo.zoid.com on port 50000" in the explanation | tracker.winbolo.com, which matches the Tracker Config screenshot. |
| 7 | Forest concealment | A tank must be completely enclosed by forest on all sides (§2.9.3) | Tanks hide in a 2 by 2 block of trees, down from 3 by 3, and stay visible within 3 squares (v1.01) | The later, more precise rule (VIS-02). |
| 8 | Message flood limit | Send is disabled for a few seconds (§3.1.15) | One message per second (v1.03) | A floor of 1 second (UI-23); the exact figure is open. |
| 9 | savemap | A command typed while the server runs (§3.3) | A "-savemap" server command, written like a flag (v1.13) | Console command. |
| 10 | Name of the menu holding alliance commands | "Bolo" menu (§2.9.7, §3.6) | "WinBolo" menu (§3.2.3) | WinBolo. |
| 11 | Alliance request switch | "Allow Alliance Request" (§3.2.3) | "Ignore Alliance Request" (v1.11) | Allow Alliance Request: same function, opposite sense. |
| 12 | Base passability | Bases have a medium top speed (§2.9.3) | Enemy tanks cannot drive onto a base that has armour (§2.9.5) | Medium for tanks allowed on; a barrier to enemies until armour is gone (BAS-07). |
| 13 | Binding a key | Click the box beside the action, then press the new key (§3.1.14) | Tick the action, then a small window asks for the key (§3.6) | Either interaction; the outcome in UI-18 is what matters. |
| 14 | Client lock and server lock | A server lock overrides a client lock (§3.3) | The server cannot unlock a game the clients have locked (§3.3) | Two independent locks; joins need both open (NET-21). |
| 15 | Forming a two-player alliance | Both players select Request Alliance (§2.9.7) | One request plus an acceptance forms an alliance (§2.9.7, §3.6) | One request and one acceptance are enough. |
| 16 | Linux brains folders | The sentence naming them is damaged (§3.4) | A brains folder exists in both the program folder and .linbolo (§2.7, §3.4) | Both are searched (BRN-01). |
| 17 | Section numbers | Skins are cited as section 3.8, How to Play as 2.7 and Brains as 2.9 (§1.2, §2, §3.1.4) | In the body they are 3.9, 2.9 and 2.10 | Body numbering throughout this spec. |

## 21. Unspecified behaviour

The manual gives almost no numbers and no file or packet formats, so a
working build needs every item below settled by the spec owner and
recorded here before it is implemented. *(treadcore note: each number
treadcore needs becomes a constant with a sentence in `rules.rs`.)*

| area | not specified in the manual |
| --- | --- |
| Tank motion | Top speed for each terrain class. Acceleration, deceleration and turn rates. The speed needed to leave a boat. Tank size relative to a map square and its hit boundary. How many headings a tank can face. |
| Tank stocks | Capacity of shells, mines, armour and trees. Starting amounts in each game type. The Tournament formula linking free shells to neutral bases. |
| Shooting | Shell speed, minimum and maximum gun range, range step and firing rate. Damage per hit to tanks, pillboxes, bases and buildings. Hits needed to turn a damaged building into rubble. |
| Tank destruction | The exact condition. What the player sees. The delay before the new tank. Where carried pillboxes land. How kills are scored and shown. |
| Tank collisions | Whether and how tanks block or push each other. BAS-10 implies pushing is possible. |
| Boats | What a boat becomes when its tank drives ashore. Where a newly built boat appears. Whether a moored boat can be shot. |
| Terrain results | What forest becomes when cleared. What a destroyed bridge and a sunk boat leave behind. Whether rubble can be rebuilt. Forest regrowth rate. The rate at which water drains shells and mines. |
| Builder | Running speed and time per task. Trees gained per farm. Whether a mine can be buried in forest. Parachute delay and speed. The radius at which an explosion kills him. |
| Mines | Damage to tanks and to surrounding squares. Width and layout of the edge minefield. The outcome of laying on a square that already holds a mine. |
| Pillboxes | Strength, range, starting and maximum fire rate, anger build-up and decay, first-shot delay and aiming method. |
| Bases | Stock capacities, refuel rate and delay, replenish rate. The distance within which the nearest base's stocks are displayed. |
| Visibility | Visual range. The radius meant by "nearby" for messages, sounds, quick-mine sighting and Select Nearby Tanks. |
| Game end | What happens when one side holds every base or when time runs out, and whether a winner is declared. |
| Maps | File format. World dimensions. How start squares, pillboxes and bases are encoded. The layout of Everard Island. |
| Network | Packet formats and update rates. The TCP port. The LAN broadcast format. The tracker protocol. The winbolo:// URL syntax. The WinBolo.net reporting interface. How passwords travel. How rejoin identifies a player beyond the name. The anti-cheat rules. |
| Files | Formats of BoloSounds.bsd, .wbl language sets and the preferences file. The arrangement of images inside skin.bmp. The size of screen.bmp. |
| Brains | The whole plug-in interface beyond the fragments in section 16. |
| Interface | Frame-rate and label-length choices. Exact window layout and sizes. Where Hide Main View lives. The tutorial script. Limits on name, password and message length. The default state of each toggle. |
| Licensing | §2.4 sets terms for the original software. Whether they bear on a re-implementation is a legal question this spec does not answer. |

## 22. Traceability

Every part of the manual that describes behaviour maps to a section of
this spec. The parts left out are the ones listed as out of scope in
section 2.

| manual section | spec sections |
| --- | --- |
| 1 Quick Start | 11, 14 |
| 2.1 to 2.2 What Bolo and WinBolo are | 2 |
| 2.3 System Requirements | 2, for reference only |
| 2.4 Shareware Information, 2.5 History | Out of scope |
| 2.6 to 2.7 Archive contents | 2, 18 |
| 2.8 Compatibility with Macintosh Bolo | 2 |
| 2.9.1 Driving the Tank | 5, 13 |
| 2.9.2 Beginning the Game | 5 |
| 2.9.3 Terrain | 4 |
| 2.9.4 Pillboxes | 8 |
| 2.9.5 Refuelling Bases | 9 |
| 2.9.6 Farming and Building | 6, 7 |
| 2.9.7 Alliances | 10 |
| 2.9.8 Building a Fortress | 4, 7, 8 |
| 2.9.9 to 2.9.10 Mine Laying and Clearing | 5, 7 |
| 2.9.11 to 2.9.12 The Screen and Remote Views | 13 |
| 2.9.13 to 2.9.14 Notes and Other Maps | 4, 5, 6 |
| 2.10 Brains | 16 |
| 3.1 Windows | 14, with rules in 11 and 12 |
| 3.2 Menus | 15 |
| 3.3 Dedicated Server | 17 |
| 3.4 Linux Specific Information | 2, 16, 18 |
| 3.5 Error Messages | 19 |
| 3.6 Differences Between Bolo and WinBolo | 10, 12, 13 |
| 3.7 Improving Performance | 13, 15 |
| 3.8 to 3.9 Customised Sounds and Skins | 18 |
| 3.10 to 3.12 Bug Reporting, Known Issues, Future Plans | Not behaviour; 3.12 is cited once in section 2 |
| 4 WinBolo.net | 12 |
| 5 Version History | Throughout, as basis H |
| 6 Further Reference | Out of scope, except 6.4, cited in sections 5 and 10 |
