//! Across the Internet (roadmap step 7): the introducer, and the peer's
//! side of it.
//!
//! On a LAN the lobby's beacon finds everyone. Across the Internet
//! nothing is broadcast, and most peers sit behind a router that
//! rewrites their address, so two of them cannot even name each other.
//! The introducer is the smallest thing that fixes that: a UDP service
//! that hosts no game, keeps no state past thirty seconds, and tells
//! the peers registered under one lobby name each other's addresses.
//! Two addresses each: the one the introducer heard the peer from (its
//! public address, as the router rewrote it) and the one the peer sees
//! for itself (its LAN address, which is the one that works for two
//! peers behind the same router).
//!
//! The lobby then does the usual thing: greets both addresses of every
//! peer it was introduced to, over and over, until one answers. Each
//! side's first packet out opens its router for the other's packets in;
//! whichever address answers first is the peer's address from then on.
//! A pair whose routers never let a packet through (symmetric NATs on
//! both sides) is a link the matrix shows as unmeasured, and the
//! election routes around it through a peer that reaches both, exactly
//! as it does on a LAN with a broken segment. Nothing in the election
//! or the relay changed for this step.
//!
//! A host keeps registering while it hosts, with `hosting` set. A peer
//! that drops and comes back (same identity: `identity_from_file`) asks
//! the introducer, is told who is hosting, and joins that host directly
//! instead of running a lobby; the host still has its seat and gives it
//! back. That is the prior art's rejoin (SPEC GAM-12), which the backlog
//! deferred to this step.
//!
//! Nothing here spawns a thread or blocks; `poll` from whoever owns the
//! loop, like everything else in this crate.

use crate::transport::Socket;
use std::collections::BTreeMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};
use treadcore_proto::{Introduced, Packet, PeerId};

/// A registration lapses after this long without a `Register`.
pub const EXPIRE: Duration = Duration::from_secs(30);
/// While looking for a game: register this often.
pub const REGISTER_LOOKING: Duration = Duration::from_secs(1);
/// While hosting one: this often.
pub const REGISTER_HOSTING: Duration = Duration::from_secs(5);
/// The introducer tells everyone in a lobby about everyone this often,
/// and at once when someone new registers.
const PUSH: Duration = Duration::from_secs(1);

// --------------------------------------------------------------- service

struct Entry {
    name: String,
    public: SocketAddr,
    local: String,
    hosting: bool,
    server: bool,
    last_seen: Instant,
}

/// The service. `treadcore-server --introduce` runs one; the lobby test
/// runs one on loopback; the test below runs one in process.
pub struct Introducer {
    entries: BTreeMap<(String, PeerId), Entry>,
    last_push: Instant,
    pub registrations: u64,
    pub introductions: u64,
}

impl Default for Introducer {
    fn default() -> Self {
        Introducer { entries: BTreeMap::new(), last_push: Instant::now(), registrations: 0, introductions: 0 }
    }
}

impl Introducer {
    pub fn new() -> Introducer {
        Introducer::default()
    }

    /// How many peers are registered, in how many lobbies.
    pub fn count(&self) -> (usize, usize) {
        let games: std::collections::BTreeSet<&str> = self.entries.keys().map(|(g, _)| g.as_str()).collect();
        (self.entries.len(), games.len())
    }

    /// Read registrations, drop stale ones, push introductions.
    pub fn poll(&mut self, sock: &mut Socket) {
        let now = Instant::now();
        let mut changed: Vec<String> = Vec::new();
        for (from, p) in sock.recv() {
            if let Packet::Register { peer, name, game, local, hosting, server } = p {
                self.registrations += 1;
                let key = (game.clone(), peer);
                let is_new = match self.entries.get_mut(&key) {
                    Some(e) => {
                        let moved = e.public != from || e.hosting != hosting || e.server != server;
                        e.public = from;
                        e.local = local;
                        e.hosting = hosting;
                        e.server = server;
                        e.name = name;
                        e.last_seen = now;
                        moved
                    }
                    None => {
                        log::info!("'{}': peer {:016x} '{}' at {} (local {}){}{}", game, peer, name, from, local, if server { ", a server" } else { "" }, if hosting { ", hosting" } else { "" });
                        self.entries.insert(key, Entry { name, public: from, local, hosting, server, last_seen: now });
                        true
                    }
                };
                if is_new {
                    changed.push(game);
                }
            }
        }
        let before = self.entries.len();
        self.entries.retain(|(game, peer), e| {
            let keep = now.duration_since(e.last_seen) < EXPIRE;
            if !keep {
                log::info!("'{}': peer {:016x} '{}' lapsed", game, peer, e.name);
            }
            keep
        });
        let expired = before != self.entries.len();
        if expired || now.duration_since(self.last_push) >= PUSH {
            self.last_push = now;
            changed = self.entries.keys().map(|(g, _)| g.clone()).collect();
        }
        changed.sort();
        changed.dedup();
        for game in changed {
            self.push(sock, &game);
        }
    }

    fn push(&mut self, sock: &mut Socket, game: &str) {
        let members: Vec<(PeerId, Introduced)> = self
            .entries
            .iter()
            .filter(|((g, _), _)| g == game)
            .map(|((_, id), e)| (*id, Introduced { peer: *id, name: e.name.clone(), public: e.public.to_string(), local: e.local.clone(), hosting: e.hosting, server: e.server }))
            .collect();
        for (id, _) in &members {
            let others: Vec<Introduced> = members.iter().filter(|(o, _)| o != id).map(|(_, i)| i.clone()).collect();
            let to = self.entries.get(&(game.to_string(), *id)).map(|e| e.public).unwrap();
            sock.send(to, &Packet::Introduce { game: game.to_string(), peers: others });
            self.introductions += 1;
        }
    }
}

