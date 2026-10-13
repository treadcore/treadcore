//! treadcore-proto -- the wire vocabulary.
//!
//! One enum, `Packet`, is everything that ever crosses a socket. It is
//! serde + bincode, prefixed by a four-byte magic and a version, so a
//! packet from another build is dropped on the floor at the first byte
//! rather than misread as something it is not.
//!
//! Three families of message live here:
//!
//!   negotiation  Beacon, Hello, Ping/Pong, RttRow, Elect. How four peers
//!                find one another, measure the links between them, and
//!                agree -- without a referee -- which of them hosts and
//!                how the rest route to it. See treadcore-net::lobby.
//!   session      Join, Welcome, Command, CommandAck, Snapshot, Events.
//!                The host-authoritative game: clients send orders, the
//!                host steps the world and sends back what each client is
//!                allowed to see.
//!   relay        Relay. A packet for a peer the sender cannot reach
//!                directly, handed to one it can, with the final address
//!                inside. This is Bolo's ring, generalised to a tree.
//!
//! Nothing here knows about sockets. treadcore-net owns those.

use serde::{Deserialize, Serialize};
use treadcore_sim::{Command, Event, Map, Observation, PlayerId, Scenario, Tick};

pub const MAGIC: [u8; 4] = *b"TRDC";
/// Bump on any change to `Packet` or to anything it carries from
/// treadcore-sim. Peers on different versions ignore each other entirely.
pub const VERSION: u16 = 10;



/// Default UDP port. 0xB8C0 -- nothing registered there.
pub const PORT: u16 = 47296;
/// A datagram this size fits one Ethernet frame with room to spare; the
/// snapshot sender splits anything bigger.
pub const MAX_DATAGRAM: usize = 1200;

/// A peer's identity for the life of the process: random 64 bits. Not an
/// address, because the address a peer sees for another depends on which
/// side of a NAT it is on; and not a name, because names collide.
pub type PeerId = u64;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Packet {
    // ---- negotiation ---------------------------------------------------
    /// Shouted to the LAN broadcast address every second by anyone looking
    /// for a game. `game` is the lobby name; peers in different lobbies
    /// ignore each other.
    Beacon { peer: PeerId, name: String, game: String },
    /// Unicast reply to a Beacon, and the first thing sent to a peer given
    /// on the command line. Carries the sender's current peer list, with
    /// the address it sees each one at, so the set converges in one round.
    /// (Addresses are as the sender sees them: right on a LAN, and the
    /// best anyone has across a NAT.)
    Hello { peer: PeerId, name: String, game: String, known: Vec<(PeerId, String, String)> },
    /// Latency probe. `sent_us` is the sender's clock, echoed back
    /// untouched; the sender diffs it on return. Clocks are never compared
    /// across machines.
    Ping { peer: PeerId, seq: u32, sent_us: u64 },
    Pong { peer: PeerId, seq: u32, sent_us: u64 },
    /// One row of the latency matrix: this peer's measured RTT in
    /// microseconds to each peer it knows, or None where probes went
    /// unanswered. `epoch` is incremented each time the peer set changes,
    /// so stale rows from a previous membership are discarded.
    RttRow { peer: PeerId, epoch: u32, rtts: Vec<(PeerId, Option<u32>)> },
    /// The sender's conclusion from the matrix it holds. Every peer
    /// computes this from the same rows and should announce the same
    /// thing; when they do not, the announcement from the lowest PeerId
    /// wins. `route` is (peer, next hop toward host) for every non-host.
    Elect { peer: PeerId, epoch: u32, host: PeerId, route: Vec<(PeerId, PeerId)> },

    // ---- session -------------------------------------------------------
    Join { peer: PeerId, name: String },
    /// The whole map, as it is at this tick (craters and walls included),
    /// not just its cells: a map from a file has its own starts, depots
    /// and pillboxes, and a client must not guess them from a seed.
    Welcome { player: PlayerId, scenario: Scenario, tick: Tick, map: Map },

    /// Rejected join: lobby full, or the match is over.
    Refuse { why: String },
    /// A client's order. `seq` is per-client and monotonic; the client
    /// resends until it sees `CommandAck { seq }`. The host applies each
    /// seq once, in seq order.
    Command { peer: PeerId, seq: u32, cmd: Command },
    CommandAck { seq: u32 },
    /// The host's view for one player, as of `tick`. Unreliable: a lost
    /// one is replaced by the next. `part`/`parts` split a big observation
    /// across datagrams; the client reassembles by tick. `acked` is the
    /// last of this client's command seqs the host had applied by this
    /// tick: what a predicting client re-applies is everything after it
    /// (roadmap step 4).
    Snapshot { tick: Tick, part: u8, parts: u8, acked: u32, bytes: Vec<u8> },
    /// Events the player is allowed to know about. Sent reliably-ish: each
    /// batch repeats until acked by `EventsAck`, so a kill is never lost.
    Events { seq: u32, tick: Tick, events: Vec<Event> },
    EventsAck { seq: u32 },
    /// A client leaving, or the host shutting down.
    Leave { peer: PeerId },
    /// Periodic from both ends; three missed and the other side is gone.
    /// From the host it also carries the world hash as of `hash_tick`
    /// (the last multiple of `HASH_EVERY`), so a client that runs the
    /// world can compare, and one that does not can at least log it:
    /// four peers' logs with the same hash at the same tick is the
    /// lobby test's proof. From a client, `hash` is 0 until it predicts.
    /// `rtt_ticks` from the host is its estimate of that client's round
    /// trip: the host's tick minus the host tick the client's last
    /// heartbeat echoed (a snapshot out, a heartbeat back). The HUD's
    /// rtt is this; a client sends 0.
    Heartbeat { peer: PeerId, tick: Tick, hash_tick: Tick, hash: u64, rtt_ticks: u32 },


    /// A packet too big for one datagram, in pieces: `Welcome` with a
    /// large map. `id` tells one big packet's pieces from another's; the
    /// receiver reassembles by id and decodes `inner`. Unreliable like a
    /// snapshot: a lost piece loses the packet, and the sender's retry
    /// (a repeated Join draws a fresh Welcome) sends a new id.
    Chunk { id: u32, part: u16, parts: u16, bytes: Vec<u8> },

    // ---- relay ---------------------------------------------------------
    /// Deliver `inner` to `to`. A peer that receives one and is not `to`
    /// looks `to` up in its route table and forwards. `hops` stops a bad
    /// table from looping forever.
    Relay { from: PeerId, to: PeerId, hops: u8, inner: Vec<u8> },
}

