//! Relic names: a seeded grammar that makes a relic read like what it is.
//!
//! The head noun comes from the weapon's `Shape` (its role), modifiers come
//! from the dream theme's own word bank, rarity picks the template family,
//! wear / foil / the strongest effect lever add words. Pure: `worth::mix`
//! only, no clock, no global RNG.

use crate::dream::vocab::{vocab, Vocab, ABSTRACTS};
use crate::dream::{DreamTheme, Shape, ALL_THEMES};
use crate::relic_types::{RelicEffect, RelicRarity};
use crate::relics::WearTier;
use crate::worth::{mix, percent_roll};

pub const MAX_NAME: usize = 40;
pub const MAX_FLAVOUR: usize = 56;
/// Same rule as `dream/names.rs`: one slot borrowed from a donor dream.
pub const MASHUP_CHANCE: u32 = 25;
const RETRIES: u32 = 12;
/// Chance the strongest effect lever shows up in the name.
const HINT_CHANCE: u32 = 75;

pub struct RelicSpec<'a> {
    pub theme: DreamTheme,
    pub seed: u64,
    pub rarity: RelicRarity,
    pub wear: WearTier,
    pub foil: bool,
    pub shape: Shape,
    pub effect: RelicEffect,
    /// Names the owner already has; the result avoids them.
    pub taken: &'a [String],
}

// ---------------------------------------------------------------- words

/// Role words per shape. A relic's name always contains one of these.
#[rustfmt::skip]
pub fn head_nouns(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::Cube => &[
            "DIE", "RELIQUARY", "BRICK", "SUGAR LUMP", "CRATE", "STRONGBOX", "MUSIC BOX",
            "LUNCHBOX", "KEYSTONE", "TILE", "BLOCK", "JEWEL CASE",
        ],
        Shape::Octahedron => &[
            "DIAMOND", "LOZENGE", "PRISM", "CRYSTAL", "JEWEL", "GEM", "SPINDLE", "KITE",
            "SPARKLER", "TWIN PYRAMID",
        ],
        Shape::Orb => &[
            "PEARL", "MARBLE", "EYE", "BAUBLE", "GLOBE", "BUBBLE", "SNOW GLOBE", "SEED",
            "YOLK", "MOON", "BEAD", "BERRY",
        ],
        Shape::Cylinder => &[
            "PILLAR", "SCROLL", "CANDLE", "BATTERY", "BARREL", "TORCH", "ROLLING PIN",
            "TUBE", "FLASK", "TRUNCHEON", "DRUM", "SPOOL",
        ],
        Shape::Cone => &[
            "SPIRE", "FUNNEL", "PARTY HAT", "THORN", "FANG", "DUNCE CAP", "CARROT",
            "STEEPLE", "WIZARD HAT", "NEEDLE", "TUSK", "SNOW CONE",
        ],
        Shape::Torus => &[
            "HOOP", "HALO", "RING", "NOOSE", "BANGLE", "WREATH", "LIFEBUOY", "DOUGHNUT",
            "BAGEL", "GASKET", "COLLAR", "LOOP",
        ],
        Shape::Arch => &[
            "GATE", "DOORWAY", "ARCH", "PORTAL", "ARCHWAY", "LINTEL", "WICKET", "TRELLIS",
            "THRESHOLD", "DOORFRAME", "HORSESHOE", "YOKE",
        ],
        Shape::Cloud => &[
            "CLOUD", "FLEECE", "HEAP", "PUFF", "CLUMP", "TUFT", "BUNDLE", "COTTON BALL",
            "SPONGE", "MOP", "DRIFT", "NEST",
        ],
        Shape::Crescent => &[
            "SICKLE", "HORN", "SMILE", "PARING", "MOON-BLADE", "SCYTHE", "CLAW", "TALON",
            "SLIVER", "SCIMITAR", "GRIN", "CRESCENT",
        ],
        Shape::Island => &[
            "CRAG", "BOULDER", "ISLET", "MESA", "MOLAR", "PEBBLE", "BARNACLE", "STALACTITE",
            "ANVIL", "ISLAND", "GEODE", "KEEP",
        ],
        Shape::Stairs => &[
            "STAIRCASE", "STEPS", "LADDER", "STAIR", "RUNGS", "ESCALATOR", "ZIGGURAT",
            "STEPLADDER", "FLIGHT", "LANDING", "SPIRAL STAIR", "PLINTH",
        ],
    }
}

/// Words for a lifted-edge relic: wear that reads in the name (Cracked family).
#[rustfmt::skip]
const CRACKED_ADJ: &[&str] = &[
    "CHIPPED", "CRACKED", "SPLIT", "BENT", "MENDED", "STAINED", "SCUFFED", "DENTED",
    "FRAYED", "HOLLOWED", "PITTED", "SECONDHAND", "TARNISHED", "NOTCHED", "SPENT",
    "GLUED", "SCRATCHED", "CHEAP", "LOANED", "FORGOTTEN",
];

