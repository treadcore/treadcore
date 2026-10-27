//! A learned tank (LR 95, 96): a small net from the senses (senses.rs)
//! to an action, and the Brain that drives by it.
//!
//! The net is a multilayer perceptron: each layer multiplies by a table
//! of weights, adds a bias, and bends the result (`max(0, x)`) in every
//! layer but the last. Its outputs are scores for the action heads
//! (`Head`); in each head the highest score among the choices allowed
//! now (the mask) is taken. Plain Rust, no framework: the census and
//! the trainer run it on any machine.

use crate::calls::{Call, CallKind, Calls, CALL_EVERY, CALL_KINDS};
use crate::senses::{self, SECTORS, SENSES};
use crate::Brain;
use std::io::{Read, Write};
use treadcore_sim::map::Map;
use treadcore_sim::rules::{DIRECT_RANGE, INDIRECT_MAX_RANGE, INDIRECT_MIN_RANGE};
use treadcore_sim::*;

/// The action heads, in the order of the net's outputs, each a short
/// list of choices; choice 0 is always "hold" or the like.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Head {
    /// Hold, then a Move 6 tiles toward each of the 8 sectors (1 to 8);
    /// 9 is stop.
    Drive,
    /// Hold, or a direct shot at the strongest hostile tank in a sector.
    Gun,
    /// Where the shot aims: where the target is, where it will be when
    /// the shell lands (`lead`), or between.
    Lead,
    /// Hold, or an indirect shot at the strongest hostile in a sector.
    Mortar,
    /// Hold, or launch recon, interceptor, strike, longshot (phase 8).
    Drone,
    /// Hold, capture, build, mend, harvest, mine, home (phase 8).
    Squad,
    /// Hold, or call friends (calls.rs): help (come to me), attack (the
    /// nearest enemy), take (the nearest thing to take).
    Signal,
}

/// The heads in output order, with their choices.
pub const HEADS: [(Head, usize); 7] = [
    (Head::Drive, 1 + SECTORS + 1),
    (Head::Gun, 1 + SECTORS),
    (Head::Lead, 3),
    (Head::Mortar, 1 + SECTORS),
    (Head::Drone, 5),
    (Head::Squad, 7),
    (Head::Signal, 1 + crate::calls::CALL_KINDS.len()),
];

/// The net's outputs: every head's choices.
pub const OUTPUTS: usize = {
    let mut n = 0;
    let mut i = 0;
    while i < HEADS.len() {
        n += HEADS[i].1;
        i += 1;
    }
    n
};

/// The hidden layers of the first nets (LR 95): two of 64.
pub const HIDDEN: [usize; 2] = [64, 64];

/// Ticks between decisions: five a second.
pub const DECIDE_EVERY: Tick = (TICK_HZ / 5) as Tick;

/// How far a drive goes toward its sector, tiles.
pub const DRIVE_STEP: f32 = 6.0;

/// Ticks a drive is kept before the same choice is given again (a Move
/// costs the world a route; a tank driving one way says so every 2 s).
pub const DRIVE_RENEW: Tick = 2 * TICK_HZ as Tick;

/// One layer: `out` rows of `inp` weights, and `out` biases.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub inp: usize,
    pub out: usize,
    pub w: Vec<f32>,
    pub b: Vec<f32>,
}

/// A multilayer perceptron.
#[derive(Clone, Debug, PartialEq)]
pub struct Net {
    pub layers: Vec<Layer>,
}

const MAGIC: &[u8; 8] = b"TCNET1\0\0";

impl Net {
    /// A net of the given sizes (inputs, hidden..., outputs), its weights
    /// drawn from a seed: uniform within +/- sqrt(6 / (in + out)), so a
    /// layer's outputs start the size of its inputs (Glorot and Bengio
    /// 2010); biases 0.
    pub fn random(sizes: &[usize], seed: u64) -> Net {
        // SplitMix64 first: `Rng` takes 0 as 1, and a small seed starts
        // its xorshift weakly; mixed, every seed is its own net.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let mut rng = Rng::new(z ^ (z >> 31));
        let layers = sizes
            .windows(2)
            .map(|p| {
                let (inp, out) = (p[0], p[1]);
                let a = (6.0 / (inp + out) as f32).sqrt();
                let w = (0..inp * out).map(|_| (rng.f32() * 2.0 - 1.0) * a).collect();
                Layer { inp, out, w, b: vec![0.0; out] }
            })
            .collect();
        Net { layers }
    }

