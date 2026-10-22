//! What the map says, worked out once (roadmap steps 12b and 12f2,
//! LAB-REPORT-BRAIN.md sections 5 and 6.2). The islands: every tile that
//! is not water labelled with its land mass, each mass with its area and
//! its forest; refreshed every `REFRESH`, since trees fall and bridges
//! join masses. The crossings: `Map::crossing` is a Dijkstra over 65,536
//! states, and a failed one (a gap over `BRIDGE_MAX`, or no way) was
//! asked again four times a second while the bot waited at the shore;
//! here each answer, failures too, is kept for `CROSSING_FOR`.
//!
//! And the prices that use them: a base site on a small island with few
//! trees scores low, and a crossing is bridged or sailed by what each
//! costs in material, a bridge past `bridge_soft` tiles costing the
//! square of its extra length, every tree dearer where trees are scarce.

use crate::weights::{Settings, W};
use treadcore_sim::*;

/// Islands are labelled again this often.
pub const REFRESH: Tick = 10 * TICK_HZ as Tick;
/// A crossing's answer is kept this long.
pub const CROSSING_FOR: Tick = 60 * TICK_HZ as Tick;

/// A land mass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Island {
    pub area: u32,
    pub forest: u32,
}

#[derive(Clone, Debug, Default)]
pub struct MapFacts {
    /// Island per tile, row-major; 0 is water.
    label: Vec<u16>,
    /// Island k is `islands[k - 1]`.
    islands: Vec<Island>,
    labelled: Option<Tick>,
    crossings: Vec<(((i32, i32), (i32, i32)), Option<Vec<V2>>, Tick)>,
}

impl MapFacts {
    /// Label the islands if they are older than `REFRESH`.
    pub fn refresh(&mut self, map: &Map, now: Tick) {
        if self.labelled.map(|t| now < t + REFRESH).unwrap_or(false) && !self.label.is_empty() {
            return;
        }
        self.labelled = Some(now);
        self.label.clear();
        self.label.resize(MAP_W * MAP_H, 0);
        self.islands.clear();
        let mut stack: Vec<(usize, usize)> = Vec::new();
        for y0 in 0..MAP_H {
            for x0 in 0..MAP_W {
                if self.label[y0 * MAP_W + x0] != 0 || map.at_tile(x0, y0) == Terrain::Water {
                    continue;
                }
                if self.islands.len() >= u16::MAX as usize - 1 {
                    return;
                }
                let id = self.islands.len() as u16 + 1;
                let mut isl = Island::default();
                self.label[y0 * MAP_W + x0] = id;
                stack.push((x0, y0));
                while let Some((x, y)) = stack.pop() {
                    isl.area += 1;
                    if map.at_tile(x, y) == Terrain::Forest {
                        isl.forest += 1;
                    }
                    let n = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
                    for (nx, ny) in n {
                        if nx < MAP_W && ny < MAP_H && self.label[ny * MAP_W + nx] == 0 && map.at_tile(nx, ny) != Terrain::Water {
                            self.label[ny * MAP_W + nx] = id;
                            stack.push((nx, ny));
                        }
                    }
                }
                self.islands.push(isl);
            }
        }
    }

    /// The island under a point, if it is land and labelled.
    pub fn island_at(&self, p: V2) -> Option<(u16, Island)> {
        let (x, y) = p.tile();
        if x < 0 || y < 0 || x as usize >= MAP_W || y as usize >= MAP_H || self.label.is_empty() {
            return None;
        }
        let id = self.label[y as usize * MAP_W + x as usize];
        (id > 0).then(|| (id, self.islands[id as usize - 1]))
    }

    /// Every island of the map, in id order (an id is its index and 1).
    pub fn islands(&self) -> &[Island] {
        &self.islands
    }

    /// `Map::crossing`, its answers kept for `CROSSING_FOR`, the failures
    /// with them.
    pub fn crossing(&mut self, map: &Map, a: V2, b: V2, max_water: usize, forbidden: impl Fn(V2) -> bool, now: Tick) -> Option<Vec<V2>> {
        let key = (a.tile(), b.tile());
        self.crossings.retain(|(_, _, t)| now < t + CROSSING_FOR);
        if let Some((_, ans, _)) = self.crossings.iter().find(|(k, _, _)| *k == key) {
            crate::meter::crossing_cached();
            return ans.clone();
        }
        crate::meter::crossing();
        let ans = map.crossing(a, b, max_water, forbidden);
        self.crossings.push((key, ans.clone(), now));
        ans
    }

    /// Forget the crossings: the map changed under them (a bridge laid).
    pub fn forget_crossings(&mut self) {
        self.crossings.clear();
    }

