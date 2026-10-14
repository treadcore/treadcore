//! The map: a 128 x 128 grid of terrain, generated from a seed, with
//! sixteen starts around the edge, a depot near each start and one in
//! the middle.
//!
//! Terrain does three things: it sets how fast a tank moves, it hides a
//! tank from eyes that are not directly over it, and water stops a tank
//! dead. Drones ignore all of it. That asymmetry is most of the game's
//! texture: the ground is the tank's problem, the air is the drones'.

use crate::{PlayerId, Rng, StructureKind, MAX_PLAYERS, V2};
use serde::{Deserialize, Serialize};

/// The generated map's size: 128 tiles of 25 m is a 3.2 km island. A
/// map file declares its own `size` (1 to 1024; the prior art's maps are 256)
/// and the world is built at the size it reads.
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
    /// Foundations: industrial ground, the only ground a structure is
    /// built on (roadmap step 10a), so the map decides where an
    /// economy can be. Driven on as a road.
    Pad = 8,
    /// A hardened wall: as a wall, but a splash cracks it to a plain
    /// wall rather than rubble, so it takes two. Two material.
    Concrete = 9,
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
            8 => Terrain::Pad,
            9 => Terrain::Concrete,
            _ => Terrain::Grass,
        }
    }
    /// Time to cross, relative to grass.
    pub fn move_cost(self) -> f32 {
        match self {
            Terrain::Road | Terrain::Pad => 0.6,
            Terrain::Grass | Terrain::Depot | Terrain::Bridge => 1.0,
            Terrain::Rubble => 1.5,
            Terrain::Forest => 1.8,
            Terrain::Water | Terrain::Wall | Terrain::Concrete => f32::INFINITY,
        }
    }
    pub fn passable(self) -> bool {
        !matches!(self, Terrain::Water | Terrain::Wall | Terrain::Concrete)
    }
    /// A wall of either kind.
    pub fn is_wall(self) -> bool {
        matches!(self, Terrain::Wall | Terrain::Concrete)
    }
    /// Blocks a flat shot: forest hides what is in it and what is
    /// behind it; a wall is a wall.
    pub fn blocks_fire(self) -> bool {
        matches!(self, Terrain::Forest | Terrain::Wall | Terrain::Concrete)
    }
}

/// Who owns a depot at the start. `ByStart` is the generator's rule:
/// the i-th depot belongs to the i-th seat, if seated. A map file may
/// say `neutral` or a seat instead.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DepotOwner {
    ByStart,
    Neutral,
    Seat(PlayerId),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapDepot {
    pub pos: V2,
    pub owner: DepotOwner,
}

/// A pillbox as the map places it: where, and whose at the start
/// (None is neutral, the prior art's default).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapPillbox {
    pub pos: V2,
    pub owner: Option<PlayerId>,
}

/// A structure as the map places it (roadmap step 10b): where, what,
/// and whose at the start (None is neutral).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapStructure {
    pub pos: V2,
    pub kind: StructureKind,
    pub owner: Option<PlayerId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Map {
    pub w: usize,
    pub h: usize,
    pub seed: u64,
    /// Row-major, `h` rows of `w`. u8 so a snapshot of the whole map is
    /// 16 KiB and a diff is the cells that changed.
    pub cells: Vec<u8>,
    pub starts: Vec<V2>,
    pub depots: Vec<MapDepot>,
    /// Pillboxes at the start. Generated maps place neutral ones: one
    /// on each spoke between a start's depot and the centre, and four
    /// around the centre. A map file may give owners.
    pub pillboxes: Vec<MapPillbox>,
    /// Structures at the start: none on a generated island unless it
    /// is industrialised; a map file places its own.
    #[serde(default)]
    pub structures: Vec<MapStructure>,
}

