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
//!
//! Across the Internet (roadmap step 7) the beacon reaches nobody, so
//! the lobby registers with an introducer instead (`introducer.rs`),
//! which answers with every other peer in the lobby and two addresses
//! for each. Until a peer has been heard from directly, every greeting
//! and every probe goes to both of its addresses; the first that
//! answers is its address from then on. If the introducer names a peer
//! that is already hosting, the lobby does not elect: it pings that
//! host until it answers and decides for it, which is how a player who
//! dropped out gets back to the match they were in.
//!
//! A dedicated server (`treadcore-server --listen`, roadmap step 8)
//! runs this same machine with `server` set. It takes no seat, so it
//! does not count toward anyone's `want`; it never hosts alone and never
//! joins a match it is told of; and it does not start probing on a timer
//! of its own but when the first seat's Ping says that seat has. The
//! election weighs it with a head start (`election::elect_among`). A
//! server that is not elected is dropped from the result, and the
//! caller goes back to listening.

use crate::election::{elect_among, Decision, Row};
use crate::introducer::{lan_address, Registration, REGISTER_LOOKING};
use crate::transport::Socket;
use std::collections::{BTreeMap, BTreeSet};
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
    /// Shout on the LAN every second (off for a peer on the Internet
    /// that has an introducer and no one on its segment to disturb).
    pub beacon: bool,
    /// An introducer's "host:port", for peers no broadcast reaches.
    pub introducer: Option<String>,
    /// Stop discovering as soon as this many peers (including me) are
    /// known. 1 means play alone, decide instantly.
    pub want: usize,
    /// A dedicated host: no seat, preferred by the election.
    pub server: bool,
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
            beacon: true,
            introducer: None,
            want: 4,
            server: false,
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
    /// A dedicated host, by its own say.
    pub server: bool,
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
    /// Addresses a peer might be at, from the introducer or a third
    /// party's Hello, tried in turn until the peer is heard from.
    candidates: BTreeMap<PeerId, Vec<SocketAddr>>,
    /// Peers a packet has come from directly: their `addr` is known to
    /// work, and the candidates are dropped.
    heard: BTreeSet<PeerId>,
    /// A peer the introducer says is hosting: join it, do not elect.
    hosting: Option<PeerId>,
    /// This socket's LAN address, told to the introducer. Guessed from
    /// the default route unless `set_local` says otherwise.
    local: Option<SocketAddr>,
    registration: Option<Registration>,
    introducer_failed: bool,
    last_punch: Instant,
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
            candidates: BTreeMap::new(),
            heard: BTreeSet::new(),
            hosting: None,
            local: None,
            registration: None,
            introducer_failed: false,
            last_punch: now - Duration::from_secs(10),
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

    /// The address to register as this peer's own (a test on loopback
    /// says loopback; otherwise the LAN address is guessed).
    pub fn set_local(&mut self, addr: SocketAddr) {
        self.local = Some(addr);
    }

    /// The registration, for whoever hosts to keep up after the lobby.
    pub fn take_registration(&mut self) -> Option<Registration> {
        self.registration.take()
    }

    /// Where to send a peer something: its one address once it has
    /// been heard from, every address it might have until then.
    fn targets(&self, id: PeerId) -> Vec<SocketAddr> {
        let Some(p) = self.peers.get(&id) else { return vec![] };
        if self.heard.contains(&id) {
            return vec![p.addr];
        }
        let mut v = vec![p.addr];
        for a in self.candidates.get(&id).into_iter().flatten() {
            if !v.contains(a) {
                v.push(*a);
            }
        }
        v
    }

    /// A packet came straight from `id` at `from`: that is its address.
    fn heard_from(&mut self, id: PeerId, from: SocketAddr) {
        if let Some(p) = self.peers.get_mut(&id) {
            p.last_seen = Instant::now();
            if p.addr != from {
                log::info!("peer {:016x} answers at {} (was {})", id, from, p.addr);
                p.addr = from;
            }
            if self.heard.insert(id) {
                self.candidates.remove(&id);
            }
        }
    }

    fn candidate(&mut self, id: PeerId, addr: SocketAddr) {
        if id == self.cfg.me || self.heard.contains(&id) {
            return;
        }
        let v = self.candidates.entry(id).or_default();
        if !v.contains(&addr) {
            v.push(addr);
        }
    }

    /// Greet every address of every peer not yet heard from. Each
    /// packet out opens this side's router for that peer's packets in;
    /// the first answer settles the address.
    fn punch(&mut self, sock: &mut Socket) {
        let unheard: Vec<PeerId> = self.peers.keys().filter(|id| !self.heard.contains(id)).copied().collect();
        if unheard.is_empty() {
            return;
        }
        let hello = self.hello();
        for id in unheard {
            if self.hosting == Some(id) {
                // A host runs no lobby and answers no Hello; it does
                // answer a Ping from a peer that has a seat.
                self.ping_seq += 1;
                let (seq, sent_us) = (self.ping_seq, self.now_us());
                for a in self.targets(id) {
                    sock.send(a, &Packet::Ping { peer: self.cfg.me, seq, sent_us });
                }
            } else {
                for a in self.targets(id) {
                    sock.send(a, &hello);
                }
            }
        }
    }

    /// All peer ids including me, sorted.
    fn members(&self) -> Vec<PeerId> {
        let mut v: Vec<PeerId> = self.peers.keys().copied().collect();
        v.push(self.cfg.me);
        v.sort_unstable();
        v
    }

    /// Every server among the members, me included if I am one.
    fn servers(&self) -> Vec<PeerId> {
        let mut v: Vec<PeerId> = self.peers.values().filter(|p| p.server).map(|p| p.id).collect();
        if self.cfg.server {
            v.push(self.cfg.me);
        }
        v.sort_unstable();
        v
    }

    /// How many seats the lobby has: the peers that are not servers,
    /// and me unless I am one.
    pub fn seats(&self) -> usize {
        self.peers.values().filter(|p| !p.server).count() + usize::from(!self.cfg.server)
    }

    fn my_row(&self) -> Row {
        self.peers.values().map(|p| (p.id, p.median_us())).collect()
    }

    fn hello(&self) -> Packet {
        Packet::Hello {
            peer: self.cfg.me,
            name: self.cfg.name.clone(),
            game: self.cfg.game.clone(),
            server: self.cfg.server,
            known: self.peers.values().map(|p| (p.id, p.name.clone(), p.addr.to_string(), p.server)).collect(),
        }
    }

    fn learn(&mut self, id: PeerId, name: &str, addr: SocketAddr, server: bool) -> bool {
        if id == self.cfg.me {
            return false;
        }
        let now = Instant::now();
        match self.peers.get_mut(&id) {
            Some(p) => {
                p.last_seen = now;
                p.server |= server;
                false
            }
            None => {
                log::info!("peer {:016x} '{}' at {}{}", id, name, addr, if server { ", a server" } else { "" });
                self.peers.insert(id, Peer { id, name: name.into(), addr, server, samples: vec![], last_seen: now });
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
        if self.registration.is_none() && !self.introducer_failed {
            if let Some(intro) = self.cfg.introducer.clone() {
                let local = self.local.unwrap_or_else(|| lan_address(sock.local.port()));
                match Registration::new(&intro, self.cfg.me, &self.cfg.name, &self.cfg.game, local) {
                    Ok(mut r) => {
                        r.server = self.cfg.server;
                        log::info!("registering with the introducer at {} as {}", r.addr, local);
                        self.registration = Some(r);
                    }
                    Err(e) => {
                        log::error!("introducer '{}': {}; going on without it", intro, e);
                        self.introducer_failed = true;
                    }
                }
            }
        }
        for (from, p) in sock.recv() {
            self.handle(sock, from, p);
        }
        match self.phase {
            Phase::Discover => {
                if let Some(r) = self.registration.as_mut() {
                    r.poll(sock, REGISTER_LOOKING, false);
                }
                if now.duration_since(self.last_punch) >= Duration::from_millis(250) {
                    self.last_punch = now;
                    self.punch(sock);
                }
                if let Some(h) = self.hosting {
                    if self.heard.contains(&h) {
                        log::info!("lobby: {:016x} is hosting and answers; joining it, no election", h);
                        let decision = Decision { host: h, route: vec![], eccentricity_us: 0 };
                        return self.finish(decision);
                    }
                }
                if now.duration_since(self.last_beacon) >= Duration::from_secs(1) {
                    self.last_beacon = now;
                    if self.cfg.beacon {
                        sock.broadcast(&Packet::Beacon { peer: self.cfg.me, name: self.cfg.name.clone(), game: self.cfg.game.clone(), server: self.cfg.server });
                    }
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
                // A host that was named but has not answered yet is
                // waited for, not elected around, until the timer.
                let waiting_for_host = self.hosting.is_some();
                let enough = self.seats() >= self.cfg.want && !waiting_for_host;
                let timed_out = now.duration_since(self.phase_start) >= self.cfg.discover;
                if self.cfg.server {
                    // A server follows: it probes when a seat does (the
                    // first seat's Ping, in `handle`), and otherwise
                    // waits as long as it takes.
                    if timed_out {
                        self.phase_start = now;
                    }
                } else if enough || timed_out || (self.cfg.want <= 1 && !waiting_for_host) {
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
                    let ids: Vec<PeerId> = self.peers.keys().copied().collect();
                    for id in ids {
                        for a in self.targets(id) {
                            sock.send(a, &Packet::Ping { peer: self.cfg.me, seq, sent_us });
                        }
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
        let d = elect_among(&members, &self.servers(), &rows);
        match &d {
            Some(d) => log::info!("lobby: my vote: host {:016x}, worst path {} ms, route {:?}", d.host, d.eccentricity_us / 1000, d.route.iter().map(|(p, v)| format!("{:04x}->{:04x}", p & 0xffff, v & 0xffff)).collect::<Vec<_>>()),
            None => log::warn!("lobby: no candidate reaches every peer"),
        }
        d
    }

    fn finish(&mut self, decision: Decision) -> Option<LobbyResult> {
        let me = self.cfg.me;
        let next_hop = decision.route.iter().find(|(p, _)| *p == me).map(|(_, via)| *via).filter(|via| *via != decision.host);
        // A server that does not host has no part in the match: no seat,
        // no route.
        let r = LobbyResult {
            peers: self.peers.values().filter(|p| !p.server || p.id == decision.host).cloned().collect(),
            decision,
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
        // Anything that comes straight from a peer settles its address.
        match &p {
            Packet::Beacon { peer, .. } | Packet::Hello { peer, .. } | Packet::Ping { peer, .. } | Packet::Pong { peer, .. } | Packet::RttRow { peer, .. } | Packet::Elect { peer, .. } => {
                let peer = *peer;
                self.heard_from(peer, from);
            }
            _ => {}
        }
        match p {
            Packet::Beacon { peer, name, game, server } => {
                if game != self.cfg.game || peer == self.cfg.me {
                    return;
                }
                self.learn(peer, &name, from, server);
                self.heard_from(peer, from);
                let hello = self.hello();
                sock.send(from, &hello);
            }
            Packet::Hello { peer, name, game, server, known } => {
                if game != self.cfg.game || peer == self.cfg.me {
                    return;
                }
                let new = self.learn(peer, &name, from, server);
                self.heard_from(peer, from);
                for (id, n, a, s) in known {
                    if let Ok(addr) = a.parse::<SocketAddr>() {
                        if self.learn(id, &n, addr, s) {
                            let hello = self.hello();
                            sock.send(addr, &hello);
                        } else {
                            self.candidate(id, addr);
                        }
                    }
                }
                if new {
                    let hello = self.hello();
                    sock.send(from, &hello);
                }
            }
            Packet::Introduce { game, peers } => {
                let from_introducer = self.registration.as_ref().map(|r| r.is_from(from)).unwrap_or(false);
                if game != self.cfg.game || !from_introducer {
                    return;
                }
                for i in peers {
                    if i.peer == self.cfg.me {
                        continue;
                    }
                    let public = i.public.parse::<SocketAddr>().ok();
                    let local = i.local.parse::<SocketAddr>().ok();
                    let Some(first) = public.or(local) else { continue };
                    if self.learn(i.peer, &i.name, first, i.server) {
                        log::info!("introduced to {:016x} '{}': public {}, local {}{}{}", i.peer, i.name, i.public, i.local, if i.server { ", a server" } else { "" }, if i.hosting { ", hosting" } else { "" });
                    }
                    for a in [public, local].into_iter().flatten() {
                        self.candidate(i.peer, a);
                    }
                    // A server never joins a match; it waits for the next.
                    if i.hosting && !self.cfg.server && self.hosting.is_none() && self.phase == Phase::Discover {
                        self.hosting = Some(i.peer);
                    }
                }
            }
            Packet::Ping { peer, seq, sent_us } => {
                sock.send(from, &Packet::Pong { peer: self.cfg.me, seq, sent_us });
                let a_seat = self.peers.get(&peer).map(|p| !p.server).unwrap_or(false);
                if self.cfg.server && self.phase == Phase::Discover && a_seat {
                    log::info!("lobby: seat {:016x} is probing; so is this server", peer);
                    self.enter(Phase::Probe);
                }
            }
            Packet::Pong { peer, sent_us, .. } => {
                let rtt = self.now_us().saturating_sub(sent_us) as u32;
                if let Some(p) = self.peers.get_mut(&peer) {
                    p.samples.push(rtt);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Three seats and a dedicated server on loopback, no beacon, every
    /// seat given the server's address and the first seat's. The seats
    /// want three: the server is not counted. The links are all alike,
    /// so the server's preference decides: it hosts, every seat agrees,
    /// and no seat lists it as a seat-mate except as the host.
    #[test]
    fn server_in_the_lobby_hosts_and_takes_no_seat() {
        let mut socks: Vec<Socket> = (0..4).map(|_| Socket::bind(0).unwrap()).collect();
        let addrs: Vec<String> = socks.iter().map(|s| s.local_with_loopback().to_string()).collect();
        let ids = [0x10u64, 0x20, 0x30, 0x05];
        let mut lobbies: Vec<Lobby> = (0..4)
            .map(|i| {
                let mut cfg = LobbyConfig::new(ids[i], &format!("p{}", i), "servertest");
                cfg.want = 3;
                cfg.beacon = false;
                cfg.server = i == 3;
                if i < 3 {
                    cfg.seeds = vec![addrs[3].clone(), addrs[0].clone()];
                }
                cfg.probe = Duration::from_millis(400);
                cfg.share = Duration::from_millis(400);
                cfg.decide = Duration::from_millis(300);
                Lobby::new(cfg)
            })
            .collect();
        let mut results: Vec<Option<LobbyResult>> = vec![None, None, None, None];
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(8) && results.iter().any(|r| r.is_none()) {
            for i in 0..4 {
                if results[i].is_none() {
                    results[i] = lobbies[i].poll(&mut socks[i]);
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let results: Vec<LobbyResult> = results.into_iter().map(|r| r.expect("every member decided")).collect();
        for (i, r) in results.iter().enumerate() {
            assert_eq!(r.decision.host, 0x05, "member {} elected {:x}", i, r.decision.host);
        }
        assert_eq!(results[3].peers.len(), 3, "the server hosts the three seats");
        assert_eq!(lobbies[3].seats(), 3);
        assert_eq!(lobbies[0].seats(), 3);
    }
}
