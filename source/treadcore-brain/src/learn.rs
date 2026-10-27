//! Training the learned tank (LR 95, 97): the phases' training grounds
//! and evolution strategies over a `Net`'s weights, in plain Rust on
//! threads.
//!
//! A ground is one tank on an open square of grass with a target: an
//! enemy tank standing still, a neutral depot, or a neutral pillbox
//! (phase 1, go to), or an enemy tank to hit (phase 3, aim). An episode
//! is 60 s at most; its score is what the tank earned (`Task::score`).
//!
//! Evolution strategies (Salimans and others 2017): each generation
//! draws pairs of random directions in weight space, plays the weights
//! a little way along each and a little way back on the same seeds,
//! ranks every score, and moves the weights toward the directions that
//! scored better (Adam). No gradients and no GPU: every episode is its
//! own, so every core is filled.
//!
//! `make train PHASE=1` runs `train` (an ignored test, as the census
//! is); `make train PHASE=1 CHECK=nets/phase1.bin` reads a net on held-
//! out seeds against its phase's pass (`check`).

use crate::net::{Net, NetBrain, DECIDE_EVERY};
use crate::Brain;
use treadcore_sim::map::{Map, Terrain};
use treadcore_sim::rules::{DIRECT_RANGE, PILLBOX_HEALTH, PILLBOX_SHELLS};
use treadcore_sim::*;

/// An episode's length: 60 s.
pub const EPISODE: Tick = 60 * TICK_HZ as Tick;
/// Half the side of the open square, tiles.
pub const GROUND: i32 = 24;
/// Within this many tiles a target is reached (LR 95, phase 1).
pub const REACH: f32 = 3.0;

/// What a ground's target is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Enemy,
    Depot,
    Pillbox,
}

pub const TARGETS: [Target; 3] = [Target::Enemy, Target::Depot, Target::Pillbox];

/// A phase's task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Task {
    /// Phase 1: drive within `REACH` of the target.
    GoTo,
    /// Phase 3: hit a still enemy tank, and still go to a depot or a
    /// pillbox (trained together, or the net forgets the going: LR 97).
    Shoot,
}

impl Task {
    /// The task a phase trains (`make train PHASE=`), if it has one yet.
    pub fn phase(p: u32) -> Option<Task> {
        match p {
            1 => Some(Task::GoTo),
            3 => Some(Task::Shoot),
            _ => None,
        }
    }

    /// The target a seed's ground has: the three in turn.
    pub fn target(self, seed: u64) -> Target {
        TARGETS[(seed % 3) as usize]
    }

    /// Whether this target is shot (else gone to).
    pub fn shoots(self, t: Target) -> bool {
        self == Task::Shoot && t == Target::Enemy
    }

    /// The target's distance at the start, tiles.
    fn distance(self, t: Target) -> (f32, f32) {
        if self.shoots(t) { (12.0, 20.0) } else { (10.0, 20.0) }
    }
}

/// A ground: the world, the target's place and kind.
#[derive(Clone)]
pub struct Ground {
    pub world: World,
    pub target: Target,
    pub at: V2,
    pub seed: u64,
}

/// The ground for a task and seed: the seed's generated map with a
/// square of grass `GROUND` tiles each way round its centre, the tank
/// at the centre facing a way the seed draws, the target 10 to 20
/// tiles off in a direction it draws; no other depot, pillbox or swarm.
/// An enemy is felt at any range (the scenario's `advantage`): a tank
/// sees 10 tiles, and phase 1 is going to, not searching.
pub fn ground(task: Task, seed: u64) -> Ground {
    let target = task.target(seed);
    let players = if target == Target::Enemy { 2 } else { 1 };
    let sc = Scenario { seed, swarm_interval: 0, time_limit: 0, advantage: true, ..Scenario::bots(players) };
    let mut w = World::new(sc);
    let mut dice = Rng::new(seed ^ 0x1ea2_7a2c);
    let c = Map::tile_centre(w.map.center());
    for dy in -GROUND..=GROUND {
        for dx in -GROUND..=GROUND {
            let p = c + V2::new(dx as f32, dy as f32);
            if w.map.in_bounds(p) {
                w.map.set(p, Terrain::Grass);
            }
        }
    }
    let (lo, hi) = task.distance(target);
    let at = Map::tile_centre(c + V2::from_heading(dice.range(-std::f32::consts::PI, std::f32::consts::PI)).scale(dice.range(lo, hi)));
    let me = &mut w.tanks[0];
    me.pos = c;
    me.heading = dice.range(-std::f32::consts::PI, std::f32::consts::PI);
    me.speed = 0.0;
    me.goal = None;
    me.route.clear();
    me.boat = false;
    w.pillboxes.clear();
    let depot = w.depots.first().cloned();
    w.depots.clear();
    match target {
        Target::Enemy => {
            let t = &mut w.tanks[1];
            t.pos = at;
            t.heading = dice.range(-std::f32::consts::PI, std::f32::consts::PI);
            t.speed = 0.0;
            t.goal = None;
            t.route.clear();
            t.boat = false;
        }
        Target::Depot => {
            let mut d = depot.expect("a generated map has a depot");
            d.pos = at;
            d.owner = None;
            w.depots.push(d);
        }
        Target::Pillbox => {
            w.pillboxes.push(Pillbox { id: 70_001, pos: at, owner: None, health: PILLBOX_HEALTH, reload: 0, capture: 0, capturer: None, shells: PILLBOX_SHELLS });
        }
    }
    Ground { world: w, target, at, seed }
}

/// What an episode came to.
#[derive(Clone, Copy, Debug, Default)]
pub struct Played {
    pub target: Option<Target>,
    /// The target's distance at the start and the end.
    pub from: f32,
    pub to: f32,
    /// The tick it came within `REACH`, if it did.
    pub reached: Option<Tick>,
    /// Shells fired directly, and those of them that hit the enemy;
    /// shells lobbed by the mortar.
    pub shots: u32,
    pub hits: u32,
    pub lobbed: u32,
    /// The enemy destroyed.
    pub killed: bool,
}

impl Task {
    /// An episode's score, the reward summed. Going to: the share of
    /// the way closed (the drop in distance, a potential: Ng, Harada and
    /// Russell 1999, so circling earns nothing), plus 1 for reaching and
    /// up to 0.5 more the sooner. Shooting (aim: the direct gun): 1 a
    /// direct hit, less 0.05 a direct miss or a lobbed shell (a mortar
    /// on a still tank never misses, and teaches no aim), and up to 0.2
    /// for closing to the gun's reach.
    pub fn score(self, p: &Played) -> f32 {
        match self.shoots(p.target.unwrap_or(Target::Depot)) {
            false => {
                let closed = ((p.from - p.to) / (p.from - REACH).max(1.0)).clamp(-1.0, 1.0);
                let reached = p.reached.map(|t| 1.0 + 0.5 * (1.0 - t as f32 / EPISODE as f32)).unwrap_or(0.0);
                closed + reached
            }
            true => {
                let near = DIRECT_RANGE - 2.0;
                let closed = ((p.from - p.to.max(near)) / (p.from - near).max(1.0)).clamp(-1.0, 1.0);
                p.hits as f32 - 0.05 * (p.shots - p.hits.min(p.shots) + p.lobbed) as f32 + 0.2 * closed
            }
        }
    }

    /// Whether an episode passed its phase's test: within `REACH` inside
    /// the minute; a direct hit on the enemy inside the minute.
    pub fn passed(self, p: &Played) -> bool {
        if self.shoots(p.target.unwrap_or(Target::Depot)) { p.hits > 0 } else { p.reached.is_some() }
    }
}

/// One episode of a brain on a ground. Going to ends when the target is
/// reached; shooting when the enemy is destroyed; both at `EPISODE`.
pub fn play(task: Task, g: &Ground, brain: &mut dyn Brain) -> Played {
    play_on(task, g, brain, None)
}

