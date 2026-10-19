//! treadcore-brain -- the controller interface, and one controller.
//!
//! The prior art called them Brains: a program that takes the tank's place at the
//! controls, given only what the tank can see. Same idea here. A Brain
//! gets an `Observation` -- the player's view of the world, filtered by
//! the host, nothing a human client does not also get -- and returns
//! `Command`s, which the world validates like anyone else's. A Brain
//! cannot cheat because it has no more access than a keyboard.
//!
//! Three implementations: `RuleBot`, which fills empty seats and is good
//! enough to kill a careless player; `Commander` (`commander.rs`), which
//! drives the player's own seat toward a goal the player chose; and
//! `JsonBrain`, which speaks the
//! observation and commands as JSON over stdin/stdout so an external
//! program -- a Python script, an LLM harness -- can play without
//! linking this crate.

use std::io::{BufRead, Write};
use treadcore_sim::*;

pub mod board;
pub mod commander;
pub mod crew;
pub mod cycle;
pub mod facts;
pub mod fleet;
pub mod fortify;
pub mod hop;
pub mod intent;
pub mod ledger;
pub mod memory;
pub mod meter;
pub mod rotation;
pub mod status;
pub mod tactics;
pub mod weights;
pub use commander::{Commander, Goal, Rung};

// Send + Sync so a seat's Brain can live in a Bevy resource; JsonBrain keeps
// its receiver behind a Mutex for the same reason.
pub trait Brain: Send + Sync {
    fn name(&self) -> &str;
    fn think(&mut self, obs: &Observation) -> Vec<Command>;
    /// `think` with the map (the host's world map, or a window's copy):
    /// public, as every window draws it, and what plans a bridge or a
    /// road needs (hop.rs). A Brain that does not plan ignores it.
    fn think_on(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        let _ = map;
        self.think(obs)
    }
    /// What it is doing, in a line, for a window that shows its fleet.
    fn status(&self) -> String {
        String::new()
    }
    /// Its weights and capabilities, for the ledger (weights.rs); None
    /// for a brain without them.
    fn settings(&self) -> Option<&weights::Settings> {
        None
    }
    /// What it is doing, as a code (status.rs): for logic and the
    /// census, where `status` is for people.
    fn act(&self) -> status::Act {
        status::Act::Idle
    }
    /// The Auto rung this think took, if it reached the ladder (the
    /// census, LR 26); None for a brain without one.
    fn rung(&self) -> Option<commander::Rung> {
        None
    }
    /// Afloat: the beach it is running for, if any (the census's read
    /// of where the gun stands, backlog 34).
    fn beach(&self) -> Option<V2> {
        None
    }
    /// Each tick the seat is dead, in place of a think: what a brain
    /// counts of its fall (the dodge's meter, LR 27).
    fn dead(&mut self, _obs: &Observation) {}
    /// New weights mid-match (the coordination screen's sliders,
    /// backlog 15); a brain without them ignores it.
    fn set_settings(&mut self, s: weights::Settings) {
        let _ = s;
    }
}

/// Run every bot seat for one tick.
/// A person took a bot's seat (roadmap step 8): that bot stops, from the
/// tick the world said so.
pub fn retire_taken(bots: &mut Vec<(PlayerId, Box<dyn Brain>)>, events: &[Event]) {
    for e in events {
        if let Event::SeatTaken { player, .. } = e {
            bots.retain(|(p, _)| p != player);
        }
    }
}

pub fn run_bots(world: &World, bots: &mut [(PlayerId, Box<dyn Brain>)]) -> Vec<Order> {
    let started = std::time::Instant::now();
    let mut out = Vec::new();
    for (p, b) in bots.iter_mut() {
        meter::seat(*p);
        let obs = world.observe(*p);
        if obs.me.as_ref().map(|t| !t.alive).unwrap_or(true) {
            b.dead(&obs);
            // A bot comes back as soon as the world lets it.
            if obs.respawn_in == Some(0) {
                out.push(Order { player: *p, cmd: Command::Respawn });
            }
            continue;
        }
        for cmd in b.think_on(&obs, Some(&world.map)) {
            if matches!(cmd, Command::Move { .. }) {
                meter::moves(1);
            }
            out.push(Order { player: *p, cmd });
        }
    }
    meter::spent(started.elapsed());
    out
}

// ------------------------------------------------------------------ rule bot

/// The first minute (PLAYABILITY.md item 1). For this long after the
/// start a bot will not fire at, or send a strike at, a hostile tank
/// that has never moved and never fired: a newcomer sitting at the
/// start reading the help. Every tank starts with full fuel and burns
/// it for every tile it drives, so a tank seen below full has moved;
/// one seen reloading has fired. Either, once seen, is remembered, and
/// that tank has joined the game. Bots drive off their starts at once,
/// so the rule spares nobody but a player who has not begun.
///
/// A first version spared any tank that had not moved since this bot
/// first saw it. Over sixty seeds of the squad proof it shifted bot
/// survival from 110:110 to 103:123, because a bot standing at a
/// firing position or a depot was spared too. This one spares only a
/// tank that has never moved.
pub const GRACE_SECONDS: u32 = 60;

/// What a bot remembers to keep the first minute: the hostile tanks it
/// has seen move or fire.
#[derive(Default)]
struct Grace {
    joined: Vec<EntityId>,
}

impl Grace {
    fn update(&mut self, obs: &Observation) {
        if obs.tick >= GRACE_SECONDS as Tick * TICK_HZ as Tick {
            return;
        }
        for t in obs.hostile_tanks() {
            if (t.fuel < TANK_FUEL || t.reload > 0) && !self.joined.contains(&t.id) {
                self.joined.push(t.id);
            }
        }
    }

    /// A fair target: the first minute is over, or the tank has been
    /// seen to move or fire.
    fn fair(&self, obs: &Observation, t: &Tank) -> bool {
        obs.tick >= GRACE_SECONDS as Tick * TICK_HZ as Tick || self.joined.contains(&t.id) || t.fuel < TANK_FUEL || t.reload > 0
    }
}

/// A doctrine, not an intelligence:
///
///   1. Keep the bubble up. Interceptors on station: four while anything
///      hostile is airborne nearby, two otherwise.
///   2. Take a firing position. The objective is the nearest depot that is
///      not mine (the centre, to begin with). Drive to `STANDOFF` tiles
///      short of it, by road: the world routes a Move around the river.
///   3. Eyes over the objective. Once stationary, one recon drone orbits
///      the objective (or the nearest known enemy), inside the link.
///      The first bot fired its recon toward the centre at launch and
///      then drove past it; a recon launched from a standing tank at its
///      firing position stays useful.
///   4. Shoot what the recon sees. A spotted tank between minimum and
///      maximum range gets a shell every reload, led by its motion.
///   5. Shoot and scoot. After firing, move a few tiles: counter-battery
///      is coming. Stay out of direct range of anything that can see you.
///   6. Strike a thin bubble: a strike drone at the nearest enemy once
///      fewer than two interceptors are visible around it.
///   7. Resupply when low; capture the nearest depot if none is mine.
///   8. Advance. Nothing seen for `PATIENCE` seconds at the firing
///      position: drive onto the objective and capture it, under the
///      recon, and the next unowned depot becomes the objective.
///   9. Pillboys (when `pillboys` is on). From a standing tank with no
///      enemy in view: send the squad to the nearest neutral or dead
///      pillbox within `PILLBOY_REACH` whose gun cannot reach the tank,
///      or to repair an owned one below half health. While the squad is
///      out the tank holds its position, so the squad has a home to
///      come back to; that wait is the cost the proof weighs.
///  10. Storm (roadmap step 6d, backlog item 14). An enemy that has
///      been in view for twice the bot's patience and is still alive is
///      sitting on a depot being repaired as fast as it is shelled; the
///      firing position is not working. The bot shells the live
///      hostile pillboxes guarding the objective, then drives onto it
///      to capture it, firing direct on the way, and stops scooting.
///      Low on anything, it goes home instead and the storm is off.
pub struct RuleBot {
    name: String,
    rng: Rng,
    last_fire_tick: Tick,
    scoot_until: Tick,
    resupplying: bool,
    last_think: Tick,
    /// Tick an enemy tank was last in view; drives the patience clock.
    last_contact: Tick,
    /// Seconds an enemy has been in view since the objective was set,
    /// accumulated across the bot's own scoots and resupplies. Twice
    /// the patience and the bot storms.
    contact_total: f32,
    /// The objective the contact clock belongs to.
    contact_objective: Option<(i32, i32)>,
    /// The depot being stormed, until it is ours or the bot runs low.
    storm: Option<V2>,
    /// Tick the tank last came to rest at a firing position.
    settled_at: Option<Tick>,
    /// This bot's patience in seconds, drawn once from its seed so four
    /// bots do not all advance on the same tick and meet in the middle.
    patience: f32,
    /// Use the squad at all. Off for the control group in the proof.
    pub pillboys: bool,
    /// A bot for a newcomer to beat (PLAYABILITY item 9): see `rookie`.
    pub rookie: bool,
    /// What the world refused and why it will stay refused: pillboxes
    /// the squad cannot reach, and tiles the tank cannot drive to. On an
    /// island map (the converted prior-art maps) half the objectives are
    /// across water, and a bot that forgot was refused four times a
    /// second for the whole match.
    unreachable_pillboxes: Vec<EntityId>,
    unreachable_tiles: Vec<(i32, i32)>,
    /// The first minute: who may be shot yet.
    grace: Grace,
    /// Who has hurt this tank: a person who has is not let onto its side.
    hurt_by: Vec<PlayerId>,
    /// When the last longshot went (step 10e).
    last_longshot: Tick,
    /// A structure of this side has been hit: the bot builds jammers
    /// from then on (step 10i). Electronic warfare is an answer, not a
    /// habit: it takes its owner's drones too.
    works_hit: bool,
    /// Structures found with no free pad beside them for a kind (10k).
    no_room: Vec<(EntityId, StructureKind)>,
    /// How the match is going for this bot, and what it does about it.
    pub form: Form,
    /// Afloat: the beach it is making for, and when it chose it.
    landfall: Option<(V2, Tick)>,
    /// `landfall_shy` (backlog 30, LR 31): at 1, each tile of the run
    /// for a beach in a hostile gun's reach costs as much as 4 tiles
    /// more to sail. 0 prices only the beach and the run's heading, as
    /// before.
    pub landfall_shy: f32,
    /// `landfall_wait` (backlog 33, LR 33): the seconds of a crossing a
    /// boat will stand off a beach a seen hostile tank covers; 0 lands
    /// anyway, as before.
    pub landfall_wait: f32,
    /// When this crossing's stand-off began (its wait is spent from
    /// then, gun or no gun), and whether the last order was one.
    standoff: Option<Tick>,
    standing: bool,
    /// Out of a shell's way (`Dodge`); not a rookie's.
    dodge: Dodge,
    /// Island hopping, when nothing is left in reach by land (hop.rs).
    pub hop: hop::Hop,
    /// Hop at all: a Commander turns it off but on its Auto goal, so a
    /// wingman or a guard keeps its post.
    pub hops: bool,
    /// Go home to resupply by its own thresholds; a Commander with one
    /// threshold for both turns it off (roadmap 12d).
    pub own_resupply: bool,
    /// Fleet tactics (tactics.rs): the target by score, not by distance
    /// alone. A Commander's; a plain RuleBot shoots the nearest.
    pub tactics: Option<tactics::Tactician>,
    /// A plain RuleBot's kiting (tactics.rs `kite`): its standoff is
    /// past direct range already; it steps in to fire direct while the
    /// target's gun cannot answer. Never a rookie's. Off: tried on 17
    /// October, `rulebot_kiting_pays` over ten seeds gave the kiting
    /// half kills 20 against 15 but deaths 26 against 18 and direct
    /// hits taken 31 against 15 (a step in walks into the next tank's
    /// gun, which a RuleBot does not weigh), and it broke
    /// `defenders_hold`.
    pub kites: bool,
}

/// Getting out of the way (Paul, 16 October: "when being shot at, drive
/// around to dodge"). Two cases, each answered once:
///
///   - An indirect shell of another side in the air that will land
///     within its splash (and a margin) of the tank, or of where the
///     tank will be when it lands, with time to move: the tank drives
///     across the shell's line, out of the splash, on the side it is
///     already on. The impact point is what every window draws at the
///     end of a shell's arc; a bot reads it the same way.
///   - Hit (a direct shot, a strike, a splash it did not see coming):
///     a jink, three tiles across its heading and two on, the other
///     side from last time, so a gunner leading it misses the next.
///
/// The bot's own route is lost to the dodge; its doctrine plans again
/// from where the dodge leaves it.
#[derive(Clone, Debug, Default)]
pub struct Dodge {
    dodged: Vec<EntityId>,
    jinked: Tick,
    side: f32,
    /// A shell's dodge holds until the shell has landed: a Commander's
    /// next tick re-ordered the tank's errand and drove it back under
    /// the shell. A jink is one order, as before: held, it ran tanks out
    /// of their own bubble (strike deaths 46 to 100 over eight quad
    /// seeds) and off the wrecks they were reviving.
    pub until: Tick,
    /// The shell dodge under way, for the meter (backlog 28, LR 27):
    /// when it began, when its shell lands, whether afloat, and whether
    /// dropped.
    watch: Option<(Tick, Tick, bool, bool)>,
    /// The brain drops the dodge's moves (the dodge off): the meter
    /// counts it as would-be (backlog 31).
    pub dropped: bool,
    /// Counted by the meter: the Commander's dodge, the one that acts.
    /// The RuleBot under a Commander dodges too, but its moves are
    /// dropped (`keep_inner`).
    pub metered: bool,
    /// The weights' numbers (horizon s, margin, clear, hold; LR 27);
    /// None is the constants, the RuleBot's.
    pub tune: Option<[f32; 4]>,
    /// When the shell dodge under way began, for `dodge_hold`.
    began: Tick,
}

impl Dodge {
    /// The meter's reading of the dodge under way: hit (or killed)
    /// between its start and its shell's landing, or clear of it. Any
    /// hit in that window counts, the dodged shell's or another's.
    pub fn watch(&mut self, me: &Tank, now: Tick) {
        let Some((start, until, afloat, dropped)) = self.watch else { return };
        if me.last_hit_tick >= start && me.last_hit_tick <= until {
            crate::meter::dodge_hit(afloat, dropped, !me.alive, me.last_hit_tick + 2 == until);
            self.watch = None;
        } else if now > until {
            self.watch = None;
        }
    }

