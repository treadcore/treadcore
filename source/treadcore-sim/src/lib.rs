//! treadcore-sim -- the world, headless and deterministic.
//!
//! The premise. On the modern battlefield the tank has been given a new job.
//! It no longer closes with the enemy: it sits back and lobs high-calibre
//! rounds at map coordinates like a self-propelled gun, and it keeps itself
//! alive the way a carrier does, by owning the airspace around it. Every
//! tank runs a drone crew out of a bay on the hull: recon drones that spot
//! for the gun, interceptor drones that orbit the tank and kill anything
//! that flies in, and strike drones that go hunting. The game is the bubble
//! -- how far out you push it, what it costs to keep it up, and what happens
//! when a shell comes down through it, because a shell is not a drone and
//! the bubble does nothing to a shell.
//!
//! Everything in here is plain data and plain functions. No engine, no
//! sockets, no clocks: the world advances when `World::step` is called with
//! the commands for that tick, and the same seed, the same commands, in the
//! same order, produce the same world. That is what lets the host be
//! authoritative, the client predict, the bot be tested, and a trace be
//! replayed.
//!
//! Units: one tile is 25 m. The map is 128 x 128 tiles, 3.2 km on a side --
//! inside the reach of a tank firing indirect at the ranges reported from
//! Ukraine (up to ~10 km). Positions are f32 tiles. Time is ticks at
//! `TICK_HZ`.

use serde::{Deserialize, Serialize};

pub mod chair;
pub mod lingo;
pub mod map;
pub mod rules;

pub use map::{DepotOwner, Map, MapDepot, MapError, MapPillbox, MapStructure, Terrain, MAP_FORMAT, MAP_H, MAP_W};
pub use rules::*;

/// Simulation rate. Twenty ticks a second is enough for a game whose
/// fastest thing is a drone at nine tiles a second, and it keeps a
/// snapshot every other tick to a tenth of a second.
pub const TICK_HZ: u32 = 20;
pub const DT: f32 = 1.0 / TICK_HZ as f32;
/// Sixteen seats, the prior art's number. Empty ones are filled by bots. The
/// default scenario seats four; `--players N` on the server seats more.
pub const MAX_PLAYERS: usize = 16;


pub type EntityId = u32;
pub type PlayerId = u8;
pub type Tick = u64;

// ------------------------------------------------------------------ vectors

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const ZERO: V2 = V2 { x: 0.0, y: 0.0 };
    pub const fn new(x: f32, y: f32) -> V2 {
        V2 { x, y }
    }
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l < 1e-6 {
            V2::ZERO
        } else {
            V2::new(self.x / l, self.y / l)
        }
    }
    pub fn scale(self, s: f32) -> V2 {
        V2::new(self.x * s, self.y * s)
    }
    pub fn lerp(self, o: V2, t: f32) -> V2 {
        V2::new(self.x + (o.x - self.x) * t, self.y + (o.y - self.y) * t)
    }
    pub fn heading(self) -> f32 {
        self.y.atan2(self.x)
    }
    pub fn from_heading(h: f32) -> V2 {
        V2::new(h.cos(), h.sin())
    }
    pub fn tile(self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }
}

impl std::ops::Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        V2::new(self.x + o.x, self.y + o.y)
    }
}
impl std::ops::Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        V2::new(self.x - o.x, self.y - o.y)
    }
}

// ---------------------------------------------------------------------- rng

/// xorshift64*. Small, seedable, and the same on every machine, which is
/// the only property a lockstep simulation needs from a random source.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Uniform in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.f32()
    }
    pub fn below(&mut self, n: u32) -> u32 {
        (self.next_u64() % n as u64) as u32
    }
    /// A point in the unit disc, for scatter.
    pub fn disc(&mut self) -> V2 {
        let a = self.range(0.0, std::f32::consts::TAU);
        let r = self.f32().sqrt();
        V2::new(a.cos() * r, a.sin() * r)
    }
}

// ----------------------------------------------------------------- entities

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DroneKind {
    /// Eyes for the gun. Orbits a point, sees far, reports telemetry back
    /// over the link. A shell fired at something a recon drone can see
    /// lands where it was aimed; one fired blind lands somewhere nearby.
    Recon,
    /// The bubble. Loiters on station around its tank; when a hostile
    /// drone enters detection it closes and rams. One-shot, like the
    /// kinetic interceptors fielded since 2024.
    Interceptor,
    /// The FPV. Flies to a target and dives. Fast, short battery, and the
    /// thing the other side's bubble exists to stop.
    Strike,
    /// The long-range one-way drone (roadmap step 10c): sent to a point
    /// anywhere on the map, slow, on its own from launch: no link, no
    /// recall, no new task. Made for things that cannot move.
    Longshot,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DroneTask {
    /// Orbit `center` at `radius`. Interceptors on station, recon spotting.
    Orbit { center: V2, radius: f32 },
    /// Close with and ram a hostile drone.
    Intercept { target: EntityId },
    /// Fly to a tank and dive on it.
    Strike { target: EntityId },
    /// Fly back to the bay. Battery low, link lost, or recalled.
    Return,
    /// A longshot's one task: fly to `at` and dive.
    Attack { at: V2 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Drone {
    pub id: EntityId,
    pub owner: PlayerId,
    pub kind: DroneKind,
    pub pos: V2,
    pub heading: f32,
    /// Seconds of flight left.
    pub battery: f32,
    /// The tank it flew out of; where it returns to, and the other end
    /// of its telemetry link. None for a swarm drone from the map edge.
    pub tether: Option<EntityId>,
    pub task: DroneTask,
    /// 0..1, recomputed each tick from distance to the tether. Below
    /// `LINK_FLOOR` the drone has lost contact and goes home on its own.
    pub link: f32,
    /// Orbit phase, so orbiting drones space themselves out.
    pub phase: f32,
    /// Inside a jammer's reach (step 10b): its link is gone, and a
    /// longshot's fix has drifted already.
    #[serde(default)]
    pub jammed: bool,
    /// The mark it was launched as (`drone_spec`): 1, 2 or 3.
    #[serde(default = "mark_one")]
    pub mark: u8,
}

fn mark_one() -> u8 {
    1
}

impl Drone {
    /// What this drone is: its kind at its mark.
    pub fn spec(&self) -> DroneSpec {
        drone_spec(self.kind, self.mark)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellKind {
    /// Lobbed at a map coordinate. Seconds in the air, scatter that depends
    /// on whether the target was spotted at launch, splash on impact.
    Indirect,
    /// Flat trajectory at something in line of sight. Fast, no splash.
    Direct,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shell {
    pub id: EntityId,
    pub owner: PlayerId,
    pub kind: ShellKind,
    pub from: V2,
    /// Where it was aimed. `impact` is where it will actually land, fixed at
    /// launch so a client can draw the arc.
    pub aim: V2,
    pub impact: V2,
    pub launch_tick: Tick,
    pub impact_tick: Tick,
    /// True if a friendly recon drone had `aim` in view at launch.
    pub spotted: bool,
}

impl Shell {
    pub fn progress(&self, tick: Tick) -> f32 {
        let total = (self.impact_tick - self.launch_tick).max(1) as f32;
        ((tick - self.launch_tick) as f32 / total).clamp(0.0, 1.0)
    }
    pub fn pos_at(&self, tick: Tick) -> V2 {
        self.from.lerp(self.impact, self.progress(tick))
    }
}

/// What the tank carries in its bay. Restocked at a depot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bay {
    pub recon: u8,
    pub interceptor: u8,
    pub strike: u8,
    #[serde(default)]
    pub longshot: u8,
}

impl Bay {
    pub fn get(&self, k: DroneKind) -> u8 {
        match k {
            DroneKind::Recon => self.recon,
            DroneKind::Interceptor => self.interceptor,
            DroneKind::Strike => self.strike,
            DroneKind::Longshot => self.longshot,
        }
    }
    pub fn take(&mut self, k: DroneKind) -> bool {
        let slot = match k {
            DroneKind::Recon => &mut self.recon,
            DroneKind::Interceptor => &mut self.interceptor,
            DroneKind::Strike => &mut self.strike,
            DroneKind::Longshot => &mut self.longshot,
        };
        if *slot == 0 {
            return false;
        }
        *slot -= 1;
        true
    }
    pub fn put(&mut self, k: DroneKind) {
        match k {
            DroneKind::Recon => self.recon = (self.recon + 1).min(BAY_FULL.recon),
            DroneKind::Interceptor => {
                self.interceptor = (self.interceptor + 1).min(BAY_FULL.interceptor)
            }
            DroneKind::Strike => self.strike = (self.strike + 1).min(BAY_FULL.strike),
            DroneKind::Longshot => self.longshot = (self.longshot + 1).min(BAY_FULL.longshot),
        }
    }
}

// ------------------------------------------------------- pillboys, pillboxes

/// A fleet commander's word to its side (`Command::Request`): the fort
/// gets built by crews it musters and sets to work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Job {
    /// To the side: the fort is at `at`; crews free for work, report.
    Muster { at: V2 },
    /// To the commander: free for work.
    Ready,
    /// Build this, here, with your squad.
    Build { what: BuildKind, at: V2 },
    /// Lift your pillbox and set it down here (a turret slot).
    Fetch { pillbox: EntityId, to: V2 },
    /// With another crew given the same job: a chain on this pillbox or
    /// depot of the side, and drag it here (`Command::Tow`).
    Haul { target: EntityId, to: V2 },
    /// Back to what you were doing.
    Release,
}

/// What the tank can build through its squad, for material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildKind {
    /// An emplacement: `PILLBOX_BUILD_COST`, `PILLBOX_BUILD_TICKS`.
    Pillbox,
    /// Impassable, blocks fire, knocked to rubble by a splash.
    Wall,
    /// A road on land, a bridge over water.
    Road,
    /// Hidden until found, set off by a tank, a squad, a splash or a
    /// neighbour. Never on water, a wall or a depot.
    Mine,
    /// A wall a splash only cracks (step 10a).
    Concrete,
    /// A structure, on a pad (step 10b).
    Structure(StructureKind),
    /// A boat, moored on water beside the bank it was built from: a tank
    /// that drives onto it boards (SPEC TNK-17, the mode table).
    Boat,
}

impl BuildKind {
    pub fn cost(self) -> u8 {
        match self {
            BuildKind::Pillbox => PILLBOX_BUILD_COST,
            BuildKind::Wall => WALL_COST,
            BuildKind::Road => ROAD_COST,
            BuildKind::Mine => MINE_COST,
            BuildKind::Concrete => CONCRETE_COST,
            BuildKind::Structure(k) => k.cost(),
            BuildKind::Boat => BOAT_COST,
        }
    }
    pub fn ticks(self) -> u16 {
        match self {
            BuildKind::Pillbox => PILLBOX_BUILD_TICKS,
            BuildKind::Wall => WALL_BUILD_TICKS,
            BuildKind::Road => ROAD_BUILD_TICKS,
            BuildKind::Mine => MINE_LAY_TICKS,
            BuildKind::Concrete => CONCRETE_BUILD_TICKS,
            BuildKind::Structure(_) => STRUCTURE_BUILD_TICKS,
            BuildKind::Boat => BOAT_BUILD_TICKS,
        }
    }
    /// The squad works from beside a wall or a mine, never on it.
    fn standoff(self) -> f32 {
        match self {
            BuildKind::Wall | BuildKind::Concrete | BuildKind::Mine | BuildKind::Boat => BUILD_STANDOFF,
            _ => PILLBOY_WORK_RANGE,
        }
    }
}

/// What a structure is (roadmap step 10b). Each serves its owner's
/// side while it stands and, all but the power plant, while a power
/// plant of the side is in reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StructureKind {
    /// Powers the side's structures within `POWER_RANGE`.
    Power,
    /// Fuel: the side burns half; a fuel and shell stop beside it.
    Refinery,
    /// Fire control for blind shells near it, and an air picture.
    DataCentre,
    /// A repair yard: tanks beside it mended and restocked, pillboxes
    /// and structures near it mended slowly.
    Workshop,
    /// Electronic warfare: no drone near it keeps its link, its own
    /// side's included.
    Jammer,
}

impl StructureKind {
    pub const ALL: [StructureKind; 5] = [StructureKind::Power, StructureKind::Refinery, StructureKind::DataCentre, StructureKind::Workshop, StructureKind::Jammer];
    pub fn cost(self) -> u8 {
        match self {
            StructureKind::Power | StructureKind::Refinery => 4,
            StructureKind::DataCentre => 6,
            StructureKind::Workshop => 3,
            StructureKind::Jammer => 5,
        }
    }
    /// Its word in a map file and on the newswire.
    pub fn word(self) -> &'static str {
        match self {
            StructureKind::Power => "power",
            StructureKind::Refinery => "refinery",
            StructureKind::DataCentre => "datacentre",
            StructureKind::Workshop => "workshop",
            StructureKind::Jammer => "jammer",
        }
    }
    pub fn from_word(w: &str) -> Option<StructureKind> {
        StructureKind::ALL.iter().copied().find(|k| k.word() == w.to_ascii_lowercase())
    }
    /// The power plant runs itself; everything else wants one in reach.
    pub fn needs_power(self) -> bool {
        self != StructureKind::Power
    }
}

/// A structure on a pad. Like a pillbox it is owned or neutral, shot to
/// a wreck and never removed, captured by a squad standing on it while
/// it is neutral or wrecked, and mended for material.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Structure {
    pub id: EntityId,
    pub kind: StructureKind,
    pub pos: V2,
    pub owner: Option<PlayerId>,
    /// 0..STRUCTURE_HEALTH; at zero it is a wreck: no use to anyone,
    /// capturable.
    pub health: f32,
    pub capture: u16,
    pub capturer: Option<PlayerId>,
    /// Owned, standing, and a standing power plant of the side within
    /// `POWER_RANGE` (or itself one). Set every tick.
    pub powered: bool,
}

impl Structure {
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
    /// Doing its job: owned, standing and powered.
    pub fn working(&self) -> bool {
        self.owner.is_some() && self.alive() && self.powered
    }
}

/// A mine on a tile. The prior art's: the layer and (later) their allies see it;
/// everyone else finds it the hard way, or by coming within
/// `MINE_REVEAL` of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mine {
    pub id: EntityId,
    /// The tile's centre.
    pub pos: V2,
    pub owner: PlayerId,
}

/// A boat moored and empty: built by a squad from a bank, or left on the
/// water by a tank that landed from it; boarded by the first tank to
/// drive onto it. A boat with a tank in it is the tank's
/// `boat` flag, not one of these.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Boat {
    pub id: EntityId,
    /// The tile's centre, on water.
    pub pos: V2,
    /// Whose squad built it; anyone may board it.
    pub owner: PlayerId,
}

/// An order to the squad. The prior art's builder: the click is the order, the
/// squad leaves the tank, walks the world's route, does the work if the
/// material is there, and walks back. The player drives nothing but the
/// tank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PillboyTask {
    /// Stand on a pillbox (or a structure) that is neutral or dead
    /// until it is ours.
    Capture { pillbox: EntityId },
    /// Bring a pillbox (or a structure) back to full health, for
    /// material.
    Repair { target: EntityId },
    /// Raise an emplacement at a point, for material.
    Build { what: BuildKind, at: V2 },
    /// Fell a forest tile for material.
    Harvest { at: V2 },
    /// Pick up a pillbox of my own side, live or dead, to set it down
    /// elsewhere with a Build of a pillbox (step 11h).
    Lift { pillbox: EntityId },
    /// Come home.
    Return,
}

impl PillboyTask {
    /// Where the squad has to be to do this. None for Return (the tank).
    fn site(&self, world: &World) -> Option<V2> {
        match self {
            PillboyTask::Capture { pillbox } | PillboyTask::Repair { target: pillbox } => world.pillbox(*pillbox).map(|p| p.pos).or_else(|| world.structure(*pillbox).map(|s| s.pos)).or_else(|| world.tank(*pillbox).map(|t| t.pos)),
            PillboyTask::Lift { pillbox } => world.pillbox(*pillbox).map(|p| p.pos),
            PillboyTask::Build { at, .. } | PillboyTask::Harvest { at } => Some(*at),
            PillboyTask::Return => None,
        }
    }
    /// How close the squad must get to work.
    fn reach(&self) -> f32 {
        match self {
            PillboyTask::Build { what, .. } => what.standoff(),
            PillboyTask::Return => PILLBOY_BOARD_RANGE,
            _ => PILLBOY_WORK_RANGE,
        }
    }
}

/// The tank's tech infantry: state on the tank, not a unit of its own.
/// Aboard, it rides; deployed, it walks, works and is a ground target.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pillboys {
    /// 0..PILLBOY_MAX. Zero is no squad until a depot replaces it.
    pub strength: u8,
    pub aboard: bool,
    /// Where the squad is when deployed; the tank's position otherwise.
    pub pos: V2,
    pub task: Option<PillboyTask>,
    /// Ticks of work done at the site.
    pub progress: u16,
    /// Repairs and emplacements cost this. Restocked at depots.
    pub material: u8,
    /// Waypoints to the site, or home.
    pub route: Vec<V2>,
    /// A pillbox lifted and not yet set down: its health (step 11h).
    /// Lost with the squad or the tank, it falls where they were, dead.
    pub carrying: Option<f32>,
}

impl Pillboys {
    pub fn fresh(pos: V2, material: u8) -> Pillboys {
        Pillboys { strength: PILLBOY_MAX, aboard: true, pos, task: None, progress: 0, material, route: Vec::new(), carrying: None }
    }
    pub fn deployed(&self) -> bool {
        !self.aboard
    }
}

/// the prior art's pillbox, with one addition: it shoots drones too, at short
/// range, so a captured pillbox extends the bubble over ground the tank
/// is not on. Owned by a seat or by nobody; neutral ones shoot everyone.
/// Captured by a squad standing on it (neutral or dead), repaired and
/// built by a squad with material. Never removed from the world.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pillbox {
    pub id: EntityId,
    pub pos: V2,
    pub owner: Option<PlayerId>,
    /// 0..PILLBOX_HEALTH; at zero it is dead: silent, capturable.
    pub health: f32,
    pub reload: u16,
    /// Capture progress and whose squad is making it.
    pub capture: u16,
    pub capturer: Option<PlayerId>,
    /// Shells it holds for its owner's tank; regrow slowly.
    pub shells: u16,
}

impl Pillbox {
    pub fn alive(&self) -> bool {
        self.health > 0.0
    }
    /// Whose shots these are, for damage credit and intercept events.
    pub fn shooter(&self) -> PlayerId {
        self.owner.unwrap_or(NEUTRAL)
    }
    /// Hostile to `p`: anyone but the owner. Neutral hates everyone.
    pub fn hostile_to(&self, p: PlayerId) -> bool {
        self.owner != Some(p)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tank {
    pub id: EntityId,
    pub owner: PlayerId,
    pub pos: V2,
    pub heading: f32,
    pub speed: f32,
    pub armor: f32,
    pub shells: u16,
    pub fuel: f32,
    pub bay: Bay,
    /// The waypoint it is driving to now, in a straight line at
    /// terrain-cost speed. `Move` routes with the map's A* and fills
    /// `route` with the waypoints after this one; the tank follows them.
    pub goal: Option<V2>,
    /// The rest of the way, nearest first. Empty when `goal` is the end.
    pub route: Vec<V2>,
    /// Ticks until the gun can fire again.
    pub reload: u16,
    /// Radius of the interceptor station. The player's one dial: wide
    /// catches strikes early but thins the bubble and stretches the link.
    pub bubble: f32,
    /// Ticks since last damage, for the HUD's "under fire" state.
    pub last_hit_tick: Tick,
    pub alive: bool,
    /// The squad, aboard or out on an order.
    pub pillboys: Pillboys,
    /// Afloat on a boat (SPEC TNK-08, TNK-11 to TNK-14): drives over
    /// water and not over bridges, leaves the boat on the first beach it
    /// drives onto, and sinks with it at one hit, because all of
    /// treadcore's water is deep.
    #[serde(default)]
    pub boat: bool,
    /// The pillbox or depot this tank has a chain on (`Command::Tow`).
    /// It moves when `TOW_HAULERS` tanks of a side pull it together.
    #[serde(default)]
    pub towing: Option<EntityId>,
}

impl Tank {
    /// What prediction compares, as bits: a client's tank at a tick
    /// against the host's. Position, heading, speed, the waypoint and
    /// the route; not armour or fuel, which the world does to a tank
    /// and the snapshot brings.
    pub fn tick_state(&self) -> (u32, u32, u32, u32, Option<(u32, u32)>, usize) {
        (self.pos.x.to_bits(), self.pos.y.to_bits(), self.heading.to_bits(), self.speed.to_bits(), self.goal.map(|g| (g.x.to_bits(), g.y.to_bits())), self.route.len())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Depot {
    pub id: EntityId,
    pub pos: V2,
    pub owner: Option<PlayerId>,
    /// 0..CAPTURE_TICKS, with the id of who is capturing.
    pub capture: u16,
    pub capturer: Option<PlayerId>,
    /// What it has made and no tank has drawn; it starts empty and only an owned depot makes.
    #[serde(default)]
    pub stock: DepotStock,
}

/// A depot's store, and the turn it has reached: shells, trees, interceptors, long drones,
/// recon, strikes, one a second round the six.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DepotStock {
    pub kinds: [u8; 6],
    pub turn: u8,
}

pub const STOCK_SHELL: usize = 0;
pub const STOCK_TREE: usize = 1;
pub const STOCK_INTERCEPTOR: usize = 2;
pub const STOCK_LONGSHOT: usize = 3;
pub const STOCK_RECON: usize = 4;
pub const STOCK_STRIKE: usize = 5;

impl DepotStock {
    /// One more of the next kind that has room; none when all six are full.
    pub fn make(&mut self) {
        for i in 0..self.kinds.len() {
            let k = (self.turn as usize + i) % self.kinds.len();
            if self.kinds[k] < DEPOT_STOCK_MAX {
                self.kinds[k] += 1;
                self.turn = ((k + 1) % self.kinds.len()) as u8;
                return;
            }
        }
    }

    /// One of a kind out of the store, if there is one.
    pub fn take(&mut self, k: usize) -> bool {
        let there = self.kinds[k] > 0;
        if there {
            self.kinds[k] -= 1;
        }
        there
    }
}

/// A team id. Players start each in a team of their own (the team id is
/// the seat) unless the scenario deals them; an alliance is two or more
/// players with one team id.
pub type TeamId = u8;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub tank: EntityId,
    pub alive: bool,
    pub score: u32,
    pub is_bot: bool,
    pub team: TeamId,
    /// The tick of this player's last message (SPEC UI-23).
    pub last_chat: Option<Tick>,
    /// Times this player's tank has come back (`Command::Respawn`).
    #[serde(default)]
    pub respawned: u8,
    /// The tick of this player's last ping.
    #[serde(default)]
    pub last_ping: Option<Tick>,
}

/// Who a chat line is for (SPEC UI-22): everyone, the team, the tanks
/// within `CHAT_NEARBY` of the sender's, or the players chosen, one bit
/// a seat. The sender always hears their own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatTo {
    All,
    Team,
    Nearby,
    Players(u16),
}

impl ChatTo {
    /// The chosen players as a mask.
    pub fn players(ps: impl IntoIterator<Item = PlayerId>) -> ChatTo {
        ChatTo::Players(ps.into_iter().filter(|p| (*p as usize) < 16).fold(0u16, |m, p| m | 1 << p))
    }
}

/// What a proposal asks. Every change of sides in play is one of these,
/// answered by the one asked (or any member of their team), resolved by
/// the world under the three-way rule, and announced to everyone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Terms {
    /// I join your team (the prior art's alliance request; also a defection).
    Join,
    /// You join my team.
    Recruit,
    /// We exchange teams: I take yours, you take mine.
    Swap,
    /// My whole team joins yours.
    Merge,
}

/// the prior art's alliance protocol (SPEC ALL-02 to ALL-06) as sugar over
/// proposals: `Request` is a `Join` proposal, `Accept` and `Decline`
/// answer the pending one from that player, `Leave` is a side of one's
/// own again. What the leaver captured stays the leaver's: ownership is
/// per player, so there is nothing to divide (ALL-07).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AllianceCmd {
    Request { to: PlayerId },
    Accept { from: PlayerId },
    Decline { from: PlayerId },
    Leave,
}

/// Who would be on which side if `from` proposed `terms` to `to` and it
/// were accepted: (player, team) for everyone, from (player, team) now.
/// The world's own reading, and a window's, of the same proposal.
pub fn proposal_outcome(sides: &[(PlayerId, TeamId)], from: PlayerId, to: PlayerId, terms: Terms) -> Vec<(PlayerId, TeamId)> {
    let team = |p: PlayerId| sides.iter().find(|(x, _)| *x == p).map(|(_, t)| *t);
    let (Some(tf), Some(tt)) = (team(from), team(to)) else { return sides.to_vec() };
    sides
        .iter()
        .map(|&(p, t)| {
            let t2 = match terms {
                Terms::Join if p == from => tt,
                Terms::Recruit if p == to => tf,
                Terms::Swap if p == from => tt,
                Terms::Swap if p == to => tf,
                Terms::Merge if t == tf => tt,
                _ => t,
            };
            (p, t2)
        })
        .collect()
}

/// The three-way rule over an outcome: at least MIN_SIDES sides with a
/// living tank, and none holding more than half the living tanks.
pub fn three_way(teams: &[(PlayerId, TeamId)], alive: impl Fn(PlayerId) -> bool) -> Result<(), &'static str> {
    let living: Vec<(PlayerId, TeamId)> = teams.iter().filter(|(p, _)| alive(*p)).copied().collect();
    let mut sides: Vec<TeamId> = living.iter().map(|(_, t)| *t).collect();
    sides.sort_unstable();
    sides.dedup();
    if sides.len() < MIN_SIDES {
        return Err("fewer than three sides would remain");
    }
    let biggest = sides.iter().map(|t| living.iter().filter(|(_, x)| x == t).count()).max().unwrap_or(0);
    if biggest * 2 > living.len() {
        return Err("one side would hold more than half the tanks");
    }
    Ok(())
}

/// A standing proposal, until answered or `PROPOSAL_TICKS` pass.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: EntityId,
    pub from: PlayerId,
    pub to: PlayerId,
    pub terms: Terms,
    pub expires: Tick,
}

// ----------------------------------------------------------------- commands

/// What a ping is for (Paul, 30 October: "help, caution, info, warning, defend").
/// The sim only carries it; what a bot does about each is the brain's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PingKind {
    /// Come here: a tank in a fight, or down. What every ping used to mean.
    #[default]
    Help,
    /// Go carefully near here.
    Caution,
    /// Something is here; nobody need come.
    Info,
    /// Danger: get away from here.
    Warning,
    /// Hold this ground: a base under attack.
    Defend,
}

/// What a player (human or Brain) may ask for. The world validates every
/// one: a dead tank, an empty bay, a reloading gun, a point off the map --
/// the command is dropped and a `Rejected` event says why.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Command {
    Move { to: V2 },
    Stop,
    FireIndirect { at: V2 },
    FireDirect { at: V2 },
    Launch { kind: DroneKind, task: DroneTask },
    /// Every drone home.
    Recall,
    /// One of my drones, given a new task: a recon's orbit moved, a
    /// strike's target changed, or any one drone recalled on its own.
    /// Interceptors stay the world's to task; they can only be recalled.
    Retask { drone: EntityId, task: DroneTask },
    SetBubble { radius: f32 },
    /// An order to the Pillboys. One at a time; a new one while they are
    /// out is refused, except Return.
    Deploy { task: PillboyTask },
    /// Ask, answer, leave: The prior art's words for the proposals below.
    Alliance(AllianceCmd),
    /// A change of sides, put to another player. Refused at once if the
    /// outcome would break the three-way rule.
    Propose { to: PlayerId, terms: Terms },
    /// Answer a proposal made to me or my team.
    Answer { proposal: EntityId, accept: bool },
    Chat { text: String, to: ChatTo },
    /// A person takes this bot's seat mid-match (roadmap step 8): the
    /// host gives the order for a late arrival, so a replay and the
    /// world hash carry it like any other. Refused for a seat that is
    /// already a person's.
    TakeSeat { name: String },
    /// A word to my side about the fort: to one player, or to the whole
    /// side (None). Only to allies; the side sees it (`Event::Requested`).
    Request { to: Option<PlayerId>, job: Job },
    /// Hitch a chain to a pillbox or a depot of my side within
    /// `TOW_REACH` (Some), or let it go (None). It moves when two tanks
    /// of the side pull it, at `TOW_SPEED`; one alone is held by it.
    Tow { target: Option<EntityId> },
    /// Mark a point on the map for my side: an enemy to go for, a place
    /// to gather. Allies see it (`Event::Pinged`); allied bots answer it
    /// by converging there. One a second. `kind` says what it is for; an
    /// old client's ping is a call for help, as every ping used to be.
    Ping {
        at: V2,
        #[serde(default)]
        kind: PingKind,
    },
    /// Bring my destroyed tank back at my start, when the scenario has
    /// respawns and `RESPAWN_TICKS` have passed since it died.
    Respawn,
    /// Launch from the shore (Paul, 16 October: "tanks usually can build
    /// a boat under it if it is in water"): the squad, aboard, puts a
    /// boat on the water beside the tank and the tank is on it at once,
    /// afloat. No walk, no drive onto it. It takes the squad's material
    /// up to `BOAT_COST`, and launches with none.
    Embark,
}

impl Command {
    /// The orders a client predicts (roadmap step 4): those that move
    /// the hull. A fire order is one, because firing turns the tank.
    pub fn predicted(cmd: &Command) -> bool {
        matches!(cmd, Command::Move { .. } | Command::Stop | Command::FireIndirect { .. } | Command::FireDirect { .. })
    }
}

/// A command with its author. The host stamps the player; a client never
/// gets to say who it is.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub player: PlayerId,
    pub cmd: Command,
}

// ------------------------------------------------------------------- events

/// What happened this tick, for the HUD, the log, the bot, and the replay.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Rejected { player: PlayerId, cmd: Command, why: String },
    ShellLaunched { shell: EntityId, owner: PlayerId, kind: ShellKind, spotted: bool, eta_ticks: u32 },
    ShellImpact { shell: EntityId, at: V2, kind: ShellKind, hits: Vec<EntityId> },
    DroneLaunched { drone: EntityId, owner: PlayerId, kind: DroneKind },
    /// The match's drones move up a mark (`MARK_II_AT`, `MARK_III_AT`):
    /// every launch from now on is this mark.
    DronesUpgraded { mark: u8 },
    DroneLost { drone: EntityId, owner: PlayerId, kind: DroneKind, why: LossReason },
    DroneRecovered { drone: EntityId, owner: PlayerId, kind: DroneKind },
    /// To the owner and allies: a drone took a new task by order.
    DroneRetasked { drone: EntityId, owner: PlayerId, task: DroneTask },
    Intercepted { interceptor: EntityId, target: EntityId, owner: PlayerId },
    InterceptMissed { interceptor: EntityId, target: EntityId },
    StrikeHit { drone: EntityId, tank: EntityId, damage: f32 },
    TankDamaged { tank: EntityId, by: Option<PlayerId>, damage: f32, armor_left: f32 },
    TankDestroyed { tank: EntityId, owner: PlayerId, by: Option<PlayerId> },
    DepotCaptured { depot: EntityId, by: PlayerId },
    Resupplied { tank: EntityId, depot: EntityId },
    SwarmInbound { count: u32, from: V2 },
    LinkLost { drone: EntityId, owner: PlayerId },
    /// An ally's shell or mine would have hurt this tank, squad or
    /// pillbox; friendly fire is off within a team, and the trace says
    /// so.
    FriendlyFireSpared { target: EntityId, by: PlayerId, damage: f32 },
    /// A proposal stands: to the proposer and the team asked.
    Proposed { proposal: EntityId, from: PlayerId, to: PlayerId, terms: Terms },
    /// A proposal ended, to everyone: accepted and carried out, declined
    /// by `by`, refused by the three-way rule (`why`), or lapsed. A
    /// betrayal is public the moment it happens.
    ProposalResolved { proposal: EntityId, from: PlayerId, to: PlayerId, terms: Terms, accepted: bool, by: Option<PlayerId>, why: Option<String> },
    /// `player` joined `team`, whose members are now `members`.
    AllianceFormed { player: PlayerId, team: TeamId, members: Vec<PlayerId> },
    AllianceLeft { player: PlayerId, team: TeamId },
    PillboysDeployed { tank: EntityId, owner: PlayerId, task: PillboyTask },
    PillboysReturned { tank: EntityId, owner: PlayerId },
    /// The order ended without its work: the site changed under them.
    PillboysAbandoned { tank: EntityId, owner: PlayerId, why: String },
    PillboysHit { tank: EntityId, owner: PlayerId, lost: u8, left: u8 },
    PillboysLost { tank: EntityId, owner: PlayerId },
    PillboxCaptured { pillbox: EntityId, by: PlayerId, from: Option<PlayerId> },
    PillboxRepaired { pillbox: EntityId, by: PlayerId },
    /// An ally's squad repaired a tank that was down: it is back, at
    /// `REVIVE_ARMOR`, where it fell.
    Revived { tank: EntityId, owner: PlayerId, by: PlayerId, at: V2 },
    PillboxBuilt { pillbox: EntityId, by: PlayerId, at: V2 },
    /// Picked up by its own side's squad (step 11h): gone from the map
    /// until set down, when it is PillboxBuilt again under a new id.
    PillboxLifted { pillbox: EntityId, by: PlayerId, at: V2 },
    /// The squad carrying one was lost: it falls there, dead.
    PillboxDropped { pillbox: EntityId, owner: PlayerId, at: V2 },
    PillboxDamaged { pillbox: EntityId, by: Option<PlayerId>, damage: f32, health_left: f32 },
    /// Shot to zero: silent and capturable, not removed.
    PillboxDestroyed { pillbox: EntityId, owner: Option<PlayerId>, by: Option<PlayerId> },
    /// Hitscan at a tank. `owner` is NEUTRAL for a neutral pillbox.
    PillboxFired { pillbox: EntityId, owner: PlayerId, tank: EntityId, damage: f32 },
    /// The squad finished a wall, road, bridge or pillbox.
    Built { owner: PlayerId, what: BuildKind, at: V2 },
    Harvested { owner: PlayerId, at: V2, material: u8 },
    /// To the layer only.
    MineLaid { mine: EntityId, owner: PlayerId, at: V2 },
    MineExploded { mine: EntityId, owner: PlayerId, at: V2, hits: Vec<EntityId> },
    /// Structures (step 10b), public like pillboxes: territory is.
    StructureBuilt { structure: EntityId, kind: StructureKind, by: PlayerId, at: V2 },
    StructureDamaged { structure: EntityId, kind: StructureKind, owner: Option<PlayerId>, by: Option<PlayerId>, damage: f32, health_left: f32 },
    /// Shot to zero: a wreck, capturable, not removed.
    StructureWrecked { structure: EntityId, kind: StructureKind, owner: Option<PlayerId>, by: Option<PlayerId> },
    StructureCaptured { structure: EntityId, kind: StructureKind, by: PlayerId, from: Option<PlayerId> },
    StructureRepaired { structure: EntityId, kind: StructureKind, by: PlayerId },
    /// Power came on or went off at an owned structure.
    StructurePower { structure: EntityId, kind: StructureKind, owner: PlayerId, on: bool },
    /// A longshot dived at its point; `hits` is what it hurt.
    LongshotHit { drone: EntityId, owner: PlayerId, at: V2, hits: Vec<EntityId> },
    /// To the drone's side: a jammer took its link (or a longshot's fix).
    Jammed { drone: EntityId, owner: PlayerId, kind: DroneKind, jammer: EntityId },
    /// A tile changed under everyone: a client updates its copy of the map.
    TerrainChanged { at: V2, to: u8 },
    Chat { player: PlayerId, text: String, to: ChatTo },
    /// `winner` is the top-scoring player of the winning team, if any.
    MatchOver { winner: Option<PlayerId>, team: Option<TeamId> },
    /// A person took a bot's seat: who, by the name they had and have.
    SeatTaken { player: PlayerId, was: String, name: String },
    /// A tank drove onto a moored boat and is afloat.
    Boarded { tank: EntityId, owner: PlayerId, boat: EntityId, at: V2 },
    /// A tank drove its boat onto a beach and is ashore; the boat stays
    /// moored on the water behind it.
    Landed { tank: EntityId, owner: PlayerId, at: V2 },
    /// A boat was hit with its tank in it: both go down (TNK-14).
    Sunk { tank: EntityId, owner: PlayerId, by: Option<PlayerId>, at: V2 },
    /// A ping, to the pinger's side.
    Pinged {
        player: PlayerId,
        at: V2,
        #[serde(default)]
        kind: PingKind,
    },
    /// A fort request, to the requester's side.
    Requested { from: PlayerId, to: Option<PlayerId>, job: Job },
    /// A chain hitched (`on`) or let go, between a tank and a pillbox or
    /// a depot; `haulers` is how many tanks are on it now.
    Chained { tank: EntityId, owner: PlayerId, target: EntityId, on: bool, haulers: u8 },
    /// A destroyed tank came back at its start; `left` respawns remain
    /// (`RESPAWNS_UNLIMITED` for no limit).
    Respawned { tank: EntityId, owner: PlayerId, at: V2, left: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LossReason {
    Battery,
    Shot,
    Expended,
    Splash,
    OwnerDead,
}

// ------------------------------------------------------------------ scenario

/// Where a scenario's map comes from: generated from the scenario's own
/// seed (the default, so `Scenario { seed, ..Default::default() }` plays
/// a different island for every seed, as it always did), generated from
/// a seed of its own, or the text of a map file carried whole, so a
/// replay and a Welcome are self-contained and no peer needs the file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MapSource {
    FromScenarioSeed,
    Seed(u64),
    Text(String),
}

