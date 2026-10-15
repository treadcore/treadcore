//! The Commander (roadmap step 11a): a Brain for the player's own seat.
//!
//! A person commands the tank with a goal instead of driving it: hunt
//! pillboxes, defend a pillbox, defend a depot, escort an ally, or hold.
//! The Commander drives, aims and sends the squad toward the goal; a
//! RuleBot inside it keeps the doctrine it already has for drones and
//! for shooting at tanks, and the Commander keeps only those of its
//! orders that serve the goal. It sees what the player sees and its
//! orders are validated like a keyboard's, so a goal is never a cheat,
//! and the world, the wire and the replay know nothing of it.
//!
//! The pillbox, by the numbers: 60 health, 8 tiles of reach, 15 every
//! two seconds, at tanks and drones only, never at a squad. The direct
//! gun reaches 10 and does 45. So a live hostile pillbox dies to two
//! direct shots from the ring between 8 and 10 tiles, taking none back;
//! then the squad stands on it for ten seconds and it is ours. A
//! neutral one does not have to be shot at all: the squad takes it live
//! while the tank stays out of its ring.

use crate::{Brain, RuleBot, PILLBOY_REACH};
use treadcore_sim::*;

/// Where the hunter fires from: outside the pillbox's ring, inside the
/// direct gun's, with a margin each side for a tank still rolling.
pub const HUNT_STANDOFF: f32 = 9.1;
/// The ring the gun is fired from, by the tank's actual distance: the
/// world ends a route at a tile's centre, so the point is snapped to one
/// inside it.
const HUNT_NEAR: f32 = PILLBOX_RANGE + 0.2;
const HUNT_FAR: f32 = DIRECT_RANGE - 0.1;
/// How far round the target the firing point moves when the world says
/// the line is blocked (a wall, forest), in radians: 45 degrees.
const STEP_ROUND: f32 = std::f32::consts::FRAC_PI_4;
/// A defender stays this close to what it defends, and chases nothing
/// further than `DEFEND_LEASH` from it.
pub const DEFEND_HOLD: f32 = 3.0;
pub const DEFEND_LEASH: f32 = 12.0;
/// An escort keeps this far from the tank it follows.
pub const ESCORT_DISTANCE: f32 = 4.0;
/// The wingman (Goal::Escort). Nothing it does takes it further than
/// `WING_LEASH` from its leader; what it wants is within `WING_GREED`
/// of the leader; a threat to the leader is anything hostile within
/// `WING_THREAT` of it; ahead is the leader's heading within 60 degrees
/// out to `WING_AHEAD`. A task is kept `WING_COMMIT` before another
/// below it in the order may take its place.
pub const WING_LEASH: f32 = 20.0;
pub const WING_GREED: f32 = 16.0;
pub const WING_THREAT: f32 = 10.0;
pub const WING_AHEAD: f32 = 16.0;
const WING_COMMIT: Tick = 3 * TICK_HZ as Tick;
/// Ask the leader in no oftener than this.
const ASK_EVERY: Tick = 30 * TICK_HZ as Tick;
/// Hold 100% this long before leaving: a total that flickers is not one.
const TOTAL_HOLD: Tick = 10 * TICK_HZ as Tick;

/// What the wingman is doing, in the order it is preferred.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Wing {
    /// 1. A hostile tank threatening the leader.
    Defend(EntityId),
    /// 2. A hostile tank ahead of the leader.
    Ahead(EntityId),
    /// 3 and 6. A pillbox near the leader: the leader's first.
    Pillbox,
    /// 4. A depot near the leader, not ours.
    Depot(EntityId),
    /// 5. Another ally's fight nearby.
    Help(EntityId),
    /// 7. Back beside the leader.
    Regroup,
}

/// Below these the Commander goes home to a depot before anything else.
pub const LOW_SHELLS: u16 = 4;
pub const LOW_ARMOR: f32 = 35.0;
/// Seconds between re-issued moves to the same place: the world routes
/// once, the tank follows; a Move every tick is noise.
const MOVE_EVERY: Tick = TICK_HZ as Tick;

/// What the tank is for, as the player chose it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    /// Nothing: the person drives. The Commander says nothing.
    Manual,
    /// Every pillbox not of my side, nearest first: shot dead if it is
    /// live and theirs, then taken by the squad.
    HuntPillboxes,
    /// One pillbox of my side, or the nearest when None.
    DefendPillbox(Option<EntityId>),
    /// One depot of my side, or the nearest when None.
    DefendDepot(Option<EntityId>),
    /// Follow an ally's tank and fight what fights it.
    Escort(PlayerId),
    /// Stay where the goal was given; fight what comes.
    Hold(V2),
}

impl Goal {
    pub fn label(&self) -> &'static str {
        match self {
            Goal::Manual => "manual",
            Goal::HuntPillboxes => "hunt pillboxes",
            Goal::DefendPillbox(_) => "defend pillbox",
            Goal::DefendDepot(_) => "defend depot",
            Goal::Escort(_) => "escort",
            Goal::Hold(_) => "hold",
        }
    }
}

