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
use treadcore_net::{new_peer_id, Client, ClientState, Host, Lobby, LobbyConfig, Phase, Socket};
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
            .add_systems(FixedUpdate, step_host)
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
    /// Wall-clock when the observation arrived, for interpolation.
    pub received: Option<Instant>,
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
    /// The host's world hash and its tick, from heartbeats (or our own
    /// when hosting).
    pub hash: (Tick, u64),
}

pub enum Session {
    Lobby { lobby: Lobby, sock: Option<Socket>, me: PeerId },
    Hosting { host: Host, bots: Vec<(PlayerId, Box<dyn Brain>)>, me_player: PlayerId },
    Client { client: Client },
    Ended,
}

#[derive(Resource)]
pub struct SessionRes(pub Session);

fn start(mut commands: Commands, args: Res<Args>, mut view: ResMut<View>) {
    let me = new_peer_id();
    if args.lan {
        match Socket::bind(PORT) {
            Ok(sock) => {
                let mut cfg = LobbyConfig::new(me, &args.name, &args.game);
                cfg.seeds = args.peers.clone();
                cfg.want = args.want;
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
        push_events(&mut view, &obs.events, *me_player);
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

fn poll(mut commands: Commands, session: Option<ResMut<SessionRes>>, args: Res<Args>, mut orders: ResMut<Orders>, mut view: ResMut<View>) {
    let Some(mut session) = session else { return };
    let next = match &mut session.0 {
        Session::Lobby { lobby, sock, me } => {
            let Some(s) = sock.as_mut() else { return };
            view.net.phase = format!("{:?}", lobby.phase);
            view.net.peers = lobby.peers.len();
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
                Some(host_local(&args, me, peers, sock, routes, &mut view))
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
                    // client's copy of the map keeps up by applying them.
                    if let Some(m) = view.map.as_mut() {
                        for e in &events {
                            match e {
                                Event::ShellImpact { at, kind: ShellKind::Indirect, .. } => m.crater(*at),
                                Event::TerrainChanged { at, to } => {
                                    m.set(*at, Terrain::from_u8(*to));
                                }
                                _ => {}
                            }
                        }
                    }

                    if let Some(obs) = client.take_observation() {
                        view.net.rtt_ms = Some((obs.tick.saturating_sub(client.host_tick) as u32).saturating_mul(1000 / TICK_HZ));
                        if obs.over {
                            view.status = match obs.winner {
                                Some(w) if Some(w) == client.player => "MATCH OVER: you win".into(),
                                Some(w) => format!("MATCH OVER: {} wins", obs.players.get(w as usize).map(|p| p.name.as_str()).unwrap_or("?")),
                                None => "MATCH OVER: draw".into(),
                            };
                        }
                        view.obs = Some(obs);
                        view.received = Some(Instant::now());
                    }
                }
            }
            None
        }
        Session::Hosting { host, .. } => {
            host.poll();
            None
        }
        Session::Ended => None,
    };
    if let Some(s) = next {
        commands.insert_resource(SessionRes(s));
    }
}

fn push_events(view: &mut View, events: &[Event], me: PlayerId) {
    for e in events {
        let line = describe(e, me, view);

        if let Some(l) = line {
            let tick = view.obs.as_ref().map(|o| o.tick).unwrap_or(0);
            view.log.push_back((tick, l));
            while view.log.len() > 10 {
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
        Event::Intercepted { owner, .. } => format!("{} intercept: KILL", who(owner)),
        Event::InterceptMissed { .. } => "intercept missed".into(),
        Event::StrikeHit { damage, .. } => format!("STRIKE hit for {:.0}", damage),
        Event::TankDamaged { by, damage, armor_left, .. } => format!("tank hit -{:.0} ({:.0} left) by {}", damage, armor_left, by.map(|b| who(&b)).unwrap_or_else(|| "swarm".into())),
        Event::TankDestroyed { owner, by, .. } => format!("{} DESTROYED by {}", who(owner), by.map(|b| who(&b)).unwrap_or_else(|| "swarm".into())),
        Event::DepotCaptured { by, .. } => format!("{} captured a depot", who(by)),
        Event::Resupplied { .. } => "resupplying".into(),
        Event::SwarmInbound { count, .. } => format!("SWARM: {} inbound", count),
        Event::LinkLost { .. } => "link lost: drone returning".into(),
        Event::Chat { player, text, to } => format!("{}{}: {}", who(player), if *to == ChatTo::Team { " (team)" } else { "" }, text),
        Event::MatchOver { winner, team } => format!("match over: {}{}", winner.map(|w| who(&w)).unwrap_or_else(|| "draw".into()), team.map(|t| format!(" (team {})", t)).unwrap_or_default()),
        Event::FriendlyFireSpared { by, damage, .. } => format!("friendly fire from {} spared ({:.0})", who(by), damage),
        Event::AllianceRequested { from, to } => format!("{} asks {} to ally", who(from), who(to)),
        Event::AllianceDeclined { from, by } => format!("{} declined {}", who(by), who(from)),
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
