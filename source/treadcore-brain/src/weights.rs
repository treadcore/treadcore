//! Weights and capabilities (roadmap step 12c, LAB-REPORT-BRAIN.md
//! sections 4 and 5). Every number a tuner may touch is one slot of a
//! flat array, `Weights`, named by `W`; every part of the brain that can
//! be switched off is one bit of `Caps`. The array is lent to whatever
//! scores (`&Weights`), never cloned per think, and written whole into
//! the ledger (ledger.rs) beside the match it played, so a high score
//! can be played again and bred from.
//!
//! The defaults are the constants the brain had on 17 October: the 22
//! weights of `Tactics` and its other settings as they were, and the
//! new parts' numbers as the lab report starts them. A capability that
//! is new is off by default, so the brain of 17 October is the baseline
//! until a census promotes a part (ROADMAP goal 4).
//!
//! The file form is TOML's simplest subset, a `name = value` a line and
//! `#` comments, so a person can write one by hand and `--weights FILE`
//! reads it without a parser crate:
//!
//! ```text
//! # bolder
//! w_wounded = 1.5
//! press_odds = 1.2
//! caps = smooth, forget, priced_crossing
//! ```

use crate::tactics::Tactics;

macro_rules! weights {
    ($($id:ident $name:literal $default:expr, $doc:literal;)*) => {
        /// The name of each weight: its index in `Weights`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(usize)]
        pub enum W { $(#[doc = $doc] $id,)* }

        impl W {
            pub const ALL: &'static [W] = &[$(W::$id,)*];
            pub fn name(self) -> &'static str {
                match self { $(W::$id => $name,)* }
            }
            pub fn default_value(self) -> f32 {
                match self { $(W::$id => $default,)* }
            }
            pub fn doc(self) -> &'static str {
                match self { $(W::$id => $doc,)* }
            }
            pub fn by_name(name: &str) -> Option<W> {
                match name { $($name => Some(W::$id),)* _ => None }
            }
            /// A slider's step (the coordination screen, backlog 15): one
            /// for a rounded weight, else a tenth of the default taken
            /// to 1, 2 or 5 of its power of ten; a tenth for nothing.
            pub fn step(self) -> f32 {
                if self.doc().contains("(rounded)") {
                    return 1.0;
                }
                let d = self.default_value().abs() / 10.0;
                if d == 0.0 {
                    return 0.1;
                }
                let e = d.log10().floor() as i32;
                let m = d / 10f32.powi(e);
                let nice = if m < 1.5 { 1.0 } else if m < 3.5 { 2.0 } else if m < 7.5 { 5.0 } else { 10.0 };
                // Divided, not multiplied, below one: 5 / 100 is 0.05 exactly.
                if e < 0 { nice / 10f32.powi(-e) } else { nice * 10f32.powi(e) }
            }
        }
    };
}