pub struct Commander {
    name: String,
    pub goal: Goal,
    /// The doctrine for drones and for tanks in sight.
    inner: RuleBot,
    /// The pillbox being hunted, and the bearing (from it) of the
    /// firing point now tried.
    target: Option<EntityId>,
    bearing: Option<f32>,
    /// Pillboxes given up on: the squad could not reach them, or no
    /// firing point round them has a line.
    given_up: Vec<EntityId>,
    /// Where it last told the tank to go, and when.
    last_move: Option<(V2, Tick)>,
    /// Going home to resupply until full again.
    resupplying: bool,
    /// The last direct shot ordered at the target: when, and the shells
    /// then. If none was spent a half second later the world refused it
    /// (a window's client hears of a refusal later than a host does, so
    /// the shell count is the word both read the same).
    fired: Option<(Tick, u16)>,
    /// Firing points tried round this target.
    walked: u32,
    /// The wingman's task now, and the tick it took it: kept for
    /// `WING_COMMIT` unless the leader is threatened.
    task: Option<(Wing, Tick)>,
    /// A bot seat (`Commander::wingman`) runs its own alliances: asks
    /// its leader in, answers proposals, and may leave at 100%.
    pub social: bool,
    /// The last Join asked of the leader, so it does not spam.
    asked: Option<Tick>,
    /// Since when every pillbox and depot has been my side's.
    total_since: Option<Tick>,
    /// The hunter has the gun this tick: its target is live and theirs.
    gun_reserved: bool,
    /// What it says it is doing, for the window's status line.
    pub doing: String,
}

impl Commander {
    /// A bot seat that follows `leader` (step 11a after Paul's notes): an
    /// aggressive, greedy wingman that also runs its own alliances.
    pub fn wingman(seed: u64, name: &str, leader: PlayerId) -> Commander {
        let mut c = Commander::new(seed, name);
        c.goal = Goal::Escort(leader);
        c.social = true;
        c
    }

    pub fn new(seed: u64, name: &str) -> Commander {
        Commander { name: name.into(), goal: Goal::Manual, inner: RuleBot::new(seed, name), target: None, bearing: None, given_up: Vec::new(), last_move: None, resupplying: false, fired: None, walked: 0, task: None, social: false, asked: None, total_since: None, gun_reserved: false, doing: String::new() }
    }

    /// A new goal: what was half done for the old one is dropped.
    pub fn set_goal(&mut self, goal: Goal) {
        if goal != self.goal {
            self.goal = goal;
            self.target = None;
            self.bearing = None;
            self.last_move = None;
            self.doing.clear();
        }
    }

    fn mine_or_allied(obs: &Observation, owner: Option<PlayerId>) -> bool {
        owner.map(|o| obs.allied(o)).unwrap_or(false)
    }

    /// A Move, unless the tank was already sent there lately.
    fn go(&mut self, obs: &Observation, me: &Tank, to: V2, cmds: &mut Vec<Command>) {
        if me.pos.dist(to) < 0.7 {
            if me.goal.is_some() {
                cmds.push(Command::Stop);
            }
            return;
        }
        let fresh = match self.last_move {
            Some((at, tick)) => at.dist(to) > 1.0 || obs.tick >= tick + MOVE_EVERY * 4 || (me.goal.is_none() && obs.tick >= tick + MOVE_EVERY),
            None => true,
        };
        if fresh {
            cmds.push(Command::Move { to });
            self.last_move = Some((to, obs.tick));
        }
    }

    /// The nearest depot of my side, for resupply.
    fn home(obs: &Observation, me: &Tank) -> Option<V2> {
        obs.depots.iter().filter(|d| Self::mine_or_allied(obs, d.owner)).map(|d| d.pos).min_by(|a, b| a.dist(me.pos).partial_cmp(&b.dist(me.pos)).unwrap())
    }

    /// Low on what the goal needs: go home, and stay until full.
    fn needs_home(&mut self, me: &Tank) -> bool {
        if me.shells <= LOW_SHELLS || me.armor <= LOW_ARMOR {
            self.resupplying = true;
        }
        if self.resupplying && me.shells >= TANK_SHELLS && me.armor >= TANK_ARMOR * 0.95 {
            self.resupplying = false;
        }
        self.resupplying
    }

    /// What the inner doctrine may still do under a goal: drones always;
    /// shots at tanks unless the goal wants the gun for itself.
    fn keep_inner(cmd: &Command, gun_free: bool) -> bool {
        match cmd {
            Command::Launch { .. } | Command::Recall | Command::Retask { .. } | Command::SetBubble { .. } => true,
            Command::FireDirect { .. } | Command::FireIndirect { .. } => gun_free,
            _ => false,
        }
    }

