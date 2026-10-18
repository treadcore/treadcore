//! Assets out of cubes. Every thing on the map is a small grid of
//! cubes, drawn in an oblique projection with the engine's sprites: a
//! cube is its top face, lit by its height, and a darker side face
//! hanging below it where no cube stands in front. No meshes, no
//! images, no third dimension in the camera; the rule that everything
//! drawn is a rectangle holds, and a tank is a tank.
//!
//! The cubes are small: a tenth of a tile for the things that move and
//! shoot, a quarter for the ground's blocks and pits, so a model has
//! the detail of a sprite and the parts of a thing. That is what lets
//! it break. Damage takes cubes off a model, highest first, in an order
//! fixed by their place, so a pillbox at half health has lost its gun
//! and the top of its pyramid, and a hull at a third has its turret
//! chipped. What comes off goes somewhere: `render.rs` throws loose
//! cubes out of an impact that roll, bounce and come to rest.
//!
//! The models are Bolo's things in Bolo's colours (the WinBolo screen
//! and the cover, journal 0015): a tan stepped pyramid of a pillbox
//! with an orange gun on top, a tan base plate with the owner's mark.
//! The tanks are the sprite kit's three chassis (`TankStyle`), each a
//! hull, four frames of running gear and a turret apart, so the turret
//! turns and the treads roll (battlefield.rs).

use crate::TILE;
use bevy::color::Mix;
use bevy::prelude::*;

/// How far up the screen one level of height lifts a top face, as a
/// fraction of the model's unit; a level below the ground sinks it as
/// far.
pub const RISE: f32 = 0.8;

#[derive(Clone, Copy)]
pub struct Cube {
    pub x: i8,
    pub y: i8,
    pub z: i8,
    pub color: Color,
}

#[derive(Clone)]
pub struct Model {
    /// The cube's edge in world units.
    pub unit: f32,
    /// The point of the grid that sits on the thing's position.
    pub cx: f32,
    pub cy: f32,
    pub cubes: Vec<Cube>,
}

impl Model {
    fn new(unit_tiles: f32) -> Model {
        Model { unit: TILE * unit_tiles, cx: 0.0, cy: 0.0, cubes: Vec::new() }
    }

    fn cube(&mut self, x: i8, y: i8, z: i8, color: Color) {
        self.cubes.push(Cube { x, y, z, color });
    }