    /// The first nets' shape: the senses, `HIDDEN`, the heads.
    pub fn tank(seed: u64) -> Net {
        let mut sizes = vec![SENSES];
        sizes.extend(HIDDEN);
        sizes.push(OUTPUTS);
        Net::random(&sizes, seed)
    }

    /// Its weights and biases in one count.
    pub fn parameters(&self) -> usize {
        self.layers.iter().map(|l| l.w.len() + l.b.len()).sum()
    }

    /// The outputs for these inputs.
    pub fn forward(&self, x: &[f32]) -> Vec<f32> {
        let mut a = x.to_vec();
        for (n, l) in self.layers.iter().enumerate() {
            assert_eq!(a.len(), l.inp, "layer {} wants {} inputs", n, l.inp);
            let last = n + 1 == self.layers.len();
            a = (0..l.out)
                .map(|o| {
                    let row = &l.w[o * l.inp..(o + 1) * l.inp];
                    let v = row.iter().zip(&a).map(|(w, x)| w * x).sum::<f32>() + l.b[o];
                    if last { v } else { v.max(0.0) }
                })
                .collect();
        }
        a
    }

    /// Every weight and bias, layer by layer, as one vector (for a
    /// trainer that moves them all at once).
    pub fn flat(&self) -> Vec<f32> {
        self.layers.iter().flat_map(|l| l.w.iter().chain(&l.b).copied()).collect()
    }

    /// The same shape with these weights (`flat`'s order).
    pub fn with_flat(&self, v: &[f32]) -> Net {
        assert_eq!(v.len(), self.parameters());
        let mut k = 0;
        let layers = self
            .layers
            .iter()
            .map(|l| {
                let w = v[k..k + l.w.len()].to_vec();
                k += l.w.len();
                let b = v[k..k + l.b.len()].to_vec();
                k += l.b.len();
                Layer { inp: l.inp, out: l.out, w, b }
            })
            .collect();
        Net { layers }
    }

    /// Written as: the magic, the count of sizes, the sizes (u32), then
    /// each layer's weights and biases (f32), all little-endian.
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        f.write_all(MAGIC)?;
        let mut sizes = vec![self.layers[0].inp as u32];
        sizes.extend(self.layers.iter().map(|l| l.out as u32));
        f.write_all(&(sizes.len() as u32).to_le_bytes())?;
        for s in &sizes {
            f.write_all(&s.to_le_bytes())?;
        }
        for v in self.flat() {
            f.write_all(&v.to_le_bytes())?;
        }
        Ok(())
    }

    pub fn load(path: &std::path::Path) -> std::io::Result<Net> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)?.read_to_end(&mut bytes)?;
        let bad = |m: &str| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{}: {}", path.display(), m));
        if bytes.len() < 12 || &bytes[..8] != MAGIC {
            return Err(bad("not a treadcore net"));
        }
        let u32_at = |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
        let n = u32_at(8) as usize;
        if n < 2 || bytes.len() < 12 + 4 * n {
            return Err(bad("no sizes"));
        }
        let sizes: Vec<usize> = (0..n).map(|i| u32_at(12 + 4 * i) as usize).collect();
        let shape = Net { layers: sizes.windows(2).map(|p| Layer { inp: p[0], out: p[1], w: vec![0.0; p[0] * p[1]], b: vec![0.0; p[1]] }).collect() };
        let start = 12 + 4 * n;
        if bytes.len() != start + 4 * shape.parameters() {
            return Err(bad("the weights do not fit the sizes"));
        }
        let v: Vec<f32> = bytes[start..].chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        Ok(shape.with_flat(&v))
    }
}