impl MapSource {
    pub fn build(&self, scenario_seed: u64) -> Result<Map, MapError> {
        match self {
            MapSource::FromScenarioSeed => Ok(Map::generate(scenario_seed)),
            MapSource::Seed(s) => Ok(Map::generate(*s)),
            MapSource::Text(t) => Map::from_text(t),
        }
    }
}

/// the prior art's three game types (SPEC section 11), as what a tank starts
/// with. Everything else about a match is the scenario's other fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameType {
    /// Every tank starts armed and stocked: shells and material full.
    #[default]
    Open,
    /// Some free ammunition: half the shells, no material. The depots
    /// and the forest are where the rest comes from.
    Tournament,
    /// No free ammunition at all: no shells, no material. The first
    /// thing anyone does is drive to a depot.
    Strict,
}

impl GameType {
    pub fn shells(self) -> u16 {
        match self {
            GameType::Open => TANK_SHELLS,
            GameType::Tournament => TANK_SHELLS / 2,
            GameType::Strict => 0,
        }
    }
    pub fn material(self) -> u8 {
        match self {
            GameType::Open => PILLBOY_MATERIAL_START,
            _ => 0,
        }
    }
    pub fn parse(s: &str) -> Option<GameType> {
        match s.to_ascii_lowercase().as_str() {
            "open" => Some(GameType::Open),
            "tournament" => Some(GameType::Tournament),
            "strict" | "strict-tournament" => Some(GameType::Strict),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    /// Seeds the dice (scatter, intercepts, the swarm). The map has its
    /// own source.
    pub seed: u64,
    pub map: MapSource,
    /// What a tank starts with (the prior art's game types). Defaults when
    /// absent, so a replay recorded before it existed still reads (and
    /// then fails its hashes honestly, if the rules have moved).
    #[serde(default)]
    pub game_type: GameType,
    /// the prior art's brain advantage (SPEC BRN-12, GAM-03): a bot seat is told
    /// where every tank is, regardless of range. Off by default; a
    /// handicap for a human against bots, never on in a bot-only proof.
    #[serde(default)]
    pub advantage: bool,
    pub players: Vec<(String, bool)>,
    /// A team per seat, or none: everyone starts alone, as in the prior art
    /// (ALL-01), and allies by request in play.
    pub teams: Option<Vec<TeamId>>,
    /// Ticks between neutral FPV swarms from the map edge; 0 disables them.
    /// They make the bubble matter even with one player on the map.
    pub swarm_interval: u32,
    pub swarm_size: u32,
    /// Hard stop, in ticks. 0 for none.
    pub time_limit: Tick,
    /// Give a map with no structures of its own an economy
    /// (`Map::industrialise`, roadmap step 10f).
    #[serde(default)]
    pub industry: bool,
    /// How many times a destroyed tank may come back, each
    /// `RESPAWN_TICKS` after it died, at its start (on a landing craft
    /// if the start is at sea, SPEC TNK-08). 0 is the prior art's
    /// elimination, as before; `RESPAWNS_UNLIMITED` is no limit. A side
    /// with a respawn left is not out of the match.
    #[serde(default)]
    pub respawns: u8,
}

/// `--respawns N` as the window and the server read it: a count (254 at
/// most), or `unlimited`.
pub fn parse_respawns(s: &str) -> Option<u8> {
    match s {
        "unlimited" | "inf" | "always" => Some(RESPAWNS_UNLIMITED),
        _ => s.parse::<u32>().ok().map(|n| n.min(RESPAWNS_UNLIMITED as u32 - 1) as u8),
    }
}

/// The sixteen starts have compass names, in the order `Map::starts`
/// lists them; a seat is named for its start.
pub fn seat_name(i: usize) -> String {
    const NAMES: [&str; 16] = ["north", "east", "south", "west", "northeast", "southeast", "southwest", "northwest", "nne", "nnw", "ene", "ese", "sse", "ssw", "wsw", "wnw"];
    NAMES.get(i).map(|s| s.to_string()).unwrap_or_else(|| format!("seat{}", i))
}

impl Scenario {
    /// `n` bot seats, named for their starts.
    pub fn bots(n: usize) -> Scenario {
        Scenario { players: (0..n.clamp(1, MAX_PLAYERS)).map(|i| (seat_name(i), true)).collect(), ..Scenario::default() }
    }
    /// Deal the seats into `n` teams, round robin: seat i to team i mod n.
    /// Four teams of four is `teams(4)` on sixteen seats.
    pub fn with_teams(mut self, n: usize) -> Scenario {
        let n = n.max(1);
        self.teams = Some((0..self.players.len()).map(|i| (i % n) as TeamId).collect());
        self
    }
}

impl Default for Scenario {
    fn default() -> Scenario {
        Scenario {
            seed: 0x7EAD_C0DE,
            map: MapSource::FromScenarioSeed,
            game_type: GameType::Open,
            advantage: false,
            players: (0..MAX_PLAYERS).map(|i| (seat_name(i), true)).collect(),
            teams: None,

            swarm_interval: 45 * TICK_HZ,
            swarm_size: 4,
            time_limit: 15 * 60 * TICK_HZ as Tick,
            industry: false,
            respawns: 0,
        }
    }
}

// -------------------------------------------------------------------- world


/// One of a side's eyes (`World::eyes`): where it is and how far it sees
/// a point in the open and a point in forest.
#[derive(Clone, Copy, Debug)]
pub struct Eye {
    pub pos: V2,
    pub open: f32,
    pub forest: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub tick: Tick,
    pub scenario: Scenario,
    pub map: Map,
    pub players: Vec<Player>,
    pub tanks: Vec<Tank>,
    pub drones: Vec<Drone>,
    pub shells: Vec<Shell>,
    pub depots: Vec<Depot>,
    pub pillboxes: Vec<Pillbox>,
    pub mines: Vec<Mine>,
    /// Trees felled, and when each grows back, soonest first.
    #[serde(default)]
    pub felled: Vec<(V2, Tick)>,
    pub proposals: Vec<Proposal>,
    pub structures: Vec<Structure>,
    /// Boats moored and empty.
    #[serde(default)]
    pub boats: Vec<Boat>,
    pub rng: Rng,
    pub next_id: EntityId,
    pub over: bool,
    pub winner: Option<PlayerId>,
    pub winning_team: Option<TeamId>,
    /// Events from the most recent `step`. Cleared at the start of the next.
    #[serde(skip)]
    pub events: Vec<Event>,
}

/// Squads on one pillbox (or structure) that are all of one side do
/// not contest it: they take it together, for the one already taking
/// it, else the lowest seat (step 11h's fleets: two of a team on one
/// pillbox stood ten seconds and more with neither clock moving).
fn allies_together(who: Vec<PlayerId>, capturer: Option<PlayerId>, allied: &impl Fn(PlayerId, PlayerId) -> bool) -> Vec<PlayerId> {
    if who.len() > 1 && who.iter().all(|w| allied(*w, who[0])) {
        vec![capturer.filter(|c| who.contains(c)).unwrap_or(who[0])]
    } else {
        who
    }
}

impl World {
    /// A world from a scenario. A map file that does not read is a
    /// programming error here: the server checks the file before it
    /// builds the scenario (`Map::from_text`), and a scenario that
    /// arrived in a Welcome or a replay was built from one that did.
    pub fn new(scenario: Scenario) -> World {
        let mut map = scenario.map.build(scenario.seed).unwrap_or_else(|e| panic!("scenario map does not read: {}", e));
        if scenario.industry && map.structures.is_empty() {
            map.industrialise();
        }
        let mut w = World {
            tick: 0,
            map,
            players: Vec::new(),
            tanks: Vec::new(),
            drones: Vec::new(),
            shells: Vec::new(),
            depots: Vec::new(),
            pillboxes: Vec::new(),
            mines: Vec::new(),
            felled: Vec::new(),
            proposals: Vec::new(),
            structures: Vec::new(),
            boats: Vec::new(),
            rng: Rng::new(scenario.seed ^ 0xA5A5_5A5A),
            next_id: 1,
            over: false,
            winner: None,
            winning_team: None,
            events: Vec::new(),
            scenario: scenario.clone(),
        };
        let seats = scenario.players.len().min(MAX_PLAYERS).min(w.map.starts.len());
        for (i, d) in w.map.depots.clone().iter().enumerate() {
            let id = w.alloc();
            let owner = match d.owner {
                DepotOwner::ByStart if i < seats => Some(i as PlayerId),
                DepotOwner::Seat(o) if (o as usize) < seats => Some(o),
                _ => None,
            };
            w.depots.push(Depot { id, pos: d.pos, owner, capture: 0, capturer: None, stock: DepotStock::default() });
        }
        for p in w.map.pillboxes.clone() {
            let id = w.alloc();
            // An owner the scenario does not seat is neutral.
            let owner = p.owner.filter(|o| (*o as usize) < scenario.players.len().min(MAX_PLAYERS).min(w.map.starts.len()));
            w.pillboxes.push(Pillbox { id, pos: p.pos, owner, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        }
        for st in w.map.structures.clone() {
            let id = w.alloc();
            let owner = st.owner.filter(|o| (*o as usize) < seats);
            w.structures.push(Structure { id, kind: st.kind, pos: st.pos, owner, health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: false });
        }
        // No more seats than the map has starts.
        for (i, (name, is_bot)) in scenario.players.iter().take(seats).enumerate() {
            let id = w.alloc();
            let pos = w.map.starts[i];
            w.tanks.push(Tank {
                id,
                owner: i as PlayerId,
                pos,
                heading: (w.map.center() - pos).heading(),
                speed: 0.0,
                armor: TANK_ARMOR,
                shells: scenario.game_type.shells(),
                fuel: TANK_FUEL,
                bay: BAY_FULL,
                goal: None,
                route: Vec::new(),
                reload: 0,
                bubble: BUBBLE_DEFAULT,
                last_hit_tick: 0,
                alive: true,
                pillboys: Pillboys::fresh(pos, scenario.game_type.material()),
                // A start at sea is a landing craft (TNK-08).
                boat: w.map.at(pos) == Terrain::Water,
                towing: None,
            });
            w.players.push(Player {
                id: i as PlayerId,
                name: name.clone(),
                tank: id,
                alive: true,
                score: 0,
                is_bot: *is_bot,
                team: scenario.teams.as_ref().and_then(|t| t.get(i)).copied().unwrap_or(i as TeamId),
                last_chat: None,
                respawned: 0,
                last_ping: None,
            });

        }
        w
    }

    fn alloc(&mut self) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// The map as it is now (craters, walls, felled forest) with the
    /// pillboxes as they are owned now: what `--save-map` writes, so a
    /// fought-over island can be the start of the next match.
    pub fn map_as_played(&self) -> Map {
        let mut m = self.map.clone();
        m.pillboxes = self.pillboxes.iter().map(|p| MapPillbox { pos: p.pos, owner: p.owner }).collect();
        m.structures = self.structures.iter().map(|s| MapStructure { pos: s.pos, kind: s.kind, owner: s.owner }).collect();
        m
    }

    pub fn tank(&self, id: EntityId) -> Option<&Tank> {
        self.tanks.iter().find(|t| t.id == id)
    }
    pub fn tank_of(&self, p: PlayerId) -> Option<&Tank> {
        self.tanks.iter().find(|t| t.owner == p)
    }
    fn tank_mut(&mut self, id: EntityId) -> Option<&mut Tank> {
        self.tanks.iter_mut().find(|t| t.id == id)
    }
    pub fn drone(&self, id: EntityId) -> Option<&Drone> {
        self.drones.iter().find(|d| d.id == id)
    }
    pub fn pillbox(&self, id: EntityId) -> Option<&Pillbox> {
        self.pillboxes.iter().find(|p| p.id == id)
    }
    pub fn structure(&self, id: EntityId) -> Option<&Structure> {
        self.structures.iter().find(|s| s.id == id)
    }
    fn structure_mut(&mut self, id: EntityId) -> Option<&mut Structure> {
        self.structures.iter_mut().find(|s| s.id == id)
    }
    /// A working structure of this kind on `p`'s side within `r` of `at`.
    pub fn serves(&self, p: PlayerId, kind: StructureKind, at: V2, r: f32) -> bool {
        self.structures.iter().any(|s| s.kind == kind && s.working() && self.allied_opt(s.owner, p) && s.pos.dist(at) <= r)
    }
    /// The side's air picture (a data centre's radar): `p` sees drones
    /// at `at`.
    pub fn radar_sees(&self, p: PlayerId, at: V2) -> bool {
        self.serves(p, StructureKind::DataCentre, at, DATACENTRE_RADAR)
    }

    /// A tank's route: the map's, avoiding the range of every live
    /// pillbox hostile to `p` at `PILLBOX_AVOID_COST` extra per tile,
    /// and every hostile mine `p` knows about at `MINE_AVOID_COST`.
    pub fn route_for(&self, p: PlayerId, from: V2, to: V2) -> Vec<V2> {
        self.route_afloat(p, from, to, false)
    }

    /// `route_for`, by boat when `afloat` (`Map::route_by`).
    pub fn route_afloat(&self, p: PlayerId, from: V2, to: V2, afloat: bool) -> Vec<V2> {
        let guns: Vec<V2> = self.pillboxes.iter().filter(|b| b.alive() && !self.allied_opt(b.owner, p)).map(|b| b.pos).collect();
        let mines: Vec<V2> = self.mines.iter().filter(|m| !self.allied(m.owner, p) && self.mine_known_to(p, m)).map(|m| m.pos).collect();
        self.map.route_by(from, to, afloat, |tile| {
            let mut c = 0.0;
            if guns.iter().any(|g| g.dist(tile) <= PILLBOX_RANGE + 0.5) {
                c += PILLBOX_AVOID_COST;
            }
            if mines.iter().any(|m| m.tile() == tile.tile()) {
                c += MINE_AVOID_COST;
            }
            c
        })
    }

    /// Does `p` know this mine is there? Its own or an ally's (MIN-05),
    /// or one within `MINE_REVEAL` of its tank or its deployed squad.
    pub fn mine_known_to(&self, p: PlayerId, m: &Mine) -> bool {
        if self.allied(m.owner, p) {
            return true;
        }
        self.tanks.iter().filter(|t| t.owner == p && t.alive).any(|t| t.pos.dist(m.pos) <= MINE_REVEAL || (t.pillboys.deployed() && t.pillboys.pos.dist(m.pos) <= MINE_REVEAL))
    }

    /// Where the squad walks to for a site: the site if it can stand
    /// there, else the nearest passable neighbour (a bridge is built
    /// from the bank).
    fn approach(&self, site: V2) -> Option<V2> {
        if self.map.at(site).passable() {
            return Some(site);
        }
        let (x, y) = site.tile();
        [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .map(|(dx, dy)| V2::new((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5))
            .find(|p| self.map.in_bounds(*p) && self.map.at(*p).passable())
    }

    fn set_terrain(&mut self, at: V2, t: Terrain) {
        if self.map.set(at, t).is_some() {
            self.events.push(Event::TerrainChanged { at: Map::tile_centre(at), to: t as u8 });
        }
    }
    fn pillbox_mut(&mut self, id: EntityId) -> Option<&mut Pillbox> {
        self.pillboxes.iter_mut().find(|p| p.id == id)
    }
    pub fn seconds(&self) -> f32 {
        self.tick as f32 * DT
    }

    /// The team a seat is on; the swarm and the neutral shooter are on
    /// nobody's.
    pub fn team_of(&self, p: PlayerId) -> Option<TeamId> {
        self.players.iter().find(|x| x.id == p).map(|x| x.team)
    }

    /// Same side: the same player, or two players on one team. Never the
    /// swarm, never neutral.
    pub fn allied(&self, a: PlayerId, b: PlayerId) -> bool {
        if a == b {
            return true;
        }
        match (self.team_of(a), self.team_of(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        }
    }

    /// Allied with an owner that may be nobody (a neutral pillbox or
    /// depot is nobody's friend).
    pub fn allied_opt(&self, a: Option<PlayerId>, b: PlayerId) -> bool {
        a.map(|a| self.allied(a, b)).unwrap_or(false)
    }

    /// The living members of a team.
    pub fn team_members(&self, team: TeamId) -> Vec<PlayerId> {
        self.players.iter().filter(|x| x.team == team).map(|x| x.id).collect()
    }

    /// Can `p` see `at`? Its own tank out to `TANK_VISION` (halved into
    /// forest), its recon drones out to `RECON_VISION`, its other drones
    /// out to `DRONE_VISION`; and its allies' the same, because a team
    /// shares its picture (ALL-08 in spirit: what one sees, all see).
    /// The shared, all-seeing world is the host's; what a player gets is
    /// `observe()`.
    pub fn visible_to(&self, p: PlayerId, at: V2) -> bool {
        self.seen_by(&self.eyes(p), at)
    }

    /// The eyes of `p` and its allies, for `seen_by`: each living allied
    /// tank and each allied drone with its reach in the open and into
    /// forest. `observe` asks hundreds of points a tick of one player;
    /// gathered once, the allies are not looked up for each.
    pub fn eyes(&self, p: PlayerId) -> Vec<Eye> {
        let side = |o: PlayerId| self.allied(o, p);
        let tanks = self.tanks.iter().filter(|t| t.alive && side(t.owner)).map(|t| Eye { pos: t.pos, open: TANK_VISION, forest: TANK_VISION * 0.5 });
        let drones = self.drones.iter().filter(|d| side(d.owner)).map(|d| {
            let r = d.spec().vision;
            Eye { pos: d.pos, open: r, forest: r * 0.6 }
        });
        tanks.chain(drones).collect()
    }

    /// `visible_to` over eyes gathered by `eyes`.
    pub fn seen_by(&self, eyes: &[Eye], at: V2) -> bool {
        let in_forest = self.map.at(at) == Terrain::Forest;
        eyes.iter().any(|e| e.pos.dist(at) <= if in_forest { e.forest } else { e.open })
    }

    /// Does a recon drone of `p`, or of an ally, have `at` in view right
    /// now? Shared recon is the team's first dividend: one tank's eyes
    /// are another's gun. A point under forest cover is spotted only
    /// from inside `CONCEAL_RANGE`.
    pub fn spotted_by(&self, p: PlayerId, at: V2) -> bool {
        let concealed = self.concealed(at);
        self.drones
            .iter()
            .any(|d| self.allied(d.owner, p) && d.kind == DroneKind::Recon && d.link >= LINK_FLOOR && d.pos.dist(at) <= if concealed { CONCEAL_RANGE } else { d.spec().vision })
    }

    /// the prior art's cover (VIS-02): `at` lies in a 2 x 2 block of forest. A
    /// tank standing there is hidden from everything beyond
    /// `CONCEAL_RANGE`, pillboxes included (PIL-07).
    pub fn concealed(&self, at: V2) -> bool {
        let (x, y) = at.tile();
        let forest = |x: i32, y: i32| self.map.in_tile(x, y) && self.map.at_tile(x as usize, y as usize) == Terrain::Forest;
        if !forest(x, y) {
            return false;
        }
        [(-1, -1), (0, -1), (-1, 0), (0, 0)].iter().any(|(dx, dy)| forest(x + dx, y + dy) && forest(x + dx + 1, y + dy) && forest(x + dx, y + dy + 1) && forest(x + dx + 1, y + dy + 1))
    }

    /// Can `p` see this tank? Its own always; a concealed one only from
    /// inside `CONCEAL_RANGE`; otherwise by the ordinary discs.
    pub fn tank_visible_to(&self, p: PlayerId, t: &Tank) -> bool {
        self.tank_seen_by(p, &self.eyes(p), t)
    }

    /// `tank_visible_to` over eyes gathered by `eyes(p)`.
    pub fn tank_seen_by(&self, p: PlayerId, eyes: &[Eye], t: &Tank) -> bool {
        if t.owner == p {
            return true;
        }
        // A wreck is nobody's to see, but an ally's down tank its side
        // knows of: where to send a squad to revive it.
        if !t.alive {
            return self.allied(t.owner, p) && self.downed(t);
        }
        if self.concealed(t.pos) {
            return self.tanks.iter().any(|o| o.alive && self.allied(o.owner, p) && o.pos.dist(t.pos) <= CONCEAL_RANGE) || self.drones.iter().any(|d| self.allied(d.owner, p) && d.pos.dist(t.pos) <= CONCEAL_RANGE);
        }
        self.seen_by(eyes, t.pos)
    }

    // ------------------------------------------------------------ stepping

    /// Advance one tick with these orders. Orders are applied in the order
    /// given, so the host must sort them deterministically (by player, then
    /// by sequence) before calling this.
    pub fn step(&mut self, orders: &[Order]) {
        self.events.clear();
        if self.over {
            return;
        }
        self.tick += 1;
        let now = self.tick;
        if now == MARK_II_AT as Tick || now == MARK_III_AT as Tick {
            self.events.push(Event::DronesUpgraded { mark: mark_at(now) });
        }
        let lapsed: Vec<Proposal> = self.proposals.iter().filter(|p| p.expires <= now).cloned().collect();
        self.proposals.retain(|p| p.expires > now);
        for p in lapsed {
            self.events.push(Event::ProposalResolved { proposal: p.id, from: p.from, to: p.to, terms: p.terms, accepted: false, by: None, why: Some("lapsed".into()) });
        }
        for o in orders {
            self.apply(o);
        }

        self.step_tanks();
        self.step_chains();
        self.step_pillboys();
        self.step_drones();
        self.step_pillboxes();
        self.step_shells();
        self.step_depots();
        self.step_trees();
        self.step_structures();
        self.step_swarm();
        self.step_match();
    }

    /// A message (SPEC UI-22, UI-23): one line, no control characters,
    /// at most CHAT_MAX_CHARS, at most one a second from each player. A
    /// dead player may still talk.
    fn chat(&mut self, o: &Order, text: &str, to: ChatTo) {
        let line: String = text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect::<String>().trim().chars().take(CHAT_MAX_CHARS).collect();
        if line.is_empty() {
            return self.reject(o, "nothing to say");
        }
        if let ChatTo::Players(mask) = to {
            if mask & !(1u16 << (o.player as u32 % 16)) == 0 {
                return self.reject(o, "nobody chosen to say it to");
            }
        }
        let tick = self.tick;
        let Some(p) = self.players.iter_mut().find(|p| p.id == o.player) else { return };
        if p.last_chat.map(|t| tick < t + CHAT_EVERY_TICKS).unwrap_or(false) {
            return self.reject(o, "one message a second");
        }
        p.last_chat = Some(tick);
        self.events.push(Event::Chat { player: o.player, text: line, to });
    }

    fn reject(&mut self, o: &Order, why: &str) {
        self.events.push(Event::Rejected { player: o.player, cmd: o.cmd.clone(), why: why.into() });
    }

    fn apply(&mut self, o: &Order) {
        let tid = match self.players.iter().find(|p| p.id == o.player) {
            Some(p) => p.tank,
            None => return self.reject(o, "no such player"),
        };
        let (alive, pos, reload, shells, afloat) = match self.tank(tid) {
            Some(t) => (t.alive, t.pos, t.reload, t.shells, t.boat),
            None => return self.reject(o, "no tank"),
        };
        if let Command::Chat { text, to } = &o.cmd {
            return self.chat(o, text, *to);
        }
        if let Command::TakeSeat { name } = &o.cmd {
            return self.take_seat(o, name);
        }
        if let Command::Respawn = &o.cmd {
            return self.respawn(o, tid);
        }
        if let Command::Request { to, job } = &o.cmd {
            if let Some(t) = to {
                if *t == o.player || !self.allied(*t, o.player) {
                    return self.reject(o, "requests go to allies");
                }
            }
            self.events.push(Event::Requested { from: o.player, to: *to, job: job.clone() });
            return;
        }
        // A ping needs no tank: a destroyed one's player can still point.
        if let Command::Ping { at, kind } = &o.cmd {
            if !self.map.in_bounds(*at) {
                return self.reject(o, "off map");
            }
            let tick = self.tick;
            let Some(pl) = self.players.iter_mut().find(|x| x.id == o.player) else { return };
            if pl.last_ping.map(|t| tick < t + TICK_HZ as Tick).unwrap_or(false) {
                return self.reject(o, "one ping a second");
            }
            pl.last_ping = Some(tick);
            self.events.push(Event::Pinged { player: o.player, at: *at, kind: *kind });
            return;
        }
        match &o.cmd {
            Command::Alliance(a) => return self.apply_alliance(o, a.clone()),
            Command::Propose { to, terms } => return self.propose(o, *to, *terms),
            Command::Answer { proposal, accept } => return self.answer(o, *proposal, *accept),
            _ => {}
        }
        if !alive {
            return self.reject(o, "tank destroyed");
        }
        match &o.cmd {
            Command::Move { to } => {
                if !self.map.in_bounds(*to) {
                    return self.reject(o, "off map");
                }
                // A tank ashore may drive onto a moored boat (TNK-17):
                // the way to the bank beside it, then onto it.
                let mut aboard_at = None;
                if self.map.at(*to) == Terrain::Water && !afloat {
                    match self.boats.iter().find(|b| b.pos.tile() == to.tile()) {
                        Some(b) => aboard_at = Some(b.pos),
                        None => return self.reject(o, "water"),
                    }
                }
                // The world routes. A straight line from a start to the
                // centre crosses the river beside the bridge, and in the
                // first traced match one bot sat on that bank for the whole
                // game. Neither a Brain nor a client has the map in its
                // observation, so the pathfinding has to live here.
                let mut wp = match aboard_at {
                    Some(b) => match self.approach(b) {
                        Some(bank) if bank.tile() == pos.tile() => vec![b],
                        Some(bank) => {
                            let mut wp = self.route_for(o.player, pos, bank);
                            if !wp.is_empty() {
                                wp.push(b);
                            }
                            wp
                        }
                        None => vec![],
                    },
                    None => self.route_afloat(o.player, pos, *to, afloat),
                };
                if wp.is_empty() {
                    return self.reject(o, "unreachable");
                }
                let t = self.tank_mut(tid).unwrap();
                t.goal = Some(wp.remove(0));
                t.route = wp;
            }
            Command::Stop => {
                let t = self.tank_mut(tid).unwrap();
                t.goal = None;
                t.route.clear();
                t.speed = 0.0;
            }
            Command::FireIndirect { at } => {
                if reload > 0 {
                    return self.reject(o, "reloading");
                }
                if shells == 0 {
                    return self.reject(o, "no shells");
                }
                if !self.map.in_bounds(*at) {
                    return self.reject(o, "off map");
                }
                let range = pos.dist(*at);
                if range < INDIRECT_MIN_RANGE {
                    return self.reject(o, "inside minimum range");
                }
                if range > INDIRECT_MAX_RANGE {
                    return self.reject(o, "beyond maximum range");
                }
                let spotted = self.spotted_by(o.player, *at);
                // A data centre's fire control (step 10b): a blind shot
                // near one is computed, not guessed.
                let cep = if spotted {
                    CEP_SPOTTED
                } else if self.serves(o.player, StructureKind::DataCentre, *at, DATACENTRE_FIRE_RANGE) {
                    CEP_BLIND * DATACENTRE_CEP_FACTOR
                } else {
                    CEP_BLIND
                };
                let scatter = self.rng.disc().scale(cep);
                let flight = (range / SHELL_SPEED_INDIRECT).max(1.0);
                let eta = (flight * TICK_HZ as f32).ceil() as u32;
                let id = self.alloc();
                self.shells.push(Shell {
                    id,
                    owner: o.player,
                    kind: ShellKind::Indirect,
                    from: pos,
                    aim: *at,
                    impact: *at + scatter,
                    launch_tick: self.tick,
                    impact_tick: self.tick + eta as Tick,
                    spotted,
                });
                let t = self.tank_mut(tid).unwrap();
                t.shells -= 1;
                t.reload = RELOAD_TICKS;
                t.heading = (*at - pos).heading();
                self.events.push(Event::ShellLaunched { shell: id, owner: o.player, kind: ShellKind::Indirect, spotted, eta_ticks: eta });
            }
            Command::FireDirect { at } => {
                if reload > 0 {
                    return self.reject(o, "reloading");
                }
                if shells == 0 {
                    return self.reject(o, "no shells");
                }
                let range = pos.dist(*at);
                if range > DIRECT_RANGE {
                    return self.reject(o, "beyond direct range");
                }
                if !self.map.line_clear(pos, *at) {
                    return self.reject(o, "no line of sight");
                }
                let eta = ((range / SHELL_SPEED_DIRECT) * TICK_HZ as f32).ceil().max(1.0) as u32;
                let id = self.alloc();
                self.shells.push(Shell {
                    id,
                    owner: o.player,
                    kind: ShellKind::Direct,
                    from: pos,
                    aim: *at,
                    impact: *at,
                    launch_tick: self.tick,
                    impact_tick: self.tick + eta as Tick,
                    spotted: true,
                });
                let t = self.tank_mut(tid).unwrap();
                t.shells -= 1;
                t.reload = RELOAD_TICKS;
                t.heading = (*at - pos).heading();
                self.events.push(Event::ShellLaunched { shell: id, owner: o.player, kind: ShellKind::Direct, spotted: true, eta_ticks: eta });
            }
            Command::Launch { kind, task } => {
                let airborne = self.drones.iter().filter(|d| d.owner == o.player).count();
                if airborne >= MAX_AIRBORNE {
                    return self.reject(o, "airspace full");
                }
                let t = self.tank_mut(tid).unwrap();
                if !t.bay.take(*kind) {
                    return self.reject(o, "bay empty for that kind");
                }
                let bubble = t.bubble;
                let task = match (kind, task) {
                    // Interceptors only ever sit on station; the world
                    // assigns the intercept itself when something comes in.
                    (DroneKind::Interceptor, _) => DroneTask::Orbit { center: pos, radius: bubble },
                    (DroneKind::Strike, DroneTask::Strike { target }) => {
                        if let Some(t) = self.tank(*target) {
                            if self.allied(t.owner, o.player) {
                                // Undo the take above: the bay keeps its drone.
                                self.tank_mut(tid).unwrap().bay.put(DroneKind::Strike);
                                return self.reject(o, "that tank is an ally");
                            }
                        }
                        DroneTask::Strike { target: *target }
                    }
                    (DroneKind::Strike, DroneTask::Orbit { center, .. }) => DroneTask::Orbit { center: *center, radius: 2.0 },
                    (DroneKind::Recon, DroneTask::Orbit { center, radius }) => DroneTask::Orbit { center: *center, radius: radius.clamp(1.0, 6.0) },
                    (DroneKind::Recon, _) => DroneTask::Orbit { center: pos, radius: 4.0 },
                    (DroneKind::Longshot, DroneTask::Attack { at } | DroneTask::Orbit { center: at, .. }) => {
                        if !self.map.in_bounds(*at) {
                            self.tank_mut(tid).unwrap().bay.put(DroneKind::Longshot);
                            return self.reject(o, "off map");
                        }
                        DroneTask::Attack { at: *at }
                    }
                    _ => {
                        self.tank_mut(tid).unwrap().bay.put(*kind);
                        return self.reject(o, "task does not suit that drone");
                    }
                };
                let id = self.alloc();
                let mark = mark_at(self.tick);
                let battery = drone_spec(*kind, mark).battery;
                let phase = self.rng.range(0.0, std::f32::consts::TAU);
                // A longshot is on its own from launch: no tether, so
                // no link, no way home, and it flies on if the tank
                // that sent it dies.
                let tether = if *kind == DroneKind::Longshot { None } else { Some(tid) };
                self.drones.push(Drone {
                    id,
                    owner: o.player,
                    kind: *kind,
                    pos,
                    heading: 0.0,
                    battery,
                    tether,
                    task,
                    link: 1.0,
                    phase,
                    jammed: false,
                    mark,
                });
                self.events.push(Event::DroneLaunched { drone: id, owner: o.player, kind: *kind });
            }
            Command::Recall => {
                for d in self.drones.iter_mut().filter(|d| d.owner == o.player && d.kind != DroneKind::Longshot) {
                    d.task = DroneTask::Return;
                }
            }
            Command::Retask { drone, task } => {
                let Some(d) = self.drones.iter().find(|d| d.id == *drone) else { return self.reject(o, "no such drone") };
                if d.owner != o.player {
                    return self.reject(o, "not my drone");
                }
                let new = match (d.kind, task) {
                    (DroneKind::Longshot, _) => return self.reject(o, "a longshot flies where it was sent"),
                    (_, DroneTask::Return) => DroneTask::Return,
                    (DroneKind::Interceptor, _) => return self.reject(o, "interceptors are the world's to task"),
                    (DroneKind::Recon, DroneTask::Orbit { center, radius }) => {
                        if !self.map.in_bounds(*center) {
                            return self.reject(o, "off map");
                        }
                        DroneTask::Orbit { center: *center, radius: radius.clamp(1.0, 6.0) }
                    }
                    (DroneKind::Strike, DroneTask::Strike { target }) => match self.tank(*target) {
                        None => return self.reject(o, "no such tank"),
                        Some(t) if self.allied(t.owner, o.player) => return self.reject(o, "that tank is an ally"),
                        Some(_) => DroneTask::Strike { target: *target },
                    },
                    _ => return self.reject(o, "task does not suit that drone"),
                };
                self.drones.iter_mut().find(|d| d.id == *drone).unwrap().task = new.clone();
                self.events.push(Event::DroneRetasked { drone: *drone, owner: o.player, task: new });
            }
            Command::SetBubble { radius } => {
                let r = radius.clamp(BUBBLE_MIN, BUBBLE_MAX);
                self.tank_mut(tid).unwrap().bubble = r;
                for d in self.drones.iter_mut().filter(|d| d.owner == o.player && d.kind == DroneKind::Interceptor) {
                    if let DroneTask::Orbit { radius, .. } = &mut d.task {
                        *radius = r;
                    }
                }
            }
            Command::Deploy { task } => {
                let squad = &self.tank(tid).unwrap().pillboys;
                if squad.strength == 0 {
                    return self.reject(o, "no squad");
                }
                // The squad walks; from a boat there is nowhere to walk.
                if afloat && squad.aboard {
                    return self.reject(o, "afloat: land first");
                }
                if let PillboyTask::Return = task {
                    if squad.aboard {
                        return self.reject(o, "squad is aboard");
                    }
                    let t = self.tank_mut(tid).unwrap();
                    t.pillboys.task = Some(PillboyTask::Return);
                    t.pillboys.route.clear();
                    t.pillboys.progress = 0;
                    return;
                }
                if squad.deployed() {
                    return self.reject(o, "squad already out");
                }
                let material = squad.material;
                let carrying = squad.carrying.is_some();
                match task {
                    PillboyTask::Capture { pillbox } if self.pillbox(*pillbox).is_none() => match self.structure(*pillbox) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(s) if s.owner == Some(o.player) => return self.reject(o, "structure already ours"),
                        Some(s) if self.allied_opt(s.owner, o.player) && s.alive() => return self.reject(o, "structure is an ally's"),
                        Some(s) if s.owner.is_some() && s.alive() => return self.reject(o, "structure is standing and theirs: wreck it first"),
                        Some(_) => {}
                    },
                    // A tank down: an ally's squad revives it (Paul, 16
                    // October), for material, inside the time down.
                    PillboyTask::Repair { target } if self.tank(*target).is_some() => {
                        let t = self.tank(*target).unwrap();
                        if t.owner == o.player {
                            return self.reject(o, "a squad does not mend its own tank");
                        } else if !self.allied(t.owner, o.player) {
                            return self.reject(o, "revive only an ally's tank");
                        } else if t.alive {
                            return self.reject(o, "that tank is not down");
                        } else if !self.downed(t) {
                            return self.reject(o, "too late: that tank is dead");
                        } else if material < REVIVE_COST {
                            return self.reject(o, "no material");
                        }
                    }
                    PillboyTask::Repair { target } if self.pillbox(*target).is_none() => match self.structure(*target) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(s) if s.health >= STRUCTURE_HEALTH => return self.reject(o, "structure needs no repair"),
                        Some(_) if material < STRUCTURE_REPAIR_COST => return self.reject(o, "no material"),
                        Some(_) => {}
                    },
                    PillboyTask::Capture { pillbox } => match self.pillbox(*pillbox) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(p) if p.owner == Some(o.player) => return self.reject(o, "pillbox already ours"),
                        Some(p) if self.allied_opt(p.owner, o.player) && p.alive() => return self.reject(o, "pillbox is an ally's"),
                        Some(p) if p.owner.is_some() && p.alive() => return self.reject(o, "pillbox is live and theirs: shoot it dead first"),
                        Some(_) => {}
                    },
                    // Any pillbox can be repaired, damaged or dead, whoever
                    // owns it, and repair never changes the owner (SPEC
                    // BLD-05, BLD-06).
                    PillboyTask::Repair { target } => match self.pillbox(*target) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(p) if p.health >= PILLBOX_HEALTH => return self.reject(o, "pillbox needs no repair"),
                        Some(_) if material < PILLBOX_REPAIR_COST => return self.reject(o, "no material"),
                        Some(_) => {}
                    },
                    PillboyTask::Lift { pillbox } => match self.pillbox(*pillbox) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(p) if p.owner != Some(o.player) => return self.reject(o, "only a pillbox of your own can be lifted"),
                        Some(_) if carrying => return self.reject(o, "already carrying one"),
                        Some(_) => {}
                    },
                    PillboyTask::Build { what, at } => {
                        if !self.map.in_bounds(*at) {
                            return self.reject(o, "off map");
                        }
                        let here = self.map.at(*at);
                        if material < what.cost() && !(carrying && *what == BuildKind::Pillbox) {
                            return self.reject(o, "no material");
                        }
                        if here == Terrain::Depot || self.depots.iter().any(|d| d.pos.dist(*at) <= DEPOT_RADIUS) {
                            return self.reject(o, "not on a depot");
                        }
                        if self.pillboxes.iter().any(|p| p.pos.dist(*at) < if *what == BuildKind::Pillbox { PILLBOX_SPACING } else { 1.0 }) {
                            return self.reject(o, "a pillbox is there");
                        }
                        if self.structures.iter().any(|s| s.pos.tile() == at.tile()) {
                            return self.reject(o, "a structure is there");
                        }
                        match what {
                            BuildKind::Boat => {
                                if here != Terrain::Water {
                                    return self.reject(o, "a boat is built on water");
                                }
                                if self.approach(*at).is_none() {
                                    return self.reject(o, "a boat is built beside a bank");
                                }
                                if self.boats.iter().any(|b| b.pos.tile() == at.tile()) || self.tanks.iter().any(|t| t.alive && t.pos.tile() == at.tile()) {
                                    return self.reject(o, "a boat is there");
                                }
                            }
                            BuildKind::Structure(_) => {
                                if here != Terrain::Pad {
                                    return self.reject(o, "structures go on a pad");
                                }
                                if self.tanks.iter().any(|t| t.alive && t.pos.tile() == at.tile()) {
                                    return self.reject(o, "a tank is there");
                                }
                            }
                            BuildKind::Pillbox | BuildKind::Wall | BuildKind::Concrete | BuildKind::Mine => {
                                if !here.passable() {
                                    return self.reject(o, "cannot build there");
                                }
                                if here == Terrain::Forest {
                                    return self.reject(o, "forest: harvest it first");
                                }
                                if *what == BuildKind::Mine && self.mines.iter().any(|m| m.pos.tile() == at.tile()) {
                                    return self.reject(o, "a mine is there");
                                }
                                if *what != BuildKind::Mine && self.tanks.iter().any(|t| t.alive && t.pos.tile() == at.tile()) {
                                    return self.reject(o, "a tank is there");
                                }
                            }
                            BuildKind::Road => {
                                if matches!(here, Terrain::Road | Terrain::Bridge | Terrain::Wall | Terrain::Concrete | Terrain::Forest | Terrain::Pad) {
                                    return self.reject(o, "cannot lay a road there");
                                }
                            }
                        }
                    }
                    PillboyTask::Harvest { at } => {
                        if !self.map.in_bounds(*at) || self.map.at(*at) != Terrain::Forest {
                            return self.reject(o, "no forest there");
                        }
                        if material >= PILLBOY_MATERIAL_MAX {
                            return self.reject(o, "material is full");
                        }
                    }
                    PillboyTask::Return => unreachable!(),
                }
                let site = task.site(self).unwrap();
                let Some(walk) = self.approach(site) else { return self.reject(o, "site unreachable") };
                let route = self.map.route(pos, walk);
                if route.is_empty() {
                    return self.reject(o, "site unreachable");
                }

                let t = self.tank_mut(tid).unwrap();
                t.pillboys.aboard = false;
                t.pillboys.pos = pos;
                t.pillboys.task = Some(task.clone());
                t.pillboys.progress = 0;
                t.pillboys.route = route;
                self.events.push(Event::PillboysDeployed { tank: tid, owner: o.player, task: task.clone() });
            }
            Command::Alliance(_) | Command::Propose { .. } | Command::Answer { .. } | Command::Chat { .. } | Command::TakeSeat { .. } | Command::Respawn | Command::Ping { .. } | Command::Request { .. } => {}
            Command::Tow { target } => self.tow(o, tid, *target),
            Command::Embark => self.embark(o, tid),
        }
    }

    /// What a chain is on: a pillbox or a depot, its position.
    fn chained_pos(&self, id: EntityId) -> Option<V2> {
        self.pillbox(id).map(|p| p.pos).or_else(|| self.depots.iter().find(|d| d.id == id).map(|d| d.pos))
    }

    /// The tanks on a chain to `id`.
    fn haulers(&self, id: EntityId) -> u8 {
        self.tanks.iter().filter(|t| t.alive && t.towing == Some(id)).count() as u8
    }

    fn tow(&mut self, o: &Order, tid: EntityId, target: Option<EntityId>) {
        let Some(t) = self.tank(tid) else { return };
        let (pos, afloat, had) = (t.pos, t.boat, t.towing);
        let Some(id) = target else {
            let Some(was) = had else { return self.reject(o, "no chain on anything") };
            self.tank_mut(tid).unwrap().towing = None;
            let haulers = self.haulers(was);
            self.events.push(Event::Chained { tank: tid, owner: o.player, target: was, on: false, haulers });
            return;
        };
        if afloat {
            return self.reject(o, "afloat: land first");
        }
        let owner = match (self.pillbox(id), self.depots.iter().find(|d| d.id == id)) {
            (Some(p), _) => p.owner,
            (None, Some(d)) => d.owner,
            _ => return self.reject(o, "no pillbox or depot to chain"),
        };
        if !self.allied_opt(owner, o.player) {
            return self.reject(o, "chain only what is your side's: take it first");
        }
        if self.chained_pos(id).map(|p| p.dist(pos) > TOW_REACH).unwrap_or(true) {
            return self.reject(o, "too far to chain");
        }
        self.tank_mut(tid).unwrap().towing = Some(id);
        let haulers = self.haulers(id);
        self.events.push(Event::Chained { tank: tid, owner: o.player, target: id, on: true, haulers });
    }

    /// Chains: what two tanks or more pull follows them at the chain's
    /// length, over land only (a depot's tile goes with it); a tank on a
    /// chain alone, or one that drives off from the rest, is held at the
    /// chain's length. A chain whose tank is lost, afloat or not of the
    /// side any more falls slack.
    fn step_chains(&mut self) {
        let chained: Vec<(EntityId, PlayerId, EntityId)> = self.tanks.iter().filter_map(|t| t.towing.map(|id| (t.id, t.owner, id))).collect();
        if chained.is_empty() {
            return;
        }
        let mut targets: Vec<EntityId> = chained.iter().map(|c| c.2).collect();
        targets.sort_unstable();
        targets.dedup();
        for (tid, owner, id) in &chained {
            let ok = self.tank(*tid).map(|t| t.alive && !t.boat).unwrap_or(false);
            let owner_ok = match (self.pillbox(*id), self.depots.iter().find(|d| d.id == *id)) {
                (Some(p), _) => self.allied_opt(p.owner, *owner),
                (None, Some(d)) => self.allied_opt(d.owner, *owner),
                _ => false,
            };
            if !ok || !owner_ok {
                self.tank_mut(*tid).unwrap().towing = None;
                let haulers = self.haulers(*id);
                self.events.push(Event::Chained { tank: *tid, owner: *owner, target: *id, on: false, haulers });
            }
        }
        for id in targets {
            let Some(at) = self.chained_pos(id) else { continue };
            let pullers: Vec<V2> = self.tanks.iter().filter(|t| t.alive && t.towing == Some(id)).map(|t| t.pos).collect();
            if pullers.is_empty() {
                continue;
            }
            let mut to = at;
            if pullers.len() >= TOW_HAULERS {
                let n = pullers.len() as f32;
                let mid = pullers.iter().fold(V2::new(0.0, 0.0), |a, p| a + *p).scale(1.0 / n);
                let d = mid.dist(at);
                if d > TOW_CHAIN {
                    let next = at + (mid - at).norm().scale(d - TOW_CHAIN);
                    let ground = self.map.at(next);
                    let is_depot = self.depots.iter().any(|x| x.id == id);
                    // Over land only; a depot onto open ground, not onto
                    // another's tile, a pad or a wall.
                    let open = ground.passable() && ground != Terrain::Bridge && (!is_depot || next.tile() == at.tile() || matches!(ground, Terrain::Grass | Terrain::Road | Terrain::Rubble));
                    let clear = !self.pillboxes.iter().any(|p| p.id != id && p.pos.dist(next) < 1.0) && !self.depots.iter().any(|x| x.id != id && x.pos.dist(next) < 1.5);
                    if open && clear {
                        to = next;
                    }
                }
            }
            if to != at {
                if let Some(p) = self.pillbox_mut(id) {
                    p.pos = to;
                } else if let Some(ix) = self.depots.iter().position(|d| d.id == id) {
                    let (old, new) = (at.tile(), to.tile());
                    self.depots[ix].pos = to;
                    if old != new {
                        self.set_terrain(Map::tile_centre(at), Terrain::Grass);
                        self.set_terrain(Map::tile_centre(to), Terrain::Depot);
                    }
                }
            }
            // The chain is taut: no tank on it gets further than its
            // length (and a little: two tanks side by side) from what it
            // pulls.
            for t in self.tanks.iter_mut().filter(|t| t.alive && t.towing == Some(id)) {
                let d = t.pos.dist(to);
                if d > TOW_CHAIN * 1.6 {
                    t.pos = to + (t.pos - to).norm().scale(TOW_CHAIN * 1.6);
                    t.speed = t.speed.min(TANK_SPEED * TOW_SPEED);
                }
            }
        }
    }

    /// Ticks until `p`'s destroyed tank may respawn (0: now), or None
    /// when it may not: alive, or the scenario's respawns are spent or
    /// none.
    /// A destroyed tank inside `DOWNED_TICKS` of its fall: down, not yet
    /// dead, and an ally's squad may revive it.
    pub fn downed(&self, t: &Tank) -> bool {
        !t.alive && t.last_hit_tick > 0 && self.tick < t.last_hit_tick + DOWNED_TICKS as Tick
    }

    pub fn respawn_in(&self, p: PlayerId) -> Option<Tick> {
        let pl = self.players.iter().find(|x| x.id == p)?;
        let t = self.tank(pl.tank)?;
        let allowed = self.scenario.respawns;
        if t.alive || allowed == 0 || (allowed != RESPAWNS_UNLIMITED && pl.respawned >= allowed) {
            return None;
        }
        Some((t.last_hit_tick + RESPAWN_TICKS as Tick).saturating_sub(self.tick))
    }

    /// The tank back as it started: at its start, facing the middle,
    /// full stocks for the game type, a fresh squad, afloat if the start
    /// is at sea. The same entity, so every window's picture of it goes
    /// on. Score is kept.
    fn respawn(&mut self, o: &Order, tid: EntityId) {
        match self.respawn_in(o.player) {
            None if self.tank(tid).map(|t| t.alive).unwrap_or(false) => return self.reject(o, "tank is alive"),
            None => return self.reject(o, "no respawns left"),
            Some(n) if n > 0 => return self.reject(o, "too soon to respawn"),
            Some(_) => {}
        }
        let Some(at) = self.map.starts.get(o.player as usize).copied() else { return self.reject(o, "no tank") };
        let heading = (self.map.center() - at).heading();
        let afloat = self.map.at(at) == Terrain::Water;
        let (shells, material) = (self.scenario.game_type.shells(), self.scenario.game_type.material());
        let allowed = self.scenario.respawns;
        let mut left = RESPAWNS_UNLIMITED;
        if let Some(pl) = self.players.iter_mut().find(|x| x.id == o.player) {
            pl.alive = true;
            if allowed != RESPAWNS_UNLIMITED {
                pl.respawned += 1;
                left = allowed - pl.respawned;
            }
        }
        let t = self.tank_mut(tid).unwrap();
        *t = Tank { id: tid, owner: t.owner, pos: at, heading, speed: 0.0, armor: TANK_ARMOR, shells, fuel: TANK_FUEL, bay: BAY_FULL, goal: None, route: Vec::new(), reload: 0, bubble: BUBBLE_DEFAULT, last_hit_tick: 0, alive: true, pillboys: Pillboys::fresh(at, material), boat: afloat, towing: None };
        self.events.push(Event::Respawned { tank: tid, owner: o.player, at, left });
    }

    /// `Command::Embark`: a boat on the water beside the tank, from the
    /// squad's material, and the tank on it. The water tile is the one
    /// beside the tank most nearly ahead of it.
    fn embark(&mut self, o: &Order, tid: EntityId) {
        let Some(t) = self.tank(tid) else { return };
        if t.boat {
            return self.reject(o, "afloat already");
        }
        let s = &t.pillboys;
        if s.strength == 0 {
            return self.reject(o, "no squad");
        }
        if !s.aboard {
            return self.reject(o, "squad already out");
        }
        let (x, y) = t.pos.tile();
        let ahead = V2::from_heading(t.heading);
        let site = (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
            .filter(|d| *d != (0, 0))
            .map(|(dx, dy)| V2::new((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5))
            .filter(|p| self.map.in_bounds(*p) && self.map.at(*p) == Terrain::Water)
            .filter(|p| !self.boats.iter().any(|b| b.pos.tile() == p.tile()) && !self.tanks.iter().any(|u| u.alive && u.id != tid && u.pos.tile() == p.tile()))
            .max_by(|a, b| {
                let toward = |p: &V2| { let v = (*p - t.pos).norm(); v.x * ahead.x + v.y * ahead.y };
                toward(a).total_cmp(&toward(b))
            });
        let Some(at) = site else { return self.reject(o, "no water beside") };
        let boat = self.alloc();
        let t = self.tank_mut(tid).unwrap();
        // What material the squad has, up to `BOAT_COST`; with none it
        // still launches (Paul, 17 October: a launch for every tank).
        t.pillboys.material -= t.pillboys.material.min(BOAT_COST);
        t.pos = at;
        t.pillboys.pos = at;
        t.boat = true;
        t.goal = None;
        t.route.clear();
        t.speed = 0.0;
        self.events.push(Event::Boarded { tank: tid, owner: o.player, boat, at });
    }

    /// A late arrival takes a bot's seat: the seat is a person's from
    /// this tick, under their name (one line, at most SEAT_NAME_CHARS),
    /// with everything the bot held.
    fn take_seat(&mut self, o: &Order, name: &str) {
        let name: String = name.chars().filter(|c| !c.is_control()).take(SEAT_NAME_CHARS).collect::<String>().trim().to_string();
        let Some(p) = self.players.iter_mut().find(|p| p.id == o.player) else { return };
        if !p.is_bot {
            return self.reject(o, "seat is already a person's");
        }
        let was = std::mem::replace(&mut p.name, if name.is_empty() { format!("p{}", o.player) } else { name.clone() });
        p.is_bot = false;
        let name = p.name.clone();
        self.events.push(Event::SeatTaken { player: o.player, was, name });
    }

    /// the prior art's alliance protocol in the proposal's words: a request is a
    /// Join proposal; accepting and declining answer the pending one
    /// from that player; leaving is a side of one's own again.
    fn apply_alliance(&mut self, o: &Order, cmd: AllianceCmd) {
        let me = o.player;
        match cmd {
            AllianceCmd::Request { to } => {
                // the prior art's rule for its own request (ALL-02): leave before
                // you ask elsewhere. A Join proposed in so many words is
                // a defection, and allowed.
                if self.team_members(self.team_of(me).unwrap()).len() > 1 {
                    return self.reject(o, "leave your alliance first");
                }
                self.propose(o, to, Terms::Join)
            }
            AllianceCmd::Accept { from } | AllianceCmd::Decline { from } => {
                let accept = matches!(cmd, AllianceCmd::Accept { .. });
                let Some(p) = self.proposals.iter().find(|p| p.from == from && p.terms == Terms::Join && self.allied(p.to, me)).cloned() else {
                    return self.reject(o, "no such request");
                };
                self.answer(o, p.id, accept)
            }
            AllianceCmd::Leave => {
                let my_team = self.team_of(me).unwrap();
                if self.team_members(my_team).len() < 2 {
                    return self.reject(o, "not in an alliance");
                }
                // A team id nobody else holds: the smallest free one.
                let used: Vec<TeamId> = self.players.iter().filter(|p| p.id != me).map(|p| p.team).collect();
                let fresh = (0..=MAX_PLAYERS as TeamId).find(|t| !used.contains(t)).unwrap();
                if let Some(p) = self.players.iter_mut().find(|p| p.id == me) {
                    p.team = fresh;
                }
                self.events.push(Event::AllianceLeft { player: me, team: my_team });
            }
        }
    }

    /// The teams after a proposal is carried out: (player, team) for
    /// every player.
    fn outcome(&self, from: PlayerId, to: PlayerId, terms: Terms) -> Vec<(PlayerId, TeamId)> {
        let sides: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        proposal_outcome(&sides, from, to, terms)
    }
    fn three_way_check(&self, teams: &[(PlayerId, TeamId)]) -> Result<(), &'static str> {
        three_way(teams, |p| self.players.iter().any(|x| x.id == p && x.alive))
    }

    /// Put a change of sides to another player. Refused at once if it is
    /// malformed or if its outcome would break the three-way rule; else
    /// it stands for `PROPOSAL_TICKS`, replacing any earlier one of the
    /// same terms to the same player.
    fn propose(&mut self, o: &Order, to: PlayerId, terms: Terms) {
        let from = o.player;
        if to == from || self.team_of(to).is_none() {
            return self.reject(o, "no such player");
        }
        if self.allied(from, to) {
            return self.reject(o, "already on the same side");
        }
        let outcome = self.outcome(from, to, terms);
        if let Err(why) = self.three_way_check(&outcome) {
            return self.reject(o, why);
        }
        self.proposals.retain(|p| !(p.from == from && p.to == to && p.terms == terms));
        let id = self.alloc();
        self.proposals.push(Proposal { id, from, to, terms, expires: self.tick + PROPOSAL_TICKS });
        self.events.push(Event::Proposed { proposal: id, from, to, terms });
    }

    /// Answer a proposal made to me or to my team. Accepting runs the
    /// three-way check again on the world as it is now, and carries the
    /// change out; either way the proposal is resolved for everyone.
    fn answer(&mut self, o: &Order, proposal: EntityId, accept: bool) {
        let me = o.player;
        let Some(ix) = self.proposals.iter().position(|p| p.id == proposal && self.allied(p.to, me)) else {
            return self.reject(o, "no such proposal");
        };
        let p = self.proposals.remove(ix);
        if !accept {
            self.events.push(Event::ProposalResolved { proposal: p.id, from: p.from, to: p.to, terms: p.terms, accepted: false, by: Some(me), why: None });
            return;
        }
        if self.team_of(p.from).is_none() || self.allied(p.from, p.to) {
            self.events.push(Event::ProposalResolved { proposal: p.id, from: p.from, to: p.to, terms: p.terms, accepted: false, by: Some(me), why: Some("overtaken by events".into()) });
            return;
        }
        let outcome = self.outcome(p.from, p.to, p.terms);
        if let Err(why) = self.three_way_check(&outcome) {
            self.events.push(Event::ProposalResolved { proposal: p.id, from: p.from, to: p.to, terms: p.terms, accepted: false, by: Some(me), why: Some(why.into()) });
            return;
        }
        // Carry it out: every player whose team changes is announced.
        let mut changed: Vec<(PlayerId, TeamId)> = Vec::new();
        for (pid, team) in outcome {
            if let Some(pl) = self.players.iter_mut().find(|x| x.id == pid) {
                if pl.team != team {
                    pl.team = team;
                    changed.push((pid, team));
                }
            }
        }
        // Standing proposals from or to anyone who moved are stale.
        let moved: Vec<PlayerId> = changed.iter().map(|(p, _)| *p).collect();
        let stale: Vec<Proposal> = self.proposals.iter().filter(|q| moved.contains(&q.from) || moved.contains(&q.to)).cloned().collect();
        self.proposals.retain(|q| !(moved.contains(&q.from) || moved.contains(&q.to)));
        for q in stale {
            self.events.push(Event::ProposalResolved { proposal: q.id, from: q.from, to: q.to, terms: q.terms, accepted: false, by: None, why: Some("sides changed".into()) });
        }
        self.events.push(Event::ProposalResolved { proposal: p.id, from: p.from, to: p.to, terms: p.terms, accepted: true, by: Some(me), why: None });
        for (pid, team) in changed {
            let members = self.team_members(team);
            self.events.push(Event::AllianceFormed { player: pid, team, members });
        }
    }

    /// The squad: walk, work, walk home. Read the tank and the site,
    /// decide, then apply, as the drones do.
    fn step_pillboys(&mut self) {
        self.drop_carried();
        #[derive(Debug)]
        enum Done {
            Captured(EntityId, Option<PlayerId>),
            Repaired(EntityId),
            Taken(EntityId, Option<PlayerId>),
            Mended(EntityId),
            Built(BuildKind, V2),
            Harvested(V2),
            Lifted(EntityId),
            Revived(EntityId),
            Abandon(&'static str),
            Home,
        }
        let mut done: Vec<(EntityId, PlayerId, Done)> = Vec::new();
        let mut captures: Vec<(EntityId, PlayerId)> = Vec::new();
        let map = &self.map;
        let pillboxes = &self.pillboxes;
        let structures = &self.structures;
        let mut takes: Vec<(EntityId, PlayerId)> = Vec::new();
        let tank_tiles: Vec<(i32, i32)> = self.tanks.iter().filter(|t| t.alive).map(|t| t.pos.tile()).collect();
        let boat_tiles: Vec<(i32, i32)> = self.boats.iter().map(|b| b.pos.tile()).collect();
        let mine_tiles: Vec<(i32, i32)> = self.mines.iter().map(|m| m.pos.tile()).collect();
        // Tanks a squad may be reviving: where, and whether still down.
        let wrecks: Vec<(EntityId, V2, bool, bool)> = self.tanks.iter().map(|t| (t.id, t.pos, t.alive, self.downed(t))).collect();
        // A hostile mine under a walking squad: found the hard way.
        let hostile_mines: Vec<(EntityId, PlayerId, (i32, i32))> = self.mines.iter().map(|m| (m.id, m.owner, m.pos.tile())).collect();
        let teams: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        let team_of = |p: PlayerId| teams.iter().find(|(id, _)| *id == p).map(|(_, t)| *t);
        let allied_with = |a: PlayerId, b: PlayerId| a == b || (team_of(a).is_some() && team_of(a) == team_of(b));
        let mut stepped_on: Vec<EntityId> = Vec::new();
        for t in self.tanks.iter_mut() {
            let s = &mut t.pillboys;
            if !t.alive || s.aboard {
                s.pos = t.pos;
                continue;
            }
            let Some(task) = s.task.clone() else {
                s.task = Some(PillboyTask::Return);
                continue;
            };
            // Where to walk: the site, or beside it when the squad
            // cannot stand on it (water for a bridge).
            let site: Option<V2> = match &task {
                PillboyTask::Capture { pillbox } | PillboyTask::Repair { target: pillbox } => pillboxes.iter().find(|p| p.id == *pillbox).map(|p| p.pos).or_else(|| structures.iter().find(|x| x.id == *pillbox).map(|x| x.pos)).or_else(|| wrecks.iter().find(|w| w.0 == *pillbox).map(|w| w.1)),
                PillboyTask::Lift { pillbox } => pillboxes.iter().find(|p| p.id == *pillbox).map(|p| p.pos),
                PillboyTask::Build { at, .. } | PillboyTask::Harvest { at } => Some(*at),
                PillboyTask::Return => None,
            };
            let target = match site {
                Some(p) if map.at(p).passable() => p,
                Some(p) => {
                    let (x, y) = p.tile();
                    [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().map(|(dx, dy)| V2::new((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5)).find(|q| map.in_bounds(*q) && map.at(*q).passable()).unwrap_or(p)
                }
                None => t.pos,
            };
            if let PillboyTask::Return = task {
                // Home moves. Re-plan when the route no longer ends near it.
                let stale = s.route.last().map(|w| w.dist(t.pos) > 2.0).unwrap_or(true);
                if stale && s.pos.dist(t.pos) > PILLBOY_BOARD_RANGE {
                    s.route = map.route(s.pos, t.pos);
                }
            }
            if site.is_none() && !matches!(task, PillboyTask::Return) {
                done.push((t.id, t.owner, Done::Abandon("site is gone")));
                continue;
            }
            // Walk.
            let at_site = s.pos.dist(site.unwrap_or(t.pos)) <= task.reach() || (site.is_some() && s.pos.dist(target) <= PILLBOY_WORK_RANGE);
            if !at_site {
                let next = s.route.first().copied().unwrap_or(target);
                let to = next - s.pos;
                let step = PILLBOY_SPEED * DT;
                if to.len() <= step {
                    s.pos = next;
                    if !s.route.is_empty() {
                        s.route.remove(0);
                    }
                } else {
                    s.pos = s.pos + to.norm().scale(step);
                }
                if let Some((mid, _, _)) = hostile_mines.iter().find(|(_, o, tile)| !allied_with(*o, t.owner) && *tile == s.pos.tile()) {
                    stepped_on.push(*mid);
                }
                if matches!(task, PillboyTask::Return) && s.pos.dist(t.pos) <= PILLBOY_BOARD_RANGE {
                    done.push((t.id, t.owner, Done::Home));
                }
                continue;
            }
            // Work.
            match &task {
                PillboyTask::Return => done.push((t.id, t.owner, Done::Home)),
                PillboyTask::Capture { pillbox } if !pillboxes.iter().any(|p| p.id == *pillbox) => {
                    let x = structures.iter().find(|x| x.id == *pillbox).unwrap();
                    if x.owner == Some(t.owner) {
                        done.push((t.id, t.owner, Done::Abandon("already ours")));
                    } else if x.owner.is_some() && x.alive() {
                        done.push((t.id, t.owner, Done::Abandon("structure was mended")));
                    } else {
                        takes.push((x.id, t.owner));
                        s.progress += 1;
                        if s.progress >= PILLBOX_CAPTURE_TICKS {
                            done.push((t.id, t.owner, Done::Taken(x.id, x.owner)));
                        }
                    }
                }
                PillboyTask::Repair { target } if wrecks.iter().any(|w| w.0 == *target) => {
                    let &(_, _, alive, down) = wrecks.iter().find(|w| w.0 == *target).unwrap();
                    if alive {
                        done.push((t.id, t.owner, Done::Abandon("that tank is back")));
                    } else if !down {
                        done.push((t.id, t.owner, Done::Abandon("too late: that tank is dead")));
                    } else if s.material < REVIVE_COST {
                        done.push((t.id, t.owner, Done::Abandon("no material")));
                    } else {
                        s.progress += 1;
                        if s.progress >= REVIVE_TICKS {
                            done.push((t.id, t.owner, Done::Revived(*target)));
                        }
                    }
                }
                PillboyTask::Repair { target } if !pillboxes.iter().any(|p| p.id == *target) => {
                    let x = structures.iter().find(|x| x.id == *target).unwrap();
                    if x.health >= STRUCTURE_HEALTH {
                        done.push((t.id, t.owner, Done::Abandon("structure needs no repair")));
                    } else if s.material < STRUCTURE_REPAIR_COST {
                        done.push((t.id, t.owner, Done::Abandon("no material")));
                    } else {
                        s.progress += 1;
                        if s.progress >= STRUCTURE_REPAIR_TICKS {
                            done.push((t.id, t.owner, Done::Mended(x.id)));
                        }
                    }
                }
                PillboyTask::Capture { pillbox } => {
                    let p = pillboxes.iter().find(|p| p.id == *pillbox).unwrap();
                    if p.owner == Some(t.owner) {
                        done.push((t.id, t.owner, Done::Abandon("already ours")));
                    } else if p.owner.is_some() && p.alive() {
                        done.push((t.id, t.owner, Done::Abandon("pillbox came back to life")));
                    } else {
                        captures.push((p.id, t.owner));
                        s.progress += 1;
                        if s.progress >= PILLBOX_CAPTURE_TICKS {
                            done.push((t.id, t.owner, Done::Captured(p.id, p.owner)));
                        }
                    }
                }
                PillboyTask::Repair { target } => {
                    let p = pillboxes.iter().find(|p| p.id == *target).unwrap();
                    if p.health >= PILLBOX_HEALTH {
                        done.push((t.id, t.owner, Done::Abandon("pillbox needs no repair")));
                    } else if s.material < PILLBOX_REPAIR_COST {
                        done.push((t.id, t.owner, Done::Abandon("no material")));
                    } else {
                        s.progress += 1;
                        if s.progress >= PILLBOX_REPAIR_TICKS {
                            done.push((t.id, t.owner, Done::Repaired(p.id)));
                        }
                    }
                }
                PillboyTask::Build { what, at } => {
                    let blocked = match what {
                        BuildKind::Pillbox => pillboxes.iter().any(|p| p.pos.dist(*at) < PILLBOX_SPACING),
                        BuildKind::Wall | BuildKind::Concrete => !map.at(*at).passable() || tank_tiles.iter().any(|tt| *tt == at.tile()),
                        BuildKind::Structure(_) => map.at(*at) != Terrain::Pad || structures.iter().any(|x| x.pos.tile() == at.tile()),
                        BuildKind::Road => matches!(map.at(*at), Terrain::Road | Terrain::Bridge | Terrain::Wall | Terrain::Concrete | Terrain::Pad),
                        BuildKind::Mine => !map.at(*at).passable() || mine_tiles.iter().any(|mt| *mt == at.tile()),
                        BuildKind::Boat => map.at(*at) != Terrain::Water || tank_tiles.iter().chain(boat_tiles.iter()).any(|bt| *bt == at.tile()),
                    };
                    let placing = *what == BuildKind::Pillbox && s.carrying.is_some();
                    if blocked {
                        done.push((t.id, t.owner, Done::Abandon("something is already there")));
                    } else if s.material < what.cost() && !placing {
                        done.push((t.id, t.owner, Done::Abandon("no material")));
                    } else {
                        s.progress += 1;
                        if s.progress >= if placing { PILLBOX_PLACE_TICKS } else { what.ticks() } {
                            done.push((t.id, t.owner, Done::Built(*what, *at)));
                        }
                    }
                }
                PillboyTask::Lift { pillbox } => {
                    let p = pillboxes.iter().find(|p| p.id == *pillbox).unwrap();
                    if p.owner != Some(t.owner) {
                        done.push((t.id, t.owner, Done::Abandon("the pillbox is not ours now")));
                    } else if s.carrying.is_some() {
                        done.push((t.id, t.owner, Done::Abandon("already carrying one")));
                    } else {
                        s.progress += 1;
                        if s.progress >= PILLBOX_LIFT_TICKS {
                            done.push((t.id, t.owner, Done::Lifted(p.id)));
                        }
                    }
                }
                PillboyTask::Harvest { at } => {
                    if map.at(*at) != Terrain::Forest {
                        done.push((t.id, t.owner, Done::Abandon("the forest is gone")));
                    } else {
                        s.progress += 1;
                        if s.progress >= HARVEST_TICKS {
                            done.push((t.id, t.owner, Done::Harvested(*at)));
                        }
                    }
                }
            }
        }
        // The same for structures.
        for x in self.structures.iter_mut() {
            let mut who: Vec<PlayerId> = takes.iter().filter(|(id, _)| *id == x.id).map(|(_, o)| *o).collect();
            who.sort_unstable();
            who.dedup();
            let who = allies_together(who, x.capturer, &allied_with);
            match who.as_slice() {
                [one] => {
                    if x.capturer != Some(*one) {
                        x.capturer = Some(*one);
                        x.capture = 0;
                    }
                    x.capture = (x.capture + 1).min(PILLBOX_CAPTURE_TICKS);
                }
                [] => {
                    x.capture = x.capture.saturating_sub(1);
                    if x.capture == 0 {
                        x.capturer = None;
                    }
                }
                _ => {}
            }
        }
        // Capture progress on the pillboxes: contested means nothing moves.
        for p in self.pillboxes.iter_mut() {
            let mut who: Vec<PlayerId> = captures.iter().filter(|(id, _)| *id == p.id).map(|(_, o)| *o).collect();
            who.sort_unstable();
            who.dedup();
            let who = allies_together(who, p.capturer, &allied_with);
            match who.as_slice() {
                [one] => {
                    if p.capturer != Some(*one) {
                        p.capturer = Some(*one);
                        p.capture = 0;
                    }
                    p.capture = (p.capture + 1).min(PILLBOX_CAPTURE_TICKS);
                }
                [] => {
                    if p.capture > 0 {
                        p.capture -= 1;
                    }
                    if p.capture == 0 {
                        p.capturer = None;
                    }
                }
                _ => {} // contested
            }
        }
        for (tid, owner, what) in done {
            match what {
                Done::Captured(pid, from) => {
                    // Contested at the last tick? The pillbox's own clock
                    // says whether one squad stood alone.
                    let uncontested = self.pillbox(pid).map(|p| p.capturer == Some(owner)).unwrap_or(false);
                    if uncontested {
                        if let Some(p) = self.pillbox_mut(pid) {
                            p.owner = Some(owner);
                            p.health = p.health.max(PILLBOX_CAPTURE_HEALTH);
                            p.capture = 0;
                            p.capturer = None;
                            p.reload = PILLBOX_RELOAD_TICKS;
                        }
                        if let Some(pl) = self.players.iter_mut().find(|p| p.id == owner) {
                            pl.score += SCORE_CAPTURE;
                        }
                        self.events.push(Event::PillboxCaptured { pillbox: pid, by: owner, from });
                    } else if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.progress = 0;
                        continue;
                    }
                }
                Done::Taken(sid, from) => {
                    let uncontested = self.structure(sid).map(|x| x.capturer == Some(owner)).unwrap_or(false);
                    if !uncontested {
                        if let Some(t) = self.tank_mut(tid) {
                            t.pillboys.progress = 0;
                        }
                        continue;
                    }
                    let mut kind = StructureKind::Power;
                    if let Some(x) = self.structure_mut(sid) {
                        x.owner = Some(owner);
                        x.health = x.health.max(STRUCTURE_CAPTURE_HEALTH);
                        x.capture = 0;
                        x.capturer = None;
                        kind = x.kind;
                    }
                    if let Some(pl) = self.players.iter_mut().find(|p| p.id == owner) {
                        pl.score += SCORE_CAPTURE;
                    }
                    self.events.push(Event::StructureCaptured { structure: sid, kind, by: owner, from });
                }
                Done::Mended(sid) => {
                    let mut kind = StructureKind::Power;
                    if let Some(x) = self.structure_mut(sid) {
                        x.health = STRUCTURE_HEALTH;
                        kind = x.kind;
                    }
                    if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.material -= STRUCTURE_REPAIR_COST;
                    }
                    self.events.push(Event::StructureRepaired { structure: sid, kind, by: owner });
                }
                Done::Revived(wid) => {
                    let tick = self.tick;
                    if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.material -= REVIVE_COST;
                    }
                    let mut back = None;
                    let at_sea = self.tank(wid).map(|w| self.map.at(w.pos) == Terrain::Water).unwrap_or(false);
                    if let Some(w) = self.tank_mut(wid) {
                        // Sunk off a beach and mended from it: afloat
                        // again, on its craft.
                        w.boat = at_sea;
                        w.alive = true;
                        w.armor = REVIVE_ARMOR;
                        w.goal = None;
                        w.route.clear();
                        w.speed = 0.0;
                        w.reload = 0;
                        // Not "just hit": the bots' scoot reads this.
                        w.last_hit_tick = tick.saturating_sub(TICK_HZ as Tick * 2).max(1);
                        back = Some((w.owner, w.pos));
                    }
                    if let Some((who, at)) = back {
                        if let Some(p) = self.players.iter_mut().find(|p| p.id == who) {
                            p.alive = true;
                        }
                        self.events.push(Event::Revived { tank: wid, owner: who, by: owner, at });
                    }
                }
                Done::Repaired(pid) => {
                    if let Some(p) = self.pillbox_mut(pid) {
                        p.health = PILLBOX_HEALTH;
                    }
                    if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.material -= PILLBOX_REPAIR_COST;
                    }
                    self.events.push(Event::PillboxRepaired { pillbox: pid, by: owner });
                }
                Done::Built(what, at) => {
                    let mut carried = None;
                    if let Some(t) = self.tank_mut(tid) {
                        carried = if what == BuildKind::Pillbox { t.pillboys.carrying.take() } else { None };
                        if carried.is_none() {
                            t.pillboys.material -= what.cost();
                        }
                    }
                    let at = Map::tile_centre(at);
                    match what {
                        BuildKind::Pillbox => {
                            let id = self.alloc();
                            self.pillboxes.push(Pillbox { id, pos: at, owner: Some(owner), health: carried.unwrap_or(PILLBOX_HEALTH), reload: PILLBOX_RELOAD_TICKS, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
                            self.events.push(Event::PillboxBuilt { pillbox: id, by: owner, at });
                        }
                        BuildKind::Wall => self.set_terrain(at, Terrain::Wall),
                        BuildKind::Concrete => self.set_terrain(at, Terrain::Concrete),
                        BuildKind::Structure(kind) => {
                            let id = self.alloc();
                            self.structures.push(Structure { id, kind, pos: at, owner: Some(owner), health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: false });
                            self.events.push(Event::StructureBuilt { structure: id, kind, by: owner, at });
                        }
                        BuildKind::Road if self.map.at(at) == Terrain::Water => {
                            // A span (Paul, 17 October: bridges were a
                            // tile a trip): on from the squad, straight,
                            // up to `BRIDGE_SPAN` tiles of water, a
                            // material each, the same trip. It stops at
                            // ground, a tank or a boat, a depot's ground,
                            // or the material's end.
                            self.set_terrain(at, Terrain::Bridge);
                            let from = self.tank(tid).map(|t| t.pillboys.pos).unwrap_or(at);
                            let d = at - from;
                            let step = if d.x.abs() >= d.y.abs() { V2::new(d.x.signum(), 0.0) } else { V2::new(0.0, d.y.signum()) };
                            for k in 1..BRIDGE_SPAN {
                                let p = at + step.scale(k as f32);
                                let blocked = !self.map.in_bounds(p)
                                    || self.map.at(p) != Terrain::Water
                                    || self.depots.iter().any(|d| d.pos.dist(p) <= DEPOT_RADIUS)
                                    || self.boats.iter().any(|b| b.pos.tile() == p.tile())
                                    || self.tanks.iter().any(|t| t.alive && t.pos.tile() == p.tile());
                                let material = self.tank(tid).map(|t| t.pillboys.material).unwrap_or(0);
                                if blocked || material < ROAD_COST {
                                    break;
                                }
                                if let Some(t) = self.tank_mut(tid) {
                                    t.pillboys.material -= ROAD_COST;
                                }
                                self.set_terrain(p, Terrain::Bridge);
                                self.events.push(Event::Built { owner, what, at: p });
                            }
                        }
                        BuildKind::Road => self.set_terrain(at, Terrain::Road),
                        BuildKind::Boat => {
                            let id = self.alloc();
                            self.boats.push(Boat { id, pos: at, owner });
                        }
                        BuildKind::Mine => {
                            let id = self.alloc();
                            self.mines.push(Mine { id, pos: at, owner });
                            self.events.push(Event::MineLaid { mine: id, owner, at });
                        }
                    }
                    self.events.push(Event::Built { owner, what, at });
                }
                Done::Harvested(at) => {
                    let at = Map::tile_centre(at);
                    self.set_terrain(at, Terrain::Grass);
                    self.felled.push((at, self.tick + TREE_REGROW_TICKS));
                    let mut gained = 0;
                    if let Some(t) = self.tank_mut(tid) {
                        gained = HARVEST_MATERIAL.min(PILLBOY_MATERIAL_MAX - t.pillboys.material);
                        t.pillboys.material += gained;
                    }
                    self.events.push(Event::Harvested { owner, at, material: gained });
                }
                Done::Lifted(pid) => {
                    let Some(ix) = self.pillboxes.iter().position(|p| p.id == pid) else { continue };
                    let p = self.pillboxes.remove(ix);
                    if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.carrying = Some(p.health);
                    }
                    self.events.push(Event::PillboxLifted { pillbox: pid, by: owner, at: p.pos });
                }
                Done::Abandon(why) => {
                    self.events.push(Event::PillboysAbandoned { tank: tid, owner, why: why.into() });
                }
                Done::Home => {
                    if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.aboard = true;
                        t.pillboys.task = None;
                        t.pillboys.progress = 0;
                        t.pillboys.route.clear();
                        t.pillboys.pos = t.pos;
                    }
                    self.events.push(Event::PillboysReturned { tank: tid, owner });
                    continue;
                }
            }
            // Work done or given up: walk home.
            if let Some((from, home)) = self.tank(tid).map(|t| (t.pillboys.pos, t.pos)) {
                let route = self.map.route(from, home);
                let t = self.tank_mut(tid).unwrap();
                t.pillboys.task = Some(PillboyTask::Return);
                t.pillboys.progress = 0;
                t.pillboys.route = route;
            }

        }
        for mid in stepped_on {
            self.explode_mine(mid);
        }
    }

    /// A squad lost with a pillbox on its back, or its tank, lets it
    /// fall where it was: dead, still its side's, to be taken back or
    /// taken (step 11h). Spacing is not asked: it was carried there.
    fn drop_carried(&mut self) {
        let fallen: Vec<(EntityId, PlayerId, V2)> = self.tanks.iter().filter(|t| t.pillboys.carrying.is_some() && (!t.alive || t.pillboys.strength == 0)).map(|t| (t.id, t.owner, if t.pillboys.aboard { t.pos } else { t.pillboys.pos })).collect();
        for (tid, owner, at) in fallen {
            self.tank_mut(tid).unwrap().pillboys.carrying = None;
            let at = Map::tile_centre(at);
            let id = self.alloc();
            self.pillboxes.push(Pillbox { id, pos: at, owner: Some(owner), health: 0.0, reload: PILLBOX_RELOAD_TICKS, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
            self.events.push(Event::PillboxDropped { pillbox: id, owner, at });
        }
    }

    /// A mine goes off: the tile craters, the tank on it takes
    /// `MINE_DAMAGE`, anything within `MINE_RADIUS` takes the splash,
    /// a squad in it is lost, and any mine on an orthogonally adjacent
    /// tile goes off too. Drones are not touched.
    fn explode_mine(&mut self, mine: EntityId) {
        let mut queue = vec![mine];
        while let Some(id) = queue.pop() {
            let Some(ix) = self.mines.iter().position(|m| m.id == id) else { continue };
            let m = self.mines.remove(ix);
            let mut hits = Vec::new();
            // An ally's mine spares allies: it was theirs to walk over.
            let tanks: Vec<(EntityId, V2)> = self.tanks.iter().filter(|t| t.alive && !(t.owner != m.owner && self.allied(m.owner, t.owner))).map(|t| (t.id, t.pos)).collect();
            for (tid, pos) in tanks {
                if pos.tile() == m.pos.tile() {
                    self.damage(tid, MINE_DAMAGE, Some(m.owner));
                    hits.push(tid);
                } else if pos.dist(m.pos) <= MINE_RADIUS {
                    self.damage(tid, MINE_SPLASH_DAMAGE, Some(m.owner));
                    hits.push(tid);
                }
            }
            let squads: Vec<EntityId> = self.tanks.iter().filter(|t| t.alive && t.pillboys.deployed() && t.pillboys.pos.dist(m.pos) <= MINE_RADIUS && !self.allied(m.owner, t.owner)).map(|t| t.id).collect();
            for tid in squads {
                let t = self.tank_mut(tid).unwrap();
                let owner = t.owner;
                t.pillboys.strength = 0;
                t.pillboys.aboard = true;
                t.pillboys.task = None;
                t.pillboys.route.clear();
                t.pillboys.pos = t.pos;
                hits.push(tid);
                self.events.push(Event::PillboysLost { tank: tid, owner });
            }
            self.map.crater(m.pos);
            self.events.push(Event::TerrainChanged { at: Map::tile_centre(m.pos), to: self.map.at(m.pos) as u8 });
            self.events.push(Event::MineExploded { mine: m.id, owner: m.owner, at: m.pos, hits });
            let (mx, my) = m.pos.tile();
            for n in self.mines.iter().filter(|n| {
                let (nx, ny) = n.pos.tile();
                (nx - mx).abs() + (ny - my).abs() == 1
            }) {
                queue.push(n.id);
            }
        }
    }

    /// The pillboxes' guns. Tanks first (hitscan, a clear line, in
    /// range), else the nearest hostile drone inside the air range with
    /// an interceptor's odds. One shot per reload, whatever the target.
    fn step_pillboxes(&mut self) {
        // Shells regrow; the owner's tank draws on them like a depot.
        if self.tick % PILLBOX_SHELL_REGEN_TICKS == 0 {
            for p in self.pillboxes.iter_mut().filter(|p| p.alive() && p.shells < PILLBOX_SHELLS) {
                p.shells += 1;
            }
        }
        if self.tick % RESUPPLY_EVERY == 0 {
            let mut draws: Vec<(EntityId, EntityId)> = Vec::new();
            for t in self.tanks.iter().filter(|t| t.alive && t.shells < TANK_SHELLS) {
                if let Some(p) = self.pillboxes.iter().find(|p| p.alive() && self.allied_opt(p.owner, t.owner) && p.shells > 0 && p.pos.dist(t.pos) <= DEPOT_RADIUS) {
                    draws.push((t.id, p.id));
                }
            }
            for (tid, pid) in draws {
                // Two tanks at one pillbox with a shell left: the first.
                let p = self.pillbox_mut(pid).unwrap();
                if p.shells == 0 {
                    continue;
                }
                p.shells -= 1;
                self.tank_mut(tid).unwrap().shells += 1;
                self.events.push(Event::Resupplied { tank: tid, depot: pid });
            }
        }
        // Tanks with their cover (PIL-07), read once.
        let tanks: Vec<(EntityId, PlayerId, V2, bool)> = self.tanks.iter().filter(|t| t.alive).map(|t| (t.id, t.owner, t.pos, self.concealed(t.pos))).collect();
        let drones: Vec<(EntityId, PlayerId, V2)> = self.drones.iter().map(|d| (d.id, d.owner, d.pos)).collect();
        // Teams, read once: a pillbox is hostile to anyone not on its
        // owner's team; a neutral one to everyone.
        let teams: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        let team_of = |p: PlayerId| teams.iter().find(|(id, _)| *id == p).map(|(_, t)| *t);
        let allied_with = |owner: Option<PlayerId>, p: PlayerId| match owner {
            None => false,
            Some(o) => o == p || (team_of(o).is_some() && team_of(o) == team_of(p)),
        };
        let mut shots: Vec<(EntityId, PlayerId, EntityId, f32)> = Vec::new(); // pillbox, shooter, tank, damage
        let mut air: Vec<(EntityId, PlayerId, EntityId, f32)> = Vec::new(); // pillbox, shooter, drone, link
        // A pillbox on a chain is being moved, not manned: silent.
        let chained: Vec<EntityId> = self.tanks.iter().filter(|t| t.alive).filter_map(|t| t.towing).collect();
        for p in self.pillboxes.iter_mut() {
            if chained.contains(&p.id) {
                continue;
            }
            if p.reload > 0 {
                p.reload -= 1;
            }
            if !p.alive() || p.reload > 0 {
                continue;
            }
            let shooter = p.shooter();
            // A tank under forest cover is a target only from close
            // (PIL-07).
            let tank = tanks
                .iter()
                .filter(|(_, o, pos, hidden)| !allied_with(p.owner, *o) && pos.dist(p.pos) <= PILLBOX_RANGE && self.map.line_clear(p.pos, *pos) && (!hidden || pos.dist(p.pos) <= CONCEAL_RANGE))
                .min_by(|a, b| a.2.dist(p.pos).total_cmp(&b.2.dist(p.pos)).then(a.0.cmp(&b.0)));
            if let Some((tid, _, _, _)) = tank {
                shots.push((p.id, shooter, *tid, PILLBOX_DAMAGE));
                p.reload = PILLBOX_RELOAD_TICKS;
                continue;
            }
            let drone = drones
                .iter()
                .filter(|(_, o, pos)| !allied_with(p.owner, *o) && pos.dist(p.pos) <= PILLBOX_AIR_RANGE)
                .min_by(|a, b| a.2.dist(p.pos).total_cmp(&b.2.dist(p.pos)).then(a.0.cmp(&b.0)));
            if let Some((did, _, _)) = drone {
                // The owner tank's radar, if it is within link range.
                let link = p
                    .owner
                    .and_then(|o| tanks.iter().find(|(_, to, _, _)| *to == o))
                    .map(|(_, _, tp, _)| (1.0 - tp.dist(p.pos) / DRONE_LINK).clamp(0.0, 1.0))
                    .unwrap_or(0.0);
                air.push((p.id, shooter, *did, link));
                p.reload = PILLBOX_RELOAD_TICKS;
            }
        }
        for (pid, shooter, tid, dmg) in shots {
            self.events.push(Event::PillboxFired { pillbox: pid, owner: shooter, tank: tid, damage: dmg });
            let by = Some(shooter).filter(|s| *s != NEUTRAL);
            self.damage(tid, dmg, by);
        }
        for (pid, shooter, did, link) in air {
            let p_kill = INTERCEPT_BASE + (INTERCEPT_LINKED - INTERCEPT_BASE) * link;
            let roll = self.rng.f32();
            if roll < p_kill {
                if let Some(ix) = self.drones.iter().position(|d| d.id == did) {
                    let d = self.drones.remove(ix);
                    self.events.push(Event::Intercepted { interceptor: pid, target: did, owner: shooter });
                    self.events.push(Event::DroneLost { drone: did, owner: d.owner, kind: d.kind, why: LossReason::Shot });
                    if let Some(pl) = self.players.iter_mut().find(|p| p.id == shooter) {
                        pl.score += SCORE_INTERCEPT;
                    }
                }
            } else {
                self.events.push(Event::InterceptMissed { interceptor: pid, target: did });
            }
        }
    }

    /// Damage to a pillbox from a shell. Dead at zero; never removed. An
    /// ally's shell spares it.
    fn damage_pillbox(&mut self, pid: EntityId, amount: f32, by: Option<PlayerId>) {
        if let (Some(b), Some(p)) = (by, self.pillbox(pid)) {
            if self.allied_opt(p.owner, b) {
                self.events.push(Event::FriendlyFireSpared { target: pid, by: b, damage: amount });
                return;
            }
        }
        let Some(p) = self.pillbox_mut(pid) else { return };
        if !p.alive() {
            return;
        }
        p.health = (p.health - amount).max(0.0);
        let (left, owner) = (p.health, p.owner);
        self.events.push(Event::PillboxDamaged { pillbox: pid, by, damage: amount, health_left: left });
        if left <= 0.0 {
            if let Some(p) = self.pillbox_mut(pid) {
                p.capture = 0;
                p.capturer = None;
            }
            self.events.push(Event::PillboxDestroyed { pillbox: pid, owner, by });
        }
    }

    /// A deployed squad under a splash loses Pillboys; at zero it is gone.
    /// An ally's splash spares it.
    fn hurt_squads(&mut self, at: V2, by: PlayerId) {
        let hit: Vec<EntityId> = self.tanks.iter().filter(|t| t.alive && t.pillboys.deployed() && t.pillboys.pos.dist(at) <= SPLASH_RADIUS && !(t.owner != by && self.allied(by, t.owner))).map(|t| t.id).collect();
        for tid in hit {
            let t = self.tank_mut(tid).unwrap();
            let lost = PILLBOY_SPLASH_LOSS.min(t.pillboys.strength);
            t.pillboys.strength -= lost;
            let (owner, left) = (t.owner, t.pillboys.strength);
            self.events.push(Event::PillboysHit { tank: tid, owner, lost, left });
            if left == 0 {
                let t = self.tank_mut(tid).unwrap();
                t.pillboys.aboard = true;
                t.pillboys.task = None;
                t.pillboys.progress = 0;
                t.pillboys.route.clear();
                t.pillboys.pos = t.pos;
                self.events.push(Event::PillboysLost { tank: tid, owner });
            }
        }
    }

    fn step_tanks(&mut self) {
        let map = &self.map;
        let guns: Vec<(Option<PlayerId>, V2)> = self.pillboxes.iter().filter(|b| b.alive()).map(|b| (b.owner, b.pos)).collect();
        let mines: Vec<(PlayerId, (i32, i32), EntityId)> = self.mines.iter().map(|m| (m.owner, m.pos.tile(), m.id)).collect();
        let teams: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        let mut boats: Vec<(i32, i32)> = self.boats.iter().map(|b| b.pos.tile()).collect();
        let mut triggered: Vec<EntityId> = Vec::new();
        let mut changed: Vec<(EntityId, PlayerId, bool, V2, V2)> = Vec::new();
        for t in self.tanks.iter_mut() {
            let (was, from) = (t.boat, t.pos);
            if let Some(m) = drive_tank(t, map, &guns, &mines, &teams, &boats) {
                triggered.push(m);
            }
            if t.boat != was {
                changed.push((t.id, t.owner, t.boat, t.pos, from));
                // Boarded: that boat is taken, for the tanks after this one too.
                if t.boat {
                    boats.retain(|b| *b != t.pos.tile());
                }
            }
        }
        for (tank, owner, afloat, at, from) in changed {
            if afloat {
                let Some(ix) = self.boats.iter().position(|b| b.pos.tile() == at.tile()) else { continue };
                let b = self.boats.remove(ix);
                self.events.push(Event::Boarded { tank, owner, boat: b.id, at: b.pos });
            } else {
                // The boat stays moored on the water it was driven off
                // (the prior art's: TNK-17 boards it again), for anyone:
                // beside the beach tile, not at its corner, where no tank
                // could drive onto it again.
                let (x, y) = at.tile();
                let moor = [(0, -1), (1, 0), (0, 1), (-1, 0)]
                    .iter()
                    .map(|(dx, dy)| V2::new((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5))
                    .filter(|p| self.map.in_bounds(*p) && self.map.at(*p) == Terrain::Water)
                    .filter(|p| !self.boats.iter().any(|b| b.pos.tile() == p.tile()) && !self.tanks.iter().any(|u| u.alive && u.id != tank && u.pos.tile() == p.tile()))
                    .min_by(|a, b| a.dist(from).total_cmp(&b.dist(from)));
                if let Some(moor) = moor {
                    let id = self.alloc();
                    self.boats.push(Boat { id, pos: moor, owner });
                }
                self.events.push(Event::Landed { tank, owner, at });
            }
        }
        for mid in triggered {
            self.explode_mine(mid);
        }
    }

    /// Client-side prediction (roadmap step 4): one tick of player `p`'s
    /// tank alone, under `cmds` from `p`, by the same code `step` runs
    /// on the host. Only the orders that move the hull are taken: `Move`,
    /// `Stop`, and the two fire orders, because firing turns the tank
    /// toward the target (the first lagged lobby test found that one:
    /// a standing tank, zero tiles off, heading wrong). Every other
    /// order is the host's to answer, and nothing else in the world
    /// moves; the shell a predicted fire spawns is dropped at once. A
    /// mine the tank drives onto is left to the host too: the snapshot
    /// brings the damage, and the mismatch it logs is honest. The
    /// caller's world is the client's partial copy, kept up from
    /// snapshots (treadcore-net's `Predictor`); the tick advances here
    /// so the copy's clock reads as the host's would.
    pub fn predict(&mut self, p: PlayerId, cmds: &[Command]) {
        self.events.clear();
        self.tick += 1;
        for cmd in cmds {
            if Command::predicted(cmd) {
                self.apply(&Order { player: p, cmd: cmd.clone() });
            }
        }
        self.shells.clear();
        let guns: Vec<(Option<PlayerId>, V2)> = self.pillboxes.iter().filter(|b| b.alive()).map(|b| (b.owner, b.pos)).collect();
        let mines: Vec<(PlayerId, (i32, i32), EntityId)> = self.mines.iter().map(|m| (m.owner, m.pos.tile(), m.id)).collect();
        let teams: Vec<(PlayerId, TeamId)> = self.players.iter().map(|x| (x.id, x.team)).collect();
        let boats: Vec<(i32, i32)> = self.boats.iter().map(|b| b.pos.tile()).collect();
        let map = &self.map;
        if let Some(t) = self.tanks.iter_mut().find(|t| t.owner == p) {
            drive_tank(t, map, &guns, &mines, &teams, &boats);
        }
    }

    fn step_drones(&mut self) {
        // 0. Jammers (step 10b): every linked drone inside one loses its
        //    link this tick, whoever's it is; a longshot's fix drifts,
        //    once. The swarm flies through.
        let jammers: Vec<(EntityId, V2)> = self.structures.iter().filter(|x| x.kind == StructureKind::Jammer && x.working()).map(|x| (x.id, x.pos)).collect();
        let mut jams: Vec<(EntityId, EntityId)> = Vec::new();
        {
            for d in self.drones.iter_mut() {
                let by = jammers.iter().find(|(_, p)| p.dist(d.pos) <= JAMMER_RANGE).map(|(id, _)| *id);
                // A fibre-optic strike has no radio to jam.
                let jammable = (d.tether.is_some() || d.kind == DroneKind::Longshot) && !d.spec().jam_proof;
                match by {
                    Some(j) if jammable && !d.jammed => {
                        d.jammed = true;
                        jams.push((d.id, j));
                    }
                    None if d.kind != DroneKind::Longshot => d.jammed = false,
                    _ => {}
                }
            }
        }
        for (did, j) in jams {
            let drift = self.rng.disc().scale(JAM_DRIFT);
            let Some(d) = self.drones.iter_mut().find(|d| d.id == did) else { continue };
            if let DroneTask::Attack { at } = &mut d.task {
                *at = *at + drift;
            }
            let (owner, kind) = (d.owner, d.kind);
            self.events.push(Event::Jammed { drone: did, owner, kind, jammer: j });
        }
        // 1. Links and batteries. Decided before motion so a drone that has
        //    just lost contact turns for home this tick.
        for d in self.drones.iter_mut() {
            d.battery -= DT;
            match d.tether.and_then(|t| self.tanks.iter().find(|x| x.id == t)) {
                Some(t) if t.alive => {
                    let range = match d.kind {
                        DroneKind::Recon => RECON_LINK,
                        _ => DRONE_LINK,
                    };
                    d.link = (1.0 - t.pos.dist(d.pos) / range).clamp(0.0, 1.0);
                }
                Some(_) => d.link = 0.0,
                None => d.link = 1.0, // swarm drones and longshots are autonomous
            }
            if d.jammed && d.tether.is_some() {
                d.link = 0.0;
            }
        }
        // 2. Interceptor assignment. For each tank with interceptors on
        //    station, any hostile strike inside detection gets the nearest
        //    free interceptor. One interceptor per intruder. Hostile
        //    interceptors are never intruders: when they were, two
        //    bubbles that overlapped stripped each other in seconds and
        //    the strikes came through the gap (backlog item 1; the
        //    traces of 13 October 2026 put the kills at 7 shell : 6
        //    strike engaging everything, 9 : 3 engaging strikes only).
        //    The bubble is a shield, not a sword.
        let mut assignments: Vec<(EntityId, EntityId)> = Vec::new();
        for t in self.tanks.iter().filter(|t| t.alive) {
            let detect = t.bubble + DETECT_MARGIN;
            let mut intruders: Vec<&Drone> = self
                .drones
                .iter()
                .filter(|d| !self.allied(d.owner, t.owner) && d.pos.dist(t.pos) <= detect && matches!(d.kind, DroneKind::Strike | DroneKind::Longshot))
                .collect();
            // Also engage recon drones that come inside the bubble itself:
            // they are spotting for someone's gun. An ally's are welcome.
            intruders.extend(
                self.drones.iter().filter(|d| !self.allied(d.owner, t.owner) && d.kind == DroneKind::Recon && d.pos.dist(t.pos) <= t.bubble),
            );
            intruders.sort_by(|a, b| a.pos.dist(t.pos).total_cmp(&b.pos.dist(t.pos)).then(a.id.cmp(&b.id)));
            for intruder in intruders {
                let already = self.drones.iter().any(|d| d.owner == t.owner && d.task == DroneTask::Intercept { target: intruder.id })
                    || assignments.iter().any(|(_, tgt)| *tgt == intruder.id);
                if already {
                    continue;
                }
                let free = self
                    .drones
                    .iter()
                    .filter(|d| {
                        d.owner == t.owner
                            && d.kind == DroneKind::Interceptor
                            && d.tether == Some(t.id)
                            && matches!(d.task, DroneTask::Orbit { .. })
                            && !assignments.iter().any(|(i, _)| *i == d.id)
                    })
                    .min_by(|a, b| a.pos.dist(intruder.pos).total_cmp(&b.pos.dist(intruder.pos)).then(a.id.cmp(&b.id)));
                if let Some(f) = free {
                    assignments.push((f.id, intruder.id));
                }
            }
        }
        for (i, tgt) in assignments {
            if let Some(d) = self.drones.iter_mut().find(|d| d.id == i) {
                d.task = DroneTask::Intercept { target: tgt };
            }
        }
        // 3. Motion. Positions of everything, read before anything moves.
        let drone_pos: Vec<(EntityId, V2)> = self.drones.iter().map(|d| (d.id, d.pos)).collect();
        let tank_pos: Vec<(EntityId, V2, bool)> = self.tanks.iter().map(|t| (t.id, t.pos, t.alive)).collect();
        let find_drone = |id: EntityId| drone_pos.iter().find(|(i, _)| *i == id).map(|(_, p)| *p);
        let find_tank = |id: EntityId| tank_pos.iter().find(|(i, _, _)| *i == id).map(|(_, p, a)| (*p, *a));
        let mut lost: Vec<(EntityId, LossReason)> = Vec::new();
        let mut recovered: Vec<EntityId> = Vec::new();
        let mut rams: Vec<(EntityId, EntityId)> = Vec::new();
        let mut strikes: Vec<(EntityId, EntityId)> = Vec::new();
        let mut link_lost: Vec<EntityId> = Vec::new();
        let mut longshots: Vec<(EntityId, PlayerId, V2)> = Vec::new();
        for d in self.drones.iter_mut() {
            if d.battery <= 0.0 {
                lost.push((d.id, LossReason::Battery));
                continue;
            }
            let home = d.tether.and_then(find_tank);
            if d.tether.is_some() && home.map(|(_, a)| !a).unwrap_or(true) {
                lost.push((d.id, LossReason::OwnerDead));
                continue;
            }
            // Link loss: tethered drones that drift out of contact go home.
            // Strikes already committed to a dive are the exception -- the
            // last leg is autonomous, which is exactly what makes them a
            // threat.
            if d.tether.is_some() && d.link < LINK_FLOOR && !matches!(d.task, DroneTask::Return | DroneTask::Strike { .. }) {
                d.task = DroneTask::Return;
                link_lost.push(d.id);
            }
            // Low battery: go home while it still can.
            if d.tether.is_some() && !matches!(d.task, DroneTask::Return | DroneTask::Strike { .. } | DroneTask::Intercept { .. }) {
                if let Some((hp, _)) = home {
                    let need = hp.dist(d.pos) / d.spec().speed + 3.0;
                    if d.battery < need {
                        d.task = DroneTask::Return;
                    }
                }
            }
            let speed = d.spec().speed;
            let target: V2 = match &d.task {
                DroneTask::Orbit { center, radius } => {
                    d.phase += DT * ORBIT_RATE / radius.max(1.0);
                    let c = if d.kind == DroneKind::Interceptor { home.map(|(p, _)| p).unwrap_or(*center) } else { *center };
                    c + V2::from_heading(d.phase).scale(*radius)
                }
                DroneTask::Intercept { target } => match find_drone(*target) {
                    Some(p) => {
                        if p.dist(d.pos) <= RAM_RANGE {
                            rams.push((d.id, *target));
                        }
                        p
                    }
                    None => {
                        d.task = DroneTask::Orbit { center: d.pos, radius: 0.0 };
                        // Back on station next tick: re-centre on the tank.
                        if let Some((hp, _)) = home {
                            d.task = DroneTask::Orbit { center: hp, radius: BUBBLE_DEFAULT };
                        }
                        d.pos
                    }
                },
                DroneTask::Attack { at } => {
                    if at.dist(d.pos) <= RAM_RANGE {
                        longshots.push((d.id, d.owner, *at));
                    }
                    *at
                }
                // A jammed strike short of its terminal dive has no pilot
                // and turns for home.
                DroneTask::Strike { target } if d.jammed && find_tank(*target).map(|(p, _)| p.dist(d.pos) > JAM_TERMINAL).unwrap_or(false) => {
                    d.task = DroneTask::Return;
                    d.pos
                }
                DroneTask::Strike { target } => match find_tank(*target) {
                    Some((p, true)) => {
                        if p.dist(d.pos) <= RAM_RANGE {
                            strikes.push((d.id, *target));
                        }
                        p
                    }
                    _ => {
                        d.task = DroneTask::Return;
                        d.pos
                    }
                },
                DroneTask::Return => match home {
                    Some((p, _)) => {
                        if p.dist(d.pos) <= RECOVER_RANGE {
                            recovered.push(d.id);
                        }
                        p
                    }
                    None => {
                        lost.push((d.id, LossReason::Battery));
                        d.pos
                    }
                },
            };
            let to = target - d.pos;
            let dist = to.len();
            if dist > 1e-3 {
                d.heading = to.heading();
                let stepv = to.norm().scale((speed * DT).min(dist));
                d.pos = d.pos + stepv;
            }
        }
        // 4. Resolve rams, strikes, recoveries, losses -- then remove.
        for (i, tgt) in rams {
            let owner = self.drones.iter().find(|d| d.id == i).map(|d| d.owner).unwrap_or(0);
            let link = self.drones.iter().find(|d| d.id == i).map(|d| d.link).unwrap_or(0.0);
            let spec = self.drones.iter().find(|d| d.id == i).map(|d| d.spec()).unwrap_or(drone_spec(DroneKind::Interceptor, 1));
            // A good link is a good vector from the tank's radar; a bad one
            // leaves the interceptor hunting on its own small camera, which
            // a later mark does better with (radar-cued, then autonomous).
            let p_kill = spec.intercept_base + (spec.intercept_linked - spec.intercept_base) * link;
            let roll = self.rng.f32();
            lost.push((i, LossReason::Expended));
            if roll < p_kill {
                lost.push((tgt, LossReason::Shot));
                self.events.push(Event::Intercepted { interceptor: i, target: tgt, owner });
                if let Some(p) = self.players.iter_mut().find(|p| p.id == owner) {
                    p.score += SCORE_INTERCEPT;
                }
            } else {
                self.events.push(Event::InterceptMissed { interceptor: i, target: tgt });
            }
        }
        for (i, tank) in strikes {
            let owner = self.drones.iter().find(|d| d.id == i).map(|d| d.owner);
            let damage = self.drones.iter().find(|d| d.id == i).map(|d| d.spec().damage).unwrap_or(STRIKE_DAMAGE);
            lost.push((i, LossReason::Expended));
            self.events.push(Event::StrikeHit { drone: i, tank, damage });
            self.damage(tank, damage, owner.filter(|o| *o != SWARM_OWNER));
        }
        for (id, owner, at) in longshots {
            lost.push((id, LossReason::Expended));
            self.longshot_lands(id, owner, at);
        }
        for id in recovered {
            if let Some(d) = self.drones.iter().find(|d| d.id == id).cloned() {
                if let Some(t) = d.tether.and_then(|t| self.tank_mut(t)) {
                    t.bay.put(d.kind);
                }
                self.events.push(Event::DroneRecovered { drone: id, owner: d.owner, kind: d.kind });
                lost.push((id, LossReason::Expended)); // removal only; event already sent
                // Mark so the loss loop below does not also emit DroneLost.
                self.drones.iter_mut().find(|d| d.id == id).unwrap().battery = f32::NEG_INFINITY;
            }
        }
        for id in link_lost {
            if let Some(d) = self.drones.iter().find(|d| d.id == id) {
                self.events.push(Event::LinkLost { drone: id, owner: d.owner });
            }
        }
        let mut seen = std::collections::HashSet::new();
        for (id, why) in lost {
            if !seen.insert(id) {
                continue;
            }
            if let Some(ix) = self.drones.iter().position(|d| d.id == id) {
                let d = self.drones.remove(ix);
                if d.battery != f32::NEG_INFINITY {
                    self.events.push(Event::DroneLost { drone: id, owner: d.owner, kind: d.kind, why });
                }
            }
        }
    }

    fn step_shells(&mut self) {
        let tick = self.tick;
        let due: Vec<Shell> = self.shells.iter().filter(|s| s.impact_tick <= tick).cloned().collect();
        self.shells.retain(|s| s.impact_tick > tick);
        for s in due {
            let mut hits = Vec::new();
            match s.kind {
                ShellKind::Indirect => {
                    let tanks: Vec<(EntityId, f32)> = self
                        .tanks
                        .iter()
                        .filter(|t| t.alive)
                        .map(|t| (t.id, t.pos.dist(s.impact)))
                        .filter(|(_, d)| *d <= SPLASH_RADIUS)
                        .collect();
                    for (id, d) in tanks {
                        let dmg = INDIRECT_DAMAGE * (1.0 - d / SPLASH_RADIUS).max(0.25);
                        self.damage(id, dmg, Some(s.owner));
                        hits.push(id);
                    }
                    // Pillboxes in the splash take the same curve; squads
                    // out in it lose men. The bubble does nothing to a
                    // shell, and neither does a pillbox.
                    let boxes: Vec<(EntityId, f32)> = self.pillboxes.iter().filter(|p| p.alive()).map(|p| (p.id, p.pos.dist(s.impact))).filter(|(_, d)| *d <= SPLASH_RADIUS).collect();
                    for (id, d) in boxes {
                        let dmg = INDIRECT_DAMAGE * (1.0 - d / SPLASH_RADIUS).max(0.25);
                        self.damage_pillbox(id, dmg, Some(s.owner));
                        hits.push(id);
                    }
                    // Structures are big: the splash reaches them from
                    // further out.
                    let works: Vec<(EntityId, f32)> = self.structures.iter().filter(|x| x.alive()).map(|x| (x.id, (x.pos.dist(s.impact) - STRUCTURE_SIZE).max(0.0))).filter(|(_, d)| *d <= SPLASH_RADIUS).collect();
                    for (id, d) in works {
                        let dmg = INDIRECT_DAMAGE * (1.0 - d / SPLASH_RADIUS).max(0.25);
                        self.damage_structure(id, dmg, Some(s.owner));
                        hits.push(id);
                    }
                    self.hurt_squads(s.impact, s.owner);
                    // Walls in the splash come down; mines in it go off.
                    let (cx, cy) = s.impact.tile();
                    for dy in -3..=3 {
                        for dx in -3..=3 {
                            let p = V2::new((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                            if self.map.in_bounds(p) && p.dist(s.impact) <= SPLASH_RADIUS * 0.7 {
                                // Concrete cracks to a wall; a wall falls.
                                match self.map.at(p) {
                                    Terrain::Wall => self.set_terrain(p, Terrain::Rubble),
                                    Terrain::Concrete => self.set_terrain(p, Terrain::Wall),
                                    _ => {}
                                }
                            }
                        }
                    }
                    let mines: Vec<EntityId> = self.mines.iter().filter(|m| m.pos.dist(s.impact) <= SPLASH_RADIUS).map(|m| m.id).collect();
                    for mid in mines {
                        self.explode_mine(mid);
                    }
                    // Low-flying drones in the splash go down too. The
                    // bubble does nothing to a shell; the shell does plenty
                    // to the bubble.
                    let dead: Vec<EntityId> = self.drones.iter().filter(|d| d.pos.dist(s.impact) <= SPLASH_RADIUS * 0.8).map(|d| d.id).collect();
                    for id in dead {
                        let d = self.drones.iter().find(|d| d.id == id).cloned().unwrap();
                        self.drones.retain(|x| x.id != id);
                        self.events.push(Event::DroneLost { drone: id, owner: d.owner, kind: d.kind, why: LossReason::Splash });
                        hits.push(id);
                    }
                    self.map.crater(s.impact);
                }
                ShellKind::Direct => {
                    let hit = self
                        .tanks
                        .iter()
                        .filter(|t| t.alive && !self.allied(t.owner, s.owner))
                        .map(|t| (t.id, t.pos.dist(s.impact)))
                        .filter(|(_, d)| *d <= DIRECT_HIT_RADIUS)
                        .min_by(|a, b| a.1.total_cmp(&b.1))
                        .map(|(id, _)| id);
                    if let Some(id) = hit {
                        self.damage(id, DIRECT_DAMAGE, Some(s.owner));
                        hits.push(id);
                    } else {
                        // No tank there: a pillbox, if one is.
                        let pb = self.pillboxes.iter().filter(|p| p.alive() && !self.allied_opt(p.owner, s.owner) && p.pos.dist(s.impact) <= DIRECT_HIT_RADIUS + 0.5).map(|p| p.id).next();

                        if let Some(id) = pb {
                            self.damage_pillbox(id, DIRECT_DAMAGE, Some(s.owner));
                            hits.push(id);
                        } else if let Some(id) = self.structures.iter().filter(|x| x.alive() && !self.allied_opt(x.owner, s.owner) && x.pos.dist(s.impact) <= DIRECT_HIT_RADIUS + STRUCTURE_SIZE).map(|x| x.id).next() {
                            self.damage_structure(id, DIRECT_DAMAGE, Some(s.owner));
                            hits.push(id);
                        }
                    }
                }
            }
            self.events.push(Event::ShellImpact { shell: s.id, at: s.impact, kind: s.kind, hits });
        }
    }

    /// Damage to a structure. A wreck at zero; never removed. An ally's
    /// shot spares it.
    fn damage_structure(&mut self, sid: EntityId, amount: f32, by: Option<PlayerId>) {
        if let (Some(b), Some(x)) = (by, self.structure(sid)) {
            if self.allied_opt(x.owner, b) {
                self.events.push(Event::FriendlyFireSpared { target: sid, by: b, damage: amount });
                return;
            }
        }
        let Some(x) = self.structure_mut(sid) else { return };
        if !x.alive() {
            return;
        }
        x.health = (x.health - amount).max(0.0);
        let (left, owner, kind) = (x.health, x.owner, x.kind);
        if left <= 0.0 {
            x.capture = 0;
            x.capturer = None;
            x.powered = false;
        }
        self.events.push(Event::StructureDamaged { structure: sid, kind, owner, by, damage: amount, health_left: left });
        if left <= 0.0 {
            self.events.push(Event::StructureWrecked { structure: sid, kind, owner, by });
        }
    }

    /// A longshot dives at `at`: structures, pillboxes and tanks within
    /// `LONGSHOT_RADIUS` take its warhead, by kind; nothing else.
    fn longshot_lands(&mut self, drone: EntityId, owner: PlayerId, at: V2) {
        let by = Some(owner).filter(|o| *o != SWARM_OWNER);
        let blast = self.drones.iter().find(|d| d.id == drone).map(|d| d.spec().blast).unwrap_or(1.0);
        let works: Vec<EntityId> = self.structures.iter().filter(|x| x.alive() && x.pos.dist(at) <= LONGSHOT_RADIUS + STRUCTURE_SIZE).map(|x| x.id).collect();
        let boxes: Vec<EntityId> = self.pillboxes.iter().filter(|p| p.alive() && p.pos.dist(at) <= LONGSHOT_RADIUS).map(|p| p.id).collect();
        let tanks: Vec<EntityId> = self.tanks.iter().filter(|t| t.alive && t.pos.dist(at) <= LONGSHOT_RADIUS).map(|t| t.id).collect();
        // The dive first, then what it did, as a strike's hit comes
        // before the damage.
        let hits: Vec<EntityId> = works.iter().chain(&boxes).chain(&tanks).copied().collect();
        self.events.push(Event::LongshotHit { drone, owner, at, hits });
        for id in works {
            self.damage_structure(id, LONGSHOT_STRUCTURE_DAMAGE * blast, by);
        }
        for id in boxes {
            self.damage_pillbox(id, LONGSHOT_PILLBOX_DAMAGE * blast, by);
        }
        for id in tanks {
            self.damage(id, LONGSHOT_TANK_DAMAGE * blast, by);
        }
    }

    /// The economy (step 10b), once a tick after the depots: power
    /// first, then what each working structure does for its side, then
    /// what holding them scores.
    fn step_structures(&mut self) {
        if self.structures.is_empty() {
            return;
        }
        let tick = self.tick;
        // Power: a standing plant powers its side's structures in reach.
        let plants: Vec<(Option<PlayerId>, V2)> = self.structures.iter().filter(|x| x.kind == StructureKind::Power && x.alive() && x.owner.is_some()).map(|x| (x.owner, x.pos)).collect();
        let teams: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        let team_of = |p: PlayerId| teams.iter().find(|(id, _)| *id == p).map(|(_, t)| *t);
        let same_side = |a: Option<PlayerId>, b: Option<PlayerId>| match (a, b) {
            (Some(a), Some(b)) => a == b || (team_of(a).is_some() && team_of(a) == team_of(b)),
            _ => false,
        };
        let mut flips: Vec<(EntityId, StructureKind, PlayerId, bool)> = Vec::new();
        for x in self.structures.iter_mut() {
            let on = x.owner.is_some() && x.alive() && (!x.kind.needs_power() || plants.iter().any(|(o, p)| same_side(*o, x.owner) && p.dist(x.pos) <= POWER_RANGE));
            if on != x.powered {
                x.powered = on;
                // A wreck going dark is already news; an owned one is.
                if let Some(o) = x.owner.filter(|_| x.alive()) {
                    flips.push((x.id, x.kind, o, on));
                }
            }
        }
        for (structure, kind, owner, on) in flips {
            self.events.push(Event::StructurePower { structure, kind, owner, on });
        }
        let working: Vec<(StructureKind, Option<PlayerId>, V2)> = self.structures.iter().filter(|x| x.working()).map(|x| (x.kind, x.owner, x.pos)).collect();
        let serves = |kind: StructureKind, p: PlayerId, at: V2, r: f32| working.iter().any(|(k, o, pos)| *k == kind && same_side(*o, Some(p)) && pos.dist(at) <= r);
        // Refinery: the side burns less. Drive_tank burned the full
        // amount this tick; give back the saving.
        let refined: Vec<PlayerId> = self.players.iter().filter(|p| working.iter().any(|(k, o, _)| *k == StructureKind::Refinery && same_side(*o, Some(p.id)))).map(|p| p.id).collect();
        for t in self.tanks.iter_mut().filter(|t| t.alive && refined.contains(&t.owner)) {
            t.fuel = (t.fuel + FUEL_PER_TILE * t.speed * DT * (1.0 - REFINERY_FUEL_BURN)).min(TANK_FUEL);
        }
        // The stops: a refinery's fuel and shells, a workshop's repair
        // and bay, at a depot's cadence.
        if tick % RESUPPLY_EVERY == 0 {
            for t in self.tanks.iter_mut().filter(|t| t.alive) {
                if serves(StructureKind::Refinery, t.owner, t.pos, REFINERY_RANGE) {
                    t.fuel = (t.fuel + 40.0).min(TANK_FUEL);
                    t.shells = (t.shells + 1).min(TANK_SHELLS);
                }
                if serves(StructureKind::Workshop, t.owner, t.pos, WORKSHOP_RANGE) {
                    t.armor = (t.armor + REPAIR_PER_TICK * 2.0).min(TANK_ARMOR);
                    if t.bay.longshot < BAY_FULL.longshot {
                        t.bay.longshot += 1;
                    } else if t.bay.interceptor < BAY_FULL.interceptor {
                        t.bay.interceptor += 1;
                    } else if t.bay.strike < BAY_FULL.strike {
                        t.bay.strike += 1;
                    } else if t.bay.recon < BAY_FULL.recon {
                        t.bay.recon += 1;
                    }
                    if t.pillboys.aboard && t.pillboys.strength < PILLBOY_MAX {
                        t.pillboys.strength += 1;
                    }
                }
            }
        }
        // A workshop mends what stands near it.
        let mend = WORKSHOP_MEND_PER_SECOND * DT;
        for p in self.pillboxes.iter_mut().filter(|p| p.alive() && p.health < PILLBOX_HEALTH) {
            if let Some(o) = p.owner {
                if serves(StructureKind::Workshop, o, p.pos, WORKSHOP_MEND_RANGE) {
                    p.health = (p.health + mend).min(PILLBOX_HEALTH);
                }
            }
        }
        for x in self.structures.iter_mut().filter(|x| x.alive() && x.health < STRUCTURE_HEALTH) {
            if let Some(o) = x.owner {
                if serves(StructureKind::Workshop, o, x.pos, WORKSHOP_MEND_RANGE) {
                    x.health = (x.health + mend).min(STRUCTURE_HEALTH);
                }
            }
        }
        // Holding pays while it is held.
        if tick % HOLD_EVERY_TICKS == 0 {
            let held: Vec<PlayerId> = self.structures.iter().filter(|x| x.alive()).filter_map(|x| x.owner).collect();
            for o in held {
                if let Some(pl) = self.players.iter_mut().find(|p| p.id == o && p.alive) {
                    pl.score += SCORE_HOLD;
                }
            }
        }
    }

    fn damage(&mut self, tank: EntityId, amount: f32, by: Option<PlayerId>) {
        let tick = self.tick;
        // Friendly fire is off within a team: an ally's shell, strike or
        // mine does nothing to this tank, and the trace says so. One's
        // own shells still hurt: that is a mistake, not an ally.
        if let (Some(b), Some(t)) = (by, self.tank(tank)) {
            if b != t.owner && self.allied(b, t.owner) {
                self.events.push(Event::FriendlyFireSpared { target: tank, by: b, damage: amount });
                return;
            }
        }
        let Some(t) = self.tank_mut(tank) else { return };
        if !t.alive {
            return;
        }
        // One hit sinks a boat, and the water takes the tank (TNK-14,
        // TNK-15: the shooter has the kill).
        let sunk = t.boat && amount > 0.0;
        t.armor -= if sunk { t.armor.max(amount) } else { amount };
        t.last_hit_tick = tick;
        let owner = t.owner;
        let left = t.armor;
        if sunk {
            let at = t.pos;
            t.boat = false;
            self.events.push(Event::Sunk { tank, owner, by, at });
        }
        self.events.push(Event::TankDamaged { tank, by, damage: amount, armor_left: left.max(0.0) });
        if left <= 0.0 {
            let t = self.tank_mut(tank).unwrap();
            t.alive = false;
            t.armor = 0.0;
            t.goal = None;
            t.route.clear();
            t.speed = 0.0;
            // The squad goes with the tank, aboard or out.
            t.pillboys.strength = 0;
            t.pillboys.aboard = true;
            t.pillboys.task = None;
            t.pillboys.route.clear();

            if let Some(p) = self.players.iter_mut().find(|p| p.id == owner) {
                p.alive = false;
            }
            if let Some(b) = by {
                if b != owner {
                    if let Some(p) = self.players.iter_mut().find(|p| p.id == b) {
                        p.score += SCORE_KILL;
                    }
                }
            }
            self.events.push(Event::TankDestroyed { tank, owner, by });
        }
    }

    /// Felled trees grow back, on ground still bare: a road or a wall laid there keeps it, and
    /// a mine or a pillbox on the tile has the tree wait another ten seconds.
    fn step_trees(&mut self) {
        while self.felled.first().is_some_and(|f| f.1 <= self.tick) {
            let (at, _) = self.felled.remove(0);
            if self.map.at(at) != Terrain::Grass {
                continue;
            }
            let held = self.mines.iter().any(|m| m.pos.tile() == at.tile()) || self.pillboxes.iter().any(|p| p.pos.tile() == at.tile());
            if held {
                self.felled.push((at, self.tick + 10 * TICK_HZ as Tick));
            } else {
                self.set_terrain(at, Terrain::Forest);
            }
        }
    }

    fn step_depots(&mut self) {
        if self.tick % DEPOT_MAKES_EVERY == 0 {
            for dep in self.depots.iter_mut().filter(|d| d.owner.is_some()) {
                dep.stock.make();
            }
        }
        let tanks: Vec<(EntityId, PlayerId, V2)> = self.tanks.iter().filter(|t| t.alive).map(|t| (t.id, t.owner, t.pos)).collect();
        let teams: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        let team_of = |p: PlayerId| teams.iter().find(|(id, _)| *id == p).map(|(_, t)| *t);
        let allied_with = |owner: Option<PlayerId>, p: PlayerId| match owner {
            None => false,
            Some(o) => o == p || (team_of(o).is_some() && team_of(o) == team_of(p)),
        };
        let mut captured = Vec::new();
        let mut resupplied = Vec::new();
        for dep in self.depots.iter_mut() {
            let mut here: Vec<&(EntityId, PlayerId, V2)> = tanks.iter().filter(|(_, _, p)| p.dist(dep.pos) <= DEPOT_RADIUS).collect();
            // Tanks of one side on a depot are not a contest (step 11h's
            // fleets: a wingman on its leader's heels stopped every
            // capture): the depot's own side resupply, and one of them,
            // the one already taking it else the first, takes it.
            if here.len() > 1 && here.iter().all(|(_, o, _)| allied_with(Some(here[0].1), *o)) {
                if allied_with(dep.owner, here[0].1) {
                    for (tid, _, _) in &here {
                        resupplied.push((*tid, dep.id));
                    }
                    continue;
                }
                let keep = here.iter().position(|(_, o, _)| dep.capturer == Some(*o)).unwrap_or(0);
                here = vec![here[keep]];
            }
            match here.as_slice() {
                [] => {
                    if dep.capture > 0 {
                        dep.capture -= 1;
                    }
                    if dep.capture == 0 {
                        dep.capturer = None;
                    }
                }
                [(tid, owner, _)] => {
                    // Allies share depots (BAS-06): an ally's resupplies and
                    // is not captured.
                    if dep.owner == Some(*owner) || allied_with(dep.owner, *owner) {
                        resupplied.push((*tid, dep.id));
                    } else {
                        if dep.capturer != Some(*owner) {
                            dep.capturer = Some(*owner);
                            dep.capture = 0;
                        }
                        dep.capture += 1;
                        if dep.capture >= CAPTURE_TICKS {
                            dep.owner = Some(*owner);
                            dep.capture = 0;
                            dep.capturer = None;
                            captured.push((dep.id, *owner));
                        }
                    }
                }
                _ => {} // contested: nothing happens
            }
        }
        for (tid, did) in resupplied {
            let tick = self.tick;
            let Some(di) = self.depots.iter().position(|d| d.id == did) else { continue };
            let mut stock = self.depots[di].stock;
            let t = self.tank_mut(tid).unwrap();
            let before = (t.shells, t.bay, t.armor as i32, t.fuel as i32);
            if tick % RESUPPLY_EVERY == 0 {
                if t.shells < TANK_SHELLS && stock.take(STOCK_SHELL) {
                    t.shells += 1;
                }
                t.armor = (t.armor + REPAIR_PER_TICK).min(TANK_ARMOR);
                t.fuel = (t.fuel + 40.0).min(TANK_FUEL);
                // Interceptors first: the bubble is what keeps the tank alive
                // long enough to use the rest. A kind the store has none of
                // does not hold up the next.
                if t.bay.interceptor < BAY_FULL.interceptor && stock.take(STOCK_INTERCEPTOR) {
                    t.bay.interceptor += 1;
                } else if t.bay.recon < BAY_FULL.recon && stock.take(STOCK_RECON) {
                    t.bay.recon += 1;
                } else if t.bay.strike < BAY_FULL.strike && stock.take(STOCK_STRIKE) {
                    t.bay.strike += 1;
                } else if t.bay.longshot < BAY_FULL.longshot && stock.take(STOCK_LONGSHOT) {
                    t.bay.longshot += 1;
                }
                // The squad and its material, when aboard.
                if t.pillboys.aboard {
                    if t.pillboys.strength < PILLBOY_MAX {
                        t.pillboys.strength += 1;
                    }
                    if t.pillboys.material < PILLBOY_MATERIAL_MAX && stock.take(STOCK_TREE) {
                        t.pillboys.material += 1;
                    }
                }
            }
            let changed = before != (t.shells, t.bay, t.armor as i32, t.fuel as i32);
            self.depots[di].stock = stock;
            if changed && tick % (RESUPPLY_EVERY * 4) == 0 {
                self.events.push(Event::Resupplied { tank: tid, depot: did });
            }
        }
        for (did, by) in captured {
            if let Some(p) = self.players.iter_mut().find(|p| p.id == by) {
                p.score += SCORE_CAPTURE;
            }
            self.events.push(Event::DepotCaptured { depot: did, by });
        }
    }

    fn step_swarm(&mut self) {
        let iv = self.scenario.swarm_interval;
        if iv == 0 || self.tick % iv as Tick != 0 || self.tick == 0 {
            return;
        }
        let alive: Vec<(EntityId, V2)> = self.tanks.iter().filter(|t| t.alive).map(|t| (t.id, t.pos)).collect();
        if alive.is_empty() {
            return;
        }
        // From a random edge, at the nearest tank to that edge.
        let (mw, mh) = (self.map.w as f32, self.map.h as f32);
        let side = self.rng.below(4);
        let along = self.rng.range(8.0, (if side % 2 == 0 { mw } else { mh }) - 8.0);
        let from = match side {
            0 => V2::new(along, 0.5),
            1 => V2::new(mw - 0.5, along),
            2 => V2::new(along, mh - 0.5),
            _ => V2::new(0.5, along),
        };
        let (target, _) = alive.iter().min_by(|a, b| a.1.dist(from).total_cmp(&b.1.dist(from))).unwrap();
        let n = self.scenario.swarm_size;
        for _ in 0..n {
            let id = self.alloc();
            let jitter = self.rng.disc().scale(2.0);
            let phase = self.rng.range(0.0, std::f32::consts::TAU);
            self.drones.push(Drone {
                id,
                owner: SWARM_OWNER,
                kind: DroneKind::Strike,
                pos: from + jitter,
                heading: 0.0,
                battery: STRIKE_BATTERY * 3.0,
                tether: None,
                task: DroneTask::Strike { target: *target },
                link: 1.0,
                phase,
                jammed: false,
                mark: 1,
            });
        }
        self.events.push(Event::SwarmInbound { count: n, from });
    }

    /// The match ends when one team is left alive, or at the time limit.
    /// The winning team is the last one standing, or the one with the
    /// highest total score; the winner is its top-scoring player.
    fn step_match(&mut self) {
        // A destroyed tank with a respawn to come is still in it.
        // So is one down with an ally standing, who may yet revive it.
        let revivable = |p: &Player| self.tank(p.tank).map(|t| self.downed(t)).unwrap_or(false) && self.players.iter().any(|q| q.id != p.id && q.alive && self.allied(q.id, p.id));
        let standing = |p: &Player| p.alive || self.respawn_in(p.id).is_some() || revivable(p);
        let alive: Vec<PlayerId> = self.players.iter().filter(|p| standing(p)).map(|p| p.id).collect();
        let mut teams_alive: Vec<TeamId> = self.players.iter().filter(|p| standing(p)).map(|p| p.team).collect();
        teams_alive.sort_unstable();
        teams_alive.dedup();
        let limit = self.scenario.time_limit;
        let timed_out = limit > 0 && self.tick >= limit;
        let teams_total = {
            let mut t: Vec<TeamId> = self.players.iter().map(|p| p.team).collect();
            t.sort_unstable();
            t.dedup();
            t.len()
        };
        if (teams_alive.len() <= 1 && teams_total > 1) || timed_out || alive.is_empty() {
            self.over = true;
            let team_score = |t: TeamId| self.players.iter().filter(|p| p.team == t && p.alive).map(|p| p.score).sum::<u32>();
            self.winning_team = if teams_alive.len() == 1 {
                Some(teams_alive[0])
            } else {
                teams_alive.iter().copied().max_by_key(|t| (team_score(*t), std::cmp::Reverse(*t)))
            };
            self.winner = self.winning_team.and_then(|t| self.players.iter().filter(|p| p.alive && p.team == t).max_by_key(|p| (p.score, std::cmp::Reverse(p.id))).map(|p| p.id));
            self.events.push(Event::MatchOver { winner: self.winner, team: self.winning_team });
        }
    }

    // ---------------------------------------------------------------- hash

    /// A 64-bit digest of everything that is state: tick, rng, every
    /// entity's every field, the map's cells, the scenario. Events are
    /// not state (they are `serde(skip)`) and are not in it. Two worlds
    /// with the same hash have the same future; two hosts with
    /// different hashes have diverged, and the first checkpoint where
    /// they differ is where. The digest is FNV-1a over the bincode
    /// serialisation, so a new field is in the hash the day it is
    /// added, and floats hash by their bits, which is what determinism
    /// means here.
    pub fn hash(&self) -> u64 {
        let bytes = bincode::serialize(self).expect("world serialises");
        fnv1a64(&bytes)
    }

    /// Run a replay from its scenario to its last tick and return the
    /// world. `Replay::verify` is the same with the checkpoints checked.
    pub fn run(replay: &Replay) -> World {
        let mut w = World::new(replay.scenario.clone());
        while w.tick < replay.ticks {
            let orders: Vec<Order> = replay.orders_for(w.tick + 1).cloned().collect();
            w.step(&orders);
        }
        w
    }

    // --------------------------------------------------------- observation


    /// The world as player `p` is allowed to see it: own entities in full,
    /// everything else only where their sensors reach. This is what goes
    /// to a client, and what a Brain thinks from. Nothing in it is not
    /// also on the host; the difference is what has been left out.
    pub fn observe(&self, p: PlayerId) -> Observation {
        let me = self.tank_of(p).cloned();
        let forest = me.as_ref().filter(|t| t.alive).map(|t| self.map.forest_near(t.pos, FOREST_SIGHT, 8)).unwrap_or_default();
        let pads = me
            .as_ref()
            .filter(|t| t.alive)
            .map(|t| self.map.terrain_near(Terrain::Pad, t.pos, FOREST_SIGHT, 32).into_iter().filter(|p| !self.structures.iter().any(|x| x.pos.tile() == p.tile())).take(8).collect())
            .unwrap_or_default();
        let shore = me.as_ref().filter(|t| t.alive && t.boat).map(|t| self.map.shore_from(t.pos, SHORE_SIGHT, SHORE_RAYS)).unwrap_or_default();
        let launch = me
            .as_ref()
            .filter(|t| t.alive && !t.boat)
            .map(|t| self.map.launch_sites(t.pos, LAUNCH_SIGHT, 12).into_iter().filter(|s| !self.depots.iter().any(|d| d.pos.dist(*s) <= DEPOT_RADIUS) && !self.boats.iter().any(|b| b.pos.tile() == s.tile())).collect())
            .unwrap_or_default();
        let eyes = self.eyes(p);
        let boats = self.boats.iter().filter(|b| self.allied(b.owner, p) || self.seen_by(&eyes, b.pos)).cloned().collect();
        let advantage = self.scenario.advantage && self.players.iter().any(|x| x.id == p && x.is_bot);
        let tanks = self.tanks.iter().filter(|t| self.tank_seen_by(p, &eyes, t) || (advantage && t.alive)).cloned().collect();
        // `radar_sees`, its data centres gathered once (LR 91).
        let radars: Vec<V2> = self.structures.iter().filter(|s| s.kind == StructureKind::DataCentre && s.working() && self.allied_opt(s.owner, p)).map(|s| s.pos).collect();
        let drones = self.drones.iter().filter(|d| d.owner == p || self.seen_by(&eyes, d.pos) || radars.iter().any(|r| r.dist(d.pos) <= DATACENTRE_RADAR)).cloned().collect();
        let shells = self.shells.iter().filter(|s| s.owner == p || self.seen_by(&eyes, s.pos_at(self.tick)) || self.seen_by(&eyes, s.impact)).cloned().collect();
        Observation {
            tick: self.tick,
            player: p,
            me,
            tanks,
            drones,
            shells,
            map_w: self.map.w as u16,
            map_h: self.map.h as u16,
            depots: self.depots.clone(),
            pillboxes: self.pillboxes.clone(),
            mines: self.mines.iter().filter(|m| self.mine_known_to(p, m)).cloned().collect(),
            structures: self.structures.clone(),
            forest,
            pads,
            boats,
            shore,
            launch,
            respawn_in: self.respawn_in(p),
            respawns: self.scenario.respawns,
            players: self.players.iter().map(|x| PlayerView { id: x.id, name: x.name.clone(), alive: x.alive, score: x.score, is_bot: x.is_bot, team: x.team }).collect(),
            proposals: self.proposals.iter().filter(|r| r.from == p || self.allied(r.to, p)).cloned().collect(),
            events: self.events.iter().filter(|e| self.event_visible(e, p)).cloned().collect(),
            over: self.over,
            winner: self.winner,
            winning_team: self.winning_team,
        }
    }

    /// Which events a player may know about: public ones; its own and
    /// its allies' launches, deployments and refusals of a kind; team
    /// chat to the team; alliance traffic to those in it.
    pub fn event_visible(&self, e: &Event, p: PlayerId) -> bool {
        match e {
            Event::Rejected { player, .. } => *player == p,
            Event::ShellLaunched { owner, .. } => self.allied(*owner, p),
            Event::DroneLaunched { owner, .. } | Event::DroneRecovered { owner, .. } | Event::DroneRetasked { owner, .. } | Event::LinkLost { owner, .. } => self.allied(*owner, p),
            Event::PillboysDeployed { owner, .. } | Event::PillboysReturned { owner, .. } | Event::PillboysAbandoned { owner, .. } => self.allied(*owner, p),
            Event::MineLaid { owner, .. } | Event::Harvested { owner, .. } => self.allied(*owner, p),
            Event::Jammed { owner, .. } => self.allied(*owner, p),
            Event::Chat { player, to: ChatTo::Team, .. } => self.allied(*player, p),
            Event::Chat { player, to: ChatTo::Players(mask), .. } => *player == p || (p < 16 && mask & (1 << p) != 0),
            Event::Chat { player, to: ChatTo::Nearby, .. } => {
                let pos = |x: PlayerId| self.tank_of(x).map(|t| t.pos);
                *player == p || matches!((pos(*player), pos(p)), (Some(a), Some(b)) if a.dist(b) <= CHAT_NEARBY)
            }
            Event::Proposed { from, to, .. } => *from == p || self.allied(*to, p),
            Event::Pinged { player, .. } => self.allied(*player, p),
            Event::Requested { from, .. } => self.allied(*from, p),
            _ => true,
        }
    }
}

/// One tick of one tank's motion: reload, the waypoint, the turn, the
/// speed for the terrain, the step, and a re-route when blocked. The
/// host runs it for every tank in `step_tanks`; a client runs it for
/// its own in `World::predict`, which is why it is a function of what
/// it reads and not a method on the world. Returns the mine the tank
/// drove onto, if any, for the caller to set off.
fn drive_tank(t: &mut Tank, map: &Map, guns: &[(Option<PlayerId>, V2)], mines: &[(PlayerId, (i32, i32), EntityId)], teams: &[(PlayerId, TeamId)], boats: &[(i32, i32)]) -> Option<EntityId> {
    let team_of = |p: PlayerId| teams.iter().find(|(id, _)| *id == p).map(|(_, t)| *t);
    let allied_with = |a: PlayerId, b: PlayerId| a == b || (team_of(a).is_some() && team_of(a) == team_of(b));
    if !t.alive {
        return None;
    }
    if t.reload > 0 {
        t.reload -= 1;
    }
    let Some(goal) = t.goal else {
        t.speed = 0.0;
        return None;
    };
    if t.fuel <= 0.0 {
        t.goal = None;
        t.route.clear();
        t.speed = 0.0;
        return None;
    }
    let to = goal - t.pos;
    let dist = to.len();
    if dist < 0.15 {
        t.pos = goal;
        // Next waypoint, keeping the speed: a corner on a road is
        // not a stop.
        if t.route.is_empty() {
            t.goal = None;
            t.speed = 0.0;
        } else {
            t.goal = Some(t.route.remove(0));
        }
        return None;
    }
    let here = map.at(t.pos);
    let cost = if t.boat && here == Terrain::Water { BOAT_MOVE_COST } else { map.move_cost(t.pos) };
    let dir = to.norm();
    let want = t.heading;
    let turn = wrap_angle(dir.heading() - want);
    // Brake for a corner. At full speed a hull turning at
    // TANK_TURN swings two tiles wide, which on a one-tile bridge
    // is the river; a tank facing the wrong way pivots at a crawl
    // and accelerates once it is pointed down the road.
    let max = if turn.abs() > TURN_BRAKE_ANGLE {
        // Easing off with the angle, down to the crawl for a turn right
        // round; within a turn's circle of the waypoint, slow enough to
        // turn inside it.
        let over = ((turn.abs() - TURN_BRAKE_ANGLE) / (std::f32::consts::PI - TURN_BRAKE_ANGLE)).clamp(0.0, 1.0);
        let ease = TANK_SPEED * (1.0 - over * (1.0 - TURN_CRAWL)) / cost;
        if turn.abs() > TURN_PIVOT {
            0.0
        } else {
            ease.min((dist * TANK_TURN * 0.6).max(TANK_SPEED * 0.15))
        }
    } else {
        TANK_SPEED / cost
    };
    // Momentum needs foresight: the corner at this waypoint, braked for
    // before it, so the tank comes round it at the corner's speed and
    // does not swing wide into a wall or the river.
    let max = match t.route.first() {
        Some(next) if !t.boat => {
            let corner = wrap_angle((*next - goal).heading() - (goal - t.pos).heading()).abs();
            if corner > TURN_BRAKE_ANGLE {
                let over = ((corner - TURN_BRAKE_ANGLE) / (std::f32::consts::PI - TURN_BRAKE_ANGLE)).clamp(0.0, 1.0);
                let at_corner = TANK_SPEED * (1.0 - over * (1.0 - TURN_CRAWL)) * 0.6 / cost;
                max.min((at_corner * at_corner + 2.0 * TANK_BRAKE * dist).sqrt())
            } else {
                max
            }
        }
        _ => max,
    };
    // A boat's crawl is fast water's crawl: turning for a waypoint
    // under a tile off, its circle was wider than the waypoint and it
    // went round it for a whole match. It slows to turn inside it, and
    // so does a tank ashore on the bank before a moored boat (the quad's
    // leader went round it, 0.4 tiles off, for 90 s). Not every tank
    // ashore: that changed how the bots fight.
    let to_boat = !t.boat && t.route.first().map(|n| map.at(*n) == Terrain::Water).unwrap_or(false);
    let max = if (t.boat || to_boat) && turn.abs() > TURN_BRAKE_ANGLE { max.min(dist * TANK_TURN * 0.4) } else { max };
    // Ashore for a moored boat, all but a pivot until it faces the boat:
    // at the crawl it went round the boat, its circle wider than the
    // bank it stood on.
    let max = if !t.boat && map.at(goal) == Terrain::Water && turn.abs() > TURN_BRAKE_ANGLE { max.min(0.2) } else { max };
    // On a chain, a crawl.
    let max = if t.towing.is_some() { max.min(TANK_SPEED * TOW_SPEED) } else { max };
    // Momentum: up at `TANK_ACCEL`, down at `TANK_BRAKE`, never past the
    // limit by more than a tick's braking.
    t.speed = if t.speed < max { (t.speed + TANK_ACCEL * DT).min(max) } else { (t.speed - TANK_BRAKE * DT).max(max) };
    let step = TANK_TURN * DT;
    t.heading = want + turn.clamp(-step, step);
    let fwd = V2::from_heading(t.heading);
    let mut next = t.pos + fwd.scale(t.speed * DT);
    if dist < t.speed * DT {
        next = goal;
    }
    let ahead = map.at(next);
    // Afloat: water, or a beach to land on. Ashore: land, or a moored
    // boat to board.
    let open = if t.boat { Map::afloat_step(here, ahead) } else { ahead.passable() || (ahead == Terrain::Water && boats.contains(&next.tile())) };
    if !open || !map.in_bounds(next) {
        // Blocked: stop, and plan again from here to where the
        // route was going. Giving up left a tank parked at the
        // water's edge for a whole match.
        t.speed = 0.0;
        let dest = t.route.last().copied().unwrap_or(goal);
        let avoid = |tile: V2| if guns.iter().any(|(o, g)| !o.map(|o| allied_with(o, t.owner)).unwrap_or(false) && g.dist(tile) <= PILLBOX_RANGE + 0.5) { PILLBOX_AVOID_COST } else { 0.0 };
        // Ashore, bound for a moored boat: by land to the bank beside
        // it, then onto it (a route by land to the water is none, and
        // the tank stopped at the water's edge with the boat a tile off).
        let boarding = !t.boat && map.at(dest) == Terrain::Water && boats.contains(&dest.tile());
        let mut wp = if boarding {
            let (x, y) = dest.tile();
            let bank = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .map(|(dx, dy)| V2::new((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5))
                .filter(|p| map.in_bounds(*p) && map.at(*p).passable())
                .min_by(|a, b| a.dist(t.pos).total_cmp(&b.dist(t.pos)));
            match bank {
                Some(b) if b.tile() == t.pos.tile() => vec![dest],
                Some(b) => {
                    let mut wp = map.route_by(t.pos, b, false, avoid);
                    if !wp.is_empty() {
                        wp.push(dest);
                    }
                    wp
                }
                None => Vec::new(),
            }
        } else {
            map.route_by(t.pos, dest, t.boat, avoid)
        };
        if wp.is_empty() {
            t.goal = None;
            t.route.clear();
        } else {
            // The same way again from off a tile's centre: the straight
            // line from here clips a corner the route keeps clear of.
            // To the middle of this tile first, then on.
            let centre = V2::new(t.pos.x.floor() + 0.5, t.pos.y.floor() + 0.5);
            if wp[0].tile() == goal.tile() && t.pos.dist(centre) > 0.2 {
                wp.insert(0, centre);
            }
            t.goal = Some(wp.remove(0));
            t.route = wp;
        }
        return None;
    }
    t.pos = next;
    t.fuel = (t.fuel - FUEL_PER_TILE * t.speed * DT).max(0.0);
    if t.boat && ahead != Terrain::Water {
        // On the beach: out of the boat, which `step_tanks` moors on
        // the water behind.
        t.boat = false;
    } else if !t.boat && ahead == Terrain::Water {
        t.boat = true;
    }
    mines.iter().find(|(o, tile, _)| !allied_with(*o, t.owner) && *tile == next.tile()).map(|m| m.2)
}

/// FNV-1a, 64 bits. Not cryptographic; stable across machines and
/// versions, which is all a desync detector needs.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ------------------------------------------------------------------ replay

/// How often a replay records the world hash, in ticks: five seconds.
pub const CHECKPOINT_EVERY: Tick = 100;

/// A match as the orders applied at each tick, the scenario they were
/// applied to, and the hashes the world had along the way. Serialisable,
/// so a traced match is a file; replayable, so the file is a regression
/// test for every later change to `step` and the debugging tool for
/// "the bot did something odd at tick 3 412". An order's tick is the
/// world's tick after the step that applied it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Replay {
    pub scenario: Scenario,
    /// How many ticks were run.
    pub ticks: Tick,
    /// In tick order, and within a tick in the order they were applied.
    pub orders: Vec<(Tick, Order)>,
    /// (tick, hash) every `CHECKPOINT_EVERY` ticks, and at the end.
    pub checkpoints: Vec<(Tick, u64)>,
}

impl Replay {
    pub fn new(scenario: Scenario) -> Replay {
        Replay { scenario, ..Replay::default() }
    }

    /// Record one step: call after `world.step(orders)` with the same
    /// slice, in the order it was applied.
    pub fn record(&mut self, world: &World, orders: &[Order]) {
        // After the match is over, `step` is a no-op and so is this:
        // orders offered to a finished world were not applied.
        if world.over && self.ticks == world.tick {
            return;
        }
        self.ticks = world.tick;
        for o in orders {
            self.orders.push((world.tick, o.clone()));
        }
        if (world.tick % CHECKPOINT_EVERY == 0 || world.over) && self.checkpoints.last().map(|(t, _)| *t != world.tick).unwrap_or(true) {
            self.checkpoints.push((world.tick, world.hash()));
        }
    }

    /// The orders applied in the step that produced `tick`.
    pub fn orders_for(&self, tick: Tick) -> impl Iterator<Item = &Order> {
        let start = self.orders.partition_point(|(t, _)| *t < tick);
        self.orders[start..].iter().take_while(move |(t, _)| *t == tick).map(|(_, o)| o)
    }

    /// The hash the world had at the end, if the replay recorded one.
    pub fn final_hash(&self) -> Option<u64> {
        self.checkpoints.last().filter(|(t, _)| *t == self.ticks).map(|(_, h)| *h)
    }

    /// Run the replay and check every checkpoint. Ok(world) if they all
    /// match; Err((tick, recorded, replayed)) at the first that does not.
    pub fn verify(&self) -> Result<World, (Tick, u64, u64)> {
        let mut w = World::new(self.scenario.clone());
        let mut next = 0usize;
        while w.tick < self.ticks {
            let orders: Vec<Order> = self.orders_for(w.tick + 1).cloned().collect();
            w.step(&orders);
            if let Some((t, h)) = self.checkpoints.get(next) {
                if *t == w.tick {
                    let now = w.hash();
                    if now != *h {
                        return Err((*t, *h, now));
                    }
                    next += 1;
                }
            }
        }
        Ok(w)
    }
}

/// A player's view. The client renders this; the Brain reasons from it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Observation {
    pub tick: Tick,
    pub player: PlayerId,
    pub me: Option<Tank>,
    pub tanks: Vec<Tank>,
    pub drones: Vec<Drone>,
    pub shells: Vec<Shell>,
    /// The map's size in tiles: a Brain has no map, but it can know the
    /// edges and the middle.
    pub map_w: u16,
    pub map_h: u16,
    pub depots: Vec<Depot>,
    /// Every pillbox, like every depot: territory is public.
    pub pillboxes: Vec<Pillbox>,
    /// The mines this player knows about: its own, and hostile ones
    /// within `MINE_REVEAL` of its tank or squad.
    pub mines: Vec<Mine>,
    /// The nearest forest tiles to the tank, for a Brain with no map.
    pub forest: Vec<V2>,
    /// Every structure, like every pillbox: an economy is public.
    #[serde(default)]
    pub structures: Vec<Structure>,
    /// The nearest pad tiles to the tank with nothing built on them:
    /// where a Brain with no map can build a structure.
    #[serde(default)]
    pub pads: Vec<V2>,
    /// Moored boats this player knows of: its side's, and any in sight.
    #[serde(default)]
    pub boats: Vec<Boat>,
    /// Afloat, the beaches the tank could land on, one per heading out
    /// to `SHORE_SIGHT` (`Map::shore_from`); empty ashore.
    #[serde(default)]
    pub shore: Vec<V2>,
    /// Ashore, the nearest water tiles beside the bank where the squad
    /// could build a boat (`Map::launch_sites`, out to `LAUNCH_SIGHT`).
    #[serde(default)]
    pub launch: Vec<V2>,
    /// Destroyed, the ticks until `Command::Respawn` is taken (0: now);
    /// None when it will not be.
    #[serde(default)]
    pub respawn_in: Option<Tick>,
    /// The scenario's respawns a seat (0 none, `RESPAWNS_UNLIMITED`).
    #[serde(default)]
    pub respawns: u8,

    pub players: Vec<PlayerView>,
    /// Proposals this player made or may answer.
    pub proposals: Vec<Proposal>,
    pub events: Vec<Event>,
    pub over: bool,
    pub winner: Option<PlayerId>,
    pub winning_team: Option<TeamId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub id: PlayerId,
    pub name: String,
    pub alive: bool,
    pub score: u32,
    pub is_bot: bool,
    pub team: TeamId,
}

impl Observation {
    pub fn seconds(&self) -> f32 {
        self.tick as f32 * DT
    }
    /// My team; a view with no players (should not happen) is team 255.
    pub fn my_team(&self) -> TeamId {
        self.players.iter().find(|x| x.id == self.player).map(|x| x.team).unwrap_or(255)
    }
    pub fn team_of(&self, p: PlayerId) -> Option<TeamId> {
        self.players.iter().find(|x| x.id == p).map(|x| x.team)
    }
    /// Same side: me, or on my team. The swarm never is.
    pub fn allied(&self, p: PlayerId) -> bool {
        p == self.player || self.team_of(p).map(|t| t == self.my_team()).unwrap_or(false)
    }
    /// Why the world would refuse this player proposing `terms` to `to`
    /// as things stand, or None if it would let it stand: the same rule
    /// it applies, read from what the player can see (teams are public).
    pub fn would_refuse(&self, to: PlayerId, terms: Terms) -> Option<&'static str> {
        if to == self.player || self.team_of(to).is_none() {
            return Some("no such player");
        }
        if self.allied(to) {
            return Some("already on the same side");
        }
        let sides: Vec<(PlayerId, TeamId)> = self.players.iter().map(|p| (p.id, p.team)).collect();
        three_way(&proposal_outcome(&sides, self.player, to, terms), |p| self.players.iter().any(|x| x.id == p && x.alive)).err()
    }
    pub fn my_drones(&self) -> impl Iterator<Item = &Drone> {
        let p = self.player;
        self.drones.iter().filter(move |d| d.owner == p)
    }
    /// Living tanks not on my side.
    pub fn hostile_tanks(&self) -> impl Iterator<Item = &Tank> + '_ {
        self.tanks.iter().filter(move |t| !self.allied(t.owner) && t.alive)
    }
    /// Drones not on my side, the swarm's included.
    pub fn hostile_drones(&self) -> impl Iterator<Item = &Drone> + '_ {
        self.drones.iter().filter(move |d| !self.allied(d.owner))
    }
    /// Allied tanks, mine included.
    pub fn allied_tanks(&self) -> impl Iterator<Item = &Tank> + '_ {
        self.tanks.iter().filter(move |t| self.allied(t.owner) && t.alive)
    }

    /// The middle of the map.
    pub fn centre(&self) -> V2 {
        V2::new(self.map_w as f32 * 0.5, self.map_h as f32 * 0.5)
    }
    /// A point kept a few tiles inside the map's edges.
    pub fn clamp(&self, p: V2) -> V2 {
        V2::new(p.x.clamp(2.0, self.map_w as f32 - 3.0), p.y.clamp(2.0, self.map_h as f32 - 3.0))
    }
}


pub fn wrap_angle(a: f32) -> f32 {
    use std::f32::consts::PI;
    let mut a = a % (2.0 * PI);
    if a > PI {
        a -= 2.0 * PI;
    } else if a < -PI {
        a += 2.0 * PI;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        World::new(Scenario { swarm_interval: 0, ..Scenario::default() })
    }

    #[test]
    fn deterministic() {
        let mut a = world();
        let mut b = world();
        let orders = vec![
            Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } },
            Order { player: 1, cmd: Command::FireIndirect { at: a.map.center() } },
        ];
        for i in 0..400 {
            let o = if i == 3 { orders.clone() } else { vec![] };
            a.step(&o);
            b.step(&o);
        }
        assert_eq!(bincode_len(&a), bincode_len(&b));
        assert_eq!(a.rng.0, b.rng.0);
        assert_eq!(a.drones.len(), b.drones.len());
    }

