//! treadcore-server -- the headless end of the game.
//!
//! Three jobs, picked by flags:
//!
//!   match    `--local`: four bots, no socket, as fast as the CPU goes
//!            (or `--realtime`), every event logged. This is what `make
//!            debug` runs: a full game's trace in a few seconds, for the
//!            log to be read by a person or an AI.
//!   host     `--net`: run the lobby, and if elected, host the match for
//!            whoever joined, filling empty seats with bots.
//!   seat     `--net` and not elected: join the elected host as a bot
//!            player. Four of these on one machine is a lobby test.
//!
//! No argument parser crate; the flags are few and the loop is explicit
//! so the trace reads top to bottom.

use std::time::{Duration, Instant};
use treadcore_brain::{run_bots, Brain, JsonBrain, RuleBot};
use treadcore_net::session::RouteTable;
use treadcore_net::{new_peer_id, Client, ClientState, Host, Lobby, LobbyConfig, Socket};
use treadcore_proto::PORT;
use treadcore_sim::*;

struct Args {
    net: bool,
    realtime: bool,
    seed: u64,
    ticks: Tick,
    name: String,
    game: String,
    peers: Vec<String>,
    want: usize,
    port: u16,
    brains: Vec<String>,
    swarm: u32,
    trace: Option<String>,
    schema: bool,
    players: usize,
}

fn parse() -> Args {
    let mut a = Args {
        net: false,
        realtime: false,
        seed: 0x7EAD_C0DE,
        ticks: 0,
        name: "server".into(),
        game: "treadcore".into(),
        peers: vec![],
        want: 4,
        port: PORT,
        brains: vec![],
        swarm: 45 * TICK_HZ,
        trace: None,
        schema: false,
        players: MAX_PLAYERS,
    };
    let mut it = std::env::args().skip(1);
    while let Some(f) = it.next() {
        let mut val = || it.next().unwrap_or_else(|| die(&format!("{} needs a value", f)));
        match f.as_str() {
            "--net" => a.net = true,
            "--local" => a.net = false,
            "--realtime" => a.realtime = true,
            "--seed" => a.seed = parse_num(&val()),
            "--ticks" => a.ticks = parse_num(&val()),
            "--seconds" => a.ticks = parse_num::<u64>(&val()) * TICK_HZ as u64,
            "--name" => a.name = val(),
            "--game" => a.game = val(),
            "--peer" => a.peers.push(val()),
            "--want" => a.want = parse_num(&val()),
            "--port" => a.port = parse_num(&val()),
            "--brain" => a.brains.push(val()),
            "--swarm" => a.swarm = parse_num::<u32>(&val()) * TICK_HZ,
            "--trace" => a.trace = Some(val()),
            "--brain-schema" => a.schema = true,
            "--players" => a.players = parse_num::<usize>(&val()).clamp(1, MAX_PLAYERS),
            "-h" | "--help" => {
                println!("{}", HELP);
                std::process::exit(0);
            }
            other => die(&format!("unknown flag {}\n{}", other, HELP)),
        }
    }
    a
}

const HELP: &str = "treadcore-server -- headless host, bot match, or bot seat

  --local            bot-only match in this process (default)
  --net              run the lobby; host if elected, else join as a bot
  --realtime         tick at 20 Hz instead of flat out (--net always does)
  --seed N           map and RNG seed (default 0x7EADC0DE)
  --seconds N        stop the match after N game seconds (default 15 min)
  --ticks N          the same, in ticks
  --swarm N          neutral FPV swarm every N seconds; 0 for none (45)
  --players N        seats in a --local match or a hosted one, 1 to 16 (16)
  --name NAME        this peer's name (server)
  --game NAME        lobby name; only peers in the same lobby meet
  --peer HOST:PORT   greet this address as well as the LAN broadcast
  --want N           how many peers to wait for before deciding (4)
  --port N           UDP port to bind (47296; falls back to any free)
  --brain 'CMD ...'  run an external JSON brain in one seat (repeatable)
  --trace FILE       write every event as JSON lines to FILE
  --brain-schema     print example observation and commands, then exit

Logging is env_logger: RUST_LOG=trace for everything, debug for per-second
summaries, info (default) for events.";

fn die(msg: &str) -> ! {
    eprintln!("treadcore-server: {}", msg);
    std::process::exit(2)
}

