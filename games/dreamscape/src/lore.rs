//! Who is dreaming. Every save has its own dreamer (you) and eight others
//! whose dreams leak into yours. Each carries a weight: something happening
//! in their life. Notes found in dreams tell their stories in order.
//! Everything is generated from the save's seed, so a save always tells the
//! same story, and two saves tell different ones.

use crate::dream::DreamTheme;
use rand::{rngs::StdRng, seq::SliceRandom, Rng, SeedableRng};
use serde::{Deserialize, Serialize};

/// You (id 0) plus eight others.
pub const DREAMERS: u32 = 9;
/// Loadout slots: 1 at first, one more per 3 of your own notes.
pub const MAX_SLOTS: usize = 5;

pub fn notes_for(id: u32) -> u32 {
    if id == 0 {
        12
    } else {
        6
    }
}

/// Another dreamer's k-th note only turns up at this depth or deeper
/// (collecting someone should take real runs).
pub fn depth_for(k: u32) -> u32 {
    3 * k
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Weight {
    Grief,
    Burnout,
    Anxiety,
    Loneliness,
    Guilt,
    Insomnia,
    Heartbreak,
    Pressure,
}

pub const ALL_WEIGHTS: [Weight; 8] = [
    Weight::Grief,
    Weight::Burnout,
    Weight::Anxiety,
    Weight::Loneliness,
    Weight::Guilt,
    Weight::Insomnia,
    Weight::Heartbreak,
    Weight::Pressure,
];

impl Weight {
    /// Shown once the dreamer is collected (never before: it's their secret).
    pub fn label(self) -> &'static str {
        match self {
            Weight::Grief => "grief",
            Weight::Burnout => "burnout",
            Weight::Anxiety => "anxiety",
            Weight::Loneliness => "loneliness",
            Weight::Guilt => "guilt",
            Weight::Insomnia => "sleeplessness",
            Weight::Heartbreak => "heartbreak",
            Weight::Pressure => "pressure",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dreamer {
    pub id: u32,
    pub name: String,
    pub job: &'static str,
    pub place: &'static str,
    /// The person at the centre of their story, and who they are to them.
    pub person: String,
    pub relation: &'static str,
    pub weight: Weight,
}

#[rustfmt::skip]
const FIRST: [&str; 48] = ["Ada","Ben","Cleo","Dev","Eli","Farah","Gus","Hana","Ivo","June","Kai","Lena","Milo","Nia","Otto","Pia","Quinn","Rosa","Sami","Tess","Uri","Vera","Wren","Yuki","Zane","Alba","Bram","Cora","Dani","Esme","Finn","Gia","Hugo","Iris","Jonah","Kira","Luca","Maya","Noor","Owen","Priya","Remy","Sol","Theo","Una","Vik","Willa","Yara"];
#[rustfmt::skip]
const JOBS: [&str; 24] = ["nurse","teacher","line cook","night guard","accountant","bus driver","student","translator","vet","barista","engineer","florist","paramedic","librarian","warehouse picker","call-centre agent","carpenter","designer","cleaner","pharmacist","lab tech","delivery rider","social worker","musician"];
#[rustfmt::skip]
const PLACES: [&str; 16] = ["the station","the corner shop","the hospital car park","the old school","the laundrette","the bridge","the office lift","the bus stop","the pier","the night bus","the supermarket","the waiting room","the park bench","the 24-hour diner","the stairwell","the library"];
#[rustfmt::skip]
const RELATIONS: [&str; 8] = ["sister","brother","mum","dad","best friend","partner","grandad","old flatmate"];

/// Save seed + id: the same person every time.
pub fn dreamer(save_seed: u64, id: u32) -> Dreamer {
    // Names are dealt from one shuffle per save so they never repeat.
    let mut deck = StdRng::seed_from_u64(save_seed ^ 0xD4EA_4E45);
    let mut firsts = FIRST.to_vec();
    firsts.shuffle(&mut deck);
    let mut weights = ALL_WEIGHTS.to_vec();
    weights.shuffle(&mut deck);
    let mut rng = StdRng::seed_from_u64(save_seed ^ (id as u64 + 1).wrapping_mul(0x9E37_79B9));
    let weight = if id == 0 {
        weights[rng.gen_range(0..weights.len())]
    } else {
        weights[(id as usize - 1) % weights.len()]
    };
    Dreamer {
        id,
        name: firsts[id as usize].to_string(),
        job: JOBS[rng.gen_range(0..JOBS.len())],
        place: PLACES[rng.gen_range(0..PLACES.len())],
        person: firsts[DREAMERS as usize + id as usize].to_string(),
        relation: RELATIONS[rng.gen_range(0..RELATIONS.len())],
        weight,
    }
}

/// Whose dream this is: you 40% of the time.
pub fn owner(save_seed: u64, dream_seed: u64) -> u32 {
    let h = StdRng::seed_from_u64(save_seed ^ dream_seed ^ 0x0A7E).gen::<u32>();
    if h % 100 < 40 {
        0
    } else {
        1 + (h / 100) % (DREAMERS - 1)
    }
}

/// The thing a note is written on, or tucked into, in each dream.
pub fn object(theme: DreamTheme, _k: u32) -> &'static str {
    use DreamTheme::*;
    match theme {
        Lobby => "luggage tag",
        LiminalOffice => "sticky note",
        VoidPlatforms => "torn page",
        Garden => "seed packet",
        NightmareFactory => "timecard",
        Awakening => "postcard",
        CursedForest => "carved bark",
        DrownedLibrary => "wet bookmark",
        SkyStairs => "paper plane",
        MirrorHall => "note written backwards",
        MyceliumGrove => "pressed leaf",
        TheTunnel => "receipt",
        FractalCathedral => "prayer card",
        Elfworks => "gift tag",
        AfterimageFields => "photo booth strip",
        SynesthesiaHall => "setlist",
        MeltingClockworks => "appointment card",
        JellyfishSky => "message in a bottle",
        WatchingWallpaper => "sketch on the wallpaper",
        WhiteDissolve => "blank page, with one line on it",
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    pub title: String,
    pub body: String,
    pub object: &'static str,
}

/// One story per weight, six beats: a hint, it growing, the worst of it,
/// a turn, reaching out, and some peace. `{p}` = their person,
/// `{place}`, `{job}`, `{obj}` filled in per dreamer and dream.
#[rustfmt::skip]
fn arc(w: Weight) -> [&'static str; 6] {
    match w {
        Weight::Grief => [
            "{p}'s number is still in my phone. I keep not deleting it.",
            "Went to {place} today. {p} would have hated the new paint.",
            "Everyone says it gets easier. It gets quieter. That's not the same thing.",
            "Dreamed {p} was in the kitchen making tea like nothing happened. I didn't want to wake up.",
            "Told someone about {p} today. My voice broke. They waited. That helped more than I thought.",
            "I kept the {obj}. Some days it hurts to look at. Today it just looked like something {p} loved.",
        ],
        Weight::Burnout => [
            "Answered emails at 1am again. Nobody asked me to. Nobody would notice if I stopped.",
            "Forgot {p}'s birthday. Remembered every deadline.",
            "Sat in the car outside {place} for twenty minutes. Couldn't make myself go in.",
            "The doctor asked when I last rested. I laughed. She didn't.",
            "Took a day off. The world didn't end. Neither did I.",
            "Being a {job} is what I do. It isn't all of who I am. Writing it down so I believe it.",
        ],
        Weight::Anxiety => [
            "Rehearsed ordering a coffee three times. Still said 'you too' when they said enjoy.",
            "Heart racing at {place}. Nothing was wrong. That's the worst part: nothing was wrong.",
            "Cancelled on {p} again. Told myself it was the weather.",
            "Tried the breathing thing. In for four, hold, out for six. It didn't fix it. It made it smaller.",
            "{p} said I don't have to explain. I explained anyway. They listened anyway.",
            "The fear still comes. I'm learning it doesn't get to drive.",
        ],
        Weight::Loneliness => [
            "Talked to the cashier at {place} longer than I needed to. First conversation all week.",
            "Scrolled past {p}'s photos. Everyone is always somewhere with someone.",
            "Set the table for one. Put a second glass out by habit.",
            "Wrote a message to {p}. Didn't send it. Wrote it again.",
            "Sent it. {p} replied in four minutes. Four minutes.",
            "Went to the thing at {place}. Didn't talk much. Going back next week anyway.",
        ],
        Weight::Guilt => [
            "I said something to {p} I can't take back. I replay it every night.",
            "Something good happened and my first thought was: you don't deserve this.",
            "Walked past {place} and crossed the street. As if the building remembers.",
            "Tried to write {p} an apology. Everything sounded like an excuse.",
            "Said sorry out loud. Not perfectly. {p} said 'thank you for saying it.'",
            "I can carry what I did without letting it carry me. Still learning which is which.",
        ],
        Weight::Insomnia => [
            "3:12am. The ceiling has a crack shaped like {place}.",
            "Counted backwards from a thousand. Got to the bottom. Started again.",
            "Fell asleep on shift as a {job}. Woke up with the {obj} stuck to my cheek.",
            "{p} says I talk in my sleep now, when I get any. Mostly I say 'wait'.",
            "No screens after ten. Window open. Slept six hours. Felt like a miracle.",
            "Maybe sleep isn't something you win. Maybe it's something you let happen.",
        ],
        Weight::Heartbreak => [
            "{p}'s jacket is still on the hook. I walk around it.",
            "Heard our song at {place}. Left my basket in the aisle.",
            "Rewrote our last conversation a hundred ways. It ends the same in every one.",
            "Deleted the photos. Undeleted them. Deleted them.",
            "Laughed at something today and didn't immediately want to tell {p}.",
            "It was real and it's over. Both are true. I'm letting both be true.",
        ],
        Weight::Pressure => [
            "Everyone at {place} thinks I have it together. The {obj} in my bag says otherwise.",
            "{p} asked what I want to do. I told them what I'm supposed to do.",
            "Made a list of everything I owe people. Ran out of paper.",
            "Came second. Felt like last.",
            "Told {p} I'm tired of being the reliable one. They said: 'you're allowed to be tired.'",
            "I don't have to earn rest. Writing it until it stops sounding like a lie.",
        ],
    }
}

/// Your own story also has six notes about the eye and the dream.
#[rustfmt::skip]
const SELF: [&str; 6] = [
    "There's an eye behind my eyes. It's been watching this whole time. I think it's mine.",
    "The nightmares aren't monsters. They're the things I haven't said.",
    "The eye opens when I stop running. I keep forgetting that.",
    "Other people's dreams leak into mine down here. Everyone is carrying something.",
    "I don't know how long I've been asleep. These notes help me remember who I was.",
    "The bottom isn't the end of the dream. It's where I keep the thing I'm afraid of.",
];

fn fill(t: &str, d: &Dreamer, theme: DreamTheme, k: u32) -> String {
    t.replace("{p}", &d.person)
        .replace("{place}", d.place)
        .replace("{job}", d.job)
        .replace("{obj}", object(theme, k))
}

/// The k-th note of dreamer `d`, as found in a `theme` dream.
pub fn note(d: &Dreamer, k: u32, theme: DreamTheme) -> Note {
    let body = if d.id == 0 {
        // Yours alternate: something about the dream, then something about your life.
        if k % 2 == 0 {
            SELF[(k / 2) as usize % 6].to_string()
        } else {
            arc(d.weight)[(k / 2) as usize % 6].to_string()
        }
    } else {
        arc(d.weight)[k as usize % 6].to_string()
    };
    let who = if d.id == 0 {
        "you".to_string()
    } else {
        d.name.clone()
    };
    Note {
        title: format!("{} · {} of {}", who, k + 1, notes_for(d.id)),
        body: fill(&body, d, theme, k),
        object: object(theme, k),
    }
}

/// Everything the save remembers about the story.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Lore {
    /// Fixed on first launch; every dreamer comes from it.
    pub save_seed: u64,
    /// (dreamer, note index) found.
    pub notes: Vec<(u32, u32)>,
    /// Other dreamers collected (every note found), in order.
    pub dreamers: Vec<u32>,
    pub prologue_done: bool,
    /// One-off eye lines and hints already shown.
    pub seen: Vec<String>,
    /// Booklet card numbers in the loadout.
    pub loadout: Vec<u32>,
    /// The collected dreamer riding along this run (M3).
    pub companion: Option<u32>,
    /// Fusion recipes discovered (sorted theme pairs, as indices into ALL_THEMES).
    pub recipes: Vec<(usize, usize)>,
    pub ending_seen: bool,
}

impl Lore {
    #[cfg(test)]
    pub fn new(save_seed: u64) -> Self {
        Self {
            save_seed,
            ..Default::default()
        }
    }

    pub fn has(&self, id: u32, k: u32) -> bool {
        self.notes.contains(&(id, k))
    }

    pub fn found(&mut self, id: u32, k: u32) {
        if !self.has(id, k) {
            self.notes.push((id, k));
        }
    }

    pub fn count(&self, id: u32) -> u32 {
        self.notes.iter().filter(|(d, _)| *d == id).count() as u32
    }

    pub fn dreamer_complete(&self, id: u32) -> bool {
        self.count(id) >= notes_for(id)
    }

    /// The next note of `id` that can turn up at `depth`, in story order.
    pub fn next_note(&self, id: u32, depth: u32) -> Option<u32> {
        let k = (0..notes_for(id)).find(|&k| !self.has(id, k))?;
        (id == 0 || depth >= depth_for(k)).then_some(k)
    }

    pub fn slots(&self) -> usize {
        (1 + self.count(0) as usize / 3).min(MAX_SLOTS)
    }

    pub fn seen_once(&mut self, key: &str) -> bool {
        if self.seen.iter().any(|s| s == key) {
            false
        } else {
            self.seen.push(key.to_string());
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::{DreamTheme, ALL_THEMES};
    use std::collections::HashSet;

    #[test]
    fn a_save_always_has_the_same_dreamers_and_saves_differ() {
        assert_eq!(dreamer(7, 3), dreamer(7, 3));
        let names = |s| {
            (0..DREAMERS)
                .map(|i| dreamer(s, i).name)
                .collect::<Vec<_>>()
        };
        assert_ne!(names(7), names(8));
        let n: HashSet<String> = names(7).into_iter().collect();
        assert_eq!(n.len(), DREAMERS as usize, "names distinct within a save");
    }

    #[test]
    fn the_other_dreamers_carry_every_weight() {
        for seed in 0..50 {
            let w: HashSet<Weight> = (1..DREAMERS).map(|i| dreamer(seed, i).weight).collect();
            assert_eq!(w.len(), ALL_WEIGHTS.len(), "seed {seed}");
        }
    }

    #[test]
    fn notes_are_filled_in_and_readable() {
        for seed in 0..20 {
            for id in 0..DREAMERS {
                let d = dreamer(seed, id);
                for k in 0..notes_for(id) {
                    for t in ALL_THEMES {
                        let n = note(&d, k, t);
                        assert!(!n.body.contains('{'), "unfilled: {}", n.body);
                        assert!(n.body.len() > 20 && n.body.len() < 220, "{}", n.body);
                        assert!(!n.object.is_empty());
                    }
                }
            }
        }
    }

    #[test]
    fn notes_come_in_order_and_later_ones_need_depth() {
        let mut lore = Lore::new(5);
        assert_eq!(lore.next_note(3, 0), Some(0));
        lore.found(3, 0);
        assert_eq!(
            lore.next_note(3, 1),
            None,
            "note 1 of another dreamer needs depth"
        );
        assert_eq!(lore.next_note(3, depth_for(1)), Some(1));
        for k in 0..notes_for(0) {
            lore.found(0, k);
        }
        assert_eq!(lore.next_note(0, 99), None, "all found");
        assert!(lore.dreamer_complete(0));
    }

    #[test]
    fn loadout_slots_grow_with_your_own_story() {
        let mut lore = Lore::new(1);
        assert_eq!(lore.slots(), 1);
        for k in 0..notes_for(0) {
            lore.found(0, k);
        }
        assert_eq!(lore.slots(), MAX_SLOTS);
        let mut half = Lore::new(1);
        for k in 0..6 {
            half.found(0, k);
        }
        assert_eq!(half.slots(), 3);
    }

    #[test]
    fn dreams_belong_to_someone_and_you_most_often() {
        let mine = (0..1000u64).filter(|&s| owner(9, s) == 0).count();
        assert!((300..500).contains(&mine), "{mine}");
        assert!((0..1000u64).all(|s| owner(9, s) < DREAMERS));
    }

    #[test]
    fn old_saves_load_with_empty_lore() {
        let l: Lore = ron::from_str("()").unwrap();
        assert_eq!(l.notes.len(), 0);
        assert!(!l.prologue_done);
    }

    #[test]
    fn every_theme_has_an_object() {
        for t in ALL_THEMES {
            assert!(!object(t, 0).is_empty(), "{t:?}");
        }
        let _ = DreamTheme::Lobby;
    }
}
