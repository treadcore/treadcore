//! Keyboard and mouse to orders, and the camera.

use crate::hud::HudState;
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
}

impl Default for Cam {
    fn default() -> Cam {
        Cam { follow: true, centred_once: false }
    }
}

fn cursor_tile(window: &Window, cam: &Camera, cam_tf: &GlobalTransform) -> Option<V2> {
    let c = window.cursor_position()?;
    let w = cam.viewport_to_world_2d(cam_tf, c).ok()?;
    Some(to_tile(w))
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
    if keys.just_pressed(KeyCode::KeyF) {
        cam.follow = !cam.follow;
    }
    if pan != Vec2::ZERO {
        cam.follow = false;
        let v = pan.normalize() * 600.0 * o.scale * time.delta_secs();
        tf.translation += v.extend(0.0);
    }
    if cam.follow || !cam.centred_once {
        if let Some(t) = view.obs.as_ref().and_then(|o| o.me.as_ref()) {
            let target = to_screen(t.pos);
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
        hud.show_help = !hud.show_help;
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
            // Route around water with the same pathfinder the bot uses;
            // the first corner becomes the goal, and the rest follow as
            // the tank arrives (the sim drives straight lines only).
            let wp = map.map(|m| m.route(me.pos, p)).unwrap_or_default();
            let to = wp.first().copied().unwrap_or(p);
            orders.0.push(Command::Move { to });
        }
    }
    if mouse.just_pressed(MouseButton::Right) {
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
