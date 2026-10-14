//! Sound (roadmap step 9): the one thing the HUD cannot convey, a shell
//! launched where you cannot see.
//!
//! No sound files. Every sound is synthesised here from a few lines of
//! arithmetic, the way everything drawn is a rectangle: the overlay is
//! the look, and this is its sound. The weapons and the metal are
//! two-operator FM (`Fm`): a gunshot is an inharmonic crack whose index
//! falls in milliseconds over a punch swept from 1 kHz to the bass; the
//! cannon a low carrier with a slow index; a shell striking armour a
//! bell at 1:1.4 falling from bright to a ring; a missile an index that
//! rises. Explosions are filtered noise over an FM thump. Each cue has
//! three takes, a few per cent apart in pitch, played in turn.
//! `treadcore --write-sounds DIR` (`make sounds`) writes each one as a
//! WAV file to listen to.
//!
//! Every sound has a place, the event's place in the world, and the
//! whole map is heard (`place`). Anything on screen is at full strength,
//! panned from the screen's left edge to its right. Off screen it fades
//! with the distance past the edge, smoothly, down to a quiet floor and
//! never to silence, and past fifteen tiles out it is played from a
//! low-passed take: a fight across the map is a dull rumble on that
//! side. The mixes are made on first use per cue, take, nine pan
//! positions and near or far, as 16-bit WAV in memory, and handed to
//! Bevy's audio as ordinary `AudioSource`s; a voice is an entity that
//! despawns when its sound ends.
//!
//! What is heard is what the player may know (the sim's `event_visible`),
//! plus one thing the window works out for itself: a hostile shell the
//! first time a snapshot shows it, which whistles where it will land.
//! Only what has no place in the world (an order refused, a proposal,
//! the end of the match) sounds in the middle.
//!
//! `M` mutes and unmutes; `--mute` starts muted.

use crate::render::MainCamera;
use crate::session::View;
use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, GlobalVolume, PlaybackSettings, Volume};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use treadcore_sim::{EntityId, Observation, PlayerId, V2};

/// Samples a second: the clicks' carriers reach 9 kHz, and their
/// sidebands must stay under half of this (`Fm::run`).
pub const RATE: u32 = 44_100;
/// The synthesis runs at twice that and is band-limited down
/// (`decimate`): FM sidebands between 22 and 44 kHz are filtered out
/// instead of folding back into the audible band as noise.
const OVERSAMPLE: usize = 2;
const ENGINE: u32 = RATE * OVERSAMPLE as u32;
/// Pan positions, left to right: enough that a sound crossing the
/// screen moves with it.
const PANS: usize = 9;
/// At most this many sounds at once; past it, the quiet ones are dropped.
const VOICES: usize = 20;
/// The same cue twice inside this is one sound (sixteen guns at once).
const REPEAT: Duration = Duration::from_millis(60);
/// The master gain: about −6 dB, so a crowd of voices does not clip.
const HEADROOM: f32 = 0.5;
/// The clock ticks this often while you take something.
const TICK_EVERY: Duration = Duration::from_secs(1);

/// A cue waiting for its entity to appear in a snapshot waits this long;
/// then it is in the fog, and heard as background from nowhere.
const WAIT: Duration = Duration::from_millis(300);

/// Every sound has a place, and the whole map is heard. On screen is
/// full strength. Off screen, the strength falls with the distance past
/// the screen's edge, half at `EDGE_HALF` tiles out, toward `FLOOR`,
/// which it never goes under: a fight across the map is background, not
/// silence. Past `FAR_TAKE` tiles out a sound is played from its far
/// take, low-passed, because distance dulls as well as quietens.
const EDGE_HALF: f32 = 20.0;
const FLOOR: f32 = 0.06;
const FAR_TAKE: f32 = 15.0;
/// The far take's cutoff, twice through a one-pole filter.
const FAR_CUTOFF: f32 = 1200.0;

/// The part of the map on screen: its centre and half its size, tiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub centre: V2,
    pub half: V2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cue {
    /// The gun, indirect: a heavy thump and a rumble.
    Gun,
    /// The gun, direct: a gunshot's crack.
    Cannon,
    /// An indirect shell lands.
    Boom,
    /// A direct shell lands.
    Pop,
    Mine,
    /// A drone leaves the bay.
    Launch,
    /// A strike drone leaves the bay: a missile's whoosh.
    Rocket,
    /// A strike drone gets its tank.
    RocketHit,
    /// An interceptor gets its target.
    Intercept,
    /// A drone shot down.
    Crash,
    /// A direct shell strikes metal.
    Strike,
    /// A direct shell lands clear and glances off.
    Ricochet,
    /// A tank is hit.
    Clang,
    /// A tank is destroyed.
    Wreck,
    PillboxFire,
    /// A hostile shell, first seen: a falling whistle.
    Incoming,
    /// The squad builds or harvests.
    Build,
    /// A pillbox goes down in place.
    Thud,
    /// The tank resupplied at a depot.
    Reload,
    /// A drone's link lost.
    Static,
    /// The breech after one of your own shots: doo, cha, klank.
    Breech,
    /// The clock while you take a depot or a pillbox.
    Tick,
    /// Out of shells: BEEEEEP.
    Empty,
    /// Your armour is low: a beating beep.
    Alarm,
    /// Your tracks, looped while you move.
    Track,
    /// Interface: something taken for you.
    Capture,
    /// Interface: something of yours taken.
    Lost,
    /// Interface: the swarm is inbound.
    Swarm,
    /// Interface: a proposal to you, an alliance made with you.
    Notice,
    /// Interface: an order refused.
    Refused,
    Win,
    Lose,
}

impl Cue {
    pub const ALL: [Cue; 32] = [
        Cue::Gun, Cue::Cannon, Cue::Boom, Cue::Pop, Cue::Mine, Cue::Launch, Cue::Rocket, Cue::RocketHit, Cue::Intercept,
        Cue::Crash, Cue::Strike, Cue::Ricochet, Cue::Clang, Cue::Wreck, Cue::PillboxFire, Cue::Incoming, Cue::Build, Cue::Thud,
        Cue::Reload, Cue::Static, Cue::Breech, Cue::Tick, Cue::Empty, Cue::Alarm, Cue::Track, Cue::Capture, Cue::Lost,
        Cue::Swarm, Cue::Notice, Cue::Refused, Cue::Win, Cue::Lose,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Cue::Gun => "gun",
            Cue::Cannon => "cannon",
            Cue::Boom => "boom",
            Cue::Pop => "pop",
            Cue::Mine => "mine",
            Cue::Launch => "launch",
            Cue::Rocket => "rocket",
            Cue::RocketHit => "rocket-hit",
            Cue::Intercept => "intercept",
            Cue::Crash => "crash",
            Cue::Strike => "strike",
            Cue::Ricochet => "ricochet",
            Cue::Clang => "clang",
            Cue::Wreck => "wreck",
            Cue::PillboxFire => "pillbox-fire",
            Cue::Incoming => "incoming",
            Cue::Build => "build",
            Cue::Thud => "thud",
            Cue::Reload => "reload",
            Cue::Static => "static",
            Cue::Breech => "breech",
            Cue::Tick => "tick",
            Cue::Empty => "empty",
            Cue::Alarm => "alarm",
            Cue::Track => "track",
            Cue::Capture => "capture",
            Cue::Lost => "lost",
            Cue::Swarm => "swarm",
            Cue::Notice => "notice",
            Cue::Refused => "refused",
            Cue::Win => "win",
            Cue::Lose => "lose",
        }
    }
}

