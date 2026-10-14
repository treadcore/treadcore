//! Sound (roadmap step 9): the one thing the HUD cannot convey, a shell
//! launched where you cannot see.
//!
//! No sound files. Every sound is synthesised here at start-up from a
//! few lines of arithmetic (a swept sine, filtered noise, a handful of
//! inharmonic partials), the way everything drawn is a rectangle: the
//! overlay is the look, and this is its sound. `treadcore --write-sounds
//! DIR` (`make sounds`) writes each one as a WAV file to listen to.
//!
//! Where a sound comes from is the event's place in the world, heard
//! from the middle of the view: louder the nearer, panned left or right
//! by how far off-centre (`place`). The left/right mixes are made once
//! per cue in five pan positions, as 16-bit WAV in memory, and handed to
//! Bevy's audio as ordinary `AudioSource`s; a voice is an entity that
//! despawns when its sound ends.
//!
//! What is heard is what the player may know (the sim's `event_visible`),
//! plus one thing the window works out for itself: a hostile shell the
//! first time a snapshot shows it, which whistles where it will land.
//! Interface cues (an order refused, a capture, the swarm, the end) are
//! not placed: they sound in the middle.
//!
//! `M` mutes and unmutes; `--mute` starts muted.

use crate::render::MainCamera;
use crate::session::View;
use bevy::audio::{AudioPlayer, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};
use treadcore_sim::{EntityId, Observation, PlayerId, V2};

/// Samples a second. Nothing here has much above 5 kHz.
pub const RATE: u32 = 22_050;
/// Pan positions, left to right.
const PANS: usize = 5;
/// At most this many sounds at once; past it, the quiet ones are dropped.
const VOICES: usize = 20;
/// The same cue twice inside this is one sound (sixteen guns at once).
const REPEAT: Duration = Duration::from_millis(60);
/// A cue waiting for its entity to appear in a snapshot waits this long.
const WAIT: Duration = Duration::from_millis(300);

/// Distance, in tiles, at which a sound is at half strength; past
/// `HEAR_FAR` it is not played. The pan is full at `PAN_FULL` tiles off
/// centre. The view is about a hundred tiles across at the default zoom.
const HEAR_HALF: f32 = 25.0;
const HEAR_FAR: f32 = 120.0;
const PAN_FULL: f32 = 50.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Cue {
    /// The gun, indirect: a heavy thump and a rumble.
    Gun,
    /// The gun, direct: a crack.
    Cannon,
    /// An indirect shell lands.
    Boom,
    /// A direct shell lands.
    Pop,
    Mine,
    /// A drone leaves the bay.
    Launch,
    /// An interceptor gets its target.
    Intercept,
    /// A tank is hit.
    Clang,
    /// A tank is destroyed.
    Wreck,
    PillboxFire,
    /// A hostile shell, first seen: a falling whistle.
    Incoming,
    /// The squad builds or harvests.
    Build,
    /// Interface: something taken for you.
    Capture,
    /// Interface: something of yours taken.
    Lost,
    /// Interface: the swarm is inbound.
    Swarm,
    /// Interface: an order refused.
    Refused,
    Win,
    Lose,
}

impl Cue {
    pub const ALL: [Cue; 18] = [
        Cue::Gun, Cue::Cannon, Cue::Boom, Cue::Pop, Cue::Mine, Cue::Launch, Cue::Intercept, Cue::Clang, Cue::Wreck,
        Cue::PillboxFire, Cue::Incoming, Cue::Build, Cue::Capture, Cue::Lost, Cue::Swarm, Cue::Refused, Cue::Win, Cue::Lose,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Cue::Gun => "gun",
            Cue::Cannon => "cannon",
            Cue::Boom => "boom",
            Cue::Pop => "pop",
            Cue::Mine => "mine",
            Cue::Launch => "launch",
            Cue::Intercept => "intercept",
            Cue::Clang => "clang",
            Cue::Wreck => "wreck",
            Cue::PillboxFire => "pillbox-fire",
            Cue::Incoming => "incoming",
            Cue::Build => "build",
            Cue::Capture => "capture",
            Cue::Lost => "lost",
            Cue::Swarm => "swarm",
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
        let a = 1.0 - (-2.0 * std::f32::consts::PI * cutoff / RATE as f32).exp();
        self.0 += a * (x - self.0);
        self.0
    }
}

/// A sine whose frequency moves; the phase is kept, so a sweep is smooth.
struct Osc(f32);

