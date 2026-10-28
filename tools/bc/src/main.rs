//! Behaviour cloning (LAB-REPORT-BRAIN.md sections 106 to 108): the learned
//! tank's net, 143 senses -> 64 -> 64 -> 47 head scores, trained on the GPU to
//! do what the scripted brain did, from recordings made by the harness
//! (`RECORD=...`, learn.rs `record`; on the fleet, `scripts/fleet-record.sh`),
//! and written in `Net::save`'s form so NetBrain, the gauntlet and the census
//! take it as any other net.
//!
//!   treadcore-bc RECORDING OUT.bin [EPOCHS=12] [BATCH=4096] [LR=1e-3]
//!   RECORDING: a TCBC2 file, or a directory of them (a fleet run's shards)
//!   BC_DEVICE=cuda (default) | flex (Burn's pure-Rust CPU backend)
//!   HELD=9000054-9000059,9001140-9001259   held-out seed ranges, each judged
//!                                          apart (default: the last tenth)
//!   GPU_GB=6   the training set is kept on the device if it fits in this,
//!              else streamed through it a chunk at a time
//!   (the fast setting, LR 107: BATCH=32768 LR=1e-2, 0.64 s an epoch on a
//!   1080 Ti for 2.2 million samples)
//!
//! It times itself: reading; the device work; each epoch's wall time, the
//! first apart (on CUDA it compiles and tunes the kernels), an epoch ending
//! at its one read of the loss.
//!
//! What makes it fast (LR 107): no step uploads anything or waits for the
//! device. Every head's loss is one masked log-softmax over a padded
//! [B, 7, 10] (each head's choices in a row of ten, the unused slots masked):
//! the 47 scores are gathered into it, the label of each head read back with
//! `gather`, and the loss summed on the device and read once an epoch.
//!
//! What lets it take twenty times the data (LR 108): a sample on the device is
//! compact, 640 bytes: its senses, the slot of each head's label, whether that
//! label counts, and its three mask bytes, expanded on the device from three
//! tables of 256 rows. A training set that fits in GPU_GB goes to the device
//! once and batches are gathered there by index; a larger one is streamed:
//! each epoch's shuffled order is cut into chunks, each chunk assembled on
//! the CPU's threads while the device trains on the one before, uploaded, and
//! cut into batches in order.
//!
//! Each trained head is a softmax over the choices NetBrain would allow at
//! that moment (the recorded masks, as `choose` applies them at play). A
//! label outside its mask is left out of that head's loss. Squad and Signal
//! are not trained. At the end, each head's agreement with the scripted brain
//! on each held-out range is printed beside always choosing the commonest
//! label.
use burn::module::Module;
use burn::nn::{Linear, LinearConfig, Relu};
use burn::optim::{AdamConfig, GradientsParams};
use burn::tensor::activation::log_softmax;
use burn::tensor::{Device, Int, Tensor, TensorData};
use std::time::Instant;

const SECTORS: usize = 8;
/// HEADS in net.rs, in output order: drive, gun, lead, mortar, drone, squad, signal.
const HEADS: [usize; 7] = [1 + SECTORS + 1, 1 + SECTORS, 3, 1 + SECTORS, 5, 7, 4];
const TRAINED: [bool; 7] = [true, true, true, true, true, false, false];
const NAMES: [&str; 7] = ["drive", "gun", "lead", "mortar", "drone", "squad", "signal"];
/// The widest head; each head's choices sit in a row of this many slots.
const SLOT: usize = 10;
const ROW: usize = 7 * SLOT;
/// The heads whose choices a mask byte gates: gun, mortar, drone.
const MASKED: [usize; 3] = [1, 3, 4];

struct Data {
    senses: usize,
    x: Vec<f32>,
    labels: Vec<[u8; 7]>,
    masks: Vec<[u8; 3]>,
    seeds: Vec<u32>,
}

