//! Tank speak: the cyberpunk, l33t voice of the newswire and the bots.
//!
//! The people at the controls are geekers. A tank is a rig, its shells
//! slugs, its drones flies; the squad are grunts, material is scrap,
//! and the map with its economy is the grid. The flies talk as FPV
//! pilots do: an interceptor is a whoop (small and ducted), a recon a
//! toothpick (ultra-light); a launch is "send it", a feed breaking up
//! is jello, and the VTX and VRX are the two ends of a link. There is
//! no settled slang for tanks driven by video, so the rig's words are
//! treadcore's own, with some from real tank crews (Power!, bite the
//! dust, pigiron, a bolo crew) and from the drone war the longshot comes
//! from (kami, loitering munition, the kill-zone). Slang aimed at people,
//! as a good deal of real crew slang is, is left out on purpose. Kills, the end of a match,
//! a bot going in and something coming back up are said several ways,
//! the way a crew that has been at it all night says them.
//!
//! Like `chair`, nothing here changes a rule: the world's events and
//! reasons stay plain (bots and tests read them), and a window or a
//! bot picks a line from a bank here. A pick is by a number the caller
//! already has (a tick, an id), never by the world's Rng, so a match
//! plays out the same to the digit whatever is said in it. The voice is
//! friendly: gg and gl hf, no line aimed at a person.

/// What the people at the controls are called.
pub const GEEKER: &str = "geeker";
pub const GEEKERS: &str = "geekers";

/// The lexicon, for the guide: tank speak, then plain.
pub const LEXICON: [(&str, &str); 40] = [
    ("geeker", "a player, at the controls"),
    ("rig, pigiron", "a tank"),
    ("slugs", "shells"),
    ("flies", "drones"),
    ("whoop", "an interceptor: small, ducted, in the bubble"),
    ("toothpick", "a recon: ultra-light, eyes only"),
    ("kami", "a strike drone: one way, into a rig"),
    ("loiter, loitering munition", "a longshot: slow, far, one way, at a fixed point"),
    ("grunts", "the squad of pillboys"),
    ("scrap", "material"),
    ("pack", "a fly's battery"),
    ("fuzz", "a jammer, and what it does to a link"),
    ("the grid", "the map and the economy on it"),
    ("kill-zone", "the ground inside the enemy's flies' reach"),
    ("cost-exchange ratio", "a cheap loiter against a refinery: why the economy is the target"),
    ("send it, rip it", "commit: launch, fire, go in"),
    ("punch out, WOT", "full throttle, all at once (wide open throttle)"),
    ("gap, shoot the gap", "a tight way through, threaded"),
    ("on the way, shot over", "the artillery's calls: a slug is in the air"),
    ("Power!", "the gun is about to move: the old tank crews' warning"),
    ("jello, ghosting", "a feed shaking apart, or two feeds over each other: a link going, or jammed"),
    ("LQ", "link quality: the % beside a fly in the air line"),
    ("failsafed, RTH", "the link is gone and the fly returns to home"),
    ("browned out", "the pack sagged flat in the air"),
    ("pancaked", "shot down, flat"),
    ("magic smoke", "what comes out of a fly caught in a blast"),
    ("VTX, VRX", "the fly's video transmitter, the rig's receiver: the link"),
    ("OSD", "the on-screen display: the frame around the map"),
    ("rekt, pwned, fragged, uninstalled", "destroyed"),
    ("bite the dust", "destroyed, the old way"),
    ("going in hot", "storming a depot"),
    ("bolo crew", "a crew that fails gunnery: the rookies"),
    ("meatware, wetware", "the geeker, as opposed to the rig"),
    ("rig's green", "it works again"),
    ("tanker ears", "what the M key spares"),
    ("dronovod", "a drone operator: a geeker flying"),
    ("spotter", "whoever watches for the geeker: a recon's job"),
    ("disarm, disarm!", "what is shouted when a fly goes down near its own rig"),
    ("gl hf", "good luck, have fun: a match begins"),
    ("gg wp", "good game, well played: a match ends"),
];

