//! What the brain spends (roadmap step 12a, LAB-REPORT-BRAIN.md section
//! 5, "measure first"). Counters, one set per thread, so the census tests
//! that run side by side each count their own match: thinks, the Moves
//! the bots asked for, the crossing searches, the plan copies, the
//! crossing answers the cache gave instead, and the time `run_bots`
//! took. Nothing reads them but a census and `Brain::status`-style
//! reports; no decision depends on them, so a replay is the same with
//! them or without.

use std::cell::Cell;
use std::time::Duration;

/// One thread's counts since the last `reset`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Meter {
    /// Seats run (`run_bots`), one per bot per tick.
    pub seats: u64,
    /// Thinks that did a doctrine's work (the RuleBot's 4 Hz).
    pub thinks: u64,
    /// Move commands asked of the world: each is an A* route.
    pub moves: u64,
    /// `Map::crossing` searches run, and those the cache answered.
    pub crossings: u64,
    pub crossings_cached: u64,
    /// Whole-plan copies made to answer a question.
    pub plan_clones: u64,
    /// Nanoseconds inside `run_bots`.
    pub nanos: u64,
    /// Ladder picks under a team posture (LR 24), by the posture read
    /// (economy, fortify, connect, expand, defend), and those the
    /// posture changed from the first rung that held.
    pub postures: [u64; 5],
    pub leaned: u64,
    /// The picks the posture changed, by the rung that held first (row)
    /// and the rung taken (column), in `Rung::ALL`'s order (LR 26).
    pub moved: [[u64; 6]; 6],
    /// Shell dodges (backlog 28, LR 27; by team and taken, backlog
    /// 31): by the seat's group (0 the rest, 1 the seats `trial`
    /// marks), then taken (0) or dropped by the brain (1, a would-be
    /// dodge, the dodge off), then ashore (0) or afloat (1).
    pub dodges: [[[Dodges; 2]; 2]; 2],
    /// Expand's fresh picks with `w_front` on (LR 58), by the seat's
    /// group: with the board's front known, of those the picks not the
    /// depot nearest the base, and with no front known.
    pub fronts: [[u64; 3]; 2],
    /// Seconds of a guard's time leaned toward the front (LR 61), by the
    /// seat's group: rotating with the push point moved toward it, and
    /// at the guard's post moved toward it; the rest as before.
    pub leans: [[u64; 4]; 2],
    /// Seconds afloat in a naval start with the beaches priced by their
    /// islands (LR 62), by the seat's group.
    pub beach_sites: [u64; 2],
    /// Bases chosen by their sites (island_site, LR 73), by the seat's
    /// group: chosen, of those not the nearest depot, and the tiles
    /// from it in all.
    pub base_sites: [[u64; 3]; 2],
    /// Seconds alive (LR 65), by the seat's group: kiting, by the step
    /// (`KiteStep::ALL`'s order), and in all (the last).
    pub kites: [[u64; 9]; 2],
    /// Seconds kiting by whom (LR 66), by group: an intruder at the base
    /// (`Act::Kite`), a wingman's (`WingKite`), a rally's (`RallyKite`).
    pub kite_kinds: [[u64; 3]; 2],
    /// Board rebuilds (a second a team) with a hostile on the held
    /// ground beyond the base's alarm, and those Defend held only for it
    /// (LR 68), by the rebuilding seat's group.
    pub grounds: [[u64; 2]; 2],
    /// Seconds alive with the squad out (LR 83), by group: by its task,
    /// in `SquadTask::ALL`'s order, and seconds alive in all (the last).
    pub squads: [[u64; 14]; 2],
}

/// What a squad out is doing, for the meter (LR 83): a Build by its
/// kind (the walk to the site and the work), a road over water a
/// bridge; material from a forest; a capture, a repair, a lift; the
/// walk home; and out with no task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SquadTask {
    Road,
    Bridge,
    Wall,
    Pillbox,
    Mine,
    Structure,
    Boat,
    Harvest,
    Capture,
    Repair,
    Lift,
    Return,
    Idle,
}

