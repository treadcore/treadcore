//! Client-side prediction of the player's own tank (roadmap step 4).
//!
//! A client is told the world at a tenth of a second, half a round trip
//! late. Its own orders take a whole round trip to show. The fix is the
//! usual one: the client keeps a copy of the world, applies its own
//! hull orders (`Move`, `Stop`, and the fire orders, which turn the
//! hull) to it the moment they are sent, and drives its own tank
//! forward at the tick rate by the same code the host runs
//! (`World::predict`). Every snapshot is the host's word: the copy's
//! tank is replaced by the host's, the orders the host had not yet
//! applied (seq after the snapshot's `acked`) are applied again, and
//! the ticks the client was ahead are driven again. The predicted tank
//! never leaves the client; the host is authoritative throughout.
//!
//! The copy is partial on purpose. A client is shown what it may see
//! (fog), so it cannot run the whole world and never tries: other tanks,
//! drones, shells, the swarm are drawn from the snapshot as before. What
//! its own tank's motion depends on it has: the map (from `Welcome`,
//! kept up from terrain events), every pillbox (territory is public),
//! the mines it knows, and the teams (for whose guns to drive round).
//!
//! The check: when no order of ours is in flight, the tank the host
//! reports at tick T must be the tank we predicted at tick T, bit for
//! bit. Any difference is logged as a mismatch with the distance. On a
//! straight road the log is empty; a mine we did not know about, or a
//! rule the copy does not run, shows up here first.

use std::collections::VecDeque;
use treadcore_sim::{Command, Event, Map, Observation, PlayerId, Scenario, ShellKind, Tank, Terrain, Tick, World, V2};

/// Ticks of prediction kept for the comparison, and the most a client
/// may run ahead of the last snapshot before it waits (two seconds).
const HISTORY: usize = 64;
const MAX_AHEAD: u32 = 40;

pub struct Predictor {
    pub world: World,
    pub me: PlayerId,
    /// Orders sent and not yet seen applied in a snapshot: (seq, cmd).
    in_flight: Vec<(u32, Command)>,
    /// What we predicted, per tick, for the check.
    history: VecDeque<(Tick, (u32, u32, u32, u32, Option<(u32, u32)>, usize), V2)>,
    /// Ticks predicted past the last snapshot.
    pub ahead: u32,
    pub checks: u64,
    pub mismatches: u64,
    /// The last mismatch: tick, and how far off, in tiles.
    pub last_mismatch: Option<(Tick, f32)>,
    /// Snapshots reconciled; and how many had us behind the host (the
    /// client's clock slower than the host's, no prediction to check).
    pub snapshots: u64,
    pub behind: u64,
    /// The tick of the last snapshot that corrected the tank: the HUD
    /// shows it for a moment.
    pub corrected_at: Option<Tick>,
    /// Whether a snapshot has been adopted yet. Until one has, the copy
    /// is the scenario's start, which is not the tank's state at the
    /// Welcome's tick when a seat comes back mid-match (rejoin): the
    /// first snapshot is the baseline, not a check.
    baselined: bool,
    /// The host's tank as of the last snapshot, and that tick: where a
    /// re-drive starts from when the map changes under ticks already
    /// predicted.
    anchor: Option<(Tick, Tank)>,
}

impl Predictor {
    /// From a `Welcome`: the scenario builds the world, the map as the
    /// host has it now replaces the one the scenario would generate.
    pub fn new(me: PlayerId, scenario: Scenario, map: Map, tick: Tick) -> Predictor {
        let mut world = World::new(scenario);
        world.map = map;
        world.tick = tick;
        Predictor { world, me, in_flight: Vec::new(), history: VecDeque::new(), ahead: 0, checks: 0, mismatches: 0, last_mismatch: None, snapshots: 0, behind: 0, corrected_at: None, baselined: false, anchor: None }
    }

    /// Our own tank as predicted, if the copy has one.
    pub fn tank(&self) -> Option<&Tank> {
        self.world.tank_of(self.me)
    }

    /// An order of ours just sent with `seq`. Applied to the copy now
    /// if it moves the tank; remembered until a snapshot shows it done.
    pub fn order(&mut self, seq: u32, cmd: &Command) {
        if Command::predicted(cmd) {
            // Applied at the current tick without advancing it: the
            // tick's drive is `tick()`'s job.
            self.world.tick = self.world.tick.wrapping_sub(1);
            self.world.predict(self.me, std::slice::from_ref(cmd));
            self.in_flight.push((seq, cmd.clone()));
        }
    }

    /// One tick of our tank, at the tick rate, from whoever owns the
    /// loop. Waits when too far ahead of the last snapshot.
    pub fn tick(&mut self) {
        if self.ahead >= MAX_AHEAD {
            return;
        }
        self.world.predict(self.me, &[]);
        self.ahead += 1;
        self.remember();
    }

    fn remember(&mut self) {
        if let Some(t) = self.tank() {
            self.history.push_back((self.world.tick, t.tick_state(), t.pos));
            while self.history.len() > HISTORY {
                self.history.pop_front();
            }
        }
    }

    /// Terrain events keep a client's map as the host's: craters from
    /// indirect fire, and walls, roads, felled forest and mine craters
    /// as `TerrainChanged`. Public events, so every copy sees them.
    pub fn map_keeps_up(map: &mut Map, events: &[Event]) {
        for e in events {
            match e {
                Event::ShellImpact { at, kind: ShellKind::Indirect, .. } => map.crater(*at),
                Event::TerrainChanged { at, to } => {
                    map.set(*at, Terrain::from_u8(*to));
                }
                _ => {}
            }
        }
    }

