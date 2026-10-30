//! Biorhythm: a slow wave a tank feels, its intuition (Paul, 30 October: "a
//! concept of intuition ... a general biorhythem as in input where we have
//! different shaped and frequency of waves").
//!
//! THREE WAVES AT DIFFERENT FREQUENCIES, as the old biorhythm charts drew them
//! (23, 28 and 33 days) and as a synthesizer's slow oscillators are: their ratio
//! is kept and `bio_period` sets the time. `bio` is the shape, a whole number; 0 is
//! no wave at all. Each seat starts at its own phase, so a team is never all in the
//! same mood at once, AND THE SAME SEED PLAYS THE SAME GAME: a wave is a function of
//! the tick and the seat, with no randomness of its own for a census to trip on.
//!
//! Nothing here claims a tank has moods worth having. It is a sense and a dial, and
//! the census decides whether a brain that swings beats one that does not.

use crate::weights::{Settings, W};
use treadcore_sim::{PlayerId, Tick, TICK_HZ};

/// How many waves.
pub const WAVES: usize = 3;
/// Each wave's period against the first: 23, 28 and 33.
const RATIO: [f32; WAVES] = [1.0, 28.0 / 23.0, 33.0 / 23.0];

/// A wave of `shape` at `phase` turns, from -1 to 1: 1 sine, 2 triangle, 3
/// square, 4 saw; anything else is silence.
pub fn wave(shape: usize, phase: f32) -> f32 {
    let p = phase - phase.floor();
    match shape {
        1 => (p * std::f32::consts::TAU).sin(),
        2 => 1.0 - 4.0 * (p - 0.5).abs(),
        3 => if p < 0.5 { 1.0 } else { -1.0 },
        4 => 2.0 * p - 1.0,
        _ => 0.0,
    }
}

/// The three waves for this seat now. All zero with `bio` at 0.
pub fn rhythm(s: &Settings, seat: PlayerId, tick: Tick) -> [f32; WAVES] {
    let shape = s.choice(W::Bio);
    let mut out = [0.0; WAVES];
    if shape == 0 {
        return out;
    }
    let secs = tick as f32 / TICK_HZ as f32;
    let period = s.w[W::BioPeriod].max(1.0);
    // The golden ratio spreads any number of seats about the circle.
    let start = seat as f32 * 0.618_034;
    for (i, o) in out.iter_mut().enumerate() {
        *o = wave(shape, start + i as f32 / 3.0 + secs / (period * RATIO[i]));
    }
    out
}

/// One number for a dial to follow: the waves' mean.
pub fn mood(s: &Settings, seat: PlayerId, tick: Tick) -> f32 {
    rhythm(s, seat, tick).iter().sum::<f32>() / WAVES as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(shape: &str) -> Settings {
        Settings::parse(&format!("bio = {}", shape)).unwrap()
    }

    #[test]
    fn with_bio_off_there_is_no_wave_at_all() {
        assert_eq!(rhythm(&Settings::default(), 3, 5000), [0.0; WAVES]);
        assert_eq!(mood(&Settings::default(), 3, 5000), 0.0);
    }

    #[test]
    fn every_shape_stays_between_minus_one_and_one_and_they_differ() {
        for shape in 1..=4 {
            for k in 0..200 {
                let v = wave(shape, k as f32 * 0.037);
                assert!((-1.0..=1.0).contains(&v), "shape {} gave {}", shape, v);
            }
        }
        assert_ne!(wave(1, 0.1), wave(2, 0.1));
        assert_ne!(wave(3, 0.1), wave(4, 0.1));
    }

    #[test]
    fn the_same_seat_and_tick_give_the_same_wave_and_seats_and_waves_differ() {
        let s = on("sine");
        assert_eq!(rhythm(&s, 2, 999), rhythm(&s, 2, 999), "not deterministic");
        assert_ne!(rhythm(&s, 2, 999), rhythm(&s, 3, 999), "two seats in step");
        let r = rhythm(&s, 2, 999);
        assert!(r[0] != r[1] && r[1] != r[2], "the three waves are one wave");
    }
}