weights! {
    // Tactics: target selection (tactics.rs; the 22 weights and their scales).
    Near "w_near" 1.0, "target: worth of nearness, w_near / (1 + d / near_scale)";
    NearScale "near_scale" 15.0, "target: the distance scale of nearness, tiles";
    Wounded "w_wounded" 1.2, "target: worth of its wounds";
    Finish "w_finish" 0.8, "target: one direct shot from dead";
    Boat "w_boat" 1.0, "target: afloat, a boat sinks at one hit";
    Called "w_called" 0.8, "target: a teammate pinged it";
    CalledFor "called_for" 15.0, "target: seconds a ping stays a call";
    Focus "w_focus" 0.5, "target: each teammate already on it";
    FocusCap "focus_cap" 2.0, "target: teammates on it before overkill (rounded)";
    Overkill "w_overkill" 0.6, "target: off for each teammate past the cap";
    InRange "w_in_range" 0.7, "target: in direct range now";
    Threat "w_threat" 0.6, "target: menacing a teammate or the base";
    BaseR "base_r" 16.0, "target: the base's reach for w_threat, tiles";
    Stick "w_stick" 0.4, "target: the one already chosen";
    // Tactics: the engagement and assignment.
    OddsR "odds_r" 14.0, "odds: tanks this near the fight count, tiles";
    PillboxWorth "pillbox_worth" 0.7, "odds: a hostile pillbox as a share of a tank";
    StrikeWorth "strike_worth" 0.35, "odds: a hostile strike drone as a share of a tank";
    EngageOdds "engage_odds" 0.7, "odds: below these, with help coming, mass";
    PressOdds "press_odds" 1.5, "odds: above these, close in";
    ComingR "coming_r" 45.0, "odds: teammates this near are coming, tiles";
    MassFor "mass_for" 12.0, "odds: seconds to hold off for them";
    AnswerCap "answer_cap" 3.0, "pings: this many nearer teammates answer instead (rounded)";
    CrewReach "crew_reach" 40.0, "pings: a crew on a job answers only this near, tiles";
    Exposure "w_exposure" 1.0, "stand: cost per gun covering a firing point";
    Flank "w_flank" 0.3, "stand: cost of leaving the flank's bearing";
    Kite "kite" 1.0, "kite: on (1) or off (0) (rounded)";
    KiteMargin "kite_margin" 1.5, "kite: the waiting ring past the target's reach, tiles";
    KiteIn "kite_in" 9.0, "kite: the ring a step in ends on, tiles";
    KiteTurn "kite_turn" 0.8, "kite: seconds a step in takes beyond the drive";
    KiteSpare "kite_spare" 0.4, "kite: seconds of the target's reload to spare";
    // Smoothing (LR 7).
    Alpha "alpha" 0.4, "smooth: the move's desire vector, share of the new pull a think";
    Beta "beta" 0.3, "smooth: the advance scalar, share of the new pull a think";
    Margin "margin" 0.15, "smooth: a challenger must beat the intent by this share";
    Confirm "confirm" 2.0, "smooth: thinks in a row a challenger must win (rounded)";
    ConfirmTurret "confirm_turret" 3.0, "smooth: picks in a row before the turret changes target (rounded)";
    Commit "commit" 0.1, "smooth: the intent's commitment bonus";
    AdvanceBand "advance_band" 0.35, "smooth: the advance scalar presses above this and falls back below minus this";
    Jitter "jitter" 3.0, "smooth: a moving goal's shift under this is smoothed, over it taken at once, tiles";
    RetreatArmor "retreat_armor" 35.0, "survival: armour at or under this goes home";
    RetreatShells "retreat_shells" 4.0, "survival: shells at or under this go home";
    // Memory (LR 5).
    ForgetFor "forget_for" 120.0, "memory: seconds a given-up target stays given up";
    // Crossings and islands (LR 6.2).
    BridgeSoft "bridge_soft" 6.0, "crossing: a bridge this long or shorter costs no extra";
    LongBridge "k_long" 0.5, "crossing: the square cost of each tile past bridge_soft";
    BoatCost "c_boat" 5.0, "crossing: a boat's price in material";
    Launch "t_launch" 3.0, "crossing: the launch and the landing, in material";
    TreesPerMaterial "tree_material" 4.0, "crossing: material a tree gives";
    ForestNeed "forest_need" 12.0, "island: trees a base will ask for";
    SiteForest "w_site_forest" 1.0, "island: a base site's worth of trees enough";
    SiteSmall "w_site_small" 1.0, "island: a base site's cost of a small island";
    SiteArea "site_area" 400.0, "island: an island this large or larger is not small, tiles";
    // Formation (LR 6.3).
    ColumnGap "column_gap" 2.0, "column: tiles between tanks in file on a bridge";
    ColumnPast "column_past" 2.0, "column: the wedge comes back this far past the bridge, tiles";
    // The team (LR 9).
    SwapGap "swap_gap" 0.3, "rotation: readiness gap between a hot and a cold point that swaps two tanks";
    // The base cycle (LR 6.1).
    GiveItem "w_give_item" 1.0, "cycle: one plan item for the base on each fill";
    GiveUpgrade "w_give_upgrade" 1.0, "cycle: one improvement for the base on each fill";
    Connect "w_connect" 1.0, "cycle: road tiles toward the next friendly base or pillbox";
    ConnectTiles "connect_tiles" 4.0, "cycle: road tiles laid on one visit at most";
    // Paul's answers of 18 October as scalars (ROADMAP 12h, LR 10).
    Mend "w_mend" 1.0, "improvement: worth of mending the most hurt pillbox of the base";
    Mine "w_mine" 0.5, "improvement: worth of a mine on the moat";
    Works "w_works" 0.5, "improvement: worth of works on a pad by the base";
    UpgradeVsMend "upgrade_vs_mend" 0.5, "improvement: share of the slot for 11g's upgrades once they exist, against mend, mines and works";
    CastleInside "castle_inside" 2.0, "castle: pillboxes inside the wall, then it stops filling (rounded)";
    CastleRoad "castle_road" 1.0, "castle: pillboxes outside on its road (rounded)";
    PostSpacing "post_spacing" 10.0, "posts: tiles between pillboxes on a road between bases";
    PostsMax "posts_max" 4.0, "posts: road posts a base fills at most (rounded)";
    JunctionClear "junction_clear" 3.0, "posts: no post this near a road junction, tiles";
    ConnectBases "connect_bases" 0.75, "connect: a base or castle preferred to a lone pillbox, 0 none, 1 its distance counted at nothing";
    BridgeKeep "bridge_keep" 180.0, "bridge jobs: seconds a part-laid bridge stays on the team board with no tile laid";
    BridgeReach "bridge_reach" 40.0, "bridge jobs: a teammate this near its next tile takes it, tiles";
    BridgeWing "bridge_wing" 1.0, "bridge jobs: a wingman with material takes one too, leaving its leader's wedge (1), or only tanks on auto (0) (rounded)";
    FetchGather "fetch_gather" 20.0, "escort: a fetch trip farther than this from the base gathers the fleet first, tiles";
    LongGame "long_game" 0.5, "score: 0 judges the win and kills, 1 the ground held over time";
    Assist "assist" 1.0, "roles: a person's team gets a builder at 0.5 and over, else is dealt as a bots' team";
}

