//! The HUD, in the prior art's frame (SPEC section 13; INTERFACE.md),
//! and in its look: WinBolo's 2026 screen is a grey bezel of inset
//! black boxes holding lamps, one per thing in the game, each lit in
//! the colour of its state; a mode tile with a lamp beside it for every
//! order; bars with a pictogram for the tank's stock; kills and deaths;
//! and a newswire in a big face across the bottom. That is what this
//! file draws, with the engine's UI nodes: a lamp is a small node whose
//! background is its state, a bar is a node whose width is its value, a
//! pictogram is a node or two. No images; every element is a
//! rectangle, rounded or not, as the rule has always been.
//!
//! The frame: a top strip (who, when, the network); the orders column
//! on the left where Bolo had its builder modes; the map in the middle,
//! never covered; the roster, kills and deaths, and the pillbox and
//! depot lamps on the right where Bolo had its info boxes; the stock
//! lamps and bars, the air table and the newswire along the bottom.
//! The events scrollback is behind Tab; the lobby and the match's end
//! are screens over the map.

use crate::player_color;
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
pub const LEFT: f32 = 156.0;
pub const RIGHT: f32 = 268.0;
pub const TOP: f32 = 58.0;
pub const BOTTOM: f32 = 128.0;
const FONT: f32 = 13.0;
const WIRE_FONT: f32 = 22.0;
const LAMP: f32 = 13.0;

// The palette: Bolo's code, red hostile and green friendly, on a bezel.
const BEZEL: Color = Color::srgb(0.47, 0.48, 0.46);
/// A bevel: light where the light falls (top, left), dark where it
/// does not. Sunken boxes take it the other way up.
const BEVEL_LIGHT: Color = Color::srgb(0.78, 0.79, 0.76);
const BEVEL_DARK: Color = Color::srgb(0.2, 0.2, 0.19);
const INSET: Color = Color::srgb(0.03, 0.03, 0.03);
const INK: Color = Color::srgb(0.9, 0.93, 0.9);
const DIM: Color = Color::srgb(0.17, 0.17, 0.16);
const GREEN: Color = Color::srgb(0.2, 0.95, 0.3);
const RED: Color = Color::srgb(0.95, 0.2, 0.15);
const RED_DIM: Color = Color::srgb(0.42, 0.12, 0.1);
const NEUTRAL: Color = Color::srgb(0.55, 0.5, 0.42);
const SHELL: Color = Color::srgb(0.92, 0.18, 0.12);
const SHELL_DIM: Color = Color::srgb(0.26, 0.07, 0.06);
const TAN: Color = Color::srgb(0.72, 0.58, 0.38);
const SKY: Color = Color::srgb(0.45, 0.78, 1.0);
const AMBER: Color = Color::srgb(1.0, 0.75, 0.3);

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

/// Text panels, rebuilt each frame.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Top,
    Roster,
    Counters,
    Air,
    Wire,
    Log,
    Screen,
}

/// Lamp groups: which thing a lamp stands for, and which one.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    Tank,
    Pillbox,
    Depot,
    Shell,
    Recon,
    Interceptor,
    Strike,
    Squad,
    Material,
    /// The lamp beside an order tile.
    Order,
}

#[derive(Component, Clone, Copy)]
pub struct Lamp(pub Group, pub usize);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Bar {
    Armor,
    Fuel,
}

/// The orders, in the column's order: key and word; the pictogram is
/// by index.
const ORDERS: [(&str, &str); 14] = [
    ("L", "drive"),
    ("R", "fire"),
    ("1", "interceptor"),
    ("2", "recon"),
    ("3", "strike"),
    ("R", "recall"),
    ("[]", "bubble"),
    ("4", "capture"),
    ("5", "pillbox"),
    ("7", "wall"),
    ("8", "mine"),
    ("9", "road"),
    ("0", "harvest"),
    ("6", "return"),
];

fn text(p: &mut ChildSpawnerCommands, which: Panel, size: f32) {
    p.spawn((Text::new(""), TextFont { font_size: size, ..default() }, TextColor(INK), which));
}

fn label(p: &mut ChildSpawnerCommands, s: &str) {
    p.spawn((Text::new(s), TextFont { font_size: FONT, ..default() }, TextColor(INK)));
}

