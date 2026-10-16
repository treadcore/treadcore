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
//!
//! Auto (step 11c's hybrid): a ladder of rungs, each with a condition,
//! tried top down every tick; the first whose condition holds is what
//! the tank does, and the player turns rungs on and off:
//!
//! 1. defend base: a hostile tank within BASE_ALARM of the base, or a
//!    pillbox of the base shot at. The tank fights it under the base's
//!    guns, a few tiles out toward it; the squad mends between shots.
//! 2. fortify: the castle round the base is not yet standing.
//! 3. turrets: the castle stands and nothing has come for QUIET; bring
//!    the side's outlying pillboxes home, one at a time, and set them
//!    down in the bastions at the wall's ends (step 11h).
//! 4. hunt: a pillbox not of my side within EXPAND_R of the base.
//! 5. expand: a depot not of my side, the nearest the base first (its
//!    guard pillboxes shot dead first, then the tank sits on it), else
//!    a pillbox anywhere. A side that only hunted pillboxes held three
//!    depots for fifteen minutes while dead teams held five each.
//! 6. guard: on guard at the base, mending.
//!
//! Low on shells or armour, the tank goes home first whatever the rung.

use crate::fortify::{Plan, Want};
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

/// The base (Goal::DefendBase and Auto): what stands within BASE_R of
/// its centre belongs to it; a hostile tank within BASE_ALARM of it is
/// a threat; a pillbox of it shot at raises the alarm for ALARM_HOLD.
pub const BASE_R: f32 = 10.0;
pub const BASE_ALARM: f32 = 16.0;
const ALARM_HOLD: Tick = 6 * TICK_HZ as Tick;
/// Turrets wait for this long without a threat.
pub const QUIET: Tick = 20 * TICK_HZ as Tick;
/// The hunt looks this far from the base first.
pub const EXPAND_R: f32 = 30.0;
/// The tank stops this close to a pillbox it is fetching.
const FETCH_NEAR: f32 = 2.5;
/// Hit with nothing hostile in sight: scoot this far, for this long, and
/// put a recon up over the tank (the RuleBot's shoot-and-scoot, from the
/// other end). Both deaths in the window's first two runs were a tank
/// standing still under shells from beyond its seven tiles of sight.
pub const SCOOT: f32 = 3.5;
const SCOOT_FOR: Tick = 4 * TICK_HZ as Tick;

/// The rungs of Auto's ladder, top first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rung {
    DefendBase,
    Fortify,
    Turrets,
    Hunt,
    Expand,
    Guard,
}

