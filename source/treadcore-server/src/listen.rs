//! `--listen DIR`: the headless server as a service (roadmap step 8).
//!
//! One process, one port, match after match. It sits in the lobby named
//! by `--game` as a dedicated host: a peer that takes no seat, that the
//! election prefers by `SERVER_PREFERENCE` and drops altogether when a
//! seat is better placed (`treadcore-net::election::elect_among`). When
//! it is elected it hosts the match for the seats, filling the rest with
//! bots as `--net` does, and writes the match's trace and replay to DIR;
//! when it is not, it goes back to the lobby for the next one. With
//! `--introducer` it is listed there as a server under the lobby's name,
//! which is how players across the Internet find it.
//!
//! The status page is plain HTTP on `--http` (TCP; the UDP port's number
//! by default): `/` for a person, `/status.json` for a script, and the
//! files of every match it has hosted, by name. It is read in the same
//! loop as the socket, one request at a time, which is all a status
//! page needs.

use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use treadcore_net::SERVER_PREFERENCE;

/// A seat that has not joined this long after the match started is not
/// coming; with no seat joined at all, the match is given up.
const NO_SHOW: Duration = Duration::from_secs(15);
/// After a lobby that elected someone else, the seats' last packets are
/// still arriving; they are read and dropped this long before the next.
const SETTLE: Duration = Duration::from_secs(2);

/// One hosted match, as the status page lists it.
struct Hosted {
    number: u32,
    ended_unix: u64,
    ticks: Tick,
    seats: Vec<(String, u32, bool)>,
    winner: Option<String>,
    given_up: bool,
    trace: String,
    replay: String,
}

struct Status {
    listener: Option<TcpListener>,
    dir: PathBuf,
    peer: PeerId,
    name: String,
    game: String,
    since: Instant,
    state: &'static str,
    lobbies: u32,
    lost: u32,
    hosted: Vec<Hosted>,
    /// The match in progress: number, tick, (name, score, joined) per seat.
    live: Option<(u32, Tick, Vec<(String, u32, bool)>)>,
}

impl Status {
    fn json(&self) -> serde_json::Value {
        let seats = |v: &[(String, u32, bool)], third: &str| -> Vec<serde_json::Value> { v.iter().map(|(n, s, b)| serde_json::json!({"name": n, "score": s, third: b})).collect() };
        serde_json::json!({
            "peer": format!("{:016x}", self.peer),
            "name": self.name,
            "game": self.game,
            "version": env!("CARGO_PKG_VERSION"),
            "up_seconds": self.since.elapsed().as_secs(),
            "state": self.state,
            "lobbies": self.lobbies,
            "elected_elsewhere": self.lost,
            "preference_ms": SERVER_PREFERENCE / 1000,
            "live": self.live.as_ref().map(|(n, t, s)| serde_json::json!({"match": n, "tick": t, "seconds": *t as f32 / TICK_HZ as f32, "seats": seats(s, "joined")})),
            "hosted": self.hosted.iter().map(|h| serde_json::json!({
                "match": h.number, "ended_unix": h.ended_unix, "ticks": h.ticks, "winner": h.winner,
                "given_up": h.given_up, "trace": h.trace, "replay": h.replay, "seats": seats(&h.seats, "bot"),
            })).collect::<Vec<_>>(),
        })
    }

