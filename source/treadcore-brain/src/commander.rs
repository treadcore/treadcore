//! The Commander (roadmap step 11a): a Brain for the player's own seat.
//!
//! A person commands the tank with a goal instead of driving it: hunt
//! pillboxes, defend a pillbox, defend a depot, escort an ally, or hold.
//! The Commander drives, aims and sends the squad toward the goal; a
//! RuleBot inside it keeps the doctrine it already has for drones and
//! for shooting at tanks, and the Commander keeps only those of its
//! orders that serve the goal. It sees what the player sees and its
//! orders are validated like a keyboard's, so a goal is never a cheat,
//! and the world, the wire and the replay know nothing of it.
//!
//! The pillbox, by the numbers: 60 health, 8 tiles of reach, 15 every
//! two seconds, at tanks and drones only, never at a squad. The direct
//! gun reaches 10 and does 45. So a live hostile pillbox dies to two
//! direct shots from the ring between 8 and 10 tiles, taking none back;
//! then the squad stands on it for ten seconds and it is ours. A
//! neutral one does not have to be shot at all: the squad takes it live
//! while the tank stays out of its ring.
//!
//! Auto (step 11c's hybrid): a ladder of rungs, each with a condition,
//! tried top down every tick; the first whose condition holds is what
//! the tank does, and the player turns rungs on and off:
//!
//! 1. defend base: a hostile tank within BASE_ALARM of the base, or a
//!    pillbox of the base shot at. The tank fights it under the base's
//!    guns, a few tiles out toward it; the squad mends between shots.
//! 2. fortify: the castle round the base is not yet standing.
//! 3. turrets: the castle stands and nothing has come for QUIET; bring
//!    the side's outlying pillboxes home, one at a time, and set them
//!    down in the bastions at the wall's ends (step 11h).
//! 4. hunt: a pillbox not of my side within EXPAND_R of the base.
//! 5. expand: a depot not of my side, the nearest the base first (its
//!    guard pillboxes shot dead first, then the tank sits on it), else
//!    a pillbox anywhere. A side that only hunted pillboxes held three
//!    depots for fifteen minutes while dead teams held five each.
//! 6. guard: on guard at the base, mending.
//!
//! Low on shells or armour, the tank goes home first whatever the rung.

use crate::fortify::{Plan, Want};
use crate::status::{Act, Why};
use crate::{Brain, RuleBot, PILLBOY_REACH};
use treadcore_sim::*;

/// Where the hunter fires from: outside the pillbox's ring, inside the
/// direct gun's, with a margin each side for a tank still rolling.
pub const HUNT_STANDOFF: f32 = 9.1;
/// The ring the gun is fired from, by the tank's actual distance: the
/// world ends a route at a tile's centre, so the point is snapped to one
/// inside it.
const HUNT_NEAR: f32 = PILLBOX_RANGE + 0.2;
const HUNT_FAR: f32 = DIRECT_RANGE - 0.1;
/// How far round the target the firing point moves when the world says
/// the line is blocked (a wall, forest), in radians: 45 degrees.
const STEP_ROUND: f32 = std::f32::consts::FRAC_PI_4;
/// A defender stays this close to what it defends, and chases nothing
/// further than `DEFEND_LEASH` from it.
pub const DEFEND_HOLD: f32 = 3.0;
pub const DEFEND_LEASH: f32 = 12.0;
/// An escort keeps this far from the tank it follows.
pub const ESCORT_DISTANCE: f32 = 4.0;
/// The wingman (Goal::Escort). Nothing it does takes it further than
/// `WING_LEASH` from its leader; what it wants is within `WING_GREED`
/// of the leader; a threat to the leader is anything hostile within
/// `WING_THREAT` of it; ahead is the leader's heading within 60 degrees
/// out to `WING_AHEAD`. A task is kept `WING_COMMIT` before another
/// below it in the order may take its place.
pub const WING_LEASH: f32 = 20.0;
pub const WING_GREED: f32 = 16.0;
pub const WING_THREAT: f32 = 10.0;
pub const WING_AHEAD: f32 = 16.0;
const WING_COMMIT: Tick = 3 * TICK_HZ as Tick;
/// Ask the leader in no oftener than this.
const ASK_EVERY: Tick = 30 * TICK_HZ as Tick;
/// Hold 100% this long before leaving: a total that flickers is not one.
const TOTAL_HOLD: Tick = 10 * TICK_HZ as Tick;

/// What the wingman is doing, in the order it is preferred.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Wing {
    /// 1. A hostile tank threatening the leader.
    Defend(EntityId),
    /// 2. A hostile tank ahead of the leader.
    Ahead(EntityId),
    /// 3 and 6. A pillbox near the leader: the leader's first.
    Pillbox,
    /// 4. A depot near the leader, not ours.
    Depot(EntityId),
    /// 5. Another ally's fight nearby.
    Help(EntityId),
    /// 7. Back beside the leader.
    Regroup,
}

/// The base (Goal::DefendBase and Auto): what stands within BASE_R of
/// its centre belongs to it; a hostile tank within BASE_ALARM of it is
/// a threat; a pillbox of it shot at raises the alarm for ALARM_HOLD.
pub const BASE_R: f32 = 10.0;
pub const BASE_ALARM: f32 = 16.0;
/// The guard's post moves at most this far toward the front (LR 61):
/// the base's ring stays inside its leash.
pub const FRONT_POST_REACH: f32 = BASE_ALARM - BASE_R;
const ALARM_HOLD: Tick = 6 * TICK_HZ as Tick;
/// Turrets wait for this long without a threat.
pub const QUIET: Tick = 20 * TICK_HZ as Tick;
/// The hunt looks this far from the base first.
pub const EXPAND_R: f32 = 30.0;
/// The tank stops this close to a pillbox it is fetching.
const FETCH_NEAR: f32 = 2.5;
/// Hit with nothing hostile in sight: scoot this far, for this long, and
/// put a recon up over the tank (the RuleBot's shoot-and-scoot, from the
/// other end). Both deaths in the window's first two runs were a tank
/// standing still under shells from beyond its seven tiles of sight.
pub const SCOOT: f32 = 3.5;
const SCOOT_FOR: Tick = 4 * TICK_HZ as Tick;

/// The rungs of Auto's ladder, top first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rung {
    DefendBase,
    Fortify,
    Turrets,
    Hunt,
    Expand,
    Guard,
}

impl Rung {
    pub const ALL: [Rung; 6] = [Rung::DefendBase, Rung::Fortify, Rung::Turrets, Rung::Hunt, Rung::Expand, Rung::Guard];
    /// The rung in words, for the status line and the census.
    pub fn label(self) -> &'static str {
        match self {
            Rung::DefendBase => "defend base",
            Rung::Fortify => "fortify",
            Rung::Turrets => "turrets",
            Rung::Hunt => "hunt",
            Rung::Expand => "expand",
            Rung::Guard => "guard",
        }
    }
    /// Its place in `ALL`, top first: the index of the ladder's flags
    /// and the census's counts.
    pub fn index(self) -> usize {
        Rung::ALL.iter().position(|r| *r == self).unwrap()
    }

    /// The rungs a team posture leans toward (LR 24). Defend keeps the
    /// tank home; Fortify builds and fills the castle; Connect fills the
    /// castle and lines the roads (Turrets) and takes the depots between
    /// (Expand); Expand takes ground; Economy, short of material, takes
    /// depots. DefendBase is never leaned: it is first whenever it holds.
    pub fn leaned(self, p: crate::board::Posture) -> bool {
        use crate::board::Posture as P;
        matches!(
            (p, self),
            (P::Defend, Rung::Guard) | (P::Fortify, Rung::Fortify | Rung::Turrets) | (P::Connect, Rung::Turrets | Rung::Expand) | (P::Expand, Rung::Expand | Rung::Hunt) | (P::Economy, Rung::Expand)
        )
    }

    /// The rung to take: of those switched on that hold, the best by
    /// `(1 - w) order + w lean`, where order runs from 1 for the first
    /// rung down to 0 for the last and lean is 1 for a rung the posture
    /// leans toward. With no posture, or `w` 0, the first that holds, as
    /// the ladder always was; DefendBase, holding, always.
    pub fn pick(on: &[bool; 6], holds: &[bool; 6], posture: Option<crate::board::Posture>, w: f32) -> Rung {
        let first = Rung::ALL.iter().copied().find(|r| on[r.index()] && holds[r.index()]).unwrap_or(Rung::Guard);
        let w = w.clamp(0.0, 1.0);
        let Some(p) = posture.filter(|_| w > 0.0 && first != Rung::DefendBase) else { return first };
        let last = (Rung::ALL.len() - 1) as f32;
        let score = |r: Rung| (1.0 - w) * (1.0 - r.index() as f32 / last) + w * r.leaned(p) as u8 as f32;
        // Ties go to the higher rung, as the order would have it.
        Rung::ALL.iter().copied().filter(|r| on[r.index()] && holds[r.index()]).fold(None, |best: Option<Rung>, r| match best {
            Some(b) if score(b) >= score(r) => Some(b),
            _ => Some(r),
        }).unwrap_or(first)
    }
}

/// Below these the Commander goes home to a depot before anything else.
pub const LOW_SHELLS: u16 = 4;
pub const LOW_ARMOR: f32 = 35.0;
/// Seconds between re-issued moves to the same place: the world routes
/// once, the tank follows; a Move every tick is noise.
const MOVE_EVERY: Tick = TICK_HZ as Tick;

/// What the tank is for, as the player chose it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Goal {
    /// Nothing: the person drives. The Commander says nothing.
    Manual,
    /// Every pillbox not of my side, nearest first: shot dead if it is
    /// live and theirs, then taken by the squad.
    HuntPillboxes,
    /// One pillbox of my side, or the nearest when None.
    DefendPillbox(Option<EntityId>),
    /// One depot of my side, or the nearest when None.
    DefendDepot(Option<EntityId>),
    /// Follow an ally's tank and fight what fights it.
    Escort(PlayerId),
    /// Stay where the goal was given; fight what comes.
    Hold(V2),
    /// Build a castle's gate round a point (step 11b; `fortify.rs`): a
    /// depot of my side by default, the nearest, else where the tank is.
    Fortify(Option<V2>),
    /// The base is under imminent threat: meet what comes for it within
    /// BASE_ALARM, mend and retake its pillboxes, else stand guard.
    DefendBase,
    /// The ladder (`Rung`): the tank picks its own goal, rung by rung.
    Auto,
}

impl Goal {
    /// The goal in words, for the window's goal line.
    pub fn label(&self) -> &'static str {
        match self {
            Goal::Manual => "manual",
            Goal::HuntPillboxes => "hunt pillboxes",
            Goal::DefendPillbox(_) => "defend pillbox",
            Goal::DefendDepot(_) => "defend depot",
            Goal::Escort(_) => "escort",
            Goal::Hold(_) => "hold",
            Goal::Fortify(_) => "fortify",
            Goal::DefendBase => "defend base",
            Goal::Auto => "auto",
        }
    }
}

pub struct Commander {
    name: String,
    pub goal: Goal,
    /// The doctrine for drones and for tanks in sight.
    inner: RuleBot,
    /// The pillbox being hunted, and the bearing (from it) of the
    /// firing point now tried.
    target: Option<EntityId>,
    bearing: Option<f32>,
    /// Pillboxes given up on: the squad could not reach them, or no
    /// firing point round them has a line.
    given_up: crate::memory::Forget<EntityId>,
    /// Where it last told the tank to go, and when.
    last_move: Option<(V2, Tick)>,
    /// Going home to resupply until full again.
    resupplying: bool,
    /// The last direct shot ordered at the target: when, and the shells
    /// then. If none was spent a half second later the world refused it
    /// (a window's client hears of a refusal later than a host does, so
    /// the shell count is the word both read the same).
    fired: Option<(Tick, u16)>,
    /// Firing points tried round this target.
    walked: u32,
    /// The wingman's task now, and the tick it took it: kept for
    /// `WING_COMMIT` unless the leader is threatened.
    task: Option<(Wing, Tick)>,
    /// A bot seat (`Commander::wingman`) runs its own alliances: asks
    /// its leader in, answers proposals, and may leave at 100%.
    pub social: bool,
    /// The last Join asked of the leader, so it does not spam.
    asked: Option<Tick>,
    /// Since when every pillbox and depot has been my side's.
    total_since: Option<Tick>,
    /// The castle being built, under Goal::Fortify.
    pub plan: Option<Plan>,
    /// The hunter has the gun this tick: its target is live and theirs.
    gun_reserved: bool,
    /// What it says it is doing, for the window's status line, and the
    /// same as a code (status.rs), for logic and the census.
    pub doing: String,
    pub act: Act,
    /// Auto's rungs on and off, the one it is on, and each one's reason
    /// on its condition as a code, put into words for the coordination
    /// screen by `why_text`.
    pub ladder: [bool; 6],
    pub rung: Option<Rung>,
    /// The rung of this think, None if this think did not reach the
    /// ladder (a reflex, a fleet slot, a job, another goal): the census
    /// counts it (LR 26), where `rung` is the last one taken.
    rung_now: Option<Rung>,
    pub why: [Why; 6],
    /// The base: set the first time a base goal needs one.
    pub base: Option<V2>,
    /// A fleet's: the base is this player's depot, not my own (fleet.rs).
    pub base_owner: Option<PlayerId>,
    /// The last tick the base was threatened, and the alarm's end.
    threatened: Option<Tick>,
    alarm_until: Tick,
    /// The base's pillbox that raised the alarm: where to go and look.
    alarm_at: Option<V2>,
    /// The outlying pillbox being fetched for a turret.
    fetching: Option<EntityId>,
    /// The plan could not be fed (no forest): fortify steps aside.
    starved: bool,
    /// Expand's depot, and the depots it could not reach.
    depot_target: Option<EntityId>,
    depots_given_up: crate::memory::Forget<EntityId>,
    /// A direct shot refused for no line (a wall of our own, trees): the
    /// gun waits until this tick instead of asking every tick.
    hold_fire_until: Tick,
    /// Scooting from unseen fire: to here, until then; and which side
    /// the last scoot went, so the next goes the other.
    scoot: Option<(V2, Tick)>,
    scoot_left: bool,
    /// Where the last shell at the tank came from.
    gun_seen_at: Option<V2>,
    /// Call out hostile tanks it sees with a ping (fleet.rs sets it).
    pub pings: bool,
    /// Answer a teammate's ping: converge on it and fight what is there.
    pub answers_pings: bool,
    /// The ping being answered: where, and until when; and the bearing
    /// this bot comes in on, fixed when it answered, its flank's turn
    /// added (`flank`).
    rally: Option<(V2, Tick)>,
    rally_from: Option<V2>,
    /// When the rally's target was last in the side's sight.
    rally_seen: Option<Tick>,
    /// The fleet (fleet.rs `fleet_of`): a wingman's place in its
    /// leader's wedge, and a leader's wingmen. A leader waits for them
    /// before it moves on (`GATHER_R`), since when, and not again until
    /// `gather_cool`.
    pub slot: Option<usize>,
    pub members: Vec<PlayerId>,
    gather_since: Option<Tick>,
    gather_cool: Tick,
    /// The last bridge tile handed to a wingman.
    bridge_asked: Tick,
    /// Kiting a rally's target past its reach with no window: which,
    /// since when (`KITE_STALL`); and no rally until `rally_cool`.
    kite_held: Option<(EntityId, Tick)>,
    rally_cool: Tick,
    /// The rally is the fleet leader's order: the fleet attacks together.
    rally_order: bool,
    /// When it last pinged, and the pings it has heard lately, so a team
    /// that all sees one tank pings it once.
    last_ping: Tick,
    heard: Vec<(V2, Tick)>,
    /// Team chat to send, a line a second (the world's limit).
    say: Vec<String>,
    /// A teammate's chain this bot is helping haul: whose, and on what
    /// (`help_chain`). Before anything else but a dodge.
    helping: Option<(PlayerId, EntityId)>,
    last_hitch: Tick,
    /// The side's fleet commander: musters crews and hands them the
    /// castle plan's work (crew.rs).
    pub fleet: crate::crew::FleetCommand,
    /// A crew: takes the fleet commander's jobs.
    pub crew: crate::crew::Crew,
    /// Push point and defense point (rotation.rs): where idle time goes.
    pub rotation: crate::rotation::Rotation,
    said: Tick,
    /// Out of a shell's way (lib.rs `Dodge`), unless `dodges` is off.
    dodge: crate::Dodge,
    pub dodges: bool,
    /// The last focus call in team chat (tactics.rs).
    focus_said: Tick,
    /// Tiles the world refused a drive to as unreachable (over the
    /// water), and when: home skips them for `REFUSED_FOR`.
    refused: Vec<((i32, i32), Tick)>,
    /// Since when the leader has been down.
    leader_down: Option<Tick>,
    /// The ally's wreck this bot's squad is reviving, and the fall it
    /// last called for help for (its own).
    reviving: Option<EntityId>,
    called_down: Tick,
    /// Wrecks the world would not let the squad reach, by tank and fall.
    revive_failed: crate::memory::Forget<EntityId>,
    /// The weights and capabilities (weights.rs, roadmap 12c).
    pub settings: crate::weights::Settings,
    /// The goal of the last Move, smoothed (intent.rs, roadmap 12d).
    desire: crate::intent::Desire,
    /// The islands (facts.rs), for a base site's worth (roadmap 12f2).
    facts: crate::facts::MapFacts,
    /// The team's shared picture (board.rs, roadmap 12e).
    pub board: Option<crate::board::Shared>,
    /// The base cycle after a fill (roadmap 12f1).
    cycle: crate::cycle::Cycle,
    /// On a bridge or about to be, in file (roadmap 12f3).
    on_bridge: bool,
    /// When road posts were last looked for (roadmap 12h).
    posts_at: Tick,
    /// A rung waiting for its confirmation, and since when (roadmap 12d).
    rung_challenger: Option<(Rung, Tick)>,
    /// What a switch turned off put away (`set_settings`).
    cap_held: CapHeld,
    /// The point last ducked to, and when (`duck`).
    duck_to: Option<(V2, Tick)>,
    /// Hostile guns seen lately: whose, where, its reload and when.
    guns_seen: Vec<(PlayerId, V2, u16, Tick)>,
}

/// The role's own settings of the switches `set_settings` turns off,
/// kept to give back when the switch is on again.
#[derive(Clone, Copy, Debug, Default)]
struct CapHeld {
    dodges: bool,
    pings: bool,
    answers_pings: bool,
    rotation_off: bool,
    no_bridges: bool,
}

/// A downed ally this far off is driven to and revived, if the time
/// left allows the drive, the squad's walk and the work.
pub const REVIVE_REACH: f32 = 45.0;
/// The tank stops this near the wreck and sends the squad.
const REVIVE_NEAR: f32 = 2.5;
/// A revive under the square law's odds at the wreck this low is not
/// tried: the reviver would be the next one down.
pub const REVIVE_ODDS: f32 = 0.5;

/// How long a tile refused as unreachable is kept out of `home`.
const REFUSED_FOR: Tick = 60 * TICK_HZ as Tick;

/// A teammate's ping this far off is answered.
pub const RALLY_REACH: f32 = 90.0;
/// How long a ping is answered with nothing found there; seen enemies
/// keep it going.
const RALLY_FOR: Tick = 40 * TICK_HZ as Tick;
/// Kiting, a target out of sight this long is looked for at its last
/// spot again (it may have gone).
const RALLY_LOST: Tick = 6 * TICK_HZ as Tick;
/// The wedge (Paul, 17 October: "i want to see the tanks prioritise
/// grouping and traveling in formation"): slot k is row k / 2 + 1
/// behind the leader, left for even k and right for odd, `WEDGE`
/// tiles back and across per row.
pub const WEDGE: f32 = 4.0;
/// A leader moves on only with half its wingmen within this, waiting
/// at most `GATHER_FOR`, and then not again for `GATHER_COOL`.
pub const GATHER_R: f32 = 10.0;
const GATHER_FOR: Tick = 30 * TICK_HZ as Tick;
const GATHER_COOL: Tick = 30 * TICK_HZ as Tick;
/// A wingman this near its leader lays the leader's bridge with it.
pub const BRIDGE_HELP_R: f32 = 20.0;
/// Kiting with no window this long is a standoff: the rally ends, and
/// no ping is answered for `RALLY_COOL`.
const KITE_STALL: Tick = 15 * TICK_HZ as Tick;
const RALLY_COOL: Tick = 20 * TICK_HZ as Tick;
/// A bot calls an enemy within this, at most once in `PING_EVERY`, and
/// not one a teammate pinged within `PING_HEARD` tiles in the last
/// `PING_EVERY`.
pub const PING_SIGHT: f32 = 25.0;
const PING_EVERY: Tick = 15 * TICK_HZ as Tick;
const PING_HEARD: f32 = 10.0;
/// At most one focus call in team chat in this long.
const FOCUS_EVERY: Tick = 8 * TICK_HZ as Tick;
/// A focus call is for a target within this.
const FOCUS_REACH: f32 = DIRECT_RANGE * 2.0;

impl Commander {
    /// A bot seat that follows `leader` (step 11a after Paul's notes): an
    /// aggressive, greedy wingman that also runs its own alliances.
    pub fn wingman(seed: u64, name: &str, leader: PlayerId) -> Commander {
        let mut c = Commander::new(seed, name);
        c.goal = Goal::Escort(leader);
        c.social = true;
        c
    }

    /// A Commander on Manual, its doctrine a RuleBot of `seed` with
    /// tactics, the default Settings applied.
    pub fn new(seed: u64, name: &str) -> Commander {
        let mut inner = RuleBot::new(seed, name);
        inner.tactics = Some(crate::tactics::Tactician::default());
        let mut c = Commander { name: name.into(), goal: Goal::Manual, inner, target: None, bearing: None, given_up: Default::default(), last_move: None, resupplying: false, fired: None, walked: 0, task: None, social: false, asked: None, total_since: None, plan: None, gun_reserved: false, doing: String::new(), act: Act::Idle, ladder: [true; 6], rung: None, why: Default::default(), base: None, base_owner: None, threatened: None, alarm_until: 0, alarm_at: None, fetching: None, starved: false, depot_target: None, depots_given_up: Default::default(), hold_fire_until: 0, scoot: None, scoot_left: false, gun_seen_at: None, pings: false, answers_pings: false, rally: None, rally_from: None, rally_seen: None, kite_held: None, rally_cool: 0, rally_order: false, slot: None, members: Vec::new(), gather_since: None, gather_cool: 0, bridge_asked: 0, last_ping: 0, heard: Vec::new(), say: Vec::new(), said: 0, helping: None, last_hitch: 0, fleet: Default::default(), crew: Default::default(), dodge: crate::Dodge { metered: true, ..Default::default() }, dodges: true, focus_said: 0, refused: Vec::new(), leader_down: None, reviving: None, called_down: 0, revive_failed: Default::default(), rotation: Default::default(), settings: Default::default(), desire: Default::default(), facts: Default::default(), board: None, cycle: Default::default(), on_bridge: false, posts_at: 0, rung_challenger: None, cap_held: CapHeld::default(), duck_to: None, guns_seen: Vec::new(), rung_now: None };
        // The default Settings applied, not only held: a Commander built
        // here plays as a dealt one does (LR 50: one built without this
        // ran the landfall's weights at 0 while its Settings said 30).
        c.set_settings(crate::weights::Settings::default());
        c
    }

    /// The weights for its fights (tactics.rs); None shoots the nearest
    /// and answers every ping, as before them.
    pub fn set_tactics(&mut self, t: Option<crate::tactics::Tactics>) {
        self.inner.tactics = t.map(crate::tactics::Tactician::with);
    }

    /// The weights and capabilities: the tactics from the weights, each
    /// part of the brain on or off by its bit (weights.rs, roadmap 12c).
    pub fn set_settings(&mut self, s: crate::weights::Settings) {
        use crate::weights::{Caps, W};
        let keep = if s.caps.has(Caps::FORGET) { s.ticks(W::ForgetFor).max(1) } else { crate::memory::NEVER };
        self.given_up.keep = keep;
        self.depots_given_up.keep = keep;
        self.revive_failed.keep = keep;
        self.inner.hop.set_forget(keep);
        self.inner.hop.pricing = s.caps.has(Caps::PRICED_CROSSING).then(|| s.clone());
        self.inner.landfall_shy = s.w[W::LandfallShy];
        self.inner.landfall_wait = s.w[W::LandfallWait];
        self.inner.landfall_memory = s.w[W::LandfallMemory];
        self.dodge.tune = Some([s.w[W::DodgeHorizon], s.w[W::DodgeMargin], s.w[W::DodgeClear], s.w[W::DodgeHold], s.w[W::DodgePlan], s.w[W::DodgeRelease], s.w[W::DodgeEvery], s.w[W::DodgeFire]]);
        // In place: a slider moved mid-match (backlog 15) keeps the
        // target and the team's calls.
        // How well it plays (LR 67): the doctrine's aim and drones, and
        // under half skill no kite and no dodge.
        self.inner.skill = s.w[W::Skill].clamp(0.0, 1.0);
        if let Some(t) = self.inner.tactics.as_mut() {
            t.w = crate::tactics::Tactics::from_weights(&s);
            t.w.kite &= s.w[W::Skill] >= 0.5;
            t.base = self.base;
            t.confirm = s.smooth_on().then(|| (s.w[W::ConfirmTurret].round().clamp(1.0, 255.0) as u8, s.w[W::Margin]));
            t.smooth = s.smooth_on().then(|| (s.w[W::Beta], s.w[W::AdvanceBand]));
        }
        self.inner.own_resupply = !s.smooth_on();
        // A switch turned off puts away what the role gave the bot; on
        // again, it gets it back.
        let was = self.settings.caps;
        let (off, on) = (|c: Caps| was.has(c) && !s.caps.has(c), |c: Caps| !was.has(c) && s.caps.has(c));
        let h = &mut self.cap_held;
        if off(Caps::DODGE) {
            h.dodges = self.dodges;
            self.dodges = false;
        } else if on(Caps::DODGE) {
            self.dodges = h.dodges;
        }
        if off(Caps::PINGS) {
            (h.pings, h.answers_pings) = (self.pings, self.answers_pings);
            (self.pings, self.answers_pings) = (false, false);
        } else if on(Caps::PINGS) {
            (self.pings, self.answers_pings) = (h.pings, h.answers_pings);
        }
        if off(Caps::ROTATE) {
            h.rotation_off = self.rotation.off;
            self.rotation.off = true;
        } else if on(Caps::ROTATE) {
            self.rotation.off = h.rotation_off;
        }
        if off(Caps::BRIDGES) {
            h.no_bridges = self.inner.hop.no_bridges;
            self.inner.hop.no_bridges = true;
        } else if on(Caps::BRIDGES) {
            self.inner.hop.no_bridges = h.no_bridges;
        }
        self.settings = s;
    }

