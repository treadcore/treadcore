//! The numbers. Every tuning constant in one place, with the reason it is
//! what it is, so a balance change is a one-line diff with a sentence
//! beside it and not an archaeology project.
//!
//! Distances are tiles (25 m). Times are seconds unless the name says ticks.

use crate::{Bay, PlayerId, TICK_HZ};

// ---- the tank ----------------------------------------------------------
pub const TANK_ARMOR: f32 = 100.0;
pub const TANK_SHELLS: u16 = 24;
pub const TANK_FUEL: f32 = 600.0;
/// Tiles per second on open ground: 4 tiles/s is 100 m/s... no: 4 tiles is
/// 100 m, so this is a brisk 360 km/h. A game speed. Real tanks at 40 km/h
/// would make a 3 km map a seven-minute drive.
pub const TANK_SPEED: f32 = 4.0;
pub const TANK_ACCEL: f32 = 3.0;
/// Radians per second of hull turn.
pub const TANK_TURN: f32 = 2.5;
/// A heading error larger than this is a corner: the tank brakes to
/// `TURN_CRAWL` of its speed and pivots, instead of swinging wide of the
/// road (and, at a bridge, into the river).
pub const TURN_BRAKE_ANGLE: f32 = 0.6;
pub const TURN_CRAWL: f32 = 0.25;

pub const FUEL_PER_TILE: f32 = 1.0;
/// Own eyes: not far. The tank sees the drones' picture, not the ground.
pub const TANK_VISION: f32 = 7.0;

// ---- the gun -----------------------------------------------------------
pub const RELOAD_TICKS: u16 = 4 * TICK_HZ as u16;
/// Indirect: lobbed. 12 to 60 tiles is 300 m to 1.5 km, which on a 3.2 km
/// map means a tank in one corner can reach the centre and the two
/// neighbouring corners, but not the far one without moving.
pub const INDIRECT_MIN_RANGE: f32 = 12.0;
pub const INDIRECT_MAX_RANGE: f32 = 60.0;
/// Tiles per second along the ground track. A 60-tile shot is in the air
/// six seconds -- long enough to move if you see the launch, which you do
/// not unless a recon drone is watching the gun.
pub const SHELL_SPEED_INDIRECT: f32 = 10.0;
/// Circular error, in tiles, blind and spotted. The whole reason to fly
/// recon: a blind shot is a suppression round, a spotted one is a kill.
pub const CEP_BLIND: f32 = 4.0;
pub const CEP_SPOTTED: f32 = 0.6;
pub const SPLASH_RADIUS: f32 = 2.5;
pub const INDIRECT_DAMAGE: f32 = 70.0;
/// Direct: flat, fast, line of sight, hull-to-hull. The old job, still
/// there for when something drives into view.
pub const DIRECT_RANGE: f32 = 10.0;
pub const SHELL_SPEED_DIRECT: f32 = 40.0;
pub const DIRECT_DAMAGE: f32 = 45.0;
pub const DIRECT_HIT_RADIUS: f32 = 0.8;

// ---- the drone crew ----------------------------------------------------
pub const BAY_FULL: Bay = Bay { recon: 2, interceptor: 6, strike: 3 };
/// Per player, in the air at once. Keeps the host's per-tick work bounded
/// and makes the bay a budget rather than a stream.
pub const MAX_AIRBORNE: usize = 10;
pub const RECON_BATTERY: f32 = 150.0;
pub const INTERCEPTOR_BATTERY: f32 = 90.0;
pub const STRIKE_BATTERY: f32 = 45.0;
pub const RECON_SPEED: f32 = 5.0;
pub const INTERCEPTOR_SPEED: f32 = 9.0;
pub const STRIKE_SPEED: f32 = 6.5;
pub const RECON_VISION: f32 = 12.0;
pub const DRONE_VISION: f32 = 4.0;
/// Telemetry link. Link quality is 1 at the tank and 0 at this range; below
/// `LINK_FLOOR` the drone is on its own and turns for home.
pub const RECON_LINK: f32 = 40.0;
pub const DRONE_LINK: f32 = 18.0;
pub const LINK_FLOOR: f32 = 0.12;
/// The bubble: where interceptors hold station. Detection reaches a little
/// past it so an interceptor is already moving when the intruder arrives.
pub const BUBBLE_DEFAULT: f32 = 5.0;
pub const BUBBLE_MIN: f32 = 2.0;
pub const BUBBLE_MAX: f32 = 12.0;
pub const DETECT_MARGIN: f32 = 4.0;
/// Orbit angular rate scale: radians/s at radius 1, divided by radius.
pub const ORBIT_RATE: f32 = 6.0;
pub const RAM_RANGE: f32 = 0.5;
pub const RECOVER_RANGE: f32 = 0.8;
/// Kill probability for a ram: on a dead link the interceptor's own camera
/// gets it right half the time; with the tank's radar feeding it, most of
/// the time.
pub const INTERCEPT_BASE: f32 = 0.5;
pub const INTERCEPT_LINKED: f32 = 0.9;
pub const STRIKE_DAMAGE: f32 = 35.0;

