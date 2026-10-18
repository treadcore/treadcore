//! The team board (roadmap step 12e, LAB-REPORT-BRAIN.md sections 5 and
//! 9). One per team, shared by its Commanders (`Shared`), so the team
//! thinks on one picture of the map instead of each tank working the
//! same things out alone from its own copy (finding L5):
//!
//!   - the influence map: 16 x 16 sectors of 8 x 8 tiles, ours minus
//!     theirs, each tank by its worth in a fight (armour, shells) and
//!     each pillbox by `pillbox_worth`, falling off with distance; the
//!     front is where it changes sign, territory where it is ours;
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

pub fn shared() -> Shared {
    Arc::new(Mutex::new(TeamBoard::default()))
}

/// Tiles a side of a sector.
pub const SECTOR: usize = 8;
/// Sectors a side of the map.
pub const SECTORS: usize = MAP_W / SECTOR;
/// The board is rebuilt this often.
pub const REBUILD: Tick = TICK_HZ as Tick;
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

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub player: PlayerId,
    pub pos: V2,
    /// Armour times shells, each as a share of full: 0..1.
    pub ready: f32,
    pub alive: bool,
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
    pub members: Vec<Member>,
    pub points: Vec<V2>,
    pub posture: Posture,
    /// Jobs given out: to whom, what, when.
    pub jobs: Vec<(PlayerId, Job, Tick)>,
    /// Part-laid bridges (`BRIDGE_JOBS`).
    pub bridges: Vec<BridgeJob>,
    /// A swap waiting for its confirmation: who falls back, who
    /// relieves, at which point, and the rebuilds it has held.
    pending: Option<(PlayerId, PlayerId, V2, u8)>,
    orders: Vec<(PlayerId, BoardOrder, Tick)>,
}

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

pub fn sector_of(p: V2) -> usize {
    let x = (p.x.max(0.0) as usize / SECTOR).min(SECTORS - 1);
    let y = (p.y.max(0.0) as usize / SECTOR).min(SECTORS - 1);
    y * SECTORS + x
}

pub fn sector_centre(s: usize) -> V2 {
    V2::new(((s % SECTORS) * SECTOR) as f32 + SECTOR as f32 / 2.0, ((s / SECTORS) * SECTOR) as f32 + SECTOR as f32 / 2.0)
}

impl TeamBoard {
    /// Every think of a member: where it is and what it is doing, and the
    /// jobs it heard given out.
    pub fn post(&mut self, obs: &Observation, me: &Tank, doing: &str) {
        let m = Member { player: obs.player, pos: me.pos, ready: readiness(me), alive: me.alive, doing: doing.to_string(), tick: obs.tick };
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

    pub fn influence_at(&self, p: V2) -> f32 {
        self.influence.get(sector_of(p)).copied().unwrap_or(0.0)
    }

    pub fn threat_at(&self, p: V2) -> f32 {
        self.threat.get(sector_of(p)).copied().unwrap_or(0.0)
    }

    /// The front: our sectors with a neighbour that is theirs.
    pub fn front(&self) -> Vec<usize> {
        let mut out = Vec::new();
        for s in 0..self.influence.len() {
            if self.influence[s] <= 0.0 {
                continue;
            }
            let (x, y) = (s % SECTORS, s / SECTORS);
            let theirs = |nx: usize, ny: usize| nx < SECTORS && ny < SECTORS && self.influence[ny * SECTORS + nx] < 0.0;
            if theirs(x.wrapping_sub(1), y) || theirs(x + 1, y) || theirs(x, y.wrapping_sub(1)) || theirs(x, y + 1) {
                out.push(s);
            }
        }
        out
    }

    /// The share of the map that is ours.
    pub fn territory(&self) -> f32 {
        if self.influence.is_empty() {
            return 0.0;
        }
        self.influence.iter().filter(|v| **v > 0.0).count() as f32 / self.influence.len() as f32
    }

    /// Once a second: the maps, the points, the posture, the swaps.
    pub fn rebuild(&mut self, obs: &Observation, base: Option<V2>, s: &Settings, starved: bool, now: Tick) {
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
        for p in obs.pillboxes.iter().filter(|p| p.alive()) {
            let Some(o) = p.owner else { continue };
            add(p.pos, pill, !obs.allied(o), &mut self.influence, &mut self.threat);
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
        // The posture.
        let base_threat = base.map(|b| self.threat_at(b)).unwrap_or(0.0);
        let spread = {
            let r: Vec<f32> = self.members.iter().filter(|m| m.alive).map(|m| m.ready).collect();
            let max = r.iter().cloned().fold(0.0, f32::max);
            let min = r.iter().cloned().fold(1.0, f32::min);
            if r.is_empty() {
                0.0
            } else {
                max - min
            }
        };
        let ours = self.influence.iter().filter(|v| **v > 0.0).count();
        self.posture = if base_threat > 0.5 || spread > 0.6 {
            Posture::Defend
        } else if starved {
            Posture::Economy
        } else if ours * 4 < n {
            Posture::Expand
        } else if self.points.len() < 3 {
            Posture::Connect
        } else {
            Posture::Fortify
        };
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

    #[test]
    fn the_front_is_where_influence_crosses_zero() {
        let obs = placed([V2::new(20.0, 64.0), V2::new(100.0, 64.0), V2::new(24.0, 64.0), V2::new(104.0, 64.0)]);
        let mut b = TeamBoard::default();
        b.rebuild(&obs, None, &Settings::default(), false, 0);
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

    #[test]
    fn a_hurt_tank_swaps_out() {
        let mut s = Settings::default();
        s.caps = s.caps.with(crate::weights::Caps::TEAM_ROTATE);
        let point = V2::new(60.0, 60.0);
        let mut b = TeamBoard { points: vec![point], ..TeamBoard::default() };
        b.members = vec![
            Member { player: 0, pos: V2::new(61.0, 60.0), ready: 0.2, alive: true, doing: String::new(), tick: 0 },
            Member { player: 1, pos: V2::new(20.0, 20.0), ready: 0.95, alive: true, doing: String::new(), tick: 0 },
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
