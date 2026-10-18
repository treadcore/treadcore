//! Proposals in the window (PLAYABILITY item 7; ROADMAP step 6b).
//!
//! The world has carried proposals since step 6b: Join (I join your
//! side), Recruit (you join mine), Swap (we exchange sides), Merge (my
//! whole side joins yours), answered by the one asked or anyone on
//! their side, under the three-way rule. Until now only bots used them.
//!
//! Answering: the newest proposal to you or your side stands on the
//! newswire with what it means for you and how long it has; **Y** takes
//! it, **N** turns it down.
//!
//! Proposing: **P** opens a list over the map of everyone not on your
//! side. Up and down (or W and S) pick a player, left and right (A and
//! D) pick the terms, and the list says beforehand whether the world
//! would refuse it and why: `Observation::would_refuse` reads the same
//! rule the world applies, from the teams every player can see. Enter
//! proposes; **L** leaves your side; Esc or P closes the list. While it
//! is open its keys are its own, as a chat line's are.

use crate::session::{Orders, View};
use bevy::prelude::*;
use treadcore_sim::{AllianceCmd, Command, Observation, PlayerId, PlayerView, Proposal, Terms, DT};

pub const TERMS: [Terms; 4] = [Terms::Join, Terms::Recruit, Terms::Swap, Terms::Merge];

/// The proposal list, open.
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub row: usize,
    pub terms: usize,
}

/// Everyone a proposal can go to: not on my side, by side then seat.
pub fn targets(obs: &Observation) -> Vec<&PlayerView> {
    let mut v: Vec<&PlayerView> = obs.players.iter().filter(|p| !obs.allied(p.id)).collect();
    v.sort_by_key(|p| (p.team, p.id));
    v
}

/// The newest proposal this player may answer: to it or its side, from
/// someone who is not on it.
pub fn answerable(obs: &Observation) -> Option<&Proposal> {
    obs.proposals.iter().filter(|p| obs.allied(p.to) && !obs.allied(p.from)).max_by_key(|p| p.id)
}

fn name(obs: &Observation, p: PlayerId) -> String {
    if p == obs.player {
        return "you".into();
    }
    obs.players.iter().find(|x| x.id == p).map(|x| x.name.clone()).unwrap_or_else(|| format!("p{}", p))
}

/// What proposing `terms` to `to` would do, said from the proposer's side.
pub fn what_it_does(obs: &Observation, to: PlayerId, terms: Terms) -> String {
    let n = name(obs, to);
    match terms {
        Terms::Join => format!("you join {}'s side", n),
        Terms::Recruit => format!("{} joins your side", n),
        Terms::Swap => format!("you and {} swap sides", n),
        Terms::Merge => format!("your whole side joins {}'s", n),
    }
}

fn seconds_left(obs: &Observation, p: &Proposal) -> u64 {
    (p.expires.saturating_sub(obs.tick) as f32 * DT).ceil() as u64
}

/// A proposal made to me or my side, as the newswire says it.
pub fn offer(obs: &Observation, p: &Proposal) -> String {
    let from = name(obs, p.from);
    let to = if p.to == obs.player { "you".to_string() } else { format!("{} (your side)", name(obs, p.to)) };
    let what = match p.terms {
        Terms::Join => format!("{} asks to join {}", from, to),
        Terms::Recruit => format!("{} asks {} to join their side", from, to),
        Terms::Swap => format!("{} offers {} a swap of sides", from, to),
        Terms::Merge => format!("{}'s whole side offers to join {}", from, to),
    };
    format!("{}: Y yes, N no ({} s)", what, seconds_left(obs, p))
}

fn word(t: Terms) -> &'static str {
    match t {
        Terms::Join => "join",
        Terms::Recruit => "recruit",
        Terms::Swap => "swap",
        Terms::Merge => "merge",
    }
}

/// The newswire's line for a proposal made or resolved, by name.
pub fn told(obs: &Observation, from: PlayerId, to: PlayerId, terms: Terms, outcome: Option<(bool, Option<PlayerId>, Option<&str>)>) -> String {
    let (f, t) = (name(obs, from), name(obs, to));
    let whose = if from == obs.player { "your".to_string() } else { format!("{}'s", f) };
    match outcome {
        None => format!("{} {} {} to {}", f, if from == obs.player { "propose" } else { "proposes" }, word(terms), t),
        Some((true, _, _)) => format!("{} {} to {}: accepted", whose, word(terms), t),
        Some((false, _, Some(why))) => format!("{} {} to {}: fell through ({})", whose, word(terms), t, why),
        Some((false, Some(by), None)) => format!("{} {} to {}: declined by {}", whose, word(terms), t, name(obs, by)),
        Some((false, None, None)) => format!("{} {} to {}: lapsed", whose, word(terms), t),
    }
}

