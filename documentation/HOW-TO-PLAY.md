# How to play treadcore

The player's guide, for someone sitting down to the window for the
first time. Every picture here is taken by `make shoot` from the same
scripted match, so what you see is the build this guide ships with
(`RELEASE.md` section 6). The game's rules and numbers are in
`DESIGN.md`; the screen's reasons in `INTERFACE.md`.

## 1. Start

```
make configure
make run                 # solo: the naval match, three teams, sixteen tanks
MAP=island make run      # the generated island instead
TEAMS=0 make run         # everyone alone
PLAYERS=4 make run       # four tanks
ROOKIES=15 make run      # against fifteen bots that play badly: start here
make lan                 # a LAN game: waits for peers, elects a host
```

Or `make install`, which puts Treadcore in your desktop's application
menu (on Copal, Super+Space, under Games). If the window's system
packages are missing, `make deps` says what to install for your
distribution.

Or, from crates.io, `cargo install treadcore-client` puts the window
on your path as `treadcore`; `treadcore --solo` starts the same solo
match. The headless server is `cargo install treadcore-server`.

The window opens on a lobby screen, then the match. By default it is
the **naval match**: sixteen tanks in **three teams** (you lead team
0, the north), each team starting together out at sea on landing
craft (section 5b), with **respawns without limit** (section 6e), so
the fifteen-minute clock ends it: the highest score wins, or the last
team standing if one is wiped out first.
`--map island` (or `MAP=island`) is the generated island of 128 by
128 tiles, `--teams 0` everyone alone, `--respawns 0` no respawns:
the old default is `--map island --teams 0 --respawns 0`.

Your team plays as a fleet: a **builder** raising a castle round your
depot (walls, a gate, your pillboxes lifted in as turrets; depots stay
where the map puts them), a **guard** on it, and **wingmen** who stay
with you and fight what fights you. The other teams have a leader, a
guard, rangers and wingmen of their own. `--plain-bots` gives plain
bots instead.

Each side has a **fleet commander**: the leader of a bots' team, the
builder on yours. It keeps the side's castle plan and organises the
work: every half minute while the fort is unfinished it musters crews
("t0-builder3: fort at (40,60), crews report"), and each crew that
reports ready gets the next piece of the plan nobody has, a wall, the
road or a tower ("asks t0-wing6: build a wall at (44,57)"), or, if
it owns an outlying pillbox, a turret slot to carry it to. A crew fells
forest for the material, sends its squad and reports ready for the
next; a job not done in a minute goes to someone else, and a tile two
crews could not build is dropped from the plan. Once the castle
stands, a pillbox of an ally's (yours, say) out beyond the fort is
**hauled** in: two crews are given the same job, chain it, drag it to a
turret slot and let go. Rangers crew, and so
do your wingmen while you are at the fort; guards guard. It is all on
your newswire.

The bots **talk** in team chat (the newswire shows it): a bot that
sees an enemy calls it, `contact: t1-ranger4 at (61,40), 80 armour.
pincer, 3 of us: left, centre, right`, and each that answers says
which flank it takes and how far out it is, then comes in from that
side: left, centre and right, sixty degrees apart, so a group closes
from three sides at once.

**Chain** (**C**): hitch a chain to the nearest pillbox or depot of
your side within two and a half tiles, or let go of yours. One tank on
a chain cannot move it and is held by it; with an ally's tank on the
same chain, the two of you drag it at a tenth of your speed, over land
(a depot's tile goes with it). The pillbox is silent while on a chain.
You need not wait for an ally: chain on alone and the nearest free bot
of your side drops what it is doing, says "on my way to the chain",
drives over, hitches on, and hauls beside you wherever you drive; it
lets go when you do.
Another way to move a pillbox is still the squad's lift (pillbox mode,
Shift-click your own; section 5); a depot only moves by chain.

On the autopilot your tank also comes back by itself after it is
destroyed, as the bots' do; by hand it is **V**.

**Ping** with **Q** or the middle mouse button: a ring in your colour
marks the spot for your side (and circles spread from it on the
minimap), and your wingmen and the other bots of
your side that answer pings (all but the builder and the guard)
converge on it and attack what is there. Bots ping the enemies they
see too; their rings show on your map, and the newswire says who
pinged what. Bots, and your autopilot, also **dodge**: a shell coming
down on them is driven out from under, and a hit makes them jink.

The big **AUTOPILOT** button at the top of the right column hands the
tank to the bot and back (the same as **O**, section 6d); it is green
while the bot drives.

**The first minute is yours.** For the first sixty seconds the bots
hold fire at a tank that has not moved and has not fired, so you can
sit at the start and read this. The moment you drive or shoot, you
have begun, and so have they. After the minute, a tank that stands
still is found: move after you fire.

**Rookies for a first match.** `ROOKIES=n make run` (or `treadcore
--solo --rookies n`) makes the first n bots rookies: no drones for a
minute and a half and one interceptor after, shells that land a few
tiles off and never lead you, and no storming of your depot. They
still fight; they fight slowly. Lower the number as you win.

**Again.** When a solo match ends, the screen gives the scores, your
kills and deaths, and how many rounds you have won of how many played.
**Enter** plays again: the same island and seats, the bots reseeded so
they do not open the same way twice. Esc leaves. Over a network,
Enter takes you back to the lobby on the same port: whoever presses
Enter within twenty seconds of you meets you there, a host is elected
again, and the next match begins. A lost connection offers the same.

