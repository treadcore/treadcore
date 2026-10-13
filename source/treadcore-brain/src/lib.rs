//! treadcore-brain -- the controller interface, and one controller.
//!
//! Bolo called them Brains: a program that takes the tank's place at the
//! controls, given only what the tank can see. Same idea here. A Brain
//! gets an `Observation` -- the player's view of the world, filtered by
//! the host, nothing a human client does not also get -- and returns
//! `Command`s, which the world validates like anyone else's. A Brain
//! cannot cheat because it has no more access than a keyboard.
//!
//! Two implementations: `RuleBot`, which fills empty seats and is good
//! enough to kill a careless player; and `JsonBrain`, which speaks the
//! observation and commands as JSON over stdin/stdout so an external
//! program -- a Python script, an LLM harness -- can play without
//! linking this crate.

use std::io::{BufRead, Write};
use treadcore_sim::*;

// Send + Sync so a seat's Brain can live in a Bevy resource; JsonBrain keeps
// its receiver behind a Mutex for the same reason.
pub trait Brain: Send + Sync {
    fn name(&self) -> &str;
    fn think(&mut self, obs: &Observation) -> Vec<Command>;
}

/// Run every bot seat for one tick.
pub fn run_bots(world: &World, bots: &mut [(PlayerId, Box<dyn Brain>)]) -> Vec<Order> {
    let mut out = Vec::new();
    for (p, b) in bots.iter_mut() {
        let obs = world.observe(*p);
        if obs.me.as_ref().map(|t| !t.alive).unwrap_or(true) {
            continue;
        }
        for cmd in b.think(&obs) {
            out.push(Order { player: *p, cmd });
        }
    }
    out
}

// ------------------------------------------------------------------ rule bot

/// A doctrine, not an intelligence:
///
///   1. Keep the bubble up. Interceptors on station: four while anything
///      hostile is airborne nearby, two otherwise.
///   2. Take a firing position. The objective is the nearest depot that is
///      not mine (the centre, to begin with). Drive to `STANDOFF` tiles
///      short of it, by road: the world routes a Move around the river.
///   3. Eyes over the objective. Once stationary, one recon drone orbits
///      the objective (or the nearest known enemy), inside the link.
///      The first bot fired its recon toward the centre at launch and
///      then drove past it; a recon launched from a standing tank at its
///      firing position stays useful.
///   4. Shoot what the recon sees. A spotted tank between minimum and
///      maximum range gets a shell every reload, led by its motion.
///   5. Shoot and scoot. After firing, move a few tiles: counter-battery
///      is coming. Stay out of direct range of anything that can see you.
///   6. Strike a thin bubble: a strike drone at the nearest enemy once
///      fewer than two interceptors are visible around it.
///   7. Resupply when low; capture the nearest depot if none is mine.
///   8. Advance. Nothing seen for `PATIENCE` seconds at the firing
///      position: drive onto the objective and capture it, under the
///      recon, and the next unowned depot becomes the objective.
///   9. Pillboys (when `pillboys` is on). From a standing tank with no
///      enemy in view: send the squad to the nearest neutral or dead
///      pillbox within `PILLBOY_REACH` whose gun cannot reach the tank,
///      or to repair an owned one below half health. While the squad is
///      out the tank holds its position, so the squad has a home to
///      come back to; that wait is the cost the proof weighs.
pub struct RuleBot {
    name: String,
    rng: Rng,
    last_fire_tick: Tick,
    scoot_until: Tick,
    resupplying: bool,
    last_think: Tick,
    /// Tick an enemy tank was last in view; drives the patience clock.
    last_contact: Tick,
    /// Tick the tank last came to rest at a firing position.
    settled_at: Option<Tick>,
    /// This bot's patience in seconds, drawn once from its seed so four
    /// bots do not all advance on the same tick and meet in the middle.
    patience: f32,
    /// Use the squad at all. Off for the control group in the proof.
    pub pillboys: bool,
    /// What the world refused and why it will stay refused: pillboxes
    /// the squad cannot reach, and tiles the tank cannot drive to. On an
    /// island map (the converted Bolo maps) half the objectives are
    /// across water, and a bot that forgot was refused four times a
    /// second for the whole match.
    unreachable_pillboxes: Vec<EntityId>,
    unreachable_tiles: Vec<(i32, i32)>,
}

