//! Drawing the world: tiles as sprites, entities as sprites keyed by id,
//! and the telemetry overlay in gizmos -- bubbles, links, vision, arcs.
//!
//! The overlay is the point of the game's look. A tank is a small thing
//! on a big map; what the player watches is the ring of interceptors
//! around it, the lines to its drones going from green to red as they
//! fly out of link range, the recon disc sliding over the enemy, and the
//! arc of a shell leaving it.

use crate::hud::{HudState, Mode};
use crate::input::cursor_tile;
use bevy::window::PrimaryWindow;
use crate::session::View;
use crate::voxel;
use crate::{player_color, to_screen, TILE};
use bevy::prelude::*;
use std::collections::HashMap;
use treadcore_sim::*;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Sprites>()
            .init_resource::<Fog>()
            .add_systems(Startup, setup_camera)
            .add_systems(Update, (build_map, update_fog, sync_entities, overlay, debris).chain());
    }
}

#[derive(Component)]
pub struct MainCamera;

/// A ground tile's sprite; `Sprites.tiles` holds them by cell index.
#[derive(Component)]
pub struct Tile;

/// A thing's model: the parent that moves; `Sprites.ents` holds them by id.
#[derive(Component)]
pub struct Ent;

#[derive(Component)]
pub struct Label;

/// A loose cube thrown out of an impact: it flies, bounces, rolls to a
/// stop and fades. Position is the parent transform; `h` is its height
/// above the ground, drawn as a lift up the screen.
#[derive(Component)]
pub struct Debris {
    vel: Vec2,
    h: f32,
    vh: f32,
    spin: f32,
    life: f32,
}

#[derive(Resource, Default)]
pub struct Sprites {
    pub map_built: bool,
    pub cells: Vec<u8>,
    pub tiles: Vec<Entity>,
    /// Every moving or standing thing: a parent whose children are its
    /// cubes (voxel.rs) and its label; and the key its cubes were
    /// built for.
    pub ents: HashMap<EntityId, Entity>,
    pub keys: HashMap<EntityId, u64>,
    /// Wall tiles as blocks of cubes and craters as pits, by cell
    /// index, with the terrain they were built for.
    pub walls: HashMap<usize, (u8, Entity)>,
}

/// Which tiles the player can see, recomputed when an observation lands.
#[derive(Resource, Default)]
pub struct Fog {
    pub tick: Tick,
    pub visible: Vec<bool>,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        MainCamera,
        Projection::Orthographic(OrthographicProjection { scale: 0.7, ..OrthographicProjection::default_2d() }),
        // Centred on a default-sized map until the first observation
        // puts the camera on the tank (input.rs centres it once).
        Transform::from_translation(Vec3::new(MAP_W as f32 * TILE * 0.5, -(MAP_H as f32) * TILE * 0.5, 0.0)),
    ));
}

/// Bolo's ground, one solid colour a tile: each tile is a solid block
/// in the colour of the sprite WinBolo drew it with (the screenshots
/// on retro-replay's Bolo page). Grass is the green of the grass
/// sprite's blades, forest the bright green of its leaves, road the
/// grey of its dashes, water the river's cyan, a wall and a bridge the
/// building's tan and brown. No checker, no dither: Bolo's dither was
/// a texture at a pixel a dot, and at a tile a dot it was a chessboard.
pub fn terrain_color(t: Terrain) -> Color {
    match t {
        Terrain::Grass => GRASS,
        Terrain::Forest => FOREST,
        Terrain::Road => ROAD,
        Terrain::Water => WATER,
        Terrain::Bridge => voxel::TAN_DARK,
        Terrain::Rubble => voxel::EARTH,
        Terrain::Depot => Color::srgb(0.50, 0.46, 0.38),
        Terrain::Wall => voxel::TAN,
        Terrain::Pad => voxel::PAD,
        Terrain::Concrete => voxel::CONCRETE,
    }
}

/// The ground's colours, sampled from WinBolo's sprites.
pub const GRASS: Color = Color::srgb(0.0, 0.44, 0.125);
pub const FOREST: Color = Color::srgb(0.12, 0.69, 0.06);
pub const ROAD: Color = Color::srgb(0.42, 0.42, 0.42);
pub const WATER: Color = Color::srgb(0.0, 0.56, 0.62);

