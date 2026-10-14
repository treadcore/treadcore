//! Chat (PLAYABILITY item 6; SPEC UI-22, UI-23).
//!
//! Enter opens a line on the newswire to everyone, Shift-Enter to the
//! team; Tab steps everyone, the team, the tanks nearby; a line that
//! starts with `@name` (or `@p3`, several allowed) goes to those players
//! only. Enter sends, Esc puts the line away, Backspace takes a
//! character back. The prompt says who will hear it and how many. While the line
//! is open the keys are the line's: input.rs holds the orders and the
//! camera keys (the mouse still drives), and the sound's M is a letter.
//!
//! The world does the rest: `Command::Chat` is an order like any other,
//! the sim strips it to one line of at most CHAT_MAX_CHARS and refuses
//! a second inside a second (UI-23), and a team message reaches only
//! the team (`event_visible`). The window holds a line it may not send
//! yet, rather than have the world refuse it: Enter again sends it.
//!
//! All four of UI-22's recipients are here. "Nearby" is the world's
//! reckoning (CHAT_NEARBY of the sender's tank); the window can count
//! only the tanks it sees, so the prompt says "seen".

use crate::session::{Orders, View};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::time::{Duration, Instant};
use treadcore_sim::{ChatTo, Command, Observation, PlayerId, CHAT_EVERY_TICKS, CHAT_MAX_CHARS, CHAT_NEARBY, TICK_HZ};

/// The line being typed, and to whom.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Typing {
    pub to: Option<ChatTo>,
    pub line: String,
}

impl Typing {
    pub fn to(&self) -> ChatTo {
        self.to.unwrap_or(ChatTo::All)
    }
}

/// What a key did to the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edit {
    Typed,
    Send,
    Cancel,
}

/// One key on an open line. Pure, so the tests can type.
pub fn edit(t: &mut Typing, key: &Key) -> Edit {
    match key {
        Key::Enter => return Edit::Send,
        Key::Escape => return Edit::Cancel,
        Key::Backspace => {
            t.line.pop();
        }
        Key::Tab => {
            t.to = Some(match t.to() {
                ChatTo::All => ChatTo::Team,
                ChatTo::Team => ChatTo::Nearby,
                _ => ChatTo::All,
            })
        }
        Key::Space => push(t, " "),
        Key::Character(s) => push(t, s),
        _ => {}
    }
    Edit::Typed
}

fn push(t: &mut Typing, s: &str) {
    for c in s.chars().filter(|c| !c.is_control()) {
        if t.line.chars().count() >= CHAT_MAX_CHARS {
            return;
        }
        t.line.push(c);
    }
}

/// The players a line's leading `@` words name, and the line without
/// them; Err with the first word that names nobody. A word is a
/// player's name (any case), a unique start of one, or `p` and a seat.
pub fn chosen(line: &str, obs: &Observation) -> Result<Option<(Vec<PlayerId>, String)>, String> {
    let mut who = Vec::new();
    let mut rest = line.trim_start();
    while let Some(word) = rest.strip_prefix('@') {
        let end = word.find(char::is_whitespace).unwrap_or(word.len());
        let name = &word[..end];
        rest = word[end..].trim_start();
        let lower = name.to_lowercase();
        let by_seat = lower.strip_prefix('p').and_then(|n| n.parse::<PlayerId>().ok()).filter(|n| obs.players.iter().any(|p| p.id == *n));
        let exact = obs.players.iter().find(|p| p.name.to_lowercase() == lower).map(|p| p.id);
        let starts: Vec<PlayerId> = obs.players.iter().filter(|p| !lower.is_empty() && p.name.to_lowercase().starts_with(&lower)).map(|p| p.id).collect();
        let id = exact.or(by_seat).or(if starts.len() == 1 { Some(starts[0]) } else { None });
        match id {
            Some(id) if !who.contains(&id) => who.push(id),
            Some(_) => {}
            None => return Err(name.to_string()),
        }
    }
    if who.is_empty() {
        return Ok(None);
    }
    Ok(Some((who, rest.to_string())))
}

/// Who an open line will reach, as the prompt says it.
pub fn audience(t: &Typing, obs: Option<&Observation>) -> String {
    let Some(o) = obs else { return "all".into() };
    if let Ok(Some((who, _))) = chosen(&t.line, o) {
        let names: Vec<String> = who.iter().filter_map(|id| o.players.iter().find(|p| p.id == *id)).map(|p| p.name.clone()).collect();
        return format!("{} ({})", names.join(", "), who.len());
    }
    match t.to() {
        ChatTo::All => format!("all ({})", o.players.len().saturating_sub(1)),
        ChatTo::Team => format!("team ({})", o.players.iter().filter(|p| p.id != o.player && o.allied(p.id)).count()),
        _ => {
            let me = o.me.as_ref().map(|t| t.pos);
            let seen = o.tanks.iter().filter(|t| t.owner != o.player && me.map(|m| t.pos.dist(m) <= CHAT_NEARBY).unwrap_or(false)).count();
            format!("nearby ({} seen)", seen)
        }
    }
}

/// How long the world wants between two messages.
pub fn interval() -> Duration {
    Duration::from_secs_f32(CHAT_EVERY_TICKS as f32 / TICK_HZ as f32)
}