    /// A filled rectangle of cubes at one height.
    fn slab(&mut self, x0: i8, y0: i8, x1: i8, y1: i8, z: i8, color: Color) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.cube(x, y, z, color);
            }
        }
    }

    fn has(&self, x: i8, y: i8, z: i8) -> bool {
        self.cubes.iter().any(|c| c.x == x && c.y == y && c.z == z)
    }

    /// Damage: take `frac` of the cubes off, the highest first, in an
    /// order fixed by each cube's place, so the same damage always
    /// looks the same and more damage only takes more.
    fn chip(&mut self, frac: f32) {
        if frac <= 0.0 {
            return;
        }
        let n = ((self.cubes.len() as f32) * frac.clamp(0.0, 1.0)) as usize;
        let mut order: Vec<usize> = (0..self.cubes.len()).collect();
        let place = |c: &Cube| -> u32 {
            let h = (c.x as i32 * 73856093) ^ (c.y as i32 * 19349663) ^ (c.z as i32 * 83492791);
            h as u32 % 1000
        };
        order.sort_by_key(|&i| (std::cmp::Reverse(self.cubes[i].z), place(&self.cubes[i])));
        let gone: Vec<usize> = order.into_iter().take(n).collect();
        let mut keep = Vec::with_capacity(self.cubes.len());
        for (i, c) in self.cubes.iter().enumerate() {
            if !gone.contains(&i) {
                keep.push(*c);
            }
        }
        self.cubes = keep;
    }

    /// Spawn the model's sprites as children. `z_base` is the layer the
    /// whole model sits on; cubes stack above it by their height, and
    /// nearer rows (smaller y, lower on the screen) draw over farther.
    /// A top face under another cube and a side face behind another
    /// cube are not drawn at all: a wall of 48 cubes is 28 sprites.
    /// A cube below the ground (z < 0) is a pit: its floor sits lower
    /// and darker, and the pit's far wall shows above it in shadow.
    pub fn spawn(&self, p: &mut ChildSpawnerCommands, z_base: f32) {
        let u = self.unit;
        let rise = u * RISE;
        for c in &self.cubes {
            let x = (c.x as f32 - self.cx) * u;
            let y = (c.y as f32 - self.cy) * u + c.z as f32 * rise;
            let depth = z_base + c.z as f32 * 0.02 - c.y as f32 * 0.0005;
            if c.z >= 0 {
                if !self.has(c.x, c.y - 1, c.z) {
                    p.spawn((
                        Sprite { color: c.color.darker(0.14), custom_size: Some(Vec2::new(u + 0.2, rise + 0.3)), ..default() },
                        Transform::from_translation(Vec3::new(x, y - u * 0.5 - rise * 0.5, depth)),
                    ));
                }
                if !self.has(c.x, c.y, c.z + 1) {
                    p.spawn((
                        Sprite { color: c.color.lighter(0.03 * c.z as f32), custom_size: Some(Vec2::splat(u + 0.2)), ..default() },
                        Transform::from_translation(Vec3::new(x, y, depth + 0.01)),
                    ));
                }
            } else {
                // The floor keeps its colour (a shade toward black a
                // level below the first) and the far wall is the same
                // earth in shadow: mixed toward black, not `darker`,
                // which on a brown this dim bottoms out at black and
                // drew every crater as a black bar.
                let drop = -(c.z as f32) * rise;
                p.spawn((
                    Sprite { color: c.color.mix(&Color::BLACK, 0.08 * (-(c.z as f32) - 1.0)), custom_size: Some(Vec2::splat(u + 0.2)), ..default() },
                    Transform::from_translation(Vec3::new(x, y, depth + 0.01)),
                ));
                if !self.has(c.x, c.y + 1, c.z) {
                    p.spawn((
                        Sprite { color: c.color.mix(&Color::BLACK, 0.4), custom_size: Some(Vec2::new(u + 0.2, drop + 0.3)), ..default() },
                        Transform::from_translation(Vec3::new(x, (c.y as f32 - self.cy) * u + u * 0.5 - drop * 0.5, depth + 0.02)),
                    ));
                }
            }
        }
    }
}

// Bolo's colours: the tan and brown of its building sprite, sampled
// from the WinBolo screenshots.
pub const TAN: Color = Color::srgb(0.75, 0.63, 0.44);
pub const TAN_DARK: Color = Color::srgb(0.50, 0.37, 0.25);
pub const GUN: Color = Color::srgb(0.95, 0.55, 0.2);
pub const OLIVE: Color = Color::srgb(0.42, 0.42, 0.26);
pub const STEEL: Color = Color::srgb(0.62, 0.64, 0.62);
pub const MINE: Color = Color::srgb(0.95, 0.2, 0.15);
pub const EARTH: Color = Color::srgb(0.45, 0.36, 0.26);

/// The three chassis a side can field (the sprite kit, journal 0037):
/// each a hull, its running gear in four frames, and a turret that
/// turns on its own. Cosmetic: the world's tank is one tank whatever
/// it is drawn as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TankStyle {
    /// Slab-sided battle tank: wide tracks, a long gun, a grille astern.
    #[default]
    Bulwark,
    /// Twin-track raider: broad chevron tracks, an open cockpit, a light
    /// bar, two short guns on a small mount.
    Strider,
    /// Wheeled scout: four fat tyres, a red body with stripes, a wing.
    Rattler,
}

impl TankStyle {
    pub const ALL: [TankStyle; 3] = [TankStyle::Bulwark, TankStyle::Strider, TankStyle::Rattler];