/// Where a cue comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Where {
    /// Not in the world: the middle, full strength.
    Ui,
    At(V2),
    /// An entity not yet in the snapshot (a shell just launched): found
    /// in the next one, or dropped.
    Entity(EntityId),
}

// ------------------------------------------------------------- synthesis

/// A tiny deterministic noise source: the same sound every run.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One-pole low-pass, its cutoff in Hz, changeable per sample.
struct Lowpass(f32);

impl Lowpass {
    fn run(&mut self, x: f32, cutoff: f32) -> f32 {
        self.run_at(x, cutoff, ENGINE)
    }
    fn run_at(&mut self, x: f32, cutoff: f32, rate: u32) -> f32 {
        let a = 1.0 - (-2.0 * std::f32::consts::PI * cutoff / rate as f32).exp();
        self.0 += a * (x - self.0);
        self.0
    }
}

/// A sine whose frequency moves; the phase is kept, so a sweep is smooth.
struct Osc(f32);

impl Osc {
    fn sine(&mut self, hz: f32) -> f32 {
        self.0 = (self.0 + hz / ENGINE as f32).fract();
        (self.0 * std::f32::consts::TAU).sin()
    }
    fn tri(&mut self, hz: f32) -> f32 {
        self.0 = (self.0 + hz / ENGINE as f32).fract();
        4.0 * (self.0 - 0.5).abs() - 1.0
    }
}

fn decay(t: f32, rate: f32) -> f32 {
    (-t * rate).exp()
}

/// A few milliseconds of rise, so nothing starts with a click.
fn attack(t: f32, secs: f32) -> f32 {
    (t / secs).min(1.0)
}

/// From `a` to `b` exponentially over `secs`, then `b`.
fn sweep(t: f32, a: f32, b: f32, secs: f32) -> f32 {
    a * (b / a).powf((t / secs).min(1.0))
}

/// Two-operator FM, the DX7's way (phase modulation, which is FM by
/// another name): y = sin(2π·fc·t + I·sin(2π·fm·t)), with fm = fc·ratio.
/// The spectrum is the carrier and sidebands at fc ± n·fm, each as
/// strong as the Bessel function Jn(I). A whole-number ratio is a tone;
/// an inharmonic one (1:1.41, 1:2.7) is metal. The index envelope is the
/// sound: a fast fall is a strike, a slow rise a whoosh. Both phases are
/// kept, so the frequency can sweep.
struct Fm {
    c: f32,
    m: f32,
}

impl Fm {
    const NEW: Fm = Fm { c: 0.0, m: 0.0 };

    /// The largest index whose sidebands stay under the engine's Nyquist,
    /// by Carson's rule: the band is about fc + (I + 1)·fm, so
    /// I ≤ (fs/2 − fc)/fm − 1. Past it the upper sidebands would fold
    /// back as noise nobody asked for. With the engine at 88.2 kHz the
    /// recipe's clicks keep most of their index (the 4 kHz doo all of
    /// its 5); what lies between 22 and 44 kHz the decimation removes.
    fn max_index(fc: f32, ratio: f32) -> f32 {
        ((ENGINE as f32 / 2.0 - fc) / (fc * ratio).max(1.0) - 1.0).max(0.0)
    }

    fn run(&mut self, fc: f32, ratio: f32, index: f32) -> f32 {
        let index = index.min(Fm::max_index(fc, ratio));
        let r = ENGINE as f32;
        self.m = (self.m + fc * ratio / r).fract();
        self.c = (self.c + fc / r).fract();
        (self.c * std::f32::consts::TAU + index * (self.m * std::f32::consts::TAU).sin()).sin()
    }
}

/// How many takes of each cue: the same gun three times in a row is
/// three slightly different guns (pitch and noise), not a machine.
pub const VARIANTS: usize = 3;

/// The explosion every boom is made of: low-passed noise with a falling
/// cutoff, and under it a falling FM thump whose modulator sits an
/// octave below the carrier (ratio 1:0.5), so the sidebands go down
/// into the sub-bass.
fn explosion(t: f32, n: &mut Noise, lp: &mut Lowpass, sub: &mut Fm, size: f32) -> f32 {
    if t < 0.0 {
        return 0.0;
    }
    let top = 2400.0 / size;
    let cutoff = sweep(t, top, 120.0, 0.6 * size);
    // Filtered noise loses power with its cutoff; put it back, so the
    // roll lasts as long as the envelope says and not a tenth of it.
    let body = lp.run(n.next(), cutoff) * (top / cutoff).sqrt() * 0.6 * decay(t, 3.5 / size) * attack(t, 0.004);
    let thump = sub.run(sweep(t, 75.0, 30.0, 0.5 * size), 0.5, 2.0 * decay(t, 6.0 / size)) * decay(t, 5.0 / size) * attack(t, 0.003);
    body * 2.2 + thump * 0.8
}

/// The bright edge of a noise burst: the noise less its low-passed self.
fn crack(n: &mut Noise, lp: &mut Lowpass, cutoff: f32) -> f32 {
    let x = n.next();
    x - lp.run(x, cutoff)
}

/// A gunshot, after the recipe: an inharmonic FM crack at `fc` whose
/// index falls in a few milliseconds, a punch swept from 1 kHz to the
/// bass, the muzzle flash as a burst of bright noise, and the report
/// rolling off behind it.
fn gunshot(t: f32, p: f32, fc: f32, ratio: f32, n: &mut Noise, lp: &mut Lowpass, lp2: &mut Lowpass, op: &mut Fm, punch: &mut Osc, report: f32) -> f32 {
    // The crack leads; the punch is under it, not over it (the first
    // mix put 9 % of the energy above 2.5 kHz: a thump, not a shot).
    let crack_fm = op.run(fc * p, ratio, 14.0 * decay(t, 60.0)) * decay(t, 32.0);
    let thump = punch.sine(sweep(t, 1000.0 * p, 50.0, 0.08)) * decay(t, 16.0);
    let flash = crack(n, lp, 1500.0) * decay(t, 70.0);
    let tail = lp2.run(n.next(), 500.0) * 2.5 * decay(t, 7.0) * attack(t, 0.01);
    (crack_fm * 1.2 + thump * 0.55 + flash * 1.3 + tail * report) * attack(t, 0.0008)
}

