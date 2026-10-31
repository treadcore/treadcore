//! The ledger (roadmap step 12g, LAB-REPORT-BRAIN.md section 8). Every
//! census match writes one JSON line per seat: the genome (the weights,
//! the capabilities, the seed, the role) beside what the seat did, so a
//! high score can be played again (the sim is deterministic: the line
//! and the map are the match) and its weights bred from
//! (`scripts/tune.sh`).
//!
//! `Tally` counts as the match runs, from the world's events and, once a
//! second, from what each team holds; `records` writes the lines.
//! Ratings are Elo over the ledger's matches: a genome's team placed
//! above another's in a match counts as a win against it, so a lucky
//! score against weak opponents counts for less than a steady one
//! against strong ones.

use crate::weights::Settings;
use serde::{Deserialize, Serialize};
use treadcore_sim::*;

/// One seat's genome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Genome {
    /// A hash of the weights, the capabilities and the role: the key.
    pub id: String,
    pub weights: Vec<f32>,
    pub caps: u64,
    pub seed: u64,
    pub role: String,
    pub parent: Option<String>,
}

impl Genome {
    /// The genome of these Settings in this role, its id their hash.
    pub fn of(s: &Settings, seed: u64, role: &str, parent: Option<String>) -> Genome {
        // FNV-1a over the weights' bits, the capabilities and the role.
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut eat = |b: u8| {
            h ^= b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        };
        for w in &s.w.0 {
            w.to_bits().to_le_bytes().into_iter().for_each(&mut eat);
        }
        s.caps.0.to_le_bytes().into_iter().for_each(&mut eat);
        role.bytes().for_each(&mut eat);
        Genome { id: format!("{:016x}", h), weights: s.w.0.clone(), caps: s.caps.0, seed, role: role.to_string(), parent }
    }
}

/// What a seat did in a match.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Outcome {
    pub team: Option<TeamId>,
    pub won: bool,
    pub kills: u32,
    pub deaths: u32,
    pub revives: u32,
    /// Anything the seat's squad built.
    pub built: u32,
    pub road_tiles: u32,
    pub bridge_tiles: u32,
    /// The pillboxes and depots the seat's team held, summed each second.
    pub held_s: u32,
    /// The rewards' counts (LR 84). Direct shells of the seat's main gun
    /// that hit a hostile tank.
    #[serde(default)]
    pub direct_hits: u32,
    /// Damage the seat did to hostile pillboxes, in pillboxes' health
    /// (`PILLBOX_HEALTH` is 1).
    #[serde(default)]
    pub pillbox_damage: f32,
    /// Pillboxes the seat's squad took: neutral, and from a hostile.
    #[serde(default)]
    pub neutral_taken: u32,
    #[serde(default)]
    pub enemy_taken: u32,
    /// Hostile tanks the seat killed within `DEFEND_R` of a live
    /// pillbox of its team.
    #[serde(default)]
    pub defends: u32,
    /// The seat's team took every pillbox that began neutral itself.
    #[serde(default)]
    pub all_neutral: bool,
    /// Times the seat's tank went inside an enemy base, a visit once.
    #[serde(default)]
    pub base_entries: u32,
    /// Trips that laid a mine on the enemy's side of the line and came back over it alive.
    #[serde(default)]
    pub raids: u32,
    /// Hostile tanks the seat's mines caught.
    #[serde(default)]
    pub mine_hits: u32,
    /// Trips left from a base with shells near full, after a trip that fired.
    #[serde(default)]
    pub full_sorties: u32,
    /// Trips ended at a base with shells low.
    #[serde(default)]
    pub low_returns: u32,
    /// Arrivals at a stocked base other than the last one.
    #[serde(default)]
    pub rotations: u32,
    /// Castles the seat's team stood, summed each second.
    #[serde(default)]
    pub castle_s: u32,
}

/// A kill this near a live pillbox of the killer's team defends it
/// (LR 84): twice a pillbox's reach.
pub const DEFEND_R: f32 = 2.0 * PILLBOX_RANGE;

/// A ledger line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub seat: PlayerId,
    pub map: String,
    pub match_seed: u64,
    pub seconds: u32,
    pub genome: Genome,
    pub outcome: Outcome,
    pub score: f32,
}

