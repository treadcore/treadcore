//! Fleet tactics (Paul, 16 October: "we want a bunch of scalars and
//! reasons to collaborative bot behavior. research ... fleet tactics in
//! rts"). Three decisions a team's bots made by distance alone, each now
//! a score of weighted considerations with the reasons kept for the
//! status line, the log and team chat:
//!
//!   1. Which hostile tank to shoot (target selection). The RTS rule is
//!      focus fire: a side that puts its shots on one tank at a time
//!      takes tanks out of the fight sooner, and a tank out of the fight
//!      shoots no more (Lanchester's square law, below). So a target
//!      scores for being near, wounded, one shot from dead, afloat (a
//!      boat sinks at one hit), called by a teammate's ping, already
//!      under a teammate's gun, in range now, and threatening a
//!      teammate; and it scores less once more of us are on it than it
//!      takes (overkill: the rest pick the next). Uriarte and Ontañón
//!      score StarCraft targets the same way, by distance, threat and
//!      damage; the GA micro bots by health alone.
//!   2. Whether to close (the engagement). Lanchester's square law: with
//!      aimed fire, a side's strength is its number squared times its
//!      quality, so three tanks beat two by more than three to two, and
//!      a side that arrives one at a time is beaten one at a time. The
//!      odds at a fight are ours squared over theirs squared; below
//!      `engage_odds` with teammates on the way a bot holds off and masses
//!      with them ("don't trickle in"), above `press_odds` it closes.
//!   3. Who answers a call (assignment). A ping drew every bot in reach,
//!      the base's too. Now the first `answer_cap` teammates nearer to
//!      it than this bot are enough, and a crew on a job answers only
//!      inside `crew_reach`.
//!
//! And where to stand (the path's end): of the flank's firing points
//! round a target, the one least exposed to its friends' guns and to
//! hostile pillboxes, as an influence map would pick it, without one.
//!
//! And kiting (Paul, 17 October: "more directed kiting, where we
//! estimate the range of the opponent and avoid getting hit while
//! aggressing closer in a group"). Every tank's direct gun reaches the
//! same 10 tiles and reloads in 4 s, and a tank's reload is in the
//! observation (a window shows it), so the estimate is the target's
//! reach (its direct gun while it has shells), when it can fire again,
//! and which other guns would cover the spot a step in ends on. The bot
//! waits on the ring just past that reach, where its own indirect gun
//! still works (12.5 tiles); it steps in to 9 tiles, fires direct and
//! steps out again only while the target's gun is reloading for longer
//! than the trip takes, no other loaded gun covers the spot, and its
//! own gun is loaded. Pressing (the square law in our favour), it also
//! steps in while a teammate is already inside the target's reach: the
//! target has one gun and many of us. Massing, it never does.
//!
//! The considerations add; each is a weight times a value in 0..1, so a
//! weight reads as "worth this many of the others". `Tactics::default`
//! is the team's; a test or a bot of another temper changes the numbers.

use treadcore_sim::*;

