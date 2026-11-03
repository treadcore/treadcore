//! The numbers. Every tuning constant in one place, with the reason it is
//! what it is, so a balance change is a one-line diff with a sentence
//! beside it and not an archaeology project.
//!
//! Distances are tiles (25 m). Times are seconds unless the name says ticks.

use crate::{Bay, PlayerId, Tick, TICK_HZ};

// ---- the tank ----------------------------------------------------------
pub const TANK_ARMOR: f32 = 100.0;
pub const TANK_SHELLS: u16 = 24;
pub const TANK_FUEL: f32 = 600.0;
/// Tiles per second on open ground: 4 tiles/s is 100 m/s... no: 4 tiles is
/// 100 m, so this is a brisk 360 km/h. A game speed. Real tanks at 40 km/h
/// would make a 3 km map a seven-minute drive.
pub const TANK_SPEED: f32 = 4.0;
/// Tiles per second per second, up and down: momentum (Paul, 17
/// October: "more momentum, more consistent top speed ... something
/// that still moves forward during a turn"). Was 3 up and an instant
/// stop down.
pub const TANK_ACCEL: f32 = 4.0;
pub const TANK_BRAKE: f32 = 6.0;
/// Radians per second of hull turn (was 2.5).
pub const TANK_TURN: f32 = 3.0;
/// A heading error larger than this is a corner: the tank eases off,
/// down to `TURN_CRAWL` of its speed for a turn right round, and keeps
/// rolling; near the corner's waypoint it slows enough to turn inside
/// it, not swing wide of the road (and, at a bridge, into the river).
/// Was a crawl at a quarter for any corner, a pivot.
pub const TURN_BRAKE_ANGLE: f32 = 0.6;
pub const TURN_CRAWL: f32 = 0.35;
/// The way on is behind it, more than this off its heading: it pivots
/// where it stands. Rolling on through that turn put a tank facing a
/// wall into the wall, blocked, every tick (the ladder's fortifier, ten
/// minutes at one spot).
pub const TURN_PIVOT: f32 = 2.0;

pub const FUEL_PER_TILE: f32 = 1.0;
/// Own eyes: as far as the direct gun reaches, so what it can shoot it
/// can see (Paul, 16 October: "increase visual sight of all, especially
/// air units"; was 7, three tiles short of `DIRECT_RANGE`). The drones'
/// picture is still the wide one.
pub const TANK_VISION: f32 = 10.0;
/// the prior art's forest concealment (SPEC VIS-02, PIL-07): a tank inside a 2 x 2
/// block of forest is hidden from every eye and every pillbox beyond
/// this many tiles, a recon drone's included. Tree cover against the
/// air is the oldest counter to it, and it gives the squad a reason to
/// leave some forest standing.
pub const CONCEAL_RANGE: f32 = 3.0;

// ---- the gun -----------------------------------------------------------
pub const RELOAD_TICKS: u16 = 4 * TICK_HZ as u16;
/// A mortar's, on the same gun, so a lob ties up the flat shot too (Paul, 30 October: "longer
/// reload for mortars"): 99% of the shots in a match were mortars. Over eight seeds of one map, at
/// 8 s shots fell a third and strike drones took their place, up by half again; at 12 s shots
/// halved and the strikes stayed about where they were.
pub const INDIRECT_RELOAD_TICKS: u16 = 12 * TICK_HZ as u16;
/// Indirect: lobbed. 12 to 60 tiles is 300 m to 1.5 km, which on a 3.2 km
/// map means a tank in one corner can reach the centre and the two
/// neighbouring corners, but not the far one without moving.
pub const INDIRECT_MIN_RANGE: f32 = 12.0;
pub const INDIRECT_MAX_RANGE: f32 = 60.0;
/// Tiles per second along the ground track. A 60-tile shot is in the air
/// six seconds -- long enough to move if you see the launch, which you do
/// not unless a recon drone is watching the gun.
pub const SHELL_SPEED_INDIRECT: f32 = 10.0;
/// Circular error, in tiles, blind and spotted. The whole reason to fly
/// recon: a blind shot is a suppression round, a spotted one is a kill.
pub const CEP_BLIND: f32 = 4.0;
pub const CEP_SPOTTED: f32 = 0.6;
pub const SPLASH_RADIUS: f32 = 2.5;
pub const INDIRECT_DAMAGE: f32 = 70.0;
/// Direct: flat, fast, line of sight, hull-to-hull. The old job, still
/// there for when something drives into view.
pub const DIRECT_RANGE: f32 = 10.0;
pub const SHELL_SPEED_DIRECT: f32 = 40.0;
pub const DIRECT_DAMAGE: f32 = 45.0;
pub const DIRECT_HIT_RADIUS: f32 = 0.8;

