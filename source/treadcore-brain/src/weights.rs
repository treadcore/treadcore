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
//! ladder = hunt_first
//! caps = smooth, forget, priced_crossing
//! ```
//!
//! A *choice* is a weight that picks one of a function's versions rather
//! than scaling one (Paul, 29 October: "a parallel B style tank brain ...
//! with the ability to toggle between them like weights ... an enum,
//! perhaps 0-3"). Its doc names the versions, `(choice: a | b | c)`, 0
//! first and 0 the brain as it was; the file takes the number or the
//! name; `Settings::choice` reads it rounded and held to the versions.
//! Being a weight, it is read, logged, promoted and searched as one: the
//! fleet's planner tries each version against the defaults (A/B), and a
//! slider steps it by one. A capability is the two-version case, kept as
//! a bit.

use crate::tactics::Tactics;

macro_rules! weights {
    ($($id:ident $name:literal $default:expr, $doc:literal;)*) => {
        /// The name of each weight: its index in `Weights`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(usize)]
        pub enum W { $(#[doc = $doc] $id,)* }

        impl W {
            pub const ALL: &'static [W] = &[$(W::$id,)*];
            /// The weight's name in a weights file and on a slider.
            pub fn name(self) -> &'static str {
                match self { $(W::$id => $name,)* }
            }
            /// The weight's default, the brain's as shipped.
            pub fn default_value(self) -> f32 {
                match self { $(W::$id => $default,)* }
            }
            /// The weight's doc line: what it does, and why its default.
            pub fn doc(self) -> &'static str {
                match self { $(W::$id => $doc,)* }
            }
            /// A choice's versions, 0 first, from its doc's
            /// `(choice: a | b | ...)`; None for a weight that scales.
            pub fn choices(self) -> Option<Vec<&'static str>> {
                let d = self.doc();
                let rest = &d[d.find("(choice: ")? + 9..];
                Some(rest[..rest.find(')')?].split('|').map(str::trim).collect())
            }
            /// The weight of that name, if any.
            pub fn by_name(name: &str) -> Option<W> {
                match name { $($name => Some(W::$id),)* _ => None }
            }
            /// A slider's step (the coordination screen, backlog 15): one
            /// for a rounded weight, else a tenth of the default taken
            /// to 1, 2 or 5 of its power of ten; a tenth for nothing.
            pub fn step(self) -> f32 {
                if self.doc().contains("(rounded)") || self.choices().is_some() {
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
    Near "w_near" 2.0, "target: worth of nearness, w_near / (1 + d / near_scale)";
    NearScale "near_scale" 30.0, "target: the distance scale of nearness, tiles";
    Wounded "w_wounded" 1.2, "target: worth of its wounds";
    Finish "w_finish" 0.552896, "target: one direct shot from dead";
    Boat "w_boat" 2.0, "target: afloat, a boat sinks at one hit";
    Called "w_called" 0.8, "target: a teammate pinged it";
    CalledFor "called_for" 15.0, "target: seconds a ping stays a call";
    Focus "w_focus" 0.5, "target: each teammate already on it";
    FocusCap "focus_cap" 2.0, "target: teammates on it before overkill (rounded)";
    Overkill "w_overkill" 0.6, "target: off for each teammate past the cap";
    InRange "w_in_range" 1.4, "target: in direct range now";
    Threat "w_threat" 0.6, "target: menacing a teammate or the base";
    BaseR "base_r" 6.15272, "target: the base's reach for w_threat, tiles";
    Stick "w_stick" 0.322391, "target: the one already chosen";
    FocusBlend "focus_blend" 0.25, "target (LR 117): 0 the near tank in range, 1 the wounded one a teammate is on; 0.5 both as before (each side's terms doubled toward its end)";
    // Tactics: the engagement and assignment.
    OddsR "odds_r" 14.0, "odds: tanks this near the fight count, tiles";
    PillboxWorth "pillbox_worth" 0.7, "odds: a hostile pillbox as a share of a tank";
    StrikeWorth "strike_worth" 0.35, "odds: a hostile strike drone as a share of a tank";
    EngageOdds "engage_odds" 0.353899, "odds: below these, with help coming, mass";
    PressOdds "press_odds" 1.33767, "odds: above these, close in";
    ComingR "coming_r" 221.036, "odds: teammates this near are coming, tiles";
    MassFor "mass_for" 12.0, "odds: seconds to hold off for them";
    AnswerCap "answer_cap" 3.0, "pings: this many nearer teammates answer instead (rounded)";
    CrewReach "crew_reach" 40.0, "pings: a crew on a job answers only this near, tiles";
    Exposure "w_exposure" 0.605073, "stand: cost per gun covering a firing point";
    Flank "w_flank" 0.3, "stand: cost of leaving the flank's bearing";
    Kite "kite" 1.0, "kite: on (1) or off (0) (rounded)";
    KiteMargin "kite_margin" 1.5, "kite: the waiting ring past the target's reach, tiles";
    KiteIn "kite_in" 2.25, "kite: the ring a step in ends on, tiles";
    KiteTurn "kite_turn" 0.8, "kite: seconds a step in takes beyond the drive";
    KiteSpare "kite_spare" 0.4, "kite: seconds of the target's reload to spare";
    // Smoothing (LR 7).
    // `smooth` as a weight (LR 46; Paul, 20 October: "give it an
    // insignificant coefficient"), once the capability `smooth`.
    SmoothShare "w_smooth" 0.05, "smooth: how much the bot smooths, 0 to 1. The move's desire vector and the retreat thresholds blend by it; the rung's and the turret's confirmation, the stance's advance scalar and resupply by the doctrine alone act above 0.5 (yes or no). 0 is the capability off, 1 on";
    Alpha "alpha" 0.179532, "smooth: the move's desire vector, share of the new pull a think";
    Beta "beta" 0.304865, "smooth: the advance scalar, share of the new pull a think";
    Margin "margin" 0.15, "smooth: a challenger must beat the intent by this share";
    Confirm "confirm" 2.0, "smooth: thinks in a row a challenger must win (rounded)";
    ConfirmTurret "confirm_turret" 3.0, "smooth: picks in a row before the turret changes target (rounded)";
    Commit "commit" 0.1, "smooth: the intent's commitment bonus";
    AdvanceBand "advance_band" 0.652128, "smooth: the advance scalar presses above this and falls back below minus this";
    Jitter "jitter" 3.0, "smooth: a moving goal's shift under this is smoothed, over it taken at once, tiles";
    RetreatArmor "retreat_armor" 50.0, "survival: armour at or under this goes home (blended from the constant by `w_smooth`, 0.05: at the defaults it barely acts, LR 86)";
    RetreatShells "retreat_shells" 4.0, "survival: shells at or under this go home (blended from the constant 4 by `w_smooth`, 0.05: at the defaults it barely acts, LR 86)";
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
    SitePoor "site_poor" 0.5, "island: island_site moves the base off the nearest depot only when that depot's site scores under this (`site_score`, -1 to 1); 2 (before LR 73) always, so on a map of continents a slightly better site an ocean away won";
    SiteReach "site_reach" 30.0, "island: island_site's better site lies within this many tiles of the nearest depot; 1000 (before LR 73) anywhere, and on atlantic and europe the bases went 70 to 275 tiles off";
    SiteBeach "w_site_beach" 20.0, "island: tiles of a boat's run a beach's island is worth per point of its site score (`site_score`, -1 a small bare islet to 1 a large forested island), while the side holds no depot (a naval start); 0 (before LR 62) the beach by its run and danger alone; needs island_site";
    // Formation (LR 6.3).
    ColumnGap "column_gap" 2.0, "column: tiles between tanks in file on a bridge";
    ColumnPast "column_past" 2.0, "column: the wedge comes back this far past the bridge, tiles";
    // The team (LR 9).
    SwapGap "swap_gap" 0.207136, "rotation: readiness gap between a hot and a cold point that swaps two tanks";
    BotStyle "bot_style" 0.0, "brain: which named style a team plays, read when its bots are made, so a match's next (choice: fleet | raider | castle | expander | timid | overrider | scholar | duelist | rome | china | mongolia | shoshone | venice | japan | polynesia | assyria | quartermaster): styles.rs; 8 to 15 are the personas of 2 November, 16 the learned tank with its squad";
    Model "model" 0.0, "brain: a team's brain, 0 the rules, k a learned model in the fleet's slot k (choice: rules | slot1 | slot2 | slot3 | slot4 | slot5 | slot6 | slot7): the census reads CENSUS_MODELS/k.bin, orrery names each slot by its generation (models.tsv)";
    Roles "roles" 0.0, "team: a bots' team of five after its leader (choice: as_built | no_guard | two_guards | three_guards): as built a guard, two rangers and a wing; no guard sends the guard out with the fleet; two and three guards keep more at the base (fleet::MIXES)";
    PingStyle "ping_style" 0.0, "pings: what a team calls and answers (choice: as_built | help | warn | defend | all): as built the switch `pings` decides and every call is help; help calls for help when outnumbered and comes when called; warn warns off and takes warnings and cautions; defend calls and comes only for a base under attack; all of them, and it says what it saw";
    PingHelpReach "ping_help_reach" 60.0, "pings: a call for help is answered this near, tiles";
    PingDefendReach "ping_defend_reach" 90.0, "pings: a call to defend is answered this near, tiles";
    PingDefendHold "ping_defend_hold" 1.5, "pings: a defence is held this many times as long as a rally";
    PingWarnReach "ping_warn_reach" 25.0, "pings: a warning moves a tank this near it, tiles";
    PingWarnFlee "ping_warn_flee" 15.0, "pings: a warned tank moves this far away, tiles";
    PingCautionReach "ping_caution_reach" 20.0, "pings: a rally this near a caution is dropped, tiles";
    PingOutnumbered "ping_outnumbered" 1.5, "pings: hostile tanks over ours by this ratio is a call for help";
    HomePick "home_pick" 1.0, "resupply: which base a tank goes home to (choice: nearest | stocked): stocked by shells over the road, front.rs; a depot's store can run dry, so the nearest may be empty";
    AdvancePlan "advance_plan" 1.0, "advance: after taking a pillbox or base (choice: as_built | forward): forward calls for help, chains it, drags it on, cuts wood and lays road back";
    MinePlan "mine_plan" 1.0, "mines: where a squad lays them (choice: as_built | line | raid): as built on the base's approach; line across the battle line's road, front.rs; raid that, and a tank inside an enemy base lays one and runs home";
    Bio "bio" 0.0, "intuition: the shape of a tank's slow waves (choice: off | sine | triangle | square | saw): bio.rs";
    BioPeriod "bio_period" 60.0, "intuition: the first wave's period, seconds; the others run 28/23 and 33/23 as long";
    BioAmount "bio_amount" 0.25, "intuition: how far the waves swing how bold a tank is, a share of engage_odds";
    Patch "patch" 0.0, "patch: a learned transform over the rules' Move (choice: off | slot1 | slot2 | slot3 | slot4 | slot5 | slot6 | slot7): patch.rs, CENSUS_PATCHES/k.bin";
    PatchConfidence "patch_confidence" 0.5, "patch: it overrides the rules only at this confidence or more";
    PatchReach "patch_reach" 10.0, "patch: at full output it moves the rules' point this far, tiles";
    Ladder "ladder" 0.0, "auto: the ladder's order, defend base first in each (choice: as_built | expand_before_hunt | hunt_first | expand_first): as built, fortify, turrets, hunt, expand, guard (Rung::order)";
    Posture "w_posture" 0.95, "posture: the team's posture against the ladder's fixed order, 0 the order alone; a leaned rung climbs past fewer than 5 w / (1 - w) rungs above it, so from 0.5 the posture decides among the rungs that hold (LR 24)";
    PostureScored "posture_scored" 1.0, "posture: the five scores (1) against the threshold ladder of 12e (0), each posture's worth the blend of the two (LR 25)";
    PostureDefend "w_pos_defend" 3.0, "posture: worth of Defend's score, hostile worth within the base's alarm, half a tank full";
    PostureEconomy "w_pos_economy" 1.0, "posture: worth of Economy's score, the team's material spent, or a starved plan";
    PostureFortify "w_pos_fortify" 0.1875, "posture: worth of Fortify's score, the share of the castle not yet built";
    PostureConnect "w_pos_connect" 1.0, "posture: worth of Connect's score, the share of pillbox slots and road posts still empty";
    DefendGround "w_defend_ground" 3.0, "posture: hostile worth on our held ground (the board's territory of pillboxes and depots alone) beyond the base's alarm, counted toward Defend at this share, as the base's own threat is (LR 68); 0 the base alone (LR 24)";
    ConnectIdle "connect_idle" 1.0, "posture: how much of Connect's score stands when this tank has no pillbox to bring in: 1 as built, 0 a score only for work that can be done";
    PostureExpand "w_pos_expand" 0.4, "posture: worth of Expand's score, the team's odds against what it sees, 1 at press_odds";
    // The dodge (lib.rs `Dodge`, LR 27): its numbers, once constants.
    DodgeHorizon "dodge_horizon" 1.08151, "dodge: seconds before a shell lands that the tank starts out from under it";
    DodgeMargin "dodge_margin" 0.8, "dodge: tiles past the splash a shell's landing still counts as on the tank";
    DodgeClear "dodge_clear" 2.5, "dodge: tiles past the splash the tank drives to, across the shell's line";
    DodgeHold "dodge_hold" 0.0, "dodge: the share of the shell's flight the tank stays out, arrived or not (1), against only while it drives (0, before LR 27)";
    DodgeRelease "dodge_release" 1.0, "dodge: once at its point before the shell lands, an errand that would drive the tank back into the splash waits (1, the default since LR 54, Paul's choice), against every errand at once (0, before LR 53); thresholded at 0.5";
    DodgeEvery "dodge_every" 1.0, "dodge: the plan and the release against every hostile shell landing within the horizon, each at its own landing (1, LR 56), against the one shell dodged (0, LR 53); thresholded at 0.5";
    DodgeFire "dodge_fire" 1.0, "dodge: with a hostile shell landing within the horizon, a shot whose turn of the hull (a tank has no turret) would carry the tank into the splash waits (1, LR 57), against the guns free (0); thresholded at 0.5";
    DodgeOwnRoute "dodge_own_route" 1.0, "dodge: the forecast of a drive (`drive_path`) ashore to where the tank is already going follows the tank's own route, the one the world will drive, with no search (1, the default since LR 93: a census faster, the same play within the noise; afloat it searches), against a route searched afresh every tick under fire (0, before LR 93); thresholded at 0.5";
    DodgePlan "dodge_plan" 1.0, "dodge: the point by where the tank's own turn and speed will have it when the shell lands (1, the default since LR 54, Paul's choice), against across the shell's line always (0, before LR 53); thresholded at 0.5";
    // The run for the beach (lib.rs `RuleBot::landfall_order`, LR 31).
    // On by default since LR 38 (Paul, 19 October: "yes all three on"):
    // together +2.31 a seat over 48 seeds (LR 36, 37).
    LandfallShy "landfall_shy" 1.0, "landfall: at 1, each tile of a boat's run in a hostile gun's reach costs as much as 4 tiles more to sail; 0 (before LR 38) prices only the beach and the heading";
    LandfallWait "landfall_wait" 10.0, "landfall: seconds of a crossing a boat stands off a beach a seen hostile tank covers, out of its reach, before it lands anyway; 0 (before LR 38) lands at once";
    LandfallMemory "landfall_memory" 30.0, "landfall: seconds a hostile tank out of sight is kept where it was last seen, for the beach, the run and the stand-off; 0 (before LR 38) keeps none";
    // The base cycle (LR 6.1).
    GiveItem "w_give_item" 1.0, "cycle: one plan item for the base on each fill";
    GiveUpgrade "w_give_upgrade" 1.0, "cycle: one improvement for the base on each fill";
    Connect "w_connect" 1.0, "cycle: road tiles toward the next friendly base or pillbox";
    ConnectTiles "connect_tiles" 5.12027, "cycle: road tiles laid on one visit at most";
    // Paul's answers of 18 October as scalars (ROADMAP 12h, LR 10).
    Mend "w_mend" 0.5, "improvement: worth of mending the most hurt pillbox of the base";
    Mine "w_mine" 0.860887, "improvement: worth of a mine on the moat";
    Works "w_works" 0.5, "improvement: worth of works on a pad by the base";
    // GAME 6, THE ORE (Paul, 2 November: "a fun zero-sum economy, where every mineral but the growing trees
    // is finite"). A smelter by the ore is an improvement the cycle offers at w_works; the ore run is a
    // crew's trip when the side's shells run low. 0 turns the run off.
    OreLow "ore_low" 10.0, "ore (game 6): a crew fetches ore when the side's nearest depot holds fewer shells than this; 0 never";
    OreReach "ore_reach" 40.0, "ore (game 6): a deposit within this of the base is worth a smelter and a run, tiles";
    // GAME 7, THE LOGISTICS OF A BASE: oil runs when the refinery is short, the city the guard builds as
    // quartermaster, and the first thing supply buys.
    OilLow "oil_low" 4.0, "oil (game 7): a crew fetches oil when the side's refinery holds less than this; 0 never";
    CityPlan "city_plan" 3.0, "city (game 7): how far the base's guard builds the city when quiet (choice: none | camp | town | works | city): a camp is the castle and the cycle; a town adds a harvester; the works the stores, the road to the smelter and a refinery; the city a data centre and a workshop";
    UpStores "w_up_stores" 1.0, "city (game 7): worth of the depot's stores at 60 supply; 0 never buys";
    Skill "skill" 1.0, "skill: how well a bot plays (PLAYABILITY item 9, LR 67), 1 in full, 0 a rookie's: the aim off by (1 - skill) of a rookie's scatter and the lead taken by skill; under 0.5 no dodge, kite or storm and one interceptor on station; drones held (1 - skill) of a rookie's 90 s; a plain RuleBot thinks every quarter to half second";
    UpgradeVsMend "upgrade_vs_mend" 0.5, "improvement: share of the slot for 11g's upgrades once they exist, against mend, mines and works";
    CastleInside "castle_inside" 2.0, "castle: pillboxes inside the wall, then it stops filling (rounded)";
    CastleRoad "castle_road" 2.0, "castle: pillboxes outside on its road (rounded)";
    PostSpacing "post_spacing" 31.8699, "posts: tiles between pillboxes on a road between bases";
    PostsMax "posts_max" 6.0, "posts: road posts a base fills at most (rounded)";
    JunctionClear "junction_clear" 3.0, "posts: no post this near a road junction, tiles";
    ConnectBases "connect_bases" 0.75, "connect: a base or castle preferred to a lone pillbox, 0 none, 1 its distance counted at nothing";
    BridgeKeep "bridge_keep" 90.0, "bridge jobs: seconds a part-laid bridge stays on the team board with no tile laid";
    BridgeReach "bridge_reach" 40.0, "bridge jobs: a teammate this near its next tile takes it, tiles";
    BridgeWing "bridge_wing" 1.0, "bridge jobs: a wingman with material takes one too, leaving its leader's wedge (1), or only tanks on auto (0) (rounded)";
    Duck "w_duck" 1.0, "duck: safety against the errand; a tank ducks with its armour share at or under this, 1 always, 0 never";
    DuckMemory "duck_memory" 2.5, "duck: seconds a hostile gun seen is remembered where it was, its reload counted down (sight and direct range are both 10 tiles, so a gun at the edge is often unseen)";
    DuckPlan "duck_plan" 1.0, "duck: the point held while it stays safe, else the one `drive_to` has out of the gun's reach or line by the tick it is ready (1, LR 57), against the nearest safe point to the errand, picked anew each think (0, LR 21); thresholded at 0.5";
    DuckReady "duck_ready" 1.5, "duck: a hostile gun ready within this many seconds threatens, if it fires before mine (a tank covers about five tiles in 1.5 s)";
    Front "w_front" 0.0, "front (0, the default since LR 75): Expand's depot by its distance to the team board's front (1, LR 58) against its distance to the base (0, before LR 58), the cost the blend of the two; a depot so chosen is kept until taken or given up; needs board";
    FrontPost "w_front_post" 0.0, "front (0, the default since LR 75): the guard's post moved this share of the way toward the nearest front sector (1, LR 61), `FRONT_POST_REACH` tiles at most, against the post at the base (0, before LR 61); a guard on rotation leaves its post, so it acts only with a raider at the base; needs board";
    FrontPush "w_front_push" 0.0, "front (0, the default since LR 75): the rotation's push point moved this share of the way toward the front sector nearest it (1, LR 61), against halfway to the side's objective (0, before LR 61); needs board";
    FrontMemory "front_memory" 30.0, "front: seconds a hostile tank out of sight still counts toward the board's front where it was last seen; 0 (before LR 58) counts only what is in sight";
    FetchGather "fetch_gather" 20.0, "escort: a fetch trip farther than this from the base gathers the fleet first, tiles";
    LongGame "long_game" 0.5, "score: 0 judges the win and kills, 1 the ground held over time";
    ShellBoxes "shell_boxes" 1.0, "spending (LR 86, the cowboy, the default since): 1 shells a hostile pillbox in the indirect gun's reach whenever the gun is ready and no tank is shot at, 0 only those guarding a stormed depot (before LR 86); thresholded at 0.5";
    LongshotEvery "longshot_every" 1.25, "spending: seconds between a tank's longshots (5 since LR 86, 30 before)";
    LongshotAny "longshot_any" 1.0, "spending (1 since LR 86): 1 a longshot with no hostile structure to hit goes at the nearest hostile pillbox, else the target tank; 0 at structures only (before LR 86); thresholded at 0.5";
    StrikesMax "strikes_max" 3.30238, "spending: a tank's strike drones out at once (3 since LR 86, 1 before, rounded)";
    Assist "assist" 1.0, "roles: a person's team gets a builder at 0.5 and over, else is dealt as a bots' team";
    TemperEngage1 "temper_engage_1" 0.0, "temper: how much the 60 s wave moves engage_odds, the bar under which a tank holds off, a share of it; bio.rs";
    TemperEngage2 "temper_engage_2" 0.0, "temper: how much the 73 s wave moves engage_odds, the bar under which a tank holds off, a share of it; bio.rs";
    TemperEngage3 "temper_engage_3" 0.0, "temper: how much the 86 s wave moves engage_odds, the bar under which a tank holds off, a share of it; bio.rs";
    TemperPress1 "temper_press_1" 0.0, "temper: how much the 60 s wave moves press_odds, the bar over which it closes, a share of it; bio.rs";
    TemperPress2 "temper_press_2" 0.0, "temper: how much the 73 s wave moves press_odds, the bar over which it closes, a share of it; bio.rs";
    TemperPress3 "temper_press_3" 0.0, "temper: how much the 86 s wave moves press_odds, the bar over which it closes, a share of it; bio.rs";
    TemperRetreat1 "temper_retreat_1" 0.0, "temper: how much the 60 s wave moves retreat_armor, the armour it goes home at, a share of it; bio.rs";
    TemperRetreat2 "temper_retreat_2" 0.0, "temper: how much the 73 s wave moves retreat_armor, the armour it goes home at, a share of it; bio.rs";
    TemperRetreat3 "temper_retreat_3" 0.0, "temper: how much the 86 s wave moves retreat_armor, the armour it goes home at, a share of it; bio.rs";
    TemperTrigger1 "temper_trigger_1" 0.0, "temper: how much the 60 s wave moves longshot_every, the seconds between its longshots, a share of it; bio.rs";
    TemperTrigger2 "temper_trigger_2" 0.0, "temper: how much the 73 s wave moves longshot_every, the seconds between its longshots, a share of it; bio.rs";
    TemperTrigger3 "temper_trigger_3" 0.0, "temper: how much the 86 s wave moves longshot_every, the seconds between its longshots, a share of it; bio.rs";
    CrewStay "crew_stay" 1.0, "crew: who keeps building while the castle is unfinished, whatever the leader does (choice: none | guard | guard_one | guard_two): crew.rs";
    CycleAll "cycle_all" 1.0, "cycle: who gives the base something after filling up (choice: leader_and_guard | every_tank): cycle.rs";
    // THE WINGMAN'S NUMBERS WERE CONSTANTS (Paul, 2 November: "a wingman ... following my moves, like an
    // escort mission"). A person's two wingmen kept station by five numbers nobody could move: not the
    // person, not a persona, not the search. As weights they are sliders on the U page and dials the
    // census reads; the defaults are the constants, so nothing plays differently until someone moves one.
    EscortDistance "escort_distance" crate::commander::ESCORT_DISTANCE, "wingman: the station behind the leader, tiles";
    WingLeash "wing_leash" crate::commander::WING_LEASH, "wingman: nothing takes it further than this from its leader, tiles";
    WingGreed "wing_greed" crate::commander::WING_GREED, "wingman: a pillbox or depot this near the leader is worth taking, tiles";
    WingThreat "wing_threat" crate::commander::WING_THREAT, "wingman: anything hostile this near the leader is its first job, tiles";
    WingAhead "wing_ahead" crate::commander::WING_AHEAD, "wingman: it hits what is ahead of the leader within this, tiles";
    WingStance "wing_stance" 0.0, "wingman: where its station is (choice: shadow | flank | screen): shadow behind the leader as built; flank off its side, each wingman its own side; screen ahead of it";
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
    PINGS 1 "pings" false, "call enemies and answer teammates' pings";
    DODGE 2 "dodge" true, "step out from under a shell";
    BRIDGES 3 "bridges" true, "lay bridges (hop.rs)";
    ROTATE 4 "rotate" true, "rotate between the push and defence points";
    FORGET 8 "forget" true, "given-up targets are tried again after forget_for (12b)";
    BOARD 10 "board" true, "the team board: one picture of the map per team (12e)";
    TEAM_ROTATE 11 "team_rotate" false, "swap a hurt tank at a hot point for a fresh one at a cold point (12e); off since LR 76, +0.55 +/- 0.15 a seat off over 400 fresh seeds";
    PRICED_CROSSING 12 "priced_crossing" true, "boat or bridge by price, long bridges and scarce trees costing more (12f2)";
    COLUMN 13 "column" true, "a fleet crosses a bridge in file (12f3)";
    CYCLE 14 "cycle" true, "after filling up, give the base an item and an improvement and road out (12f1)";
    ISLAND_SITE 15 "island_site" true, "a small island with few trees is a poor base (12f2)";
    ESCORT 17 "escort" true, "a fleet leader's far fetch trip gathers its wingmen first, so the fleet escorts the pillbox home along the road (12f)";
    BRIDGE_JOBS 18 "bridge_jobs" true, "a part-laid bridge is a job on the team board, taken by the next teammate with material; needs board (12f)";
    DUCK 19 "duck" true, "out of a hostile gun's range or line when it is ready before mine: direct fire denied, since it cannot be dodged once fired (12d, LR 19)";
    POSTURE 20 "posture" true, "the team board's posture leans the Auto ladder by w_posture; needs board (12e, LR 24)";
    CASTLE_CAP 16 "castle_cap" true, "the castle stops at castle_inside and castle_road pillboxes; the rest go to posts on the roads between bases (12h)";
    FOCUS_BLEND 21 "focus_blend" true, "the target is chosen by focus_blend, between the near tank in range and the wounded one a teammate is on (LR 117); off, 0.5, as before";
}

impl Default for Caps {
    fn default() -> Caps {
        Caps::BASELINE
    }
}

impl Caps {
    /// Every bit of `c` is on.
    pub fn has(self, c: Caps) -> bool {
        self.0 & c.0 == c.0
    }
    /// These, and `c` on.
    pub fn with(self, c: Caps) -> Caps {
        Caps(self.0 | c.0)
    }
    /// These, and `c` off.
    pub fn without(self, c: Caps) -> Caps {
        Caps(self.0 & !c.0)
    }
    /// Every capability, the new ones too.
    pub fn all() -> Caps {
        Caps(Caps::NAMES.iter().fold(0, |a, (_, c)| a | c.0))
    }
    /// The capability of that name, as a weights file spells it.
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

/// RANGES' lines, as (name, min, max). A bad line is a test's news (`every_range_reads`).
fn ranges() -> impl Iterator<Item = (&'static str, f32, f32)> {
    RANGES.lines().filter_map(|raw| {
        let mut f = raw.split('#').next()?.split_whitespace();
        Some((f.next()?, f.next()?.parse().ok()?, f.next()?.parse().ok()?))
    })
}

/// A brain's settings: the weights and the capabilities.
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub w: Weights,
    pub caps: Caps,
}

impl Default for Settings {
    fn default() -> Settings {
        let mut s = Settings { w: Weights::default(), caps: Caps::default() };
        s.hold();
        s
    }
}

/// Every number's limits (`RANGES`), compiled in: a binary carries its rules.
const RANGES: &str = include_str!("../RANGES");

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
                    // `smooth` was a capability before LR 46: now the
                    // weight at 1 (on) or 0 (off).
                    if c == "smooth" || c == "-smooth" {
                        s.w[W::SmoothShare] = if c == "smooth" { 1.0 } else { 0.0 };
                        continue;
                    }
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
            // A choice by its version's name as well as its number.
            let named = w.choices().and_then(|c| c.iter().position(|v| *v == value)).map(|i| i as f32);
            let v: f32 = match named {
                Some(v) => v,
                None => value.parse().map_err(|_| bad(format!("{} is not a number{}", value,
                    w.choices().map(|c| format!(" or one of {}", c.join(", "))).unwrap_or_default())))?,
            };
            if !v.is_finite() {
                return Err(bad(format!("{} is not finite", value)));
            }
            s.w[w] = v;
        }
        s.hold();
        Ok(s)
    }

    /// Holds every number to its range, whatever came before (Paul, 30 October: "Numbers are
    /// not adjusted beyond their max or under their min range"). A decree is min = max.
    pub fn hold(&mut self) {
        for (name, min, max) in ranges() {
            if let Some(w) = W::by_name(name) {
                self.w[w] = self.w[w].clamp(min, max);
            } else if let (Some(c), true) = (Caps::by_name(name), min == max) {
                self.caps = if min >= 1.0 { self.caps.with(c) } else { self.caps.without(c) };
            }
        }
    }

    /// A choice's version: its weight rounded, held to the versions it has
    /// (0 for a weight that is not a choice and below a half).
    pub fn choice(&self, w: W) -> usize {
        let n = w.choices().map(|c| c.len()).unwrap_or(1);
        (self.w[w].round().max(0.0) as usize).min(n - 1)
    }

    /// How much the bot smooths (`w_smooth`), 0 to 1.
    pub fn smooth(&self) -> f32 {
        self.w[W::SmoothShare].clamp(0.0, 1.0)
    }

    /// The share of a move's new pull taken a think: all of it at
    /// `w_smooth` 0, `alpha` at 1, in a line between.
    pub fn smooth_alpha(&self) -> f32 {
        1.0 - self.smooth() * (1.0 - self.w[W::Alpha])
    }

    /// The yes-or-no parts of smoothing: on above 0.5.
    pub fn smooth_on(&self) -> bool {
        self.smooth() > 0.5
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
            // Off, the blend sits at 0.5: the target is scored as before LR 117.
            focus_blend: if s.caps.has(Caps::FOCUS_BLEND) { w[W::FocusBlend] } else { 0.5 },
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
            shell_boxes: w[W::ShellBoxes] >= 0.5,
            longshot_every: w[W::LongshotEvery],
            longshot_any: w[W::LongshotAny] >= 0.5,
            strikes_max: w[W::StrikesMax].round().max(0.0) as usize,
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
    /// The curve at `x`, clamped to 0..1 both ways; a value that is not
    /// finite reads 0.
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
    fn every_range_reads_and_holds_its_own_default() {
        let mut n = 0;
        for raw in RANGES.lines() {
            let line = raw.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            n += 1;
            let f: Vec<&str> = line.split_whitespace().collect();
            assert_eq!(f.len(), 3, "{}", line);
            let (min, max): (f32, f32) = (f[1].parse().unwrap(), f[2].parse().unwrap());
            assert!(min <= max, "{}", line);
            match W::by_name(f[0]) {
                Some(w) => assert!(w.default_value() >= min && w.default_value() <= max, "{}: default {}", line, w.default_value()),
                None => assert!(Caps::by_name(f[0]).is_some() && min == max, "{}", line),
            }
        }
        assert_eq!(ranges().count(), n);
    }

    #[test]
    fn no_trial_can_move_a_decreed_value() {
        let s = Settings::parse("castle_inside = 8\nkite_in = 9").unwrap();
        assert_eq!(s.w[W::CastleInside], 2.0, "a decree held");
        assert_eq!(s.w[W::KiteIn], 9.0, "the rest move as they did");
        assert_eq!(Settings::default().w[W::CastleInside], 2.0);
        assert_eq!(Settings::parse("alpha = 7").unwrap().w[W::Alpha], 1.0, "held to its max, not refused");
    }

    #[test]
    fn default_weights_are_todays_constants() {
        let s = Settings::default();
        let t = Tactics::from_weights(&s);
        let d = Tactics::default();
        assert_eq!(format!("{:?}", t), format!("{:?}", d), "the weights at their defaults are the Tactics of 17 October");
        assert_eq!(s.caps, Caps::BASELINE);
        assert!(s.caps.has(Caps::KITE) && !s.smooth_on());
        assert!(s.caps.has(Caps::CASTLE_CAP), "Paul's castle is the default (18 October)");
    }

    #[test]
    fn a_weights_file_round_trips_and_names_its_mistakes() {
        let mut s = Settings::default();
        s.w[W::Wounded] = 1.5;
        s.w[W::SmoothShare] = 1.0;
        s.caps = s.caps.without(Caps::KITE);
        let back = Settings::parse(&s.to_text()).unwrap();
        assert_eq!(back, s);
        assert_eq!(back.diff(), vec![(W::Wounded, 1.5), (W::SmoothShare, 1.0)]);
        let e = Settings::parse("w_near = 1\nw_nope = 2\n").unwrap_err();
        assert_eq!(e.line, 2);
        // `smooth`, a capability before LR 46, reads as the weight.
        let old = Settings::parse("caps = baseline, -kite, smooth").unwrap();
        assert!(old.caps == Caps::BASELINE.without(Caps::KITE) && old.smooth_on());
        assert_eq!(Settings::parse("caps = baseline, -smooth").unwrap().smooth(), 0.0);
        assert!(Settings::parse("press_odds = nan").is_err());
    }

    /// `w_smooth` (LR 46): the default is near off, the desire vector
    /// blends in a line, and the yes-or-no parts turn on above 0.5.
    #[test]
    fn smooth_is_a_weight_that_blends_and_thresholds() {
        let mut s = Settings::default();
        // The numbers this test was written for: the fleet moves the defaults.
        s.w[W::SmoothShare] = 0.05;
        s.w[W::Alpha] = 0.4;
        assert!((s.smooth() - 0.05).abs() < 1e-6 && !s.smooth_on());
        assert!((s.smooth_alpha() - 0.97).abs() < 1e-6, "0.05 of the way from 1 to alpha 0.4");
        s.w[W::SmoothShare] = 0.0;
        assert_eq!(s.smooth_alpha(), 1.0);
        s.w[W::SmoothShare] = 0.5;
        assert!(!s.smooth_on(), "0.5 itself is off");
        s.w[W::SmoothShare] = 0.51;
        assert!(s.smooth_on());
        s.w[W::SmoothShare] = 3.0;
        assert_eq!(s.smooth(), 1.0, "clamped to 1");
        assert!((s.smooth_alpha() - s.w[W::Alpha]).abs() < 1e-6);
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
        // A step is a tenth of the default, rounded to a nice number: the fleet moves the defaults.
        for w in [W::Alpha, W::NearScale, W::ForgetFor, W::Margin] {
            let (step, d) = (w.step(), w.default_value().abs());
            assert!(step >= d / 20.0 && step <= d / 5.0, "{}: {} on {}", w.name(), step, d);
        }
        assert_eq!((W::CastleInside.step(), W::Confirm.step(), W::Kite.step()), (1.0, 1.0, 1.0), "a rounded weight steps by one");
        let mut s = Settings::default();
        let (start, step) = (s.w[W::Alpha], W::Alpha.step());
        s.nudge(W::Alpha, 3);
        // A nudge rounds to four places, so a default from the search comes back within one.
        assert!((s.w[W::Alpha] - (start + 3.0 * step)).abs() < 2e-4);
        s.nudge(W::Alpha, -3);
        assert!((s.w[W::Alpha] - start).abs() < 2e-4, "three up and three down is where it was");
        s.nudge(W::Commit, -1000);
        assert_eq!(s.w[W::Commit], 0.0, "never under nothing");
        for w in W::ALL {
            assert!(w.step() > 0.0 && w.step() <= w.default_value().max(1.0), "{}: {}", w.name(), w.step());
        }
    }

    /// THE PINNED-TEST LINT (the plan of 2 November, chapter 15). Half of all promotions for three
    /// days failed on tests that read `Settings::default()` or `Tactics::default()` and asserted on
    /// what they found, so every move of a searchable number broke a test that had nothing to say
    /// about it. A test that uses the defaults in an assertion must name the weights it tests
    /// (`s.w[W::X] = ...` or `Settings::parse("x = ...")`). The sixteen that were written before the
    /// rule are listed; a new one fails here, by name, before it can fail a promotion.
    #[test]
    fn a_test_that_uses_the_defaults_names_the_weights_it_tests() {
        const BEFORE_THE_RULE: &[&str] = &[
            "the_team_defends_a_threat_at_the_base_not_far_off", "a_hostile_on_the_held_ground_is_an_incursion", "the_front_is_where_influence_crosses_zero", "a_hurt_tank_swaps_out",
            "a_far_fetch_gathers_the_fleet", "a_teammate_finishes_a_part_laid_bridge", "a_switch_off_before_the_deal_is_off_after_it", "a_full_tank_gives_the_base_an_item",
            "the_islands_are_counted", "a_small_island_takes_a_boat", "the_default_match_plays_as_teams", "a_hop_bridges_a_narrow_gap_and_sails_a_wide_one",
            "every_style_plays_a_match_to_its_end_without_a_fault", "focus_fire_picks_the_wounded_tank_a_teammate_is_on", "the_square_law", "default_weights_are_todays_constants",
        ];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut offenders = Vec::new();
        let mut seen = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let path = e.unwrap().path();
            if path.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&path).unwrap();
            for body in src.split("#[test]").skip(1) {
                let name = body.trim_start().trim_start_matches("#[ignore]").trim_start().strip_prefix("fn ").and_then(|r| r.split(['(', '<']).next()).unwrap_or("").to_string();
                if name.is_empty() {
                    continue;
                }
                seen += 1;
                let uses = body.contains("Settings::default()") || body.contains("Tactics::default()");
                let names = body.contains(".w[W::") || body.contains(".w[crate::weights::W::") || body.contains("Settings::parse(");
                if uses && body.contains("assert") && !names && !BEFORE_THE_RULE.contains(&name.as_str()) {
                    offenders.push(format!("{}::{}", path.file_name().unwrap().to_string_lossy(), name));
                }
            }
        }
        assert!(seen > 100, "{} tests read", seen);
        assert!(offenders.is_empty(), "a test that uses the defaults in an assertion must name the weights it tests, or it pins the search: {}", offenders.join(", "));
    }

    #[test]
    fn every_profile_reads_and_differs() {
        // The play styles of backlog 60 (`profiles/`): each one parses,
        // and each moves the defaults somewhere.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profiles");
        let mut n = 0;
        for e in std::fs::read_dir(&dir).unwrap() {
            let path = e.unwrap().path();
            if path.extension().and_then(|x| x.to_str()) != Some("weights") { continue; }
            let s = Settings::parse(&std::fs::read_to_string(&path).unwrap()).unwrap_or_else(|e| panic!("{}: {}", path.display(), e));
            assert!(W::ALL.iter().any(|&w| s.w[w] != w.default_value()), "{}: the defaults", path.display());
            n += 1;
        }
        assert!(n >= 4, "{} profiles", n);
    }
}
