//! The HUD, in the prior art's frame (SPEC section 13; INTERFACE.md),
//! and in its look: WinBolo's 2026 screen is a grey bezel of inset
//! black boxes holding lamps, one per thing in the game, each lit in
//! the colour of its state; five builder mode buttons down the left,
//! each a picture of what it builds with a lamp beside it, red against
//! the one that is armed (SPEC BLD-01, UI-02); bars with a pictogram
//! for the tank's stock; kills and deaths; and a newswire in a big
//! face across the bottom. That is what this
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
const NEUTRAL: Color = Color::srgb(0.55, 0.5, 0.42);
const SHELL: Color = Color::srgb(0.92, 0.18, 0.12);
const SHELL_DIM: Color = Color::srgb(0.26, 0.07, 0.06);
const SKY: Color = Color::srgb(0.45, 0.78, 1.0);
const AMBER: Color = Color::srgb(1.0, 0.75, 0.3);
/// A key that cannot be given now.
const KEY_DIM: Color = Color::srgb(0.5, 0.45, 0.35);

/// The builder modes, in the prior art's order (SPEC section 6, UI-02):
/// farm, road or bridge, building, pillbox, mine. Arm one, click a
/// square, and the squad goes; the mode drops when it does, because
/// the squad takes one order at a time.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Farm,
    Road,
    Wall,
    Pillbox,
    Mine,
    /// Step 10d: build the selected structure on a pad, or capture or
    /// mend the structure under the click. `B` picks the kind.
    Works,
}

impl Mode {
    pub const ALL: [Mode; 6] = [Mode::Farm, Mode::Road, Mode::Wall, Mode::Pillbox, Mode::Mine, Mode::Works];

    /// The mode's row in the orders column.
    pub fn row(self) -> usize {
        MODE_ROW + Mode::ALL.iter().position(|m| *m == self).unwrap()
    }

    pub fn of_row(i: usize) -> Option<Mode> {
        Mode::ALL.get(i.wrapping_sub(MODE_ROW)).copied()
    }
}

/// The first builder row; the rows above are the drones', which are
/// not modes but orders given at once. The mouse (drive, fire) is not
/// an order and has no row; the help line at the foot names it.
pub const MODE_ROW: usize = 5;
/// The row that brings the squad home.
pub const RETURN_ROW: usize = 11;

#[derive(Resource)]
pub struct HudState {
    /// The events scrollback over the map (Tab).
    pub show_log: bool,
    /// The right button is down: the gun's envelope lights.
    pub aiming: bool,
    /// The armed builder mode: the next click on the map gives it.
    pub mode: Option<Mode>,
    /// What the works mode builds (`B` steps it).
    pub works: StructureKind,
    /// Wall mode armed with Shift held: the wall is poured in concrete.
    pub concrete: bool,
}

impl Default for HudState {
    fn default() -> HudState {
        HudState { show_log: false, aiming: false, mode: None, works: StructureKind::Power, concrete: false }
    }
}

/// The four lines of keys under the orders column, from the bindings;
/// each fits the column's width ("propose" did not, beside "K keys").
#[derive(Component)]
pub struct KeyHint;

/// The hint's text for these bindings.
pub fn key_hint(k: &crate::keys::Keys) -> String {
    use crate::keys::{label, Action as A};
    format!(
        "L drive  R fire  {} longshot\n{} tank  {} pillbox  {} alert\n{} kind  shift-{} concrete\nEnter talk  {} sides  {} keys",
        label(k.key(A::Longshot)),
        label(k.key(A::TankView)),
        label(k.key(A::PillboxView)),
        label(k.key(A::Alert)),
        label(k.key(A::WorksKind)),
        label(k.key(A::Wall)),
        label(k.key(A::Propose)),
        label(k.key(A::KeySetup))
    )
}

/// An order row's key as the column shows it.
fn order_key(k: &crate::keys::Keys, row: usize) -> String {
    use crate::keys::{label, Action as A};
    let a = [A::Interceptor, A::Recon, A::Strike, A::Recall];
    match row {
        0..=3 => label(k.key(a[row])),
        4 => format!("{}{}", label(k.key(A::BubbleIn)), label(k.key(A::BubbleOut))),
        _ => match Mode::of_row(row) {
            Some(m) => label(k.key([A::Farm, A::Road, A::Wall, A::Pillbox, A::Mine, A::Works][Mode::ALL.iter().position(|x| *x == m).unwrap()])),
            None => label(k.key(A::Return)),
        },
    }
}