    /// The capabilities in its Settings: which parts are on.
    fn caps(&self) -> crate::weights::Caps {
        self.settings.caps
    }

    /// A new goal: what was half done for the old one is dropped.
    pub fn set_goal(&mut self, goal: Goal) {
        if goal != self.goal {
            self.goal = goal;
            self.target = None;
            self.bearing = None;
            self.last_move = None;
            self.plan = None;
            self.doing.clear();
            self.rung = None;
            self.fetching = None;
            self.starved = false;
        }
    }

    /// Owned by this seat or an ally; an unowned thing is neither.
    fn mine_or_allied(obs: &Observation, owner: Option<PlayerId>) -> bool {
        owner.map(|o| obs.allied(o)).unwrap_or(false)
    }

    /// A Move, unless the tank was already sent there lately.
    fn go(&mut self, obs: &Observation, me: &Tank, to: V2, cmds: &mut Vec<Command>) {
        if me.pos.dist(to) < 0.7 {
            if me.goal.is_some() {
                cmds.push(Command::Stop);
            }
            return;
        }
        // Refused as over the water and no boat under way (the hop gave
        // it up): asking again each second changes nothing; the refusal
        // runs out (`REFUSED_FOR`) and it is asked once more then.
        if self.over_water(to) {
            return;
        }
        // Smoothed (roadmap 12d): a goal that shifts a little each think
        // (a slot ahead of a moving leader, a rally on a moving enemy) is
        // followed by its desire vector, not re-routed at every shift; a
        // jump, a fresh order (no last move) or the way home is taken as
        // it is.
        // By `w_smooth` (LR 46): the share of the new pull taken blends
        // from all of it (0) to `alpha` (1).
        let share = self.settings.smooth();
        let to = if share > 0.0 && self.last_move.is_some() && !self.resupplying {
            use crate::weights::W;
            self.desire.toward(to, self.settings.smooth_alpha(), self.settings.w[W::Jitter])
        } else {
            self.desire.reset();
            to
        };
        let fresh = match self.last_move {
            Some((at, tick)) => at.dist(to) > 1.0 || obs.tick >= tick + MOVE_EVERY * 4 || (me.goal.is_none() && obs.tick >= tick + MOVE_EVERY),
            None => true,
        };
        if fresh {
            cmds.push(Command::Move { to });
            self.last_move = Some((to, obs.tick));
        }
    }

    /// A place the world lately refused a drive to, and no sea trip on
    /// the way there.
    /// The rotation's two ends for the window: the defense point and the
    /// push point, once the base is known.
    pub fn rotation_points(&self, obs: &Observation) -> Option<(V2, V2)> {
        let base = self.base?;
        Some((self.post(base), self.rotation.peek(obs, base)))
    }

    /// Idle: rotate between the defense point (the base's post) and the
    /// push point instead of waiting (rotation.rs). False when rotation
    /// is off and the caller's waiting stands.
    fn rotate(&mut self, obs: &Observation, me: &Tank, base: V2, cmds: &mut Vec<Command>) -> bool {
        if self.rotation.off {
            return false;
        }
        let defense = self.post(base);
        let push = self.rotation.push(obs, base);
        if obs.tick % TICK_HZ as Tick == 0 && self.rotation.front_w > 0.0 {
            crate::meter::lean(false, !self.rotation.front.is_empty());
        }
        let leg = self.rotation.leg(obs, me, defense, push);
        // A point the world will not have us drive to (over the water,
        // no way across): turn back for the other end.
        if self.over_water(leg.to) {
            self.rotation.out = !self.rotation.out;
            self.set_doing(Act::Rotation, "rotation: no way there, turning back".into());
            return true;
        }
        if me.pos.dist(leg.to) > crate::rotation::ARRIVED {
            self.go(obs, me, leg.to, cmds);
        }
        self.set_doing(Act::Rotation, leg.word);
        true
    }

    /// A wingman in formation by a parked leader (`WING_PARKED`):
    /// rotate between its slot (the defense point) and a point toward
    /// the objective, inside the leader's gathering ring (rotation.rs).
    /// The leader moving off ends it (fly_wing). False: stay in the slot.
    fn wing_rotate(&mut self, obs: &Observation, me: &Tank, leader: &Tank, slot: V2, map: Option<&Map>, cmds: &mut Vec<Command>) -> bool {
        use crate::rotation::{ARRIVED, WING_PARKED};
        if self.rotation.off || leader.speed >= 0.3 || leader.boat || me.boat {
            return false;
        }
        let since = *self.rotation.parked.get_or_insert(obs.tick);
        if obs.tick < since + WING_PARKED {
            return false;
        }
        // Ground all the way from the leader: no sea trip on a patrol.
        let lp = leader.pos;
        let ground = |p: V2| {
            let steps = (p.dist(lp) * 2.0).ceil().max(1.0) as i32;
            let land = match map {
                Some(m) => (0..=steps).all(|i| {
                    let q = lp + (p - lp).scale(i as f32 / steps as f32);
                    m.in_bounds(q) && m.at(q).passable()
                }),
                None => true,
            };
            land && !self.over_water(p)
        };
        let push = self.rotation.wing_push(obs, leader.pos, ground);
        if push.dist(slot) <= ARRIVED {
            return false;
        }
        let leg = self.rotation.leg(obs, me, slot, push);
        if self.over_water(leg.to) {
            self.rotation.out = !self.rotation.out;
            self.set_doing(Act::Rotation, "rotation: no way there, turning back".into());
            return true;
        }
        if me.pos.dist(leg.to) > ARRIVED {
            self.go(obs, me, leg.to, cmds);
        }
        self.set_doing(Act::Rotation, leg.word.replace("the defense point", "the slot"));
        true
    }

    /// The world lately refused a drive to this tile as over the water,
    /// and no island hop is under way to cross it.
    fn over_water(&self, to: V2) -> bool {
        !self.inner.hop.active() && self.refused.iter().any(|(t, _)| *t == to.tile())
    }

    /// The nearest depot of my side, for resupply: of those the world
    /// has not lately refused as over the water, if any; else the
    /// nearest, and the drive's refusal sends the tank by sea.
    fn home(&self, obs: &Observation, me: &Tank) -> Option<V2> {
        let ours = || obs.depots.iter().filter(|d| Self::mine_or_allied(obs, d.owner)).map(|d| d.pos);
        let near = |a: &V2, b: &V2| a.dist(me.pos).total_cmp(&b.dist(me.pos));
        ours().filter(|p| !self.refused.iter().any(|(t, _)| *t == p.tile())).min_by(near).or_else(|| ours().min_by(near))
    }

    /// Low on what the goal needs: go home, and stay until full.
    /// Say what it is doing: the code and the words (status.rs).
    fn set_doing(&mut self, act: Act, text: String) {
        self.act = act;
        self.doing = text;
    }

    /// A rung's reason in words, for the coordination screen: the code
    /// of `why` put into the sentence it has always been.
    pub fn why_text(&self, rung: usize) -> String {
        match self.why[rung] {
            Why::Unknown => String::new(),
            Why::ThreatTank(d) => format!("THREAT: a tank {:.0} tiles from the base", d),
            Why::ThreatPillbox => "THREAT: a pillbox of the base is fighting".into(),
            Why::Quiet { secs, within } => format!("quiet {:.0} s; nothing within {:.0} tiles", secs, within),
            Why::Building { starved } => match &self.plan {
                Some(p) if starved => format!("stopped, no forest: {}", p.progress()),
                Some(p) => p.progress(),
                None => "nothing built yet".into(),
            },
            Why::NothingBuilt => "nothing built yet".into(),
            Why::CastleFirst => "after the castle stands".into(),
            Why::TurretsStand => "both turrets stand".into(),
            Why::Walling => "walling the yard in".into(),
            Why::WaitingQuiet { need, now } => format!("waiting for {:.0} s of quiet ({:.0} now)", need, now),
            Why::Carrying => "carrying one home".into(),
            Why::Outlying(d) => format!("a pillbox of ours out at {:.0} tiles", d),
            Why::NoOutlying => "no pillbox of ours out there to bring in".into(),
            Why::PillboxNear(d) => format!("a pillbox not ours {:.0} tiles from the base", d),
            Why::PillboxFar { within, nearest } => format!("none within {:.0}; the nearest is {:.0} out", within, nearest),
            Why::AllPillboxesOurs => "every pillbox is ours or an ally's".into(),
            Why::DepotNear(d) => format!("a depot not ours {:.0} tiles from the base", d),
            Why::DepotsOursPillboxesFar => "every depot ours; pillboxes left far out".into(),
            Why::MapOurs => "the map is ours".into(),
            Why::Always => "always".into(),
        }
    }

    /// Whether to go home to resupply: once low on shells or armour
    /// (the constants blended to the weights by `w_smooth`), until full
    /// again; held between thinks in `resupplying`.
    fn needs_home(&mut self, me: &Tank) -> bool {
        // The retreat thresholds blend from the constants (0) to the
        // weights (1) by `w_smooth` (LR 46).
        let share = self.settings.smooth();
        let (low_shells, low_armor) = if share > 0.0 {
            use crate::weights::W;
            let shells = LOW_SHELLS as f32 + share * (self.settings.w[W::RetreatShells].max(0.0) - LOW_SHELLS as f32);
            (shells.round() as u16, LOW_ARMOR + share * (self.settings.w[W::RetreatArmor] - LOW_ARMOR))
        } else {
            (LOW_SHELLS, LOW_ARMOR)
        };
        if me.shells <= low_shells || me.armor <= low_armor {
            self.resupplying = true;
        }
        if self.resupplying && me.shells >= TANK_SHELLS && me.armor >= TANK_ARMOR * 0.95 {
            self.resupplying = false;
        }
        self.resupplying
    }

    /// What the inner doctrine may still do under a goal: drones always;
    /// shots at tanks unless the goal wants the gun for itself.
    fn keep_inner(cmd: &Command, gun_free: bool) -> bool {
        match cmd {
            Command::Launch { .. } | Command::Recall | Command::Retask { .. } | Command::SetBubble { .. } => true,
            Command::FireDirect { .. } | Command::FireIndirect { .. } => gun_free,
            _ => false,
        }
    }