    /// `area`: only pillboxes within this distance of this point (a
    /// wingman's, by its leader); None for the whole map.
    fn hunt(&mut self, obs: &Observation, me: &Tank, area: Option<(V2, f32)>, cmds: &mut Vec<Command>) {
        // What the world refused tells the hunter what to change.
        for e in &obs.events {
            if let Event::Rejected { player, cmd, why } = e {
                if *player != obs.player {
                    continue;
                }
                match cmd {
                    Command::FireDirect { .. } if why.contains("line of sight") => {
                        self.fired = None;
                        self.blocked();
                    }
                    Command::Deploy { task: PillboyTask::Capture { pillbox } } if why.contains("unreachable") => {
                        self.given_up.push(*pillbox);
                        self.target = None;
                    }
                    _ => {}
                }
            }
        }
        if let Some((t, shells)) = self.fired {
            if obs.tick >= t + TICK_HZ as Tick / 2 {
                self.fired = None;
                if me.shells >= shells && me.reload == 0 {
                    self.blocked();
                }
            }
        }
        // The target: kept while it is still to be had, else the nearest.
        let wanted = |p: &Pillbox| !Self::mine_or_allied(obs, p.owner) && !self.given_up.contains(&p.id) && area.map(|(at, r)| p.pos.dist(at) <= r).unwrap_or(true);
        let target = self
            .target
            .and_then(|id| obs.pillboxes.iter().find(|p| p.id == id))
            .filter(|p| wanted(p))
            .or_else(|| obs.pillboxes.iter().filter(|p| wanted(p)).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap().then(a.id.cmp(&b.id))))
            .cloned();
        let Some(p) = target else {
            self.target = None;
            self.doing = if self.given_up.is_empty() { "every pillbox is ours or an ally's".into() } else { format!("none left to take ({} given up)", self.given_up.len()) };
            return;
        };
        if self.target != Some(p.id) {
            self.target = Some(p.id);
            self.bearing = None;
            self.walked = 0;
        }
        // The other live hostile pillboxes: the firing point must be
        // outside their rings too.
        let others: Vec<V2> = obs.pillboxes.iter().filter(|o| o.id != p.id && o.alive() && !Self::mine_or_allied(obs, o.owner)).map(|o| o.pos).collect();
        let base = self.bearing.unwrap_or_else(|| (me.pos - p.pos).heading());
        // The tile centre nearest the ideal point whose own distance is
        // inside the ring.
        let snap = |raw: V2| -> V2 {
            let mut best = Map::tile_centre(raw);
            let mut err = f32::MAX;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let c = Map::tile_centre(raw + V2::new(dx as f32, dy as f32));
                    let d = c.dist(p.pos);
                    if (HUNT_NEAR..=HUNT_FAR).contains(&d) && (d - HUNT_STANDOFF).abs() + c.dist(raw) * 0.1 < err {
                        err = (d - HUNT_STANDOFF).abs() + c.dist(raw) * 0.1;
                        best = c;
                    }
                }
            }
            best
        };
        let stand = (0..8)
            .map(|k| base + STEP_ROUND * k as f32)
            .map(|b| snap(p.pos + V2::from_heading(b).scale(HUNT_STANDOFF)))
            .find(|s| others.iter().all(|o| o.dist(*s) > PILLBOX_RANGE + 0.8))
            .unwrap_or_else(|| snap(p.pos + V2::from_heading(base).scale(HUNT_STANDOFF)));
        let live_theirs = p.alive() && p.owner.is_some();
        self.gun_reserved = live_theirs;
        if live_theirs {
            let d = me.pos.dist(p.pos);
            // From the firing point only: after a refusal the point moves
            // round, and the tank must go there before it tries again.
            if (HUNT_NEAR..=HUNT_FAR).contains(&d) && me.pos.dist(stand) < 1.5 {
                if me.reload == 0 && me.shells > 0 && self.fired.is_none() {
                    cmds.push(Command::FireDirect { at: p.pos });
                    self.fired = Some((obs.tick, me.shells));
                }
                self.doing = format!("shelling the pillbox, {:.0} health left", p.health);
                // Stand still while firing: a rolling tank leaves the ring.
                if me.goal.is_some() && me.pos.dist(stand) < 1.5 {
                    cmds.push(Command::Stop);
                }
            } else {
                self.doing = format!("to the firing point, {:.0} tiles off", me.pos.dist(stand));
                self.go(obs, me, stand, cmds);
            }
            return;
        }
        // Dead, or neutral: the squad takes it. The tank waits outside the
        // ring of a live one, inside the squad's reach.
        if p.alive() && me.pos.dist(p.pos) <= PILLBOX_RANGE + 0.5 {
            self.go(obs, me, stand, cmds);
        } else if me.pos.dist(p.pos) > PILLBOY_REACH * 0.8 {
            self.doing = "closing in for the squad".into();
            self.go(obs, me, stand, cmds);
            return;
        }
        let squad = &me.pillboys;
        match &squad.task {
            Some(PillboyTask::Capture { pillbox }) if *pillbox == p.id => {
                self.doing = format!("the squad is taking it, {:.0} s", (PILLBOX_CAPTURE_TICKS.saturating_sub(squad.progress)) as f32 * DT);
            }
            Some(_) => {
                // Out on something else: bring them home first.
                cmds.push(Command::Deploy { task: PillboyTask::Return });
                self.doing = "calling the squad back".into();
            }
            None if squad.aboard && squad.strength > 0 => {
                cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: p.id } });
                self.doing = "sending the squad".into();
            }
            None => {
                self.doing = "no squad: to a depot for one".into();
                self.resupplying = true;
            }
        }
    }

    /// The wingman's order of things, from Paul's notes: protect the
    /// leader; hit what is ahead of it; take the pillbox it is taking, or
    /// another near it; take a depot near it; help another ally's fight;
    /// else close up behind it. Re-read every tick, a task kept for
    /// `WING_COMMIT` unless one ranked above it appears, and nothing
    /// further than the leash from the leader. Returns whether the
    /// inner doctrine may use the gun.
    fn fly_wing(&mut self, obs: &Observation, me: &Tank, leader: &Tank, cmds: &mut Vec<Command>) -> bool {
        let lp = leader.pos;
        let hostile = |t: &&Tank| t.alive && !obs.allied(t.owner);
        let near = |a: V2, b: V2| a.dist(lp).partial_cmp(&b.dist(lp)).unwrap();
        let threat = obs.tanks.iter().filter(hostile).filter(|t| t.pos.dist(lp) <= WING_THREAT).min_by(|a, b| near(a.pos, b.pos).then(a.id.cmp(&b.id)));
        let facing = V2::from_heading(leader.heading);
        let ahead = obs
            .tanks
            .iter()
            .filter(hostile)
            .filter(|t| t.pos.dist(lp) <= WING_AHEAD && { let d = (t.pos - lp).norm(); d.x * facing.x + d.y * facing.y >= 0.5 })
            .min_by(|a, b| near(a.pos, b.pos).then(a.id.cmp(&b.id)));
        let pillbox = obs.pillboxes.iter().any(|p| !Self::mine_or_allied(obs, p.owner) && !self.given_up.contains(&p.id) && p.pos.dist(lp) <= WING_GREED);
        let depot = obs.depots.iter().filter(|d| !Self::mine_or_allied(obs, d.owner) && d.pos.dist(lp) <= WING_GREED).min_by(|a, b| near(a.pos, b.pos)).map(|d| d.id);
        let help = obs
            .tanks
            .iter()
            .filter(|t| t.alive && t.owner != obs.player && t.owner != leader.owner && obs.allied(t.owner) && t.pos.dist(lp) <= WING_LEASH)
            .find_map(|a| obs.tanks.iter().filter(hostile).find(|h| h.pos.dist(a.pos) <= WING_THREAT * 0.8).map(|h| h.id));
        let best = threat
            .map(|t| Wing::Defend(t.id))
            .or(ahead.map(|t| Wing::Ahead(t.id)))
            .or(pillbox.then_some(Wing::Pillbox))
            .or(depot.map(Wing::Depot))
            .or(help.map(Wing::Help))
            .unwrap_or(Wing::Regroup);
        // Keep the task for its commitment unless a better one outranks
        // it by a defence; a task that no longer exists ends at once.
        let still = |w: Wing| match w {
            Wing::Defend(id) | Wing::Ahead(id) | Wing::Help(id) => obs.tanks.iter().any(|t| t.id == id && t.alive && t.pos.dist(lp) <= WING_LEASH),
            Wing::Pillbox => pillbox,
            Wing::Depot(id) => depot == Some(id),
            Wing::Regroup => true,
        };
        let task = match self.task {
            Some((w, since)) if w != best && still(w) && obs.tick < since + WING_COMMIT && !matches!(best, Wing::Defend(_)) => w,
            Some((w, _)) if w == best => w,
            _ => {
                self.task = Some((best, obs.tick));
                best
            }
        };
        let behind = lp - facing.scale(ESCORT_DISTANCE);
        match task {
            Wing::Defend(id) | Wing::Ahead(id) | Wing::Help(id) => {
                let t = obs.tanks.iter().find(|t| t.id == id).unwrap();
                let d = me.pos.dist(t.pos);
                if d <= DIRECT_RANGE - 0.2 && me.reload == 0 && me.shells > 0 {
                    cmds.push(Command::FireDirect { at: t.pos });
                }
                // Close to direct range, on the leash.
                if d > DIRECT_RANGE - 1.0 {
                    let to = t.pos + (me.pos - t.pos).norm().scale(DIRECT_RANGE - 2.0);
                    let to = if to.dist(lp) > WING_LEASH { lp + (to - lp).norm().scale(WING_LEASH) } else { to };
                    self.go(obs, me, to, cmds);
                }
                self.doing = match task {
                    Wing::Defend(_) => "covering the leader".into(),
                    Wing::Ahead(_) => "hitting what is ahead".into(),
                    _ => "helping an ally".into(),
                };
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Wing::Pillbox => {
                self.hunt(obs, me, Some((lp, WING_GREED)), cmds);
                self.doing = format!("pillbox by the leader: {}", self.doing);
                !self.gun_reserved
            }
            Wing::Depot(id) => {
                let at = obs.depots.iter().find(|d| d.id == id).unwrap().pos;
                self.go(obs, me, at, cmds);
                self.doing = "taking the depot".into();
                true
            }
            Wing::Regroup => {
                if me.pos.dist(lp) > ESCORT_DISTANCE + 2.0 {
                    self.go(obs, me, behind, cmds);
                    self.doing = format!("closing up, {:.0} tiles", me.pos.dist(lp));
                } else {
                    self.doing = "on the leader's wing".into();
                }
                true
            }
        }
    }

    /// A bot seat's sides: answer what is put to it (yes to the leader and
    /// its side, yes to anyone who has not shot at it); ask to join the
    /// leader's side while not on it; and leave the side, keeping what
    /// it took, only once every pillbox and every depot on the map has
    /// been the side's for `TOTAL_HOLD`. The 100% is an unlock, read from
    /// the observation's full lists (territory is public), rechecked
    /// every tick; the world's three-way rule still has the last word.
    fn alliances(&mut self, obs: &Observation, cmds: &mut Vec<Command>) {
        let leader = match self.goal {
            Goal::Escort(p) => Some(p),
            _ => None,
        };
        for p in obs.proposals.iter().filter(|p| p.to == obs.player || obs.allied(p.to)) {
            if p.from == obs.player {
                continue;
            }
            let accept = Some(p.from) == leader || obs.allied(p.from) || !self.inner.hurt_by_player(p.from);
            cmds.push(Command::Answer { proposal: p.id, accept });
        }
        if let Some(l) = leader {
            let pending = obs.proposals.iter().any(|p| p.from == obs.player);
            let due = self.asked.map(|t| obs.tick >= t + ASK_EVERY).unwrap_or(true);
            if !obs.allied(l) && !pending && due && obs.would_refuse(l, Terms::Join).is_none() {
                cmds.push(Command::Propose { to: l, terms: Terms::Join });
                self.asked = Some(obs.tick);
            }
        }
        let total = !obs.pillboxes.is_empty() && obs.pillboxes.iter().all(|p| Self::mine_or_allied(obs, p.owner)) && obs.depots.iter().all(|d| Self::mine_or_allied(obs, d.owner));
        let sided = obs.players.iter().any(|p| p.id != obs.player && obs.allied(p.id));
        if total && sided {
            let since = *self.total_since.get_or_insert(obs.tick);
            if obs.tick >= since + TOTAL_HOLD {
                cmds.push(Command::Alliance(AllianceCmd::Leave));
                self.total_since = None;
                self.doing = "100% held: leaving the side with what it took".into();
            }
        } else {
            self.total_since = None;
        }
    }

    /// No line from here: round the target to the next firing point; a
    /// whole circle and a half without one, and it is given up.
    fn blocked(&mut self) {
        let b = self.bearing.unwrap_or(0.0) + STEP_ROUND;
        self.bearing = Some(b);
        self.walked += 1;
        if let (Some(t), true) = (self.target, self.walked > 12) {
            self.given_up.push(t);
            self.target = None;
            self.bearing = None;
        }
    }

    /// Hold by a point, fight what comes, mend what is ours there.
    fn defend(&mut self, obs: &Observation, me: &Tank, at: V2, mend: Option<EntityId>, cmds: &mut Vec<Command>) {
        // A hostile tank inside the leash and in direct reach is shot
        // here; beyond, the inner doctrine's indirect fire answers.
        let threat = obs
            .tanks
            .iter()
            .filter(|t| t.alive && !obs.allied(t.owner) && t.pos.dist(at) <= DEFEND_LEASH)
            .min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap().then(a.id.cmp(&b.id)));
        match threat {
            Some(t) if t.pos.dist(me.pos) <= DIRECT_RANGE - 0.2 => {
                if me.reload == 0 && me.shells > 0 {
                    cmds.push(Command::FireDirect { at: t.pos });
                }
                self.doing = "engaging an intruder".into();
            }
            Some(t) => {
                // Between the intruder and what is defended, not past it.
                let toward = at + (t.pos - at).norm().scale(DEFEND_HOLD.min(t.pos.dist(at)));
                self.go(obs, me, toward, cmds);
                self.doing = "moving to cover it".into();
            }
            None => {
                if me.pos.dist(at) > DEFEND_HOLD {
                    self.go(obs, me, at, cmds);
                    self.doing = format!("to the post, {:.0} tiles", me.pos.dist(at));
                } else {
                    self.doing = "on guard".into();
                }
            }
        }
        // Mend the pillbox when it is hurt and there is material.
        if let Some(id) = mend {
            if let Some(p) = obs.pillboxes.iter().find(|p| p.id == id) {
                let squad = &me.pillboys;
                if p.health < PILLBOX_HEALTH * 0.7 && squad.aboard && squad.task.is_none() && squad.material >= PILLBOX_REPAIR_COST && threat.is_none() {
                    cmds.push(Command::Deploy { task: PillboyTask::Repair { target: id } });
                    self.doing = "mending the pillbox".into();
                } else if !p.alive() && p.owner != Some(obs.player) && squad.aboard && squad.task.is_none() && squad.strength > 0 {
                    // Lost while we watched: take it back.
                    cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: id } });
                    self.doing = "taking the pillbox back".into();
                }
            }
        }
    }
}

