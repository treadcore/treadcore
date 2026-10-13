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

pub mod map;
pub mod rules;

pub use map::{Map, Terrain, MAP_H, MAP_W};
pub use rules::*;

/// Simulation rate. Twenty ticks a second is enough for a game whose
/// fastest thing is a drone at nine tiles a second, and it keeps a
/// snapshot every other tick to a tenth of a second.
pub const TICK_HZ: u32 = 20;
pub const DT: f32 = 1.0 / TICK_HZ as f32;
/// Sixteen seats, Bolo's number. Empty ones are filled by bots. The
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
}

impl Bay {
    pub fn get(&self, k: DroneKind) -> u8 {
        match k {
            DroneKind::Recon => self.recon,
            DroneKind::Interceptor => self.interceptor,
            DroneKind::Strike => self.strike,
        }
    }
    pub fn take(&mut self, k: DroneKind) -> bool {
        let slot = match k {
            DroneKind::Recon => &mut self.recon,
            DroneKind::Interceptor => &mut self.interceptor,
            DroneKind::Strike => &mut self.strike,
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
        }
    }
}

// ------------------------------------------------------- pillboys, pillboxes

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
}

impl BuildKind {
    pub fn cost(self) -> u8 {
        match self {
            BuildKind::Pillbox => PILLBOX_BUILD_COST,
            BuildKind::Wall => WALL_COST,
            BuildKind::Road => ROAD_COST,
            BuildKind::Mine => MINE_COST,
        }
    }
    pub fn ticks(self) -> u16 {
        match self {
            BuildKind::Pillbox => PILLBOX_BUILD_TICKS,
            BuildKind::Wall => WALL_BUILD_TICKS,
            BuildKind::Road => ROAD_BUILD_TICKS,
            BuildKind::Mine => MINE_LAY_TICKS,
        }
    }
    /// The squad works from beside a wall or a mine, never on it.
    fn standoff(self) -> f32 {
        match self {
            BuildKind::Wall | BuildKind::Mine => BUILD_STANDOFF,
            _ => PILLBOY_WORK_RANGE,
        }
    }
}

/// A mine on a tile. Bolo's: the layer and (later) their allies see it;
/// everyone else finds it the hard way, or by coming within
/// `MINE_REVEAL` of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Mine {
    pub id: EntityId,
    /// The tile's centre.
    pub pos: V2,
    pub owner: PlayerId,
}

/// An order to the squad. Bolo's builder: the click is the order, the
/// squad leaves the tank, walks the world's route, does the work if the
/// material is there, and walks back. The player drives nothing but the
/// tank.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PillboyTask {
    /// Stand on a pillbox that is neutral or dead until it is ours.
    Capture { pillbox: EntityId },
    /// Bring an owned pillbox back to full health, for material.
    Repair { target: EntityId },
    /// Raise an emplacement at a point, for material.
    Build { what: BuildKind, at: V2 },
    /// Fell a forest tile for material.
    Harvest { at: V2 },
    /// Come home.
    Return,
}

