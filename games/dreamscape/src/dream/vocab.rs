//! The dream word bank. Name slots are UPPERCASE, whisper slots lowercase.
//! Keep nouns unique per theme (tests enforce it) so a borrowed word is
//! recognisably from somewhere else.

use super::theme::DreamTheme;

pub struct Vocab {
    pub adjectives: &'static [&'static str],
    pub nouns: &'static [&'static str],
    /// Completes "THE {NOUN} THAT ...".
    pub relatives: &'static [&'static str],
    /// Completes "THE {NOUN} OF ..." / "BEFORE ...".
    pub abstracts: &'static [&'static str],
    /// Whisper subject: "the carpet".
    pub subjects: &'static [&'static str],
    /// Whisper predicate: "is breathing again".
    pub predicates: &'static [&'static str],
    /// Hand-written lines that don't fit the grammar.
    pub whispers: &'static [&'static str],
}

/// Shared by every dream.
#[rustfmt::skip]
pub const ABSTRACTS: &[&str] = &[
    "SLEEP", "FORGETTING", "NOON", "MIDNIGHT", "YOUR MOTHER", "SMALL HOURS", "STATIC",
    "SOFT TEETH", "SECOND THOUGHTS", "LOW TIDE", "UNSENT LETTERS", "THE HUM", "MOTHS",
    "PAPER", "THE OTHER YOU", "REHEARSAL", "NO RETURN", "WARM MILK", "FEVER", "ECHOES",
    "RUST", "DUST", "THE LONG HALL", "YESTERDAY", "TOMORROW", "NOTHING IN PARTICULAR",
    "THE DEEP END", "LOST TIME", "BORROWED LIGHT", "HALF MEMORIES", "RAINY SUNDAYS",
    "OLD PHOTOGRAPHS", "THE UNDERTOW", "WAITING ROOMS", "CHILDHOOD", "PALE BLUE",
    "SOMEBODY ELSE", "THE WRONG DOOR", "GLASS", "SALT", "TELEVISION SNOW", "LULLABIES",
    "THE THIRD FLOOR", "LOST KEYS", "ROOM TEMPERATURE", "HUMMING", "STILL WATER",
    "FLICKERING", "BACKWARDS CLOCKS", "SLOW MOTION", "PINS AND NEEDLES", "FOG",
    "THE LAST BUS", "WET PAINT", "CARDBOARD", "DEJA VU", "SHORT BREATHS", "HOLLOW BONES",
    "TWILIGHT", "FALSE AWAKENINGS", "SLEEPWALKING", "THE BLUE HOUR", "LEFTOVERS",
    "NIGHT LIGHTS", "UNFINISHED SENTENCES", "PHANTOM LIMBS",
];

/// Tacked onto some whispers after a full stop.
#[rustfmt::skip]
pub const CODAS: &[&str] = &[
    "don't turn around", "keep walking", "it's fine", "you knew that", "breathe",
    "count to ten", "don't wake it", "it remembers", "nobody minds", "not yet",
    "almost", "again", "you agreed to this", "it's still tuesday", "shh",
    "look up", "don't look up", "it was always like this", "you're late",
    "no one saw", "that's normal here", "you can stay", "please stay",
    "keep going", "it's almost over", "don't answer it", "it's been like this for hours",
    "hurry", "slower", "you were warned", "it's only paint", "it's okay to cry here",
    "no one is coming", "someone is coming", "you'll forget this", "remember this",
    "it's your turn", "it heard that", "it's getting closer", "stay close",
    "don't say it out loud", "it knows", "that was yours", "you'll be fine",
    "listen", "it smells like home", "wake up", "don't wake up",
];

