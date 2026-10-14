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
//!   introduce `--introduce`: the introducer, a rendezvous service for
//!            peers across the Internet (roadmap step 7). Hosts nothing.
//!   listen   `--listen DIR`: a dedicated host, match after match, with a
//!            status page (roadmap step 8; `listen.rs`).
//!
//! No argument parser crate; the flags are few and the loop is explicit
//! so the trace reads top to bottom.

use std::time::{Duration, Instant};
use treadcore_brain::{run_bots, Brain, Defender, JsonBrain, RuleBot};
use treadcore_net::session::RouteTable;
use treadcore_net::{identity_from_file, new_peer_id, Client, ClientState, Host, Introducer, Lobby, LobbyConfig, LobbyResult, PeerId, Registration, Socket};
use treadcore_proto::PORT;
use treadcore_sim::*;

mod listen;

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
    /// Run the introducer instead of a lobby or a match.
    introduce: bool,
    /// An introducer to register with ("host:port").
    introducer: Option<String>,
    /// Shout on the LAN while looking (off: `--no-beacon`).
    beacon: bool,
    /// A file holding this peer's id across restarts (`--identity`).
    identity: Option<String>,
    brains: Vec<String>,
    swarm: u32,
    trace: Option<String>,
    schema: bool,
    players: usize,
    record: Option<String>,
    replay: Option<String>,
    /// How many bot seats, from the last, are Defenders
    /// instead of RuleBots (attackers).
    defenders: usize,
    /// How many bot seats, just before the defenders, are rookies
    /// (PLAYABILITY item 9).
    rookies: usize,
    /// Deal the seats into this many teams, round robin (0: everyone alone).
    teams: usize,
    /// What a tank starts with (`--game-type open|tournament|strict`).
    game_type: GameType,
    /// Bot seats see every tank (`--advantage`, SPEC BRN-12).
    advantage: bool,
    /// Give a map with no structures an economy (`--industry`, step 10f).
    industry: bool,
    /// A map file's text, read and checked at start (`--map`).
    map: Option<String>,
    /// Write the map as played to this file when the match ends.
    save_map: Option<String>,
    /// Write the generated map for `--seed` to this file and exit.
    write_map: Option<String>,
    /// Be a dedicated host, writing each match's files here (`--listen`).
    listen: Option<String>,
    /// The status page's TCP port (`--http`; the UDP port's by default).
    http: Option<u16>,
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
        introduce: false,
        introducer: None,
        beacon: true,
        identity: None,
        brains: vec![],
        swarm: 45 * TICK_HZ,
        trace: None,
        schema: false,
        players: MAX_PLAYERS,
        record: None,
        replay: None,
        defenders: 0,
        rookies: 0,
        teams: 0,
        game_type: GameType::Open,
        advantage: false,
        industry: false,
        map: None,
        save_map: None,
        write_map: None,
        listen: None,
        http: None,
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
            "--introduce" => a.introduce = true,
            "--introducer" => a.introducer = Some(val()),
            "--no-beacon" => a.beacon = false,
            "--identity" => a.identity = Some(val()),
            "--brain" => a.brains.push(val()),
            "--swarm" => a.swarm = parse_num::<u32>(&val()) * TICK_HZ,
            "--trace" => a.trace = Some(val()),
            "--brain-schema" => a.schema = true,
            "--players" => a.players = parse_num::<usize>(&val()).clamp(1, MAX_PLAYERS),
            "--defenders" => a.defenders = parse_num(&val()),
            "--rookies" => a.rookies = parse_num(&val()),
            "--teams" => a.teams = parse_num(&val()),
            "--advantage" => a.advantage = true,
            "--industry" => a.industry = true,
            "--game-type" => {
                let v = val();
                a.game_type = GameType::parse(&v).unwrap_or_else(|| die(&format!("game type '{}': open, tournament or strict", v)));
            }
            "--map" => {
                let path = val();
                let text = std::fs::read_to_string(&path).unwrap_or_else(|e| die(&format!("map {}: {}", path, e)));
                // MAP-10: a file that is not a map is refused, and the game
                // does not start.
                match Map::from_text(&text) {
                    Ok(m) => log::info!("map {}: {}x{}, {} starts, {} depots, {} pillboxes", path, m.w, m.h, m.starts.len(), m.depots.len(), m.pillboxes.len()),
                    Err(e) => die(&format!("map {}: {}", path, e)),
                }
                a.map = Some(text);
            }
            "--save-map" => a.save_map = Some(val()),
            "--write-map" => a.write_map = Some(val()),
            "--record" => a.record = Some(val()),
            "--replay" => a.replay = Some(val()),
            "--listen" => a.listen = Some(val()),
            "--http" => a.http = Some(parse_num(&val())),
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
  --defenders N      of the bot seats, the last N dig in at their depot
                     (harvest, mines, walls) instead of attacking (0)
  --rookies N        of the bot seats, the N before the defenders play
                     badly: drones late, aim loose, no storming (0)
  --teams N          deal the seats into N teams, round robin (seat i to
                     team i mod N); 0 is everyone alone, allying in play
  --game-type T      what a tank starts with: open (shells and material
                     full), tournament (half the shells, no material),
                     strict (neither) (open)
  --advantage        bot seats are told where every tank is, regardless
                     of range (the prior art's brain advantage; a handicap for a
                     human against bots)
  --industry         give a map with no structures of its own an economy:
                     pad zones with neutral power plants, refineries and
                     data centres (roadmap step 10f)
  --map FILE       play on this map file (documentation/MAPS.md)
                     instead of the one generated from --seed
  --save-map FILE    when the match ends, write the map as played
                     (craters, walls, pillbox owners) to FILE
  --write-map FILE   write the map generated from --seed to FILE and
                     exit: the starting point for editing one
  --name NAME        this peer's name (server)
  --game NAME        lobby name; only peers in the same lobby meet
  --peer HOST:PORT   greet this address as well as the LAN broadcast
  --want N           how many peers to wait for before deciding (4)
  --port N           UDP port to bind (47296; falls back to any free)
  --introducer H:P   register with the introducer there, for peers no
                     broadcast reaches; a host keeps registering, so a
                     seat that drops and returns finds it again
  --no-beacon        do not shout on the LAN (with --introducer, on a
                     network that is not yours)
  --identity FILE    this peer's id, kept in FILE across restarts (made
                     the first time): what a returning seat is known by
  --introduce        be the introducer on --port: hosts nothing, keeps
                     nothing past thirty seconds; one for a group
  --listen DIR       be a dedicated host in lobby --game, match after
                     match: takes no seat, hosts when elected (preferred
                     within 10 ms of the best seat), writes each match's
                     trace and replay to DIR; with --introducer, listed
                     there as a server
  --http PORT        the --listen status page's TCP port (--port's)
  --brain 'CMD ...'  run an external JSON brain in one seat (repeatable)
  --trace FILE       write every event as JSON lines to FILE
  --record FILE      write the match as a replay (JSON: scenario, every
                     tick's orders, world hashes every 100 ticks)
  --replay FILE      play a recorded match back instead of running bots;
                     narrate it, and exit 1 if any hash differs
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
    log::info!("treadcore-server {} ({} Hz)", env!("CARGO_PKG_VERSION"), TICK_HZ);

    if let Some(path) = args.write_map.as_deref() {
        let mut m = Map::generate(args.seed);
        if args.industry {
            m.industrialise();
        }
        match std::fs::write(path, m.to_text()) {
            Ok(()) => log::info!("wrote the map for seed {:#x} to {} ({} starts, {} depots, {} pillboxes)", args.seed, path, m.starts.len(), m.depots.len(), m.pillboxes.len()),
            Err(e) => die(&format!("write-map {}: {}", path, e)),
        }
        return;
    }
    if args.replay.is_some() {
        run_replay(args);
    } else if args.introduce {
        run_introducer(args);
    } else if args.listen.is_some() {
        listen::run(args);
    } else if args.net {
        run_net(args);
    } else {
        run_local(args);
    }
}

