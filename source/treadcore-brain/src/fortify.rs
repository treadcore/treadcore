//! Fortification to a plan (roadmap step 11b): a castle's gate.
//!
//! Round a point (a depot of the side, by default), facing the threat:
//!
//! ```text
//!                    threat
//!                      ^
//!          # # #   gate   # # #         # the curtain wall, WALL_R out,
//!      # #        T   =   T     # #       WALL_ARC either side of the
//!    #                =             #     threat; the gate left open
//!                     =                 T the guard towers, TOWER_R out,
//!                     D                   TOWER_ANGLE either side of it
//!                                       = the road, home through the gate
//!                     P                 P the rear pillbox, REAR_R behind
//! ```
//!
//! Every pillbox is inside every other's 8-tile reach, so a tank that
//! comes for one is under the guns of two; the gate is the only way in
//! from the front, so the router brings an attacker through it, between
//! the towers. A plan is a list of squad orders given one at a time.
//! The Observation carries no map, so the plan learns the ground from
//! the world's answers: built, refused and why, abandoned and why.
//!
//! Turrets (step 11h) come later, when the castle stands and nothing
//! has come at it for a while: `add_turrets` puts a bastion at each end
//! of the curtain wall, set into it with one more wall tile beyond, for
//! a pillbox carried in from the side's outlying ones. The ends are
//! where an attacker goes round the wall instead of through the gate;
//! each bastion is in reach of a tower and of the rear pillbox.

use treadcore_sim::*;

/// The curtain wall's radius and its half-arc either side of the threat.
pub const WALL_R: f32 = 5.0;
pub const WALL_ARC: f32 = 70.0 * std::f32::consts::PI / 180.0;
/// Wall tiles within this of the threat's bearing are the gate: one
/// tile, or two when the bearing falls between them. The first plan had
/// 13 degrees and a gate three tiles wide, which turned nobody. The
/// wall need not be joined side to side: the router takes no diagonal
/// step past a wall tile, so a ring that touches at the corners holds.
pub const GATE_HALF: f32 = 8.0 * std::f32::consts::PI / 180.0;
/// The towers: inside the wall, either side of the gate, `PILLBOX_SPACING`
/// and more apart (2 x 4 x sin 32 degrees = 4.2 tiles).
pub const TOWER_R: f32 = 4.0;
pub const TOWER_ANGLE: f32 = 32.0 * std::f32::consts::PI / 180.0;
/// The rear pillbox, behind the point: 6.7 tiles from each tower.
pub const REAR_R: f32 = 3.0;
/// The road from just off the depot out past the gate.
pub const ROAD_FROM: f32 = 3.0;
pub const ROAD_TO: f32 = 7.0;
/// The bastions: on the wall's ring, this far round past its ends, and
/// the wall carried on this far again beyond each. 3.9 tiles from the
/// near tower, 6.2 from the rear pillbox.
pub const TURRET_PAST: f32 = 12.0 * std::f32::consts::PI / 180.0;
pub const TURRET_WALL: f32 = 14.0 * std::f32::consts::PI / 180.0;
/// The yard's ring road, between the depot and the towers.
pub const YARD_R: f32 = 3.0;
/// A tile refused because a tank stood on it is tried again after this.
const RETRY: Tick = 5 * TICK_HZ as Tick;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    Pending,
    /// Ordered at this tick; the squad is on it.
    Ordered(Tick),
    /// Refused for now (a tank on the tile): try again from this tick.
    Later(Tick),
    /// Forest on the tile: fell it first (which pays for the rest).
    Fell,
    Done,
    /// The world will not have it here.
    Skipped,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub what: BuildKind,
    pub at: V2,
    pub state: State,
    /// A turret: set down from a carried pillbox, never built new.
    pub lifted: bool,
}

/// What the plan wants of the squad this tick.
#[derive(Clone, Debug, PartialEq)]
pub enum Want {
    Order(PillboyTask),
    /// Material short: no forest the plan knows of in reach.
    Starved,
    /// The squad is out on the plan's order, or the tank must wait.
    Wait,
    Finished,
}