    pub fn order(&mut self, obs: &Observation, me: &Tank) -> Option<Command> {
        self.order_with(obs, me, true)
    }

    /// `jink`: answer a hit with a jink too. A tank whose squad is
    /// mending a wreck beside it dodges a shell it sees but takes a hit
    /// where it stands: a jink pulled revivers off and the wreck died.
    pub fn order_with(&mut self, obs: &Observation, me: &Tank, jink: bool) -> Option<Command> {
        self.watch(me, obs.tick);
        self.dodged.retain(|id| obs.shells.iter().any(|s| s.id == *id));
        let hostile = |o: PlayerId| o != obs.player && !obs.allied(o);
        for s in obs.shells.iter().filter(|s| s.kind == ShellKind::Indirect && hostile(s.owner)) {
            if self.dodged.contains(&s.id) {
                continue;
            }
            let [horizon, margin, clear, _] = self.tune.unwrap_or([4.0, 0.8, 2.5, 0.0]);
            let eta = s.impact_tick.saturating_sub(obs.tick);
            if eta < 2 || eta as f32 * DT > horizon {
                continue;
            }
            let ahead = me.pos + V2::from_heading(me.heading).scale(me.speed * eta as f32 * DT);
            if s.impact.dist(me.pos).min(s.impact.dist(ahead)) > SPLASH_RADIUS + margin {
                continue;
            }
            self.dodged.push(s.id);
            if obs.tick >= self.until {
                self.began = obs.tick;
            }
            self.until = s.impact_tick + 2;
            if self.metered && self.watch.is_none() {
                self.watch = Some((obs.tick, self.until, me.boat, self.dropped));
                crate::meter::dodge(me.boat, self.dropped);
            }
            let line = (s.impact - s.from).norm();
            let across = V2::new(-line.y, line.x);
            let off = me.pos - s.impact;
            let side = if off.x * across.x + off.y * across.y >= 0.0 { 1.0 } else { -1.0 };
            return Some(Command::Move { to: obs.clamp(s.impact + across.scale(side * (SPLASH_RADIUS + clear))) });
        }
        if jink && me.last_hit_tick > self.jinked && obs.tick.saturating_sub(me.last_hit_tick) <= TICK_HZ as Tick / 2 {
            self.jinked = me.last_hit_tick;
            self.side = if self.side > 0.0 { -1.0 } else { 1.0 };
            let f = V2::from_heading(me.heading);
            return Some(Command::Move { to: obs.clamp(me.pos + V2::new(-f.y, f.x).scale(3.0 * self.side) + f.scale(2.0)) });
        }
        None
    }

    /// A dodge under way: hold it while the tank drives to its point,
    /// and, by `dodge_hold`, that share of the shell's flight once
    /// there (standing out of the splash rather than back to its
    /// errand under the shell).
    pub fn holding(&self, obs: &Observation, me: &Tank) -> bool {
        let hold = self.tune.map(|t| t[3]).unwrap_or(0.0).clamp(0.0, 1.0);
        let held = self.began as f32 + hold * self.until.saturating_sub(self.began) as f32;
        obs.tick < self.until && (me.goal.is_some() || (obs.tick as f32) < held)
    }
}

/// Where to aim a direct shot at a moving tank: where it will be when
/// the shell lands (40 tiles a second, 0.8 tiles of hit radius; a tank
/// at full speed moves a tile in the flight from 10 tiles off, so a shot
/// at where it is misses). Inside direct range, else where it is.
pub fn lead(me: &Tank, e: &Tank) -> V2 {
    let flight = me.pos.dist(e.pos) / SHELL_SPEED_DIRECT;
    let aim = e.pos + V2::from_heading(e.heading).scale(e.speed * flight);
    if me.pos.dist(aim) <= DIRECT_RANGE {
        aim
    } else {
        e.pos
    }
}

/// Seconds between a bot's looks at its own progress.
pub const FORM_EVERY: f32 = 30.0;

/// The bot watching its own progress, every `FORM_EVERY` seconds, and
/// playing to it. What it counts: score gained (kills, captures), depots
/// its side gained or lost, armour lost. Getting hurt for nothing makes
/// it careful: it waits longer before advancing, fires from further
/// off and goes home for repairs sooner. Nothing happening at all, no
/// gain, no harm and no contact, makes it bold: a bot that sat at a
/// firing position for a whole match is the one this answers. Gaining
/// keeps it pressing. `mood` runs from -1 (careful) to +1 (bold), half
/// the last reading and half the new, so one bad half minute does not
/// turn it.
#[derive(Clone, Debug, Default)]
pub struct Form {
    /// When it last looked, and what it saw then.
    at: Tick,
    score: u32,
    depots: usize,
    armor: f32,
    contact: Tick,
    /// -1 careful .. +1 bold.
    pub mood: f32,
    /// The last reading, in words, for `status` and the trace.
    pub note: String,
}

impl Form {
    /// Look again if it is time. Returns true when the mood was read.
    fn update(&mut self, obs: &Observation, me: &Tank, last_contact: Tick) -> bool {
        let score = obs.players.iter().find(|p| p.id == obs.player).map(|p| p.score).unwrap_or(0);
        let depots = obs.depots.iter().filter(|d| d.owner.map(|o| obs.allied(o)).unwrap_or(false)).count();
        if self.at == 0 {
            *self = Form { at: obs.tick.max(1), score, depots, armor: me.armor, contact: last_contact, mood: 0.0, note: "steady".into() };
            return false;
        }
        if (obs.tick.saturating_sub(self.at) as f32 * DT) < FORM_EVERY {
            return false;
        }
        let gained = score.saturating_sub(self.score) as f32 / SCORE_KILL.max(1) as f32 + depots as f32 - self.depots as f32;
        // Repairs at a depot raise armour; only the loss counts.
        let hurt = (self.armor - me.armor).max(0.0) / 25.0;
        let quiet = last_contact == self.contact && gained == 0.0 && hurt == 0.0;
        let reading = if quiet { 0.6 } else { (gained - hurt).clamp(-1.0, 1.0) };
        self.mood = (self.mood * 0.5 + reading * 0.5).clamp(-1.0, 1.0);
        self.note = if quiet {
            "stalled: pushing".into()
        } else if reading < -0.2 {
            format!("hurt {:.0}: careful", hurt * 25.0)
        } else if reading > 0.2 {
            format!("+{:.0}: pressing", gained)
        } else {
            "even".into()
        };
        *self = Form { at: obs.tick, score, depots, armor: me.armor, contact: last_contact, mood: self.mood, note: std::mem::take(&mut self.note) };
        true
    }
    /// The patience, scaled: bold waits less.
    pub fn patience(&self, base: f32) -> f32 {
        base * (1.0 - 0.4 * self.mood)
    }
    /// The firing position's distance, scaled: careful stands further off.
    pub fn standoff(&self) -> f32 {
        STANDOFF * (1.0 - 0.2 * self.mood)
    }
    /// The armour below which the bot goes home: careful goes sooner.
    pub fn home_armor(&self) -> f32 {
        45.0 - 15.0 * self.mood
    }
}

/// Seconds between a RuleBot's longshots: a cheap drone, but two in the
/// bay and a long way to the next workshop.
pub const LONGSHOT_EVERY: f32 = 30.0;

/// A rookie's first drone goes up this many seconds into the match: the
/// first minute and a half is gun against gun.
pub const ROOKIE_LATE: f32 = 90.0;
/// How far a rookie's indirect aim falls from the tank it aims at, at
/// most, before the shell's own scatter.
pub const ROOKIE_SCATTER: f32 = 4.0;

/// How far a bot will drive to build beside a structure of its own
/// (step 10k): its own zone, not across the map.
pub const WORKS_TRIP: f32 = 45.0;

/// How far the bot will send its squad: a 20-tile walk is 13 seconds
/// each way at `PILLBOY_SPEED`, plus the work.
pub const PILLBOY_REACH: f32 = 20.0;

/// How far short of the objective the bot stops to fire from: inside
/// indirect range of anything at the objective, outside direct range
/// and outside the swarm's interest in it, and within the recon link.
pub const STANDOFF: f32 = 25.0;
/// Seconds at a firing position with nothing seen before advancing; each
/// bot draws its own between half and two and a half times this.
pub const PATIENCE: f32 = 20.0;

impl RuleBot {
    /// Has this player's fire hit the bot's tank this match?
    pub fn hurt_by_player(&self, p: PlayerId) -> bool {
        self.hurt_by.contains(&p)
    }

    pub fn new(seed: u64, name: &str) -> RuleBot {
        let mut rng = Rng::new(seed);
        let patience = PATIENCE * rng.range(0.5, 2.5);
        RuleBot { name: name.into(), rng, last_fire_tick: 0, scoot_until: 0, resupplying: false, last_think: 0, last_contact: 0, contact_total: 0.0, contact_objective: None, storm: None, settled_at: None, patience, pillboys: true, rookie: false, unreachable_pillboxes: Vec::new(), unreachable_tiles: Vec::new(), grace: Grace::default(), hurt_by: Vec::new(), last_longshot: 0, works_hit: false, no_room: Vec::new(), form: Form::default(), landfall: None, landfall_shy: 0.0, landfall_wait: 0.0, standoff: None, standing: false, dodge: Dodge::default(), hop: hop::Hop::default(), hops: true, own_resupply: true, tactics: None, kites: false }
    }

    /// Afloat and standing off a covered beach (`landfall_wait`).
    pub fn standing_off(&self) -> bool {
        self.standing
    }