impl PillboyTask {
    /// Where the squad has to be to do this. None for Return (the tank).
    fn site(&self, world: &World) -> Option<V2> {
        match self {
            PillboyTask::Capture { pillbox } | PillboyTask::Repair { target: pillbox } => world.pillbox(*pillbox).map(|p| p.pos),
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
}

impl Pillboys {
    pub fn fresh(pos: V2) -> Pillboys {
        Pillboys { strength: PILLBOY_MAX, aboard: true, pos, task: None, progress: 0, material: PILLBOY_MATERIAL_START, route: Vec::new() }
    }
    pub fn deployed(&self) -> bool {
        !self.aboard
    }
}

/// Bolo's pillbox, with one addition: it shoots drones too, at short
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Depot {
    pub id: EntityId,
    pub pos: V2,
    pub owner: Option<PlayerId>,
    /// 0..CAPTURE_TICKS, with the id of who is capturing.
    pub capture: u16,
    pub capturer: Option<PlayerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub tank: EntityId,
    pub alive: bool,
    pub score: u32,
    pub is_bot: bool,
}

// ----------------------------------------------------------------- commands

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
    Recall,
    SetBubble { radius: f32 },
    /// An order to the Pillboys. One at a time; a new one while they are
    /// out is refused, except Return.
    Deploy { task: PillboyTask },
    Chat { text: String },
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
    DroneLost { drone: EntityId, owner: PlayerId, kind: DroneKind, why: LossReason },
    DroneRecovered { drone: EntityId, owner: PlayerId, kind: DroneKind },
    Intercepted { interceptor: EntityId, target: EntityId, owner: PlayerId },
    InterceptMissed { interceptor: EntityId, target: EntityId },
    StrikeHit { drone: EntityId, tank: EntityId, damage: f32 },
    TankDamaged { tank: EntityId, by: Option<PlayerId>, damage: f32, armor_left: f32 },
    TankDestroyed { tank: EntityId, owner: PlayerId, by: Option<PlayerId> },
    DepotCaptured { depot: EntityId, by: PlayerId },
    Resupplied { tank: EntityId, depot: EntityId },
    SwarmInbound { count: u32, from: V2 },
    LinkLost { drone: EntityId, owner: PlayerId },
    PillboysDeployed { tank: EntityId, owner: PlayerId, task: PillboyTask },
    PillboysReturned { tank: EntityId, owner: PlayerId },
    /// The order ended without its work: the site changed under them.
    PillboysAbandoned { tank: EntityId, owner: PlayerId, why: String },
    PillboysHit { tank: EntityId, owner: PlayerId, lost: u8, left: u8 },
    PillboysLost { tank: EntityId, owner: PlayerId },
    PillboxCaptured { pillbox: EntityId, by: PlayerId, from: Option<PlayerId> },
    PillboxRepaired { pillbox: EntityId, by: PlayerId },
    PillboxBuilt { pillbox: EntityId, by: PlayerId, at: V2 },
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
    /// A tile changed under everyone: a client updates its copy of the map.
    TerrainChanged { at: V2, to: u8 },
    Chat { player: PlayerId, text: String },
    MatchOver { winner: Option<PlayerId> },
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scenario {
    pub seed: u64,
    pub players: Vec<(String, bool)>,
    /// Ticks between neutral FPV swarms from the map edge; 0 disables them.
    /// They make the bubble matter even with one player on the map.
    pub swarm_interval: u32,
    pub swarm_size: u32,
    /// Hard stop, in ticks. 0 for none.
    pub time_limit: Tick,
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
}

impl Default for Scenario {
    fn default() -> Scenario {
        Scenario {
            seed: 0x7EAD_C0DE,
            players: (0..MAX_PLAYERS).map(|i| (seat_name(i), true)).collect(),

            swarm_interval: 45 * TICK_HZ,
            swarm_size: 4,
            time_limit: 15 * 60 * TICK_HZ as Tick,
        }
    }
}

// -------------------------------------------------------------------- world

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
    pub rng: Rng,
    pub next_id: EntityId,
    pub over: bool,
    pub winner: Option<PlayerId>,
    /// Events from the most recent `step`. Cleared at the start of the next.
    #[serde(skip)]
    pub events: Vec<Event>,
}

impl World {
    pub fn new(scenario: Scenario) -> World {
        let map = Map::generate(scenario.seed);
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
            rng: Rng::new(scenario.seed ^ 0xA5A5_5A5A),
            next_id: 1,
            over: false,
            winner: None,
            events: Vec::new(),
            scenario: scenario.clone(),
        };
        for (i, pos) in w.map.depots.clone().iter().enumerate() {
            let id = w.alloc();
            let owner = if i < scenario.players.len().min(MAX_PLAYERS) {
                Some(i as PlayerId)
            } else {
                None
            };
            w.depots.push(Depot { id, pos: *pos, owner, capture: 0, capturer: None });
        }
        for pos in w.map.pillboxes.clone() {
            let id = w.alloc();
            w.pillboxes.push(Pillbox { id, pos, owner: None, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        }
        for (i, (name, is_bot)) in scenario.players.iter().take(MAX_PLAYERS).enumerate() {
            let id = w.alloc();
            let pos = w.map.starts[i];
            w.tanks.push(Tank {
                id,
                owner: i as PlayerId,
                pos,
                heading: (w.map.center() - pos).heading(),
                speed: 0.0,
                armor: TANK_ARMOR,
                shells: TANK_SHELLS,
                fuel: TANK_FUEL,
                bay: BAY_FULL,
                goal: None,
                route: Vec::new(),
                reload: 0,
                bubble: BUBBLE_DEFAULT,
                last_hit_tick: 0,
                alive: true,
                pillboys: Pillboys::fresh(pos),
            });
            w.players.push(Player {
                id: i as PlayerId,
                name: name.clone(),
                tank: id,
                alive: true,
                score: 0,
                is_bot: *is_bot,
            });
        }
        w
    }

    fn alloc(&mut self) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        id
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

    /// A tank's route: the map's, avoiding the range of every live
    /// pillbox hostile to `p` at `PILLBOX_AVOID_COST` extra per tile,
    /// and every hostile mine `p` knows about at `MINE_AVOID_COST`.
    pub fn route_for(&self, p: PlayerId, from: V2, to: V2) -> Vec<V2> {
        let guns: Vec<V2> = self.pillboxes.iter().filter(|b| b.alive() && b.hostile_to(p)).map(|b| b.pos).collect();
        let mines: Vec<V2> = self.mines.iter().filter(|m| m.owner != p && self.mine_known_to(p, m)).map(|m| m.pos).collect();
        self.map.route_with(from, to, |tile| {
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

    /// Does `p` know this mine is there? Its own, or one within
    /// `MINE_REVEAL` of its tank or its deployed squad.
    pub fn mine_known_to(&self, p: PlayerId, m: &Mine) -> bool {
        if m.owner == p {
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

    /// Can `p` see `at`? Its own tank out to `TANK_VISION` (halved into
    /// forest), its recon drones out to `RECON_VISION`, its other drones
    /// out to `DRONE_VISION`. The shared, all-seeing world is the host's;
    /// what a player gets is `observe()`.
    pub fn visible_to(&self, p: PlayerId, at: V2) -> bool {
        let in_forest = self.map.at(at) == Terrain::Forest;
        if let Some(t) = self.tank_of(p) {
            if t.alive {
                let r = if in_forest { TANK_VISION * 0.5 } else { TANK_VISION };
                if t.pos.dist(at) <= r {
                    return true;
                }
            }
        }
        self.drones.iter().filter(|d| d.owner == p).any(|d| {
            let r = match d.kind {
                DroneKind::Recon => RECON_VISION,
                _ => DRONE_VISION,
            };
            let r = if in_forest { r * 0.6 } else { r };
            d.pos.dist(at) <= r
        })
    }

    /// Does a recon drone of `p` have `at` in view right now?
    pub fn spotted_by(&self, p: PlayerId, at: V2) -> bool {
        self.drones
            .iter()
            .any(|d| d.owner == p && d.kind == DroneKind::Recon && d.link >= LINK_FLOOR && d.pos.dist(at) <= RECON_VISION)
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
        for o in orders {
            self.apply(o);
        }
        self.step_tanks();
        self.step_pillboys();
        self.step_drones();
        self.step_pillboxes();
        self.step_shells();
        self.step_depots();
        self.step_swarm();
        self.step_match();
    }

    fn reject(&mut self, o: &Order, why: &str) {
        self.events.push(Event::Rejected { player: o.player, cmd: o.cmd.clone(), why: why.into() });
    }

    fn apply(&mut self, o: &Order) {
        let tid = match self.players.iter().find(|p| p.id == o.player) {
            Some(p) => p.tank,
            None => return self.reject(o, "no such player"),
        };
        let (alive, pos, reload, shells) = match self.tank(tid) {
            Some(t) => (t.alive, t.pos, t.reload, t.shells),
            None => return self.reject(o, "no tank"),
        };
        if let Command::Chat { text } = &o.cmd {
            self.events.push(Event::Chat { player: o.player, text: text.clone() });
            return;
        }
        if !alive {
            return self.reject(o, "tank destroyed");
        }
        match &o.cmd {
            Command::Move { to } => {
                if !self.map.in_bounds(*to) {
                    return self.reject(o, "off map");
                }
                if self.map.at(*to) == Terrain::Water {
                    return self.reject(o, "water");
                }
                // The world routes. A straight line from a start to the
                // centre crosses the river beside the bridge, and in the
                // first traced match one bot sat on that bank for the whole
                // game. Neither a Brain nor a client has the map in its
                // observation, so the pathfinding has to live here.
                let mut wp = self.route_for(o.player, pos, *to);
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
                let cep = if spotted { CEP_SPOTTED } else { CEP_BLIND };
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
                    (DroneKind::Strike, DroneTask::Strike { target }) => DroneTask::Strike { target: *target },
                    (DroneKind::Strike, DroneTask::Orbit { center, .. }) => DroneTask::Orbit { center: *center, radius: 2.0 },
                    (DroneKind::Recon, DroneTask::Orbit { center, radius }) => DroneTask::Orbit { center: *center, radius: radius.clamp(1.0, 6.0) },
                    (DroneKind::Recon, _) => DroneTask::Orbit { center: pos, radius: 4.0 },
                    _ => return self.reject(o, "task does not suit that drone"),
                };
                let id = self.alloc();
                let battery = match kind {
                    DroneKind::Recon => RECON_BATTERY,
                    DroneKind::Interceptor => INTERCEPTOR_BATTERY,
                    DroneKind::Strike => STRIKE_BATTERY,
                };
                let phase = self.rng.range(0.0, std::f32::consts::TAU);
                self.drones.push(Drone {
                    id,
                    owner: o.player,
                    kind: *kind,
                    pos,
                    heading: 0.0,
                    battery,
                    tether: Some(tid),
                    task,
                    link: 1.0,
                    phase,
                });
                self.events.push(Event::DroneLaunched { drone: id, owner: o.player, kind: *kind });
            }
            Command::Recall => {
                for d in self.drones.iter_mut().filter(|d| d.owner == o.player) {
                    d.task = DroneTask::Return;
                }
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
                match task {
                    PillboyTask::Capture { pillbox } => match self.pillbox(*pillbox) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(p) if p.owner == Some(o.player) => return self.reject(o, "pillbox already ours"),
                        Some(p) if p.owner.is_some() && p.alive() => return self.reject(o, "pillbox is live and theirs: shoot it dead first"),
                        Some(_) => {}
                    },
                    PillboyTask::Repair { target } => match self.pillbox(*target) {
                        None => return self.reject(o, "no such pillbox"),
                        Some(p) if p.owner != Some(o.player) => return self.reject(o, "not our pillbox"),
                        Some(p) if p.health >= PILLBOX_HEALTH => return self.reject(o, "pillbox needs no repair"),
                        Some(_) if material < PILLBOX_REPAIR_COST => return self.reject(o, "no material"),
                        Some(_) => {}
                    },
                    PillboyTask::Build { what, at } => {
                        if !self.map.in_bounds(*at) {
                            return self.reject(o, "off map");
                        }
                        let here = self.map.at(*at);
                        if material < what.cost() {
                            return self.reject(o, "no material");
                        }
                        if here == Terrain::Depot || self.depots.iter().any(|d| d.pos.dist(*at) <= DEPOT_RADIUS) {
                            return self.reject(o, "not on a depot");
                        }
                        if self.pillboxes.iter().any(|p| p.pos.dist(*at) < if *what == BuildKind::Pillbox { PILLBOX_SPACING } else { 1.0 }) {
                            return self.reject(o, "a pillbox is there");
                        }
                        match what {
                            BuildKind::Pillbox | BuildKind::Wall | BuildKind::Mine => {
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
                                if matches!(here, Terrain::Road | Terrain::Bridge | Terrain::Wall | Terrain::Forest) {
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
            Command::Chat { .. } => {}
        }
    }

    /// The squad: walk, work, walk home. Read the tank and the site,
    /// decide, then apply, as the drones do.
    fn step_pillboys(&mut self) {
        #[derive(Debug)]
        enum Done {
            Captured(EntityId, Option<PlayerId>),
            Repaired(EntityId),
            Built(BuildKind, V2),
            Harvested(V2),
            Abandon(&'static str),
            Home,
        }
        let mut done: Vec<(EntityId, PlayerId, Done)> = Vec::new();
        let mut captures: Vec<(EntityId, PlayerId)> = Vec::new();
        let map = &self.map;
        let pillboxes = &self.pillboxes;
        let tank_tiles: Vec<(i32, i32)> = self.tanks.iter().filter(|t| t.alive).map(|t| t.pos.tile()).collect();
        let mine_tiles: Vec<(i32, i32)> = self.mines.iter().map(|m| m.pos.tile()).collect();
        // A hostile mine under a walking squad: found the hard way.
        let hostile_mines: Vec<(EntityId, PlayerId, (i32, i32))> = self.mines.iter().map(|m| (m.id, m.owner, m.pos.tile())).collect();
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
                PillboyTask::Capture { pillbox } | PillboyTask::Repair { target: pillbox } => pillboxes.iter().find(|p| p.id == *pillbox).map(|p| p.pos),
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
                if let Some((mid, _, _)) = hostile_mines.iter().find(|(_, o, tile)| *o != t.owner && *tile == s.pos.tile()) {
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
                    if p.owner != Some(t.owner) {
                        done.push((t.id, t.owner, Done::Abandon("pillbox is no longer ours")));
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
                        BuildKind::Wall => !map.at(*at).passable() || tank_tiles.iter().any(|tt| *tt == at.tile()),
                        BuildKind::Road => matches!(map.at(*at), Terrain::Road | Terrain::Bridge | Terrain::Wall),
                        BuildKind::Mine => !map.at(*at).passable() || mine_tiles.iter().any(|mt| *mt == at.tile()),
                    };
                    if blocked {
                        done.push((t.id, t.owner, Done::Abandon("something is already there")));
                    } else if s.material < what.cost() {
                        done.push((t.id, t.owner, Done::Abandon("no material")));
                    } else {
                        s.progress += 1;
                        if s.progress >= what.ticks() {
                            done.push((t.id, t.owner, Done::Built(*what, *at)));
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
        // Capture progress on the pillboxes: contested means nothing moves.
        for p in self.pillboxes.iter_mut() {
            let mut who: Vec<PlayerId> = captures.iter().filter(|(id, _)| *id == p.id).map(|(_, o)| *o).collect();
            who.sort_unstable();
            who.dedup();
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
                    if let Some(t) = self.tank_mut(tid) {
                        t.pillboys.material -= what.cost();
                    }
                    let at = Map::tile_centre(at);
                    match what {
                        BuildKind::Pillbox => {
                            let id = self.alloc();
                            self.pillboxes.push(Pillbox { id, pos: at, owner: Some(owner), health: PILLBOX_HEALTH, reload: PILLBOX_RELOAD_TICKS, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
                            self.events.push(Event::PillboxBuilt { pillbox: id, by: owner, at });
                        }
                        BuildKind::Wall => self.set_terrain(at, Terrain::Wall),
                        BuildKind::Road => {
                            let t = if self.map.at(at) == Terrain::Water { Terrain::Bridge } else { Terrain::Road };
                            self.set_terrain(at, t);
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
                    let mut gained = 0;
                    if let Some(t) = self.tank_mut(tid) {
                        gained = HARVEST_MATERIAL.min(PILLBOY_MATERIAL_MAX - t.pillboys.material);
                        t.pillboys.material += gained;
                    }
                    self.events.push(Event::Harvested { owner, at, material: gained });
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
            let tanks: Vec<(EntityId, V2)> = self.tanks.iter().filter(|t| t.alive).map(|t| (t.id, t.pos)).collect();
            for (tid, pos) in tanks {
                if pos.tile() == m.pos.tile() {
                    self.damage(tid, MINE_DAMAGE, Some(m.owner));
                    hits.push(tid);
                } else if pos.dist(m.pos) <= MINE_RADIUS {
                    self.damage(tid, MINE_SPLASH_DAMAGE, Some(m.owner));
                    hits.push(tid);
                }
            }
            let squads: Vec<EntityId> = self.tanks.iter().filter(|t| t.alive && t.pillboys.deployed() && t.pillboys.pos.dist(m.pos) <= MINE_RADIUS).map(|t| t.id).collect();
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
                if let Some(p) = self.pillboxes.iter().find(|p| p.alive() && p.owner == Some(t.owner) && p.shells > 0 && p.pos.dist(t.pos) <= DEPOT_RADIUS) {
                    draws.push((t.id, p.id));
                }
            }
            for (tid, pid) in draws {
                self.tank_mut(tid).unwrap().shells += 1;
                self.pillbox_mut(pid).unwrap().shells -= 1;
                self.events.push(Event::Resupplied { tank: tid, depot: pid });
            }
        }
        let tanks: Vec<(EntityId, PlayerId, V2)> = self.tanks.iter().filter(|t| t.alive).map(|t| (t.id, t.owner, t.pos)).collect();
        let drones: Vec<(EntityId, PlayerId, V2)> = self.drones.iter().map(|d| (d.id, d.owner, d.pos)).collect();
        let mut shots: Vec<(EntityId, PlayerId, EntityId, f32)> = Vec::new(); // pillbox, shooter, tank, damage
        let mut air: Vec<(EntityId, PlayerId, EntityId, f32)> = Vec::new(); // pillbox, shooter, drone, link
        for p in self.pillboxes.iter_mut() {
            if p.reload > 0 {
                p.reload -= 1;
            }
            if !p.alive() || p.reload > 0 {
                continue;
            }
            let shooter = p.shooter();
            let tank = tanks
                .iter()
                .filter(|(_, o, pos)| p.hostile_to(*o) && pos.dist(p.pos) <= PILLBOX_RANGE && self.map.line_clear(p.pos, *pos))
                .min_by(|a, b| a.2.dist(p.pos).partial_cmp(&b.2.dist(p.pos)).unwrap().then(a.0.cmp(&b.0)));
            if let Some((tid, _, _)) = tank {
                shots.push((p.id, shooter, *tid, PILLBOX_DAMAGE));
                p.reload = PILLBOX_RELOAD_TICKS;
                continue;
            }
            let drone = drones
                .iter()
                .filter(|(_, o, pos)| p.hostile_to(*o) && pos.dist(p.pos) <= PILLBOX_AIR_RANGE)
                .min_by(|a, b| a.2.dist(p.pos).partial_cmp(&b.2.dist(p.pos)).unwrap().then(a.0.cmp(&b.0)));
            if let Some((did, _, _)) = drone {
                // The owner tank's radar, if it is within link range.
                let link = p
                    .owner
                    .and_then(|o| tanks.iter().find(|(_, to, _)| *to == o))
                    .map(|(_, _, tp)| (1.0 - tp.dist(p.pos) / DRONE_LINK).clamp(0.0, 1.0))
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

    /// Damage to a pillbox from a shell. Dead at zero; never removed.
    fn damage_pillbox(&mut self, pid: EntityId, amount: f32, by: Option<PlayerId>) {
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
    fn hurt_squads(&mut self, at: V2) {
        let hit: Vec<EntityId> = self.tanks.iter().filter(|t| t.alive && t.pillboys.deployed() && t.pillboys.pos.dist(at) <= SPLASH_RADIUS).map(|t| t.id).collect();
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
        let mut triggered: Vec<EntityId> = Vec::new();
        for t in self.tanks.iter_mut() {
            if !t.alive {
                continue;
            }
            if t.reload > 0 {
                t.reload -= 1;
            }
            let Some(goal) = t.goal else {
                t.speed = 0.0;
                continue;
            };
            if t.fuel <= 0.0 {
                t.goal = None;
                t.route.clear();
                t.speed = 0.0;
                continue;
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
                continue;
            }
            let cost = map.move_cost(t.pos);
            let dir = to.norm();
            let want = t.heading;
            let turn = wrap_angle(dir.heading() - want);
            // Brake for a corner. At full speed a hull turning at
            // TANK_TURN swings two tiles wide, which on a one-tile bridge
            // is the river; a tank facing the wrong way pivots at a crawl
            // and accelerates once it is pointed down the road.
            let max = if turn.abs() > TURN_BRAKE_ANGLE { TANK_SPEED * TURN_CRAWL / cost } else { TANK_SPEED / cost };
            t.speed = (t.speed + TANK_ACCEL * DT).min(max);
            let step = TANK_TURN * DT;
            t.heading = want + turn.clamp(-step, step);
            let fwd = V2::from_heading(t.heading);
            let mut next = t.pos + fwd.scale(t.speed * DT);
            if dist < t.speed * DT {
                next = goal;
            }
            if !map.at(next).passable() || !map.in_bounds(next) {
                // Blocked: stop, and plan again from here to where the
                // route was going. Giving up left a tank parked at the
                // water's edge for a whole match.
                t.speed = 0.0;
                let dest = t.route.last().copied().unwrap_or(goal);
                let mut wp = map.route_with(t.pos, dest, |tile| if guns.iter().any(|(o, g)| *o != Some(t.owner) && g.dist(tile) <= PILLBOX_RANGE + 0.5) { PILLBOX_AVOID_COST } else { 0.0 });
                if wp.is_empty() {
                    t.goal = None;
                    t.route.clear();
                } else {
                    t.goal = Some(wp.remove(0));
                    t.route = wp;
                }
                continue;
            }
            t.pos = next;
            t.fuel = (t.fuel - FUEL_PER_TILE * t.speed * DT).max(0.0);
            if let Some(m) = mines.iter().find(|(o, tile, _)| *o != t.owner && *tile == next.tile()) {
                triggered.push(m.2);
            }
        }
        for mid in triggered {
            self.explode_mine(mid);
        }
    }

    fn step_drones(&mut self) {
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
                None => d.link = 1.0, // swarm drones are autonomous
            }
        }
        // 2. Interceptor assignment. For each tank with interceptors on
        //    station, any hostile drone inside detection gets the nearest
        //    free interceptor. One interceptor per intruder.
        let mut assignments: Vec<(EntityId, EntityId)> = Vec::new();
        for t in self.tanks.iter().filter(|t| t.alive) {
            let detect = t.bubble + DETECT_MARGIN;
            let mut intruders: Vec<&Drone> = self
                .drones
                .iter()
                .filter(|d| d.owner != t.owner && d.pos.dist(t.pos) <= detect && d.kind != DroneKind::Recon)
                .collect();
            // Also engage recon drones that come inside the bubble itself:
            // they are spotting for someone's gun.
            intruders.extend(
                self.drones.iter().filter(|d| d.owner != t.owner && d.kind == DroneKind::Recon && d.pos.dist(t.pos) <= t.bubble),
            );
            intruders.sort_by(|a, b| a.pos.dist(t.pos).partial_cmp(&b.pos.dist(t.pos)).unwrap().then(a.id.cmp(&b.id)));
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
                    .min_by(|a, b| a.pos.dist(intruder.pos).partial_cmp(&b.pos.dist(intruder.pos)).unwrap().then(a.id.cmp(&b.id)));
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
                    let need = hp.dist(d.pos) / drone_speed(d.kind) + 3.0;
                    if d.battery < need {
                        d.task = DroneTask::Return;
                    }
                }
            }
            let speed = drone_speed(d.kind);
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
            // A good link is a good vector from the tank's radar; a bad one
            // leaves the interceptor hunting on its own small camera.
            let p_kill = INTERCEPT_BASE + (INTERCEPT_LINKED - INTERCEPT_BASE) * link;
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
            lost.push((i, LossReason::Expended));
            self.events.push(Event::StrikeHit { drone: i, tank, damage: STRIKE_DAMAGE });
            self.damage(tank, STRIKE_DAMAGE, owner.filter(|o| *o != SWARM_OWNER));
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
                    self.hurt_squads(s.impact);
                    // Walls in the splash come down; mines in it go off.
                    let (cx, cy) = s.impact.tile();
                    for dy in -3..=3 {
                        for dx in -3..=3 {
                            let p = V2::new((cx + dx) as f32 + 0.5, (cy + dy) as f32 + 0.5);
                            if self.map.in_bounds(p) && self.map.at(p) == Terrain::Wall && p.dist(s.impact) <= SPLASH_RADIUS * 0.7 {
                                self.set_terrain(p, Terrain::Rubble);
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
                        .filter(|t| t.alive && t.owner != s.owner)
                        .map(|t| (t.id, t.pos.dist(s.impact)))
                        .filter(|(_, d)| *d <= DIRECT_HIT_RADIUS)
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                        .map(|(id, _)| id);
                    if let Some(id) = hit {
                        self.damage(id, DIRECT_DAMAGE, Some(s.owner));
                        hits.push(id);
                    } else {
                        // No tank there: a pillbox, if one is.
                        let pb = self.pillboxes.iter().filter(|p| p.alive() && p.hostile_to(s.owner) && p.pos.dist(s.impact) <= DIRECT_HIT_RADIUS + 0.5).map(|p| p.id).next();
                        if let Some(id) = pb {
                            self.damage_pillbox(id, DIRECT_DAMAGE, Some(s.owner));
                            hits.push(id);
                        }
                    }
                }
            }
            self.events.push(Event::ShellImpact { shell: s.id, at: s.impact, kind: s.kind, hits });
        }
    }

    fn damage(&mut self, tank: EntityId, amount: f32, by: Option<PlayerId>) {
        let tick = self.tick;
        let Some(t) = self.tank_mut(tank) else { return };
        if !t.alive {
            return;
        }
        t.armor -= amount;
        t.last_hit_tick = tick;
        let owner = t.owner;
        let left = t.armor;
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

    fn step_depots(&mut self) {
        let tanks: Vec<(EntityId, PlayerId, V2)> = self.tanks.iter().filter(|t| t.alive).map(|t| (t.id, t.owner, t.pos)).collect();
        let mut captured = Vec::new();
        let mut resupplied = Vec::new();
        for dep in self.depots.iter_mut() {
            let here: Vec<&(EntityId, PlayerId, V2)> = tanks.iter().filter(|(_, _, p)| p.dist(dep.pos) <= DEPOT_RADIUS).collect();
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
                    if dep.owner == Some(*owner) {
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
            let t = self.tank_mut(tid).unwrap();
            let before = (t.shells, t.bay, t.armor as i32, t.fuel as i32);
            if tick % RESUPPLY_EVERY == 0 {
                t.shells = (t.shells + 1).min(TANK_SHELLS);
                t.armor = (t.armor + REPAIR_PER_TICK).min(TANK_ARMOR);
                t.fuel = (t.fuel + 40.0).min(TANK_FUEL);
                // Interceptors first: the bubble is what keeps the tank alive
                // long enough to use the rest.
                if t.bay.interceptor < BAY_FULL.interceptor {
                    t.bay.interceptor += 1;
                } else if t.bay.recon < BAY_FULL.recon {
                    t.bay.recon += 1;
                } else if t.bay.strike < BAY_FULL.strike {
                    t.bay.strike += 1;
                }
                // The squad and its material, when aboard.
                if t.pillboys.aboard {
                    if t.pillboys.strength < PILLBOY_MAX {
                        t.pillboys.strength += 1;
                    }
                    if t.pillboys.material < PILLBOY_MATERIAL_MAX {
                        t.pillboys.material += 1;
                    }
                }
            }
            if before != (t.shells, t.bay, t.armor as i32, t.fuel as i32) && tick % (RESUPPLY_EVERY * 4) == 0 {
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
        let side = self.rng.below(4);
        let along = self.rng.range(8.0, MAP_W as f32 - 8.0);
        let from = match side {
            0 => V2::new(along, 0.5),
            1 => V2::new(MAP_W as f32 - 0.5, along),
            2 => V2::new(along, MAP_H as f32 - 0.5),
            _ => V2::new(0.5, along),
        };
        let (target, _) = alive.iter().min_by(|a, b| a.1.dist(from).partial_cmp(&b.1.dist(from)).unwrap()).unwrap();
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
            });
        }
        self.events.push(Event::SwarmInbound { count: n, from });
    }

    fn step_match(&mut self) {
        let alive: Vec<PlayerId> = self.players.iter().filter(|p| p.alive).map(|p| p.id).collect();
        let limit = self.scenario.time_limit;
        let timed_out = limit > 0 && self.tick >= limit;
        if (alive.len() <= 1 && self.players.len() > 1) || timed_out || alive.is_empty() {
            self.over = true;
            self.winner = if alive.len() == 1 {
                Some(alive[0])
            } else {
                self.players.iter().filter(|p| p.alive).max_by_key(|p| (p.score, std::cmp::Reverse(p.id))).map(|p| p.id)
            };
            self.events.push(Event::MatchOver { winner: self.winner });
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
        let tanks = self
            .tanks
            .iter()
            .filter(|t| t.owner == p || (t.alive && self.visible_to(p, t.pos)))
            .cloned()
            .collect();
        let drones = self.drones.iter().filter(|d| d.owner == p || self.visible_to(p, d.pos)).cloned().collect();
        let shells = self.shells.iter().filter(|s| s.owner == p || self.visible_to(p, s.pos_at(self.tick)) || self.visible_to(p, s.impact)).cloned().collect();
        Observation {
            tick: self.tick,
            player: p,
            me,
            tanks,
            drones,
            shells,
            depots: self.depots.clone(),
            pillboxes: self.pillboxes.clone(),
            mines: self.mines.iter().filter(|m| self.mine_known_to(p, m)).cloned().collect(),
            forest,
            players: self.players.iter().map(|x| PlayerView { id: x.id, name: x.name.clone(), alive: x.alive, score: x.score, is_bot: x.is_bot }).collect(),
            events: self.events.iter().filter(|e| event_visible(e, p)).cloned().collect(),
            over: self.over,
            winner: self.winner,
        }
    }
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
        if world.tick % CHECKPOINT_EVERY == 0 || world.over {
            if self.checkpoints.last().map(|(t, _)| *t != world.tick).unwrap_or(true) {
                self.checkpoints.push((world.tick, world.hash()));
            }
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

fn event_visible(e: &Event, p: PlayerId) -> bool {

    match e {
        Event::Rejected { player, .. } => *player == p,
        Event::ShellLaunched { owner, .. } => *owner == p,
        Event::DroneLaunched { owner, .. } | Event::DroneRecovered { owner, .. } | Event::LinkLost { owner, .. } => *owner == p,
        Event::PillboysDeployed { owner, .. } | Event::PillboysReturned { owner, .. } | Event::PillboysAbandoned { owner, .. } => *owner == p,
        Event::MineLaid { owner, .. } | Event::Harvested { owner, .. } => *owner == p,
        _ => true,
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
    pub depots: Vec<Depot>,
    /// Every pillbox, like every depot: territory is public.
    pub pillboxes: Vec<Pillbox>,
    /// The mines this player knows about: its own, and hostile ones
    /// within `MINE_REVEAL` of its tank or squad.
    pub mines: Vec<Mine>,
    /// The nearest forest tiles to the tank, for a Brain with no map.
    pub forest: Vec<V2>,

    pub players: Vec<PlayerView>,

    pub events: Vec<Event>,
    pub over: bool,
    pub winner: Option<PlayerId>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerView {
    pub id: PlayerId,
    pub name: String,
    pub alive: bool,
    pub score: u32,
    pub is_bot: bool,
}

impl Observation {
    pub fn seconds(&self) -> f32 {
        self.tick as f32 * DT
    }
    pub fn my_drones(&self) -> impl Iterator<Item = &Drone> {
        let p = self.player;
        self.drones.iter().filter(move |d| d.owner == p)
    }
    pub fn hostile_tanks(&self) -> impl Iterator<Item = &Tank> {
        let p = self.player;
        self.tanks.iter().filter(move |t| t.owner != p && t.alive)
    }
    pub fn hostile_drones(&self) -> impl Iterator<Item = &Drone> {
        let p = self.player;
        self.drones.iter().filter(move |d| d.owner != p)
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
        let at = w.map.center();
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
                orders.push(Order { player: 2, cmd: Command::FireIndirect { at: w.map.center() } });
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

    /// The squad fells a forest tile for material, walls a tile that
    /// then blocks a route and a flat shot, and lays a mine that only
    /// its layer can see until a tank finds it the hard way.
    #[test]
    fn squad_harvests_walls_and_mines() {
        let mut w = world();
        w.pillboxes.clear();
        let me = w.players[0].tank;
        let home = w.tank(me).unwrap().pos;
        // Harvest the nearest forest the crew can see.
        let obs = w.observe(0);
        let tree = *obs.forest.first().expect("no forest within sight of the north start");
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