impl Brain for Commander {
    fn name(&self) -> &str {
        &self.name
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        let mut cmds = Vec::new();
        // The doctrine thinks every tick so its memory stays current;
        // what it says is kept or dropped below.
        let doctrine = self.inner.think(obs);
        if self.goal == Goal::Manual {
            return cmds;
        }
        let Some(me) = obs.me.clone() else { return cmds };
        if !me.alive {
            self.doing = "dead".into();
            return cmds;
        }
        if self.needs_home(&me) {
            if let Some(home) = Self::home(obs, &me) {
                self.doing = format!("home to resupply, {:.0} tiles", me.pos.dist(home));
                self.go(obs, &me, home, &mut cmds);
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
            self.resupplying = false;
        }
        let gun_free = match self.goal {
            Goal::Manual => true,
            Goal::HuntPillboxes => {
                self.gun_reserved = false;
                self.hunt(obs, &me, None, &mut cmds);
                // The gun is the hunter's while a target is live: a shot
                // at anything else costs the four seconds of reload.
                !self.gun_reserved
            }
            Goal::DefendPillbox(id) => {
                let p = id
                    .and_then(|id| obs.pillboxes.iter().find(|p| p.id == id))
                    .or_else(|| obs.pillboxes.iter().filter(|p| p.owner == Some(obs.player)).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap()))
                    .cloned();
                match p {
                    Some(p) => {
                        self.goal = Goal::DefendPillbox(Some(p.id));
                        self.defend(obs, &me, p.pos, Some(p.id), &mut cmds);
                    }
                    None => self.doing = "no pillbox of ours to defend".into(),
                }
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::DefendDepot(id) => {
                let d = id
                    .and_then(|id| obs.depots.iter().find(|d| d.id == id))
                    .or_else(|| obs.depots.iter().filter(|d| Self::mine_or_allied(obs, d.owner)).min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap()))
                    .cloned();
                match d {
                    Some(d) => {
                        self.goal = Goal::DefendDepot(Some(d.id));
                        // The depot's own pillbox, if one stands by it.
                        let guard = obs.pillboxes.iter().filter(|p| p.owner == Some(obs.player) && p.pos.dist(d.pos) <= PILLBOX_RANGE).min_by(|a, b| a.pos.dist(d.pos).partial_cmp(&b.pos.dist(d.pos)).unwrap()).map(|p| p.id);
                        self.defend(obs, &me, d.pos, guard, &mut cmds);
                    }
                    None => self.doing = "no depot of ours to defend".into(),
                }
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::Escort(who) => match obs.tanks.iter().find(|t| t.owner == who && t.alive).cloned() {
                Some(leader) => self.fly_wing(obs, &me, &leader, &mut cmds),
                None => {
                    self.doing = "the leader is not in sight".into();
                    true
                }
            },
            Goal::Hold(at) => {
                self.defend(obs, &me, at, None, &mut cmds);
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
        };
        cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, gun_free)));
        if self.social {
            self.alliances(obs, &mut cmds);
        }
        cmds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two seats, no swarm, no time limit, no pillboxes but one placed
    /// `away` tiles from seat 0 toward the middle, owned by `owner`.
    fn one_pillbox(away: f32, owner: Option<PlayerId>) -> (World, EntityId) {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = w.tanks[0].pos;
        let toward = (w.map.center() - home).norm();
        let mut at = Map::tile_centre(home + toward.scale(away));
        // On open ground the tank can reach.
        while !w.map.at(at).passable() || w.map.route(home, at).is_empty() {
            at = Map::tile_centre(at + toward);
        }
        let id = 70_001;
        w.pillboxes.push(Pillbox { id, pos: at, owner, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        (w, id)
    }

    /// Run seat 0 under `goal` for `seconds`; the other seat sits still.
    /// Returns the tick the pillbox became ours, and the damage it did.
    fn run(w: &mut World, pillbox: EntityId, goal: Goal, seconds: u32) -> (Option<Tick>, f32) {
        let mut c = Commander::new(5, "commander");
        c.set_goal(goal);
        let (mut ours, mut hurt) = (None, 0.0);
        for _ in 0..seconds * TICK_HZ {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            if std::env::var("COMMANDER_TRACE").is_ok() && (w.tick % (5 * TICK_HZ as Tick) == 0 || orders.iter().any(|o| matches!(o.cmd, Command::FireDirect { .. }))) {
                eprintln!("[{:>5}] at {:?} armor {} shells {} reload {} doing '{}' orders {:?}", w.tick, w.tanks[0].pos, w.tanks[0].armor, w.tanks[0].shells, w.tanks[0].reload, c.doing, orders.iter().map(|o| &o.cmd).collect::<Vec<_>>());
            }
            w.step(&orders);
            for e in &w.events {
                if let Event::Rejected { player: 0, why, .. } = e {
                    if std::env::var("COMMANDER_TRACE").is_ok() {
                        eprintln!("        rejected: {}", why);
                    }
                }
                if let Event::PillboxFired { pillbox: p, damage, .. } = e {
                    if *p == pillbox {
                        hurt += damage;
                    }
                }
            }
            if ours.is_none() && w.pillboxes.iter().any(|p| p.id == pillbox && p.owner == Some(0)) {
                ours = Some(w.tick);
            }
        }
        (ours, hurt)
    }

    /// 11a's proof: a live pillbox of the enemy's, 25 tiles off, is shot
    /// dead from outside its ring and taken by the squad inside ninety
    /// seconds, and it never lands a shell on the tank.
    #[test]
    fn a_commander_takes_a_pillbox() {
        let (mut w, id) = one_pillbox(25.0, Some(1));
        let (ours, hurt) = run(&mut w, id, Goal::HuntPillboxes, 90);
        let at = ours.unwrap_or_else(|| panic!("not taken in 90 s: {:?}", w.pillboxes.iter().find(|p| p.id == id)));
        eprintln!("a_commander_takes_a_pillbox: ours at {:.1} s, {} damage taken from it", at as f32 * DT, hurt);
        assert_eq!(hurt, 0.0, "the pillbox hit the tank");
    }

    /// A wall on the near side of the pillbox: the first firing point
    /// has no line, so the hunter walks round to one that has.
    #[test]
    fn a_commander_walks_round_a_wall_to_a_line() {
        let (mut w, id) = one_pillbox(25.0, Some(1));
        let p = w.pillboxes[0].pos;
        let toward_me = (w.tanks[0].pos - p).heading();
        for k in -6..=6 {
            let b = toward_me + k as f32 * 0.15;
            let at = p + V2::from_heading(b).scale(3.0);
            if w.map.at(at).passable() {
                w.map.set(at, Terrain::Wall);
            }
        }
        assert!(!w.map.line_clear(p + V2::from_heading(toward_me).scale(HUNT_STANDOFF), p), "the wall does not block the first point");
        let (ours, hurt) = run(&mut w, id, Goal::HuntPillboxes, 150);
        let at = ours.unwrap_or_else(|| panic!("not taken in 150 s: {:?}", w.pillboxes.iter().find(|x| x.id == id)));
        eprintln!("a_commander_walks_round_a_wall_to_a_line: ours at {:.1} s, {} damage taken", at as f32 * DT, hurt);
        assert_eq!(hurt, 0.0, "the pillbox hit the tank");
    }

    /// Three seats: 0 the leader, 1 its wingman on the same side, 2 the
    /// enemy; the wingman starts beside the leader, nobody else moves.
    fn a_pair() -> World {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(3) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let team = w.players[0].team;
        w.players[1].team = team;
        let lp = w.tanks[0].pos;
        let toward = (w.map.center() - lp).norm();
        let mut at = Map::tile_centre(lp - toward.scale(3.0));
        while !w.map.at(at).passable() {
            at = Map::tile_centre(at + toward);
        }
        w.tanks[1].pos = at;
        w.tanks[1].goal = None;
        w.tanks[1].route.clear();
        w
    }

    fn fly(w: &mut World, c: &mut Commander, ticks: u32, mut each: impl FnMut(&World, u32)) {
        for i in 0..ticks {
            let obs = w.observe(1);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 1, cmd }).collect();
            w.step(&orders);
            each(w, i);
        }
    }

