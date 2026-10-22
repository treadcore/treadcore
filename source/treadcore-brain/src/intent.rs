//! Holding a decision (roadmap step 12d, LAB-REPORT-BRAIN.md section 7).
//! A tank keeps doing its last command until a new one has been
//! confirmed over several thinks; only the survival reflexes, dodging a
//! shell and going home to resupply, break in at once and at full
//! throttle. Two pieces:
//!
//! - `Desire`, a smoothed goal: D = (1 - alpha) D + alpha d. A goal that
//!   moves a little each think (a wedge slot ahead of a moving leader, a
//!   rally on a moving enemy) is followed smoothly instead of re-routed
//!   every tick; a jump past `jitter` tiles is taken at once.
//! - `Confirm`, a Schmitt trigger on a choice: a challenger replaces the
//!   held choice only when it beats it by `margin` for `need` thinks in a
//!   row, and the held choice's score carries a commitment bonus.

use treadcore_sim::V2;

/// A smoothed point.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Desire {
    at: Option<V2>,
}

impl Desire {
    /// Where to go this think, given where the raw pull says; `alpha` the
    /// share of the new pull, `jitter` the jump taken at once.
    pub fn toward(&mut self, raw: V2, alpha: f32, jitter: f32) -> V2 {
        let next = match self.at {
            Some(at) if at.dist(raw) <= jitter => at + (raw - at).scale(alpha.clamp(0.0, 1.0)),
            _ => raw,
        };
        self.at = Some(next);
        next
    }

    /// Forget: the next pull is taken as it is (a dodge, a new goal).
    pub fn reset(&mut self) {
        self.at = None;
    }

    /// Where it went last, if it has a pull.
    pub fn at(&self) -> Option<V2> {
        self.at
    }
}

/// A choice held until a challenger is confirmed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Confirm<T> {
    held: Option<T>,
    challenger: Option<(T, u8)>,
}

impl<T> Default for Confirm<T> {
    fn default() -> Self {
        Confirm { held: None, challenger: None }
    }
}

impl<T: Copy + PartialEq> Confirm<T> {
    /// This think's best `best` (score `best_s`) against the held choice
    /// (score `held_s`, None if it is no longer possible): what to do.
    /// `margin` the share it must win by, `need` the thinks in a row.
    pub fn choose(&mut self, best: T, best_s: f32, held_s: Option<f32>, margin: f32, need: u8) -> T {
        let Some(held) = self.held else {
            self.held = Some(best);
            return best;
        };
        if held == best {
            self.challenger = None;
            return held;
        }
        let Some(held_s) = held_s else {
            // What was held is gone: take the best at once.
            self.held = Some(best);
            self.challenger = None;
            return best;
        };
        if best_s > held_s * (1.0 + margin) {
            let streak = match self.challenger {
                Some((c, k)) if c == best => k.saturating_add(1),
                _ => 1,
            };
            if streak >= need.max(1) {
                self.held = Some(best);
                self.challenger = None;
                return best;
            }
            self.challenger = Some((best, streak));
        } else {
            self.challenger = None;
        }
        held
    }

    /// Take `x` now, whatever the score: a survival reflex, or an order.
    pub fn force(&mut self, x: T) {
        self.held = Some(x);
        self.challenger = None;
    }

    /// The choice held, if any.
    pub fn held(&self) -> Option<T> {
        self.held
    }

    /// Hold nothing: the next choice is taken at once.
    pub fn clear(&mut self) {
        self.held = None;
        self.challenger = None;
    }
}

/// An advance-or-fall-back scalar, smoothed, with two thresholds: push
/// above `band`, fall back below `-band`, hold between.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Advance {
    pub a: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Push,
    Hold,
    Back,
}

impl Advance {
    /// Smooth in `raw` (clamped to -1..1) at share `beta` and read the
    /// step: push above `band`, back below `-band`, else hold.
    pub fn update(&mut self, raw: f32, beta: f32, band: f32) -> Step {
        self.a = (1.0 - beta) * self.a + beta * raw.clamp(-1.0, 1.0);
        if self.a > band {
            Step::Push
        } else if self.a < -band {
            Step::Back
        } else {
            Step::Hold
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_challenger_needs_two_thinks() {
        let mut c = Confirm::default();
        assert_eq!(c.choose(1u32, 1.0, None, 0.15, 2), 1);
        // A challenger 20% better: held on the first think, taken on the second.
        assert_eq!(c.choose(2, 1.2, Some(1.0), 0.15, 2), 1);
        assert_eq!(c.choose(2, 1.2, Some(1.0), 0.15, 2), 2);
        // Only 10% better: never.
        for _ in 0..5 {
            assert_eq!(c.choose(3, 1.1, Some(1.0), 0.15, 2), 2);
        }
        // Held one gone: the best at once.
        assert_eq!(c.choose(3, 0.5, None, 0.15, 2), 3);
        // A streak broken by another starts again.
        assert_eq!(c.choose(4, 2.0, Some(1.0), 0.15, 2), 3);
        assert_eq!(c.choose(5, 2.0, Some(1.0), 0.15, 2), 3);
        assert_eq!(c.choose(5, 2.0, Some(1.0), 0.15, 2), 5);
    }

    #[test]
    fn a_dodge_skips_confirmation() {
        let mut c = Confirm::default();
        c.choose(1u32, 1.0, None, 0.15, 3);
        c.force(9);
        assert_eq!(c.held(), Some(9));
        let mut d = Desire::default();
        d.toward(V2::new(0.0, 0.0), 0.4, 3.0);
        assert_eq!(d.toward(V2::new(2.0, 0.0), 0.4, 3.0), V2::new(0.8, 0.0), "a small shift is smoothed");
        assert_eq!(d.toward(V2::new(20.0, 0.0), 0.4, 3.0), V2::new(20.0, 0.0), "a jump is taken at once");
    }

    #[test]
    fn the_advance_holds_between_its_bands() {
        let mut a = Advance::default();
        assert_eq!(a.update(1.0, 0.3, 0.35), Step::Hold, "one think forward is not yet a push");
        assert_eq!(a.update(1.0, 0.3, 0.35), Step::Push);
        assert_eq!(a.update(-1.0, 0.3, 0.35), Step::Hold);
        assert_eq!(a.update(-1.0, 0.3, 0.35), Step::Hold);
        assert_eq!(a.update(-1.0, 0.3, 0.35), Step::Back);
    }
}
