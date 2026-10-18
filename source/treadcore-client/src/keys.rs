//! Key Setup (SPEC UI-18 to UI-21, DLG-15): every action the keyboard
//! gives can be rebound, each takes exactly one key, and the bindings
//! are kept between sessions in a text file.
//!
//! `K` opens Key Setup over the map: up and down pick an action, Enter
//! waits for its new key (Esc gives up waiting), Delete puts every key
//! back to the default, `K` or Esc closes and saves. A key already
//! bound to another action is swapped onto it, so no action is ever
//! left without one or shares one. Alt, Print Screen and Pause cannot
//! be bound (UI-19); nor Esc, Enter, the arrows and Shift, which the
//! window keeps for menus, the chat line and concrete.
//!
//! The file, `$XDG_CONFIG_HOME/treadcore/keys` (or `~/.config/...`, or
//! `--keys FILE`), is a line per action: `interceptor = Digit1`. A
//! line that names no action, no key, a forbidden key or a key taken
//! twice is skipped with a warning and that action keeps its default.
//!
//! A key is bound by its place on the keyboard, not by the letter on
//! it (UI-21): `KeyW` is the key left of `KeyE` on any layout, so the
//! scroll keys keep their shape on AZERTY. The chat line reads the
//! letters themselves.

use crate::session::View;
use bevy::prelude::*;
use std::path::PathBuf;

/// Every rebindable action, in Key Setup's order and groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    TankView,
    PillboxView,
    Alert,
    Events,
    Interceptor,
    Recon,
    Strike,
    Recall,
    Longshot,
    BubbleIn,
    BubbleOut,
    Stop,
    Goal,
    Coordinate,
    Autopilot,
    Respawn,
    Ping,
    Chain,
    Embark,
    Farm,
    Road,
    Wall,
    Pillbox,
    Mine,
    Works,
    WorksKind,
    Return,
    Propose,
    Accept,
    Decline,
    Leave,
    Mute,
    TankStyle,
    Speed,
    KeySetup,
}

use Action::*;

impl Action {
    pub const ALL: [Action; 39] = [ScrollUp, ScrollDown, ScrollLeft, ScrollRight, TankView, PillboxView, Alert, Events, Interceptor, Recon, Strike, Recall, Longshot, BubbleIn, BubbleOut, Stop, Goal, Coordinate, Autopilot, Respawn, Ping, Chain, Embark, Farm, Road, Wall, Pillbox, Mine, Works, WorksKind, Return, Propose, Accept, Decline, Leave, Mute, TankStyle, Speed, KeySetup];