#[rustfmt::skip]
const FOIL_WORDS: &[&str] = &[
    "GILT", "PRISMATIC", "SHIMMERING", "OPALESCENT", "IRIDESCENT", "GLITTERING",
    "PEARLESCENT", "HOLOGRAPHIC", "MOTHER-OF-PEARL", "LUSTROUS", "SILVERED", "SPANGLED",
    "MOONLIT", "STARDUSTED",
];

/// Strongest lever -> hint words. Entries starting "OF " go after the noun,
/// the rest go before it.
#[rustfmt::skip]
const DUST: &[&str] = &[
    "HOARDER'S", "PLENTIFUL", "MAGPIE'S", "JINGLING", "OVERFLOWING", "TITHING",
    "MINTED", "PROSPEROUS", "OF PLENTY", "OF GOOD HARVEST", "OF TITHES",
];
#[rustfmt::skip]
const SPEED: &[&str] = &[
    "SWIFT", "FLEETING", "HURRIED", "WINDBORNE", "RESTLESS", "NIMBLE", "LIGHT-FOOTED",
    "FEATHERED", "OF HASTE", "OF LONG STRIDES", "OF THE LATE BUS",
];
#[rustfmt::skip]
const CALM: &[&str] = &[
    "HUSHED", "SOOTHING", "SLEEPY", "MUTED", "LULLING", "PEACEFUL", "DROWSY", "STILLED",
    "OF HUSH", "OF QUIET BREATHS", "OF SLOW TIDES",
];
#[rustfmt::skip]
const MEMORY: &[&str] = &[
    "REMEMBERING", "KEEPSAKE", "RECALLING", "NOSTALGIC", "MINDFUL", "ARCHIVAL",
    "RETENTIVE", "DOG-EARED", "OF RECALL", "OF GOOD MEMORY", "OF OLD LETTERS",
];
#[rustfmt::skip]
const REROLLS: &[&str] = &[
    "SECOND-GUESSING", "DOUBTING", "FICKLE", "SHUFFLING", "REDRAWN", "UNDECIDED",
    "RETRYING", "TWO-MINDED", "OF SECOND CHANCES", "OF ANOTHER GO", "OF CHANGED MINDS",
];
#[rustfmt::skip]
const SHARDS: &[&str] = &[
    "SHARD-BORN", "GLINTING", "SPLINTERED", "FACETED", "SHARD-HUNGRY", "GLASSY",
    "SPARKLING", "SHATTERED", "OF SHARDS", "OF BROKEN GLASS", "OF FALLEN STARS",
];
#[rustfmt::skip]
const GRACE: &[&str] = &[
    "FORGIVING", "MERCIFUL", "KINDLY", "GENTLE", "SPARING", "LENIENT", "PARDONING",
    "FORBEARING", "OF MERCY", "OF KINDNESS", "OF SMALL FAVOURS",
];
#[rustfmt::skip]
const NERVE: &[&str] = &[
    "STEADY", "STEELY", "BRAVE", "UNFLINCHING", "STOUT", "STAUNCH", "COURAGEOUS",
    "UNSHAKEN", "OF NERVE", "OF STEADY HANDS", "OF STILL KNEES",
];

/// The effect's strongest lever (each normalised by a typical quirk value) and
/// the words that hint at it. `None` when the effect does nothing.
pub fn effect_hint(e: &RelicEffect) -> Option<&'static [&'static str]> {
    let levers: [(f32, &'static [&'static str]); 8] = [
        (e.dust_bonus_pct as f32 / 10.0, DUST),
        (e.memory_bonus / 0.05, MEMORY),
        (e.speed_pct as f32 / 5.0, SPEED),
        (e.grace_bonus as f32, GRACE),
        (e.calm_pct as f32 / 10.0, CALM),
        (e.free_rerolls as f32, REROLLS),
        (e.start_shards as f32, SHARDS),
        (e.max_nerve as f32 / 10.0, NERVE),
    ];
    let mut best: Option<(f32, &'static [&'static str])> = None;
    for (score, list) in levers {
        if score > 0.0 && best.map_or(true, |(b, _)| score > b) {
            best = Some((score, list));
        }
    }
    best.map(|(_, l)| l)
}

fn wear_word(w: WearTier) -> Option<&'static str> {
    match w {
        WearTier::Heavy => Some("HEAVILY-WORN"),
        WearTier::Ruined => Some("RUINED"),
        WearTier::Clean | WearTier::Light => None,
    }
}

// ------------------------------------------------------------- randomness

fn h(seed: u64, tag: u64, attempt: u32) -> u64 {
    mix(mix(seed, tag), attempt as u64 + 0x5A17)
}

fn pick<T: Copy>(list: &[T], hash: u64) -> T {
    list[(hash % list.len() as u64) as usize]
}