/// Every TCBC2 file named, or found in a named directory, read in parallel
/// straight into one set of arrays.
fn read(path: &str, threads: usize) -> Data {
    let p = std::path::Path::new(path);
    let mut files: Vec<std::path::PathBuf> = if p.is_dir() {
        std::fs::read_dir(p).expect("the directory").flatten().map(|e| e.path()).filter(|f| f.extension().is_some_and(|x| x == "bin")).collect()
    } else {
        vec![p.to_path_buf()]
    };
    files.sort();
    assert!(!files.is_empty(), "no recordings in {path}");
    let senses = 143usize;
    let rec = senses * 4 + 7 + 3 + 4;
    let counts: Vec<usize> = files
        .iter()
        .map(|f| {
            let len = std::fs::metadata(f).expect("a recording").len() as usize;
            assert_eq!((len - 16) % rec, 0, "{} is not whole samples", f.display());
            (len - 16) / rec
        })
        .collect();
    let n: usize = counts.iter().sum();
    let mut d = Data { senses, x: vec![0f32; n * senses], labels: vec![[0u8; 7]; n], masks: vec![[0u8; 3]; n], seeds: vec![0u32; n] };
    // Disjoint slices a file, handed to threads a file at a time.
    let mut jobs = Vec::new();
    {
        let (mut x, mut l, mut m, mut s) = (&mut d.x[..], &mut d.labels[..], &mut d.masks[..], &mut d.seeds[..]);
        for (f, &c) in files.iter().zip(&counts) {
            let (x0, x1) = x.split_at_mut(c * senses);
            let (l0, l1) = l.split_at_mut(c);
            let (m0, m1) = m.split_at_mut(c);
            let (s0, s1) = s.split_at_mut(c);
            jobs.push((f.clone(), x0, l0, m0, s0));
            (x, l, m, s) = (x1, l1, m1, s1);
        }
    }
    let jobs = std::sync::Mutex::new(jobs);
    std::thread::scope(|sc| {
        for _ in 0..threads.max(1) {
            sc.spawn(|| loop {
                let Some((f, x, l, m, s)) = jobs.lock().unwrap().pop() else { break };
                let b = std::fs::read(&f).expect("a recording");
                assert_eq!(&b[..8], b"TCBC2\0\0\0", "{} is not a TCBC2 recording", f.display());
                assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize, senses);
                for i in 0..l.len() {
                    let o = 16 + i * rec;
                    for k in 0..senses {
                        let q = o + 4 * k;
                        x[i * senses + k] = f32::from_le_bytes(b[q..q + 4].try_into().unwrap());
                    }
                    let o = o + senses * 4;
                    l[i] = b[o..o + 7].try_into().unwrap();
                    m[i] = b[o + 7..o + 10].try_into().unwrap();
                    s[i] = u32::from_le_bytes(b[o + 10..o + 14].try_into().unwrap());
                }
            });
        }
    });
    println!("bc: {} recordings", files.len());
    d
}

/// Is choice c of head h allowed for this sample? Hold (0) always; drive and
/// lead always; gun, mortar and drone by their masks; squad and signal never
/// (not trained).
fn allowed(h: usize, c: usize, m: &[u8; 3]) -> bool {
    if c == 0 {
        return true;
    }
    match h {
        0 | 2 => true,
        1 => m[0] >> (c - 1) & 1 == 1,
        3 => m[1] >> (c - 1) & 1 == 1,
        4 => m[2] >> (c - 1) & 1 == 1,
        _ => false,
    }
}

#[derive(Module, Debug)]
struct Mlp {
    a: Linear,
    b: Linear,
    c: Linear,
    act: Relu,
}

impl Mlp {
    fn new(dev: &Device, inp: usize, out: usize) -> Self {
        Mlp { a: LinearConfig::new(inp, 64).init(dev), b: LinearConfig::new(64, 64).init(dev), c: LinearConfig::new(64, out).init(dev), act: Relu::new() }
    }
    fn forward(&self, x: Tensor<2>) -> Tensor<2> {
        let x = self.act.forward(self.a.forward(x));
        let x = self.act.forward(self.b.forward(x));
        self.c.forward(x)
    }
}

/// For each of the 7 x SLOT padded slots, the net output column it reads
/// (an unused slot reads column 0, and is masked).
fn columns() -> Vec<i32> {
    let mut v = Vec::with_capacity(ROW);
    let mut k = 0;
    for &w in HEADS.iter() {
        for j in 0..SLOT {
            v.push(if j < w { (k + j) as i32 } else { 0 });
        }
        k += w;
    }
    v
}

