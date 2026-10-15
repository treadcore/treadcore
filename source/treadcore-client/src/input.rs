//! Keyboard and mouse to orders, and the camera.
//!
//! The keys are the orders column's (hud.rs): the digits launch the
//! drones and arm the builder modes, the brackets set the bubble, the
//! mouse drives and fires. A builder mode is the prior art's (SPEC
//! BLD-01): arm it, by its key or its tile, and the next left click on
//! the map sends the squad to that square; the mode drops as it goes.
//! In pillbox mode a click on a pillbox repairs it, whoever owns it
//! (BLD-05), or captures it if it is dead or nobody's; on open ground
//! it builds one.
//! The views are the prior art's (SPEC UI-08 to UI-13): the tank view
//! follows the tank, the apostrophe steps through the player's and
//! allies' pillboxes, F comes back to the tank, and the scroll keys
//! suspend following until then. Enter is chat's (chat.rs): while a line
//! is open, no key here is read.

use crate::hud::{self, HudState, Mode, OrderTile, MODE_ROW, RETURN_ROW};
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
        app.init_resource::<Cam>().add_systems(Update, (camera, orders, crate::chat::chat.after(camera).after(orders), crate::proposals::proposals.after(crate::chat::chat), crate::keys::key_setup.after(crate::proposals::proposals)));
    }
}

#[derive(Resource)]
pub struct Cam {
    pub follow: bool,
    pub centred_once: bool,
    /// Pillbox view: the index into the player's (and allies') pillboxes
    /// the camera sits on; the apostrophe steps it (SPEC UI-10, UI-12).
    pub pillbox: Option<usize>,
    /// The shake applied last frame, taken off before this frame's.
    pub shake: Vec2,
}

impl Default for Cam {
    fn default() -> Cam {
        Cam { follow: true, centred_once: false, pillbox: None, shake: Vec2::ZERO }
    }
}