![The match-over screen of a solo match: the winner, the scores, kills and deaths, rounds won, and Enter to play again](../reports/images/guide-over.png)

## 2. The screen

![The whole frame: the orders column on the left, the map in the middle, the roster and lamps on the right, the stock along the bottom, the newswire under it](../reports/images/guide-screen.png)

The layout is the prior art's (Bolo's, `INTERFACE.md`): orders down
the left, the map in the middle with your tank at its centre, the
state of everyone and everything down the right, your tank's stock
along the bottom, and one line of news under it all. Nothing is ever
drawn over the map unless you ask for it.

![The top strip: who you are, the tick and seconds, the network counters](../reports/images/guide-top.png)

The top strip says who you are and which seat, the tick and the
seconds, and how the network is doing. It goes red, with `UNDER
FIRE`, for three seconds after a hit lands on you.

## 3. Driving and firing

The mouse drives and fires; the keys give orders.

- **Left click** on the map drives there. The world routes you round
  water, over bridges and round hostile pillboxes; the line from your
  tank shows the leg in progress and, fainter, the rest of the route.
- **Hold the right button** and the gun's reach lights: the two
  orange rings are the indirect gun's minimum and maximum (12 to 60
  tiles), the white one is direct fire's reach (10 tiles). Release to
  fire where the cursor is. Inside the white ring with a clear line
  you fire direct; otherwise the shell is lobbed, six seconds in the
  air at full range, and lands within four tiles of the point unless a
  recon drone can see it, when it lands within one.

