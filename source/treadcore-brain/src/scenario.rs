//! Scenario files (roadmap backlog 24, CI-001 VII.B, LAB-REPORT-BRAIN.md
//! section 48; Paul, 18 October: "is there a way to create description
//! and try to attempt these via playtests faster"). A playtest written
//! as text and run headless: the world, the seats that think and how,
//! what is placed and what happens when, and what must hold at the end.
//! Every file under `scenarios/` is a test (`every_scenario_holds`);
//! `make scenario S=file` runs one and prints its trace.
//!
//! The form is a small part of TOML, read here so the brain takes no new
//! crate: `key = value` lines, `[expect]`, and `[[brain]]`, `[[pillbox]]`,
//! `[[depot]]`, `[[shell]]` and `[[event]]` tables; values are numbers,
//! "strings", true or false, `[lists]` and `{ inline = tables }`.
//! Positions are an anchor and an offset in tiles: `"seat:0"` (where a
//! seat's tank starts), `"depot:0"` and `"pillbox:0"` (the first placed),
//! and `"centre"` (the map's middle).
//! `depots = false` keeps only the depots placed (LR 62).
//! `board = true` gives each team's Commanders one team board, a
//! Commander's `rungs = [...]` the ladder's rungs on, and a `[[depot]]`'s
//! `at` and `offset` move it there (LR 58). A family
//! (`scenarios/families/`) has `runs` and the `floor` it must hold over
//! them (`make families`, backlog 47). A `[[tank]]`'s `toward` heads it
//! at a place, and `met_beyond` asks how far from a point a seat first
//! hit another (LR 61). A Commander's `pings = true` calls what it sees
//! and answers calls (a rally), and `outscores` asks that a seat hit
//! another more often than it was hit back (LR 66).
//!
//! ```toml
//! name = "a bot drives out from under a shell"
//! players = 2
//! seconds = 4
//! [[brain]]
//! seat = 0
//! kind = "rulebot"
//! [[shell]]
//! owner = 1
//! at = "seat:0"
//! from = [30, 0]
//! lands_in = 3
//! [expect]
//! hits_on = { seat = 0, max = 0 }
//! moved = { seat = 0, more_than = 2.5 }
//! ```

use crate::commander::{Commander, Goal};
use crate::{Brain, RuleBot};
use std::collections::BTreeMap;
use treadcore_sim::*;

/// A value of the TOML subset.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Num(f64),
    Str(String),
    Bool(bool),
    List(Vec<Value>),
    Table(BTreeMap<String, Value>),
}

impl Value {
    /// A number, or None.
    pub fn num(&self) -> Option<f64> {
        if let Value::Num(n) = self { Some(*n) } else { None }
    }
    /// A string, or None.
    pub fn str(&self) -> Option<&str> {
        if let Value::Str(s) = self { Some(s) } else { None }
    }
    /// An inline table, or None.
    pub fn table(&self) -> Option<&BTreeMap<String, Value>> {
        if let Value::Table(t) = self { Some(t) } else { None }
    }
}

/// A file read: its top-level keys, `[expect]`, and each `[[name]]` list.
#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub top: BTreeMap<String, Value>,
    pub expect: BTreeMap<String, Value>,
    /// `[reward]`: what a training episode earns (LR 99, `reward`).
    pub reward: BTreeMap<String, Value>,
    pub lists: BTreeMap<String, Vec<BTreeMap<String, Value>>>,
}

/// Read the TOML subset; an error names its line.
pub fn parse(text: &str) -> Result<Doc, String> {
    let mut doc = Doc::default();
    // Where a key goes: the top, [expect] or [reward], or the last
    // [[name]].
    let mut into: Option<(String, bool)> = None;
    for (i, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim().to_string();
        if line.is_empty() {
            continue;
        }
        let at = |why: String| format!("line {}: {}", i + 1, why);
        if let Some(name) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
            doc.lists.entry(name.trim().to_string()).or_default().push(BTreeMap::new());
            into = Some((name.trim().to_string(), true));
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            if name.trim() != "expect" && name.trim() != "reward" {
                return Err(at(format!("no section [{}]", name.trim())));
            }
            into = Some((name.trim().to_string(), false));
            continue;
        }
        let (key, value) = line.split_once('=').ok_or_else(|| at("not key = value".into()))?;
        let (v, rest) = value_of(value.trim()).map_err(at)?;
        if !rest.trim().is_empty() {
            return Err(at(format!("after the value: {}", rest.trim())));
        }
        let key = key.trim().to_string();
        match &into {
            None => doc.top.insert(key, v),
            Some((name, false)) if name == "reward" => doc.reward.insert(key, v),
            Some((_, false)) => doc.expect.insert(key, v),
            Some((name, true)) => doc.lists.get_mut(name).and_then(|l| l.last_mut()).expect("a table begun").insert(key, v),
        };
    }
    Ok(doc)
}

/// A line without its `#` comment (a `#` inside a string stays).
fn strip_comment(line: &str) -> &str {
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }
    line
}

/// One value from the front of `s`, and what is left after it.
fn value_of(s: &str) -> Result<(Value, &str), String> {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('"') {
        let end = rest.find('"').ok_or("an unclosed string")?;
        return Ok((Value::Str(rest[..end].to_string()), &rest[end + 1..]));
    }
    if let Some(mut rest) = s.strip_prefix('[') {
        let mut items = Vec::new();
        loop {
            rest = rest.trim_start();
            if let Some(r) = rest.strip_prefix(']') {
                return Ok((Value::List(items), r));
            }
            let (v, r) = value_of(rest)?;
            items.push(v);
            rest = r.trim_start();
            rest = rest.strip_prefix(',').unwrap_or(rest);
        }
    }
    if let Some(mut rest) = s.strip_prefix('{') {
        let mut t = BTreeMap::new();
        loop {
            rest = rest.trim_start();
            if let Some(r) = rest.strip_prefix('}') {
                return Ok((Value::Table(t), r));
            }
            let (key, r) = rest.split_once('=').ok_or("an inline table without key = value")?;
            let (v, r) = value_of(r)?;
            t.insert(key.trim().to_string(), v);
            rest = r.trim_start();
            rest = rest.strip_prefix(',').unwrap_or(rest);
        }
    }
    let end = s.find(|c: char| c == ',' || c == ']' || c == '}' || c.is_whitespace()).unwrap_or(s.len());
    let word = &s[..end];
    let v = match word {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::Num(word.parse::<f64>().map_err(|_| format!("not a value: {}", word))?),
    };
    Ok((v, &s[end..]))
}

/// One frame of a recorded run (LR 102, the clips): what is drawn.
#[derive(Clone, Debug, Default)]
pub struct Frame {
    pub t: f64,
    /// Tanks: owner, position, heading, armour share, alive.
    pub tanks: Vec<(PlayerId, V2, f32, f32, bool)>,
    /// Shells in flight: where now, where they land, direct or not.
    pub shells: Vec<(V2, V2, bool)>,
    /// Drones: owner, position, kind.
    pub drones: Vec<(PlayerId, V2, DroneKind)>,
    /// Pillboxes: position, owner, alive; depots: position, owner.
    pub pillboxes: Vec<(V2, Option<PlayerId>, bool)>,
    pub depots: Vec<(V2, Option<PlayerId>)>,
    /// Hits this frame: where.
    pub hits: Vec<V2>,
}

/// A recorded run: the map's tiles round the action once, a frame
/// every `every` ticks.
#[derive(Clone, Debug, Default)]
pub struct Recording {
    pub every: Tick,
    /// The window drawn: its top-left tile and size, and each tile's
    /// terrain there (row-major).
    pub origin: (i32, i32),
    pub size: (i32, i32),
    pub tiles: Vec<Terrain>,
    pub frames: Vec<Frame>,
}

/// A frame of the world as it stands (the clips, LR 102).
pub fn snap(w: &World) -> Frame {
    let hits = w.events.iter().filter_map(|e| match e {
        Event::TankDamaged { tank, .. } => w.tanks.iter().find(|t| t.id == *tank).map(|t| t.pos),
        Event::ShellImpact { at, .. } => Some(*at),
        _ => None,
    });
    Frame {
        t: w.tick as f64 / TICK_HZ as f64,
        tanks: w.tanks.iter().map(|t| (t.owner, t.pos, t.heading, t.armor / TANK_ARMOR, t.alive)).collect(),
        shells: w.shells.iter().map(|s| (s.pos_at(w.tick), s.impact, s.kind == ShellKind::Direct)).collect(),
        drones: w.drones.iter().map(|d| (d.owner, d.pos, d.kind)).collect(),
        pillboxes: w.pillboxes.iter().map(|p| (p.pos, p.owner, p.alive())).collect(),
        depots: w.depots.iter().map(|d| (d.pos, d.owner)).collect(),
        hits: hits.collect(),
    }
}

