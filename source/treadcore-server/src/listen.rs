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
//! Several `--game`s are several lobbies at once, one thread and one UDP
//! port each (`--port`, then the ports after it), each going round the
//! loop above on its own and writing to DIR/NAME. A lobby is one match
//! at a time, as for any host; a second group wanting a game while the
//! first plays asks in under another name.
//!
//! The status page is plain HTTP on `--http` (TCP; the first UDP port's
//! number by default): `/` for a person, `/status.json` for a script,
//! and the files of every match it has hosted, by name. Its own thread
//! answers one request at a time, which is all a status page needs; the
//! lobbies write what it shows under a lock.

use super::*;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
    /// Paths under DIR: `match-NNNN...` with one lobby, `NAME/match-NNNN...`
    /// with several.
    trace: String,
    replay: String,
}

/// One lobby the server sits in, as the status page shows it.
struct Served {
    game: String,
    port: u16,
    state: &'static str,
    lobbies: u32,
    lost: u32,
    hosted: Vec<Hosted>,
    /// The match in progress: number, tick, (name, score, joined) per seat.
    live: Option<(u32, Tick, Vec<(String, u32, bool)>)>,
}

type Shared = Arc<Mutex<Vec<Served>>>;

/// What every lobby has in common, and the page itself.
struct Page {
    dir: PathBuf,
    peer: PeerId,
    name: String,
    since: Instant,
    served: Shared,
}

fn seats_json(v: &[(String, u32, bool)], third: &str) -> Vec<serde_json::Value> {
    v.iter().map(|(n, s, b)| serde_json::json!({"name": n, "score": s, third: b})).collect()
}

impl Served {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "game": self.game,
            "port": self.port,
            "state": self.state,
            "lobbies": self.lobbies,
            "elected_elsewhere": self.lost,
            "live": self.live.as_ref().map(|(n, t, s)| serde_json::json!({"match": n, "tick": t, "seconds": *t as f32 / TICK_HZ as f32, "seats": seats_json(s, "joined")})),
            "hosted": self.hosted.iter().map(|h| serde_json::json!({
                "match": h.number, "ended_unix": h.ended_unix, "ticks": h.ticks, "winner": h.winner,
                "given_up": h.given_up, "trace": h.trace, "replay": h.replay, "seats": seats_json(&h.seats, "bot"),
            })).collect::<Vec<_>>(),
        })
    }

    fn html(&self, b: &mut String, esc: impl Fn(&str) -> String) {
        b.push_str(&format!("<h2>Lobby '{}'</h2>", esc(&self.game)));
        b.push_str(&format!("<p>udp port {}. <b>{}</b>. {} lobbies, elected in {}, a seat elected in {}.</p>", self.port, self.state, self.lobbies, self.hosted.len(), self.lost));
        if let Some((n, t, seats)) = &self.live {
            b.push_str(&format!("<h3>Match {} at {:.0} s</h3><table><tr><th>seat<th>score<th>joined", n, *t as f32 / TICK_HZ as f32));
            for (name, score, joined) in seats {
                b.push_str(&format!("<tr><td>{}<td>{}<td>{}", esc(name), score, if *joined { "yes" } else { "-" }));
            }
            b.push_str("</table>");
        }
        b.push_str("<h3>Hosted</h3>");
        if self.hosted.is_empty() {
            b.push_str("<p>None yet.</p>");
        } else {
            b.push_str("<table><tr><th>match<th>ticks<th>winner<th>files");
            for h in self.hosted.iter().rev() {
                let winner = h.winner.as_deref().map(&esc).unwrap_or_else(|| if h.given_up { "given up".into() } else { "-".into() });
                b.push_str(&format!("<tr><td>{}<td>{}<td>{}<td><a href='{}'>trace</a> <a href='{}'>replay</a>", h.number, h.ticks, winner, esc(&h.trace), esc(&h.replay)));
            }
            b.push_str("</table>");
        }
    }
}