/// The weights. Every number here is a reason a bot can give.
#[derive(Clone, Debug)]
pub struct Tactics {
    // 1. Target selection.
    /// Distance: `w_near / (1 + d / near_scale)`.
    pub w_near: f32,
    pub near_scale: f32,
    /// Wounded: `w_wounded * (1 - armor / TANK_ARMOR)`.
    pub w_wounded: f32,
    /// One direct shot from dead.
    pub w_finish: f32,
    /// Afloat: a boat sinks at one hit.
    pub w_boat: f32,
    /// A teammate pinged it in the last `called_for` seconds.
    pub w_called: f32,
    pub called_for: f32,
    /// Each teammate already in direct range of it, up to `focus_cap`;
    /// each past the cap takes `w_overkill` off.
    pub w_focus: f32,
    pub focus_cap: usize,
    pub w_overkill: f32,
    /// In direct range of me now: a shot this second, not after a drive.
    pub w_in_range: f32,
    /// In direct range of a teammate, or inside `base_r` of the base.
    pub w_threat: f32,
    pub base_r: f32,
    /// The target already chosen: switching costs a drive and a reload.
    pub w_stick: f32,
    // 2. The engagement.
    /// Tanks inside this of the target count for the odds.
    pub odds_r: f32,
    /// A live hostile pillbox in the fight counts as this much of a tank.
    pub pillbox_worth: f32,
    /// A hostile strike drone over the fight counts as this much of a
    /// tank: 35 of 100 armour if it gets through, once.
    pub strike_worth: f32,
    /// Below these odds, with teammates coming, hold off and mass.
    pub engage_odds: f32,
    /// Above these, close to direct range.
    pub press_odds: f32,
    /// Teammates this near the fight but outside `odds_r` are coming.
    pub coming_r: f32,
    /// Hold off at most this many seconds for them.
    pub mass_for: f32,
    // 3. Assignment.
    /// This many teammates nearer a ping than me answer it; I stay.
    pub answer_cap: usize,
    /// A crew on a job answers pings only this near.
    pub crew_reach: f32,
    // Where to stand.
    /// A firing point near a hostile pillbox or a second hostile tank
    /// costs this much per gun on it, against `w_flank` for keeping to
    /// the bot's flank's bearing.
    pub w_exposure: f32,
    pub w_flank: f32,
    // Kiting.
    /// Kite at all; off fights at the stance's fixed range, as before.
    pub kite: bool,
    /// The waiting ring, this far past the target's reach (and never
    /// inside the indirect gun's minimum).
    pub kite_margin: f32,
    /// The ring a step in ends on.
    pub kite_in: f32,
    /// Seconds a step in takes beyond the drive: turning and the shot.
    pub kite_turn: f32,
    /// Seconds of the target's reload to spare past the trip.
    pub kite_spare: f32,
}

impl Default for Tactics {
    fn default() -> Tactics {
        Tactics {
            w_near: 1.0,
            near_scale: 15.0,
            w_wounded: 1.2,
            w_finish: 0.8,
            w_boat: 1.0,
            w_called: 0.8,
            called_for: 15.0,
            w_focus: 0.5,
            focus_cap: 2,
            w_overkill: 0.6,
            w_in_range: 0.7,
            w_threat: 0.6,
            base_r: 16.0,
            w_stick: 0.4,
            odds_r: 14.0,
            pillbox_worth: 0.7,
            strike_worth: 0.35,
            engage_odds: 0.7,
            press_odds: 1.5,
            coming_r: 45.0,
            mass_for: 12.0,
            answer_cap: 3,
            crew_reach: 40.0,
            w_exposure: 1.0,
            w_flank: 0.3,
            kite: true,
            kite_margin: 1.5,
            kite_in: DIRECT_RANGE - 1.0,
            kite_turn: 0.8,
            kite_spare: 0.4,
        }
    }
}

/// The odds at a fight, and what they were made of.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Odds {
    /// Our strength and theirs: tanks weighted by armour and shells,
    /// their pillboxes by `pillbox_worth`, their strike drones by
    /// `strike_worth`.
    pub ours: f32,
    pub theirs: f32,
    /// Teammates on their way (inside `coming_r`, outside `odds_r`).
    pub coming: usize,
}

impl Odds {
    /// Lanchester's square law: ours squared over theirs squared.
    pub fn ratio(&self) -> f32 {
        (self.ours * self.ours) / (self.theirs * self.theirs).max(0.01)
    }
}

/// What the engagement says to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Stance {
    /// Outnumbered with help coming: hold off out of direct range.
    Mass,
    /// Even: stand at the usual range.
    Engage,
    /// Ahead: close in.
    Press,
}

/// Where kiting says to be, and why.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Kite {
    pub to: V2,
    /// Stepping in to fire, not waiting.
    pub stepping_in: bool,
    pub word: &'static str,
}

/// How far a hostile tank's gun reaches now: its direct gun while it has
/// shells. Its indirect gun is a spotted shell in the air that the dodge
/// answers (lib.rs `Dodge`), not a range to stand outside.
pub fn reach(t: &Tank) -> f32 {
    if t.alive && t.shells > 0 {
        DIRECT_RANGE
    } else {
        0.0
    }
}

/// Seconds until a tank can fire again.
pub fn ready_in(t: &Tank) -> f32 {
    t.reload as f32 * DT
}

