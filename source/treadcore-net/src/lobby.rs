//! The lobby: from "I am here" to "this one hosts, send through that one".
//!
//! Four phases, each on a timer, every peer running the same machine:
//!
//!   Discover  beacon the LAN and greet the addresses given by hand, until
//!             the wanted number of peers is known or the timer runs out.
//!   Probe     ping every peer ten times a second; keep the samples.
//!   Share     publish my row (median RTT to each peer) and collect the
//!             others', until every row is in or the timer runs out.
//!   Decide    run `elect` on the matrix, announce the result, collect
//!             everyone else's announcement; the one from the lowest
//!             peer id is final -- which is the same as mine unless rows
//!             were lost, and then it is the tie-break everyone agrees on.
//!
//! A peer that only hears about another through a third party's Hello
//! still probes it directly, and if the probe never answers the matrix
//! says so, and the election routes around it.

use crate::election::{elect, Decision, Row};
use crate::transport::Socket;
use std::collections::BTreeMap;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use treadcore_proto::{Packet, PeerId};

#[derive(Clone, Debug)]
pub struct LobbyConfig {
    pub me: PeerId,
    pub name: String,
    pub game: String,
    /// Addresses to greet directly: "host:port" strings.
    pub seeds: Vec<String>,
    /// Stop discovering as soon as this many peers (including me) are
    /// known. 1 means play alone, decide instantly.
    pub want: usize,
    pub discover: Duration,
    pub probe: Duration,
    pub share: Duration,
    pub decide: Duration,
}

