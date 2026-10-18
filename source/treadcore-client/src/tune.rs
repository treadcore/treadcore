//! The sliders (backlog 15, roadmap 12c and 12h): on the coordination
//! screen `U` opens the weights page, a row for every capability and
//! every weight of `treadcore-brain::weights`. Up and down pick a row,
//! left and right move it a step (a capability: off and on), Delete
//! puts the row back to its default, Enter saves, `U` or Esc closes.
//!
//! The page is for the player's own team: the tank under the goal and,
//! when this window hosts, the bots of my side, given the new settings
//! at once and again at each new match (`Brain::set_settings`). A bot
//! keeps its target and its team's calls through a change. Enter keeps
//! the team's settings in `$XDG_CONFIG_HOME/treadcore/weights` (or
//! `~/.config/...`), read for my team at the next start. `--weights
//! FILE` is apart from it: that file is every bot's, as before, and the
//! page starts from it when no team file has been saved.

use crate::goal::Pilot;
use crate::session::{Session, SessionRes, View};
use bevy::prelude::*;
use std::path::PathBuf;
use treadcore_brain::weights::{Caps, Settings, W};

pub struct TunePlugin;

impl Plugin for TunePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Tuning::load(team_path())).add_systems(Update, tune.after(crate::keys::key_setup));
    }
}

/// Rows of the page shown at once.
const SHOWN: usize = 18;

#[derive(Resource)]
pub struct Tuning {
    pub s: Settings,
    pub open: bool,
    pub row: usize,
    pub note: String,
    /// Where Enter saves.
    path: Option<PathBuf>,
    /// The settings changed since the team was last given them.
    dirty: bool,
    /// The match the team was last given them in: (round, bots).
    given: Option<(u64, usize)>,
}