/// The flat array of every weight.
#[derive(Clone, Debug, PartialEq)]
pub struct Weights(pub Vec<f32>);

impl Default for Weights {
    fn default() -> Weights {
        Weights(W::ALL.iter().map(|w| w.default_value()).collect())
    }
}

impl std::ops::Index<W> for Weights {
    type Output = f32;
    fn index(&self, w: W) -> &f32 {
        &self.0[w as usize]
    }
}

impl std::ops::IndexMut<W> for Weights {
    fn index_mut(&mut self, w: W) -> &mut f32 {
        &mut self.0[w as usize]
    }
}

macro_rules! caps {
    ($($id:ident $bit:literal $name:literal $on:literal, $doc:literal;)*) => {
        /// The capabilities: one bit each.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct Caps(pub u64);

        impl Caps {
            $(#[doc = $doc] pub const $id: Caps = Caps(1 << $bit);)*
            pub const NAMES: &'static [(&'static str, Caps)] = &[$(($name, Caps::$id),)*];
            /// The default: the brain of 17 October (what was there, on;
            /// what is new, off), and what Paul has made the default since
            /// (`castle_cap`, 18 October).
            pub const BASELINE: Caps = Caps(0 $(| if $on { 1 << $bit } else { 0 })*);
            /// What a capability does, for the sliders' page.
            pub fn doc(self) -> &'static str {
                $(if self == Caps::$id { return $doc; })*
                ""
            }
        }
    };
}