// ------------------------------------------------------------------ peer

/// A peer's registration with an introducer: a timer, and the address.
pub struct Registration {
    pub addr: SocketAddr,
    pub me: PeerId,
    pub name: String,
    pub game: String,
    pub local: String,
    /// Registered as a dedicated host (`--listen`).
    pub server: bool,
    last: Instant,
}

impl Registration {
    /// `introducer` is "host:port"; resolved once, here.
    pub fn new(introducer: &str, me: PeerId, name: &str, game: &str, local: SocketAddr) -> io::Result<Registration> {
        let addr = introducer.to_socket_addrs()?.next().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("introducer '{}' resolves to nothing", introducer)))?;
        Ok(Registration { addr, me, name: name.into(), game: game.into(), local: local.to_string(), server: false, last: Instant::now() - Duration::from_secs(60) })
    }

    /// Register now if `every` has passed since the last time.
    pub fn poll(&mut self, sock: &mut Socket, every: Duration, hosting: bool) {
        let now = Instant::now();
        if now.duration_since(self.last) >= every {
            self.last = now;
            sock.send(self.addr, &Packet::Register { peer: self.me, name: self.name.clone(), game: self.game.clone(), local: self.local.clone(), hosting, server: self.server });
        }
    }

    /// Whether a packet came from the introducer.
    pub fn is_from(&self, from: SocketAddr) -> bool {
        from == self.addr
    }
}

/// The address this machine's socket is reachable at on its own LAN:
/// the interface the default route uses, found by asking the kernel
/// which source it would pick for a far address (no packet is sent),
/// with the socket's port. Loopback when there is no route at all.
pub fn lan_address(port: u16) -> SocketAddr {
    let ip = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|s| s.connect((Ipv4Addr::new(192, 0, 2, 1), 9)).and_then(|_| s.local_addr()))
        .map(|a| a.ip())
        .unwrap_or(std::net::IpAddr::V4(Ipv4Addr::LOCALHOST));
    SocketAddr::new(ip, port)
}

/// A peer id that survives a restart: read from `path`, or made and
/// written there the first time. Sixteen hex digits, one line.
pub fn identity_from_file(path: &str) -> io::Result<PeerId> {
    match std::fs::read_to_string(path) {
        Ok(text) => {
            let t = text.trim();
            u64::from_str_radix(t, 16)
                .ok()
                .filter(|v| *v != 0)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, format!("{}: not a peer id: '{}'", path, t)))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let id = crate::new_peer_id();
            if let Some(dir) = std::path::Path::new(path).parent() {
                if !dir.as_os_str().is_empty() {
                    std::fs::create_dir_all(dir)?;
                }
            }
            std::fs::write(path, format!("{:016x}\n", id))?;
            log::info!("new identity {:016x} written to {}", id, path);
            Ok(id)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lobby::{Lobby, LobbyConfig};

    /// Two peers that never beacon, on loopback, with an introducer in
    /// the same thread: each registers, is told of the other, greets
    /// it, and the lobby decides. No seed address, no broadcast: the
    /// introducer is the only way they could have met.
    #[test]
    fn introducer_brings_two_silent_peers_together() {
        let mut is = Socket::bind(0).unwrap();
        let intro_addr = is.local_with_loopback().to_string();
        let mut intro = Introducer::new();
        let mut socks = [Socket::bind(0).unwrap(), Socket::bind(0).unwrap()];
        let ids = [0x1111u64, 0x2222u64];
        let mut lobbies: Vec<Lobby> = ids
            .iter()
            .enumerate()
            .map(|(i, &id)| {
                let mut cfg = LobbyConfig::new(id, &format!("p{}", i), "introtest");
                cfg.want = 2;
                cfg.beacon = false;
                cfg.introducer = Some(intro_addr.clone());
                cfg.probe = Duration::from_millis(300);
                cfg.share = Duration::from_millis(300);
                cfg.decide = Duration::from_millis(200);
                let local = socks[i].local_with_loopback();
                let mut l = Lobby::new(cfg);
                l.set_local(local);
                l
            })
            .collect();
        let mut results = [None, None];
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(6) && results.iter().any(|r| r.is_none()) {
            intro.poll(&mut is);
            for i in 0..2 {
                if results[i].is_none() {
                    if let Some(r) = lobbies[i].poll(&mut socks[i]) {
                        results[i] = Some(r);
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        let r0 = results[0].take().expect("peer 0 decided");
        let r1 = results[1].take().expect("peer 1 decided");
        assert_eq!(r0.decision.host, r1.decision.host, "the two peers elected different hosts");
        assert_eq!(r0.peers.len(), 1);
        assert_eq!(r1.peers.len(), 1);
        assert_eq!(r0.peers[0].id, 0x2222);
        assert_eq!(r1.peers[0].id, 0x1111);
        assert!(r0.peers[0].median_us().is_some(), "peer 0 never measured peer 1: the punch did not get through");
        assert_eq!(intro.count().0, 2);
        eprintln!("introducer: {} registrations, {} introductions; host {:016x}", intro.registrations, intro.introductions, r0.decision.host);
    }

    /// The same id twice from one file; a file that is not an id is
    /// refused, not replaced.
    #[test]
    fn identity_file_round_trips() {
        let dir = std::env::temp_dir().join(format!("treadcore-identity-{}", std::process::id()));
        let path = dir.join("peer.id");
        let p = path.to_str().unwrap();
        let a = identity_from_file(p).unwrap();
        let b = identity_from_file(p).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, 0);
        std::fs::write(&path, "not hex\n").unwrap();
        assert!(identity_from_file(p).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