    /// The wingman's first rule: a hostile tank that comes up to the
    /// leader is hit, and the wingman never leaves the leash to do it.
    #[test]
    fn a_wingman_covers_its_leader() {
        let mut w = a_pair();
        let lp = w.tanks[0].pos;
        let toward = (w.map.center() - lp).norm();
        let mut at = Map::tile_centre(lp + toward.scale(8.0));
        while !w.map.at(at).passable() {
            at = Map::tile_centre(at + toward);
        }
        let mut c = Commander::wingman(3, "wing", 0);
        c.social = false;
        let (mut hits, mut furthest) = (0, 0.0f32);
        fly(&mut w, &mut c, 60 * TICK_HZ, |w, _| {
            for e in &w.events {
                if let Event::TankDamaged { tank, by: Some(1), .. } = e {
                    if w.tanks.iter().any(|t| t.id == *tank && t.owner == 2) {
                        hits += 1;
                    }
                }
            }
            furthest = furthest.max(w.tanks[1].pos.dist(w.tanks[0].pos));
        });
        // Nothing hostile came: it stayed on the wing.
        assert_eq!(hits, 0);
        assert!(furthest <= ESCORT_DISTANCE + 3.0, "wandered {:.1} tiles off with nothing to do", furthest);
        // Now the enemy drives up to the leader.
        let t = w.tanks.iter_mut().find(|t| t.owner == 2).unwrap();
        t.pos = at;
        t.goal = None;
        t.route.clear();
        furthest = 0.0;
        fly(&mut w, &mut c, 60 * TICK_HZ, |w, _| {
            for e in &w.events {
                if let Event::TankDamaged { tank, by: Some(1), .. } = e {
                    if w.tanks.iter().any(|t| t.id == *tank && t.owner == 2) {
                        hits += 1;
                    }
                }
            }
            furthest = furthest.max(w.tanks[1].pos.dist(w.tanks[0].pos));
        });
        eprintln!("a_wingman_covers_its_leader: {} hits on the intruder, furthest {:.1} tiles from the leader", hits, furthest);
        assert!(hits >= 2, "the intruder was hit {} times", hits);
        assert!(furthest <= WING_LEASH, "left the leash: {:.1} tiles", furthest);
    }

