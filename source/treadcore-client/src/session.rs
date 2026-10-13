//! The session resource: whichever of lobby, local host, network host or
//! network client this process is right now, and the `View` -- the last
//! observation, the map, and the event log -- that render and hud read.
//!
//! Only this file touches treadcore-net. The host, when this process is
//! one, is stepped from `FixedUpdate` at TICK_HZ; everything else runs in
//! `Update`, as often as frames come.

use crate::Args;
use bevy::prelude::*;
use std::collections::VecDeque;
use std::time::Instant;
use treadcore_brain::{run_bots, Brain, RuleBot};
use treadcore_net::session::RouteTable;
use treadcore_net::{identity_from_file, new_peer_id, Client, ClientState, Host, Lobby, LobbyConfig, Phase, Predictor, Socket};
use treadcore_proto::{PeerId, PORT};
use treadcore_sim::*;
// Explicit, because bevy::prelude also exports a Command and an Event and
// a glob import on each side is ambiguous: these are the sim's.
use treadcore_sim::{Command, Event};

pub struct SessionPlugin;

impl Plugin for SessionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<View>()
            .init_resource::<Orders>()
            .add_systems(Startup, start)
            .add_systems(FixedUpdate, (step_host, step_client))
            .add_systems(Update, poll);
    }
}

/// Orders from the keyboard and mouse, waiting for the next tick.
#[derive(Resource, Default)]
pub struct Orders(pub Vec<Command>);

/// What the window draws. Replaced whenever a new observation arrives.
#[derive(Resource, Default)]
pub struct View {
    pub obs: Option<Observation>,
    pub map: Option<Map>,
    pub me: Option<PlayerId>,
    pub log: VecDeque<(Tick, String)>,
    pub status: String,
    pub net: NetStats,
    /// The last tick an observation gave, for stamping events that
    /// arrive before the next one (a rejoin's first events, say).
    pub last_tick: Tick,
    /// Tanks this seat destroyed, and times it was destroyed: the
    /// prior art's two counters.
    pub kills: u32,
    pub deaths: u32,
    /// Impacts the renderer throws cubes out of: where, what colour,
    /// how many. Drained each frame.
    pub impacts: Vec<(V2, Color, u8)>,
    /// Wall-clock when the observation arrived, for interpolation.
    pub received: Option<Instant>,
    /// The player's own tank as predicted (roadmap step 4), and when it
    /// was last driven: a client draws this one, not the snapshot's.
    pub predicted: Option<(Tank, Instant)>,
    /// When the player's own tank was last hit: the frame flashes and
    /// the camera shakes for a moment after (PLAYABILITY.md item 3).
    pub hit: Option<Instant>,
    /// The last order refused, held on the newswire until the next
    /// order or a few seconds, because a refusal is the one line a new
    /// player has to read twice.
    pub refused: Option<(Instant, String)>,
}

#[derive(Default, Clone)]
pub struct NetStats {
    pub mode: &'static str,
    pub host: String,
    pub rtt_ms: Option<u32>,
    pub unacked: usize,
    pub sent: u64,
    pub received: u64,
    pub dropped: u64,
    pub peers: usize,
    pub phase: String,
    /// The lobby's peers by name with the median round trip measured so
    /// far, for the lobby screen.
    pub lobby_peers: Vec<(String, Option<u32>)>,
    /// The host's world hash and its tick, from heartbeats (or our own
    /// when hosting).
    pub hash: (Tick, u64),
    /// Prediction: ticks ahead of the last snapshot, orders in flight,
    /// checks, mismatches, and the tick of the last correction.
    pub predict: Option<(u32, usize, u64, u64, Option<Tick>)>,
}

pub enum Session {
    Lobby { lobby: Lobby, sock: Option<Socket>, me: PeerId },
    Hosting { host: Host, bots: Vec<(PlayerId, Box<dyn Brain>)>, me_player: PlayerId },
    Client { client: Client },
}

#[derive(Resource)]
pub struct SessionRes(pub Session);