/// A shell striking armour: an FM bell at 1:1.4 whose index falls from
/// bright to a near-pure ring, a second, higher pair at 1:2.7 that dies
/// first, a thunk, and the spark of the hit.
fn strike(t: f32, p: f32, base: f32, n: &mut Noise, lp: &mut Lowpass, a: &mut Fm, b: &mut Fm, thunk: &mut Osc) -> f32 {
    let bell = a.run(base * p, 1.4, 7.0 * decay(t, 9.0) + 0.7) * decay(t, 3.5);
    let high = b.run(base * 2.69 * p, 2.7, 4.0 * decay(t, 18.0)) * decay(t, 8.0) * 0.5;
    let knock = thunk.sine(sweep(t, 320.0 * p, 90.0, 0.05)) * decay(t, 25.0);
    let spark = crack(n, lp, 2000.0) * decay(t, 110.0);
    (bell * 0.6 + high + knock * 0.8 + spark) * attack(t, 0.0005)
}

/// One mechanical click, `start` seconds in: a short FM burst, high and
/// inharmonic, gone in `tau` (the recipe's y = A·e^(−t/τ)·sin(2π·fc·t +
/// I·sin(2π·fm·t))). Each click has its own operator, so its phase
/// starts at its own trigger.
fn click(t: f32, start: f32, op: &mut Fm, fc: f32, ratio: f32, index: f32, tau: f32) -> f32 {
    let local = t - start;
    if local < 0.0 || local > tau * 8.0 {
        return 0.0;
    }
    op.run(fc, ratio, index) * (-local / tau).exp() * attack(local, 0.0003)
}

/// An electronic beep: 1:1 FM at a low index, 50 ms in, flat, 100 ms
/// out; `detune` adds a second voice that many Hz above, and the two
/// beat into an alarm's wobble.
fn beep(t: f32, start: f32, len: f32, hz: f32, detune: f32, a: &mut Fm, b: &mut Fm) -> f32 {
    let local = t - start;
    if local < 0.0 || local > len {
        return 0.0;
    }
    let env = attack(local, 0.05) * attack(len - local, 0.1);
    let one = a.run(hz, 1.0, 1.2);
    let two = if detune > 0.0 { b.run(hz + detune, 1.0, 1.2) } else { 0.0 };
    (one + two) * 0.5 * env
}

/// Electric-piano FM (ratio 1:1, the index falling): the interface's
/// notes, one after another, `step` seconds apart.
fn notes(t: f32, op: &mut Fm, hz: &[f32], step: f32, ring: f32) -> f32 {
    let k = ((t / step) as usize).min(hz.len() - 1);
    let local = t - k as f32 * step;
    let len = if k == hz.len() - 1 { ring } else { 9.0 };
    op.run(hz[k], 1.0, 2.0 * decay(local, 9.0) + 0.3) * 0.5 * attack(local, 0.006) * decay(local, len)
}