    /// Afloat (SPEC TNK-08): run for land. One hit sinks a boat, so the
    /// bot does not fight from one: it picks the beach that is near and
    /// far from every hostile gun it knows of, tank or pillbox, and goes,
    /// changing its mind only for a beach clearly better, or when a
    /// threat comes near the one it chose. With no beach in sight it
    /// heads for the middle of the map, away from whoever is closest.
    fn landfall_order(&mut self, obs: &Observation, me: &Tank) -> Option<Command> {
        let threats: Vec<V2> = obs
            .hostile_tanks()
            .filter(|t| t.alive)
            .map(|t| t.pos)
            .chain(obs.pillboxes.iter().filter(|p| p.alive() && !p.owner.map(|o| obs.allied(o)).unwrap_or(false)).map(|p| p.pos))
            .collect();
        let danger = |b: V2| {
            let near = threats.iter().map(|t| t.dist(b)).fold(f32::INFINITY, f32::min);
            if near < DIRECT_RANGE + 4.0 {
                80.0
            } else if near < PILLBOX_RANGE + 6.0 {
                40.0
            } else if near < INDIRECT_MAX_RANGE {
                (INDIRECT_MAX_RANGE - near) * 0.6
            } else {
                0.0
            }
        };
        // A beach the way an enemy is costs more: the run is the
        // dangerous part, not only the landing.
        let toward = |b: V2| threats.iter().filter(|t| t.dist(me.pos) < INDIRECT_MAX_RANGE).map(|t| ((b - me.pos).norm().x * (*t - me.pos).norm().x + (b - me.pos).norm().y * (*t - me.pos).norm().y).max(0.0) * 15.0).sum::<f32>();
        // The run in a gun's reach (backlog 30): the tiles of the straight
        // line to the beach within a hostile tank's direct range, or a
        // pillbox's, with a margin. A boat sinks at one hit.
        let guns: Vec<(V2, f32)> = obs
            .hostile_tanks()
            .filter(|t| t.alive)
            .map(|t| (t.pos, DIRECT_RANGE + 4.0))
            .chain(obs.pillboxes.iter().filter(|p| p.alive() && !p.owner.map(|o| obs.allied(o)).unwrap_or(false)).map(|p| (p.pos, PILLBOX_RANGE + 2.0)))
            .collect();
        let shy = self.landfall_shy.max(0.0);
        let exposed = |b: V2| {
            if shy == 0.0 || guns.is_empty() {
                return 0.0;
            }
            let steps = b.dist(me.pos).ceil().max(1.0) as usize;
            (1..=steps).filter(|i| guns.iter().any(|(g, r)| me.pos.lerp(b, *i as f32 / steps as f32).dist(*g) < *r)).count() as f32 * b.dist(me.pos) / steps as f32
        };
        let score = |b: V2| b.dist(me.pos) + danger(b) + toward(b) + 4.0 * shy * exposed(b);
        let best = obs.shore.iter().filter(|b| !self.unreachable_tiles.contains(&b.tile())).min_by(|a, b| score(**a).total_cmp(&score(**b))).copied();
        let going = me.route.last().copied().or(me.goal);
        let Some(best) = best else {
            if going.is_some() {
                return None;
            }
            let away = threats.iter().min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos))).map(|t| (me.pos - *t).norm().scale(20.0)).unwrap_or(V2::new(0.0, 0.0));
            return Some(Command::Move { to: obs.clamp(obs.centre().lerp(me.pos, 0.5) + away) });
        };
        // Standing off a covered beach (backlog 33): a seen hostile tank
        // within its direct range of the beach, with a margin, and the
        // boat waits out of that gun's reach until the crossing's
        // `landfall_wait` is spent, then lands.
        self.standing = false;
        if self.landfall_wait > 0.0 {
            let cover = obs.hostile_tanks().filter(|t| t.alive && t.pos.dist(best) < DIRECT_RANGE + 4.0).map(|t| t.pos).min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos)));
            if let Some(gun) = cover {
                let began = *self.standoff.get_or_insert(obs.tick);
                if (obs.tick.saturating_sub(began) as f32 * DT) < self.landfall_wait {
                    self.standing = true;
                    self.landfall = None;
                    let off = DIRECT_RANGE + 6.0;
                    let to = if me.pos.dist(gun) >= off - 1.0 { me.pos } else { obs.clamp(gun + (me.pos - gun).norm().scale(off)) };
                    return match going {
                        Some(g) if g.dist(to) < 2.0 => None,
                        None if to == me.pos => None,
                        _ => Some(Command::Move { to }),
                    };
                }
            }
        }
        let since = self.landfall.map(|(_, t)| obs.tick.saturating_sub(t) as f32 * DT).unwrap_or(f32::INFINITY);
        let change = match (self.landfall, going) {
            (Some((chosen, _)), Some(_)) => since >= 2.0 && score(best) + 6.0 < score(chosen),
            _ => since >= 1.0,
        };
        if !change {
            return None;
        }
        self.landfall = Some((best, obs.tick));
        Some(Command::Move { to: best })
    }

    /// The same doctrine, badly (PLAYABILITY item 9): thinks half as
    /// often, launches nothing for `ROOKIE_LATE` seconds and then keeps
    /// one interceptor up, aims where the tank is with `ROOKIE_SCATTER`
    /// tiles of error and no lead, and never storms. The squad and the
    /// standoff are kept: a rookie still plays the game, it plays it
    /// slowly.
    pub fn rookie(seed: u64, name: &str) -> RuleBot {
        RuleBot { rookie: true, ..RuleBot::new(seed, name) }
    }

    /// The same doctrine without the squad: the control group.
    pub fn without_pillboys(seed: u64, name: &str) -> RuleBot {
        RuleBot { pillboys: false, ..RuleBot::new(seed, name) }
    }

    fn nearest_enemy<'a>(&self, obs: &'a Observation, me: &Tank) -> Option<&'a Tank> {
        obs.hostile_tanks().filter(|t| self.grace.fair(obs, t)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))
    }

    /// The squad's works (steps 10i and 10j), in this order: a power
    /// plant when a structure of this bot's stands dark with no plant of
    /// the side in reach (everything else of the economy is dead without
    /// one); a jammer once a structure of the side has been hit; a
    /// workshop when none of the side covers this bot's structures. Each
    /// on a free pad within half a jammer's reach of the structure it
    /// serves; the scrap felled from the nearest forest first if the
    /// squad has not got it. None if any of that is missing, which on a
    /// map with no pads is always.
    fn works_order(&mut self, obs: &Observation, me: &Tank) -> Option<Command> {
        if obs.pads.is_empty() {
            return None;
        }
        let (kind, at, id) = self.works_need(obs, me, PILLBOY_REACH)?;
        let side = |x: &&Structure| x.alive() && x.owner.map(|o| obs.allied(o)).unwrap_or(false);
        let powered = |p: V2| obs.structures.iter().filter(side).any(|x| x.kind == StructureKind::Power && x.pos.dist(p) <= POWER_RANGE);
        let site = obs.pads.iter().filter(|p| p.dist(at) <= JAMMER_RANGE * 0.5 && (kind == StructureKind::Power || powered(**p))).min_by(|a, b| a.dist(at).total_cmp(&b.dist(at))).copied();
        let Some(site) = site else {
            // No room beside it (step 10k): remembered, so the bot does
            // not drive back to look again.
            self.no_room.push((id, kind));
            return None;
        };
        if me.pillboys.material >= kind.cost() {
            return Some(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Structure(kind), at: site } });
        }
        self.fell(obs, me)
    }

    /// What this bot's structures within `reach` want built beside them,
    /// first want first: a power plant for a dark one with no plant of
    /// the side in reach; a jammer, once the side has been hit, for one
    /// no jammer covers; a workshop for one no workshop mends. The
    /// nearest structure wanting it, less those remembered as having no
    /// room. Pads are not looked at: a far zone's are out of sight.
    fn works_need(&self, obs: &Observation, me: &Tank, reach: f32) -> Option<(StructureKind, V2, EntityId)> {
        let side = |x: &&Structure| x.alive() && x.owner.map(|o| obs.allied(o)).unwrap_or(false);
        let near = |kind: StructureKind, at: V2, r: f32| obs.structures.iter().filter(side).any(|x| x.kind == kind && x.pos.dist(at) <= r);
        let mine: Vec<&Structure> = obs.structures.iter().filter(|x| x.alive() && x.owner == Some(obs.player) && x.pos.dist(me.pos) <= reach).collect();
        let wants = [
            (StructureKind::Power, &(|x: &Structure| x.kind.needs_power() && !x.powered && !near(StructureKind::Power, x.pos, POWER_RANGE)) as &dyn Fn(&Structure) -> bool),
            (StructureKind::Jammer, &|x: &Structure| self.works_hit && !near(StructureKind::Jammer, x.pos, JAMMER_RANGE)),
            (StructureKind::Workshop, &|x: &Structure| !near(StructureKind::Workshop, x.pos, WORKSHOP_MEND_RANGE)),
        ];
        for (kind, wants_it) in wants {
            let x = mine.iter().filter(|x| wants_it(x) && !self.no_room.contains(&(x.id, kind))).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
            if let Some(x) = x {
                return Some((kind, x.pos, x.id));
            }
        }
        None
    }

    /// Scrap: the nearest forest in reach, if the squad has room for it.
    fn fell(&self, obs: &Observation, me: &Tank) -> Option<Command> {
        if me.pillboys.material + HARVEST_MATERIAL > PILLBOY_MATERIAL_MAX {
            return None;
        }
        obs.forest.iter().find(|t| t.dist(me.pos) <= PILLBOY_REACH).map(|t| Command::Deploy { task: PillboyTask::Harvest { at: *t } })
    }

    /// The nearest depot of the side (or not), of those the world has
    /// not refused a drive to as over the water.
    fn nearest_depot<'a>(&self, obs: &'a Observation, me: &Tank, friendly: bool) -> Option<&'a Depot> {
        let ours = |d: &&Depot| d.owner.map(|o| obs.allied(o)).unwrap_or(false);
        obs.depots.iter().filter(|d| (if friendly { ours(d) } else { !ours(d) }) && !self.unreachable_tiles.contains(&d.pos.tile())).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))
    }


}

impl Brain for RuleBot {
    fn name(&self) -> &str {
        &self.name
    }

    /// The beach `landfall_order` chose, while afloat.
    fn beach(&self) -> Option<V2> {
        self.landfall.map(|(b, _)| b)
    }

    fn status(&self) -> String {
        match self.landfall {
            _ if self.hop.active() => format!("island hop: {}", self.hop.doing),
            Some((b, _)) => format!("afloat: for the beach at ({:.0},{:.0})", b.x, b.y),
            None => format!("form {:+.2} {}", self.form.mood, self.form.note),
        }
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        self.think_map(obs, None)
    }

    fn think_on(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        self.think_map(obs, map)
    }
}