fn start(mut commands: Commands, args: Res<Args>, mut view: ResMut<View>) {
    let me = match args.identity.as_deref() {
        Some(path) => match identity_from_file(path) {
            Ok(id) => id,
            Err(e) => {
                error!("identity {}: {}; using a fresh id", path, e);
                new_peer_id()
            }
        },
        None => new_peer_id(),
    };
    if args.lan {
        match Socket::bind(PORT) {
            Ok(sock) => {
                let mut cfg = LobbyConfig::new(me, &args.name, &args.game);
                cfg.seeds = args.peers.clone();
                cfg.want = args.want;
                cfg.beacon = args.beacon;
                cfg.introducer = args.introducer.clone();
                view.status = format!("lobby '{}': looking for {} peers", args.game, args.want);
                view.net.mode = "lobby";
                commands.insert_resource(SessionRes(Session::Lobby { lobby: Lobby::new(cfg), sock: Some(sock), me }));
                return;
            }
            Err(e) => {
                error!("cannot bind udp: {}; playing solo", e);
            }
        }
    }
    let session = host_local(&args, me, vec![], None, RouteTable::default(), &mut view);
    commands.insert_resource(SessionRes(session));
}

fn scenario(args: &Args, names: Vec<(String, bool)>) -> Scenario {
    let map = match &args.map {
        Some(text) => MapSource::Text(text.clone()),
        None => MapSource::FromScenarioSeed,
    };
    let mut s = Scenario { seed: args.seed, map, swarm_interval: args.swarm, ..Scenario::default() };
    if let Some(secs) = args.seconds {
        s.time_limit = secs * TICK_HZ as u64;
    }
    if !names.is_empty() {
        s.players = names;
    }
    if args.teams > 0 {
        s = s.with_teams(args.teams);
    }
    s
}

/// Become the host: remote seats first, then me, then bots to four.
fn host_local(args: &Args, me: PeerId, remote: Vec<(PeerId, String)>, sock: Option<Socket>, routes: RouteTable, view: &mut View) -> Session {
    let mut names: Vec<(String, bool)> = remote.iter().map(|(_, n)| (n.clone(), false)).collect();
    let me_player = names.len() as PlayerId;
    names.push((args.name.clone(), false));
    let seats = args.players.clamp(names.len(), MAX_PLAYERS);
    while names.len() < seats {
        names.push((format!("bot{}", names.len()), true));
    }
    let first_bot = me_player + 1;
    let bots: Vec<(PlayerId, Box<dyn Brain>)> = (first_bot..seats as PlayerId).map(|i| (i, Box::new(RuleBot::new(args.seed.wrapping_mul(31) ^ (i as u64 + 1), "rulebot")) as Box<dyn Brain>)).collect();
    let host = Host::new(me, sock, scenario(args, names), remote, routes);
    view.map = Some(host.world.map.clone());
    view.me = Some(me_player);
    view.net.mode = if host.is_networked() { "host" } else { "solo" };
    view.net.host = "me".into();
    view.status = if host.is_networked() { format!("hosting for {} peer(s)", host.seats.len()) } else { format!("solo: {} bots", seats - 1) };

    info!("{}", view.status);
    Session::Hosting { host, bots, me_player }
}

fn step_host(session: Option<ResMut<SessionRes>>, mut orders: ResMut<Orders>, mut view: ResMut<View>) {
    let Some(mut session) = session else { return };
    if let Session::Hosting { host, bots, me_player } = &mut session.0 {
        if host.world.over {
            return;
        }
        let mut local = run_bots(&host.world, bots);
        for cmd in orders.0.drain(..) {
            local.push(Order { player: *me_player, cmd });
        }
        host.tick(local);
        let obs = host.world.observe(*me_player);
        view.last_tick = obs.tick;
        push_events(&mut view, &obs.events, *me_player);
        // The window's map keeps up with the world's the way a client's
        // does: craters and walls arrive as events. (Solo play never
        // showed a crater before this line.)
        if let Some(m) = view.map.as_mut() {
            Predictor::map_keeps_up(m, &obs.events);
        }
        if let Some(s) = host.socket() {
            view.net.sent = s.sent;
            view.net.received = s.received;
            view.net.dropped = s.dropped;
        }
        view.net.peers = host.seats.iter().filter(|s| s.joined).count();
        view.net.hash = host.hash_mark;
        view.obs = Some(obs);
        view.received = Some(Instant::now());
        if host.world.over {
            view.status = match host.world.winner {
                Some(w) if w == *me_player => "MATCH OVER: you win".into(),
                Some(w) => format!("MATCH OVER: {} wins", host.world.players[w as usize].name),
                None => "MATCH OVER: draw".into(),
            };
        }
    }
}

