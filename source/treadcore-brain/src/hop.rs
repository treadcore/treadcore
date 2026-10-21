//! Island hopping: a bot that has taken everything it can reach goes
//! for what it cannot, across the water.
//!
//! The first naval match had a tank sit on its island for the rest of
//! the match once the pillbox there was taken: every depot and pillbox
//! left was over the sea, the world refused each drive to one as
//! unreachable, and the bot waited for something to come to it. This is
//! what it does instead, shared by the RuleBot and the Commander:
//!
//!   1. Idle for `HOP_IDLE` seconds (standing, the squad aboard, nothing
//!      hostile in sight) with something untaken on the map: a depot not
//!      of its side, or a pillbox or structure that is nobody's or dead.
//!      The nearest becomes the target.
//!   2. Drive to it. The world routes by land; if it says the target is
//!      unreachable, it is over water.
//!   3. Over water: board a moored boat within `BOAT_WALK` if there is
//!      one (`BOAT_NEAR` with a launch in hand); else drive to the bank
//!      of the launch site (the observation's water beside the bank)
//!      nearest the way to the target and launch from it
//!      (`Command::Embark`, which needs no material). If the bank is
//!      refused or not reached in `BANK_FOR`, the squad builds a boat
//!      there, for five material: short of it, restock at a depot or
//!      fell forest first.
//!   4. Afloat, sail for the target itself: the world routes over the
//!      water and up the nearest beach to it.
//!   5. Ashore again, the doctrine takes over within `ARRIVED` of the
//!      target; still unreachable from the new shore, back to 3. A
//!      target that took `MAX_BOATS` boats, or that even the sea does
//!      not reach, is given up.

use crate::status::HopAct;
use treadcore_sim::*;

/// Seconds of idling before a bot goes looking over the water.
pub const HOP_IDLE: f32 = 20.0;
/// A moored boat this near is boarded rather than a new one built.
pub const BOAT_WALK: f32 = 30.0;
/// With a boat's material and a bank in sight, only this near.
pub const BOAT_NEAR: f32 = 6.0;
/// Within this of the target the doctrine has it again: a depot is
/// driven onto, a pillbox is in the squad's reach.
pub const ARRIVED: f32 = 12.0;
/// Boats spent on one target before it is given up.
pub const MAX_BOATS: u8 = 3;
/// A target given up on is tried again after this many seconds: the
/// squad may have material by then, a boat may lie moored, a forest
/// may be in sight (the quad's guard gave up on its post at 100 s and
/// drove at it, refused, for the rest of the match).
pub const TRIED_FOR: f32 = 60.0;
/// No forest in sight for a boat: the tank drives along looking for
/// one this long before it gives up.
pub const SCOUT_FOR: f32 = 40.0;
/// A depot of the side this near is where a boat's material comes
/// from, one a half second, rather than felling (four a tile, slowly,
/// and the naval shores were soon bare: the quad's bots spent a sixth
/// of the match looking for a tree). A forest this much nearer still
/// wins.
pub const RESTOCK_WALK: f32 = 40.0;
pub const FOREST_NEARER: f32 = 12.0;
/// A hostile tank this near pauses a hop: the fight is the doctrine's.
pub const FIGHT_NEAR: f32 = DIRECT_RANGE * 1.5;
/// A moored boat not reached in this many seconds is someone else's
/// shore's (the quad's leaders walked at one over the water for five
/// minutes).
pub const BOAT_FOR: f32 = 30.0;
/// A hostile tank with shells this near holds a launch: its indirect
/// gun reaches 60, but beyond about 40 a crossing boat is rarely seen.
pub const COVER_R: f32 = 40.0;
/// A launch is held at most this long, then goes anyway.
pub const HOLD_LAUNCH: f32 = 20.0;
/// Afloat, a beach in sight this near the target is landed on, the
/// least covered; none, the world's beach nearest the target.
pub const LANDING_NEAR: f32 = 25.0;
/// A bank not reached in this many seconds is given up for the squad's
/// build.
pub const BANK_FOR: f32 = 20.0;

/// A bridge being laid: its water tiles still to lay, in order; when the
/// last was laid; refusals since.
#[derive(Clone, Debug)]
pub struct Bridge {
    tiles: Vec<V2>,
    progress: Tick,
    fails: u8,
}

/// A crossing of this many water tiles or fewer is bridged, not sailed:
/// the naval map's narrow channels are 3 to 5, its wide ones 17 to 20.
pub const BRIDGE_MAX: usize = 20;
/// No tile laid in this long and the bridge is given up for a boat.
pub const BRIDGE_STALL: f32 = 40.0;
/// The road on from a bridge to its target, at most this many tiles.
pub const ROAD_ON: usize = 12;
/// The road on is given up after this long without a tile laid.
pub const ROAD_STALL: f32 = 20.0;

