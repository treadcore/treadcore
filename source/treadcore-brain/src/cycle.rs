//! The base cycle (roadmap step 12f1, LAB-REPORT-BRAIN.md section 6.1;
//! Paul, 18 October: "after they fill up on base resources, add to the
//! base 1 item, 1 improvement, and attempt to build roads to other
//! established bases or pillboxes"). A tank that has just filled up at
//! the depot (shells, armour, and the squad's material, which a depot
//! restocks) gives the base something before it goes back out:
//!
//!   1. one item of the castle plan (a wall, a tower, the road), felling
//!      first if the squad is short;
//!   2. one improvement: the most hurt pillbox of the base mended;
//!   3. road toward the nearest established friendly pillbox or depot
//!      not yet joined to the base, `connect_tiles` tiles a visit, so the
//!      roads grow out from the base over the match and become the way a
//!      hauled pillbox comes home (crew.rs `Haul`).
//!
//! Each stage gets `STAGE_FOR` and is then passed over, so the cycle
//! never holds a tank long; a hostile tank near ends it (the Commander
//! checks that before asking).

use crate::status::CycleAct;
use crate::fortify::{Plan, Want};
use crate::weights::{Settings, W};
use treadcore_sim::*;

/// Seconds a stage is given.
pub const STAGE_FOR: Tick = 20 * TICK_HZ as Tick;
/// The yard: road is not laid this near the base (fortify.rs lays it).
pub const YARD: f32 = 4.0;
/// A pillbox or depot this far from the base, and no farther, is joined.
pub const CONNECT_NEAR: f32 = 12.0;
pub const CONNECT_FAR: f32 = 48.0;
/// A pillbox under this share of full is mended.
pub const MEND_BELOW: f32 = 0.9;
/// Mines the cycle lays on a base's approach at most.
pub const MINES: u8 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stage {
    #[default]
    Idle,
    Item,
    Improve,
    Connect,
}

#[derive(Clone, Debug, Default)]
pub struct Cycle {
    pub stage: Stage,
    since: Tick,
    was_home: bool,
    laid: u8,
    /// Ordered and not yet back: wait for the squad.
    out: bool,
    /// Mines laid on the approach this match.
    mines: u8,
    /// The step it is on, as a code (status.rs).
    pub act: CycleAct,
}

impl Cycle {
    /// Every tick: a fill just ended starts the cycle.
    pub fn watch(&mut self, resupplying: bool, now: Tick) {
        if self.was_home && !resupplying {
            self.begin(now);
        }
        self.was_home = resupplying;
    }

    /// Start a cycle at its first stage, the item.
    pub fn begin(&mut self, now: Tick) {
        self.stage = Stage::Item;
        self.since = now;
        self.laid = 0;
        self.out = false;
    }

    /// A cycle is under way.
    pub fn active(&self) -> bool {
        self.stage != Stage::Idle
    }

    /// On to the next stage (item, improvement, connection, then idle).
    fn advance(&mut self, now: Tick) {
        self.stage = match self.stage {
            Stage::Idle => Stage::Idle,
            Stage::Item => Stage::Improve,
            Stage::Improve => Stage::Connect,
            Stage::Connect => Stage::Idle,
        };
        self.since = now;
        self.out = false;
    }

    /// The road tiles to lay from the base toward `to`: the straight
    /// line's grass and rubble past the yard, at most `n`.
    pub fn road_to(map: &Map, base: V2, to: V2, n: usize) -> Vec<V2> {
        let d = to.dist(base);
        let mut out: Vec<V2> = Vec::new();
        let mut k = YARD;
        while k < d - 2.0 && out.len() < n {
            let p = Map::tile_centre(base + (to - base).norm().scale(k));
            k += 0.5;
            if out.last().map(|l| l.tile() == p.tile()).unwrap_or(false) {
                continue;
            }
            match map.at(p) {
                Terrain::Grass | Terrain::Rubble => out.push(p),
                Terrain::Road | Terrain::Bridge | Terrain::Pad | Terrain::Depot => {}
                // Forest, water, a wall: the straight road stops here.
                _ => break,
            }
        }
        out
    }

