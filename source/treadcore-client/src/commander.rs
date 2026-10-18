//! The commander's picture (roadmap step 10d): what a side holds and
//! what it is losing, at a glance and in one key.
//!
//! Alerts: a structure of my side hit, wrecked, taken or gone dark, a
//! pillbox of mine killed or taken, a depot of mine taken, each put on
//! the newswire in a line that says where, with the alarm; `J` jumps
//! the view to the newest. The minimap (PLAYABILITY item 11): the whole
//! map in the bottom strip's corner, the ground dimmed where the side
//! cannot see, every depot, pillbox and structure in its owner's
//! colour, the tanks the side sees, the player's own in white, and an
//! alert's place blinking red; a click on it moves the view there.
//! Everything here is read from the observation the window already has:
//! the sim is not asked anything new.

use crate::input::Cam;
use crate::render::{terrain_color, MainCamera};
use crate::session::View;
use crate::{player_color, to_screen};
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use std::time::Instant;
use treadcore_sim::*;
use treadcore_sim::Event;

pub struct CommanderPlugin;

impl Plugin for CommanderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Minimap>().add_systems(Update, (draw_minimap, minimap_click, jump_to_alert));
    }
}

/// An alert: where, what, when, and about which thing (so a structure
/// under a rain of shells is one alert every few seconds, not twenty).
#[derive(Clone, Debug)]
pub struct Alert {
    pub at: V2,
    pub text: String,
    pub when: Instant,
    pub about: EntityId,
}

/// How long an alert blinks on the minimap and answers to `J`.
pub const ALERT_HOLD: f32 = 12.0;
/// The same thing hit again inside this is not a new alert.
const ALERT_REPEAT: f32 = 4.0;

/// A structure's name on the newswire.
pub fn name(k: StructureKind) -> &'static str {
    match k {
        StructureKind::Power => "power plant",
        StructureKind::Refinery => "refinery",
        StructureKind::DataCentre => "data centre",
        StructureKind::Workshop => "workshop",
        StructureKind::Jammer => "jammer",
    }
}

/// The alert an event raises for this player's side, if any. The
/// observation is still the one before the event, so a structure about
/// to change hands still shows its old owner.
pub fn alert_for(e: &Event, me: PlayerId, view: &View) -> Option<Alert> {
    let obs = view.obs.as_ref()?;
    let ours = |o: Option<PlayerId>| o.map(|o| o == me || obs.allied(o)).unwrap_or(false);
    let whose = |o: Option<PlayerId>| if o == Some(me) { "your" } else { "an ally's" };
    let structure = |id: EntityId| obs.structures.iter().find(|x| x.id == id);
    let at_tile = |p: V2| format!("{:.0},{:.0}", p.x, p.y);
    let (about, at, text) = match e {
        Event::StructureDamaged { structure: id, kind, owner, health_left, .. } if ours(*owner) && *health_left > 0.0 => {
            let p = structure(*id)?.pos;
            (*id, p, format!("{} {} HIT at {} ({:.0}%)", whose(*owner), name(*kind), at_tile(p), health_left / STRUCTURE_HEALTH * 100.0))
        }
        Event::StructureWrecked { structure: id, kind, owner, .. } if ours(*owner) => {
            let p = structure(*id)?.pos;
            (*id, p, format!("{} {} WRECKED at {}", whose(*owner), name(*kind), at_tile(p)))
        }
        Event::StructureCaptured { structure: id, kind, by, from } if ours(*from) && !ours(Some(*by)) => {
            let p = structure(*id)?.pos;
            (*id, p, format!("{} {} TAKEN at {} by p{}", whose(*from), name(*kind), at_tile(p), by))
        }
        Event::StructurePower { structure: id, kind, owner, on: false } if ours(Some(*owner)) => {
            let p = structure(*id)?.pos;
            (*id, p, format!("{} {} DARK at {}: no power", whose(Some(*owner)), name(*kind), at_tile(p)))
        }
        Event::PillboxDestroyed { pillbox, owner, .. } if *owner == Some(me) => {
            let p = obs.pillboxes.iter().find(|b| b.id == *pillbox)?.pos;
            (*pillbox, p, format!("your pillbox DEAD at {}", at_tile(p)))
        }
        Event::PillboxCaptured { pillbox, from: Some(f), .. } if *f == me => {
            let p = obs.pillboxes.iter().find(|b| b.id == *pillbox)?.pos;
            (*pillbox, p, format!("your pillbox TAKEN at {}", at_tile(p)))
        }
        Event::DepotCaptured { depot, by } if *by != me => {
            let d = obs.depots.iter().find(|d| d.id == *depot)?;
            if d.owner != Some(me) {
                return None;
            }
            (*depot, d.pos, format!("your depot TAKEN at {} by p{}", at_tile(d.pos), by))
        }
        _ => return None,
    };
    let repeat = view.alerts.iter().rev().any(|a| a.about == about && a.when.elapsed().as_secs_f32() < ALERT_REPEAT);
    if repeat {
        return None;
    }
    Some(Alert { at, text, when: Instant::now(), about })
}