impl SquadTask {
    pub const ALL: [SquadTask; 13] = [
        SquadTask::Road,
        SquadTask::Bridge,
        SquadTask::Wall,
        SquadTask::Pillbox,
        SquadTask::Mine,
        SquadTask::Structure,
        SquadTask::Boat,
        SquadTask::Harvest,
        SquadTask::Capture,
        SquadTask::Repair,
        SquadTask::Lift,
        SquadTask::Return,
        SquadTask::Idle,
    ];
    const NAMES: [&'static str; 13] = ["road", "bridge", "wall", "pillbox", "mine", "structure", "boat", "harvest", "capture", "repair", "lift", "home", "idle"];

    /// The squad's task read: `water` whether the site is water or a
    /// bridge (a road there is a bridge).
    pub fn of(task: &treadcore_sim::PillboyTask, water: bool) -> SquadTask {
        use treadcore_sim::{BuildKind, PillboyTask};
        match task {
            PillboyTask::Build { what, .. } => match what {
                BuildKind::Road if water => SquadTask::Bridge,
                BuildKind::Road => SquadTask::Road,
                BuildKind::Wall | BuildKind::Concrete => SquadTask::Wall,
                BuildKind::Pillbox => SquadTask::Pillbox,
                BuildKind::Mine => SquadTask::Mine,
                BuildKind::Structure(_) => SquadTask::Structure,
                BuildKind::Boat => SquadTask::Boat,
            },
            PillboyTask::Harvest { .. } => SquadTask::Harvest,
            PillboyTask::Capture { .. } => SquadTask::Capture,
            PillboyTask::Repair { .. } => SquadTask::Repair,
            PillboyTask::Lift { .. } => SquadTask::Lift,
            PillboyTask::Return => SquadTask::Return,
        }
    }
}

/// One kind of shell dodge's count: begun, and of those hit before the
/// shell landed, hit by the dodged shell itself (on its landing tick),
/// and killed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dodges {
    pub begun: u64,
    pub hit: u64,
    pub shell: u64,
    pub killed: u64,
}

impl Dodges {
    /// Another count added to this one.
    fn add(&mut self, o: Dodges) {
        self.begun += o.begun;
        self.hit += o.hit;
        self.shell += o.shell;
        self.killed += o.killed;
    }
}

thread_local! {
    static METER: Cell<Meter> = Cell::new(Meter::default());
    /// The seats a census marks as its trial (a bit a player), and the
    /// seat `run_bots` is running: a count's group.
    static SEATS: Cell<(u32, crate::PlayerId)> = const { Cell::new((0, 0)) };
}

/// The trial seats, a bit a player; the rest are group 0. Kept across
/// `reset`.
pub fn trial(mask: u32) {
    SEATS.with(|s| s.set((mask, s.get().1)));
}
/// The group of the seat being run: 1 if `trial` marks it, else 0.
fn group() -> usize {
    SEATS.with(|s| {
        let (mask, p) = s.get();
        (mask >> (p as u32 & 31) & 1) as usize
    })
}

/// Change this thread's counts.
fn bump(f: impl FnOnce(&mut Meter)) {
    METER.with(|m| {
        let mut v = m.get();
        f(&mut v);
        m.set(v);
    });
}