/// Why a map file was refused, with the line it happened on.
#[derive(Clone, Debug, PartialEq)]
pub struct MapError {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for MapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

impl std::error::Error for MapError {}

/// The one character per terrain in a map file. Chosen to read as the
/// ground does in a terminal: `.` open, `T` trees, `=` road, `~` water,
/// `+` bridge, `%` rubble, `D` depot, `W` wall, `_` pad, `H` concrete.
impl Terrain {
    pub fn glyph(self) -> char {
        match self {
            Terrain::Grass => '.',
            Terrain::Forest => 'T',
            Terrain::Road => '=',
            Terrain::Water => '~',
            Terrain::Bridge => '+',
            Terrain::Rubble => '%',
            Terrain::Depot => 'D',
            Terrain::Wall => 'W',
            Terrain::Pad => '_',
            Terrain::Concrete => 'H',
        }
    }
    pub fn from_glyph(c: char) -> Option<Terrain> {
        Some(match c {
            '.' => Terrain::Grass,
            'T' => Terrain::Forest,
            '=' => Terrain::Road,
            '~' => Terrain::Water,
            '+' => Terrain::Bridge,
            '%' => Terrain::Rubble,
            'D' => Terrain::Depot,
            'W' => Terrain::Wall,
            '_' => Terrain::Pad,
            'H' => Terrain::Concrete,
            _ => return None,
        })
    }
}


impl Map {
    pub fn generate(seed: u64) -> Map {
        let (w, h) = (MAP_W, MAP_H);
        let mut rng = Rng::new(seed);
        let mut cells = vec![Terrain::Grass as u8; w * h];
        let noise = Noise::new(&mut rng);

        // Forest: thresholded smooth noise, so it comes in clumps and belts
        // rather than speckles. The frequency scales with the map so a
        // belt is the same number of tiles wide at any size.
        let f = 0.045 * 128.0 / w as f32;
        for y in 0..h {
            for x in 0..w {
                let n = noise.at(x as f32 * f, y as f32 * f);
                if n > 0.58 {
                    cells[y * w + x] = Terrain::Forest as u8;
                }
            }
        }
        // A river: a wandering line from one edge to the opposite one, two
        // or three tiles wide, with a lake where it bends hardest.
        let vertical = rng.below(2) == 0;
        let (across, along) = if vertical { (w as f32, h) } else { (h as f32, w) };
        let mut p = rng.range(across * 0.234375, across * 0.765625); // 30..98 of 128, as on day one
        for i in 0..along {
            p += rng.range(-1.2, 1.2) + (across * 0.5 - p) * 0.01;
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
        let mut depots: Vec<MapDepot> = starts.iter().map(|s| MapDepot { pos: s.lerp(center, 0.18), owner: DepotOwner::ByStart }).collect();
        depots.push(MapDepot { pos: center, owner: DepotOwner::ByStart });
        // Pillboxes: a third of the way in on every spoke (ten tiles
        // past the start's depot, eight short of where a bot stands to
        // fire), and a square of four round the centre depot. All
        // neutral: The prior art's, shooting at everyone until someone's squad
        // takes them.
        let mut pillboxes: Vec<MapPillbox> = starts.iter().map(|s| MapPillbox { pos: s.lerp(center, 0.36), owner: None }).collect();
        let ring = 9.0 * w as f32 / 128.0;
        for (dx, dy) in [(ring, 0.0), (-ring, 0.0), (0.0, ring), (0.0, -ring)] {
            pillboxes.push(MapPillbox { pos: center + V2::new(dx, dy), owner: None });
        }

        let mut map = Map { w, h, seed, cells, starts: starts.clone(), depots: depots.clone(), pillboxes: pillboxes.clone(), structures: Vec::new() };

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
        for s in starts.iter().chain(depots.iter().map(|d| &d.pos)) {
            map.clear_disc(*s, 3.0);
        }
        for p in &pillboxes {
            map.clear_disc(p.pos, 1.5);
        }

        for d in &depots {
            map.stamp(d.pos, 1, Terrain::Depot);
        }
        map
    }

    /// Give the map an economy (roadmap step 10f): a zone of pads
    /// half way along each start's spoke to the centre, with a neutral
    /// power plant and refinery on it and room for three more, and a
    /// zone either side of the centre with a power plant and a data
    /// centre. Forest and water under a pad are cleared; a pad is
    /// ground. The generated island has none of this unless asked, so
    /// every proof on it reads as before.
    pub fn industrialise(&mut self) {
        let c = self.center();
        let starts = self.starts.clone();
        let zone = |m: &mut Map, at: V2, w: i32, h: i32| {
            let (cx, cy) = at.tile();
            for y in cy..cy + h {
                for x in cx..cx + w {
                    if m.in_tile(x, y) {
                        m.cells[y as usize * m.w + x as usize] = Terrain::Pad as u8;
                    }
                }
            }
        };
        let tile = |p: V2| V2::new(p.x.floor() + 0.5, p.y.floor() + 0.5);
        for s in &starts {
            let at = tile(s.lerp(c, 0.5)) - V2::new(1.0, 1.0);
            if self.depots.iter().any(|d| d.pos.dist(at) < 4.0) || self.pillboxes.iter().any(|p| p.pos.dist(at) < 3.0) {
                continue;
            }
            zone(self, at, 3, 3);
            self.structures.push(MapStructure { pos: at, kind: StructureKind::Power, owner: None });
            self.structures.push(MapStructure { pos: at + V2::new(2.0, 2.0), kind: StructureKind::Refinery, owner: None });
        }
        for dy in [-5.0f32, 4.0] {
            let at = tile(c + V2::new(-1.0, dy));
            zone(self, at, 3, 2);
            self.structures.push(MapStructure { pos: at, kind: StructureKind::Power, owner: None });
            self.structures.push(MapStructure { pos: at + V2::new(2.0, 0.0), kind: StructureKind::DataCentre, owner: None });
        }
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

// ---------------------------------------------------------------- files

/// The map file: `documentation/MAPS.md`. A header, the rows, then the
/// places. Any text editor is the map editor.
pub const MAP_FORMAT: &str = "treadcore map 1";

impl Map {
    /// The map as a text file. `{}` on floats prints the shortest
    /// string that reads back to the same value, so a map survives the
    /// round trip exactly (the test for this step).
    pub fn to_text(&self) -> String {
        let mut s = String::with_capacity(self.w * (self.h + 1) + 1024);
        s.push_str(MAP_FORMAT);
        s.push('\n');
        s.push_str(&format!("size {} {}\n", self.w, self.h));
        s.push_str(&format!("seed {:#x}\n", self.seed));
        s.push_str("# rows: one character per tile, west to east, north first;\n");
        s.push_str("# . grass  T forest  = road  ~ water  + bridge  % rubble  D depot  W wall\n");
        s.push_str("# _ pad (structures are built on it)  H concrete (a wall that takes two)\n");
        for y in 0..self.h {
            for x in 0..self.w {
                s.push(self.at_tile(x, y).glyph());
            }
            s.push('\n');
        }
        s.push_str("# places: start x y (seat order); depot x y [seat|neutral] (unsaid: the\n");
        s.push_str("# first N depots belong to the first N seats); pillbox x y [seat|neutral]\n");
        for p in &self.starts {
            s.push_str(&format!("start {} {}\n", p.x, p.y));
        }
        for d in &self.depots {
            match d.owner {
                DepotOwner::ByStart => s.push_str(&format!("depot {} {}\n", d.pos.x, d.pos.y)),
                DepotOwner::Neutral => s.push_str(&format!("depot {} {} neutral\n", d.pos.x, d.pos.y)),
                DepotOwner::Seat(o) => s.push_str(&format!("depot {} {} {}\n", d.pos.x, d.pos.y, o)),
            }
        }
        for p in &self.pillboxes {
            match p.owner {
                Some(o) => s.push_str(&format!("pillbox {} {} {}\n", p.pos.x, p.pos.y, o)),
                None => s.push_str(&format!("pillbox {} {} neutral\n", p.pos.x, p.pos.y)),
            }
        }
        for st in &self.structures {
            match st.owner {
                Some(o) => s.push_str(&format!("structure {} {} {} {}\n", st.pos.x, st.pos.y, st.kind.word(), o)),
                None => s.push_str(&format!("structure {} {} {} neutral\n", st.pos.x, st.pos.y, st.kind.word())),
            }
        }
        s
    }

    /// Read a map file. Refuses, with the line, anything it cannot
    /// use: the wrong format line, a row of the wrong length, a glyph
    /// it does not know, a place off the map, a start or depot on water
    /// or a wall, no starts, more than sixteen. Repairs what the prior art's
    /// loader repaired: a pillbox owner that is not a seat is neutral.
    pub fn from_text(text: &str) -> Result<Map, MapError> {
        let err = |line: usize, msg: &str| MapError { line, msg: msg.to_string() };
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim_end()));
        let (n, first) = lines.next().ok_or_else(|| err(0, "empty file"))?;
        if first.trim() != MAP_FORMAT {
            return Err(err(n, &format!("not a treadcore map: expected '{}'", MAP_FORMAT)));
        }
        let (mut w, mut h, mut seed) = (0usize, 0usize, 0u64);
        let mut cells: Vec<u8> = Vec::new();
        let mut starts = Vec::new();
        let mut depots = Vec::new();
        let mut pillboxes = Vec::new();
        let mut structures = Vec::new();
        let mut rows_read = 0usize;
        for (n, raw) in lines {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut words = line.split_whitespace();
            let head = words.next().unwrap();
            // A row is a line made only of glyphs, of the map's width.
            if w > 0 && rows_read < h && line.len() == w && line.chars().all(|c| Terrain::from_glyph(c).is_some()) {
                for c in line.chars() {
                    cells.push(Terrain::from_glyph(c).unwrap() as u8);
                }
                rows_read += 1;
                continue;
            }
            let num = |s: Option<&str>, what: &str| -> Result<f32, MapError> {
                s.ok_or_else(|| err(n, &format!("{} missing", what)))?.parse::<f32>().map_err(|_| err(n, &format!("{} is not a number", what)))
            };
            match head {
                "size" => {
                    w = num(words.next(), "width")? as usize;
                    h = num(words.next(), "height")? as usize;
                    if w == 0 || h == 0 || w > 1024 || h > 1024 {
                        return Err(err(n, "size out of range (1 to 1024)"));
                    }
                }
                "seed" => {
                    let s = words.next().ok_or_else(|| err(n, "seed missing"))?;
                    seed = s.strip_prefix("0x").map(|x| u64::from_str_radix(x, 16)).unwrap_or_else(|| s.parse::<u64>()).map_err(|_| err(n, "seed is not a number"))?;
                }
                "start" | "depot" | "pillbox" | "structure" => {
                    if rows_read < h {
                        return Err(err(n, &format!("'{}' before all {} rows were read ({} so far)", head, h, rows_read)));
                    }
                    let x = num(words.next(), "x")?;
                    let y = num(words.next(), "y")?;
                    if x < 0.0 || y < 0.0 || x >= w as f32 || y >= h as f32 {
                        return Err(err(n, "place is off the map"));
                    }
                    let p = V2::new(x, y);
                    let t = Terrain::from_u8(cells[y as usize * w + x as usize]);
                    match head {
                        "start" => {
                            if !t.passable() {
                                return Err(err(n, "a start on water or a wall"));
                            }
                            starts.push(p);
                        }
                        "depot" => {
                            if !t.passable() {
                                return Err(err(n, "a depot on water or a wall"));
                            }
                            let owner = match words.next() {
                                None => DepotOwner::ByStart,
                                Some("neutral") | Some("-") => DepotOwner::Neutral,
                                Some(s) => match s.parse::<usize>() {
                                    Ok(o) if o < MAX_PLAYERS => DepotOwner::Seat(o as PlayerId),
                                    _ => DepotOwner::Neutral,
                                },
                            };
                            depots.push(MapDepot { pos: p, owner });
                        }
                        "structure" => {
                            if t != Terrain::Pad {
                                return Err(err(n, "a structure off a pad (_)"));
                            }
                            let word = words.next().ok_or_else(|| err(n, "structure kind missing"))?;
                            let kind = StructureKind::from_word(word).ok_or_else(|| err(n, &format!("no structure '{}': power, refinery, datacentre, workshop or jammer", word)))?;
                            let owner = match words.next() {
                                None | Some("neutral") | Some("-") => None,
                                Some(s) => match s.parse::<usize>() {
                                    Ok(o) if o < MAX_PLAYERS => Some(o as PlayerId),
                                    _ => None,
                                },
                            };
                            structures.push(MapStructure { pos: p, kind, owner });
                        }
                        _ => {
                            if t == Terrain::Water {
                                return Err(err(n, "a pillbox on water"));
                            }
                            let owner = match words.next() {
                                None | Some("neutral") | Some("-") => None,
                                Some(s) => match s.parse::<usize>() {
                                    Ok(o) if o < MAX_PLAYERS => Some(o as PlayerId),
                                    _ => None, // the prior art's repair: an owner that is no seat is neutral
                                },
                            };
                            pillboxes.push(MapPillbox { pos: p, owner });
                        }
                    }
                }
                _ => {
                    if w > 0 && rows_read < h {
                        return Err(err(n, &format!("row {} is not {} glyphs of . T = ~ + % D W _ H", rows_read + 1, w)));
                    }
                    return Err(err(n, &format!("unknown line '{}'", head)));
                }
            }
        }
        if w == 0 {
            return Err(err(0, "no size line"));
        }
        if rows_read != h {
            return Err(err(0, &format!("{} rows, expected {}", rows_read, h)));
        }
        if starts.is_empty() {
            return Err(err(0, "no start lines: a map needs at least one"));
        }
        if starts.len() > MAX_PLAYERS {
            return Err(err(0, &format!("{} starts; the most is {}", starts.len(), MAX_PLAYERS)));
        }
        Ok(Map { w, h, seed, cells, starts, depots, pillboxes, structures })
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
            assert_eq!(a.at(d.pos), Terrain::Depot);
        }
    }

