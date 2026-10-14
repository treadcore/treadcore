//! The session resource: whichever of lobby, local host, network host or
//! network client this process is right now, and the `View` -- the last
//! observation, the map, and the event log -- that render and hud read.
//!
//! Only this file touches treadcore-net. The host, when this process is
//! one, is stepped from `FixedUpdate` at TICK_HZ; everything else runs in
//! `Update`, as often as frames come.

use crate::sound::{Cue, Where};
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
            .add_systems(Update, (poll, again));
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
    /// Sounds the events asked for, played and drained by sound.rs.
    pub sounds: Vec<(Cue, Where, f32)>,
    /// The chat line being typed (chat.rs): while it is open, the keys
    /// are its, not the orders'.
    pub typing: Option<crate::chat::Typing>,
    /// When this seat last sent a message: the world takes one a second.
    pub chat_sent: Option<Instant>,
    /// The last message heard, held on the newswire a while.
    pub chat: Option<(Instant, String)>,
    /// The proposal list, when open (proposals.rs).
    pub menu: Option<crate::proposals::Menu>,
    /// The chat line had this frame's keys (it was open at some point
    /// in it): the list after it must not read them as well.
    pub keys_eaten: bool,
    /// What my side is losing, newest last (commander.rs).
    pub alerts: VecDeque<crate::commander::Alert>,
    /// Solo rounds (PLAYABILITY item 10): which this is, from 0, and
    /// the tally carried across them, (played, won).
    pub round: u64,
    pub record: (u32, u32),
}

impl View {
    /// A chat line or the proposal list is open: the keys are theirs.
    pub fn keys_taken(&self) -> bool {
        self.typing.is_some() || self.menu.is_some()
    }
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
    let session = host_local(&args, me, vec![], None, RouteTable::default(), &mut view, 0);
    commands.insert_resource(SessionRes(session));
}

fn scenario(args: &Args, names: Vec<(String, bool)>) -> Scenario {
    let map = match &args.map {
        Some(text) => MapSource::Text(text.clone()),
        None => MapSource::FromScenarioSeed,
    };
    let mut s = Scenario { seed: args.seed, map, swarm_interval: args.swarm, industry: args.industry, ..Scenario::default() };
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
/// `round` reseeds the bots for a solo rematch (0 is the first match, as
/// it always was): the same island and seats, not the same opening.
fn host_local(args: &Args, me: PeerId, remote: Vec<(PeerId, String)>, sock: Option<Socket>, routes: RouteTable, view: &mut View, round: u64) -> Session {
    let mut names: Vec<(String, bool)> = remote.iter().map(|(_, n)| (n.clone(), false)).collect();
    let me_player = names.len() as PlayerId;
    names.push((args.name.clone(), false));
    let seats = args.players.clamp(names.len(), MAX_PLAYERS);
    while names.len() < seats {
        names.push((format!("bot{}", names.len()), true));
    }
    let first_bot = me_player + 1;
    // The first `--rookies` bots play badly, for a newcomer
    // (PLAYABILITY item 9).
    let bots: Vec<(PlayerId, Box<dyn Brain>)> = (first_bot..seats as PlayerId)
        .map(|i| {
            let seed = args.seed.wrapping_mul(31) ^ (i as u64 + 1) ^ round.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let b: Box<dyn Brain> = if ((i - first_bot) as usize) < args.rookies { Box::new(RuleBot::rookie(seed, "rookie")) } else { Box::new(RuleBot::new(seed, "rulebot")) };
            (i, b)
        })
        .collect();
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
            if !host.is_networked() {
                view.record.0 += 1;
                view.record.1 += (host.world.winner == Some(*me_player)) as u32;
            }
            view.status = match host.world.winner {
                Some(w) if w == *me_player => "MATCH OVER: you win. gg wp".into(),
                Some(w) => format!("MATCH OVER: {} wins. gg wp", host.world.players[w as usize].name),
                None => "MATCH OVER: draw. gg".into(),
            };
        }
    }
}

