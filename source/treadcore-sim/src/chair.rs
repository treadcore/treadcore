//! The chair: how treadcore names a problem the player can fix.
//!
//! Every helpdesk has names for the fault that sits sixty centimetres in
//! front of the screen. treadcore uses them as labels, never as blame:
//! the label carries the joke, and the line under it says what is wrong
//! with the thing ("x is broken"), not what the person did ("you broke
//! x"). When something mends itself the line is "we": "we're back up",
//! never "I fixed it".
//!
//! Nothing here changes a rule. The world's refusal reasons stay as
//! they are (bots and tests read them); a window or a server log puts
//! a label in front of them with `refusal`, and a command line's
//! mistake is `Chair::Id10t`.

/// Which helpdesk name a problem goes under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chair {
    /// Problem Exists Between Keyboard And Chair: the order wants what
    /// the lamps already say is not there (shells, material, the bay,
    /// the squad).
    Pebkac,
    /// Problem In Chair, Not In Computer: the world is working as made
    /// (water is water, a tile holds one thing, a pillbox is shot dead
    /// before it is taken).
    Picnic,
    /// The 60 Centimetre Error: the fault is about sixty centimetres in
    /// front of the screen, where the aiming happens. Range, sight, the
    /// edge of the map.
    SixtyCm,
    /// A loose nut behind the keyboard: the order went to the wrong
    /// thing, or to a thing that is not there any more.
    LooseNut,
    /// A keyboard-to-chair interface fault: talking and treaties.
    KeyboardChair,
    /// An ID-10-T error (write it down and read it back): a flag, a
    /// number or a file on the command line.
    Id10t,
}

impl Chair {
    /// The tag, as it goes in front of a line.
    pub fn label(self) -> &'static str {
        match self {
            Chair::Pebkac => "PEBKAC",
            Chair::Picnic => "PICNIC",
            Chair::SixtyCm => "60 CM",
            Chair::LooseNut => "LOOSE NUT",
            Chair::KeyboardChair => "KB-TO-CHAIR",
            Chair::Id10t => "ID-10-T",
        }
    }

    /// The tag spelled out, for the scrollback and the guide.
    pub fn expansion(self) -> &'static str {
        match self {
            Chair::Pebkac => "Problem Exists Between Keyboard And Chair",
            Chair::Picnic => "Problem In Chair, Not In Computer",
            Chair::SixtyCm => "the error is 60 centimetres in front of the screen",
            Chair::LooseNut => "a loose nut behind the keyboard",
            Chair::KeyboardChair => "a keyboard-to-chair interface fault",
            Chair::Id10t => "an ID-10-T error",
        }
    }
}

/// The name for a reason the world gave when it refused an order. A
/// reason it has never given is a PEBKAC: the default at any helpdesk.
pub fn refusal(why: &str) -> Chair {
    known(why).unwrap_or(Chair::Pebkac)
}

fn known(why: &str) -> Option<Chair> {
    Some(match why {
        "inside minimum range" | "beyond maximum range" | "beyond direct range" | "no line of sight" | "off map" => Chair::SixtyCm,
        "no shells" | "reloading" | "no material" | "material is full" | "bay empty for that kind" | "airspace full" | "squad is aboard" | "squad already out" | "no squad" | "no tank" | "tank destroyed" | "not on a depot" => Chair::Pebkac,
        "task does not suit that drone" | "interceptors are the world's to task" | "a longshot flies where it was sent" | "not my drone" | "no such drone" | "no such pillbox" | "no such tank" | "no such player" | "no such proposal" | "no such request" | "that tank is an ally" | "seat is already a person's" => Chair::LooseNut,
        "one message a second" | "nothing to say" | "nobody chosen to say it to" | "already on the same side" | "leave your alliance first" | "not in an alliance" | "fewer than three sides would remain" | "one side would hold more than half the tanks" => Chair::KeyboardChair,
        "water" | "unreachable" | "site unreachable" | "a tank is there" | "a pillbox is there" | "a mine is there" | "a structure is there" | "structures go on a pad" | "cannot build there" | "cannot lay a road there" | "no forest there" | "forest: harvest it first" | "pillbox is live and theirs: shoot it dead first" | "structure is standing and theirs: wreck it first" | "pillbox is an ally's" | "structure is an ally's" | "pillbox already ours" | "structure already ours" | "pillbox needs no repair" | "structure needs no repair" | "only a pillbox of your own can be lifted" | "already carrying one" => Chair::Picnic,
        _ => return None,
    })
}

/// A refusal as one short line: the tag, then the world's reason.
pub fn tagged(why: &str) -> String {
    format!("{}: {}", refusal(why).label(), why)
}

/// A command line's mistake, for stderr: the tag, the mistake, and a
/// line that says nothing is broken and where to look.
pub fn usage(program: &str, what: &str) -> String {
    format!("{}: {}: {}\n{}: ({}: nothing is broken; the command line wants a second look, and --help lists every flag. {}.)", program, Chair::Id10t.label(), what, program, Chair::Id10t.expansion(), crate::lingo::quip(Chair::Id10t, what.len() as u64))
}

/// What is said when something that was broken works again by itself.
/// "We", and no fuss; the n-th recovery gets the n-th line, so a log
/// does not say the same thing twice running.
pub const BACK_UP: [&str; 4] = ["We're back up.", "Well, it's working now. That's the main thing.", "We're good.", "We got it."];

pub fn back_up(n: u32) -> &'static str {
    BACK_UP[n as usize % BACK_UP.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every reason the world can refuse with has a name of its own: a
    /// new refusal that falls to the default fails here until it is
    /// given one.
    #[test]
    fn every_refusal_has_a_name() {
        let src = include_str!("lib.rs");
        let mut n = 0;
        for part in src.split("self.reject(o, \"").skip(1) {
            let why = &part[..part.find('"').unwrap()];
            assert!(known(why).is_some(), "the refusal {:?} has no chair: add it to chair::known", why);
            n += 1;
        }
        assert!(n > 40, "found only {} refusals: has reject() changed shape?", n);
        for why in ["fewer than three sides would remain", "one side would hold more than half the tanks"] {
            assert!(src.contains(why) && known(why).is_some(), "{}", why);
        }
    }

    /// The tone: no line blames anyone. A tag names a helpdesk joke; the
    /// lines of recovery say "we".
    #[test]
    fn nobody_is_blamed() {
        for c in [Chair::Pebkac, Chair::Picnic, Chair::SixtyCm, Chair::LooseNut, Chair::KeyboardChair, Chair::Id10t] {
            let s = format!("{} {}", c.label(), c.expansion()).to_lowercase();
            assert!(!s.contains("you"), "{}", s);
        }
        assert_eq!(tagged("water"), "PICNIC: water");
        assert_eq!(tagged("inside minimum range"), "60 CM: inside minimum range");
        assert_eq!(refusal("a reason never given"), Chair::Pebkac);
        assert!(BACK_UP.iter().all(|l| !l.starts_with("I ") && !l.to_lowercase().contains("you")));
        assert_ne!(back_up(0), back_up(1));
        let u = usage("treadcore", "unknown flag --fast");
        assert!(u.starts_with("treadcore: ID-10-T: unknown flag --fast\n") && u.contains("nothing is broken"), "{}", u);
    }
}
