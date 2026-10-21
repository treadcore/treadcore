//! The team board (roadmap step 12e, LAB-REPORT-BRAIN.md sections 5 and
//! 9). One per team, shared by its Commanders (`Shared`), so the team
//! thinks on one picture of the map instead of each tank working the
//! same things out alone from its own copy (finding L5):
//!
//!   - the influence map: 16 x 16 sectors of 8 x 8 tiles, ours minus
//!     theirs, each tank by its worth in a fight (armour, shells) and
//!     each pillbox by `pillbox_worth`, falling off with distance; the
//!     front is where it changes sign, territory where it is ours;
//!   - the held ground: the same of the pillboxes and depots alone,
//!     ours minus theirs; a hostile tank on it beyond the base's alarm
//!     is an incursion, which Defend reads (`w_defend_ground`, LR 68);
//!   - the defence points: the side's pillboxes and depots nearest the
//!     threat, two to four;
//!   - the members: where each is, how ready (armour times shells), what
//!     it is doing, for the coordination screen (11c's allies' goals);
//!   - the jobs the fleet commander gave out, kept, so a crew that missed
//!     the one tick of its `Requested` still finds its job;
//!   - the posture: what the team should mostly be doing;
//!   - rotation across the team: a hurt tank at the hottest point is
//!     swapped for a fresh one at a cold point, confirmed over two
//!     rebuilds (`TEAM_ROTATE`);
//!   - the part-laid bridges (`BRIDGE_JOBS`, LR 6.2): each bridge a
//!     member lays is posted with its water tiles still to lay; left
//!     part-laid (the layer gave up, went home, died), it stays a job
//!     for the next teammate with material, until laid or untouched for
//!     `bridge_keep`.
//!
//! Rebuilt once a second by whichever member finds it due first; bots
//! run in seat order every tick, so that is the same member in every
//! replay. A Mutex because a Brain is Send + Sync; nobody waits on it.

use crate::weights::{Settings, W};
use std::sync::{Arc, Mutex};
use treadcore_sim::*;

pub type Shared = Arc<Mutex<TeamBoard>>;

/// A new, empty board for a team.
pub fn shared() -> Shared {
    Arc::new(Mutex::new(TeamBoard::default()))
}

/// Tiles a side of a sector.
pub const SECTOR: usize = 8;
/// Sectors a side of the map.
pub const SECTORS: usize = MAP_W / SECTOR;
/// The board is rebuilt this often.
pub const REBUILD: Tick = TICK_HZ as Tick;
/// Held ground (LR 68): a sector whose works' influence, ours minus
/// theirs, is at least one work's at this many tiles: within a
/// pillbox's reach and six more of what the side has built or taken.
pub const GROUND_R: f32 = PILLBOX_RANGE + 6.0;
/// A unit's influence falls to half at this many tiles.
pub const REACH: f32 = 8.0;
/// A member this near a defence point is at it.
pub const AT_POINT: f32 = 8.0;
/// A swap order lasts this long.
pub const ORDER_FOR: Tick = 30 * TICK_HZ as Tick;
/// A job is kept this long (crew.rs `JOB_FOR`).
pub const JOB_KEEP: Tick = crate::crew::JOB_FOR;
/// A bridge not posted by its layer in this long is free to take.
pub const BRIDGE_FREE: Tick = 3 * TICK_HZ as Tick;

/// A part-laid bridge on the board.
#[derive(Clone, Debug, PartialEq)]
pub struct BridgeJob {
    /// What it crosses to.
    pub target: V2,
    /// Its water tiles still to lay, in order from the laid end.
    pub tiles: Vec<V2>,
    /// Who lays it, and when they last said so.
    pub by: PlayerId,
    pub seen: Tick,
    /// When a tile of it was last laid (or it was posted).
    pub laid: Tick,
}

/// What the team should mostly be doing (LR 9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Posture {
    #[default]
    Economy,
    Fortify,
    Connect,
    Expand,
    Defend,
}

impl Posture {
    /// In the threshold ladder's order (12e), which is also the order
    /// ties between scores go by.
    pub const ALL: [Posture; 5] = [Posture::Defend, Posture::Economy, Posture::Expand, Posture::Connect, Posture::Fortify];
    /// The posture's name, for the screen and the census.
    pub fn word(self) -> &'static str {
        match self {
            Posture::Economy => "economy",
            Posture::Fortify => "fortify",
            Posture::Connect => "connect",
            Posture::Expand => "expand",
            Posture::Defend => "defend",
        }
    }
}

/// What the member rebuilding the board knows of the base it shares
/// with its team: the posture's inputs that are not on the map (LR 25).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Home {
    /// The castle plan is short of material.
    pub starved: bool,
    /// `Plan::castle_left`: 1 when there is a base and no castle yet.
    pub castle_left: f32,
    /// `Plan::slots_left`.
    pub slots_left: f32,
}