#[derive(Debug)]
pub enum DecodeError {
    TooShort,
    BadMagic,
    Version(u16),
    Body(bincode::Error),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::TooShort => write!(f, "datagram too short"),
            DecodeError::BadMagic => write!(f, "not a treadcore packet"),
            DecodeError::Version(v) => write!(f, "protocol version {} (ours is {})", v, VERSION),
            DecodeError::Body(e) => write!(f, "malformed body: {}", e),
        }
    }
}

impl Packet {
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        bincode::serialize_into(&mut out, self).expect("packet serialises");
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Packet, DecodeError> {
        if buf.len() < 6 {
            return Err(DecodeError::TooShort);
        }
        if buf[..4] != MAGIC {
            return Err(DecodeError::BadMagic);
        }
        let v = u16::from_le_bytes([buf[4], buf[5]]);
        if v != VERSION {
            return Err(DecodeError::Version(v));
        }
        bincode::deserialize(&buf[6..]).map_err(DecodeError::Body)
    }

    /// The peer a packet claims to be from, where it says.
    pub fn sender(&self) -> Option<PeerId> {
        match self {
            Packet::Beacon { peer, .. }
            | Packet::Hello { peer, .. }
            | Packet::Ping { peer, .. }
            | Packet::Pong { peer, .. }
            | Packet::RttRow { peer, .. }
            | Packet::Elect { peer, .. }
            | Packet::Join { peer, .. }
            | Packet::Command { peer, .. }
            | Packet::Leave { peer }
            | Packet::Heartbeat { peer, .. } => Some(*peer),
            Packet::Relay { from, .. } => Some(*from),
            _ => None,
        }
    }
}

/// Any packet, split into `Chunk`s if it does not fit a datagram, or as
/// itself if it does. For Welcome, whose map may be 65 KiB.
pub fn chunk_packets(p: &Packet, id: u32) -> Vec<Packet> {
    let bytes = p.encode();
    if bytes.len() <= MAX_DATAGRAM {
        return vec![p.clone()];
    }
    let chunk = MAX_DATAGRAM - 64;
    let parts = bytes.chunks(chunk).count() as u16;
    bytes.chunks(chunk).enumerate().map(|(i, c)| Packet::Chunk { id, part: i as u16, parts, bytes: c.to_vec() }).collect()
}

/// Reassembles one chunked packet at a time, by id; a new id starts over.
#[derive(Default)]
pub struct ChunkAssembler {
    id: u32,
    parts: Vec<Option<Vec<u8>>>,
}