impl Rung {
    pub const ALL: [Rung; 6] = [Rung::DefendBase, Rung::Fortify, Rung::Turrets, Rung::Hunt, Rung::Expand, Rung::Guard];
    pub fn label(self) -> &'static str {
        match self {
            Rung::DefendBase => "defend base",
            Rung::Fortify => "fortify",
            Rung::Turrets => "turrets",
            Rung::Hunt => "hunt",
            Rung::Expand => "expand",
            Rung::Guard => "guard",
        }
    }
    pub fn index(self) -> usize {
        Rung::ALL.iter().position(|r| *r == self).unwrap()
    }
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
    /// Build a castle's gate round a point (step 11b; `fortify.rs`): a
    /// depot of my side by default, the nearest, else where the tank is.
    Fortify(Option<V2>),
    /// The base is under imminent threat: meet what comes for it within
    /// BASE_ALARM, mend and retake its pillboxes, else stand guard.
    DefendBase,
    /// The ladder (`Rung`): the tank picks its own goal, rung by rung.
    Auto,
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
            Goal::Fortify(_) => "fortify",
            Goal::DefendBase => "defend base",
            Goal::Auto => "auto",
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
    /// The castle being built, under Goal::Fortify.
    pub plan: Option<Plan>,
    /// The hunter has the gun this tick: its target is live and theirs.
    gun_reserved: bool,
    /// What it says it is doing, for the window's status line.
    pub doing: String,
    /// Auto's rungs on and off, the one it is on, and each one's word
    /// on its condition, for the coordination screen.
    pub ladder: [bool; 6],
    pub rung: Option<Rung>,
    pub why: [String; 6],
    /// The base: set the first time a base goal needs one.
    pub base: Option<V2>,
    /// A fleet's: the base is this player's depot, not my own (fleet.rs).
    pub base_owner: Option<PlayerId>,
    /// The last tick the base was threatened, and the alarm's end.
    threatened: Option<Tick>,
    alarm_until: Tick,
    /// The base's pillbox that raised the alarm: where to go and look.
    alarm_at: Option<V2>,
    /// The outlying pillbox being fetched for a turret.
    fetching: Option<EntityId>,
    /// The plan could not be fed (no forest): fortify steps aside.
    starved: bool,
    /// Expand's depot, and the depots it could not reach.
    depot_target: Option<EntityId>,
    depots_given_up: Vec<EntityId>,
    /// A direct shot refused for no line (a wall of our own, trees): the
    /// gun waits until this tick instead of asking every tick.
    hold_fire_until: Tick,
    /// Scooting from unseen fire: to here, until then; and which side
    /// the last scoot went, so the next goes the other.
    scoot: Option<(V2, Tick)>,
    scoot_left: bool,
    /// Where the last shell at the tank came from.
    gun_seen_at: Option<V2>,
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
        Commander { name: name.into(), goal: Goal::Manual, inner: RuleBot::new(seed, name), target: None, bearing: None, given_up: Vec::new(), last_move: None, resupplying: false, fired: None, walked: 0, task: None, social: false, asked: None, total_since: None, plan: None, gun_reserved: false, doing: String::new(), ladder: [true; 6], rung: None, why: Default::default(), base: None, base_owner: None, threatened: None, alarm_until: 0, alarm_at: None, fetching: None, starved: false, depot_target: None, depots_given_up: Vec::new(), hold_fire_until: 0, scoot: None, scoot_left: false, gun_seen_at: None }
    }

    /// A new goal: what was half done for the old one is dropped.
    pub fn set_goal(&mut self, goal: Goal) {
        if goal != self.goal {
            self.goal = goal;
            self.target = None;
            self.bearing = None;
            self.last_move = None;
            self.plan = None;
            self.doing.clear();
            self.rung = None;
            self.fetching = None;
            self.starved = false;
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
                    // The firing point is water or cannot be reached:
                    // the next one round.
                    Command::Move { .. } if self.target.is_some() && (why.contains("water") || why.contains("unreachable")) => {
                        self.last_move = None;
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

    /// Build the castle: guard the point as `defend` does, and keep the
    /// squad on the plan whenever it is aboard and nothing threatens.
    fn fortify(&mut self, obs: &Observation, me: &Tank, at: Option<V2>, cmds: &mut Vec<Command>) {
        if self.plan.is_none() {
            let centre = at
                .or_else(|| obs.depots.iter().filter(|d| Self::mine_or_allied(obs, d.owner)).map(|d| d.pos).min_by(|a, b| a.dist(me.pos).partial_cmp(&b.dist(me.pos)).unwrap()))
                .unwrap_or(me.pos);
            // The threat: the nearest depot not of my side, else the
            // map's middle, which is where everyone else comes from.
            let middle = V2::new(obs.map_w as f32 / 2.0, obs.map_h as f32 / 2.0);
            let threat = obs.depots.iter().filter(|d| !Self::mine_or_allied(obs, d.owner) && d.pos.dist(centre) > 4.0).map(|d| d.pos).min_by(|a, b| a.dist(centre).partial_cmp(&b.dist(centre)).unwrap()).unwrap_or(middle);
            self.plan = Some(Plan::castle(centre, threat));
            if matches!(self.goal, Goal::Fortify(_)) {
                self.goal = Goal::Fortify(Some(centre));
            }
        }
        let plan = self.plan.as_mut().unwrap();
        plan.hear(obs);
        let centre = plan.centre;
        // The tank sits behind the work, beside the point: off the gate,
        // off the road, inside the squad's reach of every tile.
        let post = centre - plan.facing.scale(1.5);
        self.defend(obs, me, post, None, cmds);
        let busy = !self.doing.starts_with("on guard") && !self.doing.starts_with("to the post");
        let squad = &me.pillboys;
        if !squad.aboard || squad.task.is_some() || squad.strength == 0 {
            if squad.strength == 0 {
                self.doing = "no squad: waiting on the depot for one".into();
            } else if !busy {
                self.doing = format!("the squad is building: {}", self.plan.as_ref().unwrap().progress());
            }
            return;
        }
        if busy {
            return;
        }
        let plan = self.plan.as_mut().unwrap();
        match plan.next(obs, squad.material) {
            Want::Order(task) => {
                self.starved = false;
                self.doing = match &task {
                    PillboyTask::Harvest { .. } => "felling for material".into(),
                    _ => format!("building: {}", plan.progress()),
                };
                cmds.push(Command::Deploy { task });
            }
            Want::Starved => {
                self.starved = true;
                self.doing = format!("no forest in reach; {}", plan.progress());
            }
            Want::Wait => self.doing = format!("waiting on the site: {}", plan.progress()),
            Want::Finished => self.doing = format!("the castle stands: {}", plan.progress()),
        }
    }

    /// The base's centre: the castle's, else the depot of my side nearest
    /// the tank when first asked, else where the tank is.
    fn base_of(&mut self, obs: &Observation, me: &Tank) -> V2 {
        if let Some(p) = &self.plan {
            self.base = Some(p.centre);
        }
        if self.base.is_none() {
            if let Some(owner) = self.base_owner {
                // The leader's depot: the one of theirs nearest its tank
                // when it is in sight, else nearest mine.
                let near = obs.tanks.iter().find(|t| t.owner == owner).map(|t| t.pos).unwrap_or(me.pos);
                self.base = obs.depots.iter().filter(|d| d.owner == Some(owner)).min_by(|a, b| a.pos.dist(near).partial_cmp(&b.pos.dist(near)).unwrap()).map(|d| Map::tile_centre(d.pos));
            }
        }
        *self.base.get_or_insert_with(|| Self::home(obs, me).map(Map::tile_centre).unwrap_or(me.pos))
    }

    /// Where the tank waits at the base: behind the gate when there is a
    /// castle, else beside the centre.
    fn post(&self, base: V2) -> V2 {
        match &self.plan {
            Some(p) => p.centre - p.facing.scale(1.5),
            None => base,
        }
    }

    /// The threat to the base, if any: the hostile tank nearest it within
    /// BASE_ALARM, and the alarm a pillbox of the base shot at raises
    /// (by a gun it may not see: indirect fire, a longshot).
    fn base_threat(&mut self, obs: &Observation, base: V2) -> (Option<Tank>, bool) {
        // The tank sees TANK_VISION and a pillbox sees nothing, so the
        // base's guns are its ears: one of them hit, or firing, is a
        // threat there even when nobody of ours can see it.
        let ours: Vec<(EntityId, V2)> = obs.pillboxes.iter().filter(|p| p.pos.dist(base) <= BASE_R && Self::mine_or_allied(obs, p.owner)).map(|p| (p.id, p.pos)).collect();
        for e in &obs.events {
            let heard = match e {
                Event::PillboxDamaged { pillbox, by: Some(b), .. } if !obs.allied(*b) => Some(*pillbox),
                Event::PillboxFired { pillbox, .. } => Some(*pillbox),
                _ => None,
            };
            if let Some(at) = heard.and_then(|id| ours.iter().find(|(o, _)| *o == id)).map(|(_, at)| *at) {
                self.alarm_until = obs.tick + ALARM_HOLD;
                self.alarm_at = Some(at);
            }
        }
        let tank = obs.tanks.iter().filter(|t| t.alive && !obs.allied(t.owner) && t.pos.dist(base) <= BASE_ALARM).min_by(|a, b| a.pos.dist(base).partial_cmp(&b.pos.dist(base)).unwrap().then(a.id.cmp(&b.id))).cloned();
        let alarm = obs.tick < self.alarm_until;
        if tank.is_some() || alarm {
            self.threatened = Some(obs.tick);
        }
        (tank, alarm)
    }

    /// The base's pillbox that most wants the squad: one dead first (ours
    /// to mend, theirs to take back), then the most hurt of ours.
    fn neediest(obs: &Observation, base: V2) -> Option<EntityId> {
        obs.pillboxes
            .iter()
            .filter(|p| p.pos.dist(base) <= BASE_R && (p.owner == Some(obs.player) || !p.alive()))
            .filter(|p| !p.alive() || p.health < PILLBOX_HEALTH * 0.7)
            .min_by(|a, b| a.health.partial_cmp(&b.health).unwrap().then(a.id.cmp(&b.id)))
            .map(|p| p.id)
    }

    /// Defend the base: out to meet the threat as far as BASE_ALARM, else
    /// on guard at the post; the squad mends or takes back the neediest
    /// pillbox of the base whenever no hostile tank is in the leash.
    fn defend_base(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        let base = self.base_of(obs, me);
        let (tank, alarm) = self.base_threat(obs, base);
        // Nothing in sight but a gun of the base at work: go and look.
        let post = match (&tank, alarm, self.alarm_at) {
            (None, true, Some(at)) => at,
            _ => self.post(base),
        };
        let mend = Self::neediest(obs, base);
        self.defend_within(obs, me, post, BASE_ALARM, mend, cmds);
        if let Some(t) = &tank {
            let who = obs.players.iter().find(|p| p.id == t.owner).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", t.owner));
            self.doing = format!("BASE UNDER THREAT: {} {:.0} tiles out; {}", who, t.pos.dist(base), self.doing);
        } else if alarm {
            self.doing = format!("BASE UNDER THREAT out of sight: to the pillbox that heard it; {}", self.doing);
        }
        !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
    }

    /// An outlying pillbox of mine, nearest the base: one to bring in.
    fn outlying(&self, obs: &Observation, base: V2) -> Option<Pillbox> {
        let plan = self.plan.as_ref();
        obs.pillboxes
            .iter()
            .filter(|p| p.owner == Some(obs.player) && p.pos.dist(base) > BASE_R && !plan.map(|pl| pl.items.iter().any(|i| i.at.dist(p.pos) < 1.0)).unwrap_or(false))
            .min_by(|a, b| a.pos.dist(base).partial_cmp(&b.pos.dist(base)).unwrap().then(a.id.cmp(&b.id)))
            .cloned()
    }

    /// Turrets: fetch an outlying pillbox of mine and set it down in the
    /// next bastion. The tank drives to it so the squad's walk is short,
    /// and drives it home again on the squad's back.
    fn turrets(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) {
        let base = self.base_of(obs, me);
        let Some(plan) = self.plan.as_mut() else { return };
        plan.add_turrets();
        plan.hear(obs);
        let Some((slot, forest)) = plan.turret_slot() else {
            // The pillboxes are down: the walls that close them in.
            if plan.turrets_stand() {
                self.doing = format!("the turrets stand: {}", plan.progress());
            } else {
                self.fortify(obs, me, Some(base), cmds);
            }
            return;
        };
        let squad = &me.pillboys;
        if forest {
            // The bastion's tile is trees: fell it first, whatever else.
            if squad.aboard && squad.task.is_none() {
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: slot } });
                self.plan.as_mut().unwrap().felling_slot(slot);
                self.doing = "felling the turret's tile".into();
            } else {
                self.doing = "waiting for the squad".into();
            }
            return;
        }
        if squad.carrying.is_some() {
            self.fetching = None;
            if !squad.aboard || squad.task.is_some() {
                self.doing = "the squad is setting a pillbox down".into();
                return;
            }
            let post = self.post(base);
            if me.pos.dist(post) > DEFEND_HOLD {
                self.go(obs, me, post, cmds);
                self.doing = format!("bringing a pillbox home, {:.0} tiles", me.pos.dist(post));
                return;
            }
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Pillbox, at: slot } });
            self.plan.as_mut().unwrap().ordered(slot, obs.tick);
            self.doing = format!("setting it down in a turret: {}", self.plan.as_ref().unwrap().progress());
            return;
        }
        if !squad.aboard || squad.task.is_some() {
            self.doing = if matches!(squad.task, Some(PillboyTask::Lift { .. })) { "the squad is lifting it".into() } else { "waiting for the squad".into() };
            return;
        }
        let target = self.fetching.and_then(|id| obs.pillboxes.iter().find(|p| p.id == id && p.owner == Some(obs.player)).cloned()).or_else(|| self.outlying(obs, base));
        let Some(p) = target else {
            self.fetching = None;
            self.doing = "no pillbox of ours out there to bring in".into();
            return;
        };
        self.fetching = Some(p.id);
        if me.pos.dist(p.pos) > FETCH_NEAR + 1.5 {
            let to = p.pos + (me.pos - p.pos).norm().scale(FETCH_NEAR);
            self.go(obs, me, to, cmds);
            self.doing = format!("fetching the pillbox at ({:.0},{:.0}), {:.0} tiles", p.pos.x, p.pos.y, me.pos.dist(p.pos));
            return;
        }
        if me.goal.is_some() {
            cmds.push(Command::Stop);
        }
        cmds.push(Command::Deploy { task: PillboyTask::Lift { pillbox: p.id } });
        self.doing = "the squad is lifting it".into();
    }

    /// The ladder: each rung's condition, top down; the first that holds
    /// (and is on) is done this tick.
    fn auto(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        let base = self.base_of(obs, me);
        if let Some(p) = self.plan.as_mut() {
            p.hear(obs);
        }
        let (tank, alarm) = self.base_threat(obs, base);
        let quiet = self.threatened.map(|t| obs.tick.saturating_sub(t)).unwrap_or(obs.tick);
        let stands = self.plan.as_ref().map(|p| p.castle_stands()).unwrap_or(false);
        let carrying = me.pillboys.carrying.is_some();
        let (slot, walls) = self.plan.as_ref().map(|p| { let mut q = p.clone(); q.add_turrets(); (q.turret_slot().is_some(), !q.turrets_stand()) }).unwrap_or((false, false));
        let outlying = self.outlying(obs, base);
        let wanted = obs.pillboxes.iter().filter(|p| !Self::mine_or_allied(obs, p.owner) && !self.given_up.contains(&p.id)).min_by(|a, b| a.pos.dist(base).partial_cmp(&b.pos.dist(base)).unwrap()).map(|p| p.pos.dist(base));
        let depot = obs.depots.iter().filter(|d| !Self::mine_or_allied(obs, d.owner) && !self.depots_given_up.contains(&d.id)).map(|d| d.pos.dist(base)).min_by(|a, b| a.partial_cmp(b).unwrap());
        let q = quiet as f32 / TICK_HZ as f32;
        let holds = [
            tank.is_some() || alarm,
            self.plan.as_ref().map(|p| !p.castle_stands()).unwrap_or(true) && !self.starved,
            stands && quiet >= QUIET && ((slot && (carrying || outlying.is_some() || self.fetching.is_some())) || (!slot && walls)),
            wanted.map(|d| d <= EXPAND_R).unwrap_or(false),
            depot.is_some() || wanted.is_some(),
            true,
        ];
        self.why = [
            match (&tank, alarm) {
                (Some(t), _) => format!("THREAT: a tank {:.0} tiles from the base", t.pos.dist(base)),
                (None, true) => "THREAT: a pillbox of the base is fighting".into(),
                _ => format!("quiet {:.0} s; nothing within {:.0} tiles", q, BASE_ALARM),
            },
            match &self.plan {
                Some(p) if self.starved => format!("stopped, no forest: {}", p.progress()),
                Some(p) => p.progress(),
                None => "nothing built yet".into(),
            },
            if !stands {
                "after the castle stands".into()
            } else if !slot && !walls {
                "both turrets stand".into()
            } else if !slot {
                "walling the turrets in".into()
            } else if quiet < QUIET {
                format!("waiting for {:.0} s of quiet ({:.0} now)", QUIET as f32 / TICK_HZ as f32, q)
            } else if carrying {
                "carrying one home".into()
            } else {
                match &outlying {
                    Some(p) => format!("a pillbox of ours out at {:.0} tiles", p.pos.dist(base)),
                    None => "no pillbox of ours out there to bring in".into(),
                }
            },
            match wanted {
                Some(d) if d <= EXPAND_R => format!("a pillbox not ours {:.0} tiles from the base", d),
                Some(d) => format!("none within {:.0}; the nearest is {:.0} out", EXPAND_R, d),
                None => "every pillbox is ours or an ally's".into(),
            },
            match (depot, wanted) {
                (Some(d), _) => format!("a depot not ours {:.0} tiles from the base", d),
                (None, Some(_)) => "every depot ours; pillboxes left far out".into(),
                _ => "the map is ours".into(),
            },
            "always".into(),
        ];
        let rung = Rung::ALL.iter().copied().find(|r| self.ladder[r.index()] && holds[r.index()]).unwrap_or(Rung::Guard);
        if self.rung != Some(rung) {
            self.last_move = None;
            self.rung = Some(rung);
        }
        match rung {
            Rung::DefendBase | Rung::Guard => self.defend_base(obs, me, cmds),
            Rung::Fortify => {
                self.fortify(obs, me, Some(base), cmds);
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Rung::Turrets => {
                self.turrets(obs, me, cmds);
                true
            }
            Rung::Hunt => {
                self.gun_reserved = false;
                self.hunt(obs, me, Some((base, EXPAND_R)), cmds);
                !self.gun_reserved
            }
            Rung::Expand => self.expand(obs, me, base, cmds),
        }
    }

    /// Expand: the nearest depot not of my side to the base, its guards
    /// first; with none to be had, any pillbox on the map.
    fn expand(&mut self, obs: &Observation, me: &Tank, base: V2, cmds: &mut Vec<Command>) -> bool {
        for e in &obs.events {
            if let Event::Rejected { player, cmd: Command::Move { .. }, why } = e {
                if *player == obs.player && (why.contains("water") || why.contains("unreachable")) {
                    if let Some(d) = self.depot_target.take() {
                        self.depots_given_up.push(d);
                    }
                }
            }
        }
        let depot = obs
            .depots
            .iter()
            .filter(|d| !Self::mine_or_allied(obs, d.owner) && !self.depots_given_up.contains(&d.id))
            .min_by(|a, b| a.pos.dist(base).partial_cmp(&b.pos.dist(base)).unwrap().then(a.id.cmp(&b.id)))
            .cloned();
        let Some(d) = depot else {
            self.depot_target = None;
            self.gun_reserved = false;
            self.hunt(obs, me, None, cmds);
            return !self.gun_reserved;
        };
        self.depot_target = Some(d.id);
        let guard = PILLBOX_RANGE + 1.5;
        let guarded = obs.pillboxes.iter().any(|p| p.alive() && !Self::mine_or_allied(obs, p.owner) && !self.given_up.contains(&p.id) && p.pos.dist(d.pos) <= guard);
        if guarded {
            self.gun_reserved = false;
            self.hunt(obs, me, Some((d.pos, guard)), cmds);
            self.doing = format!("clearing the depot at ({:.0},{:.0}): {}", d.pos.x, d.pos.y, self.doing);
            return !self.gun_reserved;
        }
        if me.pos.dist(d.pos) > DEPOT_RADIUS * 0.5 {
            self.go(obs, me, d.pos, cmds);
            self.doing = format!("to the depot at ({:.0},{:.0}), {:.0} tiles", d.pos.x, d.pos.y, me.pos.dist(d.pos));
        } else {
            self.doing = format!("taking the depot: {:.0}%", d.capture as f32 * 100.0 / CAPTURE_TICKS as f32);
        }
        true
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
        self.defend_within(obs, me, at, DEFEND_LEASH, mend, cmds)
    }

    fn defend_within(&mut self, obs: &Observation, me: &Tank, at: V2, leash: f32, mend: Option<EntityId>, cmds: &mut Vec<Command>) {
        // A hostile tank inside the leash and in direct reach is shot
        // here; beyond, the inner doctrine's indirect fire answers.
        let threat = obs
            .tanks
            .iter()
            .filter(|t| t.alive && !obs.allied(t.owner) && t.pos.dist(at) <= leash)
            .min_by(|a, b| a.pos.dist(me.pos).partial_cmp(&b.pos.dist(me.pos)).unwrap().then(a.id.cmp(&b.id)));
        match threat {
            Some(t) if t.pos.dist(me.pos) <= DIRECT_RANGE - 0.2 => {
                if me.reload == 0 && me.shells > 0 && obs.tick >= self.hold_fire_until {
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

    fn status(&self) -> String {
        match (self.goal, self.rung) {
            (Goal::Manual, _) => String::new(),
            (Goal::Auto, Some(r)) => format!("auto > {}: {}", r.label(), self.doing),
            (g, _) => format!("{}: {}", g.label(), self.doing),
        }
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        let mut cmds = Vec::new();
        // The doctrine thinks every tick so its memory stays current;
        // what it says is kept or dropped below.
        let doctrine = self.inner.think(obs);
        if self.goal == Goal::Manual {
            return cmds;
        }
        for e in &obs.events {
            if let Event::Rejected { player, cmd: Command::FireDirect { .. }, why } = e {
                if *player == obs.player && why.contains("line") {
                    self.hold_fire_until = obs.tick + 2 * TICK_HZ as Tick;
                }
            }
        }
        let Some(me) = obs.me.clone() else { return cmds };
        if !me.alive {
            self.doing = "dead".into();
            return cmds;
        }
        // A bot whose leader is down takes command (fleet.rs): a wingman
        // or a guard left on its own goes to Auto at the same base. A
        // fleet's survivors otherwise stood guard for half an hour.
        if self.social || self.base_owner.is_some() {
            let leader = match self.goal {
                Goal::Escort(p) => Some(p),
                Goal::DefendBase => self.base_owner,
                _ => None,
            };
            if let Some(l) = leader.filter(|l| *l != obs.player) {
                if obs.players.iter().any(|p| p.id == l && !p.alive) {
                    self.base_owner.get_or_insert(l);
                    self.set_goal(Goal::Auto);
                    self.doing = "the leader is down: taking command".into();
                }
            }
        }
        // A shell in flight at the tank says where it was fired from.
        if let Some(from) = obs.shells.iter().filter(|sh| !obs.allied(sh.owner) && sh.impact.dist(me.pos) <= 4.0).max_by_key(|sh| sh.launch_tick).map(|sh| sh.from) {
            self.gun_seen_at = Some(from);
        }
        // Shot at by what nobody of ours can see: off the spot, and eyes up.
        let hit_now = me.last_hit_tick > 0 && obs.tick.saturating_sub(me.last_hit_tick) <= 1;
        let in_sight = obs.tanks.iter().any(|t| t.alive && !obs.allied(t.owner));
        if hit_now && !in_sight && self.scoot.map(|(_, until)| obs.tick >= until).unwrap_or(true) {
            self.scoot_left = !self.scoot_left;
            let side = V2::from_heading(me.heading + if self.scoot_left { 1.5708 } else { -1.5708 });
            self.scoot = Some((me.pos + side.scale(SCOOT), obs.tick + SCOOT_FOR));
            self.last_move = None;
            // A shell says where it was fired from: eyes there, and the
            // gun back at it while it is in reach.
            let origin = self.gun_seen_at;
            let over = origin.unwrap_or(me.pos);
            let task = DroneTask::Orbit { center: over, radius: if origin.is_some() { 4.0 } else { RECON_VISION * 0.6 } };
            match obs.drones.iter().find(|d| d.owner == obs.player && d.kind == DroneKind::Recon) {
                Some(d) => cmds.push(Command::Retask { drone: d.id, task }),
                None if me.bay.recon > 0 => cmds.push(Command::Launch { kind: DroneKind::Recon, task }),
                None => {}
            }
        }
        if let (Some(at), true) = (self.gun_seen_at, self.scoot.map(|(_, until)| obs.tick < until).unwrap_or(false)) {
            let d = me.pos.dist(at);
            if me.reload == 0 && me.shells > LOW_SHELLS && (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&d) {
                cmds.push(Command::FireIndirect { at });
            }
        }
        if let Some((to, until)) = self.scoot {
            if obs.tick < until && !in_sight {
                // A side that is water is refused: the other side, at once.
                if obs.events.iter().any(|e| matches!(e, Event::Rejected { player, cmd: Command::Move { .. }, .. } if *player == obs.player)) {
                    let back = me.pos + (me.pos - to);
                    self.scoot = Some((back, until));
                    self.last_move = None;
                }
                let to = self.scoot.unwrap().0;
                self.go(obs, &me, to, &mut cmds);
                self.doing = "under fire from out of sight: scooting, recon up".into();
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
            self.scoot = None;
            self.last_move = None;
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
            Goal::Fortify(at) => {
                self.fortify(obs, &me, at, &mut cmds);
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::DefendBase => self.defend_base(obs, &me, &mut cmds),
            Goal::Auto => self.auto(obs, &me, &mut cmds),
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
    use crate::fortify::TOWER_R;

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

    /// 11b's proof: alone on the island for five minutes, the squad
    /// builds the castle round the home depot: the towers and the rear
    /// pillbox, most of the wall, the gate left open, and a route from
    /// the front into the depot comes in through the gate.
    #[test]
    fn a_commander_fortifies_its_depot() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let me = w.tanks[0].pos;
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(me).partial_cmp(&b.pos.dist(me)).unwrap()).unwrap().id;
        w.depots.iter_mut().find(|d| d.id == depot).unwrap().owner = Some(0);
        let mut c = Commander::new(11, "commander");
        c.set_goal(Goal::Fortify(None));
        let mut walls = 0;
        for i in 0..(300 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            if std::env::var("COMMANDER_TRACE").is_ok() && i % (10 * TICK_HZ) == 0 {
                eprintln!("[{:>5}] material {} doing '{}'", w.tick, w.tanks[0].pillboys.material, c.doing);
            }
            w.step(&orders);
            for e in &w.events {
                if let Event::Built { owner: 0, what: BuildKind::Wall, .. } = e {
                    walls += 1;
                }
                if std::env::var("COMMANDER_TRACE").is_ok() {
                    if let Event::Rejected { player: 0, why, .. } | Event::PillboysAbandoned { owner: 0, why, .. } = e {
                        eprintln!("        refused: {}", why);
                    }
                }
            }
        }
        let plan = c.plan.clone().expect("no plan");
        let towers = w.pillboxes.iter().filter(|p| p.owner == Some(0) && p.pos.dist(plan.centre) <= TOWER_R + 1.5).count();
        let gate = plan.gate();
        eprintln!("a_commander_fortifies_its_depot: {}; {} pillboxes of ours round the depot, {} walls", plan.progress(), towers, walls);
        assert!(towers >= 3, "{} pillboxes round the depot", towers);
        assert!(walls >= 8, "{} wall tiles", walls);
        assert!(plan.gate_tiles.iter().all(|g| w.map.at(*g).passable()), "the gate is shut");
        // From twelve tiles out in front, the way in is the gate.
        let mut front = Map::tile_centre(plan.centre + plan.facing.scale(12.0));
        while !w.map.at(front).passable() {
            front = Map::tile_centre(front - plan.facing);
        }
        let route = w.map.route(front, plan.centre);
        assert!(!route.is_empty(), "no way in at all");
        // The route is waypoints with straight legs between: the gate
        // is passed if a leg runs within a tile and a half of it.
        let near_leg = |a: V2, b: V2| {
            let ab = b - a;
            let t = (((gate - a).x * ab.x + (gate - a).y * ab.y) / (ab.x * ab.x + ab.y * ab.y).max(1e-6)).clamp(0.0, 1.0);
            (a + ab.scale(t)).dist(gate)
        };
        let mut legs = vec![front];
        legs.extend(route.iter().copied());
        let through = legs.windows(2).any(|l| near_leg(l[0], l[1]) <= 1.5);
        assert!(through, "the way in does not pass the gate: {:?} from {:?}, gate {:?}", route, front, gate);
        // And the wall does its work: with the gate shut, the way in from
        // the front is longer (round the wall's end), so the gate is the
        // short way and the router's choice.
        let open_len: f32 = legs.windows(2).map(|l| l[0].dist(l[1])).sum();
        for g in &plan.gate_tiles {
            w.map.set(*g, Terrain::Wall);
        }
        let shut = w.map.route(front, plan.centre);
        let mut shut_legs = vec![front];
        shut_legs.extend(shut.iter().copied());
        let shut_len: f32 = shut_legs.windows(2).map(|l| l[0].dist(l[1])).sum();
        eprintln!("  the way in: {:.1} tiles through the gate, {:.1} with it shut", open_len, shut_len);
        if std::env::var("COMMANDER_TRACE").is_ok() {
            let c = plan.centre;
            for dy in -8..=8 {
                let mut line = String::new();
                for dx in -8..=8 {
                    let p = Map::tile_centre(c + V2::new(dx as f32, dy as f32));
                    let ch = if shut.iter().any(|q| q.tile() == p.tile()) { '*' } else if w.pillboxes.iter().any(|b| b.pos.tile() == p.tile()) { 'P' } else { match w.map.at(p) { Terrain::Wall => '#', Terrain::Road => '=', Terrain::Depot => 'D', Terrain::Forest => 'f', Terrain::Water => '~', t if !t.passable() => 'x', _ => '.' } };
                    line.push(ch);
                }
                eprintln!("  {}", line);
            }
            eprintln!("  facing {:?}; shut route {:?}", plan.facing, shut);
        }
        assert!(shut.is_empty() || shut_len > open_len + 2.0, "the wall does not turn anyone: {:.1} against {:.1}", shut_len, open_len);
    }

    /// A free tile about `r` from `from`: open ground, off depots and
    /// pillboxes' spacing.
    fn free_near(w: &World, from: V2, r: f32) -> V2 {
        (0..64)
            .map(|i| Map::tile_centre(from + V2::from_heading(i as f32 * 0.7).scale(r + (i / 16) as f32)))
            .find(|p| w.map.in_bounds(*p) && w.map.at(*p).passable() && !matches!(w.map.at(*p), Terrain::Forest | Terrain::Depot) && !w.depots.iter().any(|d| d.pos.dist(*p) <= DEPOT_RADIUS + 1.0) && !w.pillboxes.iter().any(|b| b.pos.dist(*p) < PILLBOX_SPACING) && w.map.route(from, *p).len() > 0)
            .expect("no free tile")
    }

    /// Defend the base: a tank parked 12 tiles out is a threat (beyond
    /// DefendDepot's leash, inside the base's alarm); it is met under the
    /// base's guns and shot, and the hurt pillbox is mended once it is gone.
    #[test]
    fn a_commander_defends_its_base() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(w.tanks[0].pos).partial_cmp(&b.pos.dist(w.tanks[0].pos)).unwrap()).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let at = free_near(&w, depot.pos, 4.0);
        let guard = 70_020;
        w.pillboxes.push(Pillbox { id: guard, pos: at, owner: Some(0), health: 20.0, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let mut c = Commander::new(12, "commander");
        c.set_goal(Goal::DefendBase);
        let mut warned = false;
        let mut hits = 0;
        let mut mended = None;
        for i in 0..(150 * TICK_HZ) {
            if i == 10 * TICK_HZ {
                let spot = free_near(&w, depot.pos, 12.0);
                let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
                t.pos = spot;
                t.goal = None;
                t.route.clear();
            }
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            warned |= c.doing.starts_with("BASE UNDER THREAT");
            w.step(&orders);
            for e in &w.events {
                match e {
                    Event::TankDamaged { tank, by: Some(0), .. } if w.tanks.iter().any(|t| t.id == *tank && t.owner == 1) => hits += 1,
                    Event::PillboxRepaired { pillbox, .. } if *pillbox == guard && mended.is_none() => mended = Some(w.tick),
                    _ => {}
                }
            }
        }
        let dead = !w.tanks.iter().find(|t| t.owner == 1).unwrap().alive;
        eprintln!("a_commander_defends_its_base: warned {}, {} hits, intruder dead {}, mended at {:?}", warned, hits, dead, mended.map(|t| t as f32 / TICK_HZ as f32));
        assert!(warned, "never saw the threat");
        assert!(hits >= 2, "the intruder was hit {} times", hits);
        assert!(mended.is_some(), "the base's pillbox was never mended");
        assert!(w.pillbox(guard).unwrap().health >= PILLBOX_HEALTH);
    }

    /// Shelled from twenty tiles, beyond sight: the tank scoots off the
    /// spot inside a second, puts a recon up, and so sees the shooter.
    #[test]
    fn a_commander_scoots_from_unseen_fire() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(w.tanks[0].pos).partial_cmp(&b.pos.dist(w.tanks[0].pos)).unwrap()).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let mut c = Commander::new(14, "commander");
        c.set_goal(Goal::DefendBase);
        // Settle at the post first.
        for _ in 0..(20 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
        }
        let gun = free_near(&w, w.tanks[0].pos, 20.0);
        let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
        t.pos = gun;
        t.goal = None;
        t.route.clear();
        let mut first_hit: Option<(Tick, V2)> = None;
        let mut moved = 0.0f32;
        let mut recon = false;
        let mut seen = false;
        let mut answered = 0;
        for _ in 0..(40 * TICK_HZ) {
            let obs = w.observe(0);
            seen |= obs.tanks.iter().any(|t| t.owner == 1);
            let mut orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            recon |= orders.iter().any(|o| matches!(o.cmd, Command::Launch { kind: DroneKind::Recon, .. } | Command::Retask { .. }));
            let them = w.tanks.iter().find(|t| t.owner == 1).unwrap();
            if them.reload == 0 && them.alive {
                orders.push(Order { player: 1, cmd: Command::FireIndirect { at: w.tanks[0].pos } });
            }
            answered += orders.iter().filter(|o| o.player == 0 && matches!(o.cmd, Command::FireIndirect { .. })).count();
            w.step(&orders);
            if first_hit.is_none() && w.events.iter().any(|e| matches!(e, Event::TankDamaged { tank, .. } if *tank == w.tanks[0].id)) {
                first_hit = Some((w.tick, w.tanks[0].pos));
            }
            if let Some((t0, p0)) = first_hit {
                if w.tick <= t0 + 2 * TICK_HZ as Tick {
                    moved = moved.max(w.tanks[0].pos.dist(p0));
                }
            }
        }
        eprintln!("a_commander_scoots_from_unseen_fire: hit at {:?}, moved {:.1} in 2 s, recon {}, saw the gun {}, fired back {}", first_hit.map(|(t, _)| t), moved, recon, seen, answered);
        assert!(first_hit.is_some(), "never hit: the test's gun missed");
        assert!(moved >= 1.5, "stood still under fire: {:.1}", moved);
        assert!(recon, "no recon up");
        assert!(seen, "never saw the gun");
        assert!(answered > 0, "never fired back");
    }

    /// Auto, alone: fortify first; once the castle stands and the base has
    /// been quiet, the two outlying pillboxes of ours are lifted and set
    /// down in the turrets; a tank at the base sends it to defend.
    #[test]
    fn a_commander_climbs_the_ladder() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let me = w.tanks[0].pos;
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(me).partial_cmp(&b.pos.dist(me)).unwrap()).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let mut out = Vec::new();
        for (n, r) in [16.0, 22.0].into_iter().enumerate() {
            let at = free_near(&w, depot.pos, r);
            let id = 70_030 + n as EntityId;
            w.pillboxes.push(Pillbox { id, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
            out.push(id);
        }
        // The other seat is out of the way until it is wanted.
        let far = free_near(&w, depot.pos, 40.0);
        let enemy = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
        enemy.pos = far;
        let mut c = Commander::new(13, "commander");
        c.set_goal(Goal::Auto);
        let mut rungs: Vec<(f32, Rung)> = Vec::new();
        let mut lifted = 0;
        let mut defended_at = None;
        let mut intruder_at = None;
        for _ in 0..(900 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            if let Some(r) = c.rung {
                if rungs.last().map(|(_, l)| *l != r).unwrap_or(true) {
                    rungs.push((w.tick as f32 / TICK_HZ as f32, r));
                    if std::env::var("COMMANDER_TRACE").is_ok() {
                        eprintln!("[{:>6.1}s] {:?}: {}", w.tick as f32 / TICK_HZ as f32, r, c.doing);
                    }
                }
                if r == Rung::DefendBase && intruder_at.is_some() && defended_at.is_none() {
                    defended_at = Some(w.tick);
                }
            }
            if std::env::var("COMMANDER_TRACE").is_ok() && w.tick % (5 * TICK_HZ as Tick) == 0 && w.tick > 330 * TICK_HZ as Tick {
                eprintln!("[{:>6.1}s] {:?} doing '{}' at {:?} squad {:?} why {:?}", w.tick as f32 / TICK_HZ as f32, c.rung, c.doing, w.tanks[0].pos, (w.tanks[0].pillboys.aboard, &w.tanks[0].pillboys.task, w.tanks[0].pillboys.carrying), c.why[2]);
            }
            for e in &w.events {
                if std::env::var("COMMANDER_TRACE").is_ok() {
                    if let Event::Rejected { player: 0, why, .. } | Event::PillboysAbandoned { owner: 0, why, .. } = e {
                        eprintln!("        refused: {}", why);
                    }
                }
            }
            w.step(&orders);
            lifted += w.events.iter().filter(|e| matches!(e, Event::PillboxLifted { by: 0, .. })).count();
            // Both turrets up: an intruder comes to the gate.
            if intruder_at.is_none() && c.plan.as_ref().map(|p| p.turrets() == (2, 2)).unwrap_or(false) {
                let gate = c.plan.as_ref().unwrap().gate();
                let spot = free_near(&w, gate, 0.0);
                let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
                t.pos = spot;
                intruder_at = Some(w.tick);
            }
            if defended_at.is_some() {
                break;
            }
        }
        let plan = c.plan.clone().expect("no plan");
        eprintln!("a_commander_climbs_the_ladder: {}; lifted {}; rungs {:?}", plan.progress(), lifted, rungs);
        let order: Vec<Rung> = rungs.iter().map(|(_, r)| *r).collect();
        assert_eq!(order.first(), Some(&Rung::Fortify), "{:?}", rungs);
        assert!(plan.castle_stands(), "{}", plan.progress());
        let first_turret = order.iter().position(|r| *r == Rung::Turrets).expect("never on turrets");
        assert!(order[..first_turret].contains(&Rung::Fortify));
        assert_eq!(lifted, 2, "lifted {}", lifted);
        assert_eq!(plan.turrets(), (2, 2), "{}", plan.progress());
        assert!(out.iter().all(|id| w.pillbox(*id).is_none()), "an outlying pillbox is still out");
        let (i, d) = (intruder_at.expect("never finished the turrets"), defended_at.expect("never defended"));
        assert!(d - i <= TICK_HZ as Tick, "defended {} ticks after the intruder came", d - i);
    }

    /// The window's solo match, headless: the map in LADDER_MAP, seat 0
    /// on Auto against three RuleBots seeded as the window seeds them.
    /// Run by hand (`--ignored`) to read what the ladder does in a match.
    #[test]
    #[ignore]
    fn the_ladder_in_a_match() {
        let text = std::fs::read_to_string(std::env::var("LADDER_MAP").expect("LADDER_MAP")).unwrap();
        let seed = 0x7EAD_C0DE_u64;
        let names = vec![("user".to_string(), false), ("bot1".into(), true), ("bot2".into(), true), ("bot3".into(), true)];
        let sc = Scenario { seed, map: MapSource::Text(text), players: names, ..Scenario::default() };
        let mut w = World::new(sc);
        let mut bots: Vec<(PlayerId, RuleBot)> = (1..4).map(|i| (i as PlayerId, RuleBot::new(seed.wrapping_mul(31) ^ (i as u64 + 1), "rulebot"))).collect();
        let mut c = Commander::new(0x60A1, "goal");
        c.set_goal(Goal::Auto);
        let secs: u64 = std::env::var("LADDER_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(600);
        for _ in 0..(secs * TICK_HZ as u64) {
            let mut orders: Vec<Order> = c.think(&w.observe(0)).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            for (p, b) in bots.iter_mut() {
                orders.extend(b.think(&w.observe(*p)).into_iter().map(|cmd| Order { player: *p, cmd }));
            }
            w.step(&orders);
            let t = w.tick as f32 / TICK_HZ as f32;
            if w.tick % (10 * TICK_HZ as Tick) == 0 {
                let me = &w.tanks[0];
                eprintln!("[{:>6.1}s] {:?} '{}' armour {:.0} shells {} squad {:?}/{:?} material {}", t, c.rung, c.doing, me.armor, me.shells, me.pillboys.aboard, me.pillboys.task, me.pillboys.material);
            }
            for e in &w.events {
                match e {
                    Event::TankDamaged { tank, by, damage, armor_left } if *tank == w.tanks[0].id => {
                        let near = w.tanks.iter().filter(|o| o.owner != 0 && o.alive).map(|o| format!("p{} {:.0}", o.owner, o.pos.dist(w.tanks[0].pos))).collect::<Vec<_>>();
                        eprintln!("[{:>6.1}s]   hit -{:.0} by {:?}, {:.0} left; tanks at {:?}; drones near {}", t, damage, by, armor_left, near, w.drones.iter().filter(|d| d.owner != 0 && d.pos.dist(w.tanks[0].pos) < 12.0).count());
                    }
                    Event::Rejected { player: 0, why, .. } | Event::PillboysAbandoned { owner: 0, why, .. } => eprintln!("[{:>6.1}s]   refused: {}", t, why),
                    Event::TankDestroyed { owner: 0, by, .. } => eprintln!("[{:>6.1}s] DESTROYED by {:?}", t, by),
                    _ => {}
                }
            }
            if !w.tanks[0].alive {
                break;
            }
        }
        if let Some(p) = &c.plan {
            eprintln!("plan: {}; items {:?}", p.progress(), p.items.iter().map(|i| (i.what, i.at, i.state)).collect::<Vec<_>>());
        }
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