/// A sunken box on the bezel: black, bevelled dark above and light
/// below, a little rounding.
fn inset<'a, 'w>(p: &'a mut ChildSpawnerCommands<'w>, node: Node) -> EntityCommands<'a> {
    let mut n = node;
    n.border = UiRect::all(Val::Px(2.0));
    n.padding = UiRect::all(Val::Px(5.0));
    n.margin = UiRect::all(Val::Px(3.0));
    p.spawn((n, BackgroundColor(INSET), sunken(), BorderRadius::all(Val::Px(3.0))))
}

fn sunken() -> BorderColor {
    BorderColor { top: BEVEL_DARK, left: BEVEL_DARK, bottom: BEVEL_LIGHT, right: BEVEL_LIGHT }
}

fn raised() -> BorderColor {
    BorderColor { top: BEVEL_LIGHT, left: BEVEL_LIGHT, bottom: BEVEL_DARK, right: BEVEL_DARK }
}

fn lamp(p: &mut ChildSpawnerCommands, group: Group, i: usize, round: bool) {
    p.spawn((
        Node { width: Val::Px(LAMP), height: Val::Px(LAMP), margin: UiRect::all(Val::Px(1.5)), ..default() },
        BackgroundColor(DIM),
        BorderRadius::all(if round { Val::Px(LAMP) } else { Val::Px(2.0) }),
        Lamp(group, i),
    ));
}

fn lamp_row(p: &mut ChildSpawnerCommands, group: Group, n: usize, round: bool, per_row: usize) {
    p.spawn(Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, width: Val::Px((LAMP + 3.0) * per_row as f32 + 2.0), ..default() }).with_children(|r| {
        for i in 0..n {
            lamp(r, group, i, round);
        }
    });
}

fn bar(p: &mut ChildSpawnerCommands, which: Bar, width: f32) {
    p.spawn((Node { width: Val::Px(width), height: Val::Px(9.0), margin: UiRect::axes(Val::Px(0.0), Val::Px(2.0)), ..default() }, BackgroundColor(DIM), BorderRadius::all(Val::Px(2.0)))).with_children(|b| {
        b.spawn((Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }, BackgroundColor(GREEN), BorderRadius::all(Val::Px(2.0)), which));
    });
}

/// A pictogram for an order: a shape in a 28 px tile.
fn pictogram(p: &mut ChildSpawnerCommands, i: usize) {
    let shape = |t: &mut ChildSpawnerCommands, w: f32, h: f32, c: Color, ring: bool| {
        let mut n = Node { width: Val::Px(w), height: Val::Px(h), ..default() };
        if ring {
            n.border = UiRect::all(Val::Px(2.0));
            t.spawn((n, BorderColor::all(c), BorderRadius::MAX));
        } else {
            let r = if (w - h).abs() < 0.5 && w < 12.0 { w } else { 1.5 };
            t.spawn((n, BackgroundColor(c), BorderRadius::all(Val::Px(r))));
        }
    };
    match i {
        0 => shape(p, 16.0, 9.0, SKY, false),                               // drive: the hull
        1 => shape(p, 9.0, 9.0, AMBER, false),                              // fire: a round
        2 => shape(p, 16.0, 16.0, GREEN, true),                             // interceptor: the ring
        3 => shape(p, 11.0, 11.0, SKY, false),                              // recon: the disc it sees
        4 => shape(p, 16.0, 5.0, RED, false),                               // strike: the dart
        5 => shape(p, 10.0, 10.0, GREEN, true),                             // recall: a small ring
        6 => shape(p, 20.0, 20.0, SKY, true),                               // bubble: the big ring
        7 => shape(p, 13.0, 13.0, TAN, true),                               // capture: a pillbox, hollow
        8 => shape(p, 13.0, 13.0, TAN, false),                              // pillbox: built
        9 => shape(p, 18.0, 7.0, TAN, false),                               // wall
        10 => shape(p, 8.0, 8.0, RED, false),                               // mine
        11 => shape(p, 18.0, 5.0, Color::srgb(0.6, 0.57, 0.5), false),      // road
        12 => shape(p, 12.0, 12.0, Color::srgb(0.25, 0.65, 0.3), false),    // harvest: a tree tile
        _ => shape(p, 12.0, 8.0, SKY, false),                               // return: home to the hull
    }
}