    /// `area`: only pillboxes within this distance of this point (a
    /// wingman's, by its leader); None for the whole map.
    fn hunt(&mut self, obs: &Observation, me: &Tank, area: Option<(V2, f32)>, cmds: &mut Vec<Command>) {
        // What the world refused tells the hunter what to change.
        for e in &obs.events {
            if let Event::Rejected { player, cmd, why } = e {
                if *player != obs.player {
                    continue;
                }
                match cmd {
                    Command::FireDirect { .. } if why.contains("line of sight") => {
                        self.fired = None;
                        self.blocked(obs.tick);
                    }
                    // The firing point is water or cannot be reached:
                    // the next one round.
                    Command::Move { .. } if self.target.is_some() && (why.contains("water") || why.contains("unreachable")) => {
                        self.last_move = None;
                        self.blocked(obs.tick);
                    }
                    Command::Deploy { task: PillboyTask::Capture { pillbox } } if why.contains("unreachable") => {
                        self.given_up.put(*pillbox, obs.tick);
                        self.target = None;
                    }
                    _ => {}
                }
            }
        }
        if let Some((t, shells)) = self.fired {
            if obs.tick >= t + TICK_HZ as Tick / 2 {
                self.fired = None;
                if me.shells >= shells && me.reload == 0 {
                    self.blocked(obs.tick);
                }
            }
        }
        // The target: kept while it is still to be had, else the nearest.
        let wanted = |p: &Pillbox| !Self::mine_or_allied(obs, p.owner) && !self.given_up.has(&p.id, obs.tick) && area.map(|(at, r)| p.pos.dist(at) <= r).unwrap_or(true);
        let target = self
            .target
            .and_then(|id| obs.pillboxes.iter().find(|p| p.id == id))
            .filter(|p| wanted(p))
            .or_else(|| obs.pillboxes.iter().filter(|p| wanted(p)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)).then(a.id.cmp(&b.id))))
            .cloned();
        let Some(p) = target else {
            self.target = None;
            self.set_doing(Act::NoTarget, if self.given_up.is_empty() { "every pillbox is ours or an ally's".into() } else { format!("none left to take ({} given up)", self.given_up.len()) });
            return;
        };
        if self.target != Some(p.id) {
            self.target = Some(p.id);
            self.bearing = None;
            self.walked = 0;
        }
        // The other live hostile pillboxes: the firing point must be
        // outside their rings too.
        let others: Vec<V2> = obs.pillboxes.iter().filter(|o| o.id != p.id && o.alive() && !Self::mine_or_allied(obs, o.owner)).map(|o| o.pos).collect();
        let base = self.bearing.unwrap_or_else(|| (me.pos - p.pos).heading());
        // The tile centre nearest the ideal point whose own distance is
        // inside the ring.
        let snap = |raw: V2| -> V2 {
            let mut best = Map::tile_centre(raw);
            let mut err = f32::MAX;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let c = Map::tile_centre(raw + V2::new(dx as f32, dy as f32));
                    let d = c.dist(p.pos);
                    if (HUNT_NEAR..=HUNT_FAR).contains(&d) && (d - HUNT_STANDOFF).abs() + c.dist(raw) * 0.1 < err {
                        err = (d - HUNT_STANDOFF).abs() + c.dist(raw) * 0.1;
                        best = c;
                    }
                }
            }
            best
        };
        let stand = (0..8)
            .map(|k| base + STEP_ROUND * k as f32)
            .map(|b| snap(p.pos + V2::from_heading(b).scale(HUNT_STANDOFF)))
            .find(|s| others.iter().all(|o| o.dist(*s) > PILLBOX_RANGE + 0.8))
            .unwrap_or_else(|| snap(p.pos + V2::from_heading(base).scale(HUNT_STANDOFF)));
        let live_theirs = p.alive() && p.owner.is_some();
        self.gun_reserved = live_theirs;
        if live_theirs {
            let d = me.pos.dist(p.pos);
            // From the firing point only: after a refusal the point moves
            // round, and the tank must go there before it tries again.
            if (HUNT_NEAR..=HUNT_FAR).contains(&d) && me.pos.dist(stand) < 1.5 {
                if me.reload == 0 && me.shells > 0 && self.fired.is_none() {
                    cmds.push(Command::FireDirect { at: p.pos });
                    self.fired = Some((obs.tick, me.shells));
                }
                self.set_doing(Act::TakePillbox, format!("shelling the pillbox, {:.0} health left", p.health));
                // Stand still while firing: a rolling tank leaves the ring.
                if me.goal.is_some() && me.pos.dist(stand) < 1.5 {
                    cmds.push(Command::Stop);
                }
            } else {
                self.set_doing(Act::TakePillbox, format!("to the firing point, {:.0} tiles off", me.pos.dist(stand)));
                self.go(obs, me, stand, cmds);
            }
            return;
        }
        // Dead, or neutral: the squad takes it. The tank waits outside the
        // ring of a live one, inside the squad's reach.
        if p.alive() && me.pos.dist(p.pos) <= PILLBOX_RANGE + 0.5 {
            self.go(obs, me, stand, cmds);
        } else if me.pos.dist(p.pos) > PILLBOY_REACH * 0.8 {
            self.set_doing(Act::TakePillbox, "closing in for the squad".into());
            self.go(obs, me, stand, cmds);
            return;
        }
        let squad = &me.pillboys;
        match &squad.task {
            Some(PillboyTask::Capture { pillbox }) if *pillbox == p.id => {
                self.set_doing(Act::TakePillbox, format!("the squad is taking it, {:.0} s", (PILLBOX_CAPTURE_TICKS.saturating_sub(squad.progress)) as f32 * DT));
            }
            Some(_) => {
                // Out on something else: bring them home first.
                cmds.push(Command::Deploy { task: PillboyTask::Return });
                self.set_doing(Act::TakePillbox, "calling the squad back".into());
            }
            None if squad.aboard && squad.strength > 0 => {
                cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: p.id } });
                self.set_doing(Act::TakePillbox, "sending the squad".into());
            }
            None => {
                self.set_doing(Act::NoSquad, "no squad: to a depot for one".into());
                self.resupplying = true;
            }
        }
    }

    /// The wingman's order of things, from Paul's notes: protect the
    /// leader; hit what is ahead of it; take the pillbox it is taking, or
    /// another near it; take a depot near it; help another ally's fight;
    /// else close up behind it. Re-read every tick, a task kept for
    /// `WING_COMMIT` unless one ranked above it appears, and nothing
    /// further than the leash from the leader. Returns whether the
    /// inner doctrine may use the gun.
    fn fly_wing(&mut self, obs: &Observation, me: &Tank, leader: &Tank, map: Option<&Map>, cmds: &mut Vec<Command>) -> bool {
        let lp = leader.pos;
        let hostile = |t: &&Tank| t.alive && !obs.allied(t.owner);
        let near = |a: V2, b: V2| a.dist(lp).total_cmp(&b.dist(lp));
        let threat = obs.tanks.iter().filter(hostile).filter(|t| t.pos.dist(lp) <= WING_THREAT).min_by(|a, b| near(a.pos, b.pos).then(a.id.cmp(&b.id)));
        let facing = V2::from_heading(leader.heading);
        let ahead = obs
            .tanks
            .iter()
            .filter(hostile)
            .filter(|t| t.pos.dist(lp) <= WING_AHEAD && { let d = (t.pos - lp).norm(); d.x * facing.x + d.y * facing.y >= 0.5 })
            .min_by(|a, b| near(a.pos, b.pos).then(a.id.cmp(&b.id)));
        let pillbox = obs.pillboxes.iter().any(|p| !Self::mine_or_allied(obs, p.owner) && !self.given_up.has(&p.id, obs.tick) && p.pos.dist(lp) <= WING_GREED);
        let depot = obs.depots.iter().filter(|d| !Self::mine_or_allied(obs, d.owner) && d.pos.dist(lp) <= WING_GREED).min_by(|a, b| near(a.pos, b.pos)).map(|d| d.id);
        let help = obs
            .tanks
            .iter()
            .filter(|t| t.alive && t.owner != obs.player && t.owner != leader.owner && obs.allied(t.owner) && t.pos.dist(lp) <= WING_LEASH)
            .find_map(|a| obs.tanks.iter().filter(hostile).find(|h| h.pos.dist(a.pos) <= WING_THREAT * 0.8).map(|h| h.id));
        let best = threat
            .map(|t| Wing::Defend(t.id))
            .or(ahead.map(|t| Wing::Ahead(t.id)))
            .or(pillbox.then_some(Wing::Pillbox))
            .or(depot.map(Wing::Depot))
            .or(help.map(Wing::Help))
            .unwrap_or(Wing::Regroup);
        // Keep the task for its commitment unless a better one outranks
        // it by a defence; a task that no longer exists ends at once.
        let still = |w: Wing| match w {
            Wing::Defend(id) | Wing::Ahead(id) | Wing::Help(id) => obs.tanks.iter().any(|t| t.id == id && t.alive && t.pos.dist(lp) <= WING_LEASH),
            Wing::Pillbox => pillbox,
            Wing::Depot(id) => depot == Some(id),
            Wing::Regroup => true,
        };
        let task = match self.task {
            Some((w, since)) if w != best && still(w) && obs.tick < since + WING_COMMIT && !matches!(best, Wing::Defend(_)) => w,
            Some((w, _)) if w == best => w,
            _ => {
                self.task = Some((best, obs.tick));
                best
            }
        };
        let behind = lp - facing.scale(ESCORT_DISTANCE);
        match task {
            Wing::Defend(id) | Wing::Ahead(id) | Wing::Help(id) => {
                let t = obs.tanks.iter().find(|t| t.id == id).unwrap().clone();
                let d = me.pos.dist(t.pos);
                if d <= DIRECT_RANGE - 0.2 && me.reload == 0 && me.shells > 0 {
                    cmds.push(Command::FireDirect { at: crate::lead(me, &t) });
                }
                // Kiting it (tactics.rs) on the leash; without the
                // weights, close to direct range as before.
                let kite = match self.inner.tactics.as_mut() {
                    Some(tc) if tc.w.kite => {
                        let odds = tc.odds(obs, me, t.pos);
                        let st = tc.stance(obs.tick, &odds);
                        Some(tc.kite(obs, me, &t, (me.pos - t.pos).norm(), st, DIRECT_RANGE * 1.8))
                    }
                    _ => None,
                };
                let to = match kite {
                    Some(k) => Some(k.to),
                    None if d > DIRECT_RANGE - 1.0 => Some(t.pos + (me.pos - t.pos).norm().scale(DIRECT_RANGE - 2.0)),
                    None => None,
                };
                if let Some(to) = to {
                    let to = if to.dist(lp) > WING_LEASH { lp + (to - lp).norm().scale(WING_LEASH) } else { to };
                    if kite.map(|k| k.stepping_in).unwrap_or(false) {
                        self.last_move = None;
                    }
                    self.go(obs, me, to, cmds);
                }
                let what = match task {
                    Wing::Defend(_) => "covering the leader",
                    Wing::Ahead(_) => "hitting what is ahead",
                    _ => "helping an ally",
                };
                self.set_doing(kite.as_ref().map(|k| Act::WingKite(k.step)).unwrap_or(Act::Wing), match kite {
                    Some(k) => format!("{}: {}", what, k.step.word()),
                    None => what.into(),
                });
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Wing::Pillbox => {
                self.hunt(obs, me, Some((lp, WING_GREED)), cmds);
                self.set_doing(Act::TakePillbox, format!("pillbox by the leader: {}", self.doing));
                !self.gun_reserved
            }
            Wing::Depot(id) => {
                let at = obs.depots.iter().find(|d| d.id == id).unwrap().pos;
                self.go(obs, me, at, cmds);
                self.set_doing(Act::TakeDepot, "taking the depot".into());
                true
            }
            Wing::Regroup if leader.boat && !me.boat && self.slot.is_some() && me.pos.dist(lp) <= WING_LEASH && !self.inner.hop.active() && self.inner.hop.send(lp) => {
                // The leader is launching: launch with it (the hop finds
                // the bank), and sail in the wedge (think's afloat fleet).
                self.set_doing(Act::WingSea, "the leader is launching: launching with it".into());
                true
            }
            Wing::Regroup if leader.boat && !me.boat && self.slot.is_some() => {
                // The leader crossed without us: wait at the shore for
                // where it lands, then follow it over (below).
                if me.goal.is_some() && me.pos.dist(lp) <= WING_LEASH {
                    cmds.push(Command::Stop);
                }
                self.set_doing(Act::WingSea, "the leader is afloat: waiting for its landing".into());
                true
            }
            Wing::Regroup if self.slot.is_some() && !self.inner.hop.active() && self.refused.iter().any(|(t, _)| V2::new(t.0 as f32 + 0.5, t.1 as f32 + 0.5).dist(lp) <= WING_LEASH) && self.inner.hop.send(lp) => {
                // The leader is over the water: after it by sea.
                self.set_doing(Act::WingSea, format!("following the leader by sea, to ({:.0},{:.0})", lp.x, lp.y));
                true
            }
            Wing::Regroup => {
                if leader.speed >= 0.3 || leader.boat {
                    self.rotation.parked = None;
                    self.rotation.out = false;
                }
                // The wedge: my slot, where it will be in a second at the
                // leader's speed; a slot over the water, the plain place
                // behind it.
                let slot = self.slot_point(leader, map);
                let to = match slot.map(|p| obs.clamp(p)) {
                    Some(p) if !self.over_water(p) => p,
                    _ => behind,
                };
                // A slot on the water (the leader at a bridge's end, on a
                // shore): the nearest ground to it, nearer the leader
                // first. The quad's wingmen sat "closing up" to a slot
                // the world refused, for minutes.
                let to = match map {
                    Some(m) if !m.at(to).passable() => {
                        let mut best: Option<(V2, f32)> = None;
                        for dy in -4i32..=4 {
                            for dx in -4i32..=4 {
                                let p = Map::tile_centre(to + V2::new(dx as f32, dy as f32));
                                if m.in_bounds(p) && m.at(p).passable() && p.tile() != lp.tile() {
                                    let score = p.dist(to) + 0.5 * p.dist(lp);
                                    if best.map(|(_, b)| score < b).unwrap_or(true) {
                                        best = Some((p, score));
                                    }
                                }
                            }
                        }
                        best.map(|(p, _)| p).unwrap_or(me.pos)
                    }
                    _ => to,
                };
                let word = self.slot.map(|k| format!("in formation, slot {}", k + 1)).unwrap_or_else(|| "on the leader's wing".into());
                // Beside the leader already, near the slot: in formation.
                let close = me.pos.dist(lp) <= WEDGE * 2.0 && me.pos.dist(to) <= WEDGE;
                // Rotating (the leader parked): the rotation drives to
                // the slot and away from it; closing up would undo it.
                if self.wing_rotate(obs, me, leader, to, map, cmds) {
                } else if me.pos.dist(to) > 1.5 && !close {
                    self.go(obs, me, to, cmds);
                    self.set_doing(Act::Wing, format!("{}: closing up, {:.0} tiles", word, me.pos.dist(to)));
                } else {
                    if me.goal.is_some() && leader.speed < 0.3 {
                        cmds.push(Command::Stop);
                    }
                    self.set_doing(Act::Wing, word);
                }
                true
            }
        }
    }

    /// A bot seat's sides: answer what is put to it (yes to the leader and
    /// its side, yes to anyone who has not shot at it); ask to join the
    /// leader's side while not on it; and leave the side, keeping what
    /// it took, only once every pillbox and every depot on the map has
    /// been the side's for `TOTAL_HOLD`. The 100% is an unlock, read from
    /// the observation's full lists (territory is public), rechecked
    /// every tick; the world's three-way rule still has the last word.
    fn alliances(&mut self, obs: &Observation, cmds: &mut Vec<Command>) {
        let leader = match self.goal {
            Goal::Escort(p) => Some(p),
            _ => None,
        };
        for p in obs.proposals.iter().filter(|p| p.to == obs.player || obs.allied(p.to)) {
            if p.from == obs.player {
                continue;
            }
            let accept = Some(p.from) == leader || obs.allied(p.from) || !self.inner.hurt_by_player(p.from);
            cmds.push(Command::Answer { proposal: p.id, accept });
        }
        if let Some(l) = leader {
            let pending = obs.proposals.iter().any(|p| p.from == obs.player);
            let due = self.asked.map(|t| obs.tick >= t + ASK_EVERY).unwrap_or(true);
            if !obs.allied(l) && !pending && due && obs.would_refuse(l, Terms::Join).is_none() {
                cmds.push(Command::Propose { to: l, terms: Terms::Join });
                self.asked = Some(obs.tick);
            }
        }
        let total = !obs.pillboxes.is_empty() && obs.pillboxes.iter().all(|p| Self::mine_or_allied(obs, p.owner)) && obs.depots.iter().all(|d| Self::mine_or_allied(obs, d.owner));
        let sided = obs.players.iter().any(|p| p.id != obs.player && obs.allied(p.id));
        if total && sided {
            let since = *self.total_since.get_or_insert(obs.tick);
            if obs.tick >= since + TOTAL_HOLD {
                cmds.push(Command::Alliance(AllianceCmd::Leave));
                self.total_since = None;
                self.set_doing(Act::Leave, "100% held: leaving the side with what it took".into());
            }
        } else {
            self.total_since = None;
        }
    }

    /// Build the castle: guard the point as `defend` does, and keep the
    /// squad on the plan whenever it is aboard and nothing threatens.
    fn fortify(&mut self, obs: &Observation, me: &Tank, at: Option<V2>, cmds: &mut Vec<Command>) {
        if self.plan.is_none() {
            let centre = at
                .or_else(|| obs.depots.iter().filter(|d| Self::mine_or_allied(obs, d.owner)).map(|d| d.pos).min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos))))
                .unwrap_or(me.pos);
            // The threat: the nearest depot not of my side, else the
            // map's middle, which is where everyone else comes from.
            let middle = V2::new(obs.map_w as f32 / 2.0, obs.map_h as f32 / 2.0);
            let threat = obs.depots.iter().filter(|d| !Self::mine_or_allied(obs, d.owner) && d.pos.dist(centre) > 4.0).map(|d| d.pos).min_by(|a, b| a.dist(centre).total_cmp(&b.dist(centre))).unwrap_or(middle);
            self.plan = Some(if self.caps().has(crate::weights::Caps::CASTLE_CAP) {
                use crate::weights::W;
                let w = &self.settings.w;
                Plan::castle_capped(centre, threat, w[W::CastleInside].round().max(0.0) as usize, w[W::CastleRoad].round().max(0.0) as usize)
            } else {
                Plan::castle(centre, threat)
            });
            if matches!(self.goal, Goal::Fortify(_)) {
                self.goal = Goal::Fortify(Some(centre));
            }
        }
        let plan = self.plan.as_mut().unwrap();
        plan.hear(obs);
        // The tank sits behind the work, beside the point: off the gate,
        // off the road, inside the squad's reach of every tile.
        let post = plan.post;
        self.defend(obs, me, post, None, cmds);
        let busy = !matches!(self.act, Act::OnGuard | Act::ToPost);
        let squad = &me.pillboys;
        if !squad.aboard || squad.task.is_some() || squad.strength == 0 {
            if squad.strength == 0 {
                self.set_doing(Act::NoSquad, "no squad: waiting on the depot for one".into());
            } else if !busy {
                self.set_doing(Act::Fortify, format!("the squad is building: {}", self.plan.as_ref().unwrap().progress()));
            }
            return;
        }
        if busy {
            return;
        }
        let plan = self.plan.as_mut().unwrap();
        match plan.next(obs, squad.material) {
            Want::Order(task) => {
                self.starved = false;
                self.act = Act::Fortify;
                self.doing = match &task {
                    PillboyTask::Harvest { .. } => "felling for material".into(),
                    _ => format!("building: {}", plan.progress()),
                };
                cmds.push(Command::Deploy { task });
            }
            Want::Starved => {
                self.starved = true;
                self.act = Act::Fortify;
                self.doing = format!("no forest in reach; {}", plan.progress());
            }
            Want::Wait => { self.act = Act::Fortify; self.doing = format!("waiting on the site: {}", plan.progress()) },
            Want::Finished => { self.act = Act::Fortify; self.doing = format!("the castle stands: {}", plan.progress()) },
        }
    }

    /// The base's centre: the castle's, else the depot of my side nearest
    /// the tank when first asked, else where the tank is.
    fn base_of(&mut self, obs: &Observation, me: &Tank) -> V2 {
        if let Some(p) = &self.plan {
            self.base = Some(p.centre);
        }
        if self.base.is_none() {
            if let Some(owner) = self.base_owner {
                // The leader's depot: the one of theirs nearest its tank
                // when it is in sight, else nearest mine.
                let near = obs.tanks.iter().find(|t| t.owner == owner).map(|t| t.pos).unwrap_or(me.pos);
                self.base = obs.depots.iter().filter(|d| d.owner == Some(owner)).min_by(|a, b| a.pos.dist(near).total_cmp(&b.pos.dist(near))).map(|d| Map::tile_centre(d.pos));
            }
        }
        // No depot of the side yet (a naval start: everyone at sea, every
        // depot neutral): the base is where the tank is for now, and is
        // not kept, so the first depot taken becomes it. Kept, it was a
        // patch of open water for the rest of the match.
        // A small island with few trees is a poor base (roadmap 12f2): of
        // the side's depots, the best site, a tile of distance costing a
        // little. Only when the nearest depot's own site is poor
        // (`site_poor`, LR 73), and only a site near it (`site_reach`):
        // a tile is worth a hundredth of a point, so on atlantic and
        // europe the bases went 70 to 275 tiles off, across the sea.
        if self.base.is_none() && self.caps().has(crate::weights::Caps::ISLAND_SITE) && !self.facts.islands().is_empty() {
            let s = &self.settings;
            let home = self.home(obs, me);
            if home.map(|h| self.facts.site_score(s, h) < s.w[crate::weights::W::SitePoor]).unwrap_or(true) {
                self.base = obs
                    .depots
                    .iter()
                    .filter(|d| Self::mine_or_allied(obs, d.owner))
                    .filter(|d| home.map(|h| d.pos.dist(h) <= s.w[crate::weights::W::SiteReach]).unwrap_or(true))
                    .map(|d| (d.pos, self.facts.site_score(s, d.pos) - 0.01 * d.pos.dist(me.pos)))
                    .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.x.total_cmp(&a.0.x)))
                    .map(|(p, _)| Map::tile_centre(p));
            }
            if let Some(b) = self.base {
                crate::meter::base_site(home.map(|h| Map::tile_centre(h).dist(b)).unwrap_or(0.0));
            }
        }
        if self.base.is_none() {
            self.base = self.home(obs, me).map(Map::tile_centre);
        }
        self.base.unwrap_or(me.pos)
    }

    /// Where the tank waits at the base: behind the gate when there is a
    /// castle, else beside the centre.
    fn post(&self, base: V2) -> V2 {
        match &self.plan {
            Some(p) => p.post,
            None => base,
        }
    }

    /// Each beach in sight with what its island is worth as a base, in
    /// tiles of the run (`w_site_beach` times `site_score`), while afloat
    /// and the side holds no depot; none otherwise.
    fn beach_worth(&self, obs: &Observation) -> Vec<(V2, f32)> {
        let w = self.settings.w[crate::weights::W::SiteBeach];
        let afloat = obs.me.as_ref().map(|t| t.alive && t.boat).unwrap_or(false);
        if w == 0.0 || !afloat || !self.caps().has(crate::weights::Caps::ISLAND_SITE) || self.facts.islands().is_empty() || obs.depots.iter().any(|d| Self::mine_or_allied(obs, d.owner)) {
            return Vec::new();
        }
        let worth = obs.shore.iter().map(|b| (*b, self.facts.site_score(&self.settings, *b))).filter(|(_, s)| *s > f32::MIN).map(|(b, s)| (b, w * s)).collect();
        if obs.tick % TICK_HZ as Tick == 0 {
            crate::meter::beach_site();
        }
        worth
    }

    /// The centres of the team board's front sectors (LR 58), with the
    /// board on; none without a board or with no hostile known.
    fn front_points(&self) -> Vec<V2> {
        if !self.caps().has(crate::weights::Caps::BOARD) {
            return Vec::new();
        }
        self.board.as_ref().and_then(|b| b.lock().ok().map(|b| b.front().into_iter().map(|k| b.sector_centre(k)).collect())).unwrap_or_default()
    }

    /// The guard's post (backlog 51, LR 61): the post moved `w_front_post`
    /// of the way toward the nearest front sector, `FRONT_POST_REACH` at
    /// most, so an attack is met before it reaches the base and the base
    /// stays inside the guard's leash.
    fn guard_post(&self, base: V2) -> V2 {
        let post = self.post(base);
        let w = self.settings.w[crate::weights::W::FrontPost].clamp(0.0, 1.0);
        if w <= 0.0 {
            return post;
        }
        let Some(f) = self.front_points().into_iter().min_by(|a, b| a.dist(post).total_cmp(&b.dist(post))) else {
            return post;
        };
        let d = f.dist(post);
        if d < 0.5 {
            return post;
        }
        let to = post + (f - post).norm().scale((w * d).min(FRONT_POST_REACH));
        if self.over_water(to) {
            post
        } else {
            to
        }
    }

    /// The threat to the base, if any: the hostile tank nearest it within
    /// BASE_ALARM, and the alarm a pillbox of the base shot at raises
    /// (by a gun it may not see: indirect fire, a longshot).
    fn base_threat(&mut self, obs: &Observation, base: V2) -> (Option<Tank>, bool) {
        // The tank sees TANK_VISION and a pillbox sees nothing, so the
        // base's guns are its ears: one of them hit, or firing, is a
        // threat there even when nobody of ours can see it.
        let ours: Vec<(EntityId, V2)> = obs.pillboxes.iter().filter(|p| p.pos.dist(base) <= BASE_R && Self::mine_or_allied(obs, p.owner)).map(|p| (p.id, p.pos)).collect();
        for e in &obs.events {
            let heard = match e {
                Event::PillboxDamaged { pillbox, by: Some(b), .. } if !obs.allied(*b) => Some(*pillbox),
                Event::PillboxFired { pillbox, .. } => Some(*pillbox),
                _ => None,
            };
            if let Some(at) = heard.and_then(|id| ours.iter().find(|(o, _)| *o == id)).map(|(_, at)| *at) {
                self.alarm_until = obs.tick + ALARM_HOLD;
                self.alarm_at = Some(at);
            }
        }
        let tank = obs.tanks.iter().filter(|t| t.alive && !obs.allied(t.owner) && t.pos.dist(base) <= BASE_ALARM).min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base)).then(a.id.cmp(&b.id))).cloned();
        let alarm = obs.tick < self.alarm_until;
        if tank.is_some() || alarm {
            self.threatened = Some(obs.tick);
        }
        (tank, alarm)
    }

    /// The base's pillbox that most wants the squad: one dead first (ours
    /// to mend, theirs to take back), then the most hurt of ours.
    fn neediest(obs: &Observation, base: V2) -> Option<EntityId> {
        obs.pillboxes
            .iter()
            .filter(|p| p.pos.dist(base) <= BASE_R && (p.owner == Some(obs.player) || !p.alive()))
            .filter(|p| !p.alive() || p.health < PILLBOX_HEALTH * 0.7)
            .min_by(|a, b| a.health.total_cmp(&b.health).then(a.id.cmp(&b.id)))
            .map(|p| p.id)
    }

    /// Defend the base: out to meet the threat as far as BASE_ALARM, else
    /// on guard at the post; the squad mends or takes back the neediest
    /// pillbox of the base whenever no hostile tank is in the leash.
    fn defend_base(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        let base = self.base_of(obs, me);
        let (tank, alarm) = self.base_threat(obs, base);
        // A raid on the held ground (LR 68): the side's work nearest it.
        let raid = if tank.is_none() && !alarm { self.raided(obs, base) } else { None };
        // Nothing in sight but a gun of the base at work: go and look.
        let post = match (&tank, alarm, self.alarm_at, raid) {
            (None, true, Some(at), _) => at,
            (_, _, _, Some(w)) => w,
            _ => self.guard_post(base),
        };
        if obs.tick % TICK_HZ as Tick == 0 && self.settings.w[crate::weights::W::FrontPost] > 0.0 && tank.is_some() {
            crate::meter::lean(true, post != self.post(base));
        }
        let mend = Self::neediest(obs, base);
        // Nothing at the base: out to the push point and back, not on
        // guard (rotation.rs). The guard answers what it meets where it
        // is, and the squad still mends from wherever the tank is.
        let rotating = tank.is_none() && !alarm && raid.is_none() && !self.rotation.off && me.pillboys.aboard;
        self.defend_within(obs, me, if rotating { me.pos } else { post }, BASE_ALARM, mend, cmds);
        if rotating && self.act == Act::OnGuard {
            self.rotate(obs, me, base, cmds);
        }
        if let Some(t) = &tank {
            let who = obs.players.iter().find(|p| p.id == t.owner).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", t.owner));
            self.set_doing(Act::BaseThreat, format!("BASE UNDER THREAT: {} {:.0} tiles out; {}", who, t.pos.dist(base), self.doing));
        } else if alarm {
            self.set_doing(Act::BaseThreat, format!("BASE UNDER THREAT out of sight: to the pillbox that heard it; {}", self.doing));
        } else if let Some(w) = raid {
            self.set_doing(Act::BaseThreat, format!("THE HELD GROUND RAIDED: to the work at ({:.0},{:.0}); {}", w.x, w.y, self.doing));
        }
        !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
    }

    /// A raid on the held ground (LR 68, `w_defend_ground`): the board's
    /// incursion, a hostile on the side's works' ground beyond the
    /// base's alarm; the side's pillbox or depot nearest it, where the
    /// guard goes, if that is an outlying work (past the base's ring:
    /// the base's own are the alarm's). None at 0, or without the board.
    fn raided(&self, obs: &Observation, base: V2) -> Option<V2> {
        if self.settings.w[crate::weights::W::DefendGround] <= 0.0 {
            return None;
        }
        let at = self.board.as_ref()?.lock().ok()?.incursion_at?;
        let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
        obs.pillboxes
            .iter()
            .filter(|p| p.alive() && ours(p.owner))
            .map(|p| p.pos)
            .chain(obs.depots.iter().filter(|d| ours(d.owner)).map(|d| d.pos))
            .min_by(|a, b| a.dist(at).total_cmp(&b.dist(at)))
            .filter(|w| w.dist(base) > BASE_R)
    }

    /// An outlying pillbox of mine, nearest the base: one to bring in.
    fn outlying(&self, obs: &Observation, base: V2) -> Option<Pillbox> {
        let plan = self.plan.as_ref();
        obs.pillboxes
            .iter()
            .filter(|p| p.owner == Some(obs.player) && p.pos.dist(base) > BASE_R && !plan.map(|pl| pl.items.iter().any(|i| i.at.dist(p.pos) < 1.0)).unwrap_or(false))
            .min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base)).then(a.id.cmp(&b.id)))
            .cloned()
    }

    /// Turrets: fetch an outlying pillbox of mine and set it down in the
    /// next bastion. The tank drives to it so the squad's walk is short,
    /// and drives it home again on the squad's back.
    fn turrets(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) {
        let base = self.base_of(obs, me);
        let Some(plan) = self.plan.as_mut() else { return };
        plan.add_turrets();
        plan.hear(obs);
        let Some((slot, forest)) = plan.turret_slot() else {
            // The pillboxes are down: the walls that close them in.
            if plan.turrets_stand() {
                self.act = Act::Turrets;
                self.doing = format!("the turrets stand: {}", plan.progress());
                self.rotate(obs, me, base, cmds);
            } else {
                self.fortify(obs, me, Some(base), cmds);
            }
            return;
        };
        let squad = &me.pillboys;
        if forest {
            // The bastion's tile is trees: fell it first, whatever else.
            if squad.aboard && squad.task.is_none() {
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: slot } });
                self.plan.as_mut().unwrap().felling_slot(slot);
                self.set_doing(Act::Turrets, "felling the turret's tile".into());
            } else {
                self.set_doing(Act::Turrets, "waiting for the squad".into());
            }
            return;
        }
        if squad.carrying.is_some() {
            self.fetching = None;
            if !squad.aboard || squad.task.is_some() {
                self.set_doing(Act::Turrets, "the squad is setting a pillbox down".into());
                return;
            }
            // A road post far out (roadmap 12h): carried out to it, the
            // squad's walk short; a castle slot: home to the post.
            let post = if slot.dist(base) > BASE_R + 2.0 { slot + (base - slot).norm().scale(3.0) } else { self.post(base) };
            if me.pos.dist(post) > DEFEND_HOLD {
                self.go(obs, me, post, cmds);
                self.set_doing(if slot.dist(base) > BASE_R + 2.0 { Act::RoadPost } else { Act::CarryHome }, if slot.dist(base) > BASE_R + 2.0 { format!("carrying a pillbox to a road post, {:.0} tiles", me.pos.dist(post)) } else { format!("bringing a pillbox home, {:.0} tiles", me.pos.dist(post)) });
                return;
            }
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Pillbox, at: slot } });
            self.plan.as_mut().unwrap().ordered(slot, obs.tick);
            self.set_doing(Act::Turrets, format!("setting it down in a turret: {}", self.plan.as_ref().unwrap().progress()));
            return;
        }
        if !squad.aboard || squad.task.is_some() {
            self.set_doing(Act::Turrets, if matches!(squad.task, Some(PillboyTask::Lift { .. })) { "the squad is lifting it".into() } else { "waiting for the squad".into() });
            return;
        }
        let target = self.fetching.and_then(|id| obs.pillboxes.iter().find(|p| p.id == id && p.owner == Some(obs.player)).cloned()).or_else(|| self.outlying(obs, base));
        let Some(p) = target else {
            self.fetching = None;
            // Nothing to bring in yet: the yard's walls and roads meanwhile.
            if self.plan.as_ref().map(|p| p.yard_left()).unwrap_or(false) {
                self.fortify(obs, me, Some(base), cmds);
            } else if !self.rotate(obs, me, base, cmds) {
                self.set_doing(Act::NoTarget, "no pillbox of ours out there to bring in".into());
            }
            return;
        };
        self.fetching = Some(p.id);
        // A mission with the fleet (roadmap 12f, Paul: "go out on a
        // mission with the team to start moving the pillbox along the
        // newly created road leading back to the base"): a far trip
        // waits for the wingmen first, so they go out in formation and
        // screen the carry home.
        if self.caps().has(crate::weights::Caps::ESCORT) && !self.members.is_empty() && p.pos.dist(base) > self.settings.w[crate::weights::W::FetchGather] && me.pos.dist(base) <= BASE_R + 2.0 && self.gathering(obs, me, cmds) {
            self.set_doing(Act::FetchGather, format!("{} (to fetch the pillbox at ({:.0},{:.0}))", self.doing, p.pos.x, p.pos.y));
            return;
        }
        if me.pos.dist(p.pos) > FETCH_NEAR + 1.5 {
            let to = p.pos + (me.pos - p.pos).norm().scale(FETCH_NEAR);
            self.go(obs, me, to, cmds);
            self.set_doing(Act::Fetch, format!("fetching the pillbox at ({:.0},{:.0}), {:.0} tiles", p.pos.x, p.pos.y, me.pos.dist(p.pos)));
            return;
        }
        if me.goal.is_some() {
            cmds.push(Command::Stop);
        }
        cmds.push(Command::Deploy { task: PillboyTask::Lift { pillbox: p.id } });
        self.set_doing(Act::Fetch, "the squad is lifting it".into());
    }

    /// The ladder: each rung's condition, top down; the first that holds
    /// (and is on) is done this tick.
    fn auto(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        let base = self.base_of(obs, me);
        if let Some(p) = self.plan.as_mut() {
            p.hear(obs);
        }
        let (tank, alarm) = self.base_threat(obs, base);
        let quiet = self.threatened.map(|t| obs.tick.saturating_sub(t)).unwrap_or(obs.tick);
        let stands = self.plan.as_ref().map(|p| p.castle_stands()).unwrap_or(false);
        let carrying = me.pillboys.carrying.is_some();
        let (slot, walls) = self.plan.as_ref().map(|p| p.turret_preview()).unwrap_or((false, false));
        let outlying = self.outlying(obs, base);
        let wanted = obs.pillboxes.iter().filter(|p| !Self::mine_or_allied(obs, p.owner) && !self.given_up.has(&p.id, obs.tick)).min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base))).map(|p| p.pos.dist(base));
        let depot = obs.depots.iter().filter(|d| !Self::mine_or_allied(obs, d.owner) && !self.depots_given_up.has(&d.id, obs.tick)).map(|d| d.pos.dist(base)).min_by(|a, b| a.total_cmp(b));
        let q = quiet as f32 / TICK_HZ as f32;
        let holds = [
            tank.is_some() || alarm,
            self.plan.as_ref().map(|p| !p.castle_stands()).unwrap_or(true) && !self.starved && self.base.is_some(),
            stands && quiet >= QUIET && ((slot && (carrying || outlying.is_some() || self.fetching.is_some())) || (walls && !self.starved)),
            wanted.map(|d| d <= EXPAND_R).unwrap_or(false),
            depot.is_some() || wanted.is_some(),
            true,
        ];
        self.why = [
            match (&tank, alarm) {
                (Some(t), _) => Why::ThreatTank(t.pos.dist(base)),
                (None, true) => Why::ThreatPillbox,
                _ => Why::Quiet { secs: q, within: BASE_ALARM },
            },
            match &self.plan {
                Some(_) => Why::Building { starved: self.starved },
                None => Why::NothingBuilt,
            },
            if !stands {
                Why::CastleFirst
            } else if !slot && !walls {
                Why::TurretsStand
            } else if !slot {
                Why::Walling
            } else if quiet < QUIET {
                Why::WaitingQuiet { need: QUIET as f32 / TICK_HZ as f32, now: q }
            } else if carrying {
                Why::Carrying
            } else {
                match &outlying {
                    Some(p) => Why::Outlying(p.pos.dist(base)),
                    None => Why::NoOutlying,
                }
            },
            match wanted {
                Some(d) if d <= EXPAND_R => Why::PillboxNear(d),
                Some(d) => Why::PillboxFar { within: EXPAND_R, nearest: d },
                None => Why::AllPillboxesOurs,
            },
            match (depot, wanted) {
                (Some(d), _) => Why::DepotNear(d),
                (None, Some(_)) => Why::DepotsOursPillboxesFar,
                _ => Why::MapOurs,
            },
            Why::Always,
        ];
        // The team's posture leans the ladder (LR 24): off, or with no
        // board, the first rung that holds, as before.
        let posture = match (self.caps().has(crate::weights::Caps::POSTURE), &self.board) {
            (true, Some(b)) => b.lock().ok().map(|b| b.posture),
            _ => None,
        };
        let mut rung = Rung::pick(&self.ladder, &holds, posture, self.settings.w[crate::weights::W::Posture]);
        if let Some(p) = posture {
            crate::meter::posture(p, Rung::pick(&self.ladder, &holds, None, 0.0).index(), rung.index());
        }
        // Smoothed (roadmap 12d): a new rung must win `confirm` thinks in a
        // row (a think is a quarter second) while the old one still holds;
        // a threat to the base (DefendBase), or the old rung no longer
        // holding, changes it at once.
        if self.settings.smooth_on() {
            if let Some(cur) = self.rung.filter(|c| *c != rung && rung != Rung::DefendBase && self.ladder[c.index()] && holds[c.index()]) {
                let need = (self.settings.w[crate::weights::W::Confirm].round().max(1.0) as Tick) * (TICK_HZ as Tick / 4);
                let since = match self.rung_challenger {
                    Some((r, t)) if r == rung => t,
                    _ => obs.tick,
                };
                self.rung_challenger = Some((rung, since));
                if obs.tick < since + need {
                    rung = cur;
                } else {
                    self.rung_challenger = None;
                }
            } else {
                self.rung_challenger = None;
            }
        }
        if self.rung != Some(rung) {
            self.last_move = None;
            self.rung = Some(rung);
        }
        self.rung_now = Some(rung);
        // A fleet's leader gathers it before it travels: the hunt and the
        // expand take it off, the fleet comes with it.
        if matches!(rung, Rung::Hunt | Rung::Expand) && self.gathering(obs, me, cmds) {
            return true;
        }
        match rung {
            Rung::DefendBase | Rung::Guard => self.defend_base(obs, me, cmds),
            Rung::Fortify => {
                self.fortify(obs, me, Some(base), cmds);
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Rung::Turrets => {
                self.turrets(obs, me, cmds);
                true
            }
            Rung::Hunt => {
                self.gun_reserved = false;
                self.hunt(obs, me, Some((base, EXPAND_R)), cmds);
                !self.gun_reserved
            }
            Rung::Expand => self.expand(obs, me, base, cmds),
        }
    }

    /// My place in the leader's wedge (`WEDGE`), where it will be in a
    /// second at the leader's speed.
    fn slot_point(&self, leader: &Tank, map: Option<&Map>) -> Option<V2> {
        let k = self.slot?;
        let facing = V2::from_heading(leader.heading);
        // On a bridge, in file (roadmap 12f3, LR 6.3): slot k is k + 1
        // gaps straight behind the leader, no side offset, until the
        // file is `column_past` tiles off the bridge.
        if let Some(m) = map.filter(|_| self.caps().has(crate::weights::Caps::COLUMN)) {
            use crate::weights::W;
            let gap = self.settings.w[W::ColumnGap].max(1.0);
            let back = gap * (k + 1) as f32 + self.settings.w[W::ColumnPast];
            if Self::bridge_near(m, leader, back) {
                return Some(leader.pos - facing.scale(gap * (k + 1) as f32));
            }
        }
        let row = (k / 2 + 1) as f32;
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let across = V2::new(-facing.y, facing.x);
        Some(leader.pos - facing.scale(WEDGE * row) + across.scale(side * WEDGE * row) + facing.scale(leader.speed))
    }

    /// A bridge under the leader, on its next three route tiles, or on
    /// the line behind it within `back` tiles (its file still on it).
    fn bridge_near(m: &Map, leader: &Tank, back: f32) -> bool {
        let facing = V2::from_heading(leader.heading);
        if m.at(leader.pos) == Terrain::Bridge || leader.route.iter().take(3).any(|p| m.at(*p) == Terrain::Bridge) {
            return true;
        }
        let mut d = 0.5;
        while d <= back {
            let p = leader.pos - facing.scale(d);
            if m.in_bounds(p) && m.at(p) == Terrain::Bridge {
                return true;
            }
            d += 0.5;
        }
        false
    }

    /// A fleet leader's wingmen near enough to lay its bridge.
    fn helpers<'a>(&self, obs: &'a Observation, me: &Tank) -> Vec<&'a Tank> {
        obs.tanks.iter().filter(|t| t.alive && !t.boat && self.members.contains(&t.owner) && t.pos.dist(me.pos) <= BRIDGE_HELP_R && t.pillboys.strength > 0).collect()
    }

    /// A fleet's leader, about to travel: half its wingmen within
    /// `GATHER_R`, or it holds for them (`GATHER_FOR` at most). True while
    /// it holds. Nothing to gather with no wingmen, a fight in reach, or
    /// afloat.
    /// Part-laid bridges on the team board (roadmap 12f, LR 6.2): the
    /// one I lay, posted; the tiles laid since, off; and, free of more
    /// pressing work, the squad aboard with material, the nearest one
    /// nobody lays, taken. The hop lays
    /// it this think (the doctrine drives an active hop).
    fn bridge_jobs(&mut self, b: &mut crate::board::TeamBoard, obs: &Observation, me: &Tank, map: Option<&Map>) {
        use crate::weights::W;
        if let Some((tiles, target)) = self.inner.hop.bridge_job() {
            b.post_bridge(obs.player, &tiles, target, obs.tick);
        }
        if let Some(m) = map {
            b.prune_bridges(m, obs.tick, self.settings.ticks(W::BridgeKeep));
        }
        let s = &me.pillboys;
        // On auto with nothing more pressing than expanding or guarding;
        // or a wingman, `bridge_wing` on: the leader that gave a bridge
        // up sails on, and its wingmen are the tanks there with material.
        let auto = self.goal == Goal::Auto && matches!(self.rung, Some(Rung::Expand | Rung::Guard));
        let wing = matches!(self.goal, Goal::Escort(_)) && self.settings.w[W::BridgeWing] >= 0.5;
        let idle = (auto || wing) && !self.resupplying && !self.inner.hop.active();
        if !(idle && me.alive && !me.boat && s.aboard && s.strength > 0 && s.material >= ROAD_COST) {
            return;
        }
        let hop = &self.inner.hop;
        if let Some((tiles, target)) = b.take_bridge(obs.player, me.pos, self.settings.w[W::BridgeReach], obs.tick, |t| !hop.bridge_refused(t, obs.tick)) {
            let left = tiles.len();
            if self.inner.hop.adopt_bridge(tiles, target, obs.tick) {
                self.set_doing(Act::TeamBridge, format!("taking the team's bridge to ({:.0},{:.0}), {} tiles left", target.x, target.y, left));
            }
        }
    }

    /// The duck (roadmap 12d, LR 19): a hostile tank with shells in
    /// direct range and line, its gun ready within `duck_ready` and
    /// before mine, and my armour share at or under `w_duck`: where to
    /// go, out of its range or line, and where the gun is. Of the points
    /// round at the distance that leaves its range with a margin (3 to 8
    /// tiles), the one out of reach nearest the errand (the last Move's
    /// goal, else here); none, straight away from the gun. A gun is the
    /// one in sight, or remembered for `duck_memory` from a sighting or
    /// a direct shell of its in flight.
    fn duck(&mut self, obs: &Observation, me: &Tank, map: Option<&Map>) -> Option<(V2, V2)> {
        use crate::weights::W;
        // The guns in sight, remembered: sight and direct range are both
        // 10 tiles, and a gunner just past sight leads its shot into it.
        let keep = self.settings.ticks(W::DuckMemory);
        for t in obs.hostile_tanks().filter(|t| t.alive && !t.boat && t.shells > 0) {
            self.guns_seen.retain(|g| g.0 != t.owner);
            self.guns_seen.push((t.owner, t.pos, t.reload, obs.tick));
        }
        for t in obs.hostile_tanks().filter(|t| !t.alive || t.boat || t.shells == 0) {
            self.guns_seen.retain(|g| g.0 != t.owner);
        }
        // A direct shell in flight says where its gun is, seen or not,
        // and that it has just reloaded.
        for sh in obs.shells.iter().filter(|s| s.kind == ShellKind::Direct && s.owner != obs.player && !obs.allied(s.owner)) {
            if !self.guns_seen.iter().any(|g| g.0 == sh.owner && g.3 >= sh.launch_tick) {
                self.guns_seen.retain(|g| g.0 != sh.owner);
                self.guns_seen.push((sh.owner, sh.from, RELOAD_TICKS, sh.launch_tick));
            }
        }
        self.guns_seen.retain(|g| obs.tick < g.3.saturating_add(keep));
        let w = &self.settings.w;
        if me.armor / TANK_ARMOR > w[W::Duck] {
            return None;
        }
        let ready = (w[W::DuckReady].max(0.0) * TICK_HZ as f32) as u16;
        let clear = |a: V2, b: V2| map.map(|m| m.line_clear(a, b)).unwrap_or(true);
        // A gun this near threatens (its lead reaches past its range by
        // the tank's own motion); a duck goes this far.
        let (reach, safe_at) = (DIRECT_RANGE + 2.0, DIRECT_RANGE + 3.0);
        let reload = |g: &(PlayerId, V2, u16, Tick)| g.2.saturating_sub(obs.tick.saturating_sub(g.3).min(u16::MAX as Tick) as u16);
        let gun = self
            .guns_seen
            .iter()
            .filter(|g| g.1.dist(me.pos) <= reach && reload(g) <= ready && (reload(g) < me.reload || me.shells == 0) && clear(g.1, me.pos))
            .min_by(|a, b| reload(a).cmp(&reload(b)).then(a.1.dist(me.pos).total_cmp(&b.1.dist(me.pos))))
            .map(|g| (g.1, reload(g)))?;
        let (gun, gun_ready) = gun;
        let errand = self.last_move.map(|(at, _)| at).unwrap_or(me.pos);
        let safe = |p: V2| p.dist(gun) > safe_at || !clear(gun, p);
        let ok = |p: V2| map.map(|m| m.in_bounds(p) && m.at(p).passable()).unwrap_or(true);
        let r = (safe_at - gun.dist(me.pos) + 0.5).clamp(3.0, 8.0);
        let ring = (0..16).map(|k| me.pos + V2::from_heading(k as f32 * std::f32::consts::TAU / 16.0).scale(r));
        // `duck_plan` (LR 57): the point held while it stays safe, so the
        // tank does not pivot to a new one each think; else each point
        // driven by `drive_path` to the tick the gun is ready, and the
        // nearest to the errand of those out of its reach or line by
        // then; failing that, the one farthest out of it then.
        if w[W::DuckPlan] >= 0.5 {
            if let Some((held, _)) = self.duck_to.filter(|(p, _)| ok(*p) && safe(*p) && p.dist(me.pos) <= 9.0) {
                return Some((held, gun));
            }
            let ticks = (gun_ready as u32).max(1);
            let hidden = |p: V2| p.dist(gun) > DIRECT_RANGE + 0.8 || !clear(gun, p);
            let scored: Vec<(V2, bool, f32)> = ring
                .clone()
                .filter(|p| ok(*p))
                .filter_map(|p| crate::drive_to(me, p, ticks, map).map(|at| (p, hidden(at), if clear(gun, at) { at.dist(gun) } else { f32::INFINITY })))
                .collect();
            let best = scored
                .iter()
                .filter(|(p, h, _)| *h && safe(*p))
                .min_by(|a, b| a.0.dist(errand).total_cmp(&b.0.dist(errand)))
                .or_else(|| scored.iter().filter(|(p, _, _)| safe(*p)).max_by(|a, b| a.2.total_cmp(&b.2)))
                .map(|x| x.0);
            if let Some(to) = best {
                return Some((obs.clamp(to), gun));
            }
        }
        let best = ring.filter(|p| ok(*p) && safe(*p)).min_by(|a, b| a.dist(errand).total_cmp(&b.dist(errand)));
        let to = best.unwrap_or_else(|| me.pos + (me.pos - gun).norm().scale(r));
        Some((obs.clamp(to), gun))
    }

    /// The leader waits for its fleet before going out: stopped, "form
    /// up on me", for up to `GATHER_FOR`, then on with a cool-down.
    /// Not in a fight, afloat or on a bridge. True while it waits.
    fn gathering(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        // Afloat they are on their way: counted, not near.
        let fleet: Vec<&Tank> = obs.tanks.iter().filter(|t| t.alive && self.members.contains(&t.owner)).collect();
        let fight = obs.hostile_tanks().any(|t| t.alive && t.pos.dist(me.pos) <= INDIRECT_MIN_RANGE + 4.0);
        // On a bridge the leader drives on straight and fast (LR 6.3): a
        // fleet gathers on land, not in file on the bridge.
        if fleet.is_empty() || fight || me.boat || obs.tick < self.gather_cool || self.on_bridge {
            self.gather_since = None;
            return false;
        }
        let near = fleet.iter().filter(|t| !t.boat && t.pos.dist(me.pos) <= GATHER_R).count();
        if near * 2 >= fleet.len() {
            self.gather_since = None;
            return false;
        }
        let since = *self.gather_since.get_or_insert(obs.tick);
        if obs.tick >= since + GATHER_FOR {
            self.gather_since = None;
            self.gather_cool = obs.tick + GATHER_COOL;
            self.say.push("fleet: moving on, close up on me".into());
            return false;
        }
        if since == obs.tick {
            self.say.push(format!("fleet: form up on me at ({:.0},{:.0})", me.pos.x, me.pos.y));
        }
        if me.goal.is_some() {
            cmds.push(Command::Stop);
        }
        self.last_move = None;
        self.set_doing(Act::Gather, format!("gathering the fleet: {} of {} in formation, {:.0} s", near, fleet.len(), (obs.tick - since) as f32 * DT));
        true
    }

    /// Expand: the nearest depot not of my side to the base, its guards
    /// first; with none to be had, any pillbox on the map.
    fn expand(&mut self, obs: &Observation, me: &Tank, base: V2, cmds: &mut Vec<Command>) -> bool {
        for e in &obs.events {
            if let Event::Rejected { player, cmd: Command::Move { .. }, why } = e {
                if *player == obs.player && (why.contains("water") || why.contains("unreachable")) {
                    if let Some(d) = self.depot_target.take() {
                        self.depots_given_up.put(d, obs.tick);
                    }
                }
            }
        }
        let open = |d: &&Depot| !Self::mine_or_allied(obs, d.owner) && !self.depots_given_up.has(&d.id, obs.tick);
        let nearest = obs.depots.iter().filter(open).min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base)).then(a.id.cmp(&b.id))).cloned();
        // The front read (backlog 49, LR 58): with the board's front known,
        // each depot costs `(1 - w)` its distance to the base and `w` its
        // distance to the front, so the team contests the ground where the
        // two sides meet; a depot so chosen is kept until it is taken or
        // given up, as the front moves each second.
        let w = self.settings.w[crate::weights::W::Front].clamp(0.0, 1.0);
        let board = w > 0.0 && self.caps().has(crate::weights::Caps::BOARD);
        let front = if board { self.front_points() } else { Vec::new() };
        let kept = self.depot_target.filter(|_| board).and_then(|id| obs.depots.iter().filter(open).find(|d| d.id == id)).cloned();
        let depot = if kept.is_some() {
            kept
        } else if !front.is_empty() {
            let to_front = |p: V2| front.iter().map(|c| c.dist(p)).fold(f32::INFINITY, f32::min);
            let picked = obs
                .depots
                .iter()
                .filter(open)
                .map(|d| (d, (1.0 - w) * d.pos.dist(base) + w * to_front(d.pos)))
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.id.cmp(&b.0.id)))
                .map(|(d, _)| d.clone());
            crate::meter::front(true, picked.as_ref().map(|d| d.id) != nearest.as_ref().map(|d| d.id));
            picked
        } else {
            if board {
                crate::meter::front(false, false);
            }
            nearest
        };
        let Some(d) = depot else {
            self.depot_target = None;
            self.gun_reserved = false;
            self.hunt(obs, me, None, cmds);
            return !self.gun_reserved;
        };
        self.depot_target = Some(d.id);
        let guard = PILLBOX_RANGE + 1.5;
        let guarded = obs.pillboxes.iter().any(|p| p.alive() && !Self::mine_or_allied(obs, p.owner) && !self.given_up.has(&p.id, obs.tick) && p.pos.dist(d.pos) <= guard);
        if guarded {
            self.gun_reserved = false;
            self.hunt(obs, me, Some((d.pos, guard)), cmds);
            self.set_doing(Act::TakeDepot, format!("clearing the depot at ({:.0},{:.0}): {}", d.pos.x, d.pos.y, self.doing));
            return !self.gun_reserved;
        }
        if me.pos.dist(d.pos) > DEPOT_RADIUS * 0.5 {
            self.go(obs, me, d.pos, cmds);
            self.set_doing(Act::TakeDepot, format!("to the depot at ({:.0},{:.0}), {:.0} tiles", d.pos.x, d.pos.y, me.pos.dist(d.pos)));
        } else {
            self.set_doing(Act::TakeDepot, format!("taking the depot: {:.0}%", d.capture as f32 * 100.0 / CAPTURE_TICKS as f32));
        }
        true
    }

    /// Team chat, oldest first, one line a second; a backlog of more
    /// than three is stale and dropped.
    fn flush_say(&mut self, obs: &Observation, cmds: &mut Vec<Command>) {
        if self.say.len() > 3 {
            self.say.drain(..self.say.len() - 3);
        }
        if !self.say.is_empty() && obs.tick > self.said + TICK_HZ as Tick {
            self.said = obs.tick;
            cmds.push(Command::Chat { text: self.say.remove(0), to: ChatTo::Team });
        }
    }

    /// Revive (Paul, 16 October: "can we have an ally repair a fallen
    /// tank"): an allied tank down, not yet dead, inside `REVIVE_REACH`,
    /// with time left for the drive, the squad's walk and the work. The
    /// one ally best placed goes, by what each can see: none nearer with
    /// a squad aboard and the material, and no squad already at it. It
    /// says so, drives beside the wreck, sends the squad and holds
    /// while it works. True while it is driving the tank.
    fn revive(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        let down = |t: &Tank| !t.alive && t.last_hit_tick > 0 && obs.tick < t.last_hit_tick + DOWNED_TICKS as Tick;
        let left = |t: &Tank| (t.last_hit_tick + DOWNED_TICKS as Tick).saturating_sub(obs.tick) as f32 * DT;
        // At it now: hold while the squad works.
        // The world refused the squad (a wreck at sea, out of reach):
        // that fall is given up.
        for e in &obs.events {
            if let Event::Rejected { player, cmd: Command::Deploy { task: PillboyTask::Repair { target } }, .. } = e {
                if *player == obs.player && self.reviving == Some(*target) {
                    self.revive_failed.put(*target, obs.tick);
                    self.reviving = None;
                }
            }
        }
        if let Some(id) = self.reviving {
            match obs.tanks.iter().find(|t| t.id == id) {
                Some(w) if down(w) && me.pillboys.strength > 0 => {
                    let at_it = matches!(me.pillboys.task, Some(PillboyTask::Repair { target }) if target == id);
                    let name = obs.players.iter().find(|p| p.id == w.owner).map(|p| p.name.clone()).unwrap_or_default();
                    if at_it {
                        self.set_doing(Act::Revive, format!("REVIVE: the squad is mending {}, {:.0} s left", name, left(w)));
                        if me.goal.is_some() {
                            cmds.push(Command::Stop);
                        }
                        return true;
                    }
                    // No longer time for the drive and the work: give up.
                    let need = (me.pos.dist(w.pos) - REVIVE_NEAR).max(0.0) / TANK_SPEED + REVIVE_TICKS as f32 * DT;
                    if me.pillboys.material < REVIVE_COST || me.boat || need > left(w) {
                        self.reviving = None;
                        self.last_move = None;
                        return false;
                    }
                    // Out on something else: called home, and it meets
                    // the tank on its way to the wreck.
                    if !me.pillboys.aboard && !matches!(me.pillboys.task, Some(PillboyTask::Return)) {
                        cmds.push(Command::Deploy { task: PillboyTask::Return });
                    }
                    if me.pos.dist(w.pos) > REVIVE_NEAR {
                        let to = w.pos + (me.pos - w.pos).norm().scale(REVIVE_NEAR * 0.6);
                        self.go(obs, me, to, cmds);
                        self.set_doing(Act::Revive, format!("REVIVE: to {}, {:.0} tiles, {:.0} s left{}", name, me.pos.dist(w.pos), left(w), if me.pillboys.aboard { "" } else { ", the squad called in" }));
                    } else if me.pillboys.aboard {
                        cmds.push(Command::Deploy { task: PillboyTask::Repair { target: id } });
                        self.set_doing(Act::Revive, format!("REVIVE: the squad out to {}", name));
                    } else {
                        if me.goal.is_some() {
                            cmds.push(Command::Stop);
                        }
                        self.set_doing(Act::Revive, format!("REVIVE: by {}, waiting for the squad, {:.0} s left", name, left(w)));
                    }
                    return true;
                }
                _ => {
                    self.reviving = None;
                    self.last_move = None;
                    return false;
                }
            }
        }
        if me.boat || me.pillboys.strength == 0 || me.pillboys.material < REVIVE_COST {
            return false;
        }
        // A squad out walks back first (`PILLBOY_SPEED`), but no further
        // than it is from the wreck.
        let squad_back = |w: &Tank| if me.pillboys.aboard { 0.0 } else { me.pillboys.pos.dist(w.pos).min(me.pillboys.pos.dist(me.pos)) / PILLBOY_SPEED };
        let able = |t: &Tank| t.alive && !t.boat && t.pillboys.strength > 0 && t.pillboys.material >= REVIVE_COST;
        let wreck = obs
            .tanks
            .iter()
            .filter(|w| w.owner != obs.player && obs.allied(w.owner) && down(w) && w.pos.dist(me.pos) <= REVIVE_REACH && !self.revive_failed.has(&w.id, obs.tick))
            // Time for the drive, the walk and the work.
            .filter(|w| (me.pos.dist(w.pos) / (TANK_SPEED * 0.7)).max(squad_back(w)) + 2.0 + REVIVE_TICKS as f32 * DT < left(w))
            // Nobody at it already, and nobody able nearer.
            .filter(|w| !obs.tanks.iter().any(|a| a.owner != obs.player && obs.allied(a.owner) && matches!(a.pillboys.task, Some(PillboyTask::Repair { target }) if target == w.id)))
            .filter(|w| !obs.tanks.iter().any(|a| a.owner != obs.player && a.owner != w.owner && obs.allied(a.owner) && able(a) && a.pos.dist(w.pos) < me.pos.dist(w.pos)))
            // Not into a fight being lost: the guns that downed it are
            // likely still there (tactics.rs odds, below `REVIVE_ODDS`).
            .filter(|w| self.inner.tactics.as_ref().map(|t| t.odds(obs, me, w.pos).ratio() >= REVIVE_ODDS).unwrap_or(true))
            .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)).then(a.id.cmp(&b.id)))
            .cloned();
        let Some(w) = wreck else { return false };
        self.reviving = Some(w.id);
        self.last_move = None;
        let name = obs.players.iter().find(|p| p.id == w.owner).map(|p| p.name.clone()).unwrap_or_default();
        self.say.push(format!("reviving {} at ({:.0},{:.0}), {:.0} tiles, {:.0} s left", name, w.pos.x, w.pos.y, me.pos.dist(w.pos), left(&w)));
        self.revive(obs, me, cmds)
    }

    /// Chain help (Paul: "tell the bots to prioritize chain help"). A
    /// tank of my side that hitches a chain alone cannot move what it is
    /// on: the bot of the side nearest it that is free (alive, ashore,
    /// on no chain) answers, the one bot and not all of them, by what
    /// each can see. It says so, drives over, hitches on, and then keeps
    /// beside the chainer, driving where the chainer drives, so the two
    /// haul together. It lets go when the chainer does, or is lost. True
    /// while it is driving the tank.
    fn help_chain(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        if self.goal == Goal::Manual {
            return false;
        }
        for e in &obs.events {
            let Event::Chained { owner, target, on, haulers, .. } = e else { continue };
            if *owner == obs.player || !obs.allied(*owner) {
                continue;
            }
            match (*on, self.helping) {
                // A crew on a haul of its own (crew.rs) is not chain help.
                (true, None) if *haulers == 1 && me.towing.is_none() && !me.boat && self.crew.hauling() != Some(*target) => {
                    let Some(at) = Self::chain_target(obs, *target) else { continue };
                    let bot = |p: PlayerId| obs.players.iter().any(|x| x.id == p && x.is_bot);
                    let nearest = obs.allied_tanks().chain(std::iter::once(me)).filter(|t| t.alive && !t.boat && t.towing.is_none() && t.owner != *owner && bot(t.owner)).min_by(|a, b| a.pos.dist(at).total_cmp(&b.pos.dist(at)).then(a.owner.cmp(&b.owner))).map(|t| t.owner);
                    if nearest == Some(obs.player) {
                        self.helping = Some((*owner, *target));
                        let who = obs.players.iter().find(|p| p.id == *owner).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", owner));
                        self.say.push(format!("on my way to the chain with {}, {:.0} tiles", who, me.pos.dist(at)));
                    }
                }
                (false, Some((who, t))) if who == *owner && t == *target => {
                    self.helping = None;
                    if me.towing == Some(t) {
                        cmds.push(Command::Tow { target: None });
                    }
                    return false;
                }
                _ => {}
            }
        }
        let Some((who, target)) = self.helping else { return false };
        let chainer = obs.allied_tanks().find(|t| t.owner == who && t.alive).cloned();
        let (Some(at), Some(c)) = (Self::chain_target(obs, target), chainer) else {
            self.helping = None;
            if me.towing.is_some() {
                cmds.push(Command::Tow { target: None });
            }
            return false;
        };
        if c.towing != Some(target) {
            self.helping = None;
            if me.towing == Some(target) {
                cmds.push(Command::Tow { target: None });
            }
            return false;
        }
        let name = obs.players.iter().find(|p| p.id == who).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", who));
        if me.towing != Some(target) {
            if me.pos.dist(at) <= TOW_REACH * 0.8 {
                if obs.tick >= self.last_hitch + TICK_HZ as Tick {
                    self.last_hitch = obs.tick;
                    cmds.push(Command::Tow { target: Some(target) });
                    self.say.push(format!("on the chain with {}: pull", name));
                }
            } else {
                // Beside it, on the far side from the chainer.
                let side = (at - c.pos).norm();
                let spot = at + V2::new(-side.y, side.x).scale(1.2);
                self.go(obs, me, spot, cmds);
            }
            self.set_doing(Act::Chain, format!("CHAIN: to help {} haul ({:.0},{:.0}), {:.0} tiles", name, at.x, at.y, me.pos.dist(at)));
            return true;
        }
        // Hauling: where the chainer drives, a tile and a half to its side.
        let dest = c.route.last().copied().or(c.goal).unwrap_or(c.pos);
        let across = (c.pos - at).norm();
        let beside = V2::new(-across.y, across.x).scale(1.5);
        self.go(obs, me, dest + beside, cmds);
        self.set_doing(Act::Chain, format!("CHAIN: hauling with {}", name));
        true
    }

    /// Where a chain's work is: the pillbox or depot of that id, if seen.
    fn chain_target(obs: &Observation, id: EntityId) -> Option<V2> {
        obs.pillboxes.iter().find(|p| p.id == id).map(|p| p.pos).or_else(|| obs.depots.iter().find(|d| d.id == id).map(|d| d.pos))
    }

    /// A seat's flank in a group attack: seats taken three ways, left,
    /// centre and right, each coming in sixty degrees round from the way
    /// it was coming, so three bots answering one ping close from three
    /// sides.
    fn flank(seat: PlayerId, from: V2) -> V2 {
        let turn = [-1.05f32, 0.0, 1.05][seat as usize % 3];
        let (s, c) = turn.sin_cos();
        V2::new(from.x * c - from.y * s, from.x * s + from.y * c)
    }

    /// The seat's flank in a word, as `flank` takes it.
    fn flank_word(seat: PlayerId) -> &'static str {
        ["left", "centre", "right"][seat as usize % 3]
    }

    /// What a ping marks, in a word or two: the hostile tank by it, or
    /// the place.
    fn called(obs: &Observation, at: V2) -> String {
        match obs.hostile_tanks().find(|t| t.alive && t.pos.dist(at) <= 4.0) {
            Some(t) => obs.players.iter().find(|p| p.id == t.owner).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", t.owner)),
            None => format!("ping at ({:.0},{:.0})", at.x, at.y),
        }
    }

    /// A teammate's ping within `RALLY_REACH`, if this bot answers them.
    fn hear_pings(&mut self, obs: &Observation, me: &Tank) {
        self.heard.retain(|(_, t)| obs.tick < t + PING_EVERY);
        for e in &obs.events {
            if let Event::Pinged { player, at } = e {
                if *player == obs.player || !obs.allied(*player) {
                    continue;
                }
                self.heard.push((*at, obs.tick));
                // A fleet's orders: a wingman in a wedge answers its own
                // leader's calls, all of them, and nobody else's while
                // the leader is up.
                let leader = match self.goal {
                    Goal::Escort(l) if self.slot.is_some() => Some(l),
                    _ => None,
                };
                let order = leader == Some(*player);
                let leader_up = leader.map(|l| obs.players.iter().any(|p| p.id == l && p.alive)).unwrap_or(false);
                if leader_up && !order {
                    continue;
                }
                if self.answers_pings && me.pos.dist(*at) <= RALLY_REACH && obs.tick >= self.rally_cool {
                    let fresh = self.rally.map(|(r, _)| r.dist(*at) > PING_HEARD).unwrap_or(true);
                    self.rally_order = order;
                    // Assignment (tactics.rs): enough of us nearer it
                    // already; a crew on a job keeps to its reach. Not
                    // for the leader's order: the fleet goes together.
                    if let Some(t) = self.inner.tactics.as_ref().filter(|_| !order) {
                        if fresh {
                            if let Some(n) = t.enough_answer(obs, me, *at, *player) {
                                self.set_doing(Act::Ping, format!("the ping at ({:.0},{:.0}): {} of us nearer, I keep at it", at.x, at.y, n));
                                continue;
                            }
                            if self.crew.working() && me.pos.dist(*at) > t.w.crew_reach {
                                self.set_doing(Act::Ping, format!("the ping at ({:.0},{:.0}): {:.0} out, past a crew's {:.0}", at.x, at.y, me.pos.dist(*at), t.w.crew_reach));
                                continue;
                            }
                        }
                    }
                    self.rally = Some((*at, obs.tick + RALLY_FOR));
                    if fresh {
                        self.rally_from = Some(Self::flank(obs.player, (me.pos - *at).norm()));
                        self.say.push(format!("{} on the {}, {:.0} tiles out", Self::flank_word(obs.player), Self::called(obs, *at), me.pos.dist(*at)));
                    }
                }
            }
        }
    }

    /// A hostile tank in sight that nobody of the side has called: ping
    /// it, and go for it too.
    fn call_enemy(&mut self, obs: &Observation, me: &Tank) -> Option<Command> {
        if !self.pings || (self.last_ping > 0 && obs.tick < self.last_ping + PING_EVERY) {
            return None;
        }
        let e = obs.hostile_tanks().filter(|t| t.alive && t.pos.dist(me.pos) <= PING_SIGHT).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))?;
        if self.heard.iter().any(|(at, _)| at.dist(e.pos) <= PING_HEARD) {
            return None;
        }
        self.last_ping = obs.tick;
        if self.answers_pings && obs.tick >= self.rally_cool {
            self.rally = Some((e.pos, obs.tick + RALLY_FOR / 2));
            self.rally_from = Some((me.pos - e.pos).norm());
        }
        // The call, in team chat: who, where, and the plan for how many
        // of us can come.
        let near = obs.allied_tanks().filter(|t| t.alive && t.owner != obs.player && t.pos.dist(e.pos) <= RALLY_REACH).count();
        let who = obs.players.iter().find(|p| p.id == e.owner).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", e.owner));
        let plan = match near {
            // A fleet's leader: the call is the fleet's order.
            _ if !self.members.is_empty() => format!("fleet: attack together, {} of us", 1 + obs.allied_tanks().filter(|t| t.alive && self.members.contains(&t.owner) && t.pos.dist(me.pos) <= RALLY_REACH).count()),
            0 => "engaging alone".to_string(),
            1 => "focus fire with me".to_string(),
            n => format!("pincer, {} of us: left, centre, right", n + 1),
        };
        self.say.push(format!("contact: {} at ({:.0},{:.0}), {:.0} armour. {}", who, e.pos.x, e.pos.y, e.armor, plan));
        Some(Command::Ping { at: e.pos })
    }

    /// Answering a ping: to it, and onto any hostile tank near it, with
    /// the doctrine's gun free. A wingman, a ranger, a leader on Auto; not
    /// a tank going home low. Ends when the time is up, or at the point
    /// with nothing there. True while it is driving the tank.
    fn rallying(&mut self, obs: &Observation, me: &Tank, cmds: &mut Vec<Command>) -> bool {
        let Some((at, until)) = self.rally else { return false };
        if obs.tick >= until || !matches!(self.goal, Goal::Auto | Goal::Escort(_) | Goal::HuntPillboxes | Goal::Hold(_)) || me.shells <= LOW_SHELLS || me.armor <= LOW_ARMOR {
            self.rally = None;
            return false;
        }
        let enemy = obs.hostile_tanks().filter(|t| t.alive && t.pos.dist(at) <= PING_SIGHT * 0.7).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
        match enemy {
            Some(e) => {
                // Follow it, and keep at it while it is seen.
                self.rally = Some((e.pos, until.max(obs.tick + 10 * TICK_HZ as Tick)));
                self.rally_seen = Some(obs.tick);
                let from = self.rally_from.unwrap_or_else(|| (me.pos - e.pos).norm());
                let who = obs.players.iter().find(|p| p.id == e.owner).map(|p| p.name.clone()).unwrap_or_else(|| format!("p{}", e.owner));
                // The engagement (tactics.rs): the square law's odds say
                // how near; of the flank's firing points, the one with
                // the fewest other guns on it.
                let (range, word, at_odds, stance) = match self.inner.tactics.as_mut() {
                    Some(t) => {
                        let odds = t.odds(obs, me, e.pos);
                        let r = odds.ratio();
                        let mut stance = t.stance(obs.tick, &odds);
                        // The fleet's order, with the fleet there: attack
                        // together, whatever one tank's odds say.
                        let with_me = obs.allied_tanks().filter(|a| a.alive && a.owner != obs.player && a.pos.dist(e.pos) <= t.w.odds_r + 4.0).count();
                        if self.rally_order && with_me >= 2 && stance != crate::tactics::Stance::Press {
                            stance = crate::tactics::Stance::Press;
                        }
                        match stance {
                            crate::tactics::Stance::Mass => (DIRECT_RANGE * 1.8, format!("massing, {} coming", odds.coming), r, Some(stance)),
                            crate::tactics::Stance::Engage => (DIRECT_RANGE * 0.8, "engaging".to_string(), r, Some(stance)),
                            crate::tactics::Stance::Press => (DIRECT_RANGE * 0.5, "pressing".to_string(), r, Some(stance)),
                        }
                    }
                    None => (DIRECT_RANGE * 0.7, "engaging".to_string(), 1.0, None),
                };
                // Kiting (tactics.rs): past its reach until its gun is
                // reloading, then in, a shot, and out.
                let kite = match (self.inner.tactics.as_ref(), stance) {
                    (Some(t), Some(st)) if t.w.kite => Some(t.kite(obs, me, e, from, st, DIRECT_RANGE * 1.8)),
                    _ => None,
                };
                let word = match kite {
                    Some(k) => format!("{}, {}", word, k.step.word()),
                    None => word,
                };
                let (stand, guns) = match (kite, self.inner.tactics.as_ref()) {
                    (Some(k), _) => (k.to, 0),
                    (None, Some(t)) => t.stand(obs, e, from, range),
                    (None, None) => (e.pos + from.scale(range), 0),
                };
                if kite.map(|k| k.stepping_in).unwrap_or(false) {
                    // A step in is a dash: every tick, not once a second.
                    self.last_move = None;
                }
                // A standoff (the kiting team held fewer depots, 3.8
                // against 4.4): past its reach for `KITE_STALL` with no
                // window, the errand matters more. Back to the goal, and
                // no rally for `RALLY_COOL`.
                match kite {
                    Some(k) if !k.stepping_in => {
                        let since = match self.kite_held {
                            Some((id, t)) if id == e.id => t,
                            _ => obs.tick,
                        };
                        self.kite_held = Some((e.id, since));
                        if obs.tick >= since + KITE_STALL {
                            self.kite_held = None;
                            self.rally = None;
                            self.rally_cool = obs.tick + RALLY_COOL;
                            self.last_move = None;
                            self.set_doing(Act::Rally, format!("RALLY: a standoff with {}: back to the errand", who));
                            return false;
                        }
                    }
                    _ => self.kite_held = None,
                }
                self.go(obs, me, stand, cmds);
                self.set_doing(kite.map(|k| Act::RallyKite(k.step)).unwrap_or(Act::Rally), format!("RALLY: on {} at ({:.0},{:.0}), the {} flank, odds {:.1}: {}{}", who, e.pos.x, e.pos.y, Self::flank_word(obs.player), at_odds, word, if guns > 0 { format!(", {} other guns on the spot", guns) } else { String::new() }));
            }
            None if me.pos.dist(at) <= 5.0 => {
                self.rally = None;
                return false;
            }
            // Kiting, and the target out of the side's sight: hold the
            // ring round where it was last seen, not drive onto the spot
            // and into its reach.
            None if self.inner.tactics.as_ref().map(|t| t.w.kite).unwrap_or(false) && self.rally_seen.map(|s| obs.tick < s + RALLY_LOST).unwrap_or(false) => {
                let ring = DIRECT_RANGE + self.inner.tactics.as_ref().unwrap().w.kite_margin;
                if me.pos.dist(at) < ring - 0.5 || me.pos.dist(at) > ring + 4.0 {
                    let to = at + (me.pos - at).norm().scale(ring);
                    self.go(obs, me, to, cmds);
                }
                self.set_doing(Act::RallyLost, format!("RALLY: lost sight of it at ({:.0},{:.0}): holding past its reach", at.x, at.y));
            }
            None => {
                self.go(obs, me, at, cmds);
                self.set_doing(Act::Rally, format!("RALLY: to the ping at ({:.0},{:.0}), {:.0} tiles", at.x, at.y, me.pos.dist(at)));
            }
        }
        true
    }

    /// The target the doctrine's gun is on and why, for `status`.
    fn fire_word(&self) -> String {
        match self.inner.tactics.as_ref() {
            Some(t) if t.target.is_some() => format!(" | target {}", t.reason),
            _ => String::new(),
        }
    }

    /// No line from here: round the target to the next firing point; a
    /// whole circle and a half without one, and it is given up.
    fn blocked(&mut self, now: Tick) {
        let b = self.bearing.unwrap_or(0.0) + STEP_ROUND;
        self.bearing = Some(b);
        self.walked += 1;
        if let (Some(t), true) = (self.target, self.walked > 12) {
            self.given_up.put(t, now);
            self.target = None;
            self.bearing = None;
        }
    }

    /// Hold by a point, fight what comes, mend what is ours there.
    fn defend(&mut self, obs: &Observation, me: &Tank, at: V2, mend: Option<EntityId>, cmds: &mut Vec<Command>) {
        self.defend_within(obs, me, at, DEFEND_LEASH, mend, cmds)
    }

    /// Defend `at` within `leash`: the nearest hostile tank inside it
    /// kited from the defended side, shot in direct reach or met between
    /// it and `at`; with none, to the post and on guard. `mend`, a
    /// pillbox, is repaired by the squad when hurt and nothing threatens.
    fn defend_within(&mut self, obs: &Observation, me: &Tank, at: V2, leash: f32, mend: Option<EntityId>, cmds: &mut Vec<Command>) {
        // A hostile tank inside the leash and in direct reach is shot
        // here; beyond, the inner doctrine's indirect fire answers.
        let threat = obs
            .tanks
            .iter()
            .filter(|t| t.alive && !obs.allied(t.owner) && t.pos.dist(at) <= leash)
            .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)).then(a.id.cmp(&b.id)));
        // Kiting it (tactics.rs), from the side of what is defended and
        // on the leash: in range, the gun; the tank steps in only while
        // its gun cannot answer. Without the weights, stand and trade.
        let kite = match (threat, self.inner.tactics.as_mut()) {
            (Some(t), Some(tc)) if tc.w.kite => {
                let odds = tc.odds(obs, me, t.pos);
                let st = tc.stance(obs.tick, &odds);
                Some(tc.kite(obs, me, t, (at - t.pos).norm(), st, DIRECT_RANGE * 1.8))
            }
            _ => None,
        };
        match (threat, kite) {
            (Some(t), Some(k)) => {
                if t.pos.dist(me.pos) <= DIRECT_RANGE - 0.2 && me.reload == 0 && me.shells > 0 && obs.tick >= self.hold_fire_until {
                    cmds.push(Command::FireDirect { at: crate::lead(me, t) });
                }
                let to = if k.to.dist(at) > leash { at + (k.to - at).norm().scale(leash) } else { k.to };
                if k.stepping_in {
                    self.last_move = None;
                }
                self.go(obs, me, to, cmds);
                self.set_doing(Act::Kite(k.step), format!("kiting the intruder: {}", k.step.word()));
            }
            (Some(t), None) if t.pos.dist(me.pos) <= DIRECT_RANGE - 0.2 => {
                if me.reload == 0 && me.shells > 0 && obs.tick >= self.hold_fire_until {
                    cmds.push(Command::FireDirect { at: crate::lead(me, t) });
                }
                self.set_doing(Act::Engage, "engaging an intruder".into());
            }
            (Some(t), None) => {
                // Between the intruder and what is defended, not past it.
                let toward = at + (t.pos - at).norm().scale(DEFEND_HOLD.min(t.pos.dist(at)));
                self.go(obs, me, toward, cmds);
                self.set_doing(Act::Cover, "moving to cover it".into());
            }
            (None, _) => {
                if me.pos.dist(at) > DEFEND_HOLD && self.over_water(at) {
                    self.set_doing(Act::NoWayToPost, format!("the post is over the water, {:.0} tiles, and no boat: holding here", me.pos.dist(at)));
                } else if me.pos.dist(at) > DEFEND_HOLD {
                    self.go(obs, me, at, cmds);
                    self.set_doing(Act::ToPost, format!("to the post, {:.0} tiles", me.pos.dist(at)));
                } else {
                    self.set_doing(Act::OnGuard, "on guard".into());
                }
            }
        }
        // Mend the pillbox when it is hurt and there is material.
        if let Some(id) = mend {
            if let Some(p) = obs.pillboxes.iter().find(|p| p.id == id) {
                let squad = &me.pillboys;
                if p.health < PILLBOX_HEALTH * 0.7 && squad.aboard && squad.task.is_none() && squad.material >= PILLBOX_REPAIR_COST && threat.is_none() {
                    cmds.push(Command::Deploy { task: PillboyTask::Repair { target: id } });
                    self.set_doing(Act::Mend, "mending the pillbox".into());
                } else if !p.alive() && p.owner != Some(obs.player) && squad.aboard && squad.task.is_none() && squad.strength > 0 {
                    // Lost while we watched: take it back.
                    cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: id } });
                    self.set_doing(Act::Retake, "taking the pillbox back".into());
                }
            }
        }
    }
}