/// A word different from `not` (when the list allows).
fn pick_other(list: &'static [&'static str], hash: u64, not: &str) -> &'static str {
    let w = pick(list, hash);
    if w != not {
        return w;
    }
    list[((hash % list.len() as u64) as usize + 1) % list.len()]
}

fn donor(theme: DreamTheme, hash: u64) -> DreamTheme {
    let pool: Vec<DreamTheme> = ALL_THEMES
        .iter()
        .copied()
        .filter(|&t| t != theme && t != DreamTheme::Awakening)
        .collect();
    pick(&pool, hash)
}

// ------------------------------------------------------------------- name

/// The words a template can use, all already chosen.
struct Slots {
    adj: &'static str,
    adj2: &'static str,
    tn: &'static str,
    rel: &'static str,
    abs: &'static str,
    cadj: &'static str,
    noun: &'static str,
}

fn slots(s: &RelicSpec, attempt: u32) -> Slots {
    let own: Vocab = vocab(s.theme);
    let other: Vocab = if percent_roll(s.seed, 0x4D41_5348 ^ attempt as u64) < MASHUP_CHANCE
        && s.theme != DreamTheme::Awakening
    {
        vocab(donor(s.theme, h(s.seed, 1, attempt)))
    } else {
        vocab(s.theme)
    };
    // The mashup swaps exactly one of adjective / theme noun / relative.
    let swap = h(s.seed, 2, attempt) % 3;
    let adj_src = if swap == 0 { &other } else { &own };
    let tn_src = if swap == 1 { &other } else { &own };
    let rel_src = if swap == 2 { &other } else { &own };
    let adj = pick(adj_src.adjectives, h(s.seed, 3, attempt));
    let abs = if h(s.seed, 6, attempt) % 2 == 0 {
        pick(own.abstracts, h(s.seed, 7, attempt))
    } else {
        pick(ABSTRACTS, h(s.seed, 7, attempt))
    };
    let noun = pick(head_nouns(s.shape), h(s.seed, 9, attempt));
    // A theme noun that echoes the head noun ("GATE GATE") is swapped out.
    let mut tn = pick(tn_src.nouns, h(s.seed, 5, attempt));
    if tn.contains(noun) || noun.contains(tn) {
        tn = pick(tn_src.nouns, h(s.seed, 55, attempt) | 1);
        if tn.contains(noun) || noun.contains(tn) {
            tn = "DREAM";
        }
    }
    Slots {
        adj,
        adj2: pick_other(adj_src.adjectives, h(s.seed, 4, attempt), adj),
        tn,
        rel: pick(rel_src.relatives, h(s.seed, 8, attempt)),
        abs,
        cadj: pick(CRACKED_ADJ, h(s.seed, 10, attempt)),
        noun,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lead {
    None,
    The,
    A,
    Owner,
}

/// One template: lead, soft word (dropped when wear already says it), modifiers,
/// noun, tail.
struct Core {
    lead: Lead,
    owner: String,
    soft: Option<&'static str>,
    mods: Vec<&'static str>,
    noun: &'static str,
    tail: String,
}

fn possessive(abs: &str) -> String {
    if abs.ends_with('S') {
        format!("{abs}'")
    } else {
        format!("{abs}'S")
    }
}

/// Template count per family; index picks which one.
const FAMILY: usize = 8;

fn core(rarity: RelicRarity, k: usize, x: &Slots) -> Core {
    let c = |lead, soft, mods: Vec<&'static str>, tail: String| Core {
        lead,
        owner: String::new(),
        soft,
        mods,
        noun: x.noun,
        tail,
    };
    match rarity {
        // Plain, chipped, secondhand.
        RelicRarity::Cracked => match k % FAMILY {
            0 => c(Lead::A, Some(x.cadj), vec![x.adj], format!("FROM {}", x.abs)),
            1 => c(Lead::A, Some(x.cadj), vec![x.tn], String::new()),
            2 => c(Lead::None, Some(x.cadj), vec![x.adj, x.tn], String::new()),
            3 => c(Lead::A, Some(x.cadj), vec![x.adj, x.adj2], String::new()),
            4 => c(Lead::None, None, vec![x.adj, x.tn], format!("LEFT IN {}", x.abs)),
            5 => c(Lead::A, Some(x.cadj), vec![x.tn], format!("OF {}", x.abs)),
            6 => c(Lead::None, Some(x.cadj), vec![x.adj], format!("FROM {}", x.abs)),
            _ => c(Lead::A, Some(x.cadj), vec![x.adj, x.tn], String::new()),
        },
        // "THE {ADJ} {NOUN}" and kin.
        RelicRarity::Worn => match k % FAMILY {
            0 => c(Lead::The, None, vec![x.adj], String::new()),
            1 => c(Lead::The, None, vec![x.adj, x.adj2], String::new()),
            2 => c(Lead::The, None, vec![x.adj, x.tn], String::new()),
            3 => c(Lead::The, None, vec![x.tn], format!("FROM {}", x.abs)),
            4 => c(Lead::None, None, vec![x.adj, x.adj2, x.tn], String::new()),
            5 => c(Lead::The, None, vec![x.adj], format!("FROM {}", x.abs)),
            6 => c(Lead::A, None, vec![x.adj, x.tn], String::new()),
            _ => c(Lead::The, None, vec![x.adj2, x.adj, x.tn], String::new()),
        },
        // "{NOUN} OF {ABSTRACT}".
        RelicRarity::Pristine => match k % FAMILY {
            0 => c(Lead::None, None, vec![x.adj, x.adj2], format!("OF {}", x.abs)),
            1 => c(Lead::The, None, vec![x.adj], format!("OF {}", x.abs)),
            2 => c(Lead::None, None, vec![x.adj, x.tn], format!("OF {}", x.abs)),
            3 => c(Lead::The, None, vec![x.tn], format!("OF {}", x.abs)),
            4 => c(Lead::None, None, vec![x.adj], format!("OF {}", x.abs)),
            5 => c(Lead::The, None, vec![x.adj, x.adj2], format!("OF {}", x.abs)),
            6 => c(Lead::The, None, vec![x.adj, x.tn], format!("OF {}", x.abs)),
            _ => c(Lead::None, None, vec![x.tn], format!("OF {}", x.abs)),
        },
        // Grand, owned.
        RelicRarity::Mythic => match k % FAMILY {
            0 => {
                let mut r = c(Lead::Owner, None, vec![x.adj], String::new());
                r.owner = possessive(x.abs);
                r
            }
            1 => c(Lead::The, None, Vec::new(), format!("THAT {}", x.rel)),
            2 => c(Lead::The, None, vec![x.adj], format!("THAT {}", x.rel)),
            3 => {
                let mut r = c(Lead::Owner, None, vec![x.adj, x.tn], String::new());
                r.owner = possessive(x.abs);
                r
            }
            4 => c(Lead::The, None, vec![x.adj], format!("OF {}", x.abs)),
            5 => c(Lead::The, Some("LAST"), Vec::new(), format!("BEFORE {}", x.abs)),
            6 => {
                let mut r = c(Lead::Owner, None, vec![x.tn], String::new());
                r.owner = possessive(x.abs);
                r
            }
            _ => c(Lead::The, None, vec![x.adj, x.adj2], format!("THAT {}", x.rel)),
        },
    }
}

fn article(next: &str) -> &'static str {
    match next.chars().next() {
        Some('A' | 'E' | 'I' | 'O' | 'U') => "AN",
        _ => "A",
    }
}