impl LobbyConfig {
    pub fn new(me: PeerId, name: &str, game: &str) -> LobbyConfig {
        LobbyConfig {
            me,
            name: name.into(),
            game: game.into(),
            seeds: vec![],
            want: 4,
            discover: Duration::from_secs(8),
            probe: Duration::from_millis(1500),
            share: Duration::from_millis(1500),
            decide: Duration::from_millis(800),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Discover,
    Probe,
    Share,
    Decide,
    Done,
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub id: PeerId,
    pub name: String,
    pub addr: SocketAddr,
    pub samples: Vec<u32>,
    pub last_seen: Instant,
}

impl Peer {
    pub fn median_us(&self) -> Option<u32> {
        if self.samples.len() < 2 {
            return None;
        }
        let mut s = self.samples.clone();
        s.sort_unstable();
        Some(s[s.len() / 2])
    }
}

#[derive(Clone, Debug)]
pub struct LobbyResult {
    pub decision: Decision,
    pub peers: Vec<Peer>,
    pub matrix: BTreeMap<PeerId, Row>,
    /// This peer's next hop toward the host, if it is not the host and
    /// not directly connected. None means talk to the host directly.
    pub next_hop: Option<PeerId>,
}

impl LobbyResult {
    pub fn i_host(&self, me: PeerId) -> bool {
        self.decision.host == me
    }
    pub fn addr_of(&self, p: PeerId) -> Option<SocketAddr> {
        self.peers.iter().find(|x| x.id == p).map(|x| x.addr)
    }
}

pub struct Lobby {
    pub cfg: LobbyConfig,
    pub phase: Phase,
    pub peers: BTreeMap<PeerId, Peer>,
    rows: BTreeMap<PeerId, (u32, Row)>,
    elects: BTreeMap<PeerId, (u32, Decision)>,
    epoch: u32,
    t0: Instant,
    phase_start: Instant,
    last_beacon: Instant,
    last_ping: Instant,
    last_share: Instant,
    ping_seq: u32,
    result: Option<LobbyResult>,
}

impl Lobby {
    pub fn new(cfg: LobbyConfig) -> Lobby {
        let now = Instant::now();
        Lobby {
            cfg,
            phase: Phase::Discover,
            peers: BTreeMap::new(),
            rows: BTreeMap::new(),
            elects: BTreeMap::new(),
            epoch: 0,
            t0: now,
            phase_start: now,
            last_beacon: now - Duration::from_secs(10),
            last_ping: now,
            last_share: now - Duration::from_secs(10),
            ping_seq: 0,
            result: None,
        }
    }

    fn now_us(&self) -> u64 {
        self.t0.elapsed().as_micros() as u64
    }

    pub fn result(&self) -> Option<&LobbyResult> {
        self.result.as_ref()
    }

    /// All peer ids including me, sorted.
    fn members(&self) -> Vec<PeerId> {
        let mut v: Vec<PeerId> = self.peers.keys().copied().collect();
        v.push(self.cfg.me);
        v.sort_unstable();
        v
    }

    fn my_row(&self) -> Row {
        self.peers.values().map(|p| (p.id, p.median_us())).collect()
    }

    fn hello(&self) -> Packet {
        Packet::Hello {
            peer: self.cfg.me,
            name: self.cfg.name.clone(),
            game: self.cfg.game.clone(),
            known: self.peers.values().map(|p| (p.id, p.name.clone(), p.addr.to_string())).collect(),
        }
    }

    fn learn(&mut self, id: PeerId, name: &str, addr: SocketAddr) -> bool {
        if id == self.cfg.me {
            return false;
        }
        let now = Instant::now();
        match self.peers.get_mut(&id) {
            Some(p) => {
                p.last_seen = now;
                false
            }
            None => {
                log::info!("peer {:016x} '{}' at {}", id, name, addr);
                self.peers.insert(id, Peer { id, name: name.into(), addr, samples: vec![], last_seen: now });
                self.epoch += 1;
                true
            }
        }
    }

    fn enter(&mut self, phase: Phase) {
        log::info!("lobby: {:?} -> {:?} with {} peer(s)", self.phase, phase, self.peers.len() + 1);
        self.phase = phase;
        self.phase_start = Instant::now();
    }

    /// Drive the lobby. Call often (every few ms is fine; every 50 ms is
    /// enough). Returns the result once, the tick it is decided.
    pub fn poll(&mut self, sock: &mut Socket) -> Option<LobbyResult> {
        let now = Instant::now();
        for (from, p) in sock.recv() {
            self.handle(sock, from, p);
        }
        match self.phase {
            Phase::Discover => {
                if now.duration_since(self.last_beacon) >= Duration::from_secs(1) {
                    self.last_beacon = now;
                    sock.broadcast(&Packet::Beacon { peer: self.cfg.me, name: self.cfg.name.clone(), game: self.cfg.game.clone() });
                    let hello = self.hello();
                    for s in self.cfg.seeds.clone() {
                        match s.to_socket_addrs() {
                            Ok(addrs) => {
                                for a in addrs {
                                    sock.send(a, &hello);
                                }
                            }
                            Err(e) => log::warn!("seed '{}': {}", s, e),
                        }
                    }
                }
                let enough = self.peers.len() + 1 >= self.cfg.want;
                let timed_out = now.duration_since(self.phase_start) >= self.cfg.discover;
                if enough || timed_out || self.cfg.want <= 1 {
                    if self.peers.is_empty() {
                        // Alone. No matrix, no vote: I host.
                        let decision = Decision { host: self.cfg.me, route: vec![], eccentricity_us: 0 };
                        return self.finish(decision);
                    }
                    self.enter(Phase::Probe);
                }
            }
            Phase::Probe => {
                if now.duration_since(self.last_ping) >= Duration::from_millis(100) {
                    self.last_ping = now;
                    self.ping_seq += 1;
                    let seq = self.ping_seq;
                    let sent_us = self.now_us();
                    let addrs: Vec<SocketAddr> = self.peers.values().map(|p| p.addr).collect();
                    for a in addrs {
                        sock.send(a, &Packet::Ping { peer: self.cfg.me, seq, sent_us });
                    }
                }
                if now.duration_since(self.phase_start) >= self.cfg.probe {
                    let row = self.my_row();
                    self.rows.insert(self.cfg.me, (self.epoch, row));
                    self.enter(Phase::Share);
                }
            }
            Phase::Share => {
                if now.duration_since(self.last_share) >= Duration::from_millis(200) {
                    self.last_share = now;
                    let row = self.my_row();
                    self.rows.insert(self.cfg.me, (self.epoch, row.clone()));
                    let pk = Packet::RttRow { peer: self.cfg.me, epoch: self.epoch, rtts: row };
                    let addrs: Vec<SocketAddr> = self.peers.values().map(|p| p.addr).collect();
                    for a in addrs {
                        sock.send(a, &pk);
                    }
                }
                let all_in = self.members().iter().all(|m| self.rows.contains_key(m));
                if all_in || now.duration_since(self.phase_start) >= self.cfg.share {
                    if let Some(d) = self.compute() {
                        self.elects.insert(self.cfg.me, (self.epoch, d));
                    }
                    self.enter(Phase::Decide);
                }
            }
            Phase::Decide => {
                if now.duration_since(self.last_share) >= Duration::from_millis(200) {
                    self.last_share = now;
                    if let Some((_, d)) = self.elects.get(&self.cfg.me).cloned() {
                        let pk = Packet::Elect { peer: self.cfg.me, epoch: self.epoch, host: d.host, route: d.route.clone() };
                        let addrs: Vec<SocketAddr> = self.peers.values().map(|p| p.addr).collect();
                        for a in addrs {
                            sock.send(a, &pk);
                        }
                    }
                }
                let all_in = self.members().iter().all(|m| self.elects.contains_key(m));
                if all_in || now.duration_since(self.phase_start) >= self.cfg.decide {
                    // The lowest id's verdict is final. It usually matches
                    // everyone's; when it does not, it is still the one
                    // everyone converges on.
                    let verdict = self.elects.iter().min_by_key(|(id, _)| **id).map(|(_, (_, d))| d.clone());
                    match verdict {
                        Some(d) => return self.finish(d),
                        None => {
                            log::error!("lobby: no election result; hosting alone");
                            let d = Decision { host: self.cfg.me, route: vec![], eccentricity_us: 0 };
                            return self.finish(d);
                        }
                    }
                }
            }
            Phase::Done => {}
        }
        None
    }

    fn compute(&self) -> Option<Decision> {
        let members = self.members();
        let rows: BTreeMap<PeerId, Row> = self.rows.iter().map(|(k, (_, r))| (*k, r.clone())).collect();
        let d = elect(&members, &rows);
        match &d {
            Some(d) => log::info!("lobby: my vote: host {:016x}, worst path {} ms, route {:?}", d.host, d.eccentricity_us / 1000, d.route.iter().map(|(p, v)| format!("{:04x}->{:04x}", p & 0xffff, v & 0xffff)).collect::<Vec<_>>()),
            None => log::warn!("lobby: no candidate reaches every peer"),
        }
        d
    }

    fn finish(&mut self, decision: Decision) -> Option<LobbyResult> {
        let me = self.cfg.me;
        let next_hop = decision.route.iter().find(|(p, _)| *p == me).map(|(_, via)| *via).filter(|via| *via != decision.host);
        let r = LobbyResult {
            decision,
            peers: self.peers.values().cloned().collect(),
            matrix: self.rows.iter().map(|(k, (_, r))| (*k, r.clone())).collect(),
            next_hop,
        };
        log::info!(
            "lobby: decided. host {:016x} ({}), {} peer(s), my next hop {}",
            r.decision.host,
            if r.i_host(me) { "me" } else { "remote" },
            r.peers.len() + 1,
            next_hop.map(|h| format!("{:016x}", h)).unwrap_or_else(|| "direct".into())
        );
        self.enter(Phase::Done);
        self.result = Some(r.clone());
        Some(r)
    }

    fn handle(&mut self, sock: &mut Socket, from: SocketAddr, p: Packet) {
        match p {
            Packet::Beacon { peer, name, game } => {
                if game != self.cfg.game || peer == self.cfg.me {
                    return;
                }
                self.learn(peer, &name, from);
                let hello = self.hello();
                sock.send(from, &hello);
            }
            Packet::Hello { peer, name, game, known } => {
                if game != self.cfg.game || peer == self.cfg.me {
                    return;
                }
                let new = self.learn(peer, &name, from);
                for (id, n, a) in known {
                    if let Ok(addr) = a.parse::<SocketAddr>() {
                        if self.learn(id, &n, addr) {
                            let hello = self.hello();
                            sock.send(addr, &hello);
                        }
                    }
                }
                if new {
                    let hello = self.hello();
                    sock.send(from, &hello);
                }
            }
            Packet::Ping { peer, seq, sent_us } => {
                if let Some(p) = self.peers.get_mut(&peer) {
                    p.last_seen = Instant::now();
                    p.addr = from;
                }
                sock.send(from, &Packet::Pong { peer: self.cfg.me, seq, sent_us });
            }
            Packet::Pong { peer, sent_us, .. } => {
                let rtt = self.now_us().saturating_sub(sent_us) as u32;
                if let Some(p) = self.peers.get_mut(&peer) {
                    p.samples.push(rtt);
                    p.last_seen = Instant::now();
                }
            }
            Packet::RttRow { peer, epoch, rtts } => {
                // Rows mention peers I might not know yet; nothing to do
                // about those but note it. The election only uses rows
                // from members.
                let keep = self.rows.get(&peer).map(|(e, _)| epoch >= *e).unwrap_or(true);
                if keep {
                    self.rows.insert(peer, (epoch, rtts));
                }
            }
            Packet::Elect { peer, epoch, host, route } => {
                let keep = self.elects.get(&peer).map(|(e, _)| epoch >= *e).unwrap_or(true);
                if keep {
                    self.elects.insert(peer, (epoch, Decision { host, route, eccentricity_us: 0 }));
                }
            }
            _ => {}
        }
    }
}
