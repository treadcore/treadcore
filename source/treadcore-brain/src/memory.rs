//! Memories that run out (roadmap step 12b, finding L6 of
//! LAB-REPORT-BRAIN.md). The brain kept its given-up targets in lists
//! that grew all match and were never looked at again: a pillbox walked
//! round twelve times stayed given up after its guards were dead, a
//! depot refused once over the water stayed refused after the bridge
//! was laid. `Forget` keeps each with the tick it was put there, and a
//! memory older than its `keep` is gone. A `keep` of `NEVER` remembers
//! for the match, as the lists did; that is the default until the census
//! promotes a time (the `FORGET` capability, weights.rs).

use treadcore_sim::Tick;

/// Remember for the whole match.
pub const NEVER: Tick = Tick::MAX;

#[derive(Clone, Debug)]
pub struct Forget<T> {
    items: Vec<(T, Tick)>,
    /// Ticks a memory lasts.
    pub keep: Tick,
}

impl<T> Default for Forget<T> {
    fn default() -> Self {
        Forget { items: Vec::new(), keep: NEVER }
    }
}

impl<T: PartialEq + Copy> Forget<T> {
    pub fn new(keep: Tick) -> Self {
        Forget { items: Vec::new(), keep }
    }

    fn live(&self, at: Tick, now: Tick) -> bool {
        self.keep == NEVER || now < at.saturating_add(self.keep)
    }

    /// Remember `x` from `now`; remembered already, the clock restarts.
    pub fn put(&mut self, x: T, now: Tick) {
        self.forget_old(now);
        match self.items.iter_mut().find(|(y, _)| *y == x) {
            Some(e) => e.1 = now,
            None => self.items.push((x, now)),
        }
    }

    /// Still remembered at `now`.
    pub fn has(&self, x: &T, now: Tick) -> bool {
        self.items.iter().any(|(y, at)| y == x && self.live(*at, now))
    }

    /// Drop what has run out.
    pub fn forget_old(&mut self, now: Tick) {
        if self.keep != NEVER {
            let keep = self.keep;
            self.items.retain(|(_, at)| now < at.saturating_add(keep));
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn remove(&mut self, x: &T) {
        self.items.retain(|(y, _)| y != x);
    }

    /// How many are held (old ones included until the next `put`).
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_memory_runs_out_unless_kept_for_ever() {
        let mut f = Forget::new(100);
        f.put(7u32, 10);
        assert!(f.has(&7, 109));
        assert!(!f.has(&7, 110));
        f.put(7, 200);
        assert!(f.has(&7, 250));
        f.put(8, 400);
        assert_eq!(f.len(), 1, "the old one is dropped on the next put");
        let mut ever = Forget::default();
        ever.put(1u32, 0);
        assert!(ever.has(&1, Tick::MAX - 1));
    }
}