impl Brain for Commander {
    fn name(&self) -> &str {
        &self.name
    }

    fn settings(&self) -> Option<&crate::weights::Settings> {
        Some(&self.settings)
    }

    fn set_settings(&mut self, s: crate::weights::Settings) {
        Commander::set_settings(self, s);
    }

    fn rung(&self) -> Option<Rung> {
        self.rung_now
    }
    /// The inner RuleBot's: the doctrine runs the landfall.
    fn beach(&self) -> Option<V2> {
        self.inner.beach()
    }
    fn dead(&mut self, obs: &Observation) {
        if let Some(me) = &obs.me {
            self.dodge.watch(me, obs.tick);
        }
    }
    fn act(&self) -> Act {
        self.act
    }

    fn status(&self) -> String {
        match (self.goal, self.rung) {
            (Goal::Manual, _) => String::new(),
            (Goal::Auto, Some(r)) if self.fleet.on && self.plan.is_some() => format!("auto > {} (commanding {} crews): {}{}", r.label(), self.fleet.crews(), self.doing, self.fire_word()),
            (Goal::Auto, Some(r)) => format!("auto > {}: {}{}", r.label(), self.doing, self.fire_word()),
            (g, _) => format!("{}: {}{}", g.label(), self.doing, self.fire_word()),
        }
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        self.think_released(obs, None)
    }