/// The second match (PLAYABILITY item 10): on a solo match-over screen
/// Enter plays again, the same island and seats with the bots reseeded.
/// A networked match's rematch is the lobby's (roadmap step 8).
fn again(keys: Res<ButtonInput<KeyCode>>, args: Res<Args>, session: Option<ResMut<SessionRes>>, mut view: ResMut<View>, mut cam: ResMut<crate::input::Cam>, mut hud: ResMut<crate::hud::HudState>) {
    let Some(mut session) = session else { return };
    let Session::Hosting { host, .. } = &session.0 else { return };
    if !host.world.over || host.is_networked() || view.keys_taken() || !keys.just_pressed(KeyCode::Enter) {
        return;
    }
    let me = host.me;
    let (round, record) = (view.round + 1, view.record);
    *view = View { round, record, ..View::default() };
    session.0 = host_local(&args, me, vec![], None, RouteTable::default(), &mut view, round);
    view.status = format!("{}: round {}", view.status, round + 1);
    *cam = crate::input::Cam::default();
    *hud = crate::hud::HudState::default();
    info!("again: round {}", round + 1);
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
                let mut s = host_local(&args, me, peers, sock, routes, &mut view, 0);
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
                                Some(w) if Some(w) == client.player => "MATCH OVER: you win. gg wp".into(),
                                Some(w) => format!("MATCH OVER: {} wins. gg wp", obs.players.get(w as usize).map(|p| p.name.as_str()).unwrap_or("?")),
                                None => "MATCH OVER: draw. gg".into(),
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
                // The chair names it (treadcore_sim::chair): a helpdesk tag,
                // then what is wrong with the order, never who.
                view.refused = Some((Instant::now(), format!("{}  ({})", treadcore_sim::chair::tagged(why), treadcore_sim::lingo::quip(treadcore_sim::chair::refusal(why), view.last_tick))));
            }
            Event::StructureDamaged { structure, .. } | Event::StructureWrecked { structure, .. } => {
                if let Some(x) = view.obs.as_ref().and_then(|o| o.structures.iter().find(|x| x.id == *structure)) {
                    view.impacts.push((x.pos, crate::voxel::CONCRETE, if matches!(e, Event::StructureWrecked { .. }) { 16 } else { 4 }));
                }
            }
            Event::LongshotHit { at, .. } => view.impacts.push((*at, earth, 10)),
            _ => {}
        }
        // What my side is losing: on the wire in red's place, with the
        // alarm, and on the minimap (commander.rs).
        if let Some(a) = crate::commander::alert_for(e, me, view) {
            let quiet = view.alerts.back().map(|b| b.when.elapsed().as_secs_f32() < 2.0).unwrap_or(false);
            if !quiet {
                view.sounds.push((Cue::Alarm, Where::Ui, 0.45));
            }
            let tick = view.obs.as_ref().map(|o| o.tick).unwrap_or(view.last_tick);
            view.log.push_back((tick, format!("ALERT {}  (J)", a.text)));
            view.alerts.push_back(a);
            while view.alerts.len() > 16 {
                view.alerts.pop_front();
            }
        }
        if let Some((cue, at, gain)) = cue_for(e, me, view) {
            view.sounds.push((cue, at, gain));
        }
        // The machine around the player: the breech working after each
        // of its own shots, and the alarm when a hit leaves its armour
        // under a third.
        let my_tank = view.obs.as_ref().and_then(|o| o.me.as_ref()).map(|t| t.id);
        match e {
            Event::ShellLaunched { owner, .. } if *owner == me => {
                if let Some(t) = my_tank {
                    view.sounds.push((Cue::Breech, Where::Entity(t), 1.0));
                }
            }
            Event::TankDamaged { tank, armor_left, .. } if Some(*tank) == my_tank && *armor_left < TANK_ARMOR / 3.0 && *armor_left > 0.0 => {
                view.sounds.push((Cue::Alarm, Where::Ui, 0.7));
            }
            _ => {}
        }
        let line = describe(e, me, view);
        if let Some(l) = line.as_ref() {
            debug!("wire: {}", l);
        }
        if let (Event::Chat { .. }, Some(l)) = (e, line.as_ref()) {
            info!("chat: {}", l);
            view.chat = Some((Instant::now(), l.clone()));
        }

        if let Some(l) = line {
            let tick = view.obs.as_ref().map(|o| o.tick).unwrap_or(view.last_tick);
            view.log.push_back((tick, l));
            while view.log.len() > 40 {
                view.log.pop_front();
            }
        }
    }
}