/// The minimap's picture, rebuilt a few times a second.
#[derive(Resource, Default)]
pub struct Minimap {
    pub image: Option<Handle<Image>>,
    pub size: (usize, usize),
    pub drawn: Option<Instant>,
}

/// The node the minimap is drawn in (hud.rs places it).
#[derive(Component)]
pub struct MinimapNode;

/// The minimap's box at the foot of the right column, in pixels: the
/// column's width less its padding and the box's bevel, and as tall.
pub const MINIMAP_H: f32 = crate::hud::RIGHT - 18.0;
pub const MINIMAP_MAX_W: f32 = crate::hud::RIGHT - 18.0;

fn draw_minimap(
    view: Res<View>,
    mut minimap: ResMut<Minimap>,
    mut images: ResMut<Assets<Image>>,
    mut nodes: Query<(&mut ImageNode, &mut Node), With<MinimapNode>>,
    fog: Res<crate::render::Fog>,
) {
    let (Some(map), Some(obs)) = (view.map.as_ref(), view.obs.as_ref()) else { return };
    // Four times a second; ten while a ping's circles spread.
    let pinging = view.map_pings.iter().any(|(_, _, t)| t.elapsed().as_secs_f32() < MAP_PING_FOR);
    let every = if pinging { 0.1 } else { 0.25 };
    if minimap.drawn.map(|d| d.elapsed().as_secs_f32() < every).unwrap_or(false) {
        return;
    }
    minimap.drawn = Some(Instant::now());
    let (w, h) = (map.w, map.h);
    if minimap.image.is_none() || minimap.size != (w, h) {
        let mut img = Image::new_fill(Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 }, TextureDimension::D2, &[0, 0, 0, 255], TextureFormat::Rgba8UnormSrgb, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
        img.sampler = ImageSampler::nearest();
        let handle = images.add(img);
        let scale = (MINIMAP_H / h as f32).min(MINIMAP_MAX_W / w as f32);
        for (mut n, mut node) in nodes.iter_mut() {
            n.image = handle.clone();
            node.width = Val::Px(w as f32 * scale);
            node.height = Val::Px(h as f32 * scale);
        }
        minimap.image = Some(handle);
        minimap.size = (w, h);
    }
    let Some(img) = minimap.image.as_ref().and_then(|h| images.get_mut(h)) else { return };
    let Some(data) = img.data.as_mut() else { return };
    let srgb = |c: Color| {
        let s = c.to_srgba();
        [(s.red * 255.0) as u8, (s.green * 255.0) as u8, (s.blue * 255.0) as u8, 255]
    };
    // The ground, dimmed outside what the side can see.
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let mut c = terrain_color(map.at_tile(x, y));
            if !fog.visible.get(i).copied().unwrap_or(true) {
                c = c.darker(0.12);
            }
            data[i * 4..i * 4 + 4].copy_from_slice(&srgb(c));
        }
    }
    // A mark is a square a few tiles wide, so a 512-tile map squeezed
    // into the box still shows it.
    let r = ((w.max(h) as f32 / 128.0).ceil() as i32).max(1);
    let mut mark = |p: V2, c: Color, big: i32| {
        let (cx, cy) = p.tile();
        let rr = r * big;
        for y in cy - rr..=cy + rr {
            for x in cx - rr..=cx + rr {
                if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                    let i = y as usize * w + x as usize;
                    data[i * 4..i * 4 + 4].copy_from_slice(&srgb(c));
                }
            }
        }
    };
    let me = obs.player;
    let owner_color = |o: Option<PlayerId>, alive: bool| {
        if !alive {
            Color::srgb(0.15, 0.15, 0.15)
        } else {
            match o {
                Some(o) if o == me => player_color(me),
                Some(o) if obs.allied(o) => Color::srgb(0.2, 0.95, 0.3),
                Some(_) => Color::srgb(0.95, 0.2, 0.15),
                None => Color::srgb(0.75, 0.72, 0.62),
            }
        }
    };
    for d in &obs.depots {
        mark(d.pos, owner_color(d.owner, true), 1);
    }
    for p in &obs.pillboxes {
        mark(p.pos, owner_color(p.owner, p.alive()), 0);
    }
    for x in &obs.structures {
        mark(x.pos, owner_color(x.owner, x.alive()), 1);
    }
    for t in obs.tanks.iter().filter(|t| t.alive && t.owner != me) {
        mark(t.pos, player_color(t.owner), 1);
    }
    let blink = (obs.tick / 5) % 2 == 0;
    for a in view.alerts.iter().filter(|a| a.when.elapsed().as_secs_f32() < ALERT_HOLD) {
        if blink {
            mark(a.at, Color::srgb(1.0, 0.1, 0.1), 2);
        }
    }
    if let Some(t) = obs.me.as_ref().filter(|t| t.alive) {
        mark(t.pos, Color::WHITE, 1);
    }
    // Pings: rings in the pinger's colour spreading from the point, two
    // a second-and-a-third apart, fading as they go, for `MAP_PING_FOR`.
    let reach = (w.max(h) as f32 / 12.0).max(6.0);
    let mut put = |x: i32, y: i32, c: [u8; 4], a: f32| {
        if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
            let i = (y as usize * w + x as usize) * 4;
            for k in 0..3 {
                data[i + k] = (data[i + k] as f32 * (1.0 - a) + c[k] as f32 * a) as u8;
            }
        }
    };
    for (at, who, since) in &view.map_pings {
        let age = since.elapsed().as_secs_f32();
        if age >= MAP_PING_FOR {
            continue;
        }
        let c = srgb(player_color(*who));
        for wave in [0.0, 0.67] {
            let phase = ((age + wave) / 1.33).fract();
            let r = 1.0 + phase * reach;
            let a = (1.0 - phase) * (1.0 - age / MAP_PING_FOR) + 0.15;
            let steps = (r * 6.3) as usize + 8;
            for s in 0..steps {
                let ang = s as f32 / steps as f32 * std::f32::consts::TAU;
                let (px, py) = (at.x + r * ang.cos(), at.y + r * ang.sin());
                for d in 0..r.clamp(1.0, 2.0) as i32 {
                    put(px as i32 + d, py as i32, c, a.min(1.0));
                }
            }
        }
        // The point itself, solid.
        put(at.x as i32, at.y as i32, c, 1.0);
    }
}