    pub fn word(self) -> &'static str {
        match self {
            TankStyle::Bulwark => "bulwark",
            TankStyle::Strider => "strider",
            TankStyle::Rattler => "rattler",
        }
    }

    pub fn from_word(w: &str) -> Option<TankStyle> {
        TankStyle::ALL.into_iter().find(|s| s.word() == w)
    }

    pub fn next(self) -> TankStyle {
        TankStyle::ALL[(TankStyle::ALL.iter().position(|s| *s == self).unwrap() + 1) % 3]
    }

    /// The other two, for the sides that are not mine.
    pub fn others(self) -> [TankStyle; 2] {
        let o: Vec<TankStyle> = TankStyle::ALL.into_iter().filter(|s| *s != self).collect();
        [o[0], o[1]]
    }

    /// Where the turret sits on the hull, in cubes forward of centre.
    pub fn mount(self) -> f32 {
        match self {
            TankStyle::Bulwark => -0.5,
            TankStyle::Strider => -2.0,
            TankStyle::Rattler => -0.5,
        }
    }

    /// The kit's turret traverse, radians a second: the battle tank's
    /// heavy turret slow, the raider's mount quick.
    pub fn traverse(self) -> f32 {
        match self {
            TankStyle::Bulwark => 1.7,
            TankStyle::Strider => 3.4,
            TankStyle::Rattler => 2.6,
        }
    }

    /// How far the turret kicks back on a shot, in tiles, and how many
    /// sparks leave the muzzle: the kit's 8 and 10 for the long gun,
    /// 3 and 4 for the light ones.
    pub fn recoil(self) -> f32 {
        if self == TankStyle::Bulwark { 0.14 } else { 0.05 }
    }

    pub fn sparks(self) -> usize {
        if self == TankStyle::Bulwark { 10 } else { 4 }
    }

    /// The barrels, in cubes across the turret: the raider's two fire
    /// in turn.
    pub fn barrels(self) -> &'static [f32] {
        match self {
            TankStyle::Strider => &[-1.0, 0.0],
            _ => &[0.0],
        }
    }

    /// The muzzle, in cubes forward of the turret's pivot.
    pub fn muzzle(self) -> f32 {
        match self {
            TankStyle::Bulwark => 12.0,
            TankStyle::Strider => 7.0,
            TankStyle::Rattler => 9.0,
        }
    }
}

const DEAD_HULL: Color = Color::srgb(0.28, 0.28, 0.27);
const DEAD_TOP: Color = Color::srgb(0.35, 0.35, 0.34);

const fn hex(rgb: u32) -> Color {
    Color::srgb(((rgb >> 16) & 0xff) as f32 / 255.0, ((rgb >> 8) & 0xff) as f32 / 255.0, (rgb & 0xff) as f32 / 255.0)
}

// The kit's colours, sampled from its SVG plates.
const SAND: Color = hex(0xa58d61);
const SAND_TOP: Color = hex(0xc2a877);
const SAND_BOW: Color = hex(0xcdb88a);
const SAND_DARK: Color = hex(0x6f5d3e);
const SAND_DEEP: Color = hex(0x4a3e29);
const SAND_TRACK: (Color, Color) = (hex(0x2b2620), hex(0x51483a));
const LAMP_WHITE: Color = hex(0xece5c8);
const SLATE: Color = hex(0x5b6470);
const SLATE_TOP: Color = hex(0x717c88);
const SLATE_GREY: Color = hex(0x4a525c);
const CAGE: Color = hex(0x2a2f35);
const COCKPIT: Color = hex(0x18222d);
const GLASS: Color = hex(0x46688a);
const LED: Color = hex(0xffd75e);
const TAIL: Color = hex(0xff4d3a);
const SLATE_TRACK: (Color, Color) = (hex(0x1a1d20), hex(0x3b4147));
const RED: Color = hex(0xb8342b);
const STRIPE: Color = hex(0xefe9dc);
const HEADLAMP: Color = hex(0xfff3c4);
const BLACK_TRIM: Color = hex(0x222222);
const TYRE: (Color, Color) = (hex(0x141414), hex(0x3a3a3a));
const AXLE: Color = hex(0x8a8f96);
const DISC: Color = hex(0xe4dfd4);
const GUNMETAL: Color = hex(0x2a2a2a);