/// A cue's samples, mono, at `RATE`, normalised and set to its level.
/// `variant` (0 to VARIANTS - 1) moves the pitch a few per cent and
/// reseeds the noise.
pub fn synth(cue: Cue, variant: usize) -> Vec<f32> {
    let secs = match cue {
        Cue::Gun => 1.1,
        Cue::Cannon => 0.6,
        Cue::Boom => 1.4,
        Cue::Pop => 0.5,
        Cue::Mine => 1.3,
        Cue::Launch => 0.8,
        Cue::Rocket | Cue::RocketHit => 1.3,
        Cue::Intercept => 0.3,
        Cue::Crash => 0.6,
        Cue::Strike => 0.9,
        Cue::Ricochet => 0.8,
        Cue::Clang => 0.8,
        Cue::Wreck => 2.2,
        Cue::PillboxFire => 0.5,
        Cue::Incoming => 1.1,
        Cue::Build => 0.18,
        Cue::Thud => 0.4,
        Cue::Reload => 1.5,
        Cue::Breech => 0.6,
        Cue::Tick => 0.08,
        Cue::Empty => 0.85,
        Cue::Alarm => 1.0,
        Cue::Track => 3.0,
        Cue::Static => 0.45,
        Cue::Capture | Cue::Lost => 0.5,
        Cue::Notice => 0.7,
        Cue::Swarm => 1.3,
        Cue::Refused => 0.2,
        Cue::Win | Cue::Lose => 1.3,
    };
    let p = [1.0, 0.96, 1.045][variant % VARIANTS];
    let n = (secs * ENGINE as f32) as usize;
    let mut noise = Noise(0x9E37_79B9 ^ (cue as u32) ^ ((variant as u32 + 1) * 0x0001_F3B5));
    let (mut lp, mut lp2, mut lp3, mut lp4) = (Lowpass(0.0), Lowpass(0.0), Lowpass(0.0), Lowpass(0.0));
    let (mut o1, mut o2, mut o3) = (Osc(0.0), Osc(0.0), Osc(0.0));
    let (mut f1, mut f2, mut f3, mut f4) = (Fm::NEW, Fm::NEW, Fm::NEW, Fm::NEW);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / ENGINE as f32;
        let s = match cue {
            Cue::Gun => {
                // The cannon blast: a low carrier, a moderate index that
                // falls slowly, the muzzle flash, the rumble after.
                let fc = sweep(t, 120.0 * p, 48.0 * p, 0.35);
                let boom = f1.run(fc, 0.5, 3.0 * decay(t, 5.0) + 0.3) * decay(t, 4.0);
                let flash = lp.run(noise.next(), sweep(t, 5000.0, 600.0, 0.06)) * 1.8 * decay(t, 35.0);
                let rumble = lp2.run(noise.next(), 160.0) * 4.0 * decay(t, 2.2) * attack(t, 0.02);
                (boom * 1.1 + flash + rumble * 0.6) * attack(t, 0.002)
            }
            Cue::Cannon => gunshot(t, p, 1700.0, 1.41, &mut noise, &mut lp, &mut lp2, &mut f1, &mut o1, 0.5),
            Cue::PillboxFire => gunshot(t, p, 2300.0, 1.3, &mut noise, &mut lp, &mut lp2, &mut f1, &mut o1, 0.3),
            Cue::Boom => explosion(t, &mut noise, &mut lp, &mut f1, 1.0),
            Cue::Pop => explosion(t, &mut noise, &mut lp, &mut f1, 0.35) + crack(&mut noise, &mut lp2, 1500.0) * decay(t, 60.0) * 0.4,
            Cue::Mine => {
                let shrapnel = f2.run(1500.0 * p, 2.7, 4.0 * decay(t, 25.0)) * decay(t, 18.0) * 0.3;
                explosion(t, &mut noise, &mut lp, &mut f1, 0.9) + crack(&mut noise, &mut lp2, 600.0) * decay(t, 60.0) * 0.6 + shrapnel
            }
            Cue::Launch => {
                // A motor winding up: an FM buzz (1:2) that rises,
                // chopped by the rotor, and air.
                let hz = sweep(t, 180.0 * p, 520.0 * p, 0.6);
                let buzz = f1.run(hz, 2.0, 1.5) * 0.7;
                let rotor = 0.75 + 0.25 * o3.sine(sweep(t, 30.0, 90.0, 0.6));
                let hiss = lp.run(noise.next(), 3000.0) * 0.3;
                (buzz * rotor + hiss) * attack(t, 0.08) * decay((t - 0.45).max(0.0), 9.0)
            }
            Cue::Rocket => {
                // A strike drone away: the recipe's missile, an index
                // ramping up for a rising, whooshing spectrum at 1:1.3,
                // over noise whose cutoff climbs.
                let fc = sweep(t, 140.0 * p, 260.0 * p, 0.8);
                let roar = f1.run(fc, 1.3, 8.0 * attack(t, 0.6));
                let hiss = lp.run(noise.next(), sweep(t, 400.0, 5000.0, 0.6)) * 1.5;
                (roar * 0.5 + hiss) * attack(t, 0.15) * decay((t - 0.7).max(0.0), 5.0)
            }
            Cue::RocketHit => {
                let ring = f2.run(380.0 * p, 1.4, 6.0 * decay(t, 10.0)) * decay(t, 5.0) * 0.4;
                explosion(t, &mut noise, &mut lp, &mut f1, 0.8) + ring + crack(&mut noise, &mut lp2, 1200.0) * decay(t, 80.0) * 0.6
            }
            Cue::Intercept => {
                let zap = f1.run(sweep(t, 1800.0 * p, 300.0, 0.15), 3.7, 5.0 * decay(t, 20.0)) * decay(t, 22.0);
                (crack(&mut noise, &mut lp, 1500.0) * decay(t, 55.0) + zap * 0.7) * attack(t, 0.001)
            }
            Cue::Crash => {
                // A drone shot down: the crunch, and a falling metallic
                // tone as it goes.
                let crunch = lp.run(noise.next(), 3000.0) * 2.0 * decay(t, 25.0);
                let fall = f1.run(sweep(t, 900.0 * p, 180.0, 0.5), 1.7, 3.0 * decay(t, 6.0)) * decay(t, 5.0) * 0.5;
                (crunch + fall + crack(&mut noise, &mut lp2, 2000.0) * decay(t, 90.0) * 0.5) * attack(t, 0.001)
            }
            Cue::Strike => strike(t, p, 420.0, &mut noise, &mut lp, &mut f1, &mut f2, &mut o1),
            Cue::Clang => {
                // Damage, heavier than a strike: thick armour rings
                // lower and dies sooner.
                let ring = f1.run(262.0 * p, 1.41, 5.0 * decay(t, 7.0) + 0.4) * decay(t, 5.0) + f2.run(690.0 * p, 2.3, 3.0 * decay(t, 14.0)) * decay(t, 9.0) * 0.45;
                let hit = lp.run(noise.next(), 2500.0) * 2.0 * decay(t, 70.0);
                (ring * 0.6 + hit) * attack(t, 0.001)
            }
            Cue::Ricochet => {
                // The round glances off: a ting, the spark, and a whine
                // falling away, its modulator a hair off the carrier so it
                // wobbles.
                let whine = f1.run(sweep(t, 3200.0 * p, 1300.0 * p, 0.7), 1.003, 0.9) * attack(t, 0.02) * decay(t, 3.5);
                let ting = f2.run(1900.0 * p, 1.41, 3.0 * decay(t, 30.0)) * decay(t, 25.0) * 0.5;
                whine * 0.6 + ting + crack(&mut noise, &mut lp, 3000.0) * decay(t, 120.0)
            }
            Cue::Wreck => {
                // The tank goes up, then its stores: two smaller blasts
                // after the first, a ring, and things cooking off.
                let big = explosion(t, &mut noise, &mut lp, &mut f1, 1.6);
                let second = explosion(t - 0.55, &mut noise, &mut lp2, &mut f3, 0.5) * 0.6;
                let third = explosion(t - 1.0, &mut noise, &mut lp3, &mut f4, 0.4) * 0.45;
                let ring = f2.run(233.0 * p, 1.41, 4.0 * decay(t, 5.0) + 0.3) * decay(t, 4.0) * 0.35;
                let crackle = if noise.next() > 0.995 && t > 0.2 { decay(t - 0.2, 2.0) * 0.8 } else { 0.0 };
                big + second + third + ring + crackle * noise.next()
            }
            Cue::Incoming => {
                // A shell going over: the pitch falls (Doppler), the FM
                // index rises as it nears, air under it; cut at the end,
                // where the impact's own sound takes over.
                let fc = sweep(t, 1600.0 * p, 480.0 * p, 1.05);
                let tone = f1.run(fc, 1.0, 0.4 + 1.6 * t);
                let air = lp.run(noise.next(), fc * 1.5) * 0.4;
                (tone * 0.5 + air) * attack(t, 0.6) * (1.0 - attack((t - 1.0).max(0.0), 0.1))
            }
            Cue::Build => {
                let knock = o1.sine(sweep(t, 460.0 * p, 300.0, 0.05)) * decay(t, 38.0);
                let tick = lp.run(noise.next(), 3500.0) * decay(t, 120.0);
                (knock + tick) * attack(t, 0.001)
            }
            Cue::Thud => {
                let thud = f1.run(sweep(t, 140.0 * p, 70.0, 0.1), 1.5, 3.0 * decay(t, 20.0)) * decay(t, 12.0);
                (thud + lp.run(noise.next(), 800.0) * 3.0 * decay(t, 40.0)) * attack(t, 0.001)
            }
            Cue::Reload => {
                // The ammo loader: a cycle every half second, three
                // times; the heavy punch at the start of each, the
                // ratchet at three tenths of the cycle and once more
                // just after.
                let cycle = 0.5;
                let k = (t / cycle) as usize;
                let local = t - k as f32 * cycle;
                let punch = o1.sine(120.0 * p) * (-local / 0.07).exp() * attack(local, 0.002);
                let ratchet = click(local, 0.3 * cycle, &mut f1, 4000.0 * p, 1.4, 4.0, 0.007) + click(local, 0.3 * cycle + 0.045, &mut f2, 4600.0 * p, 1.4, 4.0, 0.006) * 0.7;
                punch * 0.6 + ratchet * 1.8
            }
            Cue::Breech => {
                // Doo, cha, klank: the breech opened, the case out, the
                // next round home, a little after the shot.
                let d = 0.3;
                click(t, d, &mut f1, 4000.0 * p, 1.575, 5.0, 0.015)
                    + click(t, d + 0.08, &mut f2, 6000.0 * p, 1.4, 4.0, 0.010) * 0.8
                    + click(t, d + 0.16, &mut f3, 9000.0 * p, 1.5, 6.0, 0.020) * 0.9
                    + o1.sine(150.0) * (-(t - d - 0.16).max(0.0) / 0.05).exp() * if t >= d + 0.16 { 0.5 } else { 0.0 }
            }
            // The watch: the ratchet alone, very short.
            Cue::Tick => click(t, 0.0, &mut f1, 3500.0 * p, 1.4, 4.0, 0.004),
            // Out of ammunition: BEEEEEP, BEEEEEP.
            Cue::Empty => beep(t, 0.0, 0.35, 1000.0, 0.0, &mut f1, &mut f2) + beep(t, 0.47, 0.35, 1000.0, 0.0, &mut f3, &mut f4),
            // Armour low: three beeps, each with a voice 18 Hz above it.
            Cue::Alarm => beep(t, 0.0, 0.25, 1150.0, 18.0, &mut f1, &mut f2) + beep(t, 0.33, 0.25, 1150.0, 18.0, &mut f3, &mut f4) + beep(t, 0.66, 0.25, 1150.0, 18.0, &mut f1, &mut f2),
            Cue::Track => {
                // The tracks: a squeal whose FM index wanders with
                // slow noise (a dense, shifting sideband cloud, never
                // a note), coming and going, over a rumble from the
                // ground. Looped while the tank moves; the loop's seam
                // is crossfaded below.
                let wander = lp.run(noise.next(), 6.0) * 40.0;
                let index = 9.0 + 5.0 * wander.clamp(-1.0, 1.0);
                let squeal = f1.run(3000.0 * p, 0.1, index);
                let gate = (lp2.run(noise.next(), 1.5) * 60.0).clamp(0.0, 1.0);
                // The rumble band-passed 50 to 200 Hz: the low-passed
                // noise less its own slower low-pass.
                let low = lp3.run(noise.next(), 200.0);
                let rumble = (low - lp4.run(low, 50.0)) * 6.0;
                let clank = click(t % 0.37, 0.0, &mut f2, 2200.0, 2.7, 3.0, 0.006) * 0.3;
                squeal * 0.7 * gate + rumble * 0.6 + clank
            }
            Cue::Static => {
                // A link lost: bright noise stuttering, a falling chirp.
                let gate = if o1.sine(23.0) > -0.3 { 1.0 } else { 0.2 };
                let chirp = f1.run(sweep(t, 2400.0, 900.0, 0.4), 0.5, 2.0) * 0.15;
                (crack(&mut noise, &mut lp, 1500.0) * gate + chirp) * attack(t, 0.01) * decay(t, 4.0)
            }
            Cue::Capture => notes(t, &mut f1, &[660.0, 880.0], 0.17, 7.0),
            Cue::Lost => notes(t, &mut f1, &[880.0, 587.3], 0.17, 7.0),
            Cue::Notice => notes(t, &mut f1, &[784.0, 1046.5], 0.12, 6.0),
            Cue::Swarm => {
                let hz = 600.0 + 150.0 * o2.sine(4.0);
                o1.tri(hz) * 0.5 * attack(t, 0.05) * decay((t - 0.9).max(0.0), 12.0)
            }
            Cue::Refused => lp.run(if o1.sine(140.0) > 0.0 { 1.0 } else { -1.0 }, 900.0) * attack(t, 0.005) * decay(t, 14.0),
            // Four notes, a fifth of a second apart: up a major chord, or
            // down a minor one.
            Cue::Win => notes(t, &mut f1, &[523.3, 659.3, 784.0, 1046.5], 0.22, 3.0),
            Cue::Lose => notes(t, &mut f1, &[440.0, 349.2, 293.7, 220.0], 0.22, 3.0),
        };
        out.push(s);
    }
    let mut out = decimate(&out);
    // The blasts are driven into a soft clip: a short transient on top
    // of a long roll otherwise normalises to a click and a whisper.
    let drive = match cue {
        Cue::Boom | Cue::Mine | Cue::Wreck | Cue::Gun | Cue::RocketHit => 3.0,
        Cue::Pop | Cue::Cannon | Cue::PillboxFire | Cue::Intercept | Cue::Crash | Cue::Thud => 2.0,
        Cue::Strike | Cue::Clang | Cue::Ricochet => 1.2,
        Cue::Track => 1.5,
        _ => 0.0,
    };
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        for s in &mut out {
            *s /= peak;
            if drive > 0.0 {
                *s = (*s * drive).tanh() / drive.tanh();
            }
            *s *= 0.9 * level(cue);
        }
    }
    // The track loops: its last 150 ms are faded into its first, so the
    // seam has no click, and it is not faded out.
    if cue == Cue::Track {
        let x = (RATE as usize * 15 / 100).min(out.len() / 2);
        let tail: Vec<f32> = out.split_off(out.len() - x);
        for (i, t) in tail.into_iter().enumerate() {
            let w = i as f32 / x as f32;
            out[i] = out[i] * w + t * (1.0 - w);
        }
        return out;
    }
    // Ten milliseconds of fade at the end: no click where it stops.
    let fade = (RATE as usize / 100).min(out.len());
    let len = out.len();
    for (i, s) in out[len - fade..].iter_mut().enumerate() {
        *s *= 1.0 - i as f32 / fade as f32;
    }
    out
}