impl ChunkAssembler {
    pub fn push(&mut self, id: u32, part: u16, parts: u16, bytes: Vec<u8>) -> Option<Packet> {
        if id != self.id || self.parts.len() != parts as usize {
            self.id = id;
            self.parts = vec![None; parts as usize];
        }
        if let Some(slot) = self.parts.get_mut(part as usize) {
            *slot = Some(bytes);
        }
        if !self.parts.is_empty() && self.parts.iter().all(|p| p.is_some()) {
            let mut all = Vec::new();
            for p in self.parts.drain(..) {
                all.extend(p.unwrap());
            }
            Packet::decode(&all).ok()
        } else {
            None
        }
    }
}

/// An observation on the wire: bincode, then split into MAX_DATAGRAM
/// pieces.
/// A 4-tank, 40-drone observation is a few KiB; the map is not in
/// it (it went in `Welcome`, and craters arrive as events).
pub fn snapshot_packets(obs: &Observation, acked: u32) -> Vec<Packet> {
    let bytes = bincode::serialize(obs).expect("observation serialises");
    let chunk = MAX_DATAGRAM - 64;
    let parts = bytes.chunks(chunk).count().max(1) as u8;
    bytes
        .chunks(chunk)
        .enumerate()
        .map(|(i, c)| Packet::Snapshot { tick: obs.tick, part: i as u8, parts, acked, bytes: c.to_vec() })
        .collect()
}

/// Reassembles snapshot parts by tick. Keeps only the newest tick; a part
/// from an older tick than one already complete is dropped.
#[derive(Default)]
pub struct SnapshotAssembler {
    tick: Tick,
    parts: Vec<Option<Vec<u8>>>,
}

impl SnapshotAssembler {
    pub fn push(&mut self, tick: Tick, part: u8, parts: u8, bytes: Vec<u8>) -> Option<Observation> {
        if tick < self.tick {
            return None;
        }
        if tick > self.tick || self.parts.len() != parts as usize {
            self.tick = tick;
            self.parts = vec![None; parts as usize];
        }
        if let Some(slot) = self.parts.get_mut(part as usize) {
            *slot = Some(bytes);
        }
        if self.parts.iter().all(|p| p.is_some()) {
            let mut all = Vec::new();
            for p in self.parts.drain(..) {
                all.extend(p.unwrap());
            }
            self.parts.clear();
            self.tick = tick + 1;
            bincode::deserialize(&all).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use treadcore_sim::{DroneKind, DroneTask, V2};

    #[test]
    fn round_trip() {
        let p = Packet::Command { peer: 7, seq: 3, cmd: Command::Launch { kind: DroneKind::Recon, task: DroneTask::Orbit { center: V2::new(1.0, 2.0), radius: 3.0 } } };
        let b = p.encode();
        assert_eq!(Packet::decode(&b).unwrap(), p);
        assert!(matches!(Packet::decode(b"xxxxxx"), Err(DecodeError::BadMagic)));
    }

    /// A Welcome with a big map goes in pieces and comes back whole.
    #[test]
    fn big_packets_chunk_and_reassemble() {
        let w = treadcore_sim::World::new(treadcore_sim::Scenario::default());
        let p = Packet::Welcome { player: 3, scenario: w.scenario.clone(), tick: 7, map: w.map.clone() };
        let pieces = chunk_packets(&p, 42);
        assert!(pieces.len() > 10, "a 16 KiB map should take more than ten datagrams, got {}", pieces.len());
        let mut asm = ChunkAssembler::default();
        let mut got = None;
        // Out of order, too.
        let mut order: Vec<usize> = (0..pieces.len()).collect();
        order.reverse();
        for i in order {
            if let Packet::Chunk { id, part, parts, bytes } = &pieces[i] {
                got = asm.push(*id, *part, *parts, bytes.clone()).or(got);
            }
        }
        assert_eq!(got, Some(p));
        // A small packet is not chunked.
        assert_eq!(chunk_packets(&Packet::CommandAck { seq: 1 }, 1).len(), 1);
    }

    #[test]
    fn snapshot_splits_and_reassembles() {

        let w = treadcore_sim::World::new(treadcore_sim::Scenario::default());
        let obs = w.observe(0);
        let pk = snapshot_packets(&obs, 7);
        let mut asm = SnapshotAssembler::default();
        let mut got = None;
        for p in pk {
            if let Packet::Snapshot { tick, part, parts, acked, bytes } = p {
                assert_eq!(acked, 7);
                got = asm.push(tick, part, parts, bytes).or(got);
            }
        }
        assert_eq!(got.unwrap().tick, obs.tick);
    }
}