/// The score's weights (LR 8): what a match is judged on.
#[derive(Clone, Copy, Debug)]
pub struct ScoreWeights {
    pub win: f32,
    pub kill: f32,
    pub death: f32,
    pub held: f32,
    pub build: f32,
    pub road: f32,
    /// The rewards (LR 84), each a count's worth, about a third of a
    /// match's score in all (Paul's choice): a direct hit on a hostile
    /// tank (three or so kill it, a kill is 1); a whole
    /// pillbox's health of damage to hostile ones; a neutral pillbox
    /// taken; a hostile one taken; a kill defending a pillbox; and every
    /// neutral pillbox held at once, once a match.
    pub direct_hit: f32,
    pub pillbox_damage: f32,
    pub neutral_taken: f32,
    /// Going in, laying and getting out (Paul, 30 October): a visit to an enemy base, a mine
    /// laid over the line and brought home from, a hostile tank caught by a mine.
    pub base_entry: f32,
    pub raid: f32,
    pub mine_hit: f32,
    /// A whole trip (Paul, 30 October: "leaving base with full ammo, and returning to base with
    /// low ammo ... rotating between bases"), and castles held, as `held` is, by the match's
    /// length.
    pub full_sortie: f32,
    pub low_return: f32,
    pub rotation: f32,
    pub castle: f32,
    pub enemy_taken: f32,
    pub defend: f32,
    pub all_neutral: f32,
}

impl Default for ScoreWeights {
    fn default() -> ScoreWeights {
        ScoreWeights {
            win: 10.0,
            kill: 1.0,
            death: 1.0,
            held: 2.0,
            build: 0.1,
            road: 0.05,
            direct_hit: 1.5,
            pillbox_damage: 1.5,
            neutral_taken: 3.0,
            base_entry: 0.5,
            raid: 1.5,
            mine_hit: 1.0,
            full_sortie: 0.5,
            low_return: 0.5,
            rotation: 0.75,
            castle: 3.0,
            enemy_taken: 6.0,
            defend: 1.5,
            all_neutral: 15.0,
        }
    }
}

impl ScoreWeights {
    /// The score between elimination and holding ground (`long_game`,
    /// roadmap 12h): 0 weighs the win and kills half as much again and
    /// the holdings not at all; 1 the win and kills half as much and
    /// the holdings twice; 0.5 is the default above.
    pub fn from_long_game(lg: f32) -> ScoreWeights {
        let lg = lg.clamp(0.0, 1.0);
        let short = 1.5 - lg;
        ScoreWeights { win: 10.0 * short, kill: short, death: short, held: 4.0 * lg, ..ScoreWeights::default() }
    }

    /// The score before the rewards (LR 8 to 83): `CENSUS_REWARDS=0`.
    pub fn without_rewards(self) -> ScoreWeights {
        ScoreWeights { full_sortie: 0.0, low_return: 0.0, rotation: 0.0, castle: 0.0, base_entry: 0.0, raid: 0.0, mine_hit: 0.0, direct_hit: 0.0, pillbox_damage: 0.0, neutral_taken: 0.0, enemy_taken: 0.0, defend: 0.0, all_neutral: 0.0, ..self }
    }
}

/// The rewards' part of a seat's score (LR 84).
pub fn rewards(o: &Outcome, k: &ScoreWeights) -> f32 {
    k.full_sortie * o.full_sorties as f32 + k.low_return * o.low_returns as f32 + k.rotation * o.rotations as f32 + k.base_entry * o.base_entries as f32 + k.raid * o.raids as f32 + k.mine_hit * o.mine_hits as f32 + k.direct_hit * o.direct_hits as f32 + k.pillbox_damage * o.pillbox_damage + k.neutral_taken * o.neutral_taken as f32 + k.enemy_taken * o.enemy_taken as f32 + k.defend * o.defends as f32 + if o.all_neutral { k.all_neutral } else { 0.0 }
}

/// A seat's match score: the win, kills less deaths, ground held over
/// the match's length, works built and road laid, and the rewards, each
/// at its weight.
pub fn score(o: &Outcome, seconds: u32, k: &ScoreWeights) -> f32 {
    let t = seconds.max(1) as f32;
    (if o.won { k.win } else { 0.0 }) + k.kill * o.kills as f32 - k.death * o.deaths as f32 + k.held * o.held_s as f32 / t + k.build * o.built as f32 + k.road * (o.road_tiles + o.bridge_tiles) as f32 + k.castle * o.castle_s as f32 / t + rewards(o, k)
}