/// From the engine's rate to `RATE`: a windowed-sinc low-pass at 20 kHz
/// (63 taps, Blackman), then every second sample.
fn decimate(x: &[f32]) -> Vec<f32> {
    const TAPS: usize = 63;
    let fc = 20_000.0 / ENGINE as f32;
    let h: Vec<f32> = (0..TAPS)
        .map(|i| {
            let m = i as f32 - (TAPS - 1) as f32 / 2.0;
            let sinc = if m == 0.0 { 2.0 * fc } else { (2.0 * std::f32::consts::PI * fc * m).sin() / (std::f32::consts::PI * m) };
            let w = 0.42 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (TAPS - 1) as f32).cos() + 0.08 * (4.0 * std::f32::consts::PI * i as f32 / (TAPS - 1) as f32).cos();
            sinc * w
        })
        .collect();
    let half = TAPS / 2;
    (0..x.len() / OVERSAMPLE)
        .map(|k| {
            let c = k * OVERSAMPLE;
            h.iter().enumerate().map(|(i, hi)| (c + i).checked_sub(half).and_then(|j| x.get(j)).map(|v| v * hi).unwrap_or(0.0)).sum()
        })
        .collect()
}

/// How loud a cue is beside the others, before distance. The world is
/// loudest; the interface is there to be noticed, not to cover it.
fn level(cue: Cue) -> f32 {
    match cue {
        Cue::Capture | Cue::Lost | Cue::Notice | Cue::Refused | Cue::Swarm | Cue::Win | Cue::Lose => 0.45,
        Cue::Incoming | Cue::Launch | Cue::Rocket | Cue::Build | Cue::Reload | Cue::Static | Cue::Breech | Cue::Empty | Cue::Alarm => 0.6,
        Cue::Tick => 0.35,
        Cue::Track => 0.5,
        Cue::Ricochet => 0.75,
        _ => 1.0,
    }
}