/// A client's own tank, driven one tick ahead at the tick rate. Orders
/// are sent (and predicted) in `poll`, as often as frames come; this is
/// only the clock.
fn step_client(session: Option<ResMut<SessionRes>>, mut view: ResMut<View>) {
    let Some(mut session) = session else { return };
    if let Session::Client { client } = &mut session.0 {
        client.predict_tick();
        view.predicted = client.predicted_tank().map(|t| (t.clone(), Instant::now()));
    }
}

fn poll(mut commands: Commands, session: Option<ResMut<SessionRes>>, args: Res<Args>, mut orders: ResMut<Orders>, mut view: ResMut<View>) {
    let Some(mut session) = session else { return };
    let next = match &mut session.0 {
        Session::Lobby { lobby, sock, me } => {
            let Some(s) = sock.as_mut() else { return };
            view.net.phase = format!("{:?}", lobby.phase);
            view.net.peers = lobby.peers.len();
            view.net.lobby_peers = lobby.peers.values().map(|p| (p.name.clone(), p.median_us().map(|us| us / 1000))).collect();
            view.status = match lobby.phase {
                Phase::Discover => format!("lobby '{}': {} of {} peers found, {} ...", args.game, lobby.peers.len() + 1, args.want, lobby.peers.values().map(|p| p.name.clone()).collect::<Vec<_>>().join(" ")),
                Phase::Probe => "lobby: measuring latency to every peer".into(),
                Phase::Share => "lobby: exchanging latency rows".into(),
                Phase::Decide => "lobby: electing the host".into(),
                Phase::Done => "lobby: done".into(),
            };
            let Some(result) = lobby.poll(s) else { return };
            let me = *me;
            let mut routes = RouteTable::default();
            for p in &result.peers {
                routes.direct.insert(p.id, p.addr);
            }
            if result.i_host(me) {
                for (p, via) in &result.decision.route {
                    if *via != me && *p != me {
                        routes.via.insert(*p, *via);
                    }
                }
                let mut peers: Vec<(PeerId, String)> = result.peers.iter().map(|p| (p.id, p.name.clone())).collect();
                peers.sort();
                peers.truncate(MAX_PLAYERS - 1);
                // (seats beyond the peers and me are filled to --players by host_local)

                let sock = sock.take();
                let registration = lobby.take_registration();
                let mut s = host_local(&args, me, peers, sock, routes, &mut view);
                if let Session::Hosting { host, .. } = &mut s {
                    host.registration = registration;
                }
                Some(s)
            } else {
                if let Some(h) = result.next_hop {
                    routes.via.insert(result.decision.host, h);
                }
                let sock = sock.take().unwrap();
                let name = result.peers.iter().find(|p| p.id == result.decision.host).map(|p| p.name.clone()).unwrap_or_default();
                view.net.mode = "client";
                view.net.host = format!("{} ({:016x})", name, result.decision.host);
                view.status = format!("joining {}", name);
                Some(Session::Client { client: Client::new(me, sock, result.decision.host, routes, &args.name) })
            }
        }
        Session::Client { client } => {
            client.poll();
            for cmd in orders.0.drain(..) {
                client.send(cmd);
            }
            view.net.unacked = client.unacked();
            view.net.hash = client.host_hash;
            view.net.predict = client.predictor.as_ref().map(|p| (p.ahead, p.in_flight(), p.checks, p.mismatches, p.corrected_at));

            view.net.sent = client.socket().sent;
            view.net.received = client.socket().received;
            view.net.dropped = client.socket().dropped;
            match client.state {
                ClientState::Joining => view.status = format!("joining {} ...", view.net.host),
                ClientState::Refused => view.status = format!("refused: {}", client.refuse_reason),
                ClientState::Lost => view.status = "connection lost".into(),
                ClientState::Playing => {
                    if view.map.is_none() {
                        if let Some(m) = client.map.clone() {
                            view.map = Some(m);
                            view.me = client.player;
                            view.status = format!("playing as seat {}", client.player.unwrap_or(0));
                        }
                    }
                    let events = client.drain_events();
                    if let Some(me) = client.player {
                        push_events(&mut view, &events, me);
                    }
                    // Craters arrive as impact events, walls, roads, felled
                    // forest and mine craters as TerrainChanged; the
                    // window's copy of the map keeps up the way the
                    // predictor's does.
                    if let Some(m) = view.map.as_mut() {
                        Predictor::map_keeps_up(m, &events);
                    }

                    if let Some(obs) = client.take_observation() {
                        view.last_tick = obs.tick;
                        // The host's measurement, sent back in its heartbeat
                        // (backlog item 5): the old figure here was snapshot
                        // tick minus heartbeat tick, which is heartbeat age.
                        view.net.rtt_ms = Some(client.rtt_ticks.saturating_mul(1000 / TICK_HZ));
                        if obs.over {
                            view.status = match obs.winner {
                                Some(w) if Some(w) == client.player => "MATCH OVER: you win".into(),
                                Some(w) => format!("MATCH OVER: {} wins", obs.players.get(w as usize).map(|p| p.name.as_str()).unwrap_or("?")),
                                None => "MATCH OVER: draw".into(),
                            };
                        }
                        view.obs = Some(obs);
                        view.received = Some(Instant::now());
                        view.predicted = client.predicted_tank().map(|t| (t.clone(), Instant::now()));
                    }
                }
            }
            None
        }
        Session::Hosting { host, .. } => {
            host.poll();
            None
        }
    };
    if let Some(s) = next {
        commands.insert_resource(SessionRes(s));
    }
}