// ---- the drone crew ----------------------------------------------------
pub const BAY_FULL: Bay = Bay { recon: 2, interceptor: 6, strike: 3, longshot: 2 };
/// Per player, in the air at once. Keeps the host's per-tick work bounded
/// and makes the bay a budget rather than a stream.
pub const MAX_AIRBORNE: usize = 10;
pub const RECON_BATTERY: f32 = 150.0;
pub const INTERCEPTOR_BATTERY: f32 = 90.0;
pub const STRIKE_BATTERY: f32 = 45.0;
pub const RECON_SPEED: f32 = 5.0;
pub const INTERCEPTOR_SPEED: f32 = 9.0;
pub const STRIKE_SPEED: f32 = 6.5;
/// Eyes in the air, raised with the tank's (were 12 and 4): a recon
/// sees half its link; an interceptor or a strike its own bubble's
/// width.
pub const RECON_VISION: f32 = 20.0;
pub const DRONE_VISION: f32 = 8.0;
/// Telemetry link. Link quality is 1 at the tank and 0 at this range; below
/// `LINK_FLOOR` the drone is on its own and turns for home.
pub const RECON_LINK: f32 = 40.0;
pub const DRONE_LINK: f32 = 18.0;
pub const LINK_FLOOR: f32 = 0.12;
/// The bubble: where interceptors hold station. Detection reaches a little
/// past it so an interceptor is already moving when the intruder arrives.
pub const BUBBLE_DEFAULT: f32 = 5.0;
pub const BUBBLE_MIN: f32 = 2.0;
pub const BUBBLE_MAX: f32 = 12.0;
pub const DETECT_MARGIN: f32 = 4.0;
/// Orbit angular rate scale: radians/s at radius 1, divided by radius.
pub const ORBIT_RATE: f32 = 6.0;
pub const RAM_RANGE: f32 = 0.5;
pub const RECOVER_RANGE: f32 = 0.8;
/// Kill probability for a ram: on a dead link the interceptor's own camera
/// gets it right half the time; with the tank's radar feeding it, most of
/// the time.
pub const INTERCEPT_BASE: f32 = 0.5;
pub const INTERCEPT_LINKED: f32 = 0.9;
pub const STRIKE_DAMAGE: f32 = 35.0;

// ---- the longshot (roadmap step 10c) -----------------------------------
/// The one-way drone of the talk step 10 starts from: cheap, slow, long
/// range, sent to a fixed point and on its own from launch. Three tiles
/// a second crosses a 128-tile island in under a minute and a 512-tile
/// world map in three, slow enough that a bubble on its path has time.
pub const LONGSHOT_SPEED: f32 = 3.0;
/// Seconds of flight: 450 tiles, the far corner of a 256 map and back
/// out of it.
pub const LONGSHOT_BATTERY: f32 = 150.0;
/// What it does where it lands, inside `LONGSHOT_RADIUS`: a structure
/// is large and cannot move, so it takes the warhead; a pillbox less; a
/// tank, which was rarely where the drone was sent, least.
pub const LONGSHOT_RADIUS: f32 = 1.5;
pub const LONGSHOT_STRUCTURE_DAMAGE: f32 = 80.0;
pub const LONGSHOT_PILLBOX_DAMAGE: f32 = 30.0;
pub const LONGSHOT_TANK_DAMAGE: f32 = 25.0;

// ---- ground and walls (step 10a) ---------------------------------------
/// Concrete: a wall a splash cracks to a plain wall instead of rubble.
/// Twice the material and a little longer to pour.
pub const CONCRETE_COST: u8 = 2;
pub const CONCRETE_BUILD_TICKS: u16 = 5 * TICK_HZ as u16;

// ---- structures (step 10b) ---------------------------------------------
/// Twice a pillbox: two spotted shells, or a longshot and a shell. The
/// talk's point is that infrastructure is easy to hit; it is not meant
/// to be easy to kill in one.
pub const STRUCTURE_HEALTH: f32 = 120.0;
/// A structure is a big target: a splash reaches it from this much
/// further out than it reaches a tank, and a direct shot this much wide.
pub const STRUCTURE_SIZE: f32 = 1.0;
pub const STRUCTURE_BUILD_TICKS: u16 = 20 * TICK_HZ as u16;
pub const STRUCTURE_REPAIR_TICKS: u16 = 8 * TICK_HZ as u16;
pub const STRUCTURE_REPAIR_COST: u8 = 2;
/// A captured structure is handed over at least this well repaired.
pub const STRUCTURE_CAPTURE_HEALTH: f32 = 40.0;
/// A power plant powers every structure of its side inside this.
/// Eighteen tiles: a zone and its neighbour, never the whole map, so a
/// side's economy has a few plants and each is a target worth a drone.
pub const POWER_RANGE: f32 = 18.0;
/// Refinery: a fuel and shell stop within this, and the whole side
/// burns this fraction of the fuel it would.
pub const REFINERY_RANGE: f32 = 3.0;
pub const REFINERY_FUEL_BURN: f32 = 0.5;
/// Data centre: fire control for a blind shell aimed inside
/// `DATACENTRE_FIRE_RANGE` (its scatter is multiplied by
/// `DATACENTRE_CEP_FACTOR`: computed, not seen, so better than blind
/// and nothing like spotted), and an air picture: hostile drones inside
/// `DATACENTRE_RADAR` are seen by the side.
pub const DATACENTRE_FIRE_RANGE: f32 = 40.0;
pub const DATACENTRE_CEP_FACTOR: f32 = 0.5;
pub const DATACENTRE_RADAR: f32 = 30.0;
/// Workshop: a repair stop within `WORKSHOP_RANGE` (armour at twice a
/// depot's rate, the bay restocked longshots first), and slow mending
/// of the side's pillboxes and structures within `WORKSHOP_MEND_RANGE`.
pub const WORKSHOP_RANGE: f32 = 3.0;
pub const WORKSHOP_MEND_RANGE: f32 = 8.0;
pub const WORKSHOP_MEND_PER_SECOND: f32 = 1.0;
/// Jammer: inside this every linked drone loses its link, its owner's
/// as well as the enemy's (EW cannot tell them apart; the talk's
/// airport that cannot land its own planes). A strike more than
/// `JAM_TERMINAL` from its target turns for home; inside it the dive is
/// on its own. A longshot, which has no link, has its satellite fix
/// pushed up to `JAM_DRIFT` tiles off, once. The swarm from the map's
/// edge is autonomous and flies through: the drones that already beat
/// jamming.
pub const JAMMER_RANGE: f32 = 7.0;
pub const JAM_TERMINAL: f32 = 3.0;
pub const JAM_DRIFT: f32 = 5.0;
/// Holding ground pays while it is held: every live structure scores
/// its owner this much every `HOLD_EVERY_TICKS`.
pub const SCORE_HOLD: u32 = 2;
pub const HOLD_EVERY_TICKS: u64 = 10 * TICK_HZ as u64;