impl RuleBot {
    pub(crate) fn think_map(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        let mut cmds = Vec::new();
        let Some(me) = obs.me.as_ref() else { return cmds };
        if !me.alive {
            return cmds;
        }
        // Every tick: who hurt us, and what the world refused as
        // unreachable stays so.
        self.hop.hear(obs, me);
        if let Some(t) = self.tactics.as_mut() {
            t.hear(obs);
        }
        for e in &obs.events {
            if let Event::TankDamaged { tank, by: Some(b), .. } = e {
                if *tank == me.id && !self.hurt_by.contains(b) {
                    self.hurt_by.push(*b);
                }
            }
            if let Event::StructureDamaged { owner: Some(o), .. } = e {
                if obs.allied(*o) {
                    self.works_hit = true;
                }
            }
            // Back in a new tank at the start (respawn): what the old one
            // was in the middle of is over.
            if matches!(e, Event::Respawned { owner, .. } if *owner == obs.player) {
                self.resupplying = false;
                self.storm = None;
                self.landfall = None;
                self.standoff = None;
                self.settled_at = None;
                self.scoot_until = 0;
                self.hop.reset();
                self.unreachable_tiles.clear();
                self.unreachable_pillboxes.clear();
            }
            // A new shore: what was out of reach from the last may not be.
            if matches!(e, Event::Landed { owner, .. } if *owner == obs.player) {
                self.unreachable_tiles.clear();
                self.unreachable_pillboxes.clear();
            }
            if let Event::Rejected { cmd, why, .. } = e {
                match cmd {
                    Command::Deploy { task: PillboyTask::Capture { pillbox } } | Command::Deploy { task: PillboyTask::Repair { target: pillbox } } if why.contains("unreachable") && !self.unreachable_pillboxes.contains(pillbox) => {
                        self.unreachable_pillboxes.push(*pillbox);
                    }
                    Command::Move { to } if (why.contains("unreachable") || why == "water") && !self.unreachable_tiles.contains(&to.tile()) => {
                        self.unreachable_tiles.push(to.tile());
                    }
                    _ => {}
                }
            }
        }
        // Think four times a second; the world does not need more and the
        // host has three other bots to run.
        let every = if self.rookie { TICK_HZ / 2 } else { TICK_HZ / 4 };
        if obs.tick.saturating_sub(self.last_think) < every as Tick {
            return cmds;
        }
        self.last_think = obs.tick;
        meter::think();
        // A person's proposal (PLAYABILITY item 7) is answered: yes,
        // unless they have hurt this tank. Only people's: bot-only
        // matches, and the traces their balance rests on, are as they
        // were, because a RuleBot never answered a bot.
        let human = |p: PlayerId| obs.players.iter().any(|x| x.id == p && !x.is_bot);
        if let Some(p) = obs.proposals.iter().find(|p| p.to == obs.player && p.from != obs.player && human(p.from)) {
            cmds.push(Command::Answer { proposal: p.id, accept: !self.hurt_by.contains(&p.from) });
        }

        let my_interceptors = obs.my_drones().filter(|d| d.kind == DroneKind::Interceptor).count();
        let my_recon: Vec<&Drone> = obs.my_drones().filter(|d| d.kind == DroneKind::Recon).collect();
        let my_strikes = obs.my_drones().filter(|d| d.kind == DroneKind::Strike).count();
        let hostile_air = obs.hostile_drones().filter(|d| d.pos.dist(me.pos) <= me.bubble + DETECT_MARGIN + 6.0).count();
        self.grace.update(obs);
        let enemy = match self.tactics.as_mut() {
            Some(t) => {
                let grace = &self.grace;
                t.pick(obs, me, |e| grace.fair(obs, e))
            }
            None => self.nearest_enemy(obs, me),
        };
        if enemy.is_some() {
            self.last_contact = obs.tick;
            self.contact_total += every as f32 * DT;
        }
        // Hit in the last second: by something, seen or not.
        let just_hit = me.last_hit_tick > 0 && obs.tick.saturating_sub(me.last_hit_tick) <= TICK_HZ as Tick;
        let stationary = me.goal.is_none();
        if stationary {
            if self.settled_at.is_none() {
                self.settled_at = Some(obs.tick);
            }
        } else {
            self.settled_at = None;
        }
        // The objective: the nearest depot that is not mine and that the
        // world has not called unreachable. With none left, the centre.
        let objective = obs
            .depots
            .iter()
            .filter(|d| !d.owner.map(|o| obs.allied(o)).unwrap_or(false) && !self.unreachable_tiles.contains(&d.pos.tile()))
            .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))
            .map(|d| d.pos)
            .unwrap_or_else(|| obs.centre());
        if self.contact_objective != Some(objective.tile()) {
            self.contact_objective = Some(objective.tile());
            self.contact_total = 0.0;
        }

        // A jammer of this side jams this side's drones too (step 10i):
        // inside one, nothing is launched, and the tank leaves when it
        // can; "inside" for the tank counts its bubble, which flies that
        // far out. The first trace without this launched 610 drones
        // into five friendly jammers in five minutes, against 21
        // hostile.
        let our_jammer = |at: V2, margin: f32| obs.structures.iter().find(|x| x.kind == StructureKind::Jammer && x.working() && x.owner.map(|o| obs.allied(o)).unwrap_or(false) && x.pos.dist(at) <= JAMMER_RANGE + margin);
        let jammed_here = our_jammer(me.pos, me.bubble + 1.0);
        // A rookie's hangar opens late.
        let late = self.rookie && obs.seconds() < ROOKIE_LATE;
        let grounded = late || jammed_here.is_some();

        // 1. The bubble.
        let want_station = if self.rookie { 1 } else if hostile_air > 0 { 4 } else { 2 };
        if my_interceptors < want_station && me.bay.interceptor > 0 && !grounded {
            cmds.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
        }
        // Widen it when a swarm is in, tighten it when the link is poor.
        let want_bubble = if hostile_air >= 3 { 8.0 } else { BUBBLE_DEFAULT };
        if (me.bubble - want_bubble).abs() > 0.5 {
            cmds.push(Command::SetBubble { radius: want_bubble });
        }

        // Out of the way of a shell coming down on us, or a jink when
        // hit; the rest of the think waits for the next.
        if !self.rookie {
            if let Some(d) = self.dodge.order(obs, me) {
                cmds.push(d);
                self.settled_at = None;
                return cmds;
            }
        }

        // 0. Afloat: make for land and do nothing else. The bubble is up;
        // the gun, the drones and the squad wait for the beach. An island
        // hop sails for its target; any other boat runs for the nearest
        // safe beach.
        if me.boat {
            if let Some(hop) = self.hop.think(obs, me, false, map).filter(|_| self.hops) {
                self.landfall = None;
                self.standing = false;
                cmds.extend(hop);
                return cmds;
            }
            if let Some(cmd) = self.landfall_order(obs, me) {
                cmds.push(cmd);
            }
            return cmds;
        }
        self.landfall = None;
        self.standoff = None;
        self.standing = false;
        if self.form.update(obs, me, self.last_contact) {
            log::debug!("{} form {:+.2}: {}", self.name, self.form.mood, self.form.note);
        }
        let patience = self.form.patience(self.patience);

        // 3. Eyes. Launched from a standing tank, over what matters: the
        // enemy if one is known, else the objective. A second recon goes
        // up when the first cannot see a known enemy.
        let look_at = enemy.map(|e| e.pos).unwrap_or(objective);
        let recon_sees_it = my_recon.iter().any(|d| d.pos.dist(look_at) <= RECON_VISION);
        // "Stationary" means settled for a second, not merely not yet
        // ordered to move: on the first think the tank has no goal and a
        // recon launched then orbits the start, where the tank will not
        // be.
        let settled_s = self.settled_at.map(|t| obs.tick.saturating_sub(t) as f32 * DT).unwrap_or(0.0);
        // The recon orbits short of what it watches: a recon inside an
        // enemy's bubble is intercepted (the first 6d trace lost both of
        // an attacker's recons over a defended depot in eight seconds
        // and the attacker sat blind for the rest of the match). Ten
        // tiles short with a two-tile orbit keeps it outside a default
        // bubble and inside its own twelve tiles of vision.
        let dir = (look_at - me.pos).norm();
        let dist = (me.pos.dist(look_at) - (RECON_VISION - 2.0)).clamp(2.0, RECON_LINK * 0.7);
        let center = me.pos + dir.scale(dist);
        if my_recon.is_empty() {
            if me.bay.recon > 0 && (settled_s >= 1.0 || enemy.is_some()) && !grounded && our_jammer(center, 2.0).is_none() {
                cmds.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center, radius: 2.0 } });
            }
        } else if enemy.is_some() && !recon_sees_it {
            // A recon that cannot see the enemy is moved, not doubled
            // (backlog item 3): its orbit goes to where the enemy is,
            // once, and it flies there.
            let r = my_recon[0];
            let already = matches!(&r.task, DroneTask::Orbit { center: c, .. } if c.dist(center) < 4.0);
            if !already && matches!(r.task, DroneTask::Orbit { .. }) {
                cmds.push(Command::Retask { drone: r.id, task: DroneTask::Orbit { center, radius: 2.0 } });
            }
        }

        // 9. The squad. Deployed: hold here and let the gun and the
        // bubble carry on; the movement sections below are skipped.
        let squad_out = me.pillboys.deployed();
        if self.pillboys && !squad_out && me.pillboys.strength > 0 && settled_s >= 1.0 && enemy.is_none() {
            let safe = |p: &Pillbox| !p.alive() || p.pos.dist(me.pos) > PILLBOX_RANGE + 1.0;
            let capture = obs
                .pillboxes
                .iter()
                .filter(|p| !p.owner.map(|o| obs.allied(o)).unwrap_or(false) && (p.owner.is_none() || !p.alive()) && p.pos.dist(me.pos) <= PILLBOY_REACH && safe(p) && !self.unreachable_pillboxes.contains(&p.id))
                .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
            // Structures (step 10e), after pillboxes: neutral or wrecked
            // and in reach to take; ours below half to mend. None on
            // the generated island, so nothing changes there.
            let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
            let works = obs
                .structures
                .iter()
                .filter(|x| !ours(x.owner) && (x.owner.is_none() || !x.alive()) && x.pos.dist(me.pos) <= PILLBOY_REACH && !self.unreachable_pillboxes.contains(&x.id))
                .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
            let mend = obs
                .structures
                .iter()
                .filter(|x| x.owner == Some(obs.player) && x.health < STRUCTURE_HEALTH * 0.5 && x.pos.dist(me.pos) <= PILLBOY_REACH && !self.unreachable_pillboxes.contains(&x.id))
                .min_by(|a, b| a.health.total_cmp(&b.health));
            if let Some(p) = capture {
                cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: p.id } });
            } else if let Some(x) = works {
                cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: x.id } });
            } else if let (Some(x), true) = (mend, me.pillboys.material >= STRUCTURE_REPAIR_COST) {
                cmds.push(Command::Deploy { task: PillboyTask::Repair { target: x.id } });
            } else if let Some(cmd) = mend.and_then(|_| self.fell(obs, me)) {
                // A wreck of mine to mend and no scrap: fell some (10j).
                cmds.push(cmd);
            } else if let Some(cmd) = self.works_order(obs, me) {
                cmds.push(cmd);
            } else if me.pillboys.material >= PILLBOX_REPAIR_COST {
                let repair = obs
                    .pillboxes
                    .iter()
                    .filter(|p| p.owner == Some(obs.player) && p.health < PILLBOX_HEALTH * 0.5 && p.pos.dist(me.pos) <= PILLBOY_REACH && !self.unreachable_pillboxes.contains(&p.id))
                    .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
                if let Some(p) = repair {
                    cmds.push(Command::Deploy { task: PillboyTask::Repair { target: p.id } });
                }
            }
        }

        // 10. The storm: on when the enemy has been in view for twice the
        // patience and the objective is in reach; off when the depot
        // is ours or the bot runs low (section 7 takes it home).
        let hostile_box = |p: &&Pillbox| p.alive() && !p.owner.map(|o| obs.allied(o)).unwrap_or(false);
        if self.storm.is_none() && !self.rookie {
            let long_contact = self.contact_total >= patience * 2.0;
            if long_contact && me.pos.dist(objective) <= INDIRECT_MAX_RANGE {
                self.storm = Some(objective);
                // Said in tank speak, picked by tick and seat: never by the
                // Rng, so the match plays the same whatever is said.
                cmds.push(Command::Chat { text: treadcore_sim::lingo::pick(&treadcore_sim::lingo::STORM, obs.tick ^ obs.player as u64).into(), to: ChatTo::Team });
            }
        }
        if let Some(tgt) = self.storm {
            let taken = obs.depots.iter().any(|d| d.pos.dist(tgt) < 1.0 && d.owner.map(|o| obs.allied(o)).unwrap_or(false));
            if taken {
                self.storm = None;
            }
        }
        // The guns guarding a stormed depot: shelled before anything else.
        let guns: Vec<&Pillbox> = match self.storm {
            Some(tgt) => obs.pillboxes.iter().filter(hostile_box).filter(|p| p.pos.dist(tgt) <= PILLBOX_RANGE + 2.0).collect(),
            None => Vec::new(),
        };
        let gun_target = guns.iter().filter(|p| (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&me.pos.dist(p.pos))).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
        if let Some(g) = gun_target {
            if me.reload == 0 && me.shells > 0 {
                cmds.push(Command::FireIndirect { at: g.pos });
                self.last_fire_tick = obs.tick;
            }
        }

        // 4. The gun, and 6. strikes.
        if let Some(e) = enemy {
            let range = me.pos.dist(e.pos);
            if me.reload == 0 && me.shells > 0 && gun_target.is_none() {
                if range <= DIRECT_RANGE {
                    let aim = if self.rookie { e.pos } else { lead(me, e) };
                    cmds.push(Command::FireDirect { at: aim });
                    self.last_fire_tick = obs.tick;
                } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&range) {
                    // Lead a moving target by its flight time; if the
                    // led point falls inside minimum range (the world
                    // refused eight of these a match, backlog item 4),
                    // fire at the tank itself.
                    let flight = range / SHELL_SPEED_INDIRECT;
                    let lead = V2::from_heading(e.heading).scale(e.speed * flight * 0.8);
                    let aim = if self.rookie { e.pos + self.rng.disc().scale(ROOKIE_SCATTER) } else { e.pos + lead };
                    let aim = if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&me.pos.dist(aim)) { aim } else { e.pos };
                    cmds.push(Command::FireIndirect { at: aim });
                    self.last_fire_tick = obs.tick;
                    self.scoot_until = obs.tick + (6 * TICK_HZ) as Tick;
                }
            }
            let their_team = obs.team_of(e.owner);
            let their_cover = obs.drones.iter().filter(|d| obs.team_of(d.owner) == their_team && d.kind == DroneKind::Interceptor && d.pos.dist(e.pos) <= BUBBLE_MAX + 2.0).count();
            if their_cover < 2 && my_strikes == 0 && me.bay.strike > 0 && !late && range <= STRIKE_BATTERY * STRIKE_SPEED * 0.8 {
                cmds.push(Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: e.id } });
            }
        }

        // The longshot (step 10e): at a hostile structure every half
        // minute, a power plant first, because the rest of a side's
        // economy goes dark with it. None on the generated island.
        if me.bay.longshot > 0 && !late && obs.tick.saturating_sub(self.last_longshot) as f32 * DT >= LONGSHOT_EVERY {
            let target = obs
                .structures
                .iter()
                .filter(|x| x.alive() && x.owner.map(|o| !obs.allied(o)).unwrap_or(false))
                .min_by(|a, b| {
                    let rank = |x: &Structure| (x.kind != StructureKind::Power) as u8;
                    rank(a).cmp(&rank(b)).then(a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)))
                });
            if let Some(x) = target {
                cmds.push(Command::Launch { kind: DroneKind::Longshot, task: DroneTask::Attack { at: x.pos } });
                self.last_longshot = obs.tick;
            }
        }

        // Under fire with the squad out, the squad comes home and the
        // tank moves: the first 6d trace had three attackers die at
        // their starts holding still for a squad that was capturing a
        // pillbox ten tiles away.
        if squad_out {
            if !just_hit {
                return cmds;
            }
            cmds.push(Command::Deploy { task: PillboyTask::Return });
        }

        // 7. Resupply.
        // Blind counts as low: a bot with no recon up and none in the bay
        // cannot see to shoot or to advance.
        let blind = me.bay.recon == 0 && my_recon.is_empty();
        // Under a Commander that keeps one resupply threshold for both
        // (roadmap 12d), the doctrine does not run its own.
        if !self.own_resupply {
            self.resupplying = false;
        }
        let low = self.own_resupply && (me.shells < 6 || me.armor < self.form.home_armor() || me.bay.interceptor == 0 || me.fuel < 120.0 || blind);
        let full = me.shells >= TANK_SHELLS - 2 && me.armor >= TANK_ARMOR - 5.0 && me.bay.interceptor >= BAY_FULL.interceptor - 1 && me.bay.recon > 0;
        if self.resupplying && full {
            self.resupplying = false;
        }
        if low && !self.resupplying {
            self.resupplying = true;
            self.storm = None;
        }
        if self.resupplying {
            // Every depot of the side over the water (the quad's
            // fortifier drove at one, refused, for six minutes): there
            // by sea, the island hop's way.
            let ours = |d: &&Depot| d.owner.map(|o| obs.allied(o)).unwrap_or(false);
            if self.hops && self.nearest_depot(obs, me, true).is_none() {
                if let Some(d) = obs.depots.iter().filter(ours).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))) {
                    if self.hop.active() || self.hop.send(d.pos) {
                        if let Some(h) = self.hop.think(obs, me, true, map) {
                            cmds.extend(h);
                            return cmds;
                        }
                    }
                }
            }
            let depot = self.nearest_depot(obs, me, true).or_else(|| self.nearest_depot(obs, me, false));
            // A working workshop of the side mends and restocks the bay
            // twice as fast as a depot, but has no shells: it is the
            // stop when shells are not what is short (step 10j). None on
            // the generated island.
            let workshop = obs
                .structures
                .iter()
                .filter(|x| x.kind == StructureKind::Workshop && x.working() && x.owner.map(|o| obs.allied(o)).unwrap_or(false))
                .min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
            // Only while what a workshop gives is short: a bot that sat
            // there for shells would sit for ever.
            let workshop = workshop.filter(|_| me.shells >= 6 && (me.armor < TANK_ARMOR - 5.0 || me.bay.interceptor < BAY_FULL.interceptor - 1 || me.bay.recon == 0));
            let stop = match (workshop, depot) {
                (Some(w), Some(d)) if w.pos.dist(me.pos) < d.pos.dist(me.pos) => Some((w.pos, WORKSHOP_RANGE)),
                (Some(w), None) => Some((w.pos, WORKSHOP_RANGE)),
                (_, Some(d)) => Some((d.pos, DEPOT_RADIUS)),
                _ => None,
            };
            if let Some((at, r)) = stop {
                if me.pos.dist(at) > r * 0.6 && stationary {
                    cmds.push(Command::Move { to: at });
                }
            }
            return cmds;
        }

        // 11. Island hopping (hop.rs): with nothing left in reach for a
        // while, across the water for what is not.
        if let Some(hop) = self.hop.think(obs, me, self.storm.is_some() || !self.hops, map).filter(|_| self.hops) {
            cmds.extend(hop);
            return cmds;
        }

        // Hit by something unseen: move anyway. A tank that stands still
        // for a spotted shell's flight is where the next one lands.
        if just_hit && stationary && enemy.is_none() {
            let to = me.pos + self.rng.disc().scale(4.0);
            cmds.push(Command::Move { to: obs.clamp(to) });
            return cmds;
        }

        // Out of our own jammer, blind and bare inside it.
        if let (Some(j), true, None) = (jammed_here, stationary, enemy) {
            let to = j.pos + (me.pos - j.pos).norm().scale(JAMMER_RANGE + me.bubble + 4.0);
            cmds.push(Command::Move { to: obs.clamp(to) });
            return cmds;
        }

        // 10. Storming: guns first, from indirect range; then onto the
        // depot. No scooting.
        if let Some(tgt) = self.storm {
            if stationary {
                if let Some(g) = guns.iter().min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))) {
                    let d = me.pos.dist(g.pos);
                    if d > INDIRECT_MAX_RANGE * 0.9 || d < INDIRECT_MIN_RANGE + 1.0 {
                        // A firing position on the gun: inside indirect
                        // range, outside its own.
                        let to = g.pos + (me.pos - g.pos).norm().scale(INDIRECT_MIN_RANGE + 6.0);
                        cmds.push(Command::Move { to: obs.clamp(to) });
                    }
                } else if me.pos.dist(tgt) > DEPOT_RADIUS * 0.6 {
                    cmds.push(Command::Move { to: tgt });
                }
            }
            return cmds;
        }

        // 5. Scoot, and keep distance: out of direct range, inside indirect.
        if let Some(e) = enemy {
            // 5b. A plain RuleBot kites (a Commander drives its own): in
            // to direct range while the target's gun is reloading, a
            // shot, and the standoff below takes it out again.
            if self.kites && !self.rookie && self.tactics.is_none() {
                let k = tactics::Tactician::default().kite(obs, me, e, (me.pos - e.pos).norm(), tactics::Stance::Engage, DIRECT_RANGE * 1.8);
                if k.stepping_in {
                    cmds.push(Command::Move { to: k.to });
                    return cmds;
                }
            }
            let range = e.pos.dist(me.pos);
            let too_close = range < INDIRECT_MIN_RANGE + 2.0;
            let too_far = range > INDIRECT_MAX_RANGE * 0.9;
            if (obs.tick < self.scoot_until || too_close || too_far) && stationary {
                let away = (me.pos - e.pos).norm();
                let to = if too_far {
                    e.pos + away.scale(INDIRECT_MAX_RANGE * 0.6)
                } else if too_close {
                    e.pos + away.scale(INDIRECT_MIN_RANGE + 8.0)
                } else {
                    me.pos + away.scale(3.0) + self.rng.disc().scale(4.0)
                };
                cmds.push(Command::Move { to: obs.clamp(to) });
            }
            return cmds;
        }

        // 2b. The works trip (step 10k): with the scrap for what a
        // structure of its own wants and nothing in view, the bot drives
        // to it, and the squad (section 9) builds once it is in reach.
        // None on the generated island: no structures.
        if self.pillboys && !squad_out && stationary {
            if let Some((kind, at, _)) = self.works_need(obs, me, WORKS_TRIP) {
                if me.pillboys.material >= kind.cost() {
                    if me.pos.dist(at) > PILLBOY_REACH * 0.8 {
                        let to = obs.clamp(at + (me.pos - at).norm().scale(6.0));
                        if !self.unreachable_tiles.contains(&to.tile()) {
                            cmds.push(Command::Move { to });
                            return cmds;
                        }
                    } else {
                        // In reach: hold, so the squad (which waits for a
                        // settled second) gets to work. The first trace
                        // had every bot arrive and be sent on to its
                        // firing position the next think, and build
                        // nothing. Each hold ends: the squad builds, or
                        // finds no room and remembers it.
                        return cmds;
                    }
                }
            }
        }

        // 2. The firing position, and 8. the advance.
        let to_obj = me.pos.dist(objective);
        let standoff = self.form.standoff();
        if to_obj > standoff + 3.0 && stationary {
            let p = obs.clamp(objective + (me.pos - objective).norm().scale(standoff));
            if !self.unreachable_tiles.contains(&p.tile()) {
                cmds.push(Command::Move { to: p });
            }
        } else if to_obj > DEPOT_RADIUS * 0.6 && stationary {
            let settled = self.settled_at.map(|t| obs.tick.saturating_sub(t)).unwrap_or(0) as f32 * DT;
            let quiet = obs.tick.saturating_sub(self.last_contact) as f32 * DT;
            if settled >= patience && quiet >= patience && !my_recon.is_empty() {

                cmds.push(Command::Move { to: objective });
            }
        }
        cmds
    }
}