/// The hull without its running gear or turret. The seat's colour is
/// the kit's trim: the Bulwark's skirts, the Strider's rails. Damage
/// chips it as `tank` did; dead, it is grey.
pub fn hull(style: TankStyle, color: Color, alive: bool, damage: f32) -> Model {
    let mut m = Model::new(0.1);
    let live = |c: Color| if alive { c } else { DEAD_HULL };
    match style {
        TankStyle::Bulwark => {
            m.slab(-7, -2, 6, 1, 0, live(SAND));
            m.slab(-5, -2, 4, 1, 1, live(SAND_TOP));
            // The glacis at the bow with its two lamps and the dashed
            // plate, the engine grille and the exhausts astern.
            m.slab(5, -2, 6, 1, 1, live(SAND_BOW));
            m.cube(6, -2, 1, live(LAMP_WHITE));
            m.cube(6, 1, 1, live(LAMP_WHITE));
            m.cube(5, -1, 1, live(SAND_DEEP));
            m.cube(5, 0, 1, live(SAND_DEEP));
            for x in -6..=-4 {
                for y in -2..=1 {
                    m.cube(x, y, 1, live(if x % 2 == 0 { SAND_DEEP } else { SAND_DARK }));
                }
            }
            m.cube(-7, -2, 1, live(hex(0x3a332a)));
            m.cube(-7, 1, 1, live(hex(0x3a332a)));
            for x in -3..=2 {
                m.cube(x, -2, 1, live(color.mix(&SAND_DARK, 0.35)));
                m.cube(x, 1, 1, live(color.mix(&SAND_DARK, 0.35)));
            }
        }
        TankStyle::Strider => {
            m.slab(-7, -1, 6, 0, 0, live(SLATE));
            m.slab(-6, -1, 5, 0, 1, live(SLATE));
            m.slab(4, -1, 6, 0, 1, live(SLATE_TOP));
            // The rails down each side in the seat's colour (the kit's
            // orange), the open cockpit under its roll cage, the light
            // bar on the cage, the engine deck and the tail lights.
            for x in -5..=4 {
                m.cube(x, -2, 1, live(color));
                m.cube(x, 1, 1, live(color));
            }
            m.slab(1, -1, 3, 0, 1, live(COCKPIT));
            m.cube(3, -1, 2, live(GLASS));
            m.cube(3, 0, 2, live(GLASS));
            for y in [-1, 0] {
                m.cube(3, y, 3, live(LED));
                m.cube(0, y, 3, live(CAGE));
            }
            for x in 0..=3 {
                m.cube(x, -2, 3, live(CAGE));
                m.cube(x, 1, 3, live(CAGE));
            }
            m.slab(-7, -1, -5, 0, 2, live(SLATE_GREY));
            m.cube(-7, -1, 1, live(TAIL));
            m.cube(-7, 0, 1, live(TAIL));
        }
        TankStyle::Rattler => {
            m.slab(-6, -2, 6, 1, 0, live(RED));
            m.slab(-5, -2, 5, 1, 1, live(RED));
            // Racing stripes nose to tail, the windscreen, the lamps and
            // the bumper, the wing astern on red end plates.
            for x in -5..=5 {
                m.cube(x, -1, 2, live(STRIPE));
                m.cube(x, 0, 2, live(STRIPE));
            }
            m.slab(3, -2, 4, -2, 2, live(COCKPIT));
            m.slab(3, 1, 4, 1, 2, live(COCKPIT));
            m.cube(6, -2, 1, live(HEADLAMP));
            m.cube(6, 1, 1, live(HEADLAMP));
            m.slab(7, -3, 7, 2, 0, live(BLACK_TRIM));
            m.slab(-7, -4, -7, 3, 2, live(BLACK_TRIM));
            m.cube(-7, -4, 1, live(RED));
            m.cube(-7, 3, 1, live(RED));
        }
    }
    if alive {
        m.chip(damage * 0.45);
    } else {
        m.chip(0.4);
    }
    m
}