impl Recording {
    /// A recording of `half` tiles each way round `centre`, the map's
    /// tiles there kept once.
    pub fn round(w: &World, centre: V2, half: i32, every: Tick) -> Recording {
        let c = centre.tile();
        let origin = (c.0 - half, c.1 - half);
        let size = (2 * half + 1, 2 * half + 1);
        let mut tiles = Vec::with_capacity((size.0 * size.1) as usize);
        for y in 0..size.1 {
            for x in 0..size.0 {
                let p = V2::new((origin.0 + x) as f32 + 0.5, (origin.1 + y) as f32 + 0.5);
                tiles.push(if w.map.in_bounds(p) { w.map.at(p) } else { Terrain::Water });
            }
        }
        Recording { every: every.max(1), origin, size, tiles, frames: Vec::new() }
    }
}

/// What a run saw, for the checks and the trace.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Each seat's tank: where it started and ended, its shells then
    /// and now, and the hits it took (by whom).
    pub start: BTreeMap<PlayerId, (V2, u16)>,
    pub end: BTreeMap<PlayerId, (V2, u16)>,
    pub hits: Vec<(Option<PlayerId>, PlayerId)>,
    /// Where the tank hit was, for each of `hits`.
    pub hit_at: Vec<V2>,
    /// The damage of each of `hits`.
    pub hit_damage: Vec<f32>,
    /// Each seat's tank alive at the end.
    pub alive: BTreeMap<PlayerId, bool>,
    /// Each seat's armour at the start and the end (LR 100).
    pub armor: BTreeMap<PlayerId, (f32, f32)>,
    /// When each seat's tank first went down, seconds.
    pub died_at: BTreeMap<PlayerId, f64>,
    /// The run's length, seconds (the file's, or when it ended).
    pub seconds: f64,
    /// Reactions (LR 102), seconds: each seat's first direct shot, first
    /// indirect shot, first move of more than a tile from its start, and
    /// first launch of each drone kind (`"interceptor"`, ...).
    pub first_fire: BTreeMap<PlayerId, f64>,
    pub first_lob: BTreeMap<PlayerId, f64>,
    pub first_move: BTreeMap<PlayerId, f64>,
    pub first_launch: BTreeMap<(PlayerId, String), f64>,
    /// Each placed pillbox: the damage it did, the tick each seat first
    /// owned it, and whether it stands at the end.
    pub pillbox_damage: Vec<f32>,
    pub pillbox_owned: Vec<BTreeMap<PlayerId, Tick>>,
    /// By seat: calls for help, chains put on, trees cut, road tiles built, in `reward`'s words.
    pub advance: BTreeMap<(PlayerId, &'static str), u32>,
    pub pillbox_alive: Vec<bool>,
    /// Each placed depot: the tick each seat first owned it.
    pub depot_owned: Vec<BTreeMap<PlayerId, Tick>>,
    /// Every 5 s: each thinking seat's status code.
    pub trace: Vec<String>,
    /// When the match ended (a side gone, with no respawns), if it did:
    /// the world stops there.
    pub over: Option<f64>,
    /// The points `ends_near` and `lands_near` name, resolved at the
    /// start.
    pub points: BTreeMap<String, V2>,
    /// Where each seat's tank first came ashore from a boat.
    pub landed: BTreeMap<PlayerId, V2>,
    /// Orders the world refused a thinking seat: when, whose, and why.
    pub refused: Vec<(f64, PlayerId, String)>,
}

/// Run a scenario: the outcome, and every expectation that failed (none
/// is a pass). A file that cannot be read or set up is an error.
pub fn run(doc: &Doc) -> Result<(Outcome, Vec<String>), String> {
    run_seeded(doc, None)
}

/// As `run`, on a seed: the world's (`Scenario::seed`, so its map
/// outside what the file carves) and every brain's (theirs xor it), so
/// a run over many seeds samples the scenario's chances (`many`). None
/// is the file's own world, as `run`: its `seed = n` if it has one, so a
/// family's case found by `many` is kept as a test (LR 53).
pub fn run_seeded(doc: &Doc, seed: Option<u64>) -> Result<(Outcome, Vec<String>), String> {
    run_net(doc, seed, None)
}

/// As `run_seeded`, with a net for each `kind = "net"` brain that names
/// no `file` (the trainer's, LR 99). A team's nets share one queue of
/// calls.
pub fn run_net(doc: &Doc, seed: Option<u64>, net: Option<&crate::net::Net>) -> Result<(Outcome, Vec<String>), String> {
    run_with(doc, seed, net, None)
}

/// As `run_net`, with `make` giving the brain for each `kind = "net"`
/// seat instead (a probe of the net's actions, LR 100).
pub fn run_with(doc: &Doc, seed: Option<u64>, net: Option<&crate::net::Net>, make: Option<&dyn Fn() -> Box<dyn Brain>>) -> Result<(Outcome, Vec<String>), String> {
    run_inner(doc, seed, net, make, None)
}

/// As `run_net`, recording a frame every `every` ticks round the
/// learner's start (LR 102, the clips).
pub fn run_record(doc: &Doc, seed: Option<u64>, net: Option<&crate::net::Net>, every: Tick) -> Result<(Outcome, Vec<String>, Recording), String> {
    let mut rec = Recording { every: every.max(1), ..Default::default() };
    let (o, f) = run_inner(doc, seed, net, None, Some(&mut rec))?;
    Ok((o, f, rec))
}