fn build_map(mut commands: Commands, view: Res<View>, mut sprites: ResMut<Sprites>, mut tiles: Query<&mut Sprite, With<Tile>>, fog: Res<Fog>) {
    let Some(map) = view.map.as_ref() else { return };
    if !sprites.map_built {
        let mut ids = Vec::with_capacity(map.w * map.h);
        for y in 0..map.h {
            for x in 0..map.w {
                let t = map.at_tile(x, y);
                let pos = to_screen(V2::new(x as f32 + 0.5, y as f32 + 0.5));
                let id = commands
                    .spawn((
                        Sprite { color: terrain_color(t), custom_size: Some(Vec2::splat(TILE)), ..default() },
                        Transform::from_translation(pos.extend(0.0)),
                        Tile,
                    ))
                    .id();
                ids.push(id);
            }
        }
        sprites.tiles = ids;
        sprites.cells = map.cells.clone();
        sprites.map_built = true;
        let w = map.w;
        for (i, &c) in map.cells.iter().enumerate() {
            raise_or_sink(&mut commands, &mut sprites, w, i, Terrain::from_u8(c));
        }
        return;
    }
    // Craters, and fog changes: recolour only what changed. Walls are
    // blocks of cubes standing on their tile, raised when the tile
    // becomes a wall and taken down when it is knocked to rubble.
    let fog_changed = fog.is_changed();
    let w = map.w;
    let mut changed: Vec<(usize, u8)> = Vec::new();
    for (i, (&new, &old)) in map.cells.iter().zip(sprites.cells.iter()).enumerate() {
        let vis = fog.visible.get(i).copied().unwrap_or(true);
        if new != old || fog_changed {
            if let Ok(mut s) = tiles.get_mut(sprites.tiles[i]) {
                let c = terrain_color(Terrain::from_u8(new));
                s.color = if vis { c } else { c.darker(0.07) };
            }
        }
        if new != old {
            changed.push((i, new));
        }
    }
    for (i, new) in changed {
        raise_or_sink(&mut commands, &mut sprites, w, i, Terrain::from_u8(new));
    }
    if map.cells != sprites.cells {
        sprites.cells = map.cells.clone();
    }
}

/// A wall stands up as a block, a crater sinks as a pit; any other
/// terrain takes whichever was there down.
fn raise_or_sink(commands: &mut Commands, sprites: &mut Sprites, w: usize, i: usize, t: Terrain) {
    let want = match t {
        Terrain::Wall => Some(Terrain::Wall as u8),
        Terrain::Concrete => Some(Terrain::Concrete as u8),
        Terrain::Rubble => Some(Terrain::Rubble as u8),
        _ => None,
    };
    let have = sprites.walls.get(&i).copied();
    if have.map(|(k, _)| Some(k)) == Some(want) || (have.is_none() && want.is_none()) {
        return;
    }
    if let Some((_, e)) = have {
        commands.entity(e).despawn();
        sprites.walls.remove(&i);
    }
    if let Some(k) = want {
        let pos = to_screen(V2::new((i % w) as f32 + 0.5, (i / w) as f32 + 0.5));
        let model = match Terrain::from_u8(k) {
            Terrain::Wall => voxel::wall(),
            Terrain::Concrete => voxel::concrete(),
            _ => voxel::crater(),
        };
        let e = commands.spawn((Transform::from_translation(pos.extend(0.9)), Visibility::Visible)).with_children(|p| model.spawn(p, 0.0)).id();
        sprites.walls.insert(i, (k, e));
    }
}