// ---- pillboys and pillboxes --------------------------------------------
/// The squad: the tank's tech infantry. Four is a squad; zero is a tank
/// that cannot capture, repair or build until a depot replaces them.
pub const PILLBOY_MAX: u8 = 4;
/// Tiles per second on foot, by the world's route. A tank does 4; the
/// squad is the slow part of every order, which is what makes an order
/// a decision.
pub const PILLBOY_SPEED: f32 = 1.5;
/// Material the tank carries for the squad: pillbox repairs and new
/// emplacements. Restocked at depots; harvested from forest later (6c).
pub const PILLBOY_MATERIAL_MAX: u8 = 4;
pub const PILLBOY_MATERIAL_START: u8 = 2;
/// A shell's splash over a deployed squad costs it this many Pillboys.
pub const PILLBOY_SPLASH_LOSS: u8 = 2;
/// Within this of the tank the squad is back aboard.
pub const PILLBOY_BOARD_RANGE: f32 = 1.0;
/// Within this of the site the squad is at work.
pub const PILLBOY_WORK_RANGE: f32 = 0.7;
/// Ticks of uncontested work: capture (Bolo's "stays on it"), a repair,
/// a new emplacement.
pub const PILLBOX_CAPTURE_TICKS: u16 = 10 * TICK_HZ as u16;
pub const PILLBOX_REPAIR_TICKS: u16 = 6 * TICK_HZ as u16;
pub const PILLBOX_BUILD_TICKS: u16 = 15 * TICK_HZ as u16;
pub const PILLBOX_REPAIR_COST: u8 = 1;
pub const PILLBOX_BUILD_COST: u8 = 2;
/// A pillbox: health, and a gun. Sixty health is one indirect hit at the
/// centre of the splash and a bit, or two direct hits, or four of its
/// own kind of shot.
pub const PILLBOX_HEALTH: f32 = 60.0;
/// Against tanks: hitscan at anything hostile with a clear line inside
/// this, for this much, every reload. Eight tiles is inside the tank's
/// direct range (10) and far inside indirect (12 to 60): a pillbox is
/// out-ranged by a gun that can see it and deadly to one that drives up.
pub const PILLBOX_RANGE: f32 = 8.0;
pub const PILLBOX_DAMAGE: f32 = 15.0;
pub const PILLBOX_RELOAD_TICKS: u16 = 2 * TICK_HZ as u16;
/// Against drones: the one addition to Bolo's pillbox. Short, and with
/// an interceptor's kill curve: a link to the owner tank's radar if it is
/// within `DRONE_LINK`, else the pillbox's own sensor at `INTERCEPT_BASE`.
pub const PILLBOX_AIR_RANGE: f32 = 4.0;
/// A captured pillbox is handed over at least this well repaired.
pub const PILLBOX_CAPTURE_HEALTH: f32 = 30.0;
/// No two pillboxes closer than this; none on a depot.
pub const PILLBOX_SPACING: f32 = 3.0;
/// Extra route cost per tile inside a live hostile pillbox's range: a
/// detour of five grass tiles beats one tile under the gun.
pub const PILLBOX_AVOID_COST: f32 = 5.0;

/// The owner of a neutral pillbox's shots: nobody, and never a seat.
pub const NEUTRAL: PlayerId = 254;

// ---- depots ------------------------------------------------------------

pub const DEPOT_RADIUS: f32 = 2.0;
pub const CAPTURE_TICKS: u16 = 10 * TICK_HZ as u16;
pub const RESUPPLY_EVERY: u64 = TICK_HZ as u64 / 2;
pub const REPAIR_PER_TICK: f32 = 4.0;

// ---- scoring -----------------------------------------------------------
pub const SCORE_KILL: u32 = 100;
pub const SCORE_CAPTURE: u32 = 25;
pub const SCORE_INTERCEPT: u32 = 5;

/// The neutral swarm's player id. Never a seat; never scores.
pub const SWARM_OWNER: PlayerId = 255;

pub fn drone_speed(k: crate::DroneKind) -> f32 {
    match k {
        crate::DroneKind::Recon => RECON_SPEED,
        crate::DroneKind::Interceptor => INTERCEPTOR_SPEED,
        crate::DroneKind::Strike => STRIKE_SPEED,
    }
}
