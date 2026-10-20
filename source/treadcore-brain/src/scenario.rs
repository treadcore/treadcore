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
}

/// Run a scenario: the outcome, and every expectation that failed (none
/// is a pass). A file that cannot be read or set up is an error.
pub fn run(doc: &Doc) -> Result<(Outcome, Vec<String>), String> {
    let num = |t: &BTreeMap<String, Value>, k: &str, d: f64| t.get(k).and_then(Value::num).unwrap_or(d);
    let players = num(&doc.top, "players", 2.0) as usize;
    let seconds = num(&doc.top, "seconds", 60.0);
    let sc = Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(players) };
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
            "depot" => depots.get(k).map(|d| d.1).ok_or(format!("no {}", anchor))?,
            "pillbox" => pillboxes.get(k).and_then(|id| w.pillboxes.iter().find(|p| p.id == *id)).map(|p| p.pos).ok_or(format!("no {}", anchor))?,
            _ => return Err(format!("not an anchor: {}", anchor)),
        };
        Ok(base + offset(off)?)
    };
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
        let seed = num(b, "seed", 5.0) as u64;
        let brain: Box<dyn Brain> = match b.get("kind").and_then(Value::str).unwrap_or("commander") {
            "rulebot" => Box::new(RuleBot::new(seed, "scenario")),
            "commander" => {
                let mut c = Commander::new(seed, "scenario");
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
    let events = list("event");
    for _ in 0..(seconds * TICK_HZ as f64) as u64 {
        let now = w.tick as f64 / TICK_HZ as f64;
        for e in events.iter().filter(|e| (num(e, "at", -1.0) * TICK_HZ as f64) as Tick == w.tick) {
            let seat = num(e, "move", -1.0);
            if seat >= 0.0 {
                let to = place(&w, &depots, &pillboxes, e.get("to").and_then(Value::str).unwrap_or("seat:0"), e.get("offset"))?;
                let t = w.tanks.iter_mut().find(|t| t.owner == seat as PlayerId).ok_or(format!("no seat {}", seat))?;
                t.pos = to;
                t.goal = None;
                t.route.clear();
            }
        }
        let mut orders = Vec::new();
        for (seat, b) in brains.iter_mut() {
            let obs = w.observe(*seat);
            if obs.me.as_ref().map(|t| t.alive).unwrap_or(false) {
                orders.extend(b.think(&obs).into_iter().map(|cmd| Order { player: *seat, cmd }));
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
                Event::PillboxFired { pillbox, damage, .. } => {
                    if let Some(k) = pillboxes.iter().position(|p| p == pillbox) {
                        out.pillbox_damage[k] += damage;
                    }
                }
                _ => {}
            }
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
        if w.tick % (5 * TICK_HZ as Tick) == 0 {
            out.trace.push(format!("{:>5.1} s  {}", now, brains.iter().map(|(s, b)| format!("seat {} {}", s, b.act().code())).collect::<Vec<_>>().join(", ")));
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

    /// One file, with its trace: `SCENARIO=path cargo test -p
    /// treadcore-brain --release -- --ignored scenario_one --nocapture`
    /// (`make scenario S=path`).
    #[test]
    #[ignore]
    fn scenario_one() {
        let path = std::env::var("SCENARIO").expect("SCENARIO=path");
        let (out, failed) = run_file(std::path::Path::new(&path)).unwrap_or_else(|e| panic!("{}", e));
        for line in &out.trace {
            eprintln!("  {}", line);
        }
        eprintln!("  hits {:?}; pillbox damage {:?}; start {:?}; end {:?}", out.hits, out.pillbox_damage, out.start, out.end);
        assert!(failed.is_empty(), "{}: {}", path, failed.join("; "));
        eprintln!("scenario {}: holds", path);
    }
}