/// The roster's line for a standing proposal.
pub fn standing(obs: &Observation, p: &Proposal) -> String {
    let terms = word(p.terms);
    let fit = |s: String| s.chars().take(8).collect::<String>();
    format!("{} > {} {} {}s{}", fit(name(obs, p.from)), fit(name(obs, p.to)), terms, seconds_left(obs, p), if answerable(obs).map(|a| a.id == p.id).unwrap_or(false) { " Y/N" } else { "" })
}

/// The list, as drawn over the map.
pub fn menu_text(obs: &Observation, m: &Menu) -> String {
    let mut s = String::from("PROPOSE\nup/down who, left/right what\nEnter proposes, L leaves your side\nEsc or P closes\n\n");
    let ts = targets(obs);
    if ts.is_empty() {
        s.push_str("Everyone is on your side.\n");
        return s;
    }
    let terms = TERMS[m.terms % TERMS.len()];
    for (i, p) in ts.iter().enumerate() {
        let here = i == m.row % ts.len();
        s.push_str(&format!("{} side {:<2} {:<12} {:>4}{}\n", if here { ">" } else { " " }, p.team, p.name.chars().take(12).collect::<String>(), p.score, if p.alive { "" } else { "  dead" }));
    }
    let to = ts[m.row % ts.len()].id;
    s.push('\n');
    for (i, t) in TERMS.iter().enumerate() {
        s.push_str(&format!("{}{:?}{}  ", if i == m.terms % TERMS.len() { "[" } else { " " }, t, if i == m.terms % TERMS.len() { "]" } else { " " }));
    }
    s.push_str(&format!("\n{}\n", what_it_does(obs, to, terms)));
    match obs.would_refuse(to, terms) {
        Some(why) => s.push_str(&format!("the world would refuse it:\n  {}\n", treadcore_sim::chair::tagged(why))),
        None => s.push_str("Enter proposes it; it stands 30 s\n"),
    }
    s
}

/// What a key does with the list open. Pure, for the tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Close,
    Up,
    Down,
    Left,
    Right,
    Propose,
    Leave,
}

pub fn act(keys: &ButtonInput<KeyCode>, bind: &crate::keys::Keys) -> Option<Act> {
    use crate::keys::Action as A;
    let p = |k| keys.just_pressed(k);
    Some(if p(KeyCode::Escape) || p(bind.key(A::Propose)) {
        Act::Close
    } else if p(KeyCode::ArrowUp) || p(bind.key(A::ScrollUp)) {
        Act::Up
    } else if p(KeyCode::ArrowDown) || p(bind.key(A::ScrollDown)) {
        Act::Down
    } else if p(KeyCode::ArrowLeft) || p(bind.key(A::ScrollLeft)) {
        Act::Left
    } else if p(KeyCode::ArrowRight) || p(bind.key(A::ScrollRight)) {
        Act::Right
    } else if p(KeyCode::Enter) || p(KeyCode::NumpadEnter) {
        Act::Propose
    } else if p(bind.key(A::Leave)) {
        Act::Leave
    } else {
        return None;
    })
}

/// Apply an act to the open list: the order it gives, if any, and
/// whether the list stays open.
pub fn apply(obs: &Observation, m: &mut Menu, a: Act) -> (Option<Command>, bool) {
    let n = targets(obs).len().max(1);
    match a {
        Act::Close => (None, false),
        Act::Up => {
            m.row = (m.row + n - 1) % n;
            (None, true)
        }
        Act::Down => {
            m.row = (m.row + 1) % n;
            (None, true)
        }
        Act::Left => {
            m.terms = (m.terms + TERMS.len() - 1) % TERMS.len();
            (None, true)
        }
        Act::Right => {
            m.terms = (m.terms + 1) % TERMS.len();
            (None, true)
        }
        Act::Propose => {
            let Some(to) = targets(obs).get(m.row % n).map(|p| p.id) else { return (None, true) };
            let terms = TERMS[m.terms % TERMS.len()];
            if obs.would_refuse(to, terms).is_some() {
                // The list already says why; it stays open to pick again.
                return (None, true);
            }
            (Some(Command::Propose { to, terms }), false)
        }
        Act::Leave => {
            let alone = obs.players.iter().filter(|p| p.team == obs.my_team()).count() <= 1;
            if alone {
                return (None, true);
            }
            (Some(Command::Alliance(AllianceCmd::Leave)), false)
        }
    }
}

