//! Between-dream events (roadmap 3.5): now and then the dream offers a small
//! choice in place of the upgrade pick.
//!
//! This file is pure: it knows the events, when one is due, and what a choice
//! does (as a list of [`Applied`] changes). It never touches the game. The
//! game turns an event into choice-screen cards and applies the result.
//!
//! The text is a first draft (`plans/2026-10-21-event-drafts.md`) and may be
//! rewritten; the mechanics don't depend on the wording.

use crate::dream::{DreamTheme, Variant};
use crate::upgrades::{Roll, RunUpgrades, Tier, Upgrade, ALL_CURSES, PASSIVES};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// Chance a dream is followed by an event, when one is allowed.
pub const ODDS: f64 = 1.0 / 6.0;
/// No events before this depth.
pub const FIRST_DEPTH: u32 = 2;
/// Dust lost to the VOICE when you pay it in dust.
pub const VOICE_DUST: u32 = 15;
/// Dust it costs to knock before opening the door.
pub const KNOCK_DUST: u32 = 8;
/// Dust for taking the chair.
pub const CHAIR_DUST: u32 = 10;
/// Dust when an event has nothing else to give.
const CONSOLATION: i32 = 5;
// === events2 ===
/// Dust a coin costs at the well, and what a lucky toss returns.
pub const WELL_COST: u32 = 10;
pub const WELL_WIN: i32 = 30;
/// Dust for a handout (the bakery, the sleeping giant's blanket).
pub const HANDOUT_DUST: i32 = 8;
/// Dust the stairs and a lost chord give.
pub const STAIRS_DUST: i32 = 10;
pub const CHORD_CONSOLATION: i32 = 4;
/// Prices at the vendor, tailor, bakery, lighthouse, library, piano, crows.
pub const CUPFUL_DUST: u32 = 6;
pub const LANTERN_DUST: u32 = 18;
pub const SEAM_DUST: u32 = 12;
pub const ROLL_DUST: u32 = 4;
pub const BEACON_DUST: u32 = 14;
pub const PAGE_DUST: u32 = 8;
pub const TUNING_DUST: u32 = 10;
pub const CROW_DUST: u32 = 20;
// === end events2 ===

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Id {
    HummingDoor,
    Voice,
    EmptyChair,
    LostAndFound,
    SecondMoon,
    // === events2 ===
    LanternVendor,
    WishingWell,
    PatientTailor,
    DryLighthouse,
    NightBakery,
    MirrorPond,
    UnfinishedLibrary,
    DrownedPiano,
    DownStaircase,
    TellingCrows,
    SleepingGiant,
    ForgottenBirthday,
    // === end events2 ===
}

pub const ALL: [Id; 17] = [
    Id::HummingDoor,
    Id::Voice,
    Id::EmptyChair,
    Id::LostAndFound,
    Id::SecondMoon,
    Id::LanternVendor,
    Id::WishingWell,
    Id::PatientTailor,
    Id::DryLighthouse,
    Id::NightBakery,
    Id::MirrorPond,
    Id::UnfinishedLibrary,
    Id::DrownedPiano,
    Id::DownStaircase,
    Id::TellingCrows,
    Id::SleepingGiant,
    Id::ForgottenBirthday,
];

/// What a choice does. Random parts are settled in [`resolve`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Plan {
    Nothing,
    /// 50/50: an upgrade, or a curse.
    DoorGamble,
    /// A random upgrade, no curse.
    Upgrade,
    /// A random common upgrade.
    CommonUpgrade,
    /// Dust now, paid for with a thinner next pick.
    ChairDust,
    /// Lose a held upgrade; bank a shard.
    VoiceCard,
    /// Lose dust; the next dream is gilded.
    VoiceDust,
    /// One more free reroll.
    FreeReroll,
    /// A curse, then a pick of the usual three.
    DigThrough,
    /// The next dream is gilded and gets a random extra twist.
    MoonBlink,
    /// WIDE EYES.
    MoonSteady,
    // === events2 ===
    /// 50/50: a good deal of dust, or nothing (the coin is gone).
    DustGamble,
    /// Lose a held upgrade, take a random one.
    SwapUpgrade,
    /// A pick now; the next pick is thinner.
    BorrowedPick,
    /// Two free rerolls.
    TwoRerolls,
    /// A curse, and a shard.
    CurseForShard,
    /// Let go of a curse you hold.
    ShedCurse,
    /// A shard, banked.
    ShardOnly,
    /// A shard, and a thinner next pick.
    ShardThin,
    /// The next dream gets a random extra twist.
    TwistOnly,
    /// The next dream is gilded.
    GildOnly,
    /// The next dream is twisted; a little dust.
    TwistDust,
    /// The next dream is gilded; the next pick is thinner.
    GildThin,
    /// A little dust.
    Handout,
    /// 50/50: a random upgrade, or a little dust.
    ChordGamble,
    // === end events2 ===
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Choice {
    pub text: &'static str,
    /// One line on what it does, shown under the name.
    pub detail: &'static str,
    /// Dust it costs up front; the choice is unavailable without it.
    pub cost: u32,
    pub plan: Plan,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: Id,
    pub title: &'static str,
    pub prompt: &'static str,
    /// Always three; the last one costs nothing and does nothing.
    pub choices: [Choice; 3],
}

const WALK_AWAY: fn(&'static str) -> Choice = |text| Choice {
    text,
    detail: "nothing happens",
    cost: 0,
    plan: Plan::Nothing,
};