impl Page {
    /// One lobby's fields sit at the top as they always have; every
    /// lobby, that one too, is in `games`.
    fn json(&self) -> serde_json::Value {
        let served = self.served.lock().unwrap();
        let mut top = serde_json::json!({
            "peer": format!("{:016x}", self.peer),
            "name": self.name,
            "version": env!("CARGO_PKG_VERSION"),
            "up_seconds": self.since.elapsed().as_secs(),
            "preference_ms": SERVER_PREFERENCE / 1000,
            "games": served.iter().map(Served::json).collect::<Vec<_>>(),
        });
        if let [one] = served.as_slice() {
            if let (Some(t), serde_json::Value::Object(o)) = (top.as_object_mut(), one.json()) {
                t.extend(o.into_iter().filter(|(k, _)| k != "port"));
            }
        }
        top
    }

    fn html(&self) -> String {
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        let served = self.served.lock().unwrap();
        let games = served.iter().map(|s| format!("'{}'", esc(&s.game))).collect::<Vec<_>>().join(", ");
        let mut b = String::new();
        b.push_str("<!doctype html><meta charset=utf-8><meta name=viewport content='width=device-width'><meta http-equiv=refresh content=5>");
        b.push_str(&format!("<title>treadcore server: {}</title>", served.iter().map(|s| esc(&s.game)).collect::<Vec<_>>().join(", ")));
        b.push_str("<style>body{font:15px/1.4 ui-monospace,monospace;margin:16px;max-width:60em;background:#fff;color:#111}td,th{padding:0 1em 0 0;text-align:left}@media(prefers-color-scheme:dark){body{background:#111;color:#ddd}a{color:#8cf}}</style>");
        b.push_str(&format!("<h1>{} in {} {}</h1>", esc(&self.name), if served.len() == 1 { "lobby" } else { "lobbies" }, games));
        b.push_str(&format!("<p>peer {:016x}, treadcore {}, up {} s.</p>", self.peer, env!("CARGO_PKG_VERSION"), self.since.elapsed().as_secs()));
        for s in served.iter() {
            s.html(&mut b, esc);
        }
        b.push_str("<p><a href='status.json'>status.json</a></p>");
        b
    }

    /// Answer requests until the process ends, one at a time.
    fn serve(&self, listener: TcpListener) {
        for stream in listener.incoming() {
            match stream.and_then(|s| self.answer(s)) {
                Ok(()) => {}
                Err(e) => log::debug!("status page: {}", e),
            }
        }
    }