/// The additive masks, built once: a base row (0 where a slot may be chosen
/// whatever the masks say, or is gated by one; -1e9 where it never may) and,
/// for each gated head, a table of 256 rows, one a mask byte, adding -1e9 to
/// that head's choices the byte forbids. base + the three rows a sample's
/// bytes pick = `allowed` for every slot.
fn mask_tables() -> (Vec<f32>, [Vec<f32>; 3]) {
    let mut base = vec![0f32; ROW];
    for (h, &w) in HEADS.iter().enumerate() {
        for c in 0..SLOT {
            let never = c >= w || (c > 0 && !matches!(h, 0..=4));
            if never {
                base[h * SLOT + c] = -1e9;
            }
        }
    }
    let tables = MASKED.map(|h| {
        let mut t = vec![0f32; 256 * ROW];
        for b in 0..256usize {
            for c in 1..HEADS[h] {
                if (b >> (c - 1)) & 1 == 0 {
                    t[b * ROW + h * SLOT + c] = -1e9;
                }
            }
        }
        t
    });
    (base, tables)
}

/// A set of samples on the device, compact.
struct Block {
    n: usize,
    x: Tensor<2>,
    slot: Tensor<2, Int>,
    valid: Tensor<2>,
    mask: Tensor<2, Int>,
}

/// The same, on the host, before it is uploaded.
struct HostBlock {
    x: Vec<f32>,
    slot: Vec<i32>,
    valid: Vec<f32>,
    mask: Vec<i32>,
}

/// Rows `idx` of the data, assembled on `threads` threads.
fn assemble(d: &Data, idx: &[usize], threads: usize) -> HostBlock {
    let (n, s) = (idx.len(), d.senses);
    let mut hb = HostBlock { x: vec![0f32; n * s], slot: vec![0i32; n * 7], valid: vec![0f32; n * 7], mask: vec![0i32; n * 3] };
    let per = n.div_ceil(threads.max(1)).max(1);
    std::thread::scope(|sc| {
        let parts = hb.x.chunks_mut(per * s).zip(hb.slot.chunks_mut(per * 7)).zip(hb.valid.chunks_mut(per * 7)).zip(hb.mask.chunks_mut(per * 3)).zip(idx.chunks(per));
        for ((((x, sl), va), mk), ids) in parts {
            sc.spawn(move || {
                for (r, &i) in ids.iter().enumerate() {
                    x[r * s..(r + 1) * s].copy_from_slice(&d.x[i * s..(i + 1) * s]);
                    let m = &d.masks[i];
                    for (h, &w) in HEADS.iter().enumerate() {
                        let l = d.labels[i][h] as usize;
                        let ok = TRAINED[h] && l < w && allowed(h, l, m);
                        sl[r * 7 + h] = (h * SLOT + if ok { l } else { 0 }) as i32;
                        va[r * 7 + h] = ok as u8 as f32;
                    }
                    for k in 0..3 {
                        mk[r * 3 + k] = m[k] as i32;
                    }
                }
            });
        }
    });
    hb
}

fn upload(hb: HostBlock, s: usize, dev: &Device) -> Block {
    let n = hb.slot.len() / 7;
    Block {
        n,
        // Uploaded flat and reshaped on the device: a [n, 143] upload is
        // stored with each row padded to 256 floats (1,024 bytes a sample,
        // read off two failed allocations, LR 108), a flat one is not.
        x: Tensor::<1>::from_data(TensorData::new(hb.x, [n * s]), dev).reshape([n, s]),
        slot: Tensor::from_data(TensorData::new(hb.slot, [n, 7]), dev),
        valid: Tensor::from_data(TensorData::new(hb.valid, [n, 7]), dev),
        mask: Tensor::from_data(TensorData::new(hb.mask, [n, 3]), dev),
    }
}

fn ints(v: &[usize], dev: &Device) -> Tensor<1, Int> {
    let v: Vec<i32> = v.iter().map(|&i| i as i32).collect();
    let n = v.len();
    Tensor::from_data(TensorData::new(v, [n]), dev)
}

/// The device's constant tensors for the loss.
struct Loss {
    cols: Tensor<1, Int>,
    base: Tensor<2>,
    tables: [Tensor<2>; 3],
}

impl Loss {
    /// The batch's loss: each trained head's masked cross-entropy, summed,
    /// over the batch size.
    fn of(&self, model: &Mlp, x: Tensor<2>, slot: Tensor<2, Int>, valid: Tensor<2>, mask: Tensor<2, Int>) -> Tensor<1> {
        let b = slot.dims()[0];
        let y = model.forward(x).select(1, self.cols.clone());
        let mut m = self.base.clone();
        for k in 0..3 {
            let byte = mask.clone().slice([0..b, k..k + 1]).reshape([b]);
            m = m + self.tables[k].clone().select(0, byte);
        }
        let logp = log_softmax((y + m).reshape([b, 7, SLOT]), 2).reshape([b, ROW]);
        (logp.gather(1, slot) * valid).sum().neg() / b as f32
    }
}

