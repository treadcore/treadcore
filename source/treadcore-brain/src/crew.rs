//! The fleet commander and its crews (Paul, 16 October: "each side has
//! a fleet commander and they request and organize fort construction").
//!
//! One Commander a side commands: the leader of a bots' team, the
//! builder of a person's. It keeps the side's castle plan (fortify.rs)
//! as it always did, building with its own squad; what is new is that
//! it does not build alone. It talks to the side through the world
//! (`Command::Request`, `Event::Requested`), so it works the same over a
//! network and every window of the side reads it on the newswire:
//!
//!   1. Muster, every `MUSTER_EVERY` while the plan is unfinished: "the
//!      fort is at (x, y); crews report".
//!   2. A crew free for work within `CREW_REACH` of the fort answers
//!      Ready.
//!   3. The commander gives each ready crew the next item of the plan
//!      nobody has (a wall, the road, a tower), or, for a crew that owns
//!      an outlying pillbox, a turret slot to bring it to (Fetch: lift
//!      it, carry it, set it down).
//!   4. The crew does it: fells forest first when its squad is short,
//!      drives in reach, sends its squad, and when the world answers
//!      (built, refused, abandoned) reports Ready again for the next.
//!   5. An item not done `JOB_FOR` after it was given out is given out
//!      again; one that has failed `FAILS` times the plan does without.
//!
//! Who crews: rangers and a bots' team's wingmen, and a person's
//! wingmen only while the person is at the fort, so they still go where
//! the person goes. Guards guard.

use crate::fortify::Plan;
use treadcore_sim::*;

/// Seconds between musters while the fort is unfinished.
pub const MUSTER_EVERY: Tick = 30 * TICK_HZ as Tick;
/// A crew this far from the fort answers a muster.
pub const CREW_REACH: f32 = 60.0;
/// A job not reported done in this long is given out again.
pub const JOB_FOR: Tick = 60 * TICK_HZ as Tick;
/// A crew gives a job up after this long.
const WORK_FOR: Tick = 90 * TICK_HZ as Tick;
/// A haul is a crawl: a tenth of a tank's speed. It gets this long.
pub const HAUL_FOR: Tick = 4 * 60 * TICK_HZ as Tick;
/// An ally's pillbox this near the fort is hauled to a turret slot.
pub const HAUL_REACH: f32 = 40.0;

fn job_for(job: &Job) -> Tick {
    if matches!(job, Job::Haul { .. }) {
        HAUL_FOR
    } else {
        JOB_FOR
    }
}
/// Failures before the plan does without an item.
pub const FAILS: u8 = 2;
/// A crew's Ready again at most this often unprompted.
const READY_EVERY: Tick = 10 * TICK_HZ as Tick;

fn job_tile(job: &Job) -> Option<(i32, i32)> {
    match job {
        Job::Build { at, .. } => Some(at.tile()),
        Job::Fetch { to, .. } | Job::Haul { to, .. } => Some(to.tile()),
        _ => None,
    }
}

/// The commander's side of it.
#[derive(Clone, Debug, Default)]
pub struct FleetCommand {
    pub on: bool,
    last_muster: Tick,
    /// Crews that reported ready, and when.
    crew: Vec<(PlayerId, Tick)>,
    /// What each crew was given, and when.
    assigned: Vec<(PlayerId, Job, Tick)>,
    fails: Vec<((i32, i32), u8)>,
}

impl FleetCommand {
    pub fn crews(&self) -> usize {
        self.crew.len()
    }

    fn failed(&mut self, plan: &mut Plan, job: &Job) {
        let Some(tile) = job_tile(job) else { return };
        let n = match self.fails.iter_mut().find(|(t, _)| *t == tile) {
            Some((_, n)) => {
                *n += 1;
                *n
            }
            None => {
                self.fails.push((tile, 1));
                1
            }
        };
        if n >= FAILS {
            plan.give_up(V2::new(tile.0 as f32 + 0.5, tile.1 as f32 + 0.5));
        }
    }

