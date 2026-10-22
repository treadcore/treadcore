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
}

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
}

impl Default for ScoreWeights {
    fn default() -> ScoreWeights {
        ScoreWeights { win: 10.0, kill: 1.0, death: 1.0, held: 2.0, build: 0.1, road: 0.05 }
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
}

/// A seat's match score: the win, kills less deaths, ground held over
/// the match's length, works built and road laid, each at its weight.
pub fn score(o: &Outcome, seconds: u32, k: &ScoreWeights) -> f32 {
    let t = seconds.max(1) as f32;
    (if o.won { k.win } else { 0.0 }) + k.kill * o.kills as f32 - k.death * o.deaths as f32 + k.held * o.held_s as f32 / t + k.build * o.built as f32 + k.road * (o.road_tiles + o.bridge_tiles) as f32
}

/// The counts, as the match runs.
#[derive(Clone, Debug, Default)]
pub struct Tally {
    pub seats: Vec<Outcome>,
}

impl Tally {
    /// An empty count for each seat of the world, with its team.
    pub fn new(w: &World) -> Tally {
        Tally { seats: w.players.iter().map(|p| Outcome { team: w.team_of(p.id), ..Outcome::default() }).collect() }
    }

    /// After each `step`: the tick's events; once a second, the holdings.
    pub fn hear(&mut self, w: &World) {
        let n = self.seats.len();
        let seat = |p: PlayerId| -> Option<usize> { ((p as usize) < n).then_some(p as usize) };
        for e in &w.events {
            match e {
                Event::TankDestroyed { owner, by, .. } => {
                    if let Some(i) = seat(*owner) {
                        self.seats[i].deaths += 1;
                    }
                    if let Some(i) = by.and_then(seat) {
                        if Some(i as PlayerId) != Some(*owner) {
                            self.seats[i].kills += 1;
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
                o.held_s += (w.pillboxes.iter().filter(|p| p.alive() && ours(p.owner)).count() + w.depots.iter().filter(|d| ours(d.owner)).count()) as u32;
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
}
