//! Goal mode (roadmap step 11a): the player commands the tank instead
//! of driving it. `G` steps the goal: hunt pillboxes, defend the nearest
//! depot of my side, defend the nearest pillbox of mine, hold here,
//! fortify the nearest depot of my side (step 11b), escort the nearest
//! ally, and back to the wheel. A click to drive or
//! Space hands the wheel back at once. The Commander
//! (`treadcore-brain::commander`) thinks once a tick on the observation
//! the window already has and its orders join the player's: the world
//! takes them as it takes a keyboard's, so a goal works the same alone,
//! hosting or as a client.
//!
//! Coordination (step 11c, first cut): Space puts the coordination
//! screen over the map and hands the tank to Auto, the ladder of rungs
//! (defend base, fortify, turrets, hunt, expand, guard); Space again takes the
//! wheel back. On the screen 1 to 6 turn rungs on and off, G picks a
//! single goal instead, a click on the map holds the tank there, and
//! Tab folds it away and back, U opens the weights page (tune.rs), Esc closes it. What each rung would do and why it does or does not
//! hold is written beside it, and the allies in sight under it.

use crate::session::Orders;
use crate::session::View;
use bevy::prelude::*;
use treadcore_brain::{Brain, Commander, Goal, Rung};
use treadcore_sim::*;

pub struct GoalPlugin;

impl Plugin for GoalPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Pilot { brain: { let mut c = Commander::new(0x60A1, "goal"); c.pings = true; c.answers_pings = true; c }, coord: false, folded: false, last: None }).add_systems(Startup, autopilot).add_systems(Update, (pilot, rotation_marks));
    }
}

#[derive(Resource)]
pub struct Pilot {
    pub brain: Commander,
    /// The coordination screen is up, and folded away (Tab) to see the
    /// map under it while the ladder still drives.
    pub coord: bool,
    pub folded: bool,
    /// The tick last thought on: once a tick, not once a frame.
    last: Option<Tick>,
}

impl Pilot {
    /// The goal after this one, as `G` steps them. Escort is offered only
    /// with an ally's tank in sight.
    pub fn next(&self, obs: &Observation) -> Goal {
        let me = obs.me.as_ref().map(|t| t.pos).unwrap_or_default();
        let ally = obs.tanks.iter().filter(|t| t.alive && t.owner != obs.player && obs.allied(t.owner)).min_by(|a, b| a.pos.dist(me).partial_cmp(&b.pos.dist(me)).unwrap()).map(|t| t.owner);
        match self.brain.goal {
            Goal::Manual => Goal::Auto,
            Goal::Auto => Goal::HuntPillboxes,
            Goal::HuntPillboxes => Goal::DefendDepot(None),
            Goal::DefendDepot(_) => Goal::DefendBase,
            Goal::DefendPillbox(_) => Goal::Hold(me),
            Goal::Hold(_) => Goal::Fortify(None),
            Goal::DefendBase => Goal::DefendPillbox(None),
            Goal::Fortify(_) => ally.map(Goal::Escort).unwrap_or(Goal::Manual),
            Goal::Escort(_) => Goal::Manual,
        }
    }

    /// Space: up with the coordination screen, the tank to Auto unless
    /// a goal is already set; down again, and the wheel is the player's.
    /// True when the tank should stop.
    pub fn toggle(&mut self) -> bool {
        self.coord = !self.coord;
        self.folded = false;
        if self.coord {
            if self.brain.goal == Goal::Manual {
                self.brain.set_goal(Goal::Auto);
            }
            false
        } else {
            self.brain.set_goal(Goal::Manual);
            true
        }
    }

    /// The coordination screen.
    pub fn screen(&self, obs: &Observation, fleet: &[String]) -> String {
        let b = &self.brain;
        let mut t = String::from("COORDINATION  Space: the wheel  1-6: rungs  G: one goal  U: weights  click: hold there  right-click: push point  Tab: fold\n");
        t.push_str(&format!("goal: {}{}\n", b.goal.label(), if b.doing.is_empty() { String::new() } else { format!(": {}", b.doing) }));
        t.push_str(if b.goal == Goal::Auto { "THE LADDER  the first rung on whose condition holds is what the tank does\n" } else { "THE LADDER  (not in use: G back to auto)\n" });
        for r in Rung::ALL {
            let i = r.index();
            let now = b.goal == Goal::Auto && b.rung == Some(r);
            t.push_str(&format!(" {} [{}] {} {:<12} {}\n", if now { ">" } else { " " }, if b.ladder[i] { "x" } else { " " }, i + 1, r.label(), b.why_text(i)));
        }
        if let Some((defense, push)) = b.rotation_points(obs) {
            t.push_str(&format!(
                "\nROTATION  defense ({:.0},{:.0})  push ({:.0},{:.0}) {}\n",
                defense.x,
                defense.y,
                push.x,
                push.y,
                if b.rotation.own.is_some() { "your own (right-click it to share the fleet's)" } else { "the fleet's, halfway to the objective" }
            ));
        }
        if let Some(me) = obs.me.as_ref() {
            let s = &me.pillboys;
            t.push_str(&format!("\nYOU   armour {:.0}  shells {}  squad {} {}{}  material {}\n", me.armor, me.shells, s.strength, if s.aboard { "aboard" } else { "out" }, if s.carrying.is_some() { ", carrying a pillbox" } else { "" }, s.material));
            let allies: Vec<&Tank> = obs.tanks.iter().filter(|t| t.alive && t.owner != obs.player && obs.allied(t.owner)).collect();
            if allies.is_empty() {
                t.push_str("ALLIES none in sight\n");
            }
            for a in allies {
                let name = obs.players.iter().find(|p| p.id == a.owner).map(|p| p.name.as_str()).unwrap_or("?");
                t.push_str(&format!("ALLY  {:<10} {:>3.0} tiles  armour {:.0}  shells {}\n", name, a.pos.dist(me.pos), a.armor, a.shells));
            }
        }
        if !fleet.is_empty() {
            t.push_str("FLEET\n");
            for f in fleet {
                t.push_str(&format!("  {}\n", f));
            }
        }
        t
    }

