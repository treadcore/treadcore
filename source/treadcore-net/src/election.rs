//! Who hosts, and how everyone reaches them. A pure function of the
//! latency matrix, so every peer that holds the same matrix reaches the
//! same answer without exchanging another byte.
//!
//! The rule: for each candidate, run Dijkstra from it over the measured
//! links. Candidates that cannot reach every peer (even by relay) are
//! out. Of the rest, the winner has the smallest worst-case path latency
//! to any peer; ties go to the smaller total, then the smaller id. The
//! Dijkstra tree from the winner is the route: each peer's next hop
//! toward the host is its parent in that tree, which is the host itself
//! when the link is direct.
//!
//! A dedicated server (`treadcore-server --listen`, roadmap step 8) is a
//! candidate with a head start, `SERVER_PREFERENCE` off its worst path,
//! and only ever the host: it takes no seat, so it is never a relay and
//! never in a tree but its own. Each server is weighed over the seats
//! plus itself, every seat over the seats alone, and the least key
//! wins. A server that loses is out of the match altogether.

use std::collections::{BTreeMap, BTreeSet};
use treadcore_proto::PeerId;

pub type Row = Vec<(PeerId, Option<u32>)>;

/// How much nearer than it measures a dedicated server counts, in
/// microseconds: within 10 ms of the best seat, the server hosts. It
/// runs nothing else and never leaves mid-match, which a seat may.
pub const SERVER_PREFERENCE: u32 = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decision {
    pub host: PeerId,
    /// (peer, next hop toward host) for every peer that is not the host.
    pub route: Vec<(PeerId, PeerId)>,
    /// Worst path latency to the host, microseconds.
    pub eccentricity_us: u32,
}