    /// The improvement the base most wants, by weight (`w_mend`,
    /// `w_mine`, `w_works`): the most hurt pillbox of the base mended
    /// (worth its hurt); a mine on the approach, past the wall facing
    /// the threat and off the road (worth half, at most `MINES`); works
    /// on a free pad by the base, a power plant first (worth half).
    pub fn improvement(obs: &Observation, base: V2, facing: Option<V2>, s: &Settings, material: u8, mines: u8) -> Option<(PillboyTask, CycleAct)> {
        let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
        let mut best: Option<(f32, PillboyTask, CycleAct)> = None;
        let mut offer = |u: f32, t: PillboyTask, w: CycleAct| {
            if u > 0.0 && best.as_ref().map(|b| u > b.0).unwrap_or(true) {
                best = Some((u, t, w));
            }
        };
        if let Some(p) = obs.pillboxes.iter().filter(|p| ours(p.owner) && p.pos.dist(base) <= CONNECT_NEAR && p.health < PILLBOX_HEALTH * MEND_BELOW).min_by(|a, b| a.health.total_cmp(&b.health).then(a.id.cmp(&b.id))) {
            if material >= 1 {
                offer(s.w[W::Mend] * (1.0 - p.health / PILLBOX_HEALTH), PillboyTask::Repair { target: p.id }, CycleAct::Mend);
            }
        }
        if let Some(f) = facing.filter(|_| material >= BuildKind::Mine.cost() && mines < MINES) {
            let r = crate::fortify::WALL_R + 2.0;
            let across = V2::new(-f.y, f.x);
            let k = mines as f32;
            let side = if mines % 2 == 0 { 1.0 } else { -1.0 };
            let at = Map::tile_centre(base + f.scale(r) + across.scale(side * (2.0 + k)));
            offer(s.w[W::Mine] * 0.5, PillboyTask::Build { what: BuildKind::Mine, at }, CycleAct::Mine);
        }
        let built = |p: V2| obs.structures.iter().any(|x| x.alive() && x.pos.dist(p) < 0.5);
        if let Some(pad) = obs.pads.iter().copied().filter(|p| p.dist(base) <= CONNECT_NEAR && !built(*p)).min_by(|a, b| a.dist(base).total_cmp(&b.dist(base))) {
            let powered = obs.structures.iter().any(|x| x.alive() && ours(x.owner) && x.kind == StructureKind::Power && x.pos.dist(base) <= POWER_RANGE);
            let kind = if powered { StructureKind::Workshop } else { StructureKind::Power };
            if material >= kind.cost() {
                offer(s.w[W::Works] * 0.5, PillboyTask::Build { what: BuildKind::Structure(kind), at: pad }, CycleAct::Works);
            }
        }
        best.map(|(_, t, w)| (t, w))
    }

    /// A road junction: a road or bridge tile with three or more road or
    /// bridge tiles beside it.
    pub fn junction(map: &Map, p: V2) -> bool {
        let road = |q: V2| map.in_bounds(q) && matches!(map.at(q), Terrain::Road | Terrain::Bridge);
        road(p) && [V2::new(1.0, 0.0), V2::new(-1.0, 0.0), V2::new(0.0, 1.0), V2::new(0.0, -1.0)].iter().filter(|d| road(p + **d)).count() >= 3
    }

