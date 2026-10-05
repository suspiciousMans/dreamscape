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
    FusedDream,
    PackFoil,
    PackProphetic,
    PackFaded,
    LotteryPull,
    LotteryPity,
    StreakContinued,
    StreakRested,
    StreakReset,
    CleanStreak,
    MasteryLevelUp,
    Returning,
    RunMilestone,
}

#[cfg(test)]
pub const ALL_MOMENTS: [Moment; 25] = [
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
    Moment::PackFoil,
    Moment::PackProphetic,
    Moment::PackFaded,
    Moment::LotteryPull,
    Moment::LotteryPity,
    Moment::StreakContinued,
    Moment::StreakRested,
    Moment::StreakReset,
    Moment::CleanStreak,
    Moment::MasteryLevelUp,
    Moment::Returning,
    Moment::RunMilestone,
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
        Moment::PackFoil => &["Oh. That one catches the light.", "Shiny. Something wanted to be seen.", "That one's different. Hold it a moment.", "It glimmers. Dreams do that when they mean it."],
        Moment::PackProphetic => &["That one hasn't happened yet.", "Prophetic. I'd keep it close.", "It knows something. I don't know what.", "A dream from ahead of you. Careful with it."],
        Moment::PackFaded => &["Some dreams fade. These did. That's alright.", "Dust now. It was almost something.", "So close. The dust remembers, even if I can't.", "They let go gently. Nothing is wasted."],
        Moment::LotteryPull => &["You spent a little of yourself. Let's see.", "Dust becomes something. Maybe.", "Reach in. I'll look away.", "Every pull is a small wish."],
        Moment::LotteryPity => &["There. I was holding that one back for you.", "You kept reaching. It was always going to come.", "Patience. See? It was there.", "I wouldn't let you leave empty-handed."],
        Moment::StreakContinued => &["You came back. I noticed.", "Another night together. Good.", "Again. I like when you return.", "You're here. That's all I wanted."],
        Moment::StreakRested => &["You needed rest. I kept your place.", "Missing a night is allowed. It's still here.", "Nothing broke. I held it for you.", "Rest counts. I saved the thread."],
        Moment::StreakReset => &["It's alright. We begin again, gently.", "A new thread. No weight on it.", "You were away. You're back. That's the part that counts.", "Nothing's lost. Start from here."],
        Moment::CleanStreak => &["They didn't see you. Quiet work.", "Unseen again. You're learning the dark.", "Soft steps. Nothing noticed.", "The shadows are on your side lately."],
        Moment::MasteryLevelUp => &["That dream grew. It knows you better.", "Stronger now. It remembers using it.", "You and that one understand each other.", "It learned from you. Quietly."],
        Moment::Returning => &["You were gone a while. Welcome back.", "There you are. I kept the dark warm.", "Back again. I wondered.", "I hoped you'd return. Come in."],
        Moment::RunMilestone => &["Look how many times we've done this.", "Another landmark. I've been counting.", "You keep coming. Thank you.", "We've walked a long way down together."],
    }
}

#[rustfmt::skip]
fn weight_flavour(m: Moment, w: Weight) -> Option<&'static str> {
    Some(match (m, w) {
        (Moment::StreakReset, Weight::Burnout) => "You were tired. Of course you were. Come in.",
        (Moment::StreakReset, Weight::Pressure) => "No streak to keep up. Just you, here.",
        (Moment::StreakReset, Weight::Guilt) => "You didn't fail anyone by resting.",
        (Moment::StreakReset, Weight::Anxiety) => "Breathe. Nothing's counting against you.",
        (Moment::StreakReset, Weight::Grief) => "Some days are too heavy. You came back anyway.",
        (Moment::StreakReset, Weight::Loneliness) => "I was here the whole time. Still am.",
        (Moment::StreakReset, Weight::Insomnia) => "Sleep, then dream. There's no rush.",
        (Moment::StreakReset, Weight::Heartbreak) => "Take the time you need. I'll wait.",
        (Moment::StreakRested, Weight::Burnout) => "Good. Rest is the whole point.",
        (Moment::StreakRested, Weight::Pressure) => "Nobody's grading your nights. Rest.",
        (Moment::Returning, Weight::Loneliness) => "I'm glad it's you. I missed this.",
        (Moment::Returning, Weight::Grief) => "Come sit. We don't have to say anything.",
        (Moment::Returning, Weight::Insomnia) => "Finally, some company in the small hours.",
        (Moment::Returning, Weight::Heartbreak) => "Welcome back. Be gentle with yourself here.",
        (Moment::PackFaded, Weight::Grief) => "Some things fade. They still mattered.",
        (Moment::PackFaded, Weight::Guilt) => "Letting go isn't the same as failing.",
        (Moment::LotteryPity, Weight::Pressure) => "You didn't have to earn it. Take it.",
        _ => return None,
    })
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
        _ => weight_flavour(m, you.weight),
    };
    match flavour {
        Some(f) if seed % 3 == 0 => f.to_string(),
        _ => pool[(seed as usize) % pool.len()].to_string(),
    }
}

/// How well the eye knows you, by number of runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    Stranger,
    Acquainted,
    Familiar,
    Close,
}

pub const ACQUAINTED_RUNS: u32 = 3;
pub const FAMILIAR_RUNS: u32 = 10;
pub const CLOSE_RUNS: u32 = 25;

pub fn relationship(runs: u32) -> Stage {
    match runs {
        r if r >= CLOSE_RUNS => Stage::Close,
        r if r >= FAMILIAR_RUNS => Stage::Familiar,
        r if r >= ACQUAINTED_RUNS => Stage::Acquainted,
        _ => Stage::Stranger,
    }
}

