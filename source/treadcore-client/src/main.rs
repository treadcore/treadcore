//! treadcore -- the window.
//!
//! An overhead view of the island, the player's tank, the drones it has
//! up, and the telemetry the drones send back, drawn as an overlay on the
//! world: a ring where the interceptors hold station, a line from each
//! drone to its tank coloured by link quality, a disc where the recon
//! drone can see, an arc for every shell in the air.
//!
//! Three modes, by flag:
//!   --solo   this process hosts; three bots fill the other seats.
//!   --lan    this process joins the lobby on the LAN, helps elect a host,
//!            then either hosts (bots in empty seats) or plays as a client.
//!   neither  same as --solo.
//!
//! The world is never simulated here except when this process is the
//! host; a client draws the last observation it received and sends
//! orders. See treadcore-net for the protocol.

mod hud;
mod input;
mod render;
mod session;

use bevy::prelude::*;
use bevy::window::PresentMode;
use treadcore_sim::TICK_HZ;

/// Pixels per tile at zoom 1.
pub const TILE: f32 = 8.0;

#[derive(Resource, Clone, Debug)]
pub struct Args {
    pub lan: bool,
    pub name: String,
    pub game: String,
    pub want: usize,
    pub peers: Vec<String>,
    pub seed: u64,
    pub swarm: u32,
}

fn parse_args() -> Args {
    let mut a = Args {
        lan: false,
        name: std::env::var("USER").unwrap_or_else(|_| "player".into()),
        game: "treadcore".into(),
        want: 4,
        peers: vec![],
        seed: 0x7EAD_C0DE,
        swarm: 45 * TICK_HZ,
    };
    let mut it = std::env::args().skip(1);
    while let Some(f) = it.next() {
        let mut val = || it.next().unwrap_or_default();
        match f.as_str() {
            "--lan" => a.lan = true,
            "--solo" => a.lan = false,
            "--name" => a.name = val(),
            "--game" => a.game = val(),
            "--want" => a.want = val().parse().unwrap_or(4),
            "--peer" => a.peers.push(val()),
            "--seed" => {
                let v = val();
                a.seed = v.strip_prefix("0x").and_then(|h| u64::from_str_radix(h, 16).ok()).or_else(|| v.parse().ok()).unwrap_or(a.seed);
            }
            "--swarm" => a.swarm = val().parse::<u32>().unwrap_or(45) * TICK_HZ,
            "-h" | "--help" => {
                println!("treadcore [--solo | --lan] [--name N] [--game G] [--want N] [--peer HOST:PORT] [--seed S] [--swarm SECONDS]");
                std::process::exit(0);
            }
            _ => {}
        }
    }
    a
}

fn main() {
    let args = parse_args();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "treadcore".into(),
                        present_mode: PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: std::env::var("RUST_LOG").unwrap_or_else(|_| "info,wgpu=error,naga=warn,bevy_render=warn,bevy_app=warn".into()),
                    ..default()
                }),
        )
        .insert_resource(ClearColor(Color::srgb(0.05, 0.06, 0.07)))
        .insert_resource(Time::<Fixed>::from_hz(TICK_HZ as f64))
        .insert_resource(args)
        .add_plugins((session::SessionPlugin, render::RenderPlugin, hud::HudPlugin, input::InputPlugin))
        .run();
}

/// Tile coordinates (y down, as the sim has them) to screen (y up).
pub fn to_screen(p: treadcore_sim::V2) -> Vec2 {
    Vec2::new(p.x * TILE, (treadcore_sim::MAP_H as f32 - p.y) * TILE)
}

pub fn to_tile(p: Vec2) -> treadcore_sim::V2 {
    treadcore_sim::V2::new(p.x / TILE, treadcore_sim::MAP_H as f32 - p.y / TILE)
}

/// Seat colours. Four players and the swarm.
pub fn player_color(p: treadcore_sim::PlayerId) -> Color {
    match p {
        0 => Color::srgb(0.35, 0.75, 1.0),
        1 => Color::srgb(1.0, 0.80, 0.30),
        2 => Color::srgb(0.55, 1.0, 0.55),
        3 => Color::srgb(0.95, 0.55, 1.0),
        _ => Color::srgb(1.0, 0.25, 0.20),
    }
}