pub fn cursor_tile(window: &Window, cam: &Camera, cam_tf: &GlobalTransform) -> Option<V2> {
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
pub fn frame_offset() -> Vec2 {
    Vec2::new((hud::LEFT - hud::RIGHT) * 0.5, (hud::TOP - hud::BOTTOM) * 0.5)
}

fn camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    scroll: Res<AccumulatedMouseScroll>,
    view: Res<View>,
    mut cam: ResMut<Cam>,
    mut q: Query<(&mut Transform, &mut Projection), With<MainCamera>>,
    lamps: Query<(&Interaction, &hud::Lamp), Changed<Interaction>>,
    bind: Res<crate::keys::Keys>,
) {
    use crate::keys::Action as A;
    let Ok((mut tf, mut proj)) = q.single_mut() else { return };
    // While a chat line or the proposal list is open the keys are theirs.
    let typing = view.keys_taken();
    let held = |k: KeyCode| !typing && keys.pressed(k);
    let pressed = |k: KeyCode| !typing && keys.just_pressed(k);
    let Projection::Orthographic(o) = &mut *proj else { return };
    // Last frame's shake comes off before anything else moves the camera.
    tf.translation -= cam.shake.extend(0.0);
    cam.shake = Vec2::ZERO;
    if scroll.delta.y != 0.0 {
        o.scale = (o.scale * (1.0 - scroll.delta.y * 0.1)).clamp(0.15, 2.5);
    }
    // In pillbox view the arrows jump to the neighbouring pillbox in
    // that direction (SPEC UI-11); otherwise they scroll.
    let in_pillbox_view = cam.pillbox.is_some();
    let mut pan = Vec2::ZERO;
    if held(bind.key(A::ScrollUp)) || (!in_pillbox_view && held(KeyCode::ArrowUp)) {
        pan.y += 1.0;
    }
    if held(bind.key(A::ScrollDown)) || (!in_pillbox_view && held(KeyCode::ArrowDown)) {
        pan.y -= 1.0;
    }
    if held(bind.key(A::ScrollLeft)) || (!in_pillbox_view && held(KeyCode::ArrowLeft)) {
        pan.x -= 1.0;
    }
    if held(bind.key(A::ScrollRight)) || (!in_pillbox_view && held(KeyCode::ArrowRight)) {
        pan.x += 1.0;
    }
    // A click on a pillbox lamp jumps the view to that pillbox (UI-10).
    for (i, l) in lamps.iter() {
        if *i == Interaction::Pressed && l.0 == hud::Group::Pillbox {
            if let Some(p) = view.obs.as_ref().and_then(|o| o.pillboxes.get(l.1)) {
                cam.follow = false;
                cam.pillbox = None;
                let target = to_screen(p.pos) - frame_offset() * Vec2::new(1.0, -1.0) * o.scale;
                tf.translation = target.extend(tf.translation.z);
            }
        }
    }
    // Tank view: F (SPEC UI-13's tank view key; Enter is chat's). F on
    // an already following camera stops it, as before.
    if pressed(bind.key(A::TankView)) {
        cam.follow = cam.pillbox.is_some() || !cam.follow;
        cam.pillbox = None;
    }
    // Pillbox view: the apostrophe steps through mine and my allies';
    // in it, an arrow jumps to the nearest of them in that direction.
    let arrow = [(KeyCode::ArrowUp, V2::new(0.0, -1.0)), (KeyCode::ArrowDown, V2::new(0.0, 1.0)), (KeyCode::ArrowLeft, V2::new(-1.0, 0.0)), (KeyCode::ArrowRight, V2::new(1.0, 0.0))]
        .into_iter()
        .find(|(k, _)| in_pillbox_view && pressed(*k))
        .map(|(_, d)| d);
    if pressed(bind.key(A::PillboxView)) || arrow.is_some() {
        if let Some(obs) = view.obs.as_ref() {
            let mine: Vec<&Pillbox> = obs.pillboxes.iter().filter(|p| p.owner.map(|o| o == obs.player || obs.allied(o)).unwrap_or(false)).collect();
            if !mine.is_empty() {
                let next = match (arrow, cam.pillbox) {
                    (Some(d), Some(cur)) if cur < mine.len() => {
                        let here = mine[cur].pos;
                        // The nearest pillbox whose offset lies mostly along d.
                        mine.iter()
                            .enumerate()
                            .filter(|(i, p)| *i != cur && { let v = p.pos - here; v.x * d.x + v.y * d.y > (v.x * d.y - v.y * d.x).abs() })
                            .min_by(|a, b| a.1.pos.dist(here).partial_cmp(&b.1.pos.dist(here)).unwrap())
                            .map(|(i, _)| i)
                            .unwrap_or(cur)
                    }
                    _ => cam.pillbox.map(|i| (i + 1) % mine.len()).unwrap_or(0),
                };
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
    // A hit you can feel: the camera shakes for a moment, fading out.
    if let Some(age) = view.hit.map(|h| h.elapsed().as_secs_f32()).filter(|a| *a < hud::HIT_FLASH) {
        let a = (1.0 - age / hud::HIT_FLASH) * 4.0 * o.scale;
        let t = time.elapsed_secs();
        cam.shake = Vec2::new((t * 90.0).sin(), (t * 70.0).cos()) * a;
        tf.translation += cam.shake.extend(0.0);
    }
}

fn orders(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Query<&Window, With<PrimaryWindow>>,
    cam: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut view: ResMut<View>,
    mut orders: ResMut<Orders>,
    mut hud: ResMut<HudState>,
    tiles: Query<(&Interaction, &OrderTile), Changed<Interaction>>,
    bind: Res<crate::keys::Keys>,
    mut pilot: ResMut<crate::goal::Pilot>,
) {
    use crate::keys::Action as A;
    // While a chat line or the proposal list is open the keys are
    // theirs; the mouse and the order tiles still work.
    let typing = view.keys_taken();
    let pressed = |k: KeyCode| !typing && keys.just_pressed(k);
    // A tile clicked is its key pressed.
    let clicked: Option<usize> = tiles.iter().find(|(i, _)| **i == Interaction::Pressed).map(|(_, t)| t.0);
    let hit = |key: KeyCode, row: usize| pressed(key) || clicked == Some(row);
    if pressed(bind.key(A::Events)) {
        hud.show_log = !hud.show_log;
    }
    // Esc drops an armed mode; with none armed it quits.
    if pressed(KeyCode::Escape) {
        if hud.mode.is_some() {
            hud.mode = None;
        } else {
            info!("quit");
            std::process::exit(0);
        }
    }
    let before = orders.0.len();
    let Some(obs) = view.obs.as_ref() else { return };
    let Some(me) = obs.me.as_ref() else { return };
    let Ok(window) = window.single() else { return };
    let Ok((camera, cam_tf)) = cam.single() else { return };
    let at = cursor_tile(window, camera, cam_tf);
    let map = view.map.as_ref();

    if mouse.just_pressed(MouseButton::Left) {
        if let Some(p) = at {
            if let Some(mode) = hud.mode.take() {
                let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
                orders.0.push(Command::Deploy { task: builder_task(obs, mode, p, shift || hud.concrete, hud.works) });
            } else {
                // The world routes a Move around water itself and the tank
                // follows the waypoints; the client only names the destination.
                // Driving by hand takes the wheel back from a goal.
                pilot.brain.set_goal(treadcore_brain::Goal::Manual);
                orders.0.push(Command::Move { to: p });
            }
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
    if hit(bind.key(A::Interceptor), 0) {
        orders.0.push(Command::Launch { kind: DroneKind::Interceptor, task: DroneTask::Return });
    }
    if hit(bind.key(A::Recon), 1) {
        let center = at.unwrap_or(me.pos);
        orders.0.push(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center, radius: 4.0 } });
    }
    if hit(bind.key(A::Strike), 2) {
        let p = at.unwrap_or(me.pos);
        if let Some(t) = obs.hostile_tanks().min_by(|a, b| a.pos.dist(p).partial_cmp(&b.pos.dist(p)).unwrap()) {
            orders.0.push(Command::Launch { kind: DroneKind::Strike, task: DroneTask::Strike { target: t.id } });
        }
    }
    if hit(bind.key(A::Recall), 3) {
        orders.0.push(Command::Recall);
    }
    // The longshot (step 10c): X sends one at the cursor, anywhere on
    // the map; it will not come back.
    if pressed(bind.key(A::Longshot)) {
        if let Some(p) = at {
            orders.0.push(Command::Launch { kind: DroneKind::Longshot, task: DroneTask::Attack { at: p } });
        }
    }
    // B steps the works mode's kind.
    if pressed(bind.key(A::WorksKind)) {
        let all = StructureKind::ALL;
        let i = all.iter().position(|k| *k == hud.works).unwrap_or(0);
        hud.works = all[(i + 1) % all.len()];
    }
    // The builder modes: the key or the tile arms one (or disarms it,
    // pressed again); the click on the map gives it, above.
    let mode_keys = [A::Farm, A::Road, A::Wall, A::Pillbox, A::Mine, A::Works].map(|a| bind.key(a));
    for (n, mode) in Mode::ALL.iter().enumerate() {
        if hit(mode_keys[n], MODE_ROW + n) {
            hud.mode = if hud.mode == Some(*mode) || !hud::can_order(Some(me), mode.row(), hud.works) { None } else { Some(*mode) };
            // Shift-6 arms concrete; Shift on the click does too.
            hud.concrete = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
        }
    }
    if hit(bind.key(A::Return), RETURN_ROW) {
        orders.0.push(Command::Deploy { task: PillboyTask::Return });
    }
    if pressed(bind.key(A::BubbleIn)) {
        orders.0.push(Command::SetBubble { radius: me.bubble - 1.0 });
    }
    if pressed(bind.key(A::BubbleOut)) {
        orders.0.push(Command::SetBubble { radius: me.bubble + 1.0 });
    }
    if pressed(bind.key(A::Stop)) {
        pilot.brain.set_goal(treadcore_brain::Goal::Manual);
        orders.0.push(Command::Stop);
    }
    // G steps the goal (step 11a); the Commander drives from the next tick.
    if pressed(bind.key(A::Goal)) {
        let next = pilot.next(obs);
        pilot.brain.set_goal(next);
        if next == treadcore_brain::Goal::Manual {
            orders.0.push(Command::Stop);
        }
    }
    // A new order takes the last refusal off the wire.
    if orders.0.len() > before {
        view.refused = None;
    }
}

/// What a builder mode does on a square (SPEC section 6): farm the
/// forest there; lay a road, or a bridge on water; raise a wall; in
/// pillbox mode repair the pillbox there whoever owns it, capture it
/// if it is dead or nobody's, or build one on open ground; bury a mine.
/// With Shift a wall is poured in concrete (step 10a). In works mode
/// (step 10d) a click on a structure mends ours or takes one that is
/// nobody's or wrecked; anywhere else it builds the selected kind,
/// which the world refuses off a pad.
fn builder_task(obs: &Observation, mode: Mode, p: V2, shift: bool, works: StructureKind) -> PillboyTask {
    match mode {
        Mode::Farm => PillboyTask::Harvest { at: p },
        Mode::Road => PillboyTask::Build { what: BuildKind::Road, at: p },
        Mode::Wall if shift => PillboyTask::Build { what: BuildKind::Concrete, at: p },
        Mode::Wall => PillboyTask::Build { what: BuildKind::Wall, at: p },
        Mode::Works => {
            let near = obs.structures.iter().min_by(|a, b| a.pos.dist(p).partial_cmp(&b.pos.dist(p)).unwrap()).filter(|x| x.pos.dist(p) <= 1.0);
            match near {
                Some(x) if x.owner.map(|o| obs.allied(o)).unwrap_or(false) && x.alive() => PillboyTask::Repair { target: x.id },
                Some(x) => PillboyTask::Capture { pillbox: x.id },
                None => PillboyTask::Build { what: BuildKind::Structure(works), at: p },
            }
        }
        Mode::Mine => PillboyTask::Build { what: BuildKind::Mine, at: p },
        Mode::Pillbox => {
            let near = obs.pillboxes.iter().min_by(|a, b| a.pos.dist(p).partial_cmp(&b.pos.dist(p)).unwrap()).filter(|pb| pb.pos.dist(p) <= 1.5);
            match near {
                Some(pb) if pb.owner == Some(obs.player) || (pb.alive() && pb.owner.is_some()) => PillboyTask::Repair { target: pb.id },
                Some(pb) => PillboyTask::Capture { pillbox: pb.id },
                None => PillboyTask::Build { what: BuildKind::Pillbox, at: p },
            }
        }
    }
}