/// The counts, as the match runs.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub seats: Vec<Outcome>,
    /// The pillboxes neutral at the start, and how many of them each
    /// team has taken (LR 84; a pillbox set down after a lift is another
    /// id, so they are counted as taken, not by id).
    neutral: usize,
    neutral_by_team: std::collections::HashMap<TeamId, usize>,
    /// Shells in flight and whose (LR 84).
    shells: std::collections::HashMap<EntityId, PlayerId>,
    /// By seat: inside an enemy base at the last look, and mines laid over the line on this trip.
    inside: Vec<bool>,
    laid_out: Vec<u32>,
    /// By seat: the depot it stands at, the last it stood at, and shells fired since it left.
    at_base: Vec<Option<EntityId>>,
    last_base: Vec<Option<EntityId>>,
    fired: Vec<u32>,
}

/// A share of full at or over this leaves full; at or under `LOW_SHARE` comes back low.
const FULL_SHARE: f32 = 0.9;
const LOW_SHARE: f32 = 0.25;
/// A base with fewer shells than this is not a reason to have driven to it.
const STOCKED: u8 = 5;

/// Groups of a team's guns, as the sim's crowding has them: how many castles it stands, a castle
/// being a gun among walls, however many guns; and what it holds, a group counting at most
/// `TURRETS_FREE`, so a ring of eight earns no more than a pair.
fn castles_and_guns(w: &World, ours: impl Fn(Option<PlayerId>) -> bool) -> (u32, u32) {
    let mut groups: Vec<V2> = Vec::new();
    let mut counts: Vec<usize> = Vec::new();
    for p in w.pillboxes.iter().filter(|p| p.alive() && ours(p.owner)) {
        match groups.iter().position(|g| g.dist(p.pos) <= TURRET_CROWD_R) {
            Some(i) => counts[i] += 1,
            None => {
                groups.push(p.pos);
                counts.push(1);
            }
        }
    }
    let castles = groups.iter().filter(|g| w.in_castle(**g)).count() as u32;
    (castles, counts.iter().map(|c| (*c).min(TURRETS_FREE) as u32).sum())
}

/// How far `p` stands on `seat`'s side of the battle line, as `front::side`, from the world.
fn world_side(w: &World, seat: PlayerId, p: V2) -> Option<f32> {
    let ground = || w.depots.iter().filter_map(|d| d.owner.map(|o| (o, d.pos))).chain(w.pillboxes.iter().filter(|b| b.alive()).filter_map(|b| b.owner.map(|o| (o, b.pos))));
    let near = |ours: bool| ground().filter(|(o, _)| w.allied(*o, seat) == ours).map(|(_, g)| g.dist(p)).min_by(f32::total_cmp);
    Some(near(false)? - near(true)?)
}

impl Tally {
    /// An empty count for each seat of the world, with its team.
    pub fn new(w: &World) -> Tally {
        Tally {
            seats: w.players.iter().map(|p| Outcome { team: w.team_of(p.id), ..Outcome::default() }).collect(),
            neutral: w.pillboxes.iter().filter(|p| p.owner.is_none()).count(),
            neutral_by_team: Default::default(),
            shells: Default::default(),
            inside: vec![false; w.players.len()],
            laid_out: vec![0; w.players.len()],
            at_base: vec![None; w.players.len()],
            last_base: vec![None; w.players.len()],
            fired: vec![1; w.players.len()],
        }
    }