#[derive(Clone, Debug, Default)]
pub struct Hop {
    idle_since: Option<Tick>,
    target: Option<V2>,
    /// The world refused the drive: the target is over the water.
    by_sea: bool,
    boats: u8,
    last_order: Tick,
    /// Targets given up on, and launch sites the world refused.
    tried: Vec<((i32, i32), Tick)>,
    bad_launch: crate::memory::Forget<(i32, i32)>,
    /// The depot driven to for material, and depots the world refused
    /// a drive to (over the water).
    restock: Option<V2>,
    /// The moored boat walked to, since when; and boats given up on
    /// (refused, over the water, or not reached in `BOAT_FOR`).
    boarding: Option<(EntityId, V2, Tick)>,
    bad_boat: crate::memory::Forget<EntityId>,
    bad_depot: crate::memory::Forget<(i32, i32)>,
    /// The world refused a launch from here: the squad builds instead
    /// until then.
    no_embark_until: Tick,
    /// The trip was asked for (`send`): a Commander's errand over the
    /// water (home, a post), not a hop of the bot's own.
    sent: bool,
    /// The tick of the last think or hear, for `tried`.
    now: Tick,
    /// Looking for a forest to fell for a boat: since when, and which
    /// way.
    scouting: Option<(Tick, f32)>,
    /// The launch site driven to, the bank to launch from, since when.
    banking: Option<(V2, V2, Tick)>,
    /// Banks refused or not reached: the squad builds from those.
    bad_bank: Vec<(i32, i32)>,
    /// Holding a launch under cover since then (`HOLD_LAUNCH`).
    held: Option<Tick>,
    /// The bridge being laid; targets whose bridge was given up (by boat
    /// for those); the road on from a bridge, being laid.
    bridge: Option<Bridge>,
    no_bridge: crate::memory::Forget<(i32, i32)>,
    road: Option<Vec<V2>>,
    /// The road on's tiles left, and since when (`ROAD_STALL`).
    road_progress: Option<(usize, Tick)>,
    /// Afloat, the beach chosen to land on, and when.
    landing_at: Option<(V2, Tick)>,
    /// Start no bridges: a fleet's wingman helps its leader's and starts
    /// none of its own (two of a fleet began the same one).
    pub no_bridges: bool,
    /// The bridge is a teammate's, taken from the team board (roadmap
    /// 12f, `BRIDGE_JOBS`): laid, the errand is over.
    adopted: bool,
    /// The fleet lays the bridge with it (commander.rs): out of
    /// material, it waits at the end rather than fetches.
    pub helpers: usize,
    /// What it is doing, for `status`, and its code (status.rs).
    pub doing: String,
    pub act: crate::status::HopAct,
    /// Crossings asked of the map, kept, failures too (facts.rs,
    /// roadmap 12b); the islands, for pricing a crossing.
    pub facts: crate::facts::MapFacts,
    /// Price a crossing, boat against bridge (roadmap 12f2); None
    /// bridges every gap of `BRIDGE_MAX` or less, as before.
    pub pricing: Option<crate::weights::Settings>,
}

impl Hop {
    /// A new tank at the start: whatever it was doing is over; what it
    /// gave up on stays given up.
    pub fn reset(&mut self) {
        self.target = None;
        self.by_sea = false;
        self.boats = 0;
        self.idle_since = None;
        self.sent = false;
        self.bridge = None;
        self.road = None;
        self.adopted = false;
    }

    /// Say what it is doing: the code and the words (status.rs).
    fn say(&mut self, act: HopAct, text: String) {
        self.act = act;
        self.doing = text;
    }

    /// Laying a teammate's bridge taken from the board.
    pub fn adopted(&self) -> bool {
        self.adopted
    }

    /// A hop is under way: it has a target.
    pub fn active(&self) -> bool {
        self.target.is_some()
    }

    /// The next tile of the bridge, or of the road on from it, being
    /// laid: the fleet's job (commander.rs).
    pub fn bridge_front(&self) -> Option<V2> {
        self.bridge.as_ref().and_then(|b| b.tiles.first().copied()).or_else(|| self.road.as_ref().and_then(|r| r.first().copied()))
    }

    /// The bridge being laid, for the team board: its water tiles still
    /// to lay, in order, and what it crosses to.
    pub fn bridge_job(&self) -> Option<(Vec<V2>, V2)> {
        match (self.bridge.as_ref(), self.target) {
            (Some(b), Some(t)) if !b.tiles.is_empty() => Some((b.tiles.clone(), t)),
            _ => None,
        }
    }

    /// A bridge to `target` given up here lately: not one to take.
    pub fn bridge_refused(&self, target: V2, now: Tick) -> bool {
        self.no_bridge.has(&target.tile(), now)
    }

    /// Take a teammate's part-laid bridge from the board: lay the rest
    /// of `tiles` toward `target`, and stop there. False if a trip is
    /// under way or this bridge was given up here.
    pub fn adopt_bridge(&mut self, tiles: Vec<V2>, target: V2, now: Tick) -> bool {
        if self.target.is_some() || tiles.is_empty() || self.no_bridge.has(&target.tile(), now) {
            return false;
        }
        self.target = Some(target);
        self.by_sea = true;
        self.boats = 0;
        self.sent = true;
        self.adopted = true;
        self.idle_since = None;
        self.bridge = Some(Bridge { tiles, progress: now, fails: 0 });
        self.road = None;
        self.say(HopAct::Bridging, format!("bridging for the team to ({:.0},{:.0})", target.x, target.y));
        true
    }

    /// Ashore, the target over the water: about to launch.
    pub fn launching(&self) -> bool {
        self.target.is_some() && self.by_sea
    }

    /// An errand over the water (Paul's quad, 16 October: a guard whose
    /// post was across the sea drove at it for the whole match; a tank
    /// sent home to resupply sat on the wrong island at 25 armour): the
    /// world refused the drive as unreachable, so by sea at once. False
    /// if this target was given up on, or a trip is already under way.
    pub fn send(&mut self, to: V2) -> bool {
        if self.target.is_some() || self.given_up(to) {
            return false;
        }
        self.target = Some(to);
        self.by_sea = true;
        self.boats = 0;
        self.sent = true;
        self.say(HopAct::BySea, format!("by sea for ({:.0},{:.0})", to.x, to.y));
        true
    }