    /// Trees are scarce here: what a base will ask for over what the
    /// island holds (LR 6.2's s).
    pub fn scarcity(&self, s: &Settings, at: V2) -> f32 {
        let forest = self.island_at(at).map(|(_, i)| i.forest).unwrap_or(0) as f32;
        s.w[W::ForestNeed].max(0.0) / (forest + 1.0)
    }

    /// A base site's worth (LR 6.2): enough trees, less a small island.
    pub fn site_score(&self, s: &Settings, at: V2) -> f32 {
        let Some((_, isl)) = self.island_at(at) else { return f32::MIN };
        let need = s.w[W::ForestNeed].max(1.0);
        let area = s.w[W::SiteArea].max(1.0);
        s.w[W::SiteForest] * (isl.forest as f32 / need).min(1.0) - s.w[W::SiteSmall] * (1.0 - isl.area as f32 / area).max(0.0)
    }
}

/// What a crossing of `len` water tiles costs as a bridge and as a boat,
/// with trees `scarcity` dear (LR 6.2).
pub fn crossing_prices(s: &Settings, len: usize, scarcity: f32) -> (f32, f32) {
    let w = &s.w;
    let dear = 1.0 + scarcity.max(0.0);
    let long = (len as f32 - w[W::BridgeSoft]).max(0.0);
    let bridge = len as f32 * dear + w[W::LongBridge] * long * long;
    let boat = w[W::BoatCost] * dear + w[W::Launch];
    (bridge, boat)
}

/// Bridge it (true) or sail it (false).
pub fn bridge_it(s: &Settings, len: usize, scarcity: f32) -> bool {
    let (bridge, boat) = crossing_prices(s, len, scarcity);
    bridge <= boat
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A map of water with two islands: a big forested one west, a
    /// small bare one east.
    fn island_map() -> Map {
        // Water everywhere; a big island at the west with forest, a
        // small bare one at the east.
        let mut m = Map::generate(1);
        for y in 0..MAP_H {
            for x in 0..MAP_W {
                m.set(Map::tile_centre(V2::new(x as f32, y as f32)), Terrain::Water);
            }
        }
        for y in 10..40 {
            for x in 10..40 {
                let t = if (x + y) % 3 == 0 { Terrain::Forest } else { Terrain::Grass };
                m.set(Map::tile_centre(V2::new(x as f32, y as f32)), t);
            }
        }
        for y in 20..24 {
            for x in 60..64 {
                m.set(Map::tile_centre(V2::new(x as f32, y as f32)), Terrain::Grass);
            }
        }
        m
    }

    #[test]
    fn the_islands_are_counted() {
        let m = island_map();
        let mut f = MapFacts::default();
        f.refresh(&m, 0);
        assert_eq!(f.islands().len(), 2);
        let (_, big) = f.island_at(V2::new(20.5, 20.5)).unwrap();
        let (_, small) = f.island_at(V2::new(61.5, 21.5)).unwrap();
        assert_eq!((big.area, small.area, small.forest), (900, 16, 0));
        assert!(big.forest == 300);
        assert!(f.island_at(V2::new(50.5, 50.5)).is_none());
        let s = Settings::default();
        assert!(f.site_score(&s, V2::new(20.5, 20.5)) > f.site_score(&s, V2::new(61.5, 21.5)), "a small bare island is a poor base");
    }

    #[test]
    fn a_small_island_takes_a_boat() {
        let s = Settings::default();
        // Trees plenty: a short bridge, and up to seven tiles; a long one is sailed.
        assert!(bridge_it(&s, 3, 0.04));
        assert!(bridge_it(&s, 6, 0.04));
        assert!(!bridge_it(&s, 12, 0.04));
        assert!(!bridge_it(&s, 20, 0.0));
        // On a bare island (scarcity 12) even six tiles go by boat.
        assert!(!bridge_it(&s, 6, 12.0));
        // The crossing cache answers the second time, a failure too.
        let m = island_map();
        let mut f = MapFacts::default();
        crate::meter::reset();
        let a = f.crossing(&m, V2::new(39.5, 21.5), V2::new(60.5, 21.5), 4, |_| false, 0);
        let b = f.crossing(&m, V2::new(39.5, 21.5), V2::new(60.5, 21.5), 4, |_| false, 20);
        assert!(a.is_none() && b.is_none());
        let meter = crate::meter::read();
        assert_eq!((meter.crossings, meter.crossings_cached), (1, 1));
        assert!(f.crossing(&m, V2::new(39.5, 21.5), V2::new(60.5, 21.5), 4, |_| false, CROSSING_FOR + 1).is_none());
        assert_eq!(crate::meter::read().crossings, 2, "an old answer is asked again");
    }
}