caps! {
    KITE 0 "kite" true, "kite a target past its reach (tactics.rs)";
    PINGS 1 "pings" true, "call enemies and answer teammates' pings";
    DODGE 2 "dodge" true, "step out from under a shell";
    BRIDGES 3 "bridges" true, "lay bridges (hop.rs)";
    ROTATE 4 "rotate" true, "rotate between the push and defence points";
    FORGET 8 "forget" false, "given-up targets are tried again after forget_for (12b)";
    SMOOTH 9 "smooth" false, "desire vectors and confirmation before a change (12d)";
    BOARD 10 "board" false, "the team board: one picture of the map per team (12e)";
    TEAM_ROTATE 11 "team_rotate" false, "swap a hurt tank at a hot point for a fresh one at a cold point (12e)";
    PRICED_CROSSING 12 "priced_crossing" false, "boat or bridge by price, long bridges and scarce trees costing more (12f2)";
    COLUMN 13 "column" false, "a fleet crosses a bridge in file (12f3)";
    CYCLE 14 "cycle" false, "after filling up, give the base an item and an improvement and road out (12f1)";
    ISLAND_SITE 15 "island_site" false, "a small island with few trees is a poor base (12f2)";
    ESCORT 17 "escort" false, "a fleet leader's far fetch trip gathers its wingmen first, so the fleet escorts the pillbox home along the road (12f)";
    BRIDGE_JOBS 18 "bridge_jobs" false, "a part-laid bridge is a job on the team board, taken by the next teammate with material; needs board (12f)";
    CASTLE_CAP 16 "castle_cap" true, "the castle stops at castle_inside and castle_road pillboxes; the rest go to posts on the roads between bases (12h)";
}

impl Default for Caps {
    fn default() -> Caps {
        Caps::BASELINE
    }
}

impl Caps {
    pub fn has(self, c: Caps) -> bool {
        self.0 & c.0 == c.0
    }
    pub fn with(self, c: Caps) -> Caps {
        Caps(self.0 | c.0)
    }
    pub fn without(self, c: Caps) -> Caps {
        Caps(self.0 & !c.0)
    }
    /// Every capability, the new ones too.
    pub fn all() -> Caps {
        Caps(Caps::NAMES.iter().fold(0, |a, (_, c)| a | c.0))
    }
    pub fn by_name(name: &str) -> Option<Caps> {
        Caps::NAMES.iter().find(|(n, _)| *n == name).map(|(_, c)| *c)
    }
    /// The names of the bits that are on, comma-separated.
    pub fn names(self) -> String {
        Caps::NAMES.iter().filter(|(_, c)| self.has(*c)).map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
    }
}

/// A bad line in a weights file: which, and why.
#[derive(Debug, PartialEq)]
pub struct WeightsError {
    pub line: usize,
    pub why: String,
}

impl std::fmt::Display for WeightsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.why)
    }
}

/// A brain's settings: the weights and the capabilities.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub w: Weights,
    pub caps: Caps,
}

impl Settings {
    /// Read the file form. A name not given keeps its default; `caps =`
    /// replaces the capabilities with the names listed (`baseline` and
    /// `all` stand for those sets, `-name` takes one off).
    pub fn parse(text: &str) -> Result<Settings, WeightsError> {
        let mut s = Settings::default();
        for (i, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let bad = |why: String| WeightsError { line: i + 1, why };
            let (name, value) = line.split_once('=').ok_or_else(|| bad("not name = value".into()))?;
            let (name, value) = (name.trim(), value.trim().trim_matches('"'));
            if name == "caps" {
                let mut caps = Caps(0);
                for c in value.split(',').map(str::trim).filter(|c| !c.is_empty()) {
                    caps = match c {
                        "baseline" => caps.with(Caps::BASELINE),
                        "all" => caps.with(Caps::all()),
                        _ if c.starts_with('-') => caps.without(Caps::by_name(&c[1..]).ok_or_else(|| bad(format!("no capability {}", &c[1..])))?),
                        _ => caps.with(Caps::by_name(c).ok_or_else(|| bad(format!("no capability {}", c)))?),
                    };
                }
                s.caps = caps;
                continue;
            }
            let w = W::by_name(name).ok_or_else(|| bad(format!("no weight {}", name)))?;
            let v: f32 = value.parse().map_err(|_| bad(format!("{} is not a number", value)))?;
            if !v.is_finite() {
                return Err(bad(format!("{} is not finite", value)));
            }
            s.w[w] = v;
        }
        Ok(s)
    }