#[derive(Clone, Debug)]
pub struct Plan {
    pub centre: V2,
    /// Unit vector toward the threat.
    pub facing: V2,
    pub items: Vec<Item>,
    /// The ring's tiles left open: the gate.
    pub gate_tiles: Vec<V2>,
    /// Forest tiles already sent for, so a tile felled is not asked again.
    felled: Vec<(i32, i32)>,
    /// The harvest in progress, if the squad is out on one.
    felling: Option<V2>,
    /// Items before this are the castle; after, the turrets' (step 11h).
    castle_len: usize,
    /// Tiles a fleet commander has given to a crew: its own squad leaves
    /// them be (`next`).
    pub skip: Vec<(i32, i32)>,
    /// Where the tank sits while the squad builds: behind the point,
    /// moved onto land by `fit_post` (an island's depot at the shore had
    /// it on the water, and the leader hopped at it all match).
    pub post: V2,
    post_fitted: bool,
    /// What the turrets would leave to do, asked before they are added
    /// (`turret_preview`): it depends only on the castle's tiles, so it
    /// is worked out once (roadmap 12b; the Auto ladder copied the
    /// whole plan every tick to ask it).
    preview: std::sync::OnceLock<(bool, bool)>,
    /// A capped castle (roadmap 12h): pillboxes inside the wall, and
    /// slots outside on its road; None, the castle and bastions of 11h.
    pub cap: Option<(usize, usize)>,
    /// Road posts added (`add_posts`).
    pub posts: usize,
}