pub fn event(id: Id) -> Event {
    match id {
        Id::HummingDoor => Event {
            id,
            title: "THE HUMMING DOOR",
            prompt: "A door stands in the fog, humming one note. It isn't locked.",
            choices: [
                Choice {
                    text: "Open it",
                    detail: "an upgrade, or a curse. even odds",
                    cost: 0,
                    plan: Plan::DoorGamble,
                },
                Choice {
                    text: "Knock first",
                    detail: "a safe upgrade, for 8 dust",
                    cost: KNOCK_DUST,
                    plan: Plan::Upgrade,
                },
                WALK_AWAY("Walk past"),
            ],
        },
        Id::Voice => Event {
            id,
            title: "THE VOICE THAT CARRIES",
            prompt: "A voice offers to carry something for you.",
            choices: [
                Choice {
                    text: "Give it a card",
                    detail: "lose an upgrade. bank a shard",
                    cost: 0,
                    plan: Plan::VoiceCard,
                },
                Choice {
                    text: "Give it dust",
                    detail: "lose 15 dust. next dream gilded",
                    cost: VOICE_DUST,
                    plan: Plan::VoiceDust,
                },
                WALK_AWAY("Say you're fine"),
            ],
        },
        Id::EmptyChair => Event {
            id,
            title: "THE EMPTY CHAIR",
            prompt: "Someone left a chair here. It's still warm.",
            choices: [
                Choice {
                    text: "Sit",
                    detail: "one more free reroll",
                    cost: 0,
                    plan: Plan::FreeReroll,
                },
                Choice {
                    text: "Take the chair",
                    detail: "+10 dust. next pick: one fewer card",
                    cost: 0,
                    plan: Plan::ChairDust,
                },
                WALK_AWAY("Leave it"),
            ],
        },
        Id::LostAndFound => Event {
            id,
            title: "THE LOST AND FOUND",
            prompt: "A box of things people forgot they dreamed.",
            choices: [
                Choice {
                    text: "Dig through it",
                    detail: "take a curse, then pick from the usual cards",
                    cost: 0,
                    plan: Plan::DigThrough,
                },
                Choice {
                    text: "Take the top item",
                    detail: "a random common upgrade",
                    cost: 0,
                    plan: Plan::CommonUpgrade,
                },
                WALK_AWAY("Close the lid"),
            ],
        },
        Id::SecondMoon => Event {
            id,
            title: "THE SECOND MOON",
            prompt: "There are two moons tonight. One of them blinks.",
            choices: [
                Choice {
                    text: "Wave at the blinking one",
                    detail: "next dream: gilded, and a random extra twist",
                    cost: 0,
                    plan: Plan::MoonBlink,
                },
                Choice {
                    text: "Wave at the steady one",
                    detail: "wide eyes: you see further for the rest of the run",
                    cost: 0,
                    plan: Plan::MoonSteady,
                },
                WALK_AWAY("Don't look up"),
            ],
        },
        // === events2 ===
        Id::LanternVendor => Event {
            id,
            title: "THE LANTERN VENDOR",
            prompt: "A vendor sells light by the cupful.",
            choices: [
                Choice { text: "Buy a cupful", detail: "a random common upgrade, for 6 dust", cost: CUPFUL_DUST, plan: Plan::CommonUpgrade },
                Choice { text: "Buy the lantern", detail: "a safe upgrade, for 18 dust", cost: LANTERN_DUST, plan: Plan::Upgrade },
                WALK_AWAY("Walk on"),
            ],
        },
        Id::WishingWell => Event {
            id,
            title: "THE WISHING WELL",
            prompt: "The well is dry, but it still listens.",
            choices: [
                Choice { text: "Toss a coin", detail: "10 dust. even odds of 30 back", cost: WELL_COST, plan: Plan::DustGamble },
                Choice { text: "Toss something heavy", detail: "lose an upgrade, take a random one", cost: 0, plan: Plan::SwapUpgrade },
                WALK_AWAY("Let it be"),
            ],
        },
        Id::PatientTailor => Event {
            id,
            title: "THE PATIENT TAILOR",
            prompt: "A tailor mends a coat made of fog. Pins in her mouth.",
            choices: [
                Choice { text: "Be measured", detail: "an extra pick, one card fewer, for 8 dust", cost: PAGE_DUST, plan: Plan::BorrowedPick },
                Choice { text: "Let out a seam", detail: "two free rerolls, for 12 dust", cost: SEAM_DUST, plan: Plan::TwoRerolls },
                WALK_AWAY("Say you're late"),
            ],
        },
        Id::DryLighthouse => Event {
            id,
            title: "THE DRY LIGHTHOUSE",
            prompt: "A lighthouse stands far from any water. Its lamp is out.",
            choices: [
                Choice { text: "Climb it", detail: "next dream: a random extra twist", cost: 0, plan: Plan::TwistOnly },
                Choice { text: "Light the lamp", detail: "next dream: gilded, for 14 dust", cost: BEACON_DUST, plan: Plan::GildOnly },
                WALK_AWAY("Watch from the shore"),
            ],
        },
        Id::NightBakery => Event {
            id,
            title: "THE NIGHT BAKERY",
            prompt: "Warm bread, and no baker. A bell on the counter.",
            choices: [
                Choice { text: "Buy a roll", detail: "one free reroll, for 4 dust", cost: ROLL_DUST, plan: Plan::FreeReroll },
                Choice { text: "Take yesterday's bread", detail: "+8 dust", cost: 0, plan: Plan::Handout },
                WALK_AWAY("Don't ring the bell"),
            ],
        },
        Id::MirrorPond => Event {
            id,
            title: "THE MIRROR POND",
            prompt: "The pond shows you a little ahead of yourself.",
            choices: [
                Choice { text: "Reach through", detail: "take a curse. bank a shard", cost: 0, plan: Plan::CurseForShard },
                Choice { text: "Drink from it", detail: "let go of a curse you hold", cost: 0, plan: Plan::ShedCurse },
                WALK_AWAY("Look away"),
            ],
        },
        Id::UnfinishedLibrary => Event {
            id,
            title: "THE UNFINISHED LIBRARY",
            prompt: "Every book stops mid-sentence. A few pages are loose.",
            choices: [
                Choice { text: "Read a page", detail: "a random common upgrade, for 8 dust", cost: PAGE_DUST, plan: Plan::CommonUpgrade },
                Choice { text: "Tear one out", detail: "bank a shard. next pick: one fewer card", cost: 0, plan: Plan::ShardThin },
                WALK_AWAY("Shush the room"),
            ],
        },
        Id::DrownedPiano => Event {
            id,
            title: "THE DROWNED PIANO",
            prompt: "A piano sits knee-deep in a puddle. One key still rings.",
            choices: [
                Choice { text: "Play a chord", detail: "an upgrade, or a little dust", cost: 0, plan: Plan::ChordGamble },
                Choice { text: "Tune it", detail: "next dream: gilded, for 10 dust", cost: TUNING_DUST, plan: Plan::GildOnly },
                WALK_AWAY("Close the lid"),
            ],
        },
        Id::DownStaircase => Event {
            id,
            title: "THE DOWN STAIRCASE",
            prompt: "Stairs go down into the next dream. The rail is smooth.",
            choices: [
                Choice { text: "Take the stairs", detail: "next dream: a random twist. +10 dust", cost: 0, plan: Plan::TwistDust },
                Choice { text: "Slide the banister", detail: "next dream gilded. next pick: one fewer", cost: 0, plan: Plan::GildThin },
                WALK_AWAY("Turn back"),
            ],
        },
        Id::TellingCrows => Event {
            id,
            title: "THE TELLING CROWS",
            prompt: "Crows on a wire, all facing the way you came.",
            choices: [
                Choice { text: "Listen", detail: "next dream: a random extra twist", cost: 0, plan: Plan::TwistOnly },
                Choice { text: "Pay them", detail: "bank a shard, for 20 dust", cost: CROW_DUST, plan: Plan::ShardOnly },
                WALK_AWAY("Shoo them"),
            ],
        },
        Id::SleepingGiant => Event {
            id,
            title: "THE SLEEPING GIANT",
            prompt: "A giant sleeps on its side. Its coat pocket is open.",
            choices: [
                Choice { text: "Pick its pocket", detail: "an upgrade, or a curse. even odds", cost: 0, plan: Plan::DoorGamble },
                Choice { text: "Tuck in its blanket", detail: "+8 dust", cost: 0, plan: Plan::Handout },
                WALK_AWAY("Tiptoe past"),
            ],
        },
        Id::ForgottenBirthday => Event {
            id,
            title: "THE FORGOTTEN BIRTHDAY",
            prompt: "A cake, lit. No guests. The candles keep burning.",
            choices: [
                Choice { text: "Blow out the candles", detail: "an upgrade, or a little dust", cost: 0, plan: Plan::ChordGamble },
                Choice { text: "Cut the cake", detail: "+8 dust", cost: 0, plan: Plan::Handout },
                WALK_AWAY("Sing, quietly"),
            ],
        },
        // === end events2 ===
    }
}

