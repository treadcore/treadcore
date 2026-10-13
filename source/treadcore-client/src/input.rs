//! Keyboard and mouse to orders, and the camera.
//!
//! The keys are the orders column's (hud.rs): the digits launch and
//! deploy, the brackets set the bubble, the mouse drives and fires.
//! The views are the prior art's (SPEC UI-08 to UI-13): the tank view
//! follows the tank, the apostrophe steps through the player's and
//! allies' pillboxes, Return or F comes back to the tank, and the
//! scroll keys suspend following until then.

use crate::hud::{self, HudState};
use crate::render::MainCamera;
use crate::session::{Orders, View};
use crate::{to_screen, to_tile};
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use treadcore_sim::*;
use treadcore_sim::Command;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Cam>().add_systems(Update, (camera, orders));
    }
}

#[derive(Resource)]
pub struct Cam {
    pub follow: bool,
    pub centred_once: bool,
    /// Pillbox view: the index into the player's (and allies') pillboxes
    /// the camera sits on; the apostrophe steps it (SPEC UI-10, UI-12).
    pub pillbox: Option<usize>,
}

impl Default for Cam {
    fn default() -> Cam {
        Cam { follow: true, centred_once: false, pillbox: None }
    }
}

fn cursor_tile(window: &Window, cam: &Camera, cam_tf: &GlobalTransform) -> Option<V2> {
    let c = window.cursor_position()?;
    if hud::in_frame(c, window.size()) {
        return None;
    }
    let w = cam.viewport_to_world_2d(cam_tf, c).ok()?;
    Some(to_tile(w))
}

/// Where the map's middle is, relative to the window's, in pixels
/// (window coordinates, y down): the frame's columns and strips are
/// not symmetric, and the tank should sit in the middle of the map.
fn frame_offset() -> Vec2 {
    Vec2::new((hud::LEFT - hud::RIGHT) * 0.5, (hud::TOP - hud::BOTTOM) * 0.5)
}

fn camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    view: Res<View>,
    mut cam: ResMut<Cam>,
    mut q: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
) {
    let Ok((mut tf, mut proj)) = q.single_mut() else { return };
    let Projection::Orthographic(o) = &mut *proj else { return };
    if scroll.delta.y != 0.0 {
        o.scale = (o.scale * (1.0 - scroll.delta.y * 0.1)).clamp(0.15, 2.5);
    }
    let mut pan = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        pan.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        pan.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        pan.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        pan.x += 1.0;
    }
    // Tank view: Return or F. F on an already following camera stops
    // it, as before.
    if keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter) {
        cam.follow = true;
        cam.pillbox = None;
    }
    if keys.just_pressed(KeyCode::KeyF) {
        cam.follow = cam.pillbox.is_some() || !cam.follow;
        cam.pillbox = None;
    }
    // Pillbox view: the apostrophe steps through mine and my allies'.
    if keys.just_pressed(KeyCode::Quote) {
        if let Some(obs) = view.obs.as_ref() {
            let mine: Vec<&Pillbox> = obs.pillboxes.iter().filter(|p| p.owner.map(|o| o == obs.player || obs.allied(o)).unwrap_or(false)).collect();
            if !mine.is_empty() {
                let next = cam.pillbox.map(|i| (i + 1) % mine.len()).unwrap_or(0);
                cam.pillbox = Some(next);
                cam.follow = false;
                let target = to_screen(mine[next].pos) - frame_offset() * Vec2::new(1.0, -1.0) * o.scale;
                tf.translation = target.extend(tf.translation.z);
            }
        }
    }
    if pan != Vec2::ZERO {
        cam.follow = false;
        cam.pillbox = None;
        let v = pan.normalize() * 600.0 * o.scale * time.delta_secs();
        tf.translation += v.extend(0.0);
    }
    if cam.follow || !cam.centred_once {
        let tank = view.predicted.as_ref().map(|(t, _)| t.pos).or_else(|| view.obs.as_ref().and_then(|o| o.me.as_ref()).map(|t| t.pos));
        if let Some(pos) = tank {
            // The tank in the middle of the map area, not of the window.
            let target = to_screen(pos) - frame_offset() * Vec2::new(1.0, -1.0) * o.scale;
            let cur = tf.translation.truncate();
            let next = if cam.centred_once { cur.lerp(target, (4.0 * time.delta_secs()).min(1.0)) } else { target };
            tf.translation = next.extend(tf.translation.z);
            cam.centred_once = true;
        }
    }
}