    /// The word in the file and in Key Setup.
    pub fn word(self) -> &'static str {
        match self {
            ScrollUp => "scroll-up",
            ScrollDown => "scroll-down",
            ScrollLeft => "scroll-left",
            ScrollRight => "scroll-right",
            TankView => "tank-view",
            PillboxView => "pillbox-view",
            Alert => "alert",
            Events => "events",
            Interceptor => "interceptor",
            Recon => "recon",
            Strike => "strike",
            Recall => "recall",
            Longshot => "longshot",
            BubbleIn => "bubble-in",
            BubbleOut => "bubble-out",
            Stop => "stop",
            Goal => "goal",
            Coordinate => "coordinate",
            Autopilot => "autopilot",
            Respawn => "respawn",
            Ping => "ping",
            Chain => "chain",
            Embark => "launch a boat",
            Farm => "farm",
            Road => "road",
            Wall => "wall",
            Pillbox => "pillbox",
            Mine => "mine",
            Works => "works",
            WorksKind => "works-kind",
            Return => "return",
            Propose => "propose",
            Accept => "accept",
            Decline => "decline",
            Leave => "leave-side",
            Mute => "mute",
            TankStyle => "tank-style",
            Speed => "game-speed",
            KeySetup => "key-setup",
        }
    }

    /// Key Setup's group heading, given on the first action of each.
    pub fn group(self) -> Option<&'static str> {
        match self {
            ScrollUp => Some("Scroll and views"),
            Interceptor => Some("Drones and the gun"),
            Farm => Some("The squad"),
            Propose => Some("Sides"),
            Mute => Some("The window"),
            _ => None,
        }
    }

    pub fn default_key(self) -> KeyCode {
        match self {
            ScrollUp => KeyCode::KeyW,
            ScrollDown => KeyCode::KeyS,
            ScrollLeft => KeyCode::KeyA,
            ScrollRight => KeyCode::KeyD,
            TankView => KeyCode::KeyF,
            PillboxView => KeyCode::Quote,
            Alert => KeyCode::KeyJ,
            Events => KeyCode::Tab,
            Interceptor => KeyCode::Digit1,
            Recon => KeyCode::Digit2,
            Strike => KeyCode::Digit3,
            Recall => KeyCode::KeyR,
            Longshot => KeyCode::KeyX,
            BubbleIn => KeyCode::BracketLeft,
            BubbleOut => KeyCode::BracketRight,
            Stop => KeyCode::KeyH,
            Goal => KeyCode::KeyG,
            Coordinate => KeyCode::Space,
            Autopilot => KeyCode::KeyO,
            Respawn => KeyCode::KeyV,
            Ping => KeyCode::KeyQ,
            Chain => KeyCode::KeyC,
            Embark => KeyCode::KeyE,
            Farm => KeyCode::Digit4,
            Road => KeyCode::Digit5,
            Wall => KeyCode::Digit6,
            Pillbox => KeyCode::Digit7,
            Mine => KeyCode::Digit8,
            Works => KeyCode::Digit0,
            WorksKind => KeyCode::KeyB,
            Return => KeyCode::Digit9,
            Propose => KeyCode::KeyP,
            Accept => KeyCode::KeyY,
            Decline => KeyCode::KeyN,
            Leave => KeyCode::KeyL,
            Mute => KeyCode::KeyM,
            TankStyle => KeyCode::KeyT,
            Speed => KeyCode::Backquote,
            KeySetup => KeyCode::KeyK,
        }
    }

    fn index(self) -> usize {
        Action::ALL.iter().position(|a| *a == self).unwrap()
    }

    pub fn from_word(w: &str) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| a.word() == w)
    }
}

/// Keys that can be bound: letters, digits, punctuation, the function
/// keys, the editing block, the keypad, Control and Caps Lock.
pub const BINDABLE: [KeyCode; 76] = [
    KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE, KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyI, KeyCode::KeyJ, KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM,
    KeyCode::KeyN, KeyCode::KeyO, KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT, KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY, KeyCode::KeyZ,
    KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
    KeyCode::Quote, KeyCode::BracketLeft, KeyCode::BracketRight, KeyCode::Semicolon, KeyCode::Comma, KeyCode::Period, KeyCode::Slash, KeyCode::Backslash, KeyCode::Minus, KeyCode::Equal, KeyCode::Backquote,
    KeyCode::Tab, KeyCode::Space, KeyCode::Backspace, KeyCode::CapsLock, KeyCode::ControlLeft, KeyCode::ControlRight,
    KeyCode::F1, KeyCode::F2, KeyCode::F3, KeyCode::F4, KeyCode::F5, KeyCode::F6, KeyCode::F7, KeyCode::F8, KeyCode::F9, KeyCode::F10, KeyCode::F11, KeyCode::F12,
    KeyCode::Insert, KeyCode::Home, KeyCode::End, KeyCode::PageUp, KeyCode::PageDown,
    KeyCode::Numpad0, KeyCode::Numpad1, KeyCode::Numpad2, KeyCode::Numpad3, KeyCode::Numpad4, KeyCode::Numpad5,
];

