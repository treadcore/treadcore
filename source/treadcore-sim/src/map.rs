//! The map: a 128 x 128 grid of terrain, generated from a seed, with
//! sixteen starts around the edge, a depot near each start and one in
//! the middle.
//!
//! Terrain does three things: it sets how fast a tank moves, it hides a
//! tank from eyes that are not directly over it, and water stops a tank
//! dead. Drones ignore all of it. That asymmetry is most of the game's
//! texture: the ground is the tank's problem, the air is the drones'.

use crate::{Rng, V2};
use serde::{Deserialize, Serialize};

pub const MAP_W: usize = 128;
pub const MAP_H: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Terrain {
    Grass = 0,
    Forest = 1,
    Road = 2,
    Water = 3,
    Bridge = 4,
    Rubble = 5,
    Depot = 6,
    /// Built by the squad. Impassable; blocks a direct shot and a
    /// pillbox's line; an indirect splash knocks it down to rubble.
    Wall = 7,
}

impl Terrain {
    pub fn from_u8(v: u8) -> Terrain {
        match v {
            1 => Terrain::Forest,
            2 => Terrain::Road,
            3 => Terrain::Water,
            4 => Terrain::Bridge,
            5 => Terrain::Rubble,
            6 => Terrain::Depot,
            7 => Terrain::Wall,
            _ => Terrain::Grass,
        }
    }
    /// Time to cross, relative to grass.
    pub fn move_cost(self) -> f32 {
        match self {
            Terrain::Road => 0.6,
            Terrain::Grass | Terrain::Depot | Terrain::Bridge => 1.0,
            Terrain::Rubble => 1.5,
            Terrain::Forest => 1.8,
            Terrain::Water | Terrain::Wall => f32::INFINITY,
        }
    }
    pub fn passable(self) -> bool {
        !matches!(self, Terrain::Water | Terrain::Wall)
    }
    /// Blocks a flat shot: forest hides what is in it and what is
    /// behind it; a wall is a wall.
    pub fn blocks_fire(self) -> bool {
        matches!(self, Terrain::Forest | Terrain::Wall)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Map {
    pub w: usize,
    pub h: usize,
    pub seed: u64,
    /// Row-major, `h` rows of `w`. u8 so a snapshot of the whole map is
    /// 16 KiB and a diff is the cells that changed.
    pub cells: Vec<u8>,
    pub starts: Vec<V2>,
    pub depots: Vec<V2>,
    /// Neutral pillboxes at the start: one on each spoke between a
    /// start's depot and the centre, and four around the centre.
    pub pillboxes: Vec<V2>,
}

impl Map {
    pub fn generate(seed: u64) -> Map {
        let (w, h) = (MAP_W, MAP_H);
        let mut rng = Rng::new(seed);
        let mut cells = vec![Terrain::Grass as u8; w * h];
        let noise = Noise::new(&mut rng);

        // Forest: thresholded smooth noise, so it comes in clumps and belts
        // rather than speckles.
        for y in 0..h {
            for x in 0..w {
                let n = noise.at(x as f32 * 0.045, y as f32 * 0.045);
                if n > 0.58 {
                    cells[y * w + x] = Terrain::Forest as u8;
                }
            }
        }
        // A river: a wandering line from one edge to the opposite one, two
        // or three tiles wide, with a lake where it bends hardest.
        let vertical = rng.below(2) == 0;
        let mut p = if vertical { rng.range(30.0, 98.0) } else { rng.range(30.0, 98.0) };
        for i in 0..(if vertical { h } else { w }) {
            p += rng.range(-1.2, 1.2) + (64.0 - p) * 0.01;
            let width = 1.5 + (noise.at(i as f32 * 0.1, 7.7) * 2.0);
            for d in -3..=3 {
                let q = p + d as f32;
                if (q - p).abs() <= width {
                    let (x, y) = if vertical { (q as i32, i as i32) } else { (i as i32, q as i32) };
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        cells[y as usize * w + x as usize] = Terrain::Water as u8;
                    }
                }
            }
        }
        // Starts: sixteen, around the edge, pulled in from it. Ordered so
        // that a scenario of four gets the four sides, of eight the sides
        // and the corners, of sixteen the lot: a game of any size is
        // spread out. Depots: a few tiles toward the centre from each
        // start, plus the centre itself.
        let inset = 10.0;
        let (lo, mid, hi) = (inset, w as f32 * 0.5, w as f32 - inset);
        let q1 = lo + (hi - lo) * 0.25;
        let q3 = lo + (hi - lo) * 0.75;
        let starts = vec![
            V2::new(mid, lo), // north
            V2::new(hi, mid), // east
            V2::new(mid, hi), // south
            V2::new(lo, mid), // west
            V2::new(hi, lo),  // north-east corner
            V2::new(hi, hi),  // south-east
            V2::new(lo, hi),  // south-west
            V2::new(lo, lo),  // north-west
            V2::new(q1, lo),  // the eight in between, clockwise from north
            V2::new(q3, lo),
            V2::new(hi, q1),
            V2::new(hi, q3),
            V2::new(q3, hi),
            V2::new(q1, hi),
            V2::new(lo, q3),
            V2::new(lo, q1),
        ];
        let center = V2::new(w as f32 * 0.5, h as f32 * 0.5);
        let mut depots: Vec<V2> = starts.iter().map(|s| s.lerp(center, 0.18)).collect();
        depots.push(center);
        // Pillboxes: a third of the way in on every spoke (ten tiles
        // past the start's depot, eight short of where a bot stands to
        // fire), and a square of four round the centre depot. All
        // neutral: Bolo's, shooting at everyone until someone's squad
        // takes them.
        let mut pillboxes: Vec<V2> = starts.iter().map(|s| s.lerp(center, 0.36)).collect();
        for (dx, dy) in [(9.0, 0.0), (-9.0, 0.0), (0.0, 9.0), (0.0, -9.0)] {
            pillboxes.push(center + V2::new(dx, dy));
        }

        let mut map = Map { w, h, seed, cells, starts: starts.clone(), depots: depots.clone(), pillboxes: pillboxes.clone() };

        // Roads: the four sides to the centre, and a ring road round the
        // edge through every start, so each start reaches a spoke. Roads
        // bridge the river where they cross it.
        for s in &starts[..4] {
            map.road(*s, center, &mut rng);
        }
        let ring = [0usize, 8, 4, 10, 1, 11, 5, 12, 2, 13, 6, 14, 3, 15, 7, 9];
        for i in 0..ring.len() {
            let a = starts[ring[i]];
            let b = starts[ring[(i + 1) % ring.len()]];
            map.road(a, b, &mut rng);
        }
        // Clear the ground around starts, depots and pillboxes, then
        // stamp depots.
        for s in starts.iter().chain(depots.iter()) {
            map.clear_disc(*s, 3.0);
        }
        for p in &pillboxes {
            map.clear_disc(*p, 1.5);
        }

        for d in &depots {
            map.stamp(*d, 1, Terrain::Depot);
        }
        map
    }

    fn road(&mut self, a: V2, b: V2, rng: &mut Rng) {
        // A gently wobbling walk from a to b, one tile wide.
        let steps = (a.dist(b) * 1.5) as usize + 1;
        let mut wobble = 0.0f32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            wobble = (wobble + rng.range(-0.3, 0.3)) * 0.95;
            let perp = V2::new(-(b - a).norm().y, (b - a).norm().x).scale(wobble * 3.0 * (t * std::f32::consts::PI).sin());
            let p = a.lerp(b, t) + perp;
            let (x, y) = p.tile();
            if self.in_tile(x, y) {
                let i = y as usize * self.w + x as usize;
                if self.cells[i] == Terrain::Water as u8 {
                    // A bridge is a plus of tiles, not a line: the walk
                    // steps two-thirds of a tile and a one-tile bridge
                    // can come out diagonal, which the pathfinder (rightly)
                    // refuses to cross.
                    for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (bx, by) = (x + dx, y + dy);
                        if self.in_tile(bx, by) {
                            let j = by as usize * self.w + bx as usize;
                            if self.cells[j] == Terrain::Water as u8 {
                                self.cells[j] = Terrain::Bridge as u8;
                            }
                        }
                    }
                } else {
                    self.cells[i] = Terrain::Road as u8;
                }
            }
        }
    }

    fn clear_disc(&mut self, c: V2, r: f32) {
        for y in 0..self.h {
            for x in 0..self.w {
                if V2::new(x as f32 + 0.5, y as f32 + 0.5).dist(c) <= r {
                    let i = y * self.w + x;
                    if self.cells[i] == Terrain::Forest as u8 || self.cells[i] == Terrain::Water as u8 {
                        self.cells[i] = Terrain::Grass as u8;
                    }
                }
            }
        }
    }

    fn stamp(&mut self, c: V2, half: i32, t: Terrain) {
        let (cx, cy) = c.tile();
        for y in cy - half..=cy + half {
            for x in cx - half..=cx + half {
                if self.in_tile(x, y) {
                    self.cells[y as usize * self.w + x as usize] = t as u8;
                }
            }
        }
    }

    /// A shell landing turns grass and forest to rubble and roads to
    /// rubble; the map remembers the fight.
    pub fn crater(&mut self, at: V2) {
        let (cx, cy) = at.tile();
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (x, y) = (cx + dx, cy + dy);
            if self.in_tile(x, y) {
                let i = y as usize * self.w + x as usize;
                let t = Terrain::from_u8(self.cells[i]);
                if matches!(t, Terrain::Grass | Terrain::Forest | Terrain::Road) {
                    self.cells[i] = Terrain::Rubble as u8;
                }
            }
        }
    }

    pub fn in_tile(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h
    }
    pub fn in_bounds(&self, p: V2) -> bool {
        p.x >= 0.0 && p.y >= 0.0 && p.x < self.w as f32 && p.y < self.h as f32
    }
    pub fn at(&self, p: V2) -> Terrain {
        let (x, y) = p.tile();
        if self.in_tile(x, y) {
            Terrain::from_u8(self.cells[y as usize * self.w + x as usize])
        } else {
            Terrain::Water
        }
    }
    pub fn at_tile(&self, x: usize, y: usize) -> Terrain {
        Terrain::from_u8(self.cells[y * self.w + x])
    }
    pub fn move_cost(&self, p: V2) -> f32 {
        self.at(p).move_cost()
    }
    pub fn center(&self) -> V2 {
        V2::new(self.w as f32 * 0.5, self.h as f32 * 0.5)
    }

    /// Line of sight for direct fire: no forest or wall tile between
    /// (forest hides what is in it, and blocks what is behind it).
    pub fn line_clear(&self, a: V2, b: V2) -> bool {
        let n = (a.dist(b) * 2.0).ceil().max(1.0) as usize;
        for i in 1..n {
            let p = a.lerp(b, i as f32 / n as f32);
            if self.at(p).blocks_fire() {
                return false;
            }
        }
        true
    }

    /// Set one tile. Returns the index, for the event that tells
    /// clients their copy changed.
    pub fn set(&mut self, at: V2, t: Terrain) -> Option<usize> {
        let (x, y) = at.tile();
        if self.in_tile(x, y) {
            let i = y as usize * self.w + x as usize;
            self.cells[i] = t as u8;
            Some(i)
        } else {
            None
        }
    }

    /// The centre of the tile `p` is in.
    pub fn tile_centre(p: V2) -> V2 {
        let (x, y) = p.tile();
        V2::new(x as f32 + 0.5, y as f32 + 0.5)
    }

    /// Up to `n` forest tile centres nearest `around`, within `radius`:
    /// what a crew can see of the trees, for a Brain that has no map.
    pub fn forest_near(&self, around: V2, radius: f32, n: usize) -> Vec<V2> {
        let (cx, cy) = around.tile();
        let r = radius.ceil() as i32;
        let mut out: Vec<(f32, V2)> = Vec::new();
        for y in (cy - r).max(0)..(cy + r + 1).min(self.h as i32) {
            for x in (cx - r).max(0)..(cx + r + 1).min(self.w as i32) {
                if self.at_tile(x as usize, y as usize) == Terrain::Forest {
                    let c = V2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let d = c.dist(around);
                    if d <= radius {
                        out.push((d, c));
                    }
                }
            }
        }
        out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        out.into_iter().take(n).map(|(_, c)| c).collect()
    }


    /// A drivable route from `a` to `b` as a list of waypoints, by A* over
    /// tiles with terrain cost, simplified to corners. The sim moves in
    /// straight lines; this is what a client or bot calls before issuing
    /// a chain of `Move`s. Returns an empty vec if unreachable.
    pub fn route(&self, a: V2, b: V2) -> Vec<V2> {
        self.route_with(a, b, |_| 0.0)
    }

    /// `route`, with an extra cost per tile from `danger` (0 is none, 1
    /// doubles the tile's cost). The world passes the range rings of
    /// live hostile pillboxes, so a tank drives round a gun it knows
    /// about instead of down the road through it. Not a hard wall: a
    /// tile inside a ring is still passable when there is no other way.
    pub fn route_with(&self, a: V2, b: V2, danger: impl Fn(V2) -> f32) -> Vec<V2> {
        use std::cmp::Ordering;
        use std::collections::BinaryHeap;
        #[derive(PartialEq)]
        struct Node(f32, usize);
        impl Eq for Node {}
        impl PartialOrd for Node {
            fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
                Some(self.cmp(o))
            }
        }
        impl Ord for Node {
            fn cmp(&self, o: &Self) -> Ordering {
                o.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
            }
        }
        let (ax, ay) = a.tile();
        let (bx, by) = b.tile();
        if !self.in_tile(ax, ay) || !self.in_tile(bx, by) || !self.at(b).passable() {
            return vec![];
        }
        let idx = |x: i32, y: i32| y as usize * self.w + x as usize;
        let start = idx(ax, ay);
        let goal = idx(bx, by);
        let n = self.w * self.h;
        let mut g = vec![f32::INFINITY; n];
        let mut came = vec![usize::MAX; n];
        let mut open = BinaryHeap::new();
        g[start] = 0.0;
        open.push(Node(0.0, start));
        let h = |i: usize| {
            let (x, y) = ((i % self.w) as f32, (i / self.w) as f32);
            ((x - bx as f32).powi(2) + (y - by as f32).powi(2)).sqrt() * 0.6
        };
        let mut found = false;
        while let Some(Node(_, cur)) = open.pop() {
            if cur == goal {
                found = true;
                break;
            }
            let (cx, cy) = ((cur % self.w) as i32, (cur / self.w) as i32);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, ny) = (cx + dx, cy + dy);
                if !self.in_tile(nx, ny) {
                    continue;
                }
                let ni = idx(nx, ny);
                let t = Terrain::from_u8(self.cells[ni]);
                if !t.passable() {
                    continue;
                }
                // No cutting corners through water on a diagonal.
                if dx != 0 && dy != 0 && (!self.at_tile(cx as usize, ny as usize).passable() || !self.at_tile(nx as usize, cy as usize).passable()) {
                    continue;
                }
                let step = if dx != 0 && dy != 0 { 1.4142 } else { 1.0 } * t.move_cost() * (1.0 + danger(V2::new(nx as f32 + 0.5, ny as f32 + 0.5)));

                let ng = g[cur] + step;
                if ng < g[ni] {
                    g[ni] = ng;
                    came[ni] = cur;
                    open.push(Node(ng + h(ni), ni));
                }
            }
        }
        if !found {
            return vec![];
        }
        let mut path = vec![goal];
        let mut cur = goal;
        while cur != start {
            cur = came[cur];
            path.push(cur);
        }
        path.reverse();
        // Keep only direction changes, so a straight road is one waypoint.
        let mut out: Vec<V2> = Vec::new();
        let centre = |i: usize| V2::new((i % self.w) as f32 + 0.5, (i / self.w) as f32 + 0.5);
        for k in 1..path.len() {
            let prev = path[k - 1];
            let cur = path[k];
            let next = path.get(k + 1).copied();
            let d1 = (cur as i64 - prev as i64, 0);
            let d2 = next.map(|n| (n as i64 - cur as i64, 0));
            if Some(d1) != d2 {
                out.push(centre(cur));
            }
        }
        if let Some(last) = out.last_mut() {
            *last = b;
        } else {
            out.push(b);
        }
        out
    }
}