/// Client-side fog, from the same radii the host uses. Only to dim the
/// ground: what entities are shown is the host's decision, already made
/// in the observation.
fn update_fog(view: Res<View>, mut fog: ResMut<Fog>) {
    let Some(obs) = view.obs.as_ref() else { return };
    let Some(map) = view.map.as_ref() else { return };
    if obs.tick == fog.tick && !fog.visible.is_empty() {
        return;
    }
    fog.tick = obs.tick;
    let me = obs.player;
    let mut sensors: Vec<(V2, f32)> = Vec::new();
    if let Some(t) = obs.me.as_ref().filter(|t| t.alive) {
        sensors.push((t.pos, TANK_VISION));
    }
    for d in obs.my_drones() {
        sensors.push((d.pos, if d.kind == DroneKind::Recon { RECON_VISION } else { DRONE_VISION }));
    }
    let _ = me;
    let mut vis = vec![false; map.w * map.h];
    for (c, r) in &sensors {
        let (cx, cy) = (c.x as i32, c.y as i32);
        let ri = r.ceil() as i32;
        for y in (cy - ri).max(0)..(cy + ri + 1).min(map.h as i32) {
            for x in (cx - ri).max(0)..(cx + ri + 1).min(map.w as i32) {
                let p = V2::new(x as f32 + 0.5, y as f32 + 0.5);
                let forest = map.at_tile(x as usize, y as usize) == Terrain::Forest;
                let rr = if forest { r * 0.55 } else { *r };
                if p.dist(*c) <= rr {
                    vis[y as usize * map.w + x as usize] = true;
                }
            }
        }
    }
    fog.visible = vis;
}

/// A model's identity for the cache: kind, colour, and a variant (alive,
/// health). When it changes, the cubes are built again.
fn model_key(kind: u8, color: Color, variant: u32) -> u64 {
    let c = color.to_srgba();
    let mut h: u64 = kind as u64 ^ ((variant as u64) << 8);
    for v in [c.red, c.green, c.blue] {
        h = h.wrapping_mul(0x100000001b3) ^ v.to_bits() as u64;
    }
    h
}