/// Why a key cannot be bound, if it cannot: UI-19's three, and the
/// window's own.
pub fn forbidden(k: KeyCode) -> Option<&'static str> {
    match k {
        KeyCode::AltLeft | KeyCode::AltRight | KeyCode::PrintScreen | KeyCode::Pause => Some("Alt, Print Screen and Pause are never bound"),
        KeyCode::Escape | KeyCode::Enter | KeyCode::NumpadEnter => Some("Esc and Enter are the window's"),
        KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight => Some("the arrows are the menus'"),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some("Shift is concrete's and the team's"),
        k if !BINDABLE.contains(&k) => Some("not a key Key Setup knows"),
        _ => None,
    }
}

/// A key's name in the file: its KeyCode name.
pub fn name(k: KeyCode) -> String {
    format!("{:?}", k)
}

pub fn parse(s: &str) -> Option<KeyCode> {
    BINDABLE.iter().copied().find(|k| name(*k) == s)
}

/// A key as the frame shows it: `1`, `R`, `[`, `Space`.
pub fn label(k: KeyCode) -> String {
    let n = name(k);
    if let Some(c) = n.strip_prefix("Key") {
        return c.into();
    }
    if let Some(d) = n.strip_prefix("Digit") {
        return d.into();
    }
    match k {
        KeyCode::Quote => "'".into(),
        KeyCode::BracketLeft => "[".into(),
        KeyCode::BracketRight => "]".into(),
        KeyCode::Semicolon => ";".into(),
        KeyCode::Comma => ",".into(),
        KeyCode::Period => ".".into(),
        KeyCode::Slash => "/".into(),
        KeyCode::Backslash => "\\".into(),
        KeyCode::Minus => "-".into(),
        KeyCode::Equal => "=".into(),
        KeyCode::Backquote => "`".into(),
        _ => n.replace("Numpad", "Pad").replace("Control", "Ctrl"),
    }
}

/// The bindings: one key per action, no key twice.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct Keys {
    keys: [KeyCode; Action::ALL.len()],
    /// Where they are kept; None keeps them for this session only.
    pub path: Option<PathBuf>,
}

impl Default for Keys {
    fn default() -> Keys {
        Keys { keys: Action::ALL.map(|a| a.default_key()), path: None }
    }
}

impl Keys {
    pub fn key(&self, a: Action) -> KeyCode {
        self.keys[a.index()]
    }

    /// The action a key gives, if any.
    pub fn action(&self, k: KeyCode) -> Option<Action> {
        Action::ALL.iter().copied().find(|a| self.key(*a) == k)
    }

    /// Bind `k` to `a`; the action that had `k` takes `a`'s old key.
    pub fn bind(&mut self, a: Action, k: KeyCode) -> Result<Option<Action>, &'static str> {
        if let Some(why) = forbidden(k) {
            return Err(why);
        }
        let old = self.key(a);
        let other = self.action(k).filter(|o| *o != a);
        if let Some(o) = other {
            self.keys[o.index()] = old;
        }
        self.keys[a.index()] = k;
        Ok(other)
    }

    pub fn to_text(&self) -> String {
        let mut s = String::from("# treadcore key bindings: action = key. Key Setup (K) writes this file;\n# a key is its place on the keyboard (KeyW is left of KeyE on any layout).\n");
        for a in Action::ALL {
            s.push_str(&format!("{} = {}\n", a.word(), name(self.key(a))));
        }
        s
    }

    /// Read a file's text over the defaults; the warnings say what was
    /// skipped and why.
    pub fn from_text(text: &str) -> (Keys, Vec<String>) {
        let mut k = Keys::default();
        let mut warn = Vec::new();
        let mut set: Vec<Action> = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((a, key)) = line.split_once('=') else {
                warn.push(format!("line {}: not `action = key`", n + 1));
                continue;
            };
            let (a, key) = (a.trim(), key.trim());
            let Some(a) = Action::from_word(a) else {
                warn.push(format!("line {}: no action `{}`", n + 1, a));
                continue;
            };
            let Some(code) = parse(key) else {
                warn.push(format!("line {}: `{}` is not a key that can be bound", n + 1, key));
                continue;
            };
            if let Some(o) = set.iter().find(|o| k.key(**o) == code) {
                warn.push(format!("line {}: {} is already {}'s", n + 1, key, o.word()));
                continue;
            }
            let _ = k.bind(a, code);
            set.push(a);
        }
        (k, warn)
    }

    /// The file this session keeps its keys in.
    pub fn default_path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(base.join("treadcore").join("keys"))
    }

    pub fn load(path: Option<PathBuf>) -> Keys {
        let mut keys = match path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) {
            Some(text) => {
                let (k, warn) = Keys::from_text(&text);
                for w in warn {
                    warn!("keys {}: {}", path.as_ref().unwrap().display(), w);
                }
                k
            }
            None => Keys::default(),
        };
        keys.path = path;
        keys
    }

    pub fn save(&self) {
        let Some(p) = self.path.as_ref() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(p, self.to_text()) {
            Ok(()) => info!("keys saved to {}", p.display()),
            Err(e) => error!("keys {}: {}", p.display(), e),
        }
    }
}