/// What an event sounds like, from where, how loud (sound.rs). The
/// snapshot here is still the one before the event: a tank about to be
/// destroyed is in it; a shell just launched is not, and is found in
/// the next.
fn cue_for(e: &Event, me: PlayerId, view: &View) -> Option<(Cue, Where, f32)> {
    let obs = view.obs.as_ref();
    let at = |id: EntityId| obs.and_then(|o| crate::sound::find(o, id)).map(Where::At).unwrap_or(Where::Entity(id));
    let my_tank = obs.and_then(|o| o.me.as_ref()).map(|t| t.id);
    Some(match e {
        Event::ShellLaunched { shell, owner, kind, .. } => {
            let cue = if *kind == ShellKind::Indirect { Cue::Gun } else { Cue::Cannon };
            (cue, at(*shell), if *owner == me { 1.0 } else { 0.8 })
        }
        Event::ShellImpact { at: p, kind: ShellKind::Indirect, .. } => (Cue::Boom, Where::At(*p), 1.0),
        // A direct round: metal struck if it hit a tank, otherwise dirt,
        // and one in three of those glances off.
        Event::ShellImpact { at: p, kind: ShellKind::Direct, hits, .. } if !hits.is_empty() => (Cue::Strike, Where::At(*p), 1.0),
        Event::ShellImpact { shell, at: p, kind: ShellKind::Direct, .. } if shell % 3 == 0 => (Cue::Ricochet, Where::At(*p), 0.8),
        Event::ShellImpact { at: p, kind: ShellKind::Direct, .. } => (Cue::Pop, Where::At(*p), 0.8),
        Event::MineExploded { at: p, .. } => (Cue::Mine, Where::At(*p), 1.0),
        Event::DroneLaunched { drone, owner, kind: DroneKind::Strike | DroneKind::Longshot } => (Cue::Rocket, at(*drone), if *owner == me { 0.8 } else { 0.6 }),
        Event::DroneLaunched { drone, owner, .. } => (Cue::Launch, at(*drone), if *owner == me { 0.5 } else { 0.35 }),
        Event::Intercepted { target, .. } => (Cue::Intercept, at(*target), 0.7),
        Event::DroneLost { drone, why: LossReason::Shot | LossReason::Splash, .. } => (Cue::Crash, at(*drone), 0.6),
        Event::StrikeHit { tank, .. } => (Cue::RocketHit, at(*tank), 1.0),
        Event::LinkLost { drone, owner } if *owner == me => (Cue::Static, at(*drone), 0.5),
        Event::Resupplied { tank, .. } if Some(*tank) == my_tank => (Cue::Reload, at(*tank), 0.6),
        Event::TankDamaged { tank, .. } if Some(*tank) == my_tank => (Cue::Clang, at(*tank), 1.0),
        Event::TankDamaged { tank, .. } => (Cue::Clang, at(*tank), 0.8),
        Event::TankDestroyed { tank, .. } => (Cue::Wreck, at(*tank), 1.0),
        Event::PillboxFired { pillbox, .. } => (Cue::PillboxFire, at(*pillbox), 0.8),
        Event::PillboxDestroyed { pillbox, .. } => (Cue::Boom, at(*pillbox), 1.0),
        Event::PillboxDamaged { pillbox, .. } => (Cue::Pop, at(*pillbox), 0.6),
        Event::PillboxCaptured { pillbox, by, .. } if *by == me => (Cue::Capture, at(*pillbox), 0.6),
        Event::PillboxCaptured { pillbox, from: Some(f), .. } if *f == me => (Cue::Lost, at(*pillbox), 0.6),
        Event::DepotCaptured { depot, by } if *by == me => (Cue::Capture, at(*depot), 0.6),
        Event::DepotCaptured { depot, .. } if obs.and_then(|o| o.depots.iter().find(|d| d.id == *depot)).and_then(|d| d.owner) == Some(me) => (Cue::Lost, at(*depot), 0.6),
        Event::Built { owner, at: p, .. } | Event::Harvested { owner, at: p, .. } if *owner == me => (Cue::Build, Where::At(*p), 0.5),
        Event::PillboxBuilt { by, at: p, .. } if *by == me => (Cue::Thud, Where::At(*p), 0.8),
        Event::PillboysLost { tank, owner } if *owner == me => (Cue::Lost, at(*tank), 0.6),
        // A proposal to me or my side; and the answer to mine.
        Event::Proposed { from, to, .. } if *from != me && obs.map(|o| o.allied(*to)).unwrap_or(*to == me) => (Cue::Notice, Where::Ui, 0.6),
        Event::ProposalResolved { from, accepted, .. } if *from == me => (if *accepted { Cue::Capture } else { Cue::Lost }, Where::Ui, 0.6),
        Event::AllianceFormed { members, .. } if members.contains(&me) => (Cue::Notice, Where::Ui, 0.6),
        Event::SwarmInbound { from, .. } => (Cue::Swarm, Where::At(*from), 0.5),
        Event::LongshotHit { at: p, .. } => (Cue::Boom, Where::At(*p), 1.0),
        Event::StructureWrecked { structure, .. } => (Cue::Wreck, at(*structure), 1.0),
        Event::StructureDamaged { structure, .. } => (Cue::Clang, at(*structure), 0.7),
        Event::StructureCaptured { structure, by, .. } if *by == me => (Cue::Capture, at(*structure), 0.6),
        Event::StructureCaptured { structure, from: Some(f), .. } if *f == me => (Cue::Lost, at(*structure), 0.6),
        Event::StructureBuilt { by, at: p, .. } if *by == me => (Cue::Thud, Where::At(*p), 0.8),
        Event::Jammed { drone, owner, .. } if *owner == me => (Cue::Static, at(*drone), 0.5),
        Event::Rejected { player, why, .. } if *player == me && why == "no shells" => (Cue::Empty, Where::Ui, 0.6),
        Event::Chat { player, .. } if *player != me => (Cue::Notice, Where::Ui, 0.35),
        Event::Rejected { player, .. } if *player == me => (Cue::Refused, Where::Ui, 0.4),
        Event::MatchOver { winner, team } => {
            let my_team = obs.and_then(|o| o.players.iter().find(|p| p.id == me)).map(|p| p.team);
            let won = *winner == Some(me) || (team.is_some() && *team == my_team);
            (if won { Cue::Win } else { Cue::Lose }, Where::Ui, 0.6)
        }
        _ => return None,
    })
}

