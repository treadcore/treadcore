//! Status as codes (roadmap step 12b, finding L3 of LAB-REPORT-BRAIN.md,
//! section 22). What a tank is doing was only ever a sentence, and the
//! brain's own logic and the census read the sentence back: a guard was
//! idle if its words began "on guard", a focus call went out if the
//! reason said "called", the census counted "ducking" by substring. A
//! rewording broke them silently. Now each sentence is written with a
//! code beside it, and logic and measurement read the code; the words
//! are for people, the code for the program.
//!
//! The codes are `Copy` enums, set where the words are written
//! (`Commander::set_doing`, `Hop::say`, the cycle's `act`). `code()` is
//! a stable dotted name for logs, the census and the ledger.

/// A kiting step (tactics.rs `kite`): where the tank stands against its
/// target and why.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KiteStep {
    #[default]
    Mass,
    Close,
    StepInReloading,
    StepInBusy,
    StepOut,
    HoldReloading,
    HoldCovered,
    HoldLoaded,
}

impl KiteStep {
    pub const ALL: [KiteStep; 8] = [KiteStep::Mass, KiteStep::Close, KiteStep::StepInReloading, KiteStep::StepInBusy, KiteStep::StepOut, KiteStep::HoldReloading, KiteStep::HoldCovered, KiteStep::HoldLoaded];

    /// The words, as the status line has always had them.
    pub fn word(self) -> &'static str {
        match self {
            KiteStep::Mass => "massing out of its reach",
            KiteStep::Close => "closing: it has no shells",
            KiteStep::StepInReloading => "stepping in: its gun is reloading",
            KiteStep::StepInBusy => "stepping in: it is busy with the group",
            KiteStep::StepOut => "stepping out to reload",
            KiteStep::HoldReloading => "holding past its reach, reloading",
            KiteStep::HoldCovered => "holding: other guns cover the way in",
            KiteStep::HoldLoaded => "holding past its reach, gun loaded",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            KiteStep::Mass => "mass",
            KiteStep::Close => "close",
            KiteStep::StepInReloading => "step_in.reloading",
            KiteStep::StepInBusy => "step_in.busy",
            KiteStep::StepOut => "step_out",
            KiteStep::HoldReloading => "hold.reloading",
            KiteStep::HoldCovered => "hold.covered",
            KiteStep::HoldLoaded => "hold.loaded",
        }
    }

    pub fn stepping_in(self) -> bool {
        matches!(self, KiteStep::Close | KiteStep::StepInReloading | KiteStep::StepInBusy)
    }
}

/// An island hop's step (hop.rs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HopAct {
    #[default]
    Idle,
    BySea,
    GaveUp,
    Bridging,
    BridgeBuilt,
    BridgeGivenUp,
    /// A teammate's bridge, taken from the board, laid (roadmap 12f).
    TeamBridgeDone,
    RoadOn,
    Arrived,
    AtSea,
    Sailing,
    Driving,
    CheaperByBoat,
    HoldingLaunch,
    ToBoat,
    SquadOut,
    Launching,
    ToBank,
    BuildingBoat,
    SeekingBank,
    Restocking,
    Felling,
    SeekingForest,
}

impl HopAct {
    pub fn code(self) -> &'static str {
        match self {
            HopAct::Idle => "idle",
            HopAct::BySea => "by_sea",
            HopAct::GaveUp => "gave_up",
            HopAct::Bridging => "bridging",
            HopAct::BridgeBuilt => "bridge_built",
            HopAct::BridgeGivenUp => "bridge_given_up",
            HopAct::TeamBridgeDone => "team_bridge_done",
            HopAct::RoadOn => "road_on",
            HopAct::Arrived => "arrived",
            HopAct::AtSea => "at_sea",
            HopAct::Sailing => "sailing",
            HopAct::Driving => "driving",
            HopAct::CheaperByBoat => "cheaper_by_boat",
            HopAct::HoldingLaunch => "holding_launch",
            HopAct::ToBoat => "to_boat",
            HopAct::SquadOut => "squad_out",
            HopAct::Launching => "launching",
            HopAct::ToBank => "to_bank",
            HopAct::BuildingBoat => "building_boat",
            HopAct::SeekingBank => "seeking_bank",
            HopAct::Restocking => "restocking",
            HopAct::Felling => "felling",
            HopAct::SeekingForest => "seeking_forest",
        }
    }
}

/// A base cycle's step (cycle.rs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CycleAct {
    #[default]
    SquadOut,
    Felling,
    Item,
    Mend,
    Mine,
    Works,
    Road,
}

impl CycleAct {
    /// The words of an improvement (`Cycle::improvement`).
    pub fn word(self) -> &'static str {
        match self {
            CycleAct::SquadOut => "base cycle: the squad is out",
            CycleAct::Felling => "base cycle: felling for the item",
            CycleAct::Item => "base cycle: one item for the base",
            CycleAct::Mend => "base cycle: mending the base",
            CycleAct::Mine => "base cycle: a mine on the approach",
            CycleAct::Works => "base cycle: works for the base",
            CycleAct::Road => "base cycle: road out",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            CycleAct::SquadOut => "squad_out",
            CycleAct::Felling => "felling",
            CycleAct::Item => "item",
            CycleAct::Mend => "mend",
            CycleAct::Mine => "mine",
            CycleAct::Works => "works",
            CycleAct::Road => "road",
        }
    }
}