    /// Events as of `tick`. A terrain change (a crater, a wall) that
    /// lands under ticks already predicted is driven again: the host
    /// changed its map at `tick`, after that tick's drive, so the ticks
    /// up to it are driven over the old map and the rest over the new
    /// one, from the last snapshot's tank. Without this, a shell that
    /// craters the road two ticks ahead of the client is a mismatch
    /// of a fifth of a tile, found in the 0012 lobby runs.
    pub fn events(&mut self, tick: Tick, events: &[Event]) {
        let changes_map = events.iter().any(|e| matches!(e, Event::ShellImpact { kind: ShellKind::Indirect, .. } | Event::TerrainChanged { .. }));
        let Some((atick, atank)) = self.anchor.clone().filter(|_| changes_map) else {
            Predictor::map_keeps_up(&mut self.world.map, events);
            return;
        };
        if self.world.tick <= atick || tick < atick {
            Predictor::map_keeps_up(&mut self.world.map, events);
            return;
        }
        let total = self.ahead;
        self.world.tick = atick;
        if let Some(t) = self.world.tanks.iter_mut().find(|t| t.id == atank.id) {
            *t = atank;
        }
        self.history.retain(|(t, _, _)| *t <= atick);
        let pending: Vec<Command> = self.in_flight.iter().map(|(_, c)| c.clone()).collect();
        if !pending.is_empty() {
            self.world.tick = self.world.tick.wrapping_sub(1);
            self.world.predict(self.me, &pending);
        }
        self.ahead = 0;
        let mut applied = false;
        for _ in 0..total {
            if !applied && self.world.tick >= tick {
                Predictor::map_keeps_up(&mut self.world.map, events);
                applied = true;
            }
            self.world.predict(self.me, &[]);
            self.ahead += 1;
            self.remember();
        }
        if !applied {
            Predictor::map_keeps_up(&mut self.world.map, events);
        }
    }

    /// A snapshot: the host's tank at `obs.tick`, having applied our
    /// orders through `acked`. Check, adopt, re-apply, re-drive.
    pub fn reconcile(&mut self, obs: &Observation, acked: u32) {
        self.snapshots += 1;
        let quiet = self.in_flight.is_empty();
        self.in_flight.retain(|(seq, _)| *seq > acked);
        let predicted_tick = self.world.tick;
        // The check: only when nothing of ours was in flight across the
        // span, so a difference is the world's doing, not the lag's.
        if let Some(host) = obs.me.as_ref() {
            if let Some((_, state, pos)) = self.history.iter().find(|(t, _, _)| *t == obs.tick) {
                if quiet && host.alive && self.baselined {
                    self.checks += 1;
                    if *state != host.tick_state() {
                        self.mismatches += 1;
                        let off = pos.dist(host.pos);
                        self.last_mismatch = Some((obs.tick, off));
                        log::warn!("prediction mismatch at tick {}: predicted {:?} host {:?}, {:.3} tiles off", obs.tick, pos, host.pos, off);
                    }
                }
            } else if predicted_tick < obs.tick {
                self.behind += 1;
            }
        }
        // Adopt the host's word for everything the copy drives on.
        let w = &mut self.world;
        w.tick = obs.tick;
        w.tanks = obs.tanks.clone();
        if let Some(me) = obs.me.as_ref() {
            if !w.tanks.iter().any(|t| t.id == me.id) {
                w.tanks.push(me.clone());
            }
        }
        w.pillboxes = obs.pillboxes.clone();
        w.mines = obs.mines.clone();
        w.depots = obs.depots.clone();
        for p in &obs.players {
            if let Some(x) = w.players.iter_mut().find(|x| x.id == p.id) {
                x.team = p.team;
                x.alive = p.alive;
            }
        }
        w.over = obs.over;
        self.baselined = true;
        self.anchor = obs.me.as_ref().map(|t| (obs.tick, t.clone()));
        self.history.retain(|(t, _, _)| *t <= obs.tick);
        // Re-apply what the host has not yet applied, then drive the
        // ticks we were ahead again, from the host's state.
        let pending: Vec<Command> = self.in_flight.iter().map(|(_, c)| c.clone()).collect();
        if !pending.is_empty() {
            self.world.tick = self.world.tick.wrapping_sub(1);
            self.world.predict(self.me, &pending);
        }
        let ahead = predicted_tick.saturating_sub(obs.tick).min(MAX_AHEAD as Tick) as u32;
        self.ahead = 0;
        for _ in 0..ahead {
            self.world.predict(self.me, &[]);
            self.ahead += 1;
            self.remember();
        }
        if self.last_mismatch.map(|(t, _)| t == obs.tick).unwrap_or(false) {
            self.corrected_at = Some(obs.tick);
        }
    }

    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// One line for a log at the end of a session.
    pub fn summary(&self) -> String {
        format!("prediction: {} snapshots, {} checks, {} mismatches{}, {} behind", self.snapshots, self.checks, self.mismatches, self.last_mismatch.map(|(t, d)| format!(" (last at tick {}, {:.3} tiles)", t, d)).unwrap_or_default(), self.behind)
    }
}