/// The five postures' scores, each 0..1 (LR 25).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PostureScores {
    pub defend: f32,
    pub economy: f32,
    pub fortify: f32,
    pub connect: f32,
    pub expand: f32,
}

impl PostureScores {
    /// The score of one posture.
    pub fn of(&self, p: Posture) -> f32 {
        match p {
            Posture::Defend => self.defend,
            Posture::Economy => self.economy,
            Posture::Fortify => self.fortify,
            Posture::Connect => self.connect,
            Posture::Expand => self.expand,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub player: PlayerId,
    pub pos: V2,
    /// Armour times shells, each as a share of full: 0..1.
    pub ready: f32,
    pub alive: bool,
    /// The squad's material, as a share of full: 0..1.
    pub material: f32,
    /// What it says it is doing.
    pub doing: String,
    pub tick: Tick,
}

/// What the board tells one member.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoardOrder {
    /// Go home and fill up: someone fresh is taking your point.
    FallBack,
    /// Hold this point: you are relieving a hurt teammate.
    Relieve(V2),
}

#[derive(Clone, Debug, Default)]
pub struct TeamBoard {
    pub built: Option<Tick>,
    /// Ours minus theirs, per sector, row-major.
    pub influence: Vec<f32>,
    /// Hostile strength alone, per sector: the threat.
    pub threat: Vec<f32>,
    /// Hostile tanks as last seen: id, where, worth, when (`front_memory`,
    /// LR 58); and the influence with those out of sight still counted
    /// where they were, which the front is read from.
    pub seen: Vec<(EntityId, V2, f32, Tick)>,
    pub known: Vec<f32>,
    /// The held ground (LR 68): the influence of the pillboxes and
    /// depots alone, ours minus theirs, per sector; and the hostile
    /// worth on it beyond the base's alarm at the last rebuild.
    pub ground: Vec<f32>,
    pub incursion: f32,
    /// Where: the incursion's hostile nearest the base.
    pub incursion_at: Option<V2>,
    pub members: Vec<Member>,
    pub points: Vec<V2>,
    pub posture: Posture,
    /// The scores the posture was picked by, and the threshold ladder's
    /// own pick, for the screen, the census and the tests.
    pub scores: PostureScores,
    pub ladder: Posture,
    /// Jobs given out: to whom, what, when.
    pub jobs: Vec<(PlayerId, Job, Tick)>,
    /// Part-laid bridges (`BRIDGE_JOBS`).
    pub bridges: Vec<BridgeJob>,
    /// A swap waiting for its confirmation: who falls back, who
    /// relieves, at which point, and the rebuilds it has held.
    pending: Option<(PlayerId, PlayerId, V2, u8)>,
    orders: Vec<(PlayerId, BoardOrder, Tick)>,
}

/// A tank's worth in a fight, for the influence: its armour share, at
/// least a tenth; none dead or out of shells.
fn worth(t: &Tank) -> f32 {
    if !t.alive || t.shells == 0 {
        0.0
    } else {
        (t.armor / TANK_ARMOR).clamp(0.1, 1.0)
    }
}

/// A tank's readiness: armour times shells, as shares of full.
pub fn readiness(t: &Tank) -> f32 {
    if !t.alive {
        0.0
    } else {
        (t.armor / TANK_ARMOR).clamp(0.0, 1.0) * (t.shells as f32 / TANK_SHELLS as f32).clamp(0.0, 1.0)
    }
}

/// The sector a point is in, row-major; off the map, the nearest.
pub fn sector_of(p: V2) -> usize {
    let x = (p.x.max(0.0) as usize / SECTOR).min(SECTORS - 1);
    let y = (p.y.max(0.0) as usize / SECTOR).min(SECTORS - 1);
    y * SECTORS + x
}

/// The middle of a sector.
pub fn sector_centre(s: usize) -> V2 {
    V2::new(((s % SECTORS) * SECTOR) as f32 + SECTOR as f32 / 2.0, ((s / SECTORS) * SECTOR) as f32 + SECTOR as f32 / 2.0)
}

impl TeamBoard {
    /// Every think of a member: where it is and what it is doing, and the
    /// jobs it heard given out.
    pub fn post(&mut self, obs: &Observation, me: &Tank, doing: &str) {
        let m = Member { player: obs.player, pos: me.pos, ready: readiness(me), alive: me.alive, material: me.pillboys.material as f32 / PILLBOY_MATERIAL_MAX as f32, doing: doing.to_string(), tick: obs.tick };
        match self.members.iter_mut().find(|x| x.player == obs.player) {
            Some(x) => *x = m,
            None => self.members.push(m),
        }
        for e in &obs.events {
            if let Event::Requested { to: Some(to), job, .. } = e {
                if !obs.allied(*to) {
                    continue;
                }
                match job {
                    Job::Build { .. } | Job::Fetch { .. } | Job::Haul { .. } => {
                        self.jobs.retain(|(p, _, _)| p != to);
                        self.jobs.push((*to, job.clone(), obs.tick));
                    }
                    Job::Release | Job::Ready => self.jobs.retain(|(p, _, _)| p != to),
                    _ => {}
                }
            }
        }
        self.jobs.retain(|(_, _, t)| obs.tick < t + JOB_KEEP);
    }

