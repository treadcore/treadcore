//! Fleets (Paul, 15 October: "can we start a game with ... 3 teams of
//! base building, 5 on each, and 1 random aggro bot"): sixteen seats
//! dealt into three teams of five neighbours round the map's edge and
//! one RuleBot alone, every team's bots Commanders that share their
//! leader's base.
//!
//! The starts run north, east, south, west, the corners, then the eight
//! between (`Map::generate`); a team is five starts side by side, so
//! its base is in reach of all five:
//!
//! ```text
//!   team 0: NW, N-west, N (seat 0, the leader), N-east, NE
//!   team 1: E-north, E (seat 1, the leader), E-south, SE, S-east
//!   team 2: S (seat 2, the leader), S-west, SW, W-south, W
//!   team 3: W-north alone, the aggro RuleBot
//! ```
//!
//! A bot team's leader is on Auto and builds the castle at its own
//! depot; the others take its depot for their base too: a guard on
//! defend base, and a fleet of three wingmen (two of them called
//! rangers, crews for the castle) in a wedge on the leader.
//! Seat 0's team assists the person in it: a builder on Auto whose
//! castle goes round the person's depot, a guard on it, two wingmen on
//! the person.

use crate::commander::{Commander, Goal};
use crate::weights::{Caps, Settings};
use crate::{Brain, RuleBot};
use treadcore_sim::{PlayerId, TeamId};

pub const SEATS: usize = 16;

/// Each team's seats, the leader first.
pub const TEAMS: [[usize; 5]; 3] = [[0, 7, 8, 9, 4], [1, 10, 11, 5, 12], [2, 13, 6, 14, 3]];
/// The aggro bot's seat, a team of its own.
pub const AGGRO: usize = 15;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Role {
    /// Auto at its own depot: builds the castle.
    Leader,
    /// Auto at the leader's depot: builds the leader's castle.
    Builder,
    /// Defend base, the leader's.
    Guard,
    /// A wingman of the fleet (fleet_of); it roamed alone on Auto once.
    Ranger,
    /// Escort the leader.
    Wing,
    /// A RuleBot alone.
    Aggro,
}

impl Role {
    /// The role in a word, for the status line and the logs.
    pub fn word(self) -> &'static str {
        match self {
            Role::Leader => "leader",
            Role::Builder => "builder",
            Role::Guard => "guard",
            Role::Ranger => "ranger",
            Role::Wing => "wing",
            Role::Aggro => "aggro",
        }
    }
}

/// The team of every seat.
pub fn teams() -> Vec<TeamId> {
    (0..SEATS).map(|s| TEAMS.iter().position(|t| t.contains(&s)).unwrap_or(TEAMS.len()) as TeamId).collect()
}

/// A seat's role and its team's leader. `person` is whether the
/// leader's seat is a person's (seat 0's team, as a rule).
pub fn role(seat: usize, person_leads: bool) -> (Role, PlayerId) {
    let Some(team) = TEAMS.iter().find(|t| t.contains(&seat)) else { return (Role::Aggro, seat as PlayerId) };
    let leader = team[0] as PlayerId;
    let rank = team.iter().position(|s| *s == seat).unwrap();
    (role_by_rank(rank, person_leads), leader)
}

/// A bots' team's roles by rank, the choice `roles` (weights.rs; Paul,
/// 29 October: "think like different units"): the leader first in each,
/// then 0 as built (a guard, two rangers, a wing), 1 no guard (all four
/// with the fleet), 2 two guards, 3 three guards. A ranger and a wing
/// are both the fleet's wingmen now (commander_as); what the mixes move
/// is how many keep the base and how many travel. A team past five
/// seats (the window's) is wings past the fifth, as it was.
pub const MIXES: [[Role; 5]; 4] = [
    [Role::Leader, Role::Guard, Role::Ranger, Role::Ranger, Role::Wing],
    [Role::Leader, Role::Ranger, Role::Ranger, Role::Ranger, Role::Wing],
    [Role::Leader, Role::Guard, Role::Guard, Role::Ranger, Role::Wing],
    [Role::Leader, Role::Guard, Role::Guard, Role::Guard, Role::Wing],
];

/// A seat's role by its place in its team, the leader first.
pub fn role_by_rank(rank: usize, person_leads: bool) -> Role {
    // A person's team gets its builder while `assist` is 0.5 or more
    // (roadmap 12h); under it, the person's team is dealt as a bots'
    // team is, the same roles for every side.
    let s = settings();
    let person_leads = person_leads && s.w[crate::weights::W::Assist] >= 0.5;
    match (person_leads, rank) {
        (_, 0) => Role::Leader,
        (true, 1) => Role::Builder,
        (true, 2) => Role::Guard,
        (true, _) => Role::Wing,
        (false, r) => MIXES[s.choice(crate::weights::W::Roles)].get(r).copied().unwrap_or(Role::Wing),
    }
}

/// Any team, as the window deals one (`--teams N`, seat i on team
/// i % N; the default match): `team` is its seats in order, the first
/// the leader. The same roles as a fleet's: a person's team gets a
/// builder for the person's castle, a guard and wingmen; a bots' team a
/// leader, a guard, rangers and wingmen.
pub fn team_role(seat: usize, team: &[usize], person_leads: bool) -> (Role, PlayerId) {
    let rank = team.iter().position(|s| *s == seat).unwrap_or(team.len());
    (role_by_rank(rank, person_leads), team.first().copied().unwrap_or(seat) as PlayerId)
}

/// The brain for a seat of a team dealt by the window, named for its
/// role.
pub fn team_brain(seat: usize, team: &[usize], seed: u64, person_leads: bool, name: &str) -> Box<dyn Brain> {
    let (role, leader) = team_role(seat, team, person_leads);
    match commander_as(role, leader, seed, name) {
        Some(mut c) => {
            fleet_of(&mut c, seat, team, person_leads);
            equip(&mut c, leader);
            Box::new(c)
        }
        None => Box::new(RuleBot::new(seed, name)),
    }
}

// ------------------------------------------------- settings and boards

/// The weights every new Commander gets (`--weights FILE`, roadmap 12c):
/// the process's, unless this thread has its own (a census trying a
/// genome, `with_settings`).
static SETTINGS: std::sync::Mutex<Option<Settings>> = std::sync::Mutex::new(None);