    /// Given up on lately (`TRIED_FOR`).
    pub fn given_up(&self, at: V2) -> bool {
        self.tried.iter().any(|(t, _)| *t == at.tile())
    }

    /// How long what it gave up on stays given up (memory.rs).
    pub fn set_forget(&mut self, keep: Tick) {
        self.bad_launch.keep = keep;
        self.bad_boat.keep = keep;
        self.bad_depot.keep = keep;
        self.no_bridge.keep = keep;
    }

    /// The trip under way is an errand asked for (`send`), not a hop of
    /// the bot's own.
    pub fn sent(&self) -> bool {
        self.sent && self.target.is_some()
    }

    /// The target given up for `TRIED_FOR`, said why, and every part of
    /// the trip (boat, bank, bridge, road) dropped.
    fn give_up(&mut self, why: &str) {
        if let Some(t) = self.target.take() {
            self.tried.push((t.tile(), self.now));
            self.say(HopAct::GaveUp, format!("gave up on ({:.0},{:.0}): {}", t.x, t.y, why));
        }
        self.by_sea = false;
        self.boats = 0;
        self.idle_since = None;
        self.sent = false;
        self.adopted = false;
        self.scouting = None;
        self.banking = None;
        self.bad_bank.clear();
        self.held = None;
        self.bridge = None;
        self.road = None;
    }

    /// The land beside a launch site, on the tank's side of it.
    fn bank(site: V2, from: V2) -> V2 {
        site + (from - site).norm().scale(1.0)
    }