#[rustfmt::skip]
fn stage_lines(m: Moment, stage: Stage) -> Option<&'static [&'static str]> {
    Some(match (m, stage) {
        (Moment::FirstDream, Stage::Stranger) => &["You're here. Good. I've been waiting.", "Deep breath. Let's find out who you are.", "Don't be afraid. I'm only watching."],
        (Moment::FirstDream, Stage::Acquainted) => &["Back again. I'm starting to know your step.", "You again. Good. Stay close.", "I remember you. A little."],
        (Moment::FirstDream, Stage::Familiar) => &["There you are. I knew your footsteps.", "Welcome back. I saved you a quiet corner.", "Let's go. I know how you like to start."],
        (Moment::FirstDream, Stage::Close) => &["Hello, you. I'd know you anywhere by now.", "Home again. I stopped pretending I don't wait.", "I know what you carry. Let me carry it with you."],
        (Moment::WakeDoor, Stage::Stranger) => &["The door's there. No rush.", "You can go back up. Or not yet.", "It leads up. It's safe."],
        (Moment::WakeDoor, Stage::Acquainted) => &["The door again. You know the way now.", "Up is that way. I'll be here.", "Leave when you like. You'll be back."],
        (Moment::WakeDoor, Stage::Familiar) => &["Going already? Alright. Come back soon.", "Rest well up there. I'll keep your place.", "You always look back at the door. I do too."],
        (Moment::WakeDoor, Stage::Close) => &["Go on. I'll miss you. I always do.", "Wake gently. I'm not far, ever.", "Whatever's up there, you've faced worse down here."],
        (Moment::Returning, Stage::Stranger) => &["You came back. I wasn't sure you would.", "Hello again. Don't mind the dark.", "Oh. You're here."],
        (Moment::Returning, Stage::Acquainted) => &["You're back. I kept your place.", "There you are. It's been a while.", "I wondered when you'd come."],
        (Moment::Returning, Stage::Familiar) => &["I missed this. Come in, sit down.", "You were gone. I felt it. Welcome back.", "It's quieter without you."],
        (Moment::Returning, Stage::Close) => &["Finally. I kept the light on.", "I've thought about our dreams while you were gone.", "Whatever kept you, you're here. That's enough."],
        _ => return None,
    })
}

/// Like `line`, but the eye's voice warms with `stage`. Moments without a
/// stage pool fall back to `line`.
pub fn line_for_stage(m: Moment, you: &Dreamer, seed: u64, stage: Stage) -> String {
    match stage_lines(m, stage) {
        Some(pool) => pool[(seed as usize) % pool.len()].to_string(),
        None => line(m, you, seed),
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
    fn lines_are_short_deterministic_and_plain() {
        for w in ALL_WEIGHTS {
            let mut you = dreamer(1, 0);
            you.weight = w;
            for m in ALL_MOMENTS {
                let mut seen = std::collections::HashSet::new();
                for seed in 0..60 {
                    let l = line(m, &you, seed);
                    assert_eq!(l, line(m, &you, seed));
                    assert!(l.chars().count() <= 70, "{m:?} {w:?}: {l}");
                    assert!(!l.contains('[') || l.contains("[WASD]"), "{l}");
                    seen.insert(l);
                }
                let want = if ALL_MOMENTS[13..].contains(&m) { 3 } else { 2 };
                assert!(seen.len() >= want, "{m:?} {w:?} only {}", seen.len());
            }
        }
    }

    #[test]
    fn new_moments_pool_has_at_least_four() {
        for m in &ALL_MOMENTS[13..] {
            assert!(lines(*m).len() >= 4, "{m:?}");
        }
        assert_eq!(ALL_MOMENTS.len(), 25);
    }

    #[test]
    fn relationship_thresholds_are_monotonic() {
        assert_eq!(relationship(0), Stage::Stranger);
        assert_eq!(relationship(ACQUAINTED_RUNS - 1), Stage::Stranger);
        assert_eq!(relationship(ACQUAINTED_RUNS), Stage::Acquainted);
        assert_eq!(relationship(FAMILIAR_RUNS), Stage::Familiar);
        assert_eq!(relationship(CLOSE_RUNS), Stage::Close);
        assert_eq!(relationship(u32::MAX), Stage::Close);
        let mut prev = Stage::Stranger;
        for r in 0..200 {
            let s = relationship(r);
            assert!(s >= prev);
            prev = s;
        }
    }

    #[test]
    fn stage_lines_are_complete_short_and_distinct() {
        let stages = [Stage::Stranger, Stage::Acquainted, Stage::Familiar, Stage::Close];
        let you = dreamer(1, 0);
        for m in [Moment::FirstDream, Moment::WakeDoor, Moment::Returning] {
            for st in stages {
                let pool = stage_lines(m, st).unwrap();
                assert!(pool.len() >= 3);
                for seed in 0..9 {
                    let l = line_for_stage(m, &you, seed, st);
                    assert_eq!(l, line_for_stage(m, &you, seed, st));
                    assert!(!l.is_empty() && l.chars().count() <= 70, "{l}");
                    assert!(!l.contains('['), "{l}");
                }
            }
            let a = stage_lines(m, Stage::Stranger).unwrap();
            let b = stage_lines(m, Stage::Close).unwrap();
            assert!(a.iter().all(|x| !b.contains(x)), "{m:?}");
            assert_ne!(
                line_for_stage(m, &you, 1, Stage::Stranger),
                line_for_stage(m, &you, 1, Stage::Close)
            );
        }
    }

    #[test]
    fn stage_falls_back_to_line_for_other_moments() {
        let you = dreamer(1, 0);
        assert_eq!(
            line_for_stage(Moment::PackFoil, &you, 2, Stage::Close),
            line(Moment::PackFoil, &you, 2)
        );
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
