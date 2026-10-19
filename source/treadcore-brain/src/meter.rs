//! What the brain spends (roadmap step 12a, LAB-REPORT-BRAIN.md section
//! 5, "measure first"). Counters, one set per thread, so the census tests
//! that run side by side each count their own match: thinks, the Moves
//! the bots asked for, the crossing searches, the plan copies, the
//! crossing answers the cache gave instead, and the time `run_bots`
//! took. Nothing reads them but a census and `Brain::status`-style
//! reports; no decision depends on them, so a replay is the same with
//! them or without.

use std::cell::Cell;
use std::time::Duration;

/// One thread's counts since the last `reset`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Meter {
    /// Seats run (`run_bots`), one per bot per tick.
    pub seats: u64,
    /// Thinks that did a doctrine's work (the RuleBot's 4 Hz).
    pub thinks: u64,
    /// Move commands asked of the world: each is an A* route.
    pub moves: u64,
    /// `Map::crossing` searches run, and those the cache answered.
    pub crossings: u64,
    pub crossings_cached: u64,
    /// Whole-plan copies made to answer a question.
    pub plan_clones: u64,
    /// Nanoseconds inside `run_bots`.
    pub nanos: u64,
    /// Ladder picks under a team posture (LR 24), by the posture read
    /// (economy, fortify, connect, expand, defend), and those the
    /// posture changed from the first rung that held.
    pub postures: [u64; 5],
    pub leaned: u64,
}

thread_local! {
    static METER: Cell<Meter> = Cell::new(Meter::default());
}

fn bump(f: impl FnOnce(&mut Meter)) {
    METER.with(|m| {
        let mut v = m.get();
        f(&mut v);
        m.set(v);
    });
}

pub fn seat() {
    bump(|m| m.seats += 1);
}
pub fn think() {
    bump(|m| m.thinks += 1);
}
pub fn moves(n: u64) {
    bump(|m| m.moves += n);
}
pub fn crossing() {
    bump(|m| m.crossings += 1);
}
pub fn crossing_cached() {
    bump(|m| m.crossings_cached += 1);
}
pub fn plan_clone() {
    bump(|m| m.plan_clones += 1);
}
pub fn posture(p: crate::board::Posture, changed: bool) {
    bump(|m| {
        m.postures[p as usize] += 1;
        m.leaned += changed as u64;
    });
}
pub fn spent(d: Duration) {
    bump(|m| m.nanos += d.as_nanos() as u64);
}

/// This thread's counts.
pub fn read() -> Meter {
    METER.with(|m| m.get())
}

/// Start counting again from zero.
pub fn reset() {
    METER.with(|m| m.set(Meter::default()));
}

impl Meter {
    /// Microseconds per seat-tick: what one bot costs a tick, on average.
    pub fn micros_per_seat(&self) -> f64 {
        if self.seats == 0 {
            0.0
        } else {
            self.nanos as f64 / 1000.0 / self.seats as f64
        }
    }

    /// One line for a census: the counts and the cost.
    pub fn line(&self) -> String {
        format!(
            "meter: {} seat-ticks, {} thinks, {} moves, {} crossings (+{} cached), {} plan copies, {:.1} us a seat-tick",
            self.seats,
            self.thinks,
            self.moves,
            self.crossings,
            self.crossings_cached,
            self.plan_clones,
            self.micros_per_seat()
        ) + &if self.postures.iter().sum::<u64>() > 0 {
            format!(", postures {:?} (economy fortify connect expand defend), {} picks leaned", self.postures, self.leaned)
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_meter_counts_and_resets() {
        reset();
        seat();
        seat();
        moves(3);
        crossing();
        spent(Duration::from_micros(10));
        let m = read();
        assert_eq!((m.seats, m.moves, m.crossings), (2, 3, 1));
        assert!((m.micros_per_seat() - 5.0).abs() < 1e-9);
        reset();
        assert_eq!(read(), Meter::default());
    }
}