    /// The pillbox by the leader is the wingman's to take; one far from
    /// the leader is not.
    #[test]
    fn a_wingman_takes_the_pillbox_by_its_leader() {
        let mut w = a_pair();
        let lp = w.tanks[0].pos;
        let toward = (w.map.center() - lp).norm();
        let side = V2::new(-toward.y, toward.x);
        let place = |w: &World, from: V2, dir: V2, d: f32| {
            let mut at = Map::tile_centre(from + dir.scale(d));
            while !w.map.at(at).passable() || w.map.route(from, at).is_empty() {
                at = Map::tile_centre(at + dir);
            }
            at
        };
        let near = place(&w, lp, toward, 13.0);
        let far = place(&w, lp, side, 40.0);
        w.pillboxes.push(Pillbox { id: 70_010, pos: near, owner: Some(2), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        w.pillboxes.push(Pillbox { id: 70_011, pos: far, owner: Some(2), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let mut c = Commander::wingman(4, "wing", 0);
        c.social = false;
        let mut hurt = 0.0;
        fly(&mut w, &mut c, 120 * TICK_HZ, |w, _| {
            for e in &w.events {
                if let Event::PillboxFired { tank, damage, .. } = e {
                    if *tank == w.tanks[1].id {
                        hurt += damage;
                    }
                }
            }
        });
        let owner = |id| w.pillboxes.iter().find(|p| p.id == id).unwrap().owner;
        assert!(owner(70_010) == Some(1) || owner(70_010) == Some(0), "the pillbox by the leader is still {:?}", owner(70_010));
        assert_eq!(owner(70_011), Some(2), "it went 40 tiles off the leader for the far one");
        assert_eq!(hurt, 0.0, "a pillbox hit the wingman");
    }

    /// The defection gate: with one pillbox not the side's, never; with
    /// all of them and every depot, after `TOTAL_HOLD` and not before.
    #[test]
    fn a_wingman_leaves_only_at_one_hundred_percent() {
        let mut w = a_pair();
        let lp = w.tanks[0].pos;
        w.pillboxes.push(Pillbox { id: 70_020, pos: lp + V2::new(2.0, 0.0), owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        w.pillboxes.push(Pillbox { id: 70_021, pos: lp + V2::new(-2.0, 0.0), owner: None, health: 0.0, reload: 0, capture: 0, capturer: None, shells: 0 });
        for d in w.depots.iter_mut() {
            d.owner = Some(0);
        }
        let mut c = Commander::wingman(5, "wing", 0);
        let leaves = |c: &mut Commander, w: &World| {
            let mut cmds = Vec::new();
            c.alliances(&w.observe(1), &mut cmds);
            cmds.iter().any(|x| matches!(x, Command::Alliance(AllianceCmd::Leave)))
        };
        for _ in 0..(TOTAL_HOLD * 3) {
            assert!(!leaves(&mut c, &w), "left with a pillbox not the side's");
            w.tick += 1;
        }
        w.pillboxes.iter_mut().find(|p| p.id == 70_021).unwrap().owner = Some(1);
        let start = w.tick;
        let mut left = None;
        for _ in 0..(TOTAL_HOLD * 2) {
            if leaves(&mut c, &w) {
                left = Some(w.tick - start);
                break;
            }
            w.tick += 1;
        }
        assert_eq!(left, Some(TOTAL_HOLD), "left at {:?} ticks of 100%", left);
    }

    /// A neutral pillbox is not shot at all: the squad takes it live.
    #[test]
    fn a_commander_takes_a_neutral_pillbox_without_a_shot() {
        let (mut w, id) = one_pillbox(20.0, None);
        let shells = w.tanks[0].shells;
        let (ours, hurt) = run(&mut w, id, Goal::HuntPillboxes, 60);
        assert!(ours.is_some(), "not taken in 60 s");
        assert_eq!(hurt, 0.0, "the pillbox hit the tank");
        let p = w.pillboxes.iter().find(|p| p.id == id).unwrap();
        assert!(p.alive(), "a neutral pillbox was shot instead of taken");
        assert!(w.tanks[0].shells >= shells, "shells spent on a neutral pillbox");
    }

    /// Defend depot: an intruder that drives up to the depot is engaged,
    /// and the defender never strays past the leash.
    #[test]
    fn a_commander_defends_its_depot() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(w.tanks[0].pos).partial_cmp(&b.pos.dist(w.tanks[0].pos)).unwrap()).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let mut c = Commander::new(9, "commander");
        c.set_goal(Goal::DefendDepot(Some(depot.id)));
        let mut furthest: f32 = 0.0;
        let mut hits_on_intruder = 0;
        for i in 0..(120 * TICK_HZ) {
            // At 20 s the intruder arrives beside the depot.
            if i == 20 * TICK_HZ {
                let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
                t.pos = depot.pos + V2::new(5.0, 0.0);
                t.goal = None;
                t.route.clear();
            }
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            for e in &w.events {
                if let Event::TankDamaged { tank, by: Some(0), .. } = e {
                    if w.tanks.iter().any(|t| t.id == *tank && t.owner == 1) {
                        hits_on_intruder += 1;
                    }
                }
            }
            if i > 30 * TICK_HZ {
                furthest = furthest.max(w.tanks[0].pos.dist(depot.pos));
            }
        }
        eprintln!("a_commander_defends_its_depot: {} hits on the intruder, furthest {:.1} tiles from the depot", hits_on_intruder, furthest);
        assert!(hits_on_intruder >= 2, "the intruder was hit {} times", hits_on_intruder);
        assert!(furthest <= DEFEND_LEASH, "strayed {:.1} tiles from the depot", furthest);
    }
}