/// The works row's word: the kind it will build.
#[derive(Component)]
pub struct WorksWord;

/// An order's tile is a button: a click is the key.
#[derive(Component, Clone, Copy)]
pub struct OrderTile(pub usize);

/// The map's frame: its border flashes red when the tank is hit.
#[derive(Component)]
pub struct MapFrame;

/// How long the frame flashes and the camera shakes after a hit.
pub const HIT_FLASH: f32 = 0.3;

/// An order's key letter, dimmed when the order cannot be given.
#[derive(Component, Clone, Copy)]
pub struct OrderKey(pub usize);

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
    Longshot,
    Squad,
    Material,
    /// The ledger (step 10d): my side's structures, mine first.
    Structure,
    /// The lamp beside an order tile.
    Order,
}

#[derive(Component, Clone, Copy)]
pub struct Lamp(pub Group, pub usize);

/// The TANKS row, folded away over eight seats: sixteen lamps are wider
/// than the column, and the roster's `x` says the same.
#[derive(Component)]
struct TankRow;

/// Over this many seats the right column is compacted (`TankRow`, a
/// smaller roster) so the depots and the ledger stay in it: at sixteen
/// the roster pushed both off the bottom.
const ROOMY_SEATS: usize = 8;
const ROSTER_FONT_TIGHT: f32 = 10.0;

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Bar {
    Armor,
    Fuel,
}

/// The orders, in the column's order: key and word; the picture is by
/// index. Rows 0 to 4 are the drones'; 5 to 9 the builder modes in the
/// prior art's order, 10 the works (step 10d); 11 brings the squad home.
const ORDERS: [(&str, &str); 12] = [
    ("1", "interceptor"),
    ("2", "recon"),
    ("3", "strike"),
    ("R", "recall"),
    ("[]", "bubble"),
    ("4", "farm"),
    ("5", "road"),
    ("6", "wall"),
    ("7", "pillbox"),
    ("8", "mine"),
    ("0", "works"),
    ("9", "return"),
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
    let mut e = p.spawn((
        Node { width: Val::Px(LAMP), height: Val::Px(LAMP), margin: UiRect::all(Val::Px(1.5)), ..default() },
        BackgroundColor(DIM),
        BorderRadius::all(if round { Val::Px(LAMP) } else { Val::Px(2.0) }),
        Lamp(group, i),
    ));
    // A pillbox lamp is a button: a click jumps the view to it (SPEC
    // UI-10; input.rs).
    if group == Group::Pillbox {
        e.insert((Button, Interaction::None));
    }
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

/// A pictogram for a drone order: a shape on the ground of its tile.
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
        0 => shape(p, 18.0, 18.0, GREEN, true),                             // interceptor: the ring
        1 => shape(p, 12.0, 12.0, SKY, false),                              // recon: the disc it sees
        2 => shape(p, 20.0, 6.0, RED, false),                               // strike: the dart
        3 => shape(p, 10.0, 10.0, GREEN, true),                             // recall: a small ring
        _ => shape(p, 26.0, 26.0, SKY, true),                               // bubble: the big ring
    }
}

/// A map square in a picture tile, in pixels.
const SQ: f32 = 11.0;

