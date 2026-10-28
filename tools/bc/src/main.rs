//! Behaviour cloning (LAB-REPORT-BRAIN.md section 106): the learned tank's
//! net, 143 senses -> 64 -> 64 -> 42 head scores, trained on the GPU to do
//! what the scripted brain did, from a recording made by the harness
//! (`RECORD=...`, learn.rs `record`), and written in `Net::save`'s form so
//! NetBrain, the gauntlet and the census take it as any other net.
//!
//!   treadcore-bc RECORD.bin OUT.bin [EPOCHS=12] [BATCH=4096] [LR=1e-3]
//!   BC_DEVICE=cuda (default) | flex (Burn's pure-Rust CPU backend)
//!   (the fast setting, LR 107: BATCH=32768 LR=1e-2, 0.64 s an epoch on a 1080 Ti)
//!
//! It times itself (LEARNING-DESIGN.md X, LAB-REPORT-BRAIN.md LR 106 E and
//! 107): reading the recording; putting it on the device; then each epoch's
//! wall time, the first apart (on CUDA it compiles and tunes the kernels).
//!
//! Made efficient by LR 106 E's list (LR 107): the whole recording goes to
//! the device once (senses, and each sample's targets and masks laid out
//! for every head) and a batch is gathered there by index (`select`), so a
//! step uploads nothing but its share of the epoch's shuffled order, sent
//! once an epoch; every head's loss is one masked log-softmax over a padded
//! [B, 7, 10] (each head's choices in a row of ten, the unused slots
//! masked), not seven; and the loss is summed on the device and read once
//! an epoch, so the CPU never waits for a step.
//!
//! Each trained head is a softmax over the choices NetBrain would allow at
//! that moment (the recorded masks, as `choose` applies them at play), so
//! the gun head learns whether and where to fire when a shot is possible,
//! not that it usually is not. A label outside its mask (the scripted
//! brain fired at something NetBrain's gun does not target) is left out of
//! that head's loss. Squad and Signal are not trained.
//!
//! The last tenth of the seeds is held out; at the end, each head's
//! agreement with the scripted brain on them is printed beside the
//! agreement of always choosing the commonest label (the rule's part 1).
use burn::module::Module;
use burn::nn::{Linear, LinearConfig, Relu};
use burn::optim::{AdamConfig, GradientsParams};
use burn::tensor::activation::log_softmax;
use burn::tensor::{Device, Int, Tensor, TensorData};

const SECTORS: usize = 8;
/// HEADS in net.rs, in output order: drive, gun, lead, mortar, drone, squad, signal.
const HEADS: [usize; 7] = [1 + SECTORS + 1, 1 + SECTORS, 3, 1 + SECTORS, 5, 7, 4];
const TRAINED: [bool; 7] = [true, true, true, true, true, false, false];
const NAMES: [&str; 7] = ["drive", "gun", "lead", "mortar", "drone", "squad", "signal"];

struct Data {
    senses: usize,
    x: Vec<f32>,
    labels: Vec<[u8; 7]>,
    masks: Vec<[u8; 3]>,
    seeds: Vec<u32>,
}