/// Which decorations survive when a name is too long.
#[derive(Clone, Copy)]
struct Deco {
    wear: Option<&'static str>,
    foil: Option<&'static str>,
    /// Hint as a leading adjective.
    hint: Option<&'static str>,
    /// Hint as an "OF ..." tail; used only when the template has no tail.
    hint_of: Option<&'static str>,
}

fn assemble(c: &Core, d: Deco) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut pre: Vec<&str> = Vec::new();
    pre.extend(d.wear);
    pre.extend(d.foil);
    let use_of = c.tail.is_empty() && d.hint_of.is_some();
    if !use_of {
        pre.extend(d.hint);
    }
    if let Some(soft) = c.soft {
        // Wear already says "damaged"; the soft Cracked word would stack.
        if d.wear.is_none() || soft == "LAST" {
            pre.push(soft);
        }
    }
    pre.extend(c.mods.iter().copied());
    let first = pre.first().copied().unwrap_or(c.noun);
    match c.lead {
        Lead::None => {}
        Lead::The => words.push("THE".into()),
        Lead::A => words.push(article(first).into()),
        Lead::Owner => words.push(c.owner.clone()),
    }
    words.extend(pre.iter().map(|w| w.to_string()));
    words.push(c.noun.to_string());
    if !c.tail.is_empty() {
        words.push(c.tail.clone());
    } else if use_of {
        words.extend(d.hint_of.map(|w| w.to_string()));
    }
    words.join(" ")
}

/// Fresh word draws tried before a decoration is shed.
const DRAWS: u32 = 6;