/// As `play`, recording frames into `rec` (the clips, LR 102).
pub fn play_on(task: Task, g: &Ground, brain: &mut dyn Brain, mut rec: Option<&mut crate::scenario::Recording>) -> Played {
    let mut w = g.world.clone();
    let enemy = w.tanks.get(1).map(|t| t.id);
    let from = w.tanks[0].pos.dist(g.at);
    let mut p = Played { target: Some(g.target), from, to: from, ..Played::default() };
    let mut direct: Vec<EntityId> = Vec::new();
    let at = |w: &World| if g.target == Target::Enemy { w.tanks[1].pos } else { g.at };
    while w.tick < EPISODE && !w.over {
        let mut orders = Vec::new();
        if w.tick % DECIDE_EVERY == 0 {
            let obs = w.observe(0);
            orders = brain.think_on(&obs, Some(&w.map)).into_iter().map(|cmd| Order { player: 0, cmd }).collect();
        }
        w.step(&orders);
        if let Some(r) = rec.as_deref_mut() {
            if w.tick % r.every == 0 {
                r.frames.push(crate::scenario::snap(&w));
            }
        }
        for e in &w.events {
            match e {
                Event::ShellLaunched { shell, owner: 0, kind: ShellKind::Direct, .. } => {
                    p.shots += 1;
                    direct.push(*shell);
                }
                Event::ShellLaunched { owner: 0, .. } => p.lobbed += 1,
                Event::ShellImpact { shell, hits, .. } if direct.contains(shell) && enemy.map(|id| hits.contains(&id)).unwrap_or(false) => p.hits += 1,
                _ => {}
            }
        }
        p.to = w.tanks[0].pos.dist(at(&w));
        if g.target == Target::Enemy && !w.tanks[1].alive {
            p.killed = true;
        }
        if task.shoots(g.target) {
            if p.killed {
                break;
            }
        } else if p.to <= REACH {
            p.reached = Some(w.tick);
            break;
        }
    }
    p
}

/// A normal draw (Box and Muller).
fn normal(r: &mut Rng) -> f32 {
    let u = r.f32().max(1e-7);
    let v = r.f32();
    (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
}

/// The noise of one pair of a generation: a direction, from its seed.
fn direction(gen_seed: u64, i: usize, n: usize) -> Vec<f32> {
    let mut r = Rng::new(gen_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ (i as u64 + 1).wrapping_mul(0xD1B5_4A32_D192_ED03));
    (0..n).map(|_| normal(&mut r)).collect()
}

/// Run `jobs` jobs on `threads` threads, results in order.
pub fn spread<T: Send>(jobs: usize, threads: usize, f: &(dyn Fn(usize) -> T + Sync)) -> Vec<T> {
    let next = std::sync::atomic::AtomicUsize::new(0);
    let out: Vec<std::sync::Mutex<Option<T>>> = (0..jobs).map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| loop {
                let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if j >= jobs {
                    break;
                }
                let v = f(j);
                *out[j].lock().unwrap() = Some(v);
            });
        }
    });
    out.into_iter().map(|m| m.into_inner().unwrap().expect("every job ran")).collect()
}

/// What a net trains on (LR 99): a ground built here (`Task`), or a
/// training map, a scenario file with a `kind = "net"` brain and a
/// `[reward]` (`scenarios/training/`): the same file the scripted brain
/// is read on.
pub enum Course {
    Built(Task),
    File { name: String, doc: crate::scenario::Doc },
}

/// The `[reward]` terms a training map may use (`scenario::reward`).
pub const REWARDS: [&str; 11] = ["pass", "alive", "damage_taken", "damage_given", "killed", "depot_taken", "pillbox_taken", "pillbox_killed", "closer", "alive_time", "armor_gained"];

impl Course {
    /// `phase1`, `phase3`, or a scenario file's path.
    pub fn parse(spec: &str) -> Result<Course, String> {
        if let Some(t) = spec.strip_prefix("phase").and_then(|n| n.parse().ok()).and_then(Task::phase) {
            return Ok(Course::Built(t));
        }
        let text = std::fs::read_to_string(crate::scenario::find(spec)).map_err(|e| format!("{}: {}", spec, e))?;
        let doc = crate::scenario::parse(&text).map_err(|e| format!("{}: {}", spec, e))?;
        if !doc.lists.get("brain").map(|l| l.iter().any(|b| b.get("kind").and_then(crate::scenario::Value::str) == Some("net"))).unwrap_or(false) {
            return Err(format!("{}: no kind = \"net\" brain to train", spec));
        }
        if let Some(k) = doc.reward.keys().find(|k| !REWARDS.contains(&k.as_str())) {
            return Err(format!("{}: no reward term {} (one of {:?})", spec, k, REWARDS));
        }
        let name = std::path::Path::new(spec).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| spec.into());
        Ok(Course::File { name, doc })
    }

    /// The course's name in the trainer's lines: the task, or the
    /// file's stem.
    pub fn name(&self) -> String {
        match self {
            Course::Built(t) => format!("{:?}", t),
            Course::File { name, .. } => name.clone(),
        }
    }

    /// The share of held-out episodes it must pass: a file's `floor`,
    /// else 95%.
    pub fn floor(&self) -> f32 {
        match self {
            Course::File { doc, .. } => doc.top.get("floor").and_then(crate::scenario::Value::num).unwrap_or(0.95) as f32,
            Course::Built(_) => 0.95,
        }
    }

    /// The same file with its net seats given to the scripted brain
    /// (`kind = "commander"`, on Auto): what the scripted brain does on
    /// the same situations.
    pub fn scripted(&self) -> Option<crate::scenario::Doc> {
        let Course::File { doc, .. } = self else { return None };
        let mut d = doc.clone();
        for b in d.lists.get_mut("brain").into_iter().flatten() {
            if b.get("kind").and_then(crate::scenario::Value::str) == Some("net") {
                b.insert("kind".into(), crate::scenario::Value::Str("commander".into()));
                b.remove("file");
            }
        }
        Some(d)
    }
}

/// One episode made ready: a built ground (made once a generation, then
/// played by every net), or a file's seed.
pub enum Episode {
    Ground(Task, Ground),
    File(usize, u64),
}

/// Episode `k` of a generation: the courses in turn.
pub fn prepare(courses: &[Course], seed: u64, k: usize) -> Episode {
    match &courses[k % courses.len()] {
        Course::Built(t) => Episode::Ground(*t, ground(*t, seed)),
        Course::File { .. } => Episode::File(k % courses.len(), seed),
    }
}

/// A net on one episode: its score, and whether it passed.
pub fn episode(courses: &[Course], e: &Episode, net: &Net) -> (f32, bool) {
    match e {
        Episode::Ground(t, g) => {
            let p = play(*t, g, &mut NetBrain::new(net.clone(), "net"));
            (t.score(&p), t.passed(&p))
        }
        Episode::File(c, seed) => {
            let Course::File { name, doc } = &courses[*c] else { unreachable!("a file's episode") };
            let (out, failed) = crate::scenario::run_net(doc, Some(*seed), Some(net)).unwrap_or_else(|e| panic!("{}: {}", name, e));
            (crate::scenario::reward(doc, &out, &failed), failed.is_empty())
        }
    }
}

/// A net on episodes: the mean score, and the share passed.
pub fn judge(courses: &[Course], net: &Net, episodes: &[Episode]) -> (f32, f32) {
    let mut score = 0.0;
    let mut passed = 0;
    for e in episodes {
        let (s, p) = episode(courses, e, net);
        score += s;
        passed += p as usize;
    }
    (score / episodes.len().max(1) as f32, passed as f32 / episodes.len().max(1) as f32)
}

/// Evolution strategies' settings.
#[derive(Clone, Debug)]
pub struct Es {
    /// Pairs a generation (each played forward and back).
    pub pairs: usize,
    /// Grounds each is played on, the same for all in a generation.
    pub seeds: usize,
    /// How far along a direction, in weight units.
    pub sigma: f32,
    /// Adam's step.
    pub rate: f32,
    /// Weight decay a generation.
    pub decay: f32,
    pub threads: usize,
}

