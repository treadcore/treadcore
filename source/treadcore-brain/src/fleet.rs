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
//! defend base, two rangers on Auto with fortify and turrets off
//! (defend the base, hunt near it, guard it), a wingman on the leader.
//! Seat 0's team assists the person in it: a builder on Auto whose
//! castle goes round the person's depot, a guard on it, two wingmen on
//! the person.

use crate::commander::{Commander, Goal, Rung};
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
    /// Auto at the leader's depot with fortify and turrets off.
    Ranger,
    /// Escort the leader.
    Wing,
    /// A RuleBot alone.
    Aggro,
}

impl Role {
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
    let role = match (person_leads, rank) {
        (_, 0) => Role::Leader,
        (true, 1) => Role::Builder,
        (true, 2) => Role::Guard,
        (true, _) => Role::Wing,
        (false, 1) => Role::Guard,
        (false, 2) | (false, 3) => Role::Ranger,
        (false, _) => Role::Wing,
    };
    (role, leader)
}

/// The seat's name in the roster: team and role.
pub fn name(seat: usize) -> String {
    let (role, _) = role(seat, seat_zero_team(seat));
    match role {
        Role::Aggro => "aggro".into(),
        r => format!("t{}-{}", teams()[seat], r.word()),
    }
}

fn seat_zero_team(seat: usize) -> bool {
    TEAMS[0].contains(&seat)
}

/// The brain for a bot seat. `person_leads`: the team's leader seat is a
/// person's.
pub fn brain(seat: usize, seed: u64, person_leads: bool) -> Box<dyn Brain> {
    let (role, leader) = role(seat, person_leads);
    let name = name(seat);
    let mut c = Commander::new(seed, &name);
    match role {
        Role::Aggro => return Box::new(RuleBot::new(seed, &name)),
        Role::Leader => c.set_goal(Goal::Auto),
        Role::Builder => {
            c.base_owner = Some(leader);
            c.set_goal(Goal::Auto);
        }
        Role::Guard => {
            c.base_owner = Some(leader);
            c.set_goal(Goal::DefendBase);
        }
        Role::Ranger => {
            c.base_owner = Some(leader);
            c.ladder[Rung::Fortify.index()] = false;
            c.ladder[Rung::Turrets.index()] = false;
            c.set_goal(Goal::Auto);
        }
        Role::Wing => {
            let mut w = Commander::wingman(seed, &name, leader);
            w.base_owner = Some(leader);
            return Box::new(w);
        }
    }
    Box::new(c)
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
                    eprintln!("[{:>4}s] team {}: {} alive, {} pillboxes ({} at the base), {} depots; {}", w.tick / TICK_HZ as Tick, t, alive, all, boxes, held, TEAMS[t].iter().filter(|s| w.tanks[**s].alive).map(|s| format!("{}={}", s, bots[*s].1.status().chars().take(32).collect::<String>())).collect::<Vec<_>>().join(" | "));
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
}
