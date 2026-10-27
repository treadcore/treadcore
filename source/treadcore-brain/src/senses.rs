//! The senses (LR 95, 96): what a learned tank feels, as about a hundred
//! numbers from 0 to 1, the same on every map and every size of map.
//!
//! A compass of eight sectors round the tank, relative to its heading
//! (sector 0 ahead; with the map's y growing down the screen, sector k
//! turns k eighths of a turn clockwise: 2 is right, 4 behind, 6 left),
//! and in each sector eleven channels of signal strength: the strongest
//! thing of that kind there, `1 / (1 + d / 8)` for one d tiles off. Then
//! the tank's body; its instruments (where it is on the map, which way
//! it faces, the nearest edge, a hit just taken); a periscope's
//! contacts (LR 97: the nearest enemy, thing to take, depot of ours and
//! friend's call, each as an exact bearing and a strength, the enemy
//! with its closing speed and which way it points, as a submarine's
//! plot reads a contact); and the scripted brain's conclusions as hints.
//! Nothing here is the world's raw lists: a net that learns from these
//! learns what a tank in the field could feel. Every number is 0 to 1
//! but the contacts' bearings, a sine and a cosine, -1 to 1.

use treadcore_sim::map::{Map, Terrain};
use crate::calls::{Call, CALL_KINDS, CALL_LIFE};
use treadcore_sim::rules::{BAY_FULL, DIRECT_RANGE, PILLBOX_RANGE, PILLBOY_MATERIAL_MAX, RELOAD_TICKS, TANK_ARMOR, TANK_SHELLS, TANK_SPEED};
use treadcore_sim::*;

/// The compass's sectors.
pub const SECTORS: usize = 8;
/// The channels each sector reads (`Channel`).
pub const CHANNELS: usize = 11;
/// The body's numbers (`body`).
pub const BODY: usize = 12;
/// The instruments' numbers (`Nav`).
pub const NAV: usize = 6;
/// The periscope's numbers (`Contact`): the enemy's six, then three
/// each for the thing to take and our depot, and the call's six.
pub const CONTACTS: usize = 6 + 3 + 3 + 6;
/// The hints' numbers (`Hints`).
pub const HINTS: usize = 9;
/// Every sense: the compass, the body, the instruments, the periscope,
/// the hints.
pub const SENSES: usize = SECTORS * CHANNELS + BODY + NAV + CONTACTS + HINTS;
/// Where the instruments start.
pub const AT_NAV: usize = SECTORS * CHANNELS + BODY;
/// Where the periscope starts.
pub const AT_CONTACTS: usize = AT_NAV + NAV;
/// Where the hints start.
pub const AT_HINTS: usize = AT_CONTACTS + CONTACTS;
/// Ticks a hit is felt after it lands: 2 s.
pub const HURT_FOR: Tick = 2 * TICK_HZ as Tick;

/// The compass's channels, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    /// A hostile tank in sight, weaker when it is hurt.
    HostileTank,
    /// A teammate's tank.
    AlliedTank,
    /// A live hostile pillbox; 1 inside its reach.
    HostilePillbox,
    /// A neutral pillbox, to take.
    NeutralPillbox,
    /// One of ours, to keep.
    OurPillbox,
    /// A neutral or hostile depot, to take.
    DepotToTake,
    /// One of ours: shells and armour.
    OurDepot,
    /// A hostile shell landing within `SHELL_HORIZON`: stronger the
    /// sooner and the nearer.
    ShellComing,
    /// How soon the ground stops the tank that way: 1 at once, 0 at
    /// `GROUND_REACH` tiles or more.
    Ground,
    /// Forest near that way: cover.
    Cover,
    /// A friend's call from that way (calls.rs), weaker as it ages.
    Call,
}

/// The instruments, in order from `AT_NAV`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Nav {
    /// Across the map, 0 at the west edge, 1 at the east.
    X,
    /// Down the map, 0 at the north edge, 1 at the south.
    Y,
    /// The heading as a sine and a cosine, each shifted to 0 to 1.
    Sin,
    Cos,
    /// The nearest map edge's strength.
    Edge,
    /// A hit just taken: 1 as it lands, 0 `HURT_FOR` after.
    Hurt,
}