fn parse_num<T: std::str::FromStr>(s: &str) -> T {
    let s = s.trim();
    if let Some(h) = s.strip_prefix("0x") {
        if let Ok(v) = u64::from_str_radix(h, 16) {
            if let Ok(t) = v.to_string().parse::<T>() {
                return t;
            }
        }
    }
    s.parse::<T>().unwrap_or_else(|_| die(&format!("bad number '{}'", s)))
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).format_timestamp_millis().init();
    let args = parse();
    if args.schema {
        print!("{}", treadcore_brain::schema_examples());
        return;
    }
    log::info!("treadcore-server {} ({} Hz, {}x{} map)", env!("CARGO_PKG_VERSION"), TICK_HZ, MAP_W, MAP_H);
    if args.net {
        run_net(args);
    } else {
        run_local(args);
    }
}

fn make_brain(spec: Option<&String>, seat: PlayerId, seed: u64) -> Box<dyn Brain> {
    match spec {
        Some(s) => {
            let mut parts = s.split_whitespace();
            let prog = parts.next().unwrap_or("");
            let rest: Vec<String> = parts.map(|x| x.to_string()).collect();
            match JsonBrain::spawn(&format!("json:{}", prog), prog, &rest) {
                Ok(b) => Box::new(b),
                Err(e) => {
                    log::error!("brain '{}' failed to start ({}); seat {} gets the rule bot", s, e, seat);
                    Box::new(RuleBot::new(seed ^ seat as u64, "rulebot"))
                }
            }
        }
        None => Box::new(RuleBot::new(seed.wrapping_mul(31) ^ (seat as u64 + 1), "rulebot")),
    }
}

struct Trace(Option<std::io::BufWriter<std::fs::File>>);

impl Trace {
    fn open(path: Option<&String>) -> Trace {
        Trace(path.map(|p| std::io::BufWriter::new(std::fs::File::create(p).unwrap_or_else(|e| die(&format!("trace {}: {}", p, e))))))
    }
    fn write(&mut self, tick: Tick, events: &[Event]) {
        use std::io::Write;
        if let Some(f) = self.0.as_mut() {
            for e in events {
                let _ = writeln!(f, "{{\"tick\":{},\"event\":{}}}", tick, serde_json::to_string(e).unwrap());
            }
        }
    }
}

/// The per-tick narration. Every event at info, so a trace of a match is
/// a readable story; the per-second state at debug, so a reader can see
/// the shape of a fight without the events.
fn narrate(world: &World, events: &[Event]) {
    let t = world.tick;
    let s = world.seconds();
    for e in events {
        match e {
            Event::Rejected { player, cmd, why } => log::debug!("[{:>6} {:>6.1}s] p{} rejected {:?}: {}", t, s, player, cmd, why),
            Event::ShellLaunched { owner, kind, spotted, eta_ticks, .. } => log::info!("[{:>6} {:>6.1}s] p{} fires {:?} {} eta {:.1}s", t, s, owner, kind, if *spotted { "SPOTTED" } else { "blind" }, *eta_ticks as f32 * DT),
            Event::ShellImpact { at, kind, hits, .. } => log::info!("[{:>6} {:>6.1}s] {:?} impact at ({:.0},{:.0}) hits {:?}", t, s, kind, at.x, at.y, hits),
            Event::DroneLaunched { owner, kind, drone } => log::info!("[{:>6} {:>6.1}s] p{} launches {:?} #{}", t, s, owner, kind, drone),
            Event::DroneLost { owner, kind, why, drone } => log::info!("[{:>6} {:>6.1}s] p{} loses {:?} #{} ({:?})", t, s, owner, kind, drone, why),
            Event::DroneRecovered { owner, kind, .. } => log::debug!("[{:>6} {:>6.1}s] p{} recovers {:?}", t, s, owner, kind),
            Event::Intercepted { owner, target, .. } => log::info!("[{:>6} {:>6.1}s] p{} INTERCEPT kills drone #{}", t, s, owner, target),
            Event::InterceptMissed { target, .. } => log::info!("[{:>6} {:>6.1}s] intercept missed drone #{}", t, s, target),
            Event::StrikeHit { tank, damage, .. } => log::info!("[{:>6} {:>6.1}s] STRIKE hits tank #{} for {}", t, s, tank, damage),
            Event::TankDamaged { tank, by, damage, armor_left } => log::info!("[{:>6} {:>6.1}s] tank #{} -{:.0} armor ({:.0} left) by {:?}", t, s, tank, damage, armor_left, by),
            Event::TankDestroyed { owner, by, .. } => log::warn!("[{:>6} {:>6.1}s] p{} DESTROYED by {:?}", t, s, owner, by),
            Event::DepotCaptured { depot, by } => log::info!("[{:>6} {:>6.1}s] p{} captures depot #{}", t, s, by, depot),
            Event::Resupplied { tank, depot } => log::debug!("[{:>6} {:>6.1}s] tank #{} resupplying at depot #{}", t, s, tank, depot),
            Event::SwarmInbound { count, from } => log::warn!("[{:>6} {:>6.1}s] SWARM {} inbound from ({:.0},{:.0})", t, s, count, from.x, from.y),
            Event::LinkLost { owner, drone } => log::info!("[{:>6} {:>6.1}s] p{} link lost to #{}", t, s, owner, drone),
            Event::Chat { player, text } => log::info!("[{:>6} {:>6.1}s] p{}: {}", t, s, player, text),
            Event::MatchOver { winner } => log::warn!("[{:>6} {:>6.1}s] MATCH OVER winner {:?}", t, s, winner),
        }
    }
    if t % TICK_HZ as Tick == 0 {
        for p in &world.players {
            if let Some(tk) = world.tank_of(p.id) {
                let air: Vec<String> = world.drones.iter().filter(|d| d.owner == p.id).map(|d| format!("{}{:.0}%", match d.kind { DroneKind::Recon => "R", DroneKind::Interceptor => "I", DroneKind::Strike => "S" }, d.link * 100.0)).collect();
                log::debug!(
                    "[{:>6} {:>6.1}s] p{} {:<6} {} at ({:>5.1},{:>5.1}) armor {:>3.0} shells {:>2} fuel {:>3.0} bay R{}/I{}/S{} bubble {:.0} air [{}] score {}",
                    t, s, p.id, p.name, if tk.alive { "ok  " } else { "DEAD" }, tk.pos.x, tk.pos.y, tk.armor, tk.shells, tk.fuel, tk.bay.recon, tk.bay.interceptor, tk.bay.strike, tk.bubble, air.join(" "), p.score
                );
            }
        }
        let swarm = world.drones.iter().filter(|d| d.owner == SWARM_OWNER).count();
        if swarm > 0 {
            log::debug!("[{:>6} {:>6.1}s] swarm drones airborne: {}", t, s, swarm);
        }
    }
}