/// Every run's body: `run_with`'s, recording into `rec` when given.
fn run_inner(doc: &Doc, seed: Option<u64>, net: Option<&crate::net::Net>, make: Option<&dyn Fn() -> Box<dyn Brain>>, mut rec: Option<&mut Recording>) -> Result<(Outcome, Vec<String>), String> {
    let seed = seed.or(doc.top.get("seed").and_then(Value::num).map(|s| s as u64));
    let num = |t: &BTreeMap<String, Value>, k: &str, d: f64| t.get(k).and_then(Value::num).unwrap_or(d);
    let players = num(&doc.top, "players", 2.0) as usize;
    let seconds = num(&doc.top, "seconds", 60.0);
    // `teams = [0, 1, 0]`: a team for each seat (none is each alone).
    let teams = match doc.top.get("teams") {
        Some(Value::List(v)) => Some(v.iter().map(|t| t.num().map(|n| n as TeamId).ok_or("teams: numbers")).collect::<Result<Vec<_>, _>>()?),
        None => None,
        Some(_) => return Err("teams: a list".into()),
    };
    let mut sc = Scenario { swarm_interval: 0, time_limit: 0, teams, ..Scenario::bots(players) };
    if let Some(s) = seed {
        sc.seed = s;
    }
    let mut w = World::new(sc);
    if doc.top.get("pillboxes") == Some(&Value::Bool(false)) {
        w.pillboxes.clear();
    }
    let seat_pos = |w: &World, s: PlayerId| w.tanks.iter().find(|t| t.owner == s).map(|t| t.pos).ok_or(format!("no seat {}", s));
    let mut depots: Vec<(EntityId, V2)> = Vec::new();
    let mut pillboxes: Vec<EntityId> = Vec::new();
    let empty = Vec::new();
    let list = |name: &str| doc.lists.get(name).unwrap_or(&empty);
    // Anchor and offset to a point.
    let place = |w: &World, depots: &[(EntityId, V2)], pillboxes: &[EntityId], anchor: &str, off: Option<&Value>| -> Result<V2, String> {
        // `"centre"`, the map's middle, takes no number.
        let (kind, k) = if anchor == "centre" { ("centre", "0") } else { anchor.split_once(':').ok_or(format!("not an anchor: {}", anchor))? };
        let k: usize = k.parse().map_err(|_| format!("not an anchor: {}", anchor))?;
        let base = match kind {
            "seat" => seat_pos(w, k as PlayerId)?,
            "centre" => w.map.center(),
            "depot" => depots.get(k).map(|d| d.1).ok_or(format!("no {}", anchor))?,
            "pillbox" => pillboxes.get(k).and_then(|id| w.pillboxes.iter().find(|p| p.id == *id)).map(|p| p.pos).ok_or(format!("no {}", anchor))?,
            _ => return Err(format!("not an anchor: {}", anchor)),
        };
        Ok(base + offset(off)?)
    };
    // `[[terrain]]`: a rectangle of tiles, `x` and `y` in tiles from the
    // anchor's tile, both ends in, set to water, grass or forest.
    for t in list("terrain") {
        let near = place(&w, &depots, &pillboxes, t.get("near").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let kind = match t.get("kind").and_then(Value::str).unwrap_or("water") {
            "water" => Terrain::Water,
            "grass" => Terrain::Grass,
            "forest" => Terrain::Forest,
            k => return Err(format!("terrain: no kind {}", k)),
        };
        let span = |k: &str| match t.get(k) {
            Some(Value::List(v)) if v.len() == 2 => Ok((v[0].num().unwrap_or(0.0) as i32, v[1].num().unwrap_or(0.0) as i32)),
            _ => Err(format!("terrain: {} = [from, to]", k)),
        };
        let ((x0, x1), (y0, y1)) = (span("x")?, span("y")?);
        let c = near.tile();
        for dy in y0..=y1 {
            for dx in x0..=x1 {
                w.map.set(V2::new((c.0 + dx) as f32 + 0.5, (c.1 + dy) as f32 + 0.5), kind);
            }
        }
    }
    // The dice of a run (`[[tank]] heading = "any"`, a `[[shell]]`'s
    // `scatter`, `from_any` and a `lands_in` range), from its seed, so a
    // run over many seeds samples the situations as well as the maps.
    let mut dice = Rng::new(seed.unwrap_or(0) ^ 0x5ce4_a210);
    // `[[tank]]`: a seat's tank put at an anchor and offset at the
    // start, afloat with `boat = true`; rolling with `speed` (tiles a
    // second) on `heading` (degrees, 0 east, 90 south; "any" drawn by
    // the dice) toward a goal `drive` tiles on, so it keeps rolling
    // until its brain orders otherwise. `around = true` turns the offset
    // round its anchor by an angle the dice draw; `reload` is seconds
    // before its gun is ready, or a time drawn from `[lo, hi]` (LR 57).
    // `toward = "seat:0"` heads it at that place instead (LR 61).
    for t in list("tank") {
        let seat = num(t, "seat", 0.0) as PlayerId;
        let anchor = place(&w, &depots, &pillboxes, t.get("at").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let mut off = offset(t.get("offset"))?;
        if t.get("around") == Some(&Value::Bool(true)) {
            off = V2::from_heading(dice.range(-std::f32::consts::PI, std::f32::consts::PI)).scale(off.len());
        }
        let to = anchor + off;
        let tank = w.tanks.iter_mut().find(|x| x.owner == seat).ok_or(format!("no seat {}", seat))?;
        tank.pos = Map::tile_centre(to);
        tank.boat = t.get("boat") == Some(&Value::Bool(true));
        tank.goal = None;
        tank.route.clear();
        tank.heading = match t.get("heading") {
            Some(Value::Str(s)) if s == "any" => dice.range(-std::f32::consts::PI, std::f32::consts::PI),
            Some(v) => (v.num().ok_or("heading: degrees or \"any\"")? as f32).to_radians(),
            None => tank.heading,
        };
        if let Some(a) = t.get("toward").and_then(Value::str) {
            let at = place(&w, &depots, &pillboxes, a, None)?;
            let tank = w.tanks.iter_mut().find(|x| x.owner == seat).expect("the tank");
            tank.heading = (at - tank.pos).heading();
        }
        let tank = w.tanks.iter_mut().find(|x| x.owner == seat).expect("the tank");
        tank.speed = num(t, "speed", 0.0) as f32;
        let reload = match t.get("reload") {
            Some(Value::List(v)) if v.len() == 2 => dice.range(v[0].num().unwrap_or(0.0) as f32, v[1].num().unwrap_or(0.0) as f32) as f64,
            _ => num(t, "reload", 0.0),
        };
        tank.reload = (reload * TICK_HZ as f64) as u16;
        // `armor` (a share of full) and `shells` (a count): a tank that
        // starts hurt and short (LR 100, the survival maps).
        if let Some(a) = t.get("armor").and_then(Value::num) {
            tank.armor = TANK_ARMOR * a as f32;
        }
        if let Some(n) = t.get("shells").and_then(Value::num) {
            tank.shells = n as u16;
        }
        if tank.speed > 0.0 {
            tank.goal = Some(tank.pos + V2::from_heading(tank.heading).scale(num(t, "drive", 20.0) as f32));
        }
    }
    for d in list("depot") {
        let near = place(&w, &depots, &pillboxes, d.get("near").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let dep = w.depots.iter().filter(|x| !depots.iter().any(|(id, _)| *id == x.id)).min_by(|a, b| a.pos.dist(near).total_cmp(&b.pos.dist(near))).ok_or("no depot left")?;
        let (id, mut pos) = (dep.id, dep.pos);
        // `at` and `offset`: that depot moved to the tile there, which
        // must be open ground.
        if let Some(a) = d.get("at").and_then(Value::str) {
            pos = Map::tile_centre(place(&w, &depots, &pillboxes, a, d.get("offset"))?);
            if !w.map.at(pos).passable() {
                return Err(format!("depot at ({:.0},{:.0}): not open ground", pos.x, pos.y));
            }
        }
        let owner = d.get("owner").and_then(Value::num).map(|o| o as PlayerId);
        let dep = w.depots.iter_mut().find(|x| x.id == id).expect("the depot");
        dep.owner = owner;
        dep.pos = pos;
        depots.push((id, pos));
    }
    // `depots = false`: only the `[[depot]]`s placed stay (LR 62; a naval
    // start, the side holding none).
    if doc.top.get("depots") == Some(&Value::Bool(false)) {
        w.depots.retain(|d| depots.iter().any(|(id, _)| *id == d.id));
    }
    for (k, p) in list("pillbox").iter().enumerate() {
        // As the hand-written tests place one: `away` tiles from the
        // anchor toward the map's centre, on open ground it can reach.
        let home = place(&w, &depots, &pillboxes, p.get("near").and_then(Value::str).unwrap_or("seat:0"), None)?;
        // Toward the map's centre, or `toward` a place (LR 68).
        let toward = match p.get("toward").and_then(Value::str) {
            Some(a) => (place(&w, &depots, &pillboxes, a, None)? - home).norm(),
            None => (w.map.center() - home).norm(),
        };
        let mut at = Map::tile_centre(home + toward.scale(num(p, "away", 20.0) as f32));
        let mut tries = 0;
        while !w.map.at(at).passable() || w.map.route(home, at).is_empty() {
            at = Map::tile_centre(at + toward);
            tries += 1;
            if tries > 200 {
                return Err(format!("pillbox {}: no open ground toward the centre", k));
            }
        }
        let owner = p.get("owner").and_then(Value::num).map(|o| o as PlayerId);
        let id = 70_001 + k as EntityId;
        w.pillboxes.push(Pillbox { id, pos: at, owner, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        pillboxes.push(id);
    }
    // `[[shell]]`: lands at an anchor and `offset`, plus up to `scatter`
    // tiles drawn by the dice; fired from `from` tiles off it, turned to
    // a direction drawn by the dice with `from_any = true`; in
    // `lands_in` seconds, or a time drawn from `[lo, hi]`.
    for s in list("shell") {
        let at = place(&w, &depots, &pillboxes, s.get("at").and_then(Value::str).unwrap_or("seat:0"), s.get("offset"))?;
        let at = at + dice.disc().scale(num(s, "scatter", 0.0) as f32);
        let mut from = offset(s.get("from"))?;
        if s.get("from_any") == Some(&Value::Bool(true)) {
            from = V2::from_heading(dice.range(-std::f32::consts::PI, std::f32::consts::PI)).scale(from.len());
        }
        let from = at + from;
        let id = w.next_id;
        w.next_id += 1;
        let tick = w.tick;
        let lands_in = match s.get("lands_in") {
            Some(Value::List(v)) if v.len() == 2 => dice.range(v[0].num().unwrap_or(3.0) as f32, v[1].num().unwrap_or(3.0) as f32) as f64,
            _ => num(s, "lands_in", 3.0),
        };
        let lands = (lands_in * TICK_HZ as f64) as Tick;
        w.shells.push(Shell { id, owner: num(s, "owner", 1.0) as PlayerId, kind: ShellKind::Indirect, from, aim: at, impact: at, launch_tick: tick, impact_tick: tick + lands, spotted: true });
    }
    // `[[drone]]` (LR 102, the gauntlet): a hostile drone put in the
    // air at an anchor and offset (`around = true` turns it), owned by
    // `owner`, flying as `kind`: a strike at `target` seat's tank, a
    // longshot at its place, a recon orbiting there.
    for d in list("drone") {
        let anchor = place(&w, &depots, &pillboxes, d.get("at").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let mut off = offset(d.get("offset"))?;
        if d.get("around") == Some(&Value::Bool(true)) {
            off = V2::from_heading(dice.range(-std::f32::consts::PI, std::f32::consts::PI)).scale(off.len());
        }
        let target = num(d, "target", 0.0) as PlayerId;
        let tank = w.tanks.iter().find(|t| t.owner == target).ok_or(format!("drone: no seat {}", target))?;
        let (kind, task) = match d.get("kind").and_then(Value::str).unwrap_or("strike") {
            "strike" => (DroneKind::Strike, DroneTask::Strike { target: tank.id }),
            "longshot" => (DroneKind::Longshot, DroneTask::Attack { at: tank.pos }),
            "recon" => (DroneKind::Recon, DroneTask::Orbit { center: tank.pos, radius: 2.0 }),
            k => return Err(format!("drone: no kind {}", k)),
        };
        let id = w.next_id;
        w.next_id += 1;
        let pos = anchor + off;
        w.drones.push(Drone { id, owner: num(d, "owner", 1.0) as PlayerId, kind, pos, heading: (tank.pos - pos).heading(), battery: 120.0, tether: None, task, link: 1.0, phase: 0.0, jammed: false, mark: 1 });
    }
    let mut brains: Vec<(PlayerId, Box<dyn Brain>)> = Vec::new();
    // `board = true`: each team's Commanders share one team board, as a
    // dealt match gives them (fleet.rs `equip`); by its first seat.
    let board = doc.top.get("board") == Some(&Value::Bool(true));
    let mut boards: Vec<(TeamId, crate::board::Shared)> = Vec::new();
    let mut calls: Vec<(TeamId, crate::calls::Calls)> = Vec::new();
    for b in list("brain") {
        let seat = num(b, "seat", 0.0) as PlayerId;
        let seed = num(b, "seed", 5.0) as u64 ^ seed.unwrap_or(0);
        let brain: Box<dyn Brain> = match b.get("kind").and_then(Value::str).unwrap_or("commander") {
            "rulebot" => {
                // A plain RuleBot reads no Settings; `weights` here sets
                // the landfall's three, the only ones it has as fields.
                let mut r = RuleBot::new(seed, "scenario");
                if let Some(t) = b.get("weights") {
                    for (k, v) in t.table().ok_or("weights: an inline table")? {
                        let v = v.num().ok_or(format!("weight {}: a number", k))? as f32;
                        match k.as_str() {
                            "landfall_shy" => r.landfall_shy = v,
                            "landfall_wait" => r.landfall_wait = v,
                            "landfall_memory" => r.landfall_memory = v,
                            _ => return Err(format!("a rulebot has no weight {}", k)),
                        }
                    }
                }
                Box::new(r)
            }
            "commander" => {
                let mut c = Commander::new(seed, "scenario");
                // The default Settings, as a dealt bot has them (fleet.rs
                // `equip`); `weights = { name = value }` changes some.
                let mut s = crate::weights::Settings::default();
                if let Some(t) = b.get("weights") {
                    for (k, v) in t.table().ok_or("weights: an inline table")? {
                        if k == "caps" {
                            let line = format!("caps = {}", v.str().ok_or("caps: a string")?);
                            s.caps = crate::weights::Settings::parse(&line).map_err(|e| format!("{:?}", e))?.caps;
                            continue;
                        }
                        let w = crate::weights::W::by_name(k).ok_or(format!("no weight {}", k))?;
                        s.w[w] = v.num().ok_or(format!("weight {}: a number", k))? as f32;
                    }
                }
                if board {
                    let team = w.players.iter().find(|p| p.id == seat).map(|p| p.team).ok_or(format!("no seat {}", seat))?;
                    let shared = match boards.iter().find(|(t, _)| *t == team) {
                        Some((_, b)) => b.clone(),
                        None => {
                            let b = crate::board::shared();
                            boards.push((team, b.clone()));
                            b
                        }
                    };
                    c.board = Some(shared);
                }
                // `rungs = ["expand", "guard"]`: the Auto ladder's rungs on
                // (all by default), by their labels.
                if let Some(Value::List(v)) = b.get("rungs") {
                    c.ladder = [false; 6];
                    for r in v {
                        let name = r.str().ok_or("rungs: labels")?;
                        let k = crate::commander::Rung::ALL.iter().position(|x| x.label() == name).ok_or(format!("no rung {}", name))?;
                        c.ladder[k] = true;
                    }
                }
                c.set_settings(s);
                // `pings = true`: it calls what it sees and answers
                // calls, a rally (as fleet.rs deals a leader).
                if b.get("pings") == Some(&Value::Bool(true)) {
                    c.pings = true;
                    c.answers_pings = true;
                }
                let goal = b.get("goal").and_then(Value::str).unwrap_or("auto");
                c.set_goal(match goal.split_once(':').map(|(g, k)| (g, k.parse::<usize>().ok())).unwrap_or((goal, None)) {
                    ("auto", _) => Goal::Auto,
                    ("manual", _) => Goal::Manual,
                    ("hunt_pillboxes", _) => Goal::HuntPillboxes,
                    ("defend_base", _) => Goal::DefendBase,
                    ("defend_depot", k) => Goal::DefendDepot(k.and_then(|k| depots.get(k)).map(|d| d.0)),
                    ("defend_pillbox", k) => Goal::DefendPillbox(k.and_then(|k| pillboxes.get(k)).copied()),
                    (g, _) => return Err(format!("no goal {}", g)),
                });
                Box::new(c)
            }
            // `kind = "net"`: a learned tank (LR 99), the net in `file`,
            // else the one the trainer gives.
            "net" if make.is_some() => make.expect("a maker")(),
            "net" => {
                let n = match b.get("file").and_then(Value::str) {
                    Some(f) => crate::net::Net::load(&find(f)).map_err(|e| e.to_string())?,
                    None => net.cloned().ok_or("a net brain: no file, and no net given (make train GROUND=)")?,
                };
                let team = w.players.iter().find(|p| p.id == seat).map(|p| p.team).ok_or(format!("no seat {}", seat))?;
                let q = match calls.iter().find(|(t, _)| *t == team) {
                    Some((_, q)) => q.clone(),
                    None => {
                        let q = crate::calls::Calls::new();
                        calls.push((team, q.clone()));
                        q
                    }
                };
                Box::new(crate::net::NetBrain::new(n, "scenario").with_calls(q))
            }
            k => return Err(format!("no brain kind {}", k)),
        };
        brains.push((seat, brain));
    }
    let mut out = Outcome { pillbox_damage: vec![0.0; pillboxes.len()], pillbox_owned: vec![BTreeMap::new(); pillboxes.len()], depot_owned: vec![BTreeMap::new(); depots.len()], ..Default::default() };
    for t in &w.tanks {
        out.start.insert(t.owner, (t.pos, t.shells));
        out.armor.insert(t.owner, (t.armor, t.armor));
    }
    out.seconds = seconds;
    if let Some(Value::Table(t)) = doc.reward.get("closer") {
        let at = place(&w, &depots, &pillboxes, t.get("at").and_then(Value::str).unwrap_or("centre"), t.get("offset"))?;
        out.points.insert("closer".into(), at);
    }
    for key in ["ends_near", "lands_near", "met_beyond"] {
        if let Some(Value::Table(t)) = doc.expect.get(key) {
            let at = place(&w, &depots, &pillboxes, t.get("at").and_then(Value::str).unwrap_or("seat:0"), t.get("offset"))?;
            out.points.insert(key.into(), at);
        }
    }
    let mut afloat: BTreeMap<PlayerId, bool> = w.tanks.iter().map(|t| (t.owner, t.boat)).collect();
    if let Some(r) = rec.as_deref_mut() {
        let me = doc.top.get("learner").and_then(Value::num).unwrap_or(0.0) as PlayerId;
        let c = w.tanks.iter().find(|t| t.owner == me).map(|t| t.pos).unwrap_or(w.map.center());
        *r = Recording::round(&w, c, num(&doc.top, "view", 22.0) as i32, r.every);
    }
    let events = list("event");
    // `trace_every`: seconds between trace lines (5).
    let every = ((num(&doc.top, "trace_every", 5.0) * TICK_HZ as f64) as Tick).max(1);
    for _ in 0..(seconds * TICK_HZ as f64) as u64 {
        let now = w.tick as f64 / TICK_HZ as f64;
        for e in events.iter().filter(|e| (num(e, "at", -1.0) * TICK_HZ as f64) as Tick == w.tick) {
            // `drones = "recall"`: every drone gone (a tank moved by
            // hand leaves its drone behind, seeing for it).
            if e.get("drones").and_then(Value::str) == Some("recall") {
                out.trace.push(format!("{:>5.1} s  {} drones recalled", now, w.drones.len()));
                w.drones.clear();
            }
            let seat = num(e, "move", -1.0);
            if seat >= 0.0 {
                let mut to = place(&w, &depots, &pillboxes, e.get("to").and_then(Value::str).unwrap_or("seat:0"), e.get("offset"))?;
                // `toward` a place by `step` tiles (LR 68): a spotter set
                // beside a work on the side its quarry is.
                if let Some(a) = e.get("toward").and_then(Value::str) {
                    let q = place(&w, &depots, &pillboxes, a, None)?;
                    to = to + (q - to).norm().scale(num(e, "step", 2.0) as f32);
                }
                let t = w.tanks.iter_mut().find(|t| t.owner == seat as PlayerId).ok_or(format!("no seat {}", seat))?;
                t.pos = to;
                t.goal = None;
                t.route.clear();
                // `boat = true`: put to sea (the place must be water).
                if e.get("boat") == Some(&Value::Bool(true)) {
                    t.boat = true;
                    t.pos = Map::tile_centre(to);
                }
            }
        }
        let mut orders = Vec::new();
        for (seat, b) in brains.iter_mut() {
            let obs = w.observe(*seat);
            // With the map, as `run_bots` gives it in a match.
            if obs.me.as_ref().map(|t| t.alive).unwrap_or(false) {
                orders.extend(b.think_on(&obs, Some(&w.map)).into_iter().map(|cmd| Order { player: *seat, cmd }));
            }
        }
        w.step(&orders);
        for e in &w.events {
            match e {
                Event::TankDamaged { tank, by, damage, .. } => {
                    if let Some(t) = w.tanks.iter().find(|t| t.id == *tank) {
                        out.hits.push((*by, t.owner));
                        out.hit_at.push(t.pos);
                        out.hit_damage.push(*damage);
                        let drones: Vec<String> = w.drones.iter().map(|d| format!("{:?} of {} at ({:.1},{:.1})", d.kind, d.owner, d.pos.x, d.pos.y)).collect();
                        out.trace.push(format!("{:>5.2} s  seat {} hit by {:?} for {:.0}; drones: {}", w.tick as f64 / TICK_HZ as f64, t.owner, by, damage, drones.join(", ")));
                    }
                }
                Event::Pinged { player, kind: PingKind::Help, .. } => *out.advance.entry((*player, "called")).or_insert(0) += 1,
                Event::Chained { owner, on: true, .. } => *out.advance.entry((*owner, "chained")).or_insert(0) += 1,
                Event::Harvested { owner, .. } => *out.advance.entry((*owner, "wood")).or_insert(0) += 1,
                Event::Built { owner, what: BuildKind::Road, .. } => *out.advance.entry((*owner, "road")).or_insert(0) += 1,
                Event::Rejected { player, why, .. } if brains.iter().any(|(s, _)| s == player) => {
                    out.refused.push((w.tick as f64 / TICK_HZ as f64, *player, why.to_string()));
                }
                Event::ShellLaunched { owner, kind, .. } => {
                    let at = w.tick as f64 / TICK_HZ as f64;
                    if *kind == ShellKind::Direct { out.first_fire.entry(*owner).or_insert(at) } else { out.first_lob.entry(*owner).or_insert(at) };
                }
                Event::DroneLaunched { owner, kind, .. } => {
                    out.first_launch.entry((*owner, format!("{:?}", kind).to_lowercase())).or_insert(w.tick as f64 / TICK_HZ as f64);
                }
                Event::PillboxFired { pillbox, damage, .. } => {
                    if let Some(k) = pillboxes.iter().position(|p| p == pillbox) {
                        out.pillbox_damage[k] += damage;
                    }
                }
                _ => {}
            }
        }
        for t in w.tanks.iter().filter(|t| !t.alive) {
            out.died_at.entry(t.owner).or_insert(w.tick as f64 / TICK_HZ as f64);
        }
        for t in w.tanks.iter().filter(|t| t.alive) {
            if out.start.get(&t.owner).map(|s| s.0.dist(t.pos) > 1.0).unwrap_or(false) {
                out.first_move.entry(t.owner).or_insert(w.tick as f64 / TICK_HZ as f64);
            }
        }
        if let Some(r) = rec.as_deref_mut() {
            if w.tick % r.every == 0 {
                r.frames.push(snap(&w));
            }
        }
        for t in &w.tanks {
            if afloat.get(&t.owner) == Some(&true) && !t.boat && t.alive {
                out.landed.entry(t.owner).or_insert(t.pos);
            }
            afloat.insert(t.owner, t.boat);
        }
        for (k, id) in pillboxes.iter().enumerate() {
            if let Some(o) = w.pillboxes.iter().find(|p| p.id == *id).and_then(|p| p.owner) {
                out.pillbox_owned[k].entry(o).or_insert(w.tick);
            }
        }
        for (k, (id, _)) in depots.iter().enumerate() {
            if let Some(o) = w.depots.iter().find(|d| d.id == *id).and_then(|d| d.owner) {
                out.depot_owned[k].entry(o).or_insert(w.tick);
            }
        }
        if w.over {
            out.over = Some(w.tick as f64 / TICK_HZ as f64);
            out.seconds = w.tick as f64 / TICK_HZ as f64;
            out.trace.push(format!("{:>5.1} s  the match is over", w.tick as f64 / TICK_HZ as f64));
            break;
        }
        if w.tick % every == 0 {
            // Where each tank is, and where its route ends (`> (x,y)`).
            let at = |s: PlayerId| {
                w.tanks
                    .iter()
                    .find(|t| t.owner == s)
                    .map(|t| {
                        let dest = t.route.last().copied().or(t.goal).map(|g| format!(" > ({:.1},{:.1}) {} legs", g.x, g.y, t.route.len() + 1)).unwrap_or_default();
                        format!("({:.1},{:.1}) on {:?} at {:.1} heading {:.0}{}{}", t.pos.x, t.pos.y, w.map.at(t.pos), t.speed, t.heading.to_degrees().rem_euclid(360.0), if t.boat { " afloat" } else { "" }, dest)
                    })
                    .unwrap_or_default()
            };
            let shells: String = w.shells.iter().filter(|s| s.kind == ShellKind::Indirect).map(|s| format!("; a shell lands at ({:.1},{:.1}) in {:.2} s", s.impact.x, s.impact.y, s.impact_tick.saturating_sub(w.tick) as f32 * DT)).collect();
            out.trace.push(format!("{:>5.1} s  {}{}", now, brains.iter().map(|(s, b)| format!("seat {} at {} {} ({})", s, at(*s), b.act().code(), b.status())).collect::<Vec<_>>().join(", "), shells));
        }
    }
    for t in &w.tanks {
        out.end.insert(t.owner, (t.pos, t.shells));
        out.alive.insert(t.owner, t.alive);
        if let Some(a) = out.armor.get_mut(&t.owner) {
            a.1 = if t.alive { t.armor } else { 0.0 };
        }
    }
    for id in &pillboxes {
        out.pillbox_alive.push(w.pillboxes.iter().find(|p| p.id == *id).map(|p| p.alive()).unwrap_or(false));
    }
    let failed = check(&doc.expect, &out)?;
    Ok((out, failed))
}

/// An offset in tiles, `[x, y]`; none is no offset.
fn offset(v: Option<&Value>) -> Result<V2, String> {
    match v {
        Some(Value::List(v)) if v.len() == 2 => Ok(V2::new(v[0].num().unwrap_or(0.0) as f32, v[1].num().unwrap_or(0.0) as f32)),
        None => Ok(V2::new(0.0, 0.0)),
        Some(o) => Err(format!("an offset is [x, y], not {:?}", o)),
    }
}

/// Every expectation that failed, in words.
fn check(expect: &BTreeMap<String, Value>, out: &Outcome) -> Result<Vec<String>, String> {
    let mut failed = Vec::new();
    let field = |t: &BTreeMap<String, Value>, k: &str| t.get(k).and_then(Value::num);
    for (key, v) in expect {
        let t = v.table().ok_or(format!("expect {}: an inline table", key))?;
        let seat = field(t, "seat").map(|s| s as PlayerId);
        match key.as_str() {
            "hits_on" => {
                let s = seat.ok_or("hits_on: a seat")?;
                let n = out.hits.iter().filter(|(_, on)| *on == s).count() as f64;
                if let Some(m) = field(t, "max").filter(|m| n > *m) {
                    failed.push(format!("seat {} was hit {} times, at most {} expected", s, n, m));
                }
            }
            "hits" => {
                let (by, on) = (field(t, "by").ok_or("hits: by")? as PlayerId, field(t, "on").ok_or("hits: on")? as PlayerId);
                let n = out.hits.iter().filter(|(b, o)| *b == Some(by) && *o == on).count() as f64;
                if let Some(m) = field(t, "at_least").filter(|m| n < *m) {
                    failed.push(format!("seat {} hit seat {} {} times, at least {} expected", by, on, n, m));
                }
            }
            "outscores" => {
                let (by, on) = (field(t, "by").ok_or("outscores: by")? as PlayerId, field(t, "on").ok_or("outscores: on")? as PlayerId);
                let given = out.hits.iter().filter(|(b, o)| *b == Some(by) && *o == on).count();
                let taken = out.hits.iter().filter(|(b, o)| *b == Some(on) && *o == by).count();
                if given <= taken {
                    failed.push(format!("seat {} hit seat {} {} times and was hit {} times", by, on, given, taken));
                }
            }
            "moved" => {
                let s = seat.ok_or("moved: a seat")?;
                let d = out.start.get(&s).zip(out.end.get(&s)).map(|(a, b)| a.0.dist(b.0) as f64).unwrap_or(0.0);
                if let Some(m) = field(t, "more_than").filter(|m| d <= *m) {
                    failed.push(format!("seat {} moved {:.1} tiles, more than {} expected", s, d, m));
                }
            }
            "shells_spent" => {
                let s = seat.ok_or("shells_spent: a seat")?;
                let spent = out.start.get(&s).zip(out.end.get(&s)).map(|(a, b)| a.1 as f64 - b.1 as f64).unwrap_or(0.0);
                if let Some(m) = field(t, "max").filter(|m| spent > *m) {
                    failed.push(format!("seat {} spent {} shells, at most {} expected", s, spent, m));
                }
            }
            "pillbox_owned_by" => {
                let (k, s) = (field(t, "pillbox").unwrap_or(0.0) as usize, seat.ok_or("pillbox_owned_by: a seat")?);
                let by = field(t, "by").map(|b| (b * TICK_HZ as f64) as Tick);
                match out.pillbox_owned.get(k).and_then(|o| o.get(&s)) {
                    Some(at) if by.map(|b| *at <= b).unwrap_or(true) => {}
                    Some(at) => failed.push(format!("pillbox {} became seat {}'s at {:.1} s, by {} s expected", k, s, *at as f64 / TICK_HZ as f64, field(t, "by").unwrap_or(0.0))),
                    None => failed.push(format!("pillbox {} never became seat {}'s", k, s)),
                }
            }
            "depot_owned_by" => {
                let (k, s) = (field(t, "depot").unwrap_or(0.0) as usize, seat.ok_or("depot_owned_by: a seat")?);
                let by = field(t, "by").map(|b| (b * TICK_HZ as f64) as Tick);
                match out.depot_owned.get(k).and_then(|o| o.get(&s)) {
                    Some(at) if by.map(|b| *at <= b).unwrap_or(true) => {}
                    Some(at) => failed.push(format!("depot {} became seat {}'s at {:.1} s, by {} s expected", k, s, *at as f64 / TICK_HZ as f64, field(t, "by").unwrap_or(0.0))),
                    None => failed.push(format!("depot {} never became seat {}'s", k, s)),
                }
            }
            // `depot_first = { seat = 0, depot = 1 }`: of the placed depots,
            // the one the seat took first (one it started with is not
            // taken).
            "depot_first" => {
                let (k, s) = (field(t, "depot").unwrap_or(0.0) as usize, seat.ok_or("depot_first: a seat")?);
                let first = out.depot_owned.iter().enumerate().filter_map(|(i, o)| o.get(&s).filter(|at| **at > 1).map(|at| (*at, i))).min();
                match first {
                    Some((_, i)) if i == k => {}
                    Some((at, i)) => failed.push(format!("seat {} took depot {} first, at {:.1} s, not depot {}", s, i, at as f64 / TICK_HZ as f64, k)),
                    None => failed.push(format!("seat {} took none of the depots", s)),
                }
            }
            "ends_near" => {
                let s = seat.ok_or("ends_near: a seat")?;
                let at = out.points.get("ends_near").copied().ok_or("ends_near: no point")?;
                let d = out.end.get(&s).map(|e| e.0.dist(at) as f64).unwrap_or(f64::INFINITY);
                if let Some(m) = field(t, "within").filter(|m| d > *m) {
                    failed.push(format!("seat {} ended {:.1} tiles from the point, within {} expected", s, d, m));
                }
            }
            "lands_near" => {
                let s = seat.ok_or("lands_near: a seat")?;
                let at = out.points.get("lands_near").copied().ok_or("lands_near: no point")?;
                match out.landed.get(&s) {
                    Some(l) if field(t, "within").map(|m| l.dist(at) as f64 <= m).unwrap_or(true) => {}
                    Some(l) => failed.push(format!("seat {} came ashore {:.1} tiles from the point, within {} expected", s, l.dist(at), field(t, "within").unwrap_or(0.0))),
                    None => failed.push(format!("seat {} never came ashore", s)),
                }
            }
            // `met_beyond = { by = 0, on = 1, at = "seat:0", more_than = 12 }`:
            // the first hit `by` took on `on` was that far from the point
            // (LR 61, an attack met before it reaches the base).
            "met_beyond" => {
                let (by, on) = (field(t, "by").ok_or("met_beyond: by")? as PlayerId, field(t, "on").ok_or("met_beyond: on")? as PlayerId);
                let at = out.points.get("met_beyond").copied().ok_or("met_beyond: no point")?;
                let first = out.hits.iter().zip(&out.hit_at).find(|((b, o), _)| *b == Some(by) && *o == on).map(|(_, p)| p.dist(at) as f64);
                match (first, field(t, "more_than")) {
                    (Some(d), Some(m)) if d <= m => failed.push(format!("seat {} first hit seat {} {:.1} tiles from the point, more than {} expected", by, on, d, m)),
                    (None, _) => failed.push(format!("seat {} never hit seat {}", by, on)),
                    _ => {}
                }
            }
            "pillbox_damage" => {
                let k = field(t, "pillbox").unwrap_or(0.0) as usize;
                let d = out.pillbox_damage.get(k).copied().unwrap_or(0.0) as f64;
                if let Some(m) = field(t, "max").filter(|m| d > *m) {
                    failed.push(format!("pillbox {} did {} damage, at most {} expected", k, d, m));
                }
            }
            // `alive = { seat = 0 }`, `dead = { seat = 1 }`: the tank's
            // state at the end (LR 99).
            "alive" | "dead" => {
                let s = seat.ok_or("alive, dead: a seat")?;
                let up = out.alive.get(&s).copied().unwrap_or(false);
                if up != (key == "alive") {
                    failed.push(format!("seat {} {} at the end", s, if up { "alive" } else { "dead" }));
                }
            }
            // `fires_within = { seat = 0, seconds = 3 }`: a direct shot by
            // then; `launched = { seat = 0, kind = "interceptor", within =
            // 2 }`: that drone launched by then (LR 102, the gauntlet).
            "fires_within" => {
                let s = seat.ok_or("fires_within: a seat")?;
                let by = field(t, "seconds").unwrap_or(f64::INFINITY);
                match out.first_fire.get(&s) {
                    Some(at) if *at <= by => {}
                    Some(at) => failed.push(format!("seat {} fired first at {:.1} s, by {} s expected", s, at, by)),
                    None => failed.push(format!("seat {} never fired", s)),
                }
            }
            "launched" => {
                let s = seat.ok_or("launched: a seat")?;
                let kind = t.get("kind").and_then(Value::str).unwrap_or("interceptor").to_string();
                let by = field(t, "within").unwrap_or(f64::INFINITY);
                match out.first_launch.get(&(s, kind.clone())) {
                    Some(at) if *at <= by => {}
                    Some(at) => failed.push(format!("seat {} launched a {} first at {:.1} s, by {} s expected", s, kind, at, by)),
                    None => failed.push(format!("seat {} launched no {}", s, kind)),
                }
            }
            // `armor_at_least = { seat = 0, share = 0.6 }`: alive at the
            // end with at least that share of full armour (LR 100).
            "armor_at_least" => {
                let s = seat.ok_or("armor_at_least: a seat")?;
                let want = field(t, "share").unwrap_or(1.0) as f32;
                let have = out.armor.get(&s).map(|a| a.1 / TANK_ARMOR).unwrap_or(0.0);
                if have + 1e-4 < want {
                    failed.push(format!("seat {} ended with {:.0}% armour, {:.0}% expected", s, 100.0 * have, 100.0 * want));
                }
            }
            "pillbox_alive" => {
                let k = field(t, "pillbox").unwrap_or(0.0) as usize;
                let want = t.get("alive") != Some(&Value::Bool(false));
                if out.pillbox_alive.get(k).copied() != Some(want) {
                    failed.push(format!("pillbox {} {} at the end", k, if want { "dead" } else { "standing" }));
                }
            }
            k => return Err(format!("no expectation {}", k)),
        }
    }
    Ok(failed)
}

/// The reaction a file measures (`reaction = "fire"`, `"lob"`,
/// `"move"`, or `"launch:interceptor"`), for the learner, seconds; none
/// when it never came (LR 102, the gauntlet).
pub fn reaction(doc: &Doc, out: &Outcome) -> Option<f64> {
    let me = doc.top.get("learner").and_then(Value::num).unwrap_or(0.0) as PlayerId;
    match doc.top.get("reaction").and_then(Value::str)? {
        "fire" => out.first_fire.get(&me).copied(),
        "lob" => out.first_lob.get(&me).copied(),
        "move" => out.first_move.get(&me).copied(),
        r => r.strip_prefix("launch:").and_then(|k| out.first_launch.get(&(me, k.to_string())).copied()),
    }
}

/// What a training episode earned (LR 99): the sum of the file's
/// `[reward]` terms for the learner's seat (`learner`, 0). Each term's
/// value is its worth:
///
/// - `pass`: when every expectation held;
/// - `alive`: the learner's tank alive at the end;
/// - `damage_taken`, `damage_given`: a point of damage to or by it;
/// - `killed`: each other seat's tank dead at the end;
/// - `depot_taken`, `pillbox_taken`: each placed one the learner owned;
/// - `pillbox_killed`: each placed pillbox dead at the end;
/// - `closer = { at = "depot:0", worth = 1 }`: the share of the way to
///   the point closed, start to end (a potential: no pay for circling);
/// - `alive_time`: the share of the run the learner's tank lived;
/// - `armor_gained`: its armour at the end less at the start, a share
///   of full (dead is none).
pub fn reward(doc: &Doc, out: &Outcome, failed: &[String]) -> f32 {
    let me = doc.top.get("learner").and_then(Value::num).unwrap_or(0.0) as PlayerId;
    let mut r = 0.0f32;
    for (k, v) in &doc.reward {
        let w = v.num().or_else(|| v.table().and_then(|t| t.get("worth")).and_then(Value::num)).unwrap_or(0.0) as f32;
        r += w * match k.as_str() {
            "pass" => failed.is_empty() as u8 as f32,
            "called" | "chained" | "wood" | "road" => out.advance.iter().find(|((s, w), _)| *s == me && *w == k.as_str()).map_or(0.0, |(_, n)| *n as f32),
            "alive" => out.alive.get(&me).copied().unwrap_or(false) as u8 as f32,
            "damage_taken" => out.hits.iter().zip(&out.hit_damage).filter(|((_, on), _)| *on == me).map(|(_, d)| *d).sum(),
            "damage_given" => out.hits.iter().zip(&out.hit_damage).filter(|((by, on), _)| *by == Some(me) && *on != me).map(|(_, d)| *d).sum(),
            "killed" => out.alive.iter().filter(|(s, up)| **s != me && !**up).count() as f32,
            "depot_taken" => out.depot_owned.iter().filter(|o| o.contains_key(&me)).count() as f32,
            "pillbox_taken" => out.pillbox_owned.iter().filter(|o| o.contains_key(&me)).count() as f32,
            "pillbox_killed" => out.pillbox_alive.iter().filter(|a| !**a).count() as f32,
            "alive_time" => (out.died_at.get(&me).copied().unwrap_or(out.seconds) / out.seconds.max(1e-6)) as f32,
            "armor_gained" => out.armor.get(&me).map(|a| (a.1 - a.0) / TANK_ARMOR).unwrap_or(0.0),
            "closer" => match (out.points.get("closer"), out.start.get(&me), out.end.get(&me)) {
                (Some(p), Some(a), Some(b)) => ((a.0.dist(*p) - b.0.dist(*p)) / a.0.dist(*p).max(1.0)).clamp(-1.0, 1.0),
                _ => 0.0,
            },
            _ => 0.0,
        };
    }
    r
}

/// A scenario over `runs` seeds (`first`, `first + 1`, ...): how many
/// held, the share, and its standard error, sqrt(p (1 - p) / n). The
/// failures are kept, the first few for the report.
pub fn many(doc: &Doc, first: u64, runs: u64) -> Result<(u64, f64, f64, Vec<String>), String> {
    let (mut held, mut why) = (0u64, Vec::new());
    for k in 0..runs {
        let (_, failed) = run_seeded(doc, Some(first + k))?;
        if failed.is_empty() {
            held += 1;
        } else if why.len() < 5 {
            why.push(format!("seed {}: {}", first + k, failed.join("; ")));
        }
    }
    let p = held as f64 / runs.max(1) as f64;
    Ok((held, p, (p * (1.0 - p) / runs.max(1) as f64).sqrt(), why))
}

/// `SCENARIO_SET="name=value ..."`: those weights set on every brain in
/// the file that holds them (a RuleBot, only the landfall's three),
/// added to its `weights` or a new one, so two settings run on the same
/// seeds.
///
/// `caps=baseline,-pings` sets the switches as a weights file's `caps` line
/// does, on every Commander. A switch's A/B is a trial like a number's, and
/// until this it stopped here with "not a number": the families read the
/// panic as a floor broken, so no switch could ever be promoted (30 October).
pub fn set_weights(doc: &mut Doc, set: &str) -> Result<(), String> {
    let set = set.replace(", ", ",");
    for kv in set.split_whitespace() {
        let (k, v) = kv.split_once('=').ok_or(format!("not name=value: {}", kv))?;
        if k == "caps" {
            crate::weights::Settings::parse(&format!("caps = {}", v)).map_err(|e| format!("caps {}: {:?}", v, e))?;
            for b in doc.lists.get_mut("brain").into_iter().flatten() {
                if b.get("kind").and_then(Value::str) == Some("rulebot") {
                    continue;
                }
                let w = b.entry("weights".into()).or_insert_with(|| Value::Table(BTreeMap::new()));
                if let Value::Table(t) = w {
                    t.insert("caps".to_string(), Value::Str(v.to_string()));
                }
            }
            continue;
        }
        let v: f64 = v.parse().map_err(|_| format!("not a number: {}", v))?;
        for b in doc.lists.get_mut("brain").into_iter().flatten() {
            // A RuleBot holds only the landfall's three; the rest are a
            // Commander's.
            if b.get("kind").and_then(Value::str) == Some("rulebot") && !k.starts_with("landfall_") {
                continue;
            }
            let w = b.entry("weights".into()).or_insert_with(|| Value::Table(BTreeMap::new()));
            if let Value::Table(t) = w {
                t.insert(k.to_string(), Value::Num(v));
            }
        }
    }
    Ok(())
}

/// A path as given, or from the top of the tree when it is not found
/// there (`cargo test` runs in the crate's folder).
pub fn find(path: &str) -> std::path::PathBuf {
    let p = std::path::PathBuf::from(path);
    if p.exists() || p.is_absolute() {
        return p;
    }
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}

/// Read and run a file: its failures, or the error that stopped it.
pub fn run_file(path: &std::path::Path) -> Result<(Outcome, Vec<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    run(&parse(&text).map_err(|e| format!("{}: {}", path.display(), e))?).map_err(|e| format!("{}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_advance_scenario_earns_the_call_the_chain_and_the_capture() {
        let doc = parse(include_str!("../../../scenarios/a-tank-takes-a-pillbox-and-advances.toml")).unwrap();
        let (out, failed) = run(&doc).unwrap();
        assert!(failed.is_empty(), "{:?}", failed);
        eprintln!("advance stages {:?} reward {}", out.advance, reward(&doc, &out, &failed));
        for stage in ["called", "chained"] {
            assert!(out.advance.contains_key(&(0, stage)), "no {}: {:?}", stage, out.advance);
        }
    }

    /// The scenarios, at the top of the tree.
    fn dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios")
    }

    /// The TOML subset: strings with a `#`, numbers, lists, inline
    /// tables, `[[tables]]`, `[expect]`, and errors that name the line.
    /// `set_weights` adds a weight to every brain, creating `weights`
    /// where a brain has none.
    #[test]
    fn a_weight_is_set_on_every_brain() {
        let mut d = parse("[[brain]]\nseat = 0\n[[brain]]\nseat = 1\nweights = { landfall_shy = 1 }\n").unwrap();
        set_weights(&mut d, "landfall_memory=0").unwrap();
        for b in &d.lists["brain"] {
            assert_eq!(b["weights"].table().unwrap()["landfall_memory"], Value::Num(0.0));
        }
        assert_eq!(d.lists["brain"][1]["weights"].table().unwrap()["landfall_shy"], Value::Num(1.0));
        assert!(set_weights(&mut d, "nope").is_err());
    }

    /// A switch's A/B reaches the families: the form the fleet writes, its
    /// space and all, turns the switch off on a Commander and leaves a
    /// RuleBot alone, and a misspelt switch stops the read.
    #[test]
    fn a_switch_is_set_on_every_commander() {
        let mut d = parse("[[brain]]\nseat = 0\n[[brain]]\nseat = 1\nkind = \"rulebot\"\n").unwrap();
        set_weights(&mut d, "caps=baseline, -pings").unwrap();
        let caps = d.lists["brain"][0]["weights"].table().unwrap()["caps"].str().unwrap().to_string();
        let s = crate::weights::Settings::parse(&format!("caps = {}", caps)).unwrap();
        assert!(!s.caps.has(crate::weights::Caps::PINGS), "pings stayed on");
        assert!(s.caps.has(crate::weights::Caps::DODGE), "the rest of the baseline went too");
        assert!(d.lists["brain"][1].get("weights").is_none(), "a rulebot has no switches");
        assert!(set_weights(&mut d, "caps=baseline,-no_such_switch").is_err());
    }

    #[test]
    fn the_subset_reads_values_tables_and_lists() {
        let d = parse("name = \"a # b\"  # a comment\nn = 3\n[[brain]]\nseat = 0\n[[brain]]\nseat = 1\nat = [30, -2.5]\n[expect]\nmoved = { seat = 0, more_than = 2.5 }\n").unwrap();
        assert_eq!(d.top["name"], Value::Str("a # b".into()));
        assert_eq!(d.lists["brain"].len(), 2);
        assert_eq!(d.lists["brain"][1]["at"], Value::List(vec![Value::Num(30.0), Value::Num(-2.5)]));
        assert_eq!(d.expect["moved"].table().unwrap()["more_than"], Value::Num(2.5));
        assert!(parse("[nope]").unwrap_err().contains("line 1"));
        assert!(parse("x = y").unwrap_err().contains("not a value"));
    }

    /// Every file under `scenarios/` holds.
    #[test]
    fn every_scenario_holds() {
        let mut files: Vec<_> = std::fs::read_dir(dir()).expect("scenarios/").filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false)).collect();
        files.sort();
        assert!(!files.is_empty(), "no scenarios");
        let mut bad = Vec::new();
        for f in &files {
            match run_file(f) {
                Ok((_, failed)) if failed.is_empty() => eprintln!("scenario {}: holds", f.display()),
                Ok((_, failed)) => bad.push(format!("{}: {}", f.display(), failed.join("; "))),
                Err(e) => bad.push(e),
            }
        }
        assert!(bad.is_empty(), "{}", bad.join("\n"));
    }

    /// One file over many seeds (`RUNS`, 100, from `SEED0`, 1), with
    /// `SCENARIO_SET` applied: how often it holds. `make scenario
    /// S=path RUNS=100 SET="landfall_memory=0"`.
    #[test]
    #[ignore]
    fn scenario_many() {
        let path = std::env::var("SCENARIO").expect("SCENARIO=path");
        let runs: u64 = std::env::var("RUNS").ok().and_then(|s| s.parse().ok()).unwrap_or(100);
        let first: u64 = std::env::var("SEED0").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
        let mut doc = parse(&std::fs::read_to_string(&path).expect("the file")).unwrap_or_else(|e| panic!("{}", e));
        let set = std::env::var("SCENARIO_SET").unwrap_or_default();
        set_weights(&mut doc, &set).unwrap_or_else(|e| panic!("{}", e));
        let started = std::time::Instant::now();
        let (held, p, se, why) = many(&doc, first, runs).unwrap_or_else(|e| panic!("{}", e));
        for w in &why {
            eprintln!("  {}", w);
        }
        eprintln!("scenario {} [{}]: held {} of {}, {:.1}% +/- {:.1}%, seeds {} to {}, {:.1} s", path, set, held, runs, 100.0 * p, 100.0 * se, first, first + runs - 1, started.elapsed().as_secs_f32());
    }

    /// Every family (`scenarios/families/`, backlog 47) over its `runs`
    /// seeds (200) from 1, with `SCENARIO_SET` applied: each must hold
    /// at least its `floor` (a share, 0 to 1; none is only reported).
    /// `make families`.
    #[test]
    #[ignore]
    fn every_family_holds_its_floor() {
        let mut files: Vec<_> = std::fs::read_dir(dir().join("families")).expect("scenarios/families/").filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false)).collect();
        files.sort();
        let set = std::env::var("SCENARIO_SET").unwrap_or_default();
        let mut low = Vec::new();
        for f in &files {
            let mut doc = parse(&std::fs::read_to_string(f).expect("the file")).unwrap_or_else(|e| panic!("{}: {}", f.display(), e));
            set_weights(&mut doc, &set).unwrap_or_else(|e| panic!("{}", e));
            let runs = doc.top.get("runs").and_then(Value::num).unwrap_or(200.0) as u64;
            let floor = doc.top.get("floor").and_then(Value::num);
            let started = std::time::Instant::now();
            let (held, p, se, why) = many(&doc, 1, runs).unwrap_or_else(|e| panic!("{}: {}", f.display(), e));
            let name = f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let verdict = match floor {
                Some(fl) if p < fl => {
                    low.push(format!("{}: {:.1}% under its floor {:.1}% ({})", name, 100.0 * p, 100.0 * fl, why.first().cloned().unwrap_or_default()));
                    "UNDER"
                }
                Some(_) => "holds",
                None => "no floor",
            };
            eprintln!("family {} [{}]: held {} of {}, {:.1}% +/- {:.1}%, floor {}, {} ({:.1} s)", name, set, held, runs, 100.0 * p, 100.0 * se, floor.map(|f| format!("{:.1}%", 100.0 * f)).unwrap_or("none".into()), verdict, started.elapsed().as_secs_f32());
        }
        assert!(low.is_empty(), "{}", low.join("\n"));
    }

    /// One file, with its trace: `SCENARIO=path cargo test -p
    /// treadcore-brain --release -- --ignored scenario_one --nocapture`
    /// (`make scenario S=path`).
    #[test]
    #[ignore]
    fn scenario_one() {
        let path = std::env::var("SCENARIO").expect("SCENARIO=path");
        // `SEED`: one of `scenario_many`'s seeds, to trace a failure.
        let seed = std::env::var("SEED").ok().and_then(|s| s.parse().ok());
        let mut doc = parse(&std::fs::read_to_string(&path).expect("the file")).unwrap_or_else(|e| panic!("{}", e));
        set_weights(&mut doc, &std::env::var("SCENARIO_SET").unwrap_or_default()).unwrap_or_else(|e| panic!("{}", e));
        let (out, failed) = run_seeded(&doc, seed).unwrap_or_else(|e| panic!("{}", e));
        for line in &out.trace {
            eprintln!("  {}", line);
        }
        for (at, seat, why) in out.refused.iter().take(10) {
            eprintln!("  {:>5.1} s  seat {} refused: {}", at, seat, why);
        }
        eprintln!("  hits {:?}; pillbox damage {:?}; start {:?}; end {:?}; landed {:?}", out.hits, out.pillbox_damage, out.start, out.end, out.landed);
        assert!(failed.is_empty(), "{}: {}", path, failed.join("; "));
        eprintln!("scenario {}: holds", path);
    }
}