/// Each head's choice: the highest score among those `allowed`.
pub fn choose(scores: &[f32], allowed: &dyn Fn(Head, usize) -> bool) -> Vec<(Head, usize)> {
    let mut k = 0;
    HEADS
        .iter()
        .map(|&(h, n)| {
            let best = (0..n).filter(|&c| c == 0 || allowed(h, c)).max_by(|&a, &b| scores[k + a].total_cmp(&scores[k + b]).then(b.cmp(&a))).unwrap_or(0);
            k += n;
            (h, best)
        })
        .collect()
}

/// A Brain that drives by a net (phase 0 and on, LR 96): five times a
/// second it feels (senses.rs), scores, and acts. Drive, gun, lead and
/// mortar act now; the drone, squad and signal heads are scored and
/// held until their phases.
pub struct NetBrain {
    pub net: Net,
    name: String,
    /// The last drive given and when, so a drive one way is said again
    /// only every `DRIVE_RENEW`.
    drive: Option<(usize, Tick)>,
    /// What it chose last, for the status line.
    last: Vec<(Head, usize)>,
    /// The team's queue of calls, and when it last called.
    calls: Calls,
    called: Option<Tick>,
}

impl NetBrain {
    pub fn new(net: Net, name: &str) -> NetBrain {
        assert_eq!(net.layers.first().map(|l| l.inp), Some(SENSES), "a tank's net takes the senses");
        assert_eq!(net.layers.last().map(|l| l.out), Some(OUTPUTS), "a tank's net scores the heads");
        NetBrain { net, name: name.into(), drive: None, last: Vec::new(), calls: Calls::new(), called: None }
    }

    /// The same, sharing a team's queue of calls (one a team a match).
    pub fn with_calls(mut self, calls: Calls) -> NetBrain {
        self.calls = calls;
        self
    }

    /// Where a call of this kind would point: me for help, the nearest
    /// enemy seen (tank, else live hostile pillbox) for attack, the
    /// nearest thing to take for take.
    fn call_at(obs: &Observation, me: &Tank, kind: CallKind) -> Option<V2> {
        let near = |a: &V2, b: &V2| a.dist(me.pos).total_cmp(&b.dist(me.pos));
        match kind {
            CallKind::Help => Some(me.pos),
            CallKind::Attack => obs.hostile_tanks().map(|t| t.pos).min_by(near).or_else(|| obs.pillboxes.iter().filter(|b| b.alive() && b.owner.map(|o| !obs.allied(o)).unwrap_or(false)).map(|b| b.pos).min_by(near)),
            CallKind::Take => obs.depots.iter().filter(|p| !p.owner.map(|o| obs.allied(o)).unwrap_or(false)).map(|p| p.pos).chain(obs.pillboxes.iter().filter(|b| b.owner.is_none()).map(|b| b.pos)).min_by(near),
        }
    }

    /// The strongest hostile tank in sector k, if any.
    fn tank_in(obs: &Observation, me: &Tank, k: usize) -> Option<Tank> {
        obs.hostile_tanks().filter(|t| senses::sector(me.heading, (t.pos - me.pos).heading()) == k).min_by(|a, b| a.pos.dist(me.pos).total_cmp(&b.pos.dist(me.pos))).cloned()
    }

    /// The strongest hostile in sector k the mortar can reach: a tank,
    /// else a live hostile pillbox.
    fn mortar_target(obs: &Observation, me: &Tank, k: usize) -> Option<V2> {
        let reach = |p: V2| (INDIRECT_MIN_RANGE..=INDIRECT_MAX_RANGE).contains(&p.dist(me.pos)) && senses::sector(me.heading, (p - me.pos).heading()) == k;
        let tanks = obs.hostile_tanks().map(|t| t.pos).filter(|p| reach(*p));
        let boxes = obs.pillboxes.iter().filter(|b| b.alive() && b.owner.map(|o| !obs.allied(o)).unwrap_or(false)).map(|b| b.pos).filter(|p| reach(*p));
        tanks.chain(boxes).min_by(|a, b| a.dist(me.pos).total_cmp(&b.dist(me.pos)))
    }
}