fn sync_entities(
    mut commands: Commands,
    view: Res<View>,
    mut sprites: ResMut<Sprites>,
    mut q: Query<(&mut Transform, &mut Visibility), With<Ent>>,
    mut labels: Query<(&mut Text2d, &mut TextColor), With<Label>>,
    children: Query<&Children>,
) {
    let Some(obs) = view.obs.as_ref() else { return };
    let mut seen: Vec<EntityId> = Vec::new();
    let me = obs.player;
    // Interpolate forward from the observation by wall time, so a 10 Hz
    // snapshot stream still moves at 60 fps.
    let ahead = view.received.map(|r| r.elapsed().as_secs_f32().min(0.2)).unwrap_or(0.0);

    // Place a model: a parent that moves, children that are its cubes
    // (rebuilt when the model's key changes) and its label.
    let mut place = |commands: &mut Commands, id: EntityId, pos: Vec2, rot: f32, key: u64, build: &dyn Fn() -> voxel::Model, z: f32, label: Option<(String, Color)>| {
        seen.push(id);
        let tf = Transform::from_translation(pos.extend(z)).with_rotation(Quat::from_rotation_z(rot));
        match sprites.ents.get(&id).copied() {
            Some(e) => {
                if let Ok((mut t, mut v)) = q.get_mut(e) {
                    *t = tf;
                    *v = Visibility::Visible;
                }
                if sprites.keys.get(&id).copied() != Some(key) {
                    // The cubes again, the label kept.
                    if let Ok(ch) = children.get(e) {
                        for c_e in ch.iter() {
                            if labels.get(c_e).is_err() {
                                commands.entity(c_e).despawn();
                            }
                        }
                    }
                    let m = build();
                    commands.entity(e).with_children(|p| m.spawn(p, 0.0));
                    sprites.keys.insert(id, key);
                }
                if let Some((text, c)) = label {
                    if let Ok(ch) = children.get(e) {
                        for c_e in ch.iter() {
                            if let Ok((mut t2, mut tc)) = labels.get_mut(c_e) {
                                if t2.0 != text {
                                    t2.0 = text.clone();
                                }
                                tc.0 = c;
                            }
                        }
                    }
                }
            }
            None => {
                let m = build();
                let mut ec = commands.spawn((tf, Visibility::Visible, Ent));
                ec.with_children(|p| {
                    m.spawn(p, 0.0);
                    if let Some((text, c)) = label {
                        p.spawn((Text2d::new(text), TextFont { font_size: 11.0, ..default() }, TextColor(c), Transform::from_translation(Vec3::new(0.0, TILE * 1.4, 1.0)), Label));
                    }
                });
                let e = ec.id();
                sprites.ents.insert(id, e);
                sprites.keys.insert(id, key);
            }
        }
    };

    // The player's own tank is the predicted one when there is one
    // (a client), interpolated from its last predicted tick rather than
    // from the snapshot; every other tank is the snapshot's.
    let predicted = view.predicted.as_ref().filter(|(t, _)| t.owner == me);
    for t in &obs.tanks {
        let (t, ahead) = match predicted {
            Some((p, at)) if p.id == t.id => (p, at.elapsed().as_secs_f32().min(0.1)),
            _ => (t, ahead),
        };
        let pos = t.pos + V2::from_heading(t.heading).scale(t.speed * ahead);
        let c = player_color(t.owner);
        let name = obs.players.get(t.owner as usize).map(|p| p.name.clone()).unwrap_or_default();
        let label = format!("{}{} {:.0}", if t.owner == me { "* " } else { "" }, name, t.armor);
        let alive = t.alive;
        // Damage in tenths, so the hull chips as the armor goes.
        let damage = (1.0 - t.armor / TANK_ARMOR).clamp(0.0, 1.0);
        let bucket = (damage * 10.0).round() as u32;
        place(&mut commands, t.id, to_screen(pos), -t.heading, model_key(1, c, alive as u32 | (bucket << 1)), &|| voxel::tank(c, alive, bucket as f32 / 10.0), 2.0, Some((label, if alive { c } else { Color::srgb(0.5, 0.5, 0.5) })));
    }
    for d in &obs.drones {
        let pos = d.pos + V2::from_heading(d.heading).scale(drone_speed(d.kind) * ahead * 0.5);
        let base = player_color(d.owner);
        let c = if d.kind == DroneKind::Strike && d.owner == SWARM_OWNER { Color::srgb(1.0, 0.25, 0.2) } else { base };
        let label = if d.owner == me {
            let k = match d.kind {
                DroneKind::Recon => "R",
                DroneKind::Interceptor => "I",
                DroneKind::Strike => "S",
                DroneKind::Longshot => "L",
            };
            // A longshot has no link to show: only its time left.
            if d.kind == DroneKind::Longshot {
                Some((format!("{} {:.0}s", k, d.battery), voxel::LIT))
            } else {
                Some((format!("{} {:.0}% {:.0}s", k, d.link * 100.0, d.battery), link_color(d.link)))
            }
        } else {
            None
        };
        let kind = d.kind;
        place(&mut commands, d.id, to_screen(pos), -d.heading, model_key(2 + kind as u8, c, 0), &|| match kind {
            DroneKind::Recon => voxel::recon(c),
            DroneKind::Interceptor => voxel::interceptor(c),
            DroneKind::Strike => voxel::strike(c),
            DroneKind::Longshot => voxel::longshot(c),
        }, 4.0, label);
    }
    for s in &obs.shells {
        let p = s.pos_at(obs.tick);
        let c = if s.kind == ShellKind::Indirect { Color::srgb(1.0, 0.9, 0.6) } else { Color::srgb(1.0, 1.0, 1.0) };
        place(&mut commands, s.id, to_screen(p), 0.0, model_key(6, c, 0), &|| voxel::shell(c), 3.0, None);
    }
    for d in &obs.depots {
        let owner = d.owner.map(player_color);
        let c = owner.unwrap_or(Color::srgb(0.7, 0.7, 0.7));
        place(&mut commands, d.id, to_screen(d.pos), 0.0, model_key(7, c, d.owner.is_some() as u32), &|| voxel::depot(owner), 1.0, Some(("depot".into(), c)));
    }
    // Pillboxes: Bolo's pyramid, tinted by the owner, the gun on top
    // while it has more than half its health; labelled with health.
    for p in &obs.pillboxes {
        let owner = p.owner.map(player_color);
        let c = owner.unwrap_or(Color::srgb(0.75, 0.75, 0.7));
        let alive = p.alive();
        let health = p.health / PILLBOX_HEALTH;
        let label = if alive { format!("pb {:.0}", p.health) } else { "pb dead".to_string() };
        let bucket = if !alive { 0 } else { 1 + (health * 8.0).round() as u32 };
        let h = if alive { ((health * 8.0).round() / 8.0).clamp(0.0, 1.0) } else { 0.0 };
        place(&mut commands, p.id, to_screen(p.pos), 0.0, model_key(8, c, bucket), &|| voxel::pillbox(owner, h, alive), 1.5, Some((label, if alive { c } else { Color::srgb(0.45, 0.42, 0.4) })));
    }
    // Structures (step 10b): each kind its own shape on its pad, the
    // owner's mark along the front, lamps lit while it works; labelled
    // with its name and health, and DARK when it has no power.
    for x in &obs.structures {
        let owner = x.owner.map(player_color);
        let c = owner.unwrap_or(Color::srgb(0.75, 0.75, 0.7));
        let alive = x.alive();
        let health = x.health / STRUCTURE_HEALTH;
        let word = crate::commander::name(x.kind);
        let label = if !alive {
            format!("{} wreck", word)
        } else if x.owner.is_some() && !x.powered {
            format!("{} {:.0} DARK", word, x.health)
        } else {
            format!("{} {:.0}", word, x.health)
        };
        let bucket = if !alive { 0 } else { 1 + (health * 8.0).round() as u32 } | ((x.powered as u32) << 5) | ((x.kind as u32) << 6);
        let h = ((health * 8.0).round() / 8.0).clamp(0.0, 1.0);
        let (kind, powered) = (x.kind, x.powered);
        place(&mut commands, x.id, to_screen(x.pos), 0.0, model_key(11, c, bucket), &|| voxel::structure(kind, owner, h, alive, powered), 1.4, Some((label, if alive { c } else { Color::srgb(0.45, 0.42, 0.4) })));
    }
    // Mines I know about: mine in my colour, found hostile ones red.
    for m in &obs.mines {
        let c = if m.owner == me { player_color(me) } else { voxel::MINE };
        place(&mut commands, m.id, to_screen(m.pos), 0.0, model_key(9, c, 0), &|| voxel::mine(c), 1.2, None);
    }
    // Squads out in the field: two men in the owner's colour, keyed by
    // the tank id with a high bit so it never collides.
    for t in obs.tanks.iter().filter(|t| t.alive && t.pillboys.deployed()) {
        let c = player_color(t.owner);
        let label = if t.owner == me { Some((format!("PB {}", t.pillboys.strength), c)) } else { None };
        place(&mut commands, t.id | 0x8000_0000, to_screen(t.pillboys.pos), 0.0, model_key(10, c, 0), &|| voxel::squad(c), 2.5, label);
    }
    // Anything not in this observation is hidden, not despawned: it will
    // most likely be back in the next one.
    let gone: Vec<(EntityId, Entity)> = sprites.ents.iter().filter(|(id, _)| !seen.contains(id)).map(|(a, b)| (*a, *b)).collect();
    for (_, e) in gone {
        if let Ok((_, mut v)) = q.get_mut(e) {
            *v = Visibility::Hidden;
        }
    }
}