/// Equal-power gains (left, right) for a pan from -1 (left) to 1.
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let a = (pan.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (a.cos(), a.sin())
}

/// The pan a bucket stands for.
fn bucket_pan(b: usize) -> f32 {
    b as f32 / (PANS - 1) as f32 * 2.0 - 1.0
}

fn pan_bucket(pan: f32) -> usize {
    (((pan.clamp(-1.0, 1.0) + 1.0) / 2.0) * (PANS - 1) as f32).round() as usize
}

/// A mono sound as a 16-bit stereo WAV file, panned.
pub fn wav(mono: &[f32], pan: f32) -> Vec<u8> {
    let (l, r) = pan_gains(pan);
    let data_len = (mono.len() * 4) as u32;
    let mut b = Vec::with_capacity(44 + data_len as usize);
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data_len).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes()); // PCM
    b.extend_from_slice(&2u16.to_le_bytes()); // stereo
    b.extend_from_slice(&RATE.to_le_bytes());
    b.extend_from_slice(&(RATE * 4).to_le_bytes());
    b.extend_from_slice(&4u16.to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data_len.to_le_bytes());
    for s in mono {
        for g in [l, r] {
            let v = (s * g * 32767.0).clamp(-32768.0, 32767.0) as i16;
            b.extend_from_slice(&v.to_le_bytes());
        }
    }
    b
}

/// How a sound at `at` (tiles) is heard with `screen` on screen: its
/// gain, its pan, and whether it is far enough for the dull take. On
/// screen the gain is 1 and the pan runs from the left edge (-1) to the
/// right (1); off screen the gain falls smoothly with the distance past
/// the nearest edge toward FLOOR, and the pan stays on that side.
pub fn place(screen: Screen, at: V2) -> (f32, f32, bool) {
    let (dx, dy) = (at.x - screen.centre.x, at.y - screen.centre.y);
    let out_x = (dx.abs() - screen.half.x).max(0.0);
    let out_y = (dy.abs() - screen.half.y).max(0.0);
    let out = (out_x * out_x + out_y * out_y).sqrt();
    let gain = FLOOR + (1.0 - FLOOR) / (1.0 + (out / EDGE_HALF).powi(2));
    let pan = (dx / screen.half.x.max(1.0)).clamp(-1.0, 1.0);
    (gain, pan, out > FAR_TAKE)
}

/// A take as heard from far off: low-passed twice, the highs gone.
fn dull(mono: &[f32]) -> Vec<f32> {
    let (mut a, mut b) = (Lowpass(0.0), Lowpass(0.0));
    mono.iter().map(|x| b.run_at(a.run_at(*x, FAR_CUTOFF, RATE), FAR_CUTOFF, RATE)).collect()
}

/// `--write-sounds DIR`: every cue, centred, as DIR/NAME.wav.
pub fn write_all(dir: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for cue in Cue::ALL {
        let path = std::path::Path::new(dir).join(format!("{}.wav", cue.name()));
        let mono = synth(cue, 0);
        std::fs::write(&path, wav(&mono, 0.0))?;
        println!("{:<14} {:.2} s  {}", cue.name(), mono.len() as f32 / RATE as f32, path.display());
    }
    Ok(())
}

// ----------------------------------------------------------- the window

/// Where an entity is in this snapshot, if it is in it.
pub fn find(obs: &Observation, id: EntityId) -> Option<V2> {
    obs.me.iter().chain(obs.tanks.iter()).find(|t| t.id == id).map(|t| t.pos)
        .or_else(|| obs.shells.iter().find(|s| s.id == id).map(|s| s.from))
        .or_else(|| obs.drones.iter().find(|d| d.id == id).map(|d| d.pos))
        .or_else(|| obs.pillboxes.iter().find(|p| p.id == id).map(|p| p.pos))
        .or_else(|| obs.depots.iter().find(|d| d.id == id).map(|d| d.pos))
}

pub struct SoundPlugin {
    pub muted: bool,
}

impl Plugin for SoundPlugin {
    fn build(&self, app: &mut App) {
        // Six decibels of headroom under full scale: twenty voices at
        // once sum, and a sum of FM voices can spike where sidebands meet.
        app.insert_resource(Sounds { muted: self.muted, ..default() })
            .insert_resource(GlobalVolume::new(Volume::Linear(HEADROOM)))
            .add_systems(Update, (play, tracks));
    }
}

#[derive(Resource, Default)]
struct Sounds {
    muted: bool,
    mono: HashMap<(Cue, usize), Arc<Vec<f32>>>,
    mixes: HashMap<(Cue, usize, usize, bool), Handle<AudioSource>>,
    /// The next take of each cue, in turn.
    take: HashMap<Cue, usize>,
    /// When each cue last played, and how loud.
    last: HashMap<Cue, (Instant, f32)>,
    waiting: Vec<(Cue, EntityId, f32, Instant)>,
    /// Hostile shells already whistled for.
    heard: BTreeSet<EntityId>,
    /// When the capture clock last ticked.
    ticked: Option<Instant>,
}

impl Sounds {
    fn mix(&mut self, cue: Cue, bucket: usize, far: bool, sources: &mut Assets<AudioSource>) -> Handle<AudioSource> {
        let take = self.take.entry(cue).or_insert(0);
        let variant = *take;
        *take = (variant + 1) % VARIANTS;
        if let Some(h) = self.mixes.get(&(cue, variant, bucket, far)) {
            return h.clone();
        }
        let mono = self.mono.entry((cue, variant)).or_insert_with(|| Arc::new(synth(cue, variant))).clone();
        let bytes = if far { wav(&dull(&mono), bucket_pan(bucket)) } else { wav(&mono, bucket_pan(bucket)) };
        let h = sources.add(AudioSource { bytes: bytes.into() });
        self.mixes.insert((cue, variant, bucket, far), h.clone());
        h
    }
}

/// A sound playing; it despawns itself when done.
#[derive(Component)]
struct Voice;

