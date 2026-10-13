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
        self.step_drones();
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
                let mut wp = self.map.route(pos, *to);
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
            Command::Chat { .. } => {}
        }
    }

    fn step_tanks(&mut self) {
        let map = &self.map;
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
            if map.at(next) == Terrain::Water || !map.in_bounds(next) {
                // Blocked: stop, and plan again from here to where the
                // route was going. Giving up left a tank parked at the
                // water's edge for a whole match.
                t.speed = 0.0;
                let dest = t.route.last().copied().unwrap_or(goal);
                let mut wp = map.route(t.pos, dest);
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

    // --------------------------------------------------------- observation

    /// The world as player `p` is allowed to see it: own entities in full,
    /// everything else only where their sensors reach. This is what goes
    /// to a client, and what a Brain thinks from. Nothing in it is not
    /// also on the host; the difference is what has been left out.
    pub fn observe(&self, p: PlayerId) -> Observation {
        let me = self.tank_of(p).cloned();
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
            players: self.players.iter().map(|x| PlayerView { id: x.id, name: x.name.clone(), alive: x.alive, score: x.score, is_bot: x.is_bot }).collect(),
            events: self.events.iter().filter(|e| event_visible(e, p)).cloned().collect(),
            over: self.over,
            winner: self.winner,
        }
    }
}

fn event_visible(e: &Event, p: PlayerId) -> bool {
    match e {
        Event::Rejected { player, .. } => *player == p,
        Event::ShellLaunched { owner, .. } => *owner == p,
        Event::DroneLaunched { owner, .. } | Event::DroneRecovered { owner, .. } | Event::LinkLost { owner, .. } => *owner == p,
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
