//! Bot styles (Paul, 1 November: "We want Bot 0-7 Styles. And naming ... right now we have All the
//! permutation of brainz, I want to be able to select them. In the future we will have more than our
//! A/B tested function Brinz, and the AI neural net").
//!
//! A STYLE IS WHAT A SEAT IS, BY NAME. The dials and switches describe 10^11 brains and offer no way
//! to ask for one. Here are eight, each a whole brain anybody can name, and put on a team: a team may
//! be one style, or several in turn, which is how two kinds of mind share a side.
//!
//! EVERY KIND ENDS IN THE SAME `Command`, so all of them can be read by one census and judged by one
//! simulator, and a new kind needs only to produce commands. There are four kinds today:
//!
//! - Rules: the Commander, with a set of dials. Five of the eight.
//! - Transform: the rules decide, and the transform may move their Move if it is sure (patch.rs).
//! - Net: a learned tank decides everything (net.rs). It cannot build or crew: it is 20 a seat behind.
//! - Duel: the rules decide, and crucible's net takes over the fight when it is sure. THIS IS THE BRIDGE.
//!
//! THE BRIDGE IS AN ADAPTER AND NOT A CLONE. brains.md planned to teach a treadcore net by imitating a
//! champion. But a hybrid champion is already an override with a gate, the transform's own design: it
//! reads the fight and the scripted tank's answer and gives a correction and a confidence. So this
//! builds its 35 inputs from what a tank here can see and plays it as it is, on the real simulator.
//! That is honest about the gap: the arena has one enemy, six pillars and no base, and a champion knows
//! nothing else, so it speaks only when a hostile tank is near and the rules keep everything else.
//!
//! A WORLD CHAMPION SEES WHAT THE GAME'S TANK SEES (4 November, LR 138; Paul: "yes do step 2, the
//! duelist with world senses"). Since 3 November crucible's world islands train under the game's
//! rules -- its tank, its gun, eyes of ten tiles, four neutral pillboxes -- with 55 inputs and two
//! hidden layers of 48. Such a champion is read by its size, and its twenty more inputs are built here
//! as crucible.cu's `sense` builds them: the rays to live pillboxes, the nearest pillbox's bearing,
//! reach, reload, health and whether it has the tank, and the tank's own shells and reload. Its turn is
//! a heading error and its throttle has no reverse, as crucible's world driver. It plays as Gladiator,
//! slot 17, beside the hybrid Duelist, so a census reads the two on one binary and the same seeds.
//!
//! SHIPPED, NOT LOCATED. The files are in the binary: a package has no fleet to find them on. Nothing
//! here moves a default; style 0 is the brain exactly as it was.