/// One line of a bank, by a number the caller has: the same number, the
/// same line; the next number, the next line.
pub fn pick(bank: &'static [&'static str], n: u64) -> &'static str {
    // A small mix, so ticks that step by four do not walk the bank in
    // step with something else that does.
    let h = n.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 32;
    bank[(h % bank.len() as u64) as usize]
}

/// A rig destroyed: `{v}` the victim, `{k}` who did it.
/// Every line reads with any name, "you" included (the newswire calls
/// the player that): no "{k}'s", no verb agreeing with a name.
pub const KILLS: [&str; 9] = ["{v} DESTROYED by {k}", "{v} REKT by {k}", "{v} PWNED by {k}", "{v} FRAGGED by {k}", "{v} UNINSTALLED by {k}", "{v} SCRAPPED by {k}", "{v} BIT THE DUST: a slug from {k}", "pigiron in the dirt: {v}, by {k}", "{v} pancaked by {k}"];

/// "p4 FRAGGED by p2", a different line for a different number.
pub fn kill(victim: &str, by: &str, n: u64) -> String {
    pick(&KILLS, n).replace("{v}", victim).replace("{k}", by)
}

/// A fly's name in tank speak, with its plain kind after it.
pub fn fly(kind: crate::DroneKind) -> &'static str {
    match kind {
        crate::DroneKind::Interceptor => "whoop (Interceptor)",
        crate::DroneKind::Recon => "toothpick (Recon)",
        crate::DroneKind::Strike => "kami (Strike)",
        crate::DroneKind::Longshot => "loiter (Longshot)",
    }
}

/// A fly in the air; `{}` is its name.
pub const LAUNCH: [&str; 5] = ["{} airborne", "{} off the pad: send it", "{} up, WOT", "{} punched out", "{} away. ripping a pack"];

/// A fly gone, by why.
pub fn lost(why: crate::LossReason, n: u64) -> &'static str {
    use crate::LossReason::*;
    let bank: &'static [&'static str] = match why {
        Battery => &["browned out", "pack's flat", "pack sagged: down"],
        Shot => &["pancaked", "shot down: magic smoke", "pancaked. disarm, disarm!"],
        Expended => &["spent", "did its job", "one way, as built"],
        Splash => &["magic smoke", "caught in the splash", "prop wash and shrapnel"],
        OwnerDead => &["failsafed: no rig to fly for", "orphaned on failsafe", "RTH with no home"],
    };
    pick(bank, n)
}

/// Our own indirect slug, called as the artillery calls it.
pub const SHOT: [&str; 4] = ["round out", "on the way", "shot, over", "slug out. Power!"];

/// The end of a match; `{}` is the winner, or "nobody" for a draw.
pub const MATCH_OVER: [&str; 4] = ["gg wp. last rig rolling: {}", "gg. holding the grid: {}", "match over, gg. the grid goes to {}", "gg wp. last geeker standing: {}"];

pub fn match_over(winner: &str, n: u64) -> String {
    pick(&MATCH_OVER, n).replace("{}", winner)
}

/// A bot going in on a depot, to its team.
pub const STORM: [&str; 10] = ["storming the depot", "going in hot on the depot, geekers", "rolling on the depot. cover me", "depot's ours in ten. go go go", "slugs out, grunts ready: hitting the depot", "pushing the depot. gl hf", "send it: depot", "punching out for the depot. rip it", "WOT on the depot. shooting the gap", "into the kill-zone for the depot. gl hf"];

/// A fly's link gone (out of reach, or its rig dead): it comes home.
pub const LINK_LOST: [&str; 6] = ["link lost: drone returning", "VTX out of reach: fly coming home", "jello, then static: fly returning", "feed's gone. fly flying home on its own", "failsafed: fly on RTH", "LQ zero: fly returning to home"];

/// A fly jammed by a jammer, by kind's name (`{}`).
pub const JAMMED: [&str; 6] = ["{} jammed: link gone", "fuzz on the VRX: {} lost its link", "{} feed all jello: jammed", "{} in the fuzz: VTX drowned out", "{} ghosting under the fuzz: link gone", "{} LQ zero: jammed"];

