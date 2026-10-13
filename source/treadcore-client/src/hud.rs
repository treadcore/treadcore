//! The HUD: four text panels over the world. Telemetry top-left, the
//! scoreboard and network top-right, the event log bottom-left, the
//! controls bottom-right. Every panel is one Text rebuilt each frame;
//! at this size that is cheaper than keeping it in sync.

use crate::session::View;
use bevy::prelude::*;
use treadcore_sim::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>().add_systems(Startup, setup).add_systems(Update, update);
    }
}

#[derive(Resource)]
pub struct HudState {
    pub show_help: bool,
}

impl Default for HudState {
    fn default() -> HudState {
        HudState { show_help: true }
    }
}

#[derive(Component)]
pub struct Panel(pub u8);

fn panel(commands: &mut Commands, which: u8, left: Option<f32>, right: Option<f32>, top: Option<f32>, bottom: Option<f32>) {
    commands.spawn((
        Text::new(""),
        TextFont { font_size: 13.0, ..default() },
        TextColor(Color::srgb(0.9, 0.93, 0.9)),
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.45)),
        Node {
            position_type: PositionType::Absolute,
            left: left.map(Val::Px).unwrap_or(Val::Auto),
            right: right.map(Val::Px).unwrap_or(Val::Auto),
            top: top.map(Val::Px).unwrap_or(Val::Auto),
            bottom: bottom.map(Val::Px).unwrap_or(Val::Auto),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        Panel(which),
    ));
}

fn setup(mut commands: Commands) {
    panel(&mut commands, 0, Some(8.0), None, Some(8.0), None);
    panel(&mut commands, 1, None, Some(8.0), Some(8.0), None);
    panel(&mut commands, 2, Some(8.0), None, None, Some(8.0));
    panel(&mut commands, 3, None, Some(8.0), None, Some(8.0));
}

fn bar(v: f32, max: f32, n: usize) -> String {
    let f = (v / max).clamp(0.0, 1.0);
    let k = (f * n as f32).round() as usize;
    format!("{}{}", "#".repeat(k), ".".repeat(n - k))
}

fn update(view: Res<View>, state: Res<HudState>, mut q: Query<(&Panel, &mut Text)>) {
    for (p, mut text) in q.iter_mut() {
        let s = match p.0 {
            0 => telemetry(&view),
            1 => scoreboard(&view),
            2 => log(&view),
            3 => {
                if state.show_help {
                    HELP.to_string()
                } else {
                    "Tab: help".into()
                }
            }
            _ => String::new(),
        };
        if text.0 != s {
            text.0 = s;
        }
    }
}

const HELP: &str = "CONTROLS
left click     drive there
right click    fire at there (direct if close and clear)
1              launch interceptor (holds the bubble)
2              launch recon toward cursor
3              launch strike at nearest hostile to cursor
R              recall all drones
4              Pillboys: capture (or repair) the pillbox under cursor
5              Pillboys: build a pillbox at cursor
6              Pillboys: return
[ / ]          bubble radius - / +

space          stop
WASD / arrows  pan   wheel: zoom   F: follow tank
Tab            toggle this   Esc: quit";