/// The weights and what a bot remembers with them: its team's calls,
/// its target, and why.
#[derive(Clone, Debug, Default)]
pub struct Tactician {
    pub w: Tactics,
    /// The side's base, for `w_threat` (the Commander keeps it here).
    pub base: Option<V2>,
    /// A teammate's pings: where, and when.
    called: Vec<(V2, Tick)>,
    /// The target chosen last, and when it was chosen.
    pub target: Option<EntityId>,
    pub since: Tick,
    /// The target changed on this pick: say so.
    pub switched: bool,
    /// Why it was chosen, for the status line and team chat.
    pub reason: String,
    pub score: f32,
    /// When massing began for the fight now, so it ends.
    massing_since: Option<Tick>,
    /// Confirmation before the turret changes target (roadmap 12d):
    /// picks in a row and the margin a challenger needs; None changes at
    /// once, as before.
    pub confirm: Option<(u8, f32)>,
    challenger: Option<(EntityId, u8)>,
    /// The engagement smoothed (roadmap 12d): the odds as an advance
    /// scalar, its share of the new reading a think and its band; None
    /// takes each reading as it comes, as before.
    pub smooth: Option<(f32, f32)>,
    advance: crate::intent::Advance,
    advance_at: Option<Tick>,
    held_stance: Option<Stance>,
}

/// A tank's worth in a fight: armour, and none without shells.
fn worth(t: &Tank) -> f32 {
    if !t.alive || t.shells == 0 {
        0.0
    } else {
        (t.armor / TANK_ARMOR).clamp(0.1, 1.0)
    }
}

impl Tactician {
    pub fn with(w: Tactics) -> Tactician {
        Tactician { w, ..Tactician::default() }
    }

    /// Every tick: a teammate's ping is a call on what is under it.
    pub fn hear(&mut self, obs: &Observation) {
        let keep = (self.w.called_for * TICK_HZ as f32) as Tick;
        self.called.retain(|(_, t)| obs.tick < t + keep);
        for e in &obs.events {
            if let Event::Pinged { player, at } = e {
                if *player != obs.player && obs.allied(*player) {
                    self.called.push((*at, obs.tick));
                }
            }
        }
    }

    /// A target's score and the reasons that made it.
    pub fn score(&self, obs: &Observation, me: &Tank, t: &Tank) -> (f32, Vec<String>) {
        let mut why = Some(Vec::new());
        let s = self.rate(obs, me, t, &mut why);
        (s, why.unwrap_or_default())
    }

    /// The score alone: what `pick` asks of every candidate, with no
    /// words built for the ones that lose (roadmap 12b).
    pub fn value(&self, obs: &Observation, me: &Tank, t: &Tank) -> f32 {
        self.rate(obs, me, t, &mut None)
    }

    fn rate(&self, obs: &Observation, me: &Tank, t: &Tank, why: &mut Option<Vec<String>>) -> f32 {
        let mut say = |f: &dyn Fn() -> String| {
            if let Some(w) = why.as_mut() {
                w.push(f());
            }
        };
        let w = &self.w;
        let d = me.pos.dist(t.pos);
        let mut s = w.w_near / (1.0 + d / w.near_scale);
        say(&|| format!("{:.0} out", d));
        let hurt = 1.0 - (t.armor / TANK_ARMOR).clamp(0.0, 1.0);
        if hurt > 0.05 {
            s += w.w_wounded * hurt;
            say(&|| format!("armour {:.0}", t.armor));
        }
        if t.armor <= DIRECT_DAMAGE {
            s += w.w_finish;
            say(&|| "one shot".into());
        }
        if t.boat {
            s += w.w_boat;
            say(&|| "afloat".into());
        }
        if self.called.iter().any(|(at, _)| at.dist(t.pos) <= 8.0) {
            s += w.w_called;
            say(&|| "called".into());
        }
        let on_it = obs.allied_tanks().filter(|a| a.alive && a.owner != obs.player && a.pos.dist(t.pos) <= DIRECT_RANGE).count();
        if on_it > 0 {
            let focus = on_it.min(w.focus_cap) as f32 * w.w_focus;
            let over = on_it.saturating_sub(w.focus_cap) as f32 * w.w_overkill;
            s += focus - over;
            say(&|| if on_it > w.focus_cap { format!("{} on it: enough", on_it) } else { format!("{} on it", on_it) });
        }
        if d <= DIRECT_RANGE {
            s += w.w_in_range;
            say(&|| "in range".into());
        }
        let menaces = obs.allied_tanks().any(|a| a.alive && a.owner != obs.player && a.pos.dist(t.pos) <= DIRECT_RANGE) || self.base.map(|b| b.dist(t.pos) <= w.base_r).unwrap_or(false);
        if menaces {
            s += w.w_threat;
            say(&|| "a threat".into());
        }
        if self.target == Some(t.id) {
            s += w.w_stick;
        }
        s
    }