/// Throw cubes out of every impact the view recorded, and move the
/// ones in the air: gravity, a bounce, a roll that slows, a fade.
fn debris(mut commands: Commands, time: Res<Time>, mut view: ResMut<View>, mut q: Query<(Entity, &mut Debris, &mut Transform, &mut Sprite)>) {
    let dt = time.delta_secs().min(0.1);
    let impacts: Vec<(V2, Color, u8)> = view.impacts.drain(..).collect();
    let mut seed = (time.elapsed_secs() * 1000.0) as u32;
    let mut rnd = || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        (seed % 10000) as f32 / 10000.0
    };
    for (at, color, n) in impacts {
        for _ in 0..n {
            let a = rnd() * std::f32::consts::TAU;
            let speed = TILE * (1.5 + rnd() * 3.0);
            let size = TILE * (0.08 + rnd() * 0.1);
            commands.spawn((
                Sprite { color, custom_size: Some(Vec2::splat(size)), ..default() },
                Transform::from_translation(to_screen(at).extend(5.0)),
                Debris { vel: Vec2::new(a.cos(), a.sin()) * speed, h: 0.5, vh: TILE * (1.0 + rnd() * 2.5), spin: (rnd() - 0.5) * 20.0, life: 1.2 + rnd() * 0.8 },
            ));
        }
    }
    let g = TILE * 9.0;
    for (e, mut d, mut tf, mut s) in q.iter_mut() {
        d.life -= dt;
        if d.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        let ground = tf.translation.truncate() - Vec2::new(0.0, d.h);
        let ground = ground + d.vel * dt;
        d.h += d.vh * dt;
        d.vh -= g * dt;
        if d.h < 0.0 {
            d.h = 0.0;
            d.vh = -d.vh * 0.35;
            d.vel *= 0.55;
            if d.vh.abs() < TILE * 0.3 {
                d.vh = 0.0;
            }
        }
        d.vel *= 1.0 - 2.5 * dt;
        tf.translation = (ground + Vec2::new(0.0, d.h)).extend(5.0);
        tf.rotate_z(d.spin * dt);
        if d.life < 0.4 {
            s.color = s.color.with_alpha(d.life / 0.4);
        }
    }
}

