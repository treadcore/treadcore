//! The battlefield's tanks and the marks the fighting leaves, after the
//! sprite kit's test range (journal 0037), its animations one for one:
//! - the four-frame tread cycle, rolled by distance driven and by
//!   turning on the spot (the kit's pivot);
//! - a turret apart from the hull, turning at its chassis' traverse
//!   (1.7, 3.4, 2.6 rad/s): mine to the cursor, a standing one scanning
//!   side to side as the kit's plates do, a moving one over the bow,
//!   and every one snapped to where a shell it fired was aimed;
//! - the kick (big for the long gun, small for the light ones) that
//!   eases back, and a spray of sparks down the barrel, ten or four,
//!   the raider's two guns in turn;
//! - orange sparks where a shell lands, and scorch that stays;
//! - a shadow under the hull and track marks behind it that fade;
//! - afloat, a landing craft under the tank and a wake behind it, and
//!   empty boats moored where a squad built them.
//!
//! Each side fields one of three chassis (voxel::TankStyle). My side's
//! is the one I pick (`--style`, T in play), and the other sides take
//! the other two in turn by team, so friend and foe differ in shape as
//! well as colour. It is the window's choice, not the world's: every
//! window draws its own side in its own pick, and nothing goes on the
//! wire.
//!
//! Everything is still rectangles: the marks and the flash are sprites
//! like the cubes are.

use crate::keys::{Action, Keys};
use crate::session::View;
use crate::voxel::{self, TankStyle};
use crate::render::{Label, MainCamera};
use crate::{player_color, to_screen, Args, TILE};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::collections::{HashMap, HashSet};
use treadcore_sim::*;

pub struct BattlefieldPlugin;

impl Plugin for BattlefieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Field>().add_systems(Startup, choose).add_systems(Update, (pick_style, fade, pings).before(sync_tanks));
    }
}

/// How long a turret holds on what it shot at before it goes back to
/// its business (the bow, a slow scan standing still, or for mine the
/// cursor), and how far a standing turret scans either side.
const AIM_HOLD: f32 = 2.5;
const MY_HOLD: f32 = 0.5;
const SCAN: f32 = 0.7;
/// Tiles of tread a radian of turning on the spot rolls.
const PIVOT_ROLL: f32 = 0.5;
/// A track mark every this many tiles driven; how long it lasts; how
/// many there may be.
const MARK_EVERY: f32 = 0.3;
const MARK_LIFE: f32 = 12.0;
const MARK_MAX: usize = 1600;
const SCORCH_LIFE: f32 = 45.0;
const SCORCH_MAX: usize = 300;
/// The shadow's fall, in tiles: the light is up and to the left.
const SHADOW: Vec2 = Vec2::new(0.3, -0.4);

#[derive(Resource, Default)]
pub struct Field {
    /// My side's chassis.
    pub style: TankStyle,
    tanks: HashMap<EntityId, Drawn>,
    shells: HashSet<EntityId>,
    /// Moored boats as drawn.
    boats: HashMap<EntityId, Entity>,
}

/// One tank as drawn: its entities, the key its cubes were built for,
/// and what the window remembers of it between frames.
struct Drawn {
    root: Entity,
    body: Entity,
    gear: [Entity; 4],
    turret: Entity,
    shadow: Entity,
    /// The landing craft, shown while the tank is afloat.
    boat: Entity,
    key: u64,
    last: V2,
    /// Signed distance driven, in tiles: the running gear's phase.
    driven: f32,
    since_mark: f32,
    /// The turret's heading in the sim's angle, and what it is turning
    /// to until when.
    aim: f32,
    aim_at: Option<(f32, f32)>,
    recoil: f32,
    /// The hull's heading last frame, for the pivot's roll; the barrel
    /// that fires next.
    heading: f32,
    barrel: usize,
}

#[derive(Component)]
pub struct Mark {
    life: f32,
    full: f32,
    alpha: f32,
}

/// One square of a ping's ring: it rides out from the point and fades,
/// three times over `PING_LIFE`.
#[derive(Component)]
pub struct PingDot {
    centre: Vec2,
    angle: f32,
    life: f32,
    color: Color,
}

const PING_LIFE: f32 = 3.0;