/// Key Setup, while open: the action picked, and whether the next key
/// pressed is its new one.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Setup {
    pub row: usize,
    pub waiting: bool,
}

/// Key Setup as the scrollback's place over the map shows it.
pub fn setup_text(keys: &Keys, s: &Setup, note: &str) -> String {
    let mut t = String::from("KEY SETUP   up/down pick, Enter rebind, Delete defaults, K or Esc close\n");
    // A window of rows around the pick: thirty-two actions do not fit.
    let first = s.row.saturating_sub(9).min(Action::ALL.len().saturating_sub(19));
    for (i, a) in Action::ALL.iter().enumerate().skip(first).take(19) {
        if let Some(g) = a.group() {
            t.push_str(&format!("  {}\n", g));
        }
        let mark = if i == s.row { if s.waiting { "?" } else { ">" } } else { " " };
        let key = if i == s.row && s.waiting { "press a key...".to_string() } else { label(keys.key(*a)) };
        t.push_str(&format!(" {} {:<14} {}\n", mark, a.word(), key));
    }
    if !note.is_empty() {
        t.push_str(&format!("\n{}", note));
    }
    t
}

/// After the input systems, so the Esc that closes Key Setup does not
/// also quit, and a key bound is not also an order.
pub fn key_setup(input: Res<ButtonInput<KeyCode>>, mut keys: ResMut<Keys>, mut view: ResMut<View>) {
    if view.typing.is_some() || view.menu.is_some() || view.keys_eaten {
        return;
    }
    let Some(mut s) = view.key_setup else {
        if input.just_pressed(keys.key(KeySetup)) {
            view.key_setup = Some(Setup::default());
            view.key_note = String::new();
        }
        return;
    };
    let pressed: Vec<KeyCode> = input.get_just_pressed().copied().collect();
    if pressed.is_empty() {
        return;
    }
    if s.waiting {
        let k = pressed[0];
        let a = Action::ALL[s.row];
        view.key_note = if k == KeyCode::Escape {
            String::new()
        } else {
            match keys.bind(a, k) {
                Ok(Some(o)) => format!("{} is {}; {} took {}", label(k), a.word(), o.word(), label(keys.key(o))),
                Ok(None) => format!("{} is {}", label(k), a.word()),
                Err(why) => format!("{}: {}", treadcore_sim::chair::Chair::Pebkac.label(), why),
            }
        };
        s.waiting = false;
    } else if pressed.contains(&KeyCode::Escape) || pressed.contains(&keys.key(KeySetup)) {
        keys.save();
        view.key_setup = None;
        view.keys_eaten = true;
        return;
    } else if pressed.contains(&KeyCode::ArrowUp) {
        s.row = (s.row + Action::ALL.len() - 1) % Action::ALL.len();
    } else if pressed.contains(&KeyCode::ArrowDown) {
        s.row = (s.row + 1) % Action::ALL.len();
    } else if pressed.contains(&KeyCode::Enter) || pressed.contains(&KeyCode::NumpadEnter) {
        s.waiting = true;
    } else if pressed.contains(&KeyCode::Delete) {
        let path = keys.path.clone();
        *keys = Keys { path, ..Keys::default() };
        view.key_note = "every key back to its default".into();
    }
    view.key_setup = Some(s);
    view.keys_eaten = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// UI-18: one key an action, none twice, a taken key swapped; UI-19:
    /// Alt, Print Screen, Pause refused; UI-20: the file round-trips and
    /// a bad line keeps the default.
    #[test]
    fn keys_bind_swap_refuse_and_round_trip() {
        let mut k = Keys::default();
        let all: std::collections::BTreeSet<String> = Action::ALL.iter().map(|a| name(k.key(*a))).collect();
        assert_eq!(all.len(), Action::ALL.len(), "the defaults share a key");
        assert!(Action::ALL.iter().all(|a| forbidden(a.default_key()).is_none()), "a default is forbidden");
        assert_eq!(k.bind(Action::Interceptor, KeyCode::KeyR), Ok(Some(Action::Recall)));
        assert_eq!((k.key(Action::Interceptor), k.key(Action::Recall)), (KeyCode::KeyR, KeyCode::Digit1));
        for bad in [KeyCode::AltLeft, KeyCode::PrintScreen, KeyCode::Pause, KeyCode::Escape, KeyCode::ArrowUp] {
            assert!(k.bind(Action::Stop, bad).is_err(), "{:?} was bound", bad);
        }
        let (back, warn) = Keys::from_text(&k.to_text());
        assert!(warn.is_empty(), "{:?}", warn);
        assert_eq!(back.keys, k.keys);
        let (partial, warn) = Keys::from_text("stop = KeyQ\nnonsense\nmute = AltLeft\nfarm = KeyQ\nfly = KeyZ\n");
        assert_eq!(partial.key(Action::Stop), KeyCode::KeyQ);
        assert_eq!(partial.key(Action::Mute), KeyCode::KeyM, "a forbidden key keeps the default");
        assert_eq!(partial.key(Action::Farm), KeyCode::Digit4, "a key taken twice keeps the default");
        assert_eq!(warn.len(), 4, "{:?}", warn);
        for a in Action::ALL {
            assert_eq!(Action::from_word(a.word()), Some(a));
            assert_eq!(parse(&name(a.default_key())), Some(a.default_key()));
        }
        assert_eq!(label(KeyCode::Digit1), "1");
        assert_eq!(label(KeyCode::Quote), "'");
        assert_eq!(label(KeyCode::KeyR), "R");
    }

    /// A keys file from before Coordinate, with Stop on Space as it was:
    /// Stop keeps Space and Coordinate takes H, swapped, not lost.
    #[test]
    fn an_old_keys_file_keeps_stop_on_space() {
        let (k, warn) = Keys::from_text("stop = Space\ngoal = KeyG\n");
        assert!(warn.is_empty(), "{:?}", warn);
        assert_eq!((k.key(Action::Stop), k.key(Action::Coordinate)), (KeyCode::Space, KeyCode::KeyH));
    }

    #[test]
    fn key_setup_shows_the_pick_and_the_wait() {
        let k = Keys::default();
        let t = setup_text(&k, &Setup { row: 8, waiting: false }, "");
        assert!(t.contains("> interceptor") && t.contains("Drones and the gun"), "{}", t);
        let t = setup_text(&k, &Setup { row: Action::ALL.len() - 1, waiting: true }, "");
        assert!(t.contains("? key-setup") && t.contains("press a key"), "{}", t);
    }
}