/// A builder mode's picture, the prior art's way: three by three map
/// squares of the ground with the thing the mode makes on them, drawn
/// in the game's own colours and blocks (render.rs, voxel.rs), so a
/// button looks like what it builds. No images: every square and every
/// block face is a node.
fn picture(p: &mut ChildSpawnerCommands, i: usize) {
    use crate::render::{FOREST, ROAD, WATER};
    use crate::voxel::{GUN, MINE, OLIVE, TAN, TAN_DARK};
    let sq = |t: &mut ChildSpawnerCommands, x: usize, y: usize, c: Color| {
        t.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(x as f32 * SQ), top: Val::Px(y as f32 * SQ), width: Val::Px(SQ), height: Val::Px(SQ), ..default() }, BackgroundColor(c)));
    };
    // A block standing on the ground: its top face and the side face
    // below it, as voxel.rs draws one.
    let block = |t: &mut ChildSpawnerCommands, x: f32, y: f32, w: f32, h: f32, side: f32, c: Color| {
        t.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(x), top: Val::Px(y + h), width: Val::Px(w), height: Val::Px(side), ..default() }, BackgroundColor(c.darker(0.14))));
        t.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(x), top: Val::Px(y), width: Val::Px(w), height: Val::Px(h), ..default() }, BackgroundColor(c)));
    };
    match Mode::of_row(i) {
        Some(Mode::Farm) => {
            // A clump of forest on the grass.
            for (x, y) in [(1, 0), (0, 1), (1, 1), (2, 1), (1, 2), (2, 2)] {
                sq(p, x, y, FOREST);
            }
        }
        Some(Mode::Road) => {
            // A road across the middle, over water at its end as a bridge.
            sq(p, 2, 0, WATER);
            sq(p, 2, 2, WATER);
            sq(p, 0, 1, ROAD);
            sq(p, 1, 1, ROAD);
            sq(p, 2, 1, TAN_DARK);
        }
        Some(Mode::Wall) => block(p, SQ * 0.5, SQ * 0.3, SQ * 2.0, SQ * 1.6, SQ * 0.6, TAN),
        Some(Mode::Pillbox) => {
            // The stepped pyramid with the gun on top, and the squad beside it.
            block(p, SQ * 0.6, SQ * 0.5, SQ * 1.8, SQ * 1.5, SQ * 0.4, TAN);
            block(p, SQ * 0.9, SQ * 0.7, SQ * 1.2, SQ * 0.9, SQ * 0.3, TAN.lighter(0.04));
            block(p, SQ * 1.2, SQ * 0.85, SQ * 0.6, SQ * 0.45, SQ * 0.25, GUN);
            for (x, y) in [(0.1, 2.3), (0.5, 2.3), (0.1, 2.7), (0.5, 2.7)] {
                p.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(SQ * x), top: Val::Px(SQ * y), width: Val::Px(3.0), height: Val::Px(3.0), ..default() }, BackgroundColor(GREEN)));
            }
        }
        Some(Mode::Mine) => block(p, SQ * 1.1, SQ * 1.2, SQ * 0.8, SQ * 0.6, SQ * 0.3, MINE),
        Some(Mode::Works) => {
            // A pad, and on it a hall and a stack with its lamp.
            for x in 0..3 {
                for y in 1..3 {
                    sq(p, x, y, crate::voxel::PAD);
                }
            }
            block(p, SQ * 0.3, SQ * 1.1, SQ * 1.6, SQ * 1.0, SQ * 0.4, crate::voxel::STEEL);
            block(p, SQ * 2.1, SQ * 0.2, SQ * 0.5, SQ * 1.9, SQ * 0.3, crate::voxel::CONCRETE);
            block(p, SQ * 2.1, SQ * 0.0, SQ * 0.5, SQ * 0.4, SQ * 0.1, crate::voxel::LIT);
        }
        None => {
            // Return: the hull on the grass, the turret in the seat's blue.
            block(p, SQ * 0.4, SQ * 1.0, SQ * 2.2, SQ * 0.9, SQ * 0.4, OLIVE);
            block(p, SQ * 1.0, SQ * 1.2, SQ * 0.9, SQ * 0.5, SQ * 0.25, SKY);
        }
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
                // The orders column, the prior art's: a picture of the
                // ground with the thing on it in a raised bevel, the key
                // and the word, and for the builder rows a lamp, red
                // against the armed mode. The drones' rows above wear the
                // same tile with their glyph on it. Every tile is a button.
                mid.spawn((Node { width: Val::Px(LEFT), height: Val::Percent(100.0), padding: pad, flex_shrink: 0.0, flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() }, BackgroundColor(BEZEL))).with_children(|col| {
                    for (i, (key, word)) in ORDERS.iter().enumerate() {
                        let builder = i >= MODE_ROW;
                        if i == MODE_ROW {
                            col.spawn(Node { height: Val::Px(6.0), ..default() });
                        }
                        // The tile's size is its content; the 3 px bevel sits outside it.
                        let (tile, row_h) = (SQ * 3.0, SQ * 3.0 + 4.0);
                        col.spawn(Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, height: Val::Px(row_h), flex_shrink: 0.0, ..default() }).with_children(|row| {
                            if builder {
                                lamp(row, Group::Order, i, true);
                            } else {
                                row.spawn(Node { width: Val::Px(LAMP + 3.0), ..default() });
                            }
                            let mut n = Node { width: Val::Px(tile), height: Val::Px(tile), margin: UiRect::horizontal(Val::Px(3.0)), border: UiRect::all(Val::Px(3.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() };
                            if builder {
                                n.justify_content = JustifyContent::FlexStart;
                                n.align_items = AlignItems::FlexStart;
                            }
                            row.spawn((n, BackgroundColor(crate::render::GRASS), raised(), BorderRadius::all(Val::Px(2.0)), Button, Interaction::None, OrderTile(i)))
                                .with_children(|t| if builder { picture(t, i) } else { pictogram(t, i) });
                            row.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|w| {
                                w.spawn((Text::new(*key), TextFont { font_size: FONT, ..default() }, TextColor(AMBER), OrderKey(i)));
                                if Mode::of_row(i) == Some(Mode::Works) {
                                    w.spawn((Text::new(*word), TextFont { font_size: 11.0, ..default() }, TextColor(INK), WorksWord));
                                } else {
                                    w.spawn((Text::new(*word), TextFont { font_size: 11.0, ..default() }, TextColor(INK)));
                                }
                            });
                        });
                    }
                    // Four lines of keys, a size down and with no gap above,
                    // so the last still shows in the shoot's 740 px window.
                    col.spawn((Text::new(""), TextFont { font_size: 9.0, ..default() }, TextColor(INK), KeyHint));
                });
                // The map, in a sunken frame of its own; nothing drawn in
                // it; the scrollback and the screens sit over it.
                mid.spawn((Node { flex_grow: 1.0, height: Val::Percent(100.0), border: UiRect::all(Val::Px(3.0)), justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..default() }, sunken(), MapFrame)).with_children(|map| {
                    map.spawn((Node { position_type: PositionType::Absolute, left: Val::Px(0.0), bottom: Val::Px(0.0), padding: UiRect::all(Val::Px(8.0)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)), Visibility::Hidden, Panel::Log))
                        .with_children(|p| text(p, Panel::Log, FONT));
                    map.spawn((Node { padding: UiRect::all(Val::Px(14.0)), border: UiRect::all(Val::Px(2.0)), ..default() }, BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)), raised(), BorderRadius::all(Val::Px(4.0)), Visibility::Hidden, Panel::Screen))
                        .with_children(|p| text(p, Panel::Screen, FONT));
                });
                // The right column: the roster, kills and deaths, the lamps.
                mid.spawn((Node { width: Val::Px(RIGHT), height: Val::Percent(100.0), padding: pad, flex_shrink: 0.0, flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() }, BackgroundColor(BEZEL))).with_children(|col| {
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, ..default() }).with_children(|p| text(p, Panel::Roster, FONT));
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, flex_direction: FlexDirection::Column, ..default() }).insert(TankRow).with_children(|p| {
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
                    // The ledger: a lamp for each structure my side
                    // holds; an empty box on a map with no economy.
                    inset(col, Node { width: Val::Percent(100.0), flex_shrink: 0.0, flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
                        label(p, "WORKS");
                        lamp_row(p, Group::Structure, LEDGER, false, 12);
                    });
                    col.spawn((Text::new("mine in my colour  green ally\nred hostile  dim neutral  dark dead"), TextFont { font_size: 10.0, ..default() }, TextColor(INK), Node { margin: UiRect::all(Val::Px(4.0)), ..default() }));
                });
            });
            // The bottom strip: the stock as lamps and bars, the air, the newswire.
            // The minimap (step 10d) at its right end, the strip's height.
            root.spawn((Node { width: Val::Percent(100.0), height: Val::Px(BOTTOM), padding: pad, flex_shrink: 0.0, flex_direction: FlexDirection::Row, overflow: Overflow::clip(), ..default() }, BackgroundColor(BEZEL))).with_children(|outer| {
            outer.spawn(Node { flex_grow: 1.0, min_width: Val::Px(0.0), height: Val::Percent(100.0), flex_direction: FlexDirection::Column, overflow: Overflow::clip(), ..default() }).with_children(|strip| {
                strip.spawn(Node { flex_direction: FlexDirection::Row, flex_shrink: 0.0, height: Val::Px(72.0), overflow: Overflow::clip(), ..default() }).with_children(|row| {
                    inset(row, Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|p| {
                        label(p, "SHELLS");
                        lamp_row(p, Group::Shell, TANK_SHELLS as usize, false, 12);
                    });
                    inset(row, Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "RECON");
                            lamp_row(c, Group::Recon, BAY_FULL.recon as usize, true, 2);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "INTCP");
                            lamp_row(c, Group::Interceptor, BAY_FULL.interceptor as usize, true, 6);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "STRIKE");
                            lamp_row(c, Group::Strike, BAY_FULL.strike as usize, false, 3);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "LONG");
                            lamp_row(c, Group::Longshot, BAY_FULL.longshot as usize, false, 2);
                        });
                    });
                    inset(row, Node { flex_direction: FlexDirection::Row, column_gap: Val::Px(6.0), ..default() }).with_children(|r| {
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "SQUAD");
                            lamp_row(c, Group::Squad, PILLBOY_MAX as usize, false, 4);
                        });
                        r.spawn(Node { flex_direction: FlexDirection::Column, ..default() }).with_children(|c| {
                            label(c, "MATERIAL");
                            lamp_row(c, Group::Material, PILLBOY_MATERIAL_MAX as usize, false, 4);
                        });
                    });
                    inset(row, Node { flex_direction: FlexDirection::Column, width: Val::Px(110.0), ..default() }).with_children(|p| {
                        label(p, "ARMOR");
                        bar(p, Bar::Armor, 92.0);
                        label(p, "FUEL");
                        bar(p, Bar::Fuel, 92.0);
                    });
                    inset(row, Node { flex_grow: 1.0, ..default() }).with_children(|p| text(p, Panel::Air, FONT));
                });
                inset(strip, Node { width: Val::Percent(100.0), flex_grow: 1.0, align_items: AlignItems::Center, overflow: Overflow::clip(), ..default() }).with_children(|p| text(p, Panel::Wire, WIRE_FONT));
            });
            inset(outer, Node { flex_shrink: 0.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, padding: UiRect::all(Val::Px(2.0)), ..default() }).with_children(|p| {
                use crate::commander::{MinimapNode, MINIMAP_H};
                p.spawn((ImageNode::default(), Node { width: Val::Px(MINIMAP_H), height: Val::Px(MINIMAP_H), ..default() }, MinimapNode, Button, Interaction::None, bevy::ui::RelativeCursorPosition::default()));
            });
            });
        });
}