fn setup(mut commands: Commands) {
    let pad = UiRect::all(Val::Px(4.0));
    commands
        .spawn(Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, ..default() })
        .with_children(|root| {
            // Top strip.
            root.spawn((Node { width: Val::Percent(100.0), height: Val::Px(TOP), padding: pad, flex_shrink: 0.0, ..default() }, BackgroundColor(BEZEL))).with_children(|strip| {
                inset(strip, Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() }).with_children(|p| text(p, Panel::Top, FONT));
            });
            root.spawn(Node { width: Val::Percent(100.0), flex_grow: 1.0, flex_direction: FlexDirection::Row, min_height: Val::Px(0.0), ..default() }).with_children(|mid| {
                // The orders column: a lamp, a pictogram tile, the key and the word.
                mid.spawn((Node { width: Val::Px(LEFT), height: Val::Percent(100.0), padding: pad, flex_shrink: 0.0, flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() }, BackgroundColor(BEZEL))).with_children(|col| {
                    for (i, (key, word)) in ORDERS.iter().enumerate() {
                        if i == 7 {
                            col.spawn(Node { height: Val::Px(6.0), ..default() });
                        }
                        col.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, height: Val::Px(31.0), flex_shrink: 0.0, ..default() }).with_children(|row| {
                            lamp(row, Group::Order, i, true);
                            row.spawn((Node { width: Val::Px(28.0), height: Val::Px(28.0), margin: UiRect::horizontal(Val::Px(3.0)), border: UiRect::all(Val::Px(2.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, BackgroundColor(INSET), raised(), BorderRadius::all(Val::Px(2.0))))
                                .with_children(|t| pictogram(t, i));
                            row.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|w| {
                                w.spawn((Text::new(*key), TextFont { font_size: FONT, ..default() }, TextColor(AMBER)));
                                w.spawn((Text::new(*word), TextFont { font_size: 11.0, ..default() }, TextColor(INK)));
                            });
                        });
                    }
                    col.spawn(Node { height: Val::Px(6.0), ..default() });
                    col.spawn((Text::new("F tank  ' pillbox\nTab events  Esc quit"), TextFont { font_size: 11.0, ..default() }, TextColor(INK)));
                });
                // The map, in a sunken frame of its own; nothing drawn in
                // it; the scrollback and the screens sit over it.
                mid.spawn((Node { flex_grow: 1.0, height: Val::Percent(100.0), border: UiRect::all(Val::Px(3.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, sunken())).with_children(|map| {
                    map.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), bottom: Val::Px(0.0), padding: UiRect::all(Val::Px(8.0)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)), Visibility::Hidden, Panel::Log))
                        .with_children(|p| text(p, Panel::Log, FONT));
                    map.spawn((Node { padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)), raised(), BorderRadius::all(Val::Px(4.0)), Visibility::Hidden, Panel::Screen))
                        .with_children(|p| text(p, Panel::Screen, FONT));
                });
                // The right column: the roster, kills and deaths, the lamps.
                mid.spawn((Node { width: Val::Px(RIGHT), height: Val::Percent(100.0), padding: pad, flex_shrink: 0.0, flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() }, BackgroundColor(BEZEL))).with_children(|col| {
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, ..default() }).with_children(|p| text(p, Panel::Roster, FONT));
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
                        label(p, "TANKS");
                        lamp_row(p, Group::Tank, MAX_PLAYERS, true, 16);
                    });
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, flex_direction: FlexDirection::Row, justify_content: JustifyContent::SpaceAround, ..default() }).with_children(|p| text(p, Panel::Counters, FONT));
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
                        label(p, "PILLBOXES");
                        lamp_row(p, Group::Pillbox, 24, false, 12);
                    });
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
                        label(p, "DEPOTS");
                        lamp_row(p, Group::Depot, 20, true, 10);
                    });
                    col.spawn((Text::new("mine in my colour  green ally\nred hostile  dim neutral  dark dead"), TextFont { font_size: 10.0, ..default() }, TextColor(INK), Node { margin: UiRect::all(Val::Px(4.0)), ..default() }));
                });
            });
            // The bottom strip: the stock as lamps and bars, the air, the newswire.
            root.spawn((Node { width: Val::Percent(100.0), height: Val::Px(BOTTOM), padding: pad, flex_shrink: 0.0, flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() }, BackgroundColor(BEZEL))).with_children(|strip| {
                strip.spawn(Node { flex_direction: FlexDirection::Row, flex_shrink: 0.0, height: Val::Px(72.0), overflow: Overflow::clip(), ..default() }).with_children(|row| {
                    inset(row, Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
                        label(p, "SHELLS");
                        lamp_row(p, Group::Shell, TANK_SHELLS as usize, false, 12);
                    });
                    inset(row, Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "RECON");
                            lamp_row(c, Group::Recon, BAY_FULL.recon as usize, true, 6);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "INTCP");
                            lamp_row(c, Group::Interceptor, BAY_FULL.interceptor as usize, true, 6);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "STRIKE");
                            lamp_row(c, Group::Strike, BAY_FULL.strike as usize, false, 6);
                        });
                    });
                    inset(row, Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "SQUAD");
                            lamp_row(c, Group::Squad, PILLBOY_MAX as usize, false, 4);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "MATERIAL");
                            lamp_row(c, Group::Material, PILLBOY_MATERIAL_MAX as usize, false, 8);
                        });
                    });
                    inset(row, Node { flex_direction: FlexDirection::Column, width: Val::Px(150.0), ..default() }).with_children(|p| {
                        label(p, "ARMOR");
                        bar(p, Bar::Armor, 130.0);
                        label(p, "FUEL");
                        bar(p, Bar::Fuel, 130.0);
                    });
                    inset(row, Node { flex_grow: 1.0, ..default() }).with_children(|p| text(p, Panel::Air, FONT));
                });
                inset(strip, Node { width: Val::Percent(100.0), flex_grow: 1.0, align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() }).with_children(|p| text(p, Panel::Wire, WIRE_FONT));
            });
        });
}