// ------------------------------------------------------------- defender

/// The brief's defensive game, as the simplest doctrine that plays it
/// (roadmap step 6d; it grew out of step 6c's Fortifier, which is this
/// brain with `fortify` off plus the rings):
///
///   1. Never leave. The post is a tile off the home depot on the side
///      away from the map's centre, so the rings have room.
///   2. Two interceptors up; four while hostile drones are near.
///   3. Eyes on the approach: one recon over the gate direction,
///      `WATCH` tiles out, so the gun sees what comes while it is
///      still at its own standoff.
///   4. Shoot what is seen: direct inside 10 tiles, indirect from 12 to
///      60, led.
///   5. The squad, in this order, each when the squad is aboard and the
///      material is there: capture the neutral or dead pillbox on the
///      approach (the generated map puts one a third of the way in on
///      every spoke); repair an owned pillbox below half; the moat, a
///      gateless ring of mines at `MOAT_RADIUS` (first: cheap, quick,
///      and the thing that stops a tank; built later, the 6c proof
///      turned the other way, with the attacker walking through the
///      gap onto the depot); build a pillbox at the mouth of the
///      walls, `MOUTH` tiles out along the gate direction; lay the
///      road from the depot to it, so the squad can reach it; mine the
///      approach beyond it; the walls, a ring one tile outside the
///      moat with the gate toward the centre. Forest for material, the
///      nearest first; every refusal remembered.
///   6. Allies. While its side is small a defender proposes, once a
///      minute, to recruit the owner of the nearest depot that is not
///      its side's, and it accepts any proposal from a player who has
///      never hurt it. The three-way rule decides the rest.
///
/// With `fortify` off it does everything but the squad's work: the
/// control group for the proofs.
pub struct Defender {
    name: String,
    last_think: Tick,
    pub fortify: bool,
    home: Option<V2>,
    /// Tiles already ordered: the Observation carries no map, so the
    /// brain remembers rather than looks.
    wall_tries: Vec<(i32, i32)>,
    road_tries: Vec<(i32, i32)>,
    mouth_tries: Vec<(i32, i32)>,
    approach_tries: Vec<(i32, i32)>,
    /// Tiles the world refused to build on, from its own Rejected
    /// events; and forest tiles it was told to fell first.
    blocked: Vec<(i32, i32)>,
    fell: Vec<V2>,
    /// Pillboxes the squad could not reach.
    unreachable: Vec<EntityId>,
    /// Who has shot this tank: never an ally.
    hurt_by: Vec<PlayerId>,
    last_proposal: Tick,
    /// Where the trouble is: the depot of whoever shot us last, or the
    /// last hostile tank seen. The recon watches there.
    watch: Option<V2>,
    /// The first minute: who may be shot yet.
    grace: Grace,
    /// Sidestepping under fire until this tick; the post rule waits.
    dodge_until: Tick,
    rng: Option<Rng>,
    /// Out of a spotted shell's way, inside the moat (the attackers'
    /// dodge, as tanks keep their speed through a turn, beat a defender
    /// that only stepped once hit).
    shells: Dodge,
}

pub const MOAT_RADIUS: f32 = 3.0;
/// The mouth pillbox: outside the wall ring, inside pillbox range of
/// the depot, so a tank on the depot is under its gun.
pub const MOUTH: f32 = 6.0;
/// Where the recon watches from: `RECON_VISION` tiles each side of
/// this covers the approach from the moat to beyond an attacker's
/// standoff.
pub const WATCH: f32 = 16.0;
/// The approach mines, this far out along the gate direction.
pub const APPROACH: f32 = 8.0;
/// Seconds between a defender's proposals.
pub const PROPOSE_EVERY: f32 = 60.0;

impl Defender {
    pub fn new(name: &str, fortify: bool) -> Defender {
        Defender {
            name: name.into(),
            last_think: 0,
            fortify,
            home: None,
            wall_tries: Vec::new(),
            road_tries: Vec::new(),
            mouth_tries: Vec::new(),
            approach_tries: Vec::new(),
            blocked: Vec::new(),
            fell: Vec::new(),
            unreachable: Vec::new(),
            hurt_by: Vec::new(),
            grace: Grace::default(),
            last_proposal: 0,
            watch: None,
            dodge_until: 0,
            rng: None,
            shells: Dodge::default(),
        }
    }

    /// The ring of tiles at `r` round `c`, in the order to lay them: the
    /// far side first. With `gate`, the tile toward `toward` is left
    /// out: a wall ring needs a way out, a minefield does not, because a
    /// tank's own mines never hurt it. The first proof left a gate in
    /// the moat and the attacker's route came down the road through it.
    fn ring(c: V2, r: f32, toward: V2, gate: bool) -> Vec<V2> {
        let mut tiles: Vec<V2> = Vec::new();
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let p = Map::tile_centre(c + V2::new(dx as f32, dy as f32));
                let d = p.dist(Map::tile_centre(c));
                if d >= r - 0.5 && d < r + 0.5 {
                    tiles.push(p);
                }
            }
        }
        let dir = (toward - c).norm();
        // Farthest from the gate first; the gate tile itself is skipped.
        if gate {
            tiles.retain(|p| (*p - c).norm().dist(dir) > 0.5);
        }
        tiles.sort_by(|a, b| (*b - c).norm().dist(dir).total_cmp(&(*a - c).norm().dist(dir)));
        tiles
    }

    /// Candidate sites for the mouth pillbox: along the gate direction
    /// at `MOUTH`, then a little to either side, then further out.
    fn mouth_sites(home: V2, dir: V2) -> Vec<V2> {
        let h = dir.heading();
        let mut out = Vec::new();
        for r in [MOUTH, MOUTH + 1.0, MOUTH + 2.0] {
            for a in [0.0, 0.4, -0.4] {
                out.push(Map::tile_centre(home + V2::from_heading(h + a).scale(r)));
            }
        }
        out
    }

    /// The approach mines: three across the gate direction at
    /// `APPROACH`, the road's tile in the middle.
    fn approach_sites(home: V2, dir: V2) -> Vec<V2> {
        let h = dir.heading();
        [0.0, 0.25, -0.25].iter().map(|a| Map::tile_centre(home + V2::from_heading(h + a).scale(APPROACH))).collect()
    }
}

