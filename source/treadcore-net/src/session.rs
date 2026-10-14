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
//!
//! Rejoin (roadmap step 7, SPEC GAM-12): a seat belongs to a peer id,
//! and a peer id can outlive a process (`introducer::identity_from_file`).
//! A client that drops and comes back sends a Join with a new `session`
//! number; the host sees the same peer, a new session, and gives the
//! seat back with its sequence numbers started over. While it hosts, a
//! host keeps registering with the introducer so the returning peer can
//! find it, and pings every address the introducer gives for a seated
//! peer that is not joined, which opens its own router for the Join.

use crate::introducer::{Registration, REGISTER_HOSTING};
use crate::predict::Predictor;
use crate::transport::Socket;
use std::collections::{BTreeMap, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use treadcore_proto::{chunk_packets, snapshot_packets, ChunkAssembler, Packet, PeerId, SnapshotAssembler};
use treadcore_sim::{Command, Event, Map, Observation, Order, PlayerId, Replay, Scenario, Tank, Tick, World};


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
    /// The client process's session number from its Join; a Join with
    /// another is a restart, and the seat's sequences start over.
    pub session: u64,
    /// Times this seat was given back to a returning process.
    pub rejoins: u32,
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
    /// Numbers the chunked Welcomes so a client tells one from the next.
    welcome_id: u32,
    /// Kept up while hosting, so a peer that dropped can find us.
    pub registration: Option<Registration>,
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
                session: 0,
                rejoins: 0,
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
        Host { me, world, seats, routes, sock, inbox: Vec::new(), ticks_run: 0, replay: None, hash_mark: (0, hash0), welcome_id: 0, registration: None }
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
        if let Some(r) = self.registration.as_mut() {
            r.poll(&mut sock, REGISTER_HOSTING, true);
        }
        for (from, p) in sock.recv() {
            match p {
                Packet::Relay { from: f, to, hops, inner } => {
                    if let Some(x) = self.routes.relay(&mut sock, self.me, f, to, hops, inner) {
                        incoming.push(x);
                    }
                }
                Packet::Introduce { peers, .. } => {
                    // The introducer names a peer of ours that is looking
                    // for us: a seat that dropped. Ping every address it
                    // might be at, so our router lets its Join in.
                    let from_introducer = self.registration.as_ref().map(|r| r.is_from(from)).unwrap_or(false);
                    if !from_introducer {
                        continue;
                    }
                    for i in peers {
                        let seated = self.seats.iter().any(|s| s.peer == i.peer && !s.joined);
                        if !seated || i.hosting {
                            continue;
                        }
                        for a in [i.public.parse::<SocketAddr>(), i.local.parse::<SocketAddr>()].into_iter().flatten() {
                            sock.send(a, &Packet::Ping { peer: self.me, seq: 0, sent_us: 0 });
                        }
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
                Packet::Join { name, session, .. } => {
                    seat.name = name;
                    if session != seat.session {
                        // A new process behind an old peer id: a first
                        // join, or a return. Its sequences start at one.
                        if seat.session != 0 {
                            seat.rejoins += 1;
                            log::info!("seat {} '{}' is back ({:016x}, session {:016x}): its seat and sequences given back. {}", seat.player, seat.name, peer, session, treadcore_sim::chair::back_up(seat.rejoins - 1));
                        }
                        seat.session = session;
                        seat.next_seq = 1;
                        seat.pending.clear();
                        seat.events_out.clear();
                        seat.events_seq = 0;
                    }
                    seat.joined = true;
                    let w = Packet::Welcome { player: seat.player, scenario: self.world.scenario.clone(), tick: self.world.tick, map: self.world.map.clone() };
                    // The map can be 65 KiB: in pieces, a fresh id each time.
                    self.welcome_id = self.welcome_id.wrapping_add(1);
                    for piece in chunk_packets(&w, self.welcome_id) {
                        self.routes.send(&mut sock, self.me, peer, &piece);
                    }
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
                Packet::Ping { seq, sent_us, .. } => {
                    // A seated peer looking for its host through the
                    // introducer: the lobby pings until the host answers.
                    self.routes.send(&mut sock, self.me, peer, &Packet::Pong { peer: self.me, seq, sent_us });
                }
                _ => {}
            }
        }
        // Keepalives, event resends, timeouts.
        for seat in self.seats.iter_mut().filter(|s| s.joined) {
            if now.duration_since(seat.last_heartbeat) >= HEARTBEAT {
                seat.last_heartbeat = now;
                let (hash_tick, hash) = self.hash_mark;
                self.routes.send(&mut sock, self.me, seat.peer, &Packet::Heartbeat { peer: self.me, tick: self.world.tick, hash_tick, hash, rtt_ticks: seat.rtt_ticks });
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
                    // Every order received before this tick was applied
                    // in it: the seat's next seq, less one, is done.
                    for p in snapshot_packets(&obs, seat.next_seq - 1) {
                        self.routes.send(&mut sock, self.me, seat.peer, &p);
                    }
                }
                let mine: Vec<Event> = self.world.events.iter().filter(|e| self.world.event_visible(e, seat.player)).cloned().collect();
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

    /// The socket back, after `shutdown`: a server that hosts match
    /// after match (roadmap step 8) keeps one port for all of them.
    pub fn into_socket(self) -> Option<Socket> {
        self.sock
    }
}

// (what a seat may know about is the world's rule: World::event_visible)

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
    /// This process's number, in every Join: the host tells a restart
    /// from a repeated Join by it.
    pub session: u64,
    pub player: Option<PlayerId>,
    pub scenario: Option<Scenario>,
    pub map: Option<Map>,
    pub name: String,
    sock: Socket,
    seq: u32,
    unacked: BTreeMap<u32, (Command, Instant)>,
    asm: SnapshotAssembler,
    chunks: ChunkAssembler,
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
    /// The host's estimate of our round trip, in ticks, from its last
    /// heartbeat. What the HUD shows as rtt.
    pub rtt_ticks: u32,
    pub refuse_reason: String,
    /// Our own tank, run ahead of the snapshots (roadmap step 4). Built
    /// from the Welcome; `None` until then.
    pub predictor: Option<Predictor>,
}

impl Client {
    pub fn new(me: PeerId, sock: Socket, host: PeerId, routes: RouteTable, name: &str) -> Client {
        let now = Instant::now();
        Client {
            me,
            host,
            routes,
            state: ClientState::Joining,
            session: crate::new_peer_id(),
            player: None,
            scenario: None,
            map: None,
            name: name.into(),
            sock,
            seq: 0,
            unacked: BTreeMap::new(),
            asm: SnapshotAssembler::default(),
            chunks: ChunkAssembler::default(),
            latest: None,
            events_seen: 0,
            events: Vec::new(),
            last_join: now - Duration::from_secs(5),
            last_rx: now,
            last_heartbeat: now,
            host_tick: 0,
            host_hash: (0, 0),
            rtt_ticks: 0,
            refuse_reason: String::new(),
            predictor: None,
        }
    }

    pub fn send(&mut self, cmd: Command) {
        self.seq += 1;
        let p = Packet::Command { peer: self.me, seq: self.seq, cmd: cmd.clone() };
        self.routes.send(&mut self.sock, self.me, self.host, &p);
        if let Some(pr) = self.predictor.as_mut() {
            pr.order(self.seq, &cmd);
        }
        self.unacked.insert(self.seq, (cmd, Instant::now()));
    }

    /// One tick of prediction, at the tick rate, from whoever owns the
    /// loop (the window's fixed update; the headless seat's timer).
    pub fn predict_tick(&mut self) {
        if self.state == ClientState::Playing {
            if let Some(pr) = self.predictor.as_mut() {
                pr.tick();
            }
        }
    }

    /// Our own tank as predicted: where it is now, not where the last
    /// snapshot had it.
    pub fn predicted_tank(&self) -> Option<&Tank> {
        self.predictor.as_ref().and_then(|p| p.tank())
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
        // Pieces of a big packet become the packet.
        let mut whole: Vec<Packet> = Vec::new();
        for p in incoming {
            match p {
                Packet::Chunk { id, part, parts, bytes } => {
                    if let Some(w) = self.chunks.push(id, part, parts, bytes) {
                        whole.push(w);
                    }
                }
                p => whole.push(p),
            }
        }
        for p in whole {
            self.last_rx = now;
            match p {
                Packet::Welcome { player, scenario, tick, map } => {

                    if self.state == ClientState::Joining {
                        log::info!("welcomed as player {} at tick {}", player, tick);
                    }
                    self.player = Some(player);
                    if self.predictor.is_none() {
                        self.predictor = Some(Predictor::new(player, scenario.clone(), map.clone(), tick));
                    }
                    self.scenario = Some(scenario);
                    self.map = Some(map);
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
                Packet::Snapshot { tick, part, parts, acked, bytes } => {
                    // The newest host tick seen, for the heartbeat to echo:
                    // from snapshots (every other tick), not only from the
                    // host's heartbeats (every half second), or the rtt the
                    // host measures is mostly heartbeat age. The first
                    // window playtest read 200 to 450 ms on a 60 ms link.
                    self.host_tick = self.host_tick.max(tick);
                    if let Some(obs) = self.asm.push(tick, part, parts, bytes) {
                        if let Some(pr) = self.predictor.as_mut() {
                            pr.reconcile(&obs, acked);
                        }
                        self.latest = Some(obs);
                    }
                }
                Packet::Events { seq, tick, events } => {
                    self.routes.send(&mut self.sock, self.me, self.host, &Packet::EventsAck { seq });
                    if seq > self.events_seen {
                        self.events_seen = seq;
                        if let Some(pr) = self.predictor.as_mut() {
                            pr.events(tick, &events);
                        }
                        if let Some(m) = self.map.as_mut() {
                            Predictor::map_keeps_up(m, &events);
                        }
                        self.events.extend(events);
                    }
                }
                Packet::Heartbeat { tick, hash_tick, hash, rtt_ticks, .. } => {
                    self.host_tick = self.host_tick.max(tick);
                    self.rtt_ticks = rtt_ticks;
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
                    self.routes.send(&mut self.sock, self.me, self.host, &Packet::Join { peer: self.me, name: self.name.clone(), session: self.session });
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
                    self.routes.send(&mut self.sock, self.me, self.host, &Packet::Heartbeat { peer: self.me, tick: self.host_tick, hash_tick: 0, hash: 0, rtt_ticks: 0 });

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
        if let Some(pr) = self.predictor.as_ref() {
            log::info!("{}", pr.summary());
        }
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

    /// Roadmap step 4's proof, in process: host and client on loopback
    /// with 100 ms of simulated lag each way (a 200 ms round trip), the
    /// client predicting. One Move, driven for six seconds of real
    /// time at the tick rate. The predicted tank moves off before the
    /// host's snapshot shows it moving; between orders the prediction
    /// matches the host bit for bit, and the mismatch count is zero.
    #[test]
    fn prediction_hides_the_lag_and_matches_the_host() {
        let mut hs = Socket::bind(0).unwrap();
        let mut cs = Socket::bind(0).unwrap();
        hs.set_lag(Duration::from_millis(100));
        cs.set_lag(Duration::from_millis(100));
        let (hid, cid) = (0x0001, 0x0002);
        let mut hr = RouteTable::default();
        hr.direct.insert(cid, cs.local_with_loopback());
        let mut cr = RouteTable::default();
        cr.direct.insert(hid, hs.local_with_loopback());
        let scenario = Scenario { players: vec![("remote".into(), false), ("bot".into(), true)], swarm_interval: 0, ..Scenario::default() };
        let mut host = Host::new(hid, Some(hs), scenario, vec![(cid, "remote".into())], hr);
        // Routing alone, as the sim's own test does: the spoke pillboxes
        // would shoot the tank, which is not what is measured here.
        host.world.pillboxes.clear();
        let mut client = Client::new(cid, cs, hid, cr, "remote");
        let centre = host.world.map.center();
        let start = Instant::now();
        let mut next_host = start;
        let mut next_client = start;
        let mut sent: Option<Instant> = None;
        let mut predicted_moved: Option<Instant> = None;
        let mut host_moved: Option<Instant> = None;
        let mut ahead_max = 0u32;
        while start.elapsed() < Duration::from_secs(6) {
            let now = Instant::now();
            client.poll();
            host.poll();
            if now >= next_host {
                next_host += Duration::from_secs_f32(treadcore_sim::DT);
                host.tick(vec![]);
            }
            if now >= next_client {
                next_client += Duration::from_secs_f32(treadcore_sim::DT);
                client.predict_tick();
            }
            if client.state == ClientState::Playing && sent.is_none() && start.elapsed() > Duration::from_secs(1) {
                client.send(Command::Move { to: centre });
                sent = Some(now);
            }
            if sent.is_some() && predicted_moved.is_none() && client.predicted_tank().map(|t| t.speed > 0.0).unwrap_or(false) {
                predicted_moved = Some(now);
            }
            if let Some(obs) = client.take_observation() {
                if sent.is_some() && host_moved.is_none() && obs.me.as_ref().map(|t| t.speed > 0.0).unwrap_or(false) {
                    host_moved = Some(now);
                }
            }
            if let Some(pr) = client.predictor.as_ref() {
                ahead_max = ahead_max.max(pr.ahead);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(client.state, ClientState::Playing);
        let pr = client.predictor.as_ref().expect("a predictor from the Welcome");
        eprintln!("{}; at most {} ticks ahead of a snapshot", pr.summary(), ahead_max);
        let (p, h) = (predicted_moved.expect("predicted tank moved"), host_moved.expect("host tank moved"));
        let lead = h.saturating_duration_since(p);
        eprintln!("predicted tank moved {} ms before the host's snapshot showed it", lead.as_millis());
        assert!(lead >= Duration::from_millis(120), "prediction led by only {:?}; the round trip is 200 ms", lead);
        assert!(pr.checks >= 10, "only {} checks in six seconds", pr.checks);
        assert_eq!(pr.mismatches, 0, "{}", pr.summary());
        assert!(ahead_max >= 1, "the predictor never ran ahead of a snapshot");
        client.leave();
    }

    /// Rejoin (roadmap step 7, SPEC GAM-12), in process: a client joins,
    /// gives an order, and its process dies (the Client is dropped, no
    /// Leave). A new Client with the same peer id and a new session
    /// number joins from a new socket: the host gives the seat back at
    /// the tick the match has reached, starts its sequences over, and
    /// the newcomer's first order (seq 1 again) is applied.
    #[test]
    fn seat_comes_back_after_a_restart() {
        let hs = Socket::bind(0).unwrap();
        let host_addr = hs.local_with_loopback();
        let (hid, cid) = (0x0001, 0x0002);
        let scenario = Scenario { players: vec![("remote".into(), false), ("bot".into(), true)], swarm_interval: 0, ..Scenario::default() };
        let mut hr = RouteTable::default();
        let cs = Socket::bind(0).unwrap();
        hr.direct.insert(cid, cs.local_with_loopback());
        let mut host = Host::new(hid, Some(hs), scenario, vec![(cid, "remote".into())], hr);
        let mut cr = RouteTable::default();
        cr.direct.insert(hid, host_addr);
        let mut client = Client::new(cid, cs, hid, cr, "remote");
        let first_session = client.session;
        let mut drone_seen = false;
        for _ in 0..400 {
            client.poll();
            host.poll();
            host.tick(vec![]);
            if client.state == ClientState::Playing && client.unacked() == 0 && !drone_seen {
                if let Some(obs) = client.take_observation() {
                    if obs.my_drones().count() > 0 {
                        drone_seen = true;
                        break;
                    }
                    if obs.tick > 5 && obs.my_drones().count() == 0 && client.seq == 0 {
                        client.send(Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: treadcore_sim::V2::new(64.0, 64.0), radius: 3.0 } });
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(drone_seen, "the first process never saw its order applied");
        let tick_at_death = host.world.tick;
        drop(client);
        // Thirty ticks of the match without it; the seat is still joined,
        // as far as the host knows (no Leave, no timeout yet).
        for _ in 0..30 {
            host.poll();
            host.tick(vec![]);
        }
        assert!(host.seats[0].joined);
        let cs2 = Socket::bind(0).unwrap();
        // A real return finds the host through the introducer, and the
        // host learns the new address from the Join itself; here the
        // host's table still names the dead socket until the Join lands.
        let mut cr2 = RouteTable::default();
        cr2.direct.insert(hid, host_addr);
        let mut back = Client::new(cid, cs2, hid, cr2, "remote");
        assert_ne!(back.session, first_session);
        let mut welcomed_at = None;
        let mut recalled = false;
        let mut drones_after = None;
        for _ in 0..600 {
            back.poll();
            host.poll();
            host.tick(vec![]);
            if back.state == ClientState::Playing {
                if welcomed_at.is_none() {
                    welcomed_at = Some(host.world.tick);
                }
                if let Some(obs) = back.take_observation() {
                    if !recalled && obs.my_drones().count() > 0 {
                        back.send(Command::Recall);
                        recalled = true;
                    }
                    if recalled && back.unacked() == 0 {
                        drones_after = Some(obs.my_drones().count());
                        if obs.my_drones().count() == 0 {
                            break;
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let w = welcomed_at.expect("the returning process was never welcomed");
        assert!(w > tick_at_death, "welcomed at tick {} but the match was at {} when the first process died", w, tick_at_death);
        assert_eq!(back.player, Some(0), "the seat came back as a different player");
        assert_eq!(host.seats[0].rejoins, 1);
        assert!(recalled, "the returning process never saw its drone");
        assert_eq!(drones_after, Some(0), "the returning process's first order (seq 1 again) was not applied");
        eprintln!("died at tick {}, back at tick {}, seq restarted and applied", tick_at_death, w);
        back.leave();
    }
}