/// A seat run, the player's: the counts after it are its group's.
pub fn seat(p: crate::PlayerId) {
    SEATS.with(|s| s.set((s.get().0, p)));
    bump(|m| m.seats += 1);
}
/// A RuleBot think (four a second at full skill).
pub fn think() {
    bump(|m| m.thinks += 1);
}
/// Move orders given.
pub fn moves(n: u64) {
    bump(|m| m.moves += n);
}
/// A water crossing worked out.
pub fn crossing() {
    bump(|m| m.crossings += 1);
}
/// A water crossing answered from the facts' cache.
pub fn crossing_cached() {
    bump(|m| m.crossings_cached += 1);
}
/// A fortification plan copied.
pub fn plan_clone() {
    bump(|m| m.plan_clones += 1);
}
/// A ladder pick under a posture: the rung the ladder alone takes
/// (`first`) and the one taken with the posture's lean.
pub fn posture(p: crate::board::Posture, first: usize, taken: usize) {
    bump(|m| {
        m.postures[p as usize] += 1;
        if first != taken {
            m.leaned += 1;
            m.moved[first][taken] += 1;
        }
    });
}
/// A shell dodge begun, ashore or afloat, taken or dropped (LR 27).
pub fn dodge(afloat: bool, dropped: bool) {
    let g = group();
    bump(|m| m.dodges[g][dropped as usize][afloat as usize].begun += 1);
}
/// A dodge hit before its shell landed: killed by it, and by the
/// dodged shell itself (the hit on its landing tick).
pub fn dodge_hit(afloat: bool, dropped: bool, killed: bool, shell: bool) {
    let g = group();
    bump(|m| {
        let d = &mut m.dodges[g][dropped as usize][afloat as usize];
        d.hit += 1;
        d.killed += killed as u64;
        d.shell += shell as u64;
    });
}
/// An Expand pick with `w_front` on: the front known or not, and moved
/// from the nearest depot or not.
pub fn front(known: bool, moved: bool) {
    let g = group();
    bump(|m| {
        if known {
            m.fronts[g][0] += 1;
            m.fronts[g][1] += moved as u64;
        } else {
            m.fronts[g][2] += 1;
        }
    });
}
/// A second of the rotation's push point (`post` false) or the guard's
/// post (`post` true) worked out: moved toward the front or not.
pub fn lean(post: bool, moved: bool) {
    let g = group();
    bump(|m| m.leans[g][2 * post as usize + !moved as usize] += 1);
}
/// A second afloat with the beaches priced by their islands.
pub fn beach_site() {
    let g = group();
    bump(|m| m.beach_sites[g] += 1);
}
/// A base chosen by its site, `off` tiles from the nearest depot.
pub fn base_site(off: f32) {
    let g = group();
    bump(|m| {
        m.base_sites[g][0] += 1;
        if off > 0.5 {
            m.base_sites[g][1] += 1;
            m.base_sites[g][2] += off.round() as u64;
        }
    });
}
/// A second alive: kiting at `step`, or not.
pub fn kite(act: crate::status::Act) {
    use crate::status::Act;
    let g = group();
    let kind = match act {
        Act::Kite(_) => Some(0),
        Act::WingKite(_) => Some(1),
        Act::RallyKite(_) => Some(2),
        _ => None,
    };
    bump(|m| {
        if let Some(i) = kind {
            m.kite_kinds[g][i] += 1;
        }
        if let Some(k) = act.kite() {
            m.kites[g][crate::status::KiteStep::ALL.iter().position(|s| *s == k).unwrap_or(0)] += 1;
        }
        m.kites[g][8] += 1;
    });
}
/// A second alive: the squad out at `task`, or aboard (None).
pub fn squad(task: Option<SquadTask>) {
    let g = group();
    bump(|m| {
        if let Some(t) = task {
            m.squads[g][SquadTask::ALL.iter().position(|s| *s == t).unwrap_or(12)] += 1;
        }
        m.squads[g][13] += 1;
    });
}
/// A board's second: an incursion on the held ground, and Defend held
/// only for it.
pub fn ground(incursion: bool, defended: bool) {
    let g = group();
    bump(|m| {
        m.grounds[g][0] += incursion as u64;
        m.grounds[g][1] += defended as u64;
    });
}
/// Time spent running the bots.
pub fn spent(d: Duration) {
    bump(|m| m.nanos += d.as_nanos() as u64);
}

/// This thread's counts.
pub fn read() -> Meter {
    METER.with(|m| m.get())
}

/// Start counting again from zero.
pub fn reset() {
    METER.with(|m| m.set(Meter::default()));
}

impl Meter {
    /// Adds another count's dodges to this one's (a census's total).
    pub fn add_dodges(&mut self, o: &Meter) {
        for (g, og) in self.dodges.iter_mut().zip(o.dodges) {
            for (t, ot) in g.iter_mut().zip(og) {
                for (d, od) in t.iter_mut().zip(ot) {
                    d.add(od);
                }
            }
        }
    }

    /// Adds another count's front picks to this one's.
    pub fn add_fronts(&mut self, o: &Meter) {
        for (g, og) in self.fronts.iter_mut().zip(o.fronts) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
        for (a, b) in self.beach_sites.iter_mut().zip(o.beach_sites) {
            *a += b;
        }
        for (g, og) in self.base_sites.iter_mut().zip(o.base_sites) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
        for (g, og) in self.grounds.iter_mut().zip(o.grounds) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
        for (g, og) in self.squads.iter_mut().zip(o.squads) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
        for (g, og) in self.kite_kinds.iter_mut().zip(o.kite_kinds) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
        for (g, og) in self.kites.iter_mut().zip(o.kites) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
        for (g, og) in self.leans.iter_mut().zip(o.leans) {
            for (a, b) in g.iter_mut().zip(og) {
                *a += b;
            }
        }
    }