![Aiming: the right button held, the gun's envelope lit as rings round the tank](../reports/images/guide-aim.png)

A hit on you flashes the map's frame red and shakes the view for a
moment. Watch the newswire for what hit you and what is left.

- **H** stops. **Space** hands the tank to its own brain and opens
  the coordination screen (section 6d). **F** puts the view back on the
  tank; **W A S D** or the arrows scroll it away. The wheel zooms.

![The models close: the tank with its turret in your colour, a depot, a pillbox](../reports/images/guide-close.png)

Up close every thing on the map is a stack of small cubes: the tank's
hull and turret, a pillbox's stepped pyramid with its gun, a depot's
plate. Damage chips cubes off them, and a shell throws loose cubes
out of whatever it hits, so the state of a thing is readable from its
shape.

## 4. The orders column

![The orders column: five drone tiles, then the five builder modes with a lamp each, then return](../reports/images/guide-column.png)

The column is one row of icons down the left edge, no words: each
tile is a picture of what the order makes, with its key in a round
badge on its corner. The key is lit amber when the order can be given
and dim when it cannot (no drone left in the bay, the squad out, no
material); a builder mode's badge goes red while that mode is armed.
Under the column, the works kind `B` has chosen. Click the tile or
press the key; they are the same. (The screenshot above is the older,
worded column.)

**The drones** (the top five). Your tank carries a bay of drones.

| key | order | what happens |
| --- | --- | --- |
| `1` | interceptor | launches one to hold station on the *bubble*, the ring round your tank, and ram any strike drone that flies in. One shot each. |
| `2` | recon | launches one to orbit the point under the cursor. It sees 20 tiles (more at later marks) and feeds the gun: a shell at a target the recon can see lands within 0.6 tiles. |
| `3` | strike | launches the FPV at the nearest hostile tank to the cursor: it flies there and dives, 35 of 100 armor if it gets through the target's bubble. |
| `R` | recall | brings every drone home to the bay. |
| `[` `]` | bubble | shrinks or widens the ring: wide catches strikes early but thins the ring and stretches every link. |
| `X` | longshot | (no tile) sends the long-range drone at the point under the cursor, anywhere on the map. It is slow, it flies on its own and it does not come back; section 6b. |

The bay refills at a depot. The lamps along the bottom (RECON, INTCP,
STRIKE, LONG) show what is left in it.

**Marks.** The drones get better as the match goes on, for every side
at once: at **4 minutes** every launch is a Mk II, at **8 minutes** a
Mk III, and a drone keeps the mark it was launched as. The newswire
says so, and the air panel shows the mark after the letter (`R2`,
`S3`). Each mark is modelled on a system in `RESEARCH.md`, and each
is better at one thing rather than at everything:

| | Mk I | Mk II (4 min) | Mk III (8 min) |
| --- | --- | --- | --- |
| recon | quadcopter: 150 s, sees 20 | VTOL fixed-wing: faster, 210 s, sees 23 | high-altitude VTOL: 270 s, sees 27, a little slower |
| interceptor | kinetic rammer: kills half the time on its own camera, 9 in 10 on a good link | radar-cued: 7 in 10 on its own, faster | high-speed autonomous: half again as fast, 8 in 10 on its own, shorter flight |
| strike | FPV: 45 s, 35 armour | fibre-optic FPV: 55 s, **flies through jammers** | loitering munition: 90 s, 48 armour, slower |
| longshot | one-way drone | faster, further | heavy: 40% more on what it lands on |

Sight: a tank sees 10 tiles with its own eyes (as far as its direct
gun reaches), a recon 20, an interceptor or strike 8.

## 5. The builder: the Pillboys

A squad of four rides with the tank. They take the ground: they fell
forest for material, lay roads, raise walls, build and repair and
capture pillboxes, and bury mines. Their orders are the five *modes*
in the column's lower half, in the order the prior art had them.

![Pillbox mode armed: its lamp red, the others dark](../reports/images/guide-column-armed.png)

1. **Arm a mode**: press its key or click its tile. The red lamp
   beside it is the one lamp lit in the column, so the mode is never
   in doubt. Press it again, or Esc, to drop it.
2. **Click the square.** The square under the cursor is outlined in
   the colour of what the squad would make there. The click sends
   them; the mode drops as they go, because they take one order at a
   time.

![The square under the cursor outlined while a mode is armed](../reports/images/guide-ghost.png)

| key | mode | on the square |
| --- | --- | --- |
| `4` | farm | fells the forest there: four material a tile, in four seconds. Only on forest. |
| `5` | road | lays a road; on water, a bridge. With Shift held, on water beside the bank, it builds a **boat** instead: five material (a little over one tile of forest felled), five seconds. Section 5b. |
| `6` | wall | raises a wall three levels high. Blocks tanks, shots and a pillbox's line of fire; a shell's splash knocks it to rubble. Armed with Shift held (or clicked with Shift), it pours **concrete** instead: two material, pale grey, and a splash only cracks it to a plain wall, so it takes two. |
| `7` | pillbox | on open ground, builds one. On a pillbox: repairs it, whoever owns it; if it is dead or nobody's, captures it (ten seconds standing on it). Shift-click one of **yours** and the squad lifts it (five seconds) and carries it; the next pillbox you build is that one set down, in five seconds, for no material, at the health it had. One at a time. If the squad or the tank is lost with it, it falls there, dead, still yours to mend. |
| `8` | mine | buries a mine. Hidden until found; set off by a tank, a shell or a neighbouring mine; ignored by drones. |
| `0` | works | on a pad (the dark grey foundations), builds the structure the row names; `B` steps the kind. On a structure: mends ours, or takes one that is nobody's or wrecked. Section 6b. |
| `9` | return | brings the squad home. |

![The squad lamps amber while it is out, the material lamps beside them](../reports/images/guide-squad.png)

The SQUAD lamps along the bottom are green while the four are aboard
and amber while they are out; MATERIAL counts what they carry, eight
at most. A squad outside the tank can be killed by a shell or a mine
blast, and comes back one man at a time at a depot.

An order the world will not take is refused, and the refusal stays on
the newswire for four seconds, or until your next order, so you can
read why:

![The newswire holding a refusal: the gun fired twice in a row, and the second shot refused as PEBKAC: reloading](../reports/images/guide-refused.png)

Every refusal carries a tag in front of it, the names a helpdesk has
for the fault that is not in the machine. They are a joke at the
chair's expense, never at yours: the line after the tag says what is
wrong with the order, and the scrollback (Tab) spells the tag out.

| tag | spelled out | what it covers |
| --- | --- | --- |
| `60 CM` | the error is 60 centimetres in front of the screen | aiming: inside minimum range, beyond range, no line of sight, off the map |
| `PICNIC` | Problem In Chair, Not In Computer | the world working as made: water, a tile already taken, a pillbox still alive, a structure off a pad |
| `PEBKAC` | Problem Exists Between Keyboard And Chair | what the lamps already said: no shells, reloading, no material, the bay empty, the squad out |
| `LOOSE NUT` | a loose nut behind the keyboard | the order went to the wrong thing, or one that is gone |
| `KB-TO-CHAIR` | a keyboard-to-chair interface fault | talking and treaties: one message a second, the three-way rule |
| `ID-10-T` | an ID-10-T error | the command line: a flag, a number or a file that wants a second look |

When something that was down comes back by itself (a structure's power
after its plant is mended or retaken, a dropped seat that returns),
the newswire or the host's log says so the helpdesk way: "We're back
up." "Well, it's working now. That's the main thing."

## 5b. Boats

A start out at sea puts its tank on a **landing craft** (the grey-green
hull under it). Afloat, the tank crosses water faster than open ground
but cannot pass under a bridge. Click a beach: the world routes across
the water and the tank drives up onto the sand, and the boat stays
moored on the water beside the beach for anyone to board. Nothing else waits on the boat: the squad cannot leave
it, so build and capture once ashore.

**Launching from the shore: `E`.** A tank on the beach with its squad
aboard puts a boat on the water beside it (the tile most nearly ahead)
and is aboard at once: no walk, no drive onto it. It takes up to five
of the squad's material, and launches with none. Bots on an island hop
drive to the bank and launch this way unless a moored boat is within
six tiles.

**One hit sinks a boat**, and the tank goes down with it: all of
treadcore's water is deep. The shooter has the kill. A tank on a boat
is the easiest target on the map, so get to land.

To go back to sea, have the squad build a boat (road mode, `5`, with
Shift, on a water square beside the bank: five material). It lies
moored in its builder's colour; drive onto it and the tank is afloat.
Any tank may board any moored boat.

The bots, and your autopilot, **island hop**. One that has stood idle
for twenty seconds with nothing hostile in sight while a depot or a
neutral or dead pillbox is still untaken goes for the nearest of them;
if the world says it cannot be reached by land, the bot boards a
moored boat nearby or has its squad fell forest and build one on the
bank facing it, sails straight for it and lands. Its status (the
autopilot's line, a `--local` match's progress lines) reads
`island hop: ...`.

Otherwise the bots do what they should: afloat, a bot drops everything and runs
for the nearest beach that is away from every enemy tank and hostile
pillbox it knows of, and changes its mind only for a clearly better
one. `maps/naval.map` is an archipelago where every seat starts at sea:

    MAP=maps/naval.map make run

## 6. Pillboxes and depots

Twenty neutral pillboxes start on the map and shoot anyone within
eight tiles, tanks and drones both. Shoot one dead (its pyramid
loses its gun and its top steps as its health goes), then send the
squad in pillbox mode to capture it; it then shoots for you and holds
shells for your tank. Depots resupply and repair a tank standing on
them; stand on a neutral one for ten seconds and it is yours.

![The right column: the roster, a lamp per tank, kills and deaths, a lamp per pillbox and per depot, and the WORKS row, empty on a map with no economy](../reports/images/guide-right.png)

The right column, under the autopilot and your stock (section 7), is
the state of everyone: your kills and deaths; a round lamp per tank; a
square lamp per pillbox and a round one per depot, each in its owner's colour
(yours in your seat's colour, green an ally's, red hostile, dim
neutral, dark dead). **Click a pillbox lamp** and the view jumps to
that pillbox; the **apostrophe** steps through your own and your
allies' pillboxes, and in that view the **arrow keys** jump to the
neighbouring pillbox in that direction. **F** comes back to the
tank.

**Taking one, by the numbers.** A pillbox has 60 health and reaches
8 tiles. Your direct gun reaches 10 and does 45. So stop between 8
and 10 tiles from it, with a clear line, and two shots kill it while
it cannot touch you; then press `7`, click it, and the squad stands on
it for ten seconds. A pillbox never shoots at the squad. A neutral
pillbox does not have to be shot at all: keep the tank outside its
eight tiles and send the squad to take it live. Or let the tank do
it: press **G** (section 6d).

## 6b. Industry, and losing it

Some maps have an economy: dark grey **pads** of foundations with
**structures** on them (`INDUSTRY=1` gives the generated island one;
the world maps have their own). A structure is taken as a pillbox is,
by the squad standing on it while it is nobody's or wrecked, and it
works for your side while it stands:

| structure | what it does for your side | needs power |
| --- | --- | --- |
| power plant | powers your side's structures within 18 tiles | no |
| refinery | your tanks burn half the fuel; within 3 tiles, fuel and shells | yes |
| data centre | a blind shell aimed within 40 tiles scatters half as far; hostile drones within 30 tiles are on your screen | yes |
| workshop | within 3 tiles, armour mended twice as fast as at a depot and the bay restocked, longshots first; pillboxes and structures within 8 mend slowly | yes |
| jammer | no drone within 7 tiles keeps its link, **yours included**: recon and strikes turn for home, a longshot lands off its point | yes |

Every standing structure scores its owner two points every ten
seconds, so at the time limit the side with the economy is ahead. A
structure has twice a pillbox's health, so a shell takes a while; a
**longshot** (`X`) takes two thirds of it at once. The longshot is the
cheap one-way drone that crosses the map: slow (three tiles a second),
on its own from launch, so no recall and no new target, and it goes on
if your tank dies. Interceptors and pillboxes shoot it down like a
strike, so a parked tank and a ring of pillboxes defend a refinery.
Nothing defends all of them. Wreck a power plant and everything it
powered goes dark without being touched.

![Sixteen seats on the industrial map, fifty seconds in: the power plant handed to seat 0 has been wrecked by longshots, the newswire holds the alert in red, and the WORKS row shows the plant dark and the refinery amber, without power](../reports/images/guide-alert.png)

The structures on the map are labelled with their name and health,
and DARK when they have no power. Whatever of your side is hit,
wrecked, taken or goes dark is an **alert**: a line on the newswire
in red, saying what and where, held there five seconds over whatever
comes after it, with the alarm and a red blink on the minimap. **J**
jumps the view to the newest alert.

The **WORKS** row in the right column is your side's structures, one
lamp each, yours first: your colour (an ally's green) while it works,
amber when it has no power, red under half its health, dark wrecked.
Over eight seats the column folds the tank lamps away and sets the
roster smaller, so the depots and the works still fit.

The **roster** is under them, with team, seat and score, as many
seats as the window's height leaves room for.

The **minimap** at the foot of the right column is the whole map:
the ground dimmed where your side cannot see, every depot, pillbox
and structure in its owner's colour, the tanks you can see, your own
in white. Click it, or hold the button and drag, to move the view.

## 6c. Tank speak

The newswire and the bots talk like a crew that has been at it all
night: cyberpunk, a little l33t, with the FPV pilots' words for the
flies and some of the old tank crews' for the rig. The people at the
controls are **geekers**. Kills, launches, lost flies, jamming, the
end of a match and the bots' calls are said several ways; which way
is picked by the event's own number, never by the world's dice, so a
match plays the same whatever is said in it. The words:

| tank speak | plain |
| --- | --- |
| geeker | a player, at the controls |
| rig, pigiron | a tank |
| slugs | shells |
| flies | drones |
| whoop | an interceptor: small, ducted, in the bubble |
| toothpick | a recon: ultra-light, eyes only |
| kami | a strike drone: one way, into a rig |
| loiter, loitering munition | a longshot: slow, far, one way, at a fixed point |
| grunts | the squad of pillboys |
| scrap | material |
| pack | a fly's battery |
| fuzz | a jammer, and what it does to a link |
| the grid | the map and the economy on it |
| kill-zone | the ground inside the enemy's flies' reach |
| cost-exchange ratio | a cheap loiter against a refinery: why the economy is the target |
| send it, rip it | commit: launch, fire, go in |
| punch out, WOT | full throttle, all at once (wide open throttle) |
| gap, shoot the gap | a tight way through, threaded |
| on the way, shot over | the artillery's calls: a slug is in the air |
| Power! | the gun is about to move: the old tank crews' warning |
| jello, ghosting | a feed shaking apart, or two feeds over each other: a link going, or jammed |
| LQ | link quality: the % beside a fly in the air line |
| failsafed, RTH | the link is gone and the fly returns to home |
| browned out | the pack sagged flat in the air |
| pancaked | shot down, flat |
| magic smoke | what comes out of a fly caught in a blast |
| VTX, VRX | the fly's video transmitter, the rig's receiver: the link |
| OSD | the on-screen display: the frame around the map |
| rekt, pwned, fragged, uninstalled | destroyed |
| bite the dust | destroyed, the old way |
| going in hot | storming a depot |
| bolo crew | a crew that fails gunnery: the rookies |
| meatware, wetware | the geeker, as opposed to the rig |
| rig's green | it works again |
| tanker ears | what the M key spares |
| dronovod | a drone operator: a geeker flying |
| spotter | whoever watches for the geeker: a recon's job |
| disarm, disarm! | what is shouted when a fly goes down near its own rig |
| gl hf | good luck, have fun: a match begins |
| gg wp | good game, well played: a match ends |

Slang aimed at people (a good deal of real crew slang is) is left out
on purpose: the voice is gg and gl hf, and the joke is never at a
geeker's expense.

## 6d. Goals: commanding instead of driving

**G** steps the tank's goal, and the top strip says the goal and what
the tank is doing for it. **O** is the autopilot: it puts the tank on
auto (below) without the coordination screen, and **O** again, or a
click to drive, gives you the wheel. `--autopilot` starts a match with
it on, and on it you lead your team as a bots' leader does: you are
the side's fleet commander (you muster the crews and hand out the
fort), and your team is dealt a guard, rangers and a wingman.
`make quad` plays three windows that way, one commander a team (§11).

1. **auto**: the tank picks its own goal from a ladder, below.
2. **hunt pillboxes**: the nearest pillbox not of your side. A live
   enemy one is shot dead from the ring between 8 and 10 tiles (if a
   wall or trees block the line, the tank moves round it to find
   one), then the squad takes it; a neutral one the squad takes live.
   Then the next.
3. **defend depot**: the nearest depot of your side. The tank stays
   on it, shoots what comes within twelve tiles, and has the squad
   mend the depot's pillbox.
4. **defend base**: the base under imminent threat. The base is the
   depot (or the castle) and what stands within ten tiles of it; a
   hostile tank within sixteen tiles of it, or one of its pillboxes
   firing or fired on, is a threat. The tank fights under the base's
   guns, a few tiles out toward the threat, and the squad mends the
   most hurt pillbox of the base, or takes back a dead one, between
   shots. A tank sees ten tiles and a pillbox sees nothing, so a gun
   of the base at work is how it hears of what it cannot see: it goes
   to that pillbox to look.
5. **defend pillbox**: the same about the nearest pillbox of yours,
   and the squad takes it back if it falls.
6. **hold**: the same about the spot where you pressed G.
7. **fortify**: a castle's gate round the nearest depot of your side,
   facing the nearest depot that is not, or the map's middle. The
   squad builds, felling forest for material as it goes: first three
   pillboxes, two guard towers four tiles out either side of the gate
   and one behind the depot, each inside the others' eight tiles;
   then a curtain wall five tiles out across the threat's side, with
   the gate left open on the line to the threat; then a road from the
   depot out through the gate. A tank coming from the front is routed
   through the gate, between the towers. About five minutes, alone.
   The tank guards the depot meanwhile, and the top strip counts what
   stands.
8. **escort**, when an ally's tank is in sight: the wingman goal
   below, following that ally.

Then back to the wheel. A click to drive or H takes the wheel back
at once. Under every goal the tank keeps its drones up as a bot
would, and goes home to a depot when its shells or armour run low.

**Auto, the ladder.** Six rungs, tried from the top every tick; the
first that is on and whose condition holds is what the tank does:

| rung | when | what |
| --- | --- | --- |
| 1 defend base | a threat to the base, as above | defend base |
| 2 fortify | the castle round the base is not yet standing | fortify |
| 3 turrets | the castle stands, the base has been quiet twenty seconds, and a pillbox of yours stands more than ten tiles out | drive to it, lift it, carry it home, set it down in a **turret**: a bastion at each end of the curtain wall, walled in on the outside, where an attacker would go round the wall instead of through the gate. Two turrets, then the wall tiles that close them in. |
| 4 hunt | a pillbox not of your side within thirty tiles of the base | hunt pillboxes |
| 5 expand | a depot not of your side, or a pillbox anywhere | the depot nearest the base: its guard pillboxes shot dead, then the tank sits on it until it is yours; with none left, the pillboxes far out |
| 6 guard | always | on guard at the base, mending |

So alone on a quiet corner the tank builds the castle, takes the
pillboxes near it, brings them home into the turrets, and drops
whatever it is doing the moment something comes for the base.

**The coordination screen.** **Space** opens it over the map and puts
the tank on auto (or keeps the goal it has); Space again gives you the
wheel. On it:

- each rung, `[x]` on or `[ ]` off, with `>` at the one now in use and
  beside each what its condition sees ("quiet 30 s", "pillboxes 3/3,
  wall 6/10", "a pillbox of ours out at 16 tiles");
- you: armour, shells, the squad and whether it carries a pillbox;
- every ally in sight: how far, armour, shells.

`1` to `6` turn the rungs on and off (turn off 4 and the tank never
leaves home; turn off 2 and it never builds). `G` gives it one goal
instead of the ladder. A click on the map holds the tank there. A
right click sets the tank's own **push point** there (right click it
again to go back to the fleet's). Tab
folds the screen away to see the map under it, and back; Esc closes
it as Space does. While it is up the drone and builder keys wait.

**The weights page.** `U` on the coordination screen opens the
weights page: a row for each part of the brain that can be switched
off, and a slider for each of its weights. Up and down pick a row,
left and right move it a step (a switch goes off and on), Delete puts
it back to its default, Enter saves, and `U` or Esc returns to the
ladder. A `*` marks what you have changed, and the line under the rows
says what the picked one does. It is your team's: your tank on auto
and, when your window hosts them, your side's bots, at once and at
every new match. The enemy's bots keep theirs. Enter keeps it in
`~/.config/treadcore/weights`, read again the next time you start.
When your window hosts bots of your side (`--fleets`, `--wingmen`),
the screen lists them under FLEET with what each is doing.

**Rotation: the push point and the defense point.** A tank on auto
does not wait about. Every one has a defense point, the post at its base, and
a push point, halfway from the base to the side's objective (the
nearest depot not yours, else the nearest pillbox not yours) and
moved onto ground a tank can drive to. A side shares its base, so it
shares the push point: the fleet's. When there is nothing to shoot,
mend, build or fetch, the tank rotates: out to the push point, a look
round, home to the defense point (staying to refill if it is short of
shells or armour), a look round, out again. A guard rotates too, and
comes home the moment the base is threatened. A wingman whose leader
has stood still 8 s rotates as well, between its slot and a point
toward the objective no further than 9 tiles from the leader (inside
the leader's gathering ring, so the fleet never waits on it), and
closes up the moment the leader moves. On the coordination
screen ROTATION gives both points, and the map shows them: a ring at
the defense point, a cross at the push point.

**Fleets.** `--fleets` (`FLEETS=1 make run`) deals sixteen seats into
three teams of five neighbours round the map's edge, every team
building at its leader's depot, and one aggressive bot alone (seat
15, "aggro"). You lead team 0 from the north; your four are a builder
on auto whose castle goes round *your* depot, a guard on defend base
there, and two wingmen on you (`t0-builder`, `t0-guard`, `t0-wing`).
A bot team's leader is on auto at its own depot, with a guard, two
rangers (auto with fortify and turrets off: they defend the base and
hunt near it) and a wingman. Squads of one side on the same pillbox
take it together, and tanks of one side on a depot do not contest it.
When a team's leader dies, its wingmen and guard take command: each
goes to auto at the same base, so a fleet fights on to the end.

Under fire from something nobody can see (shells from beyond a tank's
ten tiles), the tank under any goal scoots a few tiles, puts its
recon over the place the shells came from, and fires back there.

**Fleet tactics.** A team's bots and the autopilot fight by a set of
weights (`treadcore-brain/src/tactics.rs`, `Tactics::default`), and
say why. The status line ends `| target NAME (score): reasons`:

- **Which tank to shoot.** Each hostile tank in sight scores for being
  near, wounded, one shot from dead, afloat (one hit sinks a boat),
  called by a teammate's ping, already under a teammate's gun, in
  range now, and threatening a teammate or the base; the target held
  scores a little more. Past two teammates on one tank, more is
  overkill and scores less, so the rest pick the next. A new target
  the team is on goes out in team chat: `focus NAME (1.9): 14 out,
  armour 40, one shot, called, 1 on it`.
- **Whether to close.** Answering a ping, a bot reckons the odds by
  Lanchester's square law: each side's tanks inside 14 tiles of the
  enemy, weighted by armour (and a hostile pillbox at 0.7 of a tank),
  squared, ours over theirs. Below 0.7 with teammates on the way it
  holds off out of direct range and waits for them, up to 12 seconds
  (`massing, 2 coming`); above 1.5 it closes in (`pressing`).
- **Where to stand.** Of seven firing points round the enemy on its
  flank's side, the one with the fewest other hostile guns on it.
- **Who answers.** Three teammates nearer a ping than this bot are
  enough; it keeps at what it was doing. A crew on a fort job answers
  only within 40 tiles.

A drive the world refuses because the place is over the water (home
to resupply, a guard's post, a leader across the sea) now goes by
boat, the island hop's way; a ping over the water is not answered by
boat.

**Wingmen.** `--wingmen N` (`WINGMEN=n make run`) makes the first N
bots wingmen: each follows you, asks to join your side (`Y` lets it
in), and in this order covers you from a tank within ten tiles of
you, hits what is ahead of you, takes the pillboxes within sixteen
tiles of you, takes a depot near you, helps another ally's fight, and
otherwise closes up behind you. It never goes more than twenty tiles
from you. It answers yes to a proposal from you or your side, and to
anyone who has not shot at it. It leaves your side only once every
pillbox and every depot on the map has been the side's for ten
seconds, and takes what it captured with it.

## 6e. Down, revived, respawned

**Down.** A destroyed tank is *down* for twenty seconds before it is
dead. The air panel counts them (`TANK DOWN. 14s for an ally's squad
to revive you`). Meanwhile an ally can bring it back: click the
fallen tank with any builder mode and your squad walks to it and works
four seconds, for **two material**, and the tank is back where it
fell at **half armour** (its squad was lost with it; a depot refills
one). The newswire says who revived whom. Allies see their side's
downed tanks; nobody else does. Bots do it too: the ally best placed
(within 45 tiles, with a squad and the material, time enough to get
there, and no ally better placed) says `reviving NAME ...` and goes,
calling its squad in first if it is out; it does not go into a fight
the side is losing there (square-law odds under 0.5, section 6d). A
bot that goes down pings where it fell and asks for a squad. A side with a tank down and a tank standing is still in the
match.

**Respawned.** In the default match (and with `--respawns
unlimited`) a tank that is not revived comes back without limit; the
prior art's elimination is `--respawns 0`:

    treadcore --solo --respawns 3          # three more tanks each
    treadcore --solo --respawns unlimited  # no limit; the clock ends it
    treadcore-server --respawns 3 ...      # the host decides for a LAN match

When the twenty seconds down are over the air panel reads
`V to respawn` (the autopilot does it for you); press **V** and a new
tank is at your start, facing
the middle, with full shells, armour, fuel, bay and squad (on a landing
craft, if your start is at sea). Your score is kept. The newswire says
how many respawns are left. Bots come back as soon as they may. A side
is out of the match only when its tanks are destroyed and it has no
respawns left.

## 7. The stock

![The bottom strip: shells, the bay, the squad and material, armor and fuel, the air](../reports/images/guide-bottom.png)

At the top of the right column, under the autopilot: 24 shells as red
lamps; the bay by kind; the squad and its material; armor and fuel as
bars. Along the bottom, beside the newswire, the air table, which
lists every drone of yours in the air with its link to the tank and
its battery, and what hostile air is in your detection ring. A link
goes green to red with distance; a drone on a dead link flies on its
own judgement.

## 8. What happened

![The events scrollback over the map](../reports/images/guide-events.png)

The newswire shows the last thing that happened. **Tab** opens the
scrollback over the map: the last forty events, each stamped with its
tick. Tab again closes it.

**Enter** opens a line on the newswire to everyone, **Shift-Enter**
one to your team, and **Tab** steps everyone, your team, and the
tanks nearby (within fifteen tiles of yours) while you type. Start the
line with **@name** to say it to chosen players only: `@west @p5 meet
me at the depot` (a name, the start of one, or a seat). The prompt
says who will hear it and how many; for "nearby" it counts the tanks
you can see, and the world may find one more in the fog. Enter sends;
Esc puts the line away. While it is open your keys are
the line's (the mouse still drives). A message from another player
stays on the newswire for eight seconds under their name, with a soft
tone. One message a second: send again too soon and the line waits,
marked, for Enter once more.

What you cannot see you can hear. Your gun's blast and the boom where
it lands, the crack of direct fire and the ring of a round striking
armour, a drone leaving the bay, a strike drone's whoosh, a tank going
up, a pillbox firing, each from where it happened. Anything on screen
is at full strength, left or right as it lies; off screen it fades
with the distance and dulls, so the whole map is a rumble in the
background on the side it is on. Your own tank is a machine you can
hear: the breech working after each shot, the loader at a depot, a
clock ticking while you take something, a long beep when you fire
with no shells left, an alarm when your armour is low, and the tracks
as you move. A hostile shell
whistles where it will land as soon as you know of it, which is the
warning the screen cannot give for a gun you have not spotted. A
capture, a loss, the swarm, a refused order and the end of the match
have a short tone each. **M** turns the sound off and on; `--mute`
starts with it off.

Your side's tanks are drawn as one of three chassis: the **bulwark**
(a slab-sided battle tank with a long gun), the **strider** (a
twin-track raider with an open cockpit and two short guns) or the
**rattler** (a wheeled scout). **T** steps through them;
`--style rattler` starts with one. The other sides take the other two
by team, so a foe differs from a friend in shape as well as colour.
The style is your window's alone: it changes nothing in the match,
and other players see their own pick for their own side. Turrets turn
to what they fire at, treads roll as tanks drive, and tracks and scorch
marks stay on the ground a while.

**Game speed.** **`** (backquote; game-speed in Key Setup) steps the
match through 1x, 2x and 4x: the world, the bots and the match clock
all run that many times as fast, and the title bar shows `x2` or `x4`.
Only the host's window changes it (a joined window says "the host
sets it" and follows the host's pace by itself). `--speed 4` or
`SPEED=4` starts there, also for `make quad` and `treadcore-server`.
When the host cannot keep up, its ticks falling under 85% of the
speed (frames of the world skipped), the speed steps down a notch (4x to 2x to
1x) and the newswire says why; after 30 s of keeping up easily it
tries the next notch up again, never above what you asked for.

## 9. Sides

Everyone starts on a side (with `TEAMS`, a side of several; without,
a side of one), and sides change by proposal. **P** opens the list of
everyone not on your side. Up and down pick a player, left and right
pick what you propose:

- **Join**: you join their side.
- **Recruit**: they join yours.
- **Swap**: you take their side and they take yours.
- **Merge**: your whole side joins theirs.

The list says what it would do, and whether the world would refuse
it. The world keeps three sides alive at least, and lets no side hold
more than half the tanks, so not every proposal can stand. Enter
proposes; it stands thirty seconds. **L** in the list leaves your
side; Esc or P closes it.

A proposal to you or your side stands on the newswire with its
seconds left: **Y** accepts, **N** turns it down. Anyone on the side
asked may answer. Every proposal and its end is public: a betrayal is
news the moment it happens. A bot answers a person: yes, unless that
person has shot it.

## 10. Keys, all of them

These are the defaults. **K** opens Key Setup over the map: up and
down pick an action, Enter waits for its new key, Delete puts every
key back, K or Esc closes and saves. A key another action had is
swapped onto it, so every action keeps exactly one key. Alt, Print
Screen and Pause are never bound, nor Esc, Enter, the arrows and
Shift, which the menus, the chat line and concrete need. The bindings
are kept in `~/.config/treadcore/keys` (or `--keys FILE`), one
`action = key` a line, and a key is its place on the keyboard: on
AZERTY the scroll keys are still the four under your left hand. The
orders column and the hints under it show your keys, not these.

![Key Setup over the map: the actions in groups, the interceptor just moved to Q](../reports/images/guide-keys.png)

| key | does |
| --- | --- |
| left click | drive there (and take the wheel back from a goal); with a builder mode armed, send the squad there |
| right button, held and released | aim, then fire where the cursor is |
| `1` `2` `3` | launch interceptor, recon (to the cursor), strike (at the nearest hostile to the cursor) |
| `R` | recall every drone |
| `[` `]` | bubble narrower, wider |
| `4` `5` `6` `7` `8` `0` | arm farm, road, wall, pillbox, mine, works |
| Shift-`6`, or Shift-click in wall mode | concrete instead of a wall |
| `B` | the next kind of structure for the works mode |
| `X` | a longshot at the point under the cursor |
| `J` | the view to the newest alert |
| click the minimap | the view there |
| `9` | the squad home |
| Esc | drop the armed mode; with none armed, quit |
| `H` | stop; also takes the wheel back from a goal |
| Space | the coordination screen and auto, or back to the wheel |
| `U` | on the coordination screen, the weights page: your team's sliders |
| `G` | the next goal: auto, hunt pillboxes, defend depot, defend base, defend pillbox, hold, fortify, escort, the wheel |
| `O` | autopilot on and off: the tank on auto with the map clear; a click to drive also takes the wheel |
| `V` | respawn, when the match allows it and the wait is over (section 6e) |
| `Q`, middle button | ping the cursor for your side: allied bots converge and attack (section 1) |
| `C` | chain on the nearest pillbox or depot of your side in reach, or let go; two tanks of a side haul it (section 1) |
| `E` | on the shore, launch a boat beside the tank and board it (five material; section 5b) |
| Shift-click a pillbox of yours in pillbox mode | lift it, to set it down with the next pillbox built |
| `F` | the view back on the tank |
| Enter | talk to everyone: type, Enter sends, Esc puts it away |
| Shift-Enter | talk to your team (Tab steps all, team, nearby; `@name` chooses players) |
| P | the proposal list: up/down who, left/right what, Enter proposes, L leaves your side |
| Y, N | accept or turn down the proposal on the newswire |
| `W A S D`, arrows | scroll; in pillbox view the arrows jump between pillboxes |
| `'` | pillbox view: the next of yours |
| wheel | zoom |
| Tab | the events scrollback; on the coordination screen, fold it |
| M | sound off and on |
| T | your side's tank style: bulwark, strider, rattler |
| K | Key Setup: rebind any of the keys above |

## 11. Over a network

`make lan` on each machine: the windows find each other on the LAN,
measure the links between them, and elect a host among themselves;
nobody runs a server. Across the Internet one machine runs `make
introduce` and the others point at it with `INTRODUCER=host:port`.
A player whose game drops starts it again and gets the same seat
back. A group that has a machine always on can run `make listen`
there: a server that sits in the lobby, takes no seat, and hosts
whenever its links are as good as anyone's (within 10 ms of the best
player's), match after match, keeping every match's trace and replay
and showing what it is doing at `http://that-machine:47296/`. When a
player is better placed, a player hosts, as without it. `README.md`
has the details; `PROTOCOL.md` has the reasons.

`make quad` plays a networked match on one machine to watch: three
windows, one a team, each its team's commander on the autopilot, the
teams filled with bots to fifteen seats, on the naval map. The host's
log (`build/logs/quad-*-client*.log`, whichever window was elected)
has every seat's line every 30 seconds, by team, with what each bot
says it is doing and whom it is shooting and why; each window's log
has its own commander's line. `CLIENTS=4`, `TEAMS=n`, `PLAYERS=n`,
`AUTOPILOT=0` to drive, `MATCH_SECONDS=n`.

![Before the match: the window has joined the elected host and waits for the world, with the first-minute help on screen](../reports/images/guide-lobby.png)

The lobby takes a few seconds: each window finds the others, measures
every link, and they all run the same election on the same numbers,
so they agree without a vote. Then the window joins the host and
waits for the world, with the first-minute help on screen. This frame
is that moment; the peers had already been found and measured.

![Playing as a client of a headless host, sixteen seats, the network line on the top strip](../reports/images/guide-networked.png)

Playing as a client looks the same as playing solo, except the top
strip's second line:

![The top strip as a client: the host's id, the round trip, packets, and the prediction counters](../reports/images/guide-net-top.png)

`NET client` and the host's id; `rtt`, the round trip to the host;
`unacked`, `tx`, `rx` and `drop`, the packets; and `predict`, how many
ticks ahead of the host your own tank is drawn. Your tank is
predicted, so it answers your mouse at once, and every snapshot from
the host is checked against the prediction: `checked` counts them and
`mismatch` counts the ones that disagreed, which is zero unless
something is wrong. These frames were taken on one machine with an
introducer, a headless host and a bot seat, 30 ms of simulated lag
on each link.
