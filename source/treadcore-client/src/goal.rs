//! Goal mode (roadmap step 11a): the player commands the tank instead
//! of driving it. `G` steps the goal: hunt pillboxes, defend the nearest
//! depot of my side, defend the nearest pillbox of mine, hold here,
//! escort the nearest ally, and back to the wheel. A click to drive or
//! Space hands the wheel back at once. The Commander
//! (`treadcore-brain::commander`) thinks once a tick on the observation
//! the window already has and its orders join the player's: the world
//! takes them as it takes a keyboard's, so a goal works the same alone,
//! hosting or as a client.

use crate::session::Orders;
use crate::session::View;
use bevy::prelude::*;
use treadcore_brain::{Brain, Commander, Goal};
use treadcore_sim::*;

pub struct GoalPlugin;

impl Plugin for GoalPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Pilot { brain: Commander::new(0x60A1, "goal"), last: None }).add_systems(Update, pilot);
    }
}

#[derive(Resource)]
pub struct Pilot {
    pub brain: Commander,
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
            Goal::Manual => Goal::HuntPillboxes,
            Goal::HuntPillboxes => Goal::DefendDepot(None),
            Goal::DefendDepot(_) => Goal::DefendPillbox(None),
            Goal::DefendPillbox(_) => Goal::Hold(me),
            Goal::Hold(_) => ally.map(Goal::Escort).unwrap_or(Goal::Manual),
            Goal::Escort(_) => Goal::Manual,
        }
    }

    /// The top strip's words: the goal and what it is doing.
    pub fn status(&self) -> String {
        match self.brain.goal {
            Goal::Manual => String::new(),
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
        } else {
            let cmds = pilot.brain.think(obs);
            orders.0.extend(cmds);
        }
    }
    let status = pilot.status();
    if view.goal != status {
        view.goal = status;
    }
}