use crate::fleet::with_settings;
use crate::net::{Net, NetBrain};
use crate::weights::{Settings, W};
use crate::Brain;
use treadcore_sim::rules::{
    DIRECT_HIT_RADIUS, DIRECT_RANGE, PILLBOX_HEALTH, PILLBOX_RANGE, PILLBOX_RELOAD_TICKS, RELOAD_TICKS, TANK_ARMOR, TANK_SHELLS, TANK_SPEED,
    TANK_TURN, TANK_VISION,
};
use treadcore_sim::{wrap_angle, Command, Map, Observation, ShellKind, Tank, Terrain, V2};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Kind {
    /// The rules, with these dials over the defaults; empty is the fleet's own.
    Rules(&'static str),
    /// The rules, with this transform on.
    Transform(&'static str),
    /// A learned tank.
    Net(&'static [u8]),
    /// The rules, with crucible's champion in the fight.
    Duel(&'static [u8]),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub slot: usize,
    pub name: &'static str,
    pub about: &'static str,
    pub kind: Kind,
}

/// The roster. Its order is the `bot_style` dial's, so a slot once given keeps its meaning.
///
/// SLOTS 8 TO 15 ARE PERSONAS (Paul, 2 November: "use the topography of local maxima to differentiate
/// different named personas, perhaps use the civilizations of Civ V ... according to their strengths").
/// A persona is a short weights file over today's defaults, named after a culture for the strength it
/// plays: it stands on a hill of the terrain or on a style's census read where there is one, and says
/// "unread" where there is not. Every dial it does not name rides the line, so a persona stays fresh
/// as the fleet moves the defaults. Its certificate is its own census (the plan, chapter 10).
pub const ROSTER: [Style; 18] = [
    Style { slot: 0, name: "Fleet", about: "the brain the fleet's search has carried: today's defaults", kind: Kind::Rules("") },
    Style { slot: 1, name: "Raider", about: "goes and fights: leans to expanding, closes at even odds, picks off the wounded", kind: Kind::Rules(include_str!("../../../profiles/raider.weights")) },
    Style { slot: 2, name: "Castle", about: "holds its ground: leans to fortifying and defending", kind: Kind::Rules(include_str!("../../../profiles/castle.weights")) },
    Style { slot: 3, name: "Expander", about: "takes the map: goes for depots and pillboxes first", kind: Kind::Rules(include_str!("../../../profiles/expander.weights")) },
    Style { slot: 4, name: "Timid", about: "keeps away: fights only with the odds well on its side", kind: Kind::Rules(include_str!("../../../profiles/timid.weights")) },
    Style { slot: 5, name: "Overrider", about: "the rules, with the learned transform moving their answer when it is sure", kind: Kind::Transform("patch = 1\n") },
    Style { slot: 6, name: "Scholar", about: "a net that learned by imitating the rules: all senses, no rules", kind: Kind::Net(include_bytes!("../../../nets/bc2.bin")) },
    Style { slot: 7, name: "Duelist", about: "the rules, with a champion from crucible's GPU islands in every fight", kind: Kind::Duel(include_bytes!("../../../nets/duelist.bin")) },
    Style { slot: 8, name: "Rome", about: "roads, posts and legions: every tank runs the base cycle, the guard keeps building, the roads between bases come first", kind: Kind::Rules(include_str!("../../../profiles/persona-rome.weights")) },
    Style { slot: 9, name: "China", about: "the Great Wall: a walled castle, mended first, defended hard", kind: Kind::Rules(include_str!("../../../profiles/persona-china.weights")) },
    Style { slot: 10, name: "Mongolia", about: "the horde: raids, presses at even odds, picks off the wounded, ducks a quarter as often", kind: Kind::Rules(include_str!("../../../profiles/persona-mongolia.weights")) },
    Style { slot: 11, name: "Shoshone", about: "territory: takes the map, depots and pillboxes first, the front forward", kind: Kind::Rules(include_str!("../../../profiles/persona-shoshone.weights")) },
    Style { slot: 12, name: "Venice", about: "trade and treasure: the economy first, no expansion, stocks before fights, a moat of mines", kind: Kind::Rules(include_str!("../../../profiles/persona-venice.weights")) },
    Style { slot: 13, name: "Japan", about: "bushido: fights at full strength however hurt, and does not go home", kind: Kind::Rules(include_str!("../../../profiles/persona-japan.weights")) },
    Style { slot: 14, name: "Polynesia", about: "the sea: island hops, boats and beaches, a navy's patience", kind: Kind::Rules(include_str!("../../../profiles/persona-polynesia.weights")) },
    Style { slot: 15, name: "Assyria", about: "siege: shells every pillbox in reach, longshots without pause, strikes out in threes", kind: Kind::Rules(include_str!("../../../profiles/persona-assyria.weights")) },
    // Slot 16 (3 November, LR 131, 132): the first learned tank with its squad, so a census can read
    // what a trained squad head does to a side's ore and smelted lines among sixteen seats.
    Style { slot: 16, name: "Quartermaster", about: "a learned tank with its squad: trained on the logistics courses, it mines, pumps and raids convoys", kind: Kind::Net(include_bytes!("../../../nets/logistics.bin")) },
    // Slot 17 (4 November, LR 138): the Duelist's bridge for a champion of crucible's WORLD islands,
    // trained under the game's own rules and pillboxes; style 7 keeps the hybrid it has always played.
    Style { slot: 17, name: "Gladiator", about: "the rules, with a champion of crucible's world islands in every fight: it trained under the game's rules, pillboxes and all", kind: Kind::Duel(include_bytes!("../../../nets/gladiator.bin")) },
];

/// A style by its number or name, in any case.
pub fn find(word: &str) -> Option<&'static Style> {
    let w = word.trim();
    match w.parse::<usize>() {
        Ok(k) => ROSTER.get(k),
        Err(_) => ROSTER.iter().find(|s| s.name.eq_ignore_ascii_case(w)),
    }
}

/// A team's styles from "raider" or "1,2,6"; each seat takes the next in turn.
pub fn parse_list(words: &str) -> Result<Vec<&'static Style>, String> {
    let list: Result<Vec<_>, String> = words.split(',').map(|w| find(w).ok_or_else(|| format!("no bot style {:?}: {}", w.trim(), names()))).collect();
    list.and_then(|l| if l.is_empty() { Err("no bot style given".to_string()) } else { Ok(l) })
}

/// "0 Fleet, 1 Raider, ...": what a refusal offers.
pub fn names() -> String {
    ROSTER.iter().map(|s| format!("{} {}", s.slot, s.name)).collect::<Vec<_>>().join(", ")
}

/// A style's dials over `base`. The rules' styles replace the search's dials with their own, and keep
/// the seat's skill, which is the person's choice and no part of a style. Slot 0 leaves `base` be, so
/// a `--weights` file still means what it did.
pub fn settings_for(style: &Style, base: &Settings) -> Settings {
    let text = match style.kind {
        Kind::Rules(t) | Kind::Transform(t) => t,
        _ => "",
    };
    if text.is_empty() {
        return base.clone();
    }
    let mut s = Settings::parse(text).unwrap_or_else(|e| panic!("style {}: {:?}", style.name, e));
    if let Kind::Transform(_) = style.kind {
        // On top of today's brain: a transform moves what those dials decide.
        let mut b = base.clone();
        b.w[W::Patch] = s.w[W::Patch];
        return b;
    }
    s.w[W::Skill] = base.w[W::Skill];
    s
}

/// A seat's brain in this style. `rules` makes the seat's Commander as it would be without one, which
/// knows its team, its role and its name; a net has no need of it.
pub fn make(style: &Style, base: &Settings, name: &str, rules: impl FnOnce() -> Box<dyn Brain>) -> Box<dyn Brain> {
    match style.kind {
        Kind::Rules(_) | Kind::Transform(_) => with_settings(settings_for(style, base), rules),
        Kind::Net(bytes) => Box::new(NetBrain::new(scholar_file(style).unwrap_or_else(|| Net::from_bytes(bytes).expect("a shipped net reads")), name)),
        Kind::Duel(bytes) => {
            let champion = match gladiator_file(style) {
                Some(c) => c,
                None => Champion::from_bytes(bytes).expect("a shipped champion reads"),
            };
            Box::new(Duelist::new(champion, with_settings(base.clone(), rules)))
        }
    }
}

/// THE FLEET'S READ OF A CLONE NOT YET SHIPPED (LR 140): `SCHOLAR=path` makes slot 6 play that net
/// instead of the shipped one, as `GLADIATOR` does for slot 17. orrery's clone stage judges a clone of
/// the rules as one seat beside them (`bot_style = 6`, `bot_mix = one`; Paul, 4 November: "One seat
/// beside the rules"), which `CENSUS_NET`, a whole team of nets, cannot say. A file that is not a net
/// stops the match with a sentence.
fn scholar_file(style: &Style) -> Option<Net> {
    if style.slot != 6 {
        return None;
    }
    let path = std::env::var("SCHOLAR").ok().filter(|p| !p.is_empty())?;
    Some(Net::load(std::path::Path::new(&path)).unwrap_or_else(|e| panic!("SCHOLAR={}: {}", path, e)))
}

/// THE FLEET'S READ OF A CHAMPION NOT YET SHIPPED (LR 138): `GLADIATOR=path` makes slot 17 play that
/// world champion instead of the shipped one, as `CENSUS_WEIGHTS` makes a census play dials not yet
/// promoted. orrery's bridge reads each new milestone of crucible's world islands so, against the
/// shipped one on the same seeds, before anyone ships it. A file that is not a world champion stops
/// the match with a sentence: a census on the wrong net would be read as the right one.
fn gladiator_file(style: &Style) -> Option<Champion> {
    if style.slot != 17 {
        return None;
    }
    let path = std::env::var("GLADIATOR").ok().filter(|p| !p.is_empty())?;
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("GLADIATOR={}: {}", path, e));
    let c = Champion::from_bytes(&bytes).unwrap_or_else(|e| panic!("GLADIATOR={}: {}", path, e));
    assert!(c.world(), "GLADIATOR={}: a hybrid champion, not a world one: style 7 plays those", path);
    Some(c)
}

// ---- the bridge -----------------------------------------------------------------------------

const NIN: usize = 35;
const H: usize = 32;
const NOUT: usize = 4;
/// crucible.cu's NW for a hybrid build. A champion is exactly this many floats, and nothing else.
pub const CHAMPION_WEIGHTS: usize = NIN * H + H + H * H + H + H * NOUT + NOUT;
/// A world hybrid's: 55 in (the 35 and twenty of the world's), 48, 48, 4 out (make WORLD=1 HYBRID=1 H=48).
const W_NIN: usize = 55;
const W_H: usize = 48;
pub const WORLD_CHAMPION_WEIGHTS: usize = W_NIN * W_H + W_H + W_H * W_H + W_H + W_H * NOUT + NOUT;
/// A maze champion's (crucible's NAV, LR 152): the world's 55 and nine of the way, 48, 48, and 6 out, the
/// last two the numbers it keeps (make WORLD=1 HYBRID=1 H=48 FOREST=1 NAV=1).
const N_NIN: usize = 64;
const N_NOUT: usize = 6;
pub const NAV_CHAMPION_WEIGHTS: usize = N_NIN * W_H + W_H + W_H * W_H + W_H + W_H * N_NOUT + N_NOUT;
/// Where crucible.cu's maze inputs begin (its AT_NAV), and how many numbers its net keeps (its MEM).
const AT_NAV: usize = W_NIN;
const MEM: usize = 2;
/// Where crucible.cu's world inputs begin (its AT_WORLD with the hybrid's three).
const AT_WORLD: usize = 35;
/// crucible.cu: how far its rays and its senses reach, and its scripted yardstick's range and aim.
const RANGE: f32 = 40.0;
const YARD_RANGE: f32 = 11.0;
const YARD_TOL: f32 = 0.12;
/// The world's yardstick stands inside the gun's ten, as a treadcore bot stands off.
const W_YARD_RANGE: f32 = 8.0;
const RAYS: usize = 12;
const TANK_R: f32 = 0.75;
/// crucible.cu's TANK_R, the world's rays meeting a tank; its pillbox is struck this near (PB_HIT).
const W_TANK_R: f32 = 1.0;
const PB_HIT: f32 = DIRECT_HIT_RADIUS + 0.5;
/// Five times a second, as a NetBrain: the arena's step is a tenth, and a Move takes time to bite.
const DECIDE_EVERY: u64 = treadcore_sim::TICK_HZ as u64 / 5;
/// How far a full throttle reaches at each decision, and how far a full turn swings it.
const STRIDE: f32 = 6.0;
const SWING: f32 = 1.0;

/// A champion from crucible, tanh throughout, as crucible.cu's `think`: a hybrid (35 in, 32, 32, 4
/// out), a world hybrid (55, 48, 48, 4) or a maze hybrid (64, 48, 48, 6), told apart by its size. A champion without the gate (a
/// plain build's 3 outputs) is not a Duelist's: it would take every fight, and is refused.
#[derive(Debug, Clone, PartialEq)]
pub struct Champion {
    nin: usize,
    h: usize,
    w: Vec<f32>,
}

impl Champion {
    /// A champion as crucible exports it, bare floats with no header: its kind is its size.
    pub fn from_bytes(bytes: &[u8]) -> Result<Champion, String> {
        let (nin, h) = match bytes.len() / 4 {
            n if n * 4 != bytes.len() => (0, 0),
            CHAMPION_WEIGHTS => (NIN, H),
            WORLD_CHAMPION_WEIGHTS => (W_NIN, W_H),
            NAV_CHAMPION_WEIGHTS => (N_NIN, W_H),
            _ => (0, 0),
        };
        if nin == 0 {
            return Err(format!(
                "a hybrid champion is {} bytes, a world hybrid {} and a maze hybrid {}, this is {}",
                CHAMPION_WEIGHTS * 4,
                WORLD_CHAMPION_WEIGHTS * 4,
                NAV_CHAMPION_WEIGHTS * 4,
                bytes.len()
            ));
        }
        Ok(Champion { nin, h, w: bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect() })
    }

    /// A world island's champion: it reads the twenty world inputs and drives as the world's tank (a
    /// maze champion is one too, with nine inputs more).
    pub fn world(&self) -> bool {
        self.nin >= W_NIN
    }

    /// A maze champion (crucible's NAV): it reads the way to its enemy and keeps two numbers.
    pub fn nav(&self) -> bool {
        self.nin == N_NIN
    }

    /// Its outputs: four (turn, throttle, fire, the gate), and a maze champion's two kept numbers.
    fn nout(&self) -> usize {
        let h = self.h;
        (self.w.len() - self.nin * h - h - h * h - h) / (h + 1)
    }

    /// crucible.cu's `think`: the inputs through two hidden layers to its outputs, tanh throughout. Six
    /// numbers always; a champion of four outputs leaves the last two nothing.
    pub fn think(&self, x: &[f32]) -> [f32; N_NOUT] {
        let (w, nin, h, nout) = (&self.w, self.nin, self.h, self.nout());
        let (b1, w2) = (nin * h, nin * h + h);
        let (b2, w3) = (w2 + h * h, w2 + h * h + h);
        let b3 = w3 + h * nout;
        let h1: Vec<f32> = (0..h).map(|j| (w[b1 + j] + (0..nin).map(|i| w[j * nin + i] * x[i]).sum::<f32>()).tanh()).collect();
        let h2: Vec<f32> = (0..h).map(|j| (w[b2 + j] + (0..h).map(|i| w[w2 + j * h + i] * h1[i]).sum::<f32>()).tanh()).collect();
        let mut out = [0f32; N_NOUT];
        for k in 0..nout {
            let s: f32 = w[b3 + k] + (0..h).map(|i| w[w3 + k * h + i] * h2[i]).sum::<f32>();
            out[k] = s.tanh();
        }
        out
    }
}

/// How far along a ray before something blocks a tank, in tiles; RANGE if nothing does, or no map.
fn ray_wall(map: Option<&Map>, from: V2, dir: V2) -> f32 {
    let Some(m) = map else { return RANGE };
    let mut r = 0.5;
    while r < RANGE {
        let p = from + dir.scale(r);
        if !m.in_bounds(p) || (!m.at(p).passable() && m.at(p) != Terrain::Water) {
            return r;
        }
        r += 0.5;
    }
    RANGE
}

/// Where a ray from `from` first meets a circle.
fn ray_circle(from: V2, dir: V2, at: V2, radius: f32) -> f32 {
    let to = at - from;
    let along = to.x * dir.x + to.y * dir.y;
    let off2 = (to.x * to.x + to.y * to.y) - along * along;
    if along <= 0.0 || off2 > radius * radius {
        return f32::MAX;
    }
    along - (radius * radius - off2).sqrt()
}

/// crucible's `scripted` at its yardstick: turn to the enemy, close to range, fire when aimed.
fn scripted(bearing: f32, d: f32) -> [f32; 3] {
    [
        (2.0 * bearing).clamp(-1.0, 1.0),
        if d > YARD_RANGE + 3.0 { 1.0 } else if d < YARD_RANGE - 3.0 { -0.5 } else { 0.0 },
        if bearing.abs() < YARD_TOL && d < 35.0 { 1.0 } else { -1.0 },
    ]
}

/// crucible's `sense` and the hybrid's three inputs, from what a tank here can see.
pub fn senses(obs: &Observation, map: Option<&Map>, me: &Tank, en: &Tank) -> [f32; NIN] {
    let mut x = [0f32; NIN];
    for r in 0..RAYS {
        let dir = V2::from_heading(me.heading + r as f32 * std::f32::consts::TAU / RAYS as f32);
        let tw = ray_wall(map, me.pos, dir);
        let te = ray_circle(me.pos, dir, en.pos, TANK_R);
        let near = tw.min(te).min(RANGE);
        x[2 * r] = 1.0 - near / RANGE;
        x[2 * r + 1] = if te < tw && te < RANGE { 1.0 - te / RANGE } else { 0.0 };
    }
    let b = wrap_angle((en.pos - me.pos).heading() - me.heading);
    let d = me.pos.dist(en.pos);
    x[24] = me.speed / TANK_SPEED;
    x[25] = me.armor / TANK_ARMOR;
    x[26] = if me.reload == 0 { 1.0 } else { 0.0 };
    x[27] = en.armor / TANK_ARMOR;
    x[28] = b.sin();
    x[29] = b.cos();
    let shell = obs.shells.iter().filter(|s| !obs.allied(s.owner)).map(|s| s.impact.dist(me.pos)).fold(f32::MAX, f32::min);
    x[30] = if shell < 10.0 { 1.0 - shell / 10.0 } else { 0.0 };
    x[31] = 1.0;
    let rules = scripted(b, d);
    x[32..35].copy_from_slice(&rules);
    x
}

/// crucible's `scripted` at the WORLD yardstick: the turn is the bearing itself as a heading error
/// (-1..1 is -pi..pi), it stands off inside the gun's reach, and it fires when aimed within the gun's.
fn scripted_world(bearing: f32, d: f32) -> [f32; 3] {
    [
        (bearing / std::f32::consts::PI).clamp(-1.0, 1.0),
        if d > W_YARD_RANGE + 3.0 { 1.0 } else if d < W_YARD_RANGE - 3.0 { -0.5 } else { 0.0 },
        if bearing.abs() < YARD_TOL && d < DIRECT_RANGE { 1.0 } else { -1.0 },
    ]
}

/// Where a shell is at this tick: along its flight, from launch to impact.
fn shell_at(s: &treadcore_sim::Shell, tick: treadcore_sim::Tick) -> V2 {
    let span = s.impact_tick.saturating_sub(s.launch_tick).max(1) as f32;
    let f = (tick.saturating_sub(s.launch_tick) as f32 / span).clamp(0.0, 1.0);
    s.from + (s.impact - s.from).scale(f)
}

/// crucible's `sense` under WORLD, and the hybrid's three: 55 inputs from what a tank here can see.
/// The thirty-two of the arena, as the hybrid's but that a tank further than TANK_VISION is not seen
/// on any ray (its bearing and armour still are, as crucible gives them); the yardstick at the world's
/// range; then from AT_WORLD the twenty: twelve rays to a live pillbox that would fire on this tank (a
/// neutral one or an enemy's), the nearest such pillbox -- its bearing as a sine and a cosine, its
/// reach, its reload and health as shares, and 1 when it has the tank (in its range, a clear line) --
/// and the tank's shells and reload as shares.
pub fn world_senses(obs: &Observation, map: Option<&Map>, me: &Tank, en: &Tank) -> [f32; W_NIN] {
    let mut x = [0f32; W_NIN];
    let threat: Vec<&treadcore_sim::Pillbox> =
        obs.pillboxes.iter().filter(|p| p.health > 0.0 && !p.owner.map(|o| obs.allied(o)).unwrap_or(false)).collect();
    for r in 0..RAYS {
        let dir = V2::from_heading(me.heading + r as f32 * std::f32::consts::TAU / RAYS as f32);
        let tw = ray_wall(map, me.pos, dir);
        let mut te = ray_circle(me.pos, dir, en.pos, W_TANK_R);
        if te > TANK_VISION {
            te = f32::MAX;
        }
        let tp = threat.iter().map(|p| ray_circle(me.pos, dir, p.pos, PB_HIT)).fold(f32::MAX, f32::min);
        x[AT_WORLD + r] = if tp < tw && tp < RANGE { 1.0 - tp / RANGE } else { 0.0 };
        let near = tw.min(te).min(RANGE);
        x[2 * r] = 1.0 - near / RANGE;
        x[2 * r + 1] = if te < tw && te < RANGE { 1.0 - te / RANGE } else { 0.0 };
    }
    let b = wrap_angle((en.pos - me.pos).heading() - me.heading);
    let d = me.pos.dist(en.pos);
    x[24] = me.speed / TANK_SPEED;
    x[25] = me.armor / TANK_ARMOR;
    x[26] = if me.reload == 0 { 1.0 } else { 0.0 };
    x[27] = en.armor / TANK_ARMOR;
    x[28] = b.sin();
    x[29] = b.cos();
    let shell = obs
        .shells
        .iter()
        .filter(|s| s.kind == ShellKind::Direct && !obs.allied(s.owner))
        .map(|s| shell_at(s, obs.tick).dist(me.pos))
        .fold(f32::MAX, f32::min);
    x[30] = if shell < 10.0 { 1.0 - shell / 10.0 } else { 0.0 };
    x[31] = 1.0;
    x[32..35].copy_from_slice(&scripted_world(b, d));
    let o = AT_WORLD + RAYS;
    if let Some(p) = threat.iter().min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))) {
        let pb = wrap_angle((p.pos - me.pos).heading() - me.heading);
        let nd = p.pos.dist(me.pos);
        x[o] = pb.sin();
        x[o + 1] = pb.cos();
        x[o + 2] = (1.0 - nd / RANGE).max(0.0);
        x[o + 3] = p.reload as f32 / PILLBOX_RELOAD_TICKS as f32;
        x[o + 4] = p.health / PILLBOX_HEALTH;
        x[o + 5] = if nd <= PILLBOX_RANGE && map.map(|m| m.line_clear(p.pos, me.pos)).unwrap_or(true) { 1.0 } else { 0.0 };
    }
    x[o + 6] = me.shells as f32 / TANK_SHELLS as f32;
    x[o + 7] = me.reload as f32 / RELOAD_TICKS as f32;
    x
}

