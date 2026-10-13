//! Assets out of cubes. Every thing on the map is a small grid of
//! cubes, drawn in an oblique projection with the engine's sprites: a
//! cube is its top face, lit by its height, and a darker side face
//! hanging below it where no cube stands in front. No meshes, no
//! images, no third dimension in the camera; the rule that everything
//! drawn is a rectangle holds, and a tank is a tank.
//!
//! The models are Bolo's things in Bolo's colours (the WinBolo screen
//! and the cover, journal 0015): a tan pyramid of a pillbox with an
//! orange gun on top, a tan base plate with the owner's mark, a hull
//! with tracks and a turret in the seat's colour, and the drones as
//! what they are: a cross that looks, a ring that guards, a dart.
//!
//! A unit is a fifth of a tile. Heights rise by `RISE` per level, so a
//! two-high pillbox stands up from the map and a hull sits on it.

use crate::TILE;
use bevy::color::Mix;
use bevy::prelude::*;

/// A fifth of a tile: the cube's edge on the screen at zoom 1.
pub const UNIT: f32 = TILE * 0.2;
/// How far up the screen one level of height lifts a top face.
pub const RISE: f32 = UNIT * 0.55;

#[derive(Clone, Copy)]
pub struct Cube {
    pub x: i8,
    pub y: i8,
    pub z: i8,
    pub color: Color,
}

#[derive(Clone, Default)]
pub struct Model {
    pub cubes: Vec<Cube>,
}

impl Model {
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

    /// Spawn the model's sprites as children. `z_base` is the layer the
    /// whole model sits on; cubes stack above it by their height, and
    /// nearer rows (smaller y, lower on the screen) draw over farther.
    pub fn spawn(&self, p: &mut ChildSpawnerCommands, z_base: f32) {
        for c in &self.cubes {
            let x = c.x as f32 * UNIT;
            let y = c.y as f32 * UNIT + c.z as f32 * RISE;
            let depth = z_base + c.z as f32 * 0.02 - c.y as f32 * 0.001;
            // The side face, where no cube stands in front at this level.
            if !self.has(c.x, c.y - 1, c.z) {
                p.spawn((
                    Sprite { color: c.color.darker(0.14), custom_size: Some(Vec2::new(UNIT, RISE + 0.3)), ..default() },
                    Transform::from_translation(Vec3::new(x, y - UNIT * 0.5 - RISE * 0.5, depth)),
                ));
            }
            // The top face, lighter the higher it is.
            p.spawn((
                Sprite { color: c.color.lighter(0.04 * c.z as f32), custom_size: Some(Vec2::splat(UNIT + 0.3)), ..default() },
                Transform::from_translation(Vec3::new(x, y, depth + 0.01)),
            ));
        }
    }
}

// Bolo's colours.
pub const TAN: Color = Color::srgb(0.80, 0.64, 0.42);
pub const TAN_DARK: Color = Color::srgb(0.55, 0.42, 0.26);
pub const GUN: Color = Color::srgb(0.95, 0.55, 0.2);
pub const TRACK: Color = Color::srgb(0.22, 0.2, 0.17);
pub const STEEL: Color = Color::srgb(0.6, 0.62, 0.6);
pub const MINE: Color = Color::srgb(0.95, 0.2, 0.15);

/// The tank: a hull eight long and five wide, tracks down the sides, a
/// turret on top and a barrel forward (+x). The hull and turret in the
/// seat's colour; dead, in grey.
pub fn tank(color: Color, alive: bool) -> Model {
    let mut m = Model::default();
    let body = if alive { color } else { Color::srgb(0.3, 0.3, 0.3) };
    let track = if alive { TRACK } else { Color::srgb(0.2, 0.2, 0.2) };
    m.slab(-4, -2, 3, 2, 0, body);
    m.slab(-4, -2, 3, -2, 0, track);
    m.slab(-4, 2, 3, 2, 0, track);
    m.slab(-2, -1, 0, 1, 1, body.lighter(0.08));
    for x in 1..=4 {
        m.cube(x, 0, 1, STEEL);
    }
    m
}

/// A pillbox: Bolo's tan pyramid, the gun on top in orange. Health
/// takes the top off: under half, no gun; dead, a flat dark slab.
pub fn pillbox(owner: Option<Color>, health: f32, alive: bool) -> Model {
    let mut m = Model::default();
    if !alive {
        m.slab(-2, -2, 2, 2, 0, Color::srgb(0.25, 0.22, 0.2));
        return m;
    }
    let base = owner.map(|c| c.mix(&TAN, 0.55)).unwrap_or(TAN);
    m.slab(-2, -2, 2, 2, 0, base);
    m.slab(-1, -1, 1, 1, 1, base.lighter(0.05));
    if health > 0.5 {
        m.cube(0, 0, 2, GUN);
    } else {
        m.cube(0, 0, 2, TAN_DARK);
    }
    m
}

/// A depot, Bolo's base: a plate with the owner's mark in the middle.
pub fn depot(owner: Option<Color>) -> Model {
    let mut m = Model::default();
    m.slab(-3, -3, 3, 3, 0, TAN_DARK);
    m.slab(-2, -2, 2, 2, 0, TAN);
    let mark = owner.unwrap_or(Color::srgb(0.5, 0.5, 0.45));
    m.slab(-1, -1, 1, 1, 1, mark);
    m
}

/// The recon: a cross that looks, in the owner's colour.
pub fn recon(color: Color) -> Model {
    let mut m = Model::default();
    m.cube(0, 0, 0, color);
    for d in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        m.cube(d.0, d.1, 0, color.lighter(0.1));
    }
    m
}

/// The interceptor: a small block, the lighter one.
pub fn interceptor(color: Color) -> Model {
    let mut m = Model::default();
    m.slab(0, 0, 1, 1, 0, color.lighter(0.15));
    m
}

/// The strike: a dart along +x; the swarm's in red.
pub fn strike(color: Color) -> Model {
    let mut m = Model::default();
    for x in -1..=1 {
        m.cube(x, 0, 0, color);
    }
    m.cube(2, 0, 0, color.lighter(0.2));
    m
}

pub fn shell(color: Color) -> Model {
    let mut m = Model::default();
    m.cube(0, 0, 1, color);
    m
}

pub fn mine(color: Color) -> Model {
    let mut m = Model::default();
    m.cube(0, 0, 0, color);
    m
}

/// The squad: two men, in the owner's colour.
pub fn squad(color: Color) -> Model {
    let mut m = Model::default();
    m.cube(0, 0, 0, color);
    m.cube(1, 1, 0, color);
    m.cube(0, 0, 1, color.lighter(0.1));
    m.cube(1, 1, 1, color.lighter(0.1));
    m
}

/// A wall tile: a tan block the size of the tile, one level high.
pub fn wall() -> Model {
    let mut m = Model::default();
    m.slab(-2, -2, 2, 2, 0, TAN);
    m.slab(-2, -2, 2, 2, 1, TAN.lighter(0.03));
    m
}