fn generate(s: &RelicSpec, attempt: u32, budget: usize) -> String {
    let wear = wear_word(s.wear);
    let list = effect_hint(&s.effect)
        .filter(|_| percent_roll(h(s.seed, 12, attempt), 0x91) < HINT_CHANCE);
    let split = |of: bool| -> Vec<&'static str> {
        list.map(|l| l.iter().copied().filter(|w| w.starts_with("OF ") == of).collect())
            .unwrap_or_default()
    };
    let (pre_words, of_words) = (split(false), split(true));
    let foil = s.foil.then(|| pick(FOIL_WORDS, h(s.seed, 14, attempt)));
    // Prefer every decoration; try fresh words before shedding any, hint first.
    for keep_foil in [true, false] {
        let foil = foil.filter(|_| keep_foil);
        for with_hint in [true, false] {
            for draw in 0..DRAWS {
                let a = attempt + 64 * draw;
                let x = slots(s, a);
                let k0 = h(s.seed, 11, a) as usize;
                let d = Deco {
                    wear,
                    foil,
                    hint: with_hint
                        .then(|| (!pre_words.is_empty()).then(|| pick(&pre_words, h(s.seed, 13, a))))
                        .flatten(),
                    hint_of: with_hint
                        .then(|| {
                            (!of_words.is_empty() && h(s.seed, 15, a) % 2 == 0)
                                .then(|| pick(&of_words, h(s.seed, 16, a)))
                        })
                        .flatten(),
                };
                for i in 0..FAMILY {
                    let name = assemble(&core(s.rarity, k0 + i, &x), d);
                    if name.len() <= budget {
                        return name;
                    }
                }
            }
        }
        // Still too long with the foil: trim the template before shedding it.
        if keep_foil && foil.is_some() {
            let d = Deco { wear, foil, hint: None, hint_of: None };
            for draw in 0..DRAWS {
                let a = attempt + 64 * draw;
                let x = slots(s, a);
                let k0 = h(s.seed, 11, a) as usize;
                for i in 0..FAMILY {
                    let mut c = core(s.rarity, k0 + i, &x);
                    c.tail.clear();
                    c.mods.truncate(1);
                    let name = assemble(&c, d);
                    if name.len() <= budget {
                        return name;
                    }
                }
            }
        }
    }
    // Nothing fit: keep wear and foil in the plainest shape that can hold them.
    let noun = slots(s, attempt).noun;
    let bare = |wear: Option<&str>, foil: Option<&str>| -> String {
        let mut w: Vec<&str> = Vec::new();
        w.extend(wear);
        w.extend(foil);
        w.push(noun);
        w.join(" ")
    };
    [bare(wear, foil), bare(wear, None)]
        .into_iter()
        .find(|n| n.len() <= budget)
        .unwrap_or_else(|| noun.to_string())
}

fn roman(mut n: u32) -> String {
    let table = [(10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    let mut out = String::new();
    for (v, r) in table {
        while n >= v {
            out.push_str(r);
            n -= v;
        }
    }
    out
}

/// UPPERCASE relic name, <= `MAX_NAME`, not in `taken`.
pub fn name(s: &RelicSpec) -> String {
    for attempt in 0..RETRIES {
        let n = generate(s, attempt, MAX_NAME);
        if !s.taken.contains(&n) {
            return n;
        }
    }
    let mut i = 2;
    loop {
        let numeral = roman(i);
        let base = generate(s, 0, MAX_NAME - 1 - numeral.len());
        let n = format!("{base} {numeral}");
        if !s.taken.contains(&n) {
            return n;
        }
        i += 1;
    }
}

// --------------------------------------------------------------- flavour

/// What the relic does when held, by shape. Lowercase, whisper-sized.
#[rustfmt::skip]
fn shape_lines(shape: Shape) -> &'static [&'static str] {
    match shape {
        Shape::Cube => &[
            "it is heavier than it looks", "something rattles inside", "every corner is sharp",
            "it always lands the same way up", "it sits where you put it",
        ],
        Shape::Octahedron => &[
            "it splits the light into two", "it points both ways at once",
            "it hums when held", "it is cold on one side only",
        ],
        Shape::Orb => &[
            "it rolls back to your hand", "it is watching a little", "it fogs when you sigh",
            "it is warm and slightly damp", "you can see a room inside",
        ],
        Shape::Cylinder => &[
            "it rolls toward the door", "it is hollow, and not empty",
            "it fits your grip exactly", "it ticks when nobody speaks",
        ],
        Shape::Cone => &[
            "it points at whatever you fear", "it leans toward the nearest exit",
            "the tip is never where you left it", "it hums when held",
        ],
        Shape::Torus => &[
            "the hole is deeper than the ring", "it spins if you stop looking",
            "something is missing from the middle", "it fits a larger hand than yours",
        ],
        Shape::Arch => &[
            "it frames whatever stands behind it", "it opens inward, sometimes",
            "it stays standing without you", "walk through, if you must",
        ],
        Shape::Cloud => &[
            "it weighs nothing and drags", "it follows a step behind",
            "it smells like rain that hasn't come", "it softens whatever it touches",
        ],
        Shape::Crescent => &[
            "it smiles when it cuts", "it keeps the shape of a moon",
            "it hums when held", "it is thinner every night",
        ],
        Shape::Island => &[
            "it floats a thumb above the table", "it remembers being a mountain",
            "it is heavy only at the tip", "it drifts when you sleep",
        ],
        Shape::Stairs => &[
            "it always has one step more", "it goes up and nowhere",
            "every step is a different height", "it creaks in your hand",
        ],
    }
}

#[rustfmt::skip]
fn rarity_lines(r: RelicRarity) -> &'static [&'static str] {
    match r {
        RelicRarity::Cracked => &[
            "held together by habit", "somebody else's, once", "it still mostly works",
            "found at the bottom of a pocket", "better than nothing",
        ],
        RelicRarity::Worn => &[
            "worn smooth by someone", "it knows your grip", "nicely broken in",
            "it has been carried far", "familiar as an old coat",
        ],
        RelicRarity::Pristine => &[
            "never once dropped", "kept in the good drawer", "it still smells new",
            "it has been waiting for you", "not a mark on it",
        ],
        RelicRarity::Mythic => &[
            "it chose you, not the other way", "it was here before the dream",
            "it remembers every owner", "nobody agrees where it came from",
            "the dream bends toward it",
        ],
    }
}