fn describe(e: &Event, me: PlayerId, view: &View) -> Option<String> {
    let who = |p: &PlayerId| if *p == me { "you".to_string() } else if *p == SWARM_OWNER { "swarm".to_string() } else { format!("p{}", p) };
    Some(match e {
        Event::Rejected { why, .. } => {
            let c = treadcore_sim::chair::refusal(why);
            format!("order refused: {} [{}, {}]", why, c.label(), c.expansion())
        }
        Event::ShellLaunched { shell, kind: ShellKind::Indirect, spotted, eta_ticks, .. } => format!("{}, {}, {:.1}s", treadcore_sim::lingo::pick(&treadcore_sim::lingo::SHOT, *shell as u64), if *spotted { "spotted" } else { "blind" }, *eta_ticks as f32 * DT),
        Event::ShellLaunched { kind: ShellKind::Direct, .. } => "direct fire".into(),
        Event::ShellImpact { hits, kind, .. } if !hits.is_empty() => format!("{:?} impact: {} hit", kind, hits.len()),
        Event::ShellImpact { .. } => return None,
        Event::DroneLaunched { drone, kind, .. } => treadcore_sim::lingo::pick(&treadcore_sim::lingo::LAUNCH, *drone as u64).replace("{}", treadcore_sim::lingo::fly(*kind)),
        Event::DroneLost { drone, owner, kind, why } => format!("{} lost a {}: {}", who(owner), treadcore_sim::lingo::fly(*kind), treadcore_sim::lingo::lost(*why, *drone as u64)),
        Event::DroneRecovered { kind, .. } => format!("{:?} recovered", kind),
        Event::DroneRetasked { task, .. } => match task {
            DroneTask::Orbit { .. } => "recon orbit moved".into(),
            DroneTask::Strike { .. } => "strike retargeted".into(),
            DroneTask::Return => "drone recalled".into(),
            DroneTask::Intercept { .. } | DroneTask::Attack { .. } => return None,
        },
        Event::Intercepted { owner, interceptor, .. } => treadcore_sim::lingo::pick(&treadcore_sim::lingo::INTERCEPT, *interceptor as u64).replace("{}", &who(owner)),
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
        Event::TankDestroyed { tank, owner, by } => treadcore_sim::lingo::kill(&who(owner), &by.map(|b| who(&b)).unwrap_or_else(|| "swarm".into()), *tank as u64),
        Event::DepotCaptured { by, .. } => format!("{} captured a depot", who(by)),
        Event::Resupplied { .. } => "resupplying".into(),
        Event::SwarmInbound { count, .. } => format!("SWARM: {} inbound", count),
        Event::LinkLost { drone, .. } => treadcore_sim::lingo::pick(&treadcore_sim::lingo::LINK_LOST, *drone as u64).into(),
        Event::Chat { player, text, to } => {
            // By name: a message is from a person, not a seat number.
            let name = if *player == me { "you".to_string() } else { view.obs.as_ref().and_then(|o| o.players.iter().find(|p| p.id == *player)).map(|p| p.name.clone()).unwrap_or_else(|| who(player)) };
            format!("{}{}: {}", name, if *to == ChatTo::Team { " (team)" } else { "" }, text)
        }
        Event::MatchOver { winner, team } => format!("{}{}", treadcore_sim::lingo::match_over(&winner.map(|w| who(&w)).unwrap_or_else(|| "nobody (a draw)".into()), view.last_tick), team.map(|t| format!(" (team {})", t)).unwrap_or_default()),
        Event::FriendlyFireSpared { by, damage, .. } => format!("friendly fire from {} spared ({:.0})", who(by), damage),
        Event::Proposed { from, to, terms, .. } => match view.obs.as_ref() {
            Some(o) => crate::proposals::told(o, *from, *to, *terms, None),
            None => format!("{} proposes {:?} to {}", who(from), terms, who(to)),
        },
        Event::ProposalResolved { from, to, terms, accepted, by, why, .. } => match view.obs.as_ref() {
            Some(o) => crate::proposals::told(o, *from, *to, *terms, Some((*accepted, *by, why.as_deref()))),
            None => format!("{} {:?} {}: {}", who(from), terms, who(to), if *accepted { "accepted" } else { "refused" }),
        },
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
        Event::StructureBuilt { kind, by, .. } => format!("{} built a {}", who(by), crate::commander::name(*kind)),
        // Mine and my allies' are alerts (above); the rest is noise.
        Event::StructureDamaged { .. } => return None,
        Event::StructureWrecked { kind, owner, by, .. } => format!("{} ({}) WRECKED by {}", crate::commander::name(*kind), owner.map(|o| who(&o)).unwrap_or_else(|| "neutral".into()), by.map(|b| who(&b)).unwrap_or_else(|| "?".into())),
        Event::StructureCaptured { kind, by, from, .. } => format!("{} took a {}{}", who(by), crate::commander::name(*kind), from.map(|f| format!(" from {}", who(&f))).unwrap_or_default()),
        Event::StructureRepaired { kind, by, .. } => format!("{} mended a {}", who(by), crate::commander::name(*kind)),
        // Back from dark is told the helpdesk way: "we", and no fuss.
        Event::StructurePower { structure, kind, owner, on: true } if *owner == me => {
            if view.alerts.iter().any(|a| a.about == *structure && a.text.contains("DARK")) {
                format!("your {} has power again. {} {}", crate::commander::name(*kind), treadcore_sim::chair::back_up(*structure), treadcore_sim::lingo::pick(&treadcore_sim::lingo::GREEN, *structure as u64))
            } else {
                format!("your {} has power", crate::commander::name(*kind))
            }
        }
        Event::StructurePower { .. } => return None,
        Event::LongshotHit { owner, hits, .. } => format!("LONGSHOT ({}) {}", who(owner), if hits.is_empty() { "landed on nothing".to_string() } else { format!("hit {}", hits.len()) }),
        Event::Jammed { kind, drone, .. } => treadcore_sim::lingo::pick(&treadcore_sim::lingo::JAMMED, *drone as u64).replace("{}", treadcore_sim::lingo::fly(*kind)),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A refusal is told with its chair's tag, spelled out; power back
    /// after a DARK alert is told the helpdesk way, and the first power
    /// of a structure just taken is not.
    #[test]
    fn the_newswire_tells_refusals_and_recoveries_kindly() {
        let mut view = View::default();
        let refused = Event::Rejected { player: 0, cmd: Command::Stop, why: "water".into() };
        let line = describe(&refused, 0, &view).unwrap();
        assert!(line.contains("water") && line.contains("PICNIC") && line.contains("Problem In Chair, Not In Computer"), "{}", line);
        let on = Event::StructurePower { structure: 9, kind: StructureKind::Refinery, owner: 0, on: true };
        assert_eq!(describe(&on, 0, &view).unwrap(), "your refinery has power");
        view.alerts.push_back(crate::commander::Alert { at: V2::new(1.0, 1.0), text: "your refinery DARK at 1,1: no power".into(), when: Instant::now(), about: 9 });
        let back = describe(&on, 0, &view).unwrap();
        assert!(back.starts_with("your refinery has power again. ") && treadcore_sim::chair::BACK_UP.iter().any(|l| back.contains(l)) && treadcore_sim::lingo::GREEN.iter().any(|l| back.ends_with(l)), "{}", back);
    }
}