/// How far the bot will send its squad: a 20-tile walk is 13 seconds
/// each way at `PILLBOY_SPEED`, plus the work.
pub const PILLBOY_REACH: f32 = 20.0;

/// How far short of the objective the bot stops to fire from: inside
/// indirect range of anything at the objective, outside direct range
/// and outside the swarm's interest in it, and within the recon link.
pub const STANDOFF: f32 = 25.0;
/// Seconds at a firing position with nothing seen before advancing; each
/// bot draws its own between half and two and a half times this.
pub const PATIENCE: f32 = 20.0;

impl RuleBot {
    pub fn new(seed: u64, name: &str) -> RuleBot {
        let mut rng = Rng::new(seed);
        let patience = PATIENCE * rng.range(0.5, 2.5);
        RuleBot { name: name.into(), rng, last_fire_tick: 0, scoot_until: 0, resupplying: false, last_think: 0, last_contact: 0, settled_at: None, patience, pillboys: true, unreachable_pillboxes: Vec::new(), unreachable_tiles: Vec::new() }
    }

    /// The same doctrine without the squad: the control group.
    pub fn without_pillboys(seed: u64, name: &str) -> RuleBot {
        RuleBot { pillboys: false, ..RuleBot::new(seed, name) }
    }

    fn nearest_enemy<'a>(&self, obs: &'a Observation, me: &Tank) -> Option<&'a Tank> {
        obs.hostile_tanks().min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap())
    }

    fn nearest_depot<'a>(&self, obs: &'a Observation, me: &Tank, friendly: bool) -> Option<&'a Depot> {
        let ours = |d: &&Depot| d.owner.map(|o| obs.allied(o)).unwrap_or(false);
        obs.depots.iter().filter(|d| if friendly { ours(d) } else { !ours(d) }).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap())
    }


}

impl Brain for RuleBot {
    fn name(&self) -> &str {
        &self.name
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        let mut cmds = Vec::new();
        let Some(me) = obs.me.as_ref() else { return cmds };
        if !me.alive {
            return cmds;
        }
        // Every tick: what the world refused as unreachable stays so.
        for e in &obs.events {
            if let Event::Rejected { cmd, why, .. } = e {
                match cmd {
                    Command::Deploy { task: PillboyTask::Capture { pillbox } } | Command::Deploy { task: PillboyTask::Repair { target: pillbox } } if why.contains("unreachable") => {
                        if !self.unreachable_pillboxes.contains(pillbox) {
                            self.unreachable_pillboxes.push(*pillbox);
                        }
                    }
                    Command::Move { to } if why.contains("unreachable") || why == "water" => {
                        if !self.unreachable_tiles.contains(&to.tile()) {
                            self.unreachable_tiles.push(to.tile());
                        }
                    }
                    _ => {}
                }
            }
        }
        // Think four times a second; the world does not need more and the
        // host has three other bots to run.
        if obs.tick.saturating_sub(self.last_think) < (TICK_HZ / 4) as Tick {
            return cmds;
        }
        self.last_think = obs.tick;

        let my_interceptors = obs.my_drones().filter(|d| d.kind == DroneKind::Interceptor).count();
        let my_recon: Vec<&Drone> = obs.my_drones().filter(|d| d.kind == DroneKind::Recon).collect();
        let my_strikes = obs.my_drones().filter(|d| d.kind == DroneKind::Strike).count();
        let hostile_air = obs.hostile_drones().filter(|d| d.pos.dist(me.pos) <= me.bubble + DETECT_MARGIN + 6.0).count();
        let enemy = self.nearest_enemy(obs, me);
        if enemy.is_some() {
            self.last_contact = obs.tick;
        }
        let stationary = me.goal.is_none();
        if stationary {
            if self.settled_at.is_none() {
                self.settled_at = Some(obs.tick);
            }
        } else {
            self.settled_at = None;
        }
        // The objective: the nearest depot that is not mine and that the
        // world has not called unreachable. With none left, the centre.
        let objective = obs
            .depots
            .iter()
            .filter(|d| !d.owner.map(|o| obs.allied(o)).unwrap_or(false) && !self.unreachable_tiles.contains(&d.pos.tile()))
            .min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap())
            .map(|d| d.pos)
            .unwrap_or_else(|| obs.centre());