fn summary(world: &World, started: Instant) {
    log::info!("---- match over after {} ticks ({:.0} game seconds, {:.1}s wall) ----", world.tick, world.seconds(), started.elapsed().as_secs_f32());
    let mut ps: Vec<&Player> = world.players.iter().collect();
    ps.sort_by_key(|p| std::cmp::Reverse(p.score));
    for p in ps {
        let t = world.tank_of(p.id).unwrap();
        log::info!("  p{} {:<8} {:<5} score {:>4} armor {:>3.0} shells {:>2} bay R{}/I{}/S{}{}", p.id, p.name, if p.alive { "alive" } else { "dead" }, p.score, t.armor, t.shells, t.bay.recon, t.bay.interceptor, t.bay.strike, if world.winner == Some(p.id) { "  <- winner" } else { "" });
    }
    let craters = world.map.cells.iter().filter(|c| **c == Terrain::Rubble as u8).count();
    log::info!("  craters: {}  depots: {:?}", craters, world.depots.iter().map(|d| d.owner).collect::<Vec<_>>());
}

fn scenario_from(args: &Args, names: Vec<(String, bool)>) -> Scenario {

    let mut s = Scenario { seed: args.seed, swarm_interval: args.swarm, ..Scenario::default() };
    if args.ticks > 0 {
        s.time_limit = args.ticks;
    }
    if !names.is_empty() {
        s.players = names;
    }
    s
}

fn run_local(args: Args) {
    let names: Vec<(String, bool)> = (0..args.players).map(|i| (seat_name(i), true)).collect();
    let scenario = scenario_from(&args, names);
    let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..args.players as PlayerId).map(|i| (i, make_brain(args.brains.get(i as usize), i, args.seed))).collect();
    let mut host = Host::new(new_peer_id(), None, scenario, vec![], RouteTable::default());
    let mut trace = Trace::open(args.trace.as_ref());
    log::info!("local match: seed {:#x}, {} seats: {}", args.seed, bots.len(), bots.iter().map(|(p, b)| format!("p{}={}", p, b.name())).collect::<Vec<_>>().join(" "));
    let started = Instant::now();
    let mut next = Instant::now();
    while !host.world.over {
        let orders = run_bots(&host.world, &mut bots);
        let events = host.tick(orders).to_vec();
        narrate(&host.world, &events);
        trace.write(host.world.tick, &events);
        if args.realtime {
            next += Duration::from_secs_f32(DT);
            if let Some(d) = next.checked_duration_since(Instant::now()) {
                std::thread::sleep(d);
            }
        }
    }
    summary(&host.world, started);
}

