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
//! The world is simulated here only when this process is the host. A
//! client draws the last observation it received and sends orders; the
//! one thing it runs itself is its own tank, driven ahead of the
//! snapshots and corrected by each (treadcore-net's Predictor). See
//! treadcore-net for the protocol.

mod hud;
mod input;
mod render;
mod session;
mod voxel;

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
    /// Seats in a solo or hosted match, 1 to 16; bots fill what people do not.
    pub players: usize,
    /// The text of a map file (`--map FILE`), checked at start.
    pub map: Option<String>,
    /// Deal the seats into this many teams; 0 is everyone alone.
    pub teams: usize,
    /// An introducer ("host:port") for peers no broadcast reaches.
    pub introducer: Option<String>,
    /// Shout on the LAN while looking (`--no-beacon` turns it off).
    pub beacon: bool,
    /// A file holding this peer's id across restarts (`--identity`).
    pub identity: Option<String>,
    /// The match's time limit in game seconds, when hosting (`--seconds`).
    pub seconds: Option<u64>,
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
        players: treadcore_sim::MAX_PLAYERS,
        map: None,
        teams: 0,
        introducer: None,
        beacon: true,
        identity: None,
        seconds: None,
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
            "--players" => a.players = val().parse::<usize>().unwrap_or(treadcore_sim::MAX_PLAYERS).clamp(1, treadcore_sim::MAX_PLAYERS),
            "--teams" => a.teams = val().parse().unwrap_or(0),
            "--introducer" => a.introducer = Some(val()),
            "--no-beacon" => a.beacon = false,
            "--identity" => a.identity = Some(val()),
            "--seconds" => a.seconds = val().parse().ok(),
            "--map" => {

                let path = val();
                let text = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                    eprintln!("treadcore: map {}: {}", path, e);
                    std::process::exit(2)
                });
                if let Err(e) = treadcore_sim::Map::from_text(&text) {
                    eprintln!("treadcore: map {}: {}", path, e);
                    std::process::exit(2)
                }
                a.map = Some(text);
            }

            "-h" | "--help" => {
                println!("treadcore [--solo | --lan] [--name N] [--game G] [--want N] [--peer HOST:PORT] [--introducer HOST:PORT] [--no-beacon] [--identity FILE] [--seed S] [--seconds N] [--swarm SECONDS] [--players 1..16] [--teams N] [--map FILE]");


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

/// Tile coordinates (y down, as the sim has them) to screen (y up): the
/// map hangs below the x axis, so no map height is needed and a map of
/// any size draws the same way.
pub fn to_screen(p: treadcore_sim::V2) -> Vec2 {
    Vec2::new(p.x * TILE, -p.y * TILE)
}

pub fn to_tile(p: Vec2) -> treadcore_sim::V2 {
    treadcore_sim::V2::new(p.x / TILE, -p.y / TILE)
}


/// Seat colours. Sixteen players spaced round the hue circle, the first
/// four as they always were, and the swarm in red.
pub fn player_color(p: treadcore_sim::PlayerId) -> Color {
    match p {
        // Seat 0 was sky blue, which is the river's cyan a shade over;
        // a violet blue reads against water and grass both.
        0 => Color::srgb(0.55, 0.58, 1.0),
        1 => Color::srgb(1.0, 0.80, 0.30),
        2 => Color::srgb(0.55, 1.0, 0.55),
        3 => Color::srgb(0.95, 0.55, 1.0),
        p if (p as usize) < treadcore_sim::MAX_PLAYERS => {
            // Seats 4..16: golden-angle steps round the hue circle, so
            // neighbours in seat order are far apart in colour.
            let hue = ((p as f32 - 4.0) * 137.5 + 20.0) % 360.0;
            Color::hsl(hue, 0.7, 0.62)
        }
        _ => Color::srgb(1.0, 0.25, 0.20),
    }
}