/// The running gear at `phase` (0 to 3), the kit's four-frame tread
/// cycle: the links, or the tyres' blocks, a cube further back each
/// frame, so driving rolls them under the hull. Forward is +x; the
/// pattern walks -x as the phase rises.
pub fn running_gear(style: TankStyle, alive: bool, phase: u8) -> Model {
    let mut m = Model::new(0.1);
    let p = phase as i32;
    let link = |x: i32, (a, b): (Color, Color)| if !alive { DEAD_TOP.darker(0.1) } else if (x + p).rem_euclid(4) < 2 { a } else { b };
    match style {
        TankStyle::Bulwark => {
            for x in -7..=6i8 {
                for y in [-4, -3, 2, 3] {
                    m.cube(x, y, 0, link(x as i32, SAND_TRACK));
                }
            }
        }
        TankStyle::Strider => {
            // Broad chevron tracks: three wide, the middle link a cube
            // ahead of its sides, the idlers at both ends.
            for x in -7..=6i8 {
                for (y, lead) in [(-4, 0), (-3, 1), (-2, 0), (1, 0), (2, 1), (3, 0)] {
                    m.cube(x, y, 0, link(x as i32 + lead, SLATE_TRACK));
                }
            }
            if alive {
                for x in [-7, 6] {
                    m.cube(x, -3, 1, hex(0x444b52));
                    m.cube(x, 2, 1, hex(0x444b52));
                }
            }
        }
        TankStyle::Rattler => {
            for x0 in [-6i8, 3] {
                for x in x0..x0 + 3 {
                    for y in [-5, -4, 3, 4] {
                        m.cube(x, y, 0, link(x as i32, TYRE));
                    }
                }
                if alive {
                    m.cube(x0 + 1, -3, 0, AXLE);
                    m.cube(x0 + 1, 2, 0, AXLE);
                }
            }
        }
    }
    m
}

/// The turret on its own, the pivot at the grid's origin and the gun
/// forward (+x), so it turns apart from the hull as the kit's does.
pub fn turret(style: TankStyle, color: Color, alive: bool, damage: f32) -> Model {
    let mut m = Model::new(0.1);
    let live = |c: Color| if alive { c } else { DEAD_TOP };
    match style {
        TankStyle::Bulwark => {
            m.slab(-3, -2, 2, 1, 2, live(SAND_TOP));
            m.slab(1, -2, 2, 1, 3, live(SAND_BOW));
            // The commander's cupola in the seat's colour, the hatch.
            m.slab(-2, 0, -1, 1, 3, live(color));
            m.cube(-2, -2, 3, live(SAND));
            for x in 3..=10 {
                m.cube(x, 0, 2, live(SAND_DARK));
            }
            m.cube(11, 0, 2, live(SAND_DEEP));
            m.cube(12, 0, 2, live(SAND_DEEP));
        }
        TankStyle::Strider => {
            // The remote weapon station: a slate box, cheeks either side,
            // two short guns.
            m.slab(-1, -1, 1, 0, 3, live(SLATE_TOP));
            m.cube(0, -2, 3, live(color.mix(&SLATE_TOP, 0.3)));
            m.cube(0, 1, 3, live(color.mix(&SLATE_TOP, 0.3)));
            for x in 2..=6 {
                m.cube(x, -1, 3, live(CAGE));
                m.cube(x, 0, 3, live(CAGE));
            }
            m.cube(7, -1, 3, live(SLATE_TRACK.0));
            m.cube(7, 0, 3, live(SLATE_TRACK.0));
        }
        TankStyle::Rattler => {
            // The white disc with the ring in the seat's colour (the
            // kit's red), the gun out of its middle.
            for (x, y) in [(-1, -2), (0, -2), (-2, -1), (1, -1), (-2, 0), (1, 0), (-1, 1), (0, 1)] {
                m.cube(x, y, 2, live(color));
            }
            m.slab(-1, -1, 0, 0, 3, live(DISC));
            for x in 1..=8 {
                m.cube(x, 0, 3, live(GUNMETAL));
            }
            m.cube(9, 0, 3, live(TYRE.0));
        }
    }
    if alive {
        m.chip(damage * 0.45);
    } else {
        m.chip(0.6);
    }
    m
}