/// Is an event due after this dream? Never in the first dreams, never twice
/// in a row, never when a nightmare's reward is waiting. Its own seeded
/// stream, so existing seeds replay unchanged.
pub fn due(seed: u64, depth: u32, last_dream_had_event: bool, boss_reward: bool) -> bool {
    if depth < FIRST_DEPTH || last_dream_had_event || boss_reward {
        return false;
    }
    StdRng::seed_from_u64(seed ^ 0xE7E4_7500).gen_bool(ODDS)
}

/// Which event, never the same one twice running.
pub fn pick(seed: u64, last: Option<Id>) -> Id {
    let mut rng = StdRng::seed_from_u64(seed ^ 0xE7E4_7501);
    let pool: Vec<Id> = ALL.iter().copied().filter(|&i| Some(i) != last).collect();
    pool[rng.gen_range(0..pool.len())]
}

/// One concrete change for the game to make.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    Dust(i32),
    Take(Upgrade),
    Lose(Upgrade),
    BankShard,
    FreeReroll,
    /// The next upgrade pick offers one fewer card.
    ThinNextPick,
    /// The next dream is gilded.
    GildNext,
    /// The next dream also gets this twist.
    TwistNext(Variant),
    /// Open the usual upgrade pick.
    OpenPick,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub applied: Vec<Applied>,
    /// What happened, one short line.
    pub line: String,
}

/// Can this choice be made right now? (It may need dust or something to lose.)
pub fn available(c: &Choice, run: &RunUpgrades, dust: u32) -> bool {
    if dust < c.cost {
        return false;
    }
    match c.plan {
        Plan::VoiceCard | Plan::SwapUpgrade => held_passive(run).is_some(),
        // === events2 ===
        Plan::ShedCurse => held_curse(run).is_some(),
        // === end events2 ===
        _ => true,
    }
}

fn held_passive(run: &RunUpgrades) -> Option<Upgrade> {
    run.taken
        .iter()
        .copied()
        .find(|u| !u.is_ability() && !matches!(u, Upgrade::Cursed(_)))
}

// === events2 ===
fn held_curse(run: &RunUpgrades) -> Option<Upgrade> {
    run.taken.iter().copied().find(|u| matches!(u, Upgrade::Cursed(_)))
}
// === end events2 ===

fn rng_for(seed: u64, choice: usize) -> StdRng {
    StdRng::seed_from_u64(seed ^ 0xE7E4_7502 ^ (choice as u64).wrapping_mul(0x9E37_79B9))
}