/// One row: a capability or a weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Row {
    Cap(&'static str, Caps),
    Weight(W),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Act {
    Up,
    Down,
    Less,
    More,
    Reset,
    Save,
    Close,
}

/// The team's file beside the keys file.
pub fn team_path() -> Option<PathBuf> {
    crate::keys::Keys::default_path().map(|p| p.with_file_name("weights"))
}

impl Tuning {
    /// The team's saved settings, or every bot's (`--weights`, else the
    /// defaults) when none are saved or the file does not read.
    pub fn load(path: Option<PathBuf>) -> Tuning {
        let mut note = String::new();
        let s = match path.as_ref().map(|p| (p, std::fs::read_to_string(p))) {
            Some((p, Ok(text))) => match Settings::parse(&text) {
                Ok(s) => {
                    info!("weights for my team from {}", p.display());
                    s
                }
                Err(e) => {
                    warn!("weights {}: {}", p.display(), e);
                    note = format!("{}: {}; every bot's settings instead", p.display(), e);
                    treadcore_brain::fleet::settings()
                }
            },
            _ => treadcore_brain::fleet::settings(),
        };
        Tuning { s, open: false, row: 0, note, path, dirty: true, given: None }
    }

    pub fn rows() -> impl Iterator<Item = Row> {
        Caps::NAMES.iter().map(|(n, c)| Row::Cap(n, *c)).chain(W::ALL.iter().map(|w| Row::Weight(*w)))
    }

    fn count() -> usize {
        Caps::NAMES.len() + W::ALL.len()
    }

    fn at(&self) -> Row {
        Tuning::rows().nth(self.row).unwrap()
    }

    /// A key on the open page: whether it stays open.
    pub fn act(&mut self, a: Act) -> bool {
        let n = Tuning::count();
        let before = self.s.clone();
        match (a, self.at()) {
            (Act::Up, _) => self.row = (self.row + n - 1) % n,
            (Act::Down, _) => self.row = (self.row + 1) % n,
            (Act::Less, Row::Cap(_, c)) => self.s.caps = self.s.caps.without(c),
            (Act::More, Row::Cap(_, c)) => self.s.caps = self.s.caps.with(c),
            (Act::Less, Row::Weight(w)) => self.s.nudge(w, -1),
            (Act::More, Row::Weight(w)) => self.s.nudge(w, 1),
            (Act::Reset, Row::Cap(_, c)) => self.s.caps = if Caps::BASELINE.has(c) { self.s.caps.with(c) } else { self.s.caps.without(c) },
            (Act::Reset, Row::Weight(w)) => self.s.w[w] = w.default_value(),
            (Act::Save, _) => self.note = self.save(),
            (Act::Close, _) => return false,
        }
        if self.s != before {
            self.dirty = true;
            self.note.clear();
        }
        true
    }

    fn save(&self) -> String {
        let Some(p) = self.path.as_ref() else { return "nowhere to save: no HOME".into() };
        let text = format!("# my team's weights (the coordination screen, U)\n{}", self.s.to_text());
        match p.parent().map(std::fs::create_dir_all).unwrap_or(Ok(())).and_then(|_| std::fs::write(p, text)) {
            Ok(()) => format!("saved to {}", p.display()),
            Err(e) => format!("not saved, {}: {}", p.display(), e),
        }
    }

    /// The page: the rows around the one picked, its doc, and whose.
    pub fn text(&self, whose: &str) -> String {
        let mut t = String::from("WEIGHTS  up/down: pick  left/right: move\n  Delete: default  Enter: save  U: back\n");
        t.push_str(&format!("for {}\n", whose));
        let n = Tuning::count();
        let first = self.row.saturating_sub(SHOWN / 2).min(n.saturating_sub(SHOWN));
        for (i, r) in Tuning::rows().enumerate().skip(first).take(SHOWN) {
            let mark = if i == self.row { ">" } else { " " };
            let line = match r {
                Row::Cap(name, c) => {
                    let on = self.s.caps.has(c);
                    format!("{} {} {:<16} [{}]", mark, if on != Caps::BASELINE.has(c) { "*" } else { " " }, name, if on { "on " } else { "off" })
                }
                Row::Weight(w) => {
                    let (v, d) = (self.s.w[w], w.default_value());
                    format!("{} {} {:<16} {:>8} {}", mark, if v != d { "*" } else { " " }, w.name(), trim(v), bar(v, d))
                }
            };
            t.push_str(&line);
            t.push('\n');
        }
        let doc = match self.at() {
            Row::Cap(_, c) => c.doc().to_string(),
            Row::Weight(w) => format!("{} (default {}, step {})", w.doc(), trim(w.default_value()), trim(w.step())),
        };
        t.push_str(&format!("\n{}\n", doc));
        if !self.note.is_empty() {
            t.push_str(&format!("{}\n", self.note));
        }
        t
    }
}

/// A number as short as it reads.
fn trim(v: f32) -> String {
    let s = format!("{:.4}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A slider of ten from nothing to twice the default, the default at
/// the bar.
fn bar(v: f32, d: f32) -> String {
    let top = if d > 0.0 { 2.0 * d } else { 1.0 };
    let k = ((v / top * 10.0).round() as usize).min(10);
    let mut b: Vec<char> = (0..10).map(|i| if i < k { '#' } else { '-' }).collect();
    b.insert(5, '|');
    format!("[{}]{}", b.into_iter().collect::<String>(), if v > top { "+" } else { "" })
}

/// After Key Setup, so the Esc that closes the page does not also
/// close the coordination screen (the input systems ran with the page
/// still open and its keys taken).
fn tune(keys: Res<ButtonInput<KeyCode>>, bind: Res<crate::keys::Keys>, mut view: ResMut<View>, mut tuning: ResMut<Tuning>, mut pilot: ResMut<Pilot>, mut session: Option<ResMut<SessionRes>>) {
    use crate::keys::Action as A;
    let free = view.typing.is_none() && view.menu.is_none() && view.key_setup.is_none() && !view.keys_eaten;
    if free && !tuning.open && pilot.coord && keys.just_pressed(bind.key(A::Tune)) {
        tuning.open = true;
        view.keys_eaten = true;
    } else if free && tuning.open {
        let p = |k: KeyCode| keys.just_pressed(k);
        let act = if p(KeyCode::Escape) || p(bind.key(A::Tune)) {
            Some(Act::Close)
        } else if p(KeyCode::ArrowUp) || p(bind.key(A::ScrollUp)) {
            Some(Act::Up)
        } else if p(KeyCode::ArrowDown) || p(bind.key(A::ScrollDown)) {
            Some(Act::Down)
        } else if p(KeyCode::ArrowLeft) || p(bind.key(A::ScrollLeft)) {
            Some(Act::Less)
        } else if p(KeyCode::ArrowRight) || p(bind.key(A::ScrollRight)) {
            Some(Act::More)
        } else if p(KeyCode::Delete) {
            Some(Act::Reset)
        } else if p(KeyCode::Enter) || p(KeyCode::NumpadEnter) {
            Some(Act::Save)
        } else {
            None
        };
        if let Some(a) = act {
            tuning.open = tuning.act(a);
            view.keys_eaten = true;
        }
    }
    // The coordination screen gone, the page goes with it.
    if !pilot.coord {
        tuning.open = false;
    }
    // My team: the tank under the goal, and my side's bots when this
    // window hosts them; again at each new match.
    let hosted = match session.as_deref_mut() {
        Some(SessionRes(Session::Hosting { bots, .. })) => Some(bots),
        _ => None,
    };
    let deal = hosted.as_ref().map(|b| (view.round, b.len()));
    if tuning.dirty || deal != tuning.given {
        let s = tuning.s.clone();
        pilot.brain.set_settings(s.clone());
        if let (Some(bots), Some(obs)) = (hosted, view.obs.as_ref()) {
            let mine = obs.my_team();
            for (id, b) in bots.iter_mut() {
                if obs.players.iter().any(|p| p.id == *id && p.team == mine) {
                    b.set_settings(s.clone());
                }
            }
        }
        tuning.dirty = false;
        tuning.given = deal;
    }
    let whose = if deal.is_some() { "my team: my tank and my side's bots" } else { "my tank (the host has the bots)" };
    let page = tuning.open.then(|| tuning.text(whose));
    if view.tune != page {
        view.tune = page;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> Tuning {
        Tuning { s: Settings::default(), open: true, row: 0, note: String::new(), path: None, dirty: false, given: None }
    }

    #[test]
    fn the_page_moves_a_weight_and_puts_it_back() {
        let mut t = page();
        let alpha = Tuning::rows().position(|r| r == Row::Weight(W::Alpha)).unwrap();
        t.row = alpha;
        t.act(Act::More);
        assert_eq!(t.s.w[W::Alpha], 0.45);
        assert!(t.dirty, "the team is given it");
        assert!(t.text("me").contains("> * alpha                0.45 [#####|#----]"), "{}", t.text("me"));
        t.act(Act::Reset);
        assert_eq!(t.s, Settings::default());
        assert!(t.act(Act::Up) && t.row == alpha - 1);
        assert!(!t.act(Act::Close));
    }

    #[test]
    fn the_page_switches_a_capability_and_wraps() {
        let mut t = page();
        t.act(Act::Up);
        assert_eq!(t.row, Tuning::count() - 1, "up from the top is the last row");
        t.act(Act::Down);
        assert_eq!(t.at(), Row::Cap("kite", Caps::KITE));
        t.act(Act::Less);
        assert!(!t.s.caps.has(Caps::KITE));
        assert!(t.text("me").contains("> * kite             [off]"));
        t.act(Act::Reset);
        assert!(t.s.caps.has(Caps::KITE));
        assert_eq!(t.act(Act::Save), true);
        assert_eq!(t.note, "nowhere to save: no HOME");
    }

    #[test]
    fn a_saved_page_reads_back() {
        let dir = std::env::temp_dir().join(format!("treadcore-tune-{}", std::process::id()));
        let path = dir.join("weights");
        let mut t = Tuning { path: Some(path.clone()), ..page() };
        t.s.caps = t.s.caps.with(Caps::ESCORT);
        t.s.nudge(W::PostSpacing, 2);
        t.act(Act::Save);
        assert!(t.note.starts_with("saved to"), "{}", t.note);
        let back = Tuning::load(Some(path));
        assert_eq!(back.s, t.s);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn the_bar_marks_the_default_in_the_middle() {
        assert_eq!(bar(1.0, 1.0), "[#####|-----]");
        assert_eq!(bar(0.0, 1.0), "[-----|-----]");
        assert_eq!(bar(3.0, 1.0), "[#####|#####]+");
        assert_eq!(trim(0.75), "0.75");
        assert_eq!(trim(120.0), "120");
    }
}