    /// After each `step`: the tick's events; once a second, the holdings.
    pub fn hear(&mut self, w: &World) {
        let n = self.seats.len();
        let seat = |p: PlayerId| -> Option<usize> { ((p as usize) < n).then_some(p as usize) };
        // Two players hostile: both on teams, not the same one.
        let hostile = |a: PlayerId, b: PlayerId| matches!((w.team_of(a), w.team_of(b)), (Some(x), Some(y)) if x != y);
        for e in &w.events {
            match e {
                Event::MineLaid { owner, at, .. } => {
                    if let Some(i) = seat(*owner).filter(|_| world_side(w, *owner, *at).is_some_and(|s| s < 0.0)) {
                        self.laid_out[i] += 1;
                    }
                }
                Event::MineExploded { owner, hits, .. } => {
                    if let Some(i) = seat(*owner) {
                        let n = hits.iter().filter(|h| w.tank(**h).is_some_and(|t| hostile(t.owner, *owner))).count();
                        self.seats[i].mine_hits += n as u32;
                    }
                }
                Event::TankDestroyed { tank, owner, by } => {
                    if let Some(i) = seat(*owner) {
                        self.seats[i].deaths += 1;
                        self.laid_out[i] = 0;
                        self.inside[i] = false;
                    }
                    if let Some(i) = by.and_then(seat) {
                        if Some(i as PlayerId) != Some(*owner) {
                            self.seats[i].kills += 1;
                            let killer = i as PlayerId;
                            let at = w.tank(*tank).map(|t| t.pos);
                            let defended = at.is_some_and(|at| w.pillboxes.iter().any(|p| p.alive() && p.owner.is_some_and(|o| !hostile(o, killer) && w.team_of(o).is_some()) && p.pos.dist(at) <= DEFEND_R));
                            if defended && hostile(*owner, killer) {
                                self.seats[i].defends += 1;
                            }
                        }
                    }
                }
                Event::ShellLaunched { shell, owner, .. } => {
                    self.shells.insert(*shell, *owner);
                    if let Some(i) = seat(*owner) {
                        self.fired[i] += 1;
                    }
                }
                Event::ShellImpact { shell, kind, hits, .. } => {
                    if let Some(owner) = self.shells.remove(shell) {
                        if *kind == ShellKind::Direct {
                            if let Some(i) = seat(owner) {
                                let n = hits.iter().filter(|h| w.tank(**h).is_some_and(|t| hostile(t.owner, owner))).count();
                                self.seats[i].direct_hits += n as u32;
                            }
                        }
                    }
                }
                Event::PillboxDamaged { pillbox, by: Some(by), damage, .. } => {
                    let theirs = w.pillbox(*pillbox).and_then(|p| p.owner).is_some_and(|o| hostile(o, *by));
                    if let (true, Some(i)) = (theirs, seat(*by)) {
                        self.seats[i].pillbox_damage += damage / PILLBOX_HEALTH;
                    }
                }
                Event::PillboxCaptured { by, from, .. } => {
                    if let Some(i) = seat(*by) {
                        match from {
                            None => {
                                self.seats[i].neutral_taken += 1;
                                if let Some(t) = w.team_of(*by) {
                                    let n = self.neutral_by_team.entry(t).or_insert(0);
                                    *n += 1;
                                    if *n >= self.neutral && self.neutral > 0 {
                                        self.seats.iter_mut().filter(|o| o.team == Some(t)).for_each(|o| o.all_neutral = true);
                                    }
                                }
                            }
                            Some(f) if hostile(*f, *by) => self.seats[i].enemy_taken += 1,
                            _ => {}
                        }
                    }
                }
                Event::Revived { by, .. } => {
                    if let Some(i) = seat(*by) {
                        self.seats[i].revives += 1;
                    }
                }
                Event::Built { owner, what, at } => {
                    if let Some(i) = seat(*owner) {
                        self.seats[i].built += 1;
                        if *what == BuildKind::Road {
                            if w.map.at(*at) == Terrain::Bridge {
                                self.seats[i].bridge_tiles += 1;
                            } else {
                                self.seats[i].road_tiles += 1;
                            }
                        }
                    }
                }
                Event::PillboxBuilt { by, .. } | Event::StructureBuilt { by, .. } => {
                    if let Some(i) = seat(*by) {
                        self.seats[i].built += 1;
                    }
                }
                _ => {}
            }
        }
        if w.tick % TICK_HZ as Tick == 0 {
            for o in self.seats.iter_mut() {
                let Some(t) = o.team else { continue };
                let ours = |x: Option<PlayerId>| x.and_then(|p| w.team_of(p)) == Some(t);
                let (castles, guns) = castles_and_guns(w, ours);
                o.castle_s += castles;
                o.held_s += guns + w.depots.iter().filter(|d| ours(d.owner)).count() as u32;
            }
            // The line, once a second: going in counts once a visit, and a raid when its tank
            // is back on its own side alive.
            for pl in &w.players {
                let i = pl.id as usize;
                if i >= self.seats.len() {
                    continue;
                }
                let Some(t) = w.tanks.iter().find(|t| t.owner == pl.id && t.alive) else { continue };
                // A trip: left full after firing, back low, and to another base with something in it.
                let share = t.shells as f32 / TANK_SHELLS as f32;
                let here = w.depots.iter().find(|d| d.owner.is_some_and(|o| w.allied(o, pl.id)) && d.pos.dist(t.pos) <= DEPOT_RADIUS + 1.0);
                match (self.at_base[i], here) {
                    (None, Some(d)) => {
                        if share <= LOW_SHARE {
                            self.seats[i].low_returns += 1;
                        }
                        if self.last_base[i].is_some_and(|l| l != d.id) && d.stock.kinds[STOCK_SHELL] >= STOCKED {
                            self.seats[i].rotations += 1;
                        }
                        self.at_base[i] = Some(d.id);
                        self.last_base[i] = Some(d.id);
                    }
                    (Some(_), None) => {
                        if share >= FULL_SHARE && self.fired[i] > 0 {
                            self.seats[i].full_sorties += 1;
                            self.fired[i] = 0;
                        }
                        self.at_base[i] = None;
                    }
                    _ => {}
                }
                let inside = w.depots.iter().any(|d| d.owner.is_some_and(|o| !w.allied(o, pl.id)) && d.pos.dist(t.pos) <= crate::front::ENEMY_BASE_R);
                if inside && !self.inside[i] {
                    self.seats[i].base_entries += 1;
                }
                self.inside[i] = inside;
                if self.laid_out[i] > 0 && world_side(w, pl.id, t.pos).is_some_and(|s| s > 0.0) {
                    self.seats[i].raids += 1;
                    self.laid_out[i] = 0;
                }
            }
        }
    }

