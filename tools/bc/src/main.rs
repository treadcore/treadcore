//! Behaviour cloning (LAB-REPORT-BRAIN.md section 106): the learned tank's
//! net, 143 senses -> 64 -> 64 -> 42 head scores, trained on the GPU to do
//! what the scripted brain did, from a recording made by the harness
//! (`RECORD=...`, learn.rs `record`), and written in `Net::save`'s form so
//! NetBrain, the gauntlet and the census take it as any other net.
//!
//!   treadcore-bc RECORD.bin OUT.bin [EPOCHS=12] [BATCH=4096] [LR=1e-3]
//!   BC_DEVICE=cuda (default) | flex (Burn's pure-Rust CPU backend)
//!
//! It times itself (LEARNING-DESIGN.md X): reading the recording; the first
//! step (on CUDA, mostly compiling kernels); then every step split into
//! building the batch on the CPU and the step itself on the device (forward,
//! loss, backward, Adam, and the loss read back, which waits for the device).
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
use burn::tensor::{Device, Tensor, TensorData};

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

/// The rows `idx` as tensors: senses [B, S]; per head a one-hot target
/// [B, n] (zero where the label is not trained or not allowed) and an
/// additive mask [B, n] (0 allowed, -1e9 not).
fn batch(d: &Data, idx: &[usize], dev: &Device) -> (Tensor<2>, Vec<(Tensor<2>, Tensor<2>)>) {
    let b = idx.len();
    let mut xs = Vec::with_capacity(b * d.senses);
    for &i in idx {
        xs.extend_from_slice(&d.x[i * d.senses..(i + 1) * d.senses]);
    }
    let x = Tensor::from_data(TensorData::new(xs, [b, d.senses]), dev);
    let mut heads = Vec::new();
    for (h, &n) in HEADS.iter().enumerate() {
        let mut onehot = vec![0f32; b * n];
        let mut mask = vec![0f32; b * n];
        for (r, &i) in idx.iter().enumerate() {
            let m = &d.masks[i];
            for c in 0..n {
                if !allowed(h, c, m) {
                    mask[r * n + c] = -1e9;
                }
            }
            let l = d.labels[i][h] as usize;
            if TRAINED[h] && l < n && allowed(h, l, m) {
                onehot[r * n + l] = 1.0;
            }
        }
        heads.push((Tensor::from_data(TensorData::new(onehot, [b, n]), dev), Tensor::from_data(TensorData::new(mask, [b, n]), dev)));
    }
    (x, heads)
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
    // A fixed shuffle a epoch: a small LCG, so a run is repeatable.
    let mut order = train.clone();
    let mut z: u64 = 0x2545F4914F6CDD1D;
    let t0 = std::time::Instant::now();
    let (mut build_s, mut step_s, mut first_s) = (0f64, 0f64, None::<f64>);
    let mut steps = 0usize;
    for e in 0..epochs {
        let (eb, es) = (build_s, step_s);
        for i in (1..order.len()).rev() {
            z = z.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            order.swap(i, (z >> 33) as usize % (i + 1));
        }
        let mut sum = 0f64;
        let mut batches = 0;
        for chunk in order.chunks(bsz) {
            let tb = std::time::Instant::now();
            let (x, heads) = batch(&d, chunk, &dev);
            build_s += tb.elapsed().as_secs_f64();
            let ts = std::time::Instant::now();
            let y = model.forward(x);
            let mut loss: Option<Tensor<1>> = None;
            let mut k = 0;
            for (h, &w) in HEADS.iter().enumerate() {
                if TRAINED[h] {
                    let (onehot, mask) = heads[h].clone();
                    let logits = y.clone().slice([0..chunk.len(), k..k + w]) + mask;
                    let l = (log_softmax(logits, 1) * onehot).sum().neg() / chunk.len() as f32;
                    loss = Some(match loss { Some(t) => t + l.reshape([1]), None => l.reshape([1]) });
                }
                k += w;
            }
            let loss = loss.unwrap().sum();
            sum += loss.clone().into_scalar::<f32>() as f64;
            batches += 1;
            let grads = GradientsParams::from_grads(loss.backward(), &model);
            model = opt.step(lr, model, grads);
            let st = ts.elapsed().as_secs_f64();
            if first_s.is_none() {
                first_s = Some(st);
            } else {
                step_s += st;
            }
            steps += 1;
        }
        println!("bc: epoch {:2}: training loss {:.4} ({:.0} s; this epoch: building batches {:.1} s, steps {:.1} s)", e + 1, sum / batches as f64, t0.elapsed().as_secs_f64(), build_s - eb, step_s - es);
    }

    let train_s = t0.elapsed().as_secs_f64();
    println!(
        "bc: time: read {:.1} s; first step {:.1} s; then {} steps: building batches {:.1} s, device steps {:.1} s ({:.1} ms a step, {:.0} samples/s through the device)",
        read_s, first_s.unwrap_or(0.0), steps.saturating_sub(1), build_s, step_s,
        1000.0 * step_s / steps.saturating_sub(1).max(1) as f64,
        (steps.saturating_sub(1) * bsz) as f64 / step_s.max(1e-9)
    );
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
        let (x, _) = batch(&d, chunk, &dev);
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