/// How long a ping spreads on the minimap.
pub const MAP_PING_FOR: f32 = 4.0;

/// Put the view's centre on a tile, as the pillbox view does.
pub fn look_at(cam: &mut Cam, tf: &mut Transform, scale: f32, at: V2) {
    cam.follow = false;
    cam.pillbox = None;
    let target = to_screen(at) - crate::input::frame_offset() * Vec2::new(1.0, -1.0) * scale;
    tf.translation = target.extend(tf.translation.z);
}

fn minimap_click(
    view: Res<View>,
    mut cam: ResMut<Cam>,
    nodes: Query<(&Interaction, &RelativeCursorPosition), With<MinimapNode>>,
    mut q: Query<(&mut Transform, &Projection), With<MainCamera>>,
) {
    let Some(map) = view.map.as_ref() else { return };
    let Ok((mut tf, proj)) = q.single_mut() else { return };
    let Projection::Orthographic(o) = proj else { return };
    for (i, rel) in nodes.iter() {
        // Pressed, or held and dragged: the view follows the pointer.
        if *i != Interaction::Pressed {
            continue;
        }
        let Some(n) = rel.normalized else { continue };
        let at = V2::new(((n.x + 0.5) * map.w as f32).clamp(0.0, map.w as f32 - 0.5), ((n.y + 0.5) * map.h as f32).clamp(0.0, map.h as f32 - 0.5));
        look_at(&mut cam, &mut tf, o.scale, at);
    }
}

/// `J`: the view to the newest alert still standing.
fn jump_to_alert(keys: Res<ButtonInput<KeyCode>>, bind: Res<crate::keys::Keys>, view: Res<View>, mut cam: ResMut<Cam>, mut q: Query<(&mut Transform, &Projection), With<MainCamera>>) {
    if view.keys_taken() || !keys.just_pressed(bind.key(crate::keys::Action::Alert)) {
        return;
    }
    let Some(a) = view.alerts.iter().rev().find(|a| a.when.elapsed().as_secs_f32() < ALERT_HOLD) else { return };
    let Ok((mut tf, proj)) = q.single_mut() else { return };
    let Projection::Orthographic(o) = proj else { return };
    look_at(&mut cam, &mut tf, o.scale, a.at);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A structure of mine hit raises one alert with where it is; hit
    /// again at once, none; a hostile one hit, none.
    #[test]
    fn my_structures_raise_alerts_and_theirs_do_not() {
        let mut w = treadcore_sim::World::new(Scenario { swarm_interval: 0, industry: true, ..Scenario::bots(4) });
        let (mine, theirs) = (w.structures[0].id, w.structures[1].id);
        w.structures[0].owner = Some(0);
        w.structures[1].owner = Some(1);
        let mut view = View { obs: Some(w.observe(0)), ..View::default() };
        let hit = |id: EntityId, owner: PlayerId| Event::StructureDamaged { structure: id, kind: StructureKind::Power, owner: Some(owner), by: Some(2), damage: 30.0, health_left: 90.0 };
        let a = alert_for(&hit(mine, 0), 0, &view).expect("an alert for my structure");
        assert!(a.text.contains("your power plant HIT") && a.text.contains("75%"), "{}", a.text);
        view.alerts.push_back(a);
        assert!(alert_for(&hit(mine, 0), 0, &view).is_none(), "a repeat inside the hold is not a new alert");
        assert!(alert_for(&hit(theirs, 1), 0, &view).is_none());
        let dark = Event::StructurePower { structure: theirs, kind: StructureKind::Refinery, owner: 1, on: false };
        assert!(alert_for(&dark, 1, &view).is_some(), "seat 1 hears its own refinery go dark");
    }
}
