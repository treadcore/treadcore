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
//! with an orange gun on top, a tan base plate with the owner's mark,
//! an olive hull with dark tracks and a turret in the seat's colour.

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
pub const TRACK: Color = Color::srgb(0.2, 0.19, 0.16);
pub const TREAD: Color = Color::srgb(0.3, 0.29, 0.25);
pub const STEEL: Color = Color::srgb(0.62, 0.64, 0.62);
pub const MINE: Color = Color::srgb(0.95, 0.2, 0.15);
pub const EARTH: Color = Color::srgb(0.45, 0.36, 0.26);

/// The tank, Bolo's way: an olive hull fourteen cubes long and eight
/// wide, tracks two wide down each side with their treads, a
/// superstructure, a turret in the seat's colour and a barrel forward
/// (+x). `damage` is 1 minus armor over full armor: the turret and the
/// upper hull chip away. Dead, it is grey and burnt down to the hull.
pub fn tank(color: Color, alive: bool, damage: f32) -> Model {
    let mut m = Model::new(0.1);
    let hull = if alive { OLIVE } else { Color::srgb(0.28, 0.28, 0.27) };
    let turret = if alive { color } else { Color::srgb(0.35, 0.35, 0.34) };
    m.slab(-7, -4, 6, 3, 0, hull);
    for x in -7..=6 {
        let tread = if x % 2 == 0 { TRACK } else { TREAD };
        m.cube(x, -4, 0, tread);
        m.cube(x, -3, 0, tread);
        m.cube(x, 3, 0, tread);
        m.cube(x, 2, 0, tread);
    }
    m.slab(-5, -2, 4, 1, 1, hull.lighter(0.04));
    m.slab(-3, -2, 2, 1, 2, turret);
    m.slab(-2, -1, 1, 0, 3, turret.lighter(0.06));
    for x in 3..=11 {
        m.cube(x, 0, 2, STEEL);
    }
    if alive {
        m.chip(damage * 0.45);
    } else {
        m.chip(0.6);
    }
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
