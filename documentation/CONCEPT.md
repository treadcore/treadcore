# treadcore: the concept, reviewed against the brief

This document goes back to the original brief for the game, sets out
what it asked for, and shows how the specification that came out of it
is an improvement on the base concept it started from: the multiplayer
tank battle, as Bolo defined it in 1987, carried forward to the
battlefield that the war in Ukraine has produced.

## 1. The brief

The brief asked for:

1. A new kind of game, not yet made: a four-player multiplayer map.
2. A server/client protocol in which the clients negotiate which of
   them is the server by forming a connectivity route.
3. A premise drawn from modern logistics and the modern battlefield:
   every tank has a new job. The new tank fires calibrated high-calibre
   shots like artillery. It runs a drone crew that defends it from the
   drones on the battlefield, the way a navy puts anti-air on most of
   its ships. The tank's own drones are the new game.
4. Research into battlefield technology upgrades, drones, battle
   simulation visualisation, and HUD overlay telemetry: telemetry turns
   the tank into something with an aircraft-carrier effect, controlling
   the airspace around it. A direct artillery hit still kills it.
5. An overhead, RTS-style presentation based on that HUD description.
6. Rust, with Bevy; a build system of bash scripts called from
   Makefiles, with `make configure`, `make build`, and `make debug`
   capturing verbose logs for AI-assisted development and debugging.
7. A reference document on Bolo for the Macintosh: its interface,
   AppleTalk networking (a ring of peers forwarding to their
   neighbour, with new players placed to minimise ring latency), its
   map editor, its tutorial, and its Brains (external programs that
   drove a tank through the same interface a player used).

Everything in the specification traces to one of those seven lines.
The rest of this document takes them in turn.

## 2. The base concept: Bolo

Bolo's enduring idea is not tank combat. It is that a world can be
simultaneously playable, editable, networked and programmable: a
shared, mutable terrain; many independent machines in one game; and a
controller interface open to programs as well as people. Its network
was a ring because 1989 hardware and AppleTalk made a ring the cheap
thing to build, and the ring's weakness (one slow peer slows everyone,
one dropped peer breaks the forwarding path) was the price.

Its combat model was its era's: tanks drive up to things and shoot
them, pillboxes shoot back, forest hides you, bases refuel you. The
tank is a vehicle you drive.

## 3. What the recent war changed, and what the game takes from it

The reporting gathered in RESEARCH.md says four things plainly:

- **The tank's job moved.** With FPV drones accounting for more than
  two thirds of tank losses, tanks have pulled back to fire indirect
  at up to ten kilometres. Manuals now describe "roaming tanks" that
  fire a few rounds from a prepared position and move. The tank
  became a mobile gun with a sensor problem.
- **The vehicle is getting its own air arm.** Boxer with five
  interceptor drones and a dedicated drone operator; the KNDS Leopard
  concept with recon drones ahead and tube-launched interceptors;
  ASELSAN's GÖKALP and Diehl's Garmr hunting small drones from a
  vehicle; Merops in service since 2024. Counter-drone is becoming
  organic to the vehicle, the way close-in weapons are organic to a
  ship.
- **The crew sees through the drones.** IronVision and its
  descendants put the vehicle's cameras, the drones' video and the
  tactical picture on the crew's eyes under closed hatches. The
  crew's picture is telemetry.
- **Interceptors are a consumable.** Portable interceptor factories,
  sub-million-dollar interceptors, drones as munitions. Supply is part
  of the fight.

## 4. The new mobile command: the tank with tech deployment

The specification's one central decision follows from those four
points. In Bolo the tank is a vehicle you drive. In treadcore the tank
is a **mobile command**: a platform that deploys technology around
itself and is only as safe as what it has deployed. Its systems:

| system | what it is | the decision it gives the player |
| --- | --- | --- |
| the gun | indirect fire at map coordinates, 12 to 60 tiles, six seconds of flight at full range; direct fire kept for inside 10 tiles | where to shoot, and whether to shoot blind (4-tile scatter) or wait for eyes (0.6) |
| recon drones | orbit a point, see 12 tiles, 40-tile telemetry link | where to put the eyes, and how far out before the link fails |
| interceptor drones | hold station on a ring around the tank, ram what flies in; kill 90 % on a good link, 50 % on a dead one; one shot each | the bubble radius: wide meets a strike early but thins the ring and stretches every link |
| strike drones | fly to an enemy tank and dive for 35 | when the enemy's ring is thin enough to go |
| the bay | 2 recon, 6 interceptors, 3 strikes; restocked at depots, interceptors first | when to go home |
| the link | every tethered drone has a quality from 1 at the hull to 0 at its range; below 0.12 it is on its own and turns for home | everything above |