/// An interceptor's kill (`{}` is whose).
pub const INTERCEPT: [&str; 4] = ["intercept ({}): KILL", "a whoop threaded the gap ({}): KILL", "whoop sent it ({}): KILL", "clean through the gap ({}): KILL"];

/// After the helpdesk's "we're back up": the rig, said green.
pub const GREEN: [&str; 6] = ["Rig's green.", "Grid's back.", "Uplink's solid.", "Systems green, geekers.", "Back online.", "Locked in."];

/// The quip after a chair's tag, by tag: the fault is in the meatware,
/// never in a geeker's worth.
pub fn quip(chair: crate::chair::Chair, n: u64) -> &'static str {
    use crate::chair::Chair::*;
    let bank: &'static [&'static str] = match chair {
        Picnic => &["the fault's in the geeker, not the rig", "rig's fine. the grid said no", "computer says no; the chair says why"],
        SixtyCm => &["aim lives 60 cm from the glass", "recalibrate the meatware", "optics are fine. it's the wetware"],
        Pebkac => &["the lamps called it", "check the lamps, geeker", "out of juice, not out of luck"],
        LooseNut => &["tighten the nut behind the keys", "wrong rig, right idea", "that order went to the void"],
        KeyboardChair => &["comms jam at the chair", "say less, geeker", "the treaty wire's busy"],
        Id10t => &["read it back slow, geeker", "typo in the uplink", "the rig's fine. the flag isn't"],
    };
    pick(bank, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chair::Chair;

    /// Every bank says more than one thing, picks are stable, and the
    /// voice stays friendly: no "you", and nothing in the list of words
    /// a helpdesk does not say to a person.
    #[test]
    fn tank_speak_is_varied_stable_and_friendly() {
        assert_eq!(pick(&KILLS, 7), pick(&KILLS, 7));
        let kills: std::collections::BTreeSet<&str> = (0..64).map(|n| pick(&KILLS, n)).collect();
        assert!(kills.len() >= 4, "{:?}", kills);
        for n in 0..32 {
            let k = kill("p4", "p2", n);
            assert!(k.contains("p4") && k.contains("p2") && !k.contains('{'), "{}", k);
        }
        assert!(match_over("bot4", 1).contains("bot4") && match_over("bot4", 1).contains("gg"));
        let mut all: Vec<&str> = KILLS.iter().chain(&MATCH_OVER).chain(&STORM).chain(&GREEN).chain(&LINK_LOST).chain(&JAMMED).chain(&INTERCEPT).chain(&LAUNCH).chain(&SHOT).copied().collect();
        for w in [crate::LossReason::Battery, crate::LossReason::Shot, crate::LossReason::Expended, crate::LossReason::Splash, crate::LossReason::OwnerDead] {
            let l: std::collections::BTreeSet<&str> = (0..64).map(|n| lost(w, n)).collect();
            assert!(l.len() >= 2, "{:?}", w);
            all.extend(l);
        }
        // With the player's own name, "you", every line still reads.
        for line in KILLS.iter().chain(&MATCH_OVER).chain(&INTERCEPT) {
            for bad in ["you's", "you is", "you holds", "you owns", "you whoop", "you intercept"] {
                let said = line.replace("{v}", "you").replace("{k}", "you").replace("{}", "you");
                assert!(!said.contains(bad), "{:?} says {:?}", said, bad);
            }
        }
        for (w, m) in LEXICON {
            all.push(w);
            all.push(m);
        }
        for c in [Chair::Pebkac, Chair::Picnic, Chair::SixtyCm, Chair::LooseNut, Chair::KeyboardChair, Chair::Id10t] {
            let q: std::collections::BTreeSet<&str> = (0..64).map(|n| quip(c, n)).collect();
            assert!(q.len() >= 2, "{:?} has one quip", c);
            all.extend(q);
        }
        for line in all {
            let l = line.to_lowercase();
            for bad in ["you", "idiot", "stupid", "noob", "n00b", "loser", "trash", "karen", "crunchie", "zipperhead", "communists"] {
                assert!(!l.split(|c: char| !c.is_alphanumeric()).any(|w| w == bad), "{:?} says {:?}", line, bad);
            }
        }
    }
}