/// The way to a goal, as crucible's route sense gives it (`route_from`): the next point to drive to
/// (the goal itself when the line to it is clear, or there is no way at all), the way's length in
/// tiles, whether the goal is walled off, and whether the way is the straight line.
struct Way {
    to: V2,
    len: f32,
    walled: bool,
    straight: bool,
}

/// The map's own route (`route_kept`, the planner the rules drive by) where crucible searches a grid:
/// the first of its points more than a tile off, and the length along all of them. A clear line is the
/// way itself, as crucible's `route_from` has it (a grove across the line is gone round, not through).
fn way_to(map: Option<&Map>, from: V2, goal: V2) -> Way {
    let line = Way { to: goal, len: from.dist(goal), walled: false, straight: true };
    let Some(m) = map else { return line };
    if m.line_clear(from, goal) {
        return line;
    }
    let r = m.route_kept(from, goal, false);
    if r.is_empty() {
        return Way { to: goal, len: RANGE, walled: true, straight: true };
    }
    let (mut len, mut at) = (0.0, from);
    for p in &r {
        len += at.dist(*p);
        at = *p;
    }
    let to = r.iter().copied().find(|p| p.dist(from) > 1.0).unwrap_or(goal);
    Way { to, len, walled: false, straight: false }
}

/// Where a point is no place for a tank: off the map, or a tile it cannot drive (crucible's `no_room`,
/// whose walls stand a tank's radius proud: the four corners of that square are tried as well).
fn no_room(m: &Map, p: V2) -> bool {
    let r = W_TANK_R * 0.95;
    [V2::new(0.0, 0.0), V2::new(r, r), V2::new(-r, r), V2::new(r, -r), V2::new(-r, -r)]
        .iter()
        .any(|d| !m.in_bounds(p + *d) || !m.at(p + *d).passable())
}

