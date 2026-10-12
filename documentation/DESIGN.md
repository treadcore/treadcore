# treadcore design

## Premise

The tank's job changed. In the war in Ukraine, first-person-view drones
account for more than two thirds of tank losses, and tanks on both sides
have pulled back to fire indirect at ranges of up to ten kilometres, a
role that used to be artillery's. The next generation of armoured
vehicles is being designed around that: a remotely operated turret, the
crew in the hull, and a bay of the vehicle's own drones, recon ahead and
interceptors around. The vehicle becomes a small carrier: what it owns is
the airspace in a ring around it, and what it does with its gun is reach
out past that ring.

treadcore is that vehicle as a game. One per player, four players, an
island, depots, and a neutral swarm that comes in off the edge so the
ring is never optional.

## The world

Headless and deterministic (`source/treadcore-sim`). The same seed and
the same orders in the same sequence give the same world, which is what
lets one process be authoritative and the others draw its view, what
lets a trace be replayed, and what lets a bot be tested.

- 128 x 128 tiles, one tile 25 m: a 3.2 km island.
- Terrain: grass, forest, road, water, bridge, rubble, depot. Roads are
  fast, forest slow and concealing, water impassable, rubble what a shell
  leaves.
- 20 ticks a second. A tick applies the orders for that tick, then moves
  tanks, drones and shells, resolves intercepts, strikes, impacts, depot
  capture and resupply, spawns the swarm if due, and checks for a result.
- Fog of war: a player sees out to 7 tiles from the tank (half that into
  forest), 12 from a recon drone, 4 from any other drone. The observation
  a client or Brain gets is the host's world with everything outside
  those discs removed.

## The tank

100 armor, 24 shells, 600 fuel, a bay of 2 recon, 6 interceptors, 3
strikes. Drives 4 tiles a second on open ground. Sees 7 tiles.

### The gun

Indirect: aim at any point between 12 and 60 tiles. Flight time is the
range over 10 tiles a second, so six seconds at full range. Scatter is a
uniform disc of radius 0.6 tiles if a friendly recon drone has the aim
point in view at launch, 4 tiles if not. Impact deals 70 at the centre
falling to 17 at the edge of a 2.5-tile splash, cratering the ground,
and kills every drone inside 2 tiles. Four seconds to reload.

Direct: a flat shot at anything inside 10 tiles with no forest between.
45 damage, 0.3 s flight. The old job, kept.

### The bay

| | recon | interceptor | strike |
| --- | --- | --- | --- |
| speed, tiles/s | 5 | 9 | 6.5 |
| battery, s | 150 | 90 | 45 |
| link range, tiles | 40 | 18 | 18 |
| vision, tiles | 12 | 4 | 4 |

Every tethered drone has a link quality from 1 at the tank to 0 at its
link range. Below 0.12 it has lost contact and turns for home on its
own; a strike already committed to its dive is the exception, which is
exactly what makes strikes dangerous. A drone that gets home is put back
in the bay. Battery low, it goes home first.

Interceptors orbit the tank on the *bubble*, a ring the player sets
between 2 and 12 tiles (default 5). The tank's detection reaches 4 tiles
past the ring. When a hostile drone enters detection the nearest free
interceptor is assigned to it, closes, and rams: a one-shot kinetic
kill with probability 0.5 + 0.4 x link quality. A wide bubble meets the
threat early but spreads the ring thin and runs the interceptors near
the end of their link.

Recon drones orbit a chosen point and spot for the gun. An enemy recon
drone inside the bubble gets intercepted too: it is spotting for
someone's gun.

Strikes fly to a target tank and dive for 35.

At most 10 of a player's drones in the air at once.

## Depots

Five: one near each start, one in the centre. A tank inside 2 tiles of a
friendly depot resupplies: a shell, 4 armor and 40 fuel every half
second, and one drone, interceptors first. Inside 2 tiles of anyone
else's, alone, for 10 seconds, captures it. Two tanks there: contested,
nothing happens.

## The swarm

Every 45 seconds (configurable), 4 neutral strike drones spawn at a
random map edge and fly at the nearest tank. They are the reason to
keep interceptors up even with no enemy in sight, and the reason a solo
game is a game.

## Scoring and the end

Kill 100, capture 25, intercept 5. The match ends when one tank is left
alive or at the time limit (15 minutes), highest score winning.

## Brains

A Brain gets the same observation a client draws and returns commands
the world validates exactly as a human's. `RuleBot` keeps two to four
interceptors up, one recon out toward the nearest enemy, shoots what the
recon sees and moves after firing, goes home to resupply when low, and
sends a strike at any enemy whose ring is thin. `JsonBrain` runs an
external program over stdin/stdout so a bot can be written in anything.

## What is not here yet

- Alliances and team play. The seat model supports it; the rules do not
  yet.
- Map editing. The map is generated from a seed; a file format and an
  editor would let a map be authored and shared.
- Client-side prediction of the player's own tank. Snapshots are
  interpolated forward by wall time, which hides a 100 ms link; it will
  not hide a 400 ms one.
- NAT traversal. Peers across the Internet need a reachable address;
  the relay route works once they have one.
- Sound.