    /// A member laying a bridge says so every think: the same bridge
    /// (a tile or the target in common) is updated, a new one posted.
    pub fn post_bridge(&mut self, by: PlayerId, tiles: &[V2], target: V2, now: Tick) {
        let same = |j: &BridgeJob| j.target.tile() == target.tile() || j.tiles.iter().any(|a| tiles.iter().any(|b| a.tile() == b.tile()));
        match self.bridges.iter_mut().find(|j| same(j)) {
            Some(j) => {
                if tiles.len() < j.tiles.len() {
                    j.laid = now;
                }
                j.tiles = tiles.to_vec();
                j.target = target;
                j.by = by;
                j.seen = now;
            }
            None => self.bridges.push(BridgeJob { target, tiles: tiles.to_vec(), by, seen: now, laid: now }),
        }
    }

    /// The tiles laid since, by anyone, off the jobs; a bridge laid, or
    /// untouched for `keep`, off the board.
    pub fn prune_bridges(&mut self, map: &Map, now: Tick, keep: Tick) {
        for j in self.bridges.iter_mut() {
            let before = j.tiles.len();
            j.tiles.retain(|t| map.at(*t) == Terrain::Water);
            if j.tiles.len() < before {
                j.laid = now;
            }
        }
        self.bridges.retain(|j| !j.tiles.is_empty() && now < j.laid.saturating_add(keep));
    }

    /// A part-laid bridge nobody is laying, its next tile within
    /// `reach` of `at`, that `ok` takes: the nearest, now `p`'s.
    pub fn take_bridge(&mut self, p: PlayerId, at: V2, reach: f32, now: Tick, ok: impl Fn(V2) -> bool) -> Option<(Vec<V2>, V2)> {
        let j = self
            .bridges
            .iter_mut()
            .filter(|j| now >= j.seen + BRIDGE_FREE && j.tiles[0].dist(at) <= reach && ok(j.target))
            .min_by(|a, b| a.tiles[0].dist(at).total_cmp(&b.tiles[0].dist(at)))?;
        j.by = p;
        j.seen = now;
        Some((j.tiles.clone(), j.target))
    }

    /// A rebuild is due: never built, or `REBUILD` since the last.
    pub fn due(&self, now: Tick) -> bool {
        self.built.map(|t| now >= t + REBUILD).unwrap_or(true)
    }

    /// The job given to `p` lately, if any.
    pub fn job_for(&self, p: PlayerId) -> Option<(Job, Tick)> {
        self.jobs.iter().find(|(q, _, _)| *q == p).map(|(_, j, t)| (j.clone(), *t))
    }

    /// What the board orders `p` now, if anything.
    pub fn order_for(&self, p: PlayerId, now: Tick) -> Option<BoardOrder> {
        self.orders.iter().find(|(q, _, t)| *q == p && now < t + ORDER_FOR).map(|(_, o, _)| *o)
    }

    /// The order is carried out (home and full, or relieved): drop it.
    pub fn done(&mut self, p: PlayerId) {
        self.orders.retain(|(q, _, _)| *q != p);
    }

    /// Ours minus theirs at a point's sector, as last seen.
    pub fn influence_at(&self, p: V2) -> f32 {
        self.influence.get(sector_of(p)).copied().unwrap_or(0.0)
    }

    /// Hostile strength at a point's sector, as last seen.
    pub fn threat_at(&self, p: V2) -> f32 {
        self.threat.get(sector_of(p)).copied().unwrap_or(0.0)
    }

    /// The front: our sectors with a neighbour that is theirs, on the
    /// influence with the hostiles remembered (`known`), else as seen.
    pub fn front(&self) -> Vec<usize> {
        let inf = if self.known.len() == self.influence.len() { &self.known } else { &self.influence };
        let mut out = Vec::new();
        for s in 0..inf.len() {
            if inf[s] <= 0.0 {
                continue;
            }
            let (x, y) = (s % SECTORS, s / SECTORS);
            let theirs = |nx: usize, ny: usize| nx < SECTORS && ny < SECTORS && inf[ny * SECTORS + nx] < 0.0;
            if theirs(x.wrapping_sub(1), y) || theirs(x + 1, y) || theirs(x, y.wrapping_sub(1)) || theirs(x, y + 1) {
                out.push(s);
            }
        }
        out
    }

