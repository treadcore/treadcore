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
//! Tab folds it away and back, Esc closes it. What each rung would do and why it does or does not
//! hold is written beside it, and the allies in sight under it.

use crate::session::Orders;
use crate::session::View;
use bevy::prelude::*;
use treadcore_brain::{Brain, Commander, Goal, Rung};
use treadcore_sim::*;

pub struct GoalPlugin;

impl Plugin for GoalPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Pilot { brain: Commander::new(0x60A1, "goal"), coord: false, folded: false, last: None }).add_systems(Update, pilot);
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
        let mut t = String::from("COORDINATION  Space: the wheel  1-6: rungs  G: one goal  click: hold there  Tab: fold\n");
        t.push_str(&format!("goal: {}{}\n", b.goal.label(), if b.doing.is_empty() { String::new() } else { format!(": {}", b.doing) }));
        t.push_str(if b.goal == Goal::Auto { "THE LADDER  the first rung on whose condition holds is what the tank does\n" } else { "THE LADDER  (not in use: G back to auto)\n" });
        for r in Rung::ALL {
            let i = r.index();
            let now = b.goal == Goal::Auto && b.rung == Some(r);
            t.push_str(&format!(" {} [{}] {} {:<12} {}\n", if now { ">" } else { " " }, if b.ladder[i] { "x" } else { " " }, i + 1, r.label(), b.why[i]));
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
            let mut seen = obs.clone();
            seen.events = std::mem::take(&mut view.unthought);
            let cmds = pilot.brain.think(&seen);
            orders.0.extend(cmds);
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
