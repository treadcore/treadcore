//! Chat (PLAYABILITY item 6; SPEC UI-22, UI-23).
//!
//! Enter opens a line on the newswire to everyone, Shift-Enter to the
//! team; Tab switches between the two while typing; Enter sends, Esc
//! puts the line away, Backspace takes a character back. While the line
//! is open the keys are the line's: input.rs holds the orders and the
//! camera keys (the mouse still drives), and the sound's M is a letter.
//!
//! The world does the rest: `Command::Chat` is an order like any other,
//! the sim strips it to one line of at most CHAT_MAX_CHARS and refuses
//! a second inside a second (UI-23), and a team message reaches only
//! the team (`event_visible`). The window holds a line it may not send
//! yet, rather than have the world refuse it: Enter again sends it.
//!
//! UI-22's other two recipients, the tanks nearby and a chosen list of
//! players, are not here; everyone and the team are what the alliances
//! need first.

use crate::session::{Orders, View};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use std::time::{Duration, Instant};
use treadcore_sim::{ChatTo, Command, CHAT_EVERY_TICKS, CHAT_MAX_CHARS, TICK_HZ};

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
        Key::Tab => t.to = Some(if t.to() == ChatTo::All { ChatTo::Team } else { ChatTo::All }),
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
                let text = t.line.trim().to_string();
                if text.is_empty() {
                    open = None;
                } else if view.chat_sent.map(|s| s.elapsed() < interval()).unwrap_or(false) {
                    // Too soon: the line stays, marked, for Enter again.
                } else {
                    info!("chat: said to {:?}: {}", t.to(), text);
                    orders.0.push(Command::Chat { text, to: t.to() });
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
        assert_eq!(t.to(), ChatTo::All);
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