    fn html(&self) -> String {
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let mut b = String::new();
        b.push_str("<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'><meta http-equiv=refresh content=5>");
        b.push_str(&format!("<title>treadcore server: {}</title>", esc(&self.game)));
        b.push_str("<style>body{font:15px/1.4 ui-monospace,monospace;margin:16px;max-width:60em;background:#fff;color:#111}td,th{padding:0 1em 0 0;text-align:left}@media(prefers-color-scheme:dark){body{background:#111;color:#ddd}a{color:#8cf}}</style>");
        b.push_str(&format!("<h1>{} in lobby '{}'</h1>", esc(&self.name), esc(&self.game)));
        b.push_str(&format!("<p>peer {:016x}, treadcore {}, up {} s. <b>{}</b>. {} lobbies, elected in {}, a seat elected in {}.</p>", self.peer, env!("CARGO_PKG_VERSION"), self.since.elapsed().as_secs(), self.state, self.lobbies, self.hosted.len(), self.lost));
        if let Some((n, t, seats)) = &self.live {
            b.push_str(&format!("<h2>Match {} at {:.0} s</h2><table><tr><th>seat<th>score<th>joined", n, *t as f32 / TICK_HZ as f32));
            for (name, score, joined) in seats {
                b.push_str(&format!("<tr><td>{}<td>{}<td>{}", esc(name), score, if *joined { "yes" } else { "-" }));
            }
            b.push_str("</table>");
        }
        b.push_str("<h2>Hosted</h2>");
        if self.hosted.is_empty() {
            b.push_str("<p>None yet.</p>");
        } else {
            b.push_str("<table><tr><th>match<th>ticks<th>winner<th>files");
            for h in self.hosted.iter().rev() {
                let winner = h.winner.as_deref().map(esc).unwrap_or_else(|| if h.given_up { "given up".into() } else { "-".into() });
                b.push_str(&format!("<tr><td>{}<td>{}<td>{}<td><a href='{}'>trace</a> <a href='{}'>replay</a>", h.number, h.ticks, winner, esc(&h.trace), esc(&h.replay)));
            }
            b.push_str("</table>");
        }
        b.push_str("<p><a href='status.json'>status.json</a></p>");
        b
    }

    /// Answer whoever is waiting, one request each, and return.
    fn poll(&mut self) {
        let Some(l) = self.listener.as_ref() else { return };
        while let Ok((stream, _)) = l.accept() {
            if let Err(e) = self.answer(stream) {
                log::debug!("status page: {}", e);
            }
        }
    }

    fn answer(&self, mut stream: TcpStream) -> std::io::Result<()> {
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        stream.set_write_timeout(Some(Duration::from_millis(200)))?;
        let mut line = String::new();
        BufReader::new(&stream).read_line(&mut line)?;
        let path = line.split_whitespace().nth(1).unwrap_or("/");
        let (code, kind, body): (&str, &str, Vec<u8>) = match path {
            "/" | "/index.html" => ("200 OK", "text/html; charset=utf-8", self.html().into_bytes()),
            "/status.json" => ("200 OK", "application/json", self.json().to_string().into_bytes()),
            p => {
                // Only a file this server wrote and lists; no paths.
                let name = p.trim_start_matches('/');
                let listed = self.hosted.iter().any(|h| h.trace == name || h.replay == name);
                match listed.then(|| std::fs::read(self.dir.join(name))) {
                    Some(Ok(bytes)) => ("200 OK", if name.ends_with(".json") { "application/json" } else { "application/x-ndjson" }, bytes),
                    _ => ("404 Not Found", "text/plain", b"not here\n".to_vec()),
                }
            }
        };
        write!(stream, "HTTP/1.0 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", code, kind, body.len())?;
        stream.write_all(&body)
    }
}

/// The first match number not already in DIR, so a restarted server
/// does not write over the last one's files.
fn first_free(dir: &Path) -> u32 {
    let taken = std::fs::read_dir(dir)
        .map(|d| d.filter_map(|e| e.ok()?.file_name().to_str()?.strip_prefix("match-")?.get(..4)?.parse::<u32>().ok()).max().unwrap_or(0))
        .unwrap_or(0);
    taken + 1
}