    /// The dodges for the meter's line, each kind that happened.
    fn dodge_line(&self) -> String {
        let mut out = String::new();
        for (g, group) in self.dodges.iter().enumerate() {
            for (t, taken) in group.iter().enumerate() {
                for (a, d) in taken.iter().enumerate().filter(|(_, d)| d.begun > 0) {
                    out += &format!(
                        ", dodges{} {} {} {} ({} hit, {} by the shell, {} killed)",
                        if g == 1 { " (trial)" } else { "" },
                        if t == 1 { "dropped" } else { "taken" },
                        if a == 1 { "afloat" } else { "ashore" },
                        d.begun, d.hit, d.shell, d.killed
                    );
                }
            }
        }
        out
    }

    /// Microseconds per seat-tick: what one bot costs a tick, on average.
    pub fn micros_per_seat(&self) -> f64 {
        if self.seats == 0 {
            0.0
        } else {
            self.nanos as f64 / 1000.0 / self.seats as f64
        }
    }

    /// One line for a census: the counts and the cost.
    pub fn line(&self) -> String {
        format!(
            "meter: {} seat-ticks, {} thinks, {} moves, {} crossings (+{} cached), {} plan copies, {:.1} us a seat-tick",
            self.seats,
            self.thinks,
            self.moves,
            self.crossings,
            self.crossings_cached,
            self.plan_clones,
            self.micros_per_seat()
        ) + &self.dodge_line()
            + &self.fronts.iter().enumerate().filter(|(_, f)| f[0] + f[2] > 0).map(|(g, f)| format!(", front picks{} {} ({} moved off the nearest depot, {} with no front)", if g == 1 { " (trial)" } else { "" }, f[0], f[1], f[2])).collect::<String>()
            + &self.beach_sites.iter().enumerate().filter(|(_, n)| **n > 0).map(|(g, n)| format!(", seconds afloat with beaches priced by island{}: {}", if g == 1 { " (trial)" } else { "" }, n)).collect::<String>()
            + &self.base_sites.iter().enumerate().filter(|(_, b)| b[0] > 0).map(|(g, b)| format!(", bases by site{}: {} ({} off the nearest depot, {} tiles in all)", if g == 1 { " (trial)" } else { "" }, b[0], b[1], b[2])).collect::<String>()
            + &self.kites.iter().enumerate().filter(|(_, k)| k[8] > 0).map(|(g, k)| format!(", seconds kiting{}: {} of {} alive (massing {}, closing {}, stepping in {}, stepping out {}, holding {})", if g == 1 { " (trial)" } else { "" }, k[..8].iter().sum::<u64>(), k[8], k[0], k[1], k[2] + k[3], k[4], k[5] + k[6] + k[7])).collect::<String>()
            + &self.grounds.iter().enumerate().filter(|(_, k)| k[0] > 0).map(|(g, k)| format!(", seconds with a hostile on the held ground{}: {} ({} defended for it alone)", if g == 1 { " (trial)" } else { "" }, k[0], k[1])).collect::<String>()
            + &self.kite_kinds.iter().enumerate().filter(|(_, k)| k.iter().sum::<u64>() > 0).map(|(g, k)| format!(", seconds kiting by whom{}: an intruder {}, a wingman {}, a rally {}", if g == 1 { " (trial)" } else { "" }, k[0], k[1], k[2])).collect::<String>()
            + &self.squads.iter().enumerate().filter(|(_, q)| q[13] > 0).map(|(g, q)| format!(", seconds the squad out{}: {} of {} alive ({})", if g == 1 { " (trial)" } else { "" }, q[..13].iter().sum::<u64>(), q[13], SquadTask::NAMES.iter().zip(&q[..13]).map(|(n, v)| format!("{} {}", n, v)).collect::<Vec<_>>().join(", "))).collect::<String>()
            + &self.leans.iter().enumerate().filter(|(_, l)| l.iter().sum::<u64>() > 0).map(|(g, l)| format!(", seconds leaned to the front{}: push {} of {}, post {} of {}", if g == 1 { " (trial)" } else { "" }, l[0], l[0] + l[1], l[2], l[2] + l[3])).collect::<String>()
            + &if self.postures.iter().sum::<u64>() > 0 {
            let names = ["defend base", "fortify", "turrets", "hunt", "expand", "guard"];
            let mut moves: Vec<(u64, String)> = Vec::new();
            for (i, row) in self.moved.iter().enumerate() {
                for (j, n) in row.iter().enumerate().filter(|(_, n)| **n > 0) {
                    moves.push((*n, format!("{} to {} {}", names[i], names[j], n)));
                }
            }
            moves.sort_by(|a, b| b.0.cmp(&a.0));
            format!(", postures {:?} (economy fortify connect expand defend), {} picks leaned: {}", self.postures, self.leaned, moves.into_iter().map(|(_, t)| t).collect::<Vec<_>>().join(", "))
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_meter_counts_and_resets() {
        reset();
        seat(0);
        seat(1);
        moves(3);
        crossing();
        spent(Duration::from_micros(10));
        let m = read();
        assert_eq!((m.seats, m.moves, m.crossings), (2, 3, 1));
        assert!((m.micros_per_seat() - 5.0).abs() < 1e-9);
        reset();
        assert_eq!(read(), Meter::default());
    }

    /// A squad's second counts under its seat's group, by its task, a
    /// road over water as a bridge (LR 83).
    #[test]
    fn a_squad_second_counts_by_its_task() {
        use treadcore_sim::{BuildKind, PillboyTask, V2};
        reset();
        trial(1 << 1);
        seat(1);
        let road = PillboyTask::Build { what: BuildKind::Road, at: V2::new(0.0, 0.0) };
        squad(Some(SquadTask::of(&road, true)));
        squad(Some(SquadTask::of(&road, false)));
        squad(None);
        seat(0);
        squad(Some(SquadTask::of(&PillboyTask::Harvest { at: V2::new(0.0, 0.0) }, false)));
        let m = read();
        assert_eq!(m.squads[1][..2], [1, 1]);
        assert_eq!(m.squads[1][13], 3);
        assert_eq!(m.squads[0][7], 1);
        assert!(m.line().contains("seconds the squad out (trial): 2 of 3 alive (road 1, bridge 1, wall 0"), "{}", m.line());
        trial(0);
        reset();
    }

    /// A dodge counts under the seat that made it (the trial's or the
    /// rest), taken or dropped, ashore or afloat (backlog 31).
    #[test]
    fn a_dodge_counts_by_its_seat_and_whether_taken() {
        reset();
        trial(1 << 3);
        seat(3);
        dodge(true, true);
        dodge_hit(true, true, true, false);
        seat(2);
        dodge(false, false);
        let m = read();
        assert_eq!(m.dodges[1][1][1], Dodges { begun: 1, hit: 1, shell: 0, killed: 1 });
        assert_eq!(m.dodges[0][0][0].begun, 1);
        let mut total = Meter::default();
        total.add_dodges(&m);
        total.add_dodges(&m);
        assert_eq!(total.dodges[1][1][1].killed, 2);
        assert!(m.line().contains("dodges (trial) dropped afloat 1 (1 hit, 0 by the shell, 1 killed)"), "{}", m.line());
        trial(0);
        reset();
    }
    #[test]
    fn a_second_alive_counts_its_kite_step() {
        use crate::status::{Act, KiteStep};
        reset();
        trial(1 << 1);
        seat(1);
        kite(Act::Kite(KiteStep::HoldLoaded));
        kite(Act::WingKite(KiteStep::StepInBusy));
        kite(Act::Idle);
        seat(0);
        kite(Act::Idle);
        let m = read();
        assert_eq!(m.kites[1][7] + m.kites[1][3], 2);
        assert_eq!((m.kites[1][8], m.kites[0][8]), (3, 1));
        let mut total = Meter::default();
        total.add_fronts(&m);
        assert_eq!(total.kites, m.kites);
        assert_eq!(m.kite_kinds[1], [1, 1, 0]);
        assert!(m.line().contains("seconds kiting (trial): 2 of 3 alive (massing 0, closing 0, stepping in 1, stepping out 0, holding 1)"), "{}", m.line());
        trial(0);
        reset();
    }
}
