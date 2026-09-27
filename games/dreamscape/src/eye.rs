//! The third eye: your subconscious. It guides you, and it's also where the
//! nightmares come from. It speaks in short lines above the HUD eye.

use crate::lore::{Dreamer, Weight};

/// Seconds a line stays up (after typing out).
pub const LINE_SECS: f32 = 4.5;
/// Letters per second.
const TYPE_RATE: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    FirstDream,
    ShardTaken,
    Lucid,
    EnemySeen,
    Caught,
    NightmareAhead,
    NightmareBeaten,
    NoteFound,
    DreamerComplete,
    SlotUnlocked,
    WakeDoor,
    WentDeeper,
    #[allow(dead_code)] // Part G: fused dreams
    FusedDream,
}

#[cfg(test)]
pub const ALL_MOMENTS: [Moment; 13] = [
    Moment::FirstDream,
    Moment::ShardTaken,
    Moment::Lucid,
    Moment::EnemySeen,
    Moment::Caught,
    Moment::NightmareAhead,
    Moment::NightmareBeaten,
    Moment::NoteFound,
    Moment::DreamerComplete,
    Moment::SlotUnlocked,
    Moment::WakeDoor,
    Moment::WentDeeper,
    Moment::FusedDream,
];

#[rustfmt::skip]
fn lines(m: Moment) -> &'static [&'static str] {
    match m {
        Moment::FirstDream => &["You're here. Good. I've been waiting.", "Deep breath. We've done this before.", "Let's find out who you are."],
        Moment::ShardTaken => &["There. Hold on to that.", "A little clearer now.", "That's yours. Remember it."],
        Moment::Lucid => &["I'm open. You can wake now, if you want to.", "You could leave. Or we could look deeper."],
        Moment::EnemySeen => &["Don't let it touch you.", "That one knows you're here.", "Go around. You don't have to fight everything."],
        Moment::Caught => &["It's alright. Again.", "That's not failing. That's learning the shape of it.", "Breathe. Get up."],
        Moment::NightmareAhead => &["I'm sorry. This one's mine.", "I made this. You'll have to walk through it.", "It's loud in here. Stay with me."],
        Moment::NightmareBeaten => &["You walked through it.", "Quieter now. Thank you.", "It'll come back smaller."],
        Moment::NoteFound => &["That's a piece of someone.", "Read it. Slowly.", "Someone wrote that on a bad day."],
        Moment::DreamerComplete => &["You know them now. They'll walk with you.", "Everyone's carrying something. Now you carry a bit of theirs."],
        Moment::SlotUnlocked => &["You remember more. You can bring more.", "Another memory to hold on to."],
        Moment::WakeDoor => &["The door's there. No rush.", "You can go back up. Or not yet."],
        Moment::WentDeeper => &["Deeper, then. Stay close.", "Brave. Or stubborn. Both?"],
        Moment::FusedDream => &["Two dreams at once. Careful.", "These shouldn't touch. They are anyway."],
    }
}

/// A line for `m`, sometimes coloured by what you carry.
pub fn line(m: Moment, you: &Dreamer, seed: u64) -> String {
    let pool = lines(m);
    let flavour = match (m, you.weight) {
        (Moment::Caught, Weight::Anxiety) => Some("Your heart's racing. It's allowed to. Get up."),
        (Moment::Caught, Weight::Burnout) => Some("Rest counts too. Then get up."),
        (Moment::Caught, Weight::Guilt) => Some("You don't have to punish yourself for that."),
        (Moment::NightmareBeaten, Weight::Grief) => {
            Some("It still hurts. You still walked through it.")
        }
        (Moment::NightmareBeaten, Weight::Loneliness) => {
            Some("You weren't alone in there. I was with you.")
        }
        (Moment::Lucid, Weight::Insomnia) => Some("You could wake. Or you could finally rest."),
        (Moment::Lucid, Weight::Pressure) => Some("You don't have to go deeper to prove anything."),
        (Moment::NightmareAhead, Weight::Heartbreak) => {
            Some("This one sounds like them. I'm sorry.")
        }
        _ => None,
    };
    match flavour {
        Some(f) if seed % 3 == 0 => f.to_string(),
        _ => pool[(seed as usize) % pool.len()].to_string(),
    }
}

/// Under a nightmare's title: what it's really about.
pub fn nightmare_subtitle(you: &Dreamer, tier: u32) -> String {
    let s: [&str; 3] = match you.weight {
        Weight::Grief => [
            "the empty chair",
            "every anniversary at once",
            "the call you missed",
        ],
        Weight::Burnout => [
            "the inbox that refills",
            "every deadline at once",
            "the shift that never ends",
        ],
        Weight::Anxiety => ["what if", "everyone looking", "the thing you forgot"],
        Weight::Loneliness => [
            "the unread messages",
            "the crowded room",
            "the table for one",
        ],
        Weight::Guilt => [
            "the thing you said",
            "the look on their face",
            "the apology you owe",
        ],
        Weight::Insomnia => ["3:12am", "the ticking", "the ceiling"],
        Weight::Heartbreak => [
            "the song in the shop",
            "the jacket on the hook",
            "the last conversation",
        ],
        Weight::Pressure => ["the list", "second place", "everyone counting on you"],
    };
    s[(tier as usize).min(2)].to_string()
}

/// How much of `text` is typed out `age` seconds in.
pub fn shown(text: &str, age: f32) -> String {
    text.chars()
        .take((age * TYPE_RATE).max(0.0) as usize)
        .collect()
}

/// Opacity `age` seconds after the line started.
pub fn alpha(age: f32) -> f32 {
    ((LINE_SECS + 1.0 - age) / 1.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::{dreamer, Weight, ALL_WEIGHTS};

    #[test]
    fn every_moment_has_a_line_for_every_weight() {
        for w in ALL_WEIGHTS {
            let mut you = dreamer(1, 0);
            you.weight = w;
            for m in ALL_MOMENTS {
                for seed in 0..5 {
                    let l = line(m, &you, seed);
                    assert!(!l.is_empty() && l.len() <= 90, "{m:?} {w:?}: {l:?}");
                    assert!(!l.contains('{'), "{l}");
                }
            }
        }
    }

    #[test]
    fn nightmares_are_named_after_what_you_carry() {
        let mut you = dreamer(1, 0);
        you.weight = Weight::Burnout;
        let a = nightmare_subtitle(&you, 2);
        you.weight = Weight::Grief;
        assert_ne!(a, nightmare_subtitle(&you, 2));
    }

    #[test]
    fn lines_type_out_then_fade() {
        assert_eq!(shown("hello", 0.0), "");
        assert_eq!(shown("hello", 10.0), "hello");
        assert!(alpha(0.1) > 0.9 && alpha(LINE_SECS + 1.0) == 0.0);
    }
}