// ---- pillboys and pillboxes --------------------------------------------
/// The squad: the tank's tech infantry. Four is a squad; zero is a tank
/// that cannot capture, repair or build until a depot replaces them.
pub const PILLBOY_MAX: u8 = 4;
/// Tiles per second on foot, by the world's route. A tank does 4; the
/// squad is the slow part of every order, which is what makes an order
/// a decision.
pub const PILLBOY_SPEED: f32 = 1.5;
/// Material the tank carries for the squad: trees. Harvested from
/// forest by the squad (`HARVEST_MATERIAL` a tile), restocked one at a
/// time at depots. Eight is a ring of mines, or four walls and a
/// pillbox.
pub const PILLBOY_MATERIAL_MAX: u8 = 8;
pub const PILLBOY_MATERIAL_START: u8 = 2;
/// Harvesting: the squad stands in a forest tile for this long, the
/// tile becomes grass, the tank gains this much material.
pub const HARVEST_TICKS: u16 = 4 * TICK_HZ as u16;
pub const HARVEST_MATERIAL: u8 = 4;
/// A felled tree grows back after this (Paul, 30 October: "the trees need to regrow a bit
/// more"). They had not regrown at all, so a harvested wood was gone for the match.
pub const TREE_REGROW_TICKS: u64 = 120 * TICK_HZ as u64;
/// What the squad builds, how long, for how much. The prior art's costs were
/// half a tree for a road and a wall; here material is whole and a
/// harvest is four, so a wall is a quarter of a tree.
pub const WALL_BUILD_TICKS: u16 = 3 * TICK_HZ as u16;
pub const WALL_COST: u8 = 1;
/// Walls this many, inside this many tiles, make a castle (Paul, 30 October: "a tank being in a
/// castle has better protection"): a curtain's worth, so one wall on open ground is not one.
pub const CASTLE_WALLS: usize = 8;
pub const CASTLE_R: i32 = 6;
/// What a drone does to a tank in a castle, as a share. A wall already stops a flat shot; a drone
/// flies over it, so the walls take the part of the blast they stand in the way of. The share is
/// heavy cover's -50% in Company of Heroes 2, as players report it.
pub const CASTLE_DRONE_SHARE: f32 = 0.5;
/// Friendly pillboxes this near share one castle's guns, and past `TURRETS_FREE` of them each
/// more slows them all by `TURRET_CROWD_SLOW` of a reload (Paul: "soft enforce that we dont get
/// castles of turrets"). Nothing refuses the build: a ring of eight fires as about three.
pub const TURRET_CROWD_R: f32 = 12.0;
pub const TURRETS_FREE: usize = 2;
pub const TURRET_CROWD_SLOW: f32 = 0.5;
pub const ROAD_BUILD_TICKS: u16 = 3 * TICK_HZ as u16;
pub const ROAD_COST: u8 = 1;
/// A squad laying a bridge tile lays on, straight out from where it
/// stands, up to this many tiles in the trip, a material each.
pub const BRIDGE_SPAN: usize = 3;
// ---- the chain ---------------------------------------------------------
/// How near a tank must be to hitch a chain to a pillbox or a depot.
pub const TOW_REACH: f32 = 2.5;
/// The chain's length: what is towed trails this far behind the tanks
/// pulling it, and a tank cannot get further from it than this.
pub const TOW_CHAIN: f32 = 2.0;
/// Tanks it takes to move what is chained: one alone holds it, and is
/// held by it (Paul: "it should be 10% if there are two tanks tugging
/// the chain. so this action requires an ally").
pub const TOW_HAULERS: usize = 2;
/// A tank hauling drives at this share of its speed.
pub const TOW_SPEED: f32 = 0.1;