/// The periscope's numbers, in order from `AT_CONTACTS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    /// The nearest hostile tank: its bearing from the tank's heading as
    /// a sine (right is +) and a cosine (ahead is +), its strength,
    EnemySin,
    EnemyCos,
    Enemy,
    /// how fast it closes (0.5 still, 1 closing at a tank's top speed,
    /// 0 opening), how squarely it points at the tank (1 at it, 0 away),
    /// and 1 inside the direct gun's reach.
    EnemyClosing,
    EnemyAspect,
    EnemyInReach,
    /// The nearest neutral or hostile depot, or pillbox to take (neutral,
    /// or hostile and dead).
    TakeSin,
    TakeCos,
    Take,
    /// The nearest depot of ours: supply.
    HomeSin,
    HomeCos,
    Home,
    /// A friend's newest call, and its kind, one of three.
    CallSin,
    CallCos,
    Call,
    CallHelp,
    CallAttack,
    CallTake,
}

fn dot(a: V2, b: V2) -> f32 {
    a.x * b.x + a.y * b.y
}

/// A periscope bearing: the sine and cosine of `to`'s bearing from the
/// tank's heading, and its strength.
fn bearing(me: &Tank, to: V2) -> (f32, f32, f32) {
    let d = to - me.pos;
    let a = wrap_angle(d.heading() - me.heading);
    (a.sin(), a.cos(), strength(d.len()))
}

/// Ticks ahead a shell is felt: 4 s.
pub const SHELL_HORIZON: Tick = 4 * TICK_HZ as Tick;
/// Tiles the ground ray reads.
pub const GROUND_REACH: f32 = 16.0;
/// Tiles at which a signal is half its strength.
pub const HALF: f32 = 8.0;

/// The scripted brain's conclusions, for a net that has a brain beside
/// it (LR 95, B): hints, taken away later. Zero when absent.
#[derive(Clone, Copy, Debug, Default)]
pub struct Hints {
    /// The team's five posture scores (economy, fortify, connect,
    /// expand, defend), each 0 to 1.
    pub posture: [f32; 5],
    /// The odds here, ours against theirs, 0 to 1 (0.5 even).
    pub odds: f32,
    /// The base threatened, 0 to 1.
    pub threat: f32,
    /// Where the brain would send the tank.
    pub errand: Option<V2>,
}

/// A signal's strength from its distance.
pub fn strength(d: f32) -> f32 {
    1.0 / (1.0 + d.max(0.0) / HALF)
}

/// The sector a bearing falls in, relative to `heading`.
pub fn sector(heading: f32, bearing: f32) -> usize {
    let a = wrap_angle(bearing - heading);
    ((a / (std::f32::consts::TAU / SECTORS as f32)).round() as i32).rem_euclid(SECTORS as i32) as usize
}

/// The heading at the middle of a sector.
pub fn sector_heading(heading: f32, k: usize) -> f32 {
    wrap_angle(heading + k as f32 * std::f32::consts::TAU / SECTORS as f32)
}

/// The index of a sector's channel in the senses.
pub fn at(k: usize, c: Channel) -> usize {
    k * CHANNELS + c as usize
}

