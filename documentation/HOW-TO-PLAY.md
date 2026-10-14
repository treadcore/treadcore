# How to play treadcore

The player's guide, for someone sitting down to the window for the
first time. Every picture here is taken by `make shoot` from the same
scripted match, so what you see is the build this guide ships with
(`RELEASE.md` section 6). The game's rules and numbers are in
`DESIGN.md`; the screen's reasons in `INTERFACE.md`.

## 1. Start

```
make configure
make run                 # solo: you against fifteen bots
PLAYERS=4 make run       # against three
make lan                 # a LAN game: waits for peers, elects a host
```

Or `make install`, which puts Treadcore in your desktop's application
menu (on Copal, Super+Space, under Games). If the window's system
packages are missing, `make deps` says what to install for your
distribution.

Or, from crates.io, `cargo install treadcore-client` puts the window
on your path as `treadcore`; `treadcore --solo` starts the same solo
match. The headless server is `cargo install treadcore-server`.

The window opens on a lobby screen, then the match. You are one tank
on an island of 128 by 128 tiles with up to fifteen others. Last tank
standing wins; at the time limit, the highest score.

**The first minute is yours.** For the first sixty seconds the bots
hold fire at a tank that has not moved and has not fired, so you can
sit at the start and read this. The moment you drive or shoot, you
have begun, and so have they. After the minute, a tank that stands
still is found: move after you fire.

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

- **Space** stops. **F** puts the view back on the
  tank; **W A S D** or the arrows scroll it away. The wheel zooms.

![The models close: the tank with its turret in your colour, a depot, a pillbox](../reports/images/guide-close.png)

Up close every thing on the map is a stack of small cubes: the tank's
hull and turret, a pillbox's stepped pyramid with its gun, a depot's
plate. Damage chips cubes off them, and a shell throws loose cubes
out of whatever it hits, so the state of a thing is readable from its
shape.

## 4. The orders column

![The orders column: five drone tiles, then the five builder modes with a lamp each, then return](../reports/images/guide-column.png)

Each tile is a picture of what the order makes, with its key and its
name beside it. The key is lit amber when the order can be given and
dim when it cannot (no drone left in the bay, the squad out, no
material). Click the tile or press the key; they are the same.

**The drones** (the top five). Your tank carries a bay of drones.

| key | order | what happens |
| --- | --- | --- |
| `1` | interceptor | launches one to hold station on the *bubble*, the ring round your tank, and ram any strike drone that flies in. One shot each. |
| `2` | recon | launches one to orbit the point under the cursor. It sees 12 tiles and feeds the gun: a shell at a target the recon can see lands within 0.6 tiles. |
| `3` | strike | launches the FPV at the nearest hostile tank to the cursor: it flies there and dives, 35 of 100 armor if it gets through the target's bubble. |
| `R` | recall | brings every drone home to the bay. |
| `[` `]` | bubble | shrinks or widens the ring: wide catches strikes early but thins the ring and stretches every link. |

The bay refills at a depot. The lamps along the bottom (RECON, INTCP,
STRIKE) show what is left in it.

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
| `5` | road | lays a road; on water, a bridge. |
| `6` | wall | raises a wall three levels high. Blocks tanks, shots and a pillbox's line of fire; a shell's splash knocks it to rubble. |
| `7` | pillbox | on open ground, builds one. On a pillbox: repairs it, whoever owns it; if it is dead or nobody's, captures it (ten seconds standing on it). |
| `8` | mine | buries a mine. Hidden until found; set off by a tank, a shell or a neighbouring mine; ignored by drones. |
| `9` | return | brings the squad home. |

![The squad lamps amber while it is out, the material lamps beside them](../reports/images/guide-squad.png)

The SQUAD lamps along the bottom are green while the four are aboard
and amber while they are out; MATERIAL counts what they carry, eight
at most. A squad outside the tank can be killed by a shell or a mine
blast, and comes back one man at a time at a depot.

An order the world will not take is refused, and the refusal stays on
the newswire for four seconds, or until your next order, so you can
read why:

![The newswire holding a refusal: order refused, no forest there](../reports/images/guide-refused.png)

## 6. Pillboxes and depots

Twenty neutral pillboxes start on the map and shoot anyone within
eight tiles, tanks and drones both. Shoot one dead (its pyramid
loses its gun and its top steps as its health goes), then send the
squad in pillbox mode to capture it; it then shoots for you and holds
shells for your tank. Depots resupply and repair a tank standing on
them; stand on a neutral one for ten seconds and it is yours.

![The right column: the roster, a lamp per tank, kills and deaths, a lamp per pillbox and per depot](../reports/images/guide-right.png)

The right column is the state of everyone: the roster with team, seat
and score; a round lamp per tank; your kills and deaths; a square lamp
per pillbox and a round one per depot, each in its owner's colour
(yours in your seat's colour, green an ally's, red hostile, dim
neutral, dark dead). **Click a pillbox lamp** and the view jumps to
that pillbox; the **apostrophe** steps through your own and your
allies' pillboxes, and in that view the **arrow keys** jump to the
neighbouring pillbox in that direction. **F** comes back to the
tank.

## 7. The stock

![The bottom strip: shells, the bay, the squad and material, armor and fuel, the air](../reports/images/guide-bottom.png)

Along the bottom: 24 shells as red lamps; the bay by kind; the squad
and its material; armor and fuel as bars; and the air table, which
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
one to your team, and **Tab** switches between them while you type.
Enter sends; Esc puts the line away. While it is open your keys are
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

| key | does |
| --- | --- |
| left click | drive there; with a builder mode armed, send the squad there |
| right button, held and released | aim, then fire where the cursor is |
| `1` `2` `3` | launch interceptor, recon (to the cursor), strike (at the nearest hostile to the cursor) |
| `R` | recall every drone |
| `[` `]` | bubble narrower, wider |
| `4` `5` `6` `7` `8` | arm farm, road, wall, pillbox, mine |
| `9` | the squad home |
| Esc | drop the armed mode; with none armed, quit |
| Space | stop |
| `F` | the view back on the tank |
| Enter | talk to everyone: type, Enter sends, Esc puts it away |
| Shift-Enter | talk to your team (Tab switches while typing) |
| P | the proposal list: up/down who, left/right what, Enter proposes, L leaves your side |
| Y, N | accept or turn down the proposal on the newswire |
| `W A S D`, arrows | scroll; in pillbox view the arrows jump between pillboxes |
| `'` | pillbox view: the next of yours |
| wheel | zoom |
| Tab | the events scrollback |
| M | sound off and on |

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