fn read(path: &str) -> Data {
    let b = std::fs::read(path).expect("the recording");
    assert_eq!(&b[..8], b"TCBC2\0\0\0", "not a TCBC2 recording");
    let senses = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
    let heads = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
    assert_eq!(heads, 7);
    let rec = senses * 4 + heads + 3 + 4;
    assert_eq!((b.len() - 16) % rec, 0, "a recording of whole samples");
    let n = (b.len() - 16) / rec;
    let mut d = Data { senses, x: Vec::with_capacity(n * senses), labels: Vec::with_capacity(n), masks: Vec::with_capacity(n), seeds: Vec::with_capacity(n) };
    for i in 0..n {
        let o = 16 + i * rec;
        for k in 0..senses {
            let p = o + 4 * k;
            d.x.push(f32::from_le_bytes(b[p..p + 4].try_into().unwrap()));
        }
        let o = o + senses * 4;
        d.labels.push(b[o..o + 7].try_into().unwrap());
        d.masks.push(b[o + 7..o + 10].try_into().unwrap());
        d.seeds.push(u32::from_le_bytes(b[o + 10..o + 14].try_into().unwrap()));
    }
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

/// The widest head; each head's choices sit in a row of this many slots.
const SLOT: usize = 10;

/// For each of the 7 x SLOT padded slots, the net output column it reads
/// (an unused slot reads column 0, and is masked).
fn columns() -> Vec<i32> {
    let mut v = Vec::with_capacity(HEADS.len() * SLOT);
    let mut k = 0;
    for &w in HEADS.iter() {
        for j in 0..SLOT {
            v.push(if j < w { (k + j) as i32 } else { 0 });
        }
        k += w;
    }
    v
}

/// Every sample's one-hot targets and additive masks, padded [N, 7 x SLOT]:
/// a target is 1 at the trained, allowed label; a mask is 0 where a choice
/// is allowed and -1e9 where it is not or the slot is unused.
fn targets(d: &Data) -> (Vec<f32>, Vec<f32>) {
    let n = d.labels.len();
    let row = HEADS.len() * SLOT;
    let mut t = vec![0f32; n * row];
    let mut m = vec![-1e9f32; n * row];
    for i in 0..n {
        let mk = &d.masks[i];
        for (h, &w) in HEADS.iter().enumerate() {
            let o = i * row + h * SLOT;
            for c in 0..w {
                if allowed(h, c, mk) {
                    m[o + c] = 0.0;
                }
            }
            let l = d.labels[i][h] as usize;
            if TRAINED[h] && l < w && allowed(h, l, mk) {
                t[o + l] = 1.0;
            }
        }
    }
    (t, m)
}

fn ints(v: &[usize], dev: &Device) -> Tensor<1, Int> {
    let v: Vec<i32> = v.iter().map(|&i| i as i32).collect();
    let n = v.len();
    Tensor::from_data(TensorData::new(v, [n]), dev)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let (rec, out) = (a.get(1).expect("RECORD.bin"), a.get(2).expect("OUT.bin"));
    let epochs: usize = a.get(3).map(|s| s.parse().unwrap()).unwrap_or(12);
    let bsz: usize = a.get(4).map(|s| s.parse().unwrap()).unwrap_or(4096);
    let lr: f64 = a.get(5).map(|s| s.parse().unwrap()).unwrap_or(1e-3);
    let t_read = std::time::Instant::now();
    let d = read(rec);
    let read_s = t_read.elapsed().as_secs_f64();
    let n = d.labels.len();
    let mut seeds: Vec<u32> = d.seeds.clone();
    seeds.sort();
    seeds.dedup();
    let held_from = seeds[seeds.len() - (seeds.len() / 10).max(1)];
    let train: Vec<usize> = (0..n).filter(|&i| d.seeds[i] < held_from).collect();
    let held: Vec<usize> = (0..n).filter(|&i| d.seeds[i] >= held_from).collect();
    println!("bc: {} samples, {} seeds; training on {} (seeds below {}), holding out {}", n, seeds.len(), train.len(), held_from, held.len());

    let kind = std::env::var("BC_DEVICE").unwrap_or_else(|_| "cuda".into());
    let gpu = match kind.as_str() {
        "flex" => Device::flex(),
        _ => Device::cuda(0),
    };
    let dev = gpu.clone().autodiff();
    println!("bc: device {}, read the recording in {:.1} s", kind, read_s);
    let outs: usize = HEADS.iter().sum();
    let mut model = Mlp::new(&dev, d.senses, outs);
    let mut opt = AdamConfig::new().init();

    // The recording onto the device, once.
    let t_up = std::time::Instant::now();
    let (tg, mk) = targets(&d);
    let row = HEADS.len() * SLOT;
    let xs = Tensor::<2>::from_data(TensorData::new(d.x.clone(), [n, d.senses]), &dev);
    let tg = Tensor::<2>::from_data(TensorData::new(tg, [n, row]), &dev);
    let mk = Tensor::<2>::from_data(TensorData::new(mk, [n, row]), &dev);
    let cols = Tensor::<1, Int>::from_data(TensorData::new(columns(), [row]), &dev);
    let _ = mk.clone().slice([0..1, 0..1]).into_data(); // wait for the uploads
    let up_s = t_up.elapsed().as_secs_f64();
    println!("bc: the recording on the device in {:.1} s ({:.2} GB)", up_s, (n * (d.senses + 2 * row) * 4) as f64 / 1e9);

    // A fixed shuffle a epoch: a small LCG, so a run is repeatable.
    let mut order = train.clone();
    let mut z: u64 = 0x2545F4914F6CDD1D;
    let t0 = std::time::Instant::now();
    let mut epoch_s: Vec<f64> = Vec::new();
    let mut steps_a_epoch = 0usize;
    for e in 0..epochs {
        let te = std::time::Instant::now();
        for i in (1..order.len()).rev() {
            z = z.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            order.swap(i, (z >> 33) as usize % (i + 1));
        }
        let perm = ints(&order, &dev);
        let mut sum: Option<Tensor<1>> = None;
        let mut batches = 0;
        let mut s0 = 0;
        while s0 < order.len() {
            let s1 = (s0 + bsz).min(order.len());
            let b = s1 - s0;
            let ids = perm.clone().slice([s0..s1]);
            let x = xs.clone().select(0, ids.clone());
            let t = tg.clone().select(0, ids.clone()).reshape([b, HEADS.len(), SLOT]);
            let m = mk.clone().select(0, ids).reshape([b, HEADS.len(), SLOT]);
            let y = model.forward(x).select(1, cols.clone()).reshape([b, HEADS.len(), SLOT]);
            let loss = (log_softmax(y + m, 2) * t).sum().neg() / b as f32;
            let l = loss.clone().detach().reshape([1]);
            sum = Some(match sum { Some(a) => a + l, None => l });
            batches += 1;
            let grads = GradientsParams::from_grads(loss.backward(), &model);
            model = opt.step(lr, model, grads);
            s0 = s1;
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
        "bc: time: read {:.1} s; on the device {:.1} s; first epoch {:.2} s; then {} epochs of {} steps at {}: {:.3} s +/- {:.3} an epoch ({:.2} ms a step, {:.0} samples/s)",
        read_s, up_s, epoch_s.first().copied().unwrap_or(0.0), settled.len(), steps_a_epoch, bsz, mean, sd,
        1000.0 * mean / steps_a_epoch.max(1) as f64, train.len() as f64 / mean.max(1e-9)
    );
    let train_s = t0.elapsed().as_secs_f64();
    let t_eval = std::time::Instant::now();
    // Held out: each head's choice (argmax among the allowed) against the
    // scripted brain's, beside always-the-commonest-allowed-label.
    let mut agree = [0usize; 7];
    let mut base_agree = [0usize; 7];
    let mut count = [0usize; 7];
    // commonest label a head, from the training rows
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
    let gpu_model = model.clone();
    for chunk in held.chunks(bsz) {
        let x = xs.clone().select(0, ints(chunk, &dev));
        let y: Vec<f32> = gpu_model.forward(x).into_data().try_to_vec::<f32>().unwrap();
        for (r, &i) in chunk.iter().enumerate() {
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
    println!("bc: held out, agreement with the scripted brain (where the head had a choice):");
    for h in 0..7 {
        if TRAINED[h] && count[h] > 0 {
            let p = agree[h] as f64 / count[h] as f64;
            let b = base_agree[h] as f64 / count[h] as f64;
            let se = (p * (1.0 - p) / count[h] as f64).sqrt();
            println!("  {:7} {:6.1}% +/- {:.1}  (always label {}: {:5.1}%)  of {}", NAMES[h], 100.0 * p, 100.0 * se, common[h], 100.0 * b, count[h]);
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
