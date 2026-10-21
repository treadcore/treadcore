//! Rotation (Paul, 17 October: "every bot should have a push point, and
//! a defense point. the tank does not wait around ... alternate between
//! the adjustable push point and defense point (usually your fortress
//! or base)"):
//!
//! ```text
//!   base (defense point) ----------- push ----------- target
//!          D  <-- dwell, resupply    P  <-- dwell, look
//!             ---- out ---->
//!             <--- home ----
//! ```
//!
//! Where a tank used to wait (on guard with nothing in the leash, the
//! squad home and nothing to fetch, the castle standing) it rotates:
//! out to the push point, a look round, home to the defense point, a
//! look round, out again. A fight anywhere on the way is the caller's,
//! as before; the rotation is only what the idle time is spent on.
//!
//! The push point is `PUSH_FRAC` of the way from the base to the side's
//! objective: the nearest depot not of the side, else the nearest
//! pillbox not of the side, else the map's middle. A fleet shares a
//! base, so its members work out the same point: the fleet's push
//! point. A tank can be given its own (`own`), which is used instead.

use treadcore_sim::*;

/// The push point is this far from the base toward the objective.
pub const PUSH_FRAC: f32 = 0.5;
/// Within this of a point is there.
pub const ARRIVED: f32 = 3.0;
/// The look round at each end before turning back.
pub const DWELL: Tick = 6 * TICK_HZ as Tick;
/// Short of shells or armour, the defense point (the depot) holds the
/// tank until it has this share back, `REFILL_FOR` at most.
pub const REFILL: f32 = 0.9;
pub const REFILL_FOR: Tick = 25 * TICK_HZ as Tick;
/// The push point is fitted to the ground again this often.
pub const REFIT: Tick = 10 * TICK_HZ as Tick;
/// A wingman rotates once its leader has stood this long.
pub const WING_PARKED: Tick = 8 * TICK_HZ as Tick;
/// A wingman's push point is at most this far from its leader: inside
/// the leader's gathering ring (`GATHER_R`), so a rotating wingman still
/// counts as formed up and the fleet never waits on it.
pub const WING_REACH: f32 = 9.0;
/// The flood from the base looks at this many tiles at most.
const FIT_TILES: usize = 6000;

#[derive(Clone, Debug, Default)]
pub struct Rotation {
    /// Off: the tank waits where it is told to, as it always did.
    pub off: bool,
    /// A push point of its own, instead of the fleet's.
    pub own: Option<V2>,
    /// Bound for the push point (else the defense point).
    pub out: bool,
    /// Arrived at the current end at this tick.
    arrived: Option<Tick>,
    /// The fleet's push point, last worked out (for the status line).
    pub fleet: Option<V2>,
    /// The push point moved onto ground the tank can drive to from the
    /// base (`fit`): for which raw point, the fitted one, and when.
    fitted: Option<((i32, i32), V2, Tick)>,
    /// A wingman's leader stood still since this tick (`wing_push`).
    pub parked: Option<Tick>,
    /// The team board's front sectors' centres, and the share of the way
    /// the push point moves toward the one nearest it (`w_front_push`,
    /// LR 61); set by the Commander each think.
    pub front: Vec<V2>,
    pub front_w: f32,
}

/// Where the rotation sends the tank this tick, and what it says.
pub struct Leg {
    pub to: V2,
    pub word: String,
}

impl Rotation {
    /// The side's objective seen from `base`: the nearest depot not of
    /// the side, else the nearest pillbox not of the side, else the
    /// map's middle.
    pub fn objective(obs: &Observation, base: V2) -> V2 {
        let ours = |o: Option<PlayerId>| o.map(|o| o == obs.player || obs.allied(o)).unwrap_or(false);
        let near = |a: &V2, b: &V2| a.dist(base).total_cmp(&b.dist(base));
        obs.depots
            .iter()
            .filter(|d| !ours(d.owner) && d.pos.dist(base) > ARRIVED)
            .map(|d| d.pos)
            .min_by(near)
            .or_else(|| obs.pillboxes.iter().filter(|p| !ours(p.owner) && p.pos.dist(base) > ARRIVED).map(|p| p.pos).min_by(near))
            .unwrap_or(V2::new(obs.map_w as f32 / 2.0, obs.map_h as f32 / 2.0))
    }