impl Brain for NetBrain {
    fn name(&self) -> &str {
        &self.name
    }

    fn think(&mut self, obs: &Observation) -> Vec<Command> {
        self.think_on(obs, None)
    }

    fn think_on(&mut self, obs: &Observation, map: Option<&Map>) -> Vec<Command> {
        if obs.tick % DECIDE_EVERY != 0 {
            return Vec::new();
        }
        self.calls.hear(obs);
        let Some(me) = obs.me.clone().filter(|t| t.alive) else { return Vec::new() };
        let heard = self.calls.heard(obs.player, obs.tick);
        let Some(x) = senses::sense(obs, map, None, &heard) else { return Vec::new() };
        let scores = self.net.forward(&x);
        let ready = me.reload == 0 && me.shells > 0;
        let allowed = |h: Head, c: usize| match h {
            Head::Drive => true,
            Head::Gun => ready && Self::tank_in(obs, &me, c - 1).map(|t| t.pos.dist(me.pos) <= DIRECT_RANGE).unwrap_or(false),
            Head::Lead => true,
            Head::Mortar => ready && Self::mortar_target(obs, &me, c - 1).is_some(),
            Head::Signal => self.called.map(|t| obs.tick >= t + CALL_EVERY).unwrap_or(true) && Self::call_at(obs, &me, CALL_KINDS[c - 1]).is_some(),
            // Their phases: held until then.
            Head::Drone | Head::Squad => false,
        };
        let picks = choose(&scores, &allowed);
        let pick = |h: Head| picks.iter().find(|(g, _)| *g == h).map(|p| p.1).unwrap_or(0);
        let mut cmds = Vec::new();
        match pick(Head::Drive) {
            0 => {}
            c if c == 1 + SECTORS => {
                if self.drive.map(|(k, _)| k != c).unwrap_or(true) {
                    cmds.push(Command::Stop);
                    self.drive = Some((c, obs.tick));
                }
            }
            c => {
                let fresh = self.drive.map(|(k, at)| k != c || obs.tick >= at + DRIVE_RENEW).unwrap_or(true);
                if fresh {
                    let to = obs.clamp(me.pos + V2::from_heading(senses::sector_heading(me.heading, c - 1)).scale(DRIVE_STEP));
                    cmds.push(Command::Move { to });
                    self.drive = Some((c, obs.tick));
                }
            }
        }
        let gun = pick(Head::Gun);
        if gun > 0 {
            if let Some(t) = Self::tank_in(obs, &me, gun - 1) {
                let lead = crate::lead(&me, &t);
                let at = match pick(Head::Lead) {
                    0 => t.pos,
                    1 => lead,
                    _ => t.pos + (lead - t.pos).scale(0.5),
                };
                cmds.push(Command::FireDirect { at });
            }
        } else if let Some(at) = Some(pick(Head::Mortar)).filter(|m| *m > 0).and_then(|m| Self::mortar_target(obs, &me, m - 1)) {
            cmds.push(Command::FireIndirect { at });
        }
        if let Some(kind) = Some(pick(Head::Signal)).filter(|c| *c > 0).map(|c| CALL_KINDS[c - 1]) {
            if let Some(at) = Self::call_at(obs, &me, kind) {
                self.calls.post(Call { from: obs.player, kind, at, tick: obs.tick });
                cmds.push(Command::Ping { at });
                self.called = Some(obs.tick);
            }
        }
        self.last = picks;
        cmds
    }

