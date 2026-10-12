//! treadcore-net -- sockets, discovery, negotiation, and the session.
//!
//! The question this crate answers is the one the original Bolo answered
//! with a ring: four computers, no server anybody set up, who runs the
//! game? Here:
//!
//!   1. transport  one non-blocking UDP socket per process, broadcast
//!                 enabled, framing by treadcore-proto.
//!   2. lobby      find peers (LAN beacon, or addresses given by hand),
//!                 ping every one of them, publish a row of the latency
//!                 matrix, and then -- every peer independently, from the
//!                 same matrix -- elect the host and build the route.
//!   3. election   the pure function the lobby calls. The host is the
//!                 peer that minimises the worst latency anyone has to
//!                 it; the route is the shortest-path tree from it. Peers
//!                 the host cannot reach directly send through the
//!                 neighbour that can: a connectivity route.
//!   4. session    Host and Client. The host owns the world and ticks it;
//!                 clients send orders and receive what they may see.
//!
//! Nothing in here spawns a thread. Everything is `poll()` from whoever
//! owns the loop -- the server's main, or a Bevy system -- which keeps the
//! simulation's clock the only clock.

pub mod election;
pub mod lobby;
pub mod session;
pub mod transport;

pub use election::{elect, Decision};
pub use lobby::{Lobby, LobbyConfig, LobbyResult, Phase};
pub use session::{Client, ClientState, Host, Seat};
pub use transport::{Socket, PeerId};

/// A fresh peer id: 64 random-enough bits from the hasher's per-process
/// seed, the clock, and the pid. Not cryptographic; just unlikely to
/// collide across four machines on a LAN.
pub fn new_peer_id() -> PeerId {
    use std::hash::{BuildHasher, Hash, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    std::time::SystemTime::now().hash(&mut h);
    std::process::id().hash(&mut h);
    let v = h.finish();
    if v == 0 { 1 } else { v }
}