/// A landing craft (SPEC TNK-08): a flat grey-green hull a size up from
/// a tank's, low gunwales, the bow ramp forward. Under a tank afloat it
/// is the boat the tank rides; empty and moored it carries the
/// builder's colour on its stern.
pub fn landing_craft(owner: Option<Color>) -> Model {
    let mut m = Model::new(0.1);
    let hull = hex(0x55605a);
    let rail = hex(0x6d7a72);
    let ramp = hex(0x8a978e);
    m.slab(-10, -5, 8, 4, 0, hull);
    for x in -10..=8 {
        m.cube(x, -5, 1, rail);
        m.cube(x, 4, 1, rail);
    }
    for y in -5..=4 {
        m.cube(-10, y, 1, rail);
    }
    // The ramp, hinged at the bow, a shade lighter, with its ribs.
    m.slab(9, -4, 10, 3, 0, ramp);
    m.cube(9, -2, 1, ramp.darker(0.1));
    m.cube(9, 1, 1, ramp.darker(0.1));
    if let Some(c) = owner {
        m.slab(-9, -1, -8, 0, 1, c);
    }
    m.cx = -0.5;
    m.cy = -0.5;
    m
}

/// A pillbox: Bolo's tan pyramid in four steps with the gun on top in
/// orange, tinted toward the owner. Damage takes it down from the top:
/// at half health the gun is gone; dead, a dark slab.
pub fn pillbox(owner: Option<Color>, health: f32, alive: bool) -> Model {
    let mut m = Model::new(0.1);
    if !alive {
        m.slab(-5, -5, 4, 4, 0, Color::srgb(0.25, 0.22, 0.2));
        m.cx = -0.5;
        m.cy = -0.5;
        return m;
    }
    let base = owner.map(|c| c.mix(&TAN, 0.6)).unwrap_or(TAN);
    m.slab(-5, -5, 4, 4, 0, base);
    m.slab(-4, -4, 3, 3, 1, base.lighter(0.02));
    m.slab(-3, -3, 2, 2, 2, base.lighter(0.04));
    m.slab(-2, -2, 1, 1, 3, base.lighter(0.06));
    m.slab(-1, -1, 0, 0, 4, GUN);
    m.cube(1, 0, 4, GUN);
    m.cube(2, 0, 4, GUN);
    m.cx = -0.5;
    m.cy = -0.5;
    m.chip((1.0 - health.clamp(0.0, 1.0)) * 0.5);
    m
}

/// A depot, Bolo's base: a plate with a post at each corner and the
/// owner's mark raised in the middle.
pub fn depot(owner: Option<Color>) -> Model {
    let mut m = Model::new(0.1);
    m.slab(-6, -6, 5, 5, 0, TAN_DARK);
    m.slab(-5, -5, 4, 4, 0, TAN);
    for (x, y) in [(-6, -6), (5, -6), (-6, 5), (5, 5)] {
        m.cube(x, y, 1, TAN_DARK);
    }
    let mark = owner.unwrap_or(Color::srgb(0.5, 0.5, 0.45));
    m.slab(-2, -2, 1, 1, 1, mark);
    m.cx = -0.5;
    m.cy = -0.5;
    m
}

/// The recon: a cross that looks, a sensor on top.
pub fn recon(color: Color) -> Model {
    let mut m = Model::new(0.1);
    for d in -2..=2 {
        m.cube(d, 0, 0, color);
        m.cube(0, d, 0, color);
    }
    m.cube(0, 0, 1, color.lighter(0.2));
    m
}

/// The interceptor: a small block with a nose, the lighter one.
pub fn interceptor(color: Color) -> Model {
    let mut m = Model::new(0.1);
    m.slab(-1, -1, 1, 1, 0, color.lighter(0.15));
    m.cube(2, 0, 0, color.lighter(0.3));
    m
}