Telemetry is not decoration here. Link quality is a number the rules
use: it sets the interceptor's kill probability, it decides when a
drone gives up and comes home, and it is drawn on screen as the line
from the hull to the drone going green to red. The aircraft-carrier
effect the brief asked for is literal: the bubble is a ring the player
sets, the interceptors fly it, and anything that enters it gets a
vector from the tank's radar if the link is good.

And the brief's last clause holds: a shell comes down through the ring
regardless, and the splash takes the ring down with the tank. The
bubble does nothing to a shell; a shell does plenty to the bubble.
That asymmetry is what makes indirect fire the tank's main weapon and
recon the tank's main vulnerability.

## 5. Why this is an improvement on the base concept

**The combat has a second layer.** Bolo's fight is on the ground.
treadcore's is on the ground and in a ring of air above each tank, and
the two layers interact through one number (the link) and one weapon
(the shell). A player is managing an airspace, a supply, and a gun at
once, and every one of them is visible on the overlay.

**The network is the same idea done properly.** Bolo's ring forwarded
every packet through every peer. treadcore's clients measure the
latency between every pair, publish the rows, and run the same pure
election function on the same matrix: the host is the peer whose worst
link is best, and the route is the shortest-path tree from it. A peer
the host cannot reach is relayed through the neighbour that can, and
only that peer pays the relay. No referee, no setup, and the "ring"
only exists where it is needed. That is the connectivity route the
brief asked for.

**The Brain interface is the client interface.** Bolo's Brains were an
add-on API. In treadcore the observation a bot thinks from is the
observation the window draws, filtered by the host the same way; a bot
cannot see or do anything a keyboard cannot. An external program plays
over JSON lines. The host validates every order from every source by
the same code and reports every refusal as an event.

**The world is one thing.** The simulation is headless and
deterministic; the window, the headless server and the tests all drive
it through `step` and `observe`. `make debug` runs a full four-bot
match in seconds and writes a trace that reads as the story of the
fight, for a person or an AI with the source beside it.

**The premise is current.** Every mechanic maps to something being
fielded or trialled in 2024 to 2026, cited in RESEARCH.md. The game is
a model of a real doctrine shift, not a reskin.

## 6. What was deliberately left out of the first specification

- Alliances and team play. The seat model supports four independent
  players; teams are a rules change, not an architecture change.
- Map editing and map files. Bolo's editor was half its longevity.
  treadcore generates from a seed for now; the map is a `Vec<u8>` and
  a file format is a small step.
- Client-side prediction. Snapshots are interpolated forward by wall
  time, which hides a LAN; it will not hide the Internet.
- NAT traversal. The relay route works once peers have addresses; it
  does not find them across a NAT.
- Sound and art. Everything drawn is a rectangle or a line, on
  purpose: the overlay is the look.

## 7. Where the vision goes: Pillboys and pillboxes

The brief grew by evening. The original idea was a new take on the RTS
territory game Bolo defined: capture pillboxes, build, repair, hold
ground. What changes is where it is done from. The tank is the control
centre, and it deploys two kinds of technology:

- **Drones**, in the air: built, as above.
- **Pillboys**, on the ground: the tank's tech infantry, the crew who
  ride with it and run its systems, and who deploy from it to do the
  things a tank cannot do from inside: capture a pillbox, repair it,
  build an emplacement, fix the hull in the field.

Pillboxes return as Bolo had them, stationary automated defences, with
one addition: in this world they engage drones too, so a captured
pillbox extends the bubble over ground the tank is not on. Territory is
the ground pillboxes cover; building is roads, bridges and
emplacements from material the tank carries; repair in the field is
slower than at a depot and costs Pillboys their exposure.

The open design question is the Pillboys' control model: a unit the
player moves, or an order the tank gives ("take that pillbox") that
resolves over time, under fire, with the squad's state on the HUD
beside the drones'. The second keeps the tank as the one thing the
player drives, which is the premise. None of this is implemented yet.

## 8. The specification in one paragraph

Four players, one tank each, a 128 x 128 tile island with depots and a
neutral drone swarm. Each tank is a mobile command: an indirect-fire
gun whose accuracy is its recon drone's, a ring of interceptor drones
whose kill rate is their telemetry link's, and a bay that is a budget.
The clients elect their own host from a measured latency matrix and
route around what the host cannot reach. The world is deterministic and
headless; the window, the server, the bots and the tests all drive the
same one. Rust, Bevy for the window, `std` UDP for the wire, a Makefile
in front of bash scripts in front of cargo, and a debug trace written
to be read.
