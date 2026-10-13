//! The HUD, in the prior art's frame (SPEC section 13; INTERFACE.md):
//! the map in the middle and never covered; an orders column on the
//! left where Bolo had its builder modes; the roster and one lamp per
//! pillbox and per depot on the right where Bolo had its info boxes;
//! the tank's stock as lamps, the air table and the newswire along the
//! bottom; who, when and the network along the top. The events
//! scrollback is a panel over the map behind Tab; the lobby and the
//! match's end are screens over the map.
//!
//! Every panel is one Text rebuilt each frame from the View; at this
//! size that is cheaper than keeping it in sync. Lamps are ASCII, so
//! the engine's subset font always has them: `#` lit, `.` dark, and for
//! territory `#` mine, `+` an ally's, `o` neutral, `X` hostile, `.` dead.

use crate::session::View;
use bevy::prelude::*;
use treadcore_sim::*;

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudState>().add_systems(Startup, setup).add_systems(Update, update);
    }
}

/// The frame's fixed parts, in pixels. The map takes the rest, and the
/// camera follows the tank into the middle of what is left (input.rs).
pub const LEFT: f32 = 132.0;
pub const RIGHT: f32 = 268.0;
pub const TOP: f32 = 44.0;
pub const BOTTOM: f32 = 84.0;
const FONT: f32 = 13.0;

#[derive(Resource)]
pub struct HudState {
    /// The events scrollback over the map (Tab).
    pub show_log: bool,
    /// The right button is down: the gun's envelope lights.
    pub aiming: bool,
}

impl Default for HudState {
    fn default() -> HudState {
        HudState { show_log: false, aiming: false }
    }
}

/// Is a window point inside the frame (a panel), not the map?
pub fn in_frame(p: Vec2, size: Vec2) -> bool {
    p.x < LEFT || p.x > size.x - RIGHT || p.y < TOP || p.y > size.y - BOTTOM
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Top,
    Orders,
    Right,
    Bottom,
    Log,
    Screen,
}

fn text(p: &mut ChildSpawnerCommands, which: Panel, color: Color) {
    p.spawn((Text::new(""), TextFont { font_size: FONT, ..default() }, TextColor(color), which));
}

fn setup(mut commands: Commands) {
    let ink = Color::srgb(0.9, 0.93, 0.9);
    let ground = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.62));
    let pad = UiRect::axes(Val::Px(8.0), Val::Px(5.0));
    // The frame: a column of top strip, middle row, bottom strip.
    commands
        .spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, ..default() })
        .with_children(|root| {
            root.spawn((Node { width: Val::Percent(100.0), height: Val::Px(TOP), padding: pad, flex_shrink: 0.0, ..default() }, ground)).with_children(|p| text(p, Panel::Top, ink));
            root.spawn(Node { width: Val::Percent(100.0), flex_grow: 1.0, flex_direction: FlexDirection::Row, ..default() }).with_children(|mid| {
                mid.spawn((Node { width: Val::Px(LEFT), height: Val::Percent(100.0), padding: pad, flex_shrink: 0.0, ..default() }, ground)).with_children(|p| text(p, Panel::Orders, ink));
                // The map: nothing drawn here; the scrollback and the
                // screens sit over it when shown.
                mid.spawn(Node { flex_grow: 1.0, height: Val::Percent(100.0), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }).with_children(|map| {
                    map.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), bottom: Val::Px(0.0), padding: pad, ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.75)), Visibility::Hidden, Panel::Log))
                        .with_children(|p| text(p, Panel::Log, ink));
                    map.spawn((Node { padding: UiRect::all(Val::Px(14.0)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)), Visibility::Hidden, Panel::Screen))
                        .with_children(|p| text(p, Panel::Screen, ink));
                });
                mid.spawn((Node { width: Val::Px(RIGHT), height: Val::Percent(100.0), padding: pad, flex_shrink: 0.0, ..default() }, ground)).with_children(|p| text(p, Panel::Right, ink));
            });
            root.spawn((Node { width: Val::Percent(100.0), height: Val::Px(BOTTOM), padding: pad, flex_shrink: 0.0, ..default() }, ground)).with_children(|p| text(p, Panel::Bottom, ink));
        });
}

fn lamps(n: usize, of: usize, per_row: usize) -> String {
    let mut s = String::new();
    for i in 0..of {
        if i > 0 && i % per_row == 0 {
            s.push(' ');
        }
        s.push(if i < n { '#' } else { '.' });
    }
    s
}

fn bar(v: f32, max: f32, n: usize) -> String {
    let f = (v / max).clamp(0.0, 1.0);
    let k = (f * n as f32).round() as usize;
    format!("{}{}", "#".repeat(k), ".".repeat(n - k))
}