/// ITS OWN FUTURE, PROJECTED (crucible's `arc_free`): the share of 1.5 s the tank could drive along an
/// arc turning at `k` of half its turn rate before it meets something it cannot drive through.
fn arc_free(map: Option<&Map>, me: &Tank, k: f32) -> f32 {
    let Some(m) = map else { return 1.0 };
    let (v, om) = (me.speed.max(0.5 * TANK_SPEED), k * 0.5 * TANK_TURN);
    let (mut p, mut a) = (me.pos, me.heading);
    for i in 1..=6 {
        a += om * 0.25;
        p = p + V2::from_heading(a).scale(v * 0.25);
        if no_room(m, p) {
            return (i - 1) as f32 / 6.0;
        }
    }
    1.0
}

/// A maze champion's 64 (crucible's `sense` under NAV, LR 152): the world's 55, then from AT_NAV the
/// bearing to the way's next point as a sine and a cosine, the way's length as a share of RANGE, 1 when
/// the enemy is walled off, how far the tank could drive along three arcs (left, ahead, right), and the
/// two numbers its net kept. And THE ERRAND (crucible's `errand`): the rules' answer it reads (32 and
/// 33) turns toward the way's next point at full throttle when the straight line to the enemy is shut,
/// as the game's Commander goes round a wall by its route. Where the line is clear it is the script.
pub fn nav_senses(obs: &Observation, map: Option<&Map>, me: &Tank, en: &Tank, mem: [f32; MEM]) -> [f32; N_NIN] {
    let mut x = [0f32; N_NIN];
    x[..W_NIN].copy_from_slice(&world_senses(obs, map, me, en));
    let way = way_to(map, me.pos, en.pos);
    let rb = wrap_angle((way.to - me.pos).heading() - me.heading);
    x[AT_NAV] = rb.sin();
    x[AT_NAV + 1] = rb.cos();
    x[AT_NAV + 2] = (way.len / RANGE).min(1.0);
    x[AT_NAV + 3] = if way.walled { 1.0 } else { 0.0 };
    for (i, k) in [-1.0, 0.0, 1.0].iter().enumerate() {
        x[AT_NAV + 4 + i] = arc_free(map, me, *k);
    }
    x[AT_NAV + 7..AT_NAV + 7 + MEM].copy_from_slice(&mem);
    if !way.straight {
        x[32] = (rb / std::f32::consts::PI).clamp(-1.0, 1.0);
        x[33] = 1.0;
    }
    x
}