fn random_upgrade(rng: &mut StdRng, run: &RunUpgrades, depth: u32, common_only: bool) -> Option<Upgrade> {
    let pool: Vec<Upgrade> = if common_only {
        PASSIVES
            .iter()
            .copied()
            .filter(|&u| u.tier() == Tier::Common && run.available(u))
            .collect()
    } else {
        crate::upgrades::roll_choices(rng.gen(), run, Roll { depth, boss: false })
            .into_iter()
            .filter(|u| !matches!(u, Upgrade::Cursed(_)) && !u.is_ability())
            .collect()
    };
    (!pool.is_empty()).then(|| pool[rng.gen_range(0..pool.len())])
}

fn random_curse(rng: &mut StdRng, run: &RunUpgrades) -> Option<Upgrade> {
    let pool: Vec<Upgrade> = ALL_CURSES
        .iter()
        .map(|&c| Upgrade::Cursed(c))
        .filter(|&u| run.available(u))
        .collect();
    (!pool.is_empty()).then(|| pool[rng.gen_range(0..pool.len())])
}

fn gift(u: Option<Upgrade>) -> Vec<Applied> {
    match u {
        Some(u) => vec![Applied::Take(u)],
        None => vec![Applied::Dust(CONSOLATION)],
    }
}

/// What `choice` of `event` does. `None` if it can't be made. Deterministic
/// per `seed`; every change it asks for is valid for `run` as it stands.
pub fn resolve(
    event: &Event,
    choice: usize,
    seed: u64,
    run: &RunUpgrades,
    depth: u32,
    next_theme: DreamTheme,
    dust: u32,
) -> Option<Resolved> {
    let c = event.choices.get(choice)?;
    if !available(c, run, dust) {
        return None;
    }
    let mut rng = rng_for(seed, choice);
    let mut applied = Vec::new();
    if c.cost > 0 {
        applied.push(Applied::Dust(-(c.cost as i32)));
    }
    let line = match c.plan {
        Plan::Nothing => "you move on".to_string(),
        Plan::DoorGamble => {
            if rng.gen_bool(0.5) {
                applied.extend(gift(random_upgrade(&mut rng, run, depth, false)));
                "the door opens onto something kind".into()
            } else if let Some(curse) = random_curse(&mut rng, run) {
                applied.push(Applied::Take(curse));
                "the door opens onto something hungry".into()
            } else {
                applied.extend(gift(random_upgrade(&mut rng, run, depth, false)));
                "the door opens onto something kind".into()
            }
        }
        Plan::Upgrade => {
            applied.extend(gift(random_upgrade(&mut rng, run, depth, false)));
            "the door lets you in".into()
        }
        Plan::CommonUpgrade => {
            applied.extend(gift(random_upgrade(&mut rng, run, depth, true)));
            "it was yours all along".into()
        }
        Plan::VoiceCard => {
            let lost = held_passive(run)?;
            applied.push(Applied::Lose(lost));
            applied.push(Applied::BankShard);
            "the voice takes it, and keeps it safe".into()
        }
        Plan::VoiceDust => {
            // The 15 dust is the choice's up-front cost, already charged above.
            applied.push(Applied::GildNext);
            "the voice takes it, and the next dream shines".into()
        }
        Plan::ChairDust => {
            applied.push(Applied::Dust(CHAIR_DUST as i32));
            applied.push(Applied::ThinNextPick);
            "you carry it out. it's heavier than it looks".into()
        }
        Plan::FreeReroll => {
            applied.push(Applied::FreeReroll);
            "the chair remembers you".into()
        }
        Plan::DigThrough => {
            if let Some(curse) = random_curse(&mut rng, run) {
                applied.push(Applied::Take(curse));
            }
            applied.push(Applied::OpenPick);
            "something in the box bites back".into()
        }
        Plan::MoonBlink => {
            applied.push(Applied::GildNext);
            if let Some(v) = random_twist(&mut rng, next_theme) {
                applied.push(Applied::TwistNext(v));
            }
            "the blinking moon blinks back".into()
        }
        Plan::MoonSteady => {
            applied.extend(gift(
                run.available(Upgrade::WideEyes).then_some(Upgrade::WideEyes),
            ));
            "the steady moon holds still for you".into()
        }
        // === events2 ===
        Plan::DustGamble => {
            if rng.gen_bool(0.5) {
                applied.push(Applied::Dust(WELL_WIN));
                "far below, something drops a coin back".into()
            } else {
                "the coin falls, and falls".into()
            }
        }
        Plan::SwapUpgrade => {
            let lost = held_passive(run)?;
            applied.push(Applied::Lose(lost));
            applied.extend(gift(random_upgrade(&mut rng, run, depth, false)));
            "the well swallows it, and hands back another".into()
        }
        Plan::BorrowedPick => {
            // The thin flag is spent by the pick opened right here.
            applied.push(Applied::ThinNextPick);
            applied.push(Applied::OpenPick);
            "she pins up the hem, and lets you choose".into()
        }
        Plan::TwoRerolls => {
            applied.push(Applied::FreeReroll);
            applied.push(Applied::FreeReroll);
            "there was more room in it than you thought".into()
        }
        Plan::CurseForShard => {
            if let Some(curse) = random_curse(&mut rng, run) {
                applied.push(Applied::Take(curse));
            }
            applied.push(Applied::BankShard);
            "your hand comes back holding something".into()
        }
        Plan::ShedCurse => {
            applied.push(Applied::Lose(held_curse(run)?));
            "it tastes like nothing, and you feel lighter".into()
        }
        Plan::ShardOnly => {
            applied.push(Applied::BankShard);
            "they drop it at your feet".into()
        }
        Plan::ShardThin => {
            applied.push(Applied::BankShard);
            applied.push(Applied::ThinNextPick);
            "the page was a shard all along".into()
        }
        Plan::TwistOnly => {
            if let Some(v) = random_twist(&mut rng, next_theme) {
                applied.push(Applied::TwistNext(v));
                "you see what is coming, a little".into()
            } else {
                applied.push(Applied::Dust(CONSOLATION));
                "nothing is coming, and that is a comfort".into()
            }
        }
        Plan::GildOnly => {
            applied.push(Applied::GildNext);
            "the next dream will shine".into()
        }
        Plan::TwistDust => {
            if let Some(v) = random_twist(&mut rng, next_theme) {
                applied.push(Applied::TwistNext(v));
            }
            applied.push(Applied::Dust(STAIRS_DUST));
            "you find dust on the landing".into()
        }
        Plan::GildThin => {
            applied.push(Applied::GildNext);
            applied.push(Applied::ThinNextPick);
            "you land too well, and something is left behind".into()
        }
        Plan::Handout => {
            applied.push(Applied::Dust(HANDOUT_DUST));
            "nobody minds".into()
        }
        Plan::ChordGamble => {
            if rng.gen_bool(0.5) {
                applied.extend(gift(random_upgrade(&mut rng, run, depth, false)));
                "the note hangs, and something answers".into()
            } else {
                applied.push(Applied::Dust(CHORD_CONSOLATION));
                "the note drifts off by itself".into()
            }
        }
        // === end events2 ===
    };
    Some(Resolved { applied, line })
}