fn push_events(view: &mut View, events: &[Event], me: PlayerId) {
    for e in events {
        if let Event::TankDestroyed { owner, by, .. } = e {
            if *by == Some(me) && *owner != me {
                view.kills += 1;
            }
            if *owner == me {
                view.deaths += 1;
            }
        }
        // What breaks throws cubes (render.rs): earth out of a crater,
        // tan out of a wall or a pillbox, the seat's colour out of a
        // tank. The map here is still the one before this event, so a
        // wall about to become rubble is still a wall.
        let tan = crate::voxel::TAN;
        let earth = crate::voxel::EARTH;
        match e {
            Event::ShellImpact { at, kind: ShellKind::Indirect, .. } => {
                let wall = view.map.as_ref().map(|m| m.at(*at) == Terrain::Wall).unwrap_or(false);
                view.impacts.push((*at, if wall { tan } else { earth }, 10));
            }
            Event::ShellImpact { at, kind: ShellKind::Direct, .. } => view.impacts.push((*at, earth, 4)),
            Event::MineExploded { at, .. } => view.impacts.push((*at, earth, 12)),
            Event::TerrainChanged { at, to } if Terrain::from_u8(*to) == Terrain::Rubble => {
                let wall = view.map.as_ref().map(|m| m.at(*at) == Terrain::Wall).unwrap_or(false);
                if wall {
                    view.impacts.push((*at, tan, 8));
                }
            }
            Event::PillboxDestroyed { pillbox, .. } => {
                if let Some(p) = view.obs.as_ref().and_then(|o| o.pillboxes.iter().find(|p| p.id == *pillbox)) {
                    view.impacts.push((p.pos, tan, 14));
                }
            }
            Event::PillboxDamaged { pillbox, .. } => {
                if let Some(p) = view.obs.as_ref().and_then(|o| o.pillboxes.iter().find(|p| p.id == *pillbox)) {
                    view.impacts.push((p.pos, tan, 3));
                }
            }
            Event::TankDestroyed { tank, owner, .. } => {
                if let Some(t) = view.obs.as_ref().and_then(|o| o.tanks.iter().find(|t| t.id == *tank)) {
                    view.impacts.push((t.pos, crate::player_color(*owner), 12));
                }
            }
            Event::TankDamaged { tank, .. } => {
                if let Some(t) = view.obs.as_ref().and_then(|o| o.tanks.iter().find(|t| t.id == *tank)) {
                    view.impacts.push((t.pos, crate::voxel::OLIVE, 3));
                }
                if view.obs.as_ref().and_then(|o| o.me.as_ref()).map(|t| t.id == *tank).unwrap_or(false) {
                    view.hit = Some(Instant::now());
                }
            }
            Event::Rejected { player, why, .. } if *player == me => {
                view.refused = Some((Instant::now(), format!("order refused: {}", why)));
            }
            _ => {}
        }
        let line = describe(e, me, view);

        if let Some(l) = line {
            let tick = view.obs.as_ref().map(|o| o.tick).unwrap_or(view.last_tick);
            view.log.push_back((tick, l));
            while view.log.len() > 40 {
                view.log.pop_front();
            }
        }
    }
}

