//! The last stage of Algorhythm: a learned transform over what the rules decided
//! (docs/algorhythm.md).
//!
//! The rules run first and decide, as they always have. Then this reads what the tank
//! senses, the biorhythm and THE RULES' OWN ANSWER, and may move that answer, IF IT IS
//! CONFIDENT. "It will be up to the weight to decide, up to the AI net to override if
//! its confident" (Paul, 30 October).
//!
//! MULTI-STEP: `y` starts at nothing and each step adds to it, reading the inputs and
//! the `y` so far, so a later step can correct an earlier one:
//!
//! ```text
//! y[k+1] = y[k] + tanh(A[k] [x; y[k]] + b[k])
//! ```
//!
//! ALL ZEROS IS THE BRAIN AS IT WAS. `y` stays nothing, the confidence stays 0, and
//! nothing is moved, so training starts exactly where the rules are, the way residual
//! policy learning does. A patch can only be worse than the rules by moving something,
//! and the census reads that as it reads anything else.
//!
//! ONLY THE CHOSEN ONE RUNS. `patch` is a whole number, 0 no patch; at 0 no senses are
//! read here and no time is spent. A slot is one file.

use crate::senses::SENSES;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// What the rules said: the direction to the Move's point, across and down, and how far.
pub const RULES: usize = 3;
/// Everything a step reads besides `y`.
pub const INPUTS: usize = SENSES + crate::bio::WAVES + RULES + crate::front::BASE_SENSE;
/// `y`: how far to move the point, across and down, and the confidence.
pub const OUTPUTS: usize = 3;
const MAGIC: &[u8; 4] = b"TCP1";

/// One step: `OUTPUTS` rows of `INPUTS + OUTPUTS` numbers, and `OUTPUTS` offsets.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub a: Vec<f32>,
    pub b: [f32; OUTPUTS],
}

/// The steps in order.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Patch {
    pub steps: Vec<Step>,
}

impl Patch {
    /// `n` steps of zeros: the rules, unchanged.
    pub fn zero(n: usize) -> Patch {
        Patch { steps: vec![Step { a: vec![0.0; OUTPUTS * (INPUTS + OUTPUTS)], b: [0.0; OUTPUTS] }; n] }
    }

    /// Every number in one list, for a search to move.
    pub fn flat(&self) -> Vec<f32> {
        self.steps.iter().flat_map(|s| s.a.iter().chain(s.b.iter()).copied()).collect()
    }

    /// The same shape, its numbers from `v`.
    pub fn with_flat(&self, v: &[f32]) -> Patch {
        let mut p = self.clone();
        let mut at = 0;
        for s in &mut p.steps {
            let n = s.a.len();
            s.a.copy_from_slice(&v[at..at + n]);
            s.b.copy_from_slice(&v[at + n..at + n + OUTPUTS]);
            at += n + OUTPUTS;
        }
        p
    }

    /// All the steps, in order: `y`.
    pub fn run(&self, x: &[f32]) -> [f32; OUTPUTS] {
        let mut y = [0.0f32; OUTPUTS];
        for s in &self.steps {
            let before = y;
            for (r, out) in y.iter_mut().enumerate() {
                let row = &s.a[r * (INPUTS + OUTPUTS)..(r + 1) * (INPUTS + OUTPUTS)];
                let sum: f32 = row[..INPUTS].iter().zip(x).map(|(a, v)| a * v).sum::<f32>()
                    + row[INPUTS..].iter().zip(&before).map(|(a, v)| a * v).sum::<f32>()
                    + s.b[r];
                *out += sum.tanh();
            }
        }
        y
    }

    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let mut b = Vec::new();
        b.extend_from_slice(MAGIC);
        b.extend_from_slice(&(self.steps.len() as u32).to_le_bytes());
        b.extend_from_slice(&(INPUTS as u32).to_le_bytes());
        for v in self.flat() {
            b.extend_from_slice(&v.to_le_bytes());
        }
        std::fs::write(path, b)
    }

    pub fn load(path: &std::path::Path) -> Result<Patch, String> {
        let b = std::fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        if b.len() < 12 || &b[..4] != MAGIC {
            return Err(format!("{}: not a patch", path.display()));
        }
        let word = |at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize;
        // Senses added since it was trained would feed it the wrong numbers.
        if word(8) != INPUTS {
            return Err(format!("{}: made for {} inputs, these are {}", path.display(), word(8), INPUTS));
        }
        let shape = Patch::zero(word(4));
        let v: Vec<f32> = b[12..].chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        if v.len() != shape.flat().len() {
            return Err(format!("{}: {} numbers, wanted {}", path.display(), v.len(), shape.flat().len()));
        }
        Ok(shape.with_flat(&v))
    }
}

static SLOTS: Mutex<Option<HashMap<usize, Arc<Patch>>>> = Mutex::new(None);

/// Slot `k`, from `CENSUS_PATCHES/k.bin`, read once. A slot asked for and not there
/// stops the run, as a model slot does: the rules played in its place would be read
/// as the patch.
pub fn slot(k: usize) -> Arc<Patch> {
    let mut g = SLOTS.lock().unwrap_or_else(|e| e.into_inner());
    let map = g.get_or_insert_with(HashMap::new);
    map.entry(k)
        .or_insert_with(|| {
            let dir = std::env::var("CENSUS_PATCHES").unwrap_or_else(|_| panic!("patch slot {} is asked for, and CENSUS_PATCHES is not set", k));
            let p = std::path::Path::new(&dir).join(format!("{}.bin", k));
            Arc::new(Patch::load(&p).unwrap_or_else(|e| panic!("patch slot {}: {}", k, e)))
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_of_zeros_says_nothing_with_no_confidence() {
        let x = vec![0.7f32; INPUTS];
        assert_eq!(Patch::zero(3).run(&x), [0.0; OUTPUTS]);
        assert_eq!(Patch::default().run(&x), [0.0; OUTPUTS]);
    }

    #[test]
    fn a_later_step_reads_the_one_before_it() {
        let mut p = Patch::zero(2);
        p.steps[0].b = [0.5, 0.0, 0.0];
        // Step 2 reads step 1's first output, not an input: all the inputs are 0.
        p.steps[1].a[INPUTS + OUTPUTS + INPUTS] = 2.0;
        let y = p.run(&vec![0.0; INPUTS]);
        let first = 0.5f32.tanh();
        assert!((y[0] - first).abs() < 1e-6, "{:?}", y);
        assert!((y[1] - (2.0 * first).tanh()).abs() < 1e-6, "step 2 did not compound: {:?}", y);
    }

    #[test]
    fn a_patch_comes_back_from_its_file_and_a_wrong_one_is_refused() {
        let d = std::env::temp_dir().join(format!("tc-patch-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let f = d.join("1.bin");
        let mut p = Patch::zero(2);
        p.steps[1].b = [0.25, -0.5, 1.0];
        p.steps[0].a[7] = 0.125;
        p.save(&f).unwrap();
        assert_eq!(Patch::load(&f).unwrap(), p);
        std::fs::write(&f, b"nonsense").unwrap();
        assert!(Patch::load(&f).is_err());
        let _ = std::fs::remove_dir_all(d);
    }
}