/// Everything the tank feels this tick, or None when it has no tank.
/// `calls` are the friends' calls the tank hears (calls.rs).
pub fn sense(obs: &Observation, map: Option<&Map>, hints: Option<&Hints>, calls: &[Call]) -> Option<[f32; SENSES]> {
    let me = obs.me.as_ref().filter(|t| t.alive)?;
    let mut s = [0.0f32; SENSES];
    let mut feel = |at_: V2, c: Channel, v: f32| {
        let d = at_ - me.pos;
        if d.len() < 1e-3 {
            return;
        }
        let i = at(sector(me.heading, d.heading()), c);
        if v > s[i] {
            s[i] = v;
        }
    };
    for t in obs.tanks.iter().filter(|t| t.alive && t.id != me.id) {
        let d = t.pos.dist(me.pos);
        if obs.allied(t.owner) {
            feel(t.pos, Channel::AlliedTank, strength(d));
        } else {
            feel(t.pos, Channel::HostileTank, strength(d) * (0.5 + 0.5 * (t.armor / TANK_ARMOR).clamp(0.0, 1.0)));
        }
    }
    for b in &obs.pillboxes {
        let d = b.pos.dist(me.pos);
        match b.owner {
            None => feel(b.pos, Channel::NeutralPillbox, strength(d)),
            Some(o) if obs.allied(o) => feel(b.pos, Channel::OurPillbox, strength(d)),
            Some(_) if b.alive() => feel(b.pos, Channel::HostilePillbox, if d <= PILLBOX_RANGE { 1.0 } else { strength(d) }),
            Some(_) => feel(b.pos, Channel::NeutralPillbox, strength(d)),
        }
    }
    for p in &obs.depots {
        let ours = p.owner.map(|o| obs.allied(o)).unwrap_or(false);
        feel(p.pos, if ours { Channel::OurDepot } else { Channel::DepotToTake }, strength(p.pos.dist(me.pos)));
    }
    for sh in obs.shells.iter().filter(|sh| !obs.allied(sh.owner)) {
        let left = sh.impact_tick.saturating_sub(obs.tick);
        if sh.impact_tick >= obs.tick && left <= SHELL_HORIZON {
            let soon = 1.0 - left as f32 / SHELL_HORIZON as f32;
            feel(sh.impact, Channel::ShellComing, soon * strength(sh.impact.dist(me.pos)));
        }
    }
    for f in &obs.forest {
        feel(*f, Channel::Cover, strength(f.dist(me.pos)));
    }
    let fresh = |c: &Call| 1.0 - obs.tick.saturating_sub(c.tick) as f32 / CALL_LIFE as f32;
    for c in calls {
        feel(c.at, Channel::Call, fresh(c) * strength(c.at.dist(me.pos)));
    }
    if let Some(m) = map {
        for k in 0..SECTORS {
            let dir = V2::from_heading(sector_heading(me.heading, k));
            let mut free = GROUND_REACH;
            let mut r = 0.5;
            while r <= GROUND_REACH {
                let p = me.pos + dir.scale(r);
                let open = m.in_bounds(p) && { let t = m.at(p); t.passable() || (me.boat && t == Terrain::Water) };
                if !open {
                    free = r - 0.5;
                    break;
                }
                r += 0.5;
            }
            s[at(k, Channel::Ground)] = 1.0 - free / GROUND_REACH;
        }
    }
    let b = SECTORS * CHANNELS;
    let bay = |n: u8, full: u8| if full == 0 { 0.0 } else { (n as f32 / full as f32).clamp(0.0, 1.0) };
    let body = [
        (me.armor / TANK_ARMOR).clamp(0.0, 1.0),
        (me.shells as f32 / TANK_SHELLS as f32).clamp(0.0, 1.0),
        me.fuel.clamp(0.0, 1.0),
        (me.reload as f32 / RELOAD_TICKS as f32).clamp(0.0, 1.0),
        (me.speed / TANK_SPEED).clamp(0.0, 1.0),
        if me.boat { 1.0 } else { 0.0 },
        if me.pillboys.deployed() { 1.0 } else { 0.0 },
        (me.pillboys.material as f32 / PILLBOY_MATERIAL_MAX as f32).clamp(0.0, 1.0),
        bay(me.bay.recon, BAY_FULL.recon),
        bay(me.bay.interceptor, BAY_FULL.interceptor),
        bay(me.bay.strike, BAY_FULL.strike),
        bay(me.bay.longshot, BAY_FULL.longshot),
    ];
    s[b..b + BODY].copy_from_slice(&body);
    let (w, h) = (obs.map_w.max(1) as f32, obs.map_h.max(1) as f32);
    let edge = me.pos.x.min(me.pos.y).min(w - me.pos.x).min(h - me.pos.y);
    let hurt = if me.last_hit_tick > 0 && obs.tick >= me.last_hit_tick && obs.tick < me.last_hit_tick + HURT_FOR { 1.0 - (obs.tick - me.last_hit_tick) as f32 / HURT_FOR as f32 } else { 0.0 };
    let nav = [(me.pos.x / w).clamp(0.0, 1.0), (me.pos.y / h).clamp(0.0, 1.0), 0.5 + 0.5 * me.heading.sin(), 0.5 + 0.5 * me.heading.cos(), strength(edge), hurt];
    s[AT_NAV..AT_NAV + NAV].copy_from_slice(&nav);
    let c = AT_CONTACTS;
    let nearest = |ps: &mut dyn Iterator<Item = V2>| ps.min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos)));
    if let Some(t) = obs.tanks.iter().filter(|t| t.alive && t.id != me.id && !obs.allied(t.owner)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))) {
        let (sn, cs, st) = bearing(me, t.pos);
        let to_me = (me.pos - t.pos).norm();
        let closing = dot(V2::from_heading(t.heading).scale(t.speed), to_me) - dot(V2::from_heading(me.heading).scale(me.speed), to_me);
        let aspect = dot(V2::from_heading(t.heading), to_me);
        s[c..c + 6].copy_from_slice(&[sn, cs, st, (0.5 + 0.5 * closing / TANK_SPEED).clamp(0.0, 1.0), 0.5 + 0.5 * aspect.clamp(-1.0, 1.0), if t.pos.dist(me.pos) <= DIRECT_RANGE { 1.0 } else { 0.0 }]);
    }
    let takes = obs.depots.iter().filter(|p| !p.owner.map(|o| obs.allied(o)).unwrap_or(false)).map(|p| p.pos).chain(obs.pillboxes.iter().filter(|b| b.owner.is_none() || (!b.alive() && !b.owner.map(|o| obs.allied(o)).unwrap_or(false))).map(|b| b.pos));
    if let Some(p) = nearest(&mut takes.into_iter()) {
        let (sn, cs, st) = bearing(me, p);
        s[c + 6..c + 9].copy_from_slice(&[sn, cs, st]);
    }
    if let Some(p) = nearest(&mut obs.depots.iter().filter(|p| p.owner.map(|o| obs.allied(o)).unwrap_or(false)).map(|p| p.pos)) {
        let (sn, cs, st) = bearing(me, p);
        s[c + 9..c + 12].copy_from_slice(&[sn, cs, st]);
    }
    if let Some(call) = calls.iter().max_by_key(|x| x.tick) {
        let (sn, cs, st) = bearing(me, call.at);
        s[c + 12..c + 15].copy_from_slice(&[sn, cs, st * fresh(call)]);
        s[c + 15 + CALL_KINDS.iter().position(|k| *k == call.kind).unwrap_or(0)] = 1.0;
    }
    if let Some(h) = hints {
        let k = AT_HINTS;
        s[k..k + 5].copy_from_slice(&h.posture.map(|v| v.clamp(0.0, 1.0)));
        s[k + 5] = h.odds.clamp(0.0, 1.0);
        s[k + 6] = h.threat.clamp(0.0, 1.0);
        if let Some(e) = h.errand {
            let d = e - me.pos;
            s[k + 7] = sector(me.heading, d.heading()) as f32 / (SECTORS - 1) as f32;
            s[k + 8] = strength(d.len());
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world() -> World {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        w.pillboxes.clear();
        w.depots.clear();
        w
    }

    /// The compass: a hostile tank set due right of a tank facing any
    /// way lights the right sector's hostile channel at its strength,
    /// and nothing else lights.
    #[test]
    fn a_tank_to_the_right_is_felt_to_the_right() {
        for h in 0..8 {
            let mut w = world();
            let heading = h as f32 * 0.7;
            w.tanks[0].heading = heading;
            let me = w.tanks[0].pos;
            // A quarter turn clockwise on the screen (y down) is +90 degrees.
            let at_ = me + V2::from_heading(heading + std::f32::consts::FRAC_PI_2).scale(6.0);
            w.tanks[1].pos = at_;
            w.tanks[1].armor = TANK_ARMOR;
            let obs = w.observe(0);
            let s = sense(&obs, None, None, &[]).expect("a tank");
            let i = at(2, Channel::HostileTank);
            assert!((s[i] - strength(6.0)).abs() < 1e-4, "heading {}: right sector {} not {}", heading, s[i], strength(6.0));
            for k in 0..SECTORS {
                if k != 2 {
                    assert_eq!(s[at(k, Channel::HostileTank)], 0.0, "heading {}: sector {} lit", heading, k);
                }
            }
        }
    }

    /// The body: armour, shells and reload read as shares; a dead tank
    /// feels nothing.
    #[test]
    fn the_body_reads_as_shares() {
        let mut w = world();
        w.tanks[0].armor = TANK_ARMOR * 0.25;
        w.tanks[0].shells = TANK_SHELLS / 2;
        w.tanks[0].reload = RELOAD_TICKS;
        let s = sense(&w.observe(0), None, None, &[]).expect("a tank");
        let b = SECTORS * CHANNELS;
        assert!((s[b] - 0.25).abs() < 1e-6 && (s[b + 1] - 0.5).abs() < 1e-6 && (s[b + 3] - 1.0).abs() < 1e-6, "{:?}", &s[b..b + BODY]);
        w.tanks[0].alive = false;
        assert!(sense(&w.observe(0), None, None, &[]).is_none());
    }

    /// The ground ray: open ground ahead reads 0; a wall 4 tiles ahead
    /// (its tile begins 3 to 4 tiles off, the ray's last open step half
    /// a tile short of it) reads between 1 - 3.5 / 16 and 1 - 2.5 / 16.
    #[test]
    fn the_ground_ray_finds_a_wall() {
        let mut w = world();
        w.tanks[0].heading = 0.0;
        let me = w.tanks[0].pos;
        for k in 1..=40 {
            let p = me + V2::new(k as f32 * 0.5, 0.0);
            if w.map.in_bounds(p) {
                w.map.set(p, Terrain::Grass);
            }
        }
        let open = sense(&w.observe(0), Some(&w.map), None, &[]).expect("a tank");
        assert_eq!(open[at(0, Channel::Ground)], 0.0, "open ahead");
        w.map.set(me + V2::new(4.0, 0.0), Terrain::Wall);
        let walled = sense(&w.observe(0), Some(&w.map), None, &[]).expect("a tank");
        let v = walled[at(0, Channel::Ground)];
        assert!((1.0 - 3.5 / GROUND_REACH..=1.0 - 2.5 / GROUND_REACH).contains(&v), "a wall 4 tiles ahead reads {}", v);
    }

    /// The periscope: an enemy dead right of the tank reads bearing
    /// sine 1 and cosine 0 at its strength, in reach at 6 tiles; driving
    /// at the tank it closes, pointing at it its aspect is 1; turned
    /// away, 0. The instruments place the tank on the map.
    #[test]
    fn the_periscope_reads_an_enemy_exactly() {
        let mut w = world();
        w.tanks[0].heading = 0.3;
        let me = w.tanks[0].pos;
        let right = 0.3 + std::f32::consts::FRAC_PI_2;
        w.tanks[1].pos = me + V2::from_heading(right).scale(6.0);
        w.tanks[1].heading = right + std::f32::consts::PI;
        w.tanks[1].speed = TANK_SPEED;
        let s = sense(&w.observe(0), None, None, &[]).expect("a tank");
        let c = |k: Contact| s[AT_CONTACTS + k as usize];
        assert!((c(Contact::EnemySin) - 1.0).abs() < 1e-3 && c(Contact::EnemyCos).abs() < 1e-3, "right: {} {}", c(Contact::EnemySin), c(Contact::EnemyCos));
        assert!((c(Contact::Enemy) - strength(6.0)).abs() < 1e-4);
        assert!((c(Contact::EnemyClosing) - 1.0).abs() < 1e-3, "closing {}", c(Contact::EnemyClosing));
        assert!((c(Contact::EnemyAspect) - 1.0).abs() < 1e-3 && c(Contact::EnemyInReach) == 1.0);
        w.tanks[1].heading = right;
        let s = sense(&w.observe(0), None, None, &[]).expect("a tank");
        assert!(s[AT_CONTACTS + Contact::EnemyAspect as usize] < 1e-3 && s[AT_CONTACTS + Contact::EnemyClosing as usize] < 1e-3);
        let n = |k: Nav| s[AT_NAV + k as usize];
        assert!((n(Nav::X) - me.x / w.map.w as f32).abs() < 1e-4 && (n(Nav::Y) - me.y / w.map.h as f32).abs() < 1e-4);
        assert!((n(Nav::Sin) - (0.5 + 0.5 * 0.3f32.sin())).abs() < 1e-5 && n(Nav::Hurt) == 0.0);
    }

    /// A friend's call: felt in its sector's Call channel and as the
    /// call contact, with its kind.
    #[test]
    fn a_friends_call_is_felt_where_it_points() {
        let mut w = world();
        w.tanks[0].heading = 0.0;
        let at_ = w.tanks[0].pos + V2::new(0.0, -8.0);
        let call = Call { from: 1, kind: crate::calls::CallKind::Take, at: at_, tick: 0 };
        let s = sense(&w.observe(0), None, None, &[call]).expect("a tank");
        assert!((s[at(6, Channel::Call)] - strength(8.0)).abs() < 1e-4, "left (north of an east-facing tank)");
        assert_eq!(s[AT_CONTACTS + Contact::CallTake as usize], 1.0);
        assert_eq!(s[AT_CONTACTS + Contact::CallHelp as usize], 0.0);
        assert!((s[AT_CONTACTS + Contact::CallSin as usize] + 1.0).abs() < 1e-3);
    }
}