/// The strike: a dart along +x with fins; the swarm's in red.
pub fn strike(color: Color) -> Model {
    let mut m = Model::new(0.1);
    for x in -3..=3 {
        m.cube(x, 0, 0, color);
    }
    m.cube(-3, -1, 0, color.darker(0.1));
    m.cube(-3, 1, 0, color.darker(0.1));
    m.cube(3, 0, 1, color.lighter(0.2));
    m
}

pub fn shell(color: Color) -> Model {
    let mut m = Model::new(0.1);
    m.slab(0, 0, 1, 1, 2, color);
    m
}

pub fn mine(color: Color) -> Model {
    let mut m = Model::new(0.1);
    m.slab(-1, -1, 1, 1, 0, color);
    m.cube(0, 0, 1, color.lighter(0.2));
    m
}

/// The squad: four men, in the owner's colour, a head each.
pub fn squad(color: Color) -> Model {
    let mut m = Model::new(0.1);
    for (x, y) in [(-2, -1), (1, -1), (-1, 2), (2, 2)] {
        m.cube(x, y, 0, color);
        m.cube(x, y, 1, color.lighter(0.25));
    }
    m
}

/// A wall tile: a tan block the size of the tile, three levels high at
/// a quarter of a tile, so it stands up from the map the way Bolo's
/// buildings did, aligned to its tile.
pub fn wall() -> Model {
    let mut m = Model::new(0.25);
    for z in 0..3 {
        m.slab(-2, -2, 1, 1, z, if z == 2 { TAN } else { TAN.darker(0.04) });
    }
    m.cx = -0.5;
    m.cy = -0.5;
    m
}

/// A crater: the tile sunk two levels, earth on its floor, aligned to
/// its tile.
pub fn crater() -> Model {
    let mut m = Model::new(0.25);
    m.slab(-2, -2, 1, 1, -2, EARTH);
    m.cx = -0.5;
    m.cy = -0.5;
    m
}

/// Concrete (step 10a): a wall's block in pale grey, a level taller.
pub fn concrete() -> Model {
    let mut m = Model::new(0.25);
    for z in 0..4 {
        m.slab(-2, -2, 1, 1, z, if z == 3 { CONCRETE } else { CONCRETE.darker(0.05) });
    }
    m.cx = -0.5;
    m.cy = -0.5;
    m
}

pub const CONCRETE: Color = Color::srgb(0.72, 0.72, 0.70);
pub const PAD: Color = Color::srgb(0.36, 0.37, 0.39);
/// A working structure's lamps.
pub const LIT: Color = Color::srgb(1.0, 0.9, 0.35);

/// The longshot (step 10c): a long body with a straight wing and a
/// tail, the one-way drone of the talk, larger than a strike.
pub fn longshot(color: Color) -> Model {
    let mut m = Model::new(0.1);
    for x in -4..=4 {
        m.cube(x, 0, 0, color.darker(0.05));
    }
    for y in -4..=4 {
        m.cube(0, y, 0, color);
    }
    m.cube(-4, -1, 0, color.darker(0.15));
    m.cube(-4, 1, 0, color.darker(0.15));
    m.cube(4, 0, 1, color.lighter(0.2));
    m
}