// ---- down, revive, respawn ---------------------------------------------
/// A destroyed tank is down this long before it is dead (Paul, 16
/// October: "a tank has 20 seconds to be repaired before death and
/// respawn"): an ally's squad may revive it meanwhile.
pub const DOWNED_TICKS: u32 = 20 * TICK_HZ;
/// The revive: material from the squad that does it, the squad's work
/// at the wreck, and the armour the tank comes back with.
pub const REVIVE_COST: u8 = 2;
pub const REVIVE_TICKS: u16 = (4 * TICK_HZ) as u16;
pub const REVIVE_ARMOR: f32 = TANK_ARMOR * 0.5;
/// How long a destroyed tank waits before it may come back, when the
/// scenario allows it (`Scenario::respawns`): until it is dead, the
/// time down, in which an ally may still revive it.
pub const RESPAWN_TICKS: u32 = DOWNED_TICKS;
/// `Scenario::respawns` at this is no limit.
pub const RESPAWNS_UNLIMITED: u8 = u8::MAX;

// ---- boats (SPEC TNK-08, TNK-11 to TNK-14, the mode table) -------------
/// A boat: the prior art's five trees, so a little over one harvest.
pub const BOAT_COST: u8 = 5;
pub const BOAT_BUILD_TICKS: u16 = 5 * TICK_HZ as u16;
/// A tank afloat crosses water at this terrain cost: fast (TNK-11),
/// between a road and open ground.
pub const BOAT_MOVE_COST: f32 = 0.7;
/// How far an afloat tank's observation looks for a landing, for a
/// Brain with no map: the beaches it could make for.
pub const SHORE_SIGHT: f32 = 60.0;
/// How many headings the landing search casts, one beach per heading.
pub const SHORE_RAYS: usize = 24;
/// How far an observation looks for a place to build a boat: about the
/// squad's walk.
pub const LAUNCH_SIGHT: f32 = 16.0;
pub const MINE_LAY_TICKS: u16 = 3 * TICK_HZ as u16;
pub const MINE_COST: u8 = 1;
/// Within this of a wall or mine site the squad works from beside it,
/// never on it.
pub const BUILD_STANDOFF: f32 = 1.6;
/// A mine: The prior art's, laid by the squad. Hidden from everyone but its
/// layer until a tank or squad comes within `MINE_REVEAL`; set off by a
/// tank or squad on its tile, by a shell's splash, or by a neighbour on
/// an orthogonally adjacent tile (a chequerboard does not chain). The
/// tank on it takes `MINE_DAMAGE`; anything within `MINE_RADIUS` takes
/// `MINE_SPLASH_DAMAGE`; a squad in the blast is lost; the tile craters.
/// Drones ignore mines entirely: a moat is no defence against the air.
pub const MINE_REVEAL: f32 = 1.5;
pub const MINE_DAMAGE: f32 = 50.0;
pub const MINE_RADIUS: f32 = 1.5;
pub const MINE_SPLASH_DAMAGE: f32 = 20.0;
/// Route cost for a tile holding a mine the driver knows about: all but
/// a wall.
pub const MINE_AVOID_COST: f32 = 50.0;
/// How far the crew can see the trees: the forest tiles an Observation
/// lists for a Brain that has no map.
pub const FOREST_SIGHT: f32 = 20.0;
/// A pillbox holds shells for its owner's tank, like a small depot:
/// a tank within `DEPOT_RADIUS` of its own live pillbox takes one a
/// resupply cycle while they last; they come back slowly.
pub const PILLBOX_SHELLS: u16 = 8;
pub const PILLBOX_SHELL_REGEN_TICKS: u64 = 10 * TICK_HZ as u64;
/// GAME 5, THE LOGISTICS OF A BASE (Paul, 2 November: "each time a pillbox gets hit, it loses its
/// ammo supplies, they all go boom. And then the pillbox only has its newly created ammo"; "bases
/// recharge empty supplies"). A hit pillbox's stock goes to nothing. It fills again from its own slow
/// making (one in ten seconds, above) and from a depot of its side within `PILLBOX_SUPPLY_REACH`
/// that has shells in stock: the base's workers, abstracted to a shell moved every
/// `PILLBOX_SUPPLY_EVERY`. A besieged pillbox is dangerous for a minute and quiet after, and cutting
/// a base off from its depot starves its guns, which is what a siege is.
pub const PILLBOX_SUPPLY_REACH: f32 = 12.0;
pub const PILLBOX_SUPPLY_EVERY: u64 = 2 * TICK_HZ as u64;