    /// The match is over (or out of time): a line per seat with a genome.
    pub fn records(&mut self, w: &World, map: &str, genomes: &[(PlayerId, Genome)], k: &ScoreWeights) -> Vec<Record> {
        let seconds = (w.tick / TICK_HZ as Tick) as u32;
        for o in self.seats.iter_mut() {
            o.won = w.winning_team.is_some() && o.team == w.winning_team;
        }
        genomes
            .iter()
            .filter_map(|(p, g)| {
                let o = self.seats.get(*p as usize)?.clone();
                let score = score(&o, seconds, k);
                Some(Record { seat: *p, map: map.to_string(), match_seed: w.scenario.seed, seconds, genome: g.clone(), outcome: o, score })
            })
            .collect()
    }
}

/// Append records to `dir/ledger.jsonl`.
pub fn write(dir: &std::path::Path, records: &[Record]) -> std::io::Result<()> {
    use std::io::Write;
    std::fs::create_dir_all(dir)?;
    let mut f = std::fs::OpenOptions::new().create(true).append(true).open(dir.join("ledger.jsonl"))?;
    for r in records {
        writeln!(f, "{}", serde_json::to_string(r).map_err(std::io::Error::other)?)?;
    }
    Ok(())
}

/// Read a ledger; bad lines are skipped.
pub fn read(path: &std::path::Path) -> Vec<Record> {
    std::fs::read_to_string(path).map(|t| t.lines().filter_map(|l| serde_json::from_str(l).ok()).collect()).unwrap_or_default()
}

/// The mean of a sample and its standard error (the sample's standard
/// deviation over the root of its size); the error is 0 for fewer than
/// two. The census's differences a seed are the sample (LR 24).
pub fn mean_se(x: &[f32]) -> (f32, f32) {
    let n = x.len() as f32;
    if x.is_empty() {
        return (0.0, 0.0);
    }
    let mean = x.iter().sum::<f32>() / n;
    if x.len() < 2 {
        return (mean, 0.0);
    }
    let var = x.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / (n - 1.0);
    (mean, (var / n).sqrt())
}

