//! The launch menu (Paul, 30 October: "a menu system to launch the game in various maps and
//! configurations"). `treadcore` with no flags opens it; each row is a choice, and Start runs this
//! same program with the flags those choices stand for, so everything the menu can do can also be
//! typed, and the game itself knows nothing of it. Arrow keys or the mouse; Enter starts.

use bevy::prelude::*;

/// A row: its name, and its choices as (what it says, the flags it stands for).
struct Row {
    name: &'static str,
    choices: Vec<(String, Vec<String>)>,
    at: usize,
}

fn flags(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn opt(label: &str, v: &[&str]) -> (String, Vec<String>) {
    (label.to_string(), flags(v))
}

fn rows() -> Vec<Row> {
    let mut maps = vec![opt("Naval", &["--map", "naval"]), opt("Generated island", &["--map", "island"]), opt("Industry", &["--map", "industry"])];
    for name in crate::MAP_NAMES.iter().skip(3) {
        maps.push((name.replace('-', " "), flags(&["--map", name])));
    }
    vec![
        Row { name: "Map", choices: maps, at: 0 },
        Row {
            name: "Sides",
            choices: vec![
                opt("Three teams, 16 tanks", &["--players", "16", "--teams", "3"]),
                opt("2v2v4v8", &["--players", "16", "--team-sizes", "2,2,4,8"]),
                opt("2v2", &["--players", "4", "--teams", "2"]),
                opt("4v4", &["--players", "8", "--teams", "2"]),
                opt("8v8", &["--players", "16", "--teams", "2"]),
                opt("4v4v4v4", &["--players", "16", "--teams", "4"]),
                opt("Everyone alone, 8 tanks", &["--players", "8", "--teams", "0"]),
                opt("Everyone alone, 16 tanks", &["--players", "16", "--teams", "0"]),
            ],
            at: 0,
        },
        Row {
            name: "Bots",
            choices: vec![opt("Full strength", &["--skill", "1"]), opt("Medium", &["--skill", "0.6"]), opt("Easy", &["--skill", "0.3"])],
            at: 0,
        },
        Row {
            name: "Lives",
            choices: vec![opt("Unlimited", &["--respawns", "unlimited"]), opt("3", &["--respawns", "3"]), opt("1", &["--respawns", "0"])],
            at: 0,
        },
        Row {
            name: "Length",
            choices: vec![opt("Map's own", &[]), opt("10 minutes", &["--seconds", "600"]), opt("20 minutes", &["--seconds", "1200"]), opt("30 minutes", &["--seconds", "1800"])],
            at: 0,
        },
        Row {
            name: "Speed",
            choices: vec![opt("1x", &["--speed", "1"]), opt("2x", &["--speed", "2"]), opt("4x", &["--speed", "4"])],
            at: 0,
        },
        Row {
            name: "Driver",
            choices: vec![opt("You", &[]), opt("Autopilot, to watch", &["--autopilot"])],
            at: 0,
        },
    ]
}

#[derive(Resource)]
struct Menu {
    rows: Vec<Row>,
    /// The row the keys are on; `rows.len()` is Start.
    on: usize,
}

impl Menu {
    fn args(&self) -> Vec<String> {
        let mut a = vec!["--solo".to_string()];
        for r in &self.rows {
            a.extend(r.choices[r.at].1.iter().cloned());
        }
        a
    }

    fn line(&self, i: usize) -> String {
        match self.rows.get(i) {
            Some(r) => format!("{:<8} <  {}  >", r.name, r.choices[r.at].0),
            None => "Start".to_string(),
        }
    }
}

fn start(menu: &Menu) -> ! {
    let exe = std::env::current_exe().expect("this program's path");
    match std::process::Command::new(exe).args(menu.args()).spawn() {
        Ok(_) => std::process::exit(0),
        Err(e) => {
            eprintln!("treadcore: could not start the game: {}", e);
            std::process::exit(1)
        }
    }
}

#[derive(Component)]
struct Line(usize);

fn setup(mut commands: Commands, menu: Res<Menu>) {
    commands.spawn(Camera2d);
    commands
        .spawn(Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, align_items: AlignItems::Center, justify_content: JustifyContent::Center, row_gap: px(10), ..default() })
        .with_children(|p| {
            p.spawn((Text::new("treadcore"), TextFont { font_size: 46.0, ..default() }, Node { margin: UiRect::bottom(px(24)), ..default() }));
            for i in 0..=menu.rows.len() {
                p.spawn((Button, Line(i), Node { width: px(560), padding: UiRect::axes(px(16), px(9)), ..default() }, BackgroundColor(Color::srgb(0.14, 0.16, 0.18))))
                    .with_children(|b| {
                        b.spawn((Text::new(menu.line(i)), TextFont { font_size: 22.0, ..default() }));
                    });
            }
            p.spawn((Text::new("Up and down choose a row, left and right change it, Enter starts."), TextFont { font_size: 15.0, ..default() }, TextColor(Color::srgb(0.6, 0.63, 0.66)), Node { margin: UiRect::top(px(18)), ..default() }));
        });
}