    fn think_on(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        self.rung_now = None;
        self.think_released(obs, map)
    }
}

impl Commander {
    /// The think, then `dodge_release` (lib.rs `Dodge::release`, LR 53):
    /// an errand that would drive back under a dodged shell before it
    /// lands waits, and is asked for again (`last_move` cleared); and
    /// `dodge_fire` (`Dodge::hold_fire`, LR 57), dodging or not: a shot
    /// whose turn of the hull would carry the tank into a splash waits.
    fn think_released(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        let mut cmds = self.think_map(obs, map);
        if let Some(me) = obs.me.as_ref().filter(|m| m.alive) {
            if self.act != Act::Dodge && self.dodge.release(obs, me, &mut cmds, map) {
                self.last_move = None;
            }
            self.dodge.hold_fire(obs, me, &mut cmds, map);
            if obs.tick % TICK_HZ as Tick == 0 {
                crate::meter::kite(self.act);
                let squad = &me.pillboys;
                crate::meter::squad(squad.deployed().then(|| match &squad.task {
                    Some(t) => {
                        let water = match t {
                            PillboyTask::Build { at, .. } => map.map(|m| matches!(m.at(*at), Terrain::Water | Terrain::Bridge)).unwrap_or(false),
                            _ => false,
                        };
                        crate::meter::SquadTask::of(t, water)
                    }
                    None => crate::meter::SquadTask::Idle,
                }));
            }
        }
        if self.inner.skill < 1.0 {
            self.blunt(obs, &mut cmds);
        }
        cmds
    }

    /// Under full skill (LR 67), what the Commander says, made worse
    /// where every path to the gun and the hangar meets: a shell's aim
    /// off by up to (1 - skill) of a rookie's scatter (a direct shot a
    /// quarter of that), and no drone up before (1 - skill) of a
    /// rookie's 90 s.
    fn blunt(&mut self, obs: &Observation, cmds: &mut Vec<Command>) {
        let off = 1.0 - self.inner.skill.clamp(0.0, 1.0);
        let held = obs.seconds() < off * crate::ROOKIE_LATE;
        cmds.retain(|c| !(held && matches!(c, Command::Launch { .. })));
        for c in cmds.iter_mut() {
            match c {
                Command::FireIndirect { at } => *at = *at + self.inner.rng.disc().scale(off * crate::ROOKIE_SCATTER),
                Command::FireDirect { at } => *at = *at + self.inner.rng.disc().scale(off * crate::ROOKIE_SCATTER * 0.1),
                _ => {}
            }
        }
    }
}