/// Write a recorded replay as JSON, with the hashes it ended on.
fn save_replay(host: &Host, path: &str) {
    let Some(r) = host.replay.as_ref() else { return };
    match std::fs::File::create(path).and_then(|f| serde_json::to_writer(std::io::BufWriter::new(f), r).map_err(std::io::Error::other)) {
        Ok(()) => log::info!("replay: {} ticks, {} orders, {} checkpoints, final hash {:016x} -> {}", r.ticks, r.orders.len(), r.checkpoints.len(), r.final_hash().unwrap_or(0), path),
        Err(e) => log::error!("replay {}: {}", path, e),
    }
}

/// Play a recorded match back, narrating it as a live one, and compare
/// the world hash at every checkpoint the recording holds. The exit
/// status says whether the rules still do what they did.
fn run_replay(args: Args) {
    let path = args.replay.as_deref().unwrap();
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("replay {}: {}", path, e)));
    let replay: Replay = serde_json::from_str(&text).unwrap_or_else(|e| die(&format!("replay {}: not a replay: {}", path, e)));
    log::info!("replay: {} ({} ticks, {} orders, {} checkpoints, seed {:#x}, {} seats)", path, replay.ticks, replay.orders.len(), replay.checkpoints.len(), replay.scenario.seed, replay.scenario.players.len());
    let mut trace = Trace::open(args.trace.as_ref());
    let mut w = World::new(replay.scenario.clone());
    let started = Instant::now();
    let mut next_cp = 0usize;
    let mut mismatches = 0u32;
    let mut report = Report::default();
    while w.tick < replay.ticks {
        let orders: Vec<Order> = replay.orders_for(w.tick + 1).cloned().collect();
        w.step(&orders);
        narrate(&w, &w.events);
        trace.write(w.tick, &w.events);
        report.tally(w.tick, &w.events);
        if let Some((t, recorded)) = replay.checkpoints.get(next_cp).copied() {
            if t == w.tick {
                let now = w.hash();
                if now == recorded {
                    log::debug!("[{:>6} {:>6.1}s] checkpoint ok {:016x}", w.tick, w.seconds(), now);
                } else {
                    mismatches += 1;
                    log::error!("[{:>6} {:>6.1}s] HASH MISMATCH recorded {:016x} replayed {:016x}", w.tick, w.seconds(), recorded, now);
                }
                next_cp += 1;
            }
        }
    }
    summary(&w, started, &report);
    if mismatches == 0 {
        log::info!("replay: every checkpoint matched ({})", replay.checkpoints.len());
    } else {
        log::error!("replay: {} of {} checkpoints differ: the rules have changed since this was recorded", mismatches, replay.checkpoints.len());
        std::process::exit(1);
    }
}