    fn answer(&self, mut stream: TcpStream) -> std::io::Result<()> {
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
                let listed = self.served.lock().unwrap().iter().any(|s| s.hosted.iter().any(|h| h.trace == name || h.replay == name));
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
    // Every socket first, so the status page can say where each lobby is.
    let several = args.games.len() > 1;
    let mut socks = Vec::new();
    for (i, game) in args.games.iter().enumerate() {
        let port = args.port.checked_add(i as u16).unwrap_or_else(|| usage("--port leaves no room for that many lobbies"));
        let sock = Socket::bind(port).unwrap_or_else(|e| die(&format!("bind: {}", e)));
        let files = if several { dir.join(game) } else { dir.clone() };
        std::fs::create_dir_all(&files).unwrap_or_else(|e| die(&format!("listen {}: {}", files.display(), e)));
        socks.push(sock);
    }
    let served: Shared = Arc::new(Mutex::new(
        args.games.iter().zip(&socks).map(|(g, s)| Served { game: g.clone(), port: s.local.port(), state: "listening", lobbies: 0, lost: 0, hosted: vec![], live: None }).collect(),
    ));
    let http = args.http.unwrap_or(socks[0].local.port());
    let page = Page { dir: dir.clone(), peer: me, name: args.name.clone(), since: Instant::now(), served: served.clone() };
    std::thread::scope(|scope| {
        match TcpListener::bind(("0.0.0.0", http)) {
            Ok(l) => {
                log::info!("status page on http://{}:{}/", treadcore_net::introducer::lan_address(http).ip(), http);
                let page = &page;
                std::thread::Builder::new().name("status".into()).spawn_scoped(scope, move || page.serve(l)).expect("a thread for the status page");
            }
            Err(e) => log::error!("status page: tcp port {}: {}; serving none", http, e),
        }
        for (i, sock) in socks.into_iter().enumerate() {
            let (args, dir, served) = (&args, &dir, &served);
            std::thread::Builder::new().name(args.games[i].clone()).spawn_scoped(scope, move || serve_lobby(args, me, i, sock, dir, served, several)).expect("a thread for the lobby");
        }
    });
}

/// One lobby, match after match: the server's whole life when it serves
/// one, a thread of it when several.
fn serve_lobby(args: &Args, me: PeerId, i: usize, mut sock: Socket, dir: &Path, served: &Shared, several: bool) {
    let game = &args.games[i];
    let (files, prefix) = if several { (dir.join(game), format!("{}/", game)) } else { (dir.to_path_buf(), String::new()) };
    let set = |f: &dyn Fn(&mut Served)| f(&mut served.lock().unwrap()[i]);
    let mut number = first_free(&files);
    log::info!("server {:016x} '{}' listening in lobby '{}' on {}; matches to {}", me, args.name, game, sock.local, files.display());
    loop {
        let mut cfg = LobbyConfig::new(me, &args.name, game);
        cfg.seeds = args.peers.clone();
        cfg.beacon = args.beacon;
        cfg.introducer = args.introducer.clone();
        cfg.server = true;
        let mut lobby = Lobby::new(cfg);
        set(&|s| s.state = "listening");
        let result = loop {
            if let Some(r) = lobby.poll(&mut sock) {
                break r;
            }
            if lobby.phase != treadcore_net::Phase::Discover {
                set(&|s| s.state = "in a lobby");
            }
            std::thread::sleep(Duration::from_millis(5));
        };
        set(&|s| s.lobbies += 1);
        if !result.i_host(me) {
            set(&|s| s.lost += 1);
            log::info!("'{}': not elected: {:016x} hosts this one; listening again", game, result.decision.host);
            let settle = Instant::now() + SETTLE;
            while Instant::now() < settle {
                let _ = sock.recv();
                std::thread::sleep(Duration::from_millis(5));
            }
            continue;
        }

        let stem = format!("match-{:04}", number);
        let (trace_name, replay_name) = (format!("{}.trace.jsonl", stem), format!("{}.replay.json", stem));
        let trace_path = files.join(&trace_name).to_string_lossy().into_owned();
        let mut trace = Trace::open(Some(&trace_path));
        let routes = routes_from(&result, me);
        let registration = lobby.take_registration();
        set(&|s| s.state = "hosting");
        let started = Instant::now();
        let mut given_up = false;
        let host = host_match(args, me, sock, &result, routes, registration, &mut trace, |host| {
            let w = &host.world;
            let joined = |i: usize| host.seats.get(i).map(|s| s.joined).unwrap_or(true);
            let live = (number, w.tick, w.players.iter().enumerate().map(|(i, p)| (p.name.clone(), p.score, joined(i))).collect());
            served.lock().unwrap()[i].live = Some(live);
            if started.elapsed() >= NO_SHOW && !host.seats.is_empty() && host.seats.iter().all(|s| !s.joined) {
                log::warn!("'{}': no seat joined in {} s; giving the match up", game, NO_SHOW.as_secs());
                given_up = true;
                return false;
            }
            true
        });
        drop(trace);
        save_replay(&host, &files.join(&replay_name).to_string_lossy());
        let w = &host.world;
        let ended_unix = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let bots: Vec<bool> = w.scenario.players.iter().map(|(_, bot)| *bot).collect();
        let hosted = Hosted {
            number,
            ended_unix,
            ticks: w.tick,
            seats: w.players.iter().enumerate().map(|(i, p)| (p.name.clone(), p.score, bots.get(i).copied().unwrap_or(true))).collect(),
            winner: w.winner.and_then(|id| w.players.iter().find(|p| p.id == id)).map(|p| p.name.clone()),
            given_up,
            trace: format!("{}{}", prefix, trace_name),
            replay: format!("{}{}", prefix, replay_name),
        };
        log::info!("'{}': match {} done: {} and {} in {}", game, number, trace_name, replay_name, files.display());
        {
            let mut all = served.lock().unwrap();
            all[i].hosted.push(hosted);
            all[i].live = None;
        }
        number += 1;
        sock = host.into_socket().expect("the host keeps its socket");
    }
}