fn keys(input: Res<ButtonInput<KeyCode>>, mut menu: ResMut<Menu>, mut exit: MessageWriter<AppExit>) {
    let n = menu.rows.len();
    if input.just_pressed(KeyCode::ArrowDown) {
        menu.on = (menu.on + 1) % (n + 1);
    }
    if input.just_pressed(KeyCode::ArrowUp) {
        menu.on = (menu.on + n) % (n + 1);
    }
    let on = menu.on;
    if let Some(r) = menu.rows.get_mut(on) {
        let c = r.choices.len();
        if input.just_pressed(KeyCode::ArrowRight) {
            r.at = (r.at + 1) % c;
        }
        if input.just_pressed(KeyCode::ArrowLeft) {
            r.at = (r.at + c - 1) % c;
        }
    }
    if input.just_pressed(KeyCode::Enter) {
        start(&menu);
    }
    if input.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

/// A click on a row turns it to its next choice, and a click on Start starts.
fn clicks(mut q: Query<(&Interaction, &Line), Changed<Interaction>>, mut menu: ResMut<Menu>) {
    for (i, line) in q.iter_mut() {
        if *i == Interaction::Pressed {
            menu.on = line.0;
            match menu.rows.get_mut(line.0) {
                Some(r) => r.at = (r.at + 1) % r.choices.len(),
                None => start(&menu),
            }
        }
    }
}

fn draw(menu: Res<Menu>, mut buttons: Query<(&Line, &Children, &mut BackgroundColor)>, mut texts: Query<&mut Text>) {
    for (line, children, mut bg) in buttons.iter_mut() {
        bg.0 = if line.0 == menu.on { Color::srgb(0.2, 0.38, 0.27) } else { Color::srgb(0.14, 0.16, 0.18) };
        for c in children.iter() {
            if let Ok(mut t) = texts.get_mut(c) {
                t.0 = menu.line(line.0);
            }
        }
    }
}

/// `MENU_SHOT=FILE`: a picture of the menu, then out, so it can be checked without a person.
fn shot(mut commands: Commands, mut frames: Local<u32>, mut exit: MessageWriter<AppExit>) {
    let Ok(path) = std::env::var("MENU_SHOT") else { return };
    *frames += 1;
    if *frames == 120 {
        // As a PPM by hand: this build carries no image encoders.
        commands.spawn(bevy::render::view::screenshot::Screenshot::primary_window()).observe(move |shot: On<bevy::render::view::screenshot::ScreenshotCaptured>| {
            let img = &shot.event().image;
            let Some(data) = &img.data else { return };
            let bgra = format!("{:?}", img.texture_descriptor.format).starts_with("Bgra");
            let mut out = format!("P6\n{} {}\n255\n", img.width(), img.height()).into_bytes();
            for px in data.chunks_exact(4) {
                out.extend_from_slice(&if bgra { [px[2], px[1], px[0]] } else { [px[0], px[1], px[2]] });
            }
            let _ = std::fs::write(&path, out);
        });
    }
    if *frames == 400 {
        exit.write(AppExit::Success);
    }
}

pub fn run() {
    let window = Window { title: "treadcore".into(), name: Some("org.treadcore.treadcore".into()), resolution: bevy::window::WindowResolution::new(760, 620), ..default() };
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin { primary_window: Some(window), ..default() }))
        .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.07)))
        .insert_resource(Menu { rows: rows(), on: 0 })
        .add_systems(Startup, setup)
        .add_systems(Update, (keys, clicks, draw, shot))
        .run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_offers_2v2v4v8_and_turns_choices_into_flags() {
        let mut m = Menu { rows: rows(), on: 0 };
        let sides = m.rows.iter().position(|r| r.name == "Sides").unwrap();
        let k = m.rows[sides].choices.iter().position(|c| c.0 == "2v2v4v8").unwrap();
        m.rows[sides].at = k;
        let a = m.args().join(" ");
        assert!(a.contains("--team-sizes 2,2,4,8") && a.contains("--players 16") && a.starts_with("--solo"), "{}", a);
    }

    #[test]
    fn every_map_on_the_menu_reads() {
        for (name, text) in crate::MAPS {
            assert!(treadcore_sim::Map::from_text(text).is_ok(), "{}", name);
        }
    }
}