/// Symmetric link cost from whichever rows mention it: the mean of the two
/// directions if both measured, the one that was if only one, none if
/// neither.
fn link(rows: &BTreeMap<PeerId, Row>, a: PeerId, b: PeerId) -> Option<u32> {
    let ab = rows.get(&a).and_then(|r| r.iter().find(|(p, _)| *p == b)).and_then(|(_, v)| *v);
    let ba = rows.get(&b).and_then(|r| r.iter().find(|(p, _)| *p == a)).and_then(|(_, v)| *v);
    match (ab, ba) {
        (Some(x), Some(y)) => Some((x + y) / 2),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// Per peer: its distance from the root, and the neighbour it goes through.
type Tree = BTreeMap<PeerId, (u32, PeerId)>;

fn dijkstra(peers: &BTreeSet<PeerId>, rows: &BTreeMap<PeerId, Row>, from: PeerId) -> Tree {
    // (distance, parent) for each reached peer.
    let mut done: BTreeMap<PeerId, (u32, PeerId)> = BTreeMap::new();
    let mut open: BTreeMap<PeerId, (u32, PeerId)> = BTreeMap::new();
    open.insert(from, (0, from));
    while let Some((&u, &(du, pu))) = open.iter().min_by_key(|(id, (d, _))| (*d, **id)) {
        open.remove(&u);
        done.insert(u, (du, pu));
        for &v in peers {
            if done.contains_key(&v) {
                continue;
            }
            if let Some(w) = link(rows, u, v) {
                let dv = du + w;
                let better = open.get(&v).map(|(d, _)| dv < *d).unwrap_or(true);
                if better {
                    open.insert(v, (dv, u));
                }
            }
        }
    }
    done
}

pub fn elect(peers: &[PeerId], rows: &BTreeMap<PeerId, Row>) -> Option<Decision> {
    elect_among(peers, &[], rows)
}

/// `elect`, where `servers` (a subset of `peers`) are dedicated hosts.
pub fn elect_among(peers: &[PeerId], servers: &[PeerId], rows: &BTreeMap<PeerId, Row>) -> Option<Decision> {
    let set: BTreeSet<PeerId> = peers.iter().copied().collect();
    let servers: BTreeSet<PeerId> = servers.iter().copied().filter(|s| set.contains(s)).collect();
    let seats: BTreeSet<PeerId> = set.difference(&servers).copied().collect();
    if set.is_empty() {
        return None;
    }
    if set.len() == 1 {
        let h = *set.iter().next().unwrap();
        return Some(Decision { host: h, route: vec![], eccentricity_us: 0 });
    }
    // (key, eccentricity, host, tree); the key is the eccentricity less
    // any preference, then the total, then the id.
    let mut best: Option<((u32, u64, PeerId), u32, Tree)> = None;
    let mut weigh = |cand: PeerId, among: &BTreeSet<PeerId>, preference: u32| {
        let tree = dijkstra(among, rows, cand);
        if tree.len() != among.len() {
            return;
        }
        let ecc = tree.values().map(|(d, _)| *d).max().unwrap_or(0);
        let total: u64 = tree.values().map(|(d, _)| *d as u64).sum();
        let key = (ecc.saturating_sub(preference), total, cand);
        if best.as_ref().map(|(k, _, _)| key < *k).unwrap_or(true) {
            best = Some((key, ecc, tree));
        }
    };
    for &s in &servers {
        let mut among = seats.clone();
        among.insert(s);
        weigh(s, &among, SERVER_PREFERENCE);
    }
    for &cand in &seats {
        weigh(cand, &seats, 0);
    }
    let ((_, _, host), ecc, tree) = best?;
    let route = tree.iter().filter(|(p, _)| **p != host).map(|(p, (_, parent))| (*p, *parent)).collect();
    Some(Decision { host, route, eccentricity_us: ecc })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(edges: &[(PeerId, PeerId, u32)]) -> BTreeMap<PeerId, Row> {
        let mut m: BTreeMap<PeerId, Row> = BTreeMap::new();
        for &(a, b, w) in edges {
            m.entry(a).or_default().push((b, Some(w)));
            m.entry(b).or_default().push((a, Some(w)));
        }
        m
    }

    #[test]
    fn central_peer_hosts() {
        // 1 is 10ms from everyone; the others are 20ms apart.
        let r = rows(&[(1, 2, 10_000), (1, 3, 10_000), (1, 4, 10_000), (2, 3, 20_000), (3, 4, 20_000), (2, 4, 20_000)]);
        let d = elect(&[1, 2, 3, 4], &r).unwrap();
        assert_eq!(d.host, 1);
        assert_eq!(d.eccentricity_us, 10_000);
        assert!(d.route.iter().all(|(_, via)| *via == 1));
    }

    #[test]
    fn unreachable_peer_routes_through_neighbour() {
        // 4 can only see 3. The host must still reach 4, via 3.
        let r = rows(&[(1, 2, 5_000), (1, 3, 5_000), (2, 3, 5_000), (3, 4, 5_000)]);
        let d = elect(&[1, 2, 3, 4], &r).unwrap();
        assert_eq!(d.host, 3);
        let via4 = d.route.iter().find(|(p, _)| *p == 4).unwrap().1;
        assert_eq!(via4, 3);
        // And if 3 hosts, 1 and 2 are direct.
        assert_eq!(d.route.iter().find(|(p, _)| *p == 1).unwrap().1, 3);
    }

    #[test]
    fn relay_needed_when_host_cannot_see_peer() {
        // A line: 1-2-3-4. Host is 2 or 3 (ecc 2 hops); 2 wins on id.
        let r = rows(&[(1, 2, 1000), (2, 3, 1000), (3, 4, 1000)]);
        let d = elect(&[1, 2, 3, 4], &r).unwrap();
        assert_eq!(d.host, 2);
        assert_eq!(d.route.iter().find(|(p, _)| *p == 4).unwrap().1, 3);
    }

    /// A server with the best links hosts, with every seat routed to
    /// it; a server within the preference of the best seat hosts too.
    #[test]
    fn server_with_best_links_hosts() {
        // 9 is the server, 5 ms from each seat; the seats are 20 ms apart.
        let r = rows(&[(9, 1, 5_000), (9, 2, 5_000), (9, 3, 5_000), (1, 2, 20_000), (1, 3, 20_000), (2, 3, 20_000)]);
        let d = elect_among(&[1, 2, 3, 9], &[9], &r).unwrap();
        assert_eq!(d.host, 9);
        assert_eq!(d.eccentricity_us, 5_000);
        assert_eq!(d.route.len(), 3);
        // Seats 2 ms apart, the server 8 ms from each: it still hosts.
        let r = rows(&[(9, 1, 8_000), (9, 2, 8_000), (9, 3, 8_000), (1, 2, 2_000), (1, 3, 2_000), (2, 3, 2_000)]);
        assert_eq!(elect_among(&[1, 2, 3, 9], &[9], &r).unwrap().host, 9);
    }

    /// A server further than the preference loses, and then it is out:
    /// not in the route, not a relay, even where it is the shortcut.
    #[test]
    fn far_server_is_ignored_and_never_relays() {
        let r = rows(&[(9, 1, 50_000), (9, 2, 50_000), (9, 3, 50_000), (1, 2, 2_000), (1, 3, 2_000), (2, 3, 2_000)]);
        let d = elect_among(&[1, 2, 3, 9], &[9], &r).unwrap();
        assert_ne!(d.host, 9);
        assert!(d.route.iter().all(|(p, via)| *p != 9 && *via != 9));
        // 1 and 3 see each other only through 2 or the server; 2 hosts
        // and the route goes through no server.
        let r = rows(&[(9, 1, 1_000), (9, 3, 1_000), (1, 2, 30_000), (2, 3, 30_000)]);
        let d = elect_among(&[1, 2, 3, 9], &[9], &r).unwrap();
        assert_eq!(d.host, 9, "the server reaches both ends; it hosts");
        let r = rows(&[(9, 1, 1_000), (1, 2, 30_000), (2, 3, 30_000)]);
        let d = elect_among(&[1, 2, 3, 9], &[9], &r).unwrap();
        assert_eq!(d.host, 2, "the server reaches 3 only through two seats; the seats elect among themselves");
        assert!(d.route.iter().all(|(p, via)| *p != 9 && *via != 9));
    }

    /// Two servers: the nearer hosts, the other is out of the route.
    #[test]
    fn of_two_servers_one_hosts() {
        let r = rows(&[(8, 1, 20_000), (8, 2, 20_000), (9, 1, 4_000), (9, 2, 4_000), (8, 9, 1_000), (1, 2, 30_000)]);
        let d = elect_among(&[1, 2, 8, 9], &[8, 9], &r).unwrap();
        assert_eq!(d.host, 9);
        assert!(d.route.iter().all(|(p, via)| *p != 8 && *via != 8));
    }

    #[test]
    fn solo() {
        let d = elect(&[9], &BTreeMap::new()).unwrap();
        assert_eq!(d.host, 9);
        assert!(d.route.is_empty());
    }
}