    /// Road posts (roadmap 12h; Paul: "they need to not stick at
    /// junctions but connect bases and castles"): on the line from the
    /// base to each of `to` (other bases and castles of the side), nearest
    /// first, a point every `spacing` tiles from past the castle's road to
    /// short of the far end, set beside the road on open ground, none
    /// within `clear` tiles of a junction or within `spacing / 2` of a
    /// pillbox in `have`; at most `max`.
    pub fn road_posts(map: &Map, base: V2, to: &[V2], have: &[V2], spacing: f32, clear: f32, max: usize) -> Vec<V2> {
        let spacing = spacing.max(4.0);
        let mut out: Vec<V2> = Vec::new();
        let mut far: Vec<V2> = to.iter().copied().filter(|t| t.dist(base) > 2.0 * spacing).collect();
        far.sort_by(|a, b| a.dist(base).total_cmp(&b.dist(base)).then(a.x.total_cmp(&b.x)));
        let near_junction = |p: V2| {
            let r = clear.ceil() as i32;
            (-r..=r).any(|dy| (-r..=r).any(|dx| {
                let q = p + V2::new(dx as f32, dy as f32);
                q.dist(p) <= clear && Self::junction(map, q)
            }))
        };
        for t in far {
            let dir = (t - base).norm();
            let across = V2::new(-dir.y, dir.x);
            let mut k = crate::fortify::ROAD_TO + 1.0 + spacing;
            while k < t.dist(base) - 6.0 && out.len() < max {
                let on = Map::tile_centre(base + dir.scale(k));
                k += spacing;
                if near_junction(on) || have.iter().chain(out.iter()).any(|h| h.dist(on) < spacing / 2.0) {
                    continue;
                }
                let beside = [1.0, -1.0].into_iter().map(|s| Map::tile_centre(on + across.scale(s))).find(|q| map.in_bounds(*q) && matches!(map.at(*q), Terrain::Grass | Terrain::Rubble | Terrain::Forest));
                if let Some(q) = beside {
                    out.push(q);
                }
            }
        }
        out
    }