/// A structure (step 10b) on its pad: a plate with the owner's mark
/// along its front edge, and on it the thing itself, each kind its own
/// silhouette so it reads at a glance; lamps lit while it works.
/// Damage takes it down from the top like a pillbox; a wreck is a
/// low dark heap.
pub fn structure(kind: treadcore_sim::StructureKind, owner: Option<Color>, health: f32, alive: bool, powered: bool) -> Model {
    use treadcore_sim::StructureKind as K;
    let mut m = Model::new(0.1);
    let mark = owner.unwrap_or(Color::srgb(0.55, 0.55, 0.52));
    m.slab(-6, -6, 5, 5, 0, PAD.lighter(0.04));
    for x in -6..=5 {
        m.cube(x, -6, 0, mark);
    }
    m.cx = -0.5;
    m.cy = -0.5;
    if !alive {
        m.slab(-4, -4, 3, 3, 1, Color::srgb(0.22, 0.2, 0.19));
        m.slab(-2, -3, 2, 1, 2, Color::srgb(0.18, 0.16, 0.15));
        return m;
    }
    let lamp = if powered { LIT } else { Color::srgb(0.25, 0.25, 0.25) };
    match kind {
        K::Power => {
            // A turbine hall and two cooling towers.
            m.slab(-5, -4, 0, 1, 1, STEEL.darker(0.1));
            m.slab(-5, -4, 0, 1, 2, STEEL);
            for (cx, cy) in [(3, -2), (3, 3)] {
                for z in 1..=5 {
                    let r = if z == 3 { 1 } else { 2 };
                    m.slab(cx - r, cy - r, cx + r - 1, cy + r - 1, z, CONCRETE.darker(0.02 * z as f32));
                }
            }
            m.cube(-3, -2, 3, lamp);
        }
        K::Refinery => {
            // Round tanks and a tall flare stack.
            for (cx, cy) in [(-3, -3), (-3, 2), (1, -3)] {
                m.slab(cx - 1, cy - 1, cx + 1, cy + 1, 1, TAN.lighter(0.05));
                m.slab(cx - 1, cy - 1, cx + 1, cy + 1, 2, TAN.lighter(0.1));
            }
            for z in 1..=7 {
                m.cube(3, 2, z, STEEL.darker(0.15));
            }
            m.cube(3, 2, 8, if powered { GUN } else { lamp });
        }
        K::DataCentre => {
            // A long low hall, racks of lights down its roof.
            m.slab(-5, -4, 4, 3, 1, Color::srgb(0.3, 0.32, 0.36));
            m.slab(-5, -4, 4, 3, 2, Color::srgb(0.36, 0.38, 0.42));
            for x in (-4..=3).step_by(2) {
                for y in [-3i8, 0, 2] {
                    m.cube(x, y, 3, if powered { Color::srgb(0.35, 0.8, 1.0) } else { lamp });
                }
            }
        }
        K::Workshop => {
            // A shed and a crane arm.
            m.slab(-5, -4, 1, 3, 1, TAN_DARK);
            m.slab(-5, -4, 1, 3, 2, TAN_DARK.lighter(0.05));
            m.slab(-4, -3, 0, 2, 3, TAN_DARK.lighter(0.1));
            for z in 1..=5 {
                m.cube(3, -3, z, GUN);
            }
            for y in -3..=3 {
                m.cube(3, y, 6, GUN);
            }
            m.cube(-2, -1, 4, lamp);
        }
        K::Jammer => {
            // A mast with crossbars and a red tip.
            m.slab(-2, -2, 1, 1, 1, STEEL.darker(0.2));
            for z in 2..=9 {
                m.cube(0, 0, z, STEEL);
            }
            for z in [4i8, 7] {
                for d in -2..=2 {
                    m.cube(d, 0, z, STEEL.darker(0.1));
                }
            }
            m.cube(0, 0, 10, if powered { MINE } else { lamp });
        }
    }
    m.chip((1.0 - health.clamp(0.0, 1.0)) * 0.5);
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_style_builds_and_its_treads_roll() {
        let c = Color::srgb(0.4, 0.5, 0.9);
        for s in TankStyle::ALL {
            assert_eq!(TankStyle::from_word(s.word()), Some(s));
            assert!(!s.others().contains(&s));
            for alive in [true, false] {
                assert!(!hull(s, c, alive, 0.3).cubes.is_empty(), "{:?} hull", s);
                assert!(!turret(s, c, alive, 0.3).cubes.is_empty(), "{:?} turret", s);
            }
            let colours = |p| running_gear(s, true, p).cubes.iter().map(|c| c.color.to_srgba().red.to_bits()).collect::<Vec<_>>();
            assert_ne!(colours(0), colours(1), "{:?}: the gear does not roll", s);
            assert_eq!(colours(0), colours(0), "{:?}", s);
        }
        assert_eq!(TankStyle::Rattler.next(), TankStyle::Bulwark);
    }
}