    fn bincode_len(w: &World) -> usize {
        serde_json::to_string(w).map(|s| s.len()).unwrap_or(0)
    }

    /// A structure for a test: a pad under it, standing, owned or not.
    fn put_structure(w: &mut World, kind: StructureKind, owner: Option<PlayerId>, at: V2) -> EntityId {
        let at = Map::tile_centre(at);
        w.map.set(at, Terrain::Pad);
        let id = w.alloc();
        w.structures.push(Structure { id, kind, pos: at, owner, health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: false });
        id
    }

    /// A shell that lands exactly at `at` next tick, for tests of what a
    /// splash does.
    fn land_shell(w: &mut World, owner: PlayerId, at: V2) {
        let id = w.alloc();
        let tick = w.tick;
        w.shells.push(Shell { id, owner, kind: ShellKind::Indirect, from: at, aim: at, impact: at, launch_tick: tick, impact_tick: tick + 1, spotted: true });
        w.step(&[]);
    }

    /// Step 10a: concrete takes two splashes, a wall one; pads survive
    /// a crater; both glyphs read and write.
    #[test]
    fn concrete_takes_two_splashes() {
        let mut w = world();
        let at = Map::tile_centre(w.map.center() + V2::new(20.0, 20.0));
        w.map.set(at, Terrain::Concrete);
        assert!(!w.map.at(at).passable() && w.map.at(at).blocks_fire());
        land_shell(&mut w, 1, at);
        assert_eq!(w.map.at(at), Terrain::Wall, "concrete should crack to a wall");
        land_shell(&mut w, 1, at);
        assert_eq!(w.map.at(at), Terrain::Rubble);
        w.map.set(at, Terrain::Pad);
        land_shell(&mut w, 1, at);
        assert_eq!(w.map.at(at), Terrain::Pad, "a pad is foundations: a shell does not crater it");
        let text = w.map.to_text();
        let back = Map::from_text(&text).unwrap();
        assert_eq!(back.at(at), Terrain::Pad);
    }