    /// The file form: every weight, and the capabilities.
    pub fn to_text(&self) -> String {
        let mut out = format!("caps = {}\n", self.caps.names());
        for w in W::ALL {
            out.push_str(&format!("{} = {}  # {}\n", w.name(), self.w[*w], w.doc()));
        }
        out
    }

    /// The weights that differ from the defaults: what makes a genome.
    pub fn diff(&self) -> Vec<(W, f32)> {
        W::ALL.iter().filter(|w| self.w[**w] != w.default_value()).map(|w| (*w, self.w[*w])).collect()
    }

    /// A slider moved `by` steps: never under nothing, and kept to four
    /// places so a step back is the number it was.
    pub fn nudge(&mut self, w: W, by: i32) {
        let v = (self.w[w] + by as f32 * w.step()).max(0.0);
        self.w[w] = (v * 1e4).round() / 1e4;
    }

    /// Seconds into ticks, for the time weights.
    pub fn ticks(&self, w: W) -> treadcore_sim::Tick {
        (self.w[w].max(0.0) * treadcore_sim::TICK_HZ as f32) as treadcore_sim::Tick
    }
}

impl Tactics {
    /// The tactics the weights describe.
    pub fn from_weights(s: &Settings) -> Tactics {
        let w = &s.w;
        Tactics {
            w_near: w[W::Near],
            near_scale: w[W::NearScale],
            w_wounded: w[W::Wounded],
            w_finish: w[W::Finish],
            w_boat: w[W::Boat],
            w_called: w[W::Called],
            called_for: w[W::CalledFor],
            w_focus: w[W::Focus],
            focus_cap: w[W::FocusCap].round().max(0.0) as usize,
            w_overkill: w[W::Overkill],
            w_in_range: w[W::InRange],
            w_threat: w[W::Threat],
            base_r: w[W::BaseR],
            w_stick: w[W::Stick],
            odds_r: w[W::OddsR],
            pillbox_worth: w[W::PillboxWorth],
            strike_worth: w[W::StrikeWorth],
            engage_odds: w[W::EngageOdds],
            press_odds: w[W::PressOdds],
            coming_r: w[W::ComingR],
            mass_for: w[W::MassFor],
            answer_cap: w[W::AnswerCap].round().max(0.0) as usize,
            crew_reach: w[W::CrewReach],
            w_exposure: w[W::Exposure],
            w_flank: w[W::Flank],
            kite: w[W::Kite] >= 0.5 && s.caps.has(Caps::KITE),
            kite_margin: w[W::KiteMargin],
            kite_in: w[W::KiteIn],
            kite_turn: w[W::KiteTurn],
            kite_spare: w[W::KiteSpare],
        }
    }
}

// ------------------------------------------------------------- the scorer

/// A response curve (LR 4): a consideration's input, 0..1, to its
/// value, 0..1. `Linear` is m x + b; `Power` is m x^k + b; `Logistic`
/// is the S at x = c with slope m. Each clamps to 0..1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Curve {
    Linear { m: f32, b: f32 },
    Power { m: f32, k: f32, b: f32 },
    Logistic { m: f32, c: f32 },
}