    /// Every tick: the requests to send. `plan` is the commander's.
    pub fn think(&mut self, obs: &Observation, base: V2, plan: Option<&mut Plan>) -> Vec<Command> {
        let mut out = Vec::new();
        let Some(plan) = plan else { return out };
        if !self.on {
            plan.skip.clear();
            return out;
        }
        let alive = |p: PlayerId| obs.players.iter().any(|x| x.id == p && x.alive);
        for e in &obs.events {
            if let Event::Requested { from, to: Some(to), job: Job::Ready } = e {
                if *to != obs.player || !obs.allied(*from) {
                    continue;
                }
                // Its last job is over, done or not.
                if let Some(ix) = self.assigned.iter().position(|(p, _, _)| p == from) {
                    let (_, job, _) = self.assigned.remove(ix);
                    let done = job_tile(&job).map(|t| !plan.items.iter().any(|i| i.at.tile() == t && !matches!(i.state, crate::fortify::State::Done | crate::fortify::State::Skipped))).unwrap_or(true);
                    if !done {
                        self.failed(plan, &job);
                    }
                }
                match self.crew.iter_mut().find(|(p, _)| p == from) {
                    Some(c) => c.1 = obs.tick,
                    None => self.crew.push((*from, obs.tick)),
                }
            }
        }
        self.crew.retain(|(p, _)| alive(*p));
        // Late jobs go out again.
        let late: Vec<(PlayerId, Job, Tick)> = self.assigned.iter().filter(|(p, j, t)| obs.tick >= t + job_for(j) || !alive(*p)).cloned().collect();
        self.assigned.retain(|(p, j, t)| obs.tick < t + job_for(j) && alive(*p));
        for (p, job, _) in late {
            self.failed(plan, &job);
            self.crew.retain(|(c, _)| *c != p);
        }
        let unfinished = !plan.castle_stands() || !plan.turrets_stand();
        if !unfinished {
            plan.skip.clear();
            return out;
        }
        if self.last_muster == 0 || obs.tick >= self.last_muster + MUSTER_EVERY {
            self.last_muster = obs.tick;
            out.push(Command::Request { to: None, job: Job::Muster { at: base } });
        }
        let mut taken: Vec<(i32, i32)> = self.assigned.iter().filter_map(|(_, j, _)| job_tile(j)).collect();
        plan.skip = taken.clone();
        if let Some(t) = plan.own_order() {
            taken.push(t);
        }
        let mut idle: Vec<PlayerId> = self.crew.iter().map(|(p, _)| *p).filter(|p| !self.assigned.iter().any(|(a, _, _)| a == p)).collect();
        // A pillbox of an ally (a person's, a bot's that is no crew of
        // mine) out of the fort: two crews chain it and drag it to a
        // turret slot. A crew's own it fetches with its squad, below.
        if plan.castle_stands() && idle.len() >= 2 {
            let crews: Vec<PlayerId> = self.crew.iter().map(|(p, _)| *p).collect();
            let hauled: Vec<EntityId> = self.assigned.iter().filter_map(|(_, j, _)| if let Job::Haul { target, .. } = j { Some(*target) } else { None }).collect();
            let theirs = obs
                .pillboxes
                .iter()
                .filter(|b| b.owner.map(|o| o != obs.player && obs.allied(o) && !crews.contains(&o)).unwrap_or(false) && !hauled.contains(&b.id))
                .filter(|b| b.pos.dist(base) > crate::commander::BASE_R && b.pos.dist(base) <= HAUL_REACH)
                .min_by(|a, b| a.pos.dist(base).partial_cmp(&b.pos.dist(base)).unwrap().then(a.id.cmp(&b.id)));
            if let (Some(b), Some(slot)) = (theirs, plan.free_slots(&taken).first().copied()) {
                idle.sort_by(|x, y| {
                    let d = |p: &PlayerId| obs.allied_tanks().find(|t| t.owner == *p).map(|t| t.pos.dist(b.pos)).unwrap_or(f32::INFINITY);
                    d(x).partial_cmp(&d(y)).unwrap().then(x.cmp(y))
                });
                let job = Job::Haul { target: b.id, to: slot };
                taken.push(slot.tile());
                for p in idle.drain(..2) {
                    self.assigned.push((p, job.clone(), obs.tick));
                    out.push(Command::Request { to: Some(p), job: job.clone() });
                }
            }
        }
        for p in idle {
            // A crew with a pillbox out of the fort: a turret slot for it.
            let outlying = obs.pillboxes.iter().filter(|b| b.owner == Some(p) && b.pos.dist(base) > crate::commander::BASE_R).min_by(|a, b| a.pos.dist(base).partial_cmp(&b.pos.dist(base)).unwrap().then(a.id.cmp(&b.id)));
            let slot = plan.free_slots(&taken).first().copied();
            let job = match (outlying, slot) {
                (Some(b), Some(s)) if plan.castle_stands() => Job::Fetch { pillbox: b.id, to: s },
                _ => match plan.free_item(&taken) {
                    Some((what, at)) => Job::Build { what, at },
                    None => continue,
                },
            };
            if let Some(t) = job_tile(&job) {
                taken.push(t);
            }
            self.assigned.push((p, job.clone(), obs.tick));
            out.push(Command::Request { to: Some(p), job });
        }
        plan.skip = self.assigned.iter().filter_map(|(_, j, _)| job_tile(j)).collect();
        out
    }
}