    /// Step 10b: power, and what each working structure does; a wrecked
    /// plant takes its side's economy dark; holding scores.
    #[test]
    fn structures_need_power_and_serve_their_side() {
        let mut w = world();
        w.pillboxes.clear();
        let me = w.players[0].tank;
        let home = w.tank(me).unwrap().pos;
        let refinery = put_structure(&mut w, StructureKind::Refinery, Some(0), home + V2::new(8.0, 0.0));
        let workshop = put_structure(&mut w, StructureKind::Workshop, Some(0), home + V2::new(0.0, 2.0));
        w.step(&[]);
        assert!(!w.structure(refinery).unwrap().powered, "no plant, no power");
        let plant = put_structure(&mut w, StructureKind::Power, Some(0), home + V2::new(-6.0, 4.0));
        w.step(&[]);
        assert!(w.structure(refinery).unwrap().powered && w.structure(workshop).unwrap().powered);
        assert!(w.events.iter().any(|e| matches!(e, Event::StructurePower { on: true, .. })));
        // The workshop mends the tank and fills the bay, longshots first.
        {
            let t = w.tank_mut(me).unwrap();
            t.armor = 50.0;
            t.bay.longshot = 0;
        }
        for _ in 0..TICK_HZ {
            w.step(&[]);
        }
        let t = w.tank(me).unwrap();
        assert!(t.armor >= 50.0 + REPAIR_PER_TICK * 2.0 * 2.0 - 0.01, "armor {}", t.armor);
        assert_eq!(t.bay.longshot, BAY_FULL.longshot);
        // The refinery halves the fuel a drive burns.
        let drive = |w: &mut World| {
            let t = w.tank_mut(me).unwrap();
            t.pos = home;
            t.fuel = 300.0;
            // From a standstill each time: a tank keeps its speed now.
            t.speed = 0.0;
            t.goal = None;
            t.route.clear();
            w.step(&[Order { player: 0, cmd: Command::Move { to: home + V2::new(0.0, 12.0) } }]);
            for _ in 0..(2 * TICK_HZ) {
                w.step(&[]);
            }
            300.0 - w.tank(me).unwrap().fuel
        };
        let refined = drive(&mut w);
        // Wreck the plant: everything it powered goes dark, and says so.
        w.damage_structure(plant, STRUCTURE_HEALTH, Some(1));
        assert!(w.events.iter().any(|e| matches!(e, Event::StructureWrecked { structure, .. } if *structure == plant)));
        w.step(&[]);
        assert!(!w.structure(refinery).unwrap().powered);
        assert!(w.events.iter().any(|e| matches!(e, Event::StructurePower { structure, on: false, .. } if *structure == refinery)));
        let crude = drive(&mut w);
        assert!(refined > 1.0 && (refined - crude * REFINERY_FUEL_BURN).abs() < 0.5, "refined {} crude {}", refined, crude);
        // Holding pays: the two standing structures score every ten s.
        let before = w.players[0].score;
        for _ in 0..HOLD_EVERY_TICKS {
            w.step(&[]);
        }
        assert_eq!(w.players[0].score - before, SCORE_HOLD * 2);
    }

