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

use std::collections::{BTreeMap, BTreeSet};
use treadcore_proto::PeerId;

pub type Row = Vec<(PeerId, Option<u32>)>;

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

fn dijkstra(peers: &BTreeSet<PeerId>, rows: &BTreeMap<PeerId, Row>, from: PeerId) -> BTreeMap<PeerId, (u32, PeerId)> {
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
    let set: BTreeSet<PeerId> = peers.iter().copied().collect();
    if set.is_empty() {
        return None;
    }
    if set.len() == 1 {
        let h = *set.iter().next().unwrap();
        return Some(Decision { host: h, route: vec![], eccentricity_us: 0 });
    }
    let mut best: Option<(u32, u64, PeerId, BTreeMap<PeerId, (u32, PeerId)>)> = None;
    for &cand in &set {
        let tree = dijkstra(&set, rows, cand);
        if tree.len() != set.len() {
            continue;
        }
        let ecc = tree.values().map(|(d, _)| *d).max().unwrap_or(0);
        let total: u64 = tree.values().map(|(d, _)| *d as u64).sum();
        let key = (ecc, total, cand);
        let replace = match &best {
            None => true,
            Some((e, t, c, _)) => key < (*e, *t, *c),
        };
        if replace {
            best = Some((ecc, total, cand, tree));
        }
    }
    let (ecc, _, host, tree) = best?;
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

    #[test]
    fn solo() {
        let d = elect(&[9], &BTreeMap::new()).unwrap();
        assert_eq!(d.host, 9);
        assert!(d.route.is_empty());
    }
}
