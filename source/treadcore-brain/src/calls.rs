//! Calls to friends (LR 97): a team's shared queue of requests, so a
//! learned tank can ask its teammates for something and hear what they
//! ask of it.
//!
//! A call is a kind, a place and a time. The team's learned tanks share
//! one `Calls` (one queue a team a match, as a team board is shared,
//! LR 58); each posts with its Signal head and feels the others' calls
//! as senses (senses.rs: the compass's Call channel and the call
//! contact). Every call is also said in the world as a `Command::Ping`,
//! so scripted teammates answer it as they answer one another, and a
//! teammate's ping heard in the world is taken into the queue as an
//! attack call: nets and scripted brains share one language.
//!
//! The brains of a match think in seat order on one thread, so the
//! queue is read and written in the same order every time: a match is
//! the same twice.

use std::sync::{Arc, Mutex};
use treadcore_sim::*;

/// Ticks a call is heard after it is made: 10 s.
pub const CALL_LIFE: Tick = 10 * TICK_HZ as Tick;
/// Ticks between one tank's calls: 2 s (the world takes a ping a
/// second).
pub const CALL_EVERY: Tick = 2 * TICK_HZ as Tick;
/// Calls the queue keeps; the oldest go first.
pub const CALLS_KEPT: usize = 16;

/// What a call asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallKind {
    /// Come to me: I am under fire or outnumbered.
    Help,
    /// Go for the enemy there.
    Attack,
    /// Take the depot or pillbox there.
    Take,
}

pub const CALL_KINDS: [CallKind; 3] = [CallKind::Help, CallKind::Attack, CallKind::Take];

/// One request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Call {
    pub from: PlayerId,
    pub kind: CallKind,
    pub at: V2,
    pub tick: Tick,
}

/// A team's queue, shared by its learned tanks.
#[derive(Clone, Default)]
pub struct Calls(Arc<Mutex<Vec<Call>>>);

impl Calls {
    /// An empty queue, for one team's match.
    pub fn new() -> Calls {
        Calls::default()
    }

    /// Post a call; calls past their life and the oldest past
    /// `CALLS_KEPT` go.
    pub fn post(&self, c: Call) {
        let mut q = self.0.lock().unwrap();
        q.retain(|x| c.tick < x.tick + CALL_LIFE);
        q.push(c);
        let n = q.len();
        if n > CALLS_KEPT {
            q.drain(..n - CALLS_KEPT);
        }
    }

    /// The calls a tank hears now: live, and not its own; newest last.
    pub fn heard(&self, me: PlayerId, now: Tick) -> Vec<Call> {
        self.0.lock().unwrap().iter().filter(|c| c.from != me && now < c.tick + CALL_LIFE && c.tick <= now).copied().collect()
    }

    /// Take a tick's teammates' pings from the world (`Event::Pinged`)
    /// as attack calls, once each.
    pub fn hear(&self, obs: &Observation) {
        for e in &obs.events {
            if let Event::Pinged { player, at } = e {
                if *player != obs.player && obs.allied(*player) {
                    let c = Call { from: *player, kind: CallKind::Attack, at: *at, tick: obs.tick };
                    if !self.0.lock().unwrap().contains(&c) {
                        self.post(c);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The queue: a call is heard by the others and not by its caller,
    /// for `CALL_LIFE` and no longer, and it keeps the newest
    /// `CALLS_KEPT`.
    #[test]
    fn a_call_is_heard_by_friends_for_its_life() {
        let q = Calls::new();
        let shared = q.clone();
        q.post(Call { from: 1, kind: CallKind::Help, at: V2::new(5.0, 5.0), tick: 100 });
        assert_eq!(shared.heard(2, 100).len(), 1, "a friend hears it");
        assert!(shared.heard(1, 100).is_empty(), "not its caller");
        assert_eq!(shared.heard(2, 100 + CALL_LIFE - 1).len(), 1);
        assert!(shared.heard(2, 100 + CALL_LIFE).is_empty(), "gone after its life");
        for t in 0..40 {
            q.post(Call { from: 3, kind: CallKind::Attack, at: V2::new(t as f32, 0.0), tick: 200 + t });
        }
        let h = q.heard(2, 240);
        assert_eq!(h.len(), CALLS_KEPT);
        assert_eq!(h.last().unwrap().at.x, 39.0, "the newest kept");
    }
}