    /// The fleet's push point: `PUSH_FRAC` of the way to the objective,
    /// then `front_w` of the way toward the front sector nearest that.
    pub fn fleet_push(&self, obs: &Observation, base: V2) -> V2 {
        let target = Self::objective(obs, base);
        let raw = base + (target - base).scale(PUSH_FRAC);
        match self.front.iter().min_by(|a, b| a.dist(raw).total_cmp(&b.dist(raw))) {
            Some(f) if self.front_w > 0.0 => raw + (*f - raw).scale(self.front_w),
            _ => raw,
        }
    }

    /// This tank's push point: its own, else the fleet's (on ground
    /// it can drive to, once `fit` has seen the map).
    pub fn push(&mut self, obs: &Observation, base: V2) -> V2 {
        let raw = self.fleet_push(obs, base);
        let fleet = match self.fitted {
            Some((t, p, _)) if t == raw.tile() => p,
            _ => raw,
        };
        self.fleet = Some(fleet);
        self.own.unwrap_or(fleet)
    }

    /// The push point as `push` would give it, without changing anything
    /// (for the window's coordination screen).
    pub fn peek(&self, obs: &Observation, base: V2) -> V2 {
        let raw = self.fleet_push(obs, base);
        let fleet = match self.fitted {
            Some((t, p, _)) if t == raw.tile() => p,
            _ => raw,
        };
        self.own.unwrap_or(fleet)
    }

    /// A wingman's push point: from the parked leader toward the side's
    /// objective (or toward its own point), `WING_REACH` out at most,
    /// pulled back toward the leader onto ground (`ground`).
    pub fn wing_push(&self, obs: &Observation, leader: V2, ground: impl Fn(V2) -> bool) -> V2 {
        let aim = self.own.unwrap_or_else(|| Self::objective(obs, leader));
        let d = aim.dist(leader);
        if d < 0.5 {
            return leader;
        }
        let dir = (aim - leader).norm();
        let mut r = d.min(WING_REACH);
        while r > ARRIVED {
            let p = leader + dir.scale(r);
            if ground(p) {
                return p;
            }
            r -= 1.0;
        }
        leader
    }

    /// With the map: the push point is the tile nearest it that the tank
    /// can drive to from the base, without a boat. Halfway to an island
    /// is on the water, and a guard drove at it all match, refused.
    /// Again every `REFIT` (a bridge opens the way further).
    pub fn fit(&mut self, obs: &Observation, map: &Map, base: V2) {
        let raw = self.fleet_push(obs, base);
        if let Some((t, _, at)) = self.fitted {
            if t == raw.tile() && obs.tick < at + REFIT {
                return;
            }
        }
        let ground = |x: i32, y: i32| map.in_tile(x, y) && map.at_tile(x as usize, y as usize).passable();
        let (bx, by) = base.tile();
        let (rx, ry) = raw.tile();
        let d2 = |x: i32, y: i32| (x - rx) * (x - rx) + (y - ry) * (y - ry);
        let mut best = (d2(bx, by), (bx, by));
        if ground(bx, by) {
            let mut seen = std::collections::HashSet::new();
            let mut queue = std::collections::VecDeque::new();
            seen.insert((bx, by));
            queue.push_back((bx, by));
            while let Some((x, y)) = queue.pop_front() {
                if seen.len() > FIT_TILES {
                    break;
                }
                if d2(x, y) < best.0 {
                    best = (d2(x, y), (x, y));
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let n = (x + dx, y + dy);
                    if ground(n.0, n.1) && seen.insert(n) {
                        queue.push_back(n);
                    }
                }
            }
        }
        let p = V2::new(best.1 .0 as f32 + 0.5, best.1 .1 as f32 + 0.5);
        self.fitted = Some((raw.tile(), p, obs.tick));
    }