/// Adam's moments.
pub struct Adam {
    m: Vec<f32>,
    v: Vec<f32>,
    t: i32,
}

impl Adam {
    /// Moments at zero for `n` weights.
    pub fn new(n: usize) -> Adam {
        Adam { m: vec![0.0; n], v: vec![0.0; n], t: 0 }
    }

    /// Move `theta` up the gradient `g`.
    pub fn step(&mut self, theta: &mut [f32], g: &[f32], rate: f32) {
        let (b1, b2) = (0.9f32, 0.999f32);
        self.t += 1;
        let (c1, c2) = (1.0 - b1.powi(self.t), 1.0 - b2.powi(self.t));
        for i in 0..theta.len() {
            self.m[i] = b1 * self.m[i] + (1.0 - b1) * g[i];
            self.v[i] = b2 * self.v[i] + (1.0 - b2) * g[i] * g[i];
            theta[i] += rate * (self.m[i] / c1) / ((self.v[i] / c2).sqrt() + 1e-8);
        }
    }
}

/// One generation's record.
#[derive(Clone, Copy, Debug)]
pub struct Generation {
    pub n: usize,
    /// The population's mean score and best.
    pub mean: f32,
    pub best: f32,
    /// The weights themselves on the generation's grounds.
    pub score: f32,
    pub passed: f32,
}

/// One generation: pairs of directions played forward and back on
/// `seeds` episodes drawn from `gen_seed` (the courses in turn, so a
/// mix rehearses what was learned before), ranked, and the weights
/// moved.
pub fn generation(courses: &[Course], es: &Es, theta: &mut Vec<f32>, adam: &mut Adam, shape: &Net, n: usize, gen_seed: u64) -> Generation {
    let grounds: Vec<Episode> = spread(es.seeds, es.threads, &|k| prepare(courses, gen_seed.wrapping_mul(1_000).wrapping_add(k as u64), k));
    let dim = theta.len();
    let base = &*theta;
    // Job 2i is pair i forward, 2i + 1 back; the last job the weights
    // themselves.
    let scores = spread(2 * es.pairs + 1, es.threads, &|j| {
        let w: Vec<f32> = if j == 2 * es.pairs {
            base.clone()
        } else {
            let e = direction(gen_seed, j / 2, dim);
            let s = if j % 2 == 0 { es.sigma } else { -es.sigma };
            base.iter().zip(&e).map(|(t, e)| t + s * e).collect()
        };
        judge(courses, &shape.with_flat(&w), &grounds)
    });
    let (score, passed) = scores[2 * es.pairs];
    let pop: Vec<f32> = scores[..2 * es.pairs].iter().map(|s| s.0).collect();
    // Ranks, centred: from -0.5 for the worst to 0.5 for the best, so
    // one lucky score cannot pull the weights alone.
    let mut order: Vec<usize> = (0..pop.len()).collect();
    order.sort_by(|a, b| pop[*a].total_cmp(&pop[*b]));
    let mut util = vec![0.0f32; pop.len()];
    for (r, &i) in order.iter().enumerate() {
        util[i] = r as f32 / (pop.len() - 1).max(1) as f32 - 0.5;
    }
    let mut g = vec![0.0f32; dim];
    for i in 0..es.pairs {
        let d = util[2 * i] - util[2 * i + 1];
        if d != 0.0 {
            for (gk, ek) in g.iter_mut().zip(direction(gen_seed, i, dim)) {
                *gk += d * ek;
            }
        }
    }
    let k = 1.0 / (2.0 * es.pairs as f32 * es.sigma);
    for (gk, t) in g.iter_mut().zip(theta.iter()) {
        *gk = *gk * k - es.decay * t;
    }
    adam.step(theta, &g, es.rate);
    Generation { n, mean: pop.iter().sum::<f32>() / pop.len() as f32, best: pop.iter().cloned().fold(f32::MIN, f32::max), score, passed }
}

/// A brain's reading on held-out grounds: for each target, the share
/// passed and its standard error, and the mean score.
pub fn check(task: Task, seeds: std::ops::Range<u64>, threads: usize, make: &(dyn Fn() -> Box<dyn Brain> + Sync)) -> Vec<(Target, usize, usize, f32)> {
    let seeds: Vec<u64> = seeds.collect();
    let runs = spread(seeds.len(), threads, &|k| {
        let g = ground(task, seeds[k]);
        let mut b = make();
        let p = play(task, &g, b.as_mut());
        (g.target, task.passed(&p), task.score(&p), p)
    });
    TARGETS
        .iter()
        .filter(|t| runs.iter().any(|r| r.0 == **t))
        .map(|&t| {
            let mine: Vec<_> = runs.iter().filter(|r| r.0 == t).collect();
            let passed = mine.iter().filter(|r| r.1).count();
            (t, passed, mine.len(), mine.iter().map(|r| r.2).sum::<f32>() / mine.len() as f32)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The grounds: the tank at the centre on grass, the target where
    /// the ground says and of its kind, open ground between, and a seed
    /// the same ground twice.
    #[test]
    fn a_ground_is_open_and_its_target_is_there() {
        for seed in 0..6 {
            let g = ground(Task::GoTo, seed);
            let w = &g.world;
            let d = w.tanks[0].pos.dist(g.at);
            assert!((9.0..=21.0).contains(&d), "seed {}: target {} tiles off", seed, d);
            assert!(w.map.route(w.tanks[0].pos, g.at).len() > 0, "seed {}: no way to the target", seed);
            match g.target {
                Target::Enemy => assert_eq!(w.tanks[1].pos, g.at),
                Target::Depot => assert!(w.depots.len() == 1 && w.depots[0].pos == g.at && w.depots[0].owner.is_none()),
                Target::Pillbox => assert!(w.pillboxes.len() == 1 && w.pillboxes[0].pos == g.at && w.pillboxes[0].owner.is_none()),
            }
            assert_eq!(ground(Task::GoTo, seed).world.tanks[0].heading, w.tanks[0].heading);
        }
        let kinds: std::collections::HashSet<_> = (0..3).map(|s| format!("{:?}", ground(Task::GoTo, s).target)).collect();
        assert_eq!(kinds.len(), 3);
    }

    /// The ground is fair: a brain that drives straight at the target
    /// reaches each kind well inside the minute; one that holds never
    /// does; and an enemy shot at from inside the gun's reach is hit.
    #[test]
    fn a_straight_drive_reaches_and_a_still_tank_does_not() {
        struct Straight(V2, bool);
        impl Brain for Straight {
            fn name(&self) -> &str {
                "straight"
            }
            fn think(&mut self, obs: &Observation) -> Vec<Command> {
                let Some(me) = obs.me.as_ref() else { return Vec::new() };
                let mut c = vec![Command::Move { to: self.0 }];
                if self.1 && me.reload == 0 && me.pos.dist(self.0) <= DIRECT_RANGE {
                    c.push(Command::FireDirect { at: self.0 });
                }
                c
            }
        }
        struct Still;
        impl Brain for Still {
            fn name(&self) -> &str {
                "still"
            }
            fn think(&mut self, _: &Observation) -> Vec<Command> {
                Vec::new()
            }
        }
        for seed in 0..6 {
            let g = ground(Task::GoTo, seed);
            let p = play(Task::GoTo, &g, &mut Straight(g.at, false));
            assert!(p.reached.map(|t| t < EPISODE / 2).unwrap_or(false), "seed {} {:?}: {:?}", seed, g.target, p);
            assert!(Task::GoTo.score(&p) > 2.0);
            let q = play(Task::GoTo, &g, &mut Still);
            assert!(q.reached.is_none() && Task::GoTo.score(&q).abs() < 1e-6, "{:?}", q);
        }
        let g = ground(Task::Shoot, 0);
        assert_eq!(g.target, Target::Enemy);
        let p = play(Task::Shoot, &g, &mut Straight(g.at, true));
        assert!(p.hits > 0 && Task::Shoot.passed(&p), "{:?}", p);
    }

    /// The training maps (LR 99): every file under `scenarios/training/`
    /// reads as a course (a net seat, known reward terms); a net plays
    /// one the same way twice; its scripted reading puts a Commander in
    /// the net's seat; a reward term that does not exist is refused.
    #[test]
    fn a_training_map_is_a_course() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scenarios");
        // The training maps and the gauntlet (LR 102) alike.
        let mut files: Vec<_> = ["training", "gauntlet"].iter().flat_map(|d| std::fs::read_dir(root.join(d)).expect("scenarios/").filter_map(|e| e.ok().map(|e| e.path()))).filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false)).collect();
        files.sort();
        assert!(files.len() >= 12, "{} training maps and gauntlet tests", files.len());
        let net = Net::tank(1);
        for f in &files {
            let c = Course::parse(&f.to_string_lossy()).unwrap_or_else(|e| panic!("{}", e));
            let courses = [c];
            let a = episode(&courses, &Episode::File(0, 7), &net);
            let b = episode(&courses, &Episode::File(0, 7), &net);
            assert_eq!(a, b, "{}: the same episode twice", f.display());
            assert!(a.0.is_finite());
            let d = courses[0].scripted().unwrap();
            assert!(d.lists["brain"].iter().all(|b| b.get("kind").and_then(crate::scenario::Value::str) != Some("net")), "{}", f.display());
        }
        let bad = std::env::temp_dir().join(format!("treadcore-bad-reward-{}.toml", std::process::id()));
        std::fs::write(&bad, "[[brain]]\nseat = 0\nkind = \"net\"\n[reward]\nluck = 1\n").unwrap();
        let e = Course::parse(&bad.to_string_lossy()).err().expect("refused");
        std::fs::remove_file(&bad).ok();
        assert!(e.contains("luck"), "{}", e);
    }

    /// Evolution moves the weights: on a few grounds, a handful of
    /// generations raise the score, and a generation's seed gives the
    /// same move twice.
    #[test]
    fn evolution_moves_the_weights_the_same_way_twice() {
        let es = Es { pairs: 8, seeds: 3, sigma: 0.05, rate: 0.03, decay: 0.0, threads: 4 };
        let shape = Net::tank(3);
        let run = || {
            let mut theta = shape.flat();
            let mut adam = Adam::new(theta.len());
            let g = generation(&[Course::Built(Task::GoTo)], &es, &mut theta, &mut adam, &shape, 0, 11);
            (theta, g.score)
        };
        let (a, _) = run();
        let (b, _) = run();
        assert_eq!(a, b, "one generation, the same move");
        assert_ne!(a, shape.flat(), "the weights moved");
    }
}