impl Curve {
    pub fn eval(self, x: f32) -> f32 {
        let x = x.clamp(0.0, 1.0);
        let y = match self {
            Curve::Linear { m, b } => m * x + b,
            Curve::Power { m, k, b } => m * x.powf(k) + b,
            Curve::Logistic { m, c } => 1.0 / (1.0 + (-m * (x - c)).exp()),
        };
        if y.is_finite() {
            y.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// An action's utility (LR 4): its weight times the geometric mean of
/// its considerations, raised by `commit` if it is the intent held now.
/// The geometric mean keeps an action of five considerations comparable
/// with one of two (nine at 0.9 multiply to 0.39, their mean is 0.9);
/// any consideration at zero is a veto.
pub fn utility(weight: f32, considerations: &[f32], current: bool, commit: f32) -> f32 {
    if considerations.is_empty() {
        return 0.0;
    }
    let mut product = 1.0f32;
    for c in considerations {
        let c = c.clamp(0.0, 1.0);
        if c <= 0.0 {
            return 0.0;
        }
        product *= c;
    }
    let mean = product.powf(1.0 / considerations.len() as f32);
    weight.max(0.0) * mean * if current { 1.0 + commit } else { 1.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_are_todays_constants() {
        let s = Settings::default();
        let t = Tactics::from_weights(&s);
        let d = Tactics::default();
        assert_eq!(format!("{:?}", t), format!("{:?}", d), "the weights at their defaults are the Tactics of 17 October");
        assert_eq!(s.caps, Caps::BASELINE);
        assert!(s.caps.has(Caps::KITE) && !s.caps.has(Caps::SMOOTH));
        assert!(s.caps.has(Caps::CASTLE_CAP), "Paul's castle is the default (18 October)");
    }

    #[test]
    fn a_weights_file_round_trips_and_names_its_mistakes() {
        let mut s = Settings::default();
        s.w[W::Wounded] = 1.5;
        s.caps = s.caps.with(Caps::SMOOTH).without(Caps::KITE);
        let back = Settings::parse(&s.to_text()).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.diff(), vec![(W::Wounded, 1.5)]);
        let e = Settings::parse("w_near = 1\nw_nope = 2\n").unwrap_err();
        assert_eq!(e.line, 2);
        assert!(Settings::parse("caps = baseline, -kite, smooth").unwrap().caps == Caps::BASELINE.without(Caps::KITE).with(Caps::SMOOTH));
        assert!(Settings::parse("press_odds = nan").is_err());
    }

    #[test]
    fn the_geometric_mean_compensates_and_vetoes() {
        let nine = [0.9; 9];
        assert!((utility(1.0, &nine, false, 0.1) - 0.9).abs() < 1e-4);
        assert_eq!(utility(1.0, &[0.8, 0.0], false, 0.1), 0.0);
        assert!((utility(2.0, &[0.25, 1.0], true, 0.1) - 2.0 * 0.5 * 1.1).abs() < 1e-4);
        assert_eq!(Curve::Logistic { m: 10.0, c: 0.5 }.eval(0.5), 0.5);
        assert_eq!(Curve::Linear { m: 2.0, b: 0.0 }.eval(0.9), 1.0);
    }

    #[test]
    fn a_slider_steps_and_steps_back() {
        assert_eq!((W::Alpha.step(), W::NearScale.step(), W::ForgetFor.step(), W::Margin.step()), (0.05, 2.0, 10.0, 0.02));
        assert_eq!((W::CastleInside.step(), W::Confirm.step(), W::Kite.step()), (1.0, 1.0, 1.0), "a rounded weight steps by one");
        let mut s = Settings::default();
        s.nudge(W::Alpha, 3);
        assert_eq!(s.w[W::Alpha], 0.55);
        s.nudge(W::Alpha, -3);
        assert_eq!(s.w[W::Alpha], 0.4, "three up and three down is where it was");
        s.nudge(W::Commit, -10);
        assert_eq!(s.w[W::Commit], 0.0, "never under nothing");
        for w in W::ALL {
            assert!(w.step() > 0.0 && w.step() <= w.default_value().max(1.0), "{}: {}", w.name(), w.step());
        }
    }
}
