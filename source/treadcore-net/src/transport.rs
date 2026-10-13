//! One UDP socket, non-blocking, broadcast-capable, speaking Packet.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use treadcore_proto::{Packet, PORT};

pub use treadcore_proto::PeerId;

pub struct Socket {
    sock: UdpSocket,
    pub local: SocketAddr,
    pub sent: u64,
    pub received: u64,
    pub dropped: u64,
}

impl Socket {
    /// Bind to `port`, or to any free port if that one is taken (so four
    /// instances on one machine, for testing, do not fight).
    pub fn bind(port: u16) -> io::Result<Socket> {
        let sock = match UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)) {
            Ok(s) => s,
            Err(e) if e.kind() == io::ErrorKind::AddrInUse => {
                log::warn!("port {} in use, binding to any free port", port);
                UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))?
            }
            Err(e) => return Err(e),
        };
        sock.set_nonblocking(true)?;
        sock.set_broadcast(true)?;
        let local = sock.local_addr()?;
        log::info!("udp bound {}", local);
        Ok(Socket { sock, local, sent: 0, received: 0, dropped: 0 })
    }

    pub fn send(&mut self, to: SocketAddr, p: &Packet) {
        let bytes = p.encode();
        match self.sock.send_to(&bytes, to) {
            Ok(_) => self.sent += 1,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => self.dropped += 1,
            Err(e) => {
                self.dropped += 1;
                log::debug!("send to {} failed: {}", to, e);
            }
        }
    }

    /// To the LAN. Every process on the segment listening on PORT hears it.
    pub fn broadcast(&mut self, p: &Packet) {
        self.send(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, PORT)), p);
        // And loopback, for instances on this machine on other ports that
        // broadcast does not reach on some stacks.
        self.send(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, PORT)), p);
    }

    /// Everything waiting, decoded. Undecodable datagrams are counted and
    /// logged at trace, never returned.
    ///
    /// The buffer is the UDP maximum, not a "reasonable" size: `Welcome`
    /// carries the 16 KiB map in one datagram, and a UDP read into a
    /// buffer that is too small silently truncates the datagram, which
    /// then fails to decode. With a 2 KiB buffer no client ever joined;
    /// the loopback session test is what found it.
    pub fn recv(&mut self) -> Vec<(SocketAddr, Packet)> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 65536];

        for _ in 0..256 {
            match self.sock.recv_from(&mut buf) {
                Ok((n, from)) => match Packet::decode(&buf[..n]) {
                    Ok(p) => {
                        self.received += 1;
                        out.push((from, p));
                    }
                    Err(e) => {
                        self.dropped += 1;
                        log::trace!("dropped datagram from {}: {}", from, e);
                    }
                },
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => {
                    log::trace!("recv error: {}", e);
                    break;
                }
            }
        }
        out
    }
}

impl Socket {
    /// This socket's address with the wildcard replaced by loopback, so
    /// another socket on this machine can send to it.
    pub fn local_with_loopback(&self) -> SocketAddr {
        match self.local {
            SocketAddr::V4(a) if a.ip().is_unspecified() => SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, a.port())),
            a => a,
        }
    }
}