fn wear_line(w: WearTier) -> Option<&'static str> {
    match w {
        WearTier::Ruined => Some("barely holding itself together"),
        WearTier::Heavy => Some("the edges have gone soft"),
        WearTier::Light | WearTier::Clean => None,
    }
}

const FOIL_LINE: &str = "it throws small rainbows";

fn flavour_once(s: &RelicSpec, attempt: u32, k: usize) -> String {
    let own = vocab(s.theme);
    let sp = pick(own.subjects, h(s.seed, 21, attempt));
    let pr = pick(own.predicates, h(s.seed, 22, attempt));
    let shape = pick(shape_lines(s.shape), h(s.seed, 23, attempt));
    let rar = pick(rarity_lines(s.rarity), h(s.seed, 24, attempt));
    let whisper = pick(own.whispers, h(s.seed, 25, attempt));
    let coda = pick(crate::dream::vocab::CODAS, h(s.seed, 26, attempt));
    // Rarity weights the mix: Cracked/Worn stay plain, Pristine/Mythic go grand.
    let extra = if s.foil {
        FOIL_LINE
    } else {
        wear_line(s.wear).unwrap_or(rar)
    };
    match (s.rarity, k % 6) {
        (_, 0) => format!("{sp} {pr}"),
        (_, 1) => shape.to_string(),
        (RelicRarity::Cracked | RelicRarity::Worn, 2) => extra.to_string(),
        (RelicRarity::Pristine | RelicRarity::Mythic, 2) => format!("{rar}. {coda}"),
        (_, 3) => format!("{shape}. {coda}"),
        (RelicRarity::Mythic, 4) => whisper.to_string(),
        (_, 4) => rar.to_string(),
        (_, _) => extra.to_string(),
    }
}

/// Lowercase whisper, <= `MAX_FLAVOUR`.
pub fn flavour(s: &RelicSpec) -> String {
    let k0 = h(s.seed, 20, 0) as usize;
    for attempt in 0..RETRIES {
        for i in 0..6 {
            let f = flavour_once(s, attempt, k0 + i);
            if f.len() <= MAX_FLAVOUR {
                return f;
            }
        }
    }
    shape_lines(s.shape)[0].to_string()
}