/// My side's pings, as rings in the pinger's colour (Q, the middle
/// button; a bot's when it calls an enemy).
fn pings(mut commands: Commands, time: Res<Time>, mut view: ResMut<View>, mut dots: Query<(Entity, &mut PingDot, &mut Transform, &mut Sprite)>) {
    for (at, who) in std::mem::take(&mut view.pings) {
        let color = player_color(who);
        for i in 0..12 {
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(0.35 * TILE)), ..default() },
                Transform::from_translation(to_screen(at).extend(7.0)),
                PingDot { centre: to_screen(at), angle: i as f32 * std::f32::consts::TAU / 12.0, life: PING_LIFE, color },
            ));
        }
    }
    let dt = time.delta_secs().min(0.1);
    for (e, mut d, mut t, mut s) in dots.iter_mut() {
        d.life -= dt;
        if d.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let phase = (PING_LIFE - d.life).fract();
        let r = (0.6 + phase * 2.8) * TILE;
        t.translation = (d.centre + Vec2::from_angle(d.angle) * r).extend(7.0);
        t.rotation = Quat::from_rotation_z(d.angle);
        s.color = d.color.with_alpha((1.0 - phase) * 0.9);
    }
}

#[derive(Component)]
pub struct Flash {
    life: f32,
}

/// A spark from a muzzle or a hit: it flies, slows and fades, the kit's
/// particles.
#[derive(Component)]
pub struct Spark {
    vel: Vec2,
    life: f32,
    full: f32,
    alpha: f32,
}

/// The kit's dice for the sparks: looks only, never the world's.
struct Dice(u32);

impl Dice {
    fn roll(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 % 10000) as f32 / 10000.0
    }
}

/// Throw `n` sparks from `at` (screen) along `dir` at `speed`, spread
/// by `spread`, in tiles a second.
fn sparks(commands: &mut Commands, dice: &mut Dice, at: Vec2, dir: Vec2, n: usize, speed: f32, spread: f32, size: f32, life: f32, color: Color) {
    for _ in 0..n {
        let vel = (dir * speed + Vec2::new(dice.roll() - 0.5, dice.roll() - 0.5) * 2.0 * spread) * TILE;
        let s = size * (0.6 + dice.roll() * 0.8) * TILE;
        commands.spawn((
            Sprite { color, custom_size: Some(Vec2::splat(s)), ..default() },
            Transform::from_translation(at.extend(6.5)).with_rotation(Quat::from_rotation_z(dice.roll() * 1.57)),
            Spark { vel, life, full: life, alpha: color.alpha() },
        ));
    }
}

fn choose(args: Res<Args>, mut field: ResMut<Field>) {
    field.style = args.style;
}

/// T: my side's next chassis.
fn pick_style(keys: Res<ButtonInput<KeyCode>>, bind: Res<Keys>, mut view: ResMut<View>, mut field: ResMut<Field>) {
    if keys.just_pressed(bind.key(Action::TankStyle)) && !view.keys_taken() {
        field.style = field.style.next();
        let tick = view.last_tick;
        let k = crate::keys::label(bind.key(Action::TankStyle));
        view.log.push_back((tick, format!("my side drives the {} ({})", field.style.word(), k)));
    }
}

/// The chassis a tank is drawn as: mine for my side, the other two in
/// turn for the rest by team.
pub fn style_for(obs: &Observation, owner: PlayerId, mine: TankStyle) -> TankStyle {
    if obs.allied(owner) {
        return mine;
    }
    let team = obs.team_of(owner).unwrap_or(owner as TeamId);
    mine.others()[team as usize % 2]
}

fn key_of(style: TankStyle, color: Color, alive: bool, bucket: u32) -> u64 {
    let c = color.to_srgba();
    let mut h: u64 = (style as u64) << 1 | alive as u64 | (bucket as u64) << 4;
    for v in [c.red, c.green, c.blue] {
        h = h.wrapping_mul(0x100000001b3) ^ v.to_bits() as u64;
    }
    h
}