thread_local! {
    static OWN: std::cell::RefCell<Option<Settings>> = const { std::cell::RefCell::new(None) };
    /// The team boards of the match being dealt, by leader seat.
    static BOARDS: std::cell::RefCell<Vec<(PlayerId, crate::board::Shared)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The process's weights for every Commander made from now on.
pub fn set_settings(s: Settings) {
    if let Ok(mut g) = SETTINGS.lock() {
        *g = Some(s);
    }
}

/// The weights a new Commander gets here.
pub fn settings() -> Settings {
    OWN.with(|o| o.borrow().clone()).or_else(|| SETTINGS.lock().ok().and_then(|g| g.clone())).unwrap_or_default()
}

/// Run `f` with this thread's Commanders made with `s`.
pub fn with_settings<R>(s: Settings, f: impl FnOnce() -> R) -> R {
    let old = OWN.with(|o| o.replace(Some(s)));
    let r = f();
    OWN.with(|o| *o.borrow_mut() = old);
    r
}

/// A new match is being dealt: each team gets a new board.
pub fn new_match() {
    BOARDS.with(|b| b.borrow_mut().clear());
}

/// The board of `leader`'s team in the match being dealt.
pub fn board_for(leader: PlayerId) -> crate::board::Shared {
    BOARDS.with(|b| {
        let mut b = b.borrow_mut();
        if let Some((_, s)) = b.iter().find(|(l, _)| *l == leader) {
            return s.clone();
        }
        let s = crate::board::shared();
        b.push((leader, s.clone()));
        s
    })
}

/// A Commander of `leader`'s team: the weights, and the team's board if
/// the board is on.
pub fn equip(c: &mut Commander, leader: PlayerId) {
    let s = settings();
    if s.caps.has(Caps::BOARD) {
        c.board = Some(board_for(leader));
    }
    c.set_settings(s);
}

/// The fleet (Paul, 17 October: "i want to see the tanks prioritise
/// grouping and traveling in formation. and then using shared orders
/// and attack together"): a team's wingmen, rangers among them, each
/// with its slot in the leader's wedge, in team order; the leader with
/// their seats, to gather them before it travels and to give them its
/// calls as orders.
pub fn fleet_of(c: &mut Commander, seat: usize, team: &[usize], person_leads: bool) {
    let wings: Vec<usize> = team.iter().copied().filter(|s| matches!(team_role(*s, team, person_leads).0, Role::Wing | Role::Ranger)).collect();
    if let Some(k) = wings.iter().position(|s| *s == seat) {
        c.slot = Some(k);
    }
    if team.first() == Some(&seat) {
        c.members = wings.iter().map(|s| *s as PlayerId).collect();
    }
}

/// The seat's name in the roster: team and role.
pub fn name(seat: usize) -> String {
    let (role, _) = role(seat, seat_zero_team(seat));
    match role {
        Role::Aggro => "aggro".into(),
        r => format!("t{}-{}", teams()[seat], r.word()),
    }
}

/// The seat is in seat 0's team, which the roster names as a person's to
/// lead (a window deals it so).
fn seat_zero_team(seat: usize) -> bool {
    TEAMS[0].contains(&seat)
}

/// The brain for a bot seat. `person_leads`: the team's leader seat is a
/// person's.
pub fn brain(seat: usize, seed: u64, person_leads: bool) -> Box<dyn Brain> {
    match TEAMS.iter().find(|t| t.contains(&seat)) {
        Some(team) => team_brain(seat, team, seed, person_leads, &name(seat)),
        None => {
            let (role, leader) = role(seat, person_leads);
            brain_as(role, leader, seed, &name(seat))
        }
    }
}

/// A role's brain. Every Commander of a team calls out the enemies it
/// sees with a ping; all but the builder and the guard, who keep the
/// base, answer a teammate's ping by converging on it.
pub fn brain_as(role: Role, leader: PlayerId, seed: u64, name: &str) -> Box<dyn Brain> {
    match commander_as(role, leader, seed, name) {
        Some(mut c) => {
            equip(&mut c, leader);
            Box::new(c)
        }
        None => Box::new(RuleBot::new(seed, name)),
    }
}

/// A role's Commander, to change before it plays (a test's tactics);
/// None for the aggro seat, a RuleBot.
pub fn commander_as(role: Role, leader: PlayerId, seed: u64, name: &str) -> Option<Commander> {
    let name = name.to_string();
    let mut c = Commander::new(seed, &name);
    c.pings = true;
    c.answers_pings = !matches!(role, Role::Builder | Role::Guard);
    // The side's fleet commander (crew.rs): a bots' team's leader, a
    // person's team's builder. Rangers crew for it.
    c.fleet.on = matches!(role, Role::Leader | Role::Builder);
    c.crew.on = role == Role::Ranger;
    match role {
        Role::Aggro => return None,
        Role::Leader => c.set_goal(Goal::Auto),
        Role::Builder => {
            c.base_owner = Some(leader);
            c.set_goal(Goal::Auto);
        }
        Role::Guard => {
            c.base_owner = Some(leader);
            c.set_goal(Goal::DefendBase);
        }
        // A ranger is a wingman of the fleet now: it roamed alone, and
        // a side that arrives one at a time is beaten one at a time.
        Role::Ranger | Role::Wing => {
            let mut w = Commander::wingman(seed, &name, leader);
            w.base_owner = Some(leader);
            w.pings = true;
            w.answers_pings = true;
            w.crew.on = true;
            return Some(w);
        }
    }
    Some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_teams_of_five_and_one_alone() {
        let t = teams();
        assert_eq!(t.len(), SEATS);
        for team in 0..3 {
            assert_eq!(t.iter().filter(|x| **x == team).count(), 5);
        }
        assert_eq!(t[AGGRO], 3);
        assert_eq!(t.iter().filter(|x| **x == 3).count(), 1);
        assert_eq!(role(0, true), (Role::Leader, 0));
        assert_eq!(role(7, true), (Role::Builder, 0));
        assert_eq!(role(10, false), (Role::Guard, 1));
        assert_eq!(name(AGGRO), "aggro");
        assert_eq!(name(13), "t2-guard");
    }

    /// The choice `roles`: 0 deals the roles as they always were; each
    /// mix keeps the leader first; a bots' team takes the mix of the
    /// settings it is dealt under, so a census's trial team plays its own.
    #[test]
    fn a_bots_team_takes_its_roles_from_its_settings() {
        let old = |r: usize| match r { 0 => Role::Leader, 1 => Role::Guard, 2 | 3 => Role::Ranger, _ => Role::Wing };
        for r in 0..7 {
            assert_eq!(role_by_rank(r, false), old(r), "rank {} as built", r);
        }
        for mix in MIXES {
            assert_eq!(mix[0], Role::Leader);
        }
        let guards = |text: &str| with_settings(Settings::parse(text).unwrap(), || (0..5).filter(|r| role_by_rank(*r, false) == Role::Guard).count());
        assert_eq!((guards("roles = as_built"), guards("roles = no_guard"), guards("roles = two_guards"), guards("roles = 3")), (1, 0, 2, 3));
        assert_eq!(with_settings(Settings::parse("roles = no_guard").unwrap(), || role_by_rank(1, true)), Role::Builder, "a person's team keeps its builder");
    }

    /// A fleet match with no time limit is decided: one side is left
    /// inside half an hour. Before leaderless bots took command, the
    /// survivors stood guard for good.
    #[test]
    fn a_fleet_match_plays_out() {
        use treadcore_sim::*;
        let seed = 0x7EAD_C0DE_u64;
        let names: Vec<(String, bool)> = (0..SEATS).map(|i| (name(i), true)).collect();
        let sc = Scenario { seed, players: names, teams: Some(teams()), time_limit: 0, ..Scenario::default() };
        let mut w = World::new(sc);
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..SEATS).map(|i| (i as PlayerId, brain(i, seed ^ i as u64, false))).collect();
        while !w.over && w.tick < 30 * 60 * TICK_HZ as Tick {
            let orders = crate::run_bots(&w, &mut bots);
            w.step(&orders);
        }
        eprintln!("a_fleet_match_plays_out: over {} at {} s, team {:?}", w.over, w.tick / TICK_HZ as Tick, w.winning_team);
        assert!(w.over, "no side left standing after thirty minutes");
        assert!(w.winning_team.map(|t| t < 3).unwrap_or(false), "won by {:?}", w.winning_team);
    }

    /// The window's default match: the naval map, sixteen seats dealt
    /// into three teams (seat i on team i % 3), every bot in a team role
    /// and seat 0 on Auto as a person on the autopilot would be. In five
    /// minutes the teams call enemies with pings and answer them, take
    /// depots, and a castle goes up.
    #[test]
    /// Does fleet tactics pay (tactics.rs)? Three teams of five bots on
    /// the naval map, one team with the weights and two without (the
    /// nearest tank shot, every ping answered), the team with them
    /// turned round the three seats over the seeds so no shore favours
    /// it. Kills, deaths and depots, the tactics team against the mean
    /// of the other two. `cargo test -p treadcore-brain --release --
    /// --ignored tactics_pay --nocapture`.
    #[ignore]
    fn tactics_pay() {
        use treadcore_sim::*;
        let seats = 15;
        let seeds: u64 = std::env::var("TACTICS_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(6);
        let (mut on, mut off) = ([0f32; 3], [0f32; 3]);
        for k in 0..seeds {
            let seed = 0x07AC_71C5_u64 ^ (k * 0x9E37_79B9);
            let lucky = (k % 3) as usize;
            let team = |s: usize| (0..seats).filter(|x| x % 3 == s % 3).collect::<Vec<usize>>();
            let names: Vec<(String, bool)> = (0..seats).map(|i| (format!("s{}", i), true)).collect();
            let map = MapSource::Text(include_str!("../../../maps/naval.map").into());
            let sc = Scenario { seed, players: names, map, respawns: 3, time_limit: 0, ..Scenario::default() }.with_teams(3);
            let mut w = World::new(sc);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..seats)
                .map(|i| {
                    let t = team(i);
                    let (role, leader) = team_role(i, &t, false);
                    let mut c = commander_as(role, leader, seed ^ i as u64, &format!("s{}", i)).unwrap();
                    fleet_of(&mut c, i, &t, false);
                    if i % 3 != lucky {
                        c.set_tactics(None);
                    }
                    (i as PlayerId, Box::new(c) as Box<dyn Brain>)
                })
                .collect();
            let (mut kills, mut deaths) = ([0f32; 3], [0f32; 3]);
            while w.tick < 8 * 60 * TICK_HZ as Tick && !w.over {
                let orders = crate::run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    if let Event::TankDestroyed { owner, by, .. } = e {
                        deaths[*owner as usize % 3] += 1.0;
                        if let Some(b) = by.filter(|b| *b as usize % 3 != *owner as usize % 3) {
                            kills[b as usize % 3] += 1.0;
                        }
                    }
                }
            }
            let depots: Vec<f32> = (0..3).map(|t| w.depots.iter().filter(|d| d.owner.map(|o| o as usize % 3 == t).unwrap_or(false)).count() as f32).collect();
            eprintln!("seed {} tactics on team {}: kills {:?} deaths {:?} depots {:?}", k, lucky, kills, deaths, depots);
            for (i, x) in [kills, deaths, [depots[0], depots[1], depots[2]]].iter().enumerate() {
                on[i] += x[lucky];
                off[i] += (x.iter().sum::<f32>() - x[lucky]) / 2.0;
            }
        }
        let n = seeds as f32;
        eprintln!("tactics_pay over {} seeds, a team's mean: with tactics kills {:.1} deaths {:.1} depots {:.1}; without kills {:.1} deaths {:.1} depots {:.1}", seeds, on[0] / n, on[1] / n, on[2] / n, off[0] / n, off[1] / n, off[2] / n);
    }

    /// Does kiting pay (tactics.rs `kite`)? As `tactics_pay`, every
    /// team with the weights, one of the three kiting and two not, the
    /// kiting team turned round the seats over the seeds. Kills, deaths,
    /// direct hits taken and depots, the kiting team against the mean of
    /// the other two. `KITE_SEEDS` (default 6). `cargo test -p
    /// treadcore-brain --release -- --ignored kiting_pays --nocapture`.
    #[test]
    #[ignore]
    fn kiting_pays() {
        use treadcore_sim::*;
        let seats = 15;
        let seeds: u64 = std::env::var("KITE_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(6);
        let (mut on, mut off) = ([0f32; 4], [0f32; 4]);
        for k in 0..seeds {
            let seed = 0x0C17_E5A1_u64 ^ (k * 0x9E37_79B9);
            let lucky = (k % 3) as usize;
            let team = |s: usize| (0..seats).filter(|x| x % 3 == s % 3).collect::<Vec<usize>>();
            let names: Vec<(String, bool)> = (0..seats).map(|i| (format!("s{}", i), true)).collect();
            let map = MapSource::Text(include_str!("../../../maps/naval.map").into());
            let sc = Scenario { seed, players: names, map, respawns: 3, time_limit: 0, ..Scenario::default() }.with_teams(3);
            let mut w = World::new(sc);
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..seats)
                .map(|i| {
                    let t = team(i);
                    let (role, leader) = team_role(i, &t, false);
                    let mut c = commander_as(role, leader, seed ^ i as u64, &format!("s{}", i)).unwrap();
                    fleet_of(&mut c, i, &t, false);
                    c.set_tactics(Some(crate::tactics::Tactics { kite: i % 3 == lucky, ..Default::default() }));
                    (i as PlayerId, Box::new(c) as Box<dyn Brain>)
                })
                .collect();
            let (mut kills, mut deaths, mut hit) = ([0f32; 3], [0f32; 3], [0f32; 3]);
            while w.tick < 8 * 60 * TICK_HZ as Tick && !w.over {
                let orders = crate::run_bots(&w, &mut bots);
                w.step(&orders);
                for e in &w.events {
                    match e {
                        Event::TankDestroyed { owner, by, .. } => {
                            deaths[*owner as usize % 3] += 1.0;
                            if let Some(b) = by.filter(|b| *b as usize % 3 != *owner as usize % 3) {
                                kills[b as usize % 3] += 1.0;
                            }
                        }
                        Event::TankDamaged { tank, by: Some(b), damage, .. } if *damage == DIRECT_DAMAGE => {
                            if let Some(t) = w.tanks.iter().find(|t| t.id == *tank).filter(|t| t.owner as usize % 3 != *b as usize % 3) {
                                hit[t.owner as usize % 3] += 1.0;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let depots: Vec<f32> = (0..3).map(|t| w.depots.iter().filter(|d| d.owner.map(|o| o as usize % 3 == t).unwrap_or(false)).count() as f32).collect();
            eprintln!("seed {} kiting team {}: kills {:?} deaths {:?} direct hits taken {:?} depots {:?}", k, lucky, kills, deaths, hit, depots);
            for (i, x) in [kills, deaths, hit, [depots[0], depots[1], depots[2]]].iter().enumerate() {
                on[i] += x[lucky];
                off[i] += (x.iter().sum::<f32>() - x[lucky]) / 2.0;
            }
        }
        let n = seeds as f32;
        eprintln!("kiting_pays over {} seeds, a team's mean: kiting kills {:.1} deaths {:.1} direct hits taken {:.1} depots {:.1}; not kiting kills {:.1} deaths {:.1} direct hits taken {:.1} depots {:.1}", seeds, on[0] / n, on[1] / n, on[2] / n, on[3] / n, off[0] / n, off[1] / n, off[2] / n, off[3] / n);
    }

    #[test]
    fn the_default_match_plays_as_teams() {
        use treadcore_sim::*;
        let seed = 0x7EAD_C0DE_u64;
        let seats = 16;
        let team = |s: usize| (0..seats).filter(|x| x % 3 == s % 3).collect::<Vec<usize>>();
        let names: Vec<(String, bool)> = (0..seats).map(|i| (format!("s{}", i), i != 0)).collect();
        let map = MapSource::Text(include_str!("../../../maps/naval.map").into());
        let sc = Scenario { seed, players: names, map, respawns: 3, time_limit: 0, ..Scenario::default() }.with_teams(3);
        let mut w = World::new(sc);
        // The spending before LR 86 (`profiles/steady.weights`): this one
        // seed tests the team's mechanics, and with the cowboy's drones
        // its pinging seat happens never to ping.
        let mut steady = Settings::default();
        use crate::weights::W;
        for (k, v) in [(W::ShellBoxes, 0.0), (W::LongshotEvery, 30.0), (W::LongshotAny, 0.0), (W::StrikesMax, 1.0)] {
            steady.w[k] = v;
        }
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = with_settings(steady, || {
            (0..seats)
                .map(|i| {
                    let b: Box<dyn Brain> = if i == 0 {
                        let mut c = Commander::new(seed, "me");
                        c.set_goal(Goal::Auto);
                        c.pings = true;
                        c.answers_pings = true;
                        Box::new(c)
                    } else {
                        let t = team(i);
                        team_brain(i, &t, seed ^ i as u64, t[0] == 0, &format!("s{}", i))
                    };
                    (i as PlayerId, b)
                })
                .collect()
        });
        let (mut pings, mut boats, mut walls) = (0, 0, 0);
        let (mut musters, mut jobs, mut crew_built) = (0, 0, 0);
        let (mut downs, mut revives) = (0, 0);
        let mut given: Vec<(PlayerId, (i32, i32))> = Vec::new();
        let mut rallied = std::collections::HashSet::new();
        while w.tick < 8 * 60 * TICK_HZ as Tick && !w.over {
            let orders = crate::run_bots(&w, &mut bots);
            w.step(&orders);
            for e in &w.events {
                match e {
                    Event::Pinged { .. } => pings += 1,
                    Event::TankDestroyed { .. } => downs += 1,
                    Event::Revived { .. } => revives += 1,
                    Event::Boarded { .. } => boats += 1,
                    Event::Built { what: BuildKind::Wall, .. } => walls += 1,
                    Event::Requested { job: Job::Muster { .. }, .. } => musters += 1,
                    Event::Requested { to: Some(p), job: Job::Build { at, .. } | Job::Fetch { to: at, .. }, .. } => {
                        jobs += 1;
                        given.push((*p, at.tile()));
                    }
                    _ => {}
                }
                match e {
                    Event::Built { owner, at, .. } if given.contains(&(*owner, at.tile())) => crew_built += 1,
                    Event::PillboxBuilt { by, at, .. } if given.contains(&(*by, at.tile())) => crew_built += 1,
                    _ => {}
                }
            }
            for (p, b) in &bots {
                if b.status().contains("RALLY") {
                    rallied.insert(*p);
                }
            }
        }
        let held: Vec<usize> = (0..3).map(|t| w.depots.iter().filter(|d| d.owner.map(|o| o as usize % 3 == t).unwrap_or(false)).count()).collect();
        eprintln!("the_default_match_plays_as_teams: {} pings, {} seats rallied, {} boats boarded, {} walls, depots by team {:?}; {} musters, {} jobs given, {} done by crews; {} tanks down, {} revived", pings, rallied.len(), boats, walls, held, musters, jobs, crew_built, downs, revives);
        assert!(musters > 0 && jobs > 0, "no fleet commander set a crew to work");
        assert!(crew_built > 0, "no crew built what it was asked");
        assert!(pings > 0, "nobody pinged");
        assert!(rallied.len() >= 2, "{} seats answered a ping", rallied.len());
        assert!(revives > 0, "of {} tanks down, nobody revived one", downs);
        assert!(held.iter().sum::<usize>() >= 4, "depots {:?}", held);
    }

    /// The quad match headless (scripts/quad.sh): fifteen seats, three
    /// teams, each led by a window's autopilot (a fleet commander on
    /// Auto), respawns without limit. Every 30 s each seat's place and
    /// status; a seat whose status says it is on the way somewhere but
    /// has moved under 3 tiles in 90 s is printed as stuck, and at the
    /// end the share of the match each kind of status took. A finder of
    /// what to mend, not a pass or fail: `QUAD_SEED`, `QUAD_SECS`
    /// (default 600). `cargo test -p treadcore-brain --release --
    /// --ignored quad_census --nocapture`.
    #[test]
    #[ignore]
    fn quad_census() {
        use treadcore_sim::*;
        let seed = std::env::var("QUAD_SEED").ok().and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or(0x0_0AD5_u64);
        let secs: Tick = std::env::var("QUAD_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(600);
        let seats = 15;
        let team = |s: usize| (0..seats).filter(|x| x % 3 == s % 3).collect::<Vec<usize>>();
        let names: Vec<(String, bool)> = (0..seats).map(|i| (name(i), i >= 3)).collect();
        let map = MapSource::Text(include_str!("../../../maps/naval.map").into());
        let sc = Scenario { seed, players: names, map, respawns: RESPAWNS_UNLIMITED, time_limit: 0, ..Scenario::default() }.with_teams(3);
        let mut w = World::new(sc);
        new_match();
        crate::meter::reset();
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..seats)
            .map(|i| {
                let b: Box<dyn Brain> = if i < 3 {
                    let mut c = Commander::new(seed ^ i as u64, &format!("pilot{}", i));
                    c.set_goal(Goal::Auto);
                    c.pings = true;
                    c.answers_pings = true;
                    c.fleet.on = true;
                    fleet_of(&mut c, i, &team(i), true);
                    Box::new(c)
                } else {
                    team_brain(i, &team(i), seed ^ i as u64, true, &name(i))
                };
                (i as PlayerId, b)
            })
            .collect();
        const MOVING: [&str; 9] = ["to the post", "sailing", "driving", "to the ping", "to the boat", "for the beach", "closing up", "to the firing point", "REVIVE: to"];
        let shape = |s: &str| -> String {
            let s = s.split(" | ").next().unwrap_or("");
            let s = s.split(": ").take(3).collect::<Vec<_>>().join(": ");
            s.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).collect::<String>().replace("##", "#").replace("##", "#")
        };
        let mut seen: Vec<Vec<(V2, String, bool)>> = vec![Vec::new(); seats];
        let mut kinds: std::collections::BTreeMap<String, u32> = Default::default();
        let (mut kills, mut downs, mut revives, mut stuck, mut sunk, mut boats) = (0, 0, 0, 0, 0, 0);
        let mut sunk_by: std::collections::BTreeMap<&str, u32> = Default::default();
        // Kiting's steps, seat-ticks each, by code (status.rs), with a
        // rally that lost sight of its target and kiting at the base; and
        // the direct shots fired while stepping in (tactics.rs `kite`).
        let mut kite = [0u32; 8];
        let (mut lost_sight, mut intruder) = (0u32, 0u32);
        let mut step_shots = 0u32;
        let mut cohesion = (0u32, 0u32);
        // Bridge and road tiles laid.
        let (mut bridged, mut roaded) = (0u32, 0u32);
        let (mut by_leader, mut by_other) = (0u32, 0u32);
        // What each seat was doing a second before it went down, and
        // what got it.
        let mut last_status: Vec<String> = vec![String::new(); seats];
        let mut died_doing: std::collections::BTreeMap<String, u32> = Default::default();
        // Revives: each seat's attempts and how each ended (the status
        // it went to next; "revived" when the wreck came back).
        let mut reviving = vec![false; seats];
        let mut revive_end: std::collections::BTreeMap<String, u32> = Default::default();
        // Downs with an ally of the seat in reach of the wreck, and none.
        let (mut down_near, mut down_alone) = (0u32, 0u32);
        // The last hit each seat took, by weapon, read from its damage.
        let mut last_hit: Vec<&str> = vec!["?"; seats];
        // Indirect splash falls off from 70 to a floor of 17.5, so it is
        // whatever the exact weapons are not.
        let weapon = |d: f32| -> &'static str {
            if d == DIRECT_DAMAGE {
                "direct shell"
            } else if d == STRIKE_DAMAGE {
                "strike drone"
            } else if d == PILLBOX_DAMAGE {
                "pillbox"
            } else if d == MINE_DAMAGE {
                "mine"
            } else {
                "indirect shell"
            }
        };
        while w.tick < secs * TICK_HZ as Tick && !w.over {
            let orders = crate::run_bots(&w, &mut bots);
            for (p, b) in &bots {
                let st = b.status();
                if w.tick % TICK_HZ as Tick == 0 && w.tanks.iter().any(|t| t.owner == *p && t.alive) {
                    last_status[*p as usize] = st.clone();
                }
                let act = b.act();
                let now = act == crate::status::Act::Revive;
                if reviving[*p as usize] && !now {
                    let ok = w.events.iter().any(|e| matches!(e, Event::Revived { by, .. } if by == p));
                    let alive = w.tanks.iter().any(|t| t.owner == *p && t.alive);
                    let to = if ok { "revived".to_string() } else if !alive { "(reviver down)".into() } else { shape(&st) };
                    *revive_end.entry(to).or_default() += 1;
                }
                reviving[*p as usize] = now;
                use crate::status::{Act, KiteStep};
                if let Some(k) = act.kite() {
                    kite[KiteStep::ALL.iter().position(|x| *x == k).unwrap()] += 1;
                }
                lost_sight += (act == Act::RallyLost) as u32;
                intruder += matches!(act, Act::Kite(_)) as u32;
                let stepping = matches!(act.kite(), Some(KiteStep::StepInReloading | KiteStep::StepInBusy));
                if stepping && orders.iter().any(|o| o.player == *p && matches!(o.cmd, Command::FireDirect { .. })) {
                    step_shots += 1;
                }
            }
            w.step(&orders);
            for e in &w.events {
                match e {
                    Event::TankDamaged { tank, damage, .. } => {
                        if let Some(t) = w.tanks.iter().find(|t| t.id == *tank) {
                            // A boat sunk says so first; the damage after it is the shot's.
                            if last_hit[t.owner as usize] != "boat sunk" || t.alive {
                                last_hit[t.owner as usize] = weapon(*damage);
                            }
                        }
                    }
                    Event::TankDestroyed { tank, owner, by } => {
                        if by.is_some() {
                            kills += 1;
                        }
                        let at = w.tanks.iter().find(|t| t.id == *tank).map(|t| t.pos).unwrap_or(V2::new(0.0, 0.0));
                        if w.tanks.iter().any(|t| t.alive && t.owner != *owner && w.allied(t.owner, *owner) && t.pos.dist(at) <= crate::commander::REVIVE_REACH) {
                            down_near += 1;
                        } else {
                            down_alone += 1;
                        }
                        downs += 1;
                        let by_what = match by {
                            Some(b) if w.tanks.iter().any(|t| t.owner == *b && t.alive && w.tanks.iter().any(|v| v.owner == *owner && v.pos.dist(t.pos) <= DIRECT_RANGE + 1.0)) => "tank near",
                            Some(_) => "from afar",
                            None => "swarm or mine",
                        };
                        *died_doing.entry(format!("{} <- {}, {}", shape(&last_status[*owner as usize]), by_what, last_hit[*owner as usize])).or_default() += 1;
                        *died_doing.entry(format!("ALL <- {}", last_hit[*owner as usize])).or_default() += 1;
                    }
                    Event::Revived { .. } => revives += 1,
                    Event::Sunk { owner, at, .. } => {
                        sunk += 1;
                        last_hit[*owner as usize] = "boat sunk";
                        let foe = |o: Option<PlayerId>| o.map(|o| !w.allied(o, *owner)).unwrap_or(false);
                        let why = if w.pillboxes.iter().any(|p| p.alive() && foe(p.owner) && p.pos.dist(*at) <= PILLBOX_RANGE + 0.5) {
                            "a pillbox"
                        } else if w.tanks.iter().any(|t| t.alive && foe(Some(t.owner)) && t.pos.dist(*at) <= DIRECT_RANGE + 0.5) {
                            "a tank"
                        } else {
                            "out of direct reach"
                        };
                        *sunk_by.entry(why).or_default() += 1;
                    }
                    Event::Built { owner, what: BuildKind::Road, at } if w.map.at(*at) == Terrain::Bridge => {
                        if (*owner as usize) < 3 {
                            by_leader += 1;
                        } else {
                            by_other += 1;
                        }
                    }
                    Event::TerrainChanged { to, .. } if *to == Terrain::Bridge as u8 => bridged += 1,
                    Event::TerrainChanged { to, .. } if *to == Terrain::Road as u8 => roaded += 1,
                    Event::Boarded { .. } => boats += 1,
                    _ => {}
                }
            }
            if w.tick % (30 * TICK_HZ as Tick) != 0 {
                continue;
            }
            // Cohesion: each leader's fleet, alive and ashore, and how
            // many of it are within 12 tiles of it.
            for l in 0..3usize {
                let Some(lt) = w.tanks.iter().find(|t| t.owner == l as PlayerId && t.alive) else { continue };
                let t = team(l);
                let wings: Vec<usize> = t.iter().copied().filter(|s| matches!(team_role(*s, &t, true).0, Role::Wing | Role::Ranger)).collect();
                for s in wings {
                    if let Some(x) = w.tanks.iter().find(|x| x.owner == s as PlayerId && x.alive) {
                        cohesion.1 += 1;
                        if x.pos.dist(lt.pos) <= 12.0 {
                            cohesion.0 += 1;
                        }
                    }
                }
            }
            for (p, b) in &bots {
                let i = *p as usize;
                let t = w.tanks.iter().find(|t| t.owner == *p);
                let (pos, alive) = t.map(|t| (t.pos, t.alive)).unwrap_or((V2::new(0.0, 0.0), false));
                let st = b.status();
                *kinds.entry(if alive { shape(&st) } else { "(dead)".into() }).or_default() += 1;
                seen[i].push((pos, st.clone(), alive));
                let h = &seen[i];
                if h.len() >= 4 {
                    let w4 = &h[h.len() - 4..];
                    let still = w4.iter().all(|(q, s, a)| *a && q.dist(pos) < 3.0 && MOVING.iter().any(|m| s.contains(m)));
                    if still {
                        stuck += 1;
                        eprintln!("[{:>4}s] STUCK seat {} at ({:.0},{:.0}): {}", w.tick / TICK_HZ as Tick, i, pos.x, pos.y, st);
                    }
                }
            }
        }
        let total: u32 = kinds.values().sum();
        let mut by: Vec<_> = kinds.into_iter().collect();
        by.sort_by_key(|k| std::cmp::Reverse(k.1));
        for (k, n) in by.iter().take(30) {
            eprintln!("{:>5.1}%  {}", 100.0 * *n as f32 / total as f32, k);
        }
        eprintln!("quad_census seed {:x}, {} s: {} kills, {} down, {} revived, {} stuck samples; {} boats, {} sunk", seed, w.tick / TICK_HZ as Tick, kills, downs, revives, stuck, boats, sunk);
        eprintln!("sunk by {:?}", sunk_by);
        let mut dd: Vec<_> = died_doing.into_iter().collect();
        dd.sort_by_key(|k| std::cmp::Reverse(k.1));
        for (k, n) in dd.iter().take(20) {
            eprintln!("died {:>3}  {}", n, k);
        }
        let mut re: Vec<_> = revive_end.into_iter().collect();
        re.sort_by_key(|k| std::cmp::Reverse(k.1));
        eprintln!("downs with an ally within {} tiles: {}, alone: {}", crate::commander::REVIVE_REACH, down_near, down_alone);
        for (k, n) in re.iter().take(10) {
            eprintln!("revive ended {:>3}  {}", n, k);
        }
        eprintln!("laid: {} bridge tiles, {} road tiles; bridge tiles by the three pilots {}, by the other seats {}", bridged, roaded, by_leader, by_other);
        eprintln!("fleet cohesion: {} of {} wingman samples within 12 tiles of the leader ({:.0}%)", cohesion.0, cohesion.1, 100.0 * cohesion.0 as f32 / cohesion.1.max(1) as f32);
        eprintln!(
            "kiting, seat-seconds: {}, lost sight of it {:.0}, kiting the intruder {:.0}; direct shots fired stepping in: {}",
            crate::status::KiteStep::ALL.iter().zip(kite.iter()).map(|(k, n)| format!("{} {:.0}", k.word(), *n as f32 * DT)).collect::<Vec<_>>().join(", "),
            lost_sight as f32 * DT,
            intruder as f32 * DT,
            step_shots
        );
        eprintln!("{}", crate::meter::read().line());
    }

    /// Seat-seconds a rung, in the ladder's order, for a census line.
    fn rung_line(n: &[u32; 6]) -> String {
        crate::commander::Rung::ALL.iter().zip(n).map(|(r, n)| format!("{} {}", r.label(), n)).collect::<Vec<_>>().join(", ")
    }

    /// What took a tank, read from the events of the tick it died in
    /// (backlog 28): the weapon whose hit names it, whether it sank in
    /// a boat, and whether the shooter was the lone RuleBot.
    fn killed_by(events: &[treadcore_sim::Event], tank: treadcore_sim::EntityId, by: Option<PlayerId>) -> String {
        use treadcore_sim::{Event, ShellKind};
        let (mut what, mut sunk) = ("other", false);
        for e in events {
            let w = match e {
                Event::Sunk { tank: t, .. } if *t == tank => {
                    sunk = true;
                    continue;
                }
                Event::PillboxFired { tank: t, .. } if *t == tank => "pillbox",
                Event::MineExploded { hits, .. } if hits.contains(&tank) => "mine",
                Event::StrikeHit { tank: t, .. } if *t == tank => "strike",
                Event::LongshotHit { hits, .. } if hits.contains(&tank) => "longshot",
                Event::ShellImpact { kind: ShellKind::Direct, hits, .. } if hits.contains(&tank) => "direct",
                Event::ShellImpact { kind: ShellKind::Indirect, hits, .. } if hits.contains(&tank) => "indirect",
                _ => continue,
            };
            what = w;
        }
        let who = match by {
            None => " (none)",
            Some(b) if b as usize == AGGRO => " (aggro)",
            _ => "",
        };
        format!("{}{}{}", what, if sunk { ", sunk" } else { "" }, who)
    }

    /// The brain census (roadmap 12a and 12g): sixteen seats dealt as
    /// `--fleets` deals them, one team on the trial settings and the
    /// other two on the baseline, the trial team turned round the three
    /// over the seeds so no start favours it. Each match writes its
    /// ledger lines (`reports/ledger/ledger.jsonl`, off with
    /// `CENSUS_LEDGER=0`) and the meter's line; the end prints the trial
    /// team's mean score against the baseline teams'. Each seed is played
    /// three times, every team in the trial seat once (`CENSUS_FAIR=0`
    /// for one match a seed, the trial team rotated). The trial is
    /// `CENSUS_WEIGHTS` (a weights file, weights.rs) or `CENSUS_CAPS`
    /// (capabilities added to the baseline, comma-separated), against
    /// `CENSUS_BASE_CAPS` (the baseline teams', the same form) on
    /// `CENSUS_BASE_WEIGHTS` (a weights file; the defaults if unset); seeds
    /// `CENSUS_SEEDS` (3) from `CENSUS_SEED0` (0), seconds `CENSUS_SECS`
    /// (900). `cargo test -p
    /// treadcore-brain --release -- --ignored brain_census --nocapture`.
    #[test]
    #[ignore]
    fn brain_census() {
        use crate::ledger::{Genome, ScoreWeights, Tally};
        use crate::weights::{Caps, Settings};
        use treadcore_sim::*;
        // A name adds it to the default; `-name` takes it off.
        let caps = |var: &str, mut s: Settings| {
            for c in std::env::var(var).unwrap_or_default().split(',').map(str::trim).filter(|c| !c.is_empty()) {
                // `smooth`, a capability before LR 46, is the weight.
                if c == "smooth" || c == "-smooth" {
                    s.w[crate::weights::W::SmoothShare] = if c == "smooth" { 1.0 } else { 0.0 };
                    continue;
                }
                s.caps = match c.strip_prefix('-') {
                    Some(off) => s.caps.without(Caps::by_name(off).unwrap_or_else(|| panic!("no capability {}", off))),
                    None => s.caps.with(if c == "all" { Caps::all() } else { Caps::by_name(c).unwrap_or_else(|| panic!("no capability {}", c)) }),
                };
            }
            s
        };
        // The baseline teams' (`CENSUS_BASE_CAPS`): a part that needs
        // another (bridge_jobs needs board) is tried against that one on.
        // Or a whole weights file (`CENSUS_BASE_WEIGHTS`, backlog 23):
        // SPSA's minus side, or every team but one on old defaults.
        let base = match std::env::var("CENSUS_BASE_WEIGHTS") {
            Ok(path) => caps("CENSUS_BASE_CAPS", Settings::parse(&std::fs::read_to_string(&path).expect("CENSUS_BASE_WEIGHTS")).unwrap_or_else(|e| panic!("{}: {}", path, e))),
            Err(_) => caps("CENSUS_BASE_CAPS", Settings::default()),
        };
        let trial = match std::env::var("CENSUS_WEIGHTS") {
            Ok(path) => Settings::parse(&std::fs::read_to_string(&path).expect("CENSUS_WEIGHTS")).unwrap_or_else(|e| panic!("{}: {}", path, e)),
            Err(_) => caps("CENSUS_CAPS", base.clone()),
        };
        if let Ok(path) = std::env::var("CENSUS_DUMP") {
            std::fs::write(&path, trial.to_text()).expect("CENSUS_DUMP");
        }
        let seeds: u64 = std::env::var("CENSUS_SEEDS").ok().and_then(|s| s.parse().ok()).unwrap_or(3);
        // The first seed's index (`CENSUS_SEED0`, 0): a long census split
        // over the cores, each run its own range of the same seeds.
        let seed0: u64 = std::env::var("CENSUS_SEED0").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
        let secs: Tick = std::env::var("CENSUS_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(900);
        let ledger = std::env::var("CENSUS_LEDGER").map(|v| v != "0").unwrap_or(true);
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/ledger");
        let k = ScoreWeights::from_long_game(std::env::var("CENSUS_LONG_GAME").ok().and_then(|v| v.parse().ok()).unwrap_or(crate::weights::W::LongGame.default_value()));
        // The rewards (LR 84) are in the score; `CENSUS_REWARDS=0` scores
        // as before them.
        let k = if std::env::var("CENSUS_REWARDS").map(|v| v == "0").unwrap_or(false) { k.without_rewards() } else { k };
        eprintln!("brain_census: rewards {}", if k.direct_hit > 0.0 { "on" } else { "off (CENSUS_REWARDS=0)" });
        // A team of learned tanks in the trial seat (LR 96): `CENSUS_NET`
        // is a net's file, or `seed:N` for a random net of that seed.
        let net: Option<crate::net::Net> = std::env::var("CENSUS_NET").ok().filter(|v| !v.is_empty()).map(|v| match v.strip_prefix("seed:") {
            Some(n) => crate::net::Net::tank(n.parse().expect("CENSUS_NET=seed:N")),
            None => crate::net::Net::load(std::path::Path::new(&v)).unwrap_or_else(|e| panic!("CENSUS_NET: {}", e)),
        });
        if let Some(n) = &net {
            eprintln!("brain_census: the trial team is a net ({}, {} weights)", std::env::var("CENSUS_NET").unwrap_or_default(), n.parameters());
        }
        // The fleet's model slots (Paul, 29 October: "an enum of up to 0-7
        // different alternate ai models"): a team whose settings choose
        // `model` k plays as the net in CENSUS_MODELS/k.bin, the trial's
        // and the baseline's alike, so a promoted model is the baseline
        // the next trial meets. A slot asked for and not there stops the
        // census: the rules played in its place would be read as the net.
        let models = std::env::var("CENSUS_MODELS").ok().filter(|v| !v.is_empty());
        let model_of = |s: &Settings, who: &str| -> Option<crate::net::Net> {
            let k = s.choice(crate::weights::W::Model);
            if k == 0 {
                return None;
            }
            let dir = models.as_ref().unwrap_or_else(|| panic!("the {} asks for model slot {}, and CENSUS_MODELS is not set", who, k));
            let p = std::path::Path::new(dir).join(format!("{}.bin", k));
            let n = crate::net::Net::load(&p).unwrap_or_else(|e| panic!("model slot {} ({}): {}", k, p.display(), e));
            eprintln!("brain_census: the {} plays model slot {} ({}, {} weights)", who, k, p.display(), n.parameters());
            Some(n)
        };
        let (trial_model, base_model) = (model_of(&trial, "trial"), model_of(&base, "baseline"));
        eprintln!("brain_census: trial caps [{}], {} weights changed {:?}; baseline caps [{}], {} weights changed {:?}", trial.caps.names(), trial.diff().len(), trial.diff().iter().map(|(w, v)| format!("{}={}", w.name(), v)).collect::<Vec<_>>(), base.caps.names(), base.diff().len(), base.diff().iter().map(|(w, v)| format!("{}={}", w.name(), v)).collect::<Vec<_>>());
        let (mut trial_score, mut base_score, mut trial_n, mut base_n) = (0f32, 0f32, 0u32, 0u32);
        // A seed's sums, trial and baseline, for the difference a seed
        // and its standard error (LR 24): the noise given as a number.
        let mut by_seed = vec![(0f32, 0u32, 0f32, 0u32); seeds as usize];
        // The trial team's seat-seconds on each Auto rung (LR 26).
        let mut rungs_total = [0u32; 6];
        // Where the trial team's time goes, by status code
        // (`CENSUS_ACTS=1`, LR 26): every seat-second, dead ones apart.
        let acts_on = std::env::var("CENSUS_ACTS").map(|v| v != "0").unwrap_or(false);
        let mut acts: std::collections::BTreeMap<String, u32> = Default::default();
        // What a trial seat was doing when it went down, and what took
        // it (backlog 28, LR 27): the code it held last, every code it
        // held in the 5 s before, and the weapon, read from the tick's
        // events. Counted under `CENSUS_ACTS` too.
        let mut deaths = 0u32;
        let mut died_last: std::collections::BTreeMap<String, u32> = Default::default();
        let mut died_window: std::collections::BTreeMap<String, u32> = Default::default();
        let mut died_by: std::collections::BTreeMap<String, u32> = Default::default();
        let mut died_both: std::collections::BTreeMap<String, u32> = Default::default();
        // Of the landfall deaths with a killer, who had seen it in the
        // 10 s before (backlog 32): the boat, a teammate, both, neither.
        // Sight sampled once a second.
        let mut landfall_seen: std::collections::BTreeMap<&str, u32> = Default::default();
        // Where the gun stood (backlog 34): for each landfall death with
        // a killer, the killer's distance to the boat and to its beach,
        // the boat's to its beach (NaN if it had none), and whether the
        // killer was afloat.
        let mut gun_at: Vec<(f32, f32, f32, bool)> = Vec::new();
        // Known earlier (backlog 35): of the landfall deaths with a
        // killer, whether the team saw the killer 10 to 60 s before, and
        // if so whether it stood then within its range + 4 of where the
        // boat died. Team sight, sampled once a second.
        let mut known: std::collections::BTreeMap<&str, u32> = Default::default();
        let mut total = crate::meter::Meter::default();
        // Fair (backlog 16, journal 0038): every team takes the trial seat
        // on every seed, three matches a seed, so the starts' unevenness
        // cancels; a null run then reads exactly even. `CENSUS_FAIR=0`
        // rotates the trial team over the seeds instead, a third of the
        // matches: the census before 18 October, archived
        // (documentation/archive/BRAIN-METHODS.md).
        let fair = std::env::var("CENSUS_FAIR").map(|v| v != "0").unwrap_or(true);
        // The maps (`CENSUS_MAP`, backlog 25, LR 71): a file, or a list of
        // them split by commas, `gen` for a map generated from the seed
        // (a new map every seed). Seed n plays the (n mod k)th, so a seed
        // keeps its map however the seeds are split over jobs; every team
        // takes the trial seat on it as before, and each seed is one paired
        // reading on one map. Unset: `gen` alone.
        let maps: Vec<(String, MapSource)> = std::env::var("CENSUS_MAP")
            .unwrap_or_else(|_| "gen".into())
            .split(',')
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(|m| match m {
                "gen" => ("generated".to_string(), MapSource::FromScenarioSeed),
                path => (std::path::Path::new(path).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| path.into()), MapSource::Text(std::fs::read_to_string(path).unwrap_or_else(|e| panic!("CENSUS_MAP {}: {}", path, e)))),
            })
            .collect();
        assert!(!maps.is_empty(), "CENSUS_MAP names no map");
        let map_of = |n: u64| &maps[(n % maps.len() as u64) as usize];
        let plays: Vec<(u64, TeamId)> = (seed0..seed0 + seeds).flat_map(|n| if fair { (0..3).map(|t| (n, t as TeamId)).collect::<Vec<_>>() } else { vec![(n, (n % 3) as TeamId)] }).collect();
        for (n, lucky) in plays {
            let seed = 0x7EAD_C0DE_u64 ^ (n * 0x9E37_79B9);
            let names: Vec<(String, bool)> = (0..SEATS).map(|i| (name(i), true)).collect();
            let (map_name, map) = map_of(n).clone();
            let sc = Scenario { seed, players: names, teams: Some(teams()), time_limit: 0, map, respawns: std::env::var("CENSUS_RESPAWNS").ok().and_then(|s| s.parse().ok()).unwrap_or(0), ..Scenario::default() };
            let mut w = World::new(sc);
            new_match();
            // The net team's queue of calls (LR 97): one a match.
            let calls = crate::calls::Calls::new();
            // A model's team has its own queue of calls, as the net's has.
            let team_calls: Vec<crate::calls::Calls> = (0..TEAMS.len()).map(|_| crate::calls::Calls::new()).collect();
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..SEATS)
                .map(|i| {
                    if let Some(n) = net.as_ref().filter(|_| teams()[i] == lucky) {
                        return (i as PlayerId, Box::new(crate::net::NetBrain::new(n.clone(), &name(i)).with_calls(calls.clone())) as Box<dyn Brain>);
                    }
                    // The aggro seat stays the aggro bot: a team of its own.
                    let model = if teams()[i] == lucky { trial_model.as_ref() } else { base_model.as_ref() };
                    if let (Some(n), Some(c)) = (model.filter(|_| i != AGGRO), team_calls.get(teams()[i] as usize)) {
                        return (i as PlayerId, Box::new(crate::net::NetBrain::new(n.clone(), &name(i)).with_calls(c.clone())) as Box<dyn Brain>);
                    }
                    let s = if teams()[i] == lucky { trial.clone() } else { base.clone() };
                    (i as PlayerId, with_settings(s, || brain(i, seed ^ i as u64, false)))
                })
                .collect();
            let mut tally = Tally::new(&w);
            crate::meter::reset();
            crate::meter::trial((0..SEATS).filter(|i| teams()[*i] == lucky).fold(0, |m, i| m | 1 << i));
            // Seat-seconds in each new part, the trial team's, by code
            // (status.rs): a rewording of a status cannot move a count.
            use crate::status::{Act, CycleAct, HopAct};
            const WORDS: [&str; 10] = ["base cycle", "relieving", "relieved", "cheaper by boat", "gathering the fleet", "road post", "a mine on the approach", "works for the base", "for the team", "ducking"];
            let part = |a: Act| -> [bool; 10] {
                [
                    matches!(a, Act::Cycle(_)),
                    a == Act::Relieving,
                    a == Act::Relieved,
                    a == Act::Hop(HopAct::CheaperByBoat),
                    matches!(a, Act::Gather | Act::FetchGather),
                    a == Act::RoadPost,
                    a == Act::Cycle(CycleAct::Mine),
                    a == Act::Cycle(CycleAct::Works),
                    a == Act::TeamBridge,
                    a == Act::Duck,
                ]
            };
            let mut words = [0u32; 10];
            let mut rungs = [0u32; 6];
            let mut window: std::collections::HashMap<PlayerId, std::collections::VecDeque<String>> = Default::default();
            let mut seen: std::collections::HashMap<(PlayerId, PlayerId), Tick> = Default::default();
            let mut sightings: std::collections::HashMap<PlayerId, Vec<(Tick, V2)>> = Default::default();
            while !w.over && w.tick < secs * TICK_HZ as Tick {
                let orders = crate::run_bots(&w, &mut bots);
                w.step(&orders);
                tally.hear(&w);
                if acts_on {
                    for e in &w.events {
                        let Event::TankDestroyed { tank, owner, by } = e else { continue };
                        if teams()[*owner as usize] != lucky {
                            continue;
                        }
                        let by_kind = killed_by(&w.events, *tank, *by);
                        let codes = window.remove(owner).unwrap_or_default();
                        let last = codes.back().cloned().unwrap_or_else(|| "(none)".into());
                        if let (Some(k), true) = (by, last == "landfall") {
                            let recent = |q: PlayerId| seen.get(&(q, *k)).map(|t| w.tick.saturating_sub(*t) <= 10 * TICK_HZ as Tick).unwrap_or(false);
                            let me = recent(*owner);
                            let mate = (0..SEATS).any(|q| q as PlayerId != *owner && teams()[q] == lucky && recent(q as PlayerId));
                            let boat = w.tanks.iter().find(|t| t.id == *tank).map(|t| t.pos);
                            let gun = w.tanks.iter().find(|t| t.owner == *k);
                            let beach = bots.iter().find(|(q, _)| q == owner).and_then(|(_, b)| b.beach());
                            if let Some(boat) = boat {
                                let early: Vec<V2> = sightings.get(k).map(|v| v.iter().filter(|(t, _)| (10 * TICK_HZ as Tick..=60 * TICK_HZ as Tick).contains(&w.tick.saturating_sub(*t))).map(|(_, p)| *p).collect()).unwrap_or_default();
                                *known.entry(match () {
                                    _ if early.is_empty() => "not seen 10 to 60 s before",
                                    _ if early.iter().any(|p| p.dist(boat) < DIRECT_RANGE + 4.0) => "seen 10 to 60 s before, then within reach of where the boat died",
                                    _ => "seen 10 to 60 s before, elsewhere",
                                }).or_insert(0) += 1;
                            }
                            if let (Some(boat), Some(gun)) = (boat, gun) {
                                let nan = f32::NAN;
                                gun_at.push((gun.pos.dist(boat), beach.map(|b| gun.pos.dist(b)).unwrap_or(nan), beach.map(|b| boat.dist(b)).unwrap_or(nan), gun.boat));
                            }
                            *landfall_seen.entry(match (me, mate) { (true, true) => "both", (true, false) => "the boat only", (false, true) => "a teammate only", _ => "neither" }).or_insert(0) += 1;
                        }
                        deaths += 1;
                        *died_by.entry(by_kind.clone()).or_insert(0) += 1;
                        *died_both.entry(format!("{:<18} {}", by_kind, last)).or_insert(0) += 1;
                        *died_last.entry(last).or_insert(0) += 1;
                        let mut seen: Vec<&String> = codes.iter().collect();
                        seen.sort();
                        seen.dedup();
                        for c in seen {
                            *died_window.entry(c.clone()).or_insert(0) += 1;
                        }
                    }
                }
                if w.tick % TICK_HZ as Tick == 0 {
                    for (p, b) in &bots {
                        if teams()[*p as usize] == lucky {
                            for (i, on) in part(b.act()).into_iter().enumerate() {
                                words[i] += on as u32;
                            }
                            if let Some(r) = b.rung() {
                                rungs[r.index()] += 1;
                            }
                            if acts_on {
                                let alive = w.tanks.iter().any(|t| t.owner == *p && t.alive);
                                let code = if !alive { "(dead)".to_string() } else if b.rung().is_some() { format!("[ladder] {}", b.act().code()) } else { b.act().code() };
                                if alive {
                                    for t in w.observe(*p).hostile_tanks() {
                                        seen.insert((*p, t.owner), w.tick);
                                        let v = sightings.entry(t.owner).or_default();
                                        if v.last().map(|(at, _)| *at) != Some(w.tick) {
                                            v.push((w.tick, t.pos));
                                        }
                                    }
                                    let q = window.entry(*p).or_default();
                                    q.push_back(code.clone());
                                    if q.len() > 5 {
                                        q.pop_front();
                                    }
                                }
                                *acts.entry(code).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
            let m = crate::meter::read();
            total.seats += m.seats;
            total.thinks += m.thinks;
            total.moves += m.moves;
            total.crossings += m.crossings;
            total.crossings_cached += m.crossings_cached;
            total.plan_clones += m.plan_clones;
            total.nanos += m.nanos;
            for (t, n) in total.postures.iter_mut().zip(m.postures) {
                *t += n;
            }
            total.leaned += m.leaned;
            total.add_dodges(&m);
            total.add_fronts(&m);
            for (t, r) in total.moved.iter_mut().zip(m.moved) {
                for (t, n) in t.iter_mut().zip(r) {
                    *t += n;
                }
            }
            let genomes: Vec<(PlayerId, Genome)> = bots.iter().map(|(p, b)| (*p, Genome::of(&b.settings().cloned().unwrap_or_default(), seed ^ *p as u64, &name(*p as usize), None))).collect();
            let records = tally.records(&w, &map_name, &genomes, &k);
            if ledger {
                crate::ledger::write(&dir, &records).expect("the ledger");
            }
            eprintln!("seed {:x} on {}: trial team {}, over {} at {} s, won by {:?}", seed, map_name, lucky, w.over, w.tick / TICK_HZ as Tick, w.winning_team);
            for t in 0..3u8 {
                let rs: Vec<_> = records.iter().filter(|r| r.outcome.team == Some(t)).collect();
                let sum = |f: &dyn Fn(&crate::ledger::Outcome) -> u32| rs.iter().map(|r| f(&r.outcome)).sum::<u32>();
                let score: f32 = rs.iter().map(|r| r.score).sum::<f32>() / rs.len().max(1) as f32;
                let rewards: f32 = rs.iter().map(|r| crate::ledger::rewards(&r.outcome, &k)).sum::<f32>() / rs.len().max(1) as f32;
                eprintln!(
                    "  team {}{}: score {:.2}, kills {}, deaths {}, revives {}, built {}, road {}, bridge {}, held {:.1}, rewards {:.2} (direct hits {}, pillbox damage {:.1}, pillboxes taken {} neutral {} hostile, defends {}, all neutral {})",
                    t,
                    if t == lucky { " (trial)" } else { "" },
                    score,
                    sum(&|o| o.kills),
                    sum(&|o| o.deaths),
                    sum(&|o| o.revives),
                    sum(&|o| o.built),
                    sum(&|o| o.road_tiles),
                    sum(&|o| o.bridge_tiles),
                    rs.first().map(|r| r.outcome.held_s as f32 / r.seconds.max(1) as f32).unwrap_or(0.0),
                    rewards,
                    sum(&|o| o.direct_hits),
                    rs.iter().map(|r| r.outcome.pillbox_damage).sum::<f32>(),
                    sum(&|o| o.neutral_taken),
                    sum(&|o| o.enemy_taken),
                    sum(&|o| o.defends),
                    if rs.iter().any(|r| r.outcome.all_neutral) { "yes" } else { "no" }
                );
                let e = &mut by_seed[(n - seed0) as usize];
                if t == lucky {
                    trial_score += score;
                    trial_n += 1;
                    e.0 += score;
                    e.1 += 1;
                } else {
                    base_score += score;
                    base_n += 1;
                    e.2 += score;
                    e.3 += 1;
                }
            }
            eprintln!("  trial team, seat-seconds: {}", WORDS.iter().zip(words.iter()).map(|(k, n)| format!("{} {}", k, n)).collect::<Vec<_>>().join(", "));
            eprintln!("  trial team, seat-seconds a rung: {}", rung_line(&rungs));
            for (t, n) in rungs_total.iter_mut().zip(rungs) {
                *t += n;
            }
            eprintln!("  {}", m.line());
        }
        eprintln!("brain_census: trial {:.2}, baseline {:.2} (mean score a seat, {} seeds, {}); {}", trial_score / trial_n.max(1) as f32, base_score / base_n.max(1) as f32, seeds, if fair { "every team in the trial seat" } else { "the trial seat rotated" }, total.line());
        eprintln!("brain_census: trial team, seat-seconds a rung: {}", rung_line(&rungs_total));
        if acts_on {
            let all: u32 = acts.values().sum();
            let mut v: Vec<(&String, &u32)> = acts.iter().collect();
            v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
            let ladder: u32 = acts.iter().filter(|(k, _)| k.starts_with("[ladder]")).map(|(_, n)| n).sum();
            eprintln!("brain_census: trial team's time, {} seat-seconds, {:.1}% on the ladder:", all, 100.0 * ladder as f32 / all.max(1) as f32);
            for (k, n) in v.iter().take(40) {
                eprintln!("  {:>6.2}%  {:>7}  {}", 100.0 * **n as f32 / all.max(1) as f32, n, k);
            }
            let table = |title: &str, m: &std::collections::BTreeMap<String, u32>, top: usize| {
                let mut v: Vec<(&String, &u32)> = m.iter().collect();
                v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
                eprintln!("brain_census: trial team's {} deaths, {}:", deaths, title);
                for (k, n) in v.iter().take(top) {
                    eprintln!("  {:>6.2}%  {:>6}  {}", 100.0 * **n as f32 / deaths.max(1) as f32, n, k);
                }
            };
            table("by what", &died_by, 12);
            table("the code held last", &died_last, 30);
            table("held in the 5 s before (a death counts once a code)", &died_window, 30);
            table("by what and the code held last", &died_both, 30);
            let k: u32 = known.values().sum();
            eprintln!("brain_census: trial team's {} landfall deaths with a killer, known earlier: {}", k, known.iter().map(|(c, v)| format!("{} {} ({:.1}%)", c, v, 100.0 * *v as f32 / k.max(1) as f32)).collect::<Vec<_>>().join(", "));
            let n: u32 = landfall_seen.values().sum();
            if !gun_at.is_empty() {
                let q = |mut v: Vec<f32>| -> String {
                    v.retain(|x| x.is_finite());
                    v.sort_by(|a, b| a.total_cmp(b));
                    if v.is_empty() {
                        return "none".into();
                    }
                    let at = |p: f32| v[((v.len() - 1) as f32 * p).round() as usize];
                    format!("{:.1} / {:.1} / {:.1} (quartiles, {})", at(0.25), at(0.5), at(0.75), v.len())
                };
                let n = gun_at.len() as f32;
                let with_beach: Vec<&(f32, f32, f32, bool)> = gun_at.iter().filter(|g| g.1.is_finite()).collect();
                let covers = with_beach.iter().filter(|g| g.1 < DIRECT_RANGE + 4.0).count();
                let nearer_boat = with_beach.iter().filter(|g| g.0 < g.1).count();
                eprintln!(
                    "brain_census: where the gun stood, {} landfall deaths: killer afloat {:.1}%; a beach chosen {:.1}%; of those, the killer within its range + 4 of the beach {:.1}%, nearer the boat than the beach {:.1}%; killer to boat {}; killer to beach {}; boat to beach {}",
                    gun_at.len(),
                    100.0 * gun_at.iter().filter(|g| g.3).count() as f32 / n,
                    100.0 * with_beach.len() as f32 / n,
                    100.0 * covers as f32 / with_beach.len().max(1) as f32,
                    100.0 * nearer_boat as f32 / with_beach.len().max(1) as f32,
                    q(gun_at.iter().map(|g| g.0).collect()),
                    q(gun_at.iter().map(|g| g.1).collect()),
                    q(gun_at.iter().map(|g| g.2).collect())
                );
            }
            eprintln!("brain_census: trial team's {} landfall deaths with a killer, the killer seen in the 10 s before by: {}", n, landfall_seen.iter().map(|(k, v)| format!("{} {} ({:.1}%)", k, v, 100.0 * *v as f32 / n.max(1) as f32)).collect::<Vec<_>>().join(", "));
        }
        // The seeds are the samples: a seed's matches share its starts,
        // so its difference is one paired reading. Within two standard
        // errors of zero is "inside the noise".
        let d: Vec<f32> = by_seed.iter().map(|&(t, tn, b, bn)| t / tn.max(1) as f32 - b / bn.max(1) as f32).collect();
        let (mean, se) = crate::ledger::mean_se(&d);
        let noise = |mean: f32, se: f32| if se > 0.0 && mean.abs() > 2.0 * se { "outside the noise" } else { "inside the noise" };
        eprintln!("brain_census: difference {:+.2} a seat, standard error {:.2} over {} seeds ({}); a seed: {}", mean, se, d.len(), noise(mean, se), d.iter().map(|x| format!("{:+.2}", x)).collect::<Vec<_>>().join(" "));
        // Each map's own reading, its seeds' differences listed so jobs
        // over other seeds pool by map (scripts/pool.sh).
        if maps.len() > 1 {
            for (i, (name, _)) in maps.iter().enumerate() {
                let dm: Vec<f32> = (seed0..seed0 + seeds).filter(|n| (n % maps.len() as u64) as usize == i).map(|n| d[(n - seed0) as usize]).collect();
                if dm.is_empty() {
                    continue;
                }
                let (mean, se) = crate::ledger::mean_se(&dm);
                eprintln!("brain_census: on {}: difference {:+.2} a seat, standard error {:.2} over {} seeds ({}); a seed: {}", name, mean, se, dm.len(), noise(mean, se), dm.iter().map(|x| format!("{:+.2}", x)).collect::<Vec<_>>().join(" "));
            }
        }
    }

    /// Sixteen seats as the window deals them with `--fleets`, seat 0 on
    /// Auto as a person under the coordination screen would be. Every
    /// team builds at its leader's depot; printed, and asserted for the
    /// two bot teams that live (`FLEET_SECS`, default 360).
    #[test]
    #[ignore]
    fn three_fleets_build() {
        use treadcore_sim::*;
        let seed = std::env::var("FLEET_SEED").ok().and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok()).unwrap_or(0x7EAD_C0DE_u64);
        let names: Vec<(String, bool)> = (0..SEATS).map(|i| (if i == 0 { "user".to_string() } else { name(i) }, i != 0)).collect();
        let limit: Tick = std::env::var("FLEET_LIMIT").ok().and_then(|s| s.parse().ok()).map(|m: Tick| m * 60 * TICK_HZ as Tick).unwrap_or(15 * 60 * TICK_HZ as Tick);
        let sc = Scenario { seed, players: names, teams: Some(teams()), time_limit: limit, ..Scenario::default() };
        let mut w = World::new(sc);
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..SEATS).map(|i| {
            let b: Box<dyn Brain> = if i == 0 { let mut c = Commander::new(1, "user"); c.set_goal(Goal::Auto); Box::new(c) } else { brain(i, seed ^ i as u64, TEAMS[0].contains(&i)) };
            (i as PlayerId, b)
        }).collect();
        let secs: u64 = std::env::var("FLEET_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(360);
        let depots: Vec<V2> = (0..3).map(|t| w.depots.iter().find(|d| d.owner == Some(TEAMS[t][0] as PlayerId)).unwrap().pos).collect();
        let mut deaths: Vec<String> = Vec::new();
        for _ in 0..(secs * TICK_HZ as u64) {
            let orders = crate::run_bots(&w, &mut bots);
            w.step(&orders);
            for e in &w.events {
                if let Event::TankDestroyed { owner, by, .. } = e {
                    let cause = match by { Some(b) => format!("by {} (team {})", name(*b as usize), teams()[*b as usize]), None => "by the swarm or a mine".into() };
                    deaths.push(format!("{:>4}s {} {}", w.tick / TICK_HZ as Tick, if *owner == 0 { "user".to_string() } else { name(*owner as usize) }, cause));
                }
            }
            let every: Tick = std::env::var("FLEET_EVERY").ok().and_then(|s| s.parse().ok()).unwrap_or(60);
            if w.tick % (every * TICK_HZ as Tick) == 0 || w.over {
                for (t, d) in depots.iter().enumerate() {
                    let side = |o: Option<PlayerId>| o.map(|o| teams()[o as usize] == t as TeamId).unwrap_or(false);
                    let boxes = w.pillboxes.iter().filter(|p| side(p.owner) && p.pos.dist(*d) <= 10.0).count();
                    let all = w.pillboxes.iter().filter(|p| side(p.owner)).count();
                    let held = w.depots.iter().filter(|x| side(x.owner)).count();
                    let alive = TEAMS[t].iter().filter(|s| w.tanks[**s].alive).count();
                    let near = |k: Terrain| (-7..=7).flat_map(|dy| (-7..=7).map(move |dx| V2::new(dx as f32, dy as f32))).filter(|o| o.len() <= 6.0 && w.map.at(*d + *o) == k).count();
                    eprintln!("[{:>4}s] team {}: {} alive, {} pillboxes ({} at the base, walls {} roads {}), {} depots; {}", w.tick / TICK_HZ as Tick, t, alive, all, boxes, near(Terrain::Wall), near(Terrain::Road), held, TEAMS[t].iter().filter(|s| w.tanks[**s].alive).map(|s| format!("{}={}", s, bots[*s].1.status().chars().take(32).collect::<String>())).collect::<Vec<_>>().join(" | "));
                }
                eprintln!("        aggro alive {}", w.tanks[AGGRO].alive);
            }
            if w.over {
                eprintln!("MATCH OVER at {} s: team {:?}, winner {:?}", w.tick / TICK_HZ as Tick, w.winning_team, w.winner.map(|p| name(p as usize)));
                break;
            }
        }
        for d in &deaths {
            eprintln!("  died {}", d);
        }
        for t in 1..3 {
            let boxes = w.pillboxes.iter().filter(|p| p.owner.map(|o| teams()[o as usize] == t as TeamId).unwrap_or(false) && p.pos.dist(depots[t]) <= 10.0).count();
            if TEAMS[t].iter().any(|s| w.tanks[*s].alive) {
                assert!(boxes >= 2, "team {} has {} pillboxes at its base", t, boxes);
            }
        }
    }

    /// Trace 1 of PROPOSAL-UPGRADES.md section 6 (goal 5, 11g): supply
    /// counted and nothing bought. The default brain in every seat, the
    /// naval map, `SUPPLY_SECS` (1,800) at most, `SUPPLY_RESPAWNS` (3),
    /// `SUPPLY_SEEDS` (6) a thread each. Each team is paid from outside
    /// the world at the proposal's rates (section 1.1): a live pillbox 1
    /// and a depot 2 every 10 s, a powered structure 1 every 20 s, 10 a
    /// hostile tank destroyed, 5 a hostile pillbox captured. Nothing in
    /// the world reads it, so the match is today's. `cargo test -p
    /// treadcore-brain --release -- --ignored supply_trace --nocapture`
    #[test]
    #[ignore]
    fn supply_trace() {
        use treadcore_sim::*;
        let env = |k: &str, d: u64| std::env::var(k).ok().and_then(|s| s.parse().ok()).unwrap_or(d);
        let (seeds, secs, respawns) = (env("SUPPLY_SEEDS", 6), env("SUPPLY_SECS", 1800), env("SUPPLY_RESPAWNS", 3));
        const SIDES: usize = 4;
        const SOURCES: [&str; 5] = ["pillboxes", "depots", "structures", "kills", "captures"];
        // A match: each side's supply by source, the second it first had
        // 60 and 120 (the cheapest upgrade, and a second level), its
        // supply at each 5 minutes, and how the match ended.
        type Run = (u64, [[u32; 5]; SIDES], [[Option<u32>; 2]; SIDES], Vec<[u32; SIDES]>, u32, Option<TeamId>);
        let runs: Vec<Run> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..seeds)
                .map(|n| {
                    std::thread::Builder::new()
                        .stack_size(256 << 20)
                        .spawn_scoped(sc, move || {
                            let seed = 0x7EAD_C0DE_u64 ^ (n * 0x9E37_79B9);
                            let names: Vec<(String, bool)> = (0..SEATS).map(|i| (name(i), true)).collect();
                            let map = MapSource::Text(include_str!("../../../maps/naval.map").into());
                            let sc = Scenario { seed, players: names, teams: Some(teams()), time_limit: 0, map, respawns: respawns as _, ..Scenario::default() };
                            let mut w = World::new(sc);
                            new_match();
                            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..SEATS).map(|i| (i as PlayerId, brain(i, seed ^ i as u64, false))).collect();
                            let team = teams();
                            let side = |o: Option<PlayerId>| o.map(|p| team[p as usize] as usize);
                            let mut got = [[0u32; 5]; SIDES];
                            let mut first = [[None; 2]; SIDES];
                            let mut every5 = Vec::new();
                            while !w.over && w.tick < secs * TICK_HZ as Tick {
                                let orders = crate::run_bots(&w, &mut bots);
                                w.step(&orders);
                                for e in &w.events {
                                    match e {
                                        Event::TankDestroyed { owner, by: Some(b), .. } if team[*b as usize] != team[*owner as usize] => got[team[*b as usize] as usize][3] += 10,
                                        Event::PillboxCaptured { by, from: Some(f), .. } if team[*by as usize] != team[*f as usize] => got[team[*by as usize] as usize][4] += 5,
                                        _ => {}
                                    }
                                }
                                let t = w.tick / TICK_HZ as Tick;
                                if w.tick % (10 * TICK_HZ as Tick) == 0 {
                                    for p in w.pillboxes.iter().filter(|p| p.alive()) {
                                        if let Some(s) = side(p.owner) {
                                            got[s][0] += 1;
                                        }
                                    }
                                    for d in &w.depots {
                                        if let Some(s) = side(d.owner) {
                                            got[s][1] += 2;
                                        }
                                    }
                                    if w.tick % (20 * TICK_HZ as Tick) == 0 {
                                        for x in w.structures.iter().filter(|x| x.powered) {
                                            if let Some(s) = side(x.owner) {
                                                got[s][2] += 1;
                                            }
                                        }
                                    }
                                }
                                if w.tick % TICK_HZ as Tick == 0 {
                                    for s in 0..SIDES {
                                        let sum: u32 = got[s].iter().sum();
                                        for (k, at) in [60, 120].iter().enumerate() {
                                            if first[s][k].is_none() && sum >= *at {
                                                first[s][k] = Some(t as u32);
                                            }
                                        }
                                    }
                                    if t % 300 == 0 && t > 0 {
                                        every5.push(std::array::from_fn(|s| got[s].iter().sum()));
                                    }
                                }
                            }
                            (n, got, first, every5, (w.tick / TICK_HZ as Tick) as u32, w.winning_team)
                        })
                        .unwrap()
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut total = [0u64; 5];
        let (mut minutes, mut firsts, mut seconds_first) = (0f64, 0u32, Vec::new());
        for (n, got, first, every5, ended, won) in &runs {
            eprintln!("supply_trace: seed {}: over at {} s, won by {:?}", n, ended, won);
            for s in 0..3 {
                let sum: u32 = got[s].iter().sum();
                eprintln!(
                    "  team {}: {} supply, {:.1} a minute ({}); 60 at {}, 120 at {}; by 5 min: {}",
                    s,
                    sum,
                    sum as f64 / (*ended as f64 / 60.0),
                    SOURCES.iter().zip(got[s]).map(|(k, v)| format!("{} {}", k, v)).collect::<Vec<_>>().join(", "),
                    first[s][0].map(|t| format!("{} s", t)).unwrap_or("never".into()),
                    first[s][1].map(|t| format!("{} s", t)).unwrap_or("never".into()),
                    every5.iter().map(|v| v[s].to_string()).collect::<Vec<_>>().join(" ")
                );
                for k in 0..5 {
                    total[k] += got[s][k] as u64;
                }
                minutes += *ended as f64 / 60.0;
                firsts += 1;
                if let Some(t) = first[s][0] {
                    seconds_first.push(t);
                }
            }
        }
        seconds_first.sort();
        let all: u64 = total.iter().sum();
        eprintln!(
            "supply_trace: {} seeds, {:.1} supply a team-minute ({}); 60 reached by {} of {} teams, median at {} s; matches {:.1} min on average",
            seeds,
            all as f64 / minutes,
            SOURCES.iter().zip(total).map(|(k, v)| format!("{} {:.0}%", k, 100.0 * v as f64 / all.max(1) as f64)).collect::<Vec<_>>().join(", "),
            seconds_first.len(),
            firsts,
            seconds_first.get(seconds_first.len() / 2).copied().unwrap_or(0),
            minutes / firsts.max(1) as f64
        );
    }
}