/// A crew's side of it.
#[derive(Clone, Debug, Default)]
pub struct Crew {
    pub on: bool,
    commander: Option<PlayerId>,
    /// A fleet wingman's leader: its bridge tiles are jobs too.
    pub leader: Option<PlayerId>,
    muster: Option<V2>,
    job: Option<(Job, Tick)>,
    /// The squad was sent on the job at this tick.
    sent: Option<Tick>,
    /// Forest on the job's tile: fell it first.
    fell_first: bool,
    last_ready: Tick,
    last_move: Tick,
    pub doing: String,
    /// The newest job taken, by the tick it was given: the board does
    /// not hand back one already had.
    adopted: Option<Tick>,
}

impl Crew {
    /// A new tank: the job it had is someone else's now (the commander
    /// gives it out again when it is late).
    pub fn reset(&mut self) {
        self.job = None;
        self.sent = None;
        self.fell_first = false;
    }

    pub fn muster_at(&self) -> Option<V2> {
        self.muster
    }

    /// What this crew is hauling, if its job is a haul.
    pub fn hauling(&self) -> Option<EntityId> {
        match &self.job {
            Some((Job::Haul { target, .. }, _)) => Some(*target),
            _ => None,
        }
    }

    pub fn working(&self) -> bool {
        self.job.is_some()
    }

    /// The job is a road or a bridge tile: work wherever the fleet is.
    pub fn laying_road(&self) -> bool {
        matches!(self.job, Some((Job::Build { what: BuildKind::Road, .. }, _)))
    }

    fn ready(&mut self, obs: &Observation, out: &mut Vec<Command>) {
        if let Some(c) = self.commander {
            self.last_ready = obs.tick;
            out.push(Command::Request { to: Some(c), job: Job::Ready });
        }
    }

    fn finish(&mut self, obs: &Observation, out: &mut Vec<Command>, free: bool) {
        self.job = None;
        self.sent = None;
        self.fell_first = false;
        if free {
            self.ready(obs, out);
        }
    }

    fn walk(&mut self, obs: &Observation, me: &Tank, site: V2, out: &mut Vec<Command>) {
        let near = site + (me.pos - site).norm().scale(3.0);
        let going = me.route.last().copied().or(me.goal).map(|g| g.dist(near) < 2.0).unwrap_or(false);
        if !going && obs.tick >= self.last_move + TICK_HZ as Tick {
            self.last_move = obs.tick;
            out.push(Command::Move { to: obs.clamp(near) });
        }
    }

    /// Every tick, before anything else of the bot's can return: the
    /// jobs given it. A job is an event of one tick, and a wingman busy
    /// with something else that tick (a dodge, a rally, a crossing) lost
    /// its leader's bridge tiles.
    /// A job the team board kept for this crew (board.rs, roadmap 12e):
    /// taken if the crew has none and has not had this one.
    pub fn adopt(&mut self, job: Job, at: Tick) {
        if !self.on || self.job.is_some() || self.adopted >= Some(at) {
            return;
        }
        self.adopted = Some(at);
        self.job = Some((job, at));
        self.sent = None;
        self.fell_first = false;
    }

