//! treadcore-brain -- the controller interface, and one controller.
//!
//! The prior art called them Brains: a program that takes the tank's place at the
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
///  10. Storm (roadmap step 6d, backlog item 14). An enemy that has
///      been in view for twice the bot's patience and is still alive is
///      sitting on a depot being repaired as fast as it is shelled; the
///      firing position is not working. The bot shells the live
///      hostile pillboxes guarding the objective, then drives onto it
///      to capture it, firing direct on the way, and stops scooting.
///      Low on anything, it goes home instead and the storm is off.
/// The first minute (PLAYABILITY.md item 1). For this long after the
/// start a bot will not fire at, or send a strike at, a hostile tank
/// that has never moved and never fired: a newcomer sitting at the
/// start reading the help. Every tank starts with full fuel and burns
/// it for every tile it drives, so a tank seen below full has moved;
/// one seen reloading has fired. Either, once seen, is remembered, and
/// that tank has joined the game. Bots drive off their starts at once,
/// so the rule spares nobody but a player who has not begun.
///
/// A first version spared any tank that had not moved since this bot
/// first saw it. Over sixty seeds of the squad proof it shifted bot
/// survival from 110:110 to 103:123, because a bot standing at a
/// firing position or a depot was spared too. This one spares only a
/// tank that has never moved.
pub const GRACE_SECONDS: u32 = 60;

/// What a bot remembers to keep the first minute: the hostile tanks it
/// has seen move or fire.
#[derive(Default)]
struct Grace {
    joined: Vec<EntityId>,
}

impl Grace {
    fn update(&mut self, obs: &Observation) {
        if obs.tick >= GRACE_SECONDS as Tick * TICK_HZ as Tick {
            return;
        }
        for t in obs.hostile_tanks() {
            if (t.fuel < TANK_FUEL || t.reload > 0) && !self.joined.contains(&t.id) {
                self.joined.push(t.id);
            }
        }
    }

    /// A fair target: the first minute is over, or the tank has been
    /// seen to move or fire.
    fn fair(&self, obs: &Observation, t: &Tank) -> bool {
        obs.tick >= GRACE_SECONDS as Tick * TICK_HZ as Tick || self.joined.contains(&t.id) || t.fuel < TANK_FUEL || t.reload > 0
    }
}