fn describe(e: &Event, me: PlayerId, view: &View) -> Option<String> {
    let who = |p: &PlayerId| if *p == me { "you".to_string() } else if *p == SWARM_OWNER { "swarm".to_string() } else { format!("p{}", p) };
    Some(match e {
        Event::Rejected { why, .. } => format!("order refused: {}", why),
        Event::ShellLaunched { kind: ShellKind::Indirect, spotted, eta_ticks, .. } => format!("round out, {}, {:.1}s", if *spotted { "spotted" } else { "blind" }, *eta_ticks as f32 * DT),
        Event::ShellLaunched { kind: ShellKind::Direct, .. } => "direct fire".into(),
        Event::ShellImpact { hits, kind, .. } if !hits.is_empty() => format!("{:?} impact: {} hit", kind, hits.len()),
        Event::ShellImpact { .. } => return None,
        Event::DroneLaunched { kind, .. } => format!("{:?} airborne", kind),
        Event::DroneLost { owner, kind, why, .. } => format!("{} lost {:?} ({:?})", who(owner), kind, why),
        Event::DroneRecovered { kind, .. } => format!("{:?} recovered", kind),
        Event::DroneRetasked { task, .. } => match task {
            DroneTask::Orbit { .. } => "recon orbit moved".into(),
            DroneTask::Strike { .. } => "strike retargeted".into(),
            DroneTask::Return => "drone recalled".into(),
            DroneTask::Intercept { .. } => return None,
        },
        Event::Intercepted { owner, .. } => format!("{} intercept: KILL", who(owner)),
        Event::InterceptMissed { .. } => "intercept missed".into(),
        Event::StrikeHit { damage, .. } => format!("STRIKE hit for {:.0}", damage),
        Event::TankDamaged { tank, by, damage, armor_left } => {
            let obs = view.obs.as_ref();
            let mine = obs.and_then(|o| o.me.as_ref()).map(|t| t.id == *tank).unwrap_or(false);
            if mine {
                format!("tank hit -{:.0} ({:.0} left) by {}", damage, armor_left, by.map(|b| who(&b)).unwrap_or_else(|| "swarm".into()))
            } else {
                let owner = obs.and_then(|o| o.tanks.iter().find(|t| t.id == *tank)).map(|t| who(&t.owner)).unwrap_or_else(|| "a tank".into());
                format!("{} hit -{:.0} by {}", owner, damage, by.map(|b| who(&b)).unwrap_or_else(|| "swarm".into()))
            }
        }
        Event::TankDestroyed { owner, by, .. } => format!("{} DESTROYED by {}", who(owner), by.map(|b| who(&b)).unwrap_or_else(|| "swarm".into())),
        Event::DepotCaptured { by, .. } => format!("{} captured a depot", who(by)),
        Event::Resupplied { .. } => "resupplying".into(),
        Event::SwarmInbound { count, .. } => format!("SWARM: {} inbound", count),
        Event::LinkLost { .. } => "link lost: drone returning".into(),
        Event::Chat { player, text, to } => format!("{}{}: {}", who(player), if *to == ChatTo::Team { " (team)" } else { "" }, text),
        Event::MatchOver { winner, team } => format!("match over: {}{}", winner.map(|w| who(&w)).unwrap_or_else(|| "draw".into()), team.map(|t| format!(" (team {})", t)).unwrap_or_default()),
        Event::FriendlyFireSpared { by, damage, .. } => format!("friendly fire from {} spared ({:.0})", who(by), damage),
        Event::Proposed { from, to, terms, .. } => format!("{} proposes {:?} to {}", who(from), terms, who(to)),
        Event::ProposalResolved { from, to, terms, accepted, why, .. } => format!("{} {:?} {}: {}{}", who(from), terms, who(to), if *accepted { "accepted" } else { "refused" }, why.as_ref().map(|w| format!(" ({})", w)).unwrap_or_default()),
        Event::AllianceFormed { player, team, .. } => format!("{} joined team {}", who(player), team),
        Event::AllianceLeft { player, team } => format!("{} left team {}", who(player), team),
        Event::PillboysDeployed { task, .. } => format!("Pillboys out: {}", match task { PillboyTask::Capture { .. } => "capture", PillboyTask::Repair { .. } => "repair", PillboyTask::Build { .. } => "build", PillboyTask::Harvest { .. } => "harvest", PillboyTask::Return => "return" }),
        Event::PillboysReturned { .. } => "Pillboys aboard".into(),
        Event::PillboysAbandoned { why, .. } => format!("Pillboys give up: {}", why),
        Event::PillboysHit { owner, lost, left, .. } => format!("{} Pillboys hit: -{} ({} left)", who(owner), lost, left),
        Event::PillboysLost { owner, .. } => format!("{} PILLBOYS LOST", who(owner)),
        Event::PillboxCaptured { by, from, .. } => format!("{} captured a pillbox{}", who(by), from.map(|f| format!(" from {}", who(&f))).unwrap_or_default()),
        Event::PillboxRepaired { by, .. } => format!("{} repaired a pillbox", who(by)),
        Event::PillboxBuilt { by, .. } => format!("{} built a pillbox", who(by)),
        Event::PillboxDamaged { .. } => return None,
        Event::PillboxDestroyed { owner, by, .. } => format!("pillbox ({}) dead by {}", owner.map(|o| who(&o)).unwrap_or_else(|| "neutral".into()), by.map(|b| who(&b)).unwrap_or_else(|| "?".into())),
        Event::PillboxFired { tank, damage, .. } => {
            if Some(*tank) == view.obs.as_ref().and_then(|o| o.me.as_ref()).map(|t| t.id) { format!("pillbox hit you -{:.0}", damage) } else { return None }
        }
        Event::Built { owner, what, .. } => format!("{} built a {:?}", who(owner), what),
        Event::Harvested { material, .. } => format!("harvested +{} material", material),
        Event::MineLaid { .. } => "mine laid".into(),
        Event::MineExploded { owner, hits, .. } => format!("MINE ({}) {}", who(owner), if hits.is_empty() { "cleared".to_string() } else { format!("hit {}", hits.len()) }),
        Event::TerrainChanged { .. } => return None,
    })
}

impl Drop for SessionRes {
    fn drop(&mut self) {
        match &mut self.0 {
            Session::Client { client } => client.leave(),
            Session::Hosting { host, .. } => host.shutdown(),
            _ => {}
        }
    }
}