pub(super) fn run(args: Args) {
    let dir = PathBuf::from(args.listen.clone().unwrap());
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| die(&format!("listen {}: {}", dir.display(), e)));
    let me = match args.identity.as_deref() {
        Some(path) => identity_from_file(path).unwrap_or_else(|e| die(&format!("identity: {}", e))),
        None => new_peer_id(),
    };
    let mut sock = Socket::bind(args.port).unwrap_or_else(|e| die(&format!("bind: {}", e)));
    let http = args.http.unwrap_or(sock.local.port());
    let listener = match TcpListener::bind(("0.0.0.0", http)).and_then(|l| l.set_nonblocking(true).map(|_| l)) {
        Ok(l) => {
            log::info!("status page on http://{}:{}/", treadcore_net::introducer::lan_address(http).ip(), http);
            Some(l)
        }
        Err(e) => {
            log::error!("status page: tcp port {}: {}; serving none", http, e);
            None
        }
    };
    let mut status = Status { listener, dir: dir.clone(), peer: me, name: args.name.clone(), game: args.game.clone(), since: Instant::now(), state: "listening", lobbies: 0, lost: 0, hosted: vec![], live: None };
    let mut number = first_free(&dir);
    log::info!("server {:016x} '{}' listening in lobby '{}' on {}; matches to {}", me, args.name, args.game, sock.local, dir.display());
    loop {
        let mut cfg = LobbyConfig::new(me, &args.name, &args.game);
        cfg.seeds = args.peers.clone();
        cfg.beacon = args.beacon;
        cfg.introducer = args.introducer.clone();
        cfg.server = true;
        let mut lobby = Lobby::new(cfg);
        status.state = "listening";
        let result = loop {
            if let Some(r) = lobby.poll(&mut sock) {
                break r;
            }
            if lobby.phase != treadcore_net::Phase::Discover {
                status.state = "in a lobby";
            }
            status.poll();
            std::thread::sleep(Duration::from_millis(5));
        };
        status.lobbies += 1;
        if !result.i_host(me) {
            status.lost += 1;
            log::info!("not elected: {:016x} hosts this one; listening again", result.decision.host);
            let settle = Instant::now() + SETTLE;
            while Instant::now() < settle {
                let _ = sock.recv();
                status.poll();
                std::thread::sleep(Duration::from_millis(5));
            }
            continue;
        }

        let stem = format!("match-{:04}", number);
        let (trace_name, replay_name) = (format!("{}.trace.jsonl", stem), format!("{}.replay.json", stem));
        let trace_path = dir.join(&trace_name).to_string_lossy().into_owned();
        let mut trace = Trace::open(Some(&trace_path));
        let routes = routes_from(&result, me);
        let registration = lobby.take_registration();
        status.state = "hosting";
        let started = Instant::now();
        let mut given_up = false;
        let host = host_match(&args, me, sock, &result, routes, registration, &mut trace, |host| {
            let w = &host.world;
            let joined = |i: usize| host.seats.get(i).map(|s| s.joined).unwrap_or(true);
            status.live = Some((number, w.tick, w.players.iter().enumerate().map(|(i, p)| (p.name.clone(), p.score, joined(i))).collect()));
            status.poll();
            if started.elapsed() >= NO_SHOW && !host.seats.is_empty() && host.seats.iter().all(|s| !s.joined) {
                log::warn!("no seat joined in {} s; giving the match up", NO_SHOW.as_secs());
                given_up = true;
                return false;
            }
            true
        });
        drop(trace);
        save_replay(&host, &dir.join(&replay_name).to_string_lossy());
        let w = &host.world;
        let ended_unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let bots: Vec<bool> = w.scenario.players.iter().map(|(_, bot)| *bot).collect();
        status.hosted.push(Hosted {
            number,
            ended_unix,
            ticks: w.tick,
            seats: w.players.iter().enumerate().map(|(i, p)| (p.name.clone(), p.score, bots.get(i).copied().unwrap_or(true))).collect(),
            winner: w.winner.and_then(|id| w.players.iter().find(|p| p.id == id)).map(|p| p.name.clone()),
            given_up,
            trace: trace_name,
            replay: replay_name,
        });
        status.live = None;
        log::info!("match {} done: {} and {} in {}", number, status.hosted.last().unwrap().trace, status.hosted.last().unwrap().replay, dir.display());
        number += 1;
        sock = host.into_socket().expect("the host keeps its socket");
    }
}
