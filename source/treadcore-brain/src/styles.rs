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
//! SHIPPED, NOT LOCATED. The files are in the binary: a package has no fleet to find them on. Nothing
//! here moves a default; style 0 is the brain exactly as it was.

use crate::fleet::with_settings;
use crate::net::{Net, NetBrain};
use crate::weights::{Settings, W};
use crate::Brain;
use treadcore_sim::rules::{DIRECT_RANGE, TANK_ARMOR, TANK_SPEED};
use treadcore_sim::{wrap_angle, Command, Map, Observation, Tank, Terrain, V2};

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
pub const ROSTER: [Style; 8] = [
    Style { slot: 0, name: "Fleet", about: "the brain the fleet's search has carried: today's defaults", kind: Kind::Rules("") },
    Style { slot: 1, name: "Raider", about: "goes and fights: leans to expanding, closes at even odds, picks off the wounded", kind: Kind::Rules(include_str!("../../../profiles/raider.weights")) },
    Style { slot: 2, name: "Castle", about: "holds its ground: leans to fortifying and defending", kind: Kind::Rules(include_str!("../../../profiles/castle.weights")) },
    Style { slot: 3, name: "Expander", about: "takes the map: goes for depots and pillboxes first", kind: Kind::Rules(include_str!("../../../profiles/expander.weights")) },
    Style { slot: 4, name: "Timid", about: "keeps away: fights only with the odds well on its side", kind: Kind::Rules(include_str!("../../../profiles/timid.weights")) },
    Style { slot: 5, name: "Overrider", about: "the rules, with the learned transform moving their answer when it is sure", kind: Kind::Transform("patch = 1\n") },
    Style { slot: 6, name: "Scholar", about: "a net that learned by imitating the rules: all senses, no rules", kind: Kind::Net(include_bytes!("../../../nets/bc2.bin")) },
    Style { slot: 7, name: "Duelist", about: "the rules, with a champion from crucible's GPU islands in every fight", kind: Kind::Duel(include_bytes!("../../../nets/duelist.bin")) },
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
        Kind::Net(bytes) => Box::new(NetBrain::new(Net::from_bytes(bytes).expect("a shipped net reads"), name)),
        Kind::Duel(bytes) => Box::new(Duelist::new(Champion::from_bytes(bytes).expect("a shipped champion reads"), with_settings(base.clone(), rules))),
    }
}

// ---- the bridge -----------------------------------------------------------------------------

const NIN: usize = 35;
const H: usize = 32;
const NOUT: usize = 4;
/// crucible.cu's NW for a hybrid build. A champion is exactly this many floats, and nothing else.
pub const CHAMPION_WEIGHTS: usize = NIN * H + H + H * H + H + H * NOUT + NOUT;
/// crucible.cu: how far its rays and its senses reach, and its scripted yardstick's range and aim.
const RANGE: f32 = 40.0;
const YARD_RANGE: f32 = 11.0;
const YARD_TOL: f32 = 0.12;
const RAYS: usize = 12;
const TANK_R: f32 = 0.75;
/// Five times a second, as a NetBrain: the arena's step is a tenth, and a Move takes time to bite.
const DECIDE_EVERY: u64 = treadcore_sim::TICK_HZ as u64 / 5;
/// How far a full throttle reaches at each decision, and how far a full turn swings it.
const STRIDE: f32 = 6.0;
const SWING: f32 = 1.0;

/// A hybrid champion from crucible: 35 in, 32, 32, 4 out, tanh throughout, as crucible.cu's `think`.
#[derive(Debug, Clone, PartialEq)]
pub struct Champion {
    w: Vec<f32>,
}

impl Champion {
    pub fn from_bytes(bytes: &[u8]) -> Result<Champion, String> {
        if bytes.len() != CHAMPION_WEIGHTS * 4 {
            return Err(format!("a hybrid champion is {} bytes, this is {}", CHAMPION_WEIGHTS * 4, bytes.len()));
        }
        Ok(Champion { w: bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect() })
    }