/// A random twist that suits `theme`, other than the gilded one the moon
/// already adds. Calm dreams never twist.
fn random_twist(rng: &mut StdRng, theme: DreamTheme) -> Option<Variant> {
    if matches!(theme, DreamTheme::Lobby | DreamTheme::Awakening) {
        return None;
    }
    let pool: Vec<Variant> = crate::dream::ALL_VARIANTS
        .iter()
        .copied()
        .filter(|&v| v != Variant::Gilded && v.fits(theme))
        .collect();
    (!pool.is_empty()).then(|| pool[rng.gen_range(0..pool.len())])
}

/// Applies the parts of `a` that belong to the run's upgrades. The rest
/// (dust, shards, the next dream, the pick) is the game's.
pub fn apply_to_run(run: &mut RunUpgrades, a: &Applied) {
    match *a {
        Applied::Take(u) => run.take(u),
        Applied::Lose(u) => {
            if let Some(i) = run.taken.iter().position(|&t| t == u) {
                run.taken.remove(i);
            }
        }
        Applied::FreeReroll => run.bonus_rerolls += 1,
        Applied::ThinNextPick => run.thin_next_pick = true,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_THEMES;
    use crate::upgrades::Ability;
    use std::collections::HashSet;

    fn loaded_run() -> RunUpgrades {
        let mut run = RunUpgrades::default();
        run.take(Upgrade::Learn(Ability::Dash));
        for u in [Upgrade::SwiftFeet, Upgrade::Spring, Upgrade::HeavyAir] {
            run.take(u);
        }
        run
    }

    fn runs() -> Vec<RunUpgrades> {
        let mut full = loaded_run();
        for c in ALL_CURSES {
            full.take(Upgrade::Cursed(c));
        }
        vec![RunUpgrades::default(), loaded_run(), full]
    }

    #[test]
    fn every_event_has_three_choices_ending_in_a_free_do_nothing() {
        for id in ALL {
            let e = event(id);
            assert_eq!(e.id, id);
            let last = e.choices[2];
            assert_eq!((last.plan, last.cost), (Plan::Nothing, 0), "{id:?}");
            let names: HashSet<_> = e.choices.iter().map(|c| c.text).collect();
            assert_eq!(names.len(), 3, "{id:?} repeats a choice");
        }
    }

    #[test]
    fn the_text_fits_the_choice_screen() {
        for id in ALL {
            let e = event(id);
            assert!(e.title.len() <= 28, "{}", e.title);
            assert!(e.prompt.len() <= 64, "{}", e.prompt);
            for c in e.choices {
                assert!(c.text.len() <= 30, "{}", c.text);
                assert!(c.detail.len() <= 60, "{}", c.detail);
            }
        }
    }

    #[test]
    fn events_are_rare_late_and_never_back_to_back() {
        for seed in 0..500 {
            assert!(!due(seed, 0, false, false));
            assert!(!due(seed, FIRST_DEPTH - 1, false, false));
            assert!(!due(seed, 6, true, false));
            assert!(!due(seed, 6, false, true));
        }
        let n = (0..6000).filter(|&s| due(s, 6, false, false)).count() as f64 / 6000.0;
        assert!((ODDS - 0.03..ODDS + 0.03).contains(&n), "rate {n}");
    }

    #[test]
    fn every_event_turns_up_and_never_twice_running() {
        let seen: HashSet<Id> = (0..400).map(|s| pick(s, None)).collect();
        assert_eq!(seen.len(), ALL.len());
        for id in ALL {
            for s in 0..200 {
                assert_ne!(pick(s, Some(id)), id);
            }
        }
    }

    #[test]
    fn a_choice_without_its_price_or_its_card_cannot_be_made() {
        let run = RunUpgrades::default();
        let door = event(Id::HummingDoor);
        assert!(resolve(&door, 1, 1, &run, 5, DreamTheme::Garden, KNOCK_DUST - 1).is_none());
        assert!(resolve(&door, 1, 1, &run, 5, DreamTheme::Garden, KNOCK_DUST).is_some());
        let voice = event(Id::Voice);
        // Nothing but an ability held: nothing to give up.
        let mut only_ability = RunUpgrades::default();
        only_ability.take(Upgrade::Learn(Ability::Dash));
        assert!(resolve(&voice, 0, 1, &only_ability, 5, DreamTheme::Garden, 99).is_none());
        assert!(resolve(&voice, 0, 1, &loaded_run(), 5, DreamTheme::Garden, 0).is_some());
        assert!(resolve(&voice, 1, 1, &loaded_run(), 5, DreamTheme::Garden, VOICE_DUST - 1).is_none());
        assert!(resolve(&door, 7, 1, &run, 5, DreamTheme::Garden, 99).is_none());
    }

    #[test]
    fn walking_away_does_nothing() {
        for id in ALL {
            let r = resolve(&event(id), 2, 3, &loaded_run(), 5, DreamTheme::Garden, 0).unwrap();
            assert!(r.applied.is_empty(), "{id:?}");
        }
    }

    #[test]
    fn resolving_is_deterministic() {
        let run = loaded_run();
        for id in ALL {
            for choice in 0..3 {
                for seed in 0..50 {
                    let a = resolve(&event(id), choice, seed, &run, 6, DreamTheme::Garden, 50);
                    let b = resolve(&event(id), choice, seed, &run, 6, DreamTheme::Garden, 50);
                    assert_eq!(a, b);
                }
            }
        }
    }

    #[test]
    fn nothing_an_event_asks_for_is_invalid() {
        for run in runs() {
            for id in ALL {
                for choice in 0..3 {
                    for seed in 0..120 {
                        let dust = (seed % 40) as u32;
                        let Some(r) = resolve(&event(id), choice, seed, &run, 7, DreamTheme::Garden, dust)
                        else {
                            continue;
                        };
                        let mut after = run.clone();
                        let mut dust_delta = 0i64;
                        for a in &r.applied {
                            match *a {
                                Applied::Take(u) => assert!(
                                    after.available(u),
                                    "{id:?}/{choice}: {u:?} not takeable"
                                ),
                                Applied::Lose(u) => assert!(
                                    after.count(u) > 0 && !u.is_ability(),
                                    "{id:?}/{choice}: lost {u:?} it doesn't hold"
                                ),
                                Applied::Dust(n) => dust_delta += n as i64,
                                _ => {}
                            }
                            apply_to_run(&mut after, a);
                        }
                        assert!(
                            dust as i64 + dust_delta >= 0,
                            "{id:?}/{choice}: dust {dust} {dust_delta:+}"
                        );
                        assert!(!r.line.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    fn the_door_gives_both_kinds_of_thing() {
        let run = loaded_run();
        let door = event(Id::HummingDoor);
        let (mut gifts, mut curses) = (0, 0);
        for seed in 0..200 {
            let r = resolve(&door, 0, seed, &run, 6, DreamTheme::Garden, 0).unwrap();
            match r.applied[..] {
                [Applied::Take(Upgrade::Cursed(_))] => curses += 1,
                [Applied::Take(_)] => gifts += 1,
                ref other => panic!("{other:?}"),
            }
        }
        assert!(gifts > 50 && curses > 50, "{gifts} gifts, {curses} curses");
    }

    #[test]
    fn knocking_never_hands_over_a_curse() {
        let run = loaded_run();
        let door = event(Id::HummingDoor);
        for seed in 0..300 {
            let r = resolve(&door, 1, seed, &run, 6, DreamTheme::Garden, 99).unwrap();
            assert_eq!(r.applied[0], Applied::Dust(-(KNOCK_DUST as i32)));
            assert!(r
                .applied
                .iter()
                .all(|a| !matches!(a, Applied::Take(Upgrade::Cursed(_)))));
        }
    }

    #[test]
    fn paying_the_voice_in_dust_charges_once() {
        let r = resolve(&event(Id::Voice), 1, 2, &loaded_run(), 5, DreamTheme::Garden, 40).unwrap();
        let spent: i32 = r
            .applied
            .iter()
            .filter_map(|a| match a {
                Applied::Dust(n) => Some(*n),
                _ => None,
            })
            .sum();
        assert_eq!(spent, -(VOICE_DUST as i32));
    }

    #[test]
    fn the_voice_trades_a_card_for_a_shard_and_dust_for_gold() {
        let r = resolve(&event(Id::Voice), 0, 4, &loaded_run(), 5, DreamTheme::Garden, 0).unwrap();
        assert!(matches!(r.applied[0], Applied::Lose(_)));
        assert!(r.applied.contains(&Applied::BankShard));
        assert!(!r.applied.contains(&Applied::GildNext), "gilding is the dust price");
        let r = resolve(&event(Id::Voice), 1, 4, &loaded_run(), 5, DreamTheme::Garden, VOICE_DUST).unwrap();
        assert!(r.applied.contains(&Applied::GildNext));
    }

    #[test]
    fn digging_through_the_box_costs_a_curse_then_opens_a_pick() {
        let r = resolve(&event(Id::LostAndFound), 0, 9, &loaded_run(), 5, DreamTheme::Garden, 0).unwrap();
        assert!(matches!(r.applied[0], Applied::Take(Upgrade::Cursed(_))));
        assert_eq!(r.applied.last(), Some(&Applied::OpenPick));
        // With every curse already held there is none left to take, but the
        // pick is still offered.
        let full = runs().pop().unwrap();
        let r = resolve(&event(Id::LostAndFound), 0, 9, &full, 5, DreamTheme::Garden, 0).unwrap();
        assert_eq!(r.applied, vec![Applied::OpenPick]);
    }

    #[test]
    fn the_blinking_moon_adds_a_twist_that_suits_the_next_dream() {
        let run = loaded_run();
        let moon = event(Id::SecondMoon);
        for &theme in ALL_THEMES.iter() {
            for seed in 0..40 {
                let r = resolve(&moon, 0, seed, &run, 6, theme, 0).unwrap();
                assert!(r.applied.contains(&Applied::GildNext));
                let twists: Vec<Variant> = r
                    .applied
                    .iter()
                    .filter_map(|a| match a {
                        Applied::TwistNext(v) => Some(*v),
                        _ => None,
                    })
                    .collect();
                if matches!(theme, DreamTheme::Lobby | DreamTheme::Awakening) {
                    assert!(twists.is_empty(), "{theme:?} twisted");
                }
                for v in twists {
                    assert_ne!(v, Variant::Gilded);
                    assert!(v.fits(theme), "{theme:?} got {v:?}");
                }
            }
        }
    }

    #[test]
    fn the_steady_moon_gives_wide_eyes_once_then_a_little_dust() {
        let moon = event(Id::SecondMoon);
        let mut run = loaded_run();
        let r = resolve(&moon, 1, 1, &run, 5, DreamTheme::Garden, 0).unwrap();
        assert_eq!(r.applied, vec![Applied::Take(Upgrade::WideEyes)]);
        for a in &r.applied {
            apply_to_run(&mut run, a);
        }
        while run.available(Upgrade::WideEyes) {
            run.take(Upgrade::WideEyes);
        }
        let r = resolve(&moon, 1, 1, &run, 5, DreamTheme::Garden, 0).unwrap();
        assert_eq!(r.applied, vec![Applied::Dust(CONSOLATION)]);
    }

    #[test]
    fn the_chair_gives_a_reroll_or_dust() {
        let chair = event(Id::EmptyChair);
        let mut run = loaded_run();
        let before = run.rerolls_left();
        let r = resolve(&chair, 0, 1, &run, 5, DreamTheme::Garden, 0).unwrap();
        for a in &r.applied {
            apply_to_run(&mut run, a);
        }
        assert_eq!(run.rerolls_left(), before + 1);
        let r = resolve(&chair, 1, 1, &run, 5, DreamTheme::Garden, 0).unwrap();
        assert_eq!(
            r.applied,
            vec![Applied::Dust(CHAIR_DUST as i32), Applied::ThinNextPick]
        );
        let full = run.choice_count();
        for a in &r.applied {
            apply_to_run(&mut run, a);
        }
        assert_eq!(run.choice_count(), full - 1, "the chair's price");
    }

    // === events2 ===
    fn go(id: Id, choice: usize, run: &RunUpgrades, dust: u32, seed: u64) -> Option<Resolved> {
        resolve(&event(id), choice, seed, run, 6, DreamTheme::Garden, dust)
    }

    fn with_curse() -> RunUpgrades {
        let mut run = loaded_run();
        run.take(Upgrade::Cursed(ALL_CURSES[0]));
        run
    }

    #[test]
    fn there_are_seventeen_events_each_turning_up() {
        assert_eq!(ALL.len(), 17);
        let ids: HashSet<Id> = ALL.iter().copied().collect();
        assert_eq!(ids.len(), 17);
        let seen: HashSet<Id> = (0..3000).map(|s| pick(s, None)).collect();
        assert_eq!(seen.len(), 17);
    }

    #[test]
    fn shops_charge_and_hand_over_an_upgrade() {
        let run = loaded_run();
        for (id, choice, cost) in [
            (Id::LanternVendor, 0, CUPFUL_DUST),
            (Id::LanternVendor, 1, LANTERN_DUST),
            (Id::UnfinishedLibrary, 0, PAGE_DUST),
        ] {
            assert!(go(id, choice, &run, cost - 1, 1).is_none(), "{id:?} needs {cost}");
            let r = go(id, choice, &run, cost, 1).unwrap();
            assert_eq!(r.applied[0], Applied::Dust(-(cost as i32)));
            assert!(matches!(r.applied[1], Applied::Take(u) if !matches!(u, Upgrade::Cursed(_))));
        }
        // The cupful is a common upgrade.
        for seed in 0..100 {
            let r = go(Id::LanternVendor, 0, &run, 99, seed).unwrap();
            assert!(matches!(r.applied[1], Applied::Take(u) if u.tier() == Tier::Common));
        }
    }

    #[test]
    fn the_well_pays_out_about_half_the_time_and_the_swap_trades_one_for_one() {
        let run = loaded_run();
        let wins = (0..400)
            .filter(|&s| go(Id::WishingWell, 0, &run, 99, s).unwrap().applied.contains(&Applied::Dust(WELL_WIN)))
            .count();
        assert!((140..260).contains(&wins), "{wins}");
        let r = go(Id::WishingWell, 1, &run, 0, 3).unwrap();
        assert!(matches!(r.applied[0], Applied::Lose(_)));
        assert!(matches!(r.applied[1], Applied::Take(_)));
        let mut bare = RunUpgrades::default();
        bare.take(Upgrade::Learn(Ability::Dash));
        assert!(go(Id::WishingWell, 1, &bare, 0, 3).is_none(), "nothing to throw");
    }

    #[test]
    fn the_tailor_lends_a_thin_pick_and_the_bakery_and_tailor_give_rerolls() {
        let run = loaded_run();
        let r = go(Id::PatientTailor, 0, &run, PAGE_DUST, 1).unwrap();
        assert!(r.applied.contains(&Applied::ThinNextPick) && r.applied.contains(&Applied::OpenPick));
        let mut after = run.clone();
        let before = after.rerolls_left();
        for a in &go(Id::PatientTailor, 1, &run, SEAM_DUST, 1).unwrap().applied {
            apply_to_run(&mut after, a);
        }
        assert_eq!(after.rerolls_left(), before + 2);
        let mut after = run.clone();
        for a in &go(Id::NightBakery, 0, &run, ROLL_DUST, 1).unwrap().applied {
            apply_to_run(&mut after, a);
        }
        assert_eq!(after.rerolls_left(), before + 1);
        assert_eq!(go(Id::NightBakery, 1, &run, 0, 1).unwrap().applied, vec![Applied::Dust(HANDOUT_DUST)]);
    }

    #[test]
    fn the_pond_trades_a_curse_for_a_shard_and_lets_a_held_one_go() {
        let run = loaded_run();
        let r = go(Id::MirrorPond, 0, &run, 0, 1).unwrap();
        assert!(matches!(r.applied[0], Applied::Take(Upgrade::Cursed(_))));
        assert!(r.applied.contains(&Applied::BankShard));
        // Drinking needs a curse to let go of, and removes it.
        assert!(go(Id::MirrorPond, 1, &run, 0, 1).is_none());
        let held = with_curse();
        let r = go(Id::MirrorPond, 1, &held, 0, 1).unwrap();
        let mut after = held.clone();
        for a in &r.applied {
            apply_to_run(&mut after, a);
        }
        assert!(!after.taken.iter().any(|u| matches!(u, Upgrade::Cursed(_))));
    }

    #[test]
    fn the_lighthouse_piano_stairs_and_crows_bend_the_next_dream() {
        let run = loaded_run();
        let has_twist = |r: &Resolved| r.applied.iter().any(|a| matches!(a, Applied::TwistNext(_)));
        assert!(has_twist(&go(Id::DryLighthouse, 0, &run, 0, 2).unwrap()));
        assert!(has_twist(&go(Id::TellingCrows, 0, &run, 0, 2).unwrap()));
        assert!(has_twist(&go(Id::DownStaircase, 0, &run, 0, 2).unwrap()));
        assert!(go(Id::DownStaircase, 0, &run, 0, 2).unwrap().applied.contains(&Applied::Dust(STAIRS_DUST)));
        for (id, cost) in [(Id::DryLighthouse, BEACON_DUST), (Id::DrownedPiano, TUNING_DUST)] {
            assert!(go(id, 1, &run, cost - 1, 2).is_none());
            assert!(go(id, 1, &run, cost, 2).unwrap().applied.contains(&Applied::GildNext));
        }
        let r = go(Id::DownStaircase, 1, &run, 0, 2).unwrap();
        assert_eq!(r.applied, vec![Applied::GildNext, Applied::ThinNextPick]);
        // A calm dream cannot twist: the lighthouse gives a little dust instead.
        let r = resolve(&event(Id::DryLighthouse), 0, 2, &run, 6, DreamTheme::Lobby, 0).unwrap();
        assert_eq!(r.applied, vec![Applied::Dust(CONSOLATION)]);
    }

    #[test]
    fn the_crows_sell_a_shard_and_the_library_tears_one_out() {
        let run = loaded_run();
        assert!(go(Id::TellingCrows, 1, &run, CROW_DUST - 1, 1).is_none());
        let r = go(Id::TellingCrows, 1, &run, CROW_DUST, 1).unwrap();
        assert_eq!(r.applied, vec![Applied::Dust(-(CROW_DUST as i32)), Applied::BankShard]);
        let r = go(Id::UnfinishedLibrary, 1, &run, 0, 1).unwrap();
        assert_eq!(r.applied, vec![Applied::BankShard, Applied::ThinNextPick]);
    }

    #[test]
    fn the_chord_gives_an_upgrade_or_a_little_dust_and_the_giant_gambles() {
        let run = loaded_run();
        for id in [Id::DrownedPiano, Id::ForgottenBirthday] {
            let (mut up, mut dust) = (0, 0);
            for seed in 0..200 {
                match go(id, if id == Id::DrownedPiano { 0 } else { 0 }, &run, 0, seed).unwrap().applied[..] {
                    [Applied::Take(_)] => up += 1,
                    [Applied::Dust(n)] if n == CHORD_CONSOLATION => dust += 1,
                    ref other => panic!("{other:?}"),
                }
            }
            assert!(up > 50 && dust > 50, "{id:?}: {up} / {dust}");
        }
        let (mut gifts, mut curses) = (0, 0);
        for seed in 0..200 {
            match go(Id::SleepingGiant, 0, &run, 0, seed).unwrap().applied[..] {
                [Applied::Take(Upgrade::Cursed(_))] => curses += 1,
                [Applied::Take(_)] => gifts += 1,
                ref other => panic!("{other:?}"),
            }
        }
        assert!(gifts > 50 && curses > 50);
        assert_eq!(go(Id::SleepingGiant, 1, &run, 0, 1).unwrap().applied, vec![Applied::Dust(HANDOUT_DUST)]);
        assert_eq!(go(Id::ForgottenBirthday, 1, &run, 0, 1).unwrap().applied, vec![Applied::Dust(HANDOUT_DUST)]);
    }
    // === end events2 ===

    #[test]
    fn apply_to_run_takes_and_loses_exactly_one() {
        let mut run = loaded_run();
        let n = run.count(Upgrade::SwiftFeet);
        apply_to_run(&mut run, &Applied::Lose(Upgrade::SwiftFeet));
        assert_eq!(run.count(Upgrade::SwiftFeet), n - 1);
        apply_to_run(&mut run, &Applied::Lose(Upgrade::SwiftFeet));
        assert_eq!(run.count(Upgrade::SwiftFeet), 0, "losing what you lack is a no-op");
        apply_to_run(&mut run, &Applied::Take(Upgrade::SwiftFeet));
        assert_eq!(run.count(Upgrade::SwiftFeet), 1);
    }
}