fn turn_toward(from: f32, to: f32, max: f32) -> f32 {
    let d = (to - from + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
    from + d.clamp(-max, max)
}

/// Build a model's cubes under `parent`, the old ones gone.
fn rebuild(commands: &mut Commands, parent: Entity, m: voxel::Model) {
    commands.entity(parent).despawn_related::<Children>();
    commands.entity(parent).with_children(|p| m.spawn(p, 0.0));
}

fn group(commands: &mut Commands, parent: Entity) -> Entity {
    let e = commands.spawn((Transform::default(), Visibility::Inherited)).id();
    commands.entity(parent).add_child(e);
    e
}

pub fn sync_tanks(
    mut commands: Commands,
    time: Res<Time>,
    view: Res<View>,
    mut field: ResMut<Field>,
    mut tf: Query<(&mut Transform, &mut Visibility)>,
    mut labels: Query<(&mut Text2d, &mut TextColor), With<Label>>,
    children: Query<&Children>,
    marks: Query<(), With<Mark>>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let Some(obs) = view.obs.as_ref() else { return };
    let now = time.elapsed_secs();
    let mut dice = Dice((now * 1931.0) as u32 | 1);
    // Where the cursor is on the map: my turret follows it, as the
    // kit's range has it.
    let cursor = match (window.single(), cam.single()) {
        (Ok(w), Ok((c, ct))) => crate::input::cursor_tile(w, c, ct),
        _ => None,
    };
    let dt = time.delta_secs().min(0.1);
    let me = obs.player;
    let mine = field.style;
    let ahead = view.received.map(|r| r.elapsed().as_secs_f32().min(0.2)).unwrap_or(0.0) * view.speed.max(0.25);
    let predicted = view.predicted.as_ref().filter(|(t, _)| t.owner == me);
    let mut marks_left = MARK_MAX.saturating_sub(marks.iter().count());

    // New shells: the turret that fired one snaps to where it was
    // aimed, kicks back, and flashes.
    let mut fired: Vec<(EntityId, f32)> = Vec::new();
    for s in &obs.shells {
        if field.shells.insert(s.id) {
            if let Some(t) = obs.tanks.iter().find(|t| t.alive && t.owner == s.owner && t.pos.dist(s.from) < 1.5) {
                let d = s.aim - s.from;
                fired.push((t.id, d.y.atan2(d.x)));
            }
        }
    }
    let live: HashSet<EntityId> = obs.shells.iter().map(|s| s.id).collect();
    field.shells.retain(|id| live.contains(id));

    let mut seen: HashSet<EntityId> = HashSet::new();
    for t in &obs.tanks {
        let (t, ahead) = match predicted {
            Some((p, at)) if p.id == t.id => (p, at.elapsed().as_secs_f32().min(0.1)),
            _ => (t, ahead),
        };
        seen.insert(t.id);
        let pos = t.pos + V2::from_heading(t.heading).scale(t.speed * ahead);
        let color = player_color(t.owner);
        let style = style_for(obs, t.owner, mine);
        let damage = (1.0 - t.armor / TANK_ARMOR).clamp(0.0, 1.0);
        let bucket = (damage * 10.0).round() as u32;
        let key = key_of(style, color, t.alive, bucket);
        let name = obs.players.get(t.owner as usize).map(|p| p.name.clone()).unwrap_or_default();
        let label = format!("{}{} {:.0}", if t.owner == me { "* " } else { "" }, name, t.armor);
        let label_color = if t.alive { color } else { Color::srgb(0.5, 0.5, 0.5) };

        if let std::collections::hash_map::Entry::Vacant(slot) = field.tanks.entry(t.id) {
            let root = commands.spawn((Transform::from_translation(to_screen(pos).extend(2.0)), Visibility::Visible)).id();
            let shadow = commands.spawn((Sprite { color: Color::srgba(0.12, 0.08, 0.02, 0.28), custom_size: Some(Vec2::new(1.5, 1.0) * TILE), ..default() }, Transform::from_xyz(0.0, 0.0, -0.5))).id();
            commands.entity(root).add_child(shadow);
            let boat = commands.spawn((Transform::from_xyz(0.0, 0.0, -0.4), Visibility::Hidden)).id();
            commands.entity(root).add_child(boat);
            rebuild(&mut commands, boat, voxel::landing_craft(None));
            let body = group(&mut commands, root);
            let gear = [group(&mut commands, root), group(&mut commands, root), group(&mut commands, root), group(&mut commands, root)];
            let turret = group(&mut commands, root);
            commands.entity(root).with_children(|p| {
                p.spawn((Text2d::new(label.clone()), TextFont { font_size: 11.0, ..default() }, TextColor(label_color), Transform::from_translation(Vec3::new(0.0, TILE * 1.4, 1.0)), Label));
            });
            slot.insert(Drawn { root, body, gear, turret, shadow, boat, key: 0, last: pos, driven: 0.0, since_mark: 0.0, aim: t.heading, aim_at: None, recoil: 0.0, heading: t.heading, barrel: 0 });
        }
        let d = field.tanks.get_mut(&t.id).unwrap();
        if d.key != key {
            d.key = key;
            rebuild(&mut commands, d.body, voxel::hull(style, color, t.alive, bucket as f32 / 10.0));
            for (i, g) in d.gear.iter().enumerate() {
                rebuild(&mut commands, *g, voxel::running_gear(style, t.alive, i as u8));
            }
            rebuild(&mut commands, d.turret, voxel::turret(style, color, t.alive, bucket as f32 / 10.0));
        }

        // Driving: forward rolls the gear one way, reverse the other; a
        // jump (a respawn, a correction) rolls nothing.
        let step = pos - d.last;
        d.last = pos;
        let f = V2::from_heading(t.heading);
        let moved = if step.len() < 1.0 { step.x * f.x + step.y * f.y } else { 0.0 };
        // Turning on the spot rolls the treads too, as the kit's pivot does.
        let turned = (t.heading - d.heading + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        d.heading = t.heading;
        if t.alive {
            d.driven += moved + if moved.abs() < 0.01 { turned.abs() * PIVOT_ROLL } else { 0.0 };
            d.since_mark += moved.abs();
        }
        let phase = ((d.driven / 0.1).floor() as i64).rem_euclid(4) as usize;
        for (i, g) in d.gear.iter().enumerate() {
            if let Ok((_, mut v)) = tf.get_mut(*g) {
                *v = if i == phase { Visibility::Inherited } else { Visibility::Hidden };
            }
        }

        // The turret: it snaps to a shot and kicks; then mine follows the
        // cursor, a standing one scans, a moving one looks over the bow,
        // each at its chassis' traverse.
        let shot = fired.iter().find(|(id, _)| *id == t.id).map(|(_, a)| *a);
        if let Some(a) = shot {
            d.aim = a;
            d.aim_at = Some((a, now + if t.owner == me { MY_HOLD } else { AIM_HOLD }));
            d.recoil = style.recoil();
        }
        let scan = (now * 0.8 + (t.id % 97) as f32).sin() * SCAN;
        let want = match (d.aim_at, cursor) {
            _ if !t.alive => d.aim,
            (Some((a, until)), _) if now < until => a,
            (_, Some(c)) if t.owner == me => (c.y - pos.y).atan2(c.x - pos.x),
            _ if t.speed.abs() < 0.05 => t.heading + scan,
            _ => t.heading,
        };
        d.aim = turn_toward(d.aim, want, style.traverse() * dt);
        d.recoil *= (0.001f32).powf(dt);
        let rel = d.aim - t.heading;
        let unit = TILE * 0.1;
        let back = Vec2::from_angle(-rel) * (-d.recoil * TILE);
        if let Ok((mut tt, _)) = tf.get_mut(d.turret) {
            tt.translation = Vec3::new(style.mount() * unit + back.x, back.y, 0.0);
            tt.rotation = Quat::from_rotation_z(-rel);
        }

        let rot = -t.heading;
        if let Ok((mut rt, mut v)) = tf.get_mut(d.root) {
            *rt = Transform::from_translation(to_screen(pos).extend(2.0)).with_rotation(Quat::from_rotation_z(rot));
            *v = Visibility::Visible;
        }
        // The shadow falls the same way on screen whichever way the
        // hull points.
        if let Ok((mut st, mut sv)) = tf.get_mut(d.shadow) {
            st.translation = (Vec2::from_angle(-rot).rotate(SHADOW * TILE)).extend(-0.5);
            *sv = if t.alive { Visibility::Inherited } else { Visibility::Hidden };
        }
        if let Ok((_, mut bv)) = tf.get_mut(d.boat) {
            *bv = if t.alive && t.boat { Visibility::Inherited } else { Visibility::Hidden };
        }
        if let Ok(ch) = children.get(d.root) {
            for c in ch.iter() {
                if let Ok((mut text, mut tc)) = labels.get_mut(c) {
                    if text.0 != label {
                        text.0 = label.clone();
                    }
                    tc.0 = label_color;
                    // Upright over the tank whichever way the hull points.
                    if let Ok((mut lt, _)) = tf.get_mut(c) {
                        lt.translation = Vec2::from_angle(-rot).rotate(Vec2::new(0.0, TILE * 1.4)).extend(1.0);
                        lt.rotation = Quat::from_rotation_z(-rot);
                    }
                }
            }
        }

        // Track marks: a pair behind the running gear, not on water.
        let on_water = view.map.as_ref().map(|m| matches!(m.at(pos), Terrain::Water)).unwrap_or(false);
        if d.since_mark >= MARK_EVERY {
            d.since_mark = 0.0;
            if t.alive && t.boat && on_water && marks_left >= 2 {
                // The wake: two white streaks off the stern, gone soon.
                marks_left -= 2;
                let f = V2::from_heading(t.heading);
                let side = V2::new(-f.y, f.x);
                for s in [-0.45, 0.45] {
                    let at = pos - f.scale(1.1) + side.scale(s);
                    commands.spawn((
                        Sprite { color: Color::srgba(0.9, 0.97, 1.0, 0.45), custom_size: Some(Vec2::new(0.35, 0.16) * TILE), ..default() },
                        Transform::from_translation(to_screen(at).extend(0.3)).with_rotation(Quat::from_rotation_z(rot + s * 0.8)),
                        Mark { life: 1.6, full: 1.6, alpha: 0.45 },
                    ));
                }
            }
            if t.alive && !on_water && marks_left >= 2 {
                marks_left -= 2;
                let f = V2::from_heading(t.heading);
                let side = V2::new(-f.y, f.x);
                let w = if style == TankStyle::Rattler { 0.45 } else { 0.35 };
                for s in [-w, w] {
                    let at = pos - f.scale(0.55) + side.scale(s);
                    commands.spawn((
                        Sprite { color: Color::srgba(0.2, 0.14, 0.07, 0.2), custom_size: Some(Vec2::new(0.22, 0.14) * TILE), ..default() },
                        Transform::from_translation(to_screen(at).extend(0.3)).with_rotation(Quat::from_rotation_z(rot)),
                        Mark { life: MARK_LIFE, full: MARK_LIFE, alpha: 0.2 },
                    ));
                }
            }
        }

        // The muzzle: a flash and a spray of sparks down the barrel, the
        // raider's two guns in turn.
        if shot.is_some() {
            let lane = style.barrels()[d.barrel % style.barrels().len()];
            d.barrel += 1;
            let dir = Vec2::from_angle(-d.aim);
            let muzzle = to_screen(pos) + Vec2::from_angle(rot).rotate(Vec2::new(style.mount() * unit, 0.0)) + dir.rotate(Vec2::new(style.muzzle(), lane) * unit);
            for (size, c, spin) in [(0.7, Color::srgba(1.0, 0.6, 0.15, 0.75), 0.785), (0.4, Color::srgba(1.0, 0.92, 0.6, 0.95), 0.0)] {
                commands.spawn((Sprite { color: c, custom_size: Some(Vec2::splat(size * TILE)), ..default() }, Transform::from_translation(muzzle.extend(6.0)).with_rotation(Quat::from_rotation_z(-d.aim + spin)), Flash { life: 0.08 }));
            }
            sparks(&mut commands, &mut dice, muzzle, dir, style.sparks(), 1.6, 1.0, 0.14, 0.35, Color::srgba(1.0, 0.78, 0.35, 0.9));
        }
    }
    // Chains: a row of dark links from each tank to what it pulls, drawn
    // fresh each frame (a flash that lasts one).
    for t in obs.tanks.iter().filter(|t| t.alive) {
        let Some(id) = t.towing else { continue };
        let Some(to) = obs.pillboxes.iter().find(|p| p.id == id).map(|p| p.pos).or_else(|| obs.depots.iter().find(|d| d.id == id).map(|d| d.pos)) else { continue };
        let (a, b) = (to_screen(t.pos), to_screen(to));
        let links = ((a.distance(b) / (0.3 * TILE)) as usize).clamp(2, 24);
        for k in 1..links {
            let p = a.lerp(b, k as f32 / links as f32);
            commands.spawn((Sprite { color: if k % 2 == 0 { Color::srgb(0.25, 0.25, 0.27) } else { Color::srgb(0.45, 0.45, 0.48) }, custom_size: Some(Vec2::new(0.22, 0.14) * TILE), ..default() }, Transform::from_translation(p.extend(5.0)).with_rotation(Quat::from_rotation_z((b - a).to_angle())), Flash { life: 0.0 }));
        }
    }
    // Moored boats: drawn where they lie, gone when boarded.
    let moored: HashSet<EntityId> = obs.boats.iter().map(|b| b.id).collect();
    field.boats.retain(|id, e| {
        let keep = moored.contains(id);
        if !keep {
            commands.entity(*e).despawn();
        }
        keep
    });
    for b in &obs.boats {
        if let std::collections::hash_map::Entry::Vacant(slot) = field.boats.entry(b.id) {
            let e = commands.spawn((Transform::from_translation(to_screen(b.pos).extend(1.5)).with_rotation(Quat::from_rotation_z(((b.id % 4) as f32) * 0.4 - 0.6)), Visibility::Visible)).id();
            rebuild(&mut commands, e, voxel::landing_craft(Some(player_color(b.owner))));
            slot.insert(e);
        }
    }
    // Gone from the view: hidden, as render.rs does, not despawned.
    for (id, d) in field.tanks.iter() {
        if !seen.contains(id) {
            if let Ok((_, mut v)) = tf.get_mut(d.root) {
                *v = Visibility::Hidden;
            }
        }
    }
}

/// Scorch where the view says something went off; marks fade and go,
/// flashes go at once.
fn fade(mut commands: Commands, time: Res<Time>, mut view: ResMut<View>, mut q: Query<(Entity, &mut Mark, &mut Sprite), Without<Spark>>, mut flashes: Query<(Entity, &mut Flash)>, mut flying: Query<(Entity, &mut Spark, &mut Transform, &mut Sprite), Without<Mark>>) {
    let dt = time.delta_secs().min(0.1);
    let mut seed = (time.elapsed_secs() * 977.0) as u32 | 1;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 10000) as f32 / 10000.0
    };
    let mut room = SCORCH_MAX.saturating_sub(q.iter().filter(|(_, m, _)| m.full == SCORCH_LIFE).count());
    let mut dice = Dice((time.elapsed_secs() * 3571.0) as u32 | 1);
    for (at, size) in std::mem::take(&mut view.scorches) {
        // The hit's sparks, the kit's eight orange ones, more for more.
        sparks(&mut commands, &mut dice, to_screen(at), Vec2::ZERO, (size * 9.0) as usize, 0.0, 1.8, 0.12, 0.4, Color::srgba(1.0, 0.67, 0.24, 0.9));
        // Three squares turned about each other read as a blot.
        for _ in 0..3 {
            if room == 0 {
                break;
            }
            room -= 1;
            let s = size * (0.6 + rnd() * 0.5) * TILE;
            let off = Vec2::new(rnd() - 0.5, rnd() - 0.5) * size * 0.4 * TILE;
            commands.spawn((
                Sprite { color: Color::srgba(0.1, 0.07, 0.04, 0.22), custom_size: Some(Vec2::splat(s)), ..default() },
                Transform::from_translation((to_screen(at) + off).extend(0.35)).with_rotation(Quat::from_rotation_z(rnd() * 1.57)),
                Mark { life: SCORCH_LIFE, full: SCORCH_LIFE, alpha: 0.22 },
            ));
        }
    }
    for (e, mut m, mut s) in q.iter_mut() {
        m.life -= dt;
        if m.life <= 0.0 {
            commands.entity(e).despawn();
        } else if m.life < m.full * 0.4 {
            s.color = s.color.with_alpha(m.alpha * m.life / (m.full * 0.4));
        }
    }
    // Sparks fly, slow (the kit's 0.92 a frame at 60) and fade.
    for (e, mut sp, mut t, mut s) in flying.iter_mut() {
        sp.life -= dt;
        if sp.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        t.translation += (sp.vel * dt).extend(0.0);
        sp.vel *= 0.0067f32.powf(dt);
        s.color = s.color.with_alpha(sp.alpha * (sp.life / sp.full * 2.0).min(1.0));
    }
    for (e, mut f) in flashes.iter_mut() {
        f.life -= dt;
        if f.life <= 0.0 {
            commands.entity(e).despawn();
        }
    }
}