    pub fn take_jobs(&mut self, obs: &Observation) {
        for e in &obs.events {
            if let Event::Requested { from, to: Some(to), job } = e {
                if *to == obs.player && (Some(*from) == self.commander || Some(*from) == self.leader) && self.on {
                    match job {
                        Job::Build { .. } | Job::Fetch { .. } | Job::Haul { .. } => {
                            self.adopted = Some(obs.tick);
                            self.job = Some((job.clone(), obs.tick));
                            self.sent = None;
                            self.fell_first = false;
                        }
                        Job::Release => {
                            self.job = None;
                            self.sent = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Every tick. `free`: the bot may work now (not rallying, not going
    /// home, its leader at the fort if it is a wingman). Some(orders)
    /// while it is working: the caller sends them and drives nothing
    /// else.
    pub fn think(&mut self, obs: &Observation, me: &Tank, free: bool) -> Option<Vec<Command>> {
        let mut out = Vec::new();
        let mine = obs.player;
        for e in &obs.events {
            match e {
                Event::Requested { from, to: None, job: Job::Muster { at } } if obs.allied(*from) && *from != mine => {
                    self.commander = Some(*from);
                    self.muster = Some(*at);
                    if self.on && free && self.job.is_none() && me.pos.dist(*at) <= CREW_REACH && obs.tick >= self.last_ready + READY_EVERY {
                        self.ready(obs, &mut out);
                    }
                }
                _ => {}
            }
        }
        let Some((job, given)) = self.job.clone() else {
            return if out.is_empty() { None } else { Some(out) };
        };
        // The world's answers to the squad.
        let site = job_tile(&job);
        for e in &obs.events {
            match e {
                Event::Built { owner, at, .. } if *owner == mine && Some(at.tile()) == site => {
                    self.finish(obs, &mut out, free);
                    return Some(out);
                }
                Event::PillboxBuilt { by, at, .. } if *by == mine && Some(at.tile()) == site => {
                    self.finish(obs, &mut out, free);
                    return Some(out);
                }
                Event::Rejected { player, cmd: Command::Deploy { task }, why } if *player == mine => {
                    match task {
                        PillboyTask::Build { at, .. } if Some(at.tile()) == site => {
                            if why.contains("harvest it first") {
                                self.fell_first = true;
                                self.sent = None;
                            } else if why.contains("no material") {
                                self.sent = None;
                            } else {
                                self.finish(obs, &mut out, free);
                                return Some(out);
                            }
                        }
                        PillboyTask::Lift { .. } => {
                            self.finish(obs, &mut out, free);
                            return Some(out);
                        }
                        _ => self.sent = None,
                    }
                }
                Event::PillboysAbandoned { owner, .. } if *owner == mine => self.sent = None,
                _ => {}
            }
        }
        if obs.tick >= given + if matches!(job, Job::Haul { .. }) { HAUL_FOR } else { WORK_FOR } {
            if me.towing.is_some() {
                out.push(Command::Tow { target: None });
            }
            self.finish(obs, &mut out, free);
            return Some(out);
        }
        if !free {
            return if out.is_empty() { None } else { Some(out) };
        }
        // A haul needs no squad: a chain, and a second crew on it.
        if let Job::Haul { target, to } = job {
            let at = obs.pillboxes.iter().find(|b| b.id == target).map(|b| b.pos).or_else(|| obs.depots.iter().find(|d| d.id == target).map(|d| d.pos));
            let Some(at) = at else {
                if me.towing.is_some() {
                    out.push(Command::Tow { target: None });
                }
                self.finish(obs, &mut out, true);
                return Some(out);
            };
            if at.dist(to) <= 1.5 {
                if me.towing == Some(target) {
                    out.push(Command::Tow { target: None });
                }
                self.finish(obs, &mut out, true);
                return Some(out);
            }
            if me.towing != Some(target) {
                self.doing = format!("crew: to chain the pillbox at ({:.0},{:.0})", at.x, at.y);
                if me.pos.dist(at) > TOW_REACH * 0.8 {
                    let spot = at + (me.pos - at).norm().scale(1.2);
                    let going = me.goal.map(|g| g.dist(spot) < 1.0).unwrap_or(false) || me.route.last().map(|g| g.dist(spot) < 1.0).unwrap_or(false);
                    if !going && obs.tick >= self.last_move + TICK_HZ as Tick {
                        self.last_move = obs.tick;
                        out.push(Command::Move { to: spot });
                    }
                } else if obs.tick >= self.sent.unwrap_or(0) + TICK_HZ as Tick {
                    self.sent = Some(obs.tick);
                    out.push(Command::Tow { target: Some(target) });
                }
                return Some(out);
            }
            let partners: Vec<PlayerId> = obs.allied_tanks().filter(|t| t.alive && t.owner != mine && t.towing == Some(target)).map(|t| t.owner).collect();
            let haulers = partners.len() + 1;
            if haulers < TOW_HAULERS {
                self.doing = "crew: on the chain, waiting for a second crew".into();
                return Some(out);
            }
            self.doing = format!("crew: hauling to ({:.0},{:.0}), {:.0} tiles", to.x, to.y, at.dist(to));
            // Past the slot by the chain's length, so what trails behind
            // stops on it; the lower seat on the left of the line and the
            // other on the right, so two crews do not drive the same one.
            let dir = (to - at).norm();
            let side = if partners.iter().all(|p| mine < *p) { 0.8 } else { -0.8 };
            let dest = to + dir.scale(TOW_CHAIN) + V2::new(-dir.y, dir.x).scale(side);
            let going = me.route.last().copied().or(me.goal).map(|g| g.dist(dest) < 1.0).unwrap_or(false);
            if !going && obs.tick >= self.last_move + TICK_HZ as Tick {
                self.last_move = obs.tick;
                out.push(Command::Move { to: obs.clamp(dest) });
            }
            return Some(out);
        }
        if !me.pillboys.aboard {
            self.doing = "crew: the squad is at work".into();
            return Some(out);
        }
        if me.pillboys.strength == 0 {
            self.finish(obs, &mut out, false);
            return Some(out);
        }
        let reach = crate::PILLBOY_REACH * 0.5;
        // What to build where; a carried pillbox sets down for nothing.
        let (what, at) = match job {
            Job::Build { what, at } => (what, at),
            Job::Fetch { pillbox, to } => {
                if me.pillboys.carrying.is_some() {
                    (BuildKind::Pillbox, to)
                } else {
                    let Some(b) = obs.pillboxes.iter().find(|b| b.id == pillbox && b.owner == Some(mine)) else {
                        self.finish(obs, &mut out, true);
                        return Some(out);
                    };
                    self.doing = format!("crew: fetching my pillbox from ({:.0},{:.0}) to ({:.0},{:.0})", b.pos.x, b.pos.y, to.x, to.y);
                    if me.pos.dist(b.pos) > reach {
                        self.walk(obs, me, b.pos, &mut out);
                    } else if self.sent.is_none() {
                        self.sent = Some(obs.tick);
                        out.push(Command::Deploy { task: PillboyTask::Lift { pillbox } });
                    }
                    return Some(out);
                }
            }
            _ => {
                self.finish(obs, &mut out, true);
                return Some(out);
            }
        };
        let carrying = me.pillboys.carrying.is_some() && what == BuildKind::Pillbox;
        if self.fell_first {
            self.doing = format!("crew: felling the site at ({:.0},{:.0})", at.x, at.y);
            if me.pos.dist(at) > reach {
                self.walk(obs, me, at, &mut out);
            } else {
                self.fell_first = false;
                out.push(Command::Deploy { task: PillboyTask::Harvest { at } });
            }
            return Some(out);
        }
        if !carrying && me.pillboys.material < what.cost() {
            let forest = obs.forest.iter().min_by(|a, b| a.dist(at).partial_cmp(&b.dist(at)).unwrap()).copied();
            match forest {
                Some(f) if me.pillboys.material + HARVEST_MATERIAL <= PILLBOY_MATERIAL_MAX => {
                    self.doing = format!("crew: felling for {} material", what.cost());
                    if me.pos.dist(f) > crate::PILLBOY_REACH * 0.8 {
                        self.walk(obs, me, f, &mut out);
                    } else if obs.tick >= self.sent.unwrap_or(0) + TICK_HZ as Tick {
                        self.sent = Some(obs.tick);
                        out.push(Command::Deploy { task: PillboyTask::Harvest { at: f } });
                    }
                }
                _ => self.finish(obs, &mut out, true),
            }
            return Some(out);
        }
        self.doing = format!("crew: building {:?} at ({:.0},{:.0})", what, at.x, at.y);
        if me.pos.dist(at) > reach {
            self.walk(obs, me, at, &mut out);
        } else if self.sent.map(|t| obs.tick >= t + 3 * TICK_HZ as Tick).unwrap_or(true) {
            self.sent = Some(obs.tick);
            out.push(Command::Deploy { task: PillboyTask::Build { what, at } });
        }
        Some(out)
    }
}