/// Which built-in brain a bot seat gets when no `--brain` names one.
#[derive(Clone, Copy, PartialEq)]
enum BotKind {
    Rule,
    Rookie,
    Defender,
}

impl BotKind {
    /// The last `defenders` of the seats dig in, the `rookies` before
    /// them play badly, the rest attack.
    fn of(args: &Args, seat: PlayerId, seats: usize) -> BotKind {
        let first_defender = seats.saturating_sub(args.defenders);
        let first_rookie = first_defender.saturating_sub(args.rookies);
        match seat as usize {
            s if s >= first_defender => BotKind::Defender,
            s if s >= first_rookie => BotKind::Rookie,
            _ => BotKind::Rule,
        }
    }
}

fn make_brain(spec: Option<&String>, seat: PlayerId, seed: u64) -> Box<dyn Brain> {
    make_brain_kind(spec, seat, seed, BotKind::Rule)
}

fn make_brain_kind(spec: Option<&String>, seat: PlayerId, seed: u64, kind: BotKind) -> Box<dyn Brain> {
    if spec.is_none() {
        match kind {
            BotKind::Defender => return Box::new(Defender::new("defender", true)),
            BotKind::Rookie => return Box::new(RuleBot::rookie(seed.wrapping_mul(31) ^ (seat as u64 + 1), "rookie")),
            BotKind::Rule => {}
        }
    }
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

/// The match report: the numbers a balance reader wants, tallied from
/// the event stream as it goes by and printed after the scoreboard, so
/// a trace ends with its own summary instead of a reader counting JSON
/// lines (backlog item 8). A kill's cause is the last thing that hurt
/// the tank: a strike, a pillbox or a mine announces itself in the
/// event before the damage; anything else is a shell.
#[derive(Default)]
struct Report {
    shells_indirect: u32,
    shells_spotted: u32,
    shells_direct: u32,
    shell_hits: u32,
    launched: [u32; 4],
    structures_built: u32,
    structures_captured: u32,
    structures_wrecked: u32,
    structure_damage: std::collections::BTreeMap<&'static str, f32>,
    longshot_hits: u32,
    jammed: u32,
    lost: std::collections::BTreeMap<String, u32>,
    intercepts: u32,
    intercept_misses: u32,
    interceptors_shot: u32,
    strike_hits: u32,
    swarm: u32,
    kills: std::collections::BTreeMap<&'static str, u32>,
    first_kill: Option<Tick>,
    pillbox_fired: u32,
    pillbox_captured: u32,
    pillbox_built: u32,
    pillbox_destroyed: u32,
    depots_captured: u32,
    proposals: u32,
    proposals_accepted: u32,
    refused: std::collections::BTreeMap<String, u32>,
    pending: std::collections::HashMap<EntityId, &'static str>,
    last_cause: std::collections::HashMap<EntityId, &'static str>,
}

impl Report {
    fn tally(&mut self, tick: Tick, events: &[Event]) {
        for e in events {
            match e {
                Event::ShellLaunched { kind: ShellKind::Indirect, spotted, .. } => {
                    self.shells_indirect += 1;
                    self.shells_spotted += *spotted as u32;
                }
                Event::ShellLaunched { kind: ShellKind::Direct, .. } => self.shells_direct += 1,
                Event::DroneLaunched { kind, .. } => self.launched[*kind as usize] += 1,
                Event::DroneLost { kind, why, .. } => {
                    *self.lost.entry(format!("{:?} {:?}", kind, why).to_lowercase()).or_default() += 1;
                    if *kind == DroneKind::Interceptor && *why == LossReason::Shot {
                        self.interceptors_shot += 1;
                    }
                }
                Event::Intercepted { .. } => self.intercepts += 1,
                Event::InterceptMissed { .. } => self.intercept_misses += 1,
                Event::StrikeHit { tank, .. } => {
                    self.strike_hits += 1;
                    self.pending.insert(*tank, "strike");
                }
                Event::SwarmInbound { count, .. } => self.swarm += count,
                Event::PillboxFired { tank, .. } => {
                    self.pillbox_fired += 1;
                    self.pending.insert(*tank, "pillbox");
                }
                Event::MineExploded { hits, .. } => {
                    for h in hits {
                        self.pending.insert(*h, "mine");
                    }
                }
                Event::TankDamaged { tank, by, .. } => {
                    let cause = match (self.pending.remove(tank), by) {
                        (Some("strike"), None) => "swarm",
                        (Some(c), _) => c,
                        (None, _) => {
                            self.shell_hits += 1;
                            "shell"
                        }
                    };
                    self.last_cause.insert(*tank, cause);
                }
                Event::TankDestroyed { tank, .. } => {
                    let cause = self.last_cause.get(tank).copied().unwrap_or("?");
                    *self.kills.entry(cause).or_default() += 1;
                    self.first_kill.get_or_insert(tick);
                }
                Event::PillboxCaptured { .. } => self.pillbox_captured += 1,
                Event::StructureBuilt { .. } => self.structures_built += 1,
                Event::StructureCaptured { .. } => self.structures_captured += 1,
                Event::StructureWrecked { .. } => self.structures_wrecked += 1,
                Event::StructureDamaged { damage, .. } => {
                    // A longshot's warhead is the one blow of exactly its size.
                    let by = if *damage == LONGSHOT_STRUCTURE_DAMAGE { "longshot" } else { "shell" };
                    *self.structure_damage.entry(by).or_default() += damage;
                }
                Event::LongshotHit { hits, .. } => {
                    self.longshot_hits += 1;
                    for h in hits {
                        self.pending.insert(*h, "longshot");
                    }
                }
                Event::Jammed { .. } => self.jammed += 1,
                Event::PillboxBuilt { .. } => self.pillbox_built += 1,
                Event::PillboxDestroyed { .. } => self.pillbox_destroyed += 1,
                Event::DepotCaptured { .. } => self.depots_captured += 1,
                Event::Proposed { .. } => self.proposals += 1,
                Event::ProposalResolved { accepted: true, .. } => self.proposals_accepted += 1,
                Event::Rejected { cmd, why, .. } => {
                    let what = format!("{:?}", cmd);
                    let what = what.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("?");
                    *self.refused.entry(format!("{} {}", what, why)).or_default() += 1;
                }
                _ => {}
            }
        }
    }

    fn print(&self) {
        let pct = |a: u32, b: u32| if b == 0 { 0.0 } else { 100.0 * a as f32 / b as f32 };
        log::info!(
            "  shells: {} indirect ({} spotted, {} blind), {} direct; {} tank hits",
            self.shells_indirect,
            self.shells_spotted,
            self.shells_indirect - self.shells_spotted,
            self.shells_direct,
            self.shell_hits
        );
        let tries = self.intercepts + self.intercept_misses;
        log::info!(
            "  drones: {} recon, {} interceptor, {} strike launched; intercepts {} of {} ({:.0}%); interceptors shot down {}",
            self.launched[0],
            self.launched[1],
            self.launched[2],
            self.intercepts,
            tries,
            pct(self.intercepts, tries),
            self.interceptors_shot
        );
        if self.launched[3] + self.structures_built + self.structures_captured + self.structures_wrecked + self.jammed > 0 {
            let dmg: Vec<String> = self.structure_damage.iter().map(|(k, v)| format!("{:.0} by {}", v, k)).collect();
            log::info!(
                "  economy: longshots {} launched, {} landed; structures built {}, captured {}, wrecked {}; damage {}; drones jammed {}",
                self.launched[3],
                self.longshot_hits,
                self.structures_built,
                self.structures_captured,
                self.structures_wrecked,
                if dmg.is_empty() { "none".to_string() } else { dmg.join(", ") },
                self.jammed
            );
        }
        let strikes = self.launched[2] + self.swarm;
        log::info!("  strikes: {} launched ({} swarm), {} hit ({:.0}%)", strikes, self.swarm, self.strike_hits, pct(self.strike_hits, strikes));
        let kills: u32 = self.kills.values().sum();
        let by: Vec<String> = self.kills.iter().map(|(k, v)| format!("{} {}", v, k)).collect();
        log::info!(
            "  kills: {} ({}); first at {}",
            kills,
            if by.is_empty() { "none".to_string() } else { by.join(", ") },
            self.first_kill.map(|t| format!("{:.1}s", t as f32 * DT)).unwrap_or_else(|| "never".into())
        );
        log::info!(
            "  pillboxes: fired {}; captured {}, built {}, destroyed {}; depots captured {}; proposals {} ({} accepted)",
            self.pillbox_fired,
            self.pillbox_captured,
            self.pillbox_built,
            self.pillbox_destroyed,
            self.depots_captured,
            self.proposals,
            self.proposals_accepted
        );
        let lost: Vec<String> = self.lost.iter().map(|(k, v)| format!("{} {}", v, k)).collect();
        log::info!("  drones lost: {}", if lost.is_empty() { "none".to_string() } else { lost.join(", ") });
        let mut refused: Vec<(&String, &u32)> = self.refused.iter().collect();
        refused.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        let total: u32 = self.refused.values().sum();
        let top: Vec<String> = refused.iter().take(5).map(|(k, v)| format!("{} {}", v, k)).collect();
        log::info!("  refused orders: {}{}", total, if top.is_empty() { String::new() } else { format!(" ({})", top.join("; ")) });
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
            Event::DroneRetasked { owner, drone, task } => log::info!("[{:>6} {:>6.1}s] p{} retasks #{}: {:?}", t, s, owner, drone, task),
            Event::Intercepted { owner, target, interceptor } => log::info!("[{:>6} {:>6.1}s] {} INTERCEPT kills drone #{} (by #{})", t, s, if *owner == NEUTRAL { "neutral pillbox".to_string() } else { format!("p{}", owner) }, target, interceptor),
            Event::InterceptMissed { target, .. } => log::info!("[{:>6} {:>6.1}s] intercept missed drone #{}", t, s, target),
            Event::StrikeHit { tank, damage, .. } => log::info!("[{:>6} {:>6.1}s] STRIKE hits tank #{} for {}", t, s, tank, damage),
            Event::TankDamaged { tank, by, damage, armor_left } => log::info!("[{:>6} {:>6.1}s] tank #{} -{:.0} armor ({:.0} left) by {:?}", t, s, tank, damage, armor_left, by),
            Event::TankDestroyed { owner, by, .. } => log::warn!("[{:>6} {:>6.1}s] p{} DESTROYED by {:?}", t, s, owner, by),
            Event::DepotCaptured { depot, by } => log::info!("[{:>6} {:>6.1}s] p{} captures depot #{}", t, s, by, depot),
            Event::Resupplied { tank, depot } => log::debug!("[{:>6} {:>6.1}s] tank #{} resupplying at depot #{}", t, s, tank, depot),
            Event::SwarmInbound { count, from } => log::warn!("[{:>6} {:>6.1}s] SWARM {} inbound from ({:.0},{:.0})", t, s, count, from.x, from.y),
            Event::LinkLost { owner, drone } => log::info!("[{:>6} {:>6.1}s] p{} link lost to #{}", t, s, owner, drone),
            Event::Chat { player, text, to } => log::info!("[{:>6} {:>6.1}s] p{} ({:?}): {}", t, s, player, to, text),
            Event::MatchOver { winner, team } => log::warn!("[{:>6} {:>6.1}s] MATCH OVER winner {:?} team {:?}", t, s, winner, team),
            Event::FriendlyFireSpared { target, by, damage } => log::info!("[{:>6} {:>6.1}s] friendly fire spared #{} from p{} ({:.0})", t, s, target, by, damage),
            Event::Proposed { from, to, terms, .. } => log::info!("[{:>6} {:>6.1}s] p{} proposes {:?} to p{}", t, s, from, terms, to),
            Event::ProposalResolved { from, to, terms, accepted, by, why, .. } => log::warn!("[{:>6} {:>6.1}s] PROPOSAL p{} {:?} p{}: {}{}{}", t, s, from, terms, to, if *accepted { "ACCEPTED" } else { "refused" }, by.map(|b| format!(" by p{}", b)).unwrap_or_default(), why.as_ref().map(|w| format!(" ({})", w)).unwrap_or_default()),
            Event::AllianceFormed { player, team, members } => log::warn!("[{:>6} {:>6.1}s] ALLIANCE p{} joins team {}: {:?}", t, s, player, team, members),
            Event::AllianceLeft { player, team } => log::warn!("[{:>6} {:>6.1}s] ALLIANCE p{} leaves team {}", t, s, player, team),
            Event::PillboysDeployed { owner, task, .. } => log::info!("[{:>6} {:>6.1}s] p{} PILLBOYS out: {:?}", t, s, owner, task),
            Event::PillboysReturned { owner, .. } => log::debug!("[{:>6} {:>6.1}s] p{} pillboys aboard", t, s, owner),
            Event::PillboysAbandoned { owner, why, .. } => log::info!("[{:>6} {:>6.1}s] p{} pillboys give up: {}", t, s, owner, why),
            Event::PillboysHit { owner, lost, left, .. } => log::info!("[{:>6} {:>6.1}s] p{} pillboys hit: -{} ({} left)", t, s, owner, lost, left),
            Event::PillboysLost { owner, .. } => log::warn!("[{:>6} {:>6.1}s] p{} PILLBOYS LOST", t, s, owner),
            Event::PillboxCaptured { pillbox, by, from } => log::warn!("[{:>6} {:>6.1}s] p{} CAPTURES pillbox #{} from {}", t, s, by, pillbox, from.map(|f| format!("p{}", f)).unwrap_or_else(|| "neutral".into())),
            Event::PillboxRepaired { pillbox, by } => log::info!("[{:>6} {:>6.1}s] p{} repairs pillbox #{}", t, s, by, pillbox),
            Event::PillboxBuilt { pillbox, by, at } => log::warn!("[{:>6} {:>6.1}s] p{} BUILDS pillbox #{} at ({:.0},{:.0})", t, s, by, pillbox, at.x, at.y),
            Event::PillboxDamaged { pillbox, by, damage, health_left } => log::info!("[{:>6} {:>6.1}s] pillbox #{} -{:.0} ({:.0} left) by {:?}", t, s, pillbox, damage, health_left, by),
            Event::PillboxDestroyed { pillbox, owner, by } => log::warn!("[{:>6} {:>6.1}s] pillbox #{} ({}) DEAD by {:?}", t, s, pillbox, owner.map(|o| format!("p{}", o)).unwrap_or_else(|| "neutral".into()), by),
            Event::PillboxFired { pillbox, owner, tank, damage } => log::info!("[{:>6} {:>6.1}s] {} pillbox #{} hits tank #{} for {:.0}", t, s, if *owner == NEUTRAL { "neutral".to_string() } else { format!("p{}", owner) }, pillbox, tank, damage),
            Event::Built { owner, what, at } => log::info!("[{:>6} {:>6.1}s] p{} builds {:?} at ({:.0},{:.0})", t, s, owner, what, at.x, at.y),
            Event::Harvested { owner, at, material } => log::info!("[{:>6} {:>6.1}s] p{} harvests ({:.0},{:.0}) +{} material", t, s, owner, at.x, at.y, material),
            Event::MineLaid { owner, at, .. } => log::info!("[{:>6} {:>6.1}s] p{} lays a mine at ({:.0},{:.0})", t, s, owner, at.x, at.y),
            Event::MineExploded { owner, at, hits, .. } => log::warn!("[{:>6} {:>6.1}s] MINE (p{}) at ({:.0},{:.0}) hits {:?}", t, s, owner, at.x, at.y, hits),
            Event::StructureBuilt { structure, kind, by, at } => log::warn!("[{:>6} {:>6.1}s] p{} BUILDS {} #{} at ({:.0},{:.0})", t, s, by, kind.word(), structure, at.x, at.y),
            Event::StructureDamaged { structure, kind, by, damage, health_left, .. } => log::info!("[{:>6} {:>6.1}s] {} #{} -{:.0} ({:.0} left) by {:?}", t, s, kind.word(), structure, damage, health_left, by),
            Event::StructureWrecked { structure, kind, owner, by } => log::warn!("[{:>6} {:>6.1}s] {} #{} ({}) WRECKED by {:?}", t, s, kind.word(), structure, owner.map(|o| format!("p{}", o)).unwrap_or_else(|| "neutral".into()), by),
            Event::StructureCaptured { structure, kind, by, from } => log::warn!("[{:>6} {:>6.1}s] p{} TAKES {} #{} from {}", t, s, by, kind.word(), structure, from.map(|f| format!("p{}", f)).unwrap_or_else(|| "neutral".into())),
            Event::StructureRepaired { structure, kind, by } => log::info!("[{:>6} {:>6.1}s] p{} mends {} #{}", t, s, by, kind.word(), structure),
            Event::StructurePower { structure, kind, owner, on } => log::info!("[{:>6} {:>6.1}s] p{} {} #{} power {}", t, s, owner, kind.word(), structure, if *on { "on" } else { "OFF" }),
            Event::LongshotHit { owner, at, hits, .. } => log::warn!("[{:>6} {:>6.1}s] p{} LONGSHOT lands at ({:.0},{:.0}) hits {:?}", t, s, owner, at.x, at.y, hits),
            Event::Jammed { owner, kind, drone, jammer } => log::info!("[{:>6} {:>6.1}s] p{} {:?} #{} jammed by #{}", t, s, owner, kind, drone, jammer),
            Event::TerrainChanged { at, to } => log::trace!("[{:>6} {:>6.1}s] terrain ({:.0},{:.0}) -> {:?}", t, s, at.x, at.y, Terrain::from_u8(*to)),
        }
    }
    if t % TICK_HZ as Tick == 0 {
        for p in &world.players {
            if let Some(tk) = world.tank_of(p.id) {
                let air: Vec<String> = world.drones.iter().filter(|d| d.owner == p.id).map(|d| format!("{}{:.0}%", match d.kind { DroneKind::Recon => "R", DroneKind::Interceptor => "I", DroneKind::Strike => "S", DroneKind::Longshot => "L" }, d.link * 100.0)).collect();
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

fn summary(world: &World, started: Instant, report: &Report) {
    log::info!("---- match over after {} ticks ({:.0} game seconds, {:.1}s wall) ----", world.tick, world.seconds(), started.elapsed().as_secs_f32());
    let mut ps: Vec<&Player> = world.players.iter().collect();
    ps.sort_by_key(|p| (p.team, std::cmp::Reverse(p.score)));
    for p in ps {
        let t = world.tank_of(p.id).unwrap();
        log::info!("  t{:<2} p{:<2} {:<9} {:<5} score {:>4} armor {:>3.0} shells {:>2} bay R{}/I{}/S{}{}", p.team, p.id, p.name, if p.alive { "alive" } else { "dead" }, p.score, t.armor, t.shells, t.bay.recon, t.bay.interceptor, t.bay.strike, if world.winner == Some(p.id) { "  <- winner" } else { "" });
    }
    let mut teams: Vec<TeamId> = world.players.iter().map(|p| p.team).collect();
    teams.sort_unstable();
    teams.dedup();
    if teams.len() < world.players.len() {
        let line: Vec<String> = teams.iter().map(|t| format!("t{} {}{}", t, world.players.iter().filter(|p| p.team == *t).map(|p| p.score).sum::<u32>(), if world.winning_team == Some(*t) { "*" } else { "" })).collect();
        log::info!("  teams: {}", line.join("  "));
    }
    let craters = world.map.cells.iter().filter(|c| **c == Terrain::Rubble as u8).count();
    log::info!("  craters: {}  depots: {:?}", craters, world.depots.iter().map(|d| d.owner).collect::<Vec<_>>());
    let walls = world.map.cells.iter().filter(|c| **c == Terrain::Wall as u8).count();
    log::info!("  walls: {}  mines left: {}", walls, world.mines.len());
    let owned = world.pillboxes.iter().filter(|p| p.owner.is_some()).count();
    if !world.structures.is_empty() {
        let line: Vec<String> = world.structures.iter().map(|x| format!("{}:{}{}", &x.kind.word()[..1], x.owner.map(|o| o.to_string()).unwrap_or_else(|| "-".into()), if x.alive() { "" } else { "x" })).collect();
        log::info!("  structures: {} ({} owned, {} wrecked): {}", world.structures.len(), world.structures.iter().filter(|x| x.owner.is_some()).count(), world.structures.iter().filter(|x| !x.alive()).count(), line.join(" "));
    }

    let dead = world.pillboxes.iter().filter(|p| !p.alive()).count();
    log::info!("  pillboxes: {} ({} owned, {} dead): {:?}", world.pillboxes.len(), owned, dead, world.pillboxes.iter().map(|p| p.owner.map(|o| o.to_string()).unwrap_or_else(|| "-".into())).collect::<Vec<_>>().join(" "));
    report.print();
}

/// Write the map as played, if asked. The next match can start from it.
fn save_map(world: &World, path: Option<&str>) {
    let Some(path) = path else { return };
    let m = world.map_as_played();
    match std::fs::write(path, m.to_text()) {
        Ok(()) => log::info!("map as played -> {} ({} walls, {} pillboxes of which {} owned)", path, m.cells.iter().filter(|c| **c == Terrain::Wall as u8).count(), m.pillboxes.len(), m.pillboxes.iter().filter(|p| p.owner.is_some()).count()),
        Err(e) => log::error!("save-map {}: {}", path, e),
    }
}

fn scenario_from(args: &Args, names: Vec<(String, bool)>) -> Scenario {
    let map = match &args.map {
        Some(text) => MapSource::Text(text.clone()),
        None => MapSource::FromScenarioSeed,
    };
    let mut s = Scenario { seed: args.seed, map, game_type: args.game_type, advantage: args.advantage, swarm_interval: args.swarm, industry: args.industry, ..Scenario::default() };
    if args.ticks > 0 {
        s.time_limit = args.ticks;
    }
    if !names.is_empty() {
        s.players = names;
    }
    if args.teams > 0 {
        s = s.with_teams(args.teams);
    }
    s
}


fn run_local(args: Args) {
    let names: Vec<(String, bool)> = (0..args.players).map(|i| (seat_name(i), true)).collect();
    let scenario = scenario_from(&args, names);
    let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (0..args.players as PlayerId).map(|i| (i, make_brain_kind(args.brains.get(i as usize), i, args.seed, BotKind::of(&args, i, args.players)))).collect();
    let mut host = Host::new(new_peer_id(), None, scenario, vec![], RouteTable::default());
    if args.record.is_some() {
        host.record();
    }
    let mut trace = Trace::open(args.trace.as_ref());
    log::info!("local match: seed {:#x}, {} seats: {}", args.seed, bots.len(), bots.iter().map(|(p, b)| format!("p{}={}", p, b.name())).collect::<Vec<_>>().join(" "));
    let started = Instant::now();
    let mut next = Instant::now();
    let mut report = Report::default();
    while !host.world.over {
        let orders = run_bots(&host.world, &mut bots);
        let events = host.tick(orders).to_vec();
        narrate(&host.world, &events);
        trace.write(host.world.tick, &events);
        report.tally(host.world.tick, &events);
        if args.realtime {
            next += Duration::from_secs_f32(DT);
            if let Some(d) = next.checked_duration_since(Instant::now()) {
                std::thread::sleep(d);
            }
        }
    }
    summary(&host.world, started, &report);
    if let Some(p) = args.record.as_deref() {
        save_replay(&host, p);
    }
    save_map(&host.world, args.save_map.as_deref());
}

/// The introducer: one socket, one state machine, forever. Logs each
/// registration and lapse; `--seconds` is not honoured, it runs until
/// killed.
fn run_introducer(args: Args) {
    let mut sock = Socket::bind(args.port).unwrap_or_else(|e| die(&format!("bind: {}", e)));
    let mut intro = Introducer::new();
    log::info!("introducer on {}: tell peers --introducer HOST:{}", sock.local, sock.local.port());
    let mut last_line = Instant::now();
    loop {
        intro.poll(&mut sock);
        if last_line.elapsed() >= Duration::from_secs(60) {
            last_line = Instant::now();
            let (peers, games) = intro.count();
            log::info!("introducer: {} peer(s) in {} lobby(ies); {} registrations, {} introductions so far", peers, games, intro.registrations, intro.introductions);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// The lobby's decision as a route table: every peer at the address it
/// answered from, and the next hop for those not reached directly.
fn routes_from(result: &LobbyResult, me: PeerId) -> RouteTable {
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
    routes
}

/// Host one match for the seats the lobby gave: the remote peers first,
/// then bots up to --players. `each` sees the host after every tick and
/// returns false to give the match up (a --listen server whose seats
/// never came). Returns the host, shut down, for the caller to save from.
fn host_match(args: &Args, me: PeerId, sock: Socket, result: &LobbyResult, routes: RouteTable, registration: Option<Registration>, trace: &mut Trace, mut each: impl FnMut(&Host) -> bool) -> Host {
    let started = Instant::now();
    let mut peers: Vec<(u64, String)> = result.peers.iter().map(|p| (p.id, p.name.clone())).collect();
    peers.sort();
    peers.truncate(MAX_PLAYERS);
    let mut names: Vec<(String, bool)> = peers.iter().map(|(_, n)| (n.clone(), false)).collect();
    let first_bot = names.len() as PlayerId;
    let seats = args.players.max(names.len());
    while names.len() < seats {
        names.push((format!("bot{}", names.len()), true));
    }
    let scenario = scenario_from(args, names);
    let mut bots: Vec<(PlayerId, Box<dyn Brain>)> = (first_bot..seats as PlayerId).map(|i| (i, make_brain_kind(args.brains.get((i - first_bot) as usize), i, args.seed, BotKind::of(args, i, seats)))).collect();

    let mut host = Host::new(me, Some(sock), scenario, peers, routes);
    host.registration = registration;
    if args.record.is_some() || args.listen.is_some() {
        host.record();
    }
    log::info!("hosting: {} remote seat(s), {} bot(s){}", host.seats.len(), bots.len(), if host.registration.is_some() { "; registered as hosting with the introducer" } else { "" });
    let mut next = Instant::now();
    let mut report = Report::default();
    while !host.world.over {
        host.poll();
        if Instant::now() >= next {
            next += Duration::from_secs_f32(DT);
            let orders = run_bots(&host.world, &mut bots);
            let events = host.tick(orders).to_vec();
            narrate(&host.world, &events);
            trace.write(host.world.tick, &events);
            report.tally(host.world.tick, &events);
            if !each(&host) {
                break;
            }
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
    summary(&host.world, started, &report);
    host
}

fn run_net(args: Args) {
    let me = match args.identity.as_deref() {
        Some(path) => identity_from_file(path).unwrap_or_else(|e| die(&format!("identity: {}", e))),
        None => new_peer_id(),
    };
    let mut sock = Socket::bind(args.port).unwrap_or_else(|e| die(&format!("bind: {}", e)));
    let mut cfg = LobbyConfig::new(me, &args.name, &args.game);
    cfg.seeds = args.peers.clone();
    cfg.want = args.want;
    cfg.beacon = args.beacon;
    cfg.introducer = args.introducer.clone();
    log::info!("peer {:016x} '{}' in lobby '{}', waiting for {} peers{}", me, args.name, args.game, args.want, if args.identity.is_some() { " (identity from file)" } else { "" });
    let mut lobby = Lobby::new(cfg);
    let result = loop {
        if let Some(r) = lobby.poll(&mut sock) {
            break r;
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let routes = routes_from(&result, me);
    let mut trace = Trace::open(args.trace.as_ref());
    let registration = lobby.take_registration();

    if result.i_host(me) {
        let host = host_match(&args, me, sock, &result, routes, registration, &mut trace, |_| true);
        if let Some(p) = args.record.as_deref() {
            save_replay(&host, p);
        }
        save_map(&host.world, args.save_map.as_deref());
    } else {
        let mut client = Client::new(me, sock, result.decision.host, routes, &args.name);
        let mut bot = make_brain(args.brains.first(), 0, args.seed ^ me);
        log::info!("joining host {:016x} as a bot seat (session {:016x})", result.decision.host, client.session);
        let mut last_obs: Option<Observation> = None;
        let mut next_tick = Instant::now();
        loop {
            client.poll();
            // The seat predicts its own tank like the window does, at
            // the tick rate; the headless seat is how prediction is
            // proved against a lagged link (`make lobby-test LAG=200`).
            if Instant::now() >= next_tick {
                next_tick += Duration::from_secs_f32(DT);
                client.predict_tick();
            }
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