// ------------------------------------------------------------------ tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::ALL_SHAPES;
    use std::collections::HashSet;

    const RARITIES: [RelicRarity; 4] = [
        RelicRarity::Cracked,
        RelicRarity::Worn,
        RelicRarity::Pristine,
        RelicRarity::Mythic,
    ];

    fn effect_for(seed: u64) -> RelicEffect {
        let mut e = RelicEffect::default();
        match mix(seed, 77) % 9 {
            0 => e.dust_bonus_pct = 10,
            1 => e.memory_bonus = 0.05,
            2 => e.speed_pct = 5,
            3 => e.grace_bonus = 1,
            4 => e.calm_pct = 10,
            5 => e.free_rerolls = 1,
            6 => e.start_shards = 1,
            7 => e.max_nerve = 10,
            _ => {}
        }
        e
    }

    fn wear_for(seed: u64) -> WearTier {
        match mix(seed, 78) % 4 {
            0 => WearTier::Clean,
            1 => WearTier::Light,
            2 => WearTier::Heavy,
            _ => WearTier::Ruined,
        }
    }

    fn spec(theme: DreamTheme, rarity: RelicRarity, seed: u64, taken: &[String]) -> RelicSpec<'_> {
        RelicSpec {
            theme,
            seed,
            rarity,
            wear: wear_for(seed),
            foil: mix(seed, 79) % 5 == 0,
            shape: ALL_SHAPES[(mix(seed, 80) % ALL_SHAPES.len() as u64) as usize],
            effect: effect_for(seed),
            taken,
        }
    }

    fn glyph_ok(c: char) -> bool {
        c.is_ascii_uppercase() || c == ' ' || c == '\'' || c == '-'
    }

    #[test]
    fn deterministic_per_seed() {
        for seed in 0..300u64 {
            let s = spec(ALL_THEMES[(seed % 20) as usize], RARITIES[(seed % 4) as usize], seed, &[]);
            assert_eq!(name(&s), name(&s));
            assert_eq!(flavour(&s), flavour(&s));
        }
    }

    #[test]
    fn every_shape_has_enough_head_nouns() {
        assert_eq!(ALL_SHAPES.len(), 11);
        for sh in ALL_SHAPES {
            let n = head_nouns(sh);
            assert!((8..=12).contains(&n.len()), "{sh:?} has {}", n.len());
            let uniq: HashSet<_> = n.iter().collect();
            assert_eq!(uniq.len(), n.len(), "{sh:?} duplicates");
        }
    }

    #[test]
    fn shape_word_always_in_name_and_length_capped() {
        for theme in ALL_THEMES {
            for rarity in RARITIES {
                for seed in 0..1000u64 {
                    let s = spec(theme, rarity, seed * 7919 + 3, &[]);
                    let n = name(&s);
                    assert!(n.len() <= MAX_NAME, "{n} ({} chars)", n.len());
                    assert!(
                        head_nouns(s.shape).iter().any(|w| n.contains(w)),
                        "{n} lacks a {:?} word",
                        s.shape
                    );
                    let f = flavour(&s);
                    assert!(!f.is_empty() && f.len() <= MAX_FLAVOUR, "{f}");
                }
            }
        }
    }

    #[test]
    fn length_cap_over_20k_seeds_all_themes_and_rarities() {
        for theme in ALL_THEMES {
            for rarity in RARITIES {
                for seed in 0..20_000u64 {
                    let n = name(&spec(theme, rarity, seed, &[]));
                    assert!(n.len() <= MAX_NAME && !n.is_empty(), "{theme:?} {n}");
                }
            }
        }
    }

    #[test]
    fn unique_rate_is_high_per_theme_and_rarity() {
        for theme in ALL_THEMES {
            for rarity in RARITIES {
                let set: HashSet<String> = (0..5000u64)
                    .map(|seed| name(&spec(theme, rarity, seed, &[])))
                    .collect();
                let rate = set.len() as f64 / 5000.0;
                assert!(rate >= 0.97, "{theme:?} {rarity:?}: {rate:.4}");
            }
        }
    }

    #[test]
    fn taken_names_never_collide() {
        for theme in [DreamTheme::Lobby, DreamTheme::TheTunnel, DreamTheme::DrownedLibrary] {
            for rarity in RARITIES {
                // Same seed over and over: the worst case for collisions.
                let mut taken: Vec<String> = Vec::new();
                for _ in 0..40 {
                    let s = spec(theme, rarity, 42, &taken);
                    let n = name(&s);
                    assert!(!taken.contains(&n), "{n}");
                    assert!(n.len() <= MAX_NAME);
                    assert!(head_nouns(s.shape).iter().any(|w| n.contains(w)));
                    taken.push(n);
                }
            }
        }
    }

    #[test]
    fn numeral_suffix_when_retries_run_out() {
        let s = spec(DreamTheme::Lobby, RelicRarity::Worn, 5, &[]);
        let first = name(&s);
        let taken: Vec<String> = (0..RETRIES).map(|a| generate(&s, a, MAX_NAME)).collect();
        let s2 = RelicSpec { taken: &taken, ..s };
        let n = name(&s2);
        assert!(!taken.contains(&n));
        assert!(n.ends_with(" II") || n != first, "{n}");
    }

    #[test]
    fn every_lever_has_a_list_and_it_is_chosen() {
        let mut levers = Vec::new();
        for i in 0..8 {
            let mut e = RelicEffect::default();
            match i {
                0 => e.dust_bonus_pct = 10,
                1 => e.memory_bonus = 0.05,
                2 => e.speed_pct = 5,
                3 => e.grace_bonus = 1,
                4 => e.calm_pct = 10,
                5 => e.free_rerolls = 1,
                6 => e.start_shards = 1,
                _ => e.max_nerve = 10,
            }
            let l = effect_hint(&e).expect("lever has a list");
            assert!(l.len() >= 8);
            assert!(l.iter().any(|w| w.starts_with("OF ")));
            assert!(l.iter().any(|w| !w.starts_with("OF ")));
            levers.push(l[0]);
        }
        levers.sort();
        levers.dedup();
        assert_eq!(levers.len(), 8, "each lever has its own list");
        assert!(effect_hint(&RelicEffect::default()).is_none());
        // The strongest wins: 3 rerolls (3.0) beats 10% dust (1.0).
        let e = RelicEffect { dust_bonus_pct: 10, free_rerolls: 3, ..Default::default() };
        assert_eq!(effect_hint(&e).unwrap()[0], REROLLS[0]);
    }

    #[test]
    fn hint_word_shows_up_for_a_hinted_effect() {
        let e = RelicEffect { calm_pct: 10, ..Default::default() };
        let hits = (0..400u64)
            .filter(|&seed| {
                let s = RelicSpec { effect: e, ..spec(DreamTheme::Lobby, RelicRarity::Worn, seed, &[]) };
                let n = name(&s);
                CALM.iter().any(|w| n.contains(w))
            })
            .count();
        assert!(hits > 200, "{hits}/400");
    }

    #[test]
    fn wear_prefix_present_for_ruined_and_heavy() {
        for theme in ALL_THEMES {
            for rarity in RARITIES {
                for seed in 0..200u64 {
                    let base = spec(theme, rarity, seed, &[]);
                    let r = RelicSpec { wear: WearTier::Ruined, ..base };
                    let n = name(&r);
                    assert!(n.contains("RUINED"), "{n}");
                    let base = spec(theme, rarity, seed, &[]);
                    let hv = RelicSpec { wear: WearTier::Heavy, ..base };
                    let n = name(&hv);
                    assert!(n.contains("HEAVILY-WORN"), "{n}");
                    let base = spec(theme, rarity, seed, &[]);
                    let c = RelicSpec { wear: WearTier::Clean, ..base };
                    let n = name(&c);
                    assert!(!n.contains("RUINED") && !n.contains("HEAVILY-WORN"), "{n}");
                }
            }
        }
    }

    #[test]
    fn foil_adds_a_sheen_word() {
        for seed in 0..300u64 {
            let base = spec(DreamTheme::Garden, RelicRarity::Pristine, seed, &[]);
            let s = RelicSpec { foil: true, wear: WearTier::Clean, ..base };
            let n = name(&s);
            assert!(FOIL_WORDS.iter().any(|w| n.contains(w)), "{n}");
        }
    }

    #[test]
    fn rarity_families_differ() {
        let cr = |t: DreamTheme, seed| {
            name(&RelicSpec { wear: WearTier::Clean, foil: false, effect: RelicEffect::default(), ..spec(t, RelicRarity::Cracked, seed, &[]) })
        };
        let my = |t: DreamTheme, seed| {
            name(&RelicSpec { wear: WearTier::Clean, foil: false, effect: RelicEffect::default(), ..spec(t, RelicRarity::Mythic, seed, &[]) })
        };
        let mut cracked_the = 0;
        let mut mythic_grand = 0;
        for seed in 0..500u64 {
            if cr(DreamTheme::Lobby, seed).starts_with("THE ") {
                cracked_the += 1;
            }
            let m = my(DreamTheme::Lobby, seed);
            if m.contains(" THAT ") || m.contains("'S ") || m.contains("' ") || m.contains(" BEFORE ") || m.contains(" OF ") {
                mythic_grand += 1;
            }
        }
        assert_eq!(cracked_the, 0, "Cracked never opens with THE");
        assert!(mythic_grand > 450, "{mythic_grand}/500 Mythic names are grand");
    }

    #[test]
    fn charset_is_pixel_font_safe() {
        for theme in ALL_THEMES {
            for rarity in RARITIES {
                for seed in 0..1500u64 {
                    let s = spec(theme, rarity, seed, &[]);
                    let n = name(&s);
                    assert!(n.chars().all(glyph_ok), "{n}");
                    assert!(!n.starts_with(' ') && !n.ends_with(' ') && !n.contains("  "), "{n:?}");
                    let f = flavour(&s);
                    assert!(
                        f.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || " '-.,?!:".contains(c)),
                        "{f}"
                    );
                }
            }
        }
        for sh in ALL_SHAPES {
            assert!(head_nouns(sh).iter().all(|w| w.chars().all(glyph_ok)));
        }
        let lists: [&[&str]; 9] = [CRACKED_ADJ, FOIL_WORDS, DUST, SPEED, CALM, MEMORY, REROLLS, SHARDS, GRACE];
        for l in lists.iter().copied().chain([NERVE]) {
            assert!(l.iter().all(|w| w.chars().all(glyph_ok)), "{l:?}");
        }
    }

    #[test]
    fn word_lists_are_non_empty() {
        for sh in ALL_SHAPES {
            assert!(!head_nouns(sh).is_empty());
            assert!(!shape_lines(sh).is_empty());
        }
        for r in RARITIES {
            assert!(!rarity_lines(r).is_empty());
        }
    }

    /// Prints the samples for NOTES-names.md: `cargo test print_samples -- --nocapture --ignored`.
    #[test]
    #[ignore]
    fn print_samples() {
        for rarity in RARITIES {
            println!("## {rarity:?}");
            for i in 0..40u64 {
                let theme = ALL_THEMES[((i * 7) % 20) as usize];
                let s = spec(theme, rarity, 1000 + i * 977, &[]);
                println!("SAMPLE|{rarity:?}|{:?}|{:?}|{:?}|{}|{}", s.theme, s.shape, s.wear, name(&s), flavour(&s));
            }
        }
    }
}