/// Value noise: a lattice of random values, bilinear interpolation,
/// three octaves. Enough for forests and river width; not for terrain
/// height, which this game does not have.
struct Noise {
    lattice: Vec<f32>,
    n: usize,
}

impl Noise {
    fn new(rng: &mut Rng) -> Noise {
        let n = 64;
        let lattice = (0..n * n).map(|_| rng.f32()).collect();
        Noise { lattice, n }
    }
    fn raw(&self, x: f32, y: f32) -> f32 {
        let n = self.n as i64;
        let (x0, y0) = (x.floor() as i64, y.floor() as i64);
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let g = |ix: i64, iy: i64| self.lattice[(iy.rem_euclid(n) * n + ix.rem_euclid(n)) as usize];
        let s = |t: f32| t * t * (3.0 - 2.0 * t);
        let (sx, sy) = (s(fx), s(fy));
        let top = g(x0, y0) + (g(x0 + 1, y0) - g(x0, y0)) * sx;
        let bot = g(x0, y0 + 1) + (g(x0 + 1, y0 + 1) - g(x0, y0 + 1)) * sx;
        top + (bot - top) * sy
    }
    fn at(&self, x: f32, y: f32) -> f32 {
        (self.raw(x, y) * 0.6 + self.raw(x * 2.1, y * 2.1) * 0.3 + self.raw(x * 4.3, y * 4.3) * 0.1).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic_and_starts_are_dry() {
        let a = Map::generate(42);
        let b = Map::generate(42);
        assert_eq!(a.cells, b.cells);
        for s in &a.starts {
            assert!(a.at(*s).passable());
        }
        for d in &a.depots {
            assert_eq!(a.at(*d), Terrain::Depot);
        }
    }

    #[test]
    fn every_start_can_reach_the_centre() {
        for seed in [7u64, 42, 0x7EAD_C0DE] {
            let m = Map::generate(seed);
            assert_eq!(m.starts.len(), crate::MAX_PLAYERS);
            assert_eq!(m.depots.len(), crate::MAX_PLAYERS + 1);
            for s in &m.starts {
                let r = m.route(*s, m.center());
                assert!(!r.is_empty(), "seed {}: no route from {:?}", seed, s);
            }
        }
    }

}