fn play(
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    mut view: ResMut<View>,
    mut sources: ResMut<Assets<AudioSource>>,
    keys: Res<ButtonInput<KeyCode>>,
    camera: Query<(&Transform, &Projection), With<MainCamera>>,
    window: Query<&Window, With<PrimaryWindow>>,
    voices: Query<(), With<Voice>>,
) {
    if keys.just_pressed(KeyCode::KeyM) {
        sounds.muted = !sounds.muted;
        info!("sound {}", if sounds.muted { "muted" } else { "on" });
        let tick = view.last_tick;
        view.log.push_back((tick, if sounds.muted { "sound off (M)".into() } else { "sound on (M)".into() }));
    }
    let queued: Vec<(Cue, Where, f32)> = std::mem::take(&mut view.sounds);
    if sounds.muted {
        sounds.waiting.clear();
        return;
    }
    let Some(obs) = view.obs.as_ref() else { return };
    // What is on screen, in tiles: the camera's centre, and the window's
    // size through the zoom.
    let Ok((cam, proj)) = camera.single() else { return };
    let scale = match proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let (w, h) = window.single().map(|w| (w.width(), w.height())).unwrap_or((1280.0, 720.0));
    let screen = Screen { centre: crate::to_tile(cam.translation.truncate()), half: V2::new(w * scale * 0.5 / crate::TILE, h * scale * 0.5 / crate::TILE) };
    let now = Instant::now();

    // Everything to play this frame: what was queued, what was waiting
    // for its entity and has it now, and a whistle for each hostile
    // shell seen for the first time.
    let mut due: Vec<(Cue, Option<V2>, f32)> = Vec::new();
    for (cue, at, gain) in queued {
        match at {
            Where::Ui => due.push((cue, None, gain)),
            Where::At(p) => due.push((cue, Some(p), gain)),
            Where::Entity(id) => match find(obs, id) {
                Some(p) => due.push((cue, Some(p), gain)),
                None => sounds.waiting.push((cue, id, gain, now)),
            },
        }
    }
    // A sound whose place never shows is in the fog: the sim told of it
    // and not where (a hostile drone downed out of sight). It is heard
    // from nowhere in particular, as background, and gives nothing away.
    let mut unplaced: Vec<(Cue, f32)> = Vec::new();
    let waiting = std::mem::take(&mut sounds.waiting);
    for (cue, id, gain, since) in waiting {
        match find(obs, id) {
            Some(p) => due.push((cue, Some(p), gain)),
            None if now.duration_since(since) < WAIT => sounds.waiting.push((cue, id, gain, since)),
            None => unplaced.push((cue, gain)),
        }
    }
    let team = |p: PlayerId| obs.players.iter().find(|v| v.id == p).map(|v| v.team);
    let mine = team(obs.player);
    for s in &obs.shells {
        let hostile = s.owner != obs.player && (mine.is_none() || team(s.owner) != mine);
        if hostile && sounds.heard.insert(s.id) {
            due.push((Cue::Incoming, Some(s.impact), 0.8));
        }
    }
    let live: BTreeSet<EntityId> = obs.shells.iter().map(|s| s.id).collect();
    sounds.heard.retain(|id| live.contains(id));
    // The clock, once a second, while a depot or a pillbox is being
    // taken by you: it ticks where the thing is.
    let taking = obs.depots.iter().filter(|d| d.capturer == Some(obs.player) && d.capture > 0).map(|d| d.pos)
        .chain(obs.pillboxes.iter().filter(|b| b.capturer == Some(obs.player) && b.capture > 0).map(|b| b.pos))
        .next();
    match taking {
        Some(at) if sounds.ticked.map(|t| now.duration_since(t) >= TICK_EVERY).unwrap_or(true) => {
            sounds.ticked = Some(now);
            due.push((Cue::Tick, Some(at), 1.0));
        }
        Some(_) => {}
        None => sounds.ticked = None,
    }

    let mut playing = voices.iter().count();
    // Loudest first, so a full mixer drops the far ones.
    let mut placed: Vec<(Cue, f32, f32, bool)> = due
        .into_iter()
        .map(|(cue, at, gain)| match at {
            None => (cue, gain, 0.0, false),
            Some(p) => {
                let (g, pan, far) = place(screen, p);
                (cue, gain * g, pan, far)
            }
        })
        .chain(unplaced.into_iter().map(|(cue, gain)| (cue, gain * FLOOR, 0.0, true)))
        .collect();
    placed.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (cue, gain, pan, far) in placed {
        if playing >= VOICES {
            break;
        }
        // A cue just played is not played again unless this one is
        // louder: a far gun does not mask a near one.
        if sounds.last.get(&cue).map(|(t, g)| now.duration_since(*t) < REPEAT && gain <= *g).unwrap_or(false) {
            continue;
        }
        sounds.last.insert(cue, (now, gain));
        debug!("sound {} gain {:.2} pan {:+.2}{}", cue.name(), gain, pan, if far { " far" } else { "" });
        let h = sounds.mix(cue, pan_bucket(pan), far, &mut sources);
        // A crowded mixer takes each newcomer a little quieter: voices
        // of random phase add about as the square root of their number.
        let crowd = 1.0 / (1.0 + playing as f32 / 8.0).sqrt();
        commands.spawn((AudioPlayer::new(h), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(gain * crowd)), Voice));
        playing += 1;
    }
}

/// The player's tracks: one looping voice, started once, its volume
/// following the tank's speed (eased, so starting and stopping are not
/// steps), silent when muted or dead.
#[derive(Component)]
struct Track;