// ---- game 6: ore, the smelter, the harvester ----------------------------
/// GAME 6, THE ZERO-SUM ECONOMY (Paul, 2 November: "a fun zero-sum economy, where every mineral but
/// the growing trees is finite ... bonuses for refineries, smelters and wood harvesters"; the plan's
/// chapter 13, stage 2). A map may hold deposits of ore, each with a stock that is all the ore there
/// is. On such a map a depot makes no shells of its own and a refinery's shell stop is dry: shells
/// come from a smelter of the side, one a second from the ore brought to it, into a depot of the side
/// in its reach, and from there to the tanks and the pillboxes as before. Trees stay renewable, so a
/// side can always build; a side without ore loses its guns within minutes. A map without deposits
/// plays as game 5.
/// A deposit's stock when the map file names none: thirty harvests, as many shells.
pub const DEPOSIT_STOCK: u16 = 120;
/// The squad mines a deposit as it fells a tree: it stands on it this long for this much ore, which
/// the tank carries beside its material, up to `PILLBOY_ORE_MAX`.
pub const ORE_TICKS: u16 = 6 * TICK_HZ as u16;
pub const ORE_HARVEST: u8 = 4;
pub const PILLBOY_ORE_MAX: u8 = 8;
/// A tank within this of a working smelter of its side unloads its ore into it.
pub const SMELTER_RANGE: f32 = 3.0;
/// A smelter turns one ore into one shell every `SMELTER_MAKES_EVERY`, into the nearest depot of its
/// side within `SMELTER_REACH` that has room (the base's workers, abstracted, as game 5's); with none
/// in reach the ore waits in it. It stands within `SMELTER_SITE` of its deposit, on any open ground:
/// a works is sited by its resource, not by a pad, and runs on its own crew, not the grid.
pub const SMELTER_MAKES_EVERY: u64 = TICK_HZ as u64;
pub const SMELTER_REACH: f32 = 30.0;
pub const SMELTER_SITE: f32 = 3.0;
/// The harvester stands within `HARVESTER_SITE` of forest, the same way. A harvest within
/// `HARVESTER_RANGE` of a working one of the side yields `HARVESTER_MATERIAL` rather than four, and
/// the tree grows back in `HARVESTER_REGROW_TICKS` rather than two minutes.
pub const HARVESTER_SITE: f32 = 3.0;
pub const HARVESTER_RANGE: f32 = 8.0;
pub const HARVESTER_MATERIAL: u8 = 6;
pub const HARVESTER_REGROW_TICKS: u64 = 60 * TICK_HZ as u64;

// ---- game 7: oil, the workers, supply --------------------------------------
/// GAME 7, THE LOGISTICS OF A BASE (the plan's chapter 13, stage 3). Wells of oil as deposits of ore:
/// a map object with a finite stock, pumped by the squad as ore is mined, carried to a refinery of
/// the side and unloaded beside it. A refinery with oil does what it did (the side burns half, a fuel
/// stop within three tiles); one without does neither. On a map with wells a depot's fuel comes from
/// a refinery of the side within `REFINERY_REACH` that has oil, one oil a stop; with none, the depot
/// gives `DEPOT_FUEL_DRY` a stop, a trickle of its own.
pub const WELL_STOCK: u16 = 120;
pub const OIL_TICKS: u16 = 6 * TICK_HZ as u16;
pub const OIL_HARVEST: u8 = 4;
pub const PILLBOY_OIL_MAX: u8 = 8;
pub const REFINERY_REACH: f32 = 30.0;
pub const DEPOT_FUEL_DRY: f32 = 10.0;
/// THE WORKERS: a stock in transit, no longer a number moved. A smelter's shells go to the depot on
/// a worker, and a depot's shells to its pillboxes: `WORKER_LOAD` at a time, one leaving every
/// `WORKER_EVERY`, walking the map's route at `WORKER_SPEED` on grass and quicker on a road (the
/// terrain's cost), drawn as a small figure with its load. A splash within `SPLASH_RADIUS` loses it;
/// a hostile tank within `WORKER_TAKE` takes it, the load to the tank. A base with no road to its
/// smelter is a base whose guns run dry in a siege, and cutting the road matters.
pub const WORKER_LOAD: u16 = 5;
pub const WORKER_EVERY: u64 = 10 * TICK_HZ as u64;
pub const WORKER_SPEED: f32 = 1.5;
pub const WORKER_TAKE: f32 = 2.0;
/// SUPPLY (PROPOSAL-UPGRADES 1.1, as it has it): a side's second count, earned every
/// `HOLD_EVERY_TICKS` from each live pillbox it owns, each depot, and each powered structure (every
/// second time), and at once for a kill and a capture; held by the seat, pooled by nobody. Spent on
/// upgrades (the first, depot stores: the depot holds `STORES_STOCK_MAX` of each kind and makes twice
/// a second) and, from stage 4, moved by trade.
pub const SUPPLY_PILLBOX: u32 = 1;
pub const SUPPLY_DEPOT: u32 = 2;
pub const SUPPLY_STRUCTURE: u32 = 1;
pub const SUPPLY_KILL: u32 = 10;
pub const SUPPLY_CAPTURE: u32 = 5;
pub const STORES_COST: u32 = 60;
pub const STORES_STOCK_MAX: u8 = 30;

// ---- game 8: trade ----------------------------------------------------------
/// GAME 8, TRADE (the plan's chapter 13, stage 4; the game Paul described: ports trade with ports,
/// a payout that grows with distance and is highest between allies, trade units that are captured or
/// protected). Two depots of one side (a seat's own, or its team's) trade: from `TRADE_FIRST` after
/// the start and every `TRADE_EVERY`, a convoy of supply leaves each depot for its nearest partner
/// of the side, by the map's route on land, or straight over the water when both are ports (a depot within `PORT_WATER` of
/// water). Its load is the distance times the rate, `TRADE_RATE_OWN` a tile between a seat's own two
/// depots and `TRADE_RATE_TEAM` between two seats of a side, and when it arrives both ends are paid
/// it. A convoy is a worker: a splash loses it and a hostile tank that drives up takes it, the supply
/// to the taker. A road is a thing to hold and an ally a thing to keep.
pub const TRADE_FIRST: u64 = 20 * TICK_HZ as u64;
pub const TRADE_EVERY: u64 = 60 * TICK_HZ as u64;
pub const TRADE_RATE_OWN: f32 = 0.25;
pub const TRADE_RATE_TEAM: f32 = 0.5;
pub const PORT_WATER: f32 = 2.0;
/// A convoy over the water moves at this many tiles a second.
pub const CONVOY_SEA_SPEED: f32 = 2.0;