/// The rules, with a champion in the fight. THE GATE IS THE CHAMPION'S OWN: shut, this seat is the
/// Commander to the last command, as the arena's hybrid tank plays its script.
pub struct Duelist {
    champion: Champion,
    rules: Box<dyn Brain>,
    open: bool,
    /// The rules' last order to go somewhere. THEY SAY IT ONCE, and trust the route: a duel that
    /// replaced it would leave the tank where the duel ended, and the rules believing it on its way.
    errand: Option<Command>,
    /// A maze champion's two kept numbers (crucible's MEM): what its net wrote at its last decision,
    /// read back at the next. A duel's begin at nothing, as each of crucible's runs does.
    mem: [f32; MEM],
}

impl Duelist {
    pub fn new(champion: Champion, rules: Box<dyn Brain>) -> Duelist {
        Duelist { champion, rules, open: false, errand: None, mem: [0.0; MEM] }
    }

    /// THE GATE IS A DIAL (LR 140; Paul, 4 November, of the Gladiator's gap to the defaults: "The gate
    /// as a dial"): the champion's confidence must pass `duel_gate`, 0 being its own gate as trained. A
    /// higher gate keeps it to the fights it is surest of and leaves the rest to the rules; the census
    /// reads where the line is. The rules' settings carry it, as every dial of a seat.
    fn gate(&self) -> f32 {
        self.rules.settings().map(|s| s.w[W::DuelGate]).unwrap_or(0.0)
    }

    /// Its say this decision, if the gate opens: what to do instead of the rules' moving and firing.
    fn duel(&mut self, obs: &Observation, map: Option<&Map>) -> Option<Vec<Command>> {
        let foe = obs.me.as_ref().filter(|t| t.alive).and_then(|me| {
            obs.hostile_tanks()
                .filter(|t| t.alive)
                .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))
                .filter(|en| me.pos.dist(en.pos) < RANGE)
                .map(|en| (me, en))
        });
        let Some((me, en)) = foe else {
            self.mem = [0.0; MEM];
            return None;
        };
        if self.champion.world() {
            // crucible's world driver: the turn names a heading error (-1..1 is -pi..pi) and the
            // throttle has no reverse, so a throttle at or under nothing holds the tank where it is.
            let x: Vec<f32> = if self.champion.nav() { nav_senses(obs, map, me, en, self.mem).to_vec() } else { world_senses(obs, map, me, en).to_vec() };
            let y = self.champion.think(&x);
            if self.champion.nav() {
                self.mem = [y[4], y[5]];
            }
            if y[3] <= self.gate() {
                return None;
            }
            let turn = (x[32] + y[0]).clamp(-1.0, 1.0);
            let go = (x[33] + y[1]).clamp(0.0, 1.0);
            if go < 0.05 {
                return Some(vec![Command::Stop]);
            }
            let dir = V2::from_heading(me.heading + turn * std::f32::consts::PI);
            return Some(vec![Command::Move { to: obs.clamp(me.pos + dir.scale(go * STRIDE)) }]);
        }
        let x = senses(obs, map, me, en);
        let y = self.champion.think(&x);
        if y[3] <= self.gate() {
            return None;
        }
        let act: Vec<f32> = (0..2).map(|i| (x[32 + i] + y[i]).clamp(-1.0, 1.0)).collect();
        let mut cmds = Vec::new();
        let dir = V2::from_heading(me.heading + act[0] * SWING);
        cmds.push(Command::Move { to: obs.clamp(me.pos + dir.scale(act[1] * STRIDE)) });
        Some(cmds)
    }
}

/// What the rules said that the duel now says instead: where to be. THE GUN STAYS THE RULES'. Crucible's
/// tank fires along its hull, so its fire output means "my hull is on him", and read as a turret's
/// trigger it starved the tank of shots: -17.8 a seat over 24 seeds, -14.9 with the rules aiming.
fn fight_order(c: &Command) -> bool {
    matches!(c, Command::Move { .. } | Command::Stop)
}

impl Brain for Duelist {
    fn name(&self) -> &str {
        self.rules.name()
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        self.think_on(obs, None)
    }

    fn think_on(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        let mut cmds = self.rules.think_on(obs, map);
        if let Some(m) = cmds.iter().rev().find(|c| matches!(c, Command::Move { .. })) {
            self.errand = Some(m.clone());
        }
        if (obs.tick as u64) % DECIDE_EVERY != 0 {
            if self.open {
                cmds.retain(|c| !fight_order(c));
            }
            return cmds;
        }
        match self.duel(obs, map) {
            Some(mine) => {
                self.open = true;
                cmds.retain(|c| !fight_order(c));
                cmds.extend(mine);
            }
            None => {
                // The gate shuts: back to what the rules were doing, as if never parted.
                if std::mem::replace(&mut self.open, false) && !cmds.iter().any(|c| matches!(c, Command::Move { .. })) {
                    cmds.extend(self.errand.clone());
                }
            }
        }
        cmds
    }

    fn status(&self) -> String {
        if self.open { format!("duelling; {}", self.rules.status()) } else { self.rules.status() }
    }

    fn settings(&self) -> Option<&Settings> {
        self.rules.settings()
    }

    fn act(&self) -> crate::status::Act {
        self.rules.act()
    }

    fn rung(&self) -> Option<crate::commander::Rung> {
        self.rules.rung()
    }