pub fn link_color(link: f32) -> Color {
    if link > 0.6 {
        Color::srgb(0.4, 1.0, 0.5)
    } else if link > LINK_FLOOR {
        Color::srgb(1.0, 0.85, 0.3)
    } else {
        Color::srgb(1.0, 0.3, 0.3)
    }
}

/// The telemetry overlay. Everything here is a line: cheap, redrawn every
/// frame, no entities to manage.
fn overlay(mut gizmos: Gizmos, view: Res<View>, hud: Res<HudState>, window: Query<&Window, With<PrimaryWindow>>, cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>) {
    let Some(obs) = view.obs.as_ref() else { return };
    // An armed builder mode: the square under the cursor, in the colour
    // of what the squad would make there, so the order lands where it
    // was meant (PLAYABILITY.md items 4 and 5).
    if let Some(mode) = hud.mode {
        if let (Ok(w), Ok((c, c_tf))) = (window.single(), cam.single()) {
            if let Some(p) = cursor_tile(w, c, c_tf) {
                let centre = to_screen(Map::tile_centre(p));
                let color = match mode {
                    Mode::Farm => FOREST,
                    Mode::Road => ROAD,
                    Mode::Wall if hud.concrete => voxel::CONCRETE,
                    Mode::Wall | Mode::Pillbox => voxel::TAN,
                    Mode::Mine => voxel::MINE,
                    Mode::Works => voxel::LIT,
                };
                gizmos.rect_2d(centre, Vec2::splat(TILE), color);
                gizmos.rect_2d(centre, Vec2::splat(TILE + 3.0), color.with_alpha(0.4));
            }
        }
    }
    let me = obs.player;
    let Some(my) = obs.me.as_ref() else { return };
    let mine = player_color(me);
    // The own tank is drawn where the predictor has it; the rings go
    // with it.
    let my_pos = view.predicted.as_ref().filter(|(t, _)| t.owner == me).map(|(t, _)| t.pos).unwrap_or(my.pos);

    if my.alive {
        // The bubble: where interceptors hold station, and detection.
        gizmos.circle_2d(to_screen(my_pos), my.bubble * TILE, mine.with_alpha(0.7));
        gizmos.circle_2d(to_screen(my_pos), (my.bubble + DETECT_MARGIN) * TILE, mine.with_alpha(0.18));
        // Own vision, and the gun's envelope: faint always, lit while
        // the right button is held (the prior art's gunsight), so a shot
        // inside minimum range or beyond maximum is never ordered blind.
        gizmos.circle_2d(to_screen(my_pos), TANK_VISION * TILE, Color::srgba(1.0, 1.0, 1.0, 0.08));
        let gun = if hud.aiming { 0.8 } else { 0.12 };
        gizmos.circle_2d(to_screen(my_pos), INDIRECT_MIN_RANGE * TILE, Color::srgba(1.0, 0.8, 0.4, gun));
        gizmos.circle_2d(to_screen(my_pos), INDIRECT_MAX_RANGE * TILE, Color::srgba(1.0, 0.8, 0.4, gun));
        if hud.aiming {
            gizmos.circle_2d(to_screen(my_pos), DIRECT_RANGE * TILE, Color::srgba(1.0, 1.0, 1.0, 0.5));
        }
        // Where it is driving: the leg in progress, then the rest of the
        // route the world planned, fainter.
        if let Some(g) = my.goal {
            gizmos.line_2d(to_screen(my.pos), to_screen(g), mine.with_alpha(0.4));
            let mut prev = g;
            for w in &my.route {
                gizmos.line_2d(to_screen(prev), to_screen(*w), mine.with_alpha(0.2));
                prev = *w;
            }
            gizmos.circle_2d(to_screen(prev), TILE * 0.4, mine.with_alpha(0.6));
        }

        // Barrel.
        let muzzle = my.pos + V2::from_heading(my.heading).scale(1.4);
        gizmos.line_2d(to_screen(my.pos), to_screen(muzzle), mine);
    }
    // Drones: link line to the tank, vision for recon, and for each one
    // of mine, its orbit.
    for d in &obs.drones {
        let tank = d.tether.and_then(|t| obs.tanks.iter().find(|x| x.id == t));
        if d.owner == me {
            if let Some(t) = tank {
                gizmos.line_2d(to_screen(t.pos), to_screen(d.pos), link_color(d.link).with_alpha(0.35 + 0.5 * d.link));
            }
            match d.kind {
                DroneKind::Recon => {
                    gizmos.circle_2d(to_screen(d.pos), RECON_VISION * TILE, mine.with_alpha(0.25));
                    if let DroneTask::Orbit { center, radius } = &d.task {
                        gizmos.circle_2d(to_screen(*center), *radius * TILE, mine.with_alpha(0.12));
                    }
                }
                DroneKind::Interceptor => {
                    if let DroneTask::Intercept { target } = &d.task {
                        if let Some(t) = obs.drones.iter().find(|x| x.id == *target) {
                            gizmos.line_2d(to_screen(d.pos), to_screen(t.pos), Color::srgba(1.0, 0.4, 0.3, 0.8));
                        }
                    }
                }
                DroneKind::Strike => {
                    if let DroneTask::Strike { target } = &d.task {
                        if let Some(t) = obs.tanks.iter().find(|x| x.id == *target) {
                            gizmos.line_2d(to_screen(d.pos), to_screen(t.pos), Color::srgba(1.0, 0.5, 0.2, 0.5));
                        }
                    }
                }
                // A longshot: the line to where it was sent, and a cross
                // on the point.
                DroneKind::Longshot => {
                    if let DroneTask::Attack { at } = &d.task {
                        gizmos.line_2d(to_screen(d.pos), to_screen(*at), Color::srgba(1.0, 0.75, 0.3, 0.35));
                        gizmos.cross_2d(to_screen(*at), TILE * 0.8, Color::srgba(1.0, 0.75, 0.3, 0.8));
                    }
                }
            }
        } else if d.kind == DroneKind::Longshot {
            // A hostile longshot: its track ahead, so a commander can
            // read where it is going. Where it was sent is its own.
            let c = player_color(d.owner);
            gizmos.circle_2d(to_screen(d.pos), TILE * 0.8, Color::srgb(1.0, 0.25, 0.2));
            gizmos.line_2d(to_screen(d.pos), to_screen(d.pos + V2::from_heading(d.heading).scale(12.0)), c.with_alpha(0.5));
        } else {
            // Hostile air: a red ring so it reads at a glance.
            let c = if d.owner == SWARM_OWNER { Color::srgb(1.0, 0.25, 0.2) } else { player_color(d.owner) };
            gizmos.circle_2d(to_screen(d.pos), TILE * 0.6, c.with_alpha(0.6));
        }
    }
    // Shells: the arc from gun to impact, and the impact point. Mine in
    // my colour; anything else I can see in white.
    for s in &obs.shells {
        let c = if s.owner == me { mine } else { Color::WHITE };
        let p = s.progress(obs.tick);
        let (a, b, imp) = (to_screen(s.from), to_screen(s.pos_at(obs.tick)), to_screen(s.impact));
        gizmos.line_2d(a, b, c.with_alpha(0.5));
        gizmos.line_2d(b, imp, c.with_alpha(0.15));
        gizmos.circle_2d(imp, TILE * (0.3 + 1.2 * p), c.with_alpha(0.3 + 0.5 * p));
        if s.kind == ShellKind::Indirect {
            gizmos.circle_2d(imp, SPLASH_RADIUS * TILE, c.with_alpha(0.12));
        }
    }
    // Other tanks I can see: their bubble, so I know what a strike faces.
    for t in obs.tanks.iter().filter(|t| t.owner != me && t.alive) {
        let cover = obs.drones.iter().filter(|d| d.owner == t.owner && d.kind == DroneKind::Interceptor).count();
        if cover > 0 {
            gizmos.circle_2d(to_screen(t.pos), t.bubble * TILE, player_color(t.owner).with_alpha(0.3));
        }
    }
    // Pillbox range rings: hostile ones red, so the player knows what a
    // drive-by costs; mine faint in my colour. Capture progress inside.
    for p in obs.pillboxes.iter().filter(|p| p.alive()) {
        let c = if p.owner == Some(me) { mine.with_alpha(0.15) } else { Color::srgba(1.0, 0.3, 0.25, 0.22) };
        gizmos.circle_2d(to_screen(p.pos), PILLBOX_RANGE * TILE, c);
    }
    for p in obs.pillboxes.iter().filter(|p| p.capture > 0) {
        if let Some(c) = p.capturer {
            let f = p.capture as f32 / PILLBOX_CAPTURE_TICKS as f32;
            gizmos.circle_2d(to_screen(p.pos), TILE * 2.0 * f, player_color(c).with_alpha(0.6));
        }
    }
    // Structures: a jammer's reach (red: it takes every link in it,
    // mine too), a power plant's faint, and capture progress.
    for x in obs.structures.iter().filter(|x| x.working()) {
        match x.kind {
            StructureKind::Jammer => {
                gizmos.circle_2d(to_screen(x.pos), JAMMER_RANGE * TILE, Color::srgba(1.0, 0.3, 0.25, 0.35));
            }
            StructureKind::Power if x.owner.map(|o| o == me || obs.allied(o)).unwrap_or(false) => {
                gizmos.circle_2d(to_screen(x.pos), POWER_RANGE * TILE, Color::srgba(1.0, 0.9, 0.35, 0.08));
            }
            _ => {}
        }
    }
    for x in obs.structures.iter().filter(|x| x.capture > 0) {
        if let Some(c) = x.capturer {
            let f = x.capture as f32 / PILLBOX_CAPTURE_TICKS as f32;
            gizmos.circle_2d(to_screen(x.pos), TILE * 2.0 * f, player_color(c).with_alpha(0.6));
        }
    }
    // My squad's walk: a line from the tank to the squad and on to its site.
    if my.alive && my.pillboys.deployed() {
        gizmos.line_2d(to_screen(my.pos), to_screen(my.pillboys.pos), mine.with_alpha(0.5));
        if let Some(w) = my.pillboys.route.last() {
            gizmos.line_2d(to_screen(my.pillboys.pos), to_screen(*w), mine.with_alpha(0.25));
        }
    }
    // Depot capture rings.

    for d in &obs.depots {
        gizmos.circle_2d(to_screen(d.pos), DEPOT_RADIUS * TILE, Color::srgba(0.8, 0.8, 0.8, 0.15));
        if d.capture > 0 {
            if let Some(c) = d.capturer {
                let f = d.capture as f32 / CAPTURE_TICKS as f32;
                gizmos.circle_2d(to_screen(d.pos), DEPOT_RADIUS * TILE * f, player_color(c).with_alpha(0.5));
            }
        }
    }
    // Map edge, at the map's own size.
    let (mw, mh) = (obs.map_w as f32, obs.map_h as f32);
    gizmos.rect_2d(Vec2::new(mw * TILE * 0.5, -mh * TILE * 0.5), Vec2::new(mw * TILE, mh * TILE), Color::srgba(1.0, 1.0, 1.0, 0.2));

}