/// Elo ratings by genome over the ledger's matches: within a match, each
/// genome's best seat score against every other genome's, a win for the
/// higher. Sorted best first.
pub fn ratings(records: &[Record]) -> Vec<(String, f32, u32)> {
    use std::collections::BTreeMap;
    const K: f32 = 16.0;
    let mut r: BTreeMap<String, (f32, u32)> = BTreeMap::new();
    let mut matches: BTreeMap<(u64, String), Vec<&Record>> = BTreeMap::new();
    for x in records {
        matches.entry((x.match_seed, x.map.clone())).or_default().push(x);
    }
    for (_, rs) in matches {
        let mut best: BTreeMap<String, f32> = BTreeMap::new();
        for x in rs {
            let e = best.entry(x.genome.id.clone()).or_insert(f32::MIN);
            *e = e.max(x.score);
        }
        let ids: Vec<(String, f32)> = best.into_iter().collect();
        for i in 0..ids.len() {
            for j in i + 1..ids.len() {
                let (a, b) = (&ids[i], &ids[j]);
                let ra = r.get(&a.0).map(|v| v.0).unwrap_or(1500.0);
                let rb = r.get(&b.0).map(|v| v.0).unwrap_or(1500.0);
                let ea = 1.0 / (1.0 + 10f32.powf((rb - ra) / 400.0));
                let sa = if a.1 > b.1 {
                    1.0
                } else if a.1 < b.1 {
                    0.0
                } else {
                    0.5
                };
                let ea_ = r.entry(a.0.clone()).or_insert((1500.0, 0));
                ea_.0 += K * (sa - ea);
                ea_.1 += 1;
                let eb_ = r.entry(b.0.clone()).or_insert((1500.0, 0));
                eb_.0 += K * ((1.0 - sa) - (1.0 - ea));
                eb_.1 += 1;
            }
        }
    }
    let mut out: Vec<(String, f32, u32)> = r.into_iter().map(|(k, (v, n))| (k, v, n)).collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The hall of fame: the ledger's genomes by Elo, best first, with
    /// the weights each changed (`LEDGER`, default
    /// reports/ledger/ledger.jsonl). `cargo test -p treadcore-brain
    /// --release -- --ignored ledger_ratings --nocapture`.
    #[test]
    #[ignore]
    fn ledger_ratings() {
        let path = std::env::var("LEDGER").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reports/ledger/ledger.jsonl"));
        let rs = read(&path);
        eprintln!("{} lines in {}", rs.len(), path.display());
        for (id, elo, games) in ratings(&rs).iter().take(16) {
            let Some(r) = rs.iter().find(|r| &r.genome.id == id) else { continue };
            let changed: Vec<String> = crate::weights::W::ALL.iter().filter(|w| r.genome.weights.get(**w as usize).map(|v| *v != w.default_value()).unwrap_or(false)).map(|w| format!("{}={}", w.name(), r.genome.weights[*w as usize])).collect();
            eprintln!("{:>7.1}  {:>4} games  {}  caps [{}]  {}", elo, games, id, crate::weights::Caps(r.genome.caps).names(), changed.join(" "));
        }
    }

    #[test]
    fn the_standard_error_of_a_sample() {
        assert_eq!(mean_se(&[]), (0.0, 0.0));
        assert_eq!(mean_se(&[2.0]), (2.0, 0.0), "one seed has no error");
        assert_eq!(mean_se(&[0.0; 12]), (0.0, 0.0), "a null run reads exactly even");
        // 1 and 3: mean 2, sample deviation sqrt(2), error sqrt(2)/sqrt(2) = 1.
        let (m, se) = mean_se(&[1.0, 3.0]);
        assert!((m - 2.0).abs() < 1e-6 && (se - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_ledger_line_round_trips_and_rates() {
        let s = Settings::default();
        let g = Genome::of(&s, 7, "leader", None);
        let mut s2 = Settings::default();
        s2.w[crate::weights::W::Wounded] = 2.0;
        let g2 = Genome::of(&s2, 7, "leader", Some(g.id.clone()));
        assert_ne!(g.id, g2.id, "a weight changed is another genome");
        let rec = |ge: &Genome, sc: f32, seed: u64| Record { seat: 0, map: "naval".into(), match_seed: seed, seconds: 600, genome: ge.clone(), outcome: Outcome::default(), score: sc };
        let rs = vec![rec(&g, 1.0, 1), rec(&g2, 3.0, 1), rec(&g, 0.5, 2), rec(&g2, 2.0, 2)];
        let dir = std::env::temp_dir().join(format!("treadcore-ledger-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        write(&dir, &rs).unwrap();
        let back = read(&dir.join("ledger.jsonl"));
        assert_eq!(back, rs);
        let r = ratings(&back);
        assert_eq!(r[0].0, g2.id, "the genome that won both matches rates first");
        assert!(r[0].1 > 1500.0 && r[1].1 < 1500.0);
        let _ = std::fs::remove_dir_all(&dir);
        let o = Outcome { won: true, kills: 3, deaths: 1, held_s: 600, ..Outcome::default() };
        assert!((score(&o, 600, &ScoreWeights::default()) - (10.0 + 3.0 - 1.0 + 2.0)).abs() < 1e-4);
        let half = ScoreWeights::from_long_game(0.5);
        let d = ScoreWeights::default();
        assert_eq!((half.win, half.kill, half.held), (d.win, d.kill, d.held), "long_game 0.5 is the default score");
        assert!(score(&o, 600, &ScoreWeights::from_long_game(1.0)) < score(&o, 600, &ScoreWeights::from_long_game(0.0)), "a short win counts for less in a long game");
    }

    /// Each reward counts what it says, for the seat that did it, and
    /// only against a hostile (LR 84); `without_rewards` is the score
    /// before them.
    #[test]
    fn the_rewards_count_their_events() {
        let mut w = World::new(Scenario::bots(4).with_teams(2));
        assert_eq!((w.team_of(0), w.team_of(2)), (w.team_of(2), w.team_of(0)), "0 and 2 a team");
        assert_ne!(w.team_of(0), w.team_of(1));
        let tank = |w: &World, p: PlayerId| w.tanks.iter().find(|t| t.owner == p).unwrap().id;
        // Two pillboxes only: one neutral, one seat 1's.
        let mk = |owner: Option<PlayerId>, at: V2, id: EntityId| Pillbox { id, pos: at, owner, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS };
        w.pillboxes = vec![mk(None, V2::new(10.5, 10.5), 90_001), mk(Some(1), V2::new(40.5, 40.5), 90_002)];
        let mut t = Tally::new(&w);
        let (t1, t2) = (tank(&w, 1), tank(&w, 2));
        w.events = vec![
            // Seat 0's direct shell hits seat 1 (hostile) and seat 2 (a teammate).
            Event::ShellLaunched { shell: 7, owner: 0, kind: ShellKind::Direct, spotted: false, eta_ticks: 1 },
            Event::ShellImpact { shell: 7, at: V2::new(0.0, 0.0), kind: ShellKind::Direct, hits: vec![t1, t2] },
            // An indirect shell's hit is not a direct hit.
            Event::ShellLaunched { shell: 8, owner: 0, kind: ShellKind::Indirect, spotted: false, eta_ticks: 1 },
            Event::ShellImpact { shell: 8, at: V2::new(0.0, 0.0), kind: ShellKind::Indirect, hits: vec![t1] },
            // Half a pillbox of damage to seat 1's, and some to the neutral one.
            Event::PillboxDamaged { pillbox: 90_002, by: Some(0), damage: PILLBOX_HEALTH / 2.0, health_left: PILLBOX_HEALTH / 2.0 },
            Event::PillboxDamaged { pillbox: 90_001, by: Some(0), damage: 10.0, health_left: 50.0 },
            // Seat 2 takes the neutral one: every neutral pillbox, for team 0.
            Event::PillboxCaptured { pillbox: 90_001, by: 2, from: None },
            Event::PillboxCaptured { pillbox: 90_002, by: 0, from: Some(1) },
        ];
        // Seat 1's tank by seat 2's pillbox, killed by seat 0: a defence.
        w.tanks.iter_mut().find(|x| x.owner == 1).unwrap().pos = V2::new(12.5, 10.5);
        w.pillboxes[0].owner = Some(2);
        w.events.push(Event::TankDestroyed { tank: t1, owner: 1, by: Some(0) });
        t.hear(&w);
        let (a, b, c) = (&t.seats[0], &t.seats[1], &t.seats[2]);
        assert_eq!(a.direct_hits, 1, "one hostile tank hit, direct");
        assert!((a.pillbox_damage - 0.5).abs() < 1e-6, "half a hostile pillbox: {}", a.pillbox_damage);
        assert_eq!((a.enemy_taken, a.neutral_taken, c.neutral_taken), (1, 0, 1));
        assert_eq!((a.kills, a.defends), (1, 1));
        assert!(a.all_neutral && c.all_neutral && !b.all_neutral, "team 0 took every neutral pillbox");
        let k = ScoreWeights::default();
        let r = rewards(a, &k);
        assert!((r - (1.5 + 0.75 + 6.0 + 1.5 + 15.0)).abs() < 1e-5, "{}", r);
        assert!((score(a, 600, &k) - score(a, 600, &k.without_rewards()) - r).abs() < 1e-5);
        assert_eq!(rewards(a, &k.without_rewards()), 0.0);
    }
}
