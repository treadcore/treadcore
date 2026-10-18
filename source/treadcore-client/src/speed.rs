//! Game speed (Paul, 17 October: "can we have a 4x speed adjuster. it
//! will speed up (the server and all clients), if we skip frames then
//! reduce speed").
//!
//! The host's world steps from `FixedUpdate`; at speed `s` its timestep
//! is `1 / (TICK_HZ * s)`, so the world, the bots and the match clock all
//! run `s` times as fast. `` ` `` (Key Setup: game-speed) steps the
//! host's wanted speed 1x, 2x, 4x and back; `--speed N` or `SPEED=N`
//! starts there. A client does not choose: it measures how fast the
//! host's ticks arrive and steps its own prediction at that rate, so
//! every window keeps up with the host without a wire change.
//!
//! The governor: when the host falls behind (fewer ticks than the speed
//! asks for, because frames ran long and Bevy dropped the time past
//! `max_delta`: skipped frames of the world), the speed steps down a
//! notch (4x to 2x to 1x). The frame rate is logged, not judged: the
//! guest VM's three windows draw 4 a second at 1x and at 4x alike, and
//! the world kept 3.76x of 4x through it. After `RETRY` of keeping up
//! with room to spare it tries the next notch up again, never above
//! what was asked for.

use crate::keys::{Action, Keys};
use crate::session::{Session, SessionRes, View};
use bevy::prelude::*;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use treadcore_sim::{Tick, TICK_HZ};

/// The notches.
pub const SPEEDS: [f32; 3] = [1.0, 2.0, 4.0];
/// Measured over this long.
const WINDOW: Duration = Duration::from_secs(3);
/// Below this share of the asked tick rate, the host is behind.
const BEHIND: f32 = 0.85;
/// No judging this long after a start or a change: the first frames
/// (pipelines compiling, the lobby) are slow at any speed.
const SETTLE: Duration = Duration::from_secs(10);
/// The rate and frame rate go to the log this often.
const REPORT: Duration = Duration::from_secs(30);
/// Kept up this long, try a notch up.
const RETRY: Duration = Duration::from_secs(30);

#[derive(Resource)]
pub struct Speed {
    /// What was asked for (the key, `--speed`).
    pub want: f32,
    /// What runs now: the governor's notch on the host, the host's
    /// measured rate on a client.
    pub now: f32,
    /// (when, the world's tick) samples over `WINDOW`.
    ticks: VecDeque<(Instant, Tick)>,
    /// Frame times over `WINDOW`.
    frames: VecDeque<(Instant, f32)>,
    /// Since when the host kept up at `now`.
    easy_since: Option<Instant>,
    /// Not judged before this (`SETTLE`).
    settled: Option<Instant>,
    /// The last log line.
    reported: Option<Instant>,
}

impl Speed {
    pub fn new(want: f32) -> Speed {
        let want = Self::notch(want);
        Speed { want, now: want, ticks: VecDeque::new(), frames: VecDeque::new(), easy_since: None, settled: None, reported: None }
    }

    /// The nearest notch at or under `s`.
    pub fn notch(s: f32) -> f32 {
        SPEEDS.iter().copied().filter(|n| *n <= s + 0.01).last().unwrap_or(1.0)
    }

    fn down(s: f32) -> f32 {
        SPEEDS.iter().copied().filter(|n| *n < s - 0.01).last().unwrap_or(1.0)
    }

    fn up(s: f32) -> f32 {
        SPEEDS.iter().copied().find(|n| *n > s + 0.01).unwrap_or(s)
    }

    fn sample(&mut self, at: Instant, tick: Tick, frame: f32) {
        self.ticks.push_back((at, tick));
        self.frames.push_back((at, frame));
        while self.ticks.front().map(|(t, _)| at.duration_since(*t) > WINDOW).unwrap_or(false) {
            self.ticks.pop_front();
        }
        while self.frames.front().map(|(t, _)| at.duration_since(*t) > WINDOW).unwrap_or(false) {
            self.frames.pop_front();
        }
    }

    /// Ticks a second over the window, as a speed (1.0 is TICK_HZ),
    /// once the window is nearly full.
    fn rate(&self) -> Option<f32> {
        let (a, b) = (self.ticks.front()?, self.ticks.back()?);
        let secs = b.0.duration_since(a.0).as_secs_f32();
        if secs < WINDOW.as_secs_f32() * 0.8 || b.1 < a.1 {
            return None;
        }
        Some((b.1 - a.1) as f32 / secs / TICK_HZ as f32)
    }

    fn fps(&self) -> Option<f32> {
        let n = self.frames.len();
        let total: f32 = self.frames.iter().map(|(_, f)| f).sum();
        (n >= 10 && total > 0.0).then(|| n as f32 / total)
    }

    fn reset(&mut self) {
        self.ticks.clear();
        self.frames.clear();
        self.easy_since = None;
        self.settled = None;
    }