        // 1. The bubble.
        let want_station = if hostile_air > 0 { 4 } else { 2 };
        if my_interceptors < want_station && me.bay.interceptor > 0 {
            cmds.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
        }
        // Widen it when a swarm is in, tighten it when the link is poor.
        let want_bubble = if hostile_air >= 3 { 8.0 } else { BUBBLE_DEFAULT };
        if (me.bubble - want_bubble).abs() > 0.5 {
            cmds.push(Command::SetBubble { radius: want_bubble });
        }

        // 3. Eyes. Launched from a standing tank, over what matters: the
        // enemy if one is known, else the objective. A second recon goes
        // up when the first cannot see a known enemy.
        let look_at = enemy.map(|e| e.pos).unwrap_or(objective);
        let recon_sees_it = my_recon.iter().any(|d| d.pos.dist(look_at) <= RECON_VISION);
        // "Stationary" means settled for a second, not merely not yet
        // ordered to move: on the first think the tank has no goal and a
        // recon launched then orbits the start, where the tank will not
        // be.
        let settled_s = self.settled_at.map(|t| obs.tick.saturating_sub(t) as f32 * DT).unwrap_or(0.0);
        let dir = (look_at - me.pos).norm();
        let dist = me.pos.dist(look_at).min(RECON_LINK * 0.7);
        let center = me.pos + dir.scale(dist);
        if my_recon.is_empty() {
            if me.bay.recon > 0 && (settled_s >= 1.0 || enemy.is_some()) {
                cmds.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center, radius: 3.0 } });
            }
        } else if enemy.is_some() && !recon_sees_it {
            // A recon that cannot see the enemy is moved, not doubled
            // (backlog item 3): its orbit goes to where the enemy is,
            // once, and it flies there.
            let r = my_recon[0];
            let already = matches!(&r.task, DroneTask::Orbit { center: c, .. } if c.dist(center) < 4.0);
            if !already && matches!(r.task, DroneTask::Orbit { .. }) {
                cmds.push(Command::Retask { drone: r.id, task: DroneTask::Orbit { center, radius: 3.0 } });
            }
        }

        // 9. The squad. Deployed: hold here and let the gun and the
        // bubble carry on; the movement sections below are skipped.
        let squad_out = me.pillboys.deployed();
        if self.pillboys && !squad_out && me.pillboys.strength > 0 && settled_s >= 1.0 && enemy.is_none() {
            let safe = |p: &Pillbox| !p.alive() || p.pos.dist(me.pos) > PILLBOX_RANGE + 1.0;
            let capture = obs
                .pillboxes
                .iter()
                .filter(|p| !p.owner.map(|o| obs.allied(o)).unwrap_or(false) && (p.owner.is_none() || !p.alive()) && p.pos.dist(me.pos) <= PILLBOY_REACH && safe(p) && !self.unreachable_pillboxes.contains(&p.id))
                .min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap());
            if let Some(p) = capture {
                cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: p.id } });
            } else if me.pillboys.material >= PILLBOX_REPAIR_COST {
                let repair = obs
                    .pillboxes
                    .iter()
                    .filter(|p| p.owner == Some(obs.player) && p.health < PILLBOX_HEALTH * 0.5 && p.pos.dist(me.pos) <= PILLBOY_REACH && !self.unreachable_pillboxes.contains(&p.id))
                    .min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap());
                if let Some(p) = repair {
                    cmds.push(Command::Deploy { task: PillboyTask::Repair { target: p.id } });
                }
            }
        }

        // 4. The gun, and 6. strikes.
        if let Some(e) = enemy {
            let range = me.pos.dist(e.pos);
            if me.reload == 0 && me.shells > 0 {
                if range <= DIRECT_RANGE {
                    cmds.push(Command::FireDirect { at: e.pos });
                    self.last_fire_tick = obs.tick;
                } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&range) {
                    // Lead a moving target by its flight time; if the
                    // led point falls inside minimum range (the world
                    // refused eight of these a match, backlog item 4),
                    // fire at the tank itself.
                    let flight = range / SHELL_SPEED_INDIRECT;
                    let lead = V2::from_heading(e.heading).scale(e.speed * flight * 0.8);
                    let aim = e.pos + lead;
                    let aim = if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&me.pos.dist(aim)) { aim } else { e.pos };
                    cmds.push(Command::FireIndirect { at: aim });
                    self.last_fire_tick = obs.tick;
                    self.scoot_until = obs.tick + (6 * TICK_HZ) as Tick;
                }
            }
            let their_team = obs.team_of(e.owner);
            let their_cover = obs.drones.iter().filter(|d| obs.team_of(d.owner) == their_team && d.kind == DroneKind::Interceptor && d.pos.dist(e.pos) <= BUBBLE_MAX + 2.0).count();
            if their_cover < 2 && my_strikes == 0 && me.bay.strike > 0 && range <= STRIKE_BATTERY * STRIKE_SPEED * 0.8 {
                cmds.push(Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: e.id } });
            }
        }

        if squad_out {
            return cmds;
        }

        // 7. Resupply.
        let low = me.shells < 6 || me.armor < 45.0 || me.bay.interceptor == 0 || me.fuel < 120.0;
        let full = me.shells >= TANK_SHELLS - 2 && me.armor >= TANK_ARMOR - 5.0 && me.bay.interceptor >= BAY_FULL.interceptor - 1;
        if self.resupplying && full {
            self.resupplying = false;
        }
        if low && !self.resupplying {
            self.resupplying = true;
        }
        if self.resupplying {
            let depot = self.nearest_depot(obs, me, true).or_else(|| self.nearest_depot(obs, me, false));
            if let Some(d) = depot {
                if me.pos.dist(d.pos) > DEPOT_RADIUS * 0.6 && stationary {
                    cmds.push(Command::Move { to: d.pos });
                }
            }
            return cmds;
        }

        // 5. Scoot, and keep distance: out of direct range, inside indirect.
        if let Some(e) = enemy {
            let range = e.pos.dist(me.pos);
            let too_close = range < INDIRECT_MIN_RANGE + 2.0;
            let too_far = range > INDIRECT_MAX_RANGE * 0.9;
            if (obs.tick < self.scoot_until || too_close || too_far) && stationary {
                let away = (me.pos - e.pos).norm();
                let to = if too_far {
                    e.pos + away.scale(INDIRECT_MAX_RANGE * 0.6)
                } else if too_close {
                    e.pos + away.scale(INDIRECT_MIN_RANGE + 8.0)
                } else {
                    me.pos + away.scale(3.0) + self.rng.disc().scale(4.0)
                };
                cmds.push(Command::Move { to: obs.clamp(to) });
            }
            return cmds;
        }

        // 2. The firing position, and 8. the advance.
        let to_obj = me.pos.dist(objective);
        if to_obj > STANDOFF + 3.0 && stationary {
            let p = obs.clamp(objective + (me.pos - objective).norm().scale(STANDOFF));
            if !self.unreachable_tiles.contains(&p.tile()) {
                cmds.push(Command::Move { to: p });
            }
        } else if to_obj > DEPOT_RADIUS * 0.6 && stationary {
            let settled = self.settled_at.map(|t| obs.tick.saturating_sub(t)).unwrap_or(0) as f32 * DT;
            let quiet = obs.tick.saturating_sub(self.last_contact) as f32 * DT;
            if settled >= self.patience && quiet >= self.patience && !my_recon.is_empty() {

                cmds.push(Command::Move { to: objective });
            }
        }
        cmds
    }
}