    pub fn think(&self, x: &[f32; NIN]) -> [f32; NOUT] {
        let w = &self.w;
        let (b1, w2) = (NIN * H, NIN * H + H);
        let (b2, w3) = (w2 + H * H, w2 + H * H + H);
        let b3 = w3 + H * NOUT;
        let mut h1 = [0f32; H];
        for j in 0..H {
            let s: f32 = w[b1 + j] + (0..NIN).map(|i| w[j * NIN + i] * x[i]).sum::<f32>();
            h1[j] = s.tanh();
        }
        let mut h2 = [0f32; H];
        for j in 0..H {
            let s: f32 = w[b2 + j] + (0..H).map(|i| w[w2 + j * H + i] * h1[i]).sum::<f32>();
            h2[j] = s.tanh();
        }
        let mut out = [0f32; NOUT];
        for k in 0..NOUT {
            let s: f32 = w[b3 + k] + (0..H).map(|i| w[w3 + k * H + i] * h2[i]).sum::<f32>();
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

/// The rules, with a champion in the fight. THE GATE IS THE CHAMPION'S OWN: shut, this seat is the
/// Commander to the last command, as the arena's hybrid tank plays its script.
pub struct Duelist {
    champion: Champion,
    rules: Box<dyn Brain>,
    open: bool,
}

impl Duelist {
    pub fn new(champion: Champion, rules: Box<dyn Brain>) -> Duelist {
        Duelist { champion, rules, open: false }
    }

    /// Its say this decision, if the gate opens: what to do instead of the rules' moving and firing.
    fn duel(&self, obs: &Observation, map: Option<&Map>) -> Option<Vec<Command>> {
        let me = obs.me.as_ref().filter(|t| t.alive)?;
        let en = obs.hostile_tanks().filter(|t| t.alive).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))?;
        if me.pos.dist(en.pos) >= RANGE {
            return None;
        }
        let x = senses(obs, map, me, en);
        let y = self.champion.think(&x);
        if y[3] <= 0.0 {
            return None;
        }
        let act: Vec<f32> = (0..3).map(|i| (x[32 + i] + y[i]).clamp(-1.0, 1.0)).collect();
        let mut cmds = Vec::new();
        let dir = V2::from_heading(me.heading + act[0] * SWING);
        cmds.push(Command::Move { to: obs.clamp(me.pos + dir.scale(act[1] * STRIDE)) });
        if act[2] > 0.0 && me.reload == 0 && me.shells > 0 && me.pos.dist(en.pos) <= DIRECT_RANGE {
            cmds.push(Command::FireDirect { at: crate::lead(me, en) });
        }
        Some(cmds)
    }
}

/// What the rules said that the duel now says instead.
fn fight_order(c: &Command) -> bool {
    matches!(c, Command::Move { .. } | Command::Stop | Command::FireDirect { .. })
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
            None => self.open = false,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_bots;
    use treadcore_sim::{MapSource, Scenario, World};

    #[test]
    fn eight_styles_each_by_number_and_by_name_and_none_share_either() {
        assert_eq!(ROSTER.len(), 8);
        for (i, s) in ROSTER.iter().enumerate() {
            assert_eq!(s.slot, i, "a slot keeps its place");
            assert_eq!(find(&i.to_string()), Some(s));
            assert_eq!(find(&s.name.to_lowercase()), Some(s));
            assert!(!s.about.is_empty());
            assert_eq!(ROSTER.iter().filter(|o| o.name == s.name).count(), 1);
        }
        assert_eq!(find("8"), None);
        assert_eq!(find("nobody"), None);
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
        let mut base = Settings::default();
        base.w[W::Near] = 3.25;
        assert_eq!(settings_for(&ROSTER[0], &base), base);
    }

    #[test]
    fn a_style_of_rules_keeps_the_seats_skill() {
        let mut base = Settings::default();
        base.w[W::Skill] = 0.6;
        let s = settings_for(find("raider").unwrap(), &base);
        assert_eq!(s.w[W::Skill], 0.6);
        assert_ne!(s, base, "and is its own dials");
    }

    #[test]
    fn the_overrider_changes_only_the_slot_on_todays_brain() {
        let base = Settings::default();
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
                Kind::Rules(t) => assert!(t.is_empty() || Settings::parse(t).is_ok(), "{}", s.name),
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
        let y = Champion { w }.think(&[0.5; NIN]);
        assert_eq!(&y[..3], &[0.0, 0.0, 0.0]);
        assert!((y[3] - 0.6f32.tanh()).abs() < 1e-6);
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
        let mut b = Duelist::new(Champion { w }, Box::new(Marked(said.clone())));
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
        let mut b = Duelist::new(Champion { w }, Box::new(Marked(said.clone())));
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
            let base = Settings::default();
            let mut bots: Vec<(u8, Box<dyn Brain>)> = (0..4u8)
                .map(|i| (i, make(st, &base, st.name, || crate::fleet::brain_as(crate::fleet::Role::Leader, i, 7 + i as u64, st.name))))
                .collect();
            for _ in 0..600 {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
            }
            assert_eq!(bots[0].1.name(), st.name, "named as asked");
            assert!(w.tick > 0);
        }
    }
}