impl Brain for Defender {
    fn name(&self) -> &str {
        &self.name
    }
    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        let mut cmds = Vec::new();
        let Some(me) = obs.me.as_ref() else { return cmds };
        // Every tick, before the think-rate gate: a refusal lands in the
        // tick after the order, which is rarely a think tick. The world
        // says why an order failed, and the brain has no map to have
        // known. Forest in a ring is felled first (material, too);
        // anything else is left alone. And who shoots us is noted.
        for e in &obs.events {
            match e {
                Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Build { at, .. } }, why, .. } => {
                    if why.starts_with("forest") {
                        if !self.fell.iter().any(|f| f.tile() == at.tile()) {
                            self.fell.push(*at);
                        }
                    } else if !self.blocked.contains(&at.tile()) {
                        self.blocked.push(at.tile());
                    }
                }
                Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Capture { pillbox } }, why, .. } | Event::Rejected { cmd: Command::Deploy { task: PillboyTask::Repair { target: pillbox } }, why, .. } => {
                    if why.contains("unreachable") && !self.unreachable.contains(pillbox) {
                        self.unreachable.push(*pillbox);
                    }
                }
                Event::TankDamaged { tank, by: Some(b), .. } if *tank == me.id && *b != obs.player && !obs.allied(*b) => {
                    if !self.hurt_by.contains(b) {
                        self.hurt_by.push(*b);
                    }
                    // Counter-battery by deduction: the shell came from
                    // the owner of that depot. Look there.
                    if let Some(d) = obs.depots.iter().filter(|d| d.owner == Some(*b)).min_by(|x, y| x.pos.dist(me.pos).total_cmp(&y.pos.dist(me.pos))) {
                        self.watch = Some(d.pos);
                    }
                }
                _ => {}
            }
        }
        if !me.alive || obs.tick.saturating_sub(self.last_think) < (TICK_HZ / 4) as Tick {
            return cmds;
        }
        self.last_think = obs.tick;
        let home = *self.home.get_or_insert_with(|| obs.depots.iter().filter(|d| d.owner == Some(obs.player)).chain(obs.depots.iter().filter(|d| d.owner.map(|o| obs.allied(o)).unwrap_or(false))).map(|d| d.pos).min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos))).unwrap_or(me.pos));
        let centre = obs.centre();
        let dir = (centre - home).norm();
        // 1. Sit by the depot, just off it so the squad's rings have room.
        // Under fire, sidestep inside the depot's radius for a few
        // seconds: a tank that stands still for a spotted shell's flight
        // is where the next one lands (the first 6d trace lost three
        // defenders to the opening salvo in twenty seconds). A short
        // step: the moat is a tile and a half beyond it, and a moat set
        // off by an attacker splashes its owner like anyone else (the
        // 6c proof turned the other way round when the step was longer).
        let post = home + (home - centre).norm().scale(1.0);
        let dodging = obs.tick < self.dodge_until;
        if let Some(Command::Move { to }) = self.shells.order_with(obs, me, false) {
            let r = MOAT_RADIUS - MINE_RADIUS - 0.3;
            let to = if to.dist(home) > r { home + (to - home).norm().scale(r) } else { to };
            self.dodge_until = obs.tick + 2 * TICK_HZ as Tick;
            cmds.push(Command::Move { to: obs.clamp(to) });
            return cmds;
        }
        let just_hit = me.last_hit_tick > 0 && obs.tick.saturating_sub(me.last_hit_tick) <= TICK_HZ as Tick;
        if just_hit && !dodging && me.goal.is_none() {
            let rng = self.rng.get_or_insert_with(|| Rng::new(fnv1a64(self.name.as_bytes()) ^ (obs.player as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
            let to = obs.clamp(home + rng.disc().scale(MOAT_RADIUS - MINE_RADIUS - 0.3));
            self.dodge_until = obs.tick + 4 * TICK_HZ as Tick;
            cmds.push(Command::Move { to });
            return cmds;
        }
        if !dodging && me.pos.dist(post) > 1.5 && me.goal.is_none() {
            cmds.push(Command::Move { to: post });
            return cmds;
        }
        // 6. Allies. Answer what is on the table, then ask.
        // Only the one asked answers; a teammate's answer to the same
        // proposal in the same tick is "no such proposal". When two
        // defenders have proposed to each other in the same minute, the
        // lower seat answers and the higher lets its own proposal be
        // overtaken, or both answers land in one tick and the second
        // finds nothing to answer.
        // One answer a think: accepting one changes sides, and the world
        // then resolves the rest itself.
        let answer = obs.proposals.iter().filter(|p| p.from != obs.player && p.to == obs.player).find(|p| {
            let mutual = obs.proposals.iter().any(|q| q.from == obs.player && q.to == p.from);
            !(mutual && obs.player > p.from)
        });
        if let Some(p) = answer {
            cmds.push(Command::Answer { proposal: p.id, accept: !self.hurt_by.contains(&p.from) });
        }
        let my_side = obs.players.iter().filter(|x| x.alive && x.team == obs.my_team()).count();
        let asked = obs.proposals.iter().any(|p| p.from == obs.player);
        if my_side < 3 && !asked && obs.tick.saturating_sub(self.last_proposal) as f32 * DT >= PROPOSE_EVERY {
            let alive = |o: PlayerId| obs.players.iter().any(|x| x.id == o && x.alive);
            let neighbour = obs
                .depots
                .iter()
                .filter(|d| d.owner.map(|o| !obs.allied(o) && alive(o) && !self.hurt_by.contains(&o)).unwrap_or(false))
                .min_by(|a, b| a.pos.dist(home).total_cmp(&b.pos.dist(home)))
                .and_then(|d| d.owner);
            if let Some(o) = neighbour {
                cmds.push(Command::Propose { to: o, terms: Terms::Recruit });
                self.last_proposal = obs.tick;
            }
        }
        // 2. The bubble.
        let my_interceptors = obs.my_drones().filter(|d| d.kind == DroneKind::Interceptor).count();
        let hostile_air = obs.hostile_drones().filter(|d| d.pos.dist(me.pos) <= me.bubble + DETECT_MARGIN + 6.0).count();
        let want_station = if hostile_air > 0 { 4 } else { 2 };
        if my_interceptors < want_station && me.bay.interceptor > 0 {
            cmds.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
        }
        // 3. Eyes: on the approach from the centre until something
        // happens, then on the trouble (the last shooter's depot, or the
        // last hostile tank seen), short of it by the recon's own vision
        // so it stays out of a bubble. Moved by Retask when the trouble
        // moves.
        self.grace.update(obs);
        let grace = &self.grace;
        let enemy = obs.hostile_tanks().filter(|t| grace.fair(obs, t)).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos)));
        if let Some(e) = enemy {
            self.watch = Some(e.pos);
        }
        let eye = match self.watch {
            Some(w) => {
                let d = (me.pos.dist(w) - (RECON_VISION - 2.0)).clamp(2.0, RECON_LINK * 0.7);
                obs.clamp(me.pos + (w - me.pos).norm().scale(d))
            }
            None => obs.clamp(home + dir.scale(WATCH)),
        };
        match obs.my_drones().find(|d| d.kind == DroneKind::Recon) {
            None => {
                if me.bay.recon > 0 {
                    cmds.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: eye, radius: 3.0 } });
                }
            }
            Some(r) => {
                if matches!(&r.task, DroneTask::Orbit { center, .. } if center.dist(eye) > 4.0) {
                    cmds.push(Command::Retask { drone: r.id, task: DroneTask::Orbit { center: eye, radius: 3.0 } });
                }
            }
        }
        // 4. The gun.
        if let Some(e) = enemy {
            let range = me.pos.dist(e.pos);
            if me.reload == 0 && me.shells > 0 {
                if range <= DIRECT_RANGE {
                    let aim = lead(me, e);
                    cmds.push(Command::FireDirect { at: aim });
                } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&range) {
                    let lead = V2::from_heading(e.heading).scale(e.speed * range / SHELL_SPEED_INDIRECT * 0.8);
                    let aim = e.pos + lead;
                    let aim = if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&me.pos.dist(aim)) { aim } else { e.pos };
                    cmds.push(Command::FireIndirect { at: aim });
                }
            }
        }
        // 5. The squad.
        if !self.fortify || me.pillboys.strength == 0 {
            return cmds;
        }
        if me.pillboys.deployed() || me.goal.is_some() {
            return cmds;
        }
        let material = me.pillboys.material;
        let reach = |p: &Pillbox| p.pos.dist(home) <= PILLBOY_REACH && !self.unreachable.contains(&p.id);
        // Capture: the approach's pillbox, neutral or dead, outside its
        // own range of the post (a live neutral one shoots the tank, not
        // the squad; but the squad's home must stay out of its ring).
        let capture = obs
            .pillboxes
            .iter()
            .filter(|p| reach(p) && !p.owner.map(|o| obs.allied(o)).unwrap_or(false) && (p.owner.is_none() || !p.alive()) && (!p.alive() || p.pos.dist(me.pos) > PILLBOX_RANGE + 1.0))
            .min_by(|a, b| a.pos.dist(home).total_cmp(&b.pos.dist(home)));
        if let Some(p) = capture {
            cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: p.id } });
            return cmds;
        }
        // Repair: an owned pillbox below half, when there is material.
        let repair = obs.pillboxes.iter().filter(|p| reach(p) && p.owner == Some(obs.player) && p.health < PILLBOX_HEALTH * 0.5).min_by(|a, b| a.health.total_cmp(&b.health));
        if let Some(p) = repair {
            if material >= PILLBOX_REPAIR_COST {
                cmds.push(Command::Deploy { task: PillboyTask::Repair { target: p.id } });
                return cmds;
            }
        }
        // Structures (step 10e): take a neutral or wrecked one in reach
        // of home, mend ours below half. None on the generated island.
        let near = |x: &Structure| x.pos.dist(home) <= PILLBOY_REACH && !self.unreachable.contains(&x.id);
        if let Some(x) = obs.structures.iter().filter(|x| near(x) && !x.owner.map(|o| obs.allied(o)).unwrap_or(false) && (x.owner.is_none() || !x.alive())).min_by(|a, b| a.pos.dist(home).total_cmp(&b.pos.dist(home))) {
            cmds.push(Command::Deploy { task: PillboyTask::Capture { pillbox: x.id } });
            return cmds;
        }
        if let Some(x) = obs.structures.iter().filter(|x| near(x) && x.owner == Some(obs.player) && x.health < STRUCTURE_HEALTH * 0.5).min_by(|a, b| a.health.total_cmp(&b.health)) {
            if material >= STRUCTURE_REPAIR_COST {
                cmds.push(Command::Deploy { task: PillboyTask::Repair { target: x.id } });
                return cmds;
            }
        }
        // Material. Forest tiles in the rings wait until there is room
        // for their material: the first version re-ordered a mine onto
        // forest, was told to fell it, could not (material full), and
        // did it again four times a second for three minutes.
        if material + HARVEST_MATERIAL <= PILLBOY_MATERIAL_MAX {
            if let Some(tree) = self.fell.first().copied() {
                self.fell.remove(0);
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: tree } });
                return cmds;
            }
        }
        let known: Vec<(i32, i32)> = obs.mines.iter().filter(|m| m.owner == obs.player).map(|m| m.pos.tile()).collect();
        let forest_tiles: Vec<(i32, i32)> = self.fell.iter().map(|f| f.tile()).collect();
        let open = |p: &V2| !self.blocked.contains(&p.tile()) && !forest_tiles.contains(&p.tile());
        // The mouth pillbox: one, live, ours, near the mouth.
        let has_mouth = obs.pillboxes.iter().any(|p| p.owner == Some(obs.player) && p.alive() && p.pos.dist(home) <= MOUTH + 3.0 && p.pos.dist(home) >= MOAT_RADIUS + 1.0);
        let mouth_site = if has_mouth { None } else { Self::mouth_sites(home, dir).into_iter().find(|p| open(p) && !self.mouth_tries.contains(&p.tile()) && !obs.pillboxes.iter().any(|b| b.pos.dist(*p) < PILLBOX_SPACING)) };
        // The road starts outside the depot's own radius (nothing is
        // built on a depot) and runs to the mouth.
        let road: Vec<V2> = (3..(MOUTH as i32)).map(|r| Map::tile_centre(home + dir.scale(r as f32))).filter(|p| open(p) && !self.road_tries.contains(&p.tile())).collect();
        let approach: Vec<V2> = Self::approach_sites(home, dir).into_iter().filter(|p| open(p) && !known.contains(&p.tile()) && !self.approach_tries.contains(&p.tile())).collect();
        let moat: Vec<V2> = Self::ring(home, MOAT_RADIUS, centre, false).into_iter().filter(|p| !known.contains(&p.tile()) && open(p)).collect();
        let walls: Vec<V2> = Self::ring(home, MOAT_RADIUS + 1.0, centre, true).into_iter().filter(|p| open(p) && !self.wall_tries.contains(&p.tile())).collect();
        // What the next job costs; harvest when short, wait when there
        // is no forest in sight (the depot's trickle will do).
        let need = if moat.is_empty() && mouth_site.is_some() { PILLBOX_BUILD_COST } else { 1 };
        if material < need || (material < 4 && !moat.is_empty() && material < moat.len() as u8) {
            if let Some(tree) = obs.forest.first() {
                cmds.push(Command::Deploy { task: PillboyTask::Harvest { at: *tree } });
            }
            return cmds;
        }
        // The moat first: cheap, quick, and the thing that stops a tank.
        if let Some(at) = moat.first() {
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, at: *at } });
            return cmds;
        }
        if let Some(at) = mouth_site {
            self.mouth_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Pillbox, at } });
            return cmds;
        }
        if let Some(at) = road.first().copied() {
            self.road_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Road, at } });
            return cmds;
        }
        if let Some(at) = approach.first().copied() {
            self.approach_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, at } });
            return cmds;
        }
        // Walls: the Observation has no map, so try each tile in turn;
        // a refused order (already a wall) costs nothing but a tick.
        if let Some(at) = walls.first().copied() {
            self.wall_tries.push(at.tile());
            cmds.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Wall, at } });
        }
        cmds
    }
}

// ------------------------------------------------------------- json brain


/// An external program as a Brain. Each tick the observation goes out as
/// one line of JSON on its stdin; it answers with one line: a JSON array
/// of commands (possibly empty). Slow answers are not waited for -- the
/// tick goes on without them and the next observation is sent anyway.
/// A program that wants to reason at length should do so across ticks.
///
/// Observation and Command are exactly treadcore-sim's types, serialised
/// with serde: the schema is the source. `treadcore-server --brain-schema`
/// prints an example of each.
pub struct JsonBrain {
    name: String,
    child: std::process::Child,
    stdin: std::process::ChildStdin,
    rx: std::sync::Mutex<std::sync::mpsc::Receiver<String>>,
}

impl JsonBrain {
    pub fn spawn(name: &str, program: &str, args: &[String]) -> std::io::Result<JsonBrain> {
        use std::process::{Command as P, Stdio};
        let mut child = P::new(program).args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        if tx.send(l).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(JsonBrain { name: name.into(), child, stdin, rx: std::sync::Mutex::new(rx) })
    }
}

impl Brain for JsonBrain {
    fn name(&self) -> &str {
        &self.name
    }
    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        if let Ok(line) = serde_json::to_string(obs) {
            let _ = writeln!(self.stdin, "{}", line);
        }
        let mut cmds = Vec::new();
        let rx = self.rx.lock().unwrap();
        while let Ok(line) = rx.try_recv() {
            match serde_json::from_str::<Vec<Command>>(&line) {
                Ok(c) => cmds.extend(c),
                Err(e) => log::warn!("brain '{}': bad line: {}", self.name, e),
            }
        }
        cmds
    }
}

