//! Drawing the world: tiles as sprites, entities as sprites keyed by id,
//! and the telemetry overlay in gizmos -- bubbles, links, vision, arcs.
//!
//! The overlay is the point of the game's look. A tank is a small thing
//! on a big map; what the player watches is the ring of interceptors
//! around it, the lines to its drones going from green to red as they
//! fly out of link range, the recon disc sliding over the enemy, and the
//! arc of a shell leaving it.

use crate::session::View;
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
            .add_systems(Update, (build_map, update_fog, sync_entities, overlay).chain());
    }
}

#[derive(Component)]
pub struct MainCamera;

#[derive(Component)]
pub struct Tile(pub usize);

#[derive(Component)]
pub struct Ent(pub EntityId);

#[derive(Component)]
pub struct Label;

#[derive(Resource, Default)]
pub struct Sprites {
    pub map_built: bool,
    pub cells: Vec<u8>,
    pub tiles: Vec<Entity>,
    pub ents: HashMap<EntityId, Entity>,
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
        Transform::from_translation(Vec3::new(MAP_W as f32 * TILE * 0.5, MAP_H as f32 * TILE * 0.5, 0.0)),
    ));
}

pub fn terrain_color(t: Terrain) -> Color {
    match t {
        Terrain::Grass => Color::srgb(0.28, 0.42, 0.22),
        Terrain::Forest => Color::srgb(0.12, 0.27, 0.13),
        Terrain::Road => Color::srgb(0.46, 0.42, 0.34),
        Terrain::Water => Color::srgb(0.13, 0.25, 0.42),
        Terrain::Bridge => Color::srgb(0.55, 0.47, 0.33),
        Terrain::Rubble => Color::srgb(0.30, 0.26, 0.22),
        Terrain::Depot => Color::srgb(0.55, 0.55, 0.50),
    }
}

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
                        Tile(y * map.w + x),
                    ))
                    .id();
                ids.push(id);
            }
        }
        sprites.tiles = ids;
        sprites.cells = map.cells.clone();
        sprites.map_built = true;
        return;
    }
    // Craters, and fog changes: recolour only what changed.
    let fog_changed = fog.is_changed();
    for (i, (&new, &old)) in map.cells.iter().zip(sprites.cells.iter()).enumerate() {
        let vis = fog.visible.get(i).copied().unwrap_or(true);
        if new != old || fog_changed {
            if let Ok(mut s) = tiles.get_mut(sprites.tiles[i]) {
                let c = terrain_color(Terrain::from_u8(new));
                s.color = if vis { c } else { c.darker(0.12) };
            }
        }
    }
    if map.cells != sprites.cells {
        sprites.cells = map.cells.clone();
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

fn sync_entities(mut commands: Commands, view: Res<View>, mut sprites: ResMut<Sprites>, mut q: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<Ent>>, mut labels: Query<(&mut Text2d, &mut TextColor), With<Label>>, children: Query<&Children>) {
    let Some(obs) = view.obs.as_ref() else { return };
    let mut seen: Vec<EntityId> = Vec::new();
    let me = obs.player;
    // Interpolate forward from the observation by wall time, so a 10 Hz
    // snapshot stream still moves at 60 fps.
    let ahead = view.received.map(|r| r.elapsed().as_secs_f32().min(0.2)).unwrap_or(0.0);

    let mut place = |commands: &mut Commands, id: EntityId, pos: Vec2, rot: f32, size: Vec2, color: Color, z: f32, label: Option<(String, Color)>| {
        seen.push(id);
        let tf = Transform::from_translation(pos.extend(z)).with_rotation(Quat::from_rotation_z(rot));
        match sprites.ents.get(&id).copied() {
            Some(e) => {
                if let Ok((mut t, mut s, mut v)) = q.get_mut(e) {
                    *t = tf;
                    s.color = color;
                    s.custom_size = Some(size);
                    *v = Visibility::Visible;
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
                let mut ec = commands.spawn((Sprite { color, custom_size: Some(size), ..default() }, tf, Ent(id)));
                if let Some((text, c)) = label {
                    ec.with_children(|p| {
                        p.spawn((Text2d::new(text), TextFont { font_size: 11.0, ..default() }, TextColor(c), Transform::from_translation(Vec3::new(0.0, TILE * 1.4, 1.0)), Label));
                    });
                }
                let e = ec.id();
                sprites.ents.insert(id, e);
            }
        }
    };

    for t in &obs.tanks {
        let pos = t.pos + V2::from_heading(t.heading).scale(t.speed * ahead);
        let c = if t.alive { player_color(t.owner) } else { Color::srgb(0.2, 0.2, 0.2) };
        let name = obs.players.get(t.owner as usize).map(|p| p.name.clone()).unwrap_or_default();
        let label = format!("{}{} {:.0}", if t.owner == me { "* " } else { "" }, name, t.armor);
        place(&mut commands, t.id, to_screen(pos), -t.heading, Vec2::new(TILE * 1.6, TILE * 1.0), c, 2.0, Some((label, c)));
    }
    for d in &obs.drones {
        let pos = d.pos + V2::from_heading(d.heading).scale(drone_speed(d.kind) * ahead * 0.5);
        let base = player_color(d.owner);
        let (size, c) = match d.kind {
            DroneKind::Recon => (Vec2::splat(TILE * 0.7), base),
            DroneKind::Interceptor => (Vec2::splat(TILE * 0.5), base.lighter(0.15)),
            DroneKind::Strike => (Vec2::new(TILE * 0.8, TILE * 0.4), if d.owner == SWARM_OWNER { Color::srgb(1.0, 0.25, 0.2) } else { base.darker(0.1) }),
        };
        let label = if d.owner == me {
            let k = match d.kind {
                DroneKind::Recon => "R",
                DroneKind::Interceptor => "I",
                DroneKind::Strike => "S",
            };
            let lc = link_color(d.link);
            Some((format!("{} {:.0}% {:.0}s", k, d.link * 100.0, d.battery), lc))
        } else {
            None
        };
        place(&mut commands, d.id, to_screen(pos), -d.heading, size, c, 4.0, label);
    }
    for s in &obs.shells {
        let p = s.pos_at(obs.tick);
        let c = if s.kind == ShellKind::Indirect { Color::srgb(1.0, 0.9, 0.6) } else { Color::srgb(1.0, 1.0, 1.0) };
        place(&mut commands, s.id, to_screen(p), 0.0, Vec2::splat(TILE * 0.35), c, 3.0, None);
    }
    for d in &obs.depots {
        let c = d.owner.map(player_color).unwrap_or(Color::srgb(0.7, 0.7, 0.7));
        place(&mut commands, d.id, to_screen(d.pos), 0.0, Vec2::splat(TILE * 1.2), c, 1.0, Some(("depot".into(), c)));
    }
    // Pillboxes: a square, the owner's colour (grey neutral, dark dead),
    // labelled with health.
    for p in &obs.pillboxes {
        let base = p.owner.map(player_color).unwrap_or(Color::srgb(0.75, 0.75, 0.7));
        let c = if p.alive() { base } else { Color::srgb(0.25, 0.22, 0.2) };
        let label = if p.alive() { format!("pb {:.0}", p.health) } else { "pb dead".to_string() };
        place(&mut commands, p.id, to_screen(p.pos), 0.785, Vec2::splat(TILE * 1.0), c, 1.5, Some((label, c)));
    }
    // Squads out in the field: a small figure in the owner's colour,
    // keyed by the tank id with a high bit so it never collides.
    for t in obs.tanks.iter().filter(|t| t.alive && t.pillboys.deployed()) {
        let c = player_color(t.owner);
        let label = if t.owner == me { Some((format!("PB {}", t.pillboys.strength), c)) } else { None };
        place(&mut commands, t.id | 0x8000_0000, to_screen(t.pillboys.pos), 0.0, Vec2::splat(TILE * 0.45), c.lighter(0.1), 2.5, label);
    }
    // Anything not in this observation is hidden, not despawned: it will
    // most likely be back in the next one.
    let gone: Vec<(EntityId, Entity)> = sprites.ents.iter().filter(|(id, _)| !seen.contains(id)).map(|(a, b)| (*a, *b)).collect();
    for (_, e) in gone {
        if let Ok((_, _, mut v)) = q.get_mut(e) {
            *v = Visibility::Hidden;
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
fn overlay(mut gizmos: Gizmos, view: Res<View>) {
    let Some(obs) = view.obs.as_ref() else { return };
    let me = obs.player;
    let Some(my) = obs.me.as_ref() else { return };
    let mine = player_color(me);

    if my.alive {
        // The bubble: where interceptors hold station, and detection.
        gizmos.circle_2d(to_screen(my.pos), my.bubble * TILE, mine.with_alpha(0.7));
        gizmos.circle_2d(to_screen(my.pos), (my.bubble + DETECT_MARGIN) * TILE, mine.with_alpha(0.18));
        // Own vision and the gun's envelope.
        gizmos.circle_2d(to_screen(my.pos), TANK_VISION * TILE, Color::srgba(1.0, 1.0, 1.0, 0.08));
        gizmos.circle_2d(to_screen(my.pos), INDIRECT_MIN_RANGE * TILE, Color::srgba(1.0, 0.8, 0.4, 0.12));
        gizmos.circle_2d(to_screen(my.pos), INDIRECT_MAX_RANGE * TILE, Color::srgba(1.0, 0.8, 0.4, 0.12));
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
            }
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
    // Map edge.
    gizmos.rect_2d(Vec2::new(MAP_W as f32 * TILE * 0.5, MAP_H as f32 * TILE * 0.5), Vec2::new(MAP_W as f32 * TILE, MAP_H as f32 * TILE), Color::srgba(1.0, 1.0, 1.0, 0.2));
}
