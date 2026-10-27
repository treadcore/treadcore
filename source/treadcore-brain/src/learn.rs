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
        if task.shoots(g.target) {
            if !w.tanks[1].alive {
                p.killed = true;
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

/// A net on grounds: the mean score, and the share passed.
pub fn judge(task: Task, net: &Net, grounds: &[Ground]) -> (f32, f32) {
    let mut score = 0.0;
    let mut passed = 0;
    for g in grounds {
        let mut b = NetBrain::new(net.clone(), "net");
        let p = play(task, g, &mut b);
        score += task.score(&p);
        passed += task.passed(&p) as usize;
    }
    (score / grounds.len().max(1) as f32, passed as f32 / grounds.len().max(1) as f32)
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
/// `seeds` grounds drawn from `gen_seed`, ranked, and the weights moved.
pub fn generation(task: Task, es: &Es, theta: &mut Vec<f32>, adam: &mut Adam, shape: &Net, n: usize, gen_seed: u64) -> Generation {
    let grounds: Vec<Ground> = spread(es.seeds, es.threads, &|k| ground(task, gen_seed.wrapping_mul(1_000).wrapping_add(k as u64)));
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
        judge(task, &shape.with_flat(&w), &grounds)
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
            let g = generation(Task::GoTo, &es, &mut theta, &mut adam, &shape, 0, 11);
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

    /// The trainer (LR 97): `make train PHASE=1`, or with `CHECK=file`
    /// a net read on the held-out seeds only. Settings from the
    /// environment: GENS, PAIRS, SEEDS, SIGMA, RATE, DECAY, THREADS,
    /// START (a net to begin from, else `Net::tank(NET_SEED)`), OUT (the
    /// folder for `phaseN.bin` and the log), HELD (held-out grounds).
    #[test]
    #[ignore]
    fn train() {
        let phase: u32 = env("PHASE", 1);
        let task = Task::phase(phase).unwrap_or_else(|| panic!("no phase {} to train", phase));
        let threads: usize = env("THREADS", std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4));
        let held: u64 = env("HELD", 999);
        let held = 9_000_000..9_000_000 + held;
        if std::env::var("BASELINE").map(|v| v == "rulebot").unwrap_or(false) {
            baseline(task, held.clone(), threads);
        }
        if let Some(f) = std::env::var("CHECK").ok().filter(|f| !f.is_empty()) {
            let net = Net::load(std::path::Path::new(&f)).expect("a net");
            report(task, held, threads, &net);
            return;
        }
        let es = Es { pairs: env("PAIRS", 48), seeds: env("SEEDS", 12), sigma: env("SIGMA", 0.05), rate: env("RATE", 0.02), decay: env("DECAY", 0.0005), threads };
        let out = std::path::PathBuf::from(env("OUT", "build/train".to_string()));
        std::fs::create_dir_all(&out).unwrap();
        let shape = match std::env::var("START") {
            Ok(f) if !f.is_empty() => Net::load(std::path::Path::new(&f)).expect("START: a net"),
            _ => Net::tank(env("NET_SEED", 1)),
        };
        let gens: usize = env("GENS", 200);
        let first: usize = env("GEN0", 0);
        println!("train: phase {} {:?}, {} weights, {:?}, {} generations from {}", phase, task, shape.parameters(), es, gens, first);
        let mut theta = shape.flat();
        let mut adam = Adam::new(theta.len());
        let file = out.join(format!("phase{}.bin", phase));
        let t0 = std::time::Instant::now();
        for n in first..first + gens {
            let g = generation(task, &es, &mut theta, &mut adam, &shape, n, 1 + n as u64);
            println!("train: gen {:>4} mean {:>6.3} best {:>6.3} net {:>6.3} passed {:.2} ({:.0} s)", g.n, g.mean, g.best, g.score, g.passed, t0.elapsed().as_secs_f64());
            if (n + 1) % 10 == 0 {
                shape.with_flat(&theta).save(&file).unwrap();
            }
        }
        let net = shape.with_flat(&theta);
        net.save(&file).unwrap();
        println!("train: saved {}", file.display());
        report(task, held, threads, &net);
    }
}