impl Commander {
    /// The think: the inner doctrine every tick, then the goal (Auto's
    /// ladder, or the person's), and of the doctrine's orders only what
    /// the goal leaves it (`keep_inner`).
    fn think_map(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        let mut cmds = Vec::new();
        // The doctrine thinks every tick so its memory stays current;
        // what it says is kept or dropped below.
        self.inner.hops = self.goal == Goal::Auto || self.inner.hop.sent();
        if let Some(t) = self.inner.tactics.as_mut() {
            t.base = self.base;
        }
        if let (Some(p), Some(m)) = (self.plan.as_mut(), map) {
            p.fit_post(m);
        }
        // The push point leans toward the board's front (LR 61).
        self.rotation.front_w = self.settings.w[crate::weights::W::FrontPush].clamp(0.0, 1.0);
        self.rotation.front = if self.rotation.front_w > 0.0 && !self.rotation.off { self.front_points() } else { Vec::new() };
        if let (Some(b), Some(m)) = (self.base, map) {
            if !self.rotation.off {
                self.rotation.fit(obs, m, b);
            }
        }
        // The fleet lays a bridge together: a leader's wingmen near it
        // are its helpers, and a wingman takes its leader's jobs.
        if let Goal::Escort(l) = self.goal {
            if self.slot.is_some() {
                self.crew.leader = Some(l);
            }
        }
        self.inner.hop.no_bridges = (self.slot.is_some() && matches!(self.goal, Goal::Escort(_))) || !self.caps().has(crate::weights::Caps::BRIDGES);
        // Helpers with material: the leader waits for them at the end
        // rather than leaving it to fetch its own.
        self.inner.hop.helpers = match obs.me.as_ref() {
            Some(me) => self.helpers(obs, me).iter().filter(|t| t.pillboys.material >= ROAD_COST).count(),
            None => 0,
        };
        self.crew.take_jobs(obs);
        // The team board (roadmap 12e): post where I am and what I do;
        // rebuild it if it is due; a job given me that I missed, take.
        if self.caps().has(crate::weights::Caps::BOARD) {
            if let (Some(b), Some(me)) = (self.board.clone(), obs.me.as_ref()) {
                if let Ok(mut b) = b.lock() {
                    b.post(obs, me, &self.doing);
                    if b.due(obs.tick) {
                        let home = crate::board::Home {
                            starved: self.starved,
                            castle_left: self.plan.as_ref().map(|p| p.castle_left()).unwrap_or(if self.base.is_some() { 1.0 } else { 0.0 }),
                            slots_left: self.plan.as_ref().map(|p| p.slots_left()).unwrap_or(0.0),
                        };
                        b.rebuild(obs, self.base, &self.settings, home, obs.tick);
                    }
                    if let Some((job, at)) = b.job_for(obs.player) {
                        self.crew.adopt(job, at);
                    }
                    if self.caps().has(crate::weights::Caps::BRIDGE_JOBS) {
                        self.bridge_jobs(&mut b, obs, me, map);
                    }
                }
            }
        }
        self.on_bridge = match (map.filter(|_| self.caps().has(crate::weights::Caps::COLUMN)), obs.me.as_ref()) {
            (Some(m), Some(me)) => Self::bridge_near(m, me, 0.0),
            _ => false,
        };
        // Road posts (roadmap 12h): the capped castle full, the pillboxes
        // carried in from now on line the roads to the side's other bases.
        if let (Some(m), Some(base), true) = (map, self.base, self.caps().has(crate::weights::Caps::CASTLE_CAP)) {
            use crate::weights::W;
            let w = &self.settings.w;
            let max = w[W::PostsMax].round().max(0.0) as usize;
            let due = obs.tick >= self.posts_at + 10 * TICK_HZ as Tick;
            if let Some(plan) = self.plan.as_mut().filter(|p| due && p.cap.is_some() && p.castle_stands() && p.turret_slot().is_none() && p.posts < max) {
                self.posts_at = obs.tick;
                let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
                let to: Vec<V2> = obs.depots.iter().filter(|d| ours(d.owner) && d.pos.dist(base) > 12.0).map(|d| d.pos).collect();
                let have: Vec<V2> = obs.pillboxes.iter().filter(|p| ours(p.owner)).map(|p| p.pos).collect();
                let posts = crate::cycle::Cycle::road_posts(m, base, &to, &have, w[W::PostSpacing], w[W::JunctionClear], max - plan.posts);
                plan.add_posts(&posts);
            }
        }
        if let Some(m) = map.filter(|_| self.caps().has(crate::weights::Caps::ISLAND_SITE)) {
            self.facts.refresh(m, obs.tick);
        }
        // A naval start (backlog 20, LR 62): afloat with no depot of the
        // side, a beach is worth what its island is as a base.
        self.inner.beach_worth = self.beach_worth(obs);
        let doctrine = self.inner.think_map(obs, map);
        // A new target the team is on, or called: say it, so the rest
        // of the side's scores lean the same way (a focus call).
        // Only a fight in reach: a call on a tank sixty tiles off is noise.
        let near = |t: &crate::tactics::Tactician| t.target.and_then(|id| obs.tanks.iter().find(|x| x.id == id)).zip(obs.me.as_ref()).map(|(x, me)| x.pos.dist(me.pos) <= FOCUS_REACH).unwrap_or(false);
        if let Some(t) = self.inner.tactics.as_ref().filter(|t| t.switched && near(t) && t.tags & crate::tactics::Tags::FOCUS != 0) {
            if self.pings && obs.tick >= self.focus_said + FOCUS_EVERY {
                self.focus_said = obs.tick;
                self.say.push(format!("focus {}", t.reason));
            }
        }
        // A fleet wingman ashore again: its crossing was the leader's,
        // and is over.
        if self.slot.is_some() && self.inner.hop.sent() && obs.events.iter().any(|e| matches!(e, Event::Landed { owner, .. } if *owner == obs.player)) {
            self.inner.hop.reset();
            self.last_move = None;
        }
        // Back in a new tank (respawn): the old one's errands are over.
        if obs.events.iter().any(|e| matches!(e, Event::Respawned { owner, .. } if *owner == obs.player)) {
            self.resupplying = false;
            self.rally = None;
            self.helping = None;
            self.crew.reset();
            self.last_move = None;
            self.target = None;
            self.scoot = None;
            self.task = None;
            self.fetching = None;
        }
        if self.goal == Goal::Manual {
            return cmds;
        }
        self.cycle.watch(self.resupplying, obs.tick);
        self.refused.retain(|(_, t)| obs.tick < t + REFUSED_FOR);
        for e in &obs.events {
            if let Event::Rejected { player, cmd: Command::FireDirect { .. }, why } = e {
                if *player == obs.player && why.contains("line") {
                    self.hold_fire_until = obs.tick + 2 * TICK_HZ as Tick;
                }
            }
            // A drive refused as over the water: a rally there is
            // dropped (a boat sinks at one hit, no way into a fight);
            // anything else of the goal's (home, the post, a depot) goes
            // by sea, the island hop's way (hop.rs `send`).
            if let Event::Rejected { player, cmd: Command::Move { to }, why } = e {
                if *player == obs.player && why == "unreachable" {
                    self.refused.push((to.tile(), obs.tick));
                    let ours = self.last_move.map(|(at, _)| at.tile() == to.tile()).unwrap_or(false);
                    if self.rally.map(|(r, _)| r.dist(*to) <= DIRECT_RANGE * 2.0).unwrap_or(false) {
                        self.rally = None;
                        self.set_doing(Act::NoBoat, format!("the fight at ({:.0},{:.0}) is over the water: not by boat", to.x, to.y));
                    } else if ours && obs.me.as_ref().map(|t| t.alive && !t.boat).unwrap_or(false) && self.inner.hop.send(*to) {
                        self.last_move = None;
                    }
                }
            }
        }
        let Some(me) = obs.me.clone() else { return cmds };
        if !me.alive {
            // Down: call the side to come and mend it, once a fall.
            let down = me.last_hit_tick > 0 && obs.tick < me.last_hit_tick + DOWNED_TICKS as Tick;
            if down && self.called_down != me.last_hit_tick && self.pings {
                self.called_down = me.last_hit_tick;
                cmds.push(Command::Ping { at: me.pos });
                self.say.push(format!("down at ({:.0},{:.0}): a squad with {} material revives me, {} s", me.pos.x, me.pos.y, REVIVE_COST, DOWNED_TICKS / TICK_HZ));
            }
            self.flush_say(obs, &mut cmds);
            self.reviving = None;
            self.set_doing(if down { Act::Down } else { Act::Dead }, if down { format!("down: {:.0} s for an ally to revive", (me.last_hit_tick + DOWNED_TICKS as Tick).saturating_sub(obs.tick) as f32 * DT) } else { "dead".into() });
            return cmds;
        }
        // Afloat, whatever the goal: the doctrine's run for the beach
        // (RuleBot::landfall_order). A boat sinks at one hit.
        // An island hop (hop.rs) is the doctrine's too, once it is under
        // way: the ladder had nothing left in reach by land.
        // An ally down: revive it before it dies, first of all, a hop
        // or not (a hop was why the first match's bots revived nobody).
        // Out of a shell's way, or a jink when hit, before anything that
        // holds the tank still: a tank parked by a wreck while its squad
        // mended it never dodged (the quad: a dozen revivers killed by
        // indirect shells in four matches). A shell's dodge holds until
        // it lands (lib.rs `Dodge::until`); the guns stay free; a revive
        // or a hop picks up again after.
        let mending = self.reviving.is_some() && matches!(me.pillboys.task, Some(PillboyTask::Repair { .. }));
        self.dodge.dropped = !self.dodges;
        let dodges = self.dodges && !self.inner.is_rookie();
        let dodge = self.dodge.order_on(obs, &me, !mending, map).filter(|_| dodges);
        if dodge.is_some() || (dodges && self.dodge.holding(obs, &me)) {
            cmds.extend(dodge);
            self.last_move = None;
            self.set_doing(Act::Dodge, "dodging".into());
            cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
            return cmds;
        }
        // Out of a hostile gun's range or line before it fires (LR 19:
        // a direct shot cannot be dodged once fired). The guns stay free.
        if !me.boat && !mending && self.caps().has(crate::weights::Caps::DUCK) {
            if let Some((to, from)) = self.duck(obs, &me, map) {
                let fresh = self.duck_to.map(|(at, t)| at.dist(to) > 1.0 || obs.tick >= t + TICK_HZ as Tick).unwrap_or(true);
                if fresh {
                    cmds.push(Command::Move { to });
                    self.duck_to = Some((to, obs.tick));
                }
                self.set_doing(Act::Duck, format!("ducking a gun at ({:.0},{:.0})", from.x, from.y));
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
            if self.duck_to.take().is_some() {
                self.last_move = None;
            }
        }
        if !me.boat && self.revive(obs, &me, &mut cmds) {
            self.flush_say(obs, &mut cmds);
            cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
            return cmds;
        }
        // A fleet's leader laying a bridge, out of material: the next
        // tile to a wingman of its that has some (a crew's job).
        if let Some(front) = self.inner.hop.bridge_front() {
            if me.pillboys.aboard && me.pillboys.material < ROAD_COST && obs.tick >= self.bridge_asked + 4 * TICK_HZ as Tick {
                // One with material first, nearest; else the nearest: a
                // crew short of it fells some first.
                let helper = self.helpers(obs, &me).into_iter().filter(|t| t.pillboys.aboard).min_by(|a, b| {
                    let key = |t: &Tank| (t.pillboys.material < ROAD_COST, t.pos.dist(front));
                    let (ka, kb) = (key(a), key(b));
                    ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
                });
                if let Some(h) = helper {
                    self.bridge_asked = obs.tick;
                    cmds.push(Command::Request { to: Some(h.owner), job: Job::Build { what: BuildKind::Road, at: front } });
                }
            }
        }
        // A fleet's leader crosses with its fleet: at the shore, about
        // to launch, it gathers them first.
        if !me.boat && self.inner.hops && self.inner.hop.launching() && self.gathering(obs, &me, &mut cmds) {
            self.flush_say(obs, &mut cmds);
            cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
            return cmds;
        }
        // A fleet afloat: in the wedge round the leader's boat while it
        // sails; landed, up the beach beside it.
        if me.boat {
            let leader = match self.goal {
                Goal::Escort(l) if self.slot.is_some() => obs.tanks.iter().find(|t| t.owner == l && t.alive).cloned(),
                _ => None,
            };
            // The leader ashore: the hop (sent for it) or the landfall
            // brings the boat in; a boat steered at an inland leader sat
            // on the water.
            if let Some(leader) = leader.filter(|l| l.boat && l.pos.dist(me.pos) <= RALLY_REACH) {
                let (to, word) = (self.slot_point(&leader, map).unwrap_or(leader.pos), "sailing in formation");
                if self.last_move.map(|(at, t)| at.dist(to) > 2.0 || obs.tick >= t + MOVE_EVERY).unwrap_or(true) {
                    cmds.push(Command::Move { to: obs.clamp(to) });
                    self.last_move = Some((to, obs.tick));
                }
                self.set_doing(Act::WingSea, format!("{}, slot {}", word, self.slot.unwrap() + 1));
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
        }
        if me.boat || (self.inner.hops && self.inner.hop.active()) {
            let act = match () {
                _ if self.inner.hop.adopted() => Act::TeamBridge,
                _ if !(self.inner.hops && self.inner.hop.active()) && self.inner.standing_off() => Act::StandOff,
                _ if !(self.inner.hops && self.inner.hop.active()) => Act::Landfall,
                _ => Act::Hop(self.inner.hop.act),
            };
            self.set_doing(act, self.inner.status());
            return doctrine;
        }
        // A teammate on a chain alone: help haul it.
        if self.help_chain(obs, &me, &mut cmds) {
            self.flush_say(obs, &mut cmds);
            cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
            return cmds;
        }
        // The team's pings: call what I see, answer what they call.
        self.hear_pings(obs, &me);
        if let Some(c) = self.call_enemy(obs, &me) {
            cmds.push(c);
        }
        self.flush_say(obs, &mut cmds);
        if self.rallying(obs, &me, &mut cmds) {
            cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
            return cmds;
        }
        // The fort (crew.rs): a fleet commander musters and hands out the
        // plan; a crew does what it is given, a wingman only while its
        // leader is at the fort.
        if self.fleet.on {
            let base = self.base_of(obs, &me);
            cmds.extend(self.fleet.think(obs, base, self.plan.as_mut()));
        }
        let free = !self.resupplying
            && (self.crew.laying_road() || match self.goal {
                Goal::Escort(l) => match (obs.allied_tanks().find(|t| t.owner == l && t.alive), self.crew.muster_at()) {
                    (Some(t), Some(m)) => t.pos.dist(m) <= 25.0,
                    _ => false,
                },
                Goal::Auto | Goal::HuntPillboxes => true,
                _ => false,
            });
        if let Some(work) = self.crew.think(obs, &me, free) {
            let working = self.crew.working() && free;
            cmds.extend(work);
            if working {
                self.set_doing(Act::Crew, self.crew.doing.clone());
                self.last_move = None;
                self.flush_say(obs, &mut cmds);
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
        }
        // A bot whose leader is down takes command (fleet.rs): a wingman
        // or a guard left on its own goes to Auto at the same base. A
        // fleet's survivors otherwise stood guard for half an hour.
        if self.social || self.base_owner.is_some() {
            let leader = match self.goal {
                Goal::Escort(p) => Some(p),
                Goal::DefendBase => self.base_owner,
                _ => None,
            };
            if let Some(l) = leader.filter(|l| *l != obs.player) {
                // With respawns the leader is back in five seconds: a
                // guard that took command at every death (the quad's
                // guards all had by the third minute) left its post for
                // good. Down past a respawn and more, it is for good.
                let down = obs.players.iter().any(|p| p.id == l && !p.alive);
                let since = match (down, self.leader_down) {
                    (false, _) => None,
                    (true, Some(t)) => Some(t),
                    (true, None) => Some(obs.tick),
                };
                self.leader_down = since;
                let wait = if obs.respawns > 0 { (RESPAWN_TICKS + 5 * TICK_HZ) as Tick } else { 0 };
                if since.map(|t| obs.tick >= t + wait).unwrap_or(false) {
                    self.base_owner.get_or_insert(l);
                    self.set_goal(Goal::Auto);
                    self.set_doing(Act::TakeCommand, "the leader is down: taking command".into());
                }
            }
        }
        // A shell in flight at the tank says where it was fired from.
        if let Some(from) = obs.shells.iter().filter(|sh| !obs.allied(sh.owner) && sh.impact.dist(me.pos) <= 4.0).max_by_key(|sh| sh.launch_tick).map(|sh| sh.from) {
            self.gun_seen_at = Some(from);
        }
        // Shot at by what nobody of ours can see: off the spot, and eyes up.
        let hit_now = me.last_hit_tick > 0 && obs.tick.saturating_sub(me.last_hit_tick) <= 1;
        let in_sight = obs.tanks.iter().any(|t| t.alive && !obs.allied(t.owner));
        if hit_now && !in_sight && self.scoot.map(|(_, until)| obs.tick >= until).unwrap_or(true) {
            self.scoot_left = !self.scoot_left;
            let side = V2::from_heading(me.heading + if self.scoot_left { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 });
            self.scoot = Some((me.pos + side.scale(SCOOT), obs.tick + SCOOT_FOR));
            self.last_move = None;
            // A shell says where it was fired from: eyes there, and the
            // gun back at it while it is in reach.
            let origin = self.gun_seen_at;
            let over = origin.unwrap_or(me.pos);
            let task = DroneTask::Orbit { center: over, radius: if origin.is_some() { 4.0 } else { RECON_VISION * 0.6 } };
            match obs.drones.iter().find(|d| d.owner == obs.player && d.kind == DroneKind::Recon) {
                Some(d) => cmds.push(Command::Retask { drone: d.id, task }),
                None if me.bay.recon > 0 => cmds.push(Command::Launch { kind: DroneKind::Recon, task }),
                None => {}
            }
        }
        if let (Some(at), true) = (self.gun_seen_at, self.scoot.map(|(_, until)| obs.tick < until).unwrap_or(false)) {
            let d = me.pos.dist(at);
            if me.reload == 0 && me.shells > LOW_SHELLS && (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&d) {
                cmds.push(Command::FireIndirect { at });
            }
        }
        if let Some((to, until)) = self.scoot {
            if obs.tick < until && !in_sight {
                // A side that is water is refused: the other side, at once.
                if obs.events.iter().any(|e| matches!(e, Event::Rejected { player, cmd: Command::Move { .. }, .. } if *player == obs.player)) {
                    let back = me.pos + (me.pos - to);
                    self.scoot = Some((back, until));
                    self.last_move = None;
                }
                let to = self.scoot.unwrap().0;
                self.go(obs, &me, to, &mut cmds);
                self.set_doing(Act::Scoot, "under fire from out of sight: scooting, recon up".into());
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
            self.scoot = None;
            self.last_move = None;
        }
        if self.needs_home(&me) {
            if let Some(home) = self.home(obs, &me) {
                self.set_doing(Act::Resupply, format!("home to resupply, {:.0} tiles", me.pos.dist(home)));
                self.go(obs, &me, home, &mut cmds);
                cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                return cmds;
            }
            self.resupplying = false;
        }
        // The team board's word (roadmap 12e, LR 9): fall back for a
        // fresh teammate, or relieve a hurt one at its point.
        if self.caps().has(crate::weights::Caps::TEAM_ROTATE) {
            let order = self.board.as_ref().and_then(|b| b.lock().ok().and_then(|b| b.order_for(obs.player, obs.tick)));
            match order {
                Some(crate::board::BoardOrder::FallBack) => {
                    if let Some(b) = self.board.as_ref() {
                        if let Ok(mut b) = b.lock() {
                            b.done(obs.player);
                        }
                    }
                    self.resupplying = true;
                    if let Some(home) = self.home(obs, &me) {
                        self.set_doing(Act::Relieved, "relieved: home to fill up".into());
                        self.go(obs, &me, home, &mut cmds);
                        cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                        return cmds;
                    }
                }
                Some(crate::board::BoardOrder::Relieve(at)) => {
                    self.defend(obs, &me, at, None, &mut cmds);
                    self.set_doing(Act::Relieving, format!("relieving a teammate at ({:.0},{:.0})", at.x, at.y));
                    let gun_free = !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }));
                    cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, gun_free)));
                    return cmds;
                }
                None => {}
            }
        }
        // The base cycle (roadmap 12f1): just filled up, the base gets an
        // item, an improvement and road out before the tank goes back out.
        if self.caps().has(crate::weights::Caps::CYCLE) && matches!(self.goal, Goal::Auto | Goal::Fortify(_) | Goal::DefendBase) && self.cycle.active() {
            let quiet = !obs.hostile_tanks().any(|t| t.alive && t.pos.dist(me.pos) <= crate::hop::FIGHT_NEAR);
            match (quiet, self.base) {
                (true, Some(base)) => {
                    if let Some((c, word)) = self.cycle.think(obs, &me, base, self.plan.as_mut(), &self.settings, map) {
                        cmds.extend(c);
                        self.set_doing(Act::Cycle(self.cycle.act), word);
                        let post = self.post(base);
                        if me.pos.dist(post) > 3.0 {
                            self.go(obs, &me, post, &mut cmds);
                        }
                        cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, true)));
                        return cmds;
                    }
                }
                _ => self.cycle.stage = crate::cycle::Stage::Idle,
            }
        }
        let gun_free = match self.goal {
            Goal::Manual => true,
            Goal::HuntPillboxes => {
                self.gun_reserved = false;
                self.hunt(obs, &me, None, &mut cmds);
                // The gun is the hunter's while a target is live: a shot
                // at anything else costs the four seconds of reload.
                !self.gun_reserved
            }
            Goal::DefendPillbox(id) => {
                let p = id
                    .and_then(|id| obs.pillboxes.iter().find(|p| p.id == id))
                    .or_else(|| obs.pillboxes.iter().filter(|p| p.owner == Some(obs.player)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))))
                    .cloned();
                match p {
                    Some(p) => {
                        self.goal = Goal::DefendPillbox(Some(p.id));
                        self.defend(obs, &me, p.pos, Some(p.id), &mut cmds);
                    }
                    None => self.set_doing(Act::Defend, "no pillbox of ours to defend".into()),
                }
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::DefendDepot(id) => {
                let d = id
                    .and_then(|id| obs.depots.iter().find(|d| d.id == id))
                    .or_else(|| obs.depots.iter().filter(|d| Self::mine_or_allied(obs, d.owner)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))))
                    .cloned();
                match d {
                    Some(d) => {
                        self.goal = Goal::DefendDepot(Some(d.id));
                        // The depot's own pillbox, if one stands by it.
                        let guard = obs.pillboxes.iter().filter(|p| p.owner == Some(obs.player) && p.pos.dist(d.pos) <= PILLBOX_RANGE).min_by(|a, b| a.pos.dist(d.pos).total_cmp(&b.pos.dist(d.pos))).map(|p| p.id);
                        self.defend(obs, &me, d.pos, guard, &mut cmds);
                    }
                    None => self.set_doing(Act::Defend, "no depot of ours to defend".into()),
                }
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::Escort(who) => match obs.tanks.iter().find(|t| t.owner == who && t.alive).cloned() {
                Some(leader) => self.fly_wing(obs, &me, &leader, map, &mut cmds),
                None => {
                    self.set_doing(Act::Escort, "the leader is not in sight".into());
                    true
                }
            },
            Goal::Hold(at) => {
                self.defend(obs, &me, at, None, &mut cmds);
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::Fortify(at) => {
                self.fortify(obs, &me, at, &mut cmds);
                !cmds.iter().any(|c| matches!(c, Command::FireDirect { .. }))
            }
            Goal::DefendBase => self.defend_base(obs, &me, &mut cmds),
            Goal::Auto => self.auto(obs, &me, &mut cmds),
        };
        cmds.extend(doctrine.into_iter().filter(|c| Self::keep_inner(c, gun_free)));
        if self.social {
            self.alliances(obs, &mut cmds);
        }
        cmds
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fortify::TOWER_R;

    /// LR 67: under full skill the Commander's shells land off its aim
    /// and its drones wait; in full, nothing it says is touched; under
    /// half, no kite.
    #[test]
    fn skill_spoils_the_aim_and_holds_the_drones() {
        let w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let obs = w.observe(0);
        let at = w.map.center();
        let said = || vec![Command::FireIndirect { at }, Command::Launch { kind: DroneKind::Recon, task: DroneTask::Return }];
        let mut c = Commander::new(17, "commander");
        let mut s = crate::weights::Settings::default();
        s.w[crate::weights::W::Skill] = 0.0;
        c.set_settings(s.clone());
        assert!(c.inner.is_rookie() && !c.inner.tactics.as_ref().unwrap().w.kite);
        let mut cmds = said();
        c.blunt(&obs, &mut cmds);
        assert_eq!(cmds.len(), 1, "a rookie's drone went up at once: {:?}", cmds);
        let Command::FireIndirect { at: off } = cmds[0] else { panic!("{:?}", cmds) };
        assert!(off != at && off.dist(at) <= crate::ROOKIE_SCATTER);
        s.w[crate::weights::W::Skill] = 1.0;
        c.set_settings(s);
        let mut cmds = said();
        c.blunt(&obs, &mut cmds);
        assert_eq!(cmds, said());
    }

    /// Paul's castle in play (roadmap 12h): alone, the capped castle goes
    /// up with two pillboxes inside, the first outlying pillbox is carried
    /// to the slot on its road, and the rest to posts on the road toward
    /// the side's second depot, none at the castle past its cap.
    #[test]
    fn a_capped_castle_fills_then_lines_the_road() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let me = w.tanks[0].pos;
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let second = w.depots.iter().filter(|d| d.id != depot.id && d.pos.dist(depot.pos) > 30.0 && !w.map.route(depot.pos, d.pos).is_empty()).min_by(|a, b| a.pos.dist(depot.pos).total_cmp(&b.pos.dist(depot.pos))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == second.id).unwrap().owner = Some(0);
        for (n, r) in [16.0, 22.0, 28.0].into_iter().enumerate() {
            let at = free_near(&w, depot.pos, r);
            w.pillboxes.push(Pillbox { id: 70_040 + n as EntityId, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        }
        let far = free_near(&w, depot.pos, 60.0);
        w.tanks.iter_mut().find(|t| t.owner == 1).unwrap().pos = far;
        let mut c = Commander::new(17, "commander");
        c.set_goal(Goal::Auto);
        assert!(c.settings.caps.has(crate::weights::Caps::CASTLE_CAP), "the capped castle is the default");
        // The cap of LR 61, two inside, one on the road, four posts, not
        // turtle's (LR 77): the test is of the cap.
        use crate::weights::W;
        c.settings.w[W::CastleInside] = 2.0;
        c.settings.w[W::CastleRoad] = 1.0;
        c.settings.w[W::PostsMax] = 4.0;
        let mut lifted = 0;
        for _ in 0..(1200 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think_on(&obs, Some(&w.map)).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            lifted += w.events.iter().filter(|e| matches!(e, Event::PillboxLifted { by: 0, .. })).count();
            if c.plan.as_ref().map(|p| p.posts > 0 && p.turrets().0 >= 2).unwrap_or(false) {
                break;
            }
        }
        let plan = c.plan.clone().expect("no plan");
        let base = plan.centre;
        let inside = w.pillboxes.iter().filter(|p| p.owner == Some(0) && p.pos.dist(base) < crate::fortify::WALL_R).count();
        eprintln!("a_capped_castle_fills_then_lines_the_road: {} at {} s; lifted {}; {} inside; posts {}; doing {}", plan.progress(), w.tick / TICK_HZ as Tick, lifted, inside, plan.posts, c.doing);
        assert!(plan.castle_stands(), "{}", plan.progress());
        assert_eq!(inside, 2, "the castle stops at two inside");
        assert!(lifted >= 2, "lifted {}", lifted);
        assert!(plan.posts > 0, "no road post toward the second depot");
        assert!(plan.turrets().0 >= 2, "the road slot and a post filled: {}", plan.progress());
        let towards = (second.pos - base).norm();
        let on_line = w.pillboxes.iter().filter(|p| p.owner == Some(0) && p.pos.dist(base) > 12.0).any(|p| {
            let rel = p.pos - base;
            let along = rel.x * towards.x + rel.y * towards.y;
            (rel - towards.scale(along)).len() < 2.5 && along > 0.0
        });
        assert!(on_line, "a pillbox stands by the road to the second base");
    }

    /// The escort (roadmap 12f): with `escort` on, a leader whose fetch
    /// trip runs past `fetch_gather` waits at the base for its wingman
    /// before it goes, and the pillbox still comes home.
    #[test]
    fn a_far_fetch_gathers_the_fleet() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(3) }.with_teams(2);
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let me = w.tanks[0].pos;
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        for (n, r) in [26.0, 30.0].into_iter().enumerate() {
            let at = free_near(&w, depot.pos, r);
            w.pillboxes.push(Pillbox { id: 70_050 + n as EntityId, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        }
        // The enemy far off; the wingman (seat 2, seat 0's team) far too.
        let far = free_near(&w, depot.pos, 60.0);
        w.tanks.iter_mut().find(|t| t.owner == 1).unwrap().pos = far;
        let wing_at = free_near(&w, depot.pos, 45.0);
        w.tanks.iter_mut().find(|t| t.owner == 2).unwrap().pos = wing_at;
        let mut s = crate::weights::Settings::default();
        s.caps = s.caps.with(crate::weights::Caps::ESCORT);
        let mut c = Commander::new(19, "leader");
        c.set_settings(s.clone());
        c.set_goal(Goal::Auto);
        c.members = vec![2];
        let mut wing = Commander::wingman(23, "wing", 0);
        wing.slot = Some(0);
        wing.set_settings(s);
        let (mut gathered, mut lifted) = (false, 0);
        for _ in 0..(1200 * TICK_HZ) {
            let mut orders: Vec<Order> = c.think_on(&w.observe(0), Some(&w.map)).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            orders.extend(wing.think_on(&w.observe(2), Some(&w.map)).into_iter().map(|cmd| Order { player: 2, cmd }));
            if c.rung == Some(Rung::Turrets) && c.act == Act::FetchGather {
                gathered = true;
            }
            w.step(&orders);
            lifted += w.events.iter().filter(|e| matches!(e, Event::PillboxLifted { by: 0, .. })).count();
            if gathered && lifted >= 1 {
                break;
            }
        }
        eprintln!("a_far_fetch_gathers_the_fleet: gathered {}, lifted {} at {} s; {}", gathered, lifted, w.tick / TICK_HZ as Tick, c.doing);
        assert!(gathered, "the leader never gathered for the fetch");
        assert!(lifted >= 1, "and never fetched");
    }

    /// Backlog 19 (LR 23): a NaN anywhere in what a tank sees used to
    /// panic the brain at the first `partial_cmp().unwrap()` that met
    /// it. Every comparison of floats is a `total_cmp` now: a hostile
    /// tank, a pillbox and a depot at NaN, under every goal, and the
    /// brain thinks on.
    #[test]
    fn a_nan_does_not_panic_the_brain() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let nan = V2::new(f32::NAN, f32::NAN);
        let me = w.tanks.iter().find(|t| t.owner == 0).unwrap().pos;
        let mut obs = w.observe(0);
        let mut enemy = w.tanks.iter().find(|t| t.owner == 1).unwrap().clone();
        enemy.pos = nan;
        obs.tanks.retain(|t| t.owner != 1);
        obs.tanks.push(enemy);
        if let Some(p) = obs.pillboxes.first_mut() {
            p.pos = nan;
        }
        if let Some(d) = obs.depots.first_mut() {
            d.pos = nan;
        }
        for goal in [Goal::Auto, Goal::HuntPillboxes, Goal::DefendDepot(None), Goal::DefendBase, Goal::DefendPillbox(None), Goal::Hold(me), Goal::Fortify(None)] {
            let mut c = Commander::new(37, "nan");
            c.set_goal(goal);
            for _ in 0..40 {
                c.think_on(&obs, Some(&w.map));
                obs.tick += 1;
            }
        }
        w.step(&[]);
    }

    /// The duck (roadmap 12d, LR 19): a hostile gun in range and line,
    /// ready before mine, is ducked out of range; not when my gun is
    /// ready first, not with `w_duck` at 0, not with the switch off. In
    /// a duel with a gunner that fires a led shot whenever it can, the
    /// tank holding its post is hit less ducking than standing.
    #[test]
    fn a_tank_ducks_a_gun_ready_before_its_own() {
        use crate::weights::{Caps, Settings, W};
        let setup = || {
            let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
            w.pillboxes.clear();
            for x in 40..100 {
                for y in 40..90 {
                    w.map.set(V2::new(x as f32 + 0.5, y as f32 + 0.5), Terrain::Grass);
                }
            }
            for (p, at, reload) in [(0, V2::new(64.5, 64.5), 60u16), (1, V2::new(71.5, 64.5), 0)] {
                let t = w.tanks.iter_mut().find(|t| t.owner == p).unwrap();
                (t.pos, t.reload, t.shells, t.speed) = (at, reload, 20, 0.0);
                t.route.clear();
                t.goal = None;
            }
            w
        };
        let commander = |caps: Caps, duck: f32| {
            let mut s = Settings::default();
            s.caps = caps;
            s.w[W::Duck] = duck;
            let mut c = Commander::new(31, "ducker");
            c.set_settings(s);
            c.set_goal(Goal::Hold(V2::new(64.5, 64.5)));
            c
        };
        let on = Caps::BASELINE.with(Caps::DUCK);
        let w = setup();
        let mut c = commander(on, 1.0);
        let cmds = c.think_on(&w.observe(0), Some(&w.map));
        let to = cmds.iter().find_map(|c| match c { Command::Move { to } => Some(*to), _ => None }).expect("a duck");
        assert!(to.dist(V2::new(71.5, 64.5)) > DIRECT_RANGE + 1.0, "out of its reach: {:?}", to);
        assert_eq!(c.act, Act::Duck, "{}", c.doing);
        let mut w2 = setup();
        for t in w2.tanks.iter_mut() {
            t.reload = if t.owner == 0 { 0 } else { 60 };
        }
        let mut c = commander(on, 1.0);
        c.think_on(&w2.observe(0), Some(&w2.map));
        assert_ne!(c.act, Act::Duck, "my gun is ready first: {}", c.doing);
        let mut c = commander(on, 0.0);
        c.think_on(&w.observe(0), Some(&w.map));
        assert_ne!(c.act, Act::Duck, "w_duck 0 never ducks");
        let mut c = commander(Caps::BASELINE.without(Caps::DUCK), 1.0);
        c.think_on(&w.observe(0), Some(&w.map));
        assert_ne!(c.act, Act::Duck, "off, no duck");
        // The duel: the gunner stands and fires a led shot whenever it
        // can; my tank holds its post, its gun held reloading, so the
        // gunner is always ready first and it never answers.
        let duel = |caps: Caps| {
            let mut w = setup();
            let mut c = commander(caps, 1.0);
            let (mut hits, mut shots, mut died) = (0, 0, None);
            for _ in 0..(40 * TICK_HZ) {
                w.tanks.iter_mut().find(|t| t.owner == 0).unwrap().reload = RELOAD_TICKS;
                // Its moves only: no drones or shots to end the duel.
                let mut orders: Vec<Order> = c.think_on(&w.observe(0), Some(&w.map)).into_iter().filter(|c| matches!(c, Command::Move { .. } | Command::Stop)).map(|cmd| Order { player: 0, cmd }).collect();
                let (g, me) = (w.tanks.iter().find(|t| t.owner == 1).unwrap().clone(), w.tanks.iter().find(|t| t.owner == 0).unwrap().clone());
                let aim = crate::lead(&g, &me);
                if g.reload == 0 && me.alive && g.pos.dist(aim) <= DIRECT_RANGE && w.map.line_clear(g.pos, aim) {
                    orders.push(Order { player: 1, cmd: Command::FireDirect { at: aim } });
                    shots += 1;
                }
                w.step(&orders);
                hits += w.events.iter().filter(|e| matches!(e, Event::ShellImpact { hits, kind: ShellKind::Direct, .. } if hits.contains(&me.id))).count();
                if died.is_none() && !w.tanks.iter().any(|t| t.owner == 0 && t.alive) {
                    died = Some(w.tick / TICK_HZ as Tick);
                }
            }
            (hits, shots, died)
        };
        // Kiting off in both: it already holds a fighting tank past the
        // gun's reach; the duck is for the tank that does not kite.
        let (ducking, standing) = (duel(on.without(Caps::KITE)), duel(Caps::BASELINE.without(Caps::KITE).without(Caps::DUCK)));
        eprintln!("a_tank_ducks_a_gun_ready_before_its_own: (hits taken, gunner's shots, down at s) in 40 s, ducking {:?}, standing {:?}", ducking, standing);
        assert!(ducking.0 < standing.0, "ducking {:?} against standing {:?}", ducking, standing);
    }

    /// A part-laid bridge on the team board (roadmap 12f, LR 6.2): a
    /// teammate on Auto with material, nothing to do but guard, takes it
    /// and lays the rest, and it comes off the board. Off the capability,
    /// nobody touches it.
    #[test]
    fn a_teammate_finishes_a_part_laid_bridge() {
        let run = |on: bool| -> (bool, usize, bool) {
            let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
            w.pillboxes.clear();
            let me = w.tanks[0].pos;
            // A channel four tiles wide across the map, six east of the
            // tank, its first tile bridged by a teammate who gave up.
            let x0 = me.x.floor() + 6.0;
            for y in 0..MAP_H {
                for dx in 0..4 {
                    w.map.set(V2::new(x0 + dx as f32 + 0.5, y as f32 + 0.5), Terrain::Water);
                }
            }
            let row = me.y.floor() + 0.5;
            w.map.set(V2::new(x0 + 0.5, row), Terrain::Bridge);
            let tiles: Vec<V2> = (1..4).map(|dx| V2::new(x0 + dx as f32 + 0.5, row)).collect();
            let target = V2::new(x0 + 12.5, row);
            w.tanks[0].pillboys.material = PILLBOY_MATERIAL_MAX;
            let mut s = crate::weights::Settings::default();
            s.caps = s.caps.with(crate::weights::Caps::BOARD);
            s.caps = if on { s.caps.with(crate::weights::Caps::BRIDGE_JOBS) } else { s.caps.without(crate::weights::Caps::BRIDGE_JOBS) };
            let board = crate::board::shared();
            board.lock().unwrap().post_bridge(1, &tiles, target, 0);
            let mut c = Commander::new(29, "builder");
            c.board = Some(board.clone());
            c.set_settings(s);
            c.set_goal(Goal::Auto);
            c.ladder = [false, false, false, false, false, true];
            let mut took = false;
            for _ in 0..(240 * TICK_HZ) {
                let orders: Vec<Order> = c.think_on(&w.observe(0), Some(&w.map)).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
                took |= board.lock().unwrap().bridges.iter().any(|j| j.by == 0);
                w.step(&orders);
                if tiles.iter().all(|t| w.map.at(*t) != Terrain::Water) {
                    // One more think to strike it off the board.
                    c.think_on(&w.observe(0), Some(&w.map));
                    break;
                }
            }
            let left = tiles.iter().filter(|t| w.map.at(**t) == Terrain::Water).count();
            let gone = board.lock().unwrap().bridges.is_empty();
            eprintln!("a_teammate_finishes_a_part_laid_bridge (on {}): took {}, {} left at {} s, off the board {}; {}", on, took, left, w.tick / TICK_HZ as Tick, gone, c.doing);
            (took, left, gone)
        };
        let (took, left, gone) = run(true);
        assert!(took, "nobody took the bridge");
        assert_eq!(left, 0, "the bridge was not finished");
        assert!(gone, "a finished bridge stays on the board");
        let (took, left, _) = run(false);
        assert!(!took && left == 3, "off the capability, the bridge is left as it was");
    }

    /// The posture leans the ladder (LR 24): with no posture or a
    /// weight of 0 the pick is the first rung that holds, for every one
    /// of the 4096 combinations; at 0.25 a leaned rung climbs one rung,
    /// at 0.5 (and the default 0.95, LR 77) every rung; DefendBase is
    /// never passed.
    #[test]
    fn the_posture_leans_the_ladder() {
        use crate::board::Posture as P;
        let bits = |n: u32, k: u32| -> [bool; 6] { std::array::from_fn(|i| (n >> (i as u32 + k)) & 1 == 1) };
        for n in 0..4096u32 {
            let (on, holds) = (bits(n, 0), bits(n, 6));
            let first = Rung::ALL.iter().copied().find(|r| on[r.index()] && holds[r.index()]).unwrap_or(Rung::Guard);
            assert_eq!(Rung::pick(&on, &holds, None, 0.5), first);
            for p in [P::Economy, P::Fortify, P::Connect, P::Expand, P::Defend] {
                assert_eq!(Rung::pick(&on, &holds, Some(p), 0.0), first, "weight 0 is the old ladder");
                if first == Rung::DefendBase {
                    assert_eq!(Rung::pick(&on, &holds, Some(p), 1.0), Rung::DefendBase, "the base's defence is never passed");
                }
            }
        }
        let all = [true; 6];
        let w = 0.25;
        assert_eq!(crate::weights::W::Posture.default_value(), 0.95);
        // Fortify, Hunt, Expand and Guard hold; the team would expand.
        let holds = [false, true, false, true, true, true];
        assert_eq!(Rung::pick(&all, &holds, Some(P::Expand), 0.0), Rung::Fortify);
        assert_eq!(Rung::pick(&all, &holds, Some(P::Expand), w), Rung::Fortify, "Hunt is two rungs below Fortify: one is all 0.25 climbs");
        assert_eq!(Rung::pick(&all, &holds, Some(P::Expand), 0.5), Rung::Hunt, "at 0.5 a lean passes every rung");
        // Hunt, Expand and Guard hold; the team would defend.
        let holds = [false, false, false, true, true, true];
        assert_eq!(Rung::pick(&all, &holds, Some(P::Defend), w), Rung::Hunt, "Guard is two below Hunt");
        let holds = [false, false, false, false, true, true];
        assert_eq!(Rung::pick(&all, &holds, Some(P::Defend), w), Rung::Guard, "Guard climbs one past Expand");
        // A switched-off rung is never picked, leaned or not.
        let mut on = all;
        on[Rung::Guard.index()] = false;
        assert_eq!(Rung::pick(&on, &holds, Some(P::Defend), 1.0), Rung::Expand);
    }

    /// On a bridge a wingman's slot is in file straight behind the
    /// leader (roadmap 12f3); off it, the wedge; without the capability,
    /// the wedge on the bridge too.
    #[test]
    fn a_fleet_crosses_a_bridge_in_file() {
        let w = World::new(Scenario { swarm_interval: 0, ..Scenario::bots(2) });
        let mut map = w.map.clone();
        let mut leader = w.tanks[0].clone();
        leader.pos = V2::new(64.5, 64.5);
        leader.heading = 0.0;
        leader.speed = 0.0;
        leader.route.clear();
        for dx in -12..=12 {
            map.set(V2::new(64.5 + dx as f32, 64.5), Terrain::Grass);
        }
        let mut c = Commander::wingman(1, "w", 0);
        c.slot = Some(1);
        let wedge = c.slot_point(&leader, Some(&map)).unwrap();
        let mut s = crate::weights::Settings::default();
        s.caps = s.caps.with(crate::weights::Caps::COLUMN);
        c.set_settings(s);
        assert_eq!(c.slot_point(&leader, Some(&map)).unwrap(), wedge, "on land the wedge");
        map.set(V2::new(64.5, 64.5), Terrain::Bridge);
        let file = c.slot_point(&leader, Some(&map)).unwrap();
        let facing = V2::from_heading(0.0);
        assert!(file.dist(leader.pos - facing.scale(4.0)) < 1e-4, "slot 1 is two gaps straight behind: {:?}", file);
        // The file holds while the bridge is still behind the leader.
        leader.pos = leader.pos + facing.scale(3.0);
        assert!(c.slot_point(&leader, Some(&map)).unwrap().dist(leader.pos - facing.scale(4.0)) < 1e-4);
        c.settings.caps = c.settings.caps.without(crate::weights::Caps::COLUMN);
        leader.pos = V2::new(64.5, 64.5);
        assert_eq!(c.slot_point(&leader, Some(&map)).unwrap(), wedge, "without the capability, the wedge");
    }

    /// The sliders (backlog 15): a switch off and on again mid-match
    /// gives the role's own setting back, and a weight moved keeps the
    /// turret's target.
    #[test]
    fn a_switch_comes_back_and_a_slider_keeps_the_target() {
        use crate::weights::{Caps, Settings, W};
        let mut c = Commander::new(1, "t");
        (c.pings, c.answers_pings) = (true, false);
        c.inner.tactics = Some(crate::tactics::Tactician::default());
        c.set_settings(Settings::default());
        c.inner.tactics.as_mut().unwrap().target = Some(7);
        let mut s = Settings::default();
        s.caps = s.caps.without(Caps::PINGS).without(Caps::DODGE).without(Caps::BRIDGES);
        c.set_settings(s.clone());
        assert_eq!((c.pings, c.answers_pings, c.dodges, c.inner.hop.no_bridges), (false, false, false, true));
        s.caps = Caps::BASELINE;
        s.nudge(W::Wounded, 2);
        c.set_settings(s);
        assert_eq!((c.pings, c.answers_pings, c.dodges, c.inner.hop.no_bridges), (true, false, true, false), "back as the role had them");
        let t = c.inner.tactics.as_ref().unwrap();
        assert_eq!((t.target, t.w.w_wounded), (Some(7), 1.4));
    }

    /// The settings with the castle of 11b and 11h: three towers and two
    /// bastions, no cap. Archived 18 October (documentation/archive/
    /// BRAIN-METHODS.md): `castle_cap` replaced it as the default, and
    /// the two proofs that use this keep the old castle playable.
    fn bastions() -> crate::weights::Settings {
        let mut s = crate::weights::Settings::default();
        s.caps = s.caps.without(crate::weights::Caps::CASTLE_CAP);
        s
    }

    /// Two seats, no swarm, no time limit, no pillboxes but one placed
    /// `away` tiles from seat 0 toward the middle, owned by `owner`.
    fn one_pillbox(away: f32, owner: Option<PlayerId>) -> (World, EntityId) {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = w.tanks[0].pos;
        let toward = (w.map.center() - home).norm();
        let mut at = Map::tile_centre(home + toward.scale(away));
        // On open ground the tank can reach.
        while !w.map.at(at).passable() || w.map.route(home, at).is_empty() {
            at = Map::tile_centre(at + toward);
        }
        let id = 70_001;
        w.pillboxes.push(Pillbox { id, pos: at, owner, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        (w, id)
    }

    /// Run seat 0 under `goal` for `seconds`; the other seat sits still.
    /// Returns the tick the pillbox became ours, and the damage it did.
    fn run(w: &mut World, pillbox: EntityId, goal: Goal, seconds: u32) -> (Option<Tick>, f32) {
        let mut c = Commander::new(5, "commander");
        c.set_goal(goal);
        let (mut ours, mut hurt) = (None, 0.0);
        for _ in 0..seconds * TICK_HZ {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            if std::env::var("COMMANDER_TRACE").is_ok() && (w.tick % (5 * TICK_HZ as Tick) == 0 || orders.iter().any(|o| matches!(o.cmd, Command::FireDirect { .. }))) {
                eprintln!("[{:>5}] at {:?} armor {} shells {} reload {} doing '{}' orders {:?}", w.tick, w.tanks[0].pos, w.tanks[0].armor, w.tanks[0].shells, w.tanks[0].reload, c.doing, orders.iter().map(|o| &o.cmd).collect::<Vec<_>>());
            }
            w.step(&orders);
            for e in &w.events {
                if let Event::Rejected { player: 0, why, .. } = e {
                    if std::env::var("COMMANDER_TRACE").is_ok() {
                        eprintln!("        rejected: {}", why);
                    }
                }
                if let Event::PillboxFired { pillbox: p, damage, .. } = e {
                    if *p == pillbox {
                        hurt += damage;
                    }
                }
            }
            if ours.is_none() && w.pillboxes.iter().any(|p| p.id == pillbox && p.owner == Some(0)) {
                ours = Some(w.tick);
            }
        }
        (ours, hurt)
    }

    /// 11a's proof: a live pillbox of the enemy's, 25 tiles off, is shot
    /// dead from outside its ring and taken by the squad inside ninety
    /// seconds, and it never lands a shell on the tank.
    #[test]
    fn a_commander_takes_a_pillbox() {
        let (mut w, id) = one_pillbox(25.0, Some(1));
        let (ours, hurt) = run(&mut w, id, Goal::HuntPillboxes, 90);
        let at = ours.unwrap_or_else(|| panic!("not taken in 90 s: {:?}", w.pillboxes.iter().find(|p| p.id == id)));
        eprintln!("a_commander_takes_a_pillbox: ours at {:.1} s, {} damage taken from it", at as f32 * DT, hurt);
        assert_eq!(hurt, 0.0, "the pillbox hit the tank");
    }

    /// A wall on the near side of the pillbox: the first firing point
    /// has no line, so the hunter walks round to one that has.
    #[test]
    fn a_commander_walks_round_a_wall_to_a_line() {
        let (mut w, id) = one_pillbox(25.0, Some(1));
        let p = w.pillboxes[0].pos;
        let toward_me = (w.tanks[0].pos - p).heading();
        for k in -6..=6 {
            let b = toward_me + k as f32 * 0.15;
            let at = p + V2::from_heading(b).scale(3.0);
            if w.map.at(at).passable() {
                w.map.set(at, Terrain::Wall);
            }
        }
        assert!(!w.map.line_clear(p + V2::from_heading(toward_me).scale(HUNT_STANDOFF), p), "the wall does not block the first point");
        let (ours, hurt) = run(&mut w, id, Goal::HuntPillboxes, 150);
        let at = ours.unwrap_or_else(|| panic!("not taken in 150 s: {:?}", w.pillboxes.iter().find(|x| x.id == id)));
        eprintln!("a_commander_walks_round_a_wall_to_a_line: ours at {:.1} s, {} damage taken", at as f32 * DT, hurt);
        assert_eq!(hurt, 0.0, "the pillbox hit the tank");
    }

    /// Three seats: 0 the leader, 1 its wingman on the same side, 2 the
    /// enemy; the wingman starts beside the leader, nobody else moves.
    fn a_pair() -> World {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(3) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let team = w.players[0].team;
        w.players[1].team = team;
        let lp = w.tanks[0].pos;
        let toward = (w.map.center() - lp).norm();
        let mut at = Map::tile_centre(lp - toward.scale(3.0));
        while !w.map.at(at).passable() {
            at = Map::tile_centre(at + toward);
        }
        w.tanks[1].pos = at;
        w.tanks[1].goal = None;
        w.tanks[1].route.clear();
        w
    }

    /// Step `w` for `ticks` with seat 1 thinking, `each` after a step.
    fn fly(w: &mut World, c: &mut Commander, ticks: u32, mut each: impl FnMut(&World, u32)) {
        for i in 0..ticks {
            let obs = w.observe(1);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 1, cmd }).collect();
            w.step(&orders);
            each(w, i);
        }
    }

    /// The wingman's first rule: a hostile tank that comes up to the
    /// leader is hit, and the wingman never leaves the leash to do it.
    #[test]
    fn a_wingman_covers_its_leader() {
        let mut w = a_pair();
        let lp = w.tanks[0].pos;
        let toward = (w.map.center() - lp).norm();
        let mut at = Map::tile_centre(lp + toward.scale(8.0));
        while !w.map.at(at).passable() {
            at = Map::tile_centre(at + toward);
        }
        let mut c = Commander::wingman(3, "wing", 0);
        c.social = false;
        let (mut hits, mut furthest) = (0, 0.0f32);
        fly(&mut w, &mut c, 60 * TICK_HZ, |w, _| {
            for e in &w.events {
                if let Event::TankDamaged { tank, by: Some(1), .. } = e {
                    if w.tanks.iter().any(|t| t.id == *tank && t.owner == 2) {
                        hits += 1;
                    }
                }
            }
            furthest = furthest.max(w.tanks[1].pos.dist(w.tanks[0].pos));
        });
        // Nothing hostile came: it stayed by the leader, rotating out
        // toward the objective no further than its push point.
        assert_eq!(hits, 0);
        assert!(furthest <= crate::rotation::WING_REACH + 1.0, "wandered {:.1} tiles off with nothing to do", furthest);
        // Now the enemy drives up to the leader.
        let t = w.tanks.iter_mut().find(|t| t.owner == 2).unwrap();
        t.pos = at;
        t.goal = None;
        t.route.clear();
        furthest = 0.0;
        fly(&mut w, &mut c, 60 * TICK_HZ, |w, _| {
            for e in &w.events {
                if let Event::TankDamaged { tank, by: Some(1), .. } = e {
                    if w.tanks.iter().any(|t| t.id == *tank && t.owner == 2) {
                        hits += 1;
                    }
                }
            }
            furthest = furthest.max(w.tanks[1].pos.dist(w.tanks[0].pos));
        });
        eprintln!("a_wingman_covers_its_leader: {} hits on the intruder, furthest {:.1} tiles from the leader", hits, furthest);
        assert!(hits >= 2, "the intruder was hit {} times", hits);
        assert!(furthest <= WING_LEASH, "left the leash: {:.1} tiles", furthest);
    }

    /// The pillbox by the leader is the wingman's to take; one far from
    /// the leader is not.
    #[test]
    fn a_wingman_takes_the_pillbox_by_its_leader() {
        let mut w = a_pair();
        let lp = w.tanks[0].pos;
        let toward = (w.map.center() - lp).norm();
        let side = V2::new(-toward.y, toward.x);
        let place = |w: &World, from: V2, dir: V2, d: f32| {
            let mut at = Map::tile_centre(from + dir.scale(d));
            while !w.map.at(at).passable() || w.map.route(from, at).is_empty() {
                at = Map::tile_centre(at + dir);
            }
            at
        };
        let near = place(&w, lp, toward, 13.0);
        let far = place(&w, lp, side, 40.0);
        w.pillboxes.push(Pillbox { id: 70_010, pos: near, owner: Some(2), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        w.pillboxes.push(Pillbox { id: 70_011, pos: far, owner: Some(2), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let mut c = Commander::wingman(4, "wing", 0);
        c.social = false;
        let mut hurt = 0.0;
        fly(&mut w, &mut c, 120 * TICK_HZ, |w, _| {
            for e in &w.events {
                if let Event::PillboxFired { tank, damage, .. } = e {
                    if *tank == w.tanks[1].id {
                        hurt += damage;
                    }
                }
            }
        });
        let owner = |id| w.pillboxes.iter().find(|p| p.id == id).unwrap().owner;
        assert!(owner(70_010) == Some(1) || owner(70_010) == Some(0), "the pillbox by the leader is still {:?}", owner(70_010));
        assert_eq!(owner(70_011), Some(2), "it went 40 tiles off the leader for the far one");
        assert_eq!(hurt, 0.0, "a pillbox hit the wingman");
    }

    /// The defection gate: with one pillbox not the side's, never; with
    /// all of them and every depot, after `TOTAL_HOLD` and not before.
    #[test]
    fn a_wingman_leaves_only_at_one_hundred_percent() {
        let mut w = a_pair();
        let lp = w.tanks[0].pos;
        w.pillboxes.push(Pillbox { id: 70_020, pos: lp + V2::new(2.0, 0.0), owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        w.pillboxes.push(Pillbox { id: 70_021, pos: lp + V2::new(-2.0, 0.0), owner: None, health: 0.0, reload: 0, capture: 0, capturer: None, shells: 0 });
        for d in w.depots.iter_mut() {
            d.owner = Some(0);
        }
        let mut c = Commander::wingman(5, "wing", 0);
        let leaves = |c: &mut Commander, w: &World| {
            let mut cmds = Vec::new();
            c.alliances(&w.observe(1), &mut cmds);
            cmds.iter().any(|x| matches!(x, Command::Alliance(AllianceCmd::Leave)))
        };
        for _ in 0..(TOTAL_HOLD * 3) {
            assert!(!leaves(&mut c, &w), "left with a pillbox not the side's");
            w.tick += 1;
        }
        w.pillboxes.iter_mut().find(|p| p.id == 70_021).unwrap().owner = Some(1);
        let start = w.tick;
        let mut left = None;
        for _ in 0..(TOTAL_HOLD * 2) {
            if leaves(&mut c, &w) {
                left = Some(w.tick - start);
                break;
            }
            w.tick += 1;
        }
        assert_eq!(left, Some(TOTAL_HOLD), "left at {:?} ticks of 100%", left);
    }

    /// 11b's proof: alone on the island for five minutes, the squad
    /// builds the castle round the home depot: the towers and the rear
    /// pillbox, most of the wall, the gate left open, and a route from
    /// the front into the depot comes in through the gate.
    #[test]
    fn a_commander_fortifies_its_depot() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let me = w.tanks[0].pos;
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me))).unwrap().id;
        w.depots.iter_mut().find(|d| d.id == depot).unwrap().owner = Some(0);
        let mut c = Commander::new(11, "commander");
        // The castle of 11b, three towers, kept by `-castle_cap` (the
        // capped castle is the default since 18 October, 12h).
        c.set_settings(bastions());
        c.set_goal(Goal::Fortify(None));
        let mut walls = 0;
        for i in 0..(300 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            if std::env::var("COMMANDER_TRACE").is_ok() && i % (10 * TICK_HZ) == 0 {
                eprintln!("[{:>5}] material {} doing '{}'", w.tick, w.tanks[0].pillboys.material, c.doing);
            }
            w.step(&orders);
            for e in &w.events {
                if let Event::Built { owner: 0, what: BuildKind::Wall, .. } = e {
                    walls += 1;
                }
                if std::env::var("COMMANDER_TRACE").is_ok() {
                    if let Event::Rejected { player: 0, why, .. } | Event::PillboysAbandoned { owner: 0, why, .. } = e {
                        eprintln!("        refused: {}", why);
                    }
                }
            }
        }
        let plan = c.plan.clone().expect("no plan");
        let towers = w.pillboxes.iter().filter(|p| p.owner == Some(0) && p.pos.dist(plan.centre) <= TOWER_R + 1.5).count();
        let gate = plan.gate();
        eprintln!("a_commander_fortifies_its_depot: {}; {} pillboxes of ours round the depot, {} walls", plan.progress(), towers, walls);
        assert!(towers >= 3, "{} pillboxes round the depot", towers);
        assert!(walls >= 8, "{} wall tiles", walls);
        assert!(plan.gate_tiles.iter().all(|g| w.map.at(*g).passable()), "the gate is shut");
        // From twelve tiles out in front, the way in is the gate.
        let mut front = Map::tile_centre(plan.centre + plan.facing.scale(12.0));
        while !w.map.at(front).passable() {
            front = Map::tile_centre(front - plan.facing);
        }
        let route = w.map.route(front, plan.centre);
        assert!(!route.is_empty(), "no way in at all");
        // The route is waypoints with straight legs between: the gate
        // is passed if a leg runs within a tile and a half of it.
        let near_leg = |a: V2, b: V2| {
            let ab = b - a;
            let t = (((gate - a).x * ab.x + (gate - a).y * ab.y) / (ab.x * ab.x + ab.y * ab.y).max(1e-6)).clamp(0.0, 1.0);
            (a + ab.scale(t)).dist(gate)
        };
        let mut legs = vec![front];
        legs.extend(route.iter().copied());
        let through = legs.windows(2).any(|l| near_leg(l[0], l[1]) <= 1.5);
        assert!(through, "the way in does not pass the gate: {:?} from {:?}, gate {:?}", route, front, gate);
        // And the wall does its work: with the gate shut, the way in from
        // the front is longer (round the wall's end), so the gate is the
        // short way and the router's choice.
        let open_len: f32 = legs.windows(2).map(|l| l[0].dist(l[1])).sum();
        for g in &plan.gate_tiles {
            w.map.set(*g, Terrain::Wall);
        }
        let shut = w.map.route(front, plan.centre);
        let mut shut_legs = vec![front];
        shut_legs.extend(shut.iter().copied());
        let shut_len: f32 = shut_legs.windows(2).map(|l| l[0].dist(l[1])).sum();
        eprintln!("  the way in: {:.1} tiles through the gate, {:.1} with it shut", open_len, shut_len);
        if std::env::var("COMMANDER_TRACE").is_ok() {
            let c = plan.centre;
            for dy in -8..=8 {
                let mut line = String::new();
                for dx in -8..=8 {
                    let p = Map::tile_centre(c + V2::new(dx as f32, dy as f32));
                    let ch = if shut.iter().any(|q| q.tile() == p.tile()) { '*' } else if w.pillboxes.iter().any(|b| b.pos.tile() == p.tile()) { 'P' } else { match w.map.at(p) { Terrain::Wall => '#', Terrain::Road => '=', Terrain::Depot => 'D', Terrain::Forest => 'f', Terrain::Water => '~', t if !t.passable() => 'x', _ => '.' } };
                    line.push(ch);
                }
                eprintln!("  {}", line);
            }
            eprintln!("  facing {:?}; shut route {:?}", plan.facing, shut);
        }
        assert!(shut.is_empty() || shut_len > open_len + 2.0, "the wall does not turn anyone: {:.1} against {:.1}", shut_len, open_len);
    }

    /// A free tile about `r` from `from`: open ground, off depots and
    /// pillboxes' spacing.
    fn free_near(w: &World, from: V2, r: f32) -> V2 {
        (0..64)
            .map(|i| Map::tile_centre(from + V2::from_heading(i as f32 * 0.7).scale(r + (i / 16) as f32)))
            .find(|p| w.map.in_bounds(*p) && w.map.at(*p).passable() && !matches!(w.map.at(*p), Terrain::Forest | Terrain::Depot) && !w.depots.iter().any(|d| d.pos.dist(*p) <= DEPOT_RADIUS + 1.0) && !w.pillboxes.iter().any(|b| b.pos.dist(*p) < PILLBOX_SPACING) && !w.map.route(from, *p).is_empty())
            .expect("no free tile")
    }

    /// Defend the base: a tank parked 12 tiles out is a threat (beyond
    /// DefendDepot's leash, inside the base's alarm); it is met under the
    /// base's guns and shot, and the hurt pillbox is mended once it is gone.
    #[test]
    fn a_commander_defends_its_base() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(w.tanks[0].pos).total_cmp(&b.pos.dist(w.tanks[0].pos))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let at = free_near(&w, depot.pos, 4.0);
        let guard = 70_020;
        w.pillboxes.push(Pillbox { id: guard, pos: at, owner: Some(0), health: 20.0, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        let mut c = Commander::new(12, "commander");
        c.set_goal(Goal::DefendBase);
        let mut warned = false;
        let mut hits = 0;
        let mut mended = None;
        for i in 0..(150 * TICK_HZ) {
            if i == 10 * TICK_HZ {
                let spot = free_near(&w, depot.pos, 12.0);
                let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
                t.pos = spot;
                t.goal = None;
                t.route.clear();
            }
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            warned |= c.act == Act::BaseThreat;
            w.step(&orders);
            for e in &w.events {
                match e {
                    Event::TankDamaged { tank, by: Some(0), .. } if w.tanks.iter().any(|t| t.id == *tank && t.owner == 1) => hits += 1,
                    Event::PillboxRepaired { pillbox, .. } if *pillbox == guard && mended.is_none() => mended = Some(w.tick),
                    _ => {}
                }
            }
        }
        let dead = !w.tanks.iter().find(|t| t.owner == 1).unwrap().alive;
        eprintln!("a_commander_defends_its_base: warned {}, {} hits, intruder dead {}, mended at {:?}", warned, hits, dead, mended.map(|t| t as f32 / TICK_HZ as f32));
        assert!(warned, "never saw the threat");
        assert!(hits >= 2, "the intruder was hit {} times", hits);
        assert!(mended.is_some(), "the base's pillbox was never mended");
        assert!(w.pillbox(guard).unwrap().health >= PILLBOX_HEALTH);
    }

    /// Shelled from twenty tiles, beyond sight: the tank scoots off the
    /// spot inside a second, puts a recon up, and so sees the shooter.
    #[test]
    fn a_commander_scoots_from_unseen_fire() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(w.tanks[0].pos).total_cmp(&b.pos.dist(w.tanks[0].pos))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let mut c = Commander::new(14, "commander");
        c.set_goal(Goal::DefendBase);
        // The scoot is what is proved: with the dodge on, the test's gun
        // never hits (a_bot_drives_out_from_under_a_shell).
        c.dodges = false;
        // Settle at the post first.
        for _ in 0..(20 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
        }
        // Out of every eye of the tank's: past its recon's 20 tiles.
        let gun = free_near(&w, w.tanks[0].pos, 32.0);
        let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
        t.pos = gun;
        t.goal = None;
        t.route.clear();
        let mut first_hit: Option<(Tick, V2)> = None;
        let mut moved = 0.0f32;
        let mut recon = false;
        let mut seen = false;
        let mut answered = 0;
        for _ in 0..(40 * TICK_HZ) {
            let obs = w.observe(0);
            seen |= obs.tanks.iter().any(|t| t.owner == 1);
            let mut orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            recon |= orders.iter().any(|o| matches!(o.cmd, Command::Launch { kind: DroneKind::Recon, .. } | Command::Retask { .. }));
            let them = w.tanks.iter().find(|t| t.owner == 1).unwrap();
            if them.reload == 0 && them.alive {
                orders.push(Order { player: 1, cmd: Command::FireIndirect { at: w.tanks[0].pos } });
            }
            answered += orders.iter().filter(|o| o.player == 0 && matches!(o.cmd, Command::FireIndirect { .. })).count();
            w.step(&orders);
            if first_hit.is_none() && w.events.iter().any(|e| matches!(e, Event::TankDamaged { tank, .. } if *tank == w.tanks[0].id)) {
                first_hit = Some((w.tick, w.tanks[0].pos));
            }
            if let Some((t0, p0)) = first_hit {
                if w.tick <= t0 + 2 * TICK_HZ as Tick {
                    moved = moved.max(w.tanks[0].pos.dist(p0));
                }
            }
        }
        eprintln!("a_commander_scoots_from_unseen_fire: hit at {:?}, moved {:.1} in 2 s, recon {}, saw the gun {}, fired back {}", first_hit.map(|(t, _)| t), moved, recon, seen, answered);
        assert!(first_hit.is_some(), "never hit: the test's gun missed");
        assert!(moved >= 1.5, "stood still under fire: {:.1}", moved);
        assert!(recon, "no recon up");
        assert!(seen, "never saw the gun");
        assert!(answered > 0, "never fired back");
    }

    /// Chain help: a person who chains a pillbox alone is answered by the
    /// nearest bot of the side, which hitches on and hauls beside them,
    /// so the pillbox moves when the person drives off.
    #[test]
    fn a_bot_answers_a_chain() {
        let names = vec![("me".to_string(), false), ("foe".to_string(), true), ("wing".to_string(), true), ("foe2".to_string(), true)];
        let sc = Scenario { swarm_interval: 0, time_limit: 0, players: names, ..Scenario::default() }.with_teams(2);
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = w.tanks[0].pos;
        let toward = (w.map.center() - home).norm();
        let mut at = Map::tile_centre(home + toward.scale(4.0));
        while !w.map.at(at).passable() || w.depots.iter().any(|d| d.pos.dist(at) < 4.0) {
            at = Map::tile_centre(at + toward);
        }
        let id = w.next_id;
        w.next_id += 1;
        w.pillboxes.push(Pillbox { id, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: 0 });
        // The person beside it; the bot a few tiles off; the foes far.
        w.tanks[0].pos = at - toward.scale(1.0);
        let near = free_near(&w, at, 8.0);
        w.tanks[2].pos = near;
        let mut c = Commander::new(9, "wing");
        c.set_goal(Goal::Auto);
        w.step(&[Order { player: 0, cmd: Command::Tow { target: Some(id) } }]);
        let mut hitched = false;
        let mut sent = false;
        for i in 0..60 * TICK_HZ {
            let obs = w.observe(2);
            let mut orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 2, cmd }).collect();
            if hitched && !sent {
                orders.push(Order { player: 0, cmd: Command::Move { to: at + toward.scale(12.0) } });
                sent = true;
            }
            w.step(&orders);
            hitched |= w.tanks[2].towing == Some(id);
            if i > 40 * TICK_HZ && !hitched {
                break;
            }
        }
        let moved = w.pillbox(id).unwrap().pos.dist(at);
        eprintln!("a_bot_answers_a_chain: hitched {}, pillbox hauled {:.1} tiles; {}", hitched, moved, c.doing);
        assert!(hitched, "the bot never came to the chain");
        assert!(moved > 1.0, "the pillbox did not move: {:.1}", moved);
    }

    /// Revive: a bot whose ally goes down twelve tiles off drives over,
    /// sends its squad and has the ally back at half armour inside the
    /// twenty seconds.
    #[test]
    fn a_bot_revives_a_downed_ally() {
        let names = vec![("me".to_string(), false), ("foe".to_string(), true), ("wing".to_string(), true), ("foe2".to_string(), true)];
        let sc = Scenario { swarm_interval: 0, time_limit: 0, players: names, ..Scenario::default() }.with_teams(2);
        let mut w = World::new(sc);
        w.pillboxes.clear();
        for _ in 0..TICK_HZ {
            w.step(&[]);
        }
        let home = w.tanks[0].pos;
        let near = free_near(&w, home, 12.0);
        w.tanks[2].pos = near;
        w.tanks[2].pillboys.pos = near;
        w.tanks[2].pillboys.material = 4;
        let mut c = Commander::new(9, "wing");
        c.set_goal(Goal::Auto);
        // Down as the world leaves a tank it destroyed.
        let tick = w.tick;
        let t = &mut w.tanks[0];
        t.alive = false;
        t.armor = 0.0;
        t.last_hit_tick = tick;
        t.pillboys.strength = 0;
        w.players[0].alive = false;
        let mut revived = None;
        for i in 0..DOWNED_TICKS {
            let obs = w.observe(2);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 2, cmd }).collect();
            w.step(&orders);
            if w.events.iter().any(|e| matches!(e, Event::Revived { owner: 0, by: 2, .. })) {
                revived = Some(i as f32 * DT);
                break;
            }
        }
        eprintln!("a_bot_revives_a_downed_ally: revived after {:?} s, {:.1} tiles off; {}", revived, near.dist(home), c.doing);
        assert!(revived.is_some(), "never revived: {}", c.doing);
        assert!(w.tanks[0].alive && w.tanks[0].armor == REVIVE_ARMOR);
    }

    /// Haul: two crews given the same job chain an ally's pillbox and
    /// drag it to the spot, then let go.
    #[test]
    fn two_crews_haul_a_pillbox() {
        let names: Vec<(String, bool)> = (0..6).map(|i| (format!("s{}", i), i != 0)).collect();
        let sc = Scenario { swarm_interval: 0, time_limit: 0, players: names, ..Scenario::default() }.with_teams(2);
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = w.tanks[0].pos;
        let toward = (w.map.center() - home).norm();
        let mut at = Map::tile_centre(home + toward.scale(4.0));
        let clear = |w: &World, p: V2| w.map.at(p).passable() && !w.depots.iter().any(|d| d.pos.dist(p) < 5.0);
        while !clear(&w, at) || !clear(&w, at + toward.scale(6.0)) || !clear(&w, at + toward.scale(9.0)) || w.map.route(at, at + toward.scale(9.0)).is_empty() {
            at = Map::tile_centre(at + toward);
        }
        let to = Map::tile_centre(at + toward.scale(6.0));
        let id = w.next_id;
        w.next_id += 1;
        w.pillboxes.push(Pillbox { id, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: 0 });
        // Seats 2 and 4 are seat 0's team: the crews, a few tiles off.
        w.tanks[2].pos = free_near(&w, at, 6.0);
        w.tanks[4].pos = free_near(&w, at - toward.scale(2.0), 6.0);
        let mut crews: Vec<(PlayerId, Commander)> = [2, 4].iter().map(|p| {
            let mut c = Commander::new(*p as u64, "crew");
            c.set_goal(Goal::Auto);
            c.crew.on = true;
            (*p as PlayerId, c)
        }).collect();
        w.step(&[Order { player: 0, cmd: Command::Request { to: None, job: Job::Muster { at } } }]);
        let mut sent = false;
        let mut chained = 0;
        for _ in 0..120 * TICK_HZ {
            let mut orders: Vec<Order> = Vec::new();
            for (p, c) in crews.iter_mut() {
                orders.extend(c.think(&w.observe(*p)).into_iter().map(|cmd| Order { player: *p, cmd }));
            }
            if !sent {
                sent = true;
                for p in [2, 4] {
                    orders.push(Order { player: 0, cmd: Command::Request { to: Some(p), job: Job::Haul { target: id, to } } });
                }
            }
            w.step(&orders);
            chained = chained.max(w.tanks.iter().filter(|t| t.towing == Some(id)).count());
            if w.pillbox(id).unwrap().pos.dist(to) <= 1.5 && !w.tanks.iter().any(|t| t.towing == Some(id)) {
                break;
            }
        }
        let now = w.pillbox(id).unwrap().pos;
        eprintln!("two_crews_haul_a_pillbox: {} on the chain at most; the pillbox {:.1} tiles from the spot; {} / {}", chained, now.dist(to), crews[0].1.doing, crews[1].1.doing);
        assert_eq!(chained, 2);
        assert!(now.dist(to) <= 1.5, "{:.1} tiles off", now.dist(to));
        assert!(!w.tanks.iter().any(|t| t.towing == Some(id)), "still on the chain");
    }

    /// Auto, alone: fortify first; once the castle stands and the base has
    /// been quiet, the two outlying pillboxes of ours are lifted and set
    /// down in the turrets; a tank at the base sends it to defend.
    #[test]
    fn a_commander_climbs_the_ladder() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let me = w.tanks[0].pos;
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(me).total_cmp(&b.pos.dist(me))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let mut out = Vec::new();
        for (n, r) in [16.0, 22.0].into_iter().enumerate() {
            let at = free_near(&w, depot.pos, r);
            let id = 70_030 + n as EntityId;
            w.pillboxes.push(Pillbox { id, pos: at, owner: Some(0), health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
            out.push(id);
        }
        // The other seat is out of the way until it is wanted.
        let far = free_near(&w, depot.pos, 40.0);
        let enemy = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
        enemy.pos = far;
        let mut c = Commander::new(13, "commander");
        // The bastions of 11h, kept by `-castle_cap`.
        c.set_settings(bastions());
        c.set_goal(Goal::Auto);
        let mut rungs: Vec<(f32, Rung)> = Vec::new();
        let mut lifted = 0;
        let mut defended_at = None;
        let mut intruder_at = None;
        for _ in 0..(900 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            if let Some(r) = c.rung {
                if rungs.last().map(|(_, l)| *l != r).unwrap_or(true) {
                    rungs.push((w.tick as f32 / TICK_HZ as f32, r));
                    if std::env::var("COMMANDER_TRACE").is_ok() {
                        eprintln!("[{:>6.1}s] {:?}: {}", w.tick as f32 / TICK_HZ as f32, r, c.doing);
                    }
                }
                if r == Rung::DefendBase && intruder_at.is_some() && defended_at.is_none() {
                    defended_at = Some(w.tick);
                }
            }
            if std::env::var("COMMANDER_TRACE").is_ok() && w.tick % (5 * TICK_HZ as Tick) == 0 && w.tick > 330 * TICK_HZ as Tick {
                eprintln!("[{:>6.1}s] {:?} doing '{}' at {:?} squad {:?} why {:?}", w.tick as f32 / TICK_HZ as f32, c.rung, c.doing, w.tanks[0].pos, (w.tanks[0].pillboys.aboard, &w.tanks[0].pillboys.task, w.tanks[0].pillboys.carrying), c.why_text(2));
            }
            for e in &w.events {
                if std::env::var("COMMANDER_TRACE").is_ok() {
                    if let Event::Rejected { player: 0, why, .. } | Event::PillboysAbandoned { owner: 0, why, .. } = e {
                        eprintln!("        refused: {}", why);
                    }
                }
            }
            w.step(&orders);
            lifted += w.events.iter().filter(|e| matches!(e, Event::PillboxLifted { by: 0, .. })).count();
            // Both turrets up: an intruder comes to the gate.
            if intruder_at.is_none() && c.plan.as_ref().map(|p| p.turrets() == (2, 2)).unwrap_or(false) {
                let gate = c.plan.as_ref().unwrap().gate();
                let spot = free_near(&w, gate, 0.0);
                let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
                t.pos = spot;
                intruder_at = Some(w.tick);
            }
            if defended_at.is_some() {
                break;
            }
        }
        let plan = c.plan.clone().expect("no plan");
        eprintln!("a_commander_climbs_the_ladder: {}; lifted {}; rungs {:?}", plan.progress(), lifted, rungs);
        let order: Vec<Rung> = rungs.iter().map(|(_, r)| *r).collect();
        assert_eq!(order.first(), Some(&Rung::Fortify), "{:?}", rungs);
        assert!(plan.castle_stands(), "{}", plan.progress());
        let first_turret = order.iter().position(|r| *r == Rung::Turrets).expect("never on turrets");
        assert!(order[..first_turret].contains(&Rung::Fortify));
        assert_eq!(lifted, 2, "lifted {}", lifted);
        assert_eq!(plan.turrets(), (2, 2), "{}", plan.progress());
        assert!(out.iter().all(|id| w.pillbox(*id).is_none()), "an outlying pillbox is still out");
        let (i, d) = (intruder_at.expect("never finished the turrets"), defended_at.expect("never defended"));
        assert!(d - i <= TICK_HZ as Tick, "defended {} ticks after the intruder came", d - i);
    }

    /// The window's solo match, headless: the map in LADDER_MAP, seat 0
    /// on Auto against three RuleBots seeded as the window seeds them.
    /// Run by hand (`--ignored`) to read what the ladder does in a match.
    #[test]
    #[ignore]
    fn the_ladder_in_a_match() {
        let text = std::fs::read_to_string(std::env::var("LADDER_MAP").expect("LADDER_MAP")).unwrap();
        let seed = 0x7EAD_C0DE_u64;
        let names = vec![("user".to_string(), false), ("bot1".into(), true), ("bot2".into(), true), ("bot3".into(), true)];
        let sc = Scenario { seed, map: MapSource::Text(text), players: names, ..Scenario::default() };
        let mut w = World::new(sc);
        let mut bots: Vec<(PlayerId, RuleBot)> = (1..4).map(|i| (i as PlayerId, RuleBot::new(seed.wrapping_mul(31) ^ (i as u64 + 1), "rulebot"))).collect();
        let mut c = Commander::new(0x60A1, "goal");
        c.set_goal(Goal::Auto);
        let secs: u64 = std::env::var("LADDER_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(600);
        for _ in 0..(secs * TICK_HZ as u64) {
            let mut orders: Vec<Order> = c.think(&w.observe(0)).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            for (p, b) in bots.iter_mut() {
                orders.extend(b.think(&w.observe(*p)).into_iter().map(|cmd| Order { player: *p, cmd }));
            }
            w.step(&orders);
            let t = w.tick as f32 / TICK_HZ as f32;
            if w.tick % (10 * TICK_HZ as Tick) == 0 {
                let me = &w.tanks[0];
                eprintln!("[{:>6.1}s] {:?} '{}' armour {:.0} shells {} squad {:?}/{:?} material {}", t, c.rung, c.doing, me.armor, me.shells, me.pillboys.aboard, me.pillboys.task, me.pillboys.material);
            }
            for e in &w.events {
                match e {
                    Event::TankDamaged { tank, by, damage, armor_left } if *tank == w.tanks[0].id => {
                        let near = w.tanks.iter().filter(|o| o.owner != 0 && o.alive).map(|o| format!("p{} {:.0}", o.owner, o.pos.dist(w.tanks[0].pos))).collect::<Vec<_>>();
                        eprintln!("[{:>6.1}s]   hit -{:.0} by {:?}, {:.0} left; tanks at {:?}; drones near {}", t, damage, by, armor_left, near, w.drones.iter().filter(|d| d.owner != 0 && d.pos.dist(w.tanks[0].pos) < 12.0).count());
                    }
                    Event::Rejected { player: 0, why, .. } | Event::PillboysAbandoned { owner: 0, why, .. } => eprintln!("[{:>6.1}s]   refused: {}", t, why),
                    Event::TankDestroyed { owner: 0, by, .. } => eprintln!("[{:>6.1}s] DESTROYED by {:?}", t, by),
                    _ => {}
                }
            }
            if !w.tanks[0].alive {
                break;
            }
        }
        if let Some(p) = &c.plan {
            eprintln!("plan: {}; items {:?}", p.progress(), p.items.iter().map(|i| (i.what, i.at, i.state)).collect::<Vec<_>>());
        }
    }

    /// A neutral pillbox is not shot at all: the squad takes it live.
    #[test]
    fn a_commander_takes_a_neutral_pillbox_without_a_shot() {
        let (mut w, id) = one_pillbox(20.0, None);
        let shells = w.tanks[0].shells;
        let (ours, hurt) = run(&mut w, id, Goal::HuntPillboxes, 60);
        assert!(ours.is_some(), "not taken in 60 s");
        assert_eq!(hurt, 0.0, "the pillbox hit the tank");
        let p = w.pillboxes.iter().find(|p| p.id == id).unwrap();
        assert!(p.alive(), "a neutral pillbox was shot instead of taken");
        assert!(w.tanks[0].shells >= shells, "shells spent on a neutral pillbox");
    }

    /// Defend depot: an intruder that drives up to the depot is engaged,
    /// and the defender never strays past the leash.
    #[test]
    fn a_commander_defends_its_depot() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let depot = w.depots.iter().min_by(|a, b| a.pos.dist(w.tanks[0].pos).total_cmp(&b.pos.dist(w.tanks[0].pos))).unwrap().clone();
        w.depots.iter_mut().find(|d| d.id == depot.id).unwrap().owner = Some(0);
        let mut c = Commander::new(9, "commander");
        c.set_goal(Goal::DefendDepot(Some(depot.id)));
        let mut furthest: f32 = 0.0;
        let mut hits_on_intruder = 0;
        for i in 0..(120 * TICK_HZ) {
            // At 20 s the intruder arrives beside the depot.
            if i == 20 * TICK_HZ {
                let t = w.tanks.iter_mut().find(|t| t.owner == 1).unwrap();
                t.pos = depot.pos + V2::new(5.0, 0.0);
                t.goal = None;
                t.route.clear();
            }
            let obs = w.observe(0);
            let orders: Vec<Order> = c.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            for e in &w.events {
                if let Event::TankDamaged { tank, by: Some(0), .. } = e {
                    if w.tanks.iter().any(|t| t.id == *tank && t.owner == 1) {
                        hits_on_intruder += 1;
                    }
                }
            }
            if i > 30 * TICK_HZ {
                furthest = furthest.max(w.tanks[0].pos.dist(depot.pos));
            }
        }
        eprintln!("a_commander_defends_its_depot: {} hits on the intruder, furthest {:.1} tiles from the depot", hits_on_intruder, furthest);
        assert!(hits_on_intruder >= 2, "the intruder was hit {} times", hits_on_intruder);
        assert!(furthest <= DEFEND_LEASH, "strayed {:.1} tiles from the depot", furthest);
    }
}