    /// Roadmap step 5's proof: a generated map written as text reads
    /// back equal, to the bit.
    #[test]
    fn map_survives_the_file_round_trip() {
        for seed in [7u64, 0x7EAD_C0DE] {
            let m = Map::generate(seed);
            let text = m.to_text();
            let back = Map::from_text(&text).expect("the map we wrote does not read");
            assert_eq!(back, m, "seed {}", seed);
            assert_eq!(back.to_text(), text);
        }
    }

    /// A loader that says no, with the line.
    #[test]
    fn bad_map_files_are_refused_with_a_line() {
        let m = Map::generate(3);
        let good = m.to_text();
        assert!(Map::from_text("hello\n").is_err());
        let short_row = good.replacen("\n.", "\n", 1);
        let _ = short_row;
        // A glyph nobody knows, in row 1.
        let mut lines: Vec<String> = good.lines().map(|l| l.to_string()).collect();
        let first_row = lines.iter().position(|l| l.len() == m.w && !l.starts_with('#')).unwrap();

        lines[first_row].replace_range(0..1, "?");
        let e = Map::from_text(&lines.join("\n")).unwrap_err();
        assert_eq!(e.line, first_row + 1, "{}", e);
        // A start in the river.
        let water = (0..m.w * m.h).position(|i| m.cells[i] == Terrain::Water as u8).unwrap();
        let (wx, wy) = (water % m.w, water / m.w);
        let bad = format!("{}start {} {}\n", good, wx, wy);
        let e = Map::from_text(&bad).unwrap_err();
        assert!(e.msg.contains("water"), "{}", e);
        // No starts at all.
        let no_starts: String = good.lines().filter(|l| !l.starts_with("start ")).map(|l| format!("{}\n", l)).collect();
        assert!(Map::from_text(&no_starts).unwrap_err().msg.contains("no start"));
        // An owner that is not a seat is repaired to neutral, not refused.
        let odd = format!("{}pillbox 64 64 99\n", good);
        let back = Map::from_text(&odd).unwrap();
        assert_eq!(back.pillboxes.last().unwrap().owner, None);
    }