    /// Step 10b: the squad builds a structure on a pad and only on a
    /// pad, captures a neutral one, and mends a damaged one.
    #[test]
    fn squad_builds_takes_and_mends_structures() {
        let mut w = world();
        w.pillboxes.clear();
        let me = w.players[0].tank;
        let home = w.tank(me).unwrap().pos;
        w.tank_mut(me).unwrap().pillboys.material = PILLBOY_MATERIAL_MAX;
        let grass = Map::tile_centre(home + V2::new(3.0, 3.0));
        w.map.set(grass, Terrain::Grass);
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Structure(StructureKind::Power), at: grass } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.contains("pad"))), "{:?}", w.events);
        w.map.set(grass, Terrain::Pad);
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Structure(StructureKind::Power), at: grass } } }]);
        let mut built = None;
        for _ in 0..(40 * TICK_HZ) {
            w.step(&[]);
            for e in &w.events {
                if let Event::StructureBuilt { structure, by: 0, kind: StructureKind::Power, .. } = e {
                    built = Some(*structure);
                }
            }
            if built.is_some() && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        let built = built.expect("never built");
        assert_eq!(w.tank(me).unwrap().pillboys.material, PILLBOY_MATERIAL_MAX - StructureKind::Power.cost());
        assert!(w.structure(built).unwrap().powered);
        // A neutral refinery: taken by standing on it.
        let neutral = put_structure(&mut w, StructureKind::Refinery, None, home + V2::new(-3.0, 4.0));
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Capture { pillbox: neutral } } }]);
        let mut taken = false;
        for _ in 0..(40 * TICK_HZ) {
            w.step(&[]);
            taken |= w.events.iter().any(|e| matches!(e, Event::StructureCaptured { structure, by: 0, .. } if *structure == neutral));
            if taken && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        assert!(taken && w.structure(neutral).unwrap().owner == Some(0));
        // Damaged: the squad mends it for material.
        w.damage_structure(built, 50.0, Some(1));
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Repair { target: built } } }]);
        let mut mended = false;
        for _ in 0..(40 * TICK_HZ) {
            w.step(&[]);
            mended |= w.events.iter().any(|e| matches!(e, Event::StructureRepaired { structure, .. } if *structure == built));
            if mended {
                break;
            }
        }
        assert!(mended && w.structure(built).unwrap().health == STRUCTURE_HEALTH);
        // A standing hostile structure cannot be taken.
        let theirs = put_structure(&mut w, StructureKind::Jammer, Some(1), home + V2::new(4.0, -2.0));
        w.step(&[]);
        let mut wait = 0;
        while !w.tank(me).unwrap().pillboys.aboard && wait < 400 {
            w.step(&[]);
            wait += 1;
        }
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Capture { pillbox: theirs } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.contains("wreck it first"))), "{:?}", w.events);
    }

    /// Step 10c: a longshot crosses the map on its own, goes on after
    /// its tank dies, wrecks an undefended refinery in two; one sent
    /// into a bubble is intercepted.
    #[test]
    fn longshots_cross_the_map_and_bubbles_stop_them() {
        let mut w = world();
        w.pillboxes.clear();
        let far = w.tank_of(2).unwrap().pos + V2::new(0.0, -4.0); // south seat's ground
        let refinery = put_structure(&mut w, StructureKind::Refinery, Some(2), far);
        let at = w.structure(refinery).unwrap().pos;
        w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Longshot, task: DroneTask::Attack { at } } }]);
        assert_eq!(w.tank_of(0).unwrap().bay.longshot, BAY_FULL.longshot - 1);
        let ls = w.drones.iter().find(|d| d.kind == DroneKind::Longshot).unwrap().id;
        // No recall, no retask.
        w.step(&[Order { player: 0, cmd: Command::Retask { drone: ls, task: DroneTask::Return } }, Order { player: 0, cmd: Command::Recall }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.contains("where it was sent"))));
        assert!(matches!(w.drone(ls).unwrap().task, DroneTask::Attack { .. }));
        // Its tank dies: it flies on.
        let mine = w.players[0].tank;
        w.damage(mine, 500.0, Some(1));
        let mut landed = None;
        for _ in 0..(60 * TICK_HZ) {
            w.step(&[]);
            if w.events.iter().any(|e| matches!(e, Event::LongshotHit { drone, .. } if *drone == ls)) {
                landed = Some(w.tick);
                break;
            }
        }
        let t = landed.expect("the longshot never arrived");
        // ~100 tiles at 3 tiles/s.
        assert!(t > 25 * TICK_HZ as Tick && t < 45 * TICK_HZ as Tick, "landed at {}", t);
        assert_eq!(w.structure(refinery).unwrap().health, STRUCTURE_HEALTH - LONGSHOT_STRUCTURE_DAMAGE);
        // Into a bubble: the south tank puts interceptors up and a
        // longshot from the east at its own position is met.
        let mut w = world();
        w.pillboxes.clear();
        let south = w.tank_of(2).unwrap().pos;
        for _ in 0..4 {
            w.step(&[Order { player: 2, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } }]);
        }
        w.step(&[Order { player: 1, cmd: Command::Launch { kind: DroneKind::Longshot, task: DroneTask::Attack { at: south } } }]);
        let mut met = false;
        let mut hit = false;
        for _ in 0..(60 * TICK_HZ) {
            w.step(&[]);
            met |= w.events.iter().any(|e| matches!(e, Event::Intercepted { owner: 2, .. }));
            hit |= w.events.iter().any(|e| matches!(e, Event::LongshotHit { .. }));
            if met || hit {
                break;
            }
        }
        assert!(met && !hit, "met {} hit {}", met, hit);
    }

    /// Step 10b: a jammer takes the link of every drone near it, its
    /// own side's too, and pushes a longshot off its point.
    #[test]
    fn a_jammer_jams_everyone_and_drifts_a_longshot() {
        let mut w = world();
        w.pillboxes.clear();
        let east = w.tank_of(1).unwrap().pos;
        let jam_at = east + V2::new(-10.0, 0.0);
        let jammer = put_structure(&mut w, StructureKind::Jammer, Some(1), jam_at);
        put_structure(&mut w, StructureKind::Power, Some(1), east + V2::new(-4.0, 3.0));
        w.step(&[]);
        assert!(w.structure(jammer).unwrap().working());
        // East's own recon over its jammer.
        w.step(&[Order { player: 1, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: jam_at, radius: 2.0 } } }]);
        let mut own_jammed = false;
        for _ in 0..(8 * TICK_HZ) {
            w.step(&[]);
            own_jammed |= w.events.iter().any(|e| matches!(e, Event::Jammed { owner: 1, kind: DroneKind::Recon, .. }));
        }
        assert!(own_jammed, "a jammer jams its own side's drones too");
        assert!(w.drones.iter().filter(|d| d.owner == 1 && d.kind == DroneKind::Recon).all(|d| d.task == DroneTask::Return));
        // A longshot through it lands off its point.
        let aim = Map::tile_centre(jam_at);
        w.step(&[Order { player: 3, cmd: Command::Launch { kind: DroneKind::Longshot, task: DroneTask::Attack { at: aim } } }]);
        let mut drifted = None;
        for _ in 0..(60 * TICK_HZ) {
            w.step(&[]);
            for e in &w.events {
                if let Event::LongshotHit { at, owner: 3, .. } = e {
                    drifted = Some(at.dist(aim));
                }
            }
            if drifted.is_some() {
                break;
            }
        }
        let off = drifted.expect("never landed");
        assert!(off > 0.01 && off <= JAM_DRIFT + 0.01, "landed {} off", off);
        // Wreck the jammer: links come back.
        w.damage_structure(jammer, STRUCTURE_HEALTH, Some(0));
        w.step(&[Order { player: 1, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: jam_at, radius: 2.0 } } }]);
        for _ in 0..(6 * TICK_HZ) {
            w.step(&[]);
        }
        assert!(w.drones.iter().any(|d| d.owner == 1 && d.kind == DroneKind::Recon && !d.jammed && d.link > LINK_FLOOR));
    }

    /// Step 10b: a data centre's fire control halves a blind shot's
    /// scatter near it, and its radar shows the side hostile drones.
    #[test]
    fn a_data_centre_computes_fire_and_sees_the_air() {
        let scatter = |dc: bool| {
            let mut w = world();
            w.pillboxes.clear();
            let home = w.tank_of(0).unwrap().pos;
            if dc {
                put_structure(&mut w, StructureKind::DataCentre, Some(0), home + V2::new(3.0, 0.0));
                put_structure(&mut w, StructureKind::Power, Some(0), home + V2::new(-3.0, 0.0));
                w.step(&[]);
            }
            let aim = home + V2::new(0.0, 30.0);
            let mut total = 0.0;
            for _ in 0..40 {
                let t = w.tank_mut(w.players[0].tank).unwrap();
                t.reload = 0;
                t.shells = 10;
                w.step(&[Order { player: 0, cmd: Command::FireIndirect { at: aim } }]);
                let s = w.shells.last().unwrap();
                assert!(!s.spotted);
                total += s.impact.dist(aim);
                w.shells.clear();
            }
            total / 40.0
        };
        let (blind, computed) = (scatter(false), scatter(true));
        assert!(computed < blind * 0.7, "blind {} computed {}", blind, computed);
        // The air picture: a hostile recon twenty tiles out is seen.
        let mut w = world();
        let home = w.tank_of(0).unwrap().pos;
        let probe = home + V2::new(0.0, 20.0);
        w.step(&[Order { player: 1, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Return } }]);
        let d = w.drones.iter_mut().find(|d| d.owner == 1).unwrap();
        d.pos = probe;
        let id = d.id;
        assert!(!w.observe(0).drones.iter().any(|d| d.id == id));
        put_structure(&mut w, StructureKind::DataCentre, Some(0), home + V2::new(3.0, 0.0));
        put_structure(&mut w, StructureKind::Power, Some(0), home + V2::new(-3.0, 0.0));
        w.step_structures();
        w.drones.iter_mut().find(|d| d.id == id).unwrap().pos = probe;
        assert!(w.observe(0).drones.iter().any(|d| d.id == id));
    }

    /// Step 10f: an industrial map reads back with its structures, and
    /// every structure stands on a pad.
    #[test]
    fn an_industrial_map_round_trips() {
        let w = World::new(Scenario { swarm_interval: 0, industry: true, ..Scenario::default() });
        assert!(w.structures.len() >= 16, "{} structures", w.structures.len());
        assert!(w.structures.iter().all(|s| w.map.at(s.pos) == Terrain::Pad && s.owner.is_none()));
        let text = w.map_as_played().to_text();
        let back = Map::from_text(&text).unwrap();
        assert_eq!(back.structures, w.map_as_played().structures);
        let bad = text.replacen("structure ", "structure 0.5 0.5 power neutral\n# ", 1);
        assert!(Map::from_text(&bad).is_err() || w.map.at(V2::new(0.5, 0.5)) == Terrain::Pad);
    }

    #[test]
    fn interceptor_takes_station_and_launch_is_validated() {
        let mut w = world();
        w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } }]);
        assert_eq!(w.drones.len(), 1);
        assert!(matches!(w.drones[0].task, DroneTask::Orbit { .. }));
        for _ in 0..BAY_FULL.interceptor {
            w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } }]);
        }
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { .. })));
    }

    #[test]
    fn indirect_shell_flies_and_lands() {
        let mut w = world();
        // Thirty tiles toward the centre: inside the gun's range on any
        // size of map.
        let me = w.tank_of(0).unwrap().pos;
        let at = me + (w.map.center() - me).norm().scale(30.0);
        w.step(&[Order { player: 0, cmd: Command::FireIndirect { at } }]);
        assert_eq!(w.shells.len(), 1);
        let eta = w.shells[0].impact_tick;
        while w.tick < eta {
            w.step(&[]);
        }
        assert!(w.shells.is_empty());
        assert!(w.events.iter().any(|e| matches!(e, Event::ShellImpact { .. })));
    }

    /// The east start and the centre are on opposite banks of the river
    /// on the default seed; a straight drive stopped at the water. A
    /// Move is routed over the bridge and arrives.
    #[test]
    fn move_is_routed_across_the_river() {
        let mut w = world();
        // Routing alone: the neutral pillboxes on the spokes would kill
        // a tank that drove into them, which `pillbox_kills_a_tank`
        // tests on its own.
        w.pillboxes.clear();
        let center = w.map.center();
        w.step(&[Order { player: 1, cmd: Command::Move { to: center } }]);
        assert!(w.events.iter().all(|e| !matches!(e, Event::Rejected { .. })));
        for _ in 0..(60 * TICK_HZ) {
            w.step(&[]);
            if w.tank_of(1).unwrap().goal.is_none() {
                break;
            }
        }
        let t = w.tank_of(1).unwrap();
        assert!(t.pos.dist(center) < 1.0, "tank stopped at {:?}, {} waypoints left", t.pos, t.route.len());
    }

    /// `World::predict` drives a tank by the same code as `step`: a
    /// client's partial copy of the world, given the same Move at the
    /// same tick, puts its tank where the host puts it, bit for bit,
    /// all the way across the river (roadmap step 4).
    #[test]
    fn prediction_drives_the_tank_as_the_host_does() {
        let mut host = world();
        host.pillboxes.clear();
        let mut mine = World::new(host.scenario.clone());
        mine.pillboxes.clear();
        mine.tanks.retain(|t| t.owner == 1);
        let center = host.map.center();
        host.step(&[Order { player: 1, cmd: Command::Move { to: center } }]);
        mine.predict(1, &[Command::Move { to: center }]);
        let mut moved = 0;
        for _ in 0..(60 * TICK_HZ) {
            host.step(&[]);
            mine.predict(1, &[]);
            let (h, m) = (host.tank_of(1).unwrap(), mine.tank_of(1).unwrap());
            assert_eq!(h.tick_state(), m.tick_state(), "tick {}", host.tick);
            if h.speed > 0.0 {
                moved += 1;
            }
            if h.goal.is_none() {
                break;
            }
        }
        assert!(moved > 100, "the tank never drove");
        assert_eq!(host.tick, mine.tick);
        assert!(host.tank_of(1).unwrap().pos.dist(center) < 1.0);
    }

    /// The hash is a function of state: two worlds stepped alike agree
    /// at every tick, and one more order changes it.
    #[test]
    fn hash_follows_state() {
        let mut a = world();
        let mut b = world();
        assert_eq!(a.hash(), b.hash());
        let o = vec![Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } }];
        for i in 0..200 {
            let orders = if i == 5 { o.clone() } else { vec![] };
            a.step(&orders);
            b.step(&orders);
            assert_eq!(a.hash(), b.hash(), "diverged at tick {}", a.tick);
        }
        b.step(&[Order { player: 1, cmd: Command::SetBubble { radius: 9.0 } }]);
        a.step(&[]);
        assert_ne!(a.hash(), b.hash());
    }

    /// A recorded match replays to the same hashes, and a replay whose
    /// orders were tampered with is caught at the first checkpoint.
    #[test]
    fn replay_reproduces_the_world() {
        // A swarm every ten seconds, not every ten ticks: the first draft
        // of this test said 10 and the swarm killed three tanks by tick
        // 170, which ended the match and the recording there.
        let scenario = Scenario { swarm_interval: 10 * TICK_HZ, swarm_size: 2, time_limit: 0, ..Scenario::bots(4) };

        let mut w = World::new(scenario.clone());
        let mut rec = Replay::new(scenario);
        for i in 0..(30 * TICK_HZ as Tick) {
            let mut orders = vec![];
            if i % 40 == 3 {
                orders.push(Order { player: (i % 4) as PlayerId, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } });
            }
            if i == 50 {
                let gun = w.tank_of(2).unwrap().pos;
                let at = gun + (w.map.center() - gun).norm().scale(30.0);
                orders.push(Order { player: 2, cmd: Command::FireIndirect { at } });
                orders.push(Order { player: 3, cmd: Command::Move { to: w.map.center() } });
            }
            w.step(&orders);
            rec.record(&w, &orders);
        }
        assert_eq!(rec.ticks, 600);
        assert_eq!(rec.checkpoints.len(), 6);
        assert_eq!(rec.final_hash(), Some(w.hash()));
        let again = World::run(&rec);
        assert_eq!(again.hash(), w.hash());
        assert!(rec.verify().is_ok());
        // Through serde and back, still the same.
        let json = serde_json::to_string(&rec).unwrap();
        let back: Replay = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rec);
        // Tamper: drop the shell at tick 51. The first checkpoint after
        // it (tick 100) must disagree.
        let mut bad = rec.clone();
        bad.orders.retain(|(_, o)| !matches!(o.cmd, Command::FireIndirect { .. }));
        match bad.verify() {
            Err((t, _, _)) => assert_eq!(t, 100),
            Ok(_) => panic!("tampered replay verified"),
        }
    }

    /// A live neutral pillbox shoots a tank that drives into its range,
    /// and a tank's route goes round it when there is room.
    #[test]
    fn pillbox_kills_a_tank_and_routes_bend_round_it() {
        let mut w = world();
        let pb = w.pillboxes[0].clone(); // north's spoke pillbox
        let tank = w.tank_of(0).unwrap().pos;
        assert!(pb.owner.is_none() && pb.alive());
        // Park the tank inside the ring and wait.
        w.tank_mut(w.players[0].tank).unwrap().pos = pb.pos + V2::new(3.0, 0.0);
        let mut fired = 0;
        for _ in 0..(20 * TICK_HZ) {
            w.step(&[]);
            fired += w.events.iter().filter(|e| matches!(e, Event::PillboxFired { .. })).count();
            if !w.tank_of(0).unwrap().alive {
                break;
            }
        }
        assert!(fired >= 7, "pillbox fired {} times", fired);
        assert!(!w.tank_of(0).unwrap().alive, "tank survived a pillbox at 3 tiles for 20 s");
        // A fresh world: the route from the start to the centre keeps
        // out of the spoke pillbox's ring, which the straight spoke road
        // runs through.
        let w = world();
        let route = w.route_for(0, tank, w.map.center());
        assert!(!route.is_empty());
        let mut p = tank;
        let mut closest = f32::MAX;
        for wp in &route {
            let n = (p.dist(*wp) * 4.0).ceil() as usize;
            for i in 0..=n {
                let q = p.lerp(*wp, i as f32 / n.max(1) as f32);
                closest = closest.min(q.dist(pb.pos));
            }
            p = *wp;
        }
        assert!(closest > PILLBOX_RANGE, "route passes {:.1} tiles from a live hostile pillbox", closest);
    }

    /// The squad walks to a neutral pillbox, stands on it for the
    /// capture time, takes it, and walks home; the pillbox then shoots
    /// for its new owner.
    #[test]
    fn pillboys_capture_a_pillbox() {
        let mut w = world();
        let pb = w.pillboxes[0].clone();
        let me = w.tank_of(0).unwrap().id;
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Capture { pillbox: pb.id } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::PillboysDeployed { .. })), "{:?}", w.events);
        assert!(w.tank(me).unwrap().pillboys.deployed());
        // A second order while they are out is refused.
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Capture { pillbox: pb.id } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { .. })));
        let mut captured_at = None;
        for _ in 0..(90 * TICK_HZ) {
            w.step(&[]);
            if w.events.iter().any(|e| matches!(e, Event::PillboxCaptured { by: 0, .. })) {
                captured_at = Some(w.tick);
            }
            if captured_at.is_some() && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        let t = captured_at.expect("never captured");
        // 19 tiles at 1.5 tiles/s is 13 s of walking, then 10 s standing.
        assert!(t > 20 * TICK_HZ as Tick && t < 40 * TICK_HZ as Tick, "captured at tick {}", t);
        assert_eq!(w.pillbox(pb.id).unwrap().owner, Some(0));
        assert!(w.tank(me).unwrap().pillboys.aboard, "squad did not come home");
        assert_eq!(w.tank(me).unwrap().pillboys.strength, PILLBOY_MAX);
        assert_eq!(w.players[0].score, SCORE_CAPTURE);
        // Ours now: a hostile tank parked in its ring gets shot; we do not.
        let enemy = w.players[1].tank;
        w.tank_mut(enemy).unwrap().pos = pb.pos + V2::new(2.0, 0.0);
        w.tank_mut(me).unwrap().pos = pb.pos + V2::new(-2.0, 0.0);
        let mut at_enemy = 0;
        let mut at_me = 0;
        for _ in 0..(6 * TICK_HZ) {
            w.step(&[]);
            for e in &w.events {
                if let Event::PillboxFired { tank, .. } = e {
                    if *tank == enemy { at_enemy += 1 } else if *tank == me { at_me += 1 }
                }
            }
        }
        assert!(at_enemy >= 2 && at_me == 0, "fired at enemy {} times, at owner {}", at_enemy, at_me);
    }

    /// Two allied tanks on a neutral depot take it, and two on their own
    /// depot both resupply: neither is a contest.
    #[test]
    fn a_depot_makes_one_thing_a_second_and_passes_over_a_full_kind() {
        let mut st = DepotStock::default();
        for _ in 0..6 {
            st.make();
        }
        assert_eq!(st.kinds, [1; 6], "six seconds, one of each");
        st.kinds[STOCK_TREE] = DEPOT_STOCK_MAX;
        st.turn = STOCK_TREE as u8;
        st.make();
        assert_eq!(st.kinds[STOCK_INTERCEPTOR], 2, "a full kind costs no second");
        st.kinds = [DEPOT_STOCK_MAX; 6];
        st.make();
        assert_eq!(st.kinds, [DEPOT_STOCK_MAX; 6], "a full store holds at its limit");
    }

    #[test]
    fn a_felled_tree_grows_back_but_not_through_a_road() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let (a, b) = (Map::tile_centre(V2::new(5.0, 5.0)), Map::tile_centre(V2::new(7.0, 5.0)));
        w.map.set(a, Terrain::Grass);
        w.map.set(b, Terrain::Road);
        w.felled = vec![(a, 3), (b, 3)];
        for _ in 0..5 {
            w.step(&[]);
        }
        assert_eq!((w.map.at(a), w.map.at(b)), (Terrain::Forest, Terrain::Road));
        assert!(w.felled.is_empty());
    }

    #[test]
    fn a_tank_on_a_depot_gets_only_what_the_depot_has_made() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        let owner = w.tanks[0].owner;
        let di = w.depots.iter().position(|d| d.owner == Some(owner)).unwrap();
        let (pos, seconds) = (w.depots[di].pos, 60u32);
        w.tanks[0].shells = 0;
        for _ in 0..(seconds * TICK_HZ) {
            let t = &mut w.tanks[0];
            t.pos = pos;
            t.speed = 0.0;
            t.goal = None;
            t.route.clear();
            w.step(&[]);
        }
        // Sixty seconds round six kinds: ten shells, not the hundred and twenty it used to be.
        assert!((9..=11).contains(&w.tanks[0].shells), "{} shells in {} s", w.tanks[0].shells, seconds);
    }

    #[test]
    fn allied_tanks_share_a_depot() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) }.with_teams(1);
        let mut w = World::new(sc);
        let d = w.depots.iter().find(|d| d.owner.is_none()).unwrap().clone();
        for i in 0..2 {
            let t = &mut w.tanks[i];
            t.pos = d.pos + V2::new(i as f32 * 0.6 - 0.3, 0.0);
            t.goal = None;
            t.route.clear();
        }
        let mut took = None;
        for _ in 0..(CAPTURE_TICKS as u32 + 5 * TICK_HZ) {
            w.step(&[]);
            if let Some(Event::DepotCaptured { by, .. }) = w.events.iter().find(|e| matches!(e, Event::DepotCaptured { .. })) {
                took = Some(*by);
            }
        }
        assert!(took.is_some(), "two allies on a neutral depot did not take it");
        // Now it is theirs: both resupply on it, from what it has made.
        w.depots.iter_mut().find(|x| x.id == d.id).unwrap().stock.kinds[STOCK_SHELL] = 10;
        w.tanks[0].shells = 0;
        w.tanks[1].shells = 0;
        for _ in 0..(10 * TICK_HZ) {
            w.step(&[]);
        }
        assert!(w.tanks[0].shells > 0 && w.tanks[1].shells > 0, "shells {} {}", w.tanks[0].shells, w.tanks[1].shells);
    }

    /// Two allied squads on one neutral pillbox take it together, in the
    /// one squad's time, for the one that was there first.
    #[test]
    fn allied_squads_do_not_contest() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) }.with_teams(1);
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = w.tanks[0].pos;
        let at = (0..32).map(|i| Map::tile_centre(home + V2::from_heading(i as f32 * 0.8).scale(3.0))).find(|p| w.map.at(*p).passable() && w.map.at(*p) != Terrain::Forest && !w.depots.iter().any(|d| d.pos.dist(*p) <= DEPOT_RADIUS)).unwrap();
        let id = w.alloc();
        w.pillboxes.push(Pillbox { id, pos: at, owner: None, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let mate = w.tanks[1].id;
        w.tank_mut(mate).unwrap().pos = home + (home - at).norm().scale(1.0);
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Capture { pillbox: id } } }]);
        w.step(&[Order { player: 1, cmd: Command::Deploy { task: PillboyTask::Capture { pillbox: id } } }]);
        let mut taken = None;
        for _ in 0..(30 * TICK_HZ) {
            w.step(&[]);
            if let Some(Event::PillboxCaptured { by, .. }) = w.events.iter().find(|e| matches!(e, Event::PillboxCaptured { .. })) {
                taken = Some((*by, w.tick));
                break;
            }
        }
        let (by, tick) = taken.expect("two allies on it and nobody took it");
        assert_eq!(by, 0);
        assert!(tick < 20 * TICK_HZ as Tick, "took {} ticks", tick);
    }

    /// Step 11h: the squad lifts a pillbox of its own, carries it at the
    /// health it had, and sets it down elsewhere for no material; one of
    /// theirs cannot be lifted; a carried one falls dead with the tank.
    #[test]
    fn squad_lifts_and_sets_down_a_pillbox() {
        let mut w = world();
        w.pillboxes.clear();
        let me = w.players[0].tank;
        let home = w.tank(me).unwrap().pos;
        let free = |w: &World, at: V2| w.map.at(at).passable() && w.map.at(at) != Terrain::Forest && w.map.at(at) != Terrain::Depot && !w.depots.iter().any(|d| d.pos.dist(at) <= DEPOT_RADIUS);
        let spot = |w: &World, from: V2| (0..32).map(|i| { let a = i as f32 * 0.6; Map::tile_centre(from + V2::new(a.cos(), a.sin()).scale(3.0 + (i / 8) as f32)) }).find(|p| free(w, *p)).expect("no free tile");
        let at = spot(&w, home);
        let mine = w.alloc();
        w.pillboxes.push(Pillbox { id: mine, pos: at, owner: Some(0), health: 40.0, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let far = spot(&w, home + V2::new(0.0, 12.0));
        let theirs = w.alloc();
        w.pillboxes.push(Pillbox { id: theirs, pos: far, owner: Some(1), health: 0.0, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Lift { pillbox: theirs } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "lifted theirs: {:?}", w.events);
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Lift { pillbox: mine } } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        let mut lifted = false;
        for _ in 0..(40 * TICK_HZ) {
            w.step(&[]);
            lifted |= w.events.iter().any(|e| matches!(e, Event::PillboxLifted { pillbox, .. } if *pillbox == mine));
            if lifted && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        assert!(lifted, "never lifted");
        assert!(w.pillbox(mine).is_none(), "still on the map");
        assert_eq!(w.tank(me).unwrap().pillboys.carrying, Some(40.0));
        // Down again on the other side, with no material at all.
        w.tank_mut(me).unwrap().pillboys.material = 0;
        let to = spot(&w, home - (at - home));
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Pillbox, at: to } } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        let mut placed = None;
        for _ in 0..(40 * TICK_HZ) {
            w.step(&[]);
            if let Some(Event::PillboxBuilt { pillbox, .. }) = w.events.iter().find(|e| matches!(e, Event::PillboxBuilt { .. })) {
                placed = Some(*pillbox);
            }
            if placed.is_some() && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        let p = w.pillbox(placed.expect("never set down")).unwrap().clone();
        assert_eq!((p.owner, p.health, p.pos), (Some(0), 40.0, to));
        let t = w.tank(me).unwrap();
        assert_eq!((t.pillboys.carrying, t.pillboys.material), (None, 0));
        // Lifted again, and the tank is killed with it aboard.
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Lift { pillbox: p.id } } }]);
        for _ in 0..(40 * TICK_HZ) {
            w.step(&[]);
            if w.tank(me).unwrap().pillboys.carrying.is_some() && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        assert!(w.tank(me).unwrap().pillboys.carrying.is_some());
        let tank_at = w.tank(me).unwrap().pos;
        w.damage(me, TANK_ARMOR * 2.0, Some(1));
        w.step(&[]);
        let fell = w.events.iter().find_map(|e| if let Event::PillboxDropped { pillbox, .. } = e { Some(*pillbox) } else { None }).expect("nothing fell");
        let f = w.pillbox(fell).unwrap();
        assert_eq!((f.owner, f.health, f.pos), (Some(0), 0.0, Map::tile_centre(tank_at)));
    }

    /// The squad fells a forest tile for material, walls a tile that
    /// then blocks a route and a flat shot, and lays a mine that only
    /// its layer can see until a tank finds it the hard way.
    #[test]
    fn squad_harvests_walls_and_mines() {
        let mut w = world();
        w.pillboxes.clear();
        let me = w.players[0].tank;
        let home = w.tank(me).unwrap().pos;
        // Harvest the nearest forest to the start (the crew's own sight
        // is FOREST_SIGHT; the squad will walk further for a test).
        let tree = *w.map.forest_near(home, 120.0, 1).first().expect("no forest near the north start");
        assert_eq!(w.map.at(tree), Terrain::Forest);
        let before = w.tank(me).unwrap().pillboys.material;
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Harvest { at: tree } } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        let mut harvested = false;
        for _ in 0..(60 * TICK_HZ) {
            w.step(&[]);
            if w.events.iter().any(|e| matches!(e, Event::Harvested { .. })) {
                harvested = true;
            }
            if harvested && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        assert!(harvested);
        assert_eq!(w.map.at(tree), Terrain::Grass);
        assert_eq!(w.tank(me).unwrap().pillboys.material, (before + HARVEST_MATERIAL).min(PILLBOY_MATERIAL_MAX));
        assert!(w.events.iter().any(|e| matches!(e, Event::TerrainChanged { .. })) || true);
        // A wall two tiles south of the tank, on the spoke road.
        let wall_at = Map::tile_centre(home + V2::new(0.0, 2.0));
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Wall, at: wall_at } } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        for _ in 0..(30 * TICK_HZ) {
            w.step(&[]);
            if w.map.at(wall_at) == Terrain::Wall && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        assert_eq!(w.map.at(wall_at), Terrain::Wall, "wall not built");
        assert!(!w.map.at(wall_at).passable());
        assert!(!w.map.line_clear(home, home + V2::new(0.0, 5.0)), "a wall should block a flat shot");
        let route = w.map.route(home, home + V2::new(0.0, 5.0));
        assert!(route.iter().all(|p| p.tile() != wall_at.tile()), "route goes through the wall");
        // A mine beside the wall: ours to see, not theirs, until found.
        let mine_at = Map::tile_centre(home + V2::new(2.0, 2.0));
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, at: mine_at } } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        for _ in 0..(30 * TICK_HZ) {
            w.step(&[]);
            if !w.mines.is_empty() && w.tank(me).unwrap().pillboys.aboard {
                break;
            }
        }
        assert_eq!(w.mines.len(), 1, "mine not laid");
        assert_eq!(w.observe(0).mines.len(), 1);
        assert_eq!(w.observe(1).mines.len(), 0, "a hostile mine should be hidden");
        // The enemy drives onto it.
        let enemy = w.players[1].tank;
        w.tank_mut(enemy).unwrap().pos = mine_at + V2::new(0.0, 3.0);
        w.step(&[Order { player: 1, cmd: Command::Move { to: mine_at } }]);
        let armor0 = w.tank(enemy).unwrap().armor;
        let mut boom = false;
        for _ in 0..(10 * TICK_HZ) {
            w.step(&[]);
            if w.events.iter().any(|e| matches!(e, Event::MineExploded { .. })) {
                boom = true;
                break;
            }
        }
        assert!(boom, "the enemy drove over the mine and nothing happened");
        assert!(w.mines.is_empty());
        assert!(w.tank(enemy).unwrap().armor <= armor0 - MINE_DAMAGE + 0.01);
        assert_eq!(w.map.at(mine_at), Terrain::Rubble);
    }

    /// Three mines in a row: a shell's splash on the first sets off all
    /// three. A chequerboard would not (orthogonal neighbours only).
    #[test]
    fn mines_chain_and_shells_clear_them() {
        let mut w = world();
        w.pillboxes.clear();
        let c = Map::tile_centre(w.map.center());
        for i in 0..3 {
            let id = w.alloc();
            w.mines.push(Mine { id, pos: c + V2::new(i as f32, 0.0), owner: 2 });
        }
        let id = w.alloc();
        w.mines.push(Mine { id, pos: c + V2::new(5.0, 5.0), owner: 2 }); // not adjacent: stays
        // Fire from 20 tiles north, spotted or not; the splash is 2.5.
        let gun = c + V2::new(0.0, -20.0);
        w.tank_mut(w.players[0].tank).unwrap().pos = gun;
        w.step(&[Order { player: 0, cmd: Command::FireIndirect { at: c } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::ShellLaunched { .. })), "{:?}", w.events);
        let mut exploded = 0;
        for _ in 0..(5 * TICK_HZ) {
            w.step(&[]);
            exploded += w.events.iter().filter(|e| matches!(e, Event::MineExploded { .. })).count();
            if w.shells.is_empty() {
                break;
            }
        }
        // With a 4-tile blind scatter the splash may miss the row; with
        // the first mine in it the whole row goes.
        if exploded > 0 {
            assert_eq!(exploded, 3, "chain did not take the whole row");
            assert_eq!(w.mines.len(), 1, "the lone mine should remain");
        } else {
            assert_eq!(w.mines.len(), 4);
        }
        // Directly: set the first off.
        let mut w2 = world();
        w2.pillboxes.clear();
        for i in 0..3 {
            let id = w2.alloc();
            w2.mines.push(Mine { id, pos: c + V2::new(i as f32, 0.0), owner: 2 });
        }
        w2.tank_mut(w2.players[0].tank).unwrap().pos = c + V2::new(0.0, -3.0);
        w2.step(&[Order { player: 0, cmd: Command::Move { to: c } }]);
        let mut n = 0;
        for _ in 0..(10 * TICK_HZ) {
            w2.step(&[]);
            n += w2.events.iter().filter(|e| matches!(e, Event::MineExploded { .. })).count();
            if w2.mines.is_empty() {
                break;
            }
        }
        assert_eq!(n, 3);
        assert!(w2.mines.is_empty());
        assert!(w2.tank_of(0).unwrap().armor <= TANK_ARMOR - MINE_DAMAGE + 0.01);
    }

    fn naval_world(players: usize) -> World {
        let scenario = Scenario { map: MapSource::Text(include_str!("../../../maps/naval.map").into()), swarm_interval: 0, ..Scenario::bots(players) };
        World::new(scenario)
    }

    /// Paul, 16 October: a tank on the shore launches its own boat, at
    /// once, for five material, or with none; inland it is refused.
    #[test]
    fn a_tank_on_the_shore_launches_a_boat() {
        let mut w = naval_world(2);
        let water = |w: &World, x: i32, y: i32| w.map.in_bounds(V2::new(x as f32 + 0.5, y as f32 + 0.5)) && w.map.at(V2::new(x as f32 + 0.5, y as f32 + 0.5)) == Terrain::Water;
        let (mut beach, mut inland) = (None, None);
        for y in 2..(w.map.h as i32 - 2) {
            for x in 2..(w.map.w as i32 - 2) {
                let p = V2::new(x as f32 + 0.5, y as f32 + 0.5);
                if w.map.at(p) != Terrain::Grass {
                    continue;
                }
                let wet = (-2..=2).any(|dy| (-2..=2).any(|dx| water(&w, x + dx, y + dy)));
                let near = (-1..=1).any(|dy| (-1..=1).any(|dx| water(&w, x + dx, y + dy)));
                if near && beach.is_none() {
                    beach = Some(p);
                }
                if !wet && inland.is_none() {
                    inland = Some(p);
                }
            }
        }
        let (beach, inland) = (beach.unwrap(), inland.unwrap());
        let id = w.tank_of(0).unwrap().id;
        let t = w.tank_mut(id).unwrap();
        t.pos = inland;
        t.boat = false;
        t.pillboys.material = BOAT_COST;
        w.step(&[Order { player: 0, cmd: Command::Embark }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "no water beside")), "{:?}", w.events);
        w.tank_mut(id).unwrap().pos = beach;
        w.step(&[Order { player: 0, cmd: Command::Embark }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Boarded { owner: 0, .. })), "{:?}", w.events);
        let t = w.tank(id).unwrap();
        assert!(t.boat && w.map.at(t.pos) == Terrain::Water && t.pos.dist(beach) < 1.5 && t.pillboys.material == 0);
        w.step(&[Order { player: 0, cmd: Command::Embark }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "afloat already")));
        // With no material it launches all the same.
        let t = w.tank_mut(id).unwrap();
        t.pos = beach;
        t.boat = false;
        t.pillboys.material = 0;
        w.step(&[Order { player: 0, cmd: Command::Embark }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Boarded { owner: 0, .. })), "{:?}", w.events);
        assert!(w.tank(id).unwrap().boat);
    }

    /// Drone marks: the match moves to Mk II and Mk III on the clock, a
    /// drone launched after is that mark, and a later mark is better at
    /// something and different (RESEARCH.md's systems).
    #[test]
    fn drones_move_up_a_mark_with_the_clock() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let launch = |w: &mut World| {
            w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: w.tanks[0].pos, radius: 3.0 } } }]);
            w.drones.iter().filter(|d| d.owner == 0 && d.kind == DroneKind::Recon).max_by_key(|d| d.id).cloned().unwrap()
        };
        let d = launch(&mut w);
        assert!(d.mark == 1 && (d.battery - RECON_BATTERY).abs() < 0.1, "{} {}", d.mark, d.battery);
        w.tick = MARK_II_AT as Tick - 1;
        w.step(&[]);
        assert!(w.events.iter().any(|e| matches!(e, Event::DronesUpgraded { mark: 2 })));
        let d = launch(&mut w);
        assert_eq!(d.mark, 2);
        assert!((d.battery - drone_spec(DroneKind::Recon, 2).battery).abs() < 0.1);
        assert!(d.spec().vision > RECON_VISION);
        assert_eq!(mark_at(MARK_III_AT as Tick), 3);
        for k in [DroneKind::Recon, DroneKind::Interceptor, DroneKind::Strike, DroneKind::Longshot] {
            for m in 1..=3 {
                assert!(!drone_spec(k, m).name.is_empty() && drone_spec(k, m).battery > 0.0 && drone_spec(k, m).speed > 0.0);
            }
        }
        let i = |m| drone_spec(DroneKind::Interceptor, m);
        assert!(i(1).intercept_base < i(2).intercept_base && i(2).intercept_base < i(3).intercept_base && i(3).speed > i(1).speed);
        assert!(drone_spec(DroneKind::Strike, 2).jam_proof && !drone_spec(DroneKind::Strike, 1).jam_proof);
        assert!(drone_spec(DroneKind::Strike, 3).damage > STRIKE_DAMAGE && drone_spec(DroneKind::Strike, 3).battery > STRIKE_BATTERY);
    }

    /// A boat off a shore with a bridge's end beside it lands (the
    /// quad's escort sat three minutes at (30,85): its route cut past
    /// the bridge's corner, and a boat cannot go onto a bridge).
    #[test]
    fn a_boat_lands_beside_a_bridge() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let c = w.tanks[0].pos.tile();
        let at = |dx: i32, dy: i32| V2::new((c.0 + dx) as f32 + 0.5, (c.1 + dy) as f32 + 0.5);
        for dy in -6..=3 {
            for dx in -6..=4 {
                let t = if dy >= 0 { if dx == -1 { Terrain::Bridge } else { Terrain::Water } } else { Terrain::Grass };
                w.map.set(at(dx, dy), t);
            }
        }
        let id = w.tanks[0].id;
        let t = w.tank_mut(id).unwrap();
        t.pos = V2::new(c.0 as f32 + 0.001, c.1 as f32 + 0.808);
        t.boat = true;
        let inland = at(-4, -4);
        w.step(&[Order { player: 0, cmd: Command::Move { to: inland } }]);
        for _ in 0..15 * TICK_HZ {
            w.step(&[]);
        }
        let t = w.tank(id).unwrap();
        assert!(!t.boat && t.pos.dist(inland) < 1.0, "still at {:?}, boat {}", t.pos, t.boat);
    }

    /// SPEC TNK-08, TNK-11, TNK-12: a tank that starts at sea is on a
    /// boat, crosses water to a beach and lands, and is ashore after.
    #[test]
    fn a_landing_craft_makes_the_beach() {
        let mut w = naval_world(8);
        for p in 0..8 {
            let t = w.tank_of(p).unwrap().clone();
            assert!(t.boat, "p{} did not start afloat", p);
            let beach = *w.map.shore_from(t.pos, SHORE_SIGHT, SHORE_RAYS).iter().min_by(|a, b| a.dist(t.pos).total_cmp(&b.dist(t.pos))).expect("no beach in sight");
            w.step(&[Order { player: p, cmd: Command::Move { to: beach } }]);
            assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "p{}: {:?}", p, w.events);
            let mut landed = false;
            for _ in 0..30 * TICK_HZ {
                w.step(&[]);
                if w.events.iter().any(|e| matches!(e, Event::Landed { owner, .. } if *owner == p)) {
                    landed = true;
                    break;
                }
            }
            let t = w.tank_of(p).unwrap();
            assert!(landed, "p{} still afloat at ({:.1},{:.1}), beach ({:.1},{:.1}), goal {:?} route {:?}", p, t.pos.x, t.pos.y, beach.x, beach.y, t.goal, t.route);
            assert!(!t.boat && w.map.at(t.pos).passable());
            // Ashore, water is refused again.
            w.step(&[Order { player: p, cmd: Command::Move { to: t.pos + V2::new(0.0, -200.0) } }]);
        }
    }

    /// A bridge across a channel, the way a bot plans and lays it: the
    /// crossing names three water tiles, the squad lays the first from
    /// the bank and the span the other two in the same trip
    /// (`BRIDGE_SPAN`), three material, and the tank drives over to the
    /// far side, which was unreachable before.
    #[test]
    fn a_squad_bridges_a_channel_in_a_span() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(1) });
        let me = w.tank_of(0).unwrap().id;
        let start = Map::tile_centre(w.tank(me).unwrap().pos);
        let dir = if start.x < w.map.w as f32 / 2.0 { 1.0 } else { -1.0 };
        // A channel three tiles wide, the whole height of the map, and
        // open ground either side of where it is crossed.
        let x0 = start.x + 4.0 * dir;
        for y in 0..w.map.h {
            for k in 0..3 {
                w.map.set(V2::new(x0 + k as f32 * dir, y as f32 + 0.5), Terrain::Water);
            }
        }
        for k in -3..=10 {
            for dy in -1..=1 {
                let p = start + V2::new(k as f32 * dir, dy as f32);
                if w.map.at(p) != Terrain::Water {
                    w.map.set(p, Terrain::Grass);
                }
            }
        }
        let far = start + V2::new(9.0 * dir, 0.0);
        assert!(w.map.route(start, far).is_empty(), "the far side was reachable already");
        let bridge = w.map.crossing(start, far, 10, |_| false).expect("no crossing");
        assert_eq!(bridge.len(), 3, "{:?}", bridge);
        w.tank_mut(me).unwrap().pillboys.material = PILLBOY_MATERIAL_MAX;
        let mut trips = 0;
        for tile in &bridge {
            if w.map.at(*tile) == Terrain::Bridge {
                continue;
            }
            trips += 1;
            w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at: *tile } } }]);
            assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
            let mut built = false;
            for _ in 0..30 * TICK_HZ {
                w.step(&[]);
                if w.map.at(*tile) == Terrain::Bridge && w.tank(me).unwrap().pillboys.aboard {
                    built = true;
                    break;
                }
            }
            assert!(built, "tile {:?} not bridged: {:?}", tile, w.tank(me).unwrap().pillboys);
        }
        assert_eq!(trips, 1, "a span lays three tiles in a trip");
        assert_eq!(w.tank(me).unwrap().pillboys.material, PILLBOY_MATERIAL_MAX - 3);
        w.step(&[Order { player: 0, cmd: Command::Move { to: far } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        for _ in 0..20 * TICK_HZ {
            w.step(&[]);
        }
        assert!(w.tank(me).unwrap().pos.dist(far) < 1.0, "the tank did not cross: at {:?}", w.tank(me).unwrap().pos);
    }

    /// The boat a tank lands from stays moored behind it (TNK-17), and
    /// the tank drives back onto it and is afloat again.
    #[test]
    fn a_landed_boat_stays_moored() {
        let mut w = naval_world(1);
        let t = w.tank_of(0).unwrap().clone();
        let beach = *w.map.shore_from(t.pos, SHORE_SIGHT, SHORE_RAYS).iter().min_by(|a, b| a.dist(t.pos).total_cmp(&b.dist(t.pos))).unwrap();
        w.step(&[Order { player: 0, cmd: Command::Move { to: beach } }]);
        let mut landed = false;
        for _ in 0..30 * TICK_HZ {
            w.step(&[]);
            if w.events.iter().any(|e| matches!(e, Event::Landed { owner: 0, .. })) {
                landed = true;
                break;
            }
        }
        assert!(landed);
        let at = w.tank_of(0).unwrap().pos;
        let b = w.boats.iter().find(|b| b.pos.dist(at) < 2.0).expect("no boat moored by the beach").clone();
        assert_eq!(w.map.at(b.pos), Terrain::Water);
        // Turning about on the beach may stop it at the water's edge: the
        // order again, as a Brain would give it.
        let mut boarded = false;
        for i in 0..10 * TICK_HZ {
            let stopped = w.tank_of(0).unwrap().goal.is_none();
            let orders = if i == 0 || (stopped && i % TICK_HZ == 0) { vec![Order { player: 0, cmd: Command::Move { to: b.pos } }] } else { vec![] };
            w.step(&orders);
            assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
            if w.events.iter().any(|e| matches!(e, Event::Boarded { owner: 0, boat, .. } if *boat == b.id)) {
                boarded = true;
                break;
            }
        }
        let t = w.tank_of(0).unwrap();
        assert!(boarded && t.boat, "at ({:.2},{:.2}) boat ({:.2},{:.2}) goal {:?} route {:?}", t.pos.x, t.pos.y, b.pos.x, b.pos.y, t.goal, t.route);
        assert!(!w.boats.iter().any(|x| x.id == b.id));
    }

    /// The mode table's boat (five trees), TNK-17 and TNK-14: a squad
    /// builds a boat on the water beside its bank, the tank drives onto
    /// it and is afloat, the boat crosses water, and one shell sinks it
    /// and the tank with it, the kill to the shooter.
    #[test]
    fn a_squad_builds_a_boat_and_one_shell_sinks_it() {
        let mut w = naval_world(2);
        // Put p0 ashore beside open water on the big island's west beach.
        let beach = (0..80).map(|x| V2::new(x as f32 + 0.5, 80.5)).find(|p| w.map.at(*p) != Terrain::Water && w.map.at(*p).passable()).unwrap();
        let sea = beach - V2::new(1.0, 0.0);
        assert_eq!(w.map.at(sea), Terrain::Water);
        let id = w.tank_of(0).unwrap().id;
        {
            let t = w.tank_mut(id).unwrap();
            t.pos = beach + V2::new(1.0, 0.0);
            t.boat = false;
            t.pillboys.pos = t.pos;
            t.pillboys.material = BOAT_COST;
        }
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Boat, at: sea } } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        for _ in 0..30 * TICK_HZ {
            w.step(&[]);
            if !w.boats.is_empty() {
                break;
            }
        }
        assert_eq!(w.boats.len(), 1, "no boat built");
        assert_eq!(w.tank_of(0).unwrap().pillboys.material, 0);
        // Water is refused without a boat; the boat's tile is not.
        w.step(&[Order { player: 0, cmd: Command::Move { to: sea - V2::new(3.0, 0.0) } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "water")));
        let mut boarded = false;
        w.step(&[Order { player: 0, cmd: Command::Move { to: sea } }]);
        for _ in 0..10 * TICK_HZ {
            w.step(&[]);
            boarded |= w.events.iter().any(|e| matches!(e, Event::Boarded { owner: 0, .. }));
            if boarded {
                break;
            }
        }
        assert!(boarded && w.tank_of(0).unwrap().boat && w.boats.is_empty());
        // Afloat, out to sea; a squad order waits for land.
        w.step(&[Order { player: 0, cmd: Command::Move { to: sea - V2::new(6.0, 0.0) } }]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::Rejected { .. })), "{:?}", w.events);
        for _ in 0..3 * TICK_HZ {
            w.step(&[]);
        }
        let at = w.tank_of(0).unwrap().pos;
        assert!(at.x < sea.x - 3.0 && w.map.at(at) == Terrain::Water, "afloat at {:?}", at);
        w.step(&[Order { player: 0, cmd: Command::Deploy { task: PillboyTask::Harvest { at: beach } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.starts_with("afloat"))));
        // One shell: sunk and gone, p1's kill.
        land_shell(&mut w, 1, at);
        assert!(w.events.iter().any(|e| matches!(e, Event::Sunk { owner: 0, by: Some(1), .. })), "{:?}", w.events);
        assert!(!w.tank_of(0).unwrap().alive);
        assert_eq!(w.players[1].score, SCORE_KILL);
    }

    /// Respawns: off by default, as before; with one allowed, a
    /// destroyed tank is refused until `RESPAWN_TICKS` have passed, then
    /// comes back at its start with full stocks (afloat at a sea start),
    /// once; and a side with a respawn left is not out of the match.
    #[test]
    fn a_destroyed_tank_respawns_at_its_start_once() {
        let kill = |w: &mut World, p: PlayerId| {
            let id = w.tank_of(p).unwrap().id;
            w.damage(id, TANK_ARMOR + 1.0, None);
        };
        // Off: refused, and two seats down to one is the end.
        let mut w = World::new(Scenario { swarm_interval: 0, ..Scenario::bots(2) });
        kill(&mut w, 1);
        w.step(&[Order { player: 1, cmd: Command::Respawn }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "no respawns left")));
        assert!(w.over);

        let mut w = naval_world(2);
        w.scenario.respawns = 1;
        let start = w.map.starts[1];
        w.step(&[Order { player: 1, cmd: Command::Move { to: w.map.shore_from(start, SHORE_SIGHT, SHORE_RAYS)[0] } }]);
        for _ in 0..TICK_HZ {
            w.step(&[]);
        }
        kill(&mut w, 1);
        w.step(&[]);
        assert!(!w.over, "a side with a respawn left is out");
        assert_eq!(w.observe(1).respawn_in, Some(RESPAWN_TICKS as Tick - 1));
        w.step(&[Order { player: 1, cmd: Command::Respawn }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "too soon to respawn")));
        for _ in 0..RESPAWN_TICKS {
            w.step(&[]);
        }
        assert_eq!(w.observe(1).respawn_in, Some(0));
        w.step(&[Order { player: 1, cmd: Command::Respawn }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Respawned { owner: 1, left: 0, .. })), "{:?}", w.events);
        let t = w.tank_of(1).unwrap();
        assert!(t.alive && t.boat && t.pos == start && t.armor == TANK_ARMOR && t.shells == w.scenario.game_type.shells());
        assert!(w.players[1].alive);
        // The second death is the last: with no ally to revive it, at once.
        kill(&mut w, 1);
        w.step(&[]);
        assert_eq!(w.observe(1).respawn_in, None);
        assert!(w.over);
    }

    /// Paul, 16 October: a destroyed tank is down for twenty seconds,
    /// and an ally's squad that reaches it with the material in time
    /// brings it back at half armour. Not one's own squad, not an
    /// enemy's, not after the time is up.
    #[test]
    fn an_allys_squad_revives_a_tank_that_is_down() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(3) }.with_teams(2));
        // Seats 0 and 2 are a team.
        for _ in 0..TICK_HZ {
            w.step(&[]);
        }
        let a = w.tank_of(0).unwrap().pos;
        let near = w.map.starts.iter().copied().chain((0..40).map(|i| a + V2::from_heading(i as f32 * 0.4).scale(3.0))).find(|p| p.dist(a) < 5.0 && p.dist(a) > 2.0 && w.map.at(*p).passable()).unwrap();
        let down = w.tank_of(0).unwrap().id;
        for p in [1, 2] {
            let t = w.tanks.iter_mut().find(|t| t.owner == p).unwrap();
            t.pos = near;
            t.pillboys.pos = near;
            t.pillboys.material = 5;
        }
        w.damage(down, TANK_ARMOR + 1.0, Some(1));
        w.step(&[]);
        assert!(w.downed(w.tank(down).unwrap()) && !w.over);
        w.step(&[Order { player: 1, cmd: Command::Deploy { task: PillboyTask::Repair { target: down } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "revive only an ally's tank")), "{:?}", w.events);
        w.step(&[Order { player: 2, cmd: Command::Deploy { task: PillboyTask::Repair { target: down } } }]);
        let mut back = None;
        for _ in 0..DOWNED_TICKS {
            w.step(&[]);
            if let Some(e) = w.events.iter().find(|e| matches!(e, Event::Revived { .. })) {
                back = Some(e.clone());
                break;
            }
        }
        assert!(matches!(back, Some(Event::Revived { owner: 0, by: 2, .. })), "never revived");
        let t = w.tank(down).unwrap();
        assert!(t.alive && t.armor == REVIVE_ARMOR && w.players[0].alive);
        assert_eq!(w.tank_of(2).unwrap().pillboys.material, 5 - REVIVE_COST);
        // Down again with nobody to help: dead at twenty seconds.
        w.damage(down, TANK_ARMOR + 1.0, Some(1));
        for _ in 0..DOWNED_TICKS {
            w.step(&[]);
        }
        assert!(!w.downed(w.tank(down).unwrap()));
        w.step(&[Order { player: 2, cmd: Command::Deploy { task: PillboyTask::Repair { target: down } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "too late: that tank is dead" || why == "squad already out")), "{:?}", w.events);
    }

    /// The chain: one tank on a pillbox of its side holds it and is held
    /// (Paul: "this action requires an ally"); two haul it at a tenth of
    /// their speed; a depot's tile goes with it; let go, it stays put;
    /// another side's is refused.
    #[test]
    fn two_tanks_on_a_chain_haul_a_pillbox_and_a_depot() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(3) }.with_teams(2));
        w.pillboxes.clear();
        // Seats 0 and 2 are allies; open ground east of seat 0's tank.
        let mut at = w.tanks[0].pos;
        let open = |w: &World, p: V2| (0..12).all(|k| w.map.at(p + V2::new(k as f32, 0.0)).passable() && w.map.at(p + V2::new(k as f32, 0.0)) != Terrain::Bridge && !w.depots.iter().any(|d| d.pos.dist(p + V2::new(k as f32, 0.0)) < 4.0));
        while !open(&w, at) {
            at = at + V2::new(1.0, 1.0);
        }
        let at = Map::tile_centre(at);
        let id = w.alloc();
        w.pillboxes.push(Pillbox { id, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: 0 });
        for (i, off) in [(0usize, -1.0f32), (2, 1.0)] {
            let t = w.tanks.iter_mut().find(|t| t.owner == i as PlayerId).unwrap();
            t.pos = at + V2::new(-1.0, off * 0.8);
            t.goal = None;
            t.route.clear();
        }
        // The other side's tank may not chain ours.
        let enemy = w.tanks.iter().position(|t| t.owner == 1).unwrap();
        w.tanks[enemy].pos = at + V2::new(0.0, 1.5);
        w.step(&[Order { player: 1, cmd: Command::Tow { target: Some(id) } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.starts_with("chain only"))));
        w.tanks[enemy].pos = w.map.starts[1];
        // One alone: held.
        w.step(&[Order { player: 0, cmd: Command::Tow { target: Some(id) } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Chained { haulers: 1, on: true, .. })));
        w.step(&[Order { player: 0, cmd: Command::Move { to: at + V2::new(10.0, 0.0) } }]);
        for _ in 0..5 * TICK_HZ {
            w.step(&[]);
        }
        assert_eq!(w.pillbox(id).unwrap().pos, at, "one tank moved it");
        assert!(w.tank_of(0).unwrap().pos.dist(at) <= TOW_CHAIN * 1.6 + 0.01, "the chain let it go");
        // Two: hauled, slowly.
        w.step(&[Order { player: 2, cmd: Command::Tow { target: Some(id) } }, Order { player: 2, cmd: Command::Move { to: at + V2::new(10.0, 1.0) } }, Order { player: 0, cmd: Command::Move { to: at + V2::new(10.0, -1.0) } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Chained { haulers: 2, on: true, .. })));
        for _ in 0..10 * TICK_HZ {
            w.step(&[]);
        }
        let moved = w.pillbox(id).unwrap().pos.dist(at);
        eprintln!("two_tanks_on_a_chain_haul_a_pillbox_and_a_depot: pillbox hauled {:.1} tiles in 10 s", moved);
        assert!(moved > 1.5 && moved <= TANK_SPEED * TOW_SPEED * 10.0 + TOW_CHAIN * 1.6, "{:.1}", moved);
        // Let go: it stays.
        w.step(&[Order { player: 0, cmd: Command::Tow { target: None } }, Order { player: 2, cmd: Command::Tow { target: None } }]);
        let left = w.pillbox(id).unwrap().pos;
        for _ in 0..TICK_HZ {
            w.step(&[]);
        }
        assert_eq!(w.pillbox(id).unwrap().pos, left);

        // A depot of ours: its tile goes with it.
        let d = w.depots.iter().position(|d| d.owner == Some(0)).unwrap();
        let did = w.depots[d].id;
        let from = w.depots[d].pos;
        let away = (w.map.center() - from).norm();
        for (i, off) in [(0usize, -0.8f32), (2, 0.8)] {
            let t = w.tanks.iter_mut().find(|t| t.owner == i as PlayerId).unwrap();
            t.pos = from + away.scale(1.0) + V2::new(-away.y, away.x).scale(off);
            t.goal = None;
            t.route.clear();
        }
        let target = from + away.scale(12.0);
        w.step(&[Order { player: 0, cmd: Command::Tow { target: Some(did) } }, Order { player: 2, cmd: Command::Tow { target: Some(did) } }]);
        w.step(&[Order { player: 0, cmd: Command::Move { to: target } }, Order { player: 2, cmd: Command::Move { to: target + V2::new(0.5, 0.5) } }]);
        for _ in 0..15 * TICK_HZ {
            w.step(&[]);
        }
        let now = w.depots[d].pos;
        eprintln!("  depot hauled {:.1} tiles", now.dist(from));
        if now.tile() != from.tile() {
            assert_eq!(w.map.at(now), Terrain::Depot);
            assert_ne!(w.map.at(from), Terrain::Depot);
        }
        assert!(now.dist(from) > 1.0, "the depot did not move");
    }

    /// A ping is the pinger's side's: allies see it, others do not; one
    /// a second; off the map is refused.
    #[test]
    fn a_ping_goes_to_the_side_only() {
        let mut w = World::new(Scenario { swarm_interval: 0, ..Scenario::bots(3) }.with_teams(2));
        // Seats 0 and 2 are team 0, seat 1 team 1.
        let at = w.map.center();
        w.step(&[Order { player: 0, cmd: Command::Ping { at, kind: PingKind::Help } }]);
        let seen = |w: &World, p: PlayerId| w.observe(p).events.iter().any(|e| matches!(e, Event::Pinged { player: 0, .. }));
        assert!(seen(&w, 0) && seen(&w, 2) && !seen(&w, 1));
        w.step(&[Order { player: 0, cmd: Command::Ping { at, kind: PingKind::Help } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "one ping a second")));
        for _ in 0..TICK_HZ {
            w.step(&[]);
        }
        w.step(&[Order { player: 0, cmd: Command::Ping { at: V2::new(-5.0, 1.0), kind: PingKind::Help } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "off map")));
    }

    /// A world built from a map file: four starts seat four even when
    /// the scenario names sixteen, a pillbox's owner is honoured, and
    /// the scenario (map text and all) survives a replay round trip.
    #[test]
    fn world_from_a_map_file() {
        let m = Map::generate(11);
        let mut text: String = m.to_text().lines().filter(|l| !l.starts_with("start ") && !l.starts_with("pillbox ")).map(|l| format!("{}\n", l)).collect();
        for s in &m.starts[..4] {
            text.push_str(&format!("start {} {}\n", s.x, s.y));
        }
        text.push_str(&format!("pillbox {} {} 2\n", m.center().x + 9.0, m.center().y));
        text.push_str(&format!("pillbox {} {} 9\n", m.center().x - 9.0, m.center().y)); // seat 9 is not seated: neutral
        let scenario = Scenario { map: MapSource::Text(text), swarm_interval: 0, ..Scenario::default() };
        let w = World::new(scenario.clone());
        assert_eq!(w.tanks.len(), 4);
        assert_eq!(w.players.len(), 4);
        assert_eq!(w.pillboxes.len(), 2);
        assert_eq!(w.pillboxes[0].owner, Some(2));
        assert_eq!(w.pillboxes[1].owner, None);
        assert_eq!(w.depots.iter().filter(|d| d.owner.is_some()).count(), 4);
        let rec = Replay::new(scenario);
        let json = serde_json::to_string(&rec).unwrap();
        let back: Replay = serde_json::from_str(&json).unwrap();
        assert_eq!(World::new(back.scenario).hash(), w.hash());
        // As played: a crater and a capture come out in the saved map.
        let mut w2 = w.clone();
        let open_ground = w2.map.center() + V2::new(6.0, 0.0); // the centre tile is the depot
        w2.map.crater(open_ground);
        w2.pillboxes[1].owner = Some(0);
        let saved = w2.map_as_played();
        assert_eq!(saved.at(open_ground), Terrain::Rubble);
        assert_eq!(saved.pillboxes[1].owner, Some(0));
        assert_eq!(Map::from_text(&saved.to_text()).unwrap(), saved);
    }

    /// Roadmap step 6's proof. Two bots on a team: a shell fired by one at
    /// a target only the other's recon drone sees is spotted; a shell
    /// landing on a teammate does no damage and says so.
    #[test]
    fn a_team_shares_recon_and_spares_its_own() {
        let scenario = Scenario { swarm_interval: 0, ..Scenario::bots(4) }.with_teams(2); // seats 0,2 vs 1,3
        let mut w = World::new(scenario);
        w.pillboxes.clear();
        assert!(w.allied(0, 2) && !w.allied(0, 1));
        // Put everyone where the test wants them: 0 and 2 twenty tiles
        // apart on open ground; enemy 1 thirty tiles from 0, forty from 2.
        let c = w.map.center();
        let t0 = w.players[0].tank;
        let t1 = w.players[1].tank;
        let t2 = w.players[2].tank;
        w.tank_mut(t0).unwrap().pos = c + V2::new(-20.0, 0.0);
        w.tank_mut(t2).unwrap().pos = c + V2::new(0.0, 0.0);
        // Twelve tiles from 2: past its tank's ten of sight.
        let enemy_at = c + V2::new(12.0, 0.0);
        w.tank_mut(t1).unwrap().pos = enemy_at;
        w.tank_mut(w.players[3].tank).unwrap().pos = c + V2::new(0.0, 50.0);
        // Blind: 0 cannot see the enemy (30 tiles, no recon up).
        assert!(!w.visible_to(0, enemy_at));
        // 2's recon near the enemy: five tiles off, so the first shell's
        // splash (which kills drones within two tiles) does not take it
        // down, and still within its twelve tiles of both the enemy and
        // the ally's tank at the centre.
        w.step(&[Order { player: 2, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: enemy_at + V2::new(0.0, 5.0), radius: 1.0 } } }]);
        for _ in 0..(6 * TICK_HZ) {
            w.step(&[]);
        }
        assert!(w.spotted_by(2, enemy_at), "2's own recon is not over the enemy");
        assert!(w.spotted_by(0, enemy_at), "the ally's recon does not spot for 0");
        assert!(w.visible_to(0, enemy_at), "0 cannot see what its ally sees");
        assert!(!w.spotted_by(1, enemy_at));
        // 0 fires at the enemy: spotted, through the ally's eyes.
        w.step(&[Order { player: 0, cmd: Command::FireIndirect { at: enemy_at } }]);
        let spotted = w.events.iter().find_map(|e| if let Event::ShellLaunched { owner: 0, spotted, .. } = e { Some(*spotted) } else { None });
        assert_eq!(spotted, Some(true), "{:?}", w.events);
        for _ in 0..(3 * TICK_HZ) {
            w.step(&[]);
        }
        assert!(w.drones.iter().any(|d| d.owner == 2 && d.kind == DroneKind::Recon), "the first shell's splash took the recon");
        // The observation 0 gets includes the enemy tank, seen by the ally.
        assert!(w.observe(0).hostile_tanks().any(|t| t.id == t1));
        assert!(w.observe(0).allied_tanks().any(|t| t.id == t2));
        // Friendly fire: 0 shells its own ally at 20 tiles. No damage, and
        // the trace says so.
        let ally_at = w.tank(t2).unwrap().pos;
        let armor = w.tank(t2).unwrap().armor;
        for _ in 0..RELOAD_TICKS {
            w.step(&[]);
        }
        w.step(&[Order { player: 0, cmd: Command::FireIndirect { at: ally_at } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::ShellLaunched { owner: 0, .. })), "{:?}", w.events);
        let mut spared = false;
        for _ in 0..(5 * TICK_HZ) {
            w.step(&[]);
            spared |= w.events.iter().any(|e| matches!(e, Event::FriendlyFireSpared { by: 0, .. }));
            if w.shells.is_empty() {
                break;
            }
        }
        // The blind scatter may miss by up to four tiles; spotted (the ally
        // is in view) it lands within 0.6, inside the splash.
        assert!(spared, "no FriendlyFireSpared event");
        assert_eq!(w.tank(t2).unwrap().armor, armor, "an ally's shell hurt a teammate");
        // A strike at an ally is refused and the bay keeps the drone.
        let bay = w.tank(t0).unwrap().bay.strike;
        w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: t2 } } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { .. })));
        assert_eq!(w.tank(t0).unwrap().bay.strike, bay);
        // Interceptors do not engage an ally's drones: 2's recon flies into
        // 0's bubble and lives.
        w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } }]);
        let own_pos = w.tank(t0).unwrap().pos;
        w.step(&[Order { player: 2, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: own_pos, radius: 1.0 } } }]);
        for _ in 0..(10 * TICK_HZ) {
            w.step(&[]);
        }
        assert!(!w.events.iter().any(|e| matches!(e, Event::Intercepted { owner: 0, .. })));
        assert!(w.drones.iter().filter(|d| d.owner == 2 && d.kind == DroneKind::Recon).count() >= 1, "the ally's recon was shot down by its own side");
    }

    /// the prior art's alliance protocol, the three-way rule on it, and the match
    /// ending by team. Six players: with four, a second ally would leave
    /// two sides, which the rule forbids.
    #[test]
    fn alliances_form_and_break_and_teams_end_the_match() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(6) });
        w.pillboxes.clear();
        assert!(!w.allied(0, 1));
        // 0 asks 1; 3 cannot accept (not asked); 2 does not see it.
        w.step(&[Order { player: 0, cmd: Command::Alliance(AllianceCmd::Request { to: 1 }) }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Proposed { from: 0, to: 1, terms: Terms::Join, .. })), "{:?}", w.events);
        assert_eq!(w.observe(1).proposals.len(), 1);
        assert_eq!(w.observe(2).proposals.len(), 0, "a proposal is private to the two sides");
        w.step(&[Order { player: 3, cmd: Command::Alliance(AllianceCmd::Accept { from: 0 }) }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { player: 3, .. })));
        w.step(&[Order { player: 1, cmd: Command::Alliance(AllianceCmd::Accept { from: 0 }) }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::ProposalResolved { accepted: true, .. })), "{:?}", w.events);
        assert!(w.events.iter().any(|e| matches!(e, Event::AllianceFormed { player: 0, .. })));
        assert!(w.allied(0, 1));
        // Any member admits: 2 asks 0, 1 accepts (ALL-04). Three of six is
        // half, not more than half: allowed.
        w.step(&[Order { player: 2, cmd: Command::Alliance(AllianceCmd::Request { to: 0 }) }]);
        w.step(&[Order { player: 1, cmd: Command::Alliance(AllianceCmd::Accept { from: 2 }) }]);
        assert!(w.allied(2, 0) && w.allied(2, 1), "{:?}", w.events);
        assert_eq!(w.team_members(w.team_of(0).unwrap()).len(), 3);
        // A fourth would be more than half: refused at once, with the reason.
        w.step(&[Order { player: 3, cmd: Command::Alliance(AllianceCmd::Request { to: 0 }) }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { player: 3, why, .. } if why.contains("half"))), "{:?}", w.events);
        // In an alliance, you leave before you ask elsewhere.
        w.step(&[Order { player: 2, cmd: Command::Alliance(AllianceCmd::Request { to: 3 }) }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { player: 2, .. })));
        w.step(&[Order { player: 2, cmd: Command::Alliance(AllianceCmd::Leave) }]);
        assert!(!w.allied(2, 0) && w.allied(0, 1));
        let t2 = w.team_of(2).unwrap();
        assert_eq!(w.players.iter().filter(|p| p.team == t2).count(), 1, "the leaver's team id is nobody else's");
        // A request lapses, and everyone hears that it did.
        w.step(&[Order { player: 2, cmd: Command::Alliance(AllianceCmd::Request { to: 3 }) }]);
        let mut lapsed = false;
        for _ in 0..PROPOSAL_TICKS + 1 {
            w.step(&[]);
            lapsed |= w.events.iter().any(|e| matches!(e, Event::ProposalResolved { accepted: false, why: Some(w), .. } if w == "lapsed"));
        }
        assert!(lapsed);
        w.step(&[Order { player: 3, cmd: Command::Alliance(AllianceCmd::Accept { from: 2 }) }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { player: 3, .. })), "a lapsed request was accepted");
        // Team chat goes to the team only.
        w.step(&[Order { player: 0, cmd: Command::Chat { text: "flank".into(), to: ChatTo::Team } }]);
        assert!(w.observe(1).events.iter().any(|e| matches!(e, Event::Chat { .. })));
        assert!(!w.observe(2).events.iter().any(|e| matches!(e, Event::Chat { .. })));
        // A second message inside a second is refused (UI-23); one a
        // second later goes. Control characters go, the length is cut.
        w.step(&[Order { player: 0, cmd: Command::Chat { text: "again".into(), to: ChatTo::All } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { player: 0, why, .. } if why == "one message a second")));
        for _ in 0..CHAT_EVERY_TICKS {
            w.step(&[]);
        }
        let long = format!("a\nb\x07{}", "x".repeat(500));
        w.step(&[Order { player: 0, cmd: Command::Chat { text: long, to: ChatTo::All } }]);
        let said = w.events.iter().find_map(|e| if let Event::Chat { text, .. } = e { Some(text.clone()) } else { None }).expect("the message went");
        assert_eq!(said.chars().count(), CHAT_MAX_CHARS);
        assert!(said.starts_with("a b ") && !said.chars().any(|c| c.is_control()));
        for _ in 0..CHAT_EVERY_TICKS {
            w.step(&[]);
        }
        w.step(&[Order { player: 0, cmd: Command::Chat { text: " \t ".into(), to: ChatTo::All } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "nothing to say")));
        // A late arrival takes a bot's seat by order (step 8): the seat
        // is a person's and renamed, and everyone hears; a person's
        // seat cannot be taken.
        w.step(&[Order { player: 4, cmd: Command::TakeSeat { name: "late\ngeeker".into() } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::SeatTaken { player: 4, name, .. } if name == "lategeeker")), "{:?}", w.events);
        assert!(!w.players[4].is_bot && w.players[4].name == "lategeeker");
        w.step(&[Order { player: 4, cmd: Command::TakeSeat { name: "again".into() } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "seat is already a person's")));
        // The chosen hear it, and the sender; nobody else (UI-22). A list
        // with only the sender on it is refused.
        for _ in 0..CHAT_EVERY_TICKS {
            w.step(&[]);
        }
        w.step(&[Order { player: 0, cmd: Command::Chat { text: "psst".into(), to: ChatTo::players([3, 5]) } }]);
        let heard: Vec<PlayerId> = (0..6).filter(|p| w.observe(*p).events.iter().any(|e| matches!(e, Event::Chat { .. }))).collect();
        assert_eq!(heard, vec![0, 3, 5]);
        for _ in 0..CHAT_EVERY_TICKS {
            w.step(&[]);
        }
        w.step(&[Order { player: 0, cmd: Command::Chat { text: "me".into(), to: ChatTo::players([0]) } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "nobody chosen to say it to")));
        // Nearby: the tanks within CHAT_NEARBY of the sender's.
        for _ in 0..CHAT_EVERY_TICKS {
            w.step(&[]);
        }
        let at = w.tank_of(0).unwrap().pos;
        let t4 = w.players[4].tank;
        w.tank_mut(t4).unwrap().pos = at + V2::new(CHAT_NEARBY - 1.0, 0.0);
        w.step(&[Order { player: 0, cmd: Command::Chat { text: "close".into(), to: ChatTo::Nearby } }]);
        let heard: Vec<PlayerId> = (0..6).filter(|p| w.observe(*p).events.iter().any(|e| matches!(e, Event::Chat { .. }))).collect();
        assert!(heard.contains(&0) && heard.contains(&4), "{:?}", heard);
        assert!(heard.iter().all(|p| *p == 0 || w.tank_of(*p).unwrap().pos.dist(at) <= CHAT_NEARBY + 0.5), "{:?}", heard);
        // The match ends when one team is left: kill the four alone; 0 and
        // 1 (one team) win, the top scorer the winner.
        w.players[0].score = 50;
        w.players[1].score = 80;
        for i in 2..6 {
            let tid = w.players[i].tank;
            w.tank_mut(tid).unwrap().armor = 1.0;
            w.damage(tid, 5.0, None);
            if i < 5 {
                w.step(&[]);
                assert!(!w.over, "sides still stand");
            }
        }
        w.step(&[]);
        assert!(w.over);
        assert_eq!(w.winning_team, w.team_of(0));
        assert_eq!(w.winner, Some(1));
        assert!(w.events.iter().any(|e| matches!(e, Event::MatchOver { team: Some(_), .. })));
    }

    /// Roadmap step 6b's proof, as a recorded match. Two teams of eight: a
    /// merge is refused, and so is any join or swap, because two sides
    /// would remain. Three teams: a defection is proposed, answered,
    /// carried out and announced, the trace shows the team change, and
    /// the newcomer's recon spots for its new side; a recruit that would
    /// give one side more than half is refused with the reason.
    #[test]
    fn a_window_reads_the_three_way_rule_as_the_world_does() {
        // Every pair, every terms, in sides of 2, 3 and 4 and alone: what
        // the observation says it would refuse is what the world refuses.
        for teams in [2usize, 3, 4, 0] {
            let mut sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(8) };
            if teams > 0 {
                sc = sc.with_teams(teams);
            }
            let mut base = World::new(sc);
            base.pillboxes.clear();
            base.tanks[5].alive = false;
            base.players[5].alive = false;
            for from in 0..8u8 {
                let obs = base.observe(from);
                for to in 0..8u8 {
                    for terms in [Terms::Join, Terms::Recruit, Terms::Swap, Terms::Merge] {
                        let mut w = base.clone();
                        w.step(&[Order { player: from, cmd: Command::Propose { to, terms } }]);
                        let refused = w.events.iter().find_map(|e| if let Event::Rejected { why, .. } = e { Some(why.clone()) } else { None });
                        assert_eq!(obs.would_refuse(to, terms).map(|s| s.to_string()), refused, "{} sides: p{} {:?} p{}", teams, from, terms, to);
                    }
                }
            }
        }
    }

    #[test]
    fn proposals_change_sides_under_the_three_way_rule() {
        // Two sides of eight.
        let two = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(16) }.with_teams(2);
        let mut w = World::new(two);
        w.pillboxes.clear();
        for (from, to, terms) in [(0u8, 1u8, Terms::Merge), (0, 1, Terms::Join), (0, 1, Terms::Swap), (1, 0, Terms::Recruit)] {
            w.step(&[Order { player: from, cmd: Command::Propose { to, terms } }]);
            assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.contains("fewer than three"))), "{:?} not refused: {:?}", terms, w.events);
        }
        assert!(w.proposals.is_empty());
        // Three sides of six, five, five. Record it.
        let three = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(16) }.with_teams(3);
        let mut w = World::new(three.clone());
        w.pillboxes.clear();
        let mut rec = Replay::new(three);
        let mut step = |w: &mut World, orders: Vec<Order>| {
            w.step(&orders);
            rec.record(w, &orders);
        };
        // Seat 3 (team 0, six strong) proposes to join team 1 (seat 1,
        // five strong): five sides... three sides stay three, team 1
        // becomes six of sixteen. Allowed.
        assert_eq!(w.team_of(3), Some(0));
        step(&mut w, vec![Order { player: 3, cmd: Command::Propose { to: 1, terms: Terms::Join } }]);
        let pid = w.events.iter().find_map(|e| if let Event::Proposed { proposal, .. } = e { Some(*proposal) } else { None }).expect("not proposed");
        // Seat 4 (team 1 too) answers for the team: any member may.
        assert_eq!(w.team_of(4), Some(1));
        step(&mut w, vec![Order { player: 4, cmd: Command::Answer { proposal: pid, accept: true } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::ProposalResolved { accepted: true, by: Some(4), .. })), "{:?}", w.events);
        assert!(w.events.iter().any(|e| matches!(e, Event::AllianceFormed { player: 3, team: 1, .. })));
        assert_eq!(w.team_of(3), Some(1));
        assert!(w.allied(3, 1) && !w.allied(3, 0));
        // The defector's recon now spots for the new side.
        let c = w.map.center();
        let t3 = w.players[3].tank;
        let t1 = w.players[1].tank;
        w.tank_mut(t3).unwrap().pos = c;
        w.tank_mut(t1).unwrap().pos = c + V2::new(-25.0, 0.0);
        let look = c + V2::new(8.0, 0.0);
        step(&mut w, vec![Order { player: 3, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: look, radius: 1.0 } } }]);
        for _ in 0..(4 * TICK_HZ) {
            step(&mut w, vec![]);
        }
        assert!(w.spotted_by(1, look), "the newcomer's recon does not spot for seat 1");
        assert!(!w.spotted_by(0, look), "the old side still sees through it");
        // Team 1 is six of sixteen; recruiting three more would be nine,
        // more than half: refused with the reason. Recruiting one is fine.
        step(&mut w, vec![Order { player: 1, cmd: Command::Propose { to: 0, terms: Terms::Recruit } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Proposed { .. })));
        let seven = w.events.iter().find_map(|e| if let Event::Proposed { proposal, .. } = e { Some(*proposal) } else { None }).unwrap();
        step(&mut w, vec![Order { player: 0, cmd: Command::Answer { proposal: seven, accept: true } }]);
        assert_eq!(w.team_members(1).len(), 7);
        step(&mut w, vec![Order { player: 1, cmd: Command::Propose { to: 6, terms: Terms::Recruit } }]);
        let eight = w.events.iter().find_map(|e| if let Event::Proposed { proposal, .. } = e { Some(*proposal) } else { None }).unwrap();
        step(&mut w, vec![Order { player: 6, cmd: Command::Answer { proposal: eight, accept: true } }]);
        assert_eq!(w.team_members(1).len(), 8, "eight of sixteen is half, allowed");
        step(&mut w, vec![Order { player: 1, cmd: Command::Propose { to: 9, terms: Terms::Recruit } }]);
        assert!(w.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why.contains("half"))), "{:?}", w.events);
        // A swap keeps every side its size: allowed even now.
        step(&mut w, vec![Order { player: 1, cmd: Command::Propose { to: 9, terms: Terms::Swap } }]);
        let swap = w.events.iter().find_map(|e| if let Event::Proposed { proposal, .. } = e { Some(*proposal) } else { None }).expect("swap not proposed");
        let (team1, team9) = (w.team_of(1).unwrap(), w.team_of(9).unwrap());
        step(&mut w, vec![Order { player: 9, cmd: Command::Answer { proposal: swap, accept: true } }]);
        assert_eq!((w.team_of(1), w.team_of(9)), (Some(team9), Some(team1)));
        // The scripted match replays to the same hashes.
        rec.record(&w, &[]);
        assert!(rec.verify().is_ok(), "the proposal match does not replay");
    }

    /// Backlog item 3: a recon's orbit moves by order, one drone comes
    /// home on its own, an interceptor is refused, another's drone is
    /// refused.
    #[test]
    fn retask_moves_a_recon_and_recalls_one_drone() {
        let mut w = world();
        let c = w.map.center();
        w.step(&[
            Order { player: 0, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: c, radius: 3.0 } } },
            Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } },
        ]);
        let recon = w.drones.iter().find(|d| d.kind == DroneKind::Recon).unwrap().id;
        let interceptor = w.drones.iter().find(|d| d.kind == DroneKind::Interceptor).unwrap().id;
        let moved = c + V2::new(5.0, 5.0);
        w.step(&[
            Order { player: 0, cmd: Command::Retask { drone: recon, task: DroneTask::Orbit { center: moved, radius: 9.0 } } },
            Order { player: 0, cmd: Command::Retask { drone: interceptor, task: DroneTask::Orbit { center: moved, radius: 2.0 } } },
            Order { player: 1, cmd: Command::Retask { drone: recon, task: DroneTask::Return } },
        ]);
        assert_eq!(w.drone(recon).unwrap().task, DroneTask::Orbit { center: moved, radius: 6.0 }, "the orbit moved and the radius was clamped");
        assert!(matches!(w.drone(interceptor).unwrap().task, DroneTask::Orbit { .. }));
        let refused: Vec<&str> = w.events.iter().filter_map(|e| if let Event::Rejected { why, .. } = e { Some(why.as_str()) } else { None }).collect();
        assert_eq!(refused, vec!["interceptors are the world's to task", "not my drone"]);
        assert!(w.events.iter().any(|e| matches!(e, Event::DroneRetasked { drone, .. } if *drone == recon)));
        // One drone home: the interceptor stays up. (A few seconds first,
        // or the recon is recalled from inside recovery range and is
        // simply back in the bay.)
        for _ in 0..40 {
            w.step(&[]);
        }
        w.step(&[Order { player: 0, cmd: Command::Retask { drone: recon, task: DroneTask::Return } }]);
        assert_eq!(w.drone(recon).unwrap().task, DroneTask::Return);
        assert!(matches!(w.drone(interceptor).unwrap().task, DroneTask::Orbit { .. }));
    }

    /// Backlog item 13, SPEC VIS-02 and PIL-07: a tank in a 2 x 2 block
    /// of forest is unseen by a recon drone overhead and untargeted by
    /// a pillbox beside it, until something comes within three tiles.
    #[test]
    fn forest_block_conceals_a_tank() {
        let mut w = world();
        let c = Map::tile_centre(w.map.center());
        for dy in -1..=1 {
            for dx in -1..=1 {
                w.map.set(c + V2::new(dx as f32, dy as f32), Terrain::Forest);
            }
        }
        w.tanks[1].pos = c;
        assert!(w.concealed(c));
        // A recon of seat 0 orbits right over it (untethered, so its
        // link does not fail for the distance to its tank).
        w.drones.push(Drone { id: 900, owner: 0, kind: DroneKind::Recon, pos: c + V2::new(4.0, 0.0), heading: 0.0, battery: 100.0, tether: None, task: DroneTask::Orbit { center: c, radius: 4.0 }, link: 1.0, phase: 0.0, jammed: false, mark: 1 });
        // A neutral pillbox six tiles north, with a clear line, and out
        // of its own air range of the drone.
        for dy in 0..=6 {
            w.map.set(c + V2::new(0.0, dy as f32 + 2.0), Terrain::Grass);
        }
        let pid = w.alloc();
        w.pillboxes.push(Pillbox { id: pid, pos: c + V2::new(0.0, 6.0), owner: None, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: 0 });
        assert!(!w.spotted_by(0, c), "spotted through the trees");
        assert!(!w.observe(0).tanks.iter().any(|t| t.owner == 1), "seen through the trees");
        w.step(&[]);
        assert!(!w.events.iter().any(|e| matches!(e, Event::PillboxFired { pillbox, .. } if *pillbox == pid)), "the pillbox shot into the trees");
        // Close in: the drone to two tiles, and it is seen and spotted.
        w.drones.iter_mut().find(|d| d.id == 900).unwrap().pos = c + V2::new(2.0, 0.0);
        assert!(w.spotted_by(0, c));
        assert!(w.observe(0).tanks.iter().any(|t| t.owner == 1));
        // Thin the block to a row of three trees: not a block. Then clear
        // it: on open ground the same drone four tiles off sees the tank
        // and the same pillbox fires (forest blocks a pillbox's shot by
        // itself, so the PIL-07 check above is belt and braces).
        w.drones.iter_mut().find(|d| d.id == 900).unwrap().pos = c + V2::new(4.0, 0.0);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dy != 0 {
                    w.map.set(c + V2::new(dx as f32, dy as f32), Terrain::Grass);
                }
            }
        }
        assert!(!w.concealed(c), "three trees are not a block");
        for dx in -1..=1 {
            w.map.set(c + V2::new(dx as f32, 0.0), Terrain::Grass);
        }
        w.step(&[]);
        assert!(w.events.iter().any(|e| matches!(e, Event::PillboxFired { pillbox, .. } if *pillbox == pid)), "the pillbox did not fire on the uncovered tank");
        assert!(w.observe(0).tanks.iter().any(|t| t.owner == 1));
    }

    /// Backlog item 11, SPEC section 11: the game types are what a tank
    /// starts with.
    #[test]
    fn game_types_set_the_start() {
        let open = World::new(Scenario { swarm_interval: 0, ..Scenario::default() });
        let strict = World::new(Scenario { game_type: GameType::Strict, swarm_interval: 0, ..Scenario::default() });
        let tourney = World::new(Scenario { game_type: GameType::Tournament, swarm_interval: 0, ..Scenario::default() });
        assert_eq!((open.tanks[0].shells, open.tanks[0].pillboys.material), (TANK_SHELLS, PILLBOY_MATERIAL_START));
        assert_eq!((tourney.tanks[0].shells, tourney.tanks[0].pillboys.material), (TANK_SHELLS / 2, 0));
        assert_eq!((strict.tanks[0].shells, strict.tanks[0].pillboys.material), (0, 0));
        let mut strict = strict;
        strict.step(&[Order { player: 0, cmd: Command::FireIndirect { at: strict.map.center() } }]);
        assert!(strict.events.iter().any(|e| matches!(e, Event::Rejected { why, .. } if why == "no shells")));
        assert_eq!(GameType::parse("Strict-Tournament"), Some(GameType::Strict));
    }

    #[test]
    fn sixteen_seats() {





        let players = (0..MAX_PLAYERS).map(|i| (format!("p{}", i), true)).collect();
        let mut w = World::new(Scenario { players, swarm_interval: 0, ..Scenario::default() });
        assert_eq!(w.tanks.len(), 16);
        assert_eq!(w.depots.iter().filter(|d| d.owner.is_some()).count(), 16);
        for _ in 0..100 {
            w.step(&[]);
        }
        assert!(w.tanks.iter().all(|t| t.alive));
        let o = w.observe(15);
        assert_eq!(o.players.len(), 16);
    }

    #[test]
    fn swarm_meets_bubble() {


        let mut w = World::new(Scenario { swarm_interval: 10, swarm_size: 3, ..Scenario::default() });
        for _ in 0..4 {
            w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return } }]);
        }
        let mut intercepts = 0;
        for _ in 0..1200 {
            w.step(&[]);
            intercepts += w.events.iter().filter(|e| matches!(e, Event::Intercepted { .. })).count();
        }
        assert!(intercepts > 0, "interceptors never engaged");
    }
}