impl Drop for JsonBrain {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

/// Example documents for the JSON brain, printed by the server.
pub fn schema_examples() -> String {
    let w = World::new(Scenario::default());
    let obs = w.observe(0);
    let cmds = vec![
        Command::Move { to: V2::new(40.0, 40.0) },
        Command::FireIndirect { at: V2::new(64.0, 64.0) },
        Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: V2::new(50.0, 50.0), radius: 4.0 } },
        Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return },
        Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: 7 } },
        Command::SetBubble { radius: 8.0 },
        Command::Recall,
        Command::Stop,
        Command::Chat { text: "on my way".into(), to: ChatTo::Team },
        Command::Alliance(AllianceCmd::Request { to: 1 }),
        Command::Propose { to: 2, terms: Terms::Swap },
    ];
    format!(
        "# observation (one line per tick on stdin)\n{}\n\n# commands (one JSON array per line on stdout)\n{}\n",
        serde_json::to_string_pretty(&obs).unwrap(),
        serde_json::to_string_pretty(&cmds).unwrap()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PLAYABILITY item 9: a rookie is beatable. Four RuleBots and four
    /// rookies, alternating seats, ten seeds of five minutes: the
    /// Dodge: a shell of another side's coming down on a standing bot is
    /// driven out from under, and the bot is not hurt when it lands.
    #[test]
    fn a_bot_drives_out_from_under_a_shell() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        let me = w.tanks[0].pos;
        let id = w.next_id;
        w.next_id += 1;
        let tick = w.tick;
        w.shells.push(Shell { id, owner: 1, kind: ShellKind::Indirect, from: me + V2::new(30.0, 0.0), aim: me, impact: me, launch_tick: tick, impact_tick: tick + 3 * TICK_HZ as Tick, spotted: true });
        let mut b = RuleBot::new(5, "dodger");
        let mut hurt = false;
        for _ in 0..4 * TICK_HZ {
            let obs = w.observe(0);
            let orders: Vec<Order> = b.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            hurt |= w.events.iter().any(|e| matches!(e, Event::TankDamaged { tank, .. } if *tank == w.tanks[0].id));
        }
        let now = w.tanks[0].pos;
        eprintln!("a_bot_drives_out_from_under_a_shell: {:.1} tiles from the impact", now.dist(me));
        assert!(!hurt, "the shell hurt it");
        assert!(now.dist(me) > SPLASH_RADIUS);
    }

    /// `dodge_hold` (LR 27): a tank at its dodge point before the
    /// shell lands stays there at 1, and is free to its errand at 0
    /// (the dodge before LR 27). The default numbers pick the same
    /// point as the constants.
    #[test]
    fn a_dodge_holds_out_of_the_splash_by_its_weight() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        let me = w.tanks[0].pos;
        let id = w.next_id;
        w.next_id += 1;
        let tick = w.tick;
        w.shells.push(Shell { id, owner: 1, kind: ShellKind::Indirect, from: me + V2::new(30.0, 0.0), aim: me, impact: me, launch_tick: tick, impact_tick: tick + 3 * TICK_HZ as Tick, spotted: true });
        let obs = w.observe(0);
        let tank = obs.me.clone().unwrap();
        let mut old = Dodge::default();
        let first = old.order(&obs, &tank);
        for hold in [0.0, 1.0] {
            let mut d = Dodge { tune: Some([4.0, 0.8, 2.5, hold]), ..Default::default() };
            assert_eq!(d.order(&obs, &tank), first, "the default numbers move as the constants do");
            let mut arrived = tank.clone();
            arrived.goal = None;
            let mut driving = tank.clone();
            driving.goal = Some(me + V2::new(0.0, 5.0));
            assert!(d.holding(&obs, &driving), "held while it drives, hold {}", hold);
            assert_eq!(d.holding(&obs, &arrived), hold > 0.5, "arrived, hold {}", hold);
        }
    }

    /// `landfall_shy` (backlog 30, LR 31): of two beaches, the nearer
    /// one's run passing a hostile gun in reach, at 0 the boat takes it
    /// (its price is the beach and the heading only), at 1 the farther
    /// one clear of the guns.
    #[test]
    fn a_shy_boat_runs_for_the_beach_clear_of_the_guns() {
        let w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(3) });
        let mut obs = w.observe(0);
        let mut me = obs.me.clone().unwrap();
        me.route.clear();
        me.goal = None;
        let p = me.pos;
        let (near, far) = (p + V2::new(30.0, 0.0), p + V2::new(0.0, -40.0));
        let gun = |owner: PlayerId, at: V2| Tank { owner, pos: at, alive: true, ..w.tanks[owner as usize].clone() };
        obs.tanks = vec![me.clone(), gun(1, p + V2::new(15.0, 12.0)), gun(2, p + V2::new(0.0, -55.0))];
        obs.pillboxes.clear();
        obs.shore = vec![near, far];
        for (shy, want) in [(0.0, near), (1.0, far)] {
            let mut b = RuleBot::new(1, "boat");
            b.landfall_shy = shy;
            assert_eq!(b.landfall_order(&obs, &me), Some(Command::Move { to: want }), "shy {}", shy);
        }
    }

    /// `landfall_wait` (backlog 33, LR 33): the only beach covered by a
    /// seen gun 8 tiles off, at 0 the boat lands; at 10 it backs off
    /// out of the gun's reach, and lands once the 10 s are spent.
    #[test]
    fn a_boat_stands_off_a_covered_beach_for_its_wait() {
        let w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let mut obs = w.observe(0);
        let mut me = obs.me.clone().unwrap();
        me.route.clear();
        me.goal = None;
        let p = me.pos;
        let (beach, gun) = (p + V2::new(10.0, 0.0), p + V2::new(8.0, 0.0));
        obs.tanks = vec![me.clone(), Tank { owner: 1, pos: gun, alive: true, ..w.tanks[1].clone() }];
        obs.pillboxes.clear();
        obs.shore = vec![beach];
        let mut b = RuleBot::new(1, "boat");
        assert_eq!(b.landfall_order(&obs, &me), Some(Command::Move { to: beach }), "wait 0 lands");
        let mut b = RuleBot::new(1, "boat");
        b.landfall_wait = 10.0;
        let off = obs.clamp(gun + V2::new(-(DIRECT_RANGE + 6.0), 0.0));
        assert_eq!(b.landfall_order(&obs, &me), Some(Command::Move { to: off }));
        assert!(b.standing_off());
        obs.tick += 11 * TICK_HZ as Tick;
        assert_eq!(b.landfall_order(&obs, &me), Some(Command::Move { to: beach }), "the wait spent, it lands");
        assert!(!b.standing_off());
    }

    /// RuleBots win more than twice as often, kill more and outlive
    /// them, and the rookies still kill something.
    /// Does a plain RuleBot's kiting pay? Eight RuleBots on the island,
    /// the even seats kiting and the odd not, then the other way round,
    /// over `RB_KITE_SEEDS` seeds (default 10). `cargo test -p
    /// treadcore-brain --release -- --ignored rulebot_kiting_pays
    /// --nocapture`.
    #[test]
    #[ignore]
    fn rulebot_kiting_pays() {
        let seeds: u64 = std::env::var("RB_KITE_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(10);
        let (mut on, mut off) = ([0f32; 3], [0f32; 3]);
        for seed in 0..seeds {
            let kiting = (seed % 2) as u8;
            let scenario = Scenario { seed: 0x0B0B_0000 + seed * 7919, swarm_interval: 45 * TICK_HZ, time_limit: 5 * 60 * TICK_HZ as Tick, ..Scenario::bots(8) };
            let mut w = World::new(scenario);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..8u8)
                .map(|i| {
                    let mut b = RuleBot::new(3000 + seed * 16 + i as u64, "rulebot");
                    b.kites = i % 2 == kiting;
                    (i, Box::new(b) as Box<dyn Brain>)
                })
                .collect();
            let (mut kills, mut deaths, mut hit) = ([0f32; 2], [0f32; 2], [0f32; 2]);
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    match e {
                        Event::TankDestroyed { owner, by, .. } => {
                            deaths[*owner as usize % 2] += 1.0;
                            if let Some(b) = by.filter(|b| *b != *owner) {
                                kills[b as usize % 2] += 1.0;
                            }
                        }
                        Event::TankDamaged { tank, damage, .. } if *damage == DIRECT_DAMAGE => {
                            if let Some(t) = w.tanks.iter().find(|t| t.id == *tank) {
                                hit[t.owner as usize % 2] += 1.0;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let k = kiting as usize;
            for (i, x) in [kills, deaths, hit].iter().enumerate() {
                on[i] += x[k];
                off[i] += x[1 - k];
            }
        }
        eprintln!("rulebot_kiting_pays over {} seeds, four bots each: kiting kills {} deaths {} direct hits taken {}; not kiting kills {} deaths {} direct hits taken {}", seeds, on[0], on[1], on[2], off[0], off[1], off[2]);
    }

    #[test]
    fn rookies_are_beatable() {
        let (mut wins, mut kills, mut alive) = ((0u32, 0u32), (0u32, 0u32), (0u32, 0u32));
        for seed in 0..10u64 {
            let scenario = Scenario { seed: 0x4006_1E00 + seed * 7919, swarm_interval: 45 * TICK_HZ, time_limit: 5 * 60 * TICK_HZ as Tick, ..Scenario::bots(8) };
            let mut w = World::new(scenario);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..8u8)
                .map(|i| {
                    let b: Box<dyn Brain> = if i % 2 == 0 { Box::new(RuleBot::new(2000 + seed * 16 + i as u64, "rulebot")) } else { Box::new(RuleBot::rookie(2000 + seed * 16 + i as u64, "rookie")) };
                    (i, b)
                })
                .collect();
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    if let Event::TankDestroyed { by: Some(b), owner, .. } = e {
                        if b % 2 != owner % 2 {
                            if b % 2 == 0 { kills.0 += 1 } else { kills.1 += 1 }
                        }
                    }
                }
            }
            for p in &w.players {
                if p.id % 2 == 0 { alive.0 += p.alive as u32 } else { alive.1 += p.alive as u32 }
            }
            if let Some(win) = w.winner {
                if win % 2 == 0 { wins.0 += 1 } else { wins.1 += 1 }
            }
        }
        eprintln!("rookies_are_beatable over 10 seeds: wins {}:{}  kills {}:{}  alive {}:{}", wins.0, wins.1, kills.0, kills.1, alive.0, alive.1);
        // First run, 14 October 2026: wins 9:1, kills 12:6, alive 25:18.
        assert!(wins.0 > wins.1 * 2, "rookies won too often: {:?}", wins);
        assert!(kills.0 > kills.1, "rookies killed as much as RuleBots: {:?}", kills);
        assert!(alive.0 > alive.1, "rookies outlived RuleBots: {:?}", alive);
        assert!(kills.1 > 0, "rookies never killed anything: a rookie that cannot fight teaches nothing");
    }

    /// Step 10k: a RuleBot with the scrap drives to a dark refinery of
    /// its own thirty tiles off, out of the squad's reach, and powers it.
    #[test]
    fn a_rulebot_drives_to_its_zone_to_build() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = Map::tile_centre(w.tanks[0].pos);
        let toward = (w.map.center() - home).norm();
        let zone = Map::tile_centre(home + toward.scale(30.0));
        assert!(!w.map.route(home, zone).is_empty(), "the zone is not reachable on this island");
        for dy in -1..2 {
            for dx in -1..2 {
                w.map.set(zone + V2::new(dx as f32, dy as f32), Terrain::Pad);
            }
        }
        w.structures.push(Structure { id: 60_002, kind: StructureKind::Refinery, pos: zone, owner: Some(0), health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: false });
        w.tanks[0].pillboys.material = PILLBOY_MATERIAL_MAX;
        let mut bot = RuleBot::new(17, "rulebot");
        let mut power = None;
        for i in 0..(150 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = bot.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            for e in &w.events {
                if let Event::StructureBuilt { kind: StructureKind::Power, by: 0, at, .. } = e {
                    power = power.or(Some((i, *at)));
                }
            }
        }
        let (_, at) = power.expect("no power plant built at the far zone");
        assert!(at.dist(zone) <= JAMMER_RANGE * 0.5 + 0.1, "built at {:?}, the zone is {:?}", at, zone);
        assert!(w.structures.iter().find(|x| x.id == 60_002).unwrap().working(), "the refinery is still dark");
    }

    /// Step 10j: a RuleBot holding a dark refinery builds a power plant
    /// beside it first, then a workshop; no jammer, because nothing of
    /// its side has been hit.
    #[test]
    fn a_rulebot_powers_its_zone_then_builds_a_workshop() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = Map::tile_centre(w.tanks[0].pos);
        let zone = home + V2::new(5.0, -1.0);
        for dy in 0..3 {
            for dx in 0..3 {
                w.map.set(zone + V2::new(dx as f32, dy as f32), Terrain::Pad);
            }
        }
        w.structures.push(Structure { id: 60_001, kind: StructureKind::Refinery, pos: Map::tile_centre(zone + V2::new(2.0, 2.0)), owner: Some(0), health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: false });
        w.tanks[0].pillboys.material = PILLBOY_MATERIAL_MAX;
        let mut bot = RuleBot::new(13, "rulebot");
        let mut built: Vec<StructureKind> = Vec::new();
        for _ in 0..(180 * TICK_HZ) {
            let obs = w.observe(0);
            let orders: Vec<Order> = bot.think(&obs).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            for e in &w.events {
                if let Event::StructureBuilt { kind, by: 0, .. } = e {
                    built.push(*kind);
                }
            }
        }
        assert_eq!(built.first(), Some(&StructureKind::Power), "built {:?}", built);
        assert!(built.contains(&StructureKind::Workshop), "built {:?}", built);
        assert!(!built.contains(&StructureKind::Jammer), "a jammer with no hit: {:?}", built);
        assert!(w.structures.iter().find(|x| x.id == 60_001).unwrap().working(), "the refinery is still dark");
    }

    /// Step 10i: a RuleBot holding a powered zone builds no jammer until
    /// a structure of its side is hit; then it builds one on a free pad,
    /// launches nothing inside it, and drives out of it.
    #[test]
    fn a_rulebot_jams_after_a_hit() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) };
        let mut w = World::new(sc);
        w.pillboxes.clear();
        let home = Map::tile_centre(w.tanks[0].pos);
        let zone = home + V2::new(5.0, -1.0);
        for dy in 0..3 {
            for dx in 0..3 {
                w.map.set(zone + V2::new(dx as f32, dy as f32), Terrain::Pad);
            }
        }
        let mut put = |kind: StructureKind, at: V2, id: EntityId| {
            w.structures.push(Structure { id, kind, pos: Map::tile_centre(at), owner: Some(0), health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: false });
            id
        };
        let plant = put(StructureKind::Power, zone, 60_000);
        put(StructureKind::Refinery, zone + V2::new(2.0, 2.0), 60_001);
        w.tanks[0].pillboys.material = PILLBOY_MATERIAL_MAX;
        let mut bot = RuleBot::new(11, "rulebot");
        let mut jammer: Option<(EntityId, V2)> = None;
        let mut launched_inside = 0;
        let mut out_at = None;
        for i in 0..(120 * TICK_HZ) {
            let mut obs = w.observe(0);
            // The hit comes at the twentieth second.
            if i == 20 * TICK_HZ {
                obs.events.push(Event::StructureDamaged { structure: plant, kind: StructureKind::Power, owner: Some(0), by: Some(1), damage: 10.0, health_left: STRUCTURE_HEALTH - 10.0 });
            }
            let cmds = bot.think(&obs);
            if let Some((_, at)) = jammer {
                let me = w.tanks[0].pos;
                if me.dist(at) <= JAMMER_RANGE + w.tanks[0].bubble {
                    launched_inside += cmds.iter().filter(|c| matches!(c, Command::Launch { .. })).count();
                } else if out_at.is_none() {
                    out_at = Some(i);
                }
            }
            let orders: Vec<Order> = cmds.into_iter().map(|cmd| Order { player: 0, cmd }).collect();
            w.step(&orders);
            for e in &w.events {
                if let Event::StructureBuilt { structure, kind: StructureKind::Jammer, by: 0, at } = e {
                    assert!(i > 20 * TICK_HZ, "a jammer before any hit, at tick {}", i);
                    jammer = Some((*structure, *at));
                }
            }
        }
        let (id, at) = jammer.expect("no jammer built after the hit");
        assert!(w.structure(id).unwrap().working(), "the jammer is not working");
        assert!(at.dist(zone) <= JAMMER_RANGE, "the jammer is not over the zone: {:?}", at);
        assert_eq!(launched_inside, 0, "drones launched inside its own jammer");
        assert!(out_at.is_some(), "the tank never left its own jammer");
    }

    /// A RuleBot answers a person's proposal: yes, unless that person has
    /// hurt it. Nine seats in three sides; seat 0 is a person.
    #[test]
    fn a_rulebot_answers_a_person() {
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(9) }.with_teams(3);
        for hurt in [false, true] {
            let mut w = World::new(sc.clone());
            w.pillboxes.clear();
            w.players[0].is_bot = false;
            let to = (1..9u8).find(|p| w.team_of(*p) != w.team_of(0)).unwrap();
            w.step(&[Order { player: 0, cmd: Command::Propose { to, terms: Terms::Join } }]);
            assert!(w.events.iter().any(|e| matches!(e, Event::Proposed { .. })), "{:?}", w.events);
            let mut bot = RuleBot::new(7, "rulebot");
            let tank = w.players[to as usize].tank;
            let mut resolved = None;
            for i in 0..40 {
                let mut obs = w.observe(to);
                if hurt && i == 0 {
                    obs.events.push(Event::TankDamaged { tank, by: Some(0), damage: 1.0, armor_left: 50.0 });
                }
                let orders: Vec<Order> = bot.think(&obs).into_iter().map(|cmd| Order { player: to, cmd }).collect();
                w.step(&orders);
                resolved = resolved.or_else(|| w.events.iter().find_map(|e| if let Event::ProposalResolved { accepted, .. } = e { Some(*accepted) } else { None }));
            }
            assert_eq!(resolved, Some(!hurt), "hurt {}: the bot answered {:?}", hurt, resolved);
        }
    }

    /// Sixteen bots, six minutes: the rules get exercised. (Four bots on
    /// the sixteen-start map spend the match collecting the twelve free
    /// depots and rarely meet; that is play, not a failure, but it is
    /// not a test of the gun.)
    /// Roadmap step 3's proof: over twenty seeds, bots that capture
    /// pillboxes beat the same doctrine without the squad. Eight seats,
    /// alternating; five minutes each. The margin is printed so the
    /// trace of a run can show it, and asserted on the aggregate.
    #[test]
    fn pillboys_pay() {
        let mut wins = (0u32, 0u32);
        let mut score = (0u32, 0u32);
        let mut alive = (0u32, 0u32);
        let mut captures = 0u32;
        for seed in 0..20u64 {
            let scenario = Scenario { seed: 0x5EED_0000 + seed * 7919, swarm_interval: 45 * TICK_HZ, time_limit: 5 * 60 * TICK_HZ as Tick, ..Scenario::bots(8) };
            let mut w = World::new(scenario);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..8u8)
                .map(|i| {
                    let b: Box<dyn Brain> = if i % 2 == 0 { Box::new(RuleBot::new(1000 + seed * 16 + i as u64, "squad")) } else { Box::new(RuleBot::without_pillboys(1000 + seed * 16 + i as u64, "nosquad")) };
                    (i, b)
                })
                .collect();
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                captures += w.events.iter().filter(|e| matches!(e, Event::PillboxCaptured { .. })).count() as u32;
            }
            for p in &w.players {
                let with = p.id % 2 == 0;
                if with { score.0 += p.score; alive.0 += p.alive as u32 } else { score.1 += p.score; alive.1 += p.alive as u32 }
            }
            if let Some(win) = w.winner {
                if win % 2 == 0 { wins.0 += 1 } else { wins.1 += 1 }
            }
        }
        eprintln!("pillboys_pay over 20 seeds: wins {}:{}  score {}:{}  alive {}:{}  captures {}", wins.0, wins.1, score.0, score.1, alive.0, alive.1, captures);
        assert!(captures >= 20, "the squads barely captured anything: {}", captures);
        assert!(score.0 > score.1, "capturers did not outscore: {} vs {}", score.0, score.1);
        assert!(alive.0 >= alive.1, "capturers did not outlive: {} vs {}", alive.0, alive.1);
    }

    /// Roadmap step 6c's proof: a fortified depot survives a scripted
    /// attack longer than a bare one, over twenty seeds. Two seats: the
    /// Fortifier at its home depot, and an attacker that drives for that
    /// depot after a grace period and fires at what it sees. Measured:
    /// the tick the defender dies, or the time limit; and whether the
    /// attacker died. The margin is printed so a trace can show it.
    #[test]
    fn fortification_pays() {
        struct Attacker {
            target: V2,
            start: Tick,
            last: Tick,
        }
        impl Brain for Attacker {
            fn name(&self) -> &str {
                "attacker"
            }
            fn think(&mut self, obs: &Observation) -> Vec<Command> {
                let mut c = Vec::new();
                let Some(me) = obs.me.as_ref() else { return c };
                if obs.tick < self.start || obs.tick.saturating_sub(self.last) < 5 {
                    return c;
                }
                self.last = obs.tick;
                if let Some(e) = obs.hostile_tanks().next() {
                    if me.reload == 0 && me.shells > 0 {
                        let r = me.pos.dist(e.pos);
                        if r <= DIRECT_RANGE {
                            c.push(Command::FireDirect { at: e.pos });
                        } else if (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&r) {
                            c.push(Command::FireIndirect { at: e.pos });
                        }
                    }
                }
                if me.goal.is_none() && me.pos.dist(self.target) > 1.0 {
                    c.push(Command::Move { to: self.target });
                }
                c
            }
        }
        let grace = 150 * TICK_HZ as Tick;
        let limit = 6 * 60 * TICK_HZ as Tick;
        let mut survived = (0u64, 0u64);
        let mut defender_alive = (0u32, 0u32);
        let mut attacker_dead = (0u32, 0u32);
        let mut built = (0u32, 0u32);
        for seed in 0..20u64 {
            for fortify in [true, false] {
                let scenario = Scenario { seed: 0xF0F7_0000 + seed * 104_729, swarm_interval: 0, time_limit: limit, ..Scenario::bots(2) };
                let mut w = World::new(scenario);
                w.pillboxes.clear();
                let depot = w.depots.iter().find(|d| d.owner == Some(0)).unwrap().pos;
                let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = vec![(0, Box::new(Defender::new("fort", fortify))), (1, Box::new(Attacker { target: depot, start: grace, last: 0 }))];
                let mut died_at = limit;
                let mut things = 0u32;
                while !w.over {
                    let orders = run_bots(&w, &mut bots);
                    w.step(&orders);
                    things += w.events.iter().filter(|e| matches!(e, Event::Built { .. })).count() as u32;
                    if !w.players[0].alive && died_at == limit {
                        died_at = w.tick;
                    }
                }
                let slot = if fortify { &mut survived.0 } else { &mut survived.1 };
                *slot += died_at;
                if w.players[0].alive {
                    if fortify { defender_alive.0 += 1 } else { defender_alive.1 += 1 }
                }
                if !w.players[1].alive {
                    if fortify { attacker_dead.0 += 1 } else { attacker_dead.1 += 1 }
                }
                if fortify { built.0 += things } else { built.1 += things }
            }
        }
        eprintln!(
            "fortification_pays over 20 seeds: defender survived to tick {} vs {} (of {} max); defender alive {}:{}  attacker dead {}:{}  things built {}:{}",
            survived.0, survived.1, limit * 20, defender_alive.0, defender_alive.1, attacker_dead.0, attacker_dead.1, built.0, built.1
        );
        assert!(built.0 >= 60, "the fortifier barely built anything: {}", built.0);
        assert!(survived.0 > survived.1, "fortified did not outlive bare: {} vs {}", survived.0, survived.1);
        assert!(defender_alive.0 >= defender_alive.1);
    }

    /// PLAYABILITY.md item 1, the first minute: a newcomer who sits at
    /// the start reading the help is not shelled by the bots for the
    /// first minute. Seat 0 gives no orders at all; fifteen rule bots
    /// play round it on three seeds. Nothing of a player's may touch
    /// seat 0's tank before GRACE_SECONDS, and the bots must still have
    /// fought each other, so the proof is not of bots that do nothing.
    #[test]
    fn a_newcomer_gets_a_first_minute() {
        let grace = GRACE_SECONDS as Tick * TICK_HZ as Tick;
        let mut first_hits = Vec::new();
        for seed in [0x7EAD_C0DEu64, 0x1234, 0xBEEF] {
            let scenario = Scenario { seed, swarm_interval: 0, time_limit: grace + 30 * TICK_HZ as Tick, ..Scenario::bots(16) };
            let mut w = World::new(scenario);
            let mine = w.players[0].tank;
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (1..16u8).map(|p| (p as PlayerId, Box::new(RuleBot::new(seed ^ p as u64, &seat_name(p as usize))) as Box<dyn Brain>)).collect();
            let mut first_hit: Option<Tick> = None;
            let mut fired = 0u32;
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    match e {
                        Event::TankDamaged { tank, by: Some(p), .. } if *tank == mine && *p != SWARM_OWNER && *p != NEUTRAL => {
                            first_hit.get_or_insert(w.tick);
                        }
                        Event::ShellLaunched { .. } => fired += 1,
                        _ => {}
                    }
                }
            }
            eprintln!("seed {:#x}: seat 0 first hit by a player at {:?} (grace ends {}), shells fired {}", seed, first_hit, grace, fired);
            assert!(fired > 10, "the bots barely fought: {} shells", fired);
            if let Some(t) = first_hit {
                assert!(t >= grace, "seed {:#x}: the sitting newcomer was hit at tick {}, inside the first minute", seed, t);
            }
            first_hits.push(first_hit);
        }
    }

    /// Roadmap step 6d's proof, as it came out: sixteen bots, four
    /// attacking, over ten seeds of five minutes. The match lasts to
    /// the time limit, the defended depots stay defended, and the
    /// defenders outlive the attackers. The roadmap predicted that
    /// most kills would be pillbox kills; they are shell kills, both
    /// ways, because the gun out-ranges the pillbox 60 tiles to 8 and
    /// an attacker that storms a prepared depot is one that has
    /// already lost the duel. The split is printed so the trace of a
    /// run shows it.
    #[test]
    fn defenders_hold() {
        let seeds = 10u64;
        let limit = 5 * 60 * TICK_HZ as Tick;
        let mut to_the_limit = 0u32;
        let mut alive = (0u32, 0u32); // attackers, defenders
        let mut attacker_depots = 0u32;
        let mut defender_depots = 0u32;
        let mut kills = std::collections::BTreeMap::new();
        let mut pillbox_shots = 0u32;
        let mut built = 0u32;
        for seed in 0..seeds {
            let scenario = Scenario { seed: 0x6D6D_0000 + seed * 7919, time_limit: limit, ..Scenario::bots(16) };
            let mut w = World::new(scenario);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..16u8)
                .map(|i| {
                    let b: Box<dyn Brain> = if i < 4 { Box::new(RuleBot::new(5000 + seed * 16 + i as u64, "attacker")) } else { Box::new(Defender::new("defender", true)) };
                    (i, b)
                })
                .collect();
            let mut pending: std::collections::HashMap<EntityId, &str> = std::collections::HashMap::new();
            let mut last: std::collections::HashMap<EntityId, &str> = std::collections::HashMap::new();
            while !w.over {
                let orders = run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    match e {
                        Event::StrikeHit { tank, .. } => {
                            pending.insert(*tank, "strike");
                        }
                        Event::PillboxFired { tank, .. } => {
                            pillbox_shots += 1;
                            pending.insert(*tank, "pillbox");
                        }
                        Event::MineExploded { hits, .. } => {
                            for h in hits {
                                pending.insert(*h, "mine");
                            }
                        }
                        Event::TankDamaged { tank, .. } => {
                            let c = pending.remove(tank).unwrap_or("shell");
                            last.insert(*tank, c);
                        }
                        Event::TankDestroyed { tank, .. } => {
                            *kills.entry(last.get(tank).copied().unwrap_or("?")).or_insert(0u32) += 1;
                        }
                        Event::PillboxBuilt { .. } => built += 1,
                        _ => {}
                    }
                }
            }
            if w.tick >= limit {
                to_the_limit += 1;
            }
            alive.0 += w.players.iter().filter(|p| p.alive && p.id < 4).count() as u32;
            alive.1 += w.players.iter().filter(|p| p.alive && p.id >= 4).count() as u32;
            attacker_depots += w.depots.iter().filter(|d| d.owner.map(|o| o < 4).unwrap_or(false)).count() as u32;
            defender_depots += w.depots.iter().filter(|d| d.owner.map(|o| o >= 4).unwrap_or(false)).count() as u32;
        }
        eprintln!(
            "defenders_hold over {} seeds: to the time limit {}; alive attackers {} of {}, defenders {} of {}; depots at the end: defenders {} of their {}, attackers {} (their own {}); kills by cause {:?}; pillbox shots {}; pillboxes built {}",
            seeds,
            to_the_limit,
            alive.0,
            4 * seeds,
            alive.1,
            12 * seeds,
            defender_depots,
            12 * seeds,
            attacker_depots,
            4 * seeds,
            kills,
            pillbox_shots,
            built
        );
        assert!(to_the_limit * 2 > seeds as u32, "the match did not last: {} of {}", to_the_limit, seeds);
        assert!(alive.1 * 4 > alive.0 * 12, "defenders did not outlive attackers: {} of {} vs {} of {}", alive.1, 12 * seeds, alive.0, 4 * seeds);
        assert!(defender_depots * 10 >= 12 * seeds as u32 * 6, "the defenders lost their depots: {} of {}", defender_depots, 12 * seeds);
        assert!(built >= seeds as u32, "the defenders barely built pillboxes: {}", built);
    }

    #[test]
    fn bots_fight_to_a_result() {


        let scenario = Scenario { swarm_interval: 30 * TICK_HZ, time_limit: 6 * 60 * TICK_HZ as Tick, ..Scenario::default() };
        let mut w = World::new(scenario);
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..MAX_PLAYERS).map(|i| (i as PlayerId, Box::new(RuleBot::new(100 + i as u64, "bot")) as Box<dyn Brain>)).collect();

        let mut shots = 0;
        let mut launches = 0;
        while !w.over {
            let orders = run_bots(&w, &mut bots);
            w.step(&orders);
            shots += w.events.iter().filter(|e| matches!(e, Event::ShellLaunched { .. })).count();
            launches += w.events.iter().filter(|e| matches!(e, Event::DroneLaunched { .. })).count();
        }
        assert!(shots > 10, "bots never fired: {}", shots);
        assert!(launches > 10, "bots never launched: {}", launches);
    }
}