    /// The leg to drive: out to `push` or home to `defense`, turning at
    /// each end after `DWELL` (and at home, once refilled).
    pub fn leg(&mut self, obs: &Observation, me: &Tank, defense: V2, push: V2) -> Leg {
        let to = if self.out { push } else { defense };
        let there = me.pos.dist(to) <= ARRIVED;
        if !there {
            self.arrived = None;
            let word = if self.out { "out to the push point" } else { "home to the defense point" };
            return Leg { to, word: format!("rotation: {} ({:.0},{:.0}), {:.0} tiles", word, to.x, to.y, me.pos.dist(to)) };
        }
        let since = *self.arrived.get_or_insert(obs.tick);
        let held = obs.tick.saturating_sub(since);
        let short = (me.shells as f32 / TANK_SHELLS as f32) < REFILL || me.armor / TANK_ARMOR < REFILL;
        let refilling = !self.out && short && held < REFILL_FOR;
        if held >= DWELL && !refilling {
            self.out = !self.out;
            self.arrived = None;
        }
        let word = if self.out && held < DWELL {
            "at the push point, looking round"
        } else if refilling {
            "at the defense point, refilling"
        } else {
            "at the defense point, looking round"
        };
        Leg { to, word: format!("rotation: {}", word) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Out, a look, home, a look, out: the tank never stops for long.
    #[test]
    fn the_tank_rotates_between_its_points() {
        let sc = Scenario { seed: 3, players: vec![("a".into(), true), ("b".into(), true)], ..Scenario::default() };
        let w = World::new(sc);
        let obs = w.observe(0);
        let me = obs.tanks.iter().find(|t| t.owner == 0).unwrap().clone();
        let base = me.pos;
        let mut r = Rotation::default();
        let push = r.push(&obs, base);
        let target = Rotation::objective(&obs, base);
        assert!((push.dist(base) - target.dist(base) * PUSH_FRAC).abs() < 0.5, "push {:?} base {:?} target {:?}", push, base, target);
        // At home and full: a look round, then out.
        let mut obs = obs;
        let leg = r.leg(&obs, &me, base, push);
        assert_eq!(leg.to, base);
        obs.tick += DWELL;
        let leg = r.leg(&obs, &me, base, push);
        assert!(r.out, "{}", leg.word);
        let leg = r.leg(&obs, &me, base, push);
        assert_eq!(leg.to, push, "{}", leg.word);
        // At the push point: a look round, then home.
        let mut there = me.clone();
        there.pos = push;
        obs.tick += 1;
        r.leg(&obs, &there, base, push);
        obs.tick += DWELL;
        r.leg(&obs, &there, base, push);
        assert!(!r.out);
        // Its own push point wins over the fleet's.
        r.own = Some(base + V2::new(5.0, 0.0));
        assert_eq!(r.push(&obs, base), base + V2::new(5.0, 0.0));
        // Short of shells at home: held to refill.
        let mut empty = me.clone();
        empty.shells = 0;
        r.out = false;
        obs.tick += 1;
        r.leg(&obs, &empty, base, push);
        obs.tick += DWELL;
        r.leg(&obs, &empty, base, push);
        assert!(!r.out, "left home empty");
    }

    /// The push point leans `front_w` of the way toward the front sector
    /// nearest it (LR 61); with no front, or at 0, it is as before.
    #[test]
    fn the_push_point_leans_to_the_front() {
        let sc = Scenario { seed: 3, players: vec![("a".into(), true), ("b".into(), true)], ..Scenario::default() };
        let w = World::new(sc);
        let obs = w.observe(0);
        let base = obs.tanks.iter().find(|t| t.owner == 0).unwrap().pos;
        let mut r = Rotation::default();
        let raw = r.fleet_push(&obs, base);
        let (near, far) = (raw + V2::new(0.0, 8.0), raw + V2::new(30.0, 0.0));
        r.front = vec![far, near];
        assert_eq!(r.fleet_push(&obs, base), raw, "at 0, as before");
        r.front_w = 0.75;
        let p = r.fleet_push(&obs, base);
        assert!(p.dist(raw + V2::new(0.0, 6.0)) < 0.01, "toward the nearest front sector: {:?} from {:?}", p, raw);
        r.front.clear();
        assert_eq!(r.fleet_push(&obs, base), raw, "no front, as before");
    }

    /// A wingman's push point: toward the objective, inside the leader's
    /// gathering ring, and back toward the leader off the water.
    #[test]
    fn a_wingman_pushes_inside_the_ring() {
        let sc = Scenario { seed: 3, players: vec![("a".into(), true), ("b".into(), true)], ..Scenario::default() };
        let w = World::new(sc);
        let obs = w.observe(0);
        let lp = obs.tanks.iter().find(|t| t.owner == 0).unwrap().pos;
        let r = Rotation::default();
        let p = r.wing_push(&obs, lp, |_| true);
        assert!((p.dist(lp) - WING_REACH).abs() < 0.01 || p.dist(lp) < WING_REACH, "{:?}", p);
        assert!(p.dist(lp) <= crate::commander::GATHER_R);
        let near = r.wing_push(&obs, lp, |q| q.dist(lp) <= 5.0);
        assert!(near.dist(lp) <= 5.0 && near.dist(lp) > ARRIVED, "{:?}", near);
        assert_eq!(r.wing_push(&obs, lp, |_| false), lp);
    }
}