#[cfg(test)]
mod harness {
    use super::*;

    /// A setting from the environment, or `d` when unset, empty or unread.
    fn env<T: std::str::FromStr>(k: &str, d: T) -> T {
        std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
    }

    /// Prints a net's reading on held-out grounds, each target's share
    /// passed with its standard error.
    fn report(task: Task, seeds: std::ops::Range<u64>, threads: usize, net: &Net) -> bool {
        let net = net.clone();
        if task == Task::Shoot {
            let list: Vec<u64> = seeds.clone().collect();
            let list: Vec<u64> = list.into_iter().filter(|s| task.target(*s) == Target::Enemy).collect();
            let runs = spread(list.len(), threads, &|k| play(task, &ground(task, list[k]), &mut NetBrain::new(net.clone(), "net")));
            let (shots, hits) = runs.iter().fold((0, 0), |a, p| (a.0 + p.shots, a.1 + p.hits));
            println!("train: held out net {:?}: {} direct hits of {} direct shots ({:.3} a shot), {} lobbed, {} killed", task, hits, shots, hits as f64 / shots.max(1) as f64, runs.iter().map(|p| p.lobbed).sum::<u32>(), runs.iter().filter(|p| p.killed).count());
        }
        let rows = check(task, seeds.clone(), threads, &|| Box::new(NetBrain::new(net.clone(), "net")));
        if task == Task::GoTo {
            let list: Vec<u64> = seeds.clone().filter(|s| task.target(*s) == Target::Enemy).collect();
            let runs = spread(list.len(), threads, &|k| play(task, &ground(task, list[k]), &mut NetBrain::new(net.clone(), "net")));
            let failed: Vec<&Played> = runs.iter().filter(|p| !task.passed(p)).collect();
            println!("train: held out net GoTo, the enemy ground: {} failed, the enemy destroyed in {} of them", failed.len(), failed.iter().filter(|p| p.killed).count());
        }
        let mut all = true;
        for (t, passed, n, score) in rows {
            let p = passed as f64 / n as f64;
            let se = (p * (1.0 - p) / n as f64).sqrt();
            all &= p >= 0.95;
            println!("train: held out {:?} {:>7} {:.3} +/- {:.3} passed ({} of {}), score {:.2}", t, format!("{:?}", task), p, se, passed, n, score);
        }
        println!("train: held out seeds {}..{}: {}", seeds.start, seeds.end, if all { "PASS" } else { "not yet" });
        all
    }

    /// The scripted bot on the same held-out grounds: its share passed
    /// and, shooting, its hits a shot (a reference, LR 97).
    fn baseline(task: Task, seeds: std::ops::Range<u64>, threads: usize) {
        let list: Vec<u64> = seeds.clone().collect();
        let runs = spread(list.len(), threads, &|k| {
            let g = ground(task, list[k]);
            let mut b = crate::RuleBot::new(list[k], "rulebot");
            play(task, &g, &mut b)
        });
        let passed = runs.iter().filter(|p| task.passed(p)).count();
        let (shots, hits) = runs.iter().fold((0, 0), |a, p| (a.0 + p.shots, a.1 + p.hits));
        println!("train: baseline rulebot {:?}: passed {} of {}, {} direct hits of {} direct shots ({:.3} a shot), {} lobbed", task, passed, runs.len(), hits, shots, hits as f64 / shots.max(1) as f64, runs.iter().map(|p| p.lobbed).sum::<u32>());
    }

    /// A training map read on held-out seeds, by a net or (with none)
    /// by the scripted brain in its seats: the share passed with its
    /// error, the mean reward, and whether it reaches the map's floor.
    fn report_file(c: &Course, seeds: std::ops::Range<u64>, threads: usize, net: Option<&Net>) -> bool {
        let Course::File { name, doc } = c else { return false };
        let doc = if net.is_some() { doc.clone() } else { c.scripted().expect("a file") };
        let list: Vec<u64> = seeds.clone().collect();
        let runs = spread(list.len(), threads, &|k| {
            let (out, failed) = crate::scenario::run_net(&doc, Some(list[k]), net).unwrap_or_else(|e| panic!("{}: {}", name, e));
            (crate::scenario::reward(&doc, &out, &failed), failed.is_empty(), failed.first().cloned())
        });
        let n = runs.len() as f64;
        let passed = runs.iter().filter(|r| r.1).count();
        let p = passed as f64 / n;
        let se = (p * (1.0 - p) / n).sqrt();
        let ok = p >= c.floor() as f64;
        let why: std::collections::BTreeMap<String, usize> = runs.iter().filter_map(|r| r.2.clone()).map(|w| w.split(" at ").next().unwrap_or("").chars().filter(|c| !c.is_ascii_digit()).collect::<String>()).fold(Default::default(), |mut m, w| {
            *m.entry(w).or_insert(0) += 1;
            m
        });
        println!("train: held out {} by {}: {:.3} +/- {:.3} passed ({} of {}), reward {:.2}, floor {:.2}: {}", name, if net.is_some() { "the net" } else { "the scripted brain" }, p, se, passed, runs.len(), runs.iter().map(|r| r.0).sum::<f32>() / n as f32, c.floor(), if ok { "PASS" } else { "not yet" });
        for (w, k) in why.iter().take(4) {
            println!("train:   failed {}: {}", k, w.trim());
        }
        ok
    }