    #[test]
    fn the_world_maps_read_and_their_continents_connect() {
        // Roadmap step 10g. Seats 0, 2, 4, 6, 8, 10, 12, 14 are in the
        // Americas on the Atlantic map and the odd ones in the Old
        // World: every start reaches the others on its side of the
        // ocean by land (the Darien Gap is jungle, slow but passable;
        // the Strait of Gibraltar and the Bosphorus are bridged), and
        // nothing drives across the Atlantic.
        let m = Map::from_text(include_str!("../../../maps/world/atlantic.map")).expect("atlantic.map reads");
        assert_eq!(m.starts.len(), crate::MAX_PLAYERS);
        assert!(m.structures.len() >= 30);
        for side in [0usize, 1] {
            let seats: Vec<V2> = m.starts.iter().enumerate().filter(|(i, _)| i % 2 == side).map(|(_, s)| *s).collect();
            for s in &seats[1..] {
                assert!(!m.route(seats[0], *s).is_empty(), "side {}: no road from {:?} to {:?}", side, seats[0], s);
            }
        }
        assert!(m.route(m.starts[0], m.starts[1]).is_empty(), "New York should not reach Berlin by land");
        let e = Map::from_text(include_str!("../../../maps/world/europe.map")).expect("europe.map reads");
        assert_eq!(e.starts.len(), crate::MAX_PLAYERS);
        // London and Algiers reach Moscow: the Channel and Gibraltar.
        for s in [0usize, 15] {
            assert!(!e.route(e.starts[s], e.starts[1]).is_empty(), "europe: no road from seat {} to Moscow", s);
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