/// A shell's splash over a deployed squad costs it this many Pillboys.
pub const PILLBOY_SPLASH_LOSS: u8 = 2;
/// Within this of the tank the squad is back aboard.
pub const PILLBOY_BOARD_RANGE: f32 = 1.0;
/// Within this of the site the squad is at work.
pub const PILLBOY_WORK_RANGE: f32 = 0.7;
/// Ticks of uncontested work: capture (the prior art's "stays on it"), a repair,
/// a new emplacement.
pub const PILLBOX_CAPTURE_TICKS: u16 = 10 * TICK_HZ as u16;
pub const PILLBOX_REPAIR_TICKS: u16 = 6 * TICK_HZ as u16;
pub const PILLBOX_BUILD_TICKS: u16 = 15 * TICK_HZ as u16;
pub const PILLBOX_REPAIR_COST: u8 = 1;
/// Moving a pillbox (step 11h): the squad lifts one of its side's own in
/// this long, carries it (one at a time, at the health it had) and sets
/// it down as a Build of a pillbox in `PILLBOX_PLACE_TICKS`, for no
/// material. The prior art's builder carried pillboxes the same way.
pub const PILLBOX_LIFT_TICKS: u16 = 5 * TICK_HZ as u16;
pub const PILLBOX_PLACE_TICKS: u16 = 5 * TICK_HZ as u16;
pub const PILLBOX_BUILD_COST: u8 = 2;
/// A pillbox: health, and a gun. Sixty health is one indirect hit at the
/// centre of the splash and a bit, or two direct hits, or four of its
/// own kind of shot.
pub const PILLBOX_HEALTH: f32 = 60.0;
/// Against tanks: hitscan at anything hostile with a clear line inside
/// this, for this much, every reload. Eight tiles is inside the tank's
/// direct range (10) and far inside indirect (12 to 60): a pillbox is
/// out-ranged by a gun that can see it and deadly to one that drives up.
pub const PILLBOX_RANGE: f32 = 8.0;
pub const PILLBOX_DAMAGE: f32 = 15.0;
pub const PILLBOX_RELOAD_TICKS: u16 = 2 * TICK_HZ as u16;
/// Against drones: the one addition to the prior art's pillbox. Short, and with
/// an interceptor's kill curve: a link to the owner tank's radar if it is
/// within `DRONE_LINK`, else the pillbox's own sensor at `INTERCEPT_BASE`.
pub const PILLBOX_AIR_RANGE: f32 = 4.0;
/// A captured pillbox is handed over at least this well repaired.
pub const PILLBOX_CAPTURE_HEALTH: f32 = 30.0;
/// No two pillboxes closer than this; none on a depot.
pub const PILLBOX_SPACING: f32 = 3.0;
/// Extra route cost per tile inside a live hostile pillbox's range: a
/// detour of five grass tiles beats one tile under the gun.
pub const PILLBOX_AVOID_COST: f32 = 5.0;

/// The owner of a neutral pillbox's shots: nobody, and never a seat.
pub const NEUTRAL: PlayerId = 254;

// ---- depots ------------------------------------------------------------

pub const DEPOT_RADIUS: f32 = 2.0;
pub const CAPTURE_TICKS: u16 = 10 * TICK_HZ as u16;
pub const RESUPPLY_EVERY: u64 = TICK_HZ as u64 / 2;
pub const REPAIR_PER_TICK: f32 = 4.0;
/// A depot makes one thing a second, in all, for all who draw on it (Paul, 30 October: "there
/// is just too much ammo"). It had handed every tank on it two shells and two drones a second,
/// without end. Armour and fuel are not ammunition, and stay at RESUPPLY_EVERY.
pub const DEPOT_MAKES_EVERY: u64 = TICK_HZ as u64;
/// What it holds of each kind. A full kind is passed over, so a second is not lost on it.
pub const DEPOT_STOCK_MAX: u8 = 20;

// ---- alliances ---------------------------------------------------------
/// A proposal (to ally, to defect, to swap, to merge) stands this long
/// before it lapses: thirty seconds, long enough to read, short enough
/// that an old one cannot be accepted into a changed situation.
pub const PROPOSAL_TICKS: Tick = 30 * TICK_HZ as Tick;
/// The three-way rule: no change of sides may leave fewer than this
/// many sides with a living tank, or give one side more than half the
/// living tanks. Three is the smallest number in which an alliance is a
/// choice rather than the end of the game.
pub const MIN_SIDES: usize = 3;


// ---- chat --------------------------------------------------------------

/// One message a second at most (SPEC UI-23: "at least 1 second").
pub const CHAT_EVERY_TICKS: Tick = TICK_HZ as Tick;
/// A message is cut to this many characters: one line of the newswire,
/// and an Events packet far inside a datagram.
pub const CHAT_MAX_CHARS: usize = 120;
/// "All nearby tanks" (UI-22): within this of the sender's tank. Twice
/// the tank's sight: a radio's reach, not its eyes.
pub const CHAT_NEARBY: f32 = 15.0;
/// A name taken with a seat is cut to this.
pub const SEAT_NAME_CHARS: usize = 16;