fn telemetry(view: &View) -> String {
    let mut s = String::new();
    s.push_str(&format!("TREADCORE  {}\n", view.status));
    let Some(obs) = view.obs.as_ref() else {
        s.push_str("waiting for the world...");
        return s;
    };
    s.push_str(&format!("tick {:>6}  {:>6.1}s  seat {}\n", obs.tick, obs.seconds(), obs.player));
    let Some(t) = obs.me.as_ref() else { return s };
    if !t.alive {
        s.push_str("\nTANK DESTROYED\n");
        return s;
    }
    s.push_str(&format!("\npos ({:>5.1},{:>5.1})  hdg {:>4.0}  spd {:.1}\n", t.pos.x, t.pos.y, t.heading.to_degrees(), t.speed));
    s.push_str(&format!("armor  [{}] {:>3.0}\n", bar(t.armor, TANK_ARMOR, 16), t.armor));
    s.push_str(&format!("fuel   [{}] {:>3.0}\n", bar(t.fuel, TANK_FUEL, 16), t.fuel));
    s.push_str(&format!("shells {:>2}/{}   gun {}\n", t.shells, TANK_SHELLS, if t.reload == 0 { "READY" } else { &"reloading" }));
    s.push_str(&format!("bay    recon {}  intercept {}  strike {}\n", t.bay.recon, t.bay.interceptor, t.bay.strike));
    s.push_str(&format!("bubble {:>4.1} tiles ({:.0} m)\n", t.bubble, t.bubble * 25.0));
    let sq = &t.pillboys;
    let doing = match (&sq.task, sq.aboard) {
        (_, true) if sq.strength == 0 => "none: resupply at a depot".to_string(),
        (_, true) => "aboard".to_string(),
        (Some(PillboyTask::Capture { .. }), false) => format!("CAPTURING {}%", sq.progress as u32 * 100 / PILLBOX_CAPTURE_TICKS as u32),
        (Some(PillboyTask::Repair { .. }), false) => format!("REPAIRING {}%", sq.progress as u32 * 100 / PILLBOX_REPAIR_TICKS as u32),
        (Some(PillboyTask::Build { .. }), false) => format!("BUILDING {}%", sq.progress as u32 * 100 / PILLBOX_BUILD_TICKS as u32),
        (Some(PillboyTask::Return), false) => "returning".to_string(),
        (None, false) => "out".to_string(),
    };
    s.push_str(&format!("pillboys {}/{}  material {}  {}\n", sq.strength, PILLBOY_MAX, sq.material, doing));
    let mine_pb = obs.pillboxes.iter().filter(|p| p.owner == Some(obs.player)).count();
    s.push_str(&format!("pillboxes held {}  of {}\n", mine_pb, obs.pillboxes.len()));
    let under_fire = obs.tick.saturating_sub(t.last_hit_tick) < (3 * TICK_HZ) as Tick && t.last_hit_tick > 0;
    if under_fire {
        s.push_str("!! UNDER FIRE !!\n");
    }
    s.push_str("\nAIR  kind  link  batt  task\n");
    let mut mine: Vec<&Drone> = obs.my_drones().collect();
    mine.sort_by_key(|d| (d.kind as u8, d.id));
    for d in mine {
        let task = match &d.task {
            DroneTask::Orbit { radius, .. } => format!("station r{:.0}", radius),
            DroneTask::Intercept { target } => format!("INTERCEPT #{}", target),
            DroneTask::Strike { target } => format!("STRIKE #{}", target),
            DroneTask::Return => "returning".into(),
        };
        s.push_str(&format!("#{:<4} {:<5} {:>3.0}%  {:>4.0}s  {}\n", d.id, format!("{:?}", d.kind).chars().take(5).collect::<String>(), d.link * 100.0, d.battery, task));
    }
    let hostile = obs.hostile_drones().count();
    let inbound = obs.hostile_drones().filter(|d| d.pos.dist(t.pos) <= t.bubble + DETECT_MARGIN).count();
    s.push_str(&format!("\nhostile air seen {}   inside detection {}\n", hostile, inbound));
    let spotted = obs.hostile_tanks().count();
    s.push_str(&format!("hostile tanks seen {}", spotted));
    s
}

fn scoreboard(view: &View) -> String {
    let mut s = String::new();
    let n = &view.net;
    s.push_str(&format!("NET {}  host {}\n", n.mode, if n.host.is_empty() { "-" } else { &n.host }));
    if !n.phase.is_empty() && n.mode == "lobby" {
        s.push_str(&format!("phase {}  peers {}\n", n.phase, n.peers));
    }
    s.push_str(&format!("rtt {}  unacked {}  tx {} rx {} drop {}\n", n.rtt_ms.map(|r| format!("{}ms", r)).unwrap_or_else(|| "-".into()), n.unacked, n.sent, n.received, n.dropped));
    if n.hash.0 > 0 {
        s.push_str(&format!("world hash @{} {:08x}\n", n.hash.0, (n.hash.1 >> 32) as u32));
    }

    if let Some(obs) = view.obs.as_ref() {
        s.push_str("\nSEAT  NAME      SCORE  STATE\n");
        for p in &obs.players {
            s.push_str(&format!("{}{}    {:<8}  {:>4}  {}\n", if p.id == obs.player { "*" } else { " " }, p.id, p.name, p.score, if p.alive { "alive" } else { "dead" }));
        }
        let depots: Vec<String> = obs.depots.iter().map(|d| d.owner.map(|o| format!("p{}", o)).unwrap_or_else(|| "-".into())).collect();
        s.push_str(&format!("depots {}\n", depots.join(" ")));
        if obs.over {
            s.push_str("\nMATCH OVER\n");
        }
    }
    s
}

fn log(view: &View) -> String {
    let mut s = String::from("EVENTS\n");
    for (t, l) in view.log.iter() {
        s.push_str(&format!("{:>6}  {}\n", t, l));
    }
    s
}