impl Plan {
    /// The castle round `centre`, gate toward `threat`, in building
    /// order: the towers and the rear pillbox (the guns first: a wall
    /// nobody covers only slows an attacker), the wall from its ends in
    /// to the gate, the road.
    pub fn castle(centre: V2, threat: V2) -> Plan {
        let c = Map::tile_centre(centre);
        let facing = {
            let d = threat - c;
            if d.len() < 0.5 {
                V2::new(1.0, 0.0)
            } else {
                d.norm()
            }
        };
        let h = facing.heading();
        let mut items = Vec::new();
        let push = |items: &mut Vec<Item>, what, at: V2| {
            let at = Map::tile_centre(at);
            if !items.iter().any(|i: &Item| i.at.tile() == at.tile()) {
                items.push(Item { what, at, state: State::Pending, lifted: false });
            }
        };
        for a in [TOWER_ANGLE, -TOWER_ANGLE] {
            push(&mut items, BuildKind::Pillbox, c + V2::from_heading(h + a).scale(TOWER_R));
        }
        push(&mut items, BuildKind::Pillbox, c - facing.scale(REAR_R));
        // The wall: every tile of the ring at WALL_R inside the arc and
        // outside the gate, the far ends first.
        let off = |p: V2| {
            let mut d = (p - c).heading() - h;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            d.abs()
        };
        let ri = WALL_R.ceil() as i32 + 1;
        let mut wall: Vec<V2> = Vec::new();
        let mut gate_tiles: Vec<V2> = Vec::new();
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let p = Map::tile_centre(c + V2::new(dx as f32, dy as f32));
                let d = p.dist(c);
                if (WALL_R - 0.5..WALL_R + 0.5).contains(&d) && off(p) <= WALL_ARC {
                    if off(p) > GATE_HALF {
                        wall.push(p);
                    } else {
                        gate_tiles.push(p);
                    }
                }
            }
        }
        wall.sort_by(|a, b| off(*b).total_cmp(&off(*a)).then(a.x.total_cmp(&b.x)).then(a.y.total_cmp(&b.y)));
        for p in wall {
            push(&mut items, BuildKind::Wall, p);
        }
        let mut k = ROAD_FROM;
        while k <= ROAD_TO {
            push(&mut items, BuildKind::Road, c + facing.scale(k));
            k += 1.0;
        }
        let castle_len = items.len();
        Plan { centre: c, facing, items, gate_tiles, felled: Vec::new(), felling: None, castle_len, skip: Vec::new(), post: c - facing.scale(1.5), post_fitted: false, preview: std::sync::OnceLock::new(), cap: None, posts: 0 }
    }

    /// A capped castle (Paul, 18 October: "we also stop filling the
    /// castle once it has ... 2 on the inside, 1 on the outside along the
    /// road"): `inside` pillboxes inside the wall (the gate's two towers,
    /// then the rear, then either flank), built new; and, once it stands,
    /// `road` slots outside beside its road for pillboxes carried in
    /// (`add_turrets`), in place of the bastions.
    pub fn castle_capped(centre: V2, threat: V2, inside: usize, road: usize) -> Plan {
        let mut p = Plan::castle(centre, threat);
        let c = p.centre;
        let h = p.facing.heading();
        let mut kept = 0;
        p.items.retain(|i| {
            if i.what != BuildKind::Pillbox {
                return true;
            }
            kept += 1;
            kept <= inside
        });
        // More than the towers and the rear: the flanks.
        for a in [std::f32::consts::FRAC_PI_2, -std::f32::consts::FRAC_PI_2].into_iter().take(inside.saturating_sub(3)) {
            let at = Map::tile_centre(c + V2::from_heading(h + a).scale(REAR_R));
            if !p.items.iter().any(|i| i.at.tile() == at.tile()) {
                p.items.insert(kept.min(3), Item { what: BuildKind::Pillbox, at, state: State::Pending, lifted: false });
            }
        }
        p.castle_len = p.items.len();
        p.cap = Some((inside, road));
        p
    }

    /// Road posts (roadmap 12h): slots for carried pillboxes along the
    /// roads between bases, as lifted items after everything else.
    pub fn add_posts(&mut self, posts: &[V2]) {
        for at in posts {
            let at = Map::tile_centre(*at);
            if !self.items.iter().any(|i| i.at.tile() == at.tile()) {
                self.items.push(Item { what: BuildKind::Pillbox, at, state: State::Pending, lifted: true });
                self.posts += 1;
            }
        }
    }

    /// The turrets' slots and their walls, once.
    pub fn add_turrets(&mut self) {
        if self.items.len() > self.castle_len {
            return;
        }
        let h = self.facing.heading();
        if let Some((_, road)) = self.cap {
            // Capped: the road on past the gate, a slot beside it every
            // four tiles, alternate sides, for a carried pillbox.
            let across = V2::new(-self.facing.y, self.facing.x);
            let far = ROAD_TO + 1.0 + 4.0 * road as f32;
            let mut k = ROAD_TO + 1.0;
            while k <= far {
                let at = Map::tile_centre(self.centre + self.facing.scale(k));
                if !self.items.iter().any(|i| i.at.tile() == at.tile()) {
                    self.items.push(Item { what: BuildKind::Road, at, state: State::Pending, lifted: false });
                }
                k += 1.0;
            }
            for n in 0..road {
                let side = if n % 2 == 0 { 1.0 } else { -1.0 };
                let at = Map::tile_centre(self.centre + self.facing.scale(ROAD_TO + 1.0 + 4.0 * n as f32) + across.scale(side));
                if !self.items.iter().any(|i| i.at.tile() == at.tile()) {
                    self.items.push(Item { what: BuildKind::Pillbox, at, state: State::Pending, lifted: true });
                }
            }
        }
        for side in [1.0, -1.0] {
            if self.cap.is_some() {
                break;
            }
            let at = Map::tile_centre(self.centre + V2::from_heading(h + side * (WALL_ARC + TURRET_PAST)).scale(WALL_R));
            if !self.items.iter().any(|i| i.at.tile() == at.tile()) {
                self.items.push(Item { what: BuildKind::Pillbox, at, state: State::Pending, lifted: true });
            }
        }
        for side in [1.0, -1.0] {
            if self.cap.is_some() {
                break;
            }
            let at = Map::tile_centre(self.centre + V2::from_heading(h + side * (WALL_ARC + TURRET_PAST + TURRET_WALL)).scale(WALL_R));
            if !self.items.iter().any(|i| i.at.tile() == at.tile()) {
                self.items.push(Item { what: BuildKind::Wall, at, state: State::Pending, lifted: false });
            }
        }
        // The yard (Paul, 17 October: "a base, with roads in the centre,
        // walls on the outside"): the ring closed round the back, on from
        // the bastions, and a ring road inside it round the depot.
        let c = self.centre;
        let off = |p: V2| {
            let mut d = (p - c).heading() - h;
            while d > std::f32::consts::PI {
                d -= std::f32::consts::TAU;
            }
            while d < -std::f32::consts::PI {
                d += std::f32::consts::TAU;
            }
            d.abs()
        };
        let ring = |r: f32| {
            let ri = r.ceil() as i32 + 1;
            let mut out: Vec<V2> = Vec::new();
            for dy in -ri..=ri {
                for dx in -ri..=ri {
                    let p = Map::tile_centre(c + V2::new(dx as f32, dy as f32));
                    if (r - 0.5..r + 0.5).contains(&p.dist(c)) {
                        out.push(p);
                    }
                }
            }
            out.sort_by(|a, b| off(*a).total_cmp(&off(*b)).then(a.x.total_cmp(&b.x)).then(a.y.total_cmp(&b.y)));
            out
        };
        for p in ring(WALL_R).into_iter().filter(|p| off(*p) > WALL_ARC) {
            if !self.items.iter().any(|i| i.at.tile() == p.tile()) {
                self.items.push(Item { what: BuildKind::Wall, at: p, state: State::Pending, lifted: false });
            }
        }
        for p in ring(YARD_R) {
            if !self.items.iter().any(|i| i.at.tile() == p.tile()) {
                self.items.push(Item { what: BuildKind::Road, at: p, state: State::Pending, lifted: false });
            }
        }
    }

    /// With the turrets added (whether or not they are yet): a turret
    /// slot to fill, and yard walls or roads left. Before they are added
    /// the answer depends only on the castle's tiles, so it is kept.
    pub fn turret_preview(&self) -> (bool, bool) {
        if self.items.len() > self.castle_len {
            return (self.turret_slot().is_some(), self.yard_left());
        }
        *self.preview.get_or_init(|| {
            crate::meter::plan_clone();
            let mut q = self.clone();
            q.add_turrets();
            (q.turret_slot().is_some(), q.yard_left())
        })
    }

    /// Walls and roads of the turrets' part still to build: work for the
    /// squad while there is no pillbox to fetch.
    pub fn yard_left(&self) -> bool {
        self.items.len() > self.castle_len && self.items[self.castle_len..].iter().any(|i| !i.lifted && !matches!(i.state, State::Done | State::Skipped))
    }

    /// The post on land: the nearest passable ground, not water, within
    /// 3 tiles of the centre, to the post as drawn. Once, with the map.
    pub fn fit_post(&mut self, map: &Map) {
        if self.post_fitted {
            return;
        }
        self.post_fitted = true;
        let ok = |p: V2| map.in_bounds(p) && map.at(p).passable() && !matches!(map.at(p), Terrain::Water | Terrain::Bridge);
        if ok(self.post) {
            return;
        }
        let want = self.post;
        let c = self.centre;
        if let Some(p) = (-3..=3)
            .flat_map(|dy| (-3..=3).map(move |dx| Map::tile_centre(c + V2::new(dx as f32, dy as f32))))
            .filter(|p| p.dist(c) <= 3.0 && ok(*p))
            .min_by(|a, b| a.dist(want).total_cmp(&b.dist(want)))
        {
            self.post = p;
        }
    }

    /// The castle without its turrets is finished: nothing left to build
    /// or wait on.
    pub fn castle_stands(&self) -> bool {
        self.items[..self.castle_len].iter().all(|i| matches!(i.state, State::Done | State::Skipped))
    }

    /// The turrets and their walls are all up (or cannot be).
    pub fn turrets_stand(&self) -> bool {
        self.items.len() > self.castle_len && self.items[self.castle_len..].iter().all(|i| matches!(i.state, State::Done | State::Skipped))
    }

    /// The next turret slot waiting for a pillbox, and whether it is
    /// forest to be felled first.
    pub fn turret_slot(&self) -> Option<(V2, bool)> {
        self.items.iter().find(|i| i.lifted && matches!(i.state, State::Pending | State::Fell)).map(|i| (i.at, i.state == State::Fell))
    }

    /// A forest turret slot sent for felling: pending again after.
    pub fn felling_slot(&mut self, at: V2) {
        self.felled.push(at.tile());
        if let Some(i) = self.item_at(at) {
            i.state = State::Pending;
        }
    }

    /// The turret slots: set down, and planned (not skipped).
    pub fn turrets(&self) -> (usize, usize) {
        (self.items.iter().filter(|i| i.lifted && i.state == State::Done).count(), self.items.iter().filter(|i| i.lifted && i.state != State::Skipped).count())
    }

    /// A turret slot the squad was sent to set a pillbox down on.
    pub fn ordered(&mut self, at: V2, tick: Tick) {
        if let Some(i) = self.item_at(at) {
            i.state = State::Ordered(tick);
        }
    }

    /// The first item a crew could take: pending, not a turret, not in
    /// `taken` (given out already, or the commander's own squad's).
    pub fn free_item(&self, taken: &[(i32, i32)]) -> Option<(BuildKind, V2)> {
        self.items.iter().find(|i| !i.lifted && matches!(i.state, State::Pending | State::Fell) && !taken.contains(&i.at.tile())).map(|i| (i.what, i.at))
    }

    /// Turret slots waiting for a pillbox, not in `taken`.
    pub fn free_slots(&self, taken: &[(i32, i32)]) -> Vec<V2> {
        self.items.iter().filter(|i| i.lifted && matches!(i.state, State::Pending) && !taken.contains(&i.at.tile())).map(|i| i.at).collect()
    }

    /// The commander's own squad is on this tile: crews keep off it.
    pub fn own_order(&self) -> Option<(i32, i32)> {
        self.items.iter().find(|i| matches!(i.state, State::Ordered(_))).map(|i| i.at.tile())
    }

    /// A crew failed here twice: the plan does without it.
    pub fn give_up(&mut self, at: V2) {
        if let Some(i) = self.item_at(at) {
            i.state = State::Skipped;
        }
    }

    /// The gate's middle: where the road crosses the wall.
    pub fn gate(&self) -> V2 {
        self.centre + self.facing.scale(WALL_R)
    }

    /// The share of the castle (gate, walls, towers) not yet built or
    /// given up: 1 just planned, 0 standing (the posture's Fortify, LR 25).
    pub fn castle_left(&self) -> f32 {
        let castle = &self.items[..self.castle_len];
        if castle.is_empty() {
            return 0.0;
        }
        castle.iter().filter(|i| !matches!(i.state, State::Done | State::Skipped)).count() as f32 / castle.len() as f32
    }

    /// The share of pillbox slots (turrets, the road slot, road posts)
    /// still waiting for a pillbox: the posture's Connect (LR 25).
    pub fn slots_left(&self) -> f32 {
        let slots: Vec<&Item> = self.items.iter().filter(|i| i.lifted && i.state != State::Skipped).collect();
        if slots.is_empty() {
            return 0.0;
        }
        slots.iter().filter(|i| i.state != State::Done).count() as f32 / slots.len() as f32
    }

    /// The plan's items built.
    pub fn done(&self) -> usize {
        self.items.iter().filter(|i| i.state == State::Done).count()
    }

    /// The plan's item on the tile of `at`.
    fn item_at(&mut self, at: V2) -> Option<&mut Item> {
        self.items.iter_mut().find(|i| i.at.tile() == at.tile())
    }

    /// Read the world's answers to the plan's orders.
    pub fn hear(&mut self, obs: &Observation) {
        let me = obs.player;
        for e in &obs.events {
            match e {
                // Built by me, or by a crew of the side on a tile of the
                // plan (a fleet commander's request).
                Event::Built { owner, at, .. } if *owner == me || obs.allied(*owner) => {
                    if let Some(i) = self.item_at(*at) {
                        i.state = State::Done;
                    }
                }
                Event::PillboxBuilt { by, at, .. } if *by == me || obs.allied(*by) => {
                    if let Some(i) = self.item_at(*at) {
                        i.state = State::Done;
                    }
                }
                Event::Rejected { player, cmd: Command::Deploy { task: PillboyTask::Build { at, .. } }, why } if *player == me => {
                    let tick = obs.tick;
                    if let Some(i) = self.item_at(*at) {
                        i.state = if why.contains("harvest it first") {
                            State::Fell
                        } else if why.contains("no material") {
                            State::Pending
                        } else if why.contains("a tank is there") {
                            State::Later(tick + RETRY)
                        } else {
                            State::Skipped
                        };
                    }
                }
                Event::Rejected { player, cmd: Command::Deploy { task: PillboyTask::Harvest { at } }, why } if *player == me => {
                    // Not forest after all: the item there can be built.
                    // Forest that cannot be felled now (the squad's
                    // material is full, and nothing left to spend it on)
                    // is left standing: the plan does without that tile.
                    let full = why.contains("full");
                    if let Some(i) = self.item_at(*at) {
                        if full {
                            i.state = State::Skipped;
                        } else if i.state == State::Fell {
                            i.state = State::Pending;
                        }
                    }
                    self.felling = None;
                }
                Event::PillboysAbandoned { owner, why, .. } if *owner == me => {
                    // The order out: whatever it was for.
                    let tick = obs.tick;
                    for i in self.items.iter_mut().filter(|i| matches!(i.state, State::Ordered(_))) {
                        i.state = if why.contains("no material") {
                            State::Pending
                        } else if why.contains("tank") {
                            State::Later(tick + RETRY)
                        } else {
                            // "something is already there": a wall or a
                            // road there already does the plan's work.
                            State::Done
                        };
                    }
                    self.felling = None;
                }
                _ => {}
            }
        }
        // A pillbox of the side's already standing at a tower site is the
        // tower; one of anyone's makes the site unbuildable.
        for i in self.items.iter_mut().filter(|i| i.what == BuildKind::Pillbox && !matches!(i.state, State::Done | State::Skipped)) {
            if let Some(p) = obs.pillboxes.iter().find(|p| p.pos.dist(i.at) < PILLBOX_SPACING) {
                i.state = if p.owner.map(|o| obs.allied(o)).unwrap_or(false) { State::Done } else { State::Skipped };
            }
        }
    }

    /// The next order for an idle squad with `material`, or why none.
    pub fn next(&mut self, obs: &Observation, material: u8) -> Want {
        let tick = obs.tick;
        // An order out that never came back done (the squad died, say):
        // after a minute it is pending again.
        // The squad is home and idle (that is when this is asked), so an
        // order out two seconds and more ago, and not refused or given
        // up, was done: a Built missed (a window that draws one frame for
        // several ticks) does not cost a minute's wait.
        for i in self.items.iter_mut() {
            match i.state {
                State::Ordered(t) if tick >= t + 2 * TICK_HZ as Tick => i.state = State::Done,
                State::Later(t) if tick >= t => i.state = State::Pending,
                _ => {}
            }
        }
        self.felling = None;
        let next = self.items.iter().position(|i| !i.lifted && matches!(i.state, State::Pending | State::Fell) && !self.skip.contains(&i.at.tile()));
        let Some(n) = next else {
            return if self.items.iter().any(|i| !i.lifted && matches!(i.state, State::Ordered(_) | State::Later(_))) { Want::Wait } else { Want::Finished };
        };
        let item = self.items[n].clone();
        if item.state == State::Fell {
            self.felled.push(item.at.tile());
            self.items[n].state = State::Pending;
            self.felling = Some(item.at);
            return Want::Order(PillboyTask::Harvest { at: item.at });
        }
        if material < item.what.cost() {
            // The nearest forest to the work not already sent for, and not
            // a tile the plan builds on (felling one of those is the Fell
            // case above).
            let forest = obs
                .forest
                .iter()
                .filter(|f| !self.felled.contains(&f.tile()) && !self.items.iter().any(|i| i.at.tile() == f.tile()))
                .min_by(|a, b| a.dist(self.centre).total_cmp(&b.dist(self.centre)))
                .copied();
            return match forest {
                Some(f) => {
                    self.felled.push(f.tile());
                    self.felling = Some(f);
                    Want::Order(PillboyTask::Harvest { at: f })
                }
                None => Want::Starved,
            };
        }
        self.items[n].state = State::Ordered(tick);
        Want::Order(PillboyTask::Build { what: item.what, at: item.at })
    }

    /// What the plan has, for the status line.
    pub fn progress(&self) -> String {
        let count = |k: BuildKind| (self.items.iter().filter(|i| !i.lifted && i.what == k && i.state == State::Done).count(), self.items.iter().filter(|i| !i.lifted && i.what == k && i.state != State::Skipped).count());
        let (p, pn) = count(BuildKind::Pillbox);
        let (w, wn) = count(BuildKind::Wall);
        let (r, rn) = count(BuildKind::Road);
        let mut s = format!("pillboxes {}/{}, wall {}/{}, road {}/{}", p, pn, w, wn, r, rn);
        if let (t, tn @ 1..) = self.turrets() {
            s.push_str(&format!(", turrets {}/{}", t, tn));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape, on paper: towers inside the wall and either side of the
    /// gate, every pillbox in every other's reach, the gate open, the
    /// wall on the threat's side only.
    /// Paul's castle (roadmap 12h): two pillboxes inside the wall, the
    /// gate's towers; once it stands, one slot outside beside its road
    /// and no bastions; road posts after it.
    #[test]
    fn the_capped_castle_has_two_inside_one_on_the_road() {
        let c = V2::new(50.5, 50.5);
        let mut p = Plan::castle_capped(c, V2::new(80.0, 50.5), 2, 1);
        let inside: Vec<&Item> = p.items.iter().filter(|i| i.what == BuildKind::Pillbox).collect();
        assert_eq!(inside.len(), 2);
        assert!(inside.iter().all(|i| i.at.dist(c) < WALL_R && !i.lifted));
        p.add_turrets();
        let slots: Vec<&Item> = p.items.iter().filter(|i| i.lifted).collect();
        assert_eq!(slots.len(), 1, "one on the road, no bastions");
        let s = slots[0].at;
        assert!(s.dist(c) > WALL_R && s.x > c.x + WALL_R, "outside, toward the threat: {:?}", s);
        assert!(p.items.iter().any(|i| i.what == BuildKind::Road && i.at.dist(s) < 2.0), "beside its road");
        assert!(p.items.iter().all(|i| i.at.tile() != s.tile() || i.what == BuildKind::Pillbox), "not on the road");
        p.add_posts(&[V2::new(70.5, 52.5), V2::new(70.5, 52.5)]);
        assert_eq!(p.posts, 1);
        assert_eq!(p.items.iter().filter(|i| i.lifted).count(), 2);
        let three = Plan::castle_capped(c, V2::new(80.0, 50.5), 3, 0);
        assert_eq!(three.items.iter().filter(|i| i.what == BuildKind::Pillbox).count(), 3);
    }

    #[test]
    fn the_castle_is_drawn_right() {
        let c = V2::new(40.5, 40.5);
        let plan = Plan::castle(c, V2::new(80.0, 40.5));
        let boxes: Vec<V2> = plan.items.iter().filter(|i| i.what == BuildKind::Pillbox).map(|i| i.at).collect();
        assert_eq!(boxes.len(), 3);
        for a in &boxes {
            for b in &boxes {
                if a != b {
                    assert!(a.dist(*b) <= PILLBOX_RANGE && a.dist(*b) >= PILLBOX_SPACING, "{:?} {:?}", a, b);
                }
            }
        }
        let gate = plan.gate();
        let walls: Vec<V2> = plan.items.iter().filter(|i| i.what == BuildKind::Wall).map(|i| i.at).collect();
        assert!(walls.len() >= 8, "{} wall tiles", walls.len());
        assert!(walls.iter().all(|w| w.dist(gate) >= 0.9), "the gate is walled");
        assert!((1..=2).contains(&plan.gate_tiles.len()), "a gate {} tiles wide", plan.gate_tiles.len());
        // An oblique threat: still a gate of one or two.
        let slant = Plan::castle(c, V2::new(70.0, 62.0));
        assert!((1..=2).contains(&slant.gate_tiles.len()), "an oblique gate {} tiles wide", slant.gate_tiles.len());
        assert!(walls.iter().all(|w| w.x >= c.x - 2.5), "wall behind the point: {:?}", walls);
        // The towers flank the gate, one each side.
        let (t0, t1) = (boxes[0], boxes[1]);
        assert!(t0.dist(gate) < 3.0 && t1.dist(gate) < 3.0, "towers {:?} {:?}, gate {:?}", t0, t1, gate);
        assert!((t0.y - c.y) * (t1.y - c.y) < 0.0, "both towers on one side");
        // The turrets: past the wall's ends, each in reach of two guns,
        // spaced from all, and walled on the outside.
        let mut plan = plan;
        plan.add_turrets();
        plan.add_turrets();
        let turrets: Vec<V2> = plan.items.iter().filter(|i| i.lifted).map(|i| i.at).collect();
        assert_eq!(turrets.len(), 2);
        for t in &turrets {
            assert!(boxes.iter().all(|b| b.dist(*t) >= PILLBOX_SPACING), "turret {:?} crowds {:?}", t, boxes);
            assert!(boxes.iter().filter(|b| b.dist(*t) <= PILLBOX_RANGE).count() >= 2, "turret {:?} covered by one", t);
            assert!(walls.iter().any(|w| w.dist(*t) < 1.6), "turret {:?} not at the wall's end", t);
        }
        // The yard: the ring closed but for the gate, a road round the
        // depot inside it.
        let ring: Vec<V2> = plan.items.iter().filter(|i| i.what == BuildKind::Wall).map(|i| i.at).collect();
        assert!(ring.len() >= walls.len() + 12, "{} wall tiles round, {} in front", ring.len(), walls.len());
        assert!(ring.iter().all(|w| w.dist(gate) >= 0.9), "the gate is walled");
        assert!(ring.iter().any(|w| w.x < c.x - 4.0), "no wall behind the point");
        let roads = plan.items.iter().filter(|i| i.what == BuildKind::Road && i.at.dist(c) < 4.0).count();
        assert!(roads >= 10, "{} yard road tiles", roads);
    }
}
