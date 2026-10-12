//! The smallest Bevy program that touches every part of the engine the
//! game uses: a window, a 2D camera, a sprite, UI text, a gizmo. If this
//! opens, `treadcore` can open. If `make warm` built it, `make build` has
//! nothing left to compile but the game.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window { title: "treadcore-warm: engine ok (Esc quits)".into(), ..default() }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.15, 0.09)))
        .add_systems(Startup, setup)
        .add_systems(Update, (spin, quit))
        .run();
}

#[derive(Component)]
struct Spinner;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((Sprite { color: Color::srgb(0.85, 0.70, 0.30), custom_size: Some(Vec2::new(64.0, 40.0)), ..default() }, Transform::default(), Spinner));
    commands.spawn((
        Text::new("treadcore-warm\nengine built, window open, sprite + text + gizmo drawn\nEsc to quit"),
        TextFont { font_size: 16.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, left: Val::Px(12.0), top: Val::Px(12.0), ..default() },
    ));
}

fn spin(time: Res<Time>, mut q: Query<&mut Transform, With<Spinner>>, mut gizmos: Gizmos) {
    for mut t in q.iter_mut() {
        t.rotate_z(time.delta_secs());
    }
    gizmos.circle_2d(Vec2::ZERO, 120.0, Color::srgb(0.4, 1.0, 0.5));
}

fn quit(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }
}
