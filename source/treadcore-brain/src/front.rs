//! Battle lines (Paul, 30 October: "We would need to have the concept of battle lines"). A line
//! is where the nearest ground of ours is as far as the nearest ground of theirs, ground being a
//! depot or a live pillbox held by a side. It moves as they change hands, and nothing stores it:
//! it is read from what a seat can already see.

use treadcore_sim::*;

/// Inside this of a hostile depot is inside the enemy's base.
pub const ENEMY_BASE_R: f32 = 16.0;
/// Mines stand this far back of the line, on our side of it.
pub const LINE_BACK: f32 = 3.0;
/// A squad is sent no farther than this from the base: past it, a far line's mines are laid
/// short, since a squad that long on the road does not come home.
pub const LINE_REACH: f32 = 30.0;
/// Tiles between mines along the line.
pub const LINE_SPACING: f32 = 2.0;

fn nearest(p: V2, ground: impl Iterator<Item = V2>) -> Option<f32> {
    ground.map(|g| g.dist(p)).min_by(f32::total_cmp)
}

/// How far `p` stands on our side of the line, in tiles: negative is their side. None until
/// both sides hold something.
pub fn side(obs: &Observation, p: V2) -> Option<f32> {
    let theirs = |o: PlayerId| !obs.allied(o);
    let ours = obs.depots.iter().filter(|d| d.owner.is_some_and(|o| obs.allied(o))).map(|d| d.pos)
        .chain(obs.pillboxes.iter().filter(|b| b.alive() && b.owner.is_some_and(|o| obs.allied(o))).map(|b| b.pos));
    let them = obs.depots.iter().filter(|d| d.owner.is_some_and(theirs)).map(|d| d.pos)
        .chain(obs.pillboxes.iter().filter(|b| b.alive() && b.owner.is_some_and(theirs)).map(|b| b.pos));
    Some(nearest(p, them)? - nearest(p, ours)?)
}

/// Whether `p` is inside a hostile depot's base.
pub fn in_enemy_base(obs: &Observation, p: V2) -> bool {
    obs.depots.iter().any(|d| d.owner.is_some_and(|o| !obs.allied(o)) && d.pos.dist(p) <= ENEMY_BASE_R)
}

/// The `n`th mine's site on the line between `base` and the nearest hostile depot: straight
/// across that road, alternating to each side of it, drawn back from the line and in reach.
pub fn mine_site(obs: &Observation, base: V2, n: u8) -> Option<V2> {
    let enemy = obs.depots.iter().filter(|d| d.owner.is_some_and(|o| !obs.allied(o))).map(|d| d.pos).min_by(|a, b| a.dist(base).total_cmp(&b.dist(base)))?;
    let far = enemy.dist(base);
    if far < 1.0 {
        return None;
    }
    let dir = (enemy - base).norm();
    // Forty steps along the road for where the side changes: the line need not be straight.
    let at = (0..=40).map(|i| far * i as f32 / 40.0).find(|k| side(obs, base + dir.scale(*k)).is_some_and(|s| s <= 0.0))?;
    let k = (at - LINE_BACK).clamp(0.0, LINE_REACH);
    let across = V2::new(-dir.y, dir.x);
    let step = (n / 2 + 1) as f32 * LINE_SPACING;
    let off = if n % 2 == 0 { step } else { -step };
    Some(Map::tile_centre(base + dir.scale(k) + across.scale(off)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two seats' views of one map: us at the first depot, an enemy at the second.
    fn two_sides() -> (World, V2, V2) {
        let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        w.pillboxes.clear();
        let (a, b) = (w.players[0].id, w.players[1].id);
        for d in w.depots.iter_mut() {
            d.owner = None;
        }
        w.depots[0].owner = Some(a);
        w.depots[1].owner = Some(b);
        let (us, them) = (w.depots[0].pos, w.depots[1].pos);
        (w, us, them)
    }

    #[test]
    fn the_line_falls_between_the_two_sides_ground() {
        let (w, us, them) = two_sides();
        let obs = w.observe(w.players[0].id);
        assert!(side(&obs, us).unwrap() > 0.0);
        assert!(side(&obs, them).unwrap() < 0.0);
        assert!(in_enemy_base(&obs, them) && !in_enemy_base(&obs, us));
    }

    #[test]
    fn a_mine_on_the_line_stands_on_our_side_and_in_reach() {
        let (w, us, _) = two_sides();
        let obs = w.observe(w.players[0].id);
        for n in 0..6 {
            let at = mine_site(&obs, us, n).unwrap();
            assert!(side(&obs, at).unwrap() >= 0.0, "site {} over the line", n);
            assert!(at.dist(us) <= LINE_REACH + 8.0 * LINE_SPACING, "site {} out of reach", n);
        }
        assert_ne!(mine_site(&obs, us, 0), mine_site(&obs, us, 1));
    }

    #[test]
    fn a_raid_lays_a_mine_in_their_base_then_runs_for_home() {
        use crate::commander::{Commander, Goal};
        use crate::weights::{Settings, W};
        let (mut w, us, them) = two_sides();
        w.tanks[0].pos = them;
        let mut s = Settings::default();
        s.w[W::MinePlan] = 2.0;
        let mut c = Commander::new(1, "raider");
        c.set_settings(s);
        c.set_goal(Goal::Auto);
        let obs = w.observe(w.players[0].id);
        let laid = crate::Brain::think(&mut c, &obs);
        let mut later = w.observe(w.players[0].id);
        later.tick += 6 * TICK_HZ as Tick;
        let cmds = (laid, crate::Brain::think(&mut c, &later));
        assert!(cmds.0.iter().any(|c| matches!(c, Command::Deploy { task: PillboyTask::Build { what: BuildKind::Mine, .. } })), "{:?}", cmds.0);
        let to = cmds.1.iter().find_map(|c| match c { Command::Move { to } => Some(*to), _ => None });
        assert!(to.is_some_and(|t| t.dist(us) < t.dist(them)), "no run for home: {:?}", cmds.1);
    }

    #[test]
    fn no_enemy_ground_is_no_line() {
        let (mut w, us, _) = two_sides();
        w.depots[1].owner = None;
        let obs = w.observe(w.players[0].id);
        assert!(side(&obs, us).is_none() && mine_site(&obs, us, 0).is_none());
    }
}