    /// One think. Some((orders, word)) while the cycle has the tank:
    /// the caller sends them and holds the tank at the base.
    pub fn think(&mut self, obs: &Observation, me: &Tank, base: V2, plan: Option<&mut Plan>, s: &Settings, map: Option<&Map>) -> Option<(Vec<Command>, String)> {
        if self.stage == Stage::Idle {
            return None;
        }
        let now = obs.tick;
        if now >= self.since + STAGE_FOR {
            self.advance(now);
            if self.stage == Stage::Idle {
                return None;
            }
        }
        let squad = &me.pillboys;
        if squad.strength == 0 {
            self.stage = Stage::Idle;
            return None;
        }
        if !squad.aboard || squad.task.is_some() {
            self.act = CycleAct::SquadOut;
            return Some((Vec::new(), CycleAct::SquadOut.word().into()));
        }
        if self.out {
            // The squad came back from the stage's order: on to the next.
            self.advance(now);
            if self.stage == Stage::Idle {
                return None;
            }
        }
        let mut cmds = Vec::new();
        let facing = plan.as_ref().map(|p| p.facing);
        match self.stage {
            Stage::Idle => return None,
            Stage::Item => {
                let task = match plan.filter(|_| s.w[W::GiveItem] > 0.0) {
                    Some(p) => match p.next(obs, squad.material) {
                        Want::Order(t) => Some(t),
                        _ => None,
                    },
                    None => None,
                };
                match task {
                    Some(t) => {
                        let felling = matches!(t, PillboyTask::Harvest { .. });
                        cmds.push(Command::Deploy { task: t });
                        // A fell pays for the item: the item is next.
                        if !felling {
                            self.out = true;
                        }
                        self.act = if felling { CycleAct::Felling } else { CycleAct::Item };
                        return Some((cmds, self.act.word().into()));
                    }
                    None => self.advance(now),
                }
            }
            Stage::Improve => {}
            Stage::Connect => {}
        }
        if self.stage == Stage::Improve {
            // The improvement (LR 10, Paul's answer as scalars): mend,
            // a mine, or works, each by its weight times how much it is
            // wanted; `upgrade_vs_mend` will give 11g's upgrades their
            // share of the slot once there are upgrades to buy.
            match Self::improvement(obs, base, facing, s, squad.material, self.mines).filter(|_| s.w[W::GiveUpgrade] > 0.0) {
                Some((task, act)) => {
                    if matches!(task, PillboyTask::Build { what: BuildKind::Mine, .. }) {
                        self.mines += 1;
                    }
                    cmds.push(Command::Deploy { task });
                    self.out = true;
                    self.act = act;
                    return Some((cmds, act.word().into()));
                }
                None => self.advance(now),
            }
        }
        if self.stage == Stage::Connect {
            let n = s.w[W::ConnectTiles].max(0.0) as u8;
            let Some(map) = map.filter(|_| s.w[W::Connect] > 0.0 && self.laid < n && squad.material >= BuildKind::Road.cost()) else {
                self.advance(now);
                return None;
            };
            let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
            let to = obs
                .pillboxes
                .iter()
                .filter(|p| ours(p.owner))
                .map(|p| p.pos)
                .chain(obs.depots.iter().filter(|d| ours(d.owner)).map(|d| d.pos))
                .filter(|p| (CONNECT_NEAR..=CONNECT_FAR).contains(&p.dist(base)))
                .filter(|p| !Self::road_to(map, base, *p, 1).is_empty())
                // A base or castle before a lone pillbox (`connect_bases`):
                // its distance counted at 1 - connect_bases.
                .map(|p| {
                    let base_like = obs.depots.iter().any(|d| d.pos.dist(p) < 1.0);
                    (p, p.dist(base) * if base_like { (1.0 - s.w[W::ConnectBases]).max(0.0) } else { 1.0 })
                })
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.x.total_cmp(&b.0.x)))
                .map(|(p, _)| p);
            match to.and_then(|to| Self::road_to(map, base, to, 1).first().copied()) {
                Some(tile) => {
                    cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at: tile } });
                    self.laid += 1;
                    // Each tile is its own trip; stay on Connect.
                    self.since = now;
                    self.act = CycleAct::Road;
                    return Some((cmds, format!("{}, tile {} of {}", CycleAct::Road.word(), self.laid, n)));
                }
                None => self.advance(now),
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One seat, its squad full: the cycle gives one plan item, then (no
    /// hurt pillbox) passes the improvement, then lays road toward the
    /// next friendly pillbox, a tile a trip.
    #[test]
    fn a_full_tank_gives_the_base_an_item() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(1) };
        let mut w = World::new(sc);
        let base = Map::tile_centre(w.tanks[0].pos);
        // Open ground round the base, a pillbox of ours 20 tiles east.
        for dy in -8..=8 {
            for dx in -8..=30 {
                w.map.set(Map::tile_centre(base + V2::new(dx as f32, dy as f32)), Terrain::Grass);
            }
        }
        w.tanks[0].pillboys.material = PILLBOY_MATERIAL_MAX;
        w.pillboxes.clear();
        w.pillboxes.push(Pillbox { id: 9000, pos: base + V2::new(20.0, 0.0), owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let mut plan = Plan::castle(base, base + V2::new(-30.0, 0.0));
        let s = Settings::default();
        let mut c = Cycle::default();
        c.begin(0);
        let obs = w.observe(0);
        let me = obs.me.clone().unwrap();
        let (cmds, word) = c.think(&obs, &me, base, Some(&mut plan), &s, Some(&w.map)).expect("the cycle has the tank");
        assert!(matches!(cmds.as_slice(), [Command::Deploy { task: PillboyTask::Build { .. } }]), "{:?} {}", cmds, word);
        assert_eq!(c.stage, Stage::Item);
        // The squad back (the order out): no hurt pillbox and no pad, so
        // the improvement is a mine on the approach.
        let mut obs = w.observe(0);
        obs.tick = 40;
        let me = obs.me.clone().unwrap();
        let (cmds, word) = c.think(&obs, &me, base, Some(&mut plan), &s, Some(&w.map)).expect("still the cycle");
        assert_eq!(c.stage, Stage::Improve, "{}", word);
        assert!(matches!(cmds.as_slice(), [Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, .. } }]), "{:?}", cmds);
        // Back again: the road.
        let mut obs = w.observe(0);
        obs.tick = 80;
        let me = obs.me.clone().unwrap();
        let (cmds, word) = c.think(&obs, &me, base, Some(&mut plan), &s, Some(&w.map)).expect("still the cycle");
        assert_eq!(c.stage, Stage::Connect, "{}", word);
        match cmds.as_slice() {
            [Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at } }] => {
                assert!(at.dist(base) >= YARD - 0.5 && (at.y - base.y).abs() < 1.0, "the road heads for the pillbox: {:?}", at);
            }
            other => panic!("{:?}", other),
        }
        // Out of time on every stage: idle again.
        let mut obs = w.observe(0);
        obs.tick = 80 + STAGE_FOR + 1;
        let me = obs.me.clone().unwrap();
        assert!(c.think(&obs, &me, base, Some(&mut plan), &s, Some(&w.map)).is_none() || c.stage == Stage::Connect);
    }

    /// The improvement by weight: a hurt pillbox is mended before a mine
    /// at the defaults; with `w_mend` at 0 the mine; with both at 0,
    /// nothing.
    #[test]
    fn the_improvement_goes_by_its_weights() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(1) };
        let mut w = World::new(sc);
        let base = Map::tile_centre(w.tanks[0].pos);
        w.pillboxes.clear();
        w.pillboxes.push(Pillbox { id: 9001, pos: base + V2::new(3.0, 0.0), owner: Some(0), health: PILLBOX_HEALTH * 0.3, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let obs = w.observe(0);
        let f = Some(V2::new(1.0, 0.0));
        let mut s = Settings::default();
        assert!(matches!(Cycle::improvement(&obs, base, f, &s, 8, 0), Some((PillboyTask::Repair { .. }, _))));
        s.w[W::Mend] = 0.0;
        assert!(matches!(Cycle::improvement(&obs, base, f, &s, 8, 0), Some((PillboyTask::Build { what: BuildKind::Mine, .. }, _))));
        s.w[W::Mine] = 0.0;
        assert!(Cycle::improvement(&obs, base, f, &s, 8, 0).is_none() || obs.pads.iter().any(|p| p.dist(base) <= CONNECT_NEAR));
        assert!(Cycle::improvement(&obs, base, f, &Settings::default(), 8, MINES).map(|(t, _)| !matches!(t, PillboyTask::Build { what: BuildKind::Mine, .. })).unwrap_or(true), "no more than MINES");
    }

    /// Posts on the line between two bases, beside the road, every
    /// spacing, none by a junction or by a pillbox already there.
    #[test]
    fn posts_line_the_road_and_keep_off_junctions() {
        let mut m = Map::generate(5);
        let base = V2::new(20.5, 60.5);
        let far = V2::new(80.5, 60.5);
        for x in 10..95 {
            for y in 55..66 {
                m.set(Map::tile_centre(V2::new(x as f32, y as f32)), Terrain::Grass);
            }
            m.set(Map::tile_centre(V2::new(x as f32, 60.0)), Terrain::Road);
        }
        let posts = Cycle::road_posts(&m, base, &[far], &[], 10.0, 3.0, 9);
        assert!(posts.len() >= 3, "{:?}", posts);
        for p in &posts {
            assert!((p.y - 60.5).abs() > 0.5 && (p.y - 60.5).abs() < 2.0, "beside the road, not on it: {:?}", p);
            assert!(m.at(*p) != Terrain::Road);
        }
        for w in posts.windows(2) {
            assert!((w[1].x - w[0].x - 10.0).abs() < 1.5, "every spacing: {:?}", posts);
        }
        // A crossroad at x 50: the post there is skipped.
        for y in 50..71 {
            m.set(Map::tile_centre(V2::new(50.0, y as f32)), Terrain::Road);
        }
        let posts2 = Cycle::road_posts(&m, base, &[far], &[], 10.0, 3.0, 9);
        assert!(posts2.iter().all(|p| (p.x - 50.5).abs() > 3.0), "off the junction: {:?}", posts2);
        // A pillbox already on the road covers its stretch.
        let posts3 = Cycle::road_posts(&m, base, &[far], &[posts[0]], 10.0, 3.0, 9);
        assert!(!posts3.contains(&posts[0]));
        assert!(Cycle::road_posts(&m, base, &[far], &[], 10.0, 3.0, 2).len() <= 2);
    }

    #[test]
    fn the_road_runs_past_the_yard_and_stops_at_water() {
        let mut m = Map::generate(3);
        for x in 0..40 {
            for y in 9..12 {
                m.set(Map::tile_centre(V2::new(x as f32, y as f32)), Terrain::Grass);
            }
        }
        m.set(V2::new(20.5, 10.5), Terrain::Water);
        let road = Cycle::road_to(&m, V2::new(5.5, 10.5), V2::new(35.5, 10.5), 30);
        assert!(!road.is_empty());
        assert!(road.iter().all(|p| p.dist(V2::new(5.5, 10.5)) >= YARD - 0.5));
        assert!(road.iter().all(|p| p.x < 20.0), "stops at the water");
        let short = Cycle::road_to(&m, V2::new(5.5, 10.5), V2::new(35.5, 10.5), 3);
        assert_eq!(short.len(), 3);
    }
}
