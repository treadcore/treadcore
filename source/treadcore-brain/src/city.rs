//! The city (game 7; the plan of 2 November, chapter 13): what a base's
//! quartermaster builds next, in the order a base grows, read from what
//! stands near it. A camp (the depot held, two pillboxes, the ring road)
//! is the castle plan's and the base cycle's; this module is the town,
//! the works and the city on top of them, as far as `city_plan` allows.
//! The guard is the quartermaster: quiet at its post, it does the next
//! item here before it rotates.

use crate::weights::{Settings, W};
use treadcore_sim::*;

/// How far from the base the city reaches for its works and its wood.
pub const CITY_R: f32 = 20.0;
/// How far a works of the side counts as the base's own.
pub const CITY_HAS: f32 = 40.0;

/// The next thing the city wants.
#[derive(Clone, Debug, PartialEq)]
pub enum CityItem {
    /// An order for the squad at a site the tank drives near first, and the words for it.
    Order(PillboyTask, V2, &'static str),
    /// Supply spent at a depot of the base.
    Buy(Upgrade, EntityId, &'static str),
}

impl CityItem {
    pub fn word(&self) -> &'static str {
        match self {
            CityItem::Order(_, _, w) | CityItem::Buy(_, _, w) => w,
        }
    }
}

/// The open tile (grass, rubble or road, nothing on it) within `r` of
/// `near`, nearest the base.
fn open_beside(obs: &Observation, map: &Map, near: V2, base: V2, r: f32) -> Option<V2> {
    let taken = |p: V2| {
        obs.structures.iter().any(|x| x.pos.tile() == p.tile())
            || obs.pillboxes.iter().any(|x| x.pos.tile() == p.tile())
            || obs.deposits.iter().any(|d| d.pos.tile() == p.tile())
            || obs.wells.iter().any(|d| d.pos.tile() == p.tile())
            || obs.depots.iter().any(|d| d.pos.dist(p) <= DEPOT_RADIUS)
    };
    let k = r.ceil() as i32;
    (-k..=k)
        .flat_map(|dy| (-k..=k).map(move |dx| (dx, dy)))
        .map(|(dx, dy)| Map::tile_centre(near + V2::new(dx as f32, dy as f32)))
        .filter(|p| p.dist(near) <= r && map.in_bounds(*p) && matches!(map.at(*p), Terrain::Grass | Terrain::Rubble | Terrain::Road) && !taken(*p))
        .min_by(|a, b| a.dist(base).total_cmp(&b.dist(base)))
}

/// A free pad within `CITY_R` of the base that a power plant of the
/// side covers, nearest the base.
fn powered_pad(obs: &Observation, map: &Map, base: V2) -> Option<V2> {
    let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
    let plants: Vec<V2> = obs.structures.iter().filter(|x| x.kind == StructureKind::Power && x.alive() && ours(x.owner)).map(|x| x.pos).collect();
    map.terrain_near(Terrain::Pad, base, CITY_R, 32)
        .into_iter()
        .filter(|p| !obs.structures.iter().any(|x| x.pos.tile() == p.tile()))
        .filter(|p| plants.iter().any(|q| q.dist(*p) <= POWER_RANGE))
        .min_by(|a, b| a.dist(base).total_cmp(&b.dist(base)))
}

/// The next item the city wants, or None: as far as `city_plan` allows
/// (none, camp, town, works, city), from what stands. The smelter is
/// the ore run's to build; the power plant and the workshop by the base
/// are the cycle's; the roads between bases the cycle's Connect.
pub fn next(obs: &Observation, me: &Tank, base: V2, map: Option<&Map>, s: &Settings) -> Option<CityItem> {
    let stage = s.choice(W::CityPlan);
    if stage < 2 {
        return None;
    }
    let ours = |o: Option<PlayerId>| o.map(|o| obs.allied(o)).unwrap_or(false);
    let side = |k: StructureKind, within: f32| obs.structures.iter().any(|x| x.kind == k && x.alive() && ours(x.owner) && x.pos.dist(base) <= within);
    let material = me.pillboys.material;
    // THE TOWN: a harvester by the wood.
    if let Some(m) = map {
        if !side(StructureKind::Harvester, CITY_HAS) && material >= StructureKind::Harvester.cost() {
            if let Some(tree) = m.forest_near(base, CITY_R, 1).first().copied() {
                if let Some(at) = open_beside(obs, m, tree, base, HARVESTER_SITE) {
                    return Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Structure(StructureKind::Harvester), at }, at, "the town: a harvester by the wood"));
                }
            }
        }
    }
    if stage < 3 {
        return None;
    }
    // THE WORKS: the depot's stores, the road to the smelter, a refinery where there is oil.
    if s.w[W::UpStores] > 0.0 && obs.supply >= Upgrade::DepotStores.cost() {
        if let Some(d) = obs.depots.iter().filter(|d| ours(d.owner) && !d.stores && d.pos.dist(base) <= CITY_R).min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base))) {
            return Some(CityItem::Buy(Upgrade::DepotStores, d.id, "the works: the depot's stores"));
        }
    }
    if let Some(m) = map {
        if material >= BuildKind::Road.cost() {
            if let Some(sm) = obs.structures.iter().filter(|x| x.kind == StructureKind::Smelter && x.alive() && ours(x.owner) && x.pos.dist(base) <= CITY_HAS).min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base))) {
                if let Some(tile) = crate::cycle::Cycle::road_to(m, base, sm.pos, 1).first().copied() {
                    return Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Road, at: tile }, tile, "the works: the road to the smelter"));
                }
            }
        }
    }
    // The industrial zone taken: a structure of nobody's, or a wreck, within reach, the power plant
    // first (nothing else of the zone works without it), then the refinery, the workshop, the rest.
    for kind in [StructureKind::Power, StructureKind::Refinery, StructureKind::Workshop, StructureKind::DataCentre, StructureKind::Jammer] {
        if let Some(x) = obs.structures.iter().filter(|x| x.kind == kind && (x.owner.is_none() || !x.alive()) && !ours(x.owner) && x.pos.dist(base) <= CITY_HAS).min_by(|a, b| a.pos.dist(base).total_cmp(&b.pos.dist(base))) {
            let word = match kind {
                StructureKind::Power => "the works: the power plant taken",
                StructureKind::Refinery => "the works: the refinery taken",
                StructureKind::Workshop => "the works: the workshop taken",
                _ => "the works: the zone's structure taken",
            };
            return Some(CityItem::Order(PillboyTask::Capture { pillbox: x.id }, x.pos, word));
        }
    }
    if !obs.wells.is_empty() && !side(StructureKind::Refinery, CITY_HAS) {
        // None to take: built on a pad a plant covers.
        if let Some(m) = map {
            if material >= StructureKind::Refinery.cost() {
                if let Some(pad) = powered_pad(obs, m, base) {
                    return Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Structure(StructureKind::Refinery), at: pad }, pad, "the works: a refinery on the pad"));
                }
            }
        }
    }
    if stage < 4 {
        return None;
    }
    // THE CITY: a data centre and a workshop on the pads.
    if let Some(m) = map {
        for (kind, word) in [(StructureKind::DataCentre, "the city: a data centre"), (StructureKind::Workshop, "the city: a workshop")] {
            if !side(kind, CITY_HAS) && material >= kind.cost() {
                if let Some(pad) = powered_pad(obs, m, base) {
                    return Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Structure(kind), at: pad }, pad, word));
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The town wants a harvester by the wood; the works, once a smelter stands, the road to it and
    /// the stores when the supply is there; at `city_plan` none, nothing.
    #[test]
    fn the_city_grows_in_order() {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(1) });
        w.pillboxes.clear();
        let base = Map::tile_centre(w.tanks[0].pos);
        for dy in -10..=10 {
            for dx in -10..=10 {
                w.map.set(Map::tile_centre(base + V2::new(dx as f32, dy as f32)), Terrain::Grass);
            }
        }
        w.map.set(Map::tile_centre(base + V2::new(6.0, 0.0)), Terrain::Forest);
        w.tanks[0].pillboys.material = PILLBOY_MATERIAL_MAX;
        let mut s = Settings::pinned();
        let obs = w.observe(0);
        let me = obs.me.clone().unwrap();
        s.w[W::CityPlan] = 0.0;
        assert!(next(&obs, &me, base, Some(&w.map), &s).is_none());
        s.w[W::CityPlan] = 2.0;
        match next(&obs, &me, base, Some(&w.map), &s) {
            Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Structure(StructureKind::Harvester), at }, _, _)) => assert!(at.dist(base + V2::new(6.0, 0.0)) <= HARVESTER_SITE + 0.8, "{:?}", at),
            other => panic!("{:?}", other),
        }
        // The harvester stands: the town is done; the works want the road to a smelter.
        w.structures.push(Structure { id: 9501, kind: StructureKind::Harvester, pos: Map::tile_centre(base + V2::new(5.0, 1.0)), owner: Some(0), health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: true, ore: 0, shells: 0, oil: 0 });
        w.structures.push(Structure { id: 9502, kind: StructureKind::Smelter, pos: Map::tile_centre(base + V2::new(-9.0, 0.0)), owner: Some(0), health: STRUCTURE_HEALTH, capture: 0, capturer: None, powered: true, ore: 0, shells: 0, oil: 0 });
        w.wells.clear();
        let obs = w.observe(0);
        assert!(next(&obs, &me, base, Some(&w.map), &s).is_none(), "a town wants no works");
        s.w[W::CityPlan] = 3.0;
        match next(&obs, &me, base, Some(&w.map), &s) {
            Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Road, at }, _, _)) => assert!((at.y - base.y).abs() < 1.0 && at.x < base.x, "the road heads for the smelter: {:?}", at),
            other => panic!("{:?}", other),
        }
        // Supply enough: the stores come first.
        w.players[0].supply = STORES_COST;
        let obs = w.observe(0);
        assert!(matches!(next(&obs, &me, base, Some(&w.map), &s), Some(CityItem::Buy(Upgrade::DepotStores, _, _))));
        s.w[W::UpStores] = 0.0;
        assert!(matches!(next(&obs, &me, base, Some(&w.map), &s), Some(CityItem::Order(PillboyTask::Build { what: BuildKind::Road, .. }, _, _))));
    }
}