    /// The top strip's words: the goal and what it is doing.
    pub fn status(&self) -> String {
        match self.brain.goal {
            Goal::Manual => String::new(),
            Goal::Auto => match self.brain.rung {
                Some(r) => format!("GOAL auto > {}: {}", r.label(), self.brain.doing),
                None => "GOAL auto".into(),
            },
            g if self.brain.doing.is_empty() => format!("GOAL {}", g.label()),
            g => format!("GOAL {}: {}", g.label(), self.brain.doing),
        }
    }
}

/// `--autopilot`: the tank starts on Auto, the player watching, and
/// leads its team as a bots' leader does: the side's fleet commander,
/// who musters the crews and hands out the fort (the host deals the
/// team as a bots' team for it, session.rs).
fn autopilot(args: Res<crate::Args>, mut pilot: ResMut<Pilot>) {
    if args.autopilot {
        pilot.brain.fleet.on = true;
        pilot.brain.set_goal(Goal::Auto);
    }
}

fn pilot(mut view: ResMut<View>, mut orders: ResMut<Orders>, mut pilot: ResMut<Pilot>) {
    let Some(obs) = view.obs.as_ref() else { return };
    if pilot.last != Some(obs.tick) {
        pilot.last = Some(obs.tick);
        if obs.over {
            pilot.brain.set_goal(Goal::Manual);
            pilot.coord = false;
            view.unthought.clear();
        } else {
            // The brain hears every tick's events, not only this one's.
            let back = obs.respawn_in == Some(0) && obs.tick % TICK_HZ as Tick == 0;
            // The window's log, every 30 s: what the pilot is doing.
            let log = (obs.tick % (30 * TICK_HZ as Tick) == 0).then(|| (obs.tick as f32 * DT, obs.me.as_ref().map(|t| (t.armor, t.shells)).unwrap_or_default()));
            // The fleet: this seat leads its team, and its team's
            // wingmen (fleet.rs `fleet_of`, a person's team) are the
            // fleet it gathers and gives its calls to.
            if pilot.brain.members.is_empty() {
                let mine = obs.my_team();
                let mut team: Vec<usize> = obs.players.iter().filter(|p| p.team == mine).map(|p| p.id as usize).collect();
                team.sort();
                if let Some(i) = team.iter().position(|s| *s == obs.player as usize) {
                    team.rotate_left(i);
                    treadcore_brain::fleet::fleet_of(&mut pilot.brain, obs.player as usize, &team, true);
                }
            }
            let mut seen = obs.clone();
            seen.events = std::mem::take(&mut view.unthought);
            let cmds = pilot.brain.think_on(&seen, view.map.as_ref());
            orders.0.extend(cmds);
            // On the autopilot, the tank comes back by itself when the
            // match allows it, as a bot's does (V by hand otherwise).
            if pilot.brain.goal != Goal::Manual && back {
                orders.0.push(treadcore_sim::Command::Respawn);
            }
            if let (Some((s, (armor, shells))), false) = (log, pilot.brain.goal == Goal::Manual) {
                info!("pilot at {:.0}s: armor {:.0} shells {} {}", s, armor, shells, pilot.status());
            }
        }
    }
    let status = pilot.status();
    if view.goal != status {
        view.goal = status;
    }
    let screen = if pilot.coord && !pilot.folded { view.obs.as_ref().map(|o| pilot.screen(o, &view.fleet)) } else { None };
    if view.coord != screen {
        view.coord = screen;
    }
}

/// On the coordination screen, the rotation's ends on the map: a ring
/// at the defense point, a cross at the push point, and a faint line
/// between.
fn rotation_marks(view: Res<View>, pilot: Res<Pilot>, mut gizmos: Gizmos) {
    if !pilot.coord {
        return;
    }
    let Some(obs) = view.obs.as_ref() else { return };
    let Some((defense, push)) = pilot.brain.rotation_points(obs) else { return };
    let c = crate::player_color(obs.player);
    let (d, p) = (crate::to_screen(defense), crate::to_screen(push));
    gizmos.circle_2d(d, crate::TILE * 1.2, c);
    gizmos.cross_2d(p, crate::TILE * 1.2, c);
    gizmos.line_2d(d, p, c.with_alpha(0.25));
}