// ---- scoring -----------------------------------------------------------

pub const SCORE_KILL: u32 = 100;
pub const SCORE_CAPTURE: u32 = 25;
pub const SCORE_INTERCEPT: u32 = 5;

/// The neutral swarm's player id. Never a seat; never scores.
pub const SWARM_OWNER: PlayerId = 255;

pub fn drone_speed(k: crate::DroneKind) -> f32 {
    drone_spec(k, 1).speed
}

// ---- drone marks -------------------------------------------------------
// Paul, 16 October: "reference each of the drones in the prior art
// research lab report material ... similar flight times and
// capabilities, a rotation of upgrades and a variety". Every kind comes
// in three marks, each after a system in documentation/RESEARCH.md; the
// whole match moves up a mark together at `MARK_II_AT` and
// `MARK_III_AT`, and a drone keeps the mark it was launched with. The
// game takes the systems' shape, not their numbers: flight times are
// scaled to a 3.2 km map as the guns are (RESEARCH.md).

/// When the match moves to Mk II and Mk III drones.
pub const MARK_II_AT: u32 = 4 * 60 * TICK_HZ;
pub const MARK_III_AT: u32 = 8 * 60 * TICK_HZ;

/// The mark launched at this tick.
pub fn mark_at(tick: u64) -> u8 {
    if tick >= MARK_III_AT as u64 {
        3
    } else if tick >= MARK_II_AT as u64 {
        2
    } else {
        1
    }
}

/// What a drone of a kind and mark is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DroneSpec {
    /// What it is modelled on, for the newswire and the HUD.
    pub name: &'static str,
    pub speed: f32,
    /// Seconds of flight.
    pub battery: f32,
    /// How far it sees, tiles.
    pub vision: f32,
    /// An interceptor's kill chance on its own camera and on a perfect
    /// link (a ram); unused by the others.
    pub intercept_base: f32,
    pub intercept_linked: f32,
    /// A strike's damage on the tank it dives on; a longshot's is
    /// scaled by `blast` on everything it lands on.
    pub damage: f32,
    pub blast: f32,
    /// Flies through a jammer: a fibre-optic link has no radio to jam.
    pub jam_proof: bool,
}

const SPEC_DEFAULT: DroneSpec = DroneSpec { name: "", speed: 0.0, battery: 0.0, vision: DRONE_VISION, intercept_base: INTERCEPT_BASE, intercept_linked: INTERCEPT_LINKED, damage: 0.0, blast: 1.0, jam_proof: false };

/// The table. Mark 1 is the game as it was; 2 and 3 add range, speed or
/// a trick each, never all three, so a later mark is better and also
/// different:
///
/// | kind | Mk I | Mk II | Mk III |
/// | --- | --- | --- | --- |
/// | recon | quadcopter | VTOL fixed-wing (the closed-hatch demonstrator's VTOL): faster, longer, sees further | high-altitude VTOL: further still, slower to turn on station |
/// | interceptor | kinetic rammer (the five-EOS carrier, the Turkish hunter) | radar-cued (Merops): better on its own camera | high-speed autonomous (OSIRIS UEB-1): much faster, best on its own |
/// | strike | FPV | fibre-optic FPV: unjammable, a little longer | loitering munition: twice the flight, a heavier warhead, slower dive |
/// | longshot | one-way drone | faster, further | heavier warhead |
pub fn drone_spec(k: crate::DroneKind, mark: u8) -> DroneSpec {
    use crate::DroneKind::*;
    let s = SPEC_DEFAULT;
    match (k, mark.clamp(1, 3)) {
        (Recon, 1) => DroneSpec { name: "quadcopter", speed: RECON_SPEED, battery: RECON_BATTERY, vision: RECON_VISION, ..s },
        (Recon, 2) => DroneSpec { name: "VTOL fixed-wing", speed: 6.5, battery: 210.0, vision: RECON_VISION + 3.0, ..s },
        (Recon, _) => DroneSpec { name: "high-altitude VTOL", speed: 6.0, battery: 270.0, vision: RECON_VISION + 7.0, ..s },
        (Interceptor, 1) => DroneSpec { name: "kinetic rammer", speed: INTERCEPTOR_SPEED, battery: INTERCEPTOR_BATTERY, ..s },
        (Interceptor, 2) => DroneSpec { name: "radar-cued interceptor", speed: 10.0, battery: 100.0, intercept_base: 0.68, intercept_linked: 0.92, ..s },
        (Interceptor, _) => DroneSpec { name: "high-speed interceptor", speed: 14.0, battery: 80.0, intercept_base: 0.78, intercept_linked: 0.95, ..s },
        (Strike, 1) => DroneSpec { name: "FPV", speed: STRIKE_SPEED, battery: STRIKE_BATTERY, damage: STRIKE_DAMAGE, ..s },
        (Strike, 2) => DroneSpec { name: "fibre-optic FPV", speed: STRIKE_SPEED, battery: 55.0, damage: STRIKE_DAMAGE, jam_proof: true, ..s },
        (Strike, _) => DroneSpec { name: "loitering munition", speed: 5.5, battery: 90.0, damage: 48.0, ..s },
        (Longshot, 1) => DroneSpec { name: "one-way drone", speed: LONGSHOT_SPEED, battery: LONGSHOT_BATTERY, ..s },
        (Longshot, 2) => DroneSpec { name: "fast one-way drone", speed: 4.0, battery: 170.0, ..s },
        (Longshot, _) => DroneSpec { name: "heavy one-way drone", speed: 3.5, battery: 190.0, blast: 1.4, ..s },
    }
}