#[rustfmt::skip]
pub fn vocab(theme: DreamTheme) -> Vocab {
    use DreamTheme::*;
    match theme {
        Lobby => Vocab {
            adjectives: &[
                "WAITING", "VELVET", "PATIENT", "HUSHED", "PASTEL", "CARPETED", "MIRRORED",
                "GILDED", "PERFUMED", "POLITE", "MUFFLED", "CANDLELIT", "PLUSH", "LACQUERED",
                "SUNKEN", "FADED", "BRASS", "ROSE-TINTED", "OVERBOOKED", "UNATTENDED",
                "ECHOING", "HALF-LIT", "SATIN", "REVOLVING", "COURTEOUS", "DOZING",
                "FORMAL", "WALLPAPERED", "CHANDELIERED", "DUSTED", "WELCOMING", "UNHURRIED",
                "STILL", "MARBLED", "POWDERED", "SOFTLY LIT", "VACANT", "OPULENT",
            ],
            nouns: &[
                "LOBBY", "ANTEROOM", "FOYER", "RECEPTION", "CORRIDOR", "WAITING ROOM",
                "CLOAKROOM", "CONCIERGE", "ELEVATOR", "VESTIBULE", "PARLOUR", "BALLROOM",
                "ATRIUM", "HOTEL", "STAIRCASE", "LOUNGE", "FRONT DESK", "GUESTBOOK",
                "BELLHOP", "CHANDELIER", "REVOLVING DOOR", "TICKET", "ROOM KEY",
                "LOST AND FOUND", "PORTER", "HALLWAY", "SUITE", "MEZZANINE", "DOORMAN",
                "COAT CHECK", "SERVICE BELL", "POTTED PALM", "CARPET", "DRAPES",
            ],
            relatives: &[
                "KEPT YOUR COAT", "EXPECTED YOU", "NEVER CLOSES", "CALLS YOUR NAME",
                "IS FULLY BOOKED", "SMELLS LIKE RAIN", "HAS NO EXIT", "KNOWS YOUR ROOM",
                "RINGS AT NIGHT", "WAITS FOR NO ONE", "FORGOT TO CHECK OUT", "IS ALWAYS NEW",
                "HOLDS ITS BREATH", "SEES YOU OFF", "SLEEPS STANDING UP",
                "HAS ONE MORE FLOOR", "DOESN'T TAKE CASH", "PLAYS THE SAME SONG",
                "KEEPS ITS LIGHTS ON", "IS NEVER EMPTY",
            ],
            abstracts: &[
                "ARRIVALS", "LATE CHECKOUT", "SMALL TALK", "LOST LUGGAGE", "NEW GUESTS",
                "VELVET ROPES", "ELEVATOR MUSIC", "DO NOT DISTURB",
                "ROOM SERVICE", "WAKE-UP CALLS", "LAST ORDERS", "THE HONEYMOON SUITE", "POLITE SMILES", "BRASS KEYS", "LOBBY PIANO", "CHECK-IN",
            ],
            subjects: &[
                "the bell", "the concierge", "the elevator", "your room key", "the guestbook",
                "the carpet", "the chandelier", "the doorman", "the clock above the desk",
                "a porter", "the potted palm", "the lift music", "your coat", "the front desk",
                "a guest who looks like you", "the revolving door", "the wallpaper",
                "your reservation", "the ice machine", "the velvet curtain", "the lobby piano",
                "the phone behind the counter", "a velvet rope", "the mirror",
            ],
            predicates: &[
                "has been ringing for years", "knows you by name", "is holding your room",
                "stopped at a floor that isn't there", "is smiling too long",
                "has your handwriting in it", "is waiting for someone else",
                "remembers your last stay", "only goes down", "is breathing, politely",
                "was polished this morning", "hums the same four notes",
                "says you already checked in", "is warm to the touch",
                "keeps rearranging itself", "won't make eye contact",
                "has been expecting you", "asks how long you'll be staying",
                "is slightly out of tune", "has no number on it",
                "never closes all the way", "is folded like a note",
                "turns to watch you pass",
            ],
            whispers: &[
                "someone is expected", "the bell never rings", "take a number. any number.",
                "you have been here before", "please wait to be seated",
                "your party is running late", "the lift is on its way down",
                "checkout was yesterday", "a room has been prepared",
                "please sign the guestbook", "mind the gap between floors",
                "we kept your key",
            ],
        },
        LiminalOffice => Vocab {
            adjectives: &[
                "FLUORESCENT", "ENDLESS", "BEIGE", "HUMMING", "EMPTY", "FILED", "LAMINATED",
                "OVERTIME", "STAPLED", "PHOTOCOPIED", "REDACTED", "BACKLOGGED", "QUARTERLY",
                "OPEN-PLAN", "DRAB", "FLICKERING", "MANDATORY", "PENDING", "RECYCLED",
                "OUT-OF-OFFICE", "CARBON", "TEMPORARY", "LOOPING", "UNSIGNED", "ARCHIVED",
                "AFTER-HOURS", "BUZZING", "CUBICLED", "PASSWORD-LOCKED", "TONER-STAINED",
                "UNPAID", "RE-ORGANISED", "CORPORATE", "WINDOWLESS", "CARPET-TILED",
            ],
            nouns: &[
                "OFFICE", "CUBICLES", "BREAKROOM", "ANNEX", "FLOOR ZERO", "COPY ROOM",
                "BOARDROOM", "HELP DESK", "SUPPLY CLOSET", "WATER COOLER", "SWIVEL CHAIR",
                "INBOX", "SPREADSHEET", "FILING CABINET", "PHOTOCOPIER", "TIMESHEET",
                "MEMO", "STAPLER", "CONFERENCE CALL", "HR DEPARTMENT", "MAILROOM",
                "PARKING GARAGE", "ORG CHART", "DESK PLANT", "LANYARD", "FAX MACHINE",
                "PRINTER QUEUE", "MEETING", "KEYCARD", "OUT TRAY", "WHITEBOARD",
                "CALENDAR INVITE", "SERVER ROOM", "BREAK TIMER",
            ],
            relatives: &[
                "NEVER CLOCKS OUT", "PRINTED YOU", "IS PAST DUE", "MISSED THE MEETING",
                "NEEDS YOUR SIGNATURE", "WAS REASSIGNED", "REPLIED ALL", "KEEPS BUZZING",
                "IS ON MUTE", "FORGOT YOUR NAME", "IS STILL LOADING", "WORKS WEEKENDS",
                "WAS NEVER FILED", "CAN'T BE CANCELLED", "IS JUST CIRCLING BACK",
                "LEFT A VOICEMAIL", "IS OUT OF TONER", "RECORDS EVERYTHING",
                "PROMOTED ITSELF", "HAS NO WINDOWS",
            ],
            abstracts: &[
                "OVERTIME", "PERFORMANCE REVIEWS", "PAPER JAMS", "MONDAY", "FINAL NOTICES",
                "SYNERGY", "UNPAID LEAVE", "ONBOARDING", "THE DEADLINE",
                "THE ALL-HANDS", "COFFEE BREAKS", "FRIDAY AFTERNOON", "REDUNDANCY", "TONER", "OUT OF OFFICE", "MIDDLE MANAGEMENT", "OVERHEAD LIGHTS",
            ],
            subjects: &[
                "the printer", "the water cooler", "your desk", "the fluorescent light",
                "the photocopier", "your manager", "the break timer", "the carpet",
                "the fax machine", "the elevator to floor zero", "your keycard",
                "an unread email", "the stapler", "the swivel chair", "the vending machine",
                "the hum", "the wall clock", "the conference phone", "your nametag",
                "the ceiling tile", "someone's lunch", "the org chart", "the desk plant",
            ],
            predicates: &[
                "is warm. no one printed", "is still here", "says it's 3:14 pm",
                "knows your name", "is copying itself", "has scheduled you again",
                "hasn't been refilled since you started", "is buzzing in c minor",
                "wants to circle back", "has been reassigned to you",
                "is waiting in your inbox", "is marked urgent", "has a new policy",
                "rolls back to your desk", "blinks when you look away",
                "counts every minute you're gone", "has been out of order forever",
                "still has your badge photo", "is one floor lower than yesterday",
                "is asking for a quick sync", "was never plugged in", "is growing",
                "has your handwriting",
            ],
            whispers: &[
                "the printer is warm. no one printed.", "your desk is still here",
                "it is always 3:14 pm", "the hum knows your name",
                "please remember to clock out", "the meeting has been moved to now",
                "per my last email", "you are on mute", "this could have been an email",
                "someone ate your lunch", "floor zero is not on the map",
                "your badge has expired",
            ],
        },
        VoidPlatforms => Vocab {
            adjectives: &[
                "WEIGHTLESS", "SHATTERED", "STARLESS", "DRIFTING", "HOLLOW", "SUSPENDED",
                "BOTTOMLESS", "SILENT", "ORBITING", "DISSOLVING", "TILTED", "UNMOORED",
                "INVERTED", "SPLINTERED", "GLASS", "FLOATING", "COLD", "UNLIT", "UNFINISHED",
                "VERTIGINOUS", "SCATTERED", "BROKEN", "NULL", "FROZEN", "ECLIPSED",
                "SUNDERED", "WANDERING", "FALLING", "BLACK", "INDIGO", "PHANTOM",
                "UNANCHORED", "BRITTLE", "LAST", "OUTERMOST",
            ],
            nouns: &[
                "ARCHIPELAGO", "STAIRWELL", "NOTHING", "CONSTELLATION", "SHORELINE",
                "ABYSS", "LEDGE", "STEPPING STONES", "VOID", "HORIZON", "RIFT", "CHASM",
                "ISLAND", "TETHER", "BRIDGE", "ORBIT", "PIER", "SKY", "DRIFT", "FRAGMENT",
                "OVERHANG", "CATWALK", "GANTRY", "FAULT LINE", "PRECIPICE", "SPIRE",
                "MOON", "COMET", "NEBULA", "EVENT HORIZON", "WAYPOINT", "LADDER",
                "DARK", "FAR SIDE",
            ],
            relatives: &[
                "HAS NO BOTTOM", "FELL UPWARD", "FORGOT GRAVITY", "IS FALLING SLOWLY",
                "HOLDS YOUR WEIGHT", "WON'T HOLD YOUR WEIGHT", "DRIFTED AWAY",
                "GOES NOWHERE", "WAITS BELOW", "COUNTS YOUR STEPS", "ISN'T THERE",
                "WAS HERE A MOMENT AGO", "LEADS DOWN", "SWALLOWED THE STARS",
                "ONLY GOES ONE WAY", "IS TOO FAR TO JUMP", "HUMS IN THE DARK",
                "MISSED THE LANDING", "ENDS MIDAIR", "IS MADE OF QUIET",
            ],
            abstracts: &[
                "FALLING", "VERTIGO", "ZERO", "THE DEEP", "FREEFALL", "LAST STEPS",
                "GRAVITY", "EMPTY SPACE",
                "THE DROP", "STARLIGHT", "WEIGHTLESSNESS", "BOTTOMLESS NIGHT", "THIN AIR", "THE OUTER DARK", "COLD STARS", "EDGES",
            ],
            subjects: &[
                "the floor", "the next platform", "something below", "the gap",
                "the dark", "your shadow", "the ledge", "the edge", "a falling star",
                "the far side", "the bridge", "gravity", "the staircase", "the horizon",
                "the silence", "the last step", "your footprint", "a light in the distance",
                "the void", "the tether", "the wind that isn't there", "your echo",
            ],
            predicates: &[
                "is further than it looks", "is falling with you", "has no down",
                "is holding its breath", "was never finished", "is drifting apart",
                "forgot which way is up", "is listening for your landing",
                "goes on without you", "is waiting to see if you jump",
                "is only there when you look", "is colder than yesterday",
                "rearranged itself while you blinked", "is closer than it was",
                "is the only thing left", "knows you'll make it",
                "doesn't know you'll make it", "has been falling for years",
                "is made of held breath", "catches you, usually",
                "is lit from underneath", "counts down", "wants you to look down",
            ],
            whispers: &[
                "don't look down. there is no down.", "the gaps are wider than they look",
                "something is falling with you", "step where the light is",
                "jump. we'll see.", "gravity is optional here", "the edge is closer now",
                "everything floats eventually", "you left the ground a while ago",
                "hold still. it's drifting.", "the stars went out one by one",
                "the bottom is a rumour",
            ],
        },
        Garden => Vocab {
            adjectives: &[
                "OVERGROWN", "BREATHING", "CANDIED", "WHISPERING", "SUNLESS", "BLOOMING",
                "ROTTING", "HONEYED", "THORNED", "MOSSY", "DEWY", "WILTED", "RIPE",
                "FERAL", "SWEETENED", "PETALLED", "TANGLED", "HUMID", "POLLINATED", "TWILIT",
                "SPROUTING", "HUNGRY", "GREENING", "VINE-WRAPPED", "SUGARED", "NECTARED",
                "MIDSUMMER", "WEEPING", "OVERRIPE", "FRAGRANT", "BUZZING", "VELVETEEN",
                "FLOWERING", "SECRET", "SLEEPY",
            ],
            nouns: &[
                "GARDEN", "ORCHARD", "HEDGEROW", "GREENHOUSE", "MEADOW", "HEDGE MAZE",
                "ROSEBED", "ARBOUR", "TRELLIS", "BEEHIVE", "POND", "BIRDBATH", "GAZEBO",
                "SUNDIAL", "TOPIARY", "WILLOW", "THICKET", "BRAMBLE", "FERN", "LILY PAD",
                "FOXGLOVE", "HONEYSUCKLE", "WISTERIA", "COMPOST HEAP", "SCARECROW",
                "FOUNTAIN", "GRAPEVINE", "CLOVER PATCH", "POPPY FIELD", "GROTTO",
                "WEEPING TREE", "SEED PACKET", "WHEELBARROW", "ROOT CELLAR",
            ],
            relatives: &[
                "TURNS TO WATCH YOU", "SMELLS LIKE A BIRTHDAY", "MOVED AGAIN",
                "IS STILL GROWING", "ATE THE FENCE", "BLOOMS AT NIGHT", "HUMS WITH BEES",
                "REMEMBERS SUMMER", "KNOWS YOUR SHOES", "HAS TOO MANY PETALS",
                "GREW BACK", "IS PICKING YOU", "NEVER WILTS", "DRINKS THE RAIN",
                "LEANS TOWARD YOU", "FLOWERED OVERNIGHT", "HIDES THE PATH",
                "TASTES LIKE CANDY", "WAS PLANTED FOR YOU", "SINGS AT DUSK",
            ],
            abstracts: &[
                "HONEY", "LATE SUMMER", "POLLEN", "SEEDS", "SPRING", "SWEET ROT",
                "FIRST FROST", "OVERGROWTH",
                "PETALS", "BEESWAX", "RIPE FRUIT", "THE COMPOST", "WILD MINT", "RAIN ON LEAVES", "TALL GRASS", "NECTAR",
            ],
            subjects: &[
                "the tallest flower", "the hedge", "a bee", "the rosebush", "the pond", "the sundial",
                "the scarecrow", "the orchard", "the ivy", "the moss", "the gazebo",
                "the birdbath", "the fountain", "a poppy", "the willow", "the soil",
                "the topiary swan", "a snail", "the greenhouse glass", "the honey",
                "the path", "the bramble", "the weeping tree",
            ],
            predicates: &[
                "turns to watch you", "smells like a birthday", "moved again",
                "is breathing, slowly", "wants to be picked", "hums when you pass",
                "is growing in your footprints", "is sweeter than it should be",
                "has too many petals", "knows the way out", "doesn't know the way out",
                "is whispering your middle name", "was here before the house",
                "bloomed while you blinked", "is full of tiny teeth", "is heavy with dew",
                "is leaning closer", "tells the time wrong", "has been waiting all summer",
                "drips honey", "keeps the rain", "is dreaming of you too",
                "has grown over the gate",
            ],
            whispers: &[
                "the flowers turn to watch you", "it smells like a birthday",
                "the hedges moved again", "pick nothing", "stay on the path",
                "don't eat the fruit", "the bees know", "it's always almost dusk",
                "water me", "the gate was here yesterday", "something is blooming behind you",
                "the soil is warm",
            ],
        },
        NightmareFactory => Vocab {
            adjectives: &[
                "RUSTED", "GRINDING", "SLEEPLESS", "FEVERED", "HUNGRY", "SCALDING",
                "SMOKING", "CLANKING", "OILY", "RIVETED", "SIRENED", "MOLTEN", "SOOTED",
                "PISTONED", "HOWLING", "SHUDDERING", "AUTOMATED", "OVERCLOCKED", "CORRODED",
                "SPARKING", "GREASED", "SHRIEKING", "IRON", "BURNING", "THROBBING",
                "RELENTLESS", "HEAVY", "STEAMING", "WELDED", "BLISTERING", "THUNDERING",
                "TIRELESS", "RATTLING", "SEALED", "HAZARDOUS",
            ],
            nouns: &[
                "FACTORY", "FOUNDRY", "ASSEMBLY LINE", "BOILER ROOM", "MILL", "FURNACE",
                "CONVEYOR", "SMOKESTACK", "PRESS", "KILN", "SMELTER", "PISTON", "GEARBOX",
                "WORKSHOP", "LOADING DOCK", "CATWALK GRATE", "CRANE", "VAT", "CRUCIBLE",
                "TURBINE", "ENGINE ROOM", "SHIFT WHISTLE", "PUNCH CLOCK", "FORGE",
                "SLAUGHTERHOUSE", "REFINERY", "CHIMNEY", "STEAM PIPE", "MACHINE",
                "QUOTA BOARD", "ANVIL", "HOPPER", "CONDENSER", "SPROCKET",
            ],
            relatives: &[
                "IS MAKING YOU", "COUNTS HEARTBEATS", "NEVER SLEEPS", "IS AHEAD OF SCHEDULE",
                "WANTS MORE", "HAS YOUR SHIFT", "RUNS ON NIGHTMARES", "SWALLOWS THE LIGHT",
                "KNOWS YOUR SIZE", "IS STILL WARM", "HISSES YOUR NAME", "NEVER STOPS",
                "STAMPS YOU OUT", "IS OVER QUOTA", "SCREAMS AT NOON", "FEEDS ITSELF",
                "BUILT ITSELF", "HAS NO OFF SWITCH", "IS HUNGRY AGAIN", "WATCHES THE LINE",
            ],
            abstracts: &[
                "PRODUCTION", "THE QUOTA", "THE NIGHT SHIFT", "SPARE PARTS", "SMOKE",
                "IRON", "OVERTIME PAY", "THE WHISTLE",
                "SCRAP METAL", "BURNT OIL", "THE ASSEMBLY", "SPARKS", "THE FURNACE", "EXHAUST", "COGS", "HEAVY INDUSTRY",
            ],
            subjects: &[
                "the machine", "the conveyor", "the furnace", "the shift whistle",
                "the foreman", "the press", "the smokestack", "the gearbox", "the quota board",
                "the punch clock", "the boiler", "the crane", "the steam", "the vat",
                "the product", "the siren", "the assembly line", "a spare part",
                "the anvil", "the grinding", "the heat", "the line manager",
            ],
            predicates: &[
                "is making you", "counts heartbeats", "is ahead of schedule",
                "wants another shift", "never cools down", "is measuring you",
                "is running on fumes", "has your number stamped in it",
                "screams on the hour", "is short one part", "is shaped like you",
                "hisses when you stop", "is over capacity", "won't let you clock out",
                "feeds on overtime", "is already late", "is looking for a replacement",
                "has been running since before you", "is humming your heartbeat",
                "is hungry for more", "is building something",
                "keeps the lights red", "is watching the line",
            ],
            whispers: &[
                "the machines are making you", "keep moving. they count heartbeats.",
                "production is ahead of schedule", "do not become the product",
                "the whistle blows at midnight", "hands inside the line at all times",
                "your shift never started", "quota: one more",
                "the furnace remembers everyone", "clock in. clock in. clock in.",
                "the product is looking back", "stop and they notice",
            ],
        },
        Awakening => Vocab {
            adjectives: &[
                "MORNING", "GOLDEN", "QUIET", "WARM", "FIRST", "SUNLIT", "GENTLE", "SOFT",
                "CLEAR", "EARLY", "BRIGHT", "STILL", "KIND", "FAMILIAR", "WAKING", "LINEN",
                "DAWNING", "HONEY-LIT", "SLOW", "FRESH", "RESTED", "OPEN", "HOME",
                "UNHURRIED", "CALM", "PALE", "YAWNING", "SAFE",
            ],
            nouns: &[
                "LIGHT", "ROOM", "WINDOW", "BREATH", "HOUR", "DAWN", "SUNRISE", "PILLOW",
                "BLANKET", "CURTAINS", "BEDROOM", "KETTLE", "ALARM", "COFFEE", "MORNING",
                "BIRDSONG", "DAYLIGHT", "SHEETS", "DOORWAY", "SUNBEAM", "SUNDAY", "SLIPPERS",
                "RADIO", "TOAST", "HOMECOMING",
            ],
            relatives: &[
                "LET YOU GO", "WAITED UP", "IS JUST A ROOM", "OPENED THE CURTAINS",
                "REMEMBERS YOU", "IS REALLY HERE", "BROUGHT YOU BACK", "IS ALREADY WARM",
                "SAYS GOOD MORNING", "KEPT THE LIGHT ON", "FOUND YOU", "IS STILL TODAY",
            ],
            abstracts: &[
                "WAKING", "HOME", "DAYLIGHT", "RETURNING", "COFFEE", "RELIEF", "SUNDAY",
                "CLEAN SHEETS", "BIRDSONG", "FIRST LIGHT", "BREAKFAST", "OPEN WINDOWS", "WARM TOAST",
            ],
            subjects: &[
                "the alarm", "the light", "the kettle", "the window", "your pillow",
                "the curtain", "a blackbird", "the radio", "the morning", "your name",
                "the room", "the day", "the duvet", "the sun",
                "the mug on the nightstand", "the ceiling", "the cat", "the toaster",
                "the phone on the pillow", "the floorboard", "a sunbeam", "the bathroom mirror",
                "your breath", "the street outside",
            ],
            predicates: &[
                "is ringing, softly", "is warm on your face", "is almost boiling",
                "is open a crack", "still has the shape of you", "is singing",
                "is playing something you know", "is yours again", "comes back to you",
                "is exactly where you left it", "is only just starting",
                "is real this time", "is waiting, patiently",
                "smells like toast", "is glowing at the edges", "is purring",
                "creaks, the way it always does", "has one missed call", "is fogged up",
                "is ordinary again", "is humming a morning song", "is gold for a moment",
                "is still a little warm", "feels like sunday",
            ],
            whispers: &[
                "you remember your name", "the alarm is ringing, softly",
                "it was only a dream. mostly.", "welcome back", "you're awake now",
                "good morning", "it's okay. you're home.", "stretch. breathe.",
                "the dream is already fading", "someone made coffee",
            ],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dream::theme::ALL_THEMES;
    use std::collections::HashSet;

    #[test]
    fn every_list_is_non_empty_and_big() {
        for t in ALL_THEMES {
            let v = vocab(t);
            let big = if t == DreamTheme::Awakening { 10 } else { 18 };
            for (what, list) in [
                ("adjectives", v.adjectives),
                ("nouns", v.nouns),
                ("relatives", v.relatives),
                ("subjects", v.subjects),
                ("predicates", v.predicates),
            ] {
                assert!(list.len() >= big, "{t:?} {what}: only {}", list.len());
            }
            assert!(!v.abstracts.is_empty() && !v.whispers.is_empty(), "{t:?}");
        }
    }

    #[test]
    fn the_bank_is_very_large() {
        let total: usize = ALL_THEMES
            .iter()
            .map(|&t| {
                let v = vocab(t);
                v.adjectives.len()
                    + v.nouns.len()
                    + v.relatives.len()
                    + v.abstracts.len()
                    + v.subjects.len()
                    + v.predicates.len()
                    + v.whispers.len()
            })
            .sum::<usize>()
            + ABSTRACTS.len()
            + CODAS.len();
        assert!(total >= 1000, "word bank has {total} entries");
    }

    #[test]
    fn case_rules() {
        for t in ALL_THEMES {
            let v = vocab(t);
            for w in v
                .adjectives
                .iter()
                .chain(v.nouns)
                .chain(v.relatives)
                .chain(v.abstracts)
            {
                assert_eq!(*w, w.to_uppercase(), "{t:?}: {w}");
            }
            for w in v.subjects.iter().chain(v.predicates).chain(v.whispers) {
                assert_eq!(*w, w.to_lowercase(), "{t:?}: {w}");
            }
        }
        assert!(ABSTRACTS.iter().all(|w| *w == w.to_uppercase()));
        assert!(CODAS.iter().all(|w| *w == w.to_lowercase()));
    }

    #[test]
    fn no_duplicates_and_each_theme_has_its_own_nouns() {
        let mut nouns = HashSet::new();
        for t in ALL_THEMES {
            let v = vocab(t);
            for n in v.nouns {
                assert!(nouns.insert(*n), "{t:?} reuses noun {n}");
            }
            for (what, list) in [
                ("adjectives", v.adjectives),
                ("relatives", v.relatives),
                ("subjects", v.subjects),
                ("predicates", v.predicates),
                ("whispers", v.whispers),
            ] {
                let set: HashSet<_> = list.iter().collect();
                assert_eq!(set.len(), list.len(), "{t:?} {what} has duplicates");
            }
        }
    }

    /// Whispers are built as "{subject} {predicate}" with subjects from any
    /// dream, so every subject must be singular and every predicate must
    /// conjugate for a singular subject.
    #[test]
    fn whisper_grammar_agrees() {
        const ENDS_IN_S_BUT_SINGULAR: &[&str] = &[
            "glass", "moss", "press", "hum", "grass", "gas", "this", "is", "bus", "canvas", "less",
            "boss", "dress",
        ];
        for t in ALL_THEMES {
            let v = vocab(t);
            for s in v.subjects {
                let last = s.rsplit(' ').next().unwrap();
                assert!(
                    !last.ends_with('s') || ENDS_IN_S_BUT_SINGULAR.contains(&last),
                    "{t:?}: plural subject {s:?}"
                );
            }
            for p in v.predicates {
                let first = p.split(' ').next().unwrap();
                assert!(
                    !["are", "were", "have", "turn", "know", "want", "keep"].contains(&first),
                    "{t:?}: plural predicate {p:?}"
                );
            }
        }
    }
}