// ------------------------------------------------------------ fortifier

/// The smallest brain that plays the brief's defensive game: it stays
/// at its home depot, keeps two interceptors up, shoots what it sees,
/// and sets the squad to work: harvest the nearest forest the crew can
/// see until the tank carries material, then lay a ring of mines round
/// the depot at radius `MOAT_RADIUS` (no gate: its own mines do not
/// hurt it), then a ring of walls one tile further out with the tile
/// toward the map's centre left open as the gate. With `fortify` off it does everything but the squad's work:
/// the control group for roadmap step 6c's proof.
pub struct Fortifier {
    name: String,
    last_think: Tick,
    pub fortify: bool,
    home: Option<V2>,
    /// Wall tiles already ordered: the Observation carries no map, so
    /// the brain remembers rather than looks.
    wall_tries: Vec<(i32, i32)>,
    /// Tiles the world refused to build on, from its own Rejected
    /// events; and forest tiles it was told to fell first.
    blocked: Vec<(i32, i32)>,
    fell: Vec<V2>,
}

pub const MOAT_RADIUS: f32 = 3.0;

impl Fortifier {
    pub fn new(name: &str, fortify: bool) -> Fortifier {
        Fortifier { name: name.into(), last_think: 0, fortify, home: None, wall_tries: Vec::new(), blocked: Vec::new(), fell: Vec::new() }
    }