fn update(
    view: Res<View>,
    state: Res<HudState>,
    mut texts: Query<(&Panel, &mut Text, &mut TextColor, &mut TextFont), Without<OrderKey>>,
    mut nodes: Query<(&Panel, &mut Visibility), Without<Text>>,
    mut frame: Query<&mut BorderColor, With<MapFrame>>,
    mut lamps: Query<(&Lamp, &mut BackgroundColor, &mut Visibility), (Without<Panel>, Without<Bar>)>,
    mut bars: Query<(&Bar, &mut Node), Without<Lamp>>,
    mut keys: Query<(&OrderKey, &mut TextColor, &mut Text), (Without<Panel>, Without<WorksWord>, Without<KeyHint>)>,
    mut works: Query<&mut Text, (With<WorksWord>, Without<Panel>, Without<KeyHint>)>,
    mut hint: Query<&mut Text, (With<KeyHint>, Without<Panel>, Without<WorksWord>, Without<OrderKey>)>,
    bind: Res<crate::keys::Keys>,
    mut tank_row: Query<&mut Node, (With<TankRow>, Without<Bar>)>,
) {
    let crowded = view.obs.as_ref().map(|o| o.players.len() > ROOMY_SEATS).unwrap_or(false);
    for mut n in tank_row.iter_mut() {
        let d = if crowded { Display::None } else { Display::Flex };
        if n.display != d {
            n.display = d;
        }
    }
    for mut t in works.iter_mut() {
        let w = format!("works: {}", crate::commander::name(state.works));
        if t.0 != w {
            t.0 = w;
        }
    }
    let under_fire = view.obs.as_ref().and_then(|o| o.me.as_ref().map(|t| (o.tick, t))).map(|(tick, t)| t.last_hit_tick > 0 && tick.saturating_sub(t.last_hit_tick) < (3 * TICK_HZ) as Tick).unwrap_or(false);
    // Red only while the wire shows the alert itself: a line being
    // typed or a refusal takes the wire over it and keeps its ink.
    let alarm = wire_alert(&view).is_some() && wire(&view).contains("  ALERT ");
    for (p, mut text, mut color, mut font) in texts.iter_mut() {
        if *p == Panel::Roster {
            let size = if crowded { ROSTER_FONT_TIGHT } else { FONT };
            if font.font_size != size {
                font.font_size = size;
            }
        }
        let red = match p {
            Panel::Top => Some(under_fire),
            Panel::Wire => Some(alarm),
            _ => None,
        };
        if let Some(red) = red {
            let c = if red { RED } else { INK };
            if color.0 != c {
                color.0 = c;
            }
        }
        let s = match p {
            Panel::Top => top(&view),
            Panel::Roster => roster(&view),
            Panel::Counters => format!("KILLS {:>3}     DEATHS {:>3}", view.kills, view.deaths),
            Panel::Air => air(&view),
            Panel::Wire => wire(&view),
            // The proposal list takes the scrollback's place over the map.
            // The proposal list and Key Setup take the scrollback's place.
            Panel::Log => match (view.menu.as_ref(), view.obs.as_ref(), view.key_setup.as_ref()) {
                (_, _, Some(s)) => crate::keys::setup_text(&bind, s, &view.key_note),
                (Some(m), Some(o), _) => crate::proposals::menu_text(o, m),
                _ => log(&view),
            },
            Panel::Screen => screen(&view).unwrap_or_default(),
        };
        if text.0 != s {
            text.0 = s;
        }
    }
    let flash = view.hit.map(|h| h.elapsed().as_secs_f32() < HIT_FLASH).unwrap_or(false);
    for mut b in frame.iter_mut() {
        let want = if flash { BorderColor::all(RED) } else { sunken() };
        if *b != want {
            *b = want;
        }
    }
    let show_screen = screen(&view).is_some();
    for (p, mut vis) in nodes.iter_mut() {
        let want = match p {
            Panel::Log => (state.show_log || view.menu.is_some() || view.key_setup.is_some()) && !show_screen,
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
            Group::Longshot => (if tank.map(|t| l.1 < t.bay.longshot as usize).unwrap_or(false) { AMBER } else { DIM }, true),
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
            Group::Structure => match obs.and_then(|o| ledger(o).into_iter().nth(l.1)) {
                Some(x) => (ledger_color(x, if Some(x.owner) == Some(me) { mine } else { GREEN }), true),
                None => (DIM, false),
            },
            // The prior art's lamp: red against the armed mode, dark
            // otherwise. Whether a mode can be given is the key's colour.
            Group::Order => (if state.mode.map(|m| m.row() == l.1).unwrap_or(false) { RED } else { DIM }, true),
        };
        if bg.0 != color {
            bg.0 = color;
        }
        let v = if shown { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != v {
            *vis = v;
        }
    }
    for (k, mut color, mut text) in keys.iter_mut() {
        let c = if can_order(tank, k.0, state.works) { AMBER } else { KEY_DIM };
        if color.0 != c {
            color.0 = c;
        }
        let s = order_key(&bind, k.0);
        if text.0 != s {
            text.0 = s;
        }
    }
    for mut t in hint.iter_mut() {
        let s = key_hint(&bind);
        if t.0 != s {
            t.0 = s;
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

/// Whether an order of the column can be given now: the key lights
/// amber when it can. A builder mode needs the squad aboard and the
/// material it costs (SPEC BLD-03, BLD-10).
pub fn can_order(tank: Option<&Tank>, i: usize, works: StructureKind) -> bool {
    let Some(t) = tank else { return false };
    let sq = &t.pillboys;
    let aboard = sq.aboard && sq.strength > 0;
    match (i, Mode::of_row(i)) {
        (0, _) => t.bay.interceptor > 0,
        (1, _) => t.bay.recon > 0,
        (2, _) => t.bay.strike > 0,
        (3 | 4, _) => true,
        (_, Some(Mode::Farm)) => aboard && sq.material < PILLBOY_MATERIAL_MAX,
        (_, Some(Mode::Road)) => aboard && sq.material >= BuildKind::Road.cost(),
        (_, Some(Mode::Wall)) => aboard && sq.material >= BuildKind::Wall.cost(),
        (_, Some(Mode::Pillbox)) => aboard,
        (_, Some(Mode::Mine)) => aboard && sq.material >= BuildKind::Mine.cost(),
        // A capture needs no material and a build its kind's, so the
        // key is lit whenever the squad is aboard; the word says the kind.
        (_, Some(Mode::Works)) => aboard && (works.cost() <= PILLBOY_MATERIAL_MAX),
        (RETURN_ROW, _) => !sq.aboard && sq.strength > 0,
        _ => false,
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
        // Under a goal (step 11a), what the tank is doing for it.
        if !view.goal.is_empty() {
            first.push_str("   ");
            first.push_str(&view.goal);
        }
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
    let mut s = String::from("GEEKERS    t  seat  score\n");
    let mut ps: Vec<&PlayerView> = obs.players.iter().collect();
    ps.sort_by_key(|p| (p.team, p.id));
    for p in ps {
        let mark = if p.id == obs.player { "*" } else if obs.allied(p.id) { "+" } else { " " };
        s.push_str(&format!("{}{:<9} {:<2} p{:<3} {:>4} {}\n", mark, p.name.chars().take(9).collect::<String>(), p.team, p.id, p.score, if p.alive { "" } else { "x" }));
    }
    for r in &obs.proposals {
        s.push_str(&crate::proposals::standing(obs, r));
        s.push('\n');
    }
    s.trim_end().to_string()
}

fn air(view: &View) -> String {
    let Some(obs) = view.obs.as_ref() else { return String::new() };
    let Some(t) = obs.me.as_ref() else { return String::new() };
    if !t.alive {
        return "TANK DESTROYED. rig down".into();
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
                DroneKind::Longshot => "L",
            };
            let task = match &d.task {
                DroneTask::Orbit { .. } => "".to_string(),
                DroneTask::Intercept { target } => format!(" INTERCEPT #{}", target),
                DroneTask::Strike { target } => format!(" STRIKE #{}", target),
                DroneTask::Return => " home".into(),
                DroneTask::Attack { at } => format!(" to {:.0},{:.0}", at.x, at.y),
            };
            if d.kind == DroneKind::Longshot {
                format!("{} {:.0}s{}", k, d.battery, task)
            } else {
                format!("{} {:.0}% {:.0}s{}", k, d.link * 100.0, d.battery, task)
            }
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
/// The newswire: the last event, except that a refused order is held
/// for a few seconds (or until the next order, input.rs) over whatever
/// comes after it.
fn wire(view: &View) -> String {
    // The line being typed comes first: it is what the player is doing.
    if let Some(t) = view.typing.as_ref() {
        let to = crate::chat::audience(t, view.obs.as_ref());
        let wait = if view.chat_sent.map(|s| s.elapsed() < crate::chat::interval()).unwrap_or(false) { " (a second)" } else { "" };
        return format!("say to {}{}> {}_", to, wait, t.line);
    }
    if let Some((when, why)) = view.refused.as_ref().filter(|(w, _)| w.elapsed().as_secs_f32() < REFUSAL_HOLD) {
        let _ = when;
        return format!("{:>6}  {}", view.last_tick, why);
    }
    // What my side is losing (step 10d) is held over the events after
    // it, in red: the first window run had a power plant wrecked and its
    // alert overwritten by a longshot's line five ticks later.
    if let Some(a) = wire_alert(view) {
        return format!("{:>6}  ALERT {}  (J)", view.last_tick, a.text);
    }
    // A proposal to me or my side stands on the wire while it can be
    // answered: it asks for a key.
    if let Some(o) = view.obs.as_ref() {
        if let Some(p) = crate::proposals::answerable(o) {
            return format!("{:>6}  {}", view.last_tick, crate::proposals::offer(o, p));
        }
    }
    // A message is held over the events after it: it was meant to be read.
    if let Some((_, l)) = view.chat.as_ref().filter(|(w, _)| w.elapsed().as_secs_f32() < CHAT_HOLD) {
        return format!("{:>6}  {}", view.last_tick, l);
    }
    view.log.back().map(|(t, l)| format!("{:>6}  {}", t, l)).unwrap_or_default()
}

/// How long an alert stays on the wire.
const ALERT_WIRE_HOLD: f32 = 5.0;

/// The alert the wire is holding, if any.
fn wire_alert(view: &View) -> Option<&crate::commander::Alert> {
    view.alerts.back().filter(|a| a.when.elapsed().as_secs_f32() < ALERT_WIRE_HOLD)
}

/// How many structures the ledger has lamps for.
const LEDGER: usize = 12;

/// My side's structures, mine first, each in the order the world made
/// them.
pub fn ledger(o: &Observation) -> Vec<&Structure> {
    let mut v: Vec<&Structure> = o.structures.iter().filter(|x| x.owner.map(|w| w == o.player || o.allied(w)).unwrap_or(false)).collect();
    v.sort_by_key(|x| (x.owner != Some(o.player), x.id));
    v.truncate(LEDGER);
    v
}

/// A ledger lamp: the side's colour while it works, amber dark (no
/// power), red under half, dim wrecked.
pub fn ledger_color(x: &Structure, side: Color) -> Color {
    if !x.alive() {
        DIM
    } else if x.health < STRUCTURE_HEALTH * 0.5 {
        RED
    } else if !x.powered && x.kind.needs_power() {
        AMBER
    } else {
        side
    }
}

/// How long a message stays on the wire.
const CHAT_HOLD: f32 = 8.0;

/// How long a refusal stays on the wire.
const REFUSAL_HOLD: f32 = 4.0;

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
            s.push_str(&format!("\nkills {}  deaths {}\n", view.kills, view.deaths));
            if view.net.mode == "solo" {
                // The second match (PLAYABILITY item 10).
                s.push_str(&format!("rounds won {} of {}\n\nEnter: again (same grid, bots reseeded)   Esc to leave", view.record.1, view.record.0));
            } else {
                s.push_str(&format!("rounds won {} of {}\n\nEnter: back to the lobby for another\n(the others meet you there as they press Enter)   Esc to leave", view.record.1, view.record.0));
            }
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
  the bots hold fire for a minute at a tank that has not moved or fired;
  after that, a tank that stands still is found: move after you fire
  F follows your tank; ' jumps to your pillboxes; Tab shows the events
  a falling whistle is a shell coming down there; M turns the sound off
";

#[cfg(test)]
mod tests {
    use super::*;

    /// An alert holds the wire over the events after it, and lets go
    /// after `ALERT_WIRE_HOLD`; the ledger lists my side's structures,
    /// mine first, and lights each by its state.
    #[test]
    fn alerts_hold_the_wire_and_the_ledger_tells_state() {
        let mut w = treadcore_sim::World::new(Scenario { swarm_interval: 0, industry: true, ..Scenario::bots(4) });
        w.structures[0].owner = Some(1);
        w.structures[1].owner = Some(0);
        w.structures[2].owner = Some(2);
        let mut view = View { obs: Some(w.observe(0)), ..View::default() };
        let alert = |secs: u64| crate::commander::Alert { at: V2::new(1.0, 1.0), text: "your power plant WRECKED at 1,1".into(), when: std::time::Instant::now() - std::time::Duration::from_secs(secs), about: 1 };
        view.alerts.push_back(alert(0));
        view.log.push_back((10, "p1 lost Longshot (Expended)".into()));
        assert!(wire(&view).contains("  ALERT your power plant WRECKED"), "{}", wire(&view));
        view.alerts.clear();
        view.alerts.push_back(alert(ALERT_WIRE_HOLD as u64 + 1));
        assert!(wire(&view).contains("Longshot"), "{}", wire(&view));

        let o = view.obs.as_ref().unwrap();
        let l = ledger(o);
        assert_eq!(l.len(), 1, "seat 0 alone holds one structure");
        assert_eq!(l[0].owner, Some(0));
        let mut x = l[0].clone();
        x.powered = true;
        x.health = STRUCTURE_HEALTH;
        assert_eq!(ledger_color(&x, GREEN), GREEN);
        x.health = STRUCTURE_HEALTH * 0.4;
        assert_eq!(ledger_color(&x, GREEN), RED);
        x.health = 0.0;
        assert_eq!(ledger_color(&x, GREEN), DIM);
        x.health = STRUCTURE_HEALTH;
        x.powered = false;
        x.kind = StructureKind::Refinery;
        assert_eq!(ledger_color(&x, GREEN), AMBER);
    }
}