fn orders(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    view: Res<View>,
    mut orders: ResMut<Orders>,
    mut hud: ResMut<HudState>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        hud.show_log = !hud.show_log;
    }
    if keys.just_pressed(KeyCode::Escape) {
        info!("quit");
        std::process::exit(0);
    }
    let Some(obs) = view.obs.as_ref() else { return };
    let Some(me) = obs.me.as_ref() else { return };
    let Ok(window) = window.single() else { return };
    let Ok((camera, cam_tf)) = cam.single() else { return };
    let at = cursor_tile(window, camera, cam_tf);
    let map = view.map.as_ref();

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(p) = at {
            // The world routes a Move around water itself and the tank
            // follows the waypoints; the client only names the destination.
            orders.0.push(Command::Move { to: p });
        }
    }
    // The gun: hold the right button and the envelope lights (render.rs);
    // release to fire where the cursor is.
    hud.aiming = mouse.pressed(MouseButton::Right);
    if mouse.just_released(MouseButton::Right) {
        if let Some(p) = at {
            let clear = map.map(|m| m.line_clear(me.pos, p)).unwrap_or(false);
            if me.pos.dist(p) <= DIRECT_RANGE && clear {
                orders.0.push(Command::FireDirect { at: p });
            } else {
                orders.0.push(Command::FireIndirect { at: p });
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit1) {
        orders.0.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
    }
    if keys.just_pressed(KeyCode::Digit2) {
        let center = at.unwrap_or(me.pos);
        orders.0.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center, radius: 4.0 } });
    }
    if keys.just_pressed(KeyCode::Digit3) {
        let p = at.unwrap_or(me.pos);
        if let Some(t) = obs.hostile_tanks().min_by(|a, b| a.pos.dist(p).partial_cmp(&b.pos.dist(p)).unwrap()) {
            orders.0.push(Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: t.id } });
        }
    }
    if keys.just_pressed(KeyCode::KeyR) {
        orders.0.push(Command::Recall);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        if let Some(p) = at {
            if let Some(pb) = obs.pillboxes.iter().min_by(|a, b| a.pos.dist(p).partial_cmp(&b.pos.dist(p)).unwrap()).filter(|pb| pb.pos.dist(p) <= 3.0) {
                let task = if pb.owner == Some(obs.player) { PillboyTask::Repair { target: pb.id } } else { PillboyTask::Capture { pillbox: pb.id } };
                orders.0.push(Command::Deploy { task });
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit5) {
        if let Some(p) = at {
            orders.0.push(Command::Deploy { task: PillboyTask::Build { what: BuildKind::Pillbox, at: p } });
        }
    }
    if keys.just_pressed(KeyCode::Digit6) {
        orders.0.push(Command::Deploy { task: PillboyTask::Return });
    }
    for (key, what) in [(KeyCode::Digit7, BuildKind::Wall), (KeyCode::Digit8, BuildKind::Mine), (KeyCode::Digit9, BuildKind::Road)] {
        if keys.just_pressed(key) {
            if let Some(p) = at {
                orders.0.push(Command::Deploy { task: PillboyTask::Build { what, at: p } });
            }
        }
    }
    if keys.just_pressed(KeyCode::Digit0) {
        if let Some(p) = at {
            orders.0.push(Command::Deploy { task: PillboyTask::Harvest { at: p } });
        }
    }
    if keys.just_pressed(KeyCode::BracketLeft) {
        orders.0.push(Command::SetBubble { radius: me.bubble - 1.0 });
    }
    if keys.just_pressed(KeyCode::BracketRight) {
        orders.0.push(Command::SetBubble { radius: me.bubble + 1.0 });
    }
    if keys.just_pressed(KeyCode::Space) {
        orders.0.push(Command::Stop);
    }
}