fn update(view: Res<View>, state: Res<HudState>, mut texts: Query<(&Panel, &mut Text)>, mut nodes: Query<(&Panel, &mut Visibility), Without<Text>>) {
    for (p, mut text) in texts.iter_mut() {
        let s = match p {
            Panel::Top => top(&view),
            Panel::Orders => orders(&view),
            Panel::Right => right(&view),
            Panel::Bottom => bottom(&view),
            Panel::Log => log(&view),
            Panel::Screen => screen(&view).unwrap_or_default(),
        };
        if text.0 != s {
            text.0 = s;
        }
    }
    let show_screen = screen(&view).is_some();
    for (p, mut vis) in nodes.iter_mut() {
        let want = match p {
            Panel::Log => state.show_log && !show_screen,
            Panel::Screen => show_screen,
            _ => true,
        };
        let v = if want { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != v {
            *vis = v;
        }
    }
}

fn top(view: &View) -> String {
    let n = &view.net;
    let mut first = format!("TREADCORE  {}", view.status);
    if let Some(obs) = view.obs.as_ref() {
        let me = obs.players.get(obs.player as usize);
        first = format!(
            "TREADCORE  {}  seat {}  team {}   tick {:>6}  {:>6.1}s   {}",
            me.map(|p| p.name.as_str()).unwrap_or("?"),
            obs.player,
            me.map(|p| p.team).unwrap_or(0),
            obs.tick,
            obs.seconds(),
            view.status
        );
    }
    let mut second = format!("NET {}  host {}", n.mode, if n.host.is_empty() { "-" } else { &n.host });
    if n.mode == "lobby" {
        second.push_str(&format!("  phase {}  peers {}", n.phase, n.peers));
    } else {
        second.push_str(&format!("  rtt {}  unacked {}  tx {} rx {} drop {}", n.rtt_ms.map(|r| format!("{}ms", r)).unwrap_or_else(|| "-".into()), n.unacked, n.sent, n.received, n.dropped));
        if let Some((ahead, in_flight, checks, mismatches, corrected)) = n.predict {
            let recent = corrected.zip(view.obs.as_ref().map(|o| o.tick)).map(|(c, t)| t.saturating_sub(c) < 2 * TICK_HZ as Tick).unwrap_or(false);
            second.push_str(&format!("  predict +{}  in flight {}  checked {}  mismatch {}{}", ahead, in_flight, checks, mismatches, if recent { " CORRECTED" } else { "" }));
        }
        if n.hash.0 > 0 {
            second.push_str(&format!("  hash @{} {:08x}", n.hash.0, (n.hash.1 >> 32) as u32));
        }
    }
    format!("{}\n{}", first, second)
}

/// The orders column: every key, what it does, and whether it can be
/// given now. A dash in the margin means not now, and says why in a
/// word. The column is the help; there is no other.
fn orders(view: &View) -> String {
    let Some(t) = view.obs.as_ref().and_then(|o| o.me.as_ref()).filter(|t| t.alive) else {
        return "ORDERS\n\nleft  drive\nright fire\n\nwheel zoom\nWASD  pan\nF     tank\n'     pillbox\nTab   events\nEsc   quit".into()
    };
    let sq = &t.pillboys;
    let aboard = sq.aboard && sq.strength > 0;
    let row = |ok: bool, key: &str, what: &str| format!("{}{:<2} {}\n", if ok { " " } else { "-" }, key, what);
    let mut s = String::from("ORDERS\n");
    s.push_str(&row(true, "L", "drive"));
    s.push_str(&row(t.reload == 0 && t.shells > 0, "R", if t.reload == 0 { "fire" } else { "reload" }));
    s.push('\n');
    s.push_str(&row(t.bay.interceptor > 0, "1", &format!("intcp x{}", t.bay.interceptor)));
    s.push_str(&row(t.bay.recon > 0, "2", &format!("recon x{}", t.bay.recon)));
    s.push_str(&row(t.bay.strike > 0, "3", &format!("strike x{}", t.bay.strike)));
    s.push_str(&row(true, "R", "recall"));
    s.push_str(&row(true, "[]", &format!("bubble {:.0}", t.bubble)));
    s.push('\n');
    s.push_str("SQUAD\n");
    let m = sq.material;
    s.push_str(&row(aboard, "4", "capture"));
    s.push_str(&row(aboard && m >= BuildKind::Pillbox.cost(), "5", &format!("pillbox {}m", BuildKind::Pillbox.cost())));
    s.push_str(&row(aboard && m >= BuildKind::Wall.cost(), "7", &format!("wall {}m", BuildKind::Wall.cost())));
    s.push_str(&row(aboard && m >= BuildKind::Mine.cost(), "8", &format!("mine {}m", BuildKind::Mine.cost())));
    s.push_str(&row(aboard && m >= BuildKind::Road.cost(), "9", &format!("road {}m", BuildKind::Road.cost())));
    s.push_str(&row(aboard, "0", "harvest"));
    s.push_str(&row(!sq.aboard, "6", "return"));
    s.push('\n');
    s.push_str("VIEW\n F  tank\n '  pillbox\n Tab events\n");
    s
}

/// The roster, then Bolo's info boxes as lamp strips: one per pillbox,
/// one per depot, by owner.
fn right(view: &View) -> String {
    let Some(obs) = view.obs.as_ref() else { return "ROSTER\n\nwaiting for the world...".into() };
    let mut s = String::from("ROSTER     t  seat score\n");
    let mut ps: Vec<&PlayerView> = obs.players.iter().collect();
    ps.sort_by_key(|p| (p.team, p.id));
    for p in ps {
        let mark = if p.id == obs.player { "*" } else if obs.allied(p.id) { "+" } else { " " };
        s.push_str(&format!("{}{:<9} {:<2} p{:<2}  {:>4} {}\n", mark, p.name.chars().take(9).collect::<String>(), p.team, p.id, p.score, if p.alive { "#" } else { "." }));
    }
    for r in &obs.proposals {
        s.push_str(&format!("proposal #{}: p{} {:?} p{}\n", r.id, r.from, r.terms, r.to));
    }
    let lamp = |owner: Option<PlayerId>, alive: bool| -> char {
        if !alive {
            '.'
        } else {
            match owner {
                Some(o) if o == obs.player => '#',
                Some(o) if obs.allied(o) => '+',
                Some(_) => 'X',
                None => 'o',
            }
        }
    };
    let held = obs.pillboxes.iter().filter(|p| p.owner == Some(obs.player)).count();
    s.push_str(&format!("\nPILLBOXES {} of {}\n", held, obs.pillboxes.len()));
    let mut pbs: Vec<&Pillbox> = obs.pillboxes.iter().collect();
    pbs.sort_by_key(|p| p.id);
    for (i, p) in pbs.iter().enumerate() {
        if i > 0 && i % 10 == 0 {
            s.push('\n');
        }
        s.push(lamp(p.owner, p.alive()));
        s.push(' ');
    }
    s.push_str("\n# me  + ally  o neutral\nX hostile  . dead\n");
    let mine_d = obs.depots.iter().filter(|d| d.owner == Some(obs.player)).count();
    s.push_str(&format!("\nDEPOTS {} of {}\n", mine_d, obs.depots.len()));
    for (i, d) in obs.depots.iter().enumerate() {
        if i > 0 && i % 10 == 0 {
            s.push('\n');
        }
        s.push(lamp(d.owner, true));
        s.push(' ');
    }
    s.push('\n');
    s
}

/// The tank's stock as lamps, the air table across, and the newswire.
fn bottom(view: &View) -> String {
    let Some(obs) = view.obs.as_ref() else { return String::new() };
    let mut s = String::new();
    if let Some(t) = obs.me.as_ref() {
        if t.alive {
            let under_fire = obs.tick.saturating_sub(t.last_hit_tick) < (3 * TICK_HZ) as Tick && t.last_hit_tick > 0;
            s.push_str(&format!(
                "SHELLS {} {:>2}/{} {}   ARMOR {} {:>3.0}   FUEL {} {:>3.0}{}\n",
                lamps(t.shells as usize, TANK_SHELLS as usize, 8),
                t.shells,
                TANK_SHELLS,
                if t.reload == 0 { "READY" } else { "....." },
                bar(t.armor, TANK_ARMOR, 10),
                t.armor,
                bar(t.fuel, TANK_FUEL, 10),
                t.fuel,
                if under_fire { "   !! UNDER FIRE !!" } else { "" }
            ));
            let sq = &t.pillboys;
            let doing = match (&sq.task, sq.aboard) {
                (_, true) if sq.strength == 0 => "none".to_string(),
                (_, true) => "aboard".to_string(),
                (Some(PillboyTask::Capture { .. }), false) => format!("CAPTURING {}%", sq.progress as u32 * 100 / PILLBOX_CAPTURE_TICKS as u32),
                (Some(PillboyTask::Repair { .. }), false) => format!("REPAIRING {}%", sq.progress as u32 * 100 / PILLBOX_REPAIR_TICKS as u32),
                (Some(PillboyTask::Build { what, .. }), false) => format!("BUILDING {:?} {}%", what, sq.progress as u32 * 100 / what.ticks() as u32),
                (Some(PillboyTask::Harvest { .. }), false) => format!("HARVESTING {}%", sq.progress as u32 * 100 / HARVEST_TICKS as u32),
                (Some(PillboyTask::Return), false) => "returning".to_string(),
                (None, false) => "out".to_string(),
            };
            s.push_str(&format!(
                "BAY recon {} intcp {} strike {}   SQUAD {} {}   MATERIAL {}   BUBBLE {:.0} ({:.0} m)   pos {:>5.1},{:<5.1} hdg {:>4.0}\n",
                lamps(t.bay.recon as usize, BAY_FULL.recon as usize, 9),
                lamps(t.bay.interceptor as usize, BAY_FULL.interceptor as usize, 9),
                lamps(t.bay.strike as usize, BAY_FULL.strike as usize, 9),
                lamps(sq.strength as usize, PILLBOY_MAX as usize, 9),
                doing,
                lamps(sq.material as usize, PILLBOY_MATERIAL_MAX as usize, 9),
                t.bubble,
                t.bubble * 25.0,
                t.pos.x,
                t.pos.y,
                t.heading.to_degrees()
            ));
            let mut mine: Vec<&Drone> = obs.my_drones().collect();
            mine.sort_by_key(|d| (d.kind as u8, d.id));
            let air: Vec<String> = mine
                .iter()
                .map(|d| {
                    let k = match d.kind {
                        DroneKind::Recon => "R",
                        DroneKind::Interceptor => "I",
                        DroneKind::Strike => "S",
                    };
                    let task = match &d.task {
                        DroneTask::Orbit { .. } => "".to_string(),
                        DroneTask::Intercept { target } => format!(" INTERCEPT #{}", target),
                        DroneTask::Strike { target } => format!(" STRIKE #{}", target),
                        DroneTask::Return => " home".into(),
                    };
                    format!("#{} {} {:.0}% {:.0}s{}", d.id, k, d.link * 100.0, d.battery, task)
                })
                .collect();
            let hostile = obs.hostile_drones().count();
            let inbound = obs.hostile_drones().filter(|d| d.pos.dist(t.pos) <= t.bubble + DETECT_MARGIN).count();
            s.push_str(&format!("AIR {}   hostile air {} ({} inside detection)  hostile tanks {}\n", if air.is_empty() { "none up".to_string() } else { air.join(" | ") }, hostile, inbound, obs.hostile_tanks().count()));
        } else {
            s.push_str("TANK DESTROYED\n\n\n");
        }
    }
    // The newswire: the two latest lines.
    let n = view.log.len();
    let wire: Vec<String> = view.log.iter().skip(n.saturating_sub(2)).map(|(t, l)| format!("{:>6} {}", t, l)).collect();
    s.push_str(&wire.join("   "));
    s
}

fn log(view: &View) -> String {
    let mut s = String::from("EVENTS (Tab)\n");
    let n = view.log.len();
    for (t, l) in view.log.iter().skip(n.saturating_sub(24)) {
        s.push_str(&format!("{:>6}  {}\n", t, l));
    }
    s
}

/// The screens: before the world (the lobby, or a solo start), and
/// after the match. None while playing.
fn screen(view: &View) -> Option<String> {
    let n = &view.net;
    match view.obs.as_ref() {
        None => {
            let mut s = String::from("TREADCORE\n\n");
            s.push_str(&format!("{}\n", view.status));
            if n.mode == "lobby" {
                s.push_str(&format!("\nlobby phase: {}   peers found: {}\n", n.phase, n.peers));
                for (name, rtt) in &n.lobby_peers {
                    s.push_str(&format!("  {:<12} {}\n", name, rtt.map(|r| format!("{} ms", r)).unwrap_or_else(|| "measuring".into())));
                }
                s.push_str("\nthe host is elected from the latency matrix:\nthe peer whose worst link is best\n");
            }
            s.push_str(FIRST_MINUTE);
            Some(s)
        }
        Some(obs) if obs.over => {
            let mut s = format!("{}\n\n", view.status);
            let mut ps: Vec<&PlayerView> = obs.players.iter().collect();
            ps.sort_by_key(|p| std::cmp::Reverse(p.score));
            s.push_str("  seat  name       team  score\n");
            for p in ps {
                s.push_str(&format!("{} p{:<3}  {:<9}  {:<4}  {:>5}  {}\n", if p.id == obs.player { "*" } else { " " }, p.id, p.name.chars().take(9).collect::<String>(), p.team, p.score, if p.alive { "alive" } else { "dead" }));
            }
            s.push_str("\nEsc to leave");
            Some(s)
        }
        _ => None,
    }
}

/// The first minute, on the lobby screen, because a tank that stands at
/// its start is dead inside it (the first playtest, finding 7).
const FIRST_MINUTE: &str = "
THE FIRST MINUTE
  1  launch an interceptor: it holds the bubble, the ring round you
  2  launch a recon toward the cursor: what it sees, your gun can hit
  left click drives; the world routes round water and hostile guns
  right click fires: hold to see the gun's reach, release to fire
  a tank that stands still is found by fifteen guns; move after you fire
  F follows your tank; ' jumps to your pillboxes; Tab shows the events
";