    /// The hostile tank to shoot, of those `fair` allows; the reason kept.
    pub fn pick<'a>(&mut self, obs: &'a Observation, me: &Tank, fair: impl Fn(&Tank) -> bool) -> Option<&'a Tank> {
        let best = obs
            .hostile_tanks()
            .filter(|t| t.alive && fair(t))
            .map(|t| (t, self.value(obs, me, t)))
            .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.id.cmp(&a.0.id)));
        let Some((t, s)) = best else {
            self.switched = false;
            self.target = None;
            self.challenger = None;
            return None;
        };
        // Smoothing (roadmap 12d): a live, fair target is not dropped for
        // a challenger until the challenger has won `confirm` picks in a
        // row by the margin; the turret holds between near-equal targets.
        let t = match (self.confirm, self.target) {
            (Some((n, h)), Some(cur)) if cur != t.id => match obs.hostile_tanks().find(|x| x.id == cur && x.alive && fair(x)) {
                Some(held) => {
                    let held_s = self.value(obs, me, held);
                    let streak = match self.challenger {
                        Some((id, k)) if id == t.id => k.saturating_add(1),
                        _ => 1,
                    };
                    if s > held_s * (1.0 + h) && streak >= n {
                        self.challenger = None;
                        t
                    } else {
                        self.challenger = if s > held_s * (1.0 + h) { Some((t.id, streak)) } else { None };
                        held
                    }
                }
                None => t,
            },
            _ => t,
        };
        if self.target == Some(t.id) && self.challenger.map(|(id, _)| id == t.id).unwrap_or(false) {
            self.challenger = None;
        }
        let (s, why) = self.score(obs, me, t);
        self.switched = self.target != Some(t.id);
        if self.switched {
            self.since = obs.tick;
        }
        self.target = Some(t.id);
        self.score = s;
        let name = obs.players.iter().find(|p| p.id == t.owner).map(|p| p.name.as_str()).unwrap_or("?");
        self.reason = format!("{} ({:.2}): {}", name, s, why.join(", "));
        Some(t)
    }

    /// The odds at a fight round `at`, me counted as there.
    pub fn odds(&self, obs: &Observation, me: &Tank, at: V2) -> Odds {
        let w = &self.w;
        let mut o = Odds { ours: worth(me), ..Odds::default() };
        for t in obs.tanks.iter().filter(|t| t.alive && t.id != me.id) {
            let d = t.pos.dist(at);
            if obs.allied(t.owner) {
                if d <= w.odds_r {
                    o.ours += worth(t);
                } else if d <= w.coming_r {
                    o.coming += 1;
                }
            } else if d <= w.odds_r {
                o.theirs += worth(t);
            }
        }
        o.theirs += obs.pillboxes.iter().filter(|p| p.alive() && p.owner.map(|x| !obs.allied(x)).unwrap_or(false) && p.pos.dist(at) <= w.odds_r).count() as f32 * w.pillbox_worth;
        // Air: the strike drones over the fight (journal 0036: three took
        // a full-armour tank in 0.2 s; the odds weighed tanks only).
        o.theirs += obs.hostile_drones().filter(|d| d.kind == DroneKind::Strike && d.pos.dist(at) <= w.odds_r * 1.5).count() as f32 * w.strike_worth;
        o
    }

    /// Mass, engage or press, by the odds; massing ends after
    /// `mass_for` seconds whether or not the team came.
    pub fn stance(&mut self, tick: Tick, odds: &Odds) -> Stance {
        let r = odds.ratio();
        if let Some((beta, band)) = self.smooth {
            // The decision to close, rounded off (Paul, 18 October): the
            // odds read as -1 (at or under `engage_odds`) to +1 (at or over
            // `press_odds`), averaged a think at a time; press only above
            // the band, fall back only below it, hold between.
            if self.advance_at.map(|t| tick < t + TICK_HZ as Tick / 4).unwrap_or(false) {
                if let Some(st) = self.held_stance {
                    return st;
                }
            }
            self.advance_at = Some(tick);
            let (lo, hi) = (self.w.engage_odds, self.w.press_odds.max(self.w.engage_odds + 0.01));
            let raw = ((r - lo) / (hi - lo) * 2.0 - 1.0).clamp(-1.0, 1.0);
            let st = match self.advance.update(raw, beta, band) {
                crate::intent::Step::Push => {
                    self.massing_since = None;
                    Stance::Press
                }
                crate::intent::Step::Hold => {
                    self.massing_since = None;
                    Stance::Engage
                }
                crate::intent::Step::Back if odds.coming > 0 => {
                    let since = *self.massing_since.get_or_insert(tick);
                    if (tick.saturating_sub(since) as f32 * DT) < self.w.mass_for {
                        Stance::Mass
                    } else {
                        Stance::Engage
                    }
                }
                crate::intent::Step::Back => Stance::Engage,
            };
            self.held_stance = Some(st);
            return st;
        }
        if r < self.w.engage_odds && odds.coming > 0 {
            let since = *self.massing_since.get_or_insert(tick);
            if (tick.saturating_sub(since) as f32 * DT) < self.w.mass_for {
                return Stance::Mass;
            }
            return Stance::Engage;
        }
        self.massing_since = None;
        if r >= self.w.press_odds {
            Stance::Press
        } else {
            Stance::Engage
        }
    }

    /// Kiting `target`, coming in on `from`, at `stance`: the step in
    /// when the target's gun cannot answer it, else the waiting ring
    /// (module notes). Massing waits further off, at `mass_r`.
    pub fn kite(&self, obs: &Observation, me: &Tank, target: &Tank, from: V2, stance: Stance, mass_r: f32) -> Kite {
        let w = &self.w;
        if stance == Stance::Mass {
            return Kite { to: self.stand(obs, target, from, mass_r).0, stepping_in: false, word: "massing out of its reach" };
        }
        let out = (reach(target) + w.kite_margin).max(INDIRECT_MIN_RANGE + 0.5);
        let d = me.pos.dist(target.pos);
        let into = (me.pos - target.pos).norm();
        let in_at = obs.clamp(target.pos + into.scale(w.kite_in));
        let trip = (d - w.kite_in).max(0.0) / TANK_SPEED + w.kite_turn;
        // Any other gun on the spot by the time we are there: a tank
        // loaded by then, or a live hostile pillbox (it reloads in 2 s).
        let others = obs.hostile_tanks().filter(|t| t.id != target.id && reach(t) > 0.0 && t.pos.dist(in_at) <= reach(t) + 0.5 && ready_in(t) <= trip + 1.0).count()
            + obs.pillboxes.iter().filter(|b| b.alive() && b.owner.map(|x| !obs.allied(x)).unwrap_or(false) && b.pos.dist(in_at) <= PILLBOX_RANGE + 0.5).count();
        let harmless = reach(target) == 0.0;
        let open = ready_in(target) >= trip + w.kite_spare;
        let crowd = obs.allied_tanks().any(|a| a.alive && a.owner != obs.player && a.pos.dist(target.pos) <= reach(target));
        // Our gun loaded by the time we are in: the indirect gun at the
        // ring fires every reload, so a gun loaded now was rarely the
        // case when the target's window opened (the quad: no step in on
        // a window in ten minutes).
        let loaded = me.shells > 0 && ready_in(me) <= (d - w.kite_in).max(0.0) / TANK_SPEED + 0.1;
        let step = loaded && others == 0 && (harmless || open || (stance == Stance::Press && crowd));
        if step {
            let word = if harmless {
                "closing: it has no shells"
            } else if open {
                "stepping in: its gun is reloading"
            } else {
                "stepping in: it is busy with the group"
            };
            return Kite { to: in_at, stepping_in: true, word };
        }
        let word = if !loaded && d < out - 0.5 {
            "stepping out to reload"
        } else if !loaded {
            "holding past its reach, reloading"
        } else if others > 0 {
            "holding: other guns cover the way in"
        } else {
            "holding past its reach, gun loaded"
        };
        Kite { to: self.stand(obs, target, from, out).0, stepping_in: false, word }
    }

    /// Teammates nearer a ping than me: past `answer_cap` I stay.
    pub fn enough_answer(&self, obs: &Observation, me: &Tank, at: V2, from: PlayerId) -> Option<usize> {
        let mine = me.pos.dist(at);
        let nearer = obs.allied_tanks().filter(|t| t.alive && t.owner != obs.player && t.owner != from && t.pos.dist(at) < mine).count();
        (nearer >= self.w.answer_cap).then_some(nearer)
    }

    /// Where to stand against `target` at `range`, coming in on `from`
    /// (a unit bearing from the target): of seven points on the arc
    /// within 60 degrees of it, the one with the fewest other guns on
    /// it, the bearing breaking ties. Returns the point and its guns.
    pub fn stand(&self, obs: &Observation, target: &Tank, from: V2, range: f32) -> (V2, usize) {
        let w = &self.w;
        let mut best: Option<(V2, f32, usize)> = None;
        for k in -3i32..=3 {
            let turn = k as f32 * 0.35;
            let (s, c) = turn.sin_cos();
            let dir = V2::new(from.x * c - from.y * s, from.x * s + from.y * c);
            let p = obs.clamp(target.pos + dir.scale(range));
            let guns = obs.hostile_tanks().filter(|t| t.alive && t.id != target.id && t.pos.dist(p) <= DIRECT_RANGE + 1.0).count()
                + obs.pillboxes.iter().filter(|b| b.alive() && b.owner.map(|x| !obs.allied(x)).unwrap_or(false) && b.pos.dist(p) <= PILLBOX_RANGE + 1.5).count();
            let cost = guns as f32 * w.w_exposure + (k.abs() as f32 / 3.0) * w.w_flank;
            if best.map(|(_, c, _)| cost < c).unwrap_or(true) {
                best = Some((p, cost, guns));
            }
        }
        let (p, _, g) = best.unwrap();
        (p, g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two tanks with a pillbox-free field: a fresh near one, a far
    /// wounded one an ally is already shooting. Focus fire picks the
    /// wounded one; without the focus and wound weights, the near one.
    #[test]
    fn focus_fire_picks_the_wounded_tank_a_teammate_is_on() {
        let names = (0..4).map(|i| (format!("p{}", i), true)).collect();
        let mut sc = Scenario { seed: 7, players: names, ..Scenario::default() };
        sc = sc.with_teams(2);
        let mut w = World::new(sc);
        // Seats 0 and 2 are a team, 1 and 3 the other.
        let c = w.map.center();
        let place = |w: &mut World, p: PlayerId, at: V2, armor: f32| {
            let t = w.tanks.iter_mut().find(|t| t.owner == p).unwrap();
            t.pos = at;
            t.armor = armor;
        };
        place(&mut w, 0, c, 100.0);
        place(&mut w, 2, c + V2::new(14.0, 6.0), 100.0);
        place(&mut w, 1, c + V2::new(6.0, 0.0), 100.0);
        place(&mut w, 3, c + V2::new(14.0, 0.0), 40.0);
        let obs = w.observe(0);
        let me = obs.me.clone().unwrap();
        let mut t = Tactician::default();
        let pick = t.pick(&obs, &me, |_| true).map(|x| x.owner);
        assert_eq!(pick, Some(3), "{}", t.reason);
        assert!(t.reason.contains("one shot") && t.reason.contains("1 on it"), "{}", t.reason);
        let mut plain = Tactician { w: Tactics { w_wounded: 0.0, w_finish: 0.0, w_focus: 0.0, w_threat: 0.0, ..Tactics::default() }, ..Tactician::default() };
        assert_eq!(plain.pick(&obs, &me, |_| true).map(|x| x.owner), Some(1), "{}", plain.reason);
    }

    /// Kiting's forks, one at a time: the target loaded holds us past
    /// its reach; reloading lets us step in; a second loaded gun on the
    /// spot holds us again; our own gun not loaded by the time we would
    /// be in holds us; massing
    /// waits further off still; pressing with a teammate in its face
    /// steps in though it is loaded.
    #[test]
    fn kiting_steps_in_only_when_the_gun_cannot_answer() {
        let names = (0..4).map(|i| (format!("p{}", i), true)).collect();
        let mut w = World::new(Scenario { seed: 7, players: names, swarm_interval: 0, ..Scenario::default() }.with_teams(2));
        let c = w.map.center();
        let place = |w: &mut World, p: PlayerId, at: V2| {
            let t = w.tanks.iter_mut().find(|t| t.owner == p).unwrap();
            t.pos = at;
            t.reload = 0;
        };
        // Seats 0 and 2 are a team, 1 and 3 the other; 3 far away.
        place(&mut w, 0, c + V2::new(-12.0, 0.0));
        place(&mut w, 1, c);
        place(&mut w, 2, c + V2::new(-40.0, 30.0));
        place(&mut w, 3, c + V2::new(40.0, 30.0));
        w.pillboxes.clear();
        // The side's eyes on the target: a recon over it (a tank sees 10
        // tiles, and the waiting ring is past them).
        w.step(&[Order { player: 0, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: c, radius: 2.0 } } }]);
        for _ in 0..3 * TICK_HZ {
            w.step(&[]);
        }
        place(&mut w, 0, c + V2::new(-12.0, 0.0));
        place(&mut w, 1, c);
        let t = Tactician::default();
        let look = |w: &World, st: Stance| {
            let obs = w.observe(0);
            let me = obs.me.clone().unwrap();
            let e = obs.tanks.iter().find(|x| x.owner == 1).unwrap().clone();
            let k = t.kite(&obs, &me, &e, (me.pos - e.pos).norm(), st, DIRECT_RANGE * 1.8);
            (k, k.to.dist(e.pos))
        };
        let (k, d) = look(&w, Stance::Engage);
        assert!(!k.stepping_in && d > DIRECT_RANGE + 1.0, "loaded: {:?} at {:.1}", k, d);
        w.tanks.iter_mut().find(|t| t.owner == 1).unwrap().reload = RELOAD_TICKS;
        let (k, d) = look(&w, Stance::Engage);
        assert!(k.stepping_in && d < DIRECT_RANGE, "reloading: {:?} at {:.1}", k, d);
        place(&mut w, 3, c + V2::new(-9.0, 6.0));
        let (k, _) = look(&w, Stance::Engage);
        assert!(!k.stepping_in && k.word.contains("other guns"), "a second gun: {:?}", k);
        place(&mut w, 3, c + V2::new(40.0, 30.0));
        w.tanks.iter_mut().find(|t| t.owner == 0).unwrap().reload = RELOAD_TICKS;
        let (k, _) = look(&w, Stance::Engage);
        assert!(!k.stepping_in && k.word.contains("reload"), "own gun empty: {:?}", k);
        w.tanks.iter_mut().find(|t| t.owner == 0).unwrap().reload = 0;
        let (k, d) = look(&w, Stance::Mass);
        assert!(!k.stepping_in && d > DIRECT_RANGE * 1.5, "massing: {:?} at {:.1}", k, d);
        w.tanks.iter_mut().find(|t| t.owner == 1).unwrap().reload = 0;
        place(&mut w, 2, c + V2::new(0.0, -7.0));
        let (k, _) = look(&w, Stance::Engage);
        assert!(!k.stepping_in, "engaging, loaded, a teammate in: {:?}", k);
        let (k, _) = look(&w, Stance::Press);
        assert!(k.stepping_in && k.word.contains("group"), "pressing with a teammate in: {:?}", k);
    }

    #[test]
    fn the_square_law() {
        let two_to_one = Odds { ours: 2.0, theirs: 1.0, coming: 0 };
        assert_eq!(two_to_one.ratio(), 4.0);
        let mut t = Tactician::default();
        assert_eq!(t.stance(0, &two_to_one), Stance::Press);
        let alone = Odds { ours: 1.0, theirs: 2.0, coming: 2 };
        assert_eq!(t.stance(0, &alone), Stance::Mass);
        let alone_for_good = Odds { coming: 0, ..alone };
        assert_eq!(t.stance(0, &alone_for_good), Stance::Engage);
        // Smoothed: one reading of strong odds is not yet a press; a few are.
        let mut sm = Tactician { smooth: Some((0.3, 0.35)), ..Tactician::default() };
        assert_eq!(sm.stance(0, &two_to_one), Stance::Engage, "one think is not a press");
        assert_eq!(sm.stance(1, &two_to_one), Stance::Engage, "held between thinks");
        assert_eq!(sm.stance(5, &two_to_one), Stance::Press);
    }
}