/// HELD=a-b,c-d as ranges of seeds.
fn held_ranges(seeds: &[u32]) -> Vec<(u32, u32)> {
    match std::env::var("HELD").ok().filter(|s| !s.is_empty()) {
        Some(s) => s
            .split(',')
            .map(|r| {
                let (a, b) = r.split_once('-').unwrap_or((r, r));
                (a.trim().parse().expect("HELD: a seed"), b.trim().parse().expect("HELD: a seed"))
            })
            .collect(),
        None => {
            let mut u: Vec<u32> = seeds.to_vec();
            u.sort();
            u.dedup();
            vec![(u[u.len() - (u.len() / 10).max(1)], u[u.len() - 1])]
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (rec, out) = (a.get(1).expect("RECORDING"), a.get(2).expect("OUT.bin"));
    let epochs: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(12);
    let bsz: usize = a.get(4).map(|s| s.parse().unwrap()).unwrap_or(4096);
    let lr: f64 = a.get(5).map(|s| s.parse().unwrap()).unwrap_or(1e-3);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let gpu_gb: f64 = std::env::var("GPU_GB").ok().and_then(|s| s.parse().ok()).unwrap_or(6.0);

    let t_read = Instant::now();
    let d = read(rec, threads);
    let read_s = t_read.elapsed().as_secs_f64();
    let n = d.labels.len();
    let held = held_ranges(&d.seeds);
    let is_held = |s: u32| held.iter().any(|&(a, b)| s >= a && s <= b);
    let train: Vec<usize> = (0..n).filter(|&i| !is_held(d.seeds[i])).collect();
    let mut useeds: Vec<u32> = d.seeds.clone();
    useeds.sort();
    useeds.dedup();
    println!(
        "bc: {} samples, {} seeds, read in {:.1} s ({:.1} GB); training on {}, holding out {} in {:?}",
        n, useeds.len(), read_s, (n * (d.senses * 4 + 14)) as f64 / 1e9, train.len(), n - train.len(), held
    );

    let kind = std::env::var("BC_DEVICE").unwrap_or_else(|_| "cuda".into());
    let gpu = match kind.as_str() {
        "flex" => Device::flex(),
        _ => Device::cuda(0),
    };
    let dev = gpu.clone().autodiff();
    let outs: usize = HEADS.iter().sum();
    let mut model = Mlp::new(&dev, d.senses, outs);
    let mut opt = AdamConfig::new().init();
    let (base, tables) = mask_tables();
    let loss = Loss {
        cols: Tensor::from_data(TensorData::new(columns(), [ROW]), &dev),
        base: Tensor::from_data(TensorData::new(base, [1, ROW]), &dev),
        tables: tables.map(|t| Tensor::from_data(TensorData::new(t, [256, ROW]), &dev)),
    };

    // On the device once, or streamed (see the head of this file).
    let per_sample = d.senses * 4 + 7 * 4 + 7 * 4 + 3 * 4;
    // No single upload over MAX_ROWS samples: CubeCL refused one buffer of
    // 3.7 GB on an 11 GB card with 0.6 GB in use (LR 108).
    const MAX_ROWS: usize = 2_000_000;
    let chunk = ((gpu_gb * 1e9) as usize / per_sample).max(bsz);
    let resident = train.len() <= chunk.min(MAX_ROWS);
    let chunk = chunk.min(MAX_ROWS).max(bsz);
    let t_up = Instant::now();
    let whole = if resident {
        let b = upload(assemble(&d, &train, threads), d.senses, &dev);
        let _ = b.valid.clone().slice([0..1, 0..1]).into_data(); // wait for the upload
        Some(b)
    } else {
        None
    };
    println!(
        "bc: device {}; {} ({:.2} GB a copy of the training set at {} bytes a sample){}",
        kind,
        if resident { "the training set on the device" } else { "the training set streamed" },
        (train.len() * per_sample) as f64 / 1e9,
        per_sample,
        if resident { format!(", in {:.1} s", t_up.elapsed().as_secs_f64()) } else { format!(", {} samples a chunk", chunk) }
    );

    // A fixed shuffle an epoch: a small LCG, so a run is repeatable.
    let mut order: Vec<usize> = if resident { (0..train.len()).collect() } else { train.clone() };
    let mut z: u64 = 0x2545F4914F6CDD1D;
    let t0 = Instant::now();
    let mut epoch_s: Vec<f64> = Vec::new();
    let (mut steps_a_epoch, mut wait_s, mut up_s) = (0usize, 0f64, 0f64);
    for e in 0..epochs {
        let te = Instant::now();
        for i in (1..order.len()).rev() {
            z = z.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            order.swap(i, (z >> 33) as usize % (i + 1));
        }
        let mut sum: Option<Tensor<1>> = None;
        let mut batches = 0;
        let mut step = |x: Tensor<2>, slot: Tensor<2, Int>, valid: Tensor<2>, mask: Tensor<2, Int>, model: Mlp| -> Mlp {
            let l = loss.of(&model, x, slot, valid, mask);
            let lv = l.clone().detach().reshape([1]);
            sum = Some(match sum.take() { Some(a) => a + lv, None => lv });
            batches += 1;
            let grads = GradientsParams::from_grads(l.backward(), &model);
            opt.step(lr, model, grads)
        };
        if let Some(w) = &whole {
            let perm = ints(&order, &dev);
            let mut s0 = 0;
            while s0 < w.n {
                let s1 = (s0 + bsz).min(w.n);
                let ids = perm.clone().slice([s0..s1]);
                model = step(w.x.clone().select(0, ids.clone()), w.slot.clone().select(0, ids.clone()), w.valid.clone().select(0, ids.clone()), w.mask.clone().select(0, ids), model);
                s0 = s1;
            }
        } else {
            // The next chunk is assembled on the CPU while the device trains
            // on this one.
            let chunks: Vec<&[usize]> = order.chunks(chunk).collect();
            let mut next = Some(assemble(&d, chunks[0], threads));
            for k in 0..chunks.len() {
                let tu = Instant::now();
                let b = upload(next.take().unwrap(), d.senses, &dev);
                up_s += tu.elapsed().as_secs_f64();
                std::thread::scope(|sc| {
                    let pre = (k + 1 < chunks.len()).then(|| sc.spawn(|| assemble(&d, chunks[k + 1], threads)));
                    let mut s0 = 0;
                    while s0 < b.n {
                        let s1 = (s0 + bsz).min(b.n);
                        model = step(b.x.clone().slice([s0..s1]), b.slot.clone().slice([s0..s1]), b.valid.clone().slice([s0..s1]), b.mask.clone().slice([s0..s1]), model.clone());
                        s0 = s1;
                    }
                    if let Some(p) = pre {
                        let tw = Instant::now();
                        next = Some(p.join().unwrap());
                        wait_s += tw.elapsed().as_secs_f64();
                    }
                });
            }
        }
        steps_a_epoch = batches;
        // The one wait of the epoch.
        let total = sum.unwrap().into_scalar::<f32>() as f64;
        let es = te.elapsed().as_secs_f64();
        epoch_s.push(es);
        println!("bc: epoch {:2}: training loss {:.4} ({:.2} s; {:.0} s in all)", e + 1, total / batches as f64, es, t0.elapsed().as_secs_f64());
    }

    let settled: Vec<f64> = epoch_s.iter().skip(1).copied().collect();
    let mean = settled.iter().sum::<f64>() / settled.len().max(1) as f64;
    let sd = (settled.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (settled.len().max(2) - 1) as f64).sqrt();
    println!(
        "bc: time: read {:.1} s; first epoch {:.2} s; then {} epochs of {} steps at {}: {:.3} s +/- {:.3} an epoch ({:.2} ms a step, {:.0} samples/s){}",
        read_s, epoch_s.first().copied().unwrap_or(0.0), settled.len(), steps_a_epoch, bsz, mean, sd,
        1000.0 * mean / steps_a_epoch.max(1) as f64, train.len() as f64 / mean.max(1e-9),
        if resident { String::new() } else { format!("; uploading {:.1} s, waiting for the CPU's next chunk {:.1} s, over all epochs", up_s, wait_s) }
    );
    let train_s = t0.elapsed().as_secs_f64();
    drop(whole);

    // Held out: each head's choice (argmax among the allowed) against the
    // scripted brain's, beside always-the-commonest-allowed-label, a range at
    // a time.
    let t_eval = Instant::now();
    let mut common = [0usize; 7];
    for (h, &w) in HEADS.iter().enumerate() {
        let mut c = vec![0usize; w];
        for &i in &train {
            let l = d.labels[i][h] as usize;
            if l < w && allowed(h, l, &d.masks[i]) {
                c[l] += 1;
            }
        }
        common[h] = (0..w).max_by_key(|&j| c[j]).unwrap();
    }
    let infer = model.valid();
    let gdev = gpu.clone();
    for &(lo, hi) in &held {
        let rows: Vec<usize> = (0..n).filter(|&i| d.seeds[i] >= lo && d.seeds[i] <= hi).collect();
        if rows.is_empty() {
            println!("bc: held out {lo}-{hi}: no samples");
            continue;
        }
        let (mut agree, mut base_agree, mut count) = ([0usize; 7], [0usize; 7], [0usize; 7]);
        for part in rows.chunks(1 << 20) {
            let hb = assemble(&d, part, threads);
            let x = Tensor::<2>::from_data(TensorData::new(hb.x, [part.len(), d.senses]), &gdev);
            let y: Vec<f32> = infer.forward(x).into_data().try_to_vec::<f32>().unwrap();
            for (r, &i) in part.iter().enumerate() {
                let m = &d.masks[i];
                let mut k = 0;
                for (h, &w) in HEADS.iter().enumerate() {
                    let l = d.labels[i][h] as usize;
                    // gun, mortar and drone are judged only where they had a choice
                    let choice = match h { 1 => m[0] != 0, 3 => m[1] != 0, 4 => m[2] != 0, _ => true };
                    if TRAINED[h] && l < w && allowed(h, l, m) && choice {
                        let row = &y[r * outs + k..r * outs + k + w];
                        let pick = (0..w).filter(|&c| allowed(h, c, m)).max_by(|&p, &q| row[p].total_cmp(&row[q])).unwrap();
                        let base = if allowed(h, common[h], m) { common[h] } else { 0 };
                        count[h] += 1;
                        agree[h] += (pick == l) as usize;
                        base_agree[h] += (base == l) as usize;
                    }
                    k += w;
                }
            }
        }
        println!("bc: held out, seeds {lo}-{hi}: agreement with the scripted brain (where the head had a choice):");
        for h in 0..7 {
            if TRAINED[h] && count[h] > 0 {
                let p = agree[h] as f64 / count[h] as f64;
                let b = base_agree[h] as f64 / count[h] as f64;
                let se = (p * (1.0 - p) / count[h] as f64).sqrt();
                println!("  {:7} {:6.1}% +/- {:.1}  (always label {}: {:5.1}%)  of {}", NAMES[h], 100.0 * p, 100.0 * se, common[h], 100.0 * b, count[h]);
            }
        }
    }

    // Net::save's form: magic, count of sizes, sizes (u32), then each layer's
    // weights row by row (output-major: w[o * inp + i]) and biases, f32 LE.
    // Burn keeps a Linear's weight as [inp, out], so it is transposed here.
    let mut f: Vec<u8> = b"TCNET1\0\0".to_vec();
    let sizes = [d.senses as u32, 64, 64, outs as u32];
    f.extend_from_slice(&(sizes.len() as u32).to_le_bytes());
    for s in sizes {
        f.extend_from_slice(&s.to_le_bytes());
    }
    for l in [&model.a, &model.b, &model.c] {
        let w: Vec<f32> = l.weight.val().into_data().try_to_vec::<f32>().unwrap();
        let shape = l.weight.val().dims();
        let (inp, out_) = (shape[0], shape[1]);
        for o in 0..out_ {
            for i in 0..inp {
                f.extend_from_slice(&w[i * out_ + o].to_le_bytes());
            }
        }
        let b: Vec<f32> = l.bias.as_ref().expect("a bias").val().into_data().try_to_vec::<f32>().unwrap();
        for v in b {
            f.extend_from_slice(&v.to_le_bytes());
        }
    }
    std::fs::write(out, &f).expect("the net");
    println!("bc: time: training {:.1} s, held-out judgement {:.1} s", train_s, t_eval.elapsed().as_secs_f64());
    println!("bc: wrote {} ({} bytes), {:.0} s in all", out, f.len(), t0.elapsed().as_secs_f64());
}
