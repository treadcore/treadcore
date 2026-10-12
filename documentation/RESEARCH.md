# What treadcore is modelled on

Notes from the research that shaped the rules, with sources. Most of it
is trade press and manufacturer material from 2025 and 2026; treat the
capability figures as vendor-stated. The game takes the shape of these
things, not their numbers.

## Tanks have become indirect-fire platforms

- The Adapt Institute's March 2026 analysis of the Russian tank force
  reports tanks firing indirect at ranges of up to 10 km, blurring the
  line between tank and artillery, and new manuals describing "roaming
  tanks" that fire a few rounds from a prepared position and move. Since
  2024 FPV drones have accounted for over two thirds of Russian tank
  losses. https://www.adaptinstitute.org/future-of-the-russian-tank-forces/09/03/2026/
- UNITED24 reports China developing indirect-fire capability for its
  Type 100 tank, with sensors and radar, following the Ukraine trend.
  https://united24media.com/latest-news/china-developing-indirect-fire-capabilities-for-tanks-following-war-in-ukraine-trends-12491
- Small Wars Journal argues drones are filling an artillery-shell
  shortage rather than replacing artillery, delivering battalion-mortar
  precision at tactical range. https://smallwarsjournal.com/2025/05/05/beyond-the-hype-why-drones-cannot-replace-artillery/
- New Geopolitics on artillery's new role: value is precision inside a
  kill chain of reconnaissance, fire control and drone-based targeting.
  https://www.newgeopolitics.org/2025/07/23/the-artillery-new-warfare-new-role/

In the game: the gun's primary mode is indirect, 12 to 60 tiles (300 m
to 1.5 km, scaled for a 3.2 km map). Accuracy is the recon drone's: a
spotted shot lands in 0.6 tiles, a blind one in 4. The bot shoots and
moves. Direct fire is kept for the short range.

## Armoured vehicles are getting their own drones, both kinds

- KNDS Deutschland's Boxer UAV prototype carries five Tytan Interceptor
  EOS drones for self-defence against enemy drones, with a crew of
  driver, commander and drone operator. https://militarnyi.com/en/news/interceptor-drones-boxer-armored-vehicle/
- The KNDS Leopard concept shown at Eurosatory 2026 pairs an unmanned
  turret with the crew in the hull, with reporting pointing to recon
  drones ahead and tube-launched interceptors for protection.
  https://interestingengineering.com/innovation/armored-tank-pairs-unmanned-turret-with-drone
- Hensoldt's note on the future of the tank: the vehicle's own drones
  give the crew reconnaissance beyond line of sight.
  https://www.hensoldt.net/insights/future-of-tanks-in-the-drone-era
- ASELSAN's GÖKALP: a vehicle-mounted system that finds threats by radar
  and electro-optics and launches interceptor drones that hunt on their
  own, meant to protect tanks and APCs from small drones.
  https://defence-blog.com/turkish-hunter-drone-destroys-target-by-direct-hit/
- Merops (Project Eagle): a vehicle-mounted radar, electro-optical
  sensor and interceptor launcher, in service since 2024.
  https://en.wikipedia.org/wiki/Merops_(weapon)
- OSIRIS UEB-1: a high-speed autonomous interceptor UAV from Ukraine,
  unveiled March 2026. https://en.wikipedia.org/wiki/OSIRIS_UEB-1
- Diehl's Garmr counter-drone system on a vehicle chassis at Enforce Tac
  2026, with an AI detection layer. https://www.edrmagazine.eu/enforce-tac-2026-diehl-defence-relaunches-its-garmr-counter-drone-system
- US Army AMPV 30 prototypes with a 30 mm turret, 360-degree cameras,
  target hand-off and counter-UAS, in trials from April 2026.
  https://www.armyrecognition.com/news/army-news/2026/u-s-army-evaluates-ampv-30mm-armored-vehicle-with-counter-drone-and-target-handoff-systems
- AS21 Redback in a 40 mm counter-drone configuration with SkyKeeper
  command and control, September 2026. https://www.armyrecognition.com/news/army-news/2026/lockheed-martin-uk-and-south-korea-unveil-as21-redback-40-mm-ifv-with-counter-drone-capability
- A net-launching concept for armoured vehicles as a last physical layer.
  https://nextgendefense.com/armored-vehicles-drone-catchers/

In the game: the bay holds 2 recon, 6 interceptors, 3 strikes. The
interceptor is one-shot and kinetic: it rams. Its kill probability
depends on the telemetry link to the tank, because the tank's radar is
what gives it a vector; on its own camera it gets it right half the
time. The bubble is where they hold station.

## The crew sees through the hull, and the drones' picture is the crew's

- Elbit's IronVision: a see-through-armour helmet display giving a
  closed-down crew 360-degree video, with one button switching the view
  to weapon sights, UAS video or other feeds, and tactical information
  presented in augmented reality. https://www.elbitsystems.com/air-space/aircraft-systems/helmet-mounted-display/ironvision
  https://www.militarytimes.com/news/your-military/2018/10/08/dont-open-the-hatch-ironvision-lets-crews-see-through-their-vehicle/
  https://euro-sd.com/2023/02/articles/29136/closed-down-but-what-a-view/
- Elbit's demonstrator vehicle run by a two-person crew under closed
  hatches that also operates a VTOL drone to feed its own picture.
  https://www.edrmagazine.eu/elbit-systems-demonstrates-an-innovative-armored-fighting-vehicle-operated-by-a-helmet-mounted-display

In the game: the HUD. The tank sees 7 tiles with its own eyes; the
picture the player plays from is the drones'. Each drone's link
quality, battery and task are on screen, the link drawn as a line from
tank to drone going green to red, the recon's vision as a disc, every
shell as an arc with its impact point.

## Swarms and the common operating picture

- Auterion's Nemyx swarm-control test (December 2025) and its on-screen
  mission-phase overlay. https://www.armyrecognition.com/news/aerospace-news/2025/auterions-ai-swarm-drone-strike-test-signals-the-future-battlefield-of-aerial-warfare
- Breaking Defense on real autonomy in battlefield swarms (October
  2026): drones pooling what they see so the operator confirms targets
  and redirects rather than flies. https://breakingdefense.com/2026/10/bringing-real-autonomy-to-battlefield-drone-swarms/
- Anduril's Lattice as a common operating picture connecting sensors,
  drones and weapons. https://interestingengineering.com/ai-robotics/7-ai-technologies-making-military-drone-swarms-smarter-and-deadlier
- Defense News on portable interceptor factories: interceptors as a
  consumable produced at scale. https://www.defensenews.com/news/your-military/2026/04/01/infrastructure-is-the-weapon-inside-the-race-to-build-portable-interceptor-factories/

In the game: the neutral swarm, four FPVs off a random edge every 45
seconds at the nearest tank. Interceptors are a consumable restocked at
depots, interceptors first.

## Bolo

Stuart Cheshire's Bolo (BBC Micro 1987, Macintosh later) is the
ancestor: a shared, mutable overhead world; up to sixteen players on
separate machines; external "Brains" that could drive a tank through
the same interface a player used; and a network that was a ring of
peers forwarding to their neighbour, with a new player placed where it
least increased the ring's latency. treadcore takes the shared world,
the Brain interface, and the idea that the peers organise the network
themselves, and replaces the ring with a host elected from a measured
latency matrix and a tree route from it. See documentation/PROTOCOL.md.
