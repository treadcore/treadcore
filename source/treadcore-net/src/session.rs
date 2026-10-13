//! The session: a Host that owns the world, Clients that send orders and
//! receive their view. Host-authoritative, with just enough reliability
//! on top of UDP for the two things that must not be lost (orders, and
//! events) and none for the thing that must not be late (snapshots).
//!
//! The Host is used two ways. With a socket, it is the network host the
//! lobby elected; remote seats join, local seats (the host's own player,
//! and the bots) are driven by the caller. Without a socket it is a local
//! match -- the same code, nothing on the wire -- which is how the client
//! plays alone and how the server runs a bot-only trace.
//!
//! Relay: a peer the host cannot reach directly is reached through the
//! peer the election named. Both ends wrap in `Packet::Relay`; a peer that
//! receives a Relay not addressed to it forwards along its own table.

use crate::transport::Socket;
use std::collections::{BTreeMap, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use treadcore_proto::{snapshot_packets, Packet, PeerId, SnapshotAssembler};
use treadcore_sim::{Command, Event, Observation, Order, PlayerId, Replay, Scenario, Tick, World};

const RESEND: Duration = Duration::from_millis(120);
/// The host hashes the world every this many ticks (fifty seconds),
/// logs it, and carries it in every heartbeat until the next.
pub const HASH_EVERY: Tick = 1000;
const HEARTBEAT: Duration = Duration::from_millis(500);
const TIMEOUT: Duration = Duration::from_secs(6);
/// Snapshots every other tick: 10 Hz.
const SNAPSHOT_EVERY: Tick = 2;
const EVENT_BACKLOG: usize = 64;

/// How to reach a peer: straight, or through another.
#[derive(Clone, Debug, Default)]
pub struct RouteTable {
    pub direct: BTreeMap<PeerId, SocketAddr>,
    pub via: BTreeMap<PeerId, PeerId>,
}

impl RouteTable {
    /// Where a packet for `to` goes on the wire, and whether it needs
    /// wrapping.
    fn resolve(&self, to: PeerId) -> Option<(SocketAddr, bool)> {
        if let Some(via) = self.via.get(&to) {
            if let Some(a) = self.direct.get(via) {
                return Some((*a, true));
            }
        }
        self.direct.get(&to).map(|a| (*a, false))
    }

    fn send(&self, sock: &mut Socket, me: PeerId, to: PeerId, p: &Packet) {
        match self.resolve(to) {
            Some((addr, false)) => sock.send(addr, p),
            Some((addr, true)) => sock.send(addr, &Packet::Relay { from: me, to, hops: 0, inner: p.encode() }),
            None => log::trace!("no route to {:016x}", to),
        }
    }

    /// A Relay arrived. Either it is for me (unwrap it) or it is not
    /// (forward it). Returns the inner packet if it was for me.
    fn relay(&self, sock: &mut Socket, me: PeerId, from: PeerId, to: PeerId, hops: u8, inner: Vec<u8>) -> Option<(PeerId, Packet)> {
        if to == me {
            return Packet::decode(&inner).ok().map(|p| (from, p));
        }
        if hops >= 8 {
            log::warn!("relay loop: dropping packet from {:016x} to {:016x}", from, to);
            return None;
        }
        if let Some((addr, _)) = self.resolve(to) {
            sock.send(addr, &Packet::Relay { from, to, hops: hops + 1, inner });
        }
        None
    }
}

// --------------------------------------------------------------------- host

pub struct Seat {
    pub player: PlayerId,
    pub peer: PeerId,
    pub name: String,
    pub joined: bool,
    next_seq: u32,
    pending: BTreeMap<u32, Command>,
    events_out: VecDeque<(u32, Tick, Vec<Event>)>,
    events_seq: u32,
    last_event_send: Instant,
    last_seen: Instant,
    last_heartbeat: Instant,
    pub rtt_ticks: u32,
}

pub struct Host {
    pub me: PeerId,
    pub world: World,
    pub seats: Vec<Seat>,
    pub routes: RouteTable,
    sock: Option<Socket>,
    /// Orders from remote seats, collected by `poll`, consumed by `tick`.
    inbox: Vec<Order>,
    pub ticks_run: u64,
    /// When Some, every tick's orders are recorded into it (`--record`).
    pub replay: Option<Replay>,
    /// The world hash as of the last multiple of `HASH_EVERY`: what the
    /// heartbeat carries and the log shows.
    pub hash_mark: (Tick, u64),
}

impl Host {
    /// `remote` is (peer, name) per remote seat, in the order the lobby
    /// listed them; they become players 0..n and the scenario's remaining
    /// seats stay as the scenario says (bots, or the local human).
    pub fn new(me: PeerId, sock: Option<Socket>, scenario: Scenario, remote: Vec<(PeerId, String)>, routes: RouteTable) -> Host {
        let world = World::new(scenario);
        let now = Instant::now();
        let seats = remote
            .into_iter()
            .enumerate()
            .map(|(i, (peer, name))| Seat {
                player: i as PlayerId,
                peer,
                name,
                joined: false,
                next_seq: 1,
                pending: BTreeMap::new(),
                events_out: VecDeque::new(),
                events_seq: 0,
                last_event_send: now,
                last_seen: now,
                last_heartbeat: now,
                rtt_ticks: 0,
            })
            .collect();
        let hash0 = world.hash();
        Host { me, world, seats, routes, sock, inbox: Vec::new(), ticks_run: 0, replay: None, hash_mark: (0, hash0) }
    }

    /// Start recording a replay of this match from now (normally before
    /// the first tick).
    pub fn record(&mut self) {
        self.replay = Some(Replay::new(self.world.scenario.clone()));
    }

    pub fn is_networked(&self) -> bool {
        self.sock.is_some()
    }

    pub fn socket(&self) -> Option<&Socket> {
        self.sock.as_ref()
    }

    /// Read the socket: joins, orders, acks, heartbeats, relays.
    pub fn poll(&mut self) {
        let Some(mut sock) = self.sock.take() else { return };
        let now = Instant::now();
        let mut incoming: Vec<(PeerId, Packet)> = Vec::new();
        for (from, p) in sock.recv() {
            match p {
                Packet::Relay { from: f, to, hops, inner } => {
                    if let Some(x) = self.routes.relay(&mut sock, self.me, f, to, hops, inner) {
                        incoming.push(x);
                    }
                }
                p => {
                    if let Some(peer) = p.sender() {
                        // Learn or refresh the address we hear a direct
                        // peer from; NATs rewrite ports.
                        if !self.routes.via.contains_key(&peer) {
                            self.routes.direct.insert(peer, from);
                        }
                        incoming.push((peer, p));
                    }
                }
            }
        }
        for (peer, p) in incoming {
            let Some(seat) = self.seats.iter_mut().find(|s| s.peer == peer) else {
                if let Packet::Join { .. } = p {
                    self.routes.send(&mut sock, self.me, peer, &Packet::Refuse { why: "no seat for this peer".into() });
                }
                continue;
            };
            seat.last_seen = now;
            match p {
                Packet::Join { name, .. } => {
                    seat.name = name;
                    seat.joined = true;
                    let w = Packet::Welcome { player: seat.player, scenario: self.world.scenario.clone(), tick: self.world.tick, map_cells: self.world.map.cells.clone() };
                    self.routes.send(&mut sock, self.me, peer, &w);
                    log::info!("seat {} '{}' joined from {:016x}", seat.player, seat.name, peer);
                }
                Packet::Command { seq, cmd, .. } => {
                    self.routes.send(&mut sock, self.me, peer, &Packet::CommandAck { seq });
                    if seq >= seat.next_seq {
                        seat.pending.entry(seq).or_insert(cmd);
                    }
                    while let Some(c) = seat.pending.remove(&seat.next_seq) {
                        seat.next_seq += 1;
                        self.inbox.push(Order { player: seat.player, cmd: c });
                    }
                }
                Packet::EventsAck { seq } => {
                    while seat.events_out.front().map(|(s, _, _)| *s <= seq).unwrap_or(false) {
                        seat.events_out.pop_front();
                    }
                }
                Packet::Heartbeat { tick, .. } => {
                    seat.rtt_ticks = self.world.tick.saturating_sub(tick) as u32;
                }
                Packet::Leave { .. } => {
                    log::info!("seat {} left", seat.player);
                    seat.joined = false;
                }
                _ => {}
            }
        }
        // Keepalives, event resends, timeouts.
        for seat in self.seats.iter_mut().filter(|s| s.joined) {
            if now.duration_since(seat.last_heartbeat) >= HEARTBEAT {
                seat.last_heartbeat = now;
                let (hash_tick, hash) = self.hash_mark;
                self.routes.send(&mut sock, self.me, seat.peer, &Packet::Heartbeat { peer: self.me, tick: self.world.tick, hash_tick, hash });
            }
            if now.duration_since(seat.last_event_send) >= RESEND {
                seat.last_event_send = now;
                if let Some((seq, tick, events)) = seat.events_out.front() {
                    self.routes.send(&mut sock, self.me, seat.peer, &Packet::Events { seq: *seq, tick: *tick, events: events.clone() });
                }
            }
            if now.duration_since(seat.last_seen) >= TIMEOUT {
                log::warn!("seat {} timed out; its tank drives on under no orders", seat.player);
                seat.joined = false;
            }
        }
        self.sock = Some(sock);
    }

    /// One tick. `local` are orders from seats this process drives (its
    /// own human, its bots). Remote orders come from `poll`. Everything
    /// is sorted by player so the step is deterministic regardless of
    /// arrival order.
    pub fn tick(&mut self, local: Vec<Order>) -> &[Event] {
        let mut orders = std::mem::take(&mut self.inbox);
        orders.extend(local);
        orders.sort_by_key(|o| o.player);
        self.world.step(&orders);
        self.ticks_run += 1;
        if let Some(r) = self.replay.as_mut() {
            r.record(&self.world, &orders);
        }
        if self.world.tick % HASH_EVERY == 0 || self.world.over {
            let h = self.world.hash();
            self.hash_mark = (self.world.tick, h);
            log::info!("world hash at tick {}: {:016x}", self.world.tick, h);
        }
        if let Some(mut sock) = self.sock.take() {
            let tick = self.world.tick;
            for seat in self.seats.iter_mut().filter(|s| s.joined) {
                if tick % SNAPSHOT_EVERY == 0 {
                    let obs = self.world.observe(seat.player);
                    for p in snapshot_packets(&obs) {
                        self.routes.send(&mut sock, self.me, seat.peer, &p);
                    }
                }
                let mine: Vec<Event> = self.world.events.iter().filter(|e| event_for(e, seat.player)).cloned().collect();
                if !mine.is_empty() {
                    seat.events_seq += 1;
                    seat.events_out.push_back((seat.events_seq, tick, mine));
                    while seat.events_out.len() > EVENT_BACKLOG {
                        seat.events_out.pop_front();
                    }
                    // Send the newest at once; resends cover loss.
                    if let Some((seq, t, ev)) = seat.events_out.back() {
                        self.routes.send(&mut sock, self.me, seat.peer, &Packet::Events { seq: *seq, tick: *t, events: ev.clone() });
                    }
                }
            }
            self.sock = Some(sock);
        }
        &self.world.events
    }

    pub fn shutdown(&mut self) {
        if let Some(mut sock) = self.sock.take() {
            for seat in self.seats.iter().filter(|s| s.joined) {
                self.routes.send(&mut sock, self.me, seat.peer, &Packet::Leave { peer: self.me });
            }
            self.sock = Some(sock);
        }
    }
}

fn event_for(e: &Event, p: PlayerId) -> bool {
    match e {
        Event::Rejected { player, .. } => *player == p,
        Event::ShellLaunched { owner, .. } | Event::DroneLaunched { owner, .. } | Event::DroneRecovered { owner, .. } | Event::LinkLost { owner, .. } => *owner == p,
        _ => true,
    }
}

// ------------------------------------------------------------------- client

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientState {
    Joining,
    Playing,
    Refused,
    Lost,
}

pub struct Client {
    pub me: PeerId,
    pub host: PeerId,
    pub routes: RouteTable,
    pub state: ClientState,
    pub player: Option<PlayerId>,
    pub scenario: Option<Scenario>,
    pub map_cells: Option<Vec<u8>>,
    pub name: String,
    sock: Socket,
    seq: u32,
    unacked: BTreeMap<u32, (Command, Instant)>,
    asm: SnapshotAssembler,
    latest: Option<Observation>,
    events_seen: u32,
    events: Vec<Event>,
    last_join: Instant,
    last_rx: Instant,
    last_heartbeat: Instant,
    /// Host's tick as of its last heartbeat, and ours when it arrived.
    pub host_tick: Tick,
    /// The host's world hash and the tick it was taken at, from the last
    /// heartbeat. Logged when it changes; compared once the client runs
    /// its own world (roadmap step 4).
    pub host_hash: (Tick, u64),
    pub refuse_reason: String,
}

impl Client {
    pub fn new(me: PeerId, sock: Socket, host: PeerId, routes: RouteTable, name: &str) -> Client {
        let now = Instant::now();
        Client {
            me,
            host,
            routes,
            state: ClientState::Joining,
            player: None,
            scenario: None,
            map_cells: None,
            name: name.into(),
            sock,
            seq: 0,
            unacked: BTreeMap::new(),
            asm: SnapshotAssembler::default(),
            latest: None,
            events_seen: 0,
            events: Vec::new(),
            last_join: now - Duration::from_secs(5),
            last_rx: now,
            last_heartbeat: now,
            host_tick: 0,
            host_hash: (0, 0),
            refuse_reason: String::new(),
        }
    }

    pub fn send(&mut self, cmd: Command) {
        self.seq += 1;
        let p = Packet::Command { peer: self.me, seq: self.seq, cmd: cmd.clone() };
        self.routes.send(&mut self.sock, self.me, self.host, &p);
        self.unacked.insert(self.seq, (cmd, Instant::now()));
    }

    pub fn take_observation(&mut self) -> Option<Observation> {
        self.latest.take()
    }

    pub fn drain_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn unacked(&self) -> usize {
        self.unacked.len()
    }

    pub fn socket(&self) -> &Socket {
        &self.sock
    }

    pub fn poll(&mut self) {
        let now = Instant::now();
        let mut incoming: Vec<Packet> = Vec::new();
        for (_from, p) in self.sock.recv() {
            match p {
                Packet::Relay { from, to, hops, inner } => {
                    if let Some((_, p)) = self.routes.relay(&mut self.sock, self.me, from, to, hops, inner) {
                        incoming.push(p);
                    }
                }
                p => incoming.push(p),
            }
        }
        for p in incoming {
            self.last_rx = now;
            match p {
                Packet::Welcome { player, scenario, tick, map_cells } => {
                    if self.state == ClientState::Joining {
                        log::info!("welcomed as player {} at tick {}", player, tick);
                    }
                    self.player = Some(player);
                    self.scenario = Some(scenario);
                    self.map_cells = Some(map_cells);
                    self.state = ClientState::Playing;
                }
                Packet::Refuse { why } => {
                    log::error!("refused: {}", why);
                    self.refuse_reason = why;
                    self.state = ClientState::Refused;
                }
                Packet::CommandAck { seq } => {
                    self.unacked.remove(&seq);
                }
                Packet::Snapshot { tick, part, parts, bytes } => {
                    if let Some(obs) = self.asm.push(tick, part, parts, bytes) {
                        self.latest = Some(obs);
                    }
                }
                Packet::Events { seq, events, .. } => {
                    self.routes.send(&mut self.sock, self.me, self.host, &Packet::EventsAck { seq });
                    if seq > self.events_seen {
                        self.events_seen = seq;
                        self.events.extend(events);
                    }
                }
                Packet::Heartbeat { tick, hash_tick, hash, .. } => {
                    self.host_tick = tick;
                    if hash_tick > 0 && (hash_tick, hash) != self.host_hash {
                        self.host_hash = (hash_tick, hash);
                        log::info!("host world hash at tick {}: {:016x}", hash_tick, hash);
                    }
                }
                Packet::Leave { .. } => {
                    log::warn!("host left");
                    self.state = ClientState::Lost;
                }
                _ => {}
            }
        }
        match self.state {
            ClientState::Joining => {
                if now.duration_since(self.last_join) >= Duration::from_millis(500) {
                    self.last_join = now;
                    self.routes.send(&mut self.sock, self.me, self.host, &Packet::Join { peer: self.me, name: self.name.clone() });
                }
                if now.duration_since(self.last_rx) >= TIMEOUT * 2 {
                    log::error!("host never answered");
                    self.state = ClientState::Lost;
                }
            }
            ClientState::Playing => {
                for (seq, (cmd, sent)) in self.unacked.iter_mut() {
                    if now.duration_since(*sent) >= RESEND {
                        *sent = now;
                        self.routes.send(&mut self.sock, self.me, self.host, &Packet::Command { peer: self.me, seq: *seq, cmd: cmd.clone() });
                    }
                }
                if now.duration_since(self.last_heartbeat) >= HEARTBEAT {
                    self.last_heartbeat = now;
                    self.routes.send(&mut self.sock, self.me, self.host, &Packet::Heartbeat { peer: self.me, tick: self.host_tick, hash_tick: 0, hash: 0 });

                }
                if now.duration_since(self.last_rx) >= TIMEOUT {
                    log::error!("host silent for {:?}", TIMEOUT);
                    self.state = ClientState::Lost;
                }
            }
            _ => {}
        }
    }

    pub fn leave(&mut self) {
        self.routes.send(&mut self.sock, self.me, self.host, &Packet::Leave { peer: self.me });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use treadcore_sim::{DroneKind, DroneTask};

    /// Host and client on loopback, in one thread, no lobby: the session
    /// layer on its own. Joins, sends an order, sees it in a snapshot.
    #[test]
    fn loopback_session() {
        let hs = Socket::bind(0).unwrap();
        let cs = Socket::bind(0).unwrap();
        let (hid, cid) = (0x0001, 0x0002);
        let mut hr = RouteTable::default();
        hr.direct.insert(cid, cs.local_with_loopback());
        let mut cr = RouteTable::default();
        cr.direct.insert(hid, hs.local_with_loopback());
        let scenario = Scenario { players: vec![("remote".into(), false), ("bot".into(), true)], swarm_interval: 0, ..Scenario::default() };
        let mut host = Host::new(hid, Some(hs), scenario, vec![(cid, "remote".into())], hr);
        let mut client = Client::new(cid, cs, hid, cr, "remote");
        let mut sent = false;
        let mut seen_drone = false;
        for _ in 0..400 {
            client.poll();
            host.poll();
            host.tick(vec![]);
            if client.state == ClientState::Playing && !sent {
                client.send(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: treadcore_sim::V2::new(64.0, 64.0), radius: 3.0 } });
                sent = true;
            }
            if let Some(obs) = client.take_observation() {
                if obs.my_drones().count() > 0 {
                    seen_drone = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(client.state, ClientState::Playing);
        assert!(seen_drone, "order never reflected in a snapshot");
        assert_eq!(client.unacked(), 0);
        assert!(client.drain_events().iter().any(|e| matches!(e, Event::DroneLaunched { .. })));
    }
}