/// After chat's system: a chat line open means these keys are its, and
/// the Enter that sends a line is not also one that proposes.
pub fn proposals(keys: Res<ButtonInput<KeyCode>>, bind: Res<crate::keys::Keys>, mut view: ResMut<View>, mut orders: ResMut<Orders>) {
    use crate::keys::Action as A;
    // A key typed into a chat line this frame (even one closed by the
    // Esc that came with it) is the line's, not the list's; and Key
    // Setup's keys are its own.
    if view.typing.is_some() || view.keys_eaten || view.key_setup.is_some() || view.tune.is_some() {
        return;
    }
    let mut menu = view.menu.take();
    let Some(obs) = view.obs.as_ref() else { return };
    if obs.over {
        return;
    }
    let mut give: Option<Command> = None;
    match menu.as_mut() {
        None => {
            if keys.just_pressed(bind.key(A::Propose)) {
                debug!("proposal list open");
                menu = Some(Menu { row: 0, terms: 0 });
            } else if keys.just_pressed(bind.key(A::Accept)) || keys.just_pressed(bind.key(A::Decline)) {
                if let Some(p) = answerable(obs) {
                    give = Some(Command::Answer { proposal: p.id, accept: keys.just_pressed(bind.key(A::Accept)) });
                }
            }
        }
        Some(m) => {
            if let Some(a) = act(&keys, &bind) {
                let (cmd, open) = apply(obs, m, a);
                debug!("proposal list: {:?}, open {}", a, open);
                give = cmd;
                if !open {
                    menu = None;
                }
            }
        }
    }
    if let Some(c) = give {
        info!("proposal: {:?}", c);
        orders.0.push(c);
    }
    view.menu = menu;
}

#[cfg(test)]
mod tests {
    use super::*;
    use treadcore_sim::{Order, Scenario, World};

    fn world(teams: usize) -> World {
        // 0: everyone alone (with_teams(0) would be one side of all).
        let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(9) };
        let mut w = World::new(if teams > 0 { sc.with_teams(teams) } else { sc });
        w.pillboxes.clear();
        w
    }

    #[test]
    fn the_list_offers_only_what_the_world_would_take() {
        // Three sides of three: a Join leaves three sides, three/four/two:
        // allowed; a Merge leaves two: refused, and the list stays open.
        let w = world(3);
        let obs = w.observe(0);
        let ts = targets(&obs);
        assert_eq!(ts.len(), 6, "everyone not on my side");
        assert!(ts.iter().all(|p| !obs.allied(p.id)));
        let mut m = Menu { row: 0, terms: 0 };
        let (cmd, open) = apply(&obs, &mut m, Act::Propose);
        assert!(matches!(cmd, Some(Command::Propose { terms: Terms::Join, .. })) && !open);
        let mut m = Menu { row: 0, terms: 3 };
        assert!(menu_text(&obs, &m).contains("would refuse it:\n  KB-TO-CHAIR: fewer than three sides"));
        assert_eq!(apply(&obs, &mut m, Act::Propose), (None, true));
        // Up from the top wraps to the bottom; left from Join is Merge.
        let mut m = Menu { row: 0, terms: 0 };
        apply(&obs, &mut m, Act::Up);
        apply(&obs, &mut m, Act::Left);
        assert_eq!((m.row, m.terms), (5, 3));
    }

    #[test]
    fn the_newest_offer_to_my_side_is_the_one_answered() {
        let mut w = world(3);
        let (a, b) = (w.team_of(0).unwrap(), w.team_of(1).unwrap());
        assert_ne!(a, b);
        // Seat 1 asks seat 0 to join it; then seat 2 asks seat 0's teammate.
        let mate = (1..9u8).find(|p| w.team_of(*p) == Some(a)).unwrap();
        let other = (2..9u8).find(|p| w.team_of(*p) != Some(a) && w.team_of(*p) != Some(b)).unwrap();
        w.step(&[Order { player: 1, cmd: Command::Propose { to: 0, terms: Terms::Recruit } }]);
        w.step(&[Order { player: other, cmd: Command::Propose { to: mate, terms: Terms::Join } }]);
        let obs = w.observe(0);
        let p = answerable(&obs).expect("an offer stands");
        assert_eq!(p.from, other, "the newest, to my side");
        assert!(offer(&obs, p).contains("Y yes, N no"));
        // Nothing of mine is answerable by me.
        let mine = w.observe(1);
        assert!(answerable(&mine).map(|p| p.from != 1).unwrap_or(true));
    }

    #[test]
    fn leaving_needs_a_side_to_leave() {
        let w = world(0);
        let obs = w.observe(0);
        let mut m = Menu { row: 0, terms: 0 };
        assert_eq!(apply(&obs, &mut m, Act::Leave), (None, true));
        let w = world(3);
        let obs = w.observe(0);
        assert_eq!(apply(&obs, &mut m, Act::Leave), (Some(Command::Alliance(AllianceCmd::Leave)), false));
    }
}