    /// The host's governor, once a frame: the new notch, if it changed,
    /// and why.
    fn govern(&mut self, at: Instant, tick: Tick, frame: f32) -> Option<String> {
        let settled = *self.settled.get_or_insert(at + SETTLE);
        if at < settled {
            return None;
        }
        self.sample(at, tick, frame);
        let (Some(rate), Some(fps)) = (self.rate(), self.fps()) else { return None };
        if self.reported.map(|r| at.duration_since(r) >= REPORT).unwrap_or(true) {
            self.reported = Some(at);
            log::info!("speed {}x (asked {}x): running {:.2}x, {:.0} fps", self.now, self.want, rate, fps);
        }
        let behind = rate < self.now * BEHIND;
        if self.now > 1.0 && behind {
            let was = self.now;
            self.now = Self::down(self.now);
            self.reset();
            return Some(format!("speed {}x to {}x: frames skipped ({:.1}x of {}x, {:.0} fps)", was, self.now, rate, was, fps));
        }
        if self.now < self.want && !behind {
            let since = *self.easy_since.get_or_insert(at);
            if at.duration_since(since) >= RETRY {
                let was = self.now;
                self.now = Self::up(self.now).min(self.want);
                self.reset();
                return Some(format!("speed {}x to {}x: keeping up", was, self.now));
            }
        } else {
            self.easy_since = None;
        }
        None
    }
}

pub struct SpeedPlugin {
    pub want: f32,
}

impl Plugin for SpeedPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Speed::new(self.want)).add_systems(Update, speed);
    }
}

fn speed(
    keys: Res<ButtonInput<KeyCode>>,
    bind: Res<Keys>,
    real: Res<Time<Real>>,
    mut fixed: ResMut<Time<Fixed>>,
    mut sp: ResMut<Speed>,
    session: Option<Res<SessionRes>>,
    mut view: ResMut<View>,
) {
    let hosting = matches!(session.as_deref(), Some(SessionRes(Session::Hosting { .. })));
    let client = matches!(session.as_deref(), Some(SessionRes(Session::Client { .. })));
    let now = Instant::now();
    let tick = view.obs.as_ref().map(|o| o.tick).unwrap_or(0);
    let mut said: Option<String> = None;
    if keys.just_pressed(bind.key(Action::Speed)) && !view.keys_taken() {
        if hosting {
            let next = if sp.want >= SPEEDS[SPEEDS.len() - 1] { SPEEDS[0] } else { Speed::up(sp.want) };
            sp.want = next;
            sp.now = next;
            sp.reset();
            said = Some(format!("speed {}x", next));
        } else {
            said = Some("speed: the host sets it".into());
        }
    }
    // A finished match stands still: nothing to judge or follow.
    let over = view.obs.as_ref().map(|o| o.over).unwrap_or(false);
    if over {
        sp.reset();
    } else if hosting {
        if let Some(why) = sp.govern(now, tick, real.delta_secs()) {
            log::info!("{}", why);
            said = Some(why);
        }
    } else if client {
        // Follow the host: its tick rate, as measured, in quarters.
        sp.sample(now, tick, real.delta_secs());
        if let Some(rate) = sp.rate() {
            sp.now = ((rate * 4.0).round() / 4.0).clamp(0.25, SPEEDS[SPEEDS.len() - 1]);
        }
    }
    let step = Duration::from_secs_f64(1.0 / (TICK_HZ as f64 * sp.now as f64));
    if fixed.timestep() != step {
        fixed.set_timestep(step);
    }
    view.speed = sp.now;
    if let Some(s) = said {
        view.log.push_back((tick, s));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notches() {
        assert_eq!(Speed::notch(4.0), 4.0);
        assert_eq!(Speed::notch(3.0), 2.0);
        assert_eq!(Speed::notch(0.5), 1.0);
        assert_eq!(Speed::down(4.0), 2.0);
        assert_eq!(Speed::down(1.0), 1.0);
        assert_eq!(Speed::up(2.0), 4.0);
    }

    /// Behind: a notch down; keeping up for `RETRY`: a notch back up.
    #[test]
    fn the_governor_steps_down_and_back_up() {
        let mut s = Speed::new(4.0);
        let t0 = Instant::now();
        let hz = TICK_HZ as f32;
        // Settling: nothing judged.
        assert!(s.govern(t0, 0, 1.0).is_none());
        let t0 = t0 + SETTLE;
        // Half the asked rate at a good frame rate.
        let mut said = None;
        for i in 0..400u64 {
            let at = t0 + Duration::from_millis(i * 10);
            let tick = (i as f32 * 0.01 * hz * 2.0) as Tick;
            if let Some(w) = s.govern(at, tick, 0.01) {
                said = Some(w);
                break;
            }
        }
        assert_eq!(s.now, 2.0, "{:?}", said);
        // Now keeping up at 2x for long enough: back to 4x.
        let t1 = t0 + Duration::from_secs(10);
        assert!(s.govern(t1, 10_000, 0.01).is_none());
        let t1 = t1 + SETTLE;
        let mut up = false;
        for i in 0..4000u64 {
            let at = t1 + Duration::from_millis(i * 10);
            let tick = 10_000 + (i as f32 * 0.01 * hz * 2.0) as Tick;
            if s.govern(at, tick, 0.01).is_some() {
                up = true;
                break;
            }
        }
        assert!(up && s.now == 4.0, "now {}", s.now);
        // Slow frames, the ticks kept: no change.
        let t2 = t1 + Duration::from_secs(60);
        assert!(s.govern(t2, 50_000, 0.01).is_none());
        let t2 = t2 + SETTLE;
        for i in 0..400u64 {
            let at = t2 + Duration::from_millis(i * 250);
            let tick = 50_000 + (i as f32 * 0.25 * hz * 4.0) as Tick;
            assert!(s.govern(at, tick, 0.25).is_none());
        }
        assert_eq!(s.now, 4.0);
    }
}