impl Osc {
    fn sine(&mut self, hz: f32) -> f32 {
        self.0 = (self.0 + hz / RATE as f32).fract();
        (self.0 * std::f32::consts::TAU).sin()
    }
    fn tri(&mut self, hz: f32) -> f32 {
        self.0 = (self.0 + hz / RATE as f32).fract();
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

/// The explosion every boom is made of: low-passed noise with a falling
/// cutoff, and a falling sub-bass thump under it.
fn explosion(t: f32, n: &mut Noise, lp: &mut Lowpass, sub: &mut Osc, size: f32) -> f32 {
    let top = 2400.0 / size;
    let cutoff = sweep(t, top, 120.0, 0.6 * size);
    // Filtered noise loses power with its cutoff; put it back, so the
    // roll lasts as long as the envelope says and not a tenth of it.
    let body = lp.run(n.next(), cutoff) * (top / cutoff).sqrt() * 0.6 * decay(t, 3.5 / size) * attack(t, 0.004);
    let thump = sub.sine(sweep(t, 70.0, 28.0, 0.5 * size)) * decay(t, 5.0 / size) * attack(t, 0.003);
    body * 2.2 + thump * 0.8
}

/// A cue's samples, mono, at `RATE`, normalised to a peak of 0.9.
pub fn synth(cue: Cue) -> Vec<f32> {
    let secs = match cue {
        Cue::Gun => 0.9,
        Cue::Cannon => 0.45,
        Cue::Boom => 1.4,
        Cue::Pop => 0.5,
        Cue::Mine => 1.3,
        Cue::Launch => 0.8,
        Cue::Intercept => 0.3,
        Cue::Clang => 0.7,
        Cue::Wreck => 2.2,
        Cue::PillboxFire => 0.4,
        Cue::Incoming => 1.1,
        Cue::Build => 0.18,
        Cue::Capture | Cue::Lost => 0.5,
        Cue::Swarm => 1.3,
        Cue::Refused => 0.2,
        Cue::Win | Cue::Lose => 1.3,
    };
    let n = (secs * RATE as f32) as usize;
    let mut noise = Noise(0x9E37_79B9 ^ cue as u32);
    let (mut lp, mut lp2) = (Lowpass(0.0), Lowpass(0.0));
    let (mut o1, mut o2, mut o3, mut o4) = (Osc(0.0), Osc(0.0), Osc(0.0), Osc(0.0));
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / RATE as f32;
        let s = match cue {
            Cue::Gun => {
                let blast = lp.run(noise.next(), sweep(t, 3000.0, 200.0, 0.25)) * decay(t, 14.0);
                let thump = o1.sine(sweep(t, 95.0, 34.0, 0.4)) * decay(t, 6.0);
                let rumble = lp2.run(noise.next(), 180.0) * decay(t, 3.0);
                (blast * 2.0 + thump + rumble * 2.5) * attack(t, 0.002)
            }
            Cue::Cannon => {
                let crack = (noise.next() - lp.run(noise.next(), 800.0)) * decay(t, 45.0);
                let body = o1.sine(sweep(t, 160.0, 60.0, 0.2)) * decay(t, 11.0);
                (crack + body) * attack(t, 0.001)
            }
            Cue::Boom => explosion(t, &mut noise, &mut lp, &mut o1, 1.0),
            Cue::Pop => explosion(t, &mut noise, &mut lp, &mut o1, 0.35),
            Cue::Mine => {
                let crack = (noise.next() - lp2.run(noise.next(), 600.0)) * decay(t, 60.0);
                explosion(t, &mut noise, &mut lp, &mut o1, 0.9) + crack * 0.6
            }
            Cue::Launch => {
                // A motor winding up: a buzz that rises, chopped by the rotor.
                let hz = sweep(t, 180.0, 520.0, 0.6);
                let buzz = o1.tri(hz) * 0.6 + o2.tri(hz * 2.01) * 0.25;
                let rotor = 0.75 + 0.25 * o3.sine(sweep(t, 30.0, 90.0, 0.6));
                let hiss = lp.run(noise.next(), 3000.0) * 0.3;
                (buzz * rotor + hiss) * attack(t, 0.08) * decay((t - 0.45).max(0.0), 9.0)
            }
            Cue::Intercept => {
                let pop = (noise.next() - lp.run(noise.next(), 1500.0)) * decay(t, 55.0);
                let zap = o1.sine(sweep(t, 1400.0, 320.0, 0.15)) * decay(t, 22.0);
                (pop + zap * 0.7) * attack(t, 0.001)
            }
            Cue::Clang => {
                // Armour: inharmonic partials that die at different rates.
                let ring = o1.sine(311.0) * decay(t, 6.0) + o2.sine(467.0) * decay(t, 8.0) * 0.7 + o3.sine(781.0) * decay(t, 11.0) * 0.5 + o4.sine(1173.0) * decay(t, 15.0) * 0.35;
                let hit = lp.run(noise.next(), 2500.0) * decay(t, 70.0) * 2.0;
                (ring * 0.6 + hit) * attack(t, 0.001)
            }
            Cue::Wreck => {
                let big = explosion(t, &mut noise, &mut lp, &mut o1, 1.6);
                let ring = (o2.sine(233.0) * decay(t, 4.0) + o3.sine(347.0) * decay(t, 5.5) * 0.6) * 0.35;
                // Things still cooking off after the blast.
                let crackle = if noise.next() > 0.995 && t > 0.2 { decay(t - 0.2, 2.0) * 0.8 } else { 0.0 };
                big + ring + crackle * noise.next()
            }
            Cue::PillboxFire => {
                let crack = (noise.next() - lp.run(noise.next(), 1200.0)) * decay(t, 50.0);
                let body = o1.sine(sweep(t, 240.0, 90.0, 0.15)) * decay(t, 16.0);
                (crack + body * 0.8) * attack(t, 0.001)
            }
            Cue::Incoming => {
                // A falling whistle, louder as it falls, cut at the end
                // (the boom is the impact's own sound).
                let hz = sweep(t, 1500.0, 520.0, 1.05) * (1.0 + 0.004 * o2.sine(7.0));
                let air = lp.run(noise.next(), hz * 1.5) * 0.15;
                (o1.sine(hz) * 0.5 + air) * attack(t, 0.6) * (1.0 - attack((t - 1.0).max(0.0), 0.1))
            }
            Cue::Build => {
                let knock = o1.sine(sweep(t, 460.0, 300.0, 0.05)) * decay(t, 38.0);
                let tick = lp.run(noise.next(), 3500.0) * decay(t, 120.0);
                (knock + tick) * attack(t, 0.001)
            }
            Cue::Capture | Cue::Lost => {
                let (a, b) = if cue == Cue::Capture { (660.0, 880.0) } else { (880.0, 587.3) };
                let hz = if t < 0.17 { a } else { b };
                let local = if t < 0.17 { t } else { t - 0.17 };
                o1.tri(hz) * attack(local, 0.005) * decay(local, 7.0) * 0.6
            }
            Cue::Swarm => {
                let hz = 600.0 + 150.0 * o2.sine(4.0);
                o1.tri(hz) * 0.5 * attack(t, 0.05) * decay((t - 0.9).max(0.0), 12.0)
            }
            Cue::Refused => lp.run(if o1.sine(140.0) > 0.0 { 1.0 } else { -1.0 }, 900.0) * attack(t, 0.005) * decay(t, 14.0),
            Cue::Win | Cue::Lose => {
                // Four notes, a quarter-second apart: up a major chord, or
                // down a minor one.
                let notes: [f32; 4] = if cue == Cue::Win { [523.3, 659.3, 784.0, 1046.5] } else { [440.0, 349.2, 293.7, 220.0] };
                let k = ((t / 0.22) as usize).min(3);
                let local = t - k as f32 * 0.22;
                let len = if k == 3 { 3.0 } else { 9.0 };
                o1.tri(notes[k]) * 0.5 * attack(local, 0.006) * decay(local, len)
            }
        };
        out.push(s);
    }
    // The blasts are driven into a soft clip: a short transient on top
    // of a long roll otherwise normalises to a click and a whisper.
    let drive = match cue {
        Cue::Boom | Cue::Mine | Cue::Wreck | Cue::Gun => 3.0,
        Cue::Pop | Cue::Cannon | Cue::PillboxFire | Cue::Clang | Cue::Intercept => 1.5,
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
    // Ten milliseconds of fade at the end: no click where it stops.
    let fade = (RATE as usize / 100).min(out.len());
    let len = out.len();
    for (i, s) in out[len - fade..].iter_mut().enumerate() {
        *s *= 1.0 - i as f32 / fade as f32;
    }
    out
}

/// How loud a cue is beside the others, before distance. The world is
/// loudest; the interface is there to be noticed, not to cover it.
fn level(cue: Cue) -> f32 {
    match cue {
        Cue::Capture | Cue::Lost | Cue::Refused | Cue::Swarm | Cue::Win | Cue::Lose => 0.45,
        Cue::Incoming | Cue::Launch | Cue::Build => 0.6,
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

/// How a sound at `at` is heard from `listener` (both in tiles): its
/// gain and pan, or None when it is too far to hear.
pub fn place(listener: V2, at: V2) -> Option<(f32, f32)> {
    let (dx, dy) = (at.x - listener.x, at.y - listener.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d > HEAR_FAR {
        return None;
    }
    let gain = 1.0 / (1.0 + (d / HEAR_HALF).powi(2));
    Some((gain, (dx / PAN_FULL).clamp(-1.0, 1.0)))
}

/// `--write-sounds DIR`: every cue, centred, as DIR/NAME.wav.
pub fn write_all(dir: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)?;
    for cue in Cue::ALL {
        let path = std::path::Path::new(dir).join(format!("{}.wav", cue.name()));
        let mono = synth(cue);
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
        app.insert_resource(Sounds { muted: self.muted, ..default() }).add_systems(Update, play);
    }
}

#[derive(Resource, Default)]
struct Sounds {
    muted: bool,
    mono: HashMap<Cue, Arc<Vec<f32>>>,
    mixes: HashMap<(Cue, usize), Handle<AudioSource>>,
    last: HashMap<Cue, Instant>,
    waiting: Vec<(Cue, EntityId, f32, Instant)>,
    /// Hostile shells already whistled for.
    heard: BTreeSet<EntityId>,
}

impl Sounds {
    fn mix(&mut self, cue: Cue, bucket: usize, sources: &mut Assets<AudioSource>) -> Handle<AudioSource> {
        if let Some(h) = self.mixes.get(&(cue, bucket)) {
            return h.clone();
        }
        let mono = self.mono.entry(cue).or_insert_with(|| Arc::new(synth(cue))).clone();
        let h = sources.add(AudioSource { bytes: wav(&mono, bucket_pan(bucket)).into() });
        self.mixes.insert((cue, bucket), h.clone());
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
    camera: Query<&Transform, With<MainCamera>>,
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
    let listener = camera.single().map(|t| crate::to_tile(t.translation.truncate())).unwrap_or(V2::new(0.0, 0.0));
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
    let waiting = std::mem::take(&mut sounds.waiting);
    for (cue, id, gain, since) in waiting {
        match find(obs, id) {
            Some(p) => due.push((cue, Some(p), gain)),
            None if now.duration_since(since) < WAIT => sounds.waiting.push((cue, id, gain, since)),
            None => {}
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

    let mut playing = voices.iter().count();
    // Loudest first, so a full mixer drops the far ones.
    let mut placed: Vec<(Cue, f32, f32)> = due
        .into_iter()
        .filter_map(|(cue, at, gain)| match at {
            None => Some((cue, gain, 0.0)),
            Some(p) => place(listener, p).map(|(g, pan)| (cue, gain * g, pan)),
        })
        .filter(|(_, g, _)| *g > 0.02)
        .collect();
    placed.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (cue, gain, pan) in placed {
        if playing >= VOICES {
            break;
        }
        if sounds.last.get(&cue).map(|t| now.duration_since(*t) < REPEAT).unwrap_or(false) {
            continue;
        }
        sounds.last.insert(cue, now);
        debug!("sound {} gain {:.2} pan {:+.1}", cue.name(), gain, pan);
        let h = sounds.mix(cue, pan_bucket(pan), &mut sources);
        commands.spawn((AudioPlayer::new(h), PlaybackSettings::DESPAWN.with_volume(Volume::Linear(gain)), Voice));
        playing += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_cue_is_a_sound() {
        for cue in Cue::ALL {
            let s = synth(cue);
            let secs = s.len() as f32 / RATE as f32;
            assert!((0.1..=2.5).contains(&secs), "{:?}: {} s", cue, secs);
            assert!(s.iter().all(|x| x.is_finite() && x.abs() <= 0.9001), "{:?} clips or is not a number", cue);
            let rms = (s.iter().map(|x| x * x).sum::<f32>() / s.len() as f32).sqrt();
            assert!(rms > 0.02, "{:?} is nearly silent (rms {})", cue, rms);
            assert!(s.last().unwrap().abs() < 0.01, "{:?} ends with a click", cue);
        }
    }

    #[test]
    fn same_sound_every_time() {
        assert_eq!(synth(Cue::Boom), synth(Cue::Boom));
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
    fn nearer_is_louder_and_off_centre_pans() {
        let me = V2::new(50.0, 50.0);
        let (g0, p0) = place(me, me).unwrap();
        assert_eq!((g0, p0), (1.0, 0.0));
        let (g1, p1) = place(me, V2::new(50.0 + HEAR_HALF, 50.0)).unwrap();
        assert!((g1 - 0.5).abs() < 1e-6 && p1 > 0.0);
        let (_, p2) = place(me, V2::new(0.0, 50.0)).unwrap();
        assert_eq!(p2, -1.0);
        assert!(place(me, V2::new(50.0, 50.0 + HEAR_FAR + 1.0)).is_none());
        let (l, r) = pan_gains(0.0);
        assert!((l * l + r * r - 1.0).abs() < 1e-6 && (l - r).abs() < 1e-6);
        assert_eq!(pan_bucket(-1.0), 0);
        assert_eq!(pan_bucket(1.0), PANS - 1);
        assert_eq!(pan_bucket(0.0), PANS / 2);
    }
}