/// After input.rs's systems, so that the Esc that closes the line is not
/// also the Esc that quits, and a letter typed is not also an order.
/// The frame's keys are taken in order: an Enter that opens the line and
/// the letters that follow it in the same frame all land in it.
pub fn chat(mut typed: MessageReader<KeyboardInput>, keys: Res<ButtonInput<KeyCode>>, mut view: ResMut<View>, mut orders: ResMut<Orders>) {
    let playing = view.obs.as_ref().map(|o| !o.over).unwrap_or(false);
    let team = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let mut open = view.typing.take();
    let mut eaten = open.is_some();
    for k in typed.read() {
        if !k.state.is_pressed() {
            continue;
        }
        let Some(t) = open.as_mut() else {
            // With the proposal list open, Enter is the list's.
            if playing && view.menu.is_none() && k.logical_key == Key::Enter {
                open = Some(Typing { to: Some(if team { ChatTo::Team } else { ChatTo::All }), line: String::new() });
                eaten = true;
            }
            continue;
        };
        match edit(t, &k.logical_key) {
            Edit::Typed => {}
            Edit::Cancel => open = None,
            Edit::Send => {
                // `@` words choose the players (UI-22's fourth recipient).
                let (to, text) = match view.obs.as_ref().map(|o| chosen(&t.line, o)) {
                    Some(Err(name)) => {
                        // A name that is nobody: the line stays, to mend.
                        view.refused = Some((Instant::now(), format!("{}: no geeker called @{}", treadcore_sim::chair::Chair::LooseNut.label(), name)));
                        continue;
                    }
                    Some(Ok(Some((who, rest)))) => (ChatTo::players(who), rest.trim().to_string()),
                    _ => (t.to(), t.line.trim().to_string()),
                };
                if text.is_empty() {
                    open = None;
                } else if view.chat_sent.map(|s| s.elapsed() < interval()).unwrap_or(false) {
                    // Too soon: the line stays, marked, for Enter again.
                } else {
                    info!("chat: said to {:?}: {}", to, text);
                    orders.0.push(Command::Chat { text, to });
                    view.chat_sent = Some(Instant::now());
                    open = None;
                }
            }
        }
    }
    view.typing = open;
    view.keys_eaten = eaten;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typing(keys: &[Key]) -> (Typing, Vec<Edit>) {
        let mut t = Typing::default();
        let e = keys.iter().map(|k| edit(&mut t, k)).collect();
        (t, e)
    }

    #[test]
    fn a_line_is_typed_and_sent() {
        let (t, e) = typing(&[Key::Character("h".into()), Key::Character("i".into()), Key::Space, Key::Character("all".into()), Key::Backspace, Key::Enter]);
        assert_eq!(t.line, "hi al");
        assert_eq!(t.to(), ChatTo::All);
        assert_eq!(*e.last().unwrap(), Edit::Send);
    }

    #[test]
    fn tab_switches_the_team_and_esc_cancels() {
        let (t, e) = typing(&[Key::Tab, Key::Character("go".into()), Key::Escape]);
        assert_eq!(t.to(), ChatTo::Team);
        assert_eq!(*e.last().unwrap(), Edit::Cancel);
        let (t, _) = typing(&[Key::Tab, Key::Tab]);
        assert_eq!(t.to(), ChatTo::Nearby);
        let (t, _) = typing(&[Key::Tab, Key::Tab, Key::Tab]);
        assert_eq!(t.to(), ChatTo::All);
    }

    /// `@` words choose players by name, by a unique start of one, or by
    /// seat; a word that is nobody is said back; no `@`, no choice.
    #[test]
    fn at_words_choose_the_players() {
        let w = treadcore_sim::World::new(treadcore_sim::Scenario { swarm_interval: 0, ..treadcore_sim::Scenario::bots(6) });
        let obs = w.observe(0);
        // Seats are named by compass here: 2 south, 3 west, 5 southeast.
        let (who, rest) = chosen("@West @P5 @south meet at the depot", &obs).unwrap().unwrap();
        assert_eq!((who, rest.as_str()), (vec![3, 5, 2], "meet at the depot"), "an exact name beats a start of others");
        assert_eq!(chosen("@nobody hi", &obs), Err("nobody".to_string()));
        assert_eq!(chosen("hello @west", &obs), Ok(None));
        let t = Typing { to: Some(ChatTo::All), line: "@we hi".into() };
        assert_eq!(audience(&t, Some(&obs)), "west (1)");
        let t = Typing { to: Some(ChatTo::Nearby), line: "hi".into() };
        assert!(audience(&t, Some(&obs)).starts_with("nearby ("), "{}", audience(&t, Some(&obs)));
    }

    #[test]
    fn the_line_has_the_worlds_limit_and_no_control_characters() {
        let long = Key::Character("x".repeat(500).as_str().into());
        let (t, _) = typing(&[long, Key::Character("\u{7}".into())]);
        assert_eq!(t.line.chars().count(), CHAT_MAX_CHARS);
        assert!(!t.line.chars().any(|c| c.is_control()));
        assert_eq!(interval(), Duration::from_secs(1));
    }
}
