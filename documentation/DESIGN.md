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

treadcore is that vehicle as a game. One per player, up to sixteen
players (Bolo's number; bots in the seats people do not take), an
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
- The world has a hash: FNV-1a over its serialised state, every field,
  floats by their bits. The host logs it every fifty seconds and sends
  it in heartbeats. A match records as a replay (the scenario, every
  tick's orders, the hash every five seconds) and plays back to the
  same hashes or names the first tick where it does not; `make replay`.
- Fog of war: a player sees out to 7 tiles from the tank (half that into

  forest), 12 from a recon drone, 4 from any other drone. The observation
  a client or Brain gets is the host's world with everything outside
  those discs removed.

## The tank

100 armor, 24 shells, 600 fuel, a bay of 2 recon, 6 interceptors, 3
strikes. Drives 4 tiles a second on open ground. Sees 7 tiles.

A `Move` names a destination; the world routes to it with the map's
own A* (roads preferred, water avoided, bridges where the roads cross
the river) and the tank follows the waypoints, braking to a crawl to
pivot at a corner. A tank that finds water in front of it stops and
plans again from where it is. Neither a client nor a Brain holds the
map in its observation, so the routing has to be the world's: the
first traced match had a bot parked at the river bank for its whole
life because the straight line to the centre crossed the water beside
the bridge.

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

Seventeen: one near each of the sixteen starts, one in the centre. A
depot whose start is unseated is neutral, and worth capturing. A tank inside 2 tiles of a

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

## Pillboys and pillboxes: the decision

Decided 12 October 2026, before any code (ROADMAP step 3 asks for
that), and checked against the game this descends from. Bolo's
*builder* leaves the tank, walks to the square the player clicked,
does the work (build, repair, lay, harvest), and walks back; it can be
killed while out, and the work needs material the tank carries. Bolo's
*refuelling bases* are a different thing: the tank drives onto one to
take on shells, mines and armor, and holds it by being there; there is
no "repair the base" action. The audit of a tutorial that confused the
two (`documentation/LAB-REPORT-PILLBOYS.md`) is what fixed the
following in place.

**Pillboys are the builder: an order the tank gives, not a unit the
player drives.** The tank is the one thing the player drives; that is
the premise. The squad is state on the tank, and the player's
interaction is one order, the way a click in Bolo is one order to the
builder:

    Command::Deploy { task: PillboyTask }
    PillboyTask::Capture { pillbox } | Repair { target } | Build { what, at } | Return

The squad: `Pillboys { strength: u8 (0..PILLBOY_MAX), aboard: bool,
pos: V2, task: Option<PillboyTask>, progress: u16, material: u8 }` on
`Tank`. Deployed, it walks at `PILLBOY_SPEED` (slower than the tank,
faster than nothing) along the world's route to the task, works for
`CAPTURE_TICKS` / `REPAIR_TICKS` / `BUILD_TICKS`, and walks back.
Deployed, it is a ground target: a shell's splash or a strike drone
over it reduces `strength`; at zero the order fails and the tank
has no squad until a depot replaces it. Deploying under a thin bubble
is therefore a decision, which is the point. The HUD shows the squad
beside the drones: strength, where, what, how far along.

**Depots stay refuelling bases.** Drive onto one to resupply; stay on
another's to capture it; two tanks there contest it. No Pillboy task
touches a depot. `Repair { target }` is for pillboxes, and later for
the tank's own hull in the field (slower than a depot, at the cost of
the squad's exposure).

**Pillboxes are Bolo's, with one addition.** Stationary, owned or
neutral, with health; a gun that engages enemy tanks within
`PILLBOX_RANGE` and, in this world, enemy drones within a shorter
`PILLBOX_AIR_RANGE`, with the same kill-probability shape as an
interceptor (a link to the owner tank's radar if in range, else its
own sensor). A pillbox is captured by a squad that stays on it
uncontested for `CAPTURE_TICKS`; repaired by a squad with material;
built, as an emplacement, from material the tank carries. Territory
is the ground pillboxes cover. Pillboxes shoot drones so that a
captured pillbox extends the bubble over ground the tank is not on.

**What this does not change.** Drone and shell mechanics; the gun's
indirect and direct modes (the aim point is already a map coordinate,
which is Bolo's range cursor generalised); the tank's movement model
(acceleration, terrain-limited speed, a brake for corners), which is
Bolo's momentum model with the manual steering replaced by a routed
destination.

**Where this is implemented.** `treadcore-sim`, since 12 October 2026
(roadmap step 3, journal 0004): `Pillboys`, `Pillbox`, `PillboyTask`,
`Command::Deploy`, `step_pillboys`, `step_pillboxes`; the numbers in
`rules.rs` under "pillboys and pillboxes". The squad walks at 1.5 tiles
a second (the tank does 4), captures in 10 s, repairs in 6 for one
material, builds in 15 for two. A pillbox has 60 health, shoots tanks
within 8 tiles for 15 every two seconds and drones within 4 with an
interceptor's odds, and is captured only when neutral or dead: a live
enemy pillbox is shot dead first, by the gun, from outside its range.
Twenty neutral pillboxes start on the map, one a third of the way in on
every spoke and four round the centre depot. A tank's own route keeps
out of the ring of any live pillbox that would shoot it, at the cost
of a detour, because the spoke roads run straight through them.
Since the same night (roadmap step 6c, journal 0005) the squad also
harvests and builds. `Harvest { at }` fells a forest tile in four
seconds for four material; the tank carries eight at most. `Build`
lays a wall (one material, three seconds: impassable, blocks a flat
shot, a splash knocks it to rubble), a road (one material; a bridge
when the tile is water, built from the bank), or a mine (one material,
three seconds). A mine is Bolo's: hidden from everyone but its layer
until a tank or squad comes within a tile and a half; set off by a
tank or squad on its tile, by a shell's splash, or by a mine on an
orthogonally adjacent tile, so a line chains and a chequerboard does
not; the tank on it takes 50, anything within a tile and a half takes
20, a squad in the blast is lost, and the tile craters. Drones ignore
mines: a moat is no defence against the air. A tank's own mines never
hurt it, so a defender's moat needs no gate. A pillbox holds eight
shells for its owner's tank, drawn one a resupply cycle inside two
tiles, regrowing one every ten seconds: ammunition comes from pillboxes
as well as depots.



## Teams and alliances

Players start each on a side of their own, as in Bolo, unless the
scenario deals them into teams. An alliance is a team formed in play:
a player alone asks another; any member of the alliance asked may
accept; the newcomer takes that team; anyone may leave at any time and
is alone again. A request stands thirty seconds. Every step of it is
an event everyone may see, so there are no secret pacts.

What a team shares: its picture (an ally's tank or drone seeing a point
is as good as yours, and a shell fired at what only an ally's recon
sees is spotted), its depots and its pillboxes' shells, the knowledge
of its mines, and team chat. What a team is spared: friendly fire.
Shells, strikes, splash and mines from an ally do nothing to your tank,
squad or pillbox, and the trace says so; your own shells still hurt
you, because that is a mistake and not an ally. Interceptors and
pillboxes do not engage an ally's drones or tanks; a strike at an ally
is refused. Ownership stays per player, so what an ally captured is the
ally's, and goes with them if they leave.

The match ends when one team is left standing, or at the time limit
with the highest team score; the winning team's top scorer is the
winner. Scores stay per player, so a team's standings are its members'.
Swapping sides and recruiting mid-match, under the rule that three
sides must remain, are roadmap step 6b.

## What is not here yet

- Pillboxes, walls, mines and roads are built; the defender brain that
  uses them is roadmap step 6d.


- Map editing. The map is generated from a seed; a file format and an
  editor would let a map be authored and shared.
- Client-side prediction of the player's own tank. Snapshots are
  interpolated forward by wall time, which hides a 100 ms link; it will
  not hide a 400 ms one.
- NAT traversal. Peers across the Internet need a reachable address;
  the relay route works once they have one.
- Sound.