    /// The ring of tiles at `r` round `c`, in the order to lay them: the
    /// far side first. With `gate`, the tile toward `toward` is left
    /// out: a wall ring needs a way out, a minefield does not, because a
    /// tank's own mines never hurt it. The first proof left a gate in
    /// the moat and the attacker's route came down the road through it.
    fn ring(c: V2, r: f32, toward: V2, gate: bool) -> Vec<V2> {
        let mut tiles: Vec<V2> = Vec::new();
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let p = Map::tile_centre(c + V2::new(dx as f32, dy as f32));
                let d = p.dist(Map::tile_centre(c));
                if d >= r - 0.5 && d < r + 0.5 {
                    tiles.push(p);
                }
            }
        }
        let dir = (toward - c).norm();
        // Farthest from the gate first; the gate tile itself is skipped.
        if gate {
            tiles.retain(|p| (*p - c).norm().dist(dir) > 0.5);
        }
        tiles.sort_by(|a, b| (*b - c).norm().dist(dir).partial_cmp(&(*a - c).norm().dist(dir)).unwrap());
        tiles
    }
}

impl Brain for Fortifier {
    fn name(&self) -> &str {
        &self.name
    }
    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        let mut cmds = Vec::new();
        let Some(me) = obs.me.as_ref() else { return cmds };
        // Every tick, before the think-rate gate: a refusal lands in the
        // tick after the order, which is rarely a think tick.
        // The world says why an order failed, and the brain has no map
        // to have known. Forest in the ring is felled first (material,
        // too); anything else is left alone.
        for e in &obs.events {
            if let Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Build { at, .. } }, why, .. } = e {
                if why.starts_with("forest") {
                    if !self.fell.iter().any(|f| f.tile() == at.tile()) {
                        self.fell.push(*at);
                    }
                } else if !self.blocked.contains(&at.tile()) {
                    self.blocked.push(at.tile());
                }
            }
        }
        if !me.alive || obs.tick.saturating_sub(self.last_think) < (TICK_HZ / 4) as Tick {
            return cmds;
        }
        self.last_think = obs.tick;
        let home = *self.home.get_or_insert_with(|| obs.depots.iter().filter(|d| d.owner == Some(obs.player)).chain(obs.depots.iter().filter(|d| d.owner.map(|o| obs.allied(o)).unwrap_or(false))).map(|d| d.pos).min_by(|a, b| a.dist(me.pos).partial_cmp(&b.dist(me.pos)).unwrap()).unwrap_or(me.pos));
        let centre = obs.centre();
        // Sit by the depot, just off it so the squad's rings have room.
        let post = home + (home - centre).norm().scale(1.0);
        if me.pos.dist(post) > 1.5 && me.goal.is_none() {
            cmds.push(Command::Move { to: post });
            return cmds;
        }
        // The bubble, and the gun.
        let my_interceptors = obs.my_drones().filter(|d| d.kind == DroneKind::Interceptor).count();
        if my_interceptors < 2 && me.bay.interceptor > 0 {
            cmds.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
        }
        if let Some(e) = obs.hostile_tanks().min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap()) {
            let range = me.pos.dist(e.pos);
            if me.reload == 0 && me.shells > 0 {
                if range <= DIRECT_RANGE {
                    cmds.push(Command::FireDirect { at: e.pos });
                } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&range) {
                    let lead = V2::from_heading(e.heading).scale(e.speed * range / SHELL_SPEED_INDIRECT * 0.8);
                    cmds.push(Command::FireIndirect { at: e.pos + lead });
                }
            }
        }
        // The squad.
        if !self.fortify || me.pillboys.strength == 0 {
            return cmds;
        }
        if me.pillboys.deployed() || me.goal.is_some() {
            return cmds;
        }
        let known: Vec<(i32, i32)> = obs.mines.iter().filter(|m| m.owner == obs.player).map(|m| m.pos.tile()).collect();
        // Forest tiles in the rings wait until there is room for their
        // material: the first version re-ordered a mine onto forest,
        // was told to fell it, could not (material full), and did it
        // again four times a second for three minutes.
        let forest_tiles: Vec<(i32, i32)> = self.fell.iter().map(|f| f.tile()).collect();
        let open = |p: &V2| !self.blocked.contains(&p.tile()) && !forest_tiles.contains(&p.tile());
        let moat: Vec<V2> = Self::ring(home, MOAT_RADIUS, centre, false).into_iter().filter(|p| !known.contains(&p.tile()) && open(p)).collect();
        let walls: Vec<V2> = Self::ring(home, MOAT_RADIUS + 1.0, centre, true).into_iter().filter(|p| open(p) && !self.wall_tries.contains(&p.tile())).collect();
        if me.pillboys.material + HARVEST_MATERIAL <= PILLBOY_MATERIAL_MAX {
            if let Some(tree) = self.fell.first().copied() {
                self.fell.remove(0);
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: tree } });
                return cmds;
            }
        }
        // Material first: a harvest is four, a mine one. With none and no
        // forest in sight there is nothing to build with: wait for the
        // depot's trickle rather than order a build the world refuses
        // four times a second (backlog item 4).
        if me.pillboys.material < 1 || (me.pillboys.material < 4 && !moat.is_empty() && me.pillboys.material < moat.len() as u8) {
            if let Some(tree) = obs.forest.first() {
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: *tree } });
                return cmds;
            }
        }
        if me.pillboys.material < 1 {
            return cmds;
        }
        if let Some(at) = moat.first() {
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, at: *at } });
            return cmds;
        }
        // Walls: the Observation has no map, so try each tile in turn;
        // a refused order (already a wall) costs nothing but a tick.
        if let Some(at) = walls.first().copied() {
            self.wall_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Wall, at } });
        }
        cmds
    }
}