    /// One think of the bridge: the next water tile laid from the end so
    /// far, the tank at that end, the material felled or fetched. None
    /// when it is done (by land from now) or given up (by boat).
    fn bridging(&mut self, obs: &Observation, me: &Tank, map: &Map, target: V2) -> Option<Vec<Command>> {
        let b = self.bridge.as_mut()?;
        let before = b.tiles.len();
        b.tiles.retain(|t| map.at(*t) == Terrain::Water);
        if b.tiles.len() < before {
            b.progress = obs.tick;
            b.fails = 0;
        }
        let Some(&next) = b.tiles.first() else {
            self.bridge = None;
            // A teammate's bridge, taken from the board: laid, the errand
            // is over; the one who wanted the crossing drives it.
            if self.adopted {
                self.adopted = false;
                self.target = None;
                self.sent = false;
                self.by_sea = false;
                self.say(HopAct::TeamBridgeDone, format!("bridge to ({:.0},{:.0}) finished for the team", target.x, target.y));
                return None;
            }
            // Done: by land, and the road on to the target.
            self.by_sea = false;
            self.road = Some(Self::road_line(map, me.pos, target));
            self.road_progress = None;
            self.say(HopAct::BridgeBuilt, format!("bridge built for ({:.0},{:.0})", target.x, target.y));
            return None;
        };
        let left = b.tiles.len();
        // A tile laid (by anyone) is progress; waiting is not. A quad's
        // leader once waited out the match on a part-laid bridge, its
        // wait counted as progress. Fetching material takes longer.
        let limit = if me.pillboys.material < ROAD_COST { BRIDGE_STALL * 2.0 } else { BRIDGE_STALL };
        if obs.tick.saturating_sub(b.progress) as f32 * DT > limit || b.fails >= 3 {
            self.no_bridge.put(target.tile(), obs.tick);
            self.bridge = None;
            self.say(HopAct::BridgeGivenUp, format!("bridge to ({:.0},{:.0}) given up: by boat", target.x, target.y));
            return None;
        }
        let mut cmds = Vec::new();
        let ready = obs.tick.saturating_sub(self.last_order) >= TICK_HZ as Tick;
        let squad = &me.pillboys;
        if !squad.aboard {
            self.say(HopAct::Bridging, format!("bridging to ({:.0},{:.0}): the squad is laying it, {} tiles left", target.x, target.y, left));
            return Some(cmds);
        }
        if squad.strength == 0 {
            self.no_bridge.put(target.tile(), obs.tick);
            self.bridge = None;
            return None;
        }
        if squad.material < ROAD_COST && self.helpers > 0 {
            self.say(HopAct::Bridging, format!("bridging to ({:.0},{:.0}): the fleet lays the next tile, {} left", target.x, target.y, left));
            return Some(Vec::new());
        }
        if squad.material < ROAD_COST {
            // Material: a forest in reach first, else a depot of the side.
            let forest = obs.forest.iter().filter(|f| f.dist(me.pos) <= 20.0).min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos))).copied();
            let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
            let depot = obs.depots.iter().filter(|d| ours(d.owner) && d.pos.dist(me.pos) <= RESTOCK_WALK).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))).map(|d| d.pos);
            self.say(HopAct::Bridging, format!("bridging to ({:.0},{:.0}): fetching material, {} tiles left", target.x, target.y, left));
            if ready {
                match (forest, depot) {
                    (Some(f), _) => cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: f } }),
                    (None, Some(d)) if me.pos.dist(d) > DEPOT_RADIUS * 0.5 => cmds.push(Command::Move { to: d }),
                    (None, Some(_)) => {}
                    (None, None) => {
                        self.no_bridge.put(target.tile(), obs.tick);
                        self.bridge = None;
                        return None;
                    }
                }
                self.last_order = obs.tick;
            }
            return Some(cmds);
        }
        // Stand at the end so far: the ground beside the next tile on my
        // side.
        let (x, y) = next.tile();
        let stand = [(1, 0), (-1, 0), (0, 1), (0, -1)]
            .iter()
            .map(|(dx, dy)| V2::new((x + dx) as f32 + 0.5, (y + dy) as f32 + 0.5))
            .filter(|p| map.in_bounds(*p) && map.at(*p).passable())
            .min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos)));
        let Some(stand) = stand else {
            if let Some(b) = self.bridge.as_mut() {
                b.fails += 1;
            }
            return Some(cmds);
        };
        self.say(HopAct::Bridging, format!("bridging to ({:.0},{:.0}): {} tiles left", target.x, target.y, left));
        if ready {
            if me.pos.dist(stand) > 1.2 {
                cmds.push(Command::Move { to: stand });
            } else {
                cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at: next } });
            }
            self.last_order = obs.tick;
        }
        Some(cmds)
    }

    /// The road on from a bridge to the target: the grass and rubble
    /// tiles on the straight line between, `ROAD_ON` at most, the last
    /// few short of the target left for it.
    fn road_line(map: &Map, from: V2, to: V2) -> Vec<V2> {
        let d = from.dist(to);
        let steps = d.ceil() as usize;
        let mut out: Vec<V2> = Vec::new();
        for k in 1..steps {
            let p = Map::tile_centre(from + (to - from).scale(k as f32 / d));
            if p.dist(to) < 2.5 || out.len() >= ROAD_ON {
                break;
            }
            match map.at(p) {
                Terrain::Grass | Terrain::Rubble => {
                    if !out.iter().any(|q| q.tile() == p.tile()) {
                        out.push(p);
                    }
                }
                Terrain::Road | Terrain::Bridge | Terrain::Depot | Terrain::Forest => {}
                // Water or a wall ahead: the road ends at the shore. Past
                // it, the tank could not reach the next tile (a quad sat
                // at its depot 440 s, its drive to the far shore refused).
                _ => break,
            }
        }
        out
    }

    /// Lay the road on, a tile ahead of the tank each time; true while
    /// it does. Out of material, or blocked, it is left as far as it got.
    fn lay_road(&mut self, obs: &Observation, me: &Tank, map: &Map, cmds: &mut Vec<Command>) -> bool {
        let Some(road) = self.road.as_mut() else { return false };
        road.retain(|t| matches!(map.at(*t), Terrain::Grass | Terrain::Rubble));
        let Some(&next) = road.first() else {
            self.road = None;
            return false;
        };
        let left = road.len();
        // No tile laid for a while: left as far as it got.
        match self.road_progress {
            Some((n, _)) if n != left => self.road_progress = Some((left, obs.tick)),
            Some((_, t)) if obs.tick.saturating_sub(t) as f32 * DT > ROAD_STALL => {
                self.road = None;
                self.road_progress = None;
                return false;
            }
            None => self.road_progress = Some((left, obs.tick)),
            _ => {}
        }
        let squad = &me.pillboys;
        if squad.material < ROAD_COST && squad.strength > 0 && self.helpers > 0 {
            self.say(HopAct::RoadOn, format!("laying the road on from the bridge: the fleet lays the next tile, {} left", left));
            return true;
        }
        if squad.material < ROAD_COST || squad.strength == 0 {
            self.road = None;
            return false;
        }
        self.say(HopAct::RoadOn, format!("laying the road on from the bridge: {} tiles left", left));
        if !squad.aboard {
            return true;
        }
        if obs.tick.saturating_sub(self.last_order) < TICK_HZ as Tick {
            return true;
        }
        self.last_order = obs.tick;
        if me.pos.dist(next) > 1.6 {
            // Up to it, not onto it: a tile with a tank on it is refused.
            let to = next + (me.pos - next).norm().scale(1.0);
            cmds.push(Command::Move { to });
        } else if me.pos.tile() == next.tile() {
            cmds.push(Command::Move { to: next + (me.pos - next).norm().scale(1.0) + V2::new(0.3, 0.3) });
        } else {
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at: next } });
        }
        true
    }

    /// The beach to land on for `target`, if one in sight is within
    /// `LANDING_NEAR` of it: distance to the target plus its danger, the
    /// choice kept unless another is clearly better (`landing_at`).
    fn landing(&mut self, obs: &Observation, target: V2) -> Option<V2> {
        let hostile = |o: Option<PlayerId>| o.map(|o| o != obs.player && !obs.allied(o)).unwrap_or(false);
        let guns: Vec<(V2, f32)> = obs
            .hostile_tanks()
            .filter(|t| t.alive && t.shells > 0)
            .map(|t| (t.pos, DIRECT_RANGE))
            .chain(obs.pillboxes.iter().filter(|p| p.alive() && hostile(p.owner)).map(|p| (p.pos, PILLBOX_RANGE)))
            .collect();
        let danger = |b: V2| -> f32 {
            guns.iter()
                .map(|(g, reach)| {
                    let d = g.dist(b);
                    if d <= reach + 4.0 {
                        80.0
                    } else if d < INDIRECT_MAX_RANGE {
                        (INDIRECT_MAX_RANGE - d) * 0.6
                    } else {
                        0.0
                    }
                })
                .fold(0.0, f32::max)
        };
        let score = |b: V2| b.dist(target) + danger(b);
        let best = obs.shore.iter().filter(|b| b.dist(target) <= LANDING_NEAR).min_by(|a, b| score(**a).total_cmp(&score(**b))).copied()?;
        let keep = self.landing_at.map(|(b, t)| (b, t)).filter(|(b, t)| obs.shore.iter().any(|x| x.dist(*b) < 1.5) && (obs.tick.saturating_sub(*t) as f32 * DT < 2.0 || score(best) + 6.0 >= score(*b)));
        match keep {
            Some((b, _)) => Some(b),
            None => {
                self.landing_at = Some((best, obs.tick));
                Some(best)
            }
        }
    }

    /// What covers the water here, if anything (`COVER_R`).
    fn covered(obs: &Observation, me: &Tank) -> Option<String> {
        let hostile = |o: PlayerId| o != obs.player && !obs.allied(o);
        if let Some(t) = obs.hostile_tanks().filter(|t| t.alive && t.shells > 0 && t.pos.dist(me.pos) <= COVER_R).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))) {
            return Some(format!("a hostile tank {:.0} tiles off", t.pos.dist(me.pos)));
        }
        if obs.shells.iter().any(|s| s.kind == ShellKind::Indirect && hostile(s.owner) && s.impact.dist(me.pos) <= 8.0) {
            return Some("shells coming down near".into());
        }
        if obs.pillboxes.iter().any(|p| p.alive() && p.owner.map(hostile).unwrap_or(false) && p.pos.dist(me.pos) <= PILLBOX_RANGE + 4.0) {
            return Some("a hostile pillbox on the bank".into());
        }
        None
    }

    /// Something untaken that is not where the tank is: the nearest.
    fn pick(&self, obs: &Observation, me: &Tank) -> Option<V2> {
        let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
        let depots = obs.depots.iter().filter(|d| !ours(d.owner)).map(|d| d.pos);
        let boxes = obs.pillboxes.iter().filter(|p| !ours(p.owner) && (p.owner.is_none() || !p.alive())).map(|p| p.pos);
        let works = obs.structures.iter().filter(|x| !ours(x.owner) && (x.owner.is_none() || !x.alive())).map(|x| x.pos);
        depots.chain(boxes).chain(works).filter(|p| p.dist(me.pos) > ARRIVED && !self.given_up(*p)).min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos)))
    }

    /// A depot or live pillbox of the side stands at `at`: the hop's
    /// target is taken.
    fn taken(obs: &Observation, at: V2) -> bool {
        let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
        obs.depots.iter().any(|d| d.pos.dist(at) < 0.5 && ours(d.owner)) || obs.pillboxes.iter().any(|p| p.pos.dist(at) < 0.5 && ours(p.owner) && p.alive())
    }

    /// Every tick, think or not: the world's answer to an order comes
    /// the tick after it, which is rarely a think tick.
    pub fn hear(&mut self, obs: &Observation, me: &Tank) {
        self.now = obs.tick;
        self.tried.retain(|(_, t)| obs.tick < t + (TRIED_FOR * TICK_HZ as f32) as Tick);
        for e in &obs.events {
            match e {
                Event::Rejected { player, cmd: Command::Move { to }, why } if *player == obs.player && self.target.map(|t| t.tile() == to.tile()).unwrap_or(false) && (why.contains("unreachable") || why == "water") => {
                    if me.boat {
                        self.give_up("not even by sea");
                    } else {
                        self.by_sea = true;
                    }
                }
                Event::Rejected { player, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Boat, at } }, .. } if *player == obs.player => self.bad_launch.put(at.tile(), obs.tick),
                Event::Rejected { player, cmd: Command::Move { to }, .. } if *player == obs.player && self.restock.map(|d| d.tile() == to.tile()).unwrap_or(false) => {
                    self.bad_depot.put(to.tile(), obs.tick);
                    self.restock = None;
                }
                Event::Rejected { player, cmd: Command::Move { to }, .. } if *player == obs.player && self.banking.map(|(_, b, _)| b.tile() == to.tile()).unwrap_or(false) => {
                    self.bad_bank.extend(self.banking.take().map(|(s, _, _)| s.tile()));
                }
                Event::Rejected { player, cmd: Command::Move { to }, .. } if *player == obs.player && self.boarding.map(|(_, at, _)| at.tile() == to.tile()).unwrap_or(false) => {
                    if let Some((id, _, _)) = self.boarding.take() {
                        self.bad_boat.put(id, obs.tick);
                    }
                }
                Event::Boarded { owner, .. } if *owner == obs.player => {
                    self.boats += 1;
                    self.boarding = None;
                    self.banking = None;
                }
                Event::Rejected { player, cmd: Command::Embark, .. } if *player == obs.player => self.no_embark_until = obs.tick + 10 * TICK_HZ as Tick,
                Event::Rejected { player, cmd: Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at } }, .. } if *player == obs.player => {
                    if let Some(b) = self.bridge.as_mut().filter(|b| b.tiles.first().map(|t| t.tile() == at.tile()).unwrap_or(false)) {
                        b.fails += 1;
                    }
                    if let Some(r) = self.road.as_mut() {
                        r.retain(|t| t.tile() != at.tile());
                    }
                }
                // A new shore: try by land again.
                Event::Landed { owner, .. } if *owner == obs.player => {
                    self.by_sea = false;
                    self.landing_at = None;
                }
                _ => {}
            }
        }
    }

    /// Every think. `busy` is the doctrine saying it has something to do
    /// (a storm, a resupply). Some(orders) while hopping: the caller
    /// sends them and nothing else of its own; None and the doctrine
    /// carries on.
    pub fn think(&mut self, obs: &Observation, me: &Tank, busy: bool, map: Option<&Map>) -> Option<Vec<Command>> {
        // A hostile in fighting reach, not merely in sight: since sight
        // grew (a recon drone sees 20 tiles) a far tank, afloat or over
        // the water, held every hop up.
        let enemy = obs.hostile_tanks().any(|t| t.alive && t.pos.dist(me.pos) <= FIGHT_NEAR);
        let Some(target) = self.target else {
            let idle = me.goal.is_none() && me.pillboys.aboard && !enemy && !busy && !me.boat;
            if !idle {
                self.idle_since = None;
                return None;
            }
            let since = *self.idle_since.get_or_insert(obs.tick);
            if (obs.tick.saturating_sub(since) as f32 * DT) < HOP_IDLE {
                return None;
            }
            self.target = Some(self.pick(obs, me)?);
            self.by_sea = false;
            self.boats = 0;
            return Some(Vec::new());
        };
        if (!self.sent && Self::taken(obs, target)) || (!me.boat && me.pos.dist(target) <= ARRIVED) {
            self.target = None;
            self.idle_since = None;
            self.sent = false;
            self.adopted = false;
            self.say(HopAct::Arrived, format!("arrived at ({:.0},{:.0})", target.x, target.y));
            return None;
        }
        if self.boats > MAX_BOATS {
            self.give_up("too many boats");
            return None;
        }
        // A fight ashore is the doctrine's; afloat there is no fighting.
        if enemy && !me.boat {
            return None;
        }
        let mut cmds = Vec::new();
        let ready = obs.tick.saturating_sub(self.last_order) >= TICK_HZ as Tick;
        let order = |c: Command, cmds: &mut Vec<Command>, last: &mut Tick| {
            if ready {
                cmds.push(c);
                *last = obs.tick;
            }
        };
        if me.boat {
            // An errand to a place that turned out to be sea (a leader
            // who was afloat): there is the end of it; the doctrine's
            // landfall takes the boat to a beach.
            if self.sent && me.pos.dist(target) <= ARRIVED * 0.5 {
                self.target = None;
                self.sent = false;
                self.say(HopAct::AtSea, format!("at ({:.0},{:.0}), which is sea", target.x, target.y));
                return None;
            }
            // The landing: of the beaches in sight near the target, the
            // one least covered by hostile guns (the doctrine's landfall
            // scale); none near, the target itself and the world's
            // nearest beach to it. One hit sinks a boat, and the hop sailed
            // at the target whatever stood on its beach.
            let landing = self.landing(obs, target);
            match landing {
                Some(b) => self.say(HopAct::Sailing, format!("sailing for ({:.0},{:.0}), {:.0} tiles, landing at ({:.0},{:.0})", target.x, target.y, me.pos.dist(target), b.x, b.y)),
                None => self.say(HopAct::Sailing, format!("sailing for ({:.0},{:.0}), {:.0} tiles", target.x, target.y, me.pos.dist(target))),
            }
            let to = landing.unwrap_or(target);
            let changed = self.landing_at.map(|(b, t)| b.tile() == to.tile() && t == obs.tick).unwrap_or(false);
            if me.goal.is_none() || changed {
                order(Command::Move { to }, &mut cmds, &mut self.last_order);
            }
            return Some(cmds);
        }
        if !self.by_sea {
            // Over a bridge of ours: the road on from its end to the
            // target, laid ahead of the tank as it drives.
            if let Some(map) = map {
                if self.lay_road(obs, me, map, &mut cmds) {
                    return Some(cmds);
                }
            }
            self.say(HopAct::Driving, format!("driving for ({:.0},{:.0})", target.x, target.y));
            if me.goal.is_none() {
                order(Command::Move { to: target }, &mut cmds, &mut self.last_order);
            }
            return Some(cmds);
        }
        // A bridge before a boat (Paul, 17 October: "the tanks should not
        // try to cross water ... build roads where it would drive"): with
        // the map, a crossing of `BRIDGE_MAX` water tiles or fewer is
        // bridged by the squad, a tile at a time from the bank.
        if let Some(map) = map {
            if self.bridge.is_none() && !self.no_bridges && !self.no_bridge.has(&target.tile(), obs.tick) {
                // Nothing is built beside a depot: no bridge through there.
                let by_depot = |p: V2| obs.depots.iter().any(|d| d.pos.dist(p) <= DEPOT_RADIUS + 0.5);
                let tiles = self.facts.crossing(map, me.pos, target, BRIDGE_MAX, by_depot, obs.tick).filter(|t| !t.is_empty());
                // Priced (LR 6.2): a long bridge, or one from an island
                // short of trees, loses to a boat.
                let tiles = match (&self.pricing, tiles) {
                    (Some(s), Some(t)) => {
                        self.facts.refresh(map, obs.tick);
                        let scarce = self.facts.scarcity(s, me.pos);
                        if crate::facts::bridge_it(s, t.len(), scarce) {
                            Some(t)
                        } else {
                            self.say(HopAct::CheaperByBoat, format!("{} tiles of water: cheaper by boat", t.len()));
                            self.no_bridge.put(target.tile(), obs.tick);
                            None
                        }
                    }
                    (_, t) => t,
                };
                if let Some(tiles) = tiles {
                    self.bridge = Some(Bridge { tiles, progress: obs.tick, fails: 0 });
                    self.road = None;
                }
            }
            if self.bridge.is_some() {
                if let Some(c) = self.bridging(obs, me, map, target) {
                    if self.adopted {
                        self.doing = format!("for the team: {}", self.doing);
                    }
                    cmds.extend(c);
                    return Some(cmds);
                }
            }
            // A teammate's bridge given up here: the errand was the
            // bridge, not the crossing; it stays on the board for another.
            if self.adopted && self.bridge.is_none() {
                self.adopted = false;
                self.target = None;
                self.sent = false;
                self.by_sea = false;
                return None;
            }
        }
        // A covered crossing: one hit sinks a boat, and the quad's boats
        // were sunk from beyond direct reach, a third of all deaths. With
        // a hostile gun in reach of the water here (a tank with shells
        // within `COVER_R`, a shell coming down near, a live pillbox on
        // the bank), hold the launch for up to `HOLD_LAUNCH` and go then
        // anyway: an errand cannot wait for ever.
        if let Some(why) = Self::covered(obs, me) {
            let since = *self.held.get_or_insert(obs.tick);
            if (obs.tick.saturating_sub(since) as f32 * DT) < HOLD_LAUNCH {
                self.say(HopAct::HoldingLaunch, format!("holding the launch: {}", why));
                return Some(cmds);
            }
        } else {
            self.held = None;
        }
        // Over the water: a boat to hand, or one to build.
        if let Some((id, _, since)) = self.boarding {
            if (obs.tick.saturating_sub(since) as f32 * DT) > BOAT_FOR {
                self.bad_boat.put(id, obs.tick);
                self.boarding = None;
            }
        }
        // With a bank in sight, a moored boat is only worth it close by:
        // the quad's bots walked up to 30 tiles to one (a twelfth of
        // their time) with a launch in hand.
        let can_launch = me.pillboys.aboard && me.pillboys.strength > 0 && obs.tick >= self.no_embark_until && !obs.launch.is_empty();
        let walk = if can_launch { BOAT_NEAR } else { BOAT_WALK };
        if let Some(b) = obs.boats.iter().filter(|b| b.pos.dist(me.pos) <= walk && !self.bad_boat.has(&b.id, obs.tick)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))) {
            if self.boarding.map(|(id, _, _)| id != b.id).unwrap_or(true) {
                self.boarding = Some((b.id, b.pos, obs.tick));
            }
            self.say(HopAct::ToBoat, format!("to the boat at ({:.0},{:.0})", b.pos.x, b.pos.y));
            if me.goal.is_none() {
                order(Command::Move { to: b.pos }, &mut cmds, &mut self.last_order);
            }
            return Some(cmds);
        }
        self.boarding = None;
        if !me.pillboys.aboard {
            self.say(HopAct::SquadOut, "the squad is out".into());
            return Some(cmds);
        }
        if me.pillboys.strength == 0 {
            self.give_up("no squad to build a boat");
            return None;
        }
        // Beside the water already: launch at once (`Command::Embark`),
        // no squad walk; refused, the squad builds one as before.
        if obs.tick >= self.no_embark_until && obs.launch.iter().any(|s| s.dist(me.pos) <= 1.6) {
            self.say(HopAct::Launching, format!("launching a boat for ({:.0},{:.0})", target.x, target.y));
            order(Command::Embark, &mut cmds, &mut self.last_order);
            return Some(cmds);
        }
        let site = obs.launch.iter().filter(|s| !self.bad_launch.has(&s.tile(), obs.tick)).min_by(|a, b| (a.dist(target) + 0.5 * a.dist(me.pos)).total_cmp(&(b.dist(target) + 0.5 * b.dist(me.pos)))).copied();
        // The bank already on the way to stays the one.
        let site = self.banking.map(|(s, _, _)| s).filter(|s| !self.bad_bank.contains(&s.tile())).or(site);
        // The tank drives to the bank and launches itself (the quad's
        // bots spent a tenth of the match on squads walking out to
        // build boats); refused, or not there in `BANK_FOR`, the
        // squad builds one there instead.
        if let Some(s) = site.filter(|s| obs.tick >= self.no_embark_until && !self.bad_bank.contains(&s.tile())) {
            let (bank, since) = match self.banking {
                Some((b, at, t)) if b.tile() == s.tile() => (at, t),
                _ => (Self::bank(s, me.pos), obs.tick),
            };
            if (obs.tick.saturating_sub(since) as f32 * DT) <= BANK_FOR {
                self.banking = Some((s, bank, since));
                self.say(HopAct::ToBank, format!("to the bank at ({:.0},{:.0}) to launch for ({:.0},{:.0})", s.x, s.y, target.x, target.y));
                if me.goal.is_none() || self.last_order + 4 * TICK_HZ as Tick <= obs.tick {
                    order(Command::Move { to: bank }, &mut cmds, &mut self.last_order);
                }
                return Some(cmds);
            }
            self.bad_bank.push(s.tile());
            self.banking = None;
        }
        // The launch failed (the squad builds, for five material; short,
        // the material first, below), or there is no bank in sight.
        if me.pillboys.material >= BOAT_COST || site.is_none() {
            match site {
                Some(s) => {
                    self.say(HopAct::BuildingBoat, format!("building a boat at ({:.0},{:.0}) for ({:.0},{:.0})", s.x, s.y, target.x, target.y));
                    order(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Boat, at: s } }, &mut cmds, &mut self.last_order);
                }
                None => {
                    // No bank in sight: walk toward the target's side of
                    // the island and look again.
                    self.say(HopAct::SeekingBank, "looking for a bank to launch from".into());
                    if me.goal.is_none() {
                        let to = obs.clamp(me.pos + (target - me.pos).norm().scale(8.0));
                        order(Command::Move { to }, &mut cmds, &mut self.last_order);
                    }
                }
            }
            return Some(cmds);
        }
        if me.pillboys.material + HARVEST_MATERIAL > PILLBOY_MATERIAL_MAX && me.pillboys.material < BOAT_COST {
            self.give_up("the squad cannot carry a boat's worth");
            return None;
        }
        let forest = obs.forest.iter().min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos))).copied();
        let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
        let depot = obs
            .depots
            .iter()
            .filter(|d| ours(d.owner) && !self.bad_depot.has(&d.pos.tile(), obs.tick) && d.pos.dist(me.pos) <= RESTOCK_WALK)
            .map(|d| d.pos)
            .min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos)))
            .filter(|d| forest.map(|f| f.dist(me.pos) + FOREST_NEARER > d.dist(me.pos)).unwrap_or(true));
        if let Some(d) = depot {
            self.scouting = None;
            self.restock = Some(d);
            self.say(HopAct::Restocking, format!("restocking at the depot for a boat: {}/{} material", me.pillboys.material, BOAT_COST));
            if me.pos.dist(d) > DEPOT_RADIUS * 0.5 && (me.goal.is_none() || self.last_order + 4 * TICK_HZ as Tick <= obs.tick) {
                order(Command::Move { to: d }, &mut cmds, &mut self.last_order);
            }
            return Some(cmds);
        }
        self.restock = None;
        match forest {
            Some(t) => {
                self.scouting = None;
                self.say(HopAct::Felling, format!("felling for a boat: {}/{} material", me.pillboys.material, BOAT_COST));
                order(Command::Deploy { task: PillboyTask::Harvest { at: t } }, &mut cmds, &mut self.last_order);
                Some(cmds)
            }
            None => {
                // None in sight: drive along the island looking, a new
                // way each time one runs out, for a while.
                let (since, way) = *self.scouting.get_or_insert((obs.tick, (target - me.pos).y.atan2((target - me.pos).x) + std::f32::consts::FRAC_PI_2));
                if (obs.tick.saturating_sub(since) as f32 * DT) > SCOUT_FOR {
                    self.give_up("no forest for a boat");
                    return None;
                }
                self.say(HopAct::SeekingForest, format!("looking for forest for a boat, {:.0} s", (obs.tick - since) as f32 * DT));
                if me.goal.is_none() {
                    let way = way + std::f32::consts::FRAC_PI_2 * ((obs.tick - since) as f32 * DT / 10.0).floor();
                    let to = obs.clamp(me.pos + V2::new(way.cos(), way.sin()).scale(12.0));
                    order(Command::Move { to }, &mut cmds, &mut self.last_order);
                }
                Some(cmds)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A hop sent across a channel `wide` tiles of water, the map grass
    /// round it and 200 tiles of forest behind: its first think, and the
    /// Hop after.
    fn across(wide: i32, pricing: Option<crate::weights::Settings>) -> (Hop, Vec<Command>) {
        let mut w = World::new(Scenario::bots(1));
        w.depots.clear();
        w.pillboxes.clear();
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                // Forest on the near island, away from the way across:
                // trees plenty, so a bridge is priced at its length.
                let t = if (60..60 + wide).contains(&x) {
                    Terrain::Water
                } else if (10..30).contains(&x) && (10..20).contains(&y) {
                    Terrain::Forest
                } else {
                    Terrain::Grass
                };
                w.map.set(V2::new(x as f32 + 0.5, y as f32 + 0.5), t);
            }
        }
        let start = V2::new(55.5, 64.5);
        w.tanks[0].pos = start;
        let target = V2::new((60 + wide + 6) as f32 + 0.5, 64.5);
        let mut h = Hop { pricing, ..Hop::default() };
        assert!(h.send(target));
        let obs = w.observe(0);
        let me = obs.me.clone().unwrap();
        let cmds = h.think(&obs, &me, false, Some(&w.map)).expect("the hop has the tank");
        (h, cmds)
    }

    /// The bridge choice (backlog 26; Paul, 17 October: "build roads
    /// where it would drive"): a gap of `BRIDGE_MAX` tiles or fewer is
    /// bridged from the near bank, a wider one sailed; priced (LR 6.2), a
    /// bridge that costs more than a boat is sailed too, which at
    /// today's weights and 200 trees is from 8 tiles of water (7.9
    /// against a boat's 8.3 at 7; 10.5 at 8).
    #[test]
    fn a_hop_bridges_a_narrow_gap_and_sails_a_wide_one() {
        let (h, _) = across(4, None);
        assert_eq!(h.act, HopAct::Bridging, "{}", h.doing);
        assert_eq!(h.bridge_front().map(|t| t.tile().0), Some(60), "laid from the near bank");
        assert_eq!(h.bridge_job().map(|(t, _)| t.len()), Some(4));
        let (h, _) = across(BRIDGE_MAX as i32 + 5, None);
        assert!(h.bridge_front().is_none() && h.act != HopAct::Bridging, "too wide to bridge: {}", h.doing);
        let s = crate::weights::Settings::default();
        let (h, _) = across(7, Some(s.clone()));
        assert_eq!(h.act, HopAct::Bridging, "7 tiles priced: a bridge ({})", h.doing);
        let (h, _) = across(8, Some(s));
        assert!(h.bridge_front().is_none(), "8 tiles priced: by boat ({})", h.doing);
    }
}