    /// Tiles from `p` to the nearest front sector's centre (LR 58); none
    /// with no front (no hostile seen at the last rebuild).
    pub fn front_dist(&self, p: V2) -> Option<f32> {
        self.front().into_iter().map(|s| sector_centre(s).dist(p)).min_by(|a, b| a.total_cmp(b))
    }

    /// The share of the map that is ours.
    pub fn territory(&self) -> f32 {
        if self.influence.is_empty() {
            return 0.0;
        }
        self.influence.iter().filter(|v| **v > 0.0).count() as f32 / self.influence.len() as f32
    }

    /// Once a second: the maps, the points, the posture, the swaps.
    pub fn rebuild(&mut self, obs: &Observation, base: Option<V2>, s: &Settings, home: Home, now: Tick) {
        self.built = Some(now);
        let n = SECTORS * SECTORS;
        self.influence.clear();
        self.influence.resize(n, 0.0);
        self.threat.clear();
        self.threat.resize(n, 0.0);
        let pill = s.w[W::PillboxWorth];
        let add = |at: V2, v: f32, hostile: bool, inf: &mut Vec<f32>, thr: &mut Vec<f32>| {
            for (k, cell) in inf.iter_mut().enumerate() {
                let d = sector_centre(k).dist(at);
                let f = v / (1.0 + d / REACH);
                *cell += if hostile { -f } else { f };
                if hostile {
                    thr[k] += f;
                }
            }
        };
        for t in obs.tanks.iter().filter(|t| t.alive) {
            let hostile = !obs.allied(t.owner);
            add(t.pos, worth(t), hostile, &mut self.influence, &mut self.threat);
        }
        // The held ground: what the side's works hold, without the
        // tanks that come and go (LR 68).
        self.ground.clear();
        self.ground.resize(n, 0.0);
        let mut none = vec![0.0; n];
        for p in obs.pillboxes.iter().filter(|p| p.alive()) {
            let Some(o) = p.owner else { continue };
            add(p.pos, pill, !obs.allied(o), &mut self.influence, &mut self.threat);
            add(p.pos, pill, !obs.allied(o), &mut self.ground, &mut none);
        }
        for d in obs.depots.iter() {
            let Some(o) = d.owner else { continue };
            add(d.pos, pill, !obs.allied(o), &mut self.ground, &mut none);
        }
        // The front's memory (LR 58): each hostile tank in sight is kept
        // where it is; one seen dead, or not seen for `front_memory`
        // seconds, is dropped; the rest still count where they were.
        let keep = s.ticks(W::FrontMemory);
        for t in obs.tanks.iter().filter(|t| !obs.allied(t.owner)) {
            self.seen.retain(|x| x.0 != t.id);
            if t.alive && keep > 0 {
                self.seen.push((t.id, t.pos, worth(t), now));
            }
        }
        self.seen.retain(|x| now < x.3 + keep);
        self.known.clone_from(&self.influence);
        let mut none = vec![0.0; n];
        for x in self.seen.iter().filter(|x| x.3 < now) {
            add(x.1, x.2, true, &mut self.known, &mut none);
        }
        // Defence points: the side's pillboxes and depots under the most
        // threat, up to four; none, the base.
        let mut cands: Vec<(V2, f32)> = obs
            .pillboxes
            .iter()
            .filter(|p| p.alive() && p.owner.map(|o| obs.allied(o)).unwrap_or(false))
            .map(|p| p.pos)
            .chain(obs.depots.iter().filter(|d| d.owner.map(|o| obs.allied(o)).unwrap_or(false)).map(|d| d.pos))
            .map(|p| (p, self.threat_at(p)))
            .collect();
        cands.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.x.total_cmp(&b.0.x)).then(a.0.y.total_cmp(&b.0.y)));
        self.points = cands.iter().take(4).map(|(p, _)| *p).collect();
        if self.points.is_empty() {
            self.points.extend(base);
        }
        // The posture. Defend is a threat at the base itself: hostile
        // worth within the base's alarm, a full tank 1. The sector
        // threat has a long tail (every hostile tank on the map adds to
        // it), and a spread in readiness is rotation's business, not a
        // reason for the team to go home (LR 24).
        let base_threat: f32 = base
            .map(|b| obs.tanks.iter().filter(|t| t.alive && !obs.allied(t.owner) && t.pos.dist(b) <= crate::commander::BASE_ALARM).map(worth).sum())
            .unwrap_or(0.0);
        // An incursion (LR 68): hostile tanks on the held ground beyond
        // the base's alarm, counted toward Defend at `w_defend_ground`.
        let ground = &self.ground;
        let held = pill / (1.0 + GROUND_R / REACH);
        let raiders: Vec<&Tank> = obs
            .tanks
            .iter()
            .filter(|t| t.alive && !obs.allied(t.owner) && ground.get(sector_of(t.pos)).copied().unwrap_or(0.0) >= held && base.map(|b| t.pos.dist(b) > crate::commander::BASE_ALARM).unwrap_or(true))
            .collect();
        self.incursion = raiders.iter().map(|t| worth(t)).sum();
        self.incursion_at = raiders.iter().map(|t| t.pos).min_by(|a, b| base.map(|b0| a.dist(b0).total_cmp(&b.dist(b0))).unwrap_or(std::cmp::Ordering::Equal));
        let base_only = base_threat;
        let base_threat = base_threat + s.w[W::DefendGround].max(0.0) * self.incursion;
        crate::meter::ground(self.incursion > 0.0, base_only <= 0.5 && base_threat > 0.5);
        let ours = self.influence.iter().filter(|v| **v > 0.0).count();
        // The threshold ladder of 12e (Defend fixed in LR 24).
        let ladder = if base_threat > 0.5 {
            Posture::Defend
        } else if home.starved {
            Posture::Economy
        } else if ours * 4 < n {
            Posture::Expand
        } else if self.points.len() < 3 {
            Posture::Connect
        } else {
            Posture::Fortify
        };
        // The scores (LR 9's table, LR 25), each 0..1.
        let alive: Vec<&Member> = self.members.iter().filter(|m| m.alive).collect();
        let material = if alive.is_empty() { 1.0 } else { alive.iter().map(|m| m.material).sum::<f32>() / alive.len() as f32 };
        let mine: f32 = obs.tanks.iter().filter(|t| t.alive && obs.allied(t.owner)).map(worth).sum();
        let theirs: f32 = obs.tanks.iter().filter(|t| t.alive && !obs.allied(t.owner)).map(worth).sum();
        let press = s.w[W::PressOdds].max(1.01);
        // No hostile seen is no front: even odds, nothing to press.
        let odds = if theirs > 0.0 { mine / theirs } else { 1.0 };
        self.scores = PostureScores {
            defend: (base_threat / 0.5).clamp(0.0, 1.0),
            economy: if home.starved { 1.0 } else { (1.0 - material).clamp(0.0, 1.0) },
            fortify: home.castle_left.clamp(0.0, 1.0),
            connect: home.slots_left.clamp(0.0, 1.0),
            expand: ((odds - 1.0) / (press - 1.0)).clamp(0.0, 1.0),
        };
        // Each posture's worth: the blend of the ladder's pick (1 for it,
        // 0 for the rest) and its weighted score; the best, ties in the
        // ladder's order; nothing pressing, Fortify as the ladder had it.
        let k = s.w[W::PostureScored].clamp(0.0, 1.0);
        let pw = |p: Posture| match p {
            Posture::Defend => s.w[W::PostureDefend],
            Posture::Economy => s.w[W::PostureEconomy],
            Posture::Fortify => s.w[W::PostureFortify],
            Posture::Connect => s.w[W::PostureConnect],
            Posture::Expand => s.w[W::PostureExpand],
        };
        self.ladder = ladder;
        let value = |p: Posture| (1.0 - k) * (p == ladder) as u8 as f32 + k * pw(p).max(0.0) * self.scores.of(p);
        self.posture = Posture::ALL.iter().copied().fold(None, |best: Option<Posture>, p| match best {
            Some(b) if value(b) >= value(p) => Some(b),
            _ => Some(p),
        }).filter(|p| value(*p) > 0.0).unwrap_or(Posture::Fortify);
        self.members.retain(|m| now < m.tick + 10 * TICK_HZ as Tick);
        self.orders.retain(|(_, _, t)| now < t + ORDER_FOR);
        if s.caps.has(crate::weights::Caps::TEAM_ROTATE) {
            self.rotate(s, now);
        }
    }

    /// Rotation across the team (LR 9): the hottest point's least ready
    /// member against the most ready member elsewhere.
    fn rotate(&mut self, s: &Settings, now: Tick) {
        if !self.orders.is_empty() {
            return;
        }
        let at = |m: &Member| self.points.iter().copied().filter(|p| p.dist(m.pos) <= AT_POINT).min_by(|a, b| a.dist(m.pos).total_cmp(&b.dist(m.pos)));
        let hot = self.points.iter().copied().filter(|p| self.members.iter().any(|m| m.alive && m.pos.dist(*p) <= AT_POINT)).max_by(|a, b| self.threat_at(*a).total_cmp(&self.threat_at(*b)));
        let Some(hot) = hot else {
            self.pending = None;
            return;
        };
        let tired = self.members.iter().filter(|m| m.alive && at(m) == Some(hot)).min_by(|a, b| a.ready.total_cmp(&b.ready).then(a.player.cmp(&b.player)));
        let fresh = self.members.iter().filter(|m| m.alive && at(m) != Some(hot)).max_by(|a, b| a.ready.total_cmp(&b.ready).then(b.player.cmp(&a.player)));
        let (Some(tired), Some(fresh)) = (tired, fresh) else {
            self.pending = None;
            return;
        };
        if fresh.ready - tired.ready <= s.w[W::SwapGap] {
            self.pending = None;
            return;
        }
        let k = match self.pending {
            Some((a, b, p, k)) if a == tired.player && b == fresh.player && p == hot => k + 1,
            _ => 1,
        };
        if k >= 2 {
            self.orders.push((tired.player, BoardOrder::FallBack, now));
            self.orders.push((fresh.player, BoardOrder::Relieve(hot), now));
            self.pending = None;
        } else {
            self.pending = Some((tired.player, fresh.player, hot, k));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Four seats, two teams (0 and 2, 1 and 3), placed; every tank seen.
    fn placed(at: [V2; 4]) -> Observation {
        let mut sc = Scenario::bots(4).with_teams(2);
        sc.advantage = true;
        let mut w = World::new(sc);
        for (p, a) in at.iter().enumerate() {
            let t = w.tanks.iter_mut().find(|t| t.owner == p as PlayerId).unwrap();
            t.pos = *a;
        }
        w.observe(0)
    }

    /// Defend is a threat at the base, not the map's hostile tanks
    /// summed (LR 24): eight of them 80 tiles off read 0.73 on the
    /// sector threat, over its 0.5, and sent the team home in 94% of a
    /// match's picks. Far, the team is not defending; one at the gate,
    /// it is.
    #[test]
    fn the_team_defends_a_threat_at_the_base_not_far_off() {
        let mut sc = Scenario::bots(16).with_teams(2);
        sc.advantage = true;
        let mut w = World::new(sc);
        let base = V2::new(20.5, 64.5);
        for t in w.tanks.iter_mut() {
            let k = t.owner as f32;
            t.pos = if t.owner % 2 == 0 { V2::new(20.5 + k / 4.0, 60.5) } else { V2::new(100.5, 40.5 + 2.0 * k) };
        }
        let mut b = TeamBoard::default();
        b.rebuild(&w.observe(0), Some(base), &Settings::default(), Home::default(), 0);
        assert!(b.threat_at(base) > 0.5, "the sector threat has its long tail: {}", b.threat_at(base));
        assert_ne!(b.posture, Posture::Defend, "eight hostile tanks 80 tiles off");
        let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
        t.pos = base + V2::new(6.0, 0.0);
        b.rebuild(&w.observe(0), Some(base), &Settings::default(), Home::default(), REBUILD);
        assert_eq!(b.posture, Posture::Defend, "one at the gate");
    }

    /// Each posture wins on its own case (LR 25), and `posture_scored`
    /// 0 is the threshold ladder in every case.
    #[test]
    fn each_posture_scores_its_own_case() {
        let mut sc = Scenario::bots(16).with_teams(2);
        sc.advantage = true;
        let mut w = World::new(sc);
        let base = V2::new(20.5, 64.5);
        // Ours by the base, theirs 80 tiles off; two of theirs moved in
        // sight-range of nobody but counted (advantage) for the odds case.
        for t in w.tanks.iter_mut() {
            let k = t.owner as f32;
            t.pos = if t.owner % 2 == 0 { V2::new(20.5 + k / 4.0, 60.5) } else { V2::new(100.5, 40.5 + 2.0 * k) };
        }
        let full = |b: &mut TeamBoard, share: f32| {
            b.members = (0..16u8).filter(|p| p % 2 == 0).map(|p| Member { player: p, pos: base, ready: 1.0, alive: true, material: share, doing: String::new(), tick: 0 }).collect();
        };
        let run = |obs: &Observation, home: Home, share: f32, scored: f32| {
            let mut s = Settings::default();
            s.w[W::PostureScored] = scored;
            let mut b = TeamBoard::default();
            full(&mut b, share);
            b.rebuild(obs, Some(base), &s, home, 0);
            (b.posture, b.ladder, b.scores)
        };
        let obs = w.observe(0);
        let planned = Home { starved: false, castle_left: 1.0, slots_left: 0.0 };
        let standing = Home { starved: false, castle_left: 0.0, slots_left: 0.0 };
        let cases: Vec<(&str, Observation, Home, f32, Posture)> = vec![
            ("a castle just planned, even odds", obs.clone(), planned, 1.0, Posture::Fortify),
            ("the castle up, its slots half empty", obs.clone(), Home { slots_left: 0.5, ..standing }, 1.0, Posture::Connect),
            ("the castle up, the squads spent", obs.clone(), standing, 0.2, Posture::Economy),
            ("a starved plan", obs.clone(), Home { starved: true, ..planned }, 1.0, Posture::Economy),
            ("eight of ours against two of theirs in sight", {
                let mut o = obs.clone();
                let mut n = 0;
                o.tanks.retain(|t| obs.allied(t.owner) || { n += 1; n <= 2 });
                o
            }, standing, 1.0, Posture::Expand),
        ];
        for (what, o, home, share, want) in &cases {
            let (got, ladder, scores) = run(o, *home, *share, 1.0);
            eprintln!("each_posture_scores_its_own_case: {}: {:?} (the ladder: {:?}) {:?}", what, got, ladder, scores);
            assert_eq!(got, *want, "{}: {:?}", what, scores);
            assert_eq!(run(o, *home, *share, 0.0).0, ladder, "{}: posture_scored 0 is the ladder", what);
        }
        // A hostile tank at the gate: Defend over a castle just planned.
        let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
        t.pos = base + V2::new(6.0, 0.0);
        assert_eq!(run(&w.observe(0), planned, 1.0, 1.0).0, Posture::Defend);
    }

    #[test]
    fn a_hostile_on_the_held_ground_is_an_incursion() {
        // LR 68: seat 0's team (0 and 2) holds a pillbox at (90, 64), 26
        // tiles from its base at (64, 64). A hostile at 9 tiles from the
        // pillbox is on the held ground; one at the base is the base's
        // own threat; one far from every work is on nobody's ground.
        let base = V2::new(64.0, 64.0);
        let mut sc = Scenario::bots(4).with_teams(2);
        sc.advantage = true;
        let at = |hostile: V2| {
            let mut w = World::new(sc.clone());
            w.pillboxes.clear();
            for d in w.depots.iter_mut() {
                d.owner = None;
            }
            w.pillboxes.push(Pillbox { id: 9_001, pos: V2::new(90.0, 64.0), owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: 0 });
            for (p, pos) in [(0, base), (2, base + V2::new(0.0, 3.0)), (1, hostile), (3, V2::new(10.0, 120.0))] {
                w.tanks.iter_mut().find(|t| t.owner == p).unwrap().pos = pos;
            }
            let mut b = TeamBoard::default();
            b.rebuild(&w.observe(0), Some(base), &Settings::default(), Home::default(), 0);
            (b.incursion, b.incursion_at)
        };
        let (n, where_) = at(V2::new(99.0, 64.0));
        assert!(n > 0.5 && where_ == Some(V2::new(99.0, 64.0)), "{} {:?}", n, where_);
        assert_eq!(at(V2::new(70.0, 64.0)).0, 0.0, "inside the base's alarm: the base's threat, not an incursion");
        assert_eq!(at(V2::new(64.0, 10.0)).0, 0.0, "far from every work: nobody's ground");
    }

    #[test]
    fn the_front_is_where_influence_crosses_zero() {
        let obs = placed([V2::new(20.0, 64.0), V2::new(100.0, 64.0), V2::new(24.0, 64.0), V2::new(104.0, 64.0)]);
        let mut b = TeamBoard::default();
        b.rebuild(&obs, None, &Settings::default(), Home::default(), 0);
        assert!(b.influence_at(V2::new(20.0, 64.0)) > 0.0);
        assert!(b.influence_at(V2::new(104.0, 64.0)) < 0.0);
        let front = b.front();
        assert!(!front.is_empty());
        for s in &front {
            let x = sector_centre(*s).x;
            assert!((40.0..=72.0).contains(&x), "the front sits between the two sides, not at x {}", x);
        }
        assert!(b.territory() > 0.3 && b.territory() < 0.7);
    }

    /// The front's memory (LR 58): the hostiles seen at one rebuild and
    /// out of sight at the next still make the front where they were,
    /// for `front_memory` seconds; at 0 the front goes with the sighting.
    #[test]
    fn a_front_seen_is_remembered() {
        let seen = placed([V2::new(20.0, 64.0), V2::new(100.0, 64.0), V2::new(24.0, 64.0), V2::new(104.0, 64.0)]);
        let mut lost = seen.clone();
        lost.tanks.retain(|t| seen.allied(t.owner));
        for keep in [30.0, 0.0] {
            let mut s = Settings::default();
            s.w[W::FrontMemory] = keep;
            let mut b = TeamBoard::default();
            b.rebuild(&seen, None, &s, Home::default(), 0);
            let front = b.front();
            assert!(!front.is_empty());
            b.rebuild(&lost, None, &s, Home::default(), REBUILD);
            assert!(b.influence.iter().all(|v| *v >= 0.0), "out of sight, the hostiles are not in the influence");
            if keep > 0.0 {
                assert_eq!(b.front(), front, "remembered, the front stays where it was");
                b.rebuild(&lost, None, &s, Home::default(), 31 * TICK_HZ as Tick);
                assert!(b.front().is_empty(), "forgotten after front_memory");
            } else {
                assert!(b.front().is_empty(), "at 0 the front goes with the sighting");
            }
        }
    }

    #[test]
    fn a_hurt_tank_swaps_out() {
        let mut s = Settings::default();
        s.caps = s.caps.with(crate::weights::Caps::TEAM_ROTATE);
        let point = V2::new(60.0, 60.0);
        let mut b = TeamBoard { points: vec![point], ..TeamBoard::default() };
        b.members = vec![
            Member { player: 0, pos: V2::new(61.0, 60.0), ready: 0.2, alive: true, material: 1.0, doing: String::new(), tick: 0 },
            Member { player: 1, pos: V2::new(20.0, 20.0), ready: 0.95, alive: true, material: 1.0, doing: String::new(), tick: 0 },
        ];
        b.threat = vec![0.0; SECTORS * SECTORS];
        b.threat[sector_of(point)] = 1.0;
        b.rotate(&s, 20);
        assert_eq!(b.order_for(0, 20), None, "one rebuild is not yet a swap");
        b.rotate(&s, 40);
        assert_eq!(b.order_for(0, 40), Some(BoardOrder::FallBack));
        assert_eq!(b.order_for(1, 40), Some(BoardOrder::Relieve(point)));
        assert_eq!(b.order_for(1, 40 + ORDER_FOR), None, "an order runs out");
    }

    #[test]
    fn a_part_laid_bridge_waits_for_the_next_tank() {
        let mut map = World::new(Scenario::bots(2)).map.clone();
        let tiles: Vec<V2> = (0..3).map(|i| V2::new(40.5 + i as f32, 40.5)).collect();
        for t in &tiles {
            map.set(*t, Terrain::Water);
        }
        let target = V2::new(50.5, 40.5);
        let mut b = TeamBoard::default();
        b.post_bridge(1, &tiles, target, 0);
        let yes = |_: V2| true;
        assert_eq!(b.take_bridge(2, V2::new(35.0, 40.0), 40.0, 10, yes), None, "its layer still lays it");
        assert_eq!(b.take_bridge(3, V2::new(120.0, 120.0), 40.0, BRIDGE_FREE, yes), None, "too far to take");
        assert_eq!(b.take_bridge(2, V2::new(35.0, 40.0), 40.0, BRIDGE_FREE, |_| false), None, "given up by the taker");
        assert_eq!(b.take_bridge(2, V2::new(35.0, 40.0), 40.0, BRIDGE_FREE, yes), Some((tiles.clone(), target)));
        assert_eq!(b.take_bridge(3, V2::new(35.0, 40.0), 40.0, BRIDGE_FREE + 1, yes), None, "taken");
        map.set(tiles[0], Terrain::Bridge);
        b.prune_bridges(&map, 100, 180 * TICK_HZ as Tick);
        assert_eq!(b.bridges[0].tiles.len(), 2, "a tile laid comes off");
        b.prune_bridges(&map, 100 + 180 * TICK_HZ as Tick, 180 * TICK_HZ as Tick);
        assert!(b.bridges.is_empty(), "untouched for bridge_keep, dropped");
        b.post_bridge(1, &tiles[1..], target, 200);
        for t in &tiles {
            map.set(*t, Terrain::Bridge);
        }
        b.prune_bridges(&map, 201, 180 * TICK_HZ as Tick);
        assert!(b.bridges.is_empty(), "laid, done");
    }

    #[test]
    fn a_missed_job_is_still_on_the_board() {
        let mut obs = placed([V2::new(20.0, 20.0), V2::new(100.0, 64.0), V2::new(24.0, 20.0), V2::new(104.0, 64.0)]);
        let me = obs.me.clone().unwrap();
        obs.events.push(Event::Requested { from: 0, to: Some(2), job: Job::Build { what: BuildKind::Wall, at: V2::new(10.5, 10.5) } });
        let mut b = TeamBoard::default();
        b.post(&obs, &me, "building");
        assert!(matches!(b.job_for(2), Some((Job::Build { .. }, _))));
        obs.events = vec![Event::Requested { from: 2, to: Some(2), job: Job::Ready }];
        b.post(&obs, &me, "building");
        assert!(b.job_for(2).is_none(), "Ready ends it");
    }
}
