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

/// Where this seat's three waves stand now, in turns: whatever `bio` says, so a census can fold what
/// tanks did over waves that were driving nothing, and see what chance looks like.
pub fn phases(s: &Settings, seat: PlayerId, tick: Tick) -> [f32; WAVES] {
    let secs = tick as f32 / TICK_HZ as f32;
    let period = s.w[W::BioPeriod].max(1.0);
    // The golden ratio spreads any number of seats about the circle.
    let start = seat as f32 * 0.618_034;
    let mut out = [0.0; WAVES];
    for (i, o) in out.iter_mut().enumerate() {
        *o = start + i as f32 / 3.0 + secs / (period * RATIO[i]);
    }
    out
}

/// The three waves for this seat now. All zero with `bio` at 0.
pub fn rhythm(s: &Settings, seat: PlayerId, tick: Tick) -> [f32; WAVES] {
    let shape = s.choice(W::Bio);
    let mut out = [0.0; WAVES];
    if shape == 0 {
        return out;
    }
    let at = phases(s, seat, tick);
    for (o, p) in out.iter_mut().zip(at) {
        *o = wave(shape, p);
    }
    out
}

/// THE TEMPERAMENT MATRIX (Paul, 1 November: "connect biorhythems to the output temperment"). Rows are
/// what a tank's temper moves, columns are the three waves, and each entry is a dial, so the census, the
/// road and a person read them as they read any other. Each row: its dial x (1 + sum of entry x wave).
/// All zero, as they start, nothing moves. `bio_amount` is the older road, the mean of the waves into
/// `engage_odds`: one row of equal numbers, and it stays.
pub const TEMPERS: usize = 4;
pub const TEMPER: [[W; WAVES]; TEMPERS] = [
    [W::TemperEngage1, W::TemperEngage2, W::TemperEngage3],
    [W::TemperPress1, W::TemperPress2, W::TemperPress3],
    [W::TemperRetreat1, W::TemperRetreat2, W::TemperRetreat3],
    [W::TemperTrigger1, W::TemperTrigger2, W::TemperTrigger3],
];

/// One matrix row at these waves: its multiplier, held from a quarter to four times.
pub fn temper_row(s: &Settings, row: usize, waves: &[f32; WAVES]) -> f32 {
    let sum: f32 = TEMPER[row].iter().zip(waves).map(|(w, v)| s.w[*w] * v).sum();
    (1.0 + sum).clamp(0.25, 4.0)
}

/// Whether any entry of a row is set: a row of zeros leaves its dial alone altogether.
pub fn temper_on(s: &Settings, row: usize) -> bool {
    TEMPER[row].iter().any(|w| s.w[*w] != 0.0)
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
        // Named, not the default: the fleet may turn bio on.
        let off = on("off");
        assert_eq!(rhythm(&off, 3, 5000), [0.0; WAVES]);
        assert_eq!(mood(&off, 3, 5000), 0.0);
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

    /// Every entry named: the fleet may promote one, and these tests are not about the defaults.
    fn matrix(set: &str) -> Settings {
        let mut text = String::from("bio = sine\n");
        for row in TEMPER.iter() {
            for w in row {
                text += &format!("{} = 0\n", w.name());
            }
        }
        Settings::parse(&(text + set)).unwrap()
    }

    #[test]
    fn an_empty_matrix_moves_nothing_and_a_row_follows_its_own_wave() {
        let s = matrix("");
        for row in 0..TEMPERS {
            assert!(!temper_on(&s, row), "named at 0, and a row is on");
            assert_eq!(temper_row(&s, row, &[1.0, -1.0, 0.5]), 1.0);
        }
        let set = matrix("temper_press_2 = 0.5\n");
        assert!(temper_on(&set, 1) && !temper_on(&set, 0));
        assert_eq!(temper_row(&set, 1, &[1.0, 1.0, 1.0]), 1.5);
        assert_eq!(temper_row(&set, 1, &[1.0, -1.0, 1.0]), 0.5, "the second wave, not the first");
        let neg = matrix("temper_retreat_1 = -1\ntemper_retreat_2 = -1\ntemper_retreat_3 = -1\n");
        assert_eq!(neg.w[W::TemperRetreat1], -1.0, "a negative entry is allowed: a wave may calm");
        assert_eq!(temper_row(&neg, 2, &[1.0, 1.0, 1.0]), 0.25, "held at a quarter");
    }

    #[test]
    fn phases_stand_whatever_bio_says_and_the_waves_are_unchanged() {
        let off = on("off");
        assert_ne!(phases(&off, 3, 500), phases(&off, 4, 500));
        let s = on("sine");
        let p = phases(&s, 3, 500);
        let r = rhythm(&s, 3, 500);
        for i in 0..WAVES {
            assert_eq!(r[i], wave(1, p[i]));
        }
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