/// What a Commander is doing: the code beside its `doing` words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Act {
    #[default]
    Idle,
    Down,
    Dead,
    Dodge,
    Duck,
    Revive,
    Chain,
    Rally,
    /// On a rally, kiting its target.
    RallyKite(KiteStep),
    RallyLost,
    Ping,
    BaseThreat,
    Fortify,
    Turrets,
    Fetch,
    /// A far fetch, the fleet gathered first (`escort`).
    FetchGather,
    RoadPost,
    CarryHome,
    Gather,
    TakePillbox,
    TakeDepot,
    NoTarget,
    NoSquad,
    Wing,
    WingKite(KiteStep),
    WingSea,
    Leave,
    /// Kiting an intruder at the base.
    Kite(KiteStep),
    Engage,
    Cover,
    ToPost,
    NoWayToPost,
    OnGuard,
    Mend,
    Retake,
    Defend,
    Escort,
    Rotation,
    Resupply,
    Relieved,
    Relieving,
    Scoot,
    TakeCommand,
    Crew,
    Hop(HopAct),
    /// Taking a teammate's part-laid bridge from the board.
    TeamBridge,
    Cycle(CycleAct),
    NoBoat,
}

impl Act {
    /// A stable dotted name: `kite.step_in.reloading`, `hop.bridging`.
    pub fn code(self) -> String {
        match self {
            Act::WingKite(k) => format!("wing_kite.{}", k.code()),
            Act::Kite(k) => format!("kite.{}", k.code()),
            Act::RallyKite(k) => format!("rally_kite.{}", k.code()),
            Act::Hop(h) => format!("hop.{}", h.code()),
            Act::Cycle(c) => format!("cycle.{}", c.code()),
            a => Self::plain(a).to_string(),
        }
    }

    fn plain(self) -> &'static str {
        match self {
            Act::Idle => "idle",
            Act::Down => "down",
            Act::Dead => "dead",
            Act::Dodge => "dodge",
            Act::Duck => "duck",
            Act::Revive => "revive",
            Act::Chain => "chain",
            Act::Rally => "rally",
            Act::RallyLost => "rally_lost",
            Act::Ping => "ping",
            Act::BaseThreat => "base_threat",
            Act::Fortify => "fortify",
            Act::Turrets => "turrets",
            Act::Fetch => "fetch",
            Act::FetchGather => "fetch_gather",
            Act::RoadPost => "road_post",
            Act::CarryHome => "carry_home",
            Act::Gather => "gather",
            Act::TakePillbox => "take_pillbox",
            Act::TakeDepot => "take_depot",
            Act::NoTarget => "no_target",
            Act::NoSquad => "no_squad",
            Act::Wing => "wing",
            Act::WingSea => "wing_sea",
            Act::Leave => "leave",
            Act::Engage => "engage",
            Act::Cover => "cover",
            Act::ToPost => "to_post",
            Act::NoWayToPost => "no_way_to_post",
            Act::OnGuard => "on_guard",
            Act::Mend => "mend",
            Act::Retake => "retake",
            Act::Defend => "defend",
            Act::Escort => "escort",
            Act::Rotation => "rotation",
            Act::Resupply => "resupply",
            Act::Relieved => "relieved",
            Act::Relieving => "relieving",
            Act::Scoot => "scoot",
            Act::TakeCommand => "take_command",
            Act::Crew => "crew",
            Act::TeamBridge => "team_bridge",
            Act::NoBoat => "no_boat",
            Act::WingKite(_) | Act::Kite(_) | Act::RallyKite(_) | Act::Hop(_) | Act::Cycle(_) => unreachable!(),
        }
    }

    /// The kiting step in it, if any.
    pub fn kite(self) -> Option<KiteStep> {
        match self {
            Act::Kite(k) | Act::WingKite(k) | Act::RallyKite(k) => Some(k),
            _ => None,
        }
    }
}

/// Why a rung of the Auto ladder holds or does not (the coordination
/// screen's column): the code and the numbers, put into words only when
/// the screen asks (`Commander::why_text`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Why {
    #[default]
    Unknown,
    /// DefendBase.
    ThreatTank(f32),
    ThreatPillbox,
    Quiet { secs: f32, within: f32 },
    /// Fortify: the plan's progress, put in words from the plan.
    Building { starved: bool },
    NothingBuilt,
    /// Turrets.
    CastleFirst,
    TurretsStand,
    Walling,
    WaitingQuiet { need: f32, now: f32 },
    Carrying,
    Outlying(f32),
    NoOutlying,
    /// Hunt.
    PillboxNear(f32),
    PillboxFar { within: f32, nearest: f32 },
    AllPillboxesOurs,
    /// Expand.
    DepotNear(f32),
    DepotsOursPillboxesFar,
    MapOurs,
    /// Guard.
    Always,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_its_own() {
        let mut codes: Vec<String> = KiteStep::ALL.iter().map(|k| Act::Kite(*k).code()).collect();
        codes.extend(KiteStep::ALL.iter().map(|k| Act::WingKite(*k).code()));
        codes.extend(KiteStep::ALL.iter().map(|k| Act::RallyKite(*k).code()));
        codes.extend([Act::Duck, Act::Revive, Act::OnGuard, Act::ToPost, Act::TeamBridge, Act::Hop(HopAct::Bridging), Act::Cycle(CycleAct::Mine)].iter().map(|a| a.code()));
        let n = codes.len();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), n, "two codes the same");
        assert_eq!(Act::Kite(KiteStep::StepInReloading).code(), "kite.step_in.reloading");
        assert!(KiteStep::StepInBusy.stepping_in() && !KiteStep::HoldLoaded.stepping_in());
    }
}