#[cfg(test)]
mod direct_fire {
    use super::*;

    /// Roadmap 12d, backlog 8 (LR 19): a direct shell flies five ticks at
    /// most (10 tiles at 40 a second), and a tank at full speed turning
    /// as hard as it can moves about 0.4 tiles off the gunner's lead in
    /// that time, inside the 0.8 tiles of `DIRECT_HIT_RADIUS`. So no
    /// answer after the launch saves it, at any range: a dodge of direct
    /// fire has to come before the shot.
    #[test]
    fn a_led_direct_shot_cannot_be_dodged() {
        for r in [4.0f32, 7.0, 9.5] {
            let mut line = format!("range {:>4}:", r);
            for (k, react) in ["none", "reverse", "turn away", "turn toward", "stop"].iter().enumerate() {
                let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
                w.pillboxes.clear();
                for x in 30..110 {
                    for y in 40..90 {
                        w.map.set(V2::new(x as f32 + 0.5, y as f32 + 0.5), Terrain::Grass);
                    }
                }
                let (g, t) = (w.players[0].tank, w.players[1].tank);
                {
                    let tt = w.tanks.iter_mut().find(|x| x.id == t).unwrap();
                    tt.pos = V2::new(50.5, 64.5);
                    tt.heading = 0.0;
                    tt.speed = 0.0;
                }
                {
                    let gt = w.tanks.iter_mut().find(|x| x.id == g).unwrap();
                    gt.pos = V2::new(70.5, 64.5 + r);
                    gt.reload = 0;
                    gt.shells = 10;
                }
                let far = V2::new(105.5, 64.5);
                for _ in 0..(4 * TICK_HZ) {
                    w.step(&[Order { player: 1, cmd: Command::Move { to: far } }]);
                }
                let e = w.tank(t).unwrap().clone();
                {
                    let gt = w.tanks.iter_mut().find(|x| x.id == g).unwrap();
                    gt.pos = e.pos + V2::new(0.0, r);
                    gt.speed = 0.0;
                    gt.goal = None;
                    gt.route.clear();
                }
                let me = w.tank(g).unwrap().clone();
                let aim = lead(&me, &e);
                let armor = e.armor;
                w.step(&[Order { player: 0, cmd: Command::FireDirect { at: aim } }]);
                let f = V2::from_heading(e.heading);
                let side = V2::new(-f.y, f.x);
                let away = if { let d = e.pos - me.pos; d.x * side.x + d.y * side.y >= 0.0 } { side } else { side.scale(-1.0) };
                let cmd = match k {
                    1 => Some(Command::Move { to: e.pos - f.scale(10.0) }),
                    2 => Some(Command::Move { to: e.pos + away.scale(10.0) }),
                    3 => Some(Command::Move { to: e.pos - away.scale(10.0) }),
                    4 => Some(Command::Stop),
                    _ => None,
                };
                let mut orders: Vec<Order> = cmd.into_iter().map(|cmd| Order { player: 1, cmd }).collect();
                for _ in 0..10 {
                    w.step(&orders);
                    orders.clear();
                }
                let hit = w.tank(t).unwrap().armor < armor;
                line.push_str(&format!("  {} {}", react, if hit { "hit" } else { "miss" }));
                assert!(hit, "{}: {} got out of it", line, react);
            }
            eprintln!("{}", line);
        }
    }
}