    fn status(&self) -> String {
        format!("net: {:?}", self.last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The net: its shape (about 13,000 weights), the same outputs for
    /// the same seed, other outputs for another, and a file that reads
    /// back as the same net.
    #[test]
    fn a_net_is_its_seed_and_its_file() {
        let a = Net::tank(7);
        assert_eq!(a.layers[0].inp, SENSES);
        assert_eq!(a.layers.last().unwrap().out, OUTPUTS);
        assert!((10_000..20_000).contains(&a.parameters()), "{} weights", a.parameters());
        let x: Vec<f32> = (0..SENSES).map(|i| (i % 7) as f32 / 7.0).collect();
        assert_eq!(a.forward(&x), Net::tank(7).forward(&x));
        assert_ne!(a.forward(&x), Net::tank(8).forward(&x));
        let path = std::env::temp_dir().join(format!("treadcore-net-test-{}.bin", std::process::id()));
        a.save(&path).unwrap();
        let b = Net::load(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(a, b);
        assert_eq!(a.with_flat(&a.flat()), a);
    }

    /// The mask: a head's choice is never one not allowed; hold always is.
    #[test]
    fn a_masked_choice_is_never_taken() {
        let scores: Vec<f32> = (0..OUTPUTS).map(|i| i as f32).collect();
        let picks = choose(&scores, &|h, _| h == Head::Drive);
        for (h, c) in picks {
            if h == Head::Drive {
                assert_eq!(c, 1 + SECTORS, "drive takes its highest");
            } else {
                assert_eq!(c, 0, "{:?} held", h);
            }
        }
    }

    /// A net drives a tank: random nets seated in a world give orders
    /// the world takes (most seeds drive; a net that prefers to hold may
    /// not), each its own way, and a seed plays the same match twice.
    #[test]
    fn a_random_net_drives_a_tank_the_same_way_twice() {
        let play = |seed: u64| {
            let mut w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(2) });
            let mut b = NetBrain::new(Net::tank(seed), "net");
            let (mut orders, mut moved) = (0, 0.0);
            let start = w.tanks[0].pos;
            for _ in 0..600 {
                let obs = w.observe(0);
                let cmds = b.think_on(&obs, Some(&w.map));
                assert!(cmds.len() <= 3, "{:?}", cmds);
                orders += cmds.len();
                let o: Vec<Order> = cmds.into_iter().map(|cmd| Order { player: 0, cmd }).collect();
                w.step(&o);
            }
            moved += w.tanks[0].pos.dist(start);
            (orders, moved, w.tanks[0].pos)
        };
        let runs: Vec<(usize, f32, V2)> = (0..8).map(play).collect();
        let driving = runs.iter().filter(|r| r.0 > 0 && r.1 > 1.0).count();
        eprintln!("random nets, 30 s each: {:?}", runs.iter().map(|r| (r.0, (r.1 * 10.0).round() / 10.0)).collect::<Vec<_>>());
        assert!(driving >= 4, "{} of 8 random nets drove", driving);
        let ends: std::collections::HashSet<(i32, i32)> = runs.iter().map(|r| r.2.tile()).collect();
        assert!(ends.len() >= 4, "random nets all ended in {} places", ends.len());
        assert_eq!(play(0).2, runs[0].2, "the same match twice");
    }
}

#[cfg(test)]
mod speed {
    use super::*;

    /// Decisions a second (phase 0, LR 96): the senses and the net's
    /// forward pass for one tank in a world of sixteen, on one thread.
    /// `cargo test --release -- --ignored net_decisions_a_second --nocapture`
    #[test]
    #[ignore]
    fn net_decisions_a_second() {
        let w = World::new(Scenario { swarm_interval: 0, time_limit: 0, ..Scenario::bots(16) });
        let obs = w.observe(0);
        let net = Net::tank(1);
        let n = 20_000;
        let t0 = std::time::Instant::now();
        let mut sum = 0.0;
        for _ in 0..n {
            let x = senses::sense(&obs, Some(&w.map), None, &[]).unwrap();
            sum += net.forward(&x)[0];
        }
        let per = t0.elapsed().as_secs_f64() / n as f64;
        eprintln!("net_decisions_a_second: {:.1} us a decision (senses and forward pass), {:.0} a second on one thread ({})", per * 1e6, 1.0 / per, sum.is_finite());
    }
}