fn tracks(
    mut commands: Commands,
    mut sounds: ResMut<Sounds>,
    view: Res<View>,
    mut sources: ResMut<Assets<AudioSource>>,
    mut sinks: Query<&mut AudioSink, With<Track>>,
    started: Query<(), With<Track>>,
    time: Res<Time>,
    mut level: Local<f32>,
) {
    if started.is_empty() {
        let h = sounds.mix(Cue::Track, PANS / 2, false, &mut sources);
        commands.spawn((AudioPlayer::new(h), PlaybackSettings::LOOP.with_volume(Volume::Linear(0.0)), Track));
        return;
    }
    let speed = view.obs.as_ref().and_then(|o| o.me.as_ref()).filter(|t| t.alive).map(|t| t.speed.abs()).unwrap_or(0.0);
    let want = if sounds.muted { 0.0 } else { (speed / treadcore_sim::TANK_SPEED).min(1.0) };
    // Up in a quarter-second, down in a half.
    let rate = if want > *level { 4.0 } else { 2.0 };
    *level += (want - *level) * (rate * time.delta_secs()).min(1.0);
    for mut sink in &mut sinks {
        sink.set_volume(Volume::Linear(*level));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cue_is_a_sound() {
        for cue in Cue::ALL {
            for v in 0..VARIANTS {
                let s = synth(cue, v);
                let secs = s.len() as f32 / RATE as f32;
                assert!((0.05..=3.0).contains(&secs), "{:?}: {} s", cue, secs);
                assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 0.9001), "{:?}/{} clips or is not a number", cue, v);
                // Loudness where it sounds: the breech waits for the shot.
                let active: Vec<f32> = s.iter().copied().filter(|x| x.abs() > 1e-4).collect();
                let rms = (active.iter().map(|x| x * x).sum::<f32>() / active.len().max(1) as f32).sqrt();
                assert!(rms > 0.02, "{:?}/{} is nearly silent (rms {})", cue, v, rms);
                if cue != Cue::Track {
                    assert!(s.last().unwrap().abs() < 0.01, "{:?}/{} ends with a click", cue, v);
                }
            }
        }
    }

    #[test]
    fn same_sound_every_time_and_takes_differ() {
        assert_eq!(synth(Cue::Boom, 0), synth(Cue::Boom, 0));
        assert_ne!(synth(Cue::Cannon, 0), synth(Cue::Cannon, 1));
    }

    /// The FM operator at index 0 is a plain sine at the carrier; with
    /// an index, the sidebands take power from it (J0(I) < 1).
    #[test]
    fn fm_operator() {
        let mut op = Fm::NEW;
        let pure: Vec<f32> = (0..ENGINE).map(|_| op.run(441.0, 1.41, 0.0)).collect();
        let mut sine = Osc(0.0);
        assert!(pure.iter().all(|x| (x - sine.sine(441.0)).abs() < 1e-3));
        // Correlation with the carrier: J0(2.4) is about 0 (the first
        // zero of J0), so at that index the carrier all but vanishes.
        let carrier = |index: f32| {
            let (mut op, mut c) = (Fm::NEW, Osc(0.0));
            (0..ENGINE).map(|_| op.run(441.0, 1.41, index) * c.sine(441.0)).sum::<f32>() / ENGINE as f32 * 2.0
        };
        assert!((carrier(0.0) - 1.0).abs() < 0.01);
        assert!(carrier(2.405).abs() < 0.05, "J0(2.405) is a zero; got {}", carrier(2.405));
        assert!((carrier(1.0) - 0.765).abs() < 0.05, "J0(1) is 0.765; got {}", carrier(1.0));
    }

    #[test]
    fn wav_is_well_formed() {
        let w = wav(&[0.0, 0.5, -0.5], 0.0);
        assert_eq!(&w[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(w[4..8].try_into().unwrap()) as usize, w.len() - 8);
        assert_eq!(&w[36..40], b"data");
        assert_eq!(w.len(), 44 + 3 * 4);
        // Hard left: nothing in the right channel.
        let left = wav(&[0.5], -1.0);
        assert_ne!(i16::from_le_bytes([left[44], left[45]]), 0);
        assert_eq!(i16::from_le_bytes([left[46], left[47]]), 0);
    }

    #[test]
    fn on_screen_is_full_and_the_whole_map_is_heard() {
        let screen = Screen { centre: V2::new(100.0, 100.0), half: V2::new(50.0, 30.0) };
        // Anywhere on screen: full, panned edge to edge.
        assert_eq!(place(screen, V2::new(100.0, 100.0)), (1.0, 0.0, false));
        let (g, pan, far) = place(screen, V2::new(75.0, 125.0));
        assert_eq!((g, pan, far), (1.0, -0.5, false));
        assert_eq!(place(screen, V2::new(150.0, 100.0)).1, 1.0);
        // Off screen: half way to the floor at EDGE_HALF past the edge,
        // the far take past FAR_TAKE, held to that side.
        let (g, pan, far) = place(screen, V2::new(150.0 + EDGE_HALF, 100.0));
        assert!((g - (FLOOR + (1.0 - FLOOR) / 2.0)).abs() < 1e-6 && pan == 1.0 && far);
        // Across the map: quiet, never silent, and always quieter further.
        let mut last = 1.0;
        for d in [0.0, 5.0, 20.0, 60.0, 200.0, 1000.0] {
            let (g, _, _) = place(screen, V2::new(100.0, 130.0 + d));
            assert!(g <= last && g >= FLOOR, "{} tiles out: {}", d, g);
            last = g;
        }
        let (l, r) = pan_gains(0.0);
        assert!((l * l + r * r - 1.0).abs() < 1e-6 && (l - r).abs() < 1e-6);
        assert_eq!(pan_bucket(-1.0), 0);
        assert_eq!(pan_bucket(1.0), PANS - 1);
        assert_eq!(pan_bucket(0.0), PANS / 2);
    }

    /// Carson's rule holds: a carrier's band stays under the engine's
    /// Nyquist.
    #[test]
    fn fm_stays_under_nyquist() {
        for (fc, ratio, i) in [(9000.0f32, 1.5f32, 6.0f32), (6000.0, 1.4, 4.0), (4000.0, 1.575, 5.0), (3000.0, 0.1, 14.0), (2200.0, 2.7, 3.0), (120.0, 0.5, 3.0)] {
            let allowed = i.min(Fm::max_index(fc, ratio));
            assert!(fc + (allowed + 1.0) * fc * ratio <= ENGINE as f32 / 2.0 + 1.0, "{} Hz 1:{} I {}", fc, ratio, allowed);
        }
        // The recipe's doo keeps its index; the klank keeps some.
        assert!(Fm::max_index(4000.0, 1.575) >= 5.0);
        assert!(Fm::max_index(9000.0, 1.5) > 1.5);
        assert_eq!(Fm::max_index(120.0, 0.5).min(3.0), 3.0);
    }

    /// The decimator passes the audible band and stops what would fold.
    #[test]
    fn decimation_band_limits() {
        let tone = |hz: f32| {
            let mut o = Osc(0.0);
            let x: Vec<f32> = (0..ENGINE).map(|_| o.sine(hz)).collect();
            let y = decimate(&x);
            (y[1000..].iter().map(|v| v * v).sum::<f32>() / (y.len() - 1000) as f32 * 2.0).sqrt()
        };
        assert!((tone(1000.0) - 1.0).abs() < 0.02, "1 kHz passes: {}", tone(1000.0));
        assert!(tone(30_000.0) < 0.01, "30 kHz would fold to 14 kHz: {}", tone(30_000.0));
    }

    /// The track's loop has no seam: its end runs into its start.
    #[test]
    fn track_loops_without_a_seam() {
        let s = synth(Cue::Track, 0);
        let step = (s[0] - s[s.len() - 1]).abs();
        let typical = s.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / s.len() as f32;
        assert!(step < typical * 20.0 + 0.02, "seam step {} against typical {}", step, typical);
    }

    /// The far take keeps the low end and loses the highs.
    #[test]
    fn far_is_dull() {
        let near = synth(Cue::Cannon, 0);
        let far = dull(&near);
        let highs = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>();
        assert!(highs(&far) < highs(&near) * 0.2);
    }
}