pub struct RuleBot {
    name: String,
    rng: Rng,
    last_fire_tick: Tick,
    scoot_until: Tick,
    resupplying: bool,
    last_think: Tick,
    /// Tick an enemy tank was last in view; drives the patience clock.
    last_contact: Tick,
    /// Seconds an enemy has been in view since the objective was set,
    /// accumulated across the bot's own scoots and resupplies. Twice
    /// the patience and the bot storms.
    contact_total: f32,
    /// The objective the contact clock belongs to.
    contact_objective: Option<(i32, i32)>,
    /// The depot being stormed, until it is ours or the bot runs low.
    storm: Option<V2>,
    /// Tick the tank last came to rest at a firing position.
    settled_at: Option<Tick>,
    /// This bot's patience in seconds, drawn once from its seed so four
    /// bots do not all advance on the same tick and meet in the middle.
    patience: f32,
    /// Use the squad at all. Off for the control group in the proof.
    pub pillboys: bool,
    /// What the world refused and why it will stay refused: pillboxes
    /// the squad cannot reach, and tiles the tank cannot drive to. On an
    /// island map (the converted prior-art maps) half the objectives are
    /// across water, and a bot that forgot was refused four times a
    /// second for the whole match.
    unreachable_pillboxes: Vec<EntityId>,
    unreachable_tiles: Vec<(i32, i32)>,
    /// The first minute: who may be shot yet.
    grace: Grace,
    /// Who has hurt this tank: a person who has is not let onto its side.
    hurt_by: Vec<PlayerId>,
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
        RuleBot { name: name.into(), rng, last_fire_tick: 0, scoot_until: 0, resupplying: false, last_think: 0, last_contact: 0, contact_total: 0.0, contact_objective: None, storm: None, settled_at: None, patience, pillboys: true, unreachable_pillboxes: Vec::new(), unreachable_tiles: Vec::new(), grace: Grace::default(), hurt_by: Vec::new() }
    }

    /// The same doctrine without the squad: the control group.
    pub fn without_pillboys(seed: u64, name: &str) -> RuleBot {
        RuleBot { pillboys: false, ..RuleBot::new(seed, name) }
    }

    fn nearest_enemy<'a>(&self, obs: &'a Observation, me: &Tank) -> Option<&'a Tank> {
        obs.hostile_tanks().filter(|t| self.grace.fair(obs, t)).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap())
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
        // Every tick: who hurt us, and what the world refused as
        // unreachable stays so.
        for e in &obs.events {
            if let Event::TankDamaged { tank, by: Some(b), .. } = e {
                if *tank == me.id && !self.hurt_by.contains(b) {
                    self.hurt_by.push(*b);
                }
            }
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
        // A person's proposal (PLAYABILITY item 7) is answered: yes,
        // unless they have hurt this tank. Only people's: bot-only
        // matches, and the traces their balance rests on, are as they
        // were, because a RuleBot never answered a bot.
        let human = |p: PlayerId| obs.players.iter().any(|x| x.id == p && !x.is_bot);
        if let Some(p) = obs.proposals.iter().find(|p| p.to == obs.player && p.from != obs.player && human(p.from)) {
            cmds.push(Command::Answer { proposal: p.id, accept: !self.hurt_by.contains(&p.from) });
        }

        let my_interceptors = obs.my_drones().filter(|d| d.kind == DroneKind::Interceptor).count();
        let my_recon: Vec<&Drone> = obs.my_drones().filter(|d| d.kind == DroneKind::Recon).collect();
        let my_strikes = obs.my_drones().filter(|d| d.kind == DroneKind::Strike).count();
        let hostile_air = obs.hostile_drones().filter(|d| d.pos.dist(me.pos) <= me.bubble + DETECT_MARGIN + 6.0).count();
        self.grace.update(obs);
        let enemy = self.nearest_enemy(obs, me);
        if enemy.is_some() {
            self.last_contact = obs.tick;
            self.contact_total += (TICK_HZ / 4) as f32 * DT;
        }
        // Hit in the last second: by something, seen or not.
        let just_hit = me.last_hit_tick > 0 && obs.tick.saturating_sub(me.last_hit_tick) <= TICK_HZ as Tick;
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
        if self.contact_objective != Some(objective.tile()) {
            self.contact_objective = Some(objective.tile());
            self.contact_total = 0.0;
        }

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
        // The recon orbits short of what it watches: a recon inside an
        // enemy's bubble is intercepted (the first 6d trace lost both of
        // an attacker's recons over a defended depot in eight seconds
        // and the attacker sat blind for the rest of the match). Ten
        // tiles short with a two-tile orbit keeps it outside a default
        // bubble and inside its own twelve tiles of vision.
        let dir = (look_at - me.pos).norm();
        let dist = (me.pos.dist(look_at) - (RECON_VISION - 2.0)).clamp(2.0, RECON_LINK * 0.7);
        let center = me.pos + dir.scale(dist);
        if my_recon.is_empty() {
            if me.bay.recon > 0 && (settled_s >= 1.0 || enemy.is_some()) {
                cmds.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center, radius: 2.0 } });
            }
        } else if enemy.is_some() && !recon_sees_it {
            // A recon that cannot see the enemy is moved, not doubled
            // (backlog item 3): its orbit goes to where the enemy is,
            // once, and it flies there.
            let r = my_recon[0];
            let already = matches!(&r.task, DroneTask::Orbit { center: c, .. } if c.dist(center) < 4.0);
            if !already && matches!(r.task, DroneTask::Orbit { .. }) {
                cmds.push(Command::Retask { drone: r.id, task: DroneTask::Orbit { center, radius: 2.0 } });
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

        // 10. The storm: on when the enemy has been in view for twice the
        // patience and the objective is in reach; off when the depot
        // is ours or the bot runs low (section 7 takes it home).
        let hostile_box = |p: &&Pillbox| p.alive() && !p.owner.map(|o| obs.allied(o)).unwrap_or(false);
        if self.storm.is_none() {
            let long_contact = self.contact_total >= self.patience * 2.0;
            if long_contact && me.pos.dist(objective) <= INDIRECT_MAX_RANGE {
                self.storm = Some(objective);
                cmds.push(Command::Chat { text: "storming the depot".into(), to: ChatTo::Team });
            }
        }
        if let Some(tgt) = self.storm {
            let taken = obs.depots.iter().any(|d| d.pos.dist(tgt) < 1.0 && d.owner.map(|o| obs.allied(o)).unwrap_or(false));
            if taken {
                self.storm = None;
            }
        }
        // The guns guarding a stormed depot: shelled before anything else.
        let guns: Vec<&Pillbox> = match self.storm {
            Some(tgt) => obs.pillboxes.iter().filter(hostile_box).filter(|p| p.pos.dist(tgt) <= PILLBOX_RANGE + 2.0).collect(),
            None => Vec::new(),
        };
        let gun_target = guns.iter().filter(|p| (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&me.pos.dist(p.pos))).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap());
        if let Some(g) = gun_target {
            if me.reload == 0 && me.shells > 0 {
                cmds.push(Command::FireIndirect { at: g.pos });
                self.last_fire_tick = obs.tick;
            }
        }

        // 4. The gun, and 6. strikes.
        if let Some(e) = enemy {
            let range = me.pos.dist(e.pos);
            if me.reload == 0 && me.shells > 0 && gun_target.is_none() {
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

        // Under fire with the squad out, the squad comes home and the
        // tank moves: the first 6d trace had three attackers die at
        // their starts holding still for a squad that was capturing a
        // pillbox ten tiles away.
        if squad_out {
            if !just_hit {
                return cmds;
            }
            cmds.push(Command::Deploy { task: PillboyTask::Return });
        }

        // 7. Resupply.
        // Blind counts as low: a bot with no recon up and none in the bay
        // cannot see to shoot or to advance.
        let blind = me.bay.recon == 0 && my_recon.is_empty();
        let low = me.shells < 6 || me.armor < 45.0 || me.bay.interceptor == 0 || me.fuel < 120.0 || blind;
        let full = me.shells >= TANK_SHELLS - 2 && me.armor >= TANK_ARMOR - 5.0 && me.bay.interceptor >= BAY_FULL.interceptor - 1 && me.bay.recon > 0;
        if self.resupplying && full {
            self.resupplying = false;
        }
        if low && !self.resupplying {
            self.resupplying = true;
            self.storm = None;
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

        // Hit by something unseen: move anyway. A tank that stands still
        // for a spotted shell's flight is where the next one lands.
        if just_hit && stationary && enemy.is_none() {
            let to = me.pos + self.rng.disc().scale(4.0);
            cmds.push(Command::Move { to: obs.clamp(to) });
            return cmds;
        }

        // 10. Storming: guns first, from indirect range; then onto the
        // depot. No scooting.
        if let Some(tgt) = self.storm {
            if stationary {
                if let Some(g) = guns.iter().min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap()) {
                    let d = me.pos.dist(g.pos);
                    if d > INDIRECT_MAX_RANGE * 0.9 || d < INDIRECT_MIN_RANGE + 1.0 {
                        // A firing position on the gun: inside indirect
                        // range, outside its own.
                        let to = g.pos + (me.pos - g.pos).norm().scale(INDIRECT_MIN_RANGE + 6.0);
                        cmds.push(Command::Move { to: obs.clamp(to) });
                    }
                } else if me.pos.dist(tgt) > DEPOT_RADIUS * 0.6 {
                    cmds.push(Command::Move { to: tgt });
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

// ------------------------------------------------------------- defender

/// The brief's defensive game, as the simplest doctrine that plays it
/// (roadmap step 6d; it grew out of step 6c's Fortifier, which is this
/// brain with `fortify` off plus the rings):
///
///   1. Never leave. The post is a tile off the home depot on the side
///      away from the map's centre, so the rings have room.
///   2. Two interceptors up; four while hostile drones are near.
///   3. Eyes on the approach: one recon over the gate direction,
///      `WATCH` tiles out, so the gun sees what comes while it is
///      still at its own standoff.
///   4. Shoot what is seen: direct inside 10 tiles, indirect from 12 to
///      60, led.
///   5. The squad, in this order, each when the squad is aboard and the
///      material is there: capture the neutral or dead pillbox on the
///      approach (the generated map puts one a third of the way in on
///      every spoke); repair an owned pillbox below half; the moat, a
///      gateless ring of mines at `MOAT_RADIUS` (first: cheap, quick,
///      and the thing that stops a tank; built later, the 6c proof
///      turned the other way, with the attacker walking through the
///      gap onto the depot); build a pillbox at the mouth of the
///      walls, `MOUTH` tiles out along the gate direction; lay the
///      road from the depot to it, so the squad can reach it; mine the
///      approach beyond it; the walls, a ring one tile outside the
///      moat with the gate toward the centre. Forest for material, the
///      nearest first; every refusal remembered.
///   6. Allies. While its side is small a defender proposes, once a
///      minute, to recruit the owner of the nearest depot that is not
///      its side's, and it accepts any proposal from a player who has
///      never hurt it. The three-way rule decides the rest.
///
/// With `fortify` off it does everything but the squad's work: the
/// control group for the proofs.
pub struct Defender {
    name: String,
    last_think: Tick,
    pub fortify: bool,
    home: Option<V2>,
    /// Tiles already ordered: the Observation carries no map, so the
    /// brain remembers rather than looks.
    wall_tries: Vec<(i32, i32)>,
    road_tries: Vec<(i32, i32)>,
    mouth_tries: Vec<(i32, i32)>,
    approach_tries: Vec<(i32, i32)>,
    /// Tiles the world refused to build on, from its own Rejected
    /// events; and forest tiles it was told to fell first.
    blocked: Vec<(i32, i32)>,
    fell: Vec<V2>,
    /// Pillboxes the squad could not reach.
    unreachable: Vec<EntityId>,
    /// Who has shot this tank: never an ally.
    hurt_by: Vec<PlayerId>,
    last_proposal: Tick,
    /// Where the trouble is: the depot of whoever shot us last, or the
    /// last hostile tank seen. The recon watches there.
    watch: Option<V2>,
    /// The first minute: who may be shot yet.
    grace: Grace,
    /// Sidestepping under fire until this tick; the post rule waits.
    dodge_until: Tick,
    rng: Option<Rng>,
}

pub const MOAT_RADIUS: f32 = 3.0;
/// The mouth pillbox: outside the wall ring, inside pillbox range of
/// the depot, so a tank on the depot is under its gun.
pub const MOUTH: f32 = 6.0;
/// Where the recon watches from: `RECON_VISION` tiles each side of
/// this covers the approach from the moat to beyond an attacker's
/// standoff.
pub const WATCH: f32 = 16.0;
/// The approach mines, this far out along the gate direction.
pub const APPROACH: f32 = 8.0;
/// Seconds between a defender's proposals.
pub const PROPOSE_EVERY: f32 = 60.0;

impl Defender {
    pub fn new(name: &str, fortify: bool) -> Defender {
        Defender {
            name: name.into(),
            last_think: 0,
            fortify,
            home: None,
            wall_tries: Vec::new(),
            road_tries: Vec::new(),
            mouth_tries: Vec::new(),
            approach_tries: Vec::new(),
            blocked: Vec::new(),
            fell: Vec::new(),
            unreachable: Vec::new(),
            hurt_by: Vec::new(),
            grace: Grace::default(),
            last_proposal: 0,
            watch: None,
            dodge_until: 0,
            rng: None,
        }
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

    /// Candidate sites for the mouth pillbox: along the gate direction
    /// at `MOUTH`, then a little to either side, then further out.
    fn mouth_sites(home: V2, dir: V2) -> Vec<V2> {
        let h = dir.heading();
        let mut out = Vec::new();
        for r in [MOUTH, MOUTH + 1.0, MOUTH + 2.0] {
            for a in [0.0, 0.4, -0.4] {
                out.push(Map::tile_centre(home + V2::from_heading(h + a).scale(r)));
            }
        }
        out
    }

    /// The approach mines: three across the gate direction at
    /// `APPROACH`, the road's tile in the middle.
    fn approach_sites(home: V2, dir: V2) -> Vec<V2> {
        let h = dir.heading();
        [0.0, 0.25, -0.25].iter().map(|a| Map::tile_centre(home + V2::from_heading(h + a).scale(APPROACH))).collect()
    }
}

impl Brain for Defender {
    fn name(&self) -> &str {
        &self.name
    }
    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        let mut cmds = Vec::new();
        let Some(me) = obs.me.as_ref() else { return cmds };
        // Every tick, before the think-rate gate: a refusal lands in the
        // tick after the order, which is rarely a think tick. The world
        // says why an order failed, and the brain has no map to have
        // known. Forest in a ring is felled first (material, too);
        // anything else is left alone. And who shoots us is noted.
        for e in &obs.events {
            match e {
                Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Build { at, .. } }, why, .. } => {
                    if why.starts_with("forest") {
                        if !self.fell.iter().any(|f| f.tile() == at.tile()) {
                            self.fell.push(*at);
                        }
                    } else if !self.blocked.contains(&at.tile()) {
                        self.blocked.push(at.tile());
                    }
                }
                Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Capture { pillbox } }, why, .. } | Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Repair { target: pillbox } }, why, .. } => {
                    if why.contains("unreachable") && !self.unreachable.contains(pillbox) {
                        self.unreachable.push(*pillbox);
                    }
                }
                Event::TankDamaged { tank, by: Some(b), .. } if *tank == me.id && *b != obs.player && !obs.allied(*b) => {
                    if !self.hurt_by.contains(b) {
                        self.hurt_by.push(*b);
                    }
                    // Counter-battery by deduction: the shell came from
                    // the owner of that depot. Look there.
                    if let Some(d) = obs.depots.iter().filter(|d| d.owner == Some(*b)).min_by(|x, y| x.pos.dist(me.pos).partial_cmp(&y.pos.dist(me.pos)).unwrap()) {
                        self.watch = Some(d.pos);
                    }
                }
                _ => {}
            }
        }
        if !me.alive || obs.tick.saturating_sub(self.last_think) < (TICK_HZ / 4) as Tick {
            return cmds;
        }
        self.last_think = obs.tick;
        let home = *self.home.get_or_insert_with(|| obs.depots.iter().filter(|d| d.owner == Some(obs.player)).chain(obs.depots.iter().filter(|d| d.owner.map(|o| obs.allied(o)).unwrap_or(false))).map(|d| d.pos).min_by(|a, b| a.dist(me.pos).partial_cmp(&b.dist(me.pos)).unwrap()).unwrap_or(me.pos));
        let centre = obs.centre();
        let dir = (centre - home).norm();
        // 1. Sit by the depot, just off it so the squad's rings have room.
        // Under fire, sidestep inside the depot's radius for a few
        // seconds: a tank that stands still for a spotted shell's flight
        // is where the next one lands (the first 6d trace lost three
        // defenders to the opening salvo in twenty seconds). A short
        // step: the moat is a tile and a half beyond it, and a moat set
        // off by an attacker splashes its owner like anyone else (the
        // 6c proof turned the other way round when the step was longer).
        let post = home + (home - centre).norm().scale(1.0);
        let dodging = obs.tick < self.dodge_until;
        let just_hit = me.last_hit_tick > 0 && obs.tick.saturating_sub(me.last_hit_tick) <= TICK_HZ as Tick;
        if just_hit && !dodging && me.goal.is_none() {
            let rng = self.rng.get_or_insert_with(|| Rng::new(fnv1a64(self.name.as_bytes()) ^ (obs.player as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
            let to = obs.clamp(home + rng.disc().scale(MOAT_RADIUS - MINE_RADIUS - 0.3));
            self.dodge_until = obs.tick + 4 * TICK_HZ as Tick;
            cmds.push(Command::Move { to });
            return cmds;
        }
        if !dodging && me.pos.dist(post) > 1.5 && me.goal.is_none() {
            cmds.push(Command::Move { to: post });
            return cmds;
        }
        // 6. Allies. Answer what is on the table, then ask.
        // Only the one asked answers; a teammate's answer to the same
        // proposal in the same tick is "no such proposal". When two
        // defenders have proposed to each other in the same minute, the
        // lower seat answers and the higher lets its own proposal be
        // overtaken, or both answers land in one tick and the second
        // finds nothing to answer.
        // One answer a think: accepting one changes sides, and the world
        // then resolves the rest itself.
        let answer = obs.proposals.iter().filter(|p| p.from != obs.player && p.to == obs.player).find(|p| {
            let mutual = obs.proposals.iter().any(|q| q.from == obs.player && q.to == p.from);
            !(mutual && obs.player > p.from)
        });
        if let Some(p) = answer {
            cmds.push(Command::Answer { proposal: p.id, accept: !self.hurt_by.contains(&p.from) });
        }
        let my_side = obs.players.iter().filter(|x| x.alive && x.team == obs.my_team()).count();
        let asked = obs.proposals.iter().any(|p| p.from == obs.player);
        if my_side < 3 && !asked && obs.tick.saturating_sub(self.last_proposal) as f32 * DT >= PROPOSE_EVERY {
            let alive = |o: PlayerId| obs.players.iter().any(|x| x.id == o && x.alive);
            let neighbour = obs
                .depots
                .iter()
                .filter(|d| d.owner.map(|o| !obs.allied(o) && alive(o) && !self.hurt_by.contains(&o)).unwrap_or(false))
                .min_by(|a, b| a.pos.dist(home).partial_cmp(&b.pos.dist(home)).unwrap())
                .and_then(|d| d.owner);
            if let Some(o) = neighbour {
                cmds.push(Command::Propose { to: o, terms: Terms::Recruit });
                self.last_proposal = obs.tick;
            }
        }
        // 2. The bubble.
        let my_interceptors = obs.my_drones().filter(|d| d.kind == DroneKind::Interceptor).count();
        let hostile_air = obs.hostile_drones().filter(|d| d.pos.dist(me.pos) <= me.bubble + DETECT_MARGIN + 6.0).count();
        let want_station = if hostile_air > 0 { 4 } else { 2 };
        if my_interceptors < want_station && me.bay.interceptor > 0 {
            cmds.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
        }
        // 3. Eyes: on the approach from the centre until something
        // happens, then on the trouble (the last shooter's depot, or the
        // last hostile tank seen), short of it by the recon's own vision
        // so it stays out of a bubble. Moved by Retask when the trouble
        // moves.
        self.grace.update(obs);
        let grace = &self.grace;
        let enemy = obs.hostile_tanks().filter(|t| grace.fair(obs, t)).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap());
        if let Some(e) = enemy {
            self.watch = Some(e.pos);
        }
        let eye = match self.watch {
            Some(w) => {
                let d = (me.pos.dist(w) - (RECON_VISION - 2.0)).clamp(2.0, RECON_LINK * 0.7);
                obs.clamp(me.pos + (w - me.pos).norm().scale(d))
            }
            None => obs.clamp(home + dir.scale(WATCH)),
        };
        match obs.my_drones().find(|d| d.kind == DroneKind::Recon) {
            None => {
                if me.bay.recon > 0 {
                    cmds.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: eye, radius: 3.0 } });
                }
            }
            Some(r) => {
                if matches!(&r.task, DroneTask::Orbit { center, .. } if center.dist(eye) > 4.0) {
                    cmds.push(Command::Retask { drone: r.id, task: DroneTask::Orbit { center: eye, radius: 3.0 } });
                }
            }
        }
        // 4. The gun.
        if let Some(e) = enemy {
            let range = me.pos.dist(e.pos);
            if me.reload == 0 && me.shells > 0 {
                if range <= DIRECT_RANGE {
                    cmds.push(Command::FireDirect { at: e.pos });
                } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&range) {
                    let lead = V2::from_heading(e.heading).scale(e.speed * range / SHELL_SPEED_INDIRECT * 0.8);
                    let aim = e.pos + lead;
                    let aim = if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&me.pos.dist(aim)) { aim } else { e.pos };
                    cmds.push(Command::FireIndirect { at: aim });
                }
            }
        }
        // 5. The squad.
        if !self.fortify || me.pillboys.strength == 0 {
            return cmds;
        }
        if me.pillboys.deployed() || me.goal.is_some() {
            return cmds;
        }
        let material = me.pillboys.material;
        let reach = |p: &Pillbox| p.pos.dist(home) <= PILLBOY_REACH && !self.unreachable.contains(&p.id);
        // Capture: the approach's pillbox, neutral or dead, outside its
        // own range of the post (a live neutral one shoots the tank, not
        // the squad; but the squad's home must stay out of its ring).
        let capture = obs
            .pillboxes
            .iter()
            .filter(|p| reach(p) && !p.owner.map(|o| obs.allied(o)).unwrap_or(false) && (p.owner.is_none() || !p.alive()) && (!p.alive() || p.pos.dist(me.pos) > PILLBOX_RANGE + 1.0))
            .min_by(|a, b| a.pos.dist(home).partial_cmp(&b.pos.dist(home)).unwrap());
        if let Some(p) = capture {
            cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: p.id } });
            return cmds;
        }
        // Repair: an owned pillbox below half, when there is material.
        let repair = obs.pillboxes.iter().filter(|p| reach(p) && p.owner == Some(obs.player) && p.health < PILLBOX_HEALTH * 0.5).min_by(|a, b| a.health.partial_cmp(&b.health).unwrap());
        if let Some(p) = repair {
            if material >= PILLBOX_REPAIR_COST {
                cmds.push(Command::Deploy { task: PillboyTask::Repair { target: p.id } });
                return cmds;
            }
        }
        // Material. Forest tiles in the rings wait until there is room
        // for their material: the first version re-ordered a mine onto
        // forest, was told to fell it, could not (material full), and
        // did it again four times a second for three minutes.
        if material + HARVEST_MATERIAL <= PILLBOY_MATERIAL_MAX {
            if let Some(tree) = self.fell.first().copied() {
                self.fell.remove(0);
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: tree } });
                return cmds;
            }
        }
        let known: Vec<(i32, i32)> = obs.mines.iter().filter(|m| m.owner == obs.player).map(|m| m.pos.tile()).collect();
        let forest_tiles: Vec<(i32, i32)> = self.fell.iter().map(|f| f.tile()).collect();
        let open = |p: &V2| !self.blocked.contains(&p.tile()) && !forest_tiles.contains(&p.tile());
        // The mouth pillbox: one, live, ours, near the mouth.
        let has_mouth = obs.pillboxes.iter().any(|p| p.owner == Some(obs.player) && p.alive() && p.pos.dist(home) <= MOUTH + 3.0 && p.pos.dist(home) >= MOAT_RADIUS + 1.0);
        let mouth_site = if has_mouth { None } else { Self::mouth_sites(home, dir).into_iter().find(|p| open(p) && !self.mouth_tries.contains(&p.tile()) && !obs.pillboxes.iter().any(|b| b.pos.dist(*p) < PILLBOX_SPACING)) };
        // The road starts outside the depot's own radius (nothing is
        // built on a depot) and runs to the mouth.
        let road: Vec<V2> = (3..(MOUTH as i32)).map(|r| Map::tile_centre(home + dir.scale(r as f32))).filter(|p| open(p) && !self.road_tries.contains(&p.tile())).collect();
        let approach: Vec<V2> = Self::approach_sites(home, dir).into_iter().filter(|p| open(p) && !known.contains(&p.tile()) && !self.approach_tries.contains(&p.tile())).collect();
        let moat: Vec<V2> = Self::ring(home, MOAT_RADIUS, centre, false).into_iter().filter(|p| !known.contains(&p.tile()) && open(p)).collect();
        let walls: Vec<V2> = Self::ring(home, MOAT_RADIUS + 1.0, centre, true).into_iter().filter(|p| open(p) && !self.wall_tries.contains(&p.tile())).collect();
        // What the next job costs; harvest when short, wait when there
        // is no forest in sight (the depot's trickle will do).
        let need = if moat.is_empty() && mouth_site.is_some() { PILLBOX_BUILD_COST } else { 1 };
        if material < need || (material < 4 && !moat.is_empty() && material < moat.len() as u8) {
            if let Some(tree) = obs.forest.first() {
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: *tree } });
            }
            return cmds;
        }
        // The moat first: cheap, quick, and the thing that stops a tank.
        if let Some(at) = moat.first() {
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, at: *at } });
            return cmds;
        }
        if let Some(at) = mouth_site {
            self.mouth_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Pillbox, at } });
            return cmds;
        }
        if let Some(at) = road.first().copied() {
            self.road_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at } });
            return cmds;
        }
        if let Some(at) = approach.first().copied() {
            self.approach_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, at } });
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

    /// A RuleBot answers a person's proposal: yes, unless that person has
    /// hurt it. Nine seats in three sides; seat 0 is a person.
    #[test]
    fn a_rulebot_answers_a_person() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(9) }.with_teams(3);
        for hurt in [false, true] {
            let mut w = World::new(sc.clone());
            w.pillboxes.clear();
            w.players[0].is_bot = false;
            let to = (1..9u8).find(|p| w.team_of(*p) != w.team_of(0)).unwrap();
            w.step(&[Order { player: 0, cmd: Command::Propose { to, terms: Terms::Join } }]);
            assert!(w.events.iter().any(|e| matches!(e, Event::Proposed { .. })), "{:?}", w.events);
            let mut bot = RuleBot::new(7, "rulebot");
            let tank = w.players[to as usize].tank;
            let mut resolved = None;
            for i in 0..40 {
                let mut obs = w.observe(to);
                if hurt && i == 0 {
                    obs.events.push(Event::TankDamaged { tank, by: Some(0), damage: 1.0, armor_left: 50.0 });
                }
                let orders: Vec<Order> = bot.think(&obs).into_iter().map(|cmd| Order { player: to, cmd }).collect();
                w.step(&orders);
                resolved = resolved.or_else(|| w.events.iter().find_map(|e| if let Event::ProposalResolved { accepted, .. } = e { Some(*accepted) } else { None }));
            }
            assert_eq!(resolved, Some(!hurt), "hurt {}: the bot answered {:?}", hurt, resolved);
        }
    }

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
                let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = vec![(0, Box::new(Defender::new("fort", fortify))), (1, Box::new(Attacker { target: depot, start: grace, last: 0 }))];
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

    /// PLAYABILITY.md item 1, the first minute: a newcomer who sits at
    /// the start reading the help is not shelled by the bots for the
    /// first minute. Seat 0 gives no orders at all; fifteen rule bots
    /// play round it on three seeds. Nothing of a player's may touch
    /// seat 0's tank before GRACE_SECONDS, and the bots must still have
    /// fought each other, so the proof is not of bots that do nothing.
    #[test]
    fn a_newcomer_gets_a_first_minute() {
        let grace = GRACE_SECONDS as Tick * TICK_HZ as Tick;
        let mut first_hits = Vec::new();
        for seed in [0x7EAD_C0DEu64, 0x1234, 0xBEEF] {
            let scenario = Scenario { seed, swarm_interval: 0, time_limit: grace + 30 * TICK_HZ as Tick, ..Scenario::bots(16) };
            let mut w = World::new(scenario);
            let mine = w.players[0].tank;
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (1..16u8).map(|p| (p as PlayerId, Box::new(RuleBot::new(seed ^ p as u64, &seat_name(p as usize))) as Box<dyn Brain>)).collect();
            let mut first_hit: Option<Tick> = None;
            let mut fired = 0u32;
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    match e {
                        Event::TankDamaged { tank, by: Some(p), .. } if *tank == mine && *p != SWARM_OWNER && *p != NEUTRAL => {
                            first_hit.get_or_insert(w.tick);
                        }
                        Event::ShellLaunched { .. } => fired += 1,
                        _ => {}
                    }
                }
            }
            eprintln!("seed {:#x}: seat 0 first hit by a player at {:?} (grace ends {}), shells fired {}", seed, first_hit, grace, fired);
            assert!(fired > 10, "the bots barely fought: {} shells", fired);
            if let Some(t) = first_hit {
                assert!(t >= grace, "seed {:#x}: the sitting newcomer was hit at tick {}, inside the first minute", seed, t);
            }
            first_hits.push(first_hit);
        }
    }

    /// Roadmap step 6d's proof, as it came out: sixteen bots, four
    /// attacking, over ten seeds of five minutes. The match lasts to
    /// the time limit, the defended depots stay defended, and the
    /// defenders outlive the attackers. The roadmap predicted that
    /// most kills would be pillbox kills; they are shell kills, both
    /// ways, because the gun out-ranges the pillbox 60 tiles to 8 and
    /// an attacker that storms a prepared depot is one that has
    /// already lost the duel. The split is printed so the trace of a
    /// run shows it.
    #[test]
    fn defenders_hold() {
        let seeds = 10u64;
        let limit = 5 * 60 * TICK_HZ as Tick;
        let mut to_the_limit = 0u32;
        let mut alive = (0u32, 0u32); // attackers, defenders
        let mut attacker_depots = 0u32;
        let mut defender_depots = 0u32;
        let mut kills = std::collections::BTreeMap::new();
        let mut pillbox_shots = 0u32;
        let mut built = 0u32;
        for seed in 0..seeds {
            let scenario = Scenario { seed: 0x6D6D_0000 + seed * 7919, time_limit: limit, ..Scenario::bots(16) };
            let mut w = World::new(scenario);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..16u8)
                .map(|i| {
                    let b: Box<dyn Brain> = if i < 4 { Box::new(RuleBot::new(5000 + seed * 16 + i as u64, "attacker")) } else { Box::new(Defender::new("defender", true)) };
                    (i, b)
                })
                .collect();
            let mut pending: std::collections::HashMap<EntityId, &str> = std::collections::HashMap::new();
            let mut last: std::collections::HashMap<EntityId, &str> = std::collections::HashMap::new();
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    match e {
                        Event::StrikeHit { tank, .. } => {
                            pending.insert(*tank, "strike");
                        }
                        Event::PillboxFired { tank, .. } => {
                            pillbox_shots += 1;
                            pending.insert(*tank, "pillbox");
                        }
                        Event::MineExploded { hits, .. } => {
                            for h in hits {
                                pending.insert(*h, "mine");
                            }
                        }
                        Event::TankDamaged { tank, .. } => {
                            let c = pending.remove(tank).unwrap_or("shell");
                            last.insert(*tank, c);
                        }
                        Event::TankDestroyed { tank, .. } => {
                            *kills.entry(last.get(tank).copied().unwrap_or("?")).or_insert(0u32) += 1;
                        }
                        Event::PillboxBuilt { .. } => built += 1,
                        _ => {}
                    }
                }
            }
            if w.tick >= limit {
                to_the_limit += 1;
            }
            alive.0 += w.players.iter().filter(|p| p.alive && p.id < 4).count() as u32;
            alive.1 += w.players.iter().filter(|p| p.alive && p.id >= 4).count() as u32;
            attacker_depots += w.depots.iter().filter(|d| d.owner.map(|o| o < 4).unwrap_or(false)).count() as u32;
            defender_depots += w.depots.iter().filter(|d| d.owner.map(|o| o >= 4).unwrap_or(false)).count() as u32;
        }
        eprintln!(
            "defenders_hold over {} seeds: to the time limit {}; alive attackers {} of {}, defenders {} of {}; depots at the end: defenders {} of their {}, attackers {} (their own {}); kills by cause {:?}; pillbox shots {}; pillboxes built {}",
            seeds,
            to_the_limit,
            alive.0,
            4 * seeds,
            alive.1,
            12 * seeds,
            defender_depots,
            12 * seeds,
            attacker_depots,
            4 * seeds,
            kills,
            pillbox_shots,
            built
        );
        assert!(to_the_limit * 2 > seeds as u32, "the match did not last: {} of {}", to_the_limit, seeds);
        assert!(alive.1 * 4 > alive.0 * 12, "defenders did not outlive attackers: {} of {} vs {} of {}", alive.1, 12 * seeds, alive.0, 4 * seeds);
        assert!(defender_depots * 10 >= 12 * seeds as u32 * 6, "the defenders lost their depots: {} of {}", defender_depots, 12 * seeds);
        assert!(built >= seeds as u32, "the defenders barely built pillboxes: {}", built);
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