    fn stance(&self) -> Option<(treadcore_sim::Tick, crate::tactics::Stance)> {
        self.rules.stance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_bots;
    use treadcore_sim::{MapSource, Scenario, World};

    #[test]
    fn sixteen_styles_each_by_number_and_by_name_and_none_share_either() {
        assert_eq!(ROSTER.len(), 18, "eight styles, eight personas, the quartermaster and the gladiator");
        for (i, s) in ROSTER.iter().enumerate() {
            assert_eq!(s.slot, i, "a slot keeps its place");
            assert_eq!(find(&i.to_string()), Some(s));
            assert_eq!(find(&s.name.to_lowercase()), Some(s));
            assert!(!s.about.is_empty());
            assert_eq!(ROSTER.iter().filter(|o| o.name == s.name).count(), 1);
        }
        assert_eq!(find("16").map(|s| s.name), Some("Quartermaster"));
        assert_eq!(find("17").map(|s| s.name), Some("Gladiator"));
        assert_eq!(find("18"), None);
        assert_eq!(find("nobody"), None);
        assert_eq!(find("rome").map(|s| s.slot), Some(8), "a persona by name");
    }

    #[test]
    fn the_dial_names_the_roster_in_its_order() {
        let names = W::BotStyle.choices().expect("bot_style is a choice");
        let want: Vec<String> = ROSTER.iter().map(|s| s.name.to_lowercase()).collect();
        assert_eq!(names.iter().map(|n| n.to_string()).collect::<Vec<_>>(), want);
    }

    #[test]
    fn a_team_may_be_one_style_or_several_and_a_mistake_says_what_there_is() {
        assert_eq!(parse_list("raider").unwrap()[0].slot, 1);
        assert_eq!(parse_list("1, castle,7").unwrap().iter().map(|s| s.slot).collect::<Vec<_>>(), vec![1, 2, 7]);
        let e = parse_list("1,nobody").unwrap_err();
        assert!(e.contains("nobody") && e.contains("7 Duelist"), "{}", e);
    }

    #[test]
    fn style_zero_is_the_brain_as_it_was() {
        let mut base = Settings::pinned();
        base.w[W::Near] = 3.25;
        assert_eq!(settings_for(&ROSTER[0], &base), base);
    }

    #[test]
    fn a_style_of_rules_keeps_the_seats_skill() {
        let mut base = Settings::pinned();
        base.w[W::Skill] = 0.6;
        let s = settings_for(find("raider").unwrap(), &base);
        assert_eq!(s.w[W::Skill], 0.6);
        assert_ne!(s, base, "and is its own dials");
    }

    #[test]
    fn the_overrider_changes_only_the_slot_on_todays_brain() {
        let base = Settings::pinned();
        let mut want = base.clone();
        let s = settings_for(find("overrider").unwrap(), &base);
        want.w[W::Patch] = 1.0;
        assert_eq!(s, want);
    }

    #[test]
    fn every_shipped_file_reads() {
        for s in &ROSTER {
            match s.kind {
                Kind::Net(b) => assert!(Net::from_bytes(b).is_ok(), "{}", s.name),
                Kind::Duel(b) => assert!(Champion::from_bytes(b).is_ok(), "{}", s.name),
                Kind::Transform(_) => assert!(crate::patch::slot(1).is_some(), "{}", s.name),
                Kind::Rules(t) => assert!(t.is_empty() || Settings::pinned_with(t).is_ok(), "{}", s.name),
            }
        }
    }

    #[test]
    fn a_champion_of_the_wrong_size_is_refused_not_misread() {
        assert!(Champion::from_bytes(&[0u8; 100]).is_err());
        assert_eq!(CHAMPION_WEIGHTS, 2340, "crucible.cu's NW at 35 in, 32, 32, 4 out");
    }

    #[test]
    fn the_champions_arithmetic_is_crucibles() {
        // All zeros but one bias: tanh of nothing is nothing, and the confidence is that bias's tanh.
        let mut w = vec![0f32; CHAMPION_WEIGHTS];
        w[CHAMPION_WEIGHTS - 1] = 0.6;
        let y = Champion { nin: NIN, h: H, w }.think(&[0.5; NIN]);
        assert_eq!(&y[..3], &[0.0, 0.0, 0.0]);
        assert!((y[3] - 0.6f32.tanh()).abs() < 1e-6);
    }

    #[test]
    fn a_world_champion_is_read_by_its_size_and_a_gateless_one_is_refused() {
        assert_eq!(WORLD_CHAMPION_WEIGHTS, 5236, "crucible.cu's NW at 55 in, 48, 48, 4 out");
        let c = Champion::from_bytes(&vec![0u8; WORLD_CHAMPION_WEIGHTS * 4]).unwrap();
        assert!(c.world());
        assert!(!Champion::from_bytes(&vec![0u8; CHAMPION_WEIGHTS * 4]).unwrap().world());
        // A plain world build, 52 in and 3 out: no gate, so no Duelist.
        let plain = 52 * 48 + 48 + 48 * 48 + 48 + 48 * 3 + 3;
        assert!(Champion::from_bytes(&vec![0u8; plain * 4]).is_err());
        let gladiator = ROSTER.iter().find(|s| s.name == "Gladiator").unwrap();
        let Kind::Duel(b) = gladiator.kind else { panic!("the gladiator duels") };
        assert!(Champion::from_bytes(b).unwrap().world(), "the shipped gladiator is a world champion");
    }

    #[test]
    fn a_world_champions_arithmetic_is_crucibles() {
        let mut w = vec![0f32; WORLD_CHAMPION_WEIGHTS];
        w[WORLD_CHAMPION_WEIGHTS - 1] = -0.4;
        let y = Champion { nin: W_NIN, h: W_H, w }.think(&[0.5; W_NIN]);
        assert_eq!(&y[..3], &[0.0, 0.0, 0.0]);
        assert!((y[3] - (-0.4f32).tanh()).abs() < 1e-6);
    }

    #[test]
    fn the_world_yardstick_turns_by_the_bearing_and_fires_inside_the_guns_reach() {
        assert_eq!(scripted_world(0.0, 4.0), [0.0, -0.5, 1.0], "close: back off, and it is aimed");
        assert_eq!(scripted_world(std::f32::consts::FRAC_PI_2, 30.0), [0.5, 1.0, -1.0], "a quarter turn off and far: half a heading error");
        assert_eq!(scripted_world(0.0, 12.0)[2], -1.0, "aimed but past the gun's ten");
    }

    /// Seat 0, an enemy six tiles ahead of it (at the start nobody is in sight on naval), and the
    /// observation with that enemy alone and every pillbox taken away.
    fn seat_and_enemy() -> (World, Observation, Tank, Tank) {
        let w = world();
        let mut obs = w.observe(0);
        obs.pillboxes.clear();
        let me = obs.me.clone().unwrap();
        let foe = (1..4u8).find(|p| !obs.allied(*p)).expect("an enemy side");
        let mut en = me.clone();
        en.owner = foe;
        en.pos = me.pos + V2::from_heading(me.heading).scale(6.0);
        obs.tanks.retain(|t| t.owner == 0);
        obs.tanks.push(en.clone());
        (w, obs, me, en)
    }

    #[test]
    fn a_world_tank_does_not_see_an_enemy_past_its_eyes_but_knows_its_bearing() {
        let (w, obs, me, mut en) = seat_and_enemy();
        en.pos = me.pos + V2::from_heading(me.heading).scale(TANK_VISION + 5.0);
        let x = world_senses(&obs, Some(&w.map), &me, &en);
        assert!((0..RAYS).all(|r| x[2 * r + 1] == 0.0), "no ray's enemy channel past the eyes");
        assert!((x[29] - 1.0).abs() < 1e-4, "dead ahead, by bearing");
        en.pos = me.pos + V2::from_heading(me.heading).scale(4.0);
        let x = world_senses(&obs, Some(&w.map), &me, &en);
        assert!(x[1] > 0.0, "ahead and near: the forward ray sees it");
        assert!(x[32].abs() < 1e-5 && x[33] == -0.5 && x[34] == 1.0, "the world's yardstick: aimed, too close: {:?}", &x[32..35]);
    }

    #[test]
    fn a_world_tank_sees_the_pillbox_that_has_it_and_not_its_own() {
        let (w, mut obs, me, en) = seat_and_enemy();
        let mut p = w.pillboxes.first().cloned().expect("naval has pillboxes");
        p.pos = me.pos + V2::from_heading(me.heading).scale(3.0);
        p.owner = None;
        p.health = PILLBOX_HEALTH;
        p.reload = 0;
        obs.pillboxes = vec![p.clone()];
        let x = world_senses(&obs, None, &me, &en);
        let o = AT_WORLD + RAYS;
        assert!(x[AT_WORLD] > 0.9, "the forward ray meets it");
        assert!(x[o].abs() < 1e-4 && (x[o + 1] - 1.0).abs() < 1e-4, "ahead");
        assert!((x[o + 2] - (1.0 - 3.0 / RANGE)).abs() < 1e-4, "three tiles off");
        assert_eq!((x[o + 3], x[o + 4], x[o + 5]), (0.0, 1.0, 1.0), "loaded, whole, and it has the tank");
        p.owner = Some(0);
        obs.pillboxes = vec![p];
        let x = world_senses(&obs, None, &me, &en);
        assert!(x[AT_WORLD..AT_WORLD + RAYS + 6].iter().all(|v| *v == 0.0), "a pillbox of its own side fires on nobody of it");
        assert!((x[o + 6] - me.shells as f32 / TANK_SHELLS as f32).abs() < 1e-6);
    }

    #[test]
    fn a_world_duelist_with_its_gate_open_and_no_throttle_holds_its_ground() {
        // The gate open, and the throttle's output pinned under the yardstick's: no reverse, so Stop.
        let mut w = vec![0f32; WORLD_CHAMPION_WEIGHTS];
        w[WORLD_CHAMPION_WEIGHTS - 1] = 9.0;
        w[WORLD_CHAMPION_WEIGHTS - 3] = -9.0;
        let (world, obs, _, _) = seat_and_enemy();
        let mut b = Duelist::new(Champion { nin: W_NIN, h: W_H, w }, Box::new(Marked(vec![])));
        assert_eq!(format!("{:?}", b.duel(&obs, Some(&world.map))), format!("{:?}", Some(vec![Command::Stop])));
    }

    #[test]
    fn the_gate_is_a_dial_and_a_sure_champion_passes_a_higher_one() {
        // A champion whose confidence is tanh(0.6), about 0.54, and whose throttle output is pinned low.
        let mut w = vec![0f32; WORLD_CHAMPION_WEIGHTS];
        w[WORLD_CHAMPION_WEIGHTS - 1] = 0.6;
        w[WORLD_CHAMPION_WEIGHTS - 3] = -9.0;
        let (world, obs, _, _) = seat_and_enemy();
        let at = |gate: f32| {
            let mut s = Settings::pinned();
            s.w[W::DuelGate] = gate;
            let mut b = Duelist::new(Champion { nin: W_NIN, h: W_H, w: w.clone() }, Box::new(Dialed(s)));
            b.duel(&obs, Some(&world.map)).is_some()
        };
        assert!(at(0.0) && at(0.5), "0.54 passes its own gate and one at a half");
        assert!(!at(0.6), "and not one at 0.6: the rules keep that fight");
    }

    /// A 30 x 30 field of grass with a wall bar at x = 15 from y = 5 to 24 (`bar`), or a ring of wall
    /// round (19..25, 12..18) (`ring`): the maze's corner and the walled base, as small as they come.
    fn maze_map(bar: bool, ring: bool) -> Map {
        let mut rows = vec![vec!['.'; 30]; 30];
        if bar {
            for row in rows.iter_mut().take(25).skip(5) {
                row[15] = 'W';
            }
        }
        if ring {
            for y in 12..=18 {
                for x in 19..=25 {
                    if y == 12 || y == 18 || x == 19 || x == 25 {
                        rows[y][x] = 'W';
                    }
                }
            }
        }
        let text = format!("treadcore map 1\nsize 30 30\n{}\nstart 2.5 2.5\nstart 27.5 27.5\n", rows.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n"));
        Map::from_text(&text).expect("the test map")
    }

    /// A world champion as crucible widens it to a maze one (`widen`): every new weight nothing.
    fn widened(old: &[f32]) -> Vec<f32> {
        let (h, oi) = (W_H, W_NIN);
        let mut w = vec![0f32; NAV_CHAMPION_WEIGHTS];
        for j in 0..h {
            for i in 0..oi {
                w[j * N_NIN + i] = old[j * oi + i];
            }
        }
        let (ob, nb) = (oi * h, N_NIN * h);
        w[nb..nb + h + h * h + h].copy_from_slice(&old[ob..ob + h + h * h + h]);
        let (ow3, nw3) = (ob + h + h * h + h, nb + h + h * h + h);
        w[nw3..nw3 + NOUT * h].copy_from_slice(&old[ow3..ow3 + NOUT * h]);
        let (ob3, nb3) = (ow3 + NOUT * h, nw3 + N_NOUT * h);
        w[nb3..nb3 + NOUT].copy_from_slice(&old[ob3..ob3 + NOUT]);
        w
    }

    #[test]
    fn a_maze_champion_is_read_by_its_size() {
        assert_eq!(NAV_CHAMPION_WEIGHTS, 5766, "crucible.cu's NW at 64 in, 48, 48, 6 out");
        let c = Champion::from_bytes(&vec![0u8; NAV_CHAMPION_WEIGHTS * 4]).unwrap();
        assert!(c.world() && c.nav(), "a maze champion drives as the world's tank, and reads the way");
        assert!(!Champion::from_bytes(&vec![0u8; WORLD_CHAMPION_WEIGHTS * 4]).unwrap().nav());
    }

    #[test]
    fn a_maze_champion_widened_from_a_world_one_answers_as_it_did() {
        let old: Vec<f32> = (0..WORLD_CHAMPION_WEIGHTS).map(|i| ((i * 7919 % 1000) as f32 / 1000.0 - 0.5) * 0.3).collect();
        let x: Vec<f32> = (0..N_NIN).map(|i| ((i * 31 % 17) as f32 / 17.0) - 0.4).collect();
        let a = Champion { nin: W_NIN, h: W_H, w: old.clone() }.think(&x[..W_NIN]);
        let b = Champion { nin: N_NIN, h: W_H, w: widened(&old) }.think(&x);
        for k in 0..NOUT {
            assert!((a[k] - b[k]).abs() < 1e-5, "output {}: {} and {}", k, a[k], b[k]);
        }
        assert_eq!((b[4], b[5]), (0.0, 0.0), "the kept numbers start at nothing");
    }

    #[test]
    fn round_a_wall_the_way_turns_off_the_line_and_the_rules_go_with_it() {
        let (_, obs, mut me, mut en) = seat_and_enemy();
        let m = maze_map(true, false);
        me.pos = V2::new(11.5, 14.5);
        me.heading = 0.0;
        en.pos = V2::new(19.5, 14.5);
        let x = nav_senses(&obs, Some(&m), &me, &en, [0.25, -0.5]);
        assert_eq!(x[AT_NAV + 3], 0.0, "there is a way round: not walled");
        assert!(x[AT_NAV + 1] < 0.8, "the way's next point is off the straight line: cos {}", x[AT_NAV + 1]);
        assert!(x[AT_NAV + 2] * RANGE > 8.5, "the way is longer than the crow's 8 tiles: {}", x[AT_NAV + 2] * RANGE);
        assert!(x[AT_NAV + 5] < 1.0, "the wall ahead cuts the straight arc short");
        assert_eq!(&x[AT_NAV + 7..AT_NAV + 9], &[0.25, -0.5], "the kept numbers, read back");
        assert_eq!(x[33], 1.0, "the errand: the rules drive round, at full throttle");
        assert!((x[32] - (wrap_angle(x[AT_NAV].atan2(x[AT_NAV + 1])) / std::f32::consts::PI)).abs() < 1e-4, "and turn toward the way's next point");
        assert_eq!(&x[..32], &world_senses(&obs, Some(&m), &me, &en)[..32], "the world's own senses unchanged");
    }

    #[test]
    fn a_walled_off_enemy_reads_walled_and_the_rules_are_the_script() {
        let (_, obs, mut me, mut en) = seat_and_enemy();
        let m = maze_map(false, true);
        me.pos = V2::new(10.5, 15.5);
        me.heading = 0.0;
        en.pos = V2::new(22.5, 15.5);
        let x = nav_senses(&obs, Some(&m), &me, &en, [0.0; MEM]);
        assert_eq!(x[AT_NAV + 3], 1.0, "no way in");
        assert!((x[AT_NAV + 2] - 1.0).abs() < 1e-6, "and no length to it");
        assert!((x[AT_NAV + 1] - 1.0).abs() < 1e-4, "it points at the enemy itself");
        assert_eq!(&x[32..35], &world_senses(&obs, Some(&m), &me, &en)[32..35], "the script's answer: the breach is the rules' to choose");
    }

    #[test]
    fn in_the_open_the_maze_senses_add_and_change_nothing() {
        let (_, obs, mut me, mut en) = seat_and_enemy();
        let m = maze_map(false, false);
        me.pos = V2::new(10.5, 15.5);
        me.heading = 0.0;
        en.pos = V2::new(18.5, 15.5);
        let x = nav_senses(&obs, Some(&m), &me, &en, [0.0; MEM]);
        assert_eq!(&x[..W_NIN], &world_senses(&obs, Some(&m), &me, &en)[..], "the world's 55 as they were");
        assert!((x[AT_NAV + 1] - 1.0).abs() < 1e-4 && (x[AT_NAV + 2] * RANGE - 8.0).abs() < 1e-3, "dead ahead, 8 tiles");
        assert_eq!(&x[AT_NAV + 4..AT_NAV + 7], &[1.0, 1.0, 1.0], "every arc free");
    }

    #[test]
    fn a_maze_duelist_keeps_its_two_numbers_and_forgets_them_when_the_fight_ends() {
        // Gate open, and the two kept numbers' biases at 0.5 and -0.25.
        let mut w = vec![0f32; NAV_CHAMPION_WEIGHTS];
        let b3 = NAV_CHAMPION_WEIGHTS - N_NOUT;
        w[b3 + 3] = 9.0;
        w[b3 + 4] = 0.5;
        w[b3 + 5] = -0.25;
        let (world, obs, _, _) = seat_and_enemy();
        let mut b = Duelist::new(Champion { nin: N_NIN, h: W_H, w }, Box::new(Marked(vec![])));
        assert!(b.duel(&obs, Some(&world.map)).is_some());
        assert!((b.mem[0] - 0.5f32.tanh()).abs() < 1e-6 && (b.mem[1] - (-0.25f32).tanh()).abs() < 1e-6, "{:?}", b.mem);
        let mut alone = obs.clone();
        alone.tanks.retain(|t| t.owner == 0);
        assert!(b.duel(&alone, Some(&world.map)).is_none());
        assert_eq!(b.mem, [0.0; MEM], "no enemy in reach: the next fight starts from nothing");
    }

    #[test]
    fn a_ray_meets_what_is_in_front_and_misses_what_is_not() {
        let d = V2::new(1.0, 0.0);
        let hit = ray_circle(V2::new(0.0, 0.0), d, V2::new(10.0, 0.0), 1.0);
        assert!((hit - 9.0).abs() < 1e-5);
        assert_eq!(ray_circle(V2::new(0.0, 0.0), d, V2::new(-10.0, 0.0), 1.0), f32::MAX, "behind");
        assert_eq!(ray_circle(V2::new(0.0, 0.0), d, V2::new(10.0, 5.0), 1.0), f32::MAX, "beside");
    }

    #[test]
    fn the_yardstick_turns_closes_and_fires_as_crucibles() {
        assert_eq!(scripted(0.0, 5.0), [0.0, -0.5, 1.0], "close: back off, and it is aimed");
        assert_eq!(scripted(1.0, 30.0), [1.0, 1.0, -1.0], "far and off: turn, close, hold fire");
    }

    /// Stands in for the rules with a seat's dials, as the Commander carries them: the gate reads there.
    struct Dialed(Settings);
    impl Brain for Dialed {
        fn name(&self) -> &str {
            "dialed"
        }
        fn think(&mut self, _: &Observation) -> Vec<Command> {
            vec![]
        }
        fn settings(&self) -> Option<&Settings> {
            Some(&self.0)
        }
    }

    /// Stands in for the rules, and says so: every Command of its own is one the duel may replace.
    struct Marked(Vec<Command>);
    impl Brain for Marked {
        fn name(&self) -> &str {
            "marked"
        }
        fn think(&mut self, _: &Observation) -> Vec<Command> {
            self.0.clone()
        }
    }

    fn world() -> World {
        World::new(Scenario { map: MapSource::Text(include_str!("../../../maps/naval.map").into()), ..Scenario::bots(4) })
    }

    #[test]
    fn with_its_gate_shut_a_duelist_is_the_rules_to_the_last_command() {
        // The last bias holds confidence far under zero: the gate never opens.
        let mut w = vec![0f32; CHAMPION_WEIGHTS];
        w[CHAMPION_WEIGHTS - 1] = -9.0;
        let said = vec![Command::Move { to: V2::new(5.0, 5.0) }];
        let mut b = Duelist::new(Champion { nin: NIN, h: H, w }, Box::new(Marked(said.clone())));
        let w = world();
        for p in 0..4 {
            assert_eq!(format!("{:?}", b.think_on(&w.observe(p), Some(&w.map))), format!("{:?}", said));
        }
    }

    #[test]
    fn no_enemy_to_see_means_the_rules_alone() {
        let mut w = vec![0f32; CHAMPION_WEIGHTS];
        w[CHAMPION_WEIGHTS - 1] = 9.0;
        let said = vec![Command::Stop];
        let mut b = Duelist::new(Champion { nin: NIN, h: H, w }, Box::new(Marked(said.clone())));
        let world = world();
        let mut obs = world.observe(0);
        obs.tanks.retain(|t| t.owner == 0);
        assert_eq!(format!("{:?}", b.think_on(&obs, Some(&world.map))), format!("{:?}", said));
        assert!(!b.open);
    }

    #[test]
    fn every_style_plays_a_match_to_its_end_without_a_fault() {
        for st in &ROSTER {
            let mut w = world();
            let base = Settings::pinned();
            let mut bots: Vec<(u8, Box<dyn Brain>)> = crate::fleet::with_settings(base.clone(), || {
                (0..4u8).map(|i| (i, make(st, &base, st.name, || crate::fleet::brain_as(crate::fleet::Role::Leader, i, 7 + i as u64, st.name)))).collect()
            });
            for _ in 0..600 {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
            }
            assert_eq!(bots[0].1.name(), st.name, "named as asked");
            assert!(w.tick > 0);
        }
    }
}