// ------------------------------------------------------------- json brain


/// An external program as a Brain. Each tick the observation goes out as
/// one line of JSON on its stdin; it answers with one line: a JSON array
/// of commands (possibly empty). Slow answers are not waited for -- the
/// tick goes on without them and the next observation is sent anyway.
/// A program that wants to reason at length should do so across ticks.
///
/// Observation and Command are exactly treadcore-sim's types, serialised
/// with serde: the schema is the source. `treadcore-server --brain-schema`
/// prints an example of each.
pub struct JsonBrain {
    name: String,
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<String>>,
}

impl JsonBrain {
    pub fn spawn(name: &str, program: &str, args: &[String]) -> std::io::Result<JsonBrain> {
        use std::process::{Command as P, Stdio};
        let mut child = P::new(program).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(JsonBrain { name: name.into(), child, stdin, rx: std::sync::Mutex::new(rx) })
    }
}

impl Brain for JsonBrain {
    fn name(&self) -> &str {
        &self.name
    }
    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        if let Ok(line) = serde_json::to_string(obs) {
            let _ = writeln!(self.stdin, "{}", line);
        }
        let mut cmds = Vec::new();
        let rx = self.rx.lock().unwrap();
        while let Ok(line) = rx.try_recv() {
            match serde_json::from_str::<Vec<Command>>(&line) {
                Ok(c) => cmds.extend(c),
                Err(e) => log::warn!("brain '{}': bad line: {}", self.name, e),
            }
        }
        cmds
    }
}

