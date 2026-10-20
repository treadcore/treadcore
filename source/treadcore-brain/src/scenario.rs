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
//! seat's tank starts), `"depot:0"` and `"pillbox:0"` (the first placed).
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
    fn num(&self) -> Option<f64> {
        if let Value::Num(n) = self { Some(*n) } else { None }
    }
    /// A string, or None.
    fn str(&self) -> Option<&str> {
        if let Value::Str(s) = self { Some(s) } else { None }
    }
    /// An inline table, or None.
    fn table(&self) -> Option<&BTreeMap<String, Value>> {
        if let Value::Table(t) = self { Some(t) } else { None }
    }
}

/// A file read: its top-level keys, `[expect]`, and each `[[name]]` list.
#[derive(Clone, Debug, Default)]
pub struct Doc {
    pub top: BTreeMap<String, Value>,
    pub expect: BTreeMap<String, Value>,
    pub lists: BTreeMap<String, Vec<BTreeMap<String, Value>>>,
}

/// Read the TOML subset; an error names its line.
pub fn parse(text: &str) -> Result<Doc, String> {
    let mut doc = Doc::default();
    // Where a key goes: the top, [expect], or the last [[name]].
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
            if name.trim() != "expect" {
                return Err(at(format!("no section [{}]", name.trim())));
            }
            into = Some(("expect".into(), false));
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

/// What a run saw, for the checks and the trace.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Each seat's tank: where it started and ended, its shells then
    /// and now, and the hits it took (by whom).
    pub start: BTreeMap<PlayerId, (V2, u16)>,
    pub end: BTreeMap<PlayerId, (V2, u16)>,
    pub hits: Vec<(Option<PlayerId>, PlayerId)>,
    /// Each placed pillbox: the damage it did, the tick each seat first
    /// owned it, and whether it stands at the end.
    pub pillbox_damage: Vec<f32>,
    pub pillbox_owned: Vec<BTreeMap<PlayerId, Tick>>,
    pub pillbox_alive: Vec<bool>,
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
/// is the file's own world, as `run`.
pub fn run_seeded(doc: &Doc, seed: Option<u64>) -> Result<(Outcome, Vec<String>), String> {
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
        let (kind, k) = anchor.split_once(':').ok_or(format!("not an anchor: {}", anchor))?;
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
    // anchor's tile, both ends in, set to water or grass.
    for t in list("terrain") {
        let near = place(&w, &depots, &pillboxes, t.get("near").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let kind = match t.get("kind").and_then(Value::str).unwrap_or("water") {
            "water" => Terrain::Water,
            "grass" => Terrain::Grass,
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
    // `[[tank]]`: a seat's tank put at an anchor and offset at the
    // start, afloat with `boat = true`.
    for t in list("tank") {
        let seat = num(t, "seat", 0.0) as PlayerId;
        let to = place(&w, &depots, &pillboxes, t.get("at").and_then(Value::str).unwrap_or("seat:0"), t.get("offset"))?;
        let tank = w.tanks.iter_mut().find(|x| x.owner == seat).ok_or(format!("no seat {}", seat))?;
        tank.pos = Map::tile_centre(to);
        tank.boat = t.get("boat") == Some(&Value::Bool(true));
        tank.goal = None;
        tank.route.clear();
    }
    for d in list("depot") {
        let near = place(&w, &depots, &pillboxes, d.get("near").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let dep = w.depots.iter().filter(|x| !depots.iter().any(|(id, _)| *id == x.id)).min_by(|a, b| a.pos.dist(near).total_cmp(&b.pos.dist(near))).ok_or("no depot left")?;
        let (id, pos) = (dep.id, dep.pos);
        let owner = d.get("owner").and_then(Value::num).map(|o| o as PlayerId);
        w.depots.iter_mut().find(|x| x.id == id).expect("the depot").owner = owner;
        depots.push((id, pos));
    }
    for (k, p) in list("pillbox").iter().enumerate() {
        // As the hand-written tests place one: `away` tiles from the
        // anchor toward the map's centre, on open ground it can reach.
        let home = place(&w, &depots, &pillboxes, p.get("near").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let toward = (w.map.center() - home).norm();
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
    for s in list("shell") {
        let at = place(&w, &depots, &pillboxes, s.get("at").and_then(Value::str).unwrap_or("seat:0"), None)?;
        let from = at + offset(s.get("from"))?;
        let id = w.next_id;
        w.next_id += 1;
        let tick = w.tick;
        let lands = (num(s, "lands_in", 3.0) * TICK_HZ as f64) as Tick;
        w.shells.push(Shell { id, owner: num(s, "owner", 1.0) as PlayerId, kind: ShellKind::Indirect, from, aim: at, impact: at, launch_tick: tick, impact_tick: tick + lands, spotted: true });
    }
    let mut brains: Vec<(PlayerId, Box<dyn Brain>)> = Vec::new();
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
                        let w = crate::weights::W::by_name(k).ok_or(format!("no weight {}", k))?;
                        s.w[w] = v.num().ok_or(format!("weight {}: a number", k))? as f32;
                    }
                }
                c.set_settings(s);
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
            k => return Err(format!("no brain kind {}", k)),
        };
        brains.push((seat, brain));
    }
    let mut out = Outcome { pillbox_damage: vec![0.0; pillboxes.len()], pillbox_owned: vec![BTreeMap::new(); pillboxes.len()], ..Default::default() };
    for t in &w.tanks {
        out.start.insert(t.owner, (t.pos, t.shells));
    }
    for key in ["ends_near", "lands_near"] {
        if let Some(Value::Table(t)) = doc.expect.get(key) {
            let at = place(&w, &depots, &pillboxes, t.get("at").and_then(Value::str).unwrap_or("seat:0"), t.get("offset"))?;
            out.points.insert(key.into(), at);
        }
    }
    let mut afloat: BTreeMap<PlayerId, bool> = w.tanks.iter().map(|t| (t.owner, t.boat)).collect();
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
                let to = place(&w, &depots, &pillboxes, e.get("to").and_then(Value::str).unwrap_or("seat:0"), e.get("offset"))?;
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
                Event::TankDamaged { tank, by, .. } => {
                    if let Some(t) = w.tanks.iter().find(|t| t.id == *tank) {
                        out.hits.push((*by, t.owner));
                    }
                }
                Event::Rejected { player, why, .. } if brains.iter().any(|(s, _)| s == player) => {
                    out.refused.push((w.tick as f64 / TICK_HZ as f64, *player, why.to_string()));
                }
                Event::PillboxFired { pillbox, damage, .. } => {
                    if let Some(k) = pillboxes.iter().position(|p| p == pillbox) {
                        out.pillbox_damage[k] += damage;
                    }
                }
                _ => {}
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
        if w.over {
            out.over = Some(w.tick as f64 / TICK_HZ as f64);
            out.trace.push(format!("{:>5.1} s  the match is over", w.tick as f64 / TICK_HZ as f64));
            break;
        }
        if w.tick % every == 0 {
            let at = |s: PlayerId| w.tanks.iter().find(|t| t.owner == s).map(|t| format!("({:.1},{:.1}){}", t.pos.x, t.pos.y, if t.boat { " afloat" } else { "" })).unwrap_or_default();
            out.trace.push(format!("{:>5.1} s  {}", now, brains.iter().map(|(s, b)| format!("seat {} at {} {} ({})", s, at(*s), b.act().code(), b.status())).collect::<Vec<_>>().join(", ")));
        }
    }
    for t in &w.tanks {
        out.end.insert(t.owner, (t.pos, t.shells));
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
            "pillbox_damage" => {
                let k = field(t, "pillbox").unwrap_or(0.0) as usize;
                let d = out.pillbox_damage.get(k).copied().unwrap_or(0.0) as f64;
                if let Some(m) = field(t, "max").filter(|m| d > *m) {
                    failed.push(format!("pillbox {} did {} damage, at most {} expected", k, d, m));
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
/// the file (added to its `weights`, or a new one), so two settings run
/// on the same seeds.
pub fn set_weights(doc: &mut Doc, set: &str) -> Result<(), String> {
    for kv in set.split_whitespace() {
        let (k, v) = kv.split_once('=').ok_or(format!("not name=value: {}", kv))?;
        let v: f64 = v.parse().map_err(|_| format!("not a number: {}", v))?;
        for b in doc.lists.get_mut("brain").into_iter().flatten() {
            let w = b.entry("weights".into()).or_insert_with(|| Value::Table(BTreeMap::new()));
            if let Value::Table(t) = w {
                t.insert(k.to_string(), Value::Num(v));
            }
        }
    }
    Ok(())
}

/// Read and run a file: its failures, or the error that stopped it.
pub fn run_file(path: &std::path::Path) -> Result<(Outcome, Vec<String>), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    run(&parse(&text).map_err(|e| format!("{}: {}", path.display(), e))?).map_err(|e| format!("{}: {}", path.display(), e))
}

#[cfg(test)]
mod tests {
    use super::*;

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