    /// A net (or with none, the scripted brain) on every course's
    /// held-out seeds.
    fn report_all(courses: &[Course], held: u64, threads: usize, net: Option<&Net>) -> bool {
        let mut all = true;
        let mut seen = std::collections::BTreeSet::new();
        // A course listed twice (to weight it) is read once.
        for c in courses.iter().filter(|c| seen.insert(c.name())) {
            let seeds = 9_000_000..9_000_000 + held;
            all &= match (c, net) {
                (Course::Built(t), Some(n)) => report(*t, seeds, threads, n),
                (Course::Built(t), None) => {
                    baseline(*t, seeds, threads);
                    true
                }
                (Course::File { doc, .. }, _) => {
                    let h = doc.top.get("held").and_then(crate::scenario::Value::num).map(|h| h as u64).unwrap_or(held);
                    report_file(c, 9_000_000..9_000_000 + h, threads, net)
                }
            };
        }
        all
    }

    /// The match reading (LR 100): seat 9 of a dealt match (a ranger of
    /// team 0, whom nobody follows) given to the net, or kept by its
    /// scripted brain, on a map from the seed, three respawns, for
    /// `secs`: when the seat first went down (`secs` if never) and how
    /// often. Between the training maps and the census: does the seat
    /// live in a real match's opening minutes?
    fn match_minutes(net: Option<&Net>, seed: u64, secs: u64) -> (f64, u32, f32, u32) {
        const SEAT: usize = 9;
        let names: Vec<(String, bool)> = (0..crate::fleet::SEATS).map(|i| (crate::fleet::name(i), true)).collect();
        let sc = Scenario { seed, players: names, teams: Some(crate::fleet::teams()), time_limit: 0, respawns: 3, ..Scenario::default() };
        let mut w = World::new(sc);
        crate::fleet::new_match();
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..crate::fleet::SEATS)
            .map(|i| match net.filter(|_| i == SEAT) {
                Some(n) => (i as PlayerId, Box::new(NetBrain::new(n.clone(), "net")) as Box<dyn Brain>),
                None => (i as PlayerId, crate::fleet::brain(i, seed ^ i as u64, false)),
            })
            .collect();
        let (mut first, mut deaths, mut up, mut given, mut kills) = (None, 0u32, true, 0f32, 0u32);
        for _ in 0..secs * TICK_HZ as u64 {
            let orders = crate::run_bots(&w, &mut bots);
            w.step(&orders);
            for e in &w.events {
                if let Event::TankDamaged { by: Some(b), damage, armor_left, .. } = e {
                    if *b == SEAT as PlayerId {
                        given += damage;
                        kills += (*armor_left <= 0.0) as u32;
                    }
                }
            }
            let alive = w.tanks.iter().find(|t| t.owner == SEAT as PlayerId).map(|t| t.alive).unwrap_or(false);
            if up && !alive {
                deaths += 1;
                first.get_or_insert(w.tick as f64 / TICK_HZ as f64);
            }
            up = alive;
            if w.over {
                break;
            }
        }
        (first.unwrap_or(secs as f64), deaths, given, kills)
    }

    /// `MATCH=n`: the match reading over n seeds from 8,000,000, the net
    /// (`CHECK`) and the scripted ranger on the same seeds.
    fn match_reading(net: &Net, n: u64, secs: u64, threads: usize) {
        let seeds: Vec<u64> = (8_000_000..8_000_000 + n).collect();
        for (who, net) in [("the net", Some(net)), ("the scripted ranger", None)] {
            let runs = spread(seeds.len(), threads, &|k| match_minutes(net, seeds[k], secs));
            let m = runs.iter().map(|r| r.0).sum::<f64>() / n as f64;
            let se = (runs.iter().map(|r| (r.0 - m).powi(2)).sum::<f64>() / (n as f64 * (n as f64 - 1.0).max(1.0))).sqrt();
            let d = runs.iter().map(|r| r.1 as f64).sum::<f64>() / n as f64;
            let lived = runs.iter().filter(|r| r.1 == 0).count();
            let g = runs.iter().map(|r| r.2 as f64).sum::<f64>() / n as f64;
            let gse = (runs.iter().map(|r| (r.2 as f64 - g).powi(2)).sum::<f64>() / (n as f64 * (n as f64 - 1.0).max(1.0))).sqrt();
            let k = runs.iter().map(|r| r.3 as f64).sum::<f64>() / n as f64;
            println!("train: match, {}: first down at {:.0} +/- {:.0} s of {}, {:.2} deaths, never down in {} of {} matches; damage dealt {:.0} +/- {:.0}, kills {:.2}", who, m, se, secs, d, lived, n, g, gse, k);
        }
    }

    /// A dodger with the net's drive actions only (LR 100): five times
    /// a second, for a shell coming, each of the nine (a Move of
    /// `DRIVE_STEP` toward a sector, or Stop) is scored by where it
    /// leaves the tank when the shell lands, straight at full speed from
    /// now (turning not counted), and the farthest from the impact taken.
    /// It reads the world's shells, as no net can: an upper bound on
    /// what the actions allow.
    struct Oracle;
    impl Brain for Oracle {
        fn name(&self) -> &str {
            "oracle"
        }
        fn think(&mut self, obs: &Observation) -> Vec<Command> {
            if obs.tick % DECIDE_EVERY != 0 {
                return Vec::new();
            }
            let Some(me) = obs.me.as_ref().filter(|t| t.alive) else { return Vec::new() };
            let Some(sh) = obs.shells.iter().filter(|s| !obs.allied(s.owner) && s.impact_tick >= obs.tick).min_by_key(|s| s.impact_tick) else { return Vec::new() };
            let left = (sh.impact_tick - obs.tick) as f32 / TICK_HZ as f32;
            let go = |k: usize| me.pos + V2::from_heading(crate::senses::sector_heading(me.heading, k)).scale((treadcore_sim::rules::TANK_SPEED * left).min(crate::net::DRIVE_STEP));
            let best = (0..crate::senses::SECTORS).max_by(|a, b| go(*a).dist(sh.impact).total_cmp(&go(*b).dist(sh.impact))).unwrap();
            if me.pos.dist(sh.impact) >= go(best).dist(sh.impact) {
                return vec![Command::Stop];
            }
            vec![Command::Move { to: obs.clamp(me.pos + V2::from_heading(crate::senses::sector_heading(me.heading, best)).scale(crate::net::DRIVE_STEP)) }]
        }
    }

    /// When a drive one way is given again: every decision; as
    /// `NetBrain` did (a new choice, or 2 s); or a new choice, 2 s, or
    /// within half a step of its point (LR 100).
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Renew {
        Always,
        TwoSeconds,
        Near,
    }

    /// The bound by simulation (LR 100): map 5's situation built here
    /// (the tank rolling at full speed on a heading the seed draws, a
    /// shell landing within 2.5 tiles of it in 1.5 to 3.5 s), and every
    /// 0.2 s each of the nine drive actions (a Move of `step` tiles
    /// toward a sector, renewed every decision, or Stop) played forward
    /// in a copy of the world to the landing; the one ending farthest
    /// from the impact is taken. Whether the tank is hit, with the
    /// world's own turning and speed.
    fn sim_dodge(seed: u64, step: f32, renew: Renew) -> bool {
        let mut w = World::new(Scenario { seed, swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
        let mut dice = Rng::new(seed ^ 0x5ce4_a210);
        let c = Map::tile_centre(w.map.center());
        for dy in -20..=20 {
            for dx in -20..=20 {
                let p = c + V2::new(dx as f32, dy as f32);
                if w.map.in_bounds(p) {
                    w.map.set(p, Terrain::Grass);
                }
            }
        }
        let h = dice.range(-std::f32::consts::PI, std::f32::consts::PI);
        let t = &mut w.tanks[0];
        t.pos = c;
        t.heading = h;
        t.speed = treadcore_sim::rules::TANK_SPEED;
        t.goal = Some(c + V2::from_heading(h).scale(20.0));
        t.route.clear();
        t.boat = false;
        w.tanks[1].pos = c + V2::new(0.0, 60.0);
        w.pillboxes.clear();
        w.depots.clear();
        let at = c + dice.disc().scale(2.5);
        let from = at + V2::from_heading(dice.range(-std::f32::consts::PI, std::f32::consts::PI)).scale(30.0);
        let lands = (dice.range(1.5, 3.5) * TICK_HZ as f32) as Tick;
        let id = w.next_id;
        w.next_id += 1;
        w.shells.push(Shell { id, owner: 1, kind: ShellKind::Indirect, from, aim: at, impact: at, launch_tick: 0, impact_tick: lands, spotted: true });
        let order = |w: &World, k: usize| -> Command {
            let me = &w.tanks[0];
            if k == crate::senses::SECTORS {
                Command::Stop
            } else {
                Command::Move { to: me.pos + V2::from_heading(crate::senses::sector_heading(me.heading, k)).scale(step) }
            }
        };
        let mut hit = false;
        let mut last: Option<(usize, Tick, V2)> = None;
        while w.tick < lands + 2 {
            let mut orders = Vec::new();
            if w.tick % DECIDE_EVERY == 0 && w.tick < lands {
                let best = (0..=crate::senses::SECTORS)
                    .max_by(|a, b| {
                        let end = |k: usize| {
                            let mut v = w.clone();
                            let mut first = true;
                            while v.tick < lands {
                                let o = if first || v.tick % DECIDE_EVERY == 0 { vec![Order { player: 0, cmd: order(&v, k) }] } else { Vec::new() };
                                first = false;
                                v.step(&o);
                            }
                            v.tanks[0].pos.dist(at)
                        };
                        end(*a).total_cmp(&end(*b))
                    })
                    .unwrap();
                let fresh = match (renew, last) {
                    (Renew::Always, _) | (_, None) => true,
                    (_, Some((k, _, _))) if k != best => true,
                    (_, Some((_, at_, _))) if w.tick >= at_ + crate::net::DRIVE_RENEW => true,
                    (Renew::Near, Some((_, _, to))) => w.tanks[0].pos.dist(to) < step / 2.0,
                    _ => false,
                };
                if fresh {
                    let cmd = order(&w, best);
                    let to = if let Command::Move { to } = cmd { to } else { w.tanks[0].pos };
                    orders.push(Order { player: 0, cmd });
                    last = Some((best, w.tick, to));
                }
            }
            w.step(&orders);
            if std::env::var("PROBE_DEBUG").is_ok() {
                for e in &w.events {
                    eprintln!("probe: tick {} pos ({:.1},{:.1}) speed {:.1} impact ({:.1},{:.1}) at {}: {:?}", w.tick, w.tanks[0].pos.x, w.tanks[0].pos.y, w.tanks[0].speed, at.x, at.y, lands, e);
                }
            }
            hit |= w.events.iter().any(|e| matches!(e, Event::ShellImpact { shell, hits, .. } if *shell == id && hits.contains(&w.tanks[0].id)));
        }
        !hit
    }

    /// `PROBE=oracle GROUND=map`: the oracle in the net's seat on the
    /// held-out seeds.
    fn probe(c: &Course, held: u64, threads: usize) {
        let Course::File { name, doc } = c else { return };
        let list: Vec<u64> = (9_000_000..9_000_000 + held).collect();
        let runs = spread(list.len(), threads, &|k| crate::scenario::run_with(doc, Some(list[k]), None, Some(&|| Box::new(Oracle) as Box<dyn Brain>)).map(|(_, f)| f.is_empty()).unwrap_or(false));
        let p = runs.iter().filter(|r| **r).count() as f64 / runs.len() as f64;
        println!("train: probe {} by the oracle (the net's drive actions, the world's shells): {:.3} +/- {:.3} of {}", name, p, (p * (1.0 - p) / runs.len() as f64).sqrt(), runs.len());
    }

    /// The milestone nets in order (LR 102): each phase's or map's net
    /// as it was kept.
    pub const MILESTONES: [&str; 6] = ["phase1", "phase3", "map4", "map7a", "map5", "map7"];

    /// `make gauntlet` (LR 102): every file in `scenarios/gauntlet/`, for
    /// the scripted brain, the untrained net and each milestone net, on
    /// the same `GAUNTLET_RUNS` seeds (200) from 9,500,000: the share
    /// passed with its error, and the median reaction where it came.
    /// Written to `OUT/gauntlet.tsv` as well.
    fn gauntlet(out: &std::path::Path, threads: usize) {
        let dir = crate::scenario::find("scenarios/gauntlet");
        let mut files: Vec<_> = std::fs::read_dir(&dir).expect("scenarios/gauntlet/").filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().map(|x| x == "toml").unwrap_or(false)).collect();
        files.sort();
        let runs: u64 = env("GAUNTLET_RUNS", 200);
        let mut who: Vec<(String, Option<Net>)> = vec![("scripted".into(), None), ("untrained".into(), Some(Net::tank(1)))];
        for m in MILESTONES {
            let f = crate::scenario::find(&format!("nets/{}.bin", m));
            if let Ok(n) = Net::load(&f) {
                who.push((m.to_string(), Some(n.widen(crate::senses::SENSES))));
            }
        }
        let mut tsv = String::from("item\tbrain\tpassed\truns\tshare\tse\treaction_median\treaction_n\n");
        for f in &files {
            let c = Course::parse(&f.to_string_lossy()).unwrap_or_else(|e| panic!("{}", e));
            let Course::File { name, doc } = &c else { continue };
            let title = doc.top.get("name").and_then(crate::scenario::Value::str).unwrap_or(name).to_string();
            for (w, net) in &who {
                let d = if net.is_some() { doc.clone() } else { c.scripted().expect("a file") };
                let seeds: Vec<u64> = (9_500_000..9_500_000 + runs).collect();
                let res = spread(seeds.len(), threads, &|k| {
                    let (o, failed) = crate::scenario::run_net(&d, Some(seeds[k]), net.as_ref()).unwrap_or_else(|e| panic!("{}: {}", name, e));
                    (failed.is_empty(), crate::scenario::reaction(&d, &o))
                });
                let passed = res.iter().filter(|r| r.0).count();
                let p = passed as f64 / runs as f64;
                let se = (p * (1.0 - p) / runs as f64).sqrt();
                let mut re: Vec<f64> = res.iter().filter_map(|r| r.1).collect();
                re.sort_by(|a, b| a.total_cmp(b));
                let med = re.get(re.len() / 2).copied();
                println!("gauntlet: {:<36} {:<10} {:>5.1}% +/- {:>4.1}  reaction {}", title, w, 100.0 * p, 100.0 * se, med.map(|m| format!("{:.2} s ({} of {})", m, re.len(), runs)).unwrap_or("none".into()));
                tsv += &format!("{}\t{}\t{}\t{}\t{:.4}\t{:.4}\t{}\t{}\n", title, w, passed, runs, p, se, med.map(|m| format!("{:.2}", m)).unwrap_or_default(), re.len());
            }
        }
        std::fs::create_dir_all(out).ok();
        std::fs::write(out.join("gauntlet.tsv"), tsv).unwrap();
        println!("gauntlet: written {}", out.join("gauntlet.tsv").display());
    }

    /// A recording drawn and made a GIF (LR 102): each frame to a PPM,
    /// then `ffmpeg` with a palette of its own. Frames past `secs` are
    /// left out.
    fn gif(rec: &crate::scenario::Recording, label: &str, secs: f64, path: &std::path::Path) -> usize {
        let dir = path.with_extension("frames");
        std::fs::create_dir_all(&dir).unwrap();
        let mut n = 0;
        for f in rec.frames.iter().filter(|f| f.t <= secs) {
            let armor = f.tanks.iter().find(|t| t.0 == 0).map(|t| if t.4 { (100.0 * t.3).round() as i32 } else { 0 }).unwrap_or(0);
            let cap = format!("{} T{:.1} A{}%", label, f.t, armor);
            std::fs::write(dir.join(format!("f{:04}.ppm", n)), crate::clip::render(rec, f, 0, &cap).ppm()).unwrap();
            n += 1;
        }
        let fps = (TICK_HZ as u64 / rec.every.max(1)).max(1);
        let st = std::process::Command::new("ffmpeg")
            .args(["-y", "-loglevel", "error", "-framerate", &fps.to_string(), "-i"])
            .arg(dir.join("f%04d.ppm"))
            .args(["-vf", "split[a][b];[a]palettegen=max_colors=48:stats_mode=diff[p];[b][p]paletteuse=dither=none", "-loop", "0"])
            .arg(path)
            .status()
            .expect("ffmpeg");
        assert!(st.success(), "ffmpeg failed for {}", path.display());
        std::fs::remove_dir_all(&dir).ok();
        n
    }

    /// The match clip (LR 102): seat 9 of a dealt match given to `net`,
    /// recorded round its start for `secs`.
    fn match_clip(net: &Net, seed: u64, secs: u64) -> crate::scenario::Recording {
        const SEAT: usize = 9;
        let names: Vec<(String, bool)> = (0..crate::fleet::SEATS).map(|i| (crate::fleet::name(i), true)).collect();
        let sc = Scenario { seed, players: names, teams: Some(crate::fleet::teams()), time_limit: 0, respawns: 3, ..Scenario::default() };
        let mut w = World::new(sc);
        crate::fleet::new_match();
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..crate::fleet::SEATS)
            .map(|i| if i == SEAT { (i as PlayerId, Box::new(NetBrain::new(net.clone(), "net")) as Box<dyn Brain>) } else { (i as PlayerId, crate::fleet::brain(i, seed ^ i as u64, false)) })
            .collect();
        let start = w.tanks.iter().find(|t| t.owner == SEAT as PlayerId).map(|t| t.pos).unwrap();
        let mut rec = crate::scenario::Recording::round(&w, start, 26, 2);
        for _ in 0..secs * TICK_HZ as u64 {
            let orders = crate::run_bots(&w, &mut bots);
            w.step(&orders);
            if w.tick % rec.every == 0 {
                let mut f = crate::scenario::snap(&w);
                // Seen from seat 9's side: its tank drawn as the learner's.
                for t in f.tanks.iter_mut() {
                    t.0 = if t.0 == SEAT as PlayerId { 0 } else if crate::fleet::teams()[t.0 as usize] == crate::fleet::teams()[SEAT] { 0 } else { t.0.max(1) };
                }
                for d in f.drones.iter_mut() {
                    d.0 = if crate::fleet::teams().get(d.0 as usize) == Some(&crate::fleet::teams()[SEAT]) { 0 } else { d.0.max(1) };
                }
                for p in f.pillboxes.iter_mut().chain(std::iter::empty()) {
                    p.1 = p.1.map(|o| if crate::fleet::teams().get(o as usize) == Some(&crate::fleet::teams()[SEAT]) { 0 } else { o.max(1) });
                }
                for d in f.depots.iter_mut() {
                    d.1 = d.1.map(|o| if crate::fleet::teams().get(o as usize) == Some(&crate::fleet::teams()[SEAT]) { 0 } else { o.max(1) });
                }
                // The learner's own tank first, so its armour heads the caption.
                if let Some(k) = w.tanks.iter().position(|t| t.owner == SEAT as PlayerId) {
                    f.tanks.swap(0, k);
                    f.tanks[0].0 = 0;
                }
                rec.frames.push(f);
            }
        }
        rec
    }

    /// `make clips` (LR 102): a short GIF of each milestone at work, on
    /// its defining test, the first held-out seed it passes (the
    /// untrained net on the first seed), and the newest net in a real
    /// match. `OUT/clips/`, and `clips.tsv` for the report.
    fn clips(out: &std::path::Path, threads: usize) {
        let dir = out.join("clips");
        std::fs::create_dir_all(&dir).unwrap();
        let load = |m: &str| -> Net {
            if m == "untrained" {
                return Net::tank(1);
            }
            Net::load(&crate::scenario::find(&format!("nets/{}.bin", m))).expect("a net").widen(crate::senses::SENSES)
        };
        // name, the net, the course, the label.
        let specs: [(&str, &str, &str, &str); 9] = [
            ("0-untrained", "untrained", "phase1", "UNTRAINED"),
            ("1-phase1", "phase1", "phase1", "P1 GO TO"),
            ("2-phase3", "phase3", "phase3", "P3 SHOOT"),
            ("3-map4", "map4", "scenarios/training/04-the-guarded-depot.toml", "M4 GUARDED DEPOT"),
            ("4-map7a", "map7a", "scenarios/training/07a-low-go-home.toml", "M7A GO HOME"),
            ("5-map5", "map5", "scenarios/training/05-under-a-shell.toml", "M5 SHELL"),
            ("6-map7", "map7", "scenarios/training/07-low-and-far-from-home.toml", "M7 LOW AND FAR"),
            ("7-map7-duel", "map7", "scenarios/training/06-an-enemy-that-fires-back.toml", "M7 NET: DUEL"),
            ("8-map7-gauntlet-g4", "map7", "scenarios/gauntlet/g4-strike-drone-coming.toml", "G4 STRIKE DRONE"),
        ];
        let mut tsv = String::from("clip\tbrain\tcourse\tseed\tpassed\tframes\tseconds\n");
        let made = spread(specs.len(), threads, &|i| {
            let (name, m, course, label) = specs[i];
            let net = load(m);
            let c = Course::parse(course).unwrap_or_else(|e| panic!("{}", e));
            let tries: u64 = if m == "untrained" { 1 } else { 60 };
            let mut pick = None;
            for k in 0..tries {
                let seed = 9_000_000 + k;
                let (passed, rec) = match &c {
                    Course::Built(t) => {
                        // Phase 1's clip shows the pillbox ground; phase 3's the enemy.
                        let seed = if *t == Task::GoTo { 9_000_000 + 3 * k + 2 } else { 9_000_000 + 3 * k };
                        let g = ground(*t, seed);
                        let mut rec = crate::scenario::Recording::round(&g.world, g.world.tanks[0].pos, 20, 2);
                        let p = play_on(*t, &g, &mut NetBrain::new(net.clone(), "net"), Some(&mut rec));
                        (t.passed(&p), (seed, rec))
                    }
                    Course::File { doc, .. } => {
                        let (_, failed, rec) = crate::scenario::run_record(doc, Some(seed), Some(&net), 2).unwrap();
                        (failed.is_empty(), (seed, rec))
                    }
                };
                if passed || k + 1 == tries {
                    pick = Some((passed, rec));
                    break;
                }
            }
            let (passed, (seed, rec)) = pick.unwrap();
            let n = gif(&rec, label, 15.0, &dir.join(format!("{}.gif", name)));
            format!("{}\t{}\t{}\t{}\t{}\t{}\t{:.1}\n", name, m, course, seed, passed, n, n as f64 / 10.0)
        });
        for l in made {
            tsv += &l;
        }
        let rec = match_clip(&load("map7"), 8_000_001, 30);
        let n = gif(&rec, "M7 NET IN A MATCH", 30.0, &dir.join("9-map7-match.gif"));
        tsv += &format!("9-map7-match\tmap7\tmatch\t8000001\t\t{}\t{:.1}\n", n, n as f64 / 10.0);
        std::fs::write(dir.join("clips.tsv"), tsv).unwrap();
        println!("clips: written {}", dir.display());
    }

    /// `TIMING=1` (LR 102): what the learned tank costs alone, on one
    /// thread: a decision (the senses and the forward pass); a one-tank
    /// episode of phase 3's ground; and 120 s of a 16-seat match with
    /// every seat a net against every seat scripted, the world's step
    /// and every brain's think together.
    fn timing() {
        let net = Net::load(&crate::scenario::find("nets/map7.bin")).expect("nets/map7.bin").widen(crate::senses::SENSES);
        let w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(16) });
        let obs = w.observe(0);
        let n = 20_000;
        let t0 = std::time::Instant::now();
        let mut sum = 0.0;
        for _ in 0..n {
            let x = crate::senses::sense(&obs, Some(&w.map), None, &[]).unwrap();
            sum += net.forward(&x)[0];
        }
        let dec = t0.elapsed().as_secs_f64() / n as f64;
        println!("timing: a decision (senses {} and forward pass, {} weights): {:.1} us, {:.0} a second ({})", crate::senses::SENSES, net.parameters(), dec * 1e6, 1.0 / dec, sum.is_finite());
        let t0 = std::time::Instant::now();
        let mut ticks = 0;
        for k in 0..50 {
            let g = ground(Task::Shoot, 9_000_000 + 3 * k);
            let p = play(Task::Shoot, &g, &mut NetBrain::new(net.clone(), "net"));
            ticks += if p.killed { 1 } else { 0 };
        }
        let ep = t0.elapsed().as_secs_f64() / 50.0;
        println!("timing: a phase 3 episode (one tank and its target): {:.2} ms ({} of 50 killed)", ep * 1e3, ticks);
        let _ = ticks;
        for (who, nets) in [("every seat a net", true), ("every seat scripted", false)] {
            let names: Vec<(String, bool)> = (0..crate::fleet::SEATS).map(|i| (crate::fleet::name(i), true)).collect();
            let sc = Scenario { seed: 8_000_001, players: names, teams: Some(crate::fleet::teams()), time_limit: 0, respawns: 3, ..Scenario::default() };
            let mut w = World::new(sc);
            crate::fleet::new_match();
            let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..crate::fleet::SEATS)
                .map(|i| if nets { (i as PlayerId, Box::new(NetBrain::new(net.clone(), "net")) as Box<dyn Brain>) } else { (i as PlayerId, crate::fleet::brain(i, 8_000_001 ^ i as u64, false)) })
                .collect();
            let (mut think, mut step) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
            let secs = 120;
            for _ in 0..secs * TICK_HZ as u64 {
                let a = std::time::Instant::now();
                let orders = crate::run_bots(&w, &mut bots);
                let b = std::time::Instant::now();
                w.step(&orders);
                think += b - a;
                step += b.elapsed();
                if w.over {
                    break;
                }
            }
            let st = w.tick as f64;
            println!("timing: a 16-seat match, {}: {:.1} s of play in {:.2} s ({:.0} times real time); brains {:.1} us a seat-tick, the world {:.1} us a tick", who, st / TICK_HZ as f64, (think + step).as_secs_f64(), st / TICK_HZ as f64 / (think + step).as_secs_f64(), think.as_secs_f64() * 1e6 / (st * 16.0), step.as_secs_f64() * 1e6 / st);
        }
    }

    /// The trainer (LR 97, 99): `make train PHASE=1`, or `GROUND=` a
    /// comma list of courses (`phase1`, `phase3`, training maps' paths)
    /// trained together, each episode the next course in turn. With
    /// `CHECK=file` a net is read on the held-out seeds only; with
    /// `BASELINE=1` the scripted brain is read first. Settings from the
    /// environment: GENS, PAIRS, SEEDS, SIGMA, RATE, DECAY, THREADS,
    /// START (a net to begin from, else `Net::tank(NET_SEED)`), OUT (the
    /// folder for the net and the log), NAME (the net's file name),
    /// HELD (held-out episodes a course; a file's `held` wins).
    #[test]
    #[ignore]
    fn train() {
        let phase: u32 = env("PHASE", 1);
        let ground = std::env::var("GROUND").ok().filter(|g| !g.is_empty());
        let courses: Vec<Course> = match &ground {
            Some(g) => g.split(',').map(|c| Course::parse(c.trim()).unwrap_or_else(|e| panic!("{}", e))).collect(),
            None => vec![Course::Built(Task::phase(phase).unwrap_or_else(|| panic!("no phase {} to train", phase)))],
        };
        let name = env("NAME", if ground.is_some() { "ground".to_string() } else { format!("phase{}", phase) });
        let threads: usize = env("THREADS", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));
        let held: u64 = env("HELD", 999);
        if !std::env::var("TIMING").unwrap_or_default().is_empty() {
            timing();
            return;
        }
        if !std::env::var("CLIPS").unwrap_or_default().is_empty() {
            clips(&std::path::PathBuf::from(env("OUT", "build/train".to_string())), threads);
            return;
        }
        if !std::env::var("GAUNTLET").unwrap_or_default().is_empty() {
            gauntlet(&std::path::PathBuf::from(env("OUT", "build/train".to_string())), threads);
            return;
        }
        if std::env::var("PROBE").map(|p| p == "sim").unwrap_or(false) {
            let n: u64 = env("HELD", 300);
            for renew in [Renew::Always, Renew::TwoSeconds, Renew::Near] {
                let step = crate::net::DRIVE_STEP;
                let runs = spread(n as usize, threads, &|k| sim_dodge(9_000_000 + k as u64, step, renew));
                let p = runs.iter().filter(|r| **r).count() as f64 / n as f64;
                println!("train: probe sim, under a shell, drive step {} tiles, renewed {:?}: not hit {:.3} +/- {:.3} of {}", step, renew, p, (p * (1.0 - p) / n as f64).sqrt(), n);
            }
            return;
        }
        if std::env::var("PROBE").map(|p| p == "oracle").unwrap_or(false) {
            for c in &courses {
                let h = match c {
                    Course::File { doc, .. } => doc.top.get("held").and_then(crate::scenario::Value::num).map(|h| h as u64).unwrap_or(held),
                    _ => held,
                };
                probe(c, h, threads);
            }
            return;
        }
        if !std::env::var("BASELINE").unwrap_or_default().is_empty() {
            report_all(&courses, held, threads, None);
        }
        if let Some(f) = std::env::var("CHECK").ok().filter(|f| !f.is_empty()) {
            let net = Net::load(std::path::Path::new(&f)).expect("a net");
            let m: u64 = env("MATCH", 0);
            if m > 0 {
                match_reading(&net, m, env("MATCH_SECS", 180), threads);
                return;
            }
            report_all(&courses, held, threads, Some(&net));
            return;
        }
        let es = Es { pairs: env("PAIRS", 48), seeds: env("SEEDS", 12), sigma: env("SIGMA", 0.05), rate: env("RATE", 0.02), decay: env("DECAY", 0.0005), threads };
        let out = std::path::PathBuf::from(env("OUT", "build/train".to_string()));
        std::fs::create_dir_all(&out).unwrap();
        let shape = match std::env::var("START") {
            // A net of the senses before the shell contact, widened (LR 100).
            Ok(f) if !f.is_empty() => Net::load(std::path::Path::new(&f)).expect("START: a net").widen(crate::senses::SENSES),
            _ => Net::tank(env("NET_SEED", 1)),
        };
        let gens: usize = env("GENS", 200);
        let first: usize = env("GEN0", 0);
        println!("train: {} on [{}], {} weights, {:?}, {} generations from {}", name, courses.iter().map(|c| c.name()).collect::<Vec<_>>().join(", "), shape.parameters(), es, gens, first);
        let mut theta = shape.flat();
        let mut adam = Adam::new(theta.len());
        let file = out.join(format!("{}.bin", name));
        let t0 = std::time::Instant::now();
        for n in first..first + gens {
            let g = generation(&courses, &es, &mut theta, &mut adam, &shape, n, 1 + n as u64);
            println!("train: gen {:>4} mean {:>6.3} best {:>6.3} net {:>6.3} passed {:.2} ({:.0} s)", g.n, g.mean, g.best, g.score, g.passed, t0.elapsed().as_secs_f64());
            if (n + 1) % 10 == 0 {
                shape.with_flat(&theta).save(&file).unwrap();
            }
        }
        let net = shape.with_flat(&theta);
        net.save(&file).unwrap();
        println!("train: saved {}", file.display());
        let all = report_all(&courses, held, threads, Some(&net));
        println!("train: every course: {}", if all { "PASS" } else { "not yet" });
    }
}