impl Drop for JsonBrain {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// Example documents for the JSON brain, printed by the server.
pub fn schema_examples() -> String {
    let w = World::new(Scenario::default());
    let obs = w.observe(0);
    let cmds = vec![
        Command::Move { to: V2::new(40.0, 40.0) },
        Command::FireIndirect { at: V2::new(64.0, 64.0) },
        Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: V2::new(50.0, 50.0), radius: 4.0 } },
        Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return },
        Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: 7 } },
        Command::SetBubble { radius: 8.0 },
        Command::Recall,
        Command::Stop,
        Command::Chat { text: "on my way".into(), to: ChatTo::Team },
        Command::Alliance(AllianceCmd::Request { to: 1 }),
        Command::Propose { to: 2, terms: Terms::Swap },
    ];
    format!(
        "# observation (one line per tick on stdin)\n{}\n\n# commands (one JSON array per line on stdout)\n{}\n",
        serde_json::to_string_pretty(&obs).unwrap(),
        serde_json::to_string_pretty(&cmds).unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sixteen bots, six minutes: the rules get exercised. (Four bots on
    /// the sixteen-start map spend the match collecting the twelve free
    /// depots and rarely meet; that is play, not a failure, but it is
    /// not a test of the gun.)
    /// Roadmap step 3's proof: over twenty seeds, bots that capture
    /// pillboxes beat the same doctrine without the squad. Eight seats,
    /// alternating; five minutes each. The margin is printed so the
    /// trace of a run can show it, and asserted on the aggregate.
    #[test]
    fn pillboys_pay() {
        let mut wins = (0u32, 0u32);
        let mut score = (0u32, 0u32);
        let mut alive = (0u32, 0u32);
        let mut captures = 0u32;
        for seed in 0..20u64 {
            let scenario = Scenario { seed: 0x5EED_0000 + seed * 7919, swarm_interval: 45 * TICK_HZ, time_limit: 5 * 60 * TICK_HZ as Tick, ..Scenario::bots(8) };
            let mut w = World::new(scenario);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..8u8)
                .map(|i| {
                    let b: Box<dyn Brain> = if i % 2 == 0 { Box::new(RuleBot::new(1000 + seed * 16 + i as u64, "squad")) } else { Box::new(RuleBot::without_pillboys(1000 + seed * 16 + i as u64, "nosquad")) };
                    (i, b)
                })
                .collect();
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                captures += w.events.iter().filter(|e| matches!(e, Event::PillboxCaptured { .. })).count() as u32;
            }
            for p in &w.players {
                let with = p.id % 2 == 0;
                if with { score.0 += p.score; alive.0 += p.alive as u32 } else { score.1 += p.score; alive.1 += p.alive as u32 }
            }
            if let Some(win) = w.winner {
                if win % 2 == 0 { wins.0 += 1 } else { wins.1 += 1 }
            }
        }
        eprintln!("pillboys_pay over 20 seeds: wins {}:{}  score {}:{}  alive {}:{}  captures {}", wins.0, wins.1, score.0, score.1, alive.0, alive.1, captures);
        assert!(captures >= 20, "the squads barely captured anything: {}", captures);
        assert!(score.0 > score.1, "capturers did not outscore: {} vs {}", score.0, score.1);
        assert!(alive.0 >= alive.1, "capturers did not outlive: {} vs {}", alive.0, alive.1);
    }

    /// Roadmap step 6c's proof: a fortified depot survives a scripted
    /// attack longer than a bare one, over twenty seeds. Two seats: the
    /// Fortifier at its home depot, and an attacker that drives for that
    /// depot after a grace period and fires at what it sees. Measured:
    /// the tick the defender dies, or the time limit; and whether the
    /// attacker died. The margin is printed so a trace can show it.
    #[test]
    fn fortification_pays() {
        struct Attacker {
            target: V2,
            start: Tick,
            last: Tick,
        }
        impl Brain for Attacker {
            fn name(&self) -> &str {
                "attacker"
            }
            fn think(&mut self, obs: &Observation) -> Vec<Command> {
                let mut c = Vec::new();
                let Some(me) = obs.me.as_ref() else { return c };
                if obs.tick < self.start || obs.tick.saturating_sub(self.last) < 5 {
                    return c;
                }
                self.last = obs.tick;
                if let Some(e) = obs.hostile_tanks().next() {
                    if me.reload == 0 && me.shells > 0 {
                        let r = me.pos.dist(e.pos);
                        if r <= DIRECT_RANGE {
                            c.push(Command::FireDirect { at: e.pos });
                        } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&r) {
                            c.push(Command::FireIndirect { at: e.pos });
                        }
                    }
                }
                if me.goal.is_none() && me.pos.dist(self.target) > 1.0 {
                    c.push(Command::Move { to: self.target });
                }
                c
            }
        }
        let grace = 150 * TICK_HZ as Tick;
        let limit = 6 * 60 * TICK_HZ as Tick;
        let mut survived = (0u64, 0u64);
        let mut defender_alive = (0u32, 0u32);
        let mut attacker_dead = (0u32, 0u32);
        let mut built = (0u32, 0u32);
        for seed in 0..20u64 {
            for fortify in [true, false] {
                let scenario = Scenario { seed: 0xF0F7_0000 + seed * 104_729, swarm_interval: 0, time_limit: limit, ..Scenario::bots(2) };
                let mut w = World::new(scenario);
                w.pillboxes.clear();
                let depot = w.depots.iter().find(|d| d.owner == Some(0)).unwrap().pos;
                let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = vec![(0, Box::new(Fortifier::new("fort", fortify))), (1, Box::new(Attacker { target: depot, start: grace, last: 0 }))];
                let mut died_at = limit;
                let mut things = 0u32;
                while !w.over {
                    let orders = run_bots(&w, &mut bots);
                    w.step(&orders);
                    things += w.events.iter().filter(|e| matches!(e, Event::Built { .. })).count() as u32;
                    if !w.players[0].alive && died_at == limit {
                        died_at = w.tick;
                    }
                }
                let slot = if fortify { &mut survived.0 } else { &mut survived.1 };
                *slot += died_at;
                if w.players[0].alive {
                    if fortify { defender_alive.0 += 1 } else { defender_alive.1 += 1 }
                }
                if !w.players[1].alive {
                    if fortify { attacker_dead.0 += 1 } else { attacker_dead.1 += 1 }
                }
                if fortify { built.0 += things } else { built.1 += things }
            }
        }
        eprintln!(
            "fortification_pays over 20 seeds: defender survived to tick {} vs {} (of {} max); defender alive {}:{}  attacker dead {}:{}  things built {}:{}",
            survived.0, survived.1, limit * 20, defender_alive.0, defender_alive.1, attacker_dead.0, attacker_dead.1, built.0, built.1
        );
        assert!(built.0 >= 60, "the fortifier barely built anything: {}", built.0);
        assert!(survived.0 > survived.1, "fortified did not outlive bare: {} vs {}", survived.0, survived.1);
        assert!(defender_alive.0 >= defender_alive.1);
    }

    #[test]
    fn bots_fight_to_a_result() {


        let scenario = Scenario { swarm_interval: 30 * TICK_HZ, time_limit: 6 * 60 * TICK_HZ as Tick, ..Scenario::default() };
        let mut w = World::new(scenario);
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..MAX_PLAYERS).map(|i| (i as PlayerId, Box::new(RuleBot::new(100 + i as u64, "bot")) as Box<dyn Brain>)).collect();

        let mut shots = 0;
        let mut launches = 0;
        while !w.over {
            let orders = run_bots(&w, &mut bots);
            w.step(&orders);
            shots += w.events.iter().filter(|e| matches!(e, Event::ShellLaunched { .. })).count();
            launches += w.events.iter().filter(|e| matches!(e, Event::DroneLaunched { .. })).count();
        }
        assert!(shots > 10, "bots never fired: {}", shots);
        assert!(launches > 10, "bots never launched: {}", launches);
    }
}