// ---- the scale (Paul, 2 November: "a scale system so we can get ballistic arcs and drone speed") --
/// A tile in metres, and how many real seconds a game second stands for. THE GAME IS A GAME SPEED,
/// NOT A SCALE MODEL: at 25 m a tile its vehicles and drones fit the systems they are modelled on at
/// a tempo of about six (a tank at 4 tiles a second is 360 km/h in game time and 60 km/h at tempo
/// 6; a recon quadcopter 75 km/h; an FPV 97 km/h), and the two exceptions are kept on purpose: the
/// longshot is four times too slow, so a bubble on its path has time, and the gun is short and its
/// shell flies twice as long, so the island is a battlefield and a shell can be moved from. `physical`
/// is the table in real units, computed from the constants so it cannot go stale; a map may one day
/// declare its own scale (europe at 320 tiles is a theatre, not a field), and nothing here is a rule.
pub const METRES_PER_TILE: f32 = 25.0;
pub const TEMPO: f32 = 6.0;

/// One row of the physical table: what, the game's number and unit, the real number and unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Physical {
    pub what: &'static str,
    pub game: f32,
    pub game_unit: &'static str,
    pub real: f32,
    pub real_unit: &'static str,
}

fn kmh(tiles_per_s: f32) -> f32 {
    tiles_per_s * METRES_PER_TILE * 3.6 / TEMPO
}

/// The constants in real units at `METRES_PER_TILE` and `TEMPO`: the table the plan's chapter 9 prints.
pub fn physical() -> Vec<Physical> {
    use crate::DroneKind::*;
    let row = |what: &'static str, game: f32, game_unit: &'static str, real: f32, real_unit: &'static str| Physical { what, game, game_unit, real, real_unit };
    let m = |tiles: f32| tiles * METRES_PER_TILE;
    let flight = INDIRECT_MAX_RANGE / SHELL_SPEED_INDIRECT;
    let mut v = vec![
        row("tank speed", TANK_SPEED, "tiles/s", kmh(TANK_SPEED), "km/h"),
        row("tank vision", TANK_VISION, "tiles", m(TANK_VISION), "m"),
        row("direct gun range", DIRECT_RANGE, "tiles", m(DIRECT_RANGE), "m"),
        row("direct shell speed", SHELL_SPEED_DIRECT, "tiles/s", SHELL_SPEED_DIRECT * METRES_PER_TILE / TEMPO, "m/s"),
        row("indirect gun, least range", INDIRECT_MIN_RANGE, "tiles", m(INDIRECT_MIN_RANGE), "m"),
        row("indirect gun, full range", INDIRECT_MAX_RANGE, "tiles", m(INDIRECT_MAX_RANGE), "m"),
        row("a full-range shell's flight", flight, "s", flight * TEMPO, "s"),
        row("pillbox range", PILLBOX_RANGE, "tiles", m(PILLBOX_RANGE), "m"),
        row("recon vision", RECON_VISION, "tiles", m(RECON_VISION), "m"),
        row("recon link", RECON_LINK, "tiles", m(RECON_LINK), "m"),
        row("a 128-tile island", 128.0, "tiles", m(128.0) / 1000.0, "km"),
        row("a sixteen-minute match", 16.0 * 60.0, "s", 16.0 * TEMPO, "min"),
    ];
    for k in [Recon, Interceptor, Strike, Longshot] {
        for mark in 1..=3u8 {
            let s = drone_spec(k, mark);
            v.push(row(s.name, s.speed, "tiles/s", kmh(s.speed), "km/h"));
            v.push(row(s.name, s.battery, "s of flight", s.battery * TEMPO / 60.0, "min"));
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plan's chapter 9: the table follows the constants, and at tempo 6 the tank drives like a
    /// tank. `cargo test -p treadcore-sim physical -- --nocapture` prints it.
    #[test]
    fn the_physical_table_follows_the_constants() {
        let t = physical();
        let tank = t.iter().find(|r| r.what == "tank speed").unwrap();
        assert!((tank.real - 60.0).abs() < 1e-3, "{} km/h", tank.real);
        let shell = t.iter().find(|r| r.what == "a full-range shell's flight").unwrap();
        assert!((shell.game - 6.0).abs() < 1e-6 && (shell.real - 36.0).abs() < 1e-3, "{} s, {} s", shell.game, shell.real);
        assert!(t.iter().all(|r| r.game > 0.0 && r.real > 0.0));
        assert_eq!(t.iter().filter(|r| r.real_unit == "km/h").count(), 13, "the tank and twelve drone marks");
        for r in &t {
            println!("{:<30} {:>9.2} {:<12} {:>9.2} {}", r.what, r.game, r.game_unit, r.real, r.real_unit);
        }
    }
}