fn run_net(args: Args) {
    let me = new_peer_id();
    let mut sock = Socket::bind(args.port).unwrap_or_else(|e| die(&format!("bind: {}", e)));
    let mut cfg = LobbyConfig::new(me, &args.name, &args.game);
    cfg.seeds = args.peers.clone();
    cfg.want = args.want;
    log::info!("peer {:016x} '{}' in lobby '{}', waiting for {} peers", me, args.name, args.game, args.want);
    let mut lobby = Lobby::new(cfg);
    let result = loop {
        if let Some(r) = lobby.poll(&mut sock) {
            break r;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let mut routes = RouteTable::default();
    for p in &result.peers {
        routes.direct.insert(p.id, p.addr);
    }
    for (p, via) in &result.decision.route {
        if *via != me && *p != me {
            // Peers whose next hop toward the host is not the host: if I am
            // the host, I reach them back through that hop.
            if result.i_host(me) {
                routes.via.insert(*p, *via);
            }
        }
    }
    if let Some(h) = result.next_hop {
        routes.via.insert(result.decision.host, h);
    }
    let mut trace = Trace::open(args.trace.as_ref());
    let started = Instant::now();

    if result.i_host(me) {
        let mut peers: Vec<(u64, String)> = result.peers.iter().map(|p| (p.id, p.name.clone())).collect();
        peers.sort();
        peers.truncate(MAX_PLAYERS);
        let mut names: Vec<(String, bool)> = peers.iter().map(|(_, n)| (n.clone(), false)).collect();
        let first_bot = names.len() as PlayerId;
        let seats = args.players.max(names.len());
        while names.len() < seats {
            names.push((format!("bot{}", names.len()), true));
        }
        let scenario = scenario_from(&args, names);
        let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (first_bot..seats as PlayerId).map(|i| (i, make_brain(args.brains.get((i - first_bot) as usize), i, args.seed))).collect();
        let mut host = Host::new(me, Some(sock), scenario, peers, routes);
        log::info!("hosting: {} remote seat(s), {} bot(s)", host.seats.len(), bots.len());
        let mut next = Instant::now();
        while !host.world.over {
            host.poll();
            if Instant::now() >= next {
                next += Duration::from_secs_f32(DT);
                let orders = run_bots(&host.world, &mut bots);
                let events = host.tick(orders).to_vec();
                narrate(&host.world, &events);
                trace.write(host.world.tick, &events);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        // A second of grace: the final snapshot and the MatchOver event
        // are still in flight (and the event is resent until acked). A
        // Leave on their heels was read in the same poll as the result,
        // and the client called it a lost connection.
        let grace = Instant::now() + Duration::from_secs(1);
        while Instant::now() < grace {
            host.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        host.shutdown();
        summary(&host.world, started);
    } else {
        let mut client = Client::new(me, sock, result.decision.host, routes, &args.name);
        let mut bot = make_brain(args.brains.first(), 0, args.seed ^ me);
        log::info!("joining host {:016x} as a bot seat", result.decision.host);
        let mut last_obs: Option<Observation> = None;
        loop {
            client.poll();
            // Read what arrived before judging the connection: the last
            // snapshot and the host's Leave can come in one poll.
            let obs = client.take_observation();
            match client.state {
                ClientState::Playing => {}
                ClientState::Joining => {
                    std::thread::sleep(Duration::from_millis(5));
                    continue;
                }
                ClientState::Refused | ClientState::Lost => {
                    if let Some(o) = obs.as_ref().filter(|o| o.over) {
                        log::warn!("match over; winner {:?}", o.winner);
                        last_obs = obs;
                        break;
                    }
                    log::error!("session ended: {:?} {}", client.state, client.refuse_reason);
                    break;
                }
            }
            if let Some(obs) = obs {

                let events = client.drain_events();
                if !events.is_empty() {
                    for e in &events {
                        log::info!("[{:>6}] {:?}", obs.tick, e);
                    }
                    trace.write(obs.tick, &events);
                }
                if obs.over {
                    log::warn!("match over; winner {:?}", obs.winner);
                    break;
                }
                for cmd in bot.think(&obs) {
                    client.send(cmd);
                }
                last_obs = Some(obs);
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        client.leave();
        if let Some(o) = last_obs {
            log::info!("last seen tick {}; scores {:?}", o.tick, o.players.iter().map(|p| (p.name.clone(), p.score)).collect::<Vec<_>>());
        }
    }
}
