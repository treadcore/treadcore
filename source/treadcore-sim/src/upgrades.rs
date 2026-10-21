//! Upgrades, and supply to buy them: a prototype of
//! documentation/PROPOSAL-UPGRADES.md (roadmap 11g), on the branch
//! `proposal/upgrades`, for the proposal's trace 2 (bots buying).
//!
//! Off unless the scenario asks (`Scenario::upgrades`). The economy is
//! kept beside the world, not in it (`serde(skip)`): not in the hash,
//! the replay or the wire, so a match without upgrades is today's
//! number for number, and a match with them is checked only on the
//! machine that ran it. The proposal's build puts it in the world, on
//! wire 17; this one answers whether it is worth that.
//!
//! Four of the proposal's sixteen, those bought and applied at once
//! with nothing to install: pillbox calibre and plating and depot
//! stores at a base, plating on a tank. Supply is earned as section
//! 1.1 says, at `SUPPLY_SCALE` of its rates (section 7's halving is
//! the scale at 0.5).

use crate::{EntityId, TeamId, PILLBOX_HEALTH, PILLBOX_RANGE, TANK_ARMOR};
use serde::{Deserialize, Serialize};

/// The share of section 1.1's earning rates paid by default (1.0 the
/// proposal's; `Economy::scale` 0.5 is section 7's suggestion). A kill's
/// bounty is scaled too.
pub const SUPPLY_SCALE: f32 = 1.0;
/// A base upgrade applies within this of its depot (the brain's
/// `BASE_R`, the castle's ring).
pub const UPGRADE_BASE_R: f32 = 10.0;
/// A base upgrade is bought within this of its depot; a tank's on a
/// depot of the side (`DEPOT_RADIUS`).
pub const UPGRADE_BUY_REACH: f32 = crate::DEPOT_RADIUS + 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upgrade {
    /// The base's pillboxes reach 9, then 10 tiles.
    Calibre,
    /// The base's pillboxes 75, then 90 health.
    PillboxPlating,
    /// The depot restocks twice as fast.
    Stores,
    /// The tank's armour 110, then 120; lost with the tank.
    TankPlating,
}

impl Upgrade {
    pub const ALL: [Upgrade; 4] = [Upgrade::Calibre, Upgrade::PillboxPlating, Upgrade::Stores, Upgrade::TankPlating];

    /// Bought at a depot for its base (true), or for the buyer's tank.
    pub fn base(self) -> bool {
        self != Upgrade::TankPlating
    }

    pub fn levels(self) -> u8 {
        match self {
            Upgrade::Stores => 1,
            _ => 2,
        }
    }

    /// The supply for `level` (1 the first).
    pub fn price(self, level: u8) -> u32 {
        match (self, level) {
            (Upgrade::TankPlating, 1) => 50,
            (Upgrade::TankPlating, _) => 100,
            (_, 1) => 60,
            _ => 120,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Upgrade::Calibre => "calibre",
            Upgrade::PillboxPlating => "pillbox plating",
            Upgrade::Stores => "depot stores",
            Upgrade::TankPlating => "tank plating",
        }
    }
}

/// A pillbox's reach at calibre `level`.
pub fn pillbox_range(level: u8) -> f32 {
    PILLBOX_RANGE + level as f32
}

/// A pillbox's full health at plating `level`.
pub fn pillbox_health(level: u8) -> f32 {
    PILLBOX_HEALTH + 15.0 * level as f32
}

/// A tank's full armour at plating `level`.
pub fn tank_armor(level: u8) -> f32 {
    TANK_ARMOR + 10.0 * level as f32
}

/// What each side has earned and bought.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Economy {
    /// Supply held, by team; fractions kept so a scaled rate adds up.
    pub supply: Vec<f32>,
    /// Earned in all, by team and source: pillboxes, depots,
    /// structures, kills, captures.
    pub earned: Vec<[f32; 5]>,
    /// Base upgrades: the depot, the upgrade, its level.
    pub bases: Vec<(EntityId, Upgrade, u8)>,
    /// Tank upgrades: the tank, the upgrade, its level.
    pub tanks: Vec<(EntityId, Upgrade, u8)>,
    /// Every purchase: tick, team, upgrade, level.
    pub bought: Vec<(u64, TeamId, Upgrade, u8)>,
    /// The share of the earning rates paid (`SUPPLY_SCALE` at first).
    pub scale: f32,
}

impl Economy {
    pub fn new(teams: usize) -> Economy {
        Economy { supply: vec![0.0; teams], earned: vec![[0.0; 5]; teams], scale: SUPPLY_SCALE, ..Economy::default() }
    }

    /// A depot's level of a base upgrade.
    pub fn base_level(&self, depot: EntityId, u: Upgrade) -> u8 {
        self.bases.iter().find(|(d, x, _)| *d == depot && *x == u).map(|b| b.2).unwrap_or(0)
    }

    /// A tank's level of a tank upgrade.
    pub fn tank_level(&self, tank: EntityId, u: Upgrade) -> u8 {
        self.tanks.iter().find(|(t, x, _)| *t == tank && *x == u).map(|b| b.2).unwrap_or(0)
    }

    /// Pays `team` from `source` (0 pillboxes, 1 depots, 2 structures,
    /// 3 kills, 4 captures), at `scale`.
    pub fn earn(&mut self, team: TeamId, source: usize, amount: f32) {
        let t = team as usize;
        if t < self.supply.len() {
            let a = amount * self.scale;
            self.supply[t] += a;
            self.earned[t][source] += a;
        }
    }

    /// Sets a level, a base's (`base`) or a tank's.
    pub fn set(&mut self, base: bool, id: EntityId, u: Upgrade, level: u8) {
        let list = if base { &mut self.bases } else { &mut self.tanks };
        match list.iter_mut().find(|(i, x, _)| *i == id && *x == u) {
            Some(e) => e.2 = level,
            None => list.push((id, u, level)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_and_prices_follow_the_proposal() {
        assert_eq!(pillbox_range(0), PILLBOX_RANGE);
        assert_eq!(pillbox_range(2), PILLBOX_RANGE + 2.0);
        assert_eq!(pillbox_health(1), 75.0);
        assert_eq!(tank_armor(2), 120.0);
        assert_eq!(Upgrade::Calibre.price(2), 120);
        assert_eq!(Upgrade::TankPlating.price(1), 50);
        assert_eq!(Upgrade::Stores.levels(), 1);
        let mut e = Economy::new(2);
        e.set(true, 7, Upgrade::Calibre, 1);
        e.set(true, 7, Upgrade::Calibre, 2);
        assert_eq!((e.base_level(7, Upgrade::Calibre), e.base_level(8, Upgrade::Calibre), e.bases.len()), (2, 0, 1));
        e.earn(1, 1, 2.0);
        assert_eq!(e.supply, vec![0.0, 2.0 * SUPPLY_SCALE]);
    }
}