fn update(
    view: Res<View>,
    state: Res<HudState>,
    mut texts: Query<(&Panel, &mut Text)>,
    mut nodes: Query<(&Panel, &mut Visibility), Without<Text>>,
    mut lamps: Query<(&Lamp, &mut BackgroundColor, &mut Visibility), (Without<Panel>, Without<Bar>)>,
    mut bars: Query<(&Bar, &mut Node), Without<Lamp>>,
) {
    for (p, mut text) in texts.iter_mut() {
        let s = match p {
            Panel::Top => top(&view),
            Panel::Roster => roster(&view),
            Panel::Counters => format!("KILLS {:>3}     DEATHS {:>3}", view.kills, view.deaths),
            Panel::Air => air(&view),
            Panel::Wire => wire(&view),
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
    let obs = view.obs.as_ref();
    let me = obs.map(|o| o.player);
    let tank = obs.and_then(|o| o.me.as_ref()).filter(|t| t.alive);
    let mine = me.map(player_color).unwrap_or(GREEN);
    let territory = |owner: Option<PlayerId>, alive: bool| -> Color {
        if !alive {
            DIM
        } else {
            match (owner, obs) {
                (Some(o), Some(ob)) if o == ob.player => mine,
                (Some(o), Some(ob)) if ob.allied(o) => GREEN,
                (Some(_), _) => RED,
                (None, _) => NEUTRAL,
            }
        }
    };
    for (l, mut bg, mut vis) in lamps.iter_mut() {
        let (color, shown) = match l.0 {
            Group::Tank => match obs.and_then(|o| o.players.get(l.1)) {
                Some(p) => (territory(Some(p.id), p.alive), true),
                None => (DIM, obs.is_none()),
            },
            Group::Pillbox => match obs.and_then(|o| o.pillboxes.get(l.1)) {
                Some(p) => (territory(p.owner, p.alive()), true),
                None => (DIM, obs.is_none()),
            },
            Group::Depot => match obs.and_then(|o| o.depots.get(l.1)) {
                Some(d) => (territory(d.owner, true), true),
                None => (DIM, obs.is_none()),
            },
            Group::Shell => (if tank.map(|t| l.1 < t.shells as usize).unwrap_or(false) { SHELL } else { SHELL_DIM }, true),
            Group::Recon => (if tank.map(|t| l.1 < t.bay.recon as usize).unwrap_or(false) { SKY } else { DIM }, true),
            Group::Interceptor => (if tank.map(|t| l.1 < t.bay.interceptor as usize).unwrap_or(false) { GREEN } else { DIM }, true),
            Group::Strike => (if tank.map(|t| l.1 < t.bay.strike as usize).unwrap_or(false) { RED } else { DIM }, true),
            Group::Squad => (
                match tank {
                    Some(t) if l.1 < t.pillboys.strength as usize => {
                        if t.pillboys.aboard {
                            GREEN
                        } else {
                            AMBER
                        }
                    }
                    _ => DIM,
                },
                true,
            ),
            Group::Material => (if tank.map(|t| l.1 < t.pillboys.material as usize).unwrap_or(false) { Color::srgb(0.3, 0.75, 0.35) } else { DIM }, true),
            Group::Order => (if can_order(tank, l.1) { RED } else { RED_DIM }, true),
        };
        if bg.0 != color {
            bg.0 = color;
        }
        let v = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != v {
            *vis = v;
        }
    }
    for (b, mut node) in bars.iter_mut() {
        let f = match (b, tank) {
            (Bar::Armor, Some(t)) => t.armor / TANK_ARMOR,
            (Bar::Fuel, Some(t)) => t.fuel / TANK_FUEL,
            _ => 0.0,
        };
        node.width = Val::Percent((f.clamp(0.0, 1.0) * 100.0).round());
    }
}

/// Whether an order of the column can be given now: Bolo's red lamp
/// beside the mode, lit when it can.
fn can_order(tank: Option<&Tank>, i: usize) -> bool {
    let Some(t) = tank else { return false };
    let sq = &t.pillboys;
    let aboard = sq.aboard && sq.strength > 0;
    match i {
        0 => true,
        1 => t.reload == 0 && t.shells > 0,
        2 => t.bay.interceptor > 0,
        3 => t.bay.recon > 0,
        4 => t.bay.strike > 0,
        5 | 6 => true,
        7 => aboard,
        8 => aboard && sq.material >= BuildKind::Pillbox.cost(),
        9 => aboard && sq.material >= BuildKind::Wall.cost(),
        10 => aboard && sq.material >= BuildKind::Mine.cost(),
        11 => aboard && sq.material >= BuildKind::Road.cost(),
        12 => aboard,
        _ => !sq.aboard,
    }
}

fn top(view: &View) -> String {
    let n = &view.net;
    let mut first = format!("TREADCORE  {}", view.status);
    if let Some(obs) = view.obs.as_ref() {
        let me = obs.players.get(obs.player as usize);
        let t = obs.me.as_ref();
        first = format!(
            "TREADCORE  {}  seat {}  team {}   tick {:>6}  {:>6.1}s   {}{}",
            me.map(|p| p.name.as_str()).unwrap_or("?"),
            obs.player,
            me.map(|p| p.team).unwrap_or(0),
            obs.tick,
            obs.seconds(),
            view.status,
            t.map(|t| if obs.tick.saturating_sub(t.last_hit_tick) < (3 * TICK_HZ) as Tick && t.last_hit_tick > 0 { "   !! UNDER FIRE !!" } else { "" }).unwrap_or("")
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

fn roster(view: &View) -> String {
    let Some(obs) = view.obs.as_ref() else { return "ROSTER\nwaiting for the world...".into() };
    let mut s = String::from("ROSTER     t  seat  score\n");
    let mut ps: Vec<&PlayerView> = obs.players.iter().collect();
    ps.sort_by_key(|p| (p.team, p.id));
    for p in ps {
        let mark = if p.id == obs.player { "*" } else if obs.allied(p.id) { "+" } else { " " };
        s.push_str(&format!("{}{:<9} {:<2} p{:<3} {:>4} {}\n", mark, p.name.chars().take(9).collect::<String>(), p.team, p.id, p.score, if p.alive { "" } else { "x" }));
    }
    for r in &obs.proposals {
        s.push_str(&format!("proposal #{}: p{} {:?} p{}\n", r.id, r.from, r.terms, r.to));
    }
    s.trim_end().to_string()
}

fn air(view: &View) -> String {
    let Some(obs) = view.obs.as_ref() else { return String::new() };
    let Some(t) = obs.me.as_ref() else { return String::new() };
    if !t.alive {
        return "TANK DESTROYED".into();
    }
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
            format!("{} {:.0}% {:.0}s{}", k, d.link * 100.0, d.battery, task)
        })
        .collect();
    let hostile = obs.hostile_drones().count();
    let inbound = obs.hostile_drones().filter(|d| d.pos.dist(t.pos) <= t.bubble + DETECT_MARGIN).count();
    format!(
        "AIR {}\nhostile air {} ({} in detection)  tanks {}\nbubble {:.0}  gun {}  pos {:.0},{:.0}",
        if air.is_empty() { "none up".to_string() } else { air.join(" ") },
        hostile,
        inbound,
        obs.hostile_tanks().count(),
        t.bubble,
        if t.reload == 0 { "READY" } else { "reload" },
        t.pos.x,
        t.pos.y
    )
}

/// The newswire: the latest line, in the big face.
fn wire(view: &View) -> String {
    view.log.back().map(|(t, l)| format!("{:>6}  {}", t, l)).unwrap_or_default()
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
            s.push_str(&format!("\nkills {}  deaths {}\n\nEsc to leave", view.kills, view.deaths));
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
