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
        CursedForest => Vocab {
            adjectives: &[
                "HOLLOW", "WHISPERING", "BRAMBLED", "MOSS-EATEN", "CROOKED", "HUNGRY",
                "MOONLESS", "TANGLED", "DRIPPING", "ROTTING", "HUSHED GREEN", "LANTERN-LIT",
                "UNMAPPED", "THORNED", "BREATHING", "OLD-GROWTH", "SPORE-HEAVY", "WITCHED",
                "KNOTTED", "LICHENED", "SLEEPLESS", "FOGBOUND", "ROOT-BOUND", "CAWING",
                "UNDERGROWN", "PALE-BARKED", "MOULDERING", "WET BLACK",
            ],
            nouns: &[
                "BLACK THICKET", "HOLLOW OAK", "THORN BUSH", "CLEARING", "TOADSTOOL RING", "WEEPING ELM",
                "COPSE", "DEADFALL", "FERN GULLY", "WOODCUTTER", "LANTERN", "CROW",
                "MUSHROOM", "STUMP", "BURROW", "WOLF", "WITCH HUT", "GALLOWS TREE",
                "BIRCH", "PINE NEEDLE", "OWL", "FOX DEN", "MARSH LIGHT", "BRIAR",
            ],
            relatives: &[
                "LEANS CLOSER", "EATS THE PATH", "COUNTS YOUR STEPS", "HAS TEETH",
                "GROWS WHILE YOU SLEEP", "KNOWS THE WAY OUT", "WON'T LET GO",
                "HUMS UNDERGROUND", "SWALLOWED THE MOON", "WATCHES FROM ABOVE",
                "CALLS IN YOUR VOICE", "MOVES WHEN YOU BLINK", "IS OLDER THAN YOU",
                "ASKS FOR A NAME", "LEADS YOU IN CIRCLES", "DRINKS THE FOG",
                "NEVER CASTS A SHADOW", "KEEPS THE LOST", "SMELLS OF SMOKE",
                "HAS ALWAYS BEEN HERE",
            ],
            abstracts: &[
                "BREADCRUMBS", "WOLF HOURS", "THE DEEP WOODS", "SPORES", "OLD STORIES",
                "BROKEN TWIGS", "THE WITCHING HOUR", "WET LEAVES",
            ],
            subjects: &[
                "the crow", "a lantern", "the old oak", "the path", "the woodcutter",
                "a toadstool", "the bramble", "the owl", "the fox", "a root",
                "the stump", "the marsh light", "the clearing", "a birch", "the wolf",
                "the witch", "the moss", "the fern", "the hollow", "a pine cone",
            ],
            predicates: &[
                "is following you", "has a face in it", "is whispering your name",
                "was not there before", "moved while you blinked", "is counting backwards",
                "is growing through the ground", "is hungry again", "smells like smoke",
                "knows which way you came", "keeps pointing north", "is laughing, quietly",
                "is wearing your coat", "has too many eyes", "hums under the soil",
                "is leaning closer", "drinks the fog", "is carved with your initials",
                "only lights when you look away",
            ],
            whispers: &[
                "stay on the path", "the trees are closer now", "don't eat anything",
                "leave a trail", "it's older than the dark", "the crows are counting",
                "someone left a lantern burning", "the woods remember you",
            ],
        },
        DrownedLibrary => Vocab {
            adjectives: &[
                "DROWNED", "SUNKEN", "WATERLOGGED", "SILENT", "BLUE-LIT", "FLOODED",
                "INKY", "SALT-STAINED", "BUBBLING", "OVERDUE", "SWOLLEN", "DEEP",
                "BARNACLED", "UNREAD", "RIPPLING", "TIDAL", "SHUSHING", "BLOATED",
                "CATALOGUED", "SUBMERGED", "PAPER-THIN", "GLASS-BOTTOMED", "GURGLING",
                "SEAWEED-BOUND", "AQUARIUM-LIT",
            ],
            nouns: &[
                "LIBRARY", "ARCHIVE", "READING ROOM", "STACKS", "CARD CATALOGUE",
                "BOOKSHELF", "LIBRARIAN", "ATLAS", "MANUSCRIPT", "ENCYCLOPEDIA",
                "BOOKMARK", "LECTERN", "INKWELL", "MARGIN", "FOOTNOTE", "GLOSSARY",
                "DIVING BELL", "JELLYFISH", "ANCHOR", "PORTHOLE", "SEA CHART", "EEL",
                "SHIPWRECK", "LIGHTHOUSE",
            ],
            relatives: &[
                "READS YOU BACK", "IS STILL SINKING", "FORGOT HOW TO BREATHE",
                "IS OVERDUE", "HAS NO LAST PAGE", "HUMS LIKE A WHALE", "RUNS OUT OF INK",
                "FILLS WITH WATER", "SHUSHES THE SEA", "KNOWS YOUR HANDWRITING",
                "FLOATS UPWARD", "LOST ITS INDEX", "WRITES ITSELF", "DROWNS SLOWLY",
                "IS FULL OF FISH", "KEEPS THE TIDE", "HAS WET PAGES", "IS ALWAYS CLOSING",
                "SPEAKS IN BUBBLES", "WAS NEVER RETURNED",
            ],
            abstracts: &[
                "LATE FEES", "WET INK", "LOST CHAPTERS", "THE DEEP SHELF", "BUBBLES",
                "PRESSURE", "QUIET HOURS", "THE ABYSS",
            ],
            subjects: &[
                "the librarian", "a book", "the card catalogue", "the globe", "an eel",
                "the lectern", "your library card", "the inkwell", "a jellyfish",
                "the anchor", "the porthole", "the footnote", "a bookmark",
                "the reading lamp", "the index", "the diving bell", "the stamp",
                "the lighthouse", "the sea chart", "the margin",
            ],
            predicates: &[
                "is reading over your shoulder", "is full of water", "is still sinking",
                "has your name in the index", "is overdue by a century",
                "is breathing bubbles", "keeps turning its own pages",
                "is written in a language you almost know", "glows a little blue",
                "is humming like a whale", "has a fish living in it", "shushes you",
                "is missing a chapter", "is stamped with tomorrow's date",
                "is drifting upward", "is swollen with salt", "tastes of ink",
                "has been underlined twice", "drips when you open it",
            ],
            whispers: &[
                "quiet, please", "hold your breath", "the tide is in the stacks",
                "no talking below the waterline", "every book is a little wet",
                "return by high tide", "it's further down", "the ink is still running",
            ],
        },
        SkyStairs => Vocab {
            adjectives: &[
                "WEIGHTLESS", "CLOUDED", "ENDLESS", "SPIRALLING", "ASCENDING", "WINDSWEPT",
                "BRIGHT", "FEATHERED", "DIZZY", "UPSIDE-DOWN", "SUNLIT", "HIGH",
                "FLOATING", "VAPOROUS", "THINNING", "SOARING", "CIRRUS", "BOTTOMLESS",
                "HALOED", "GLIDING", "STARWARD", "BREEZY", "HANGING", "SWAYING",
            ],
            nouns: &[
                "STAIRWAY", "STEP", "LANDING", "BANISTER", "CLOUD", "KITE", "BALLOON",
                "WEATHERVANE", "BELL TOWER", "ROOFTOP", "SKYLIGHT", "FEATHER",
                "HOT AIR", "SHOOTING STAR", "SKY LADDER", "PAPER PLANE", "HALO", "GONDOLA",
                "RAINBOW", "WIND CHIME", "HANG GLIDER", "OBSERVATORY",
            ],
            relatives: &[
                "GOES UP FOREVER", "HAS NO TOP", "IS MADE OF AIR", "SKIPS A STEP",
                "HOLDS YOU UP", "FORGETS TO FALL", "TURNS THE WRONG WAY", "HUMS IN THE WIND",
                "CATCHES THE SUN", "LEADS NOWHERE", "IS LIGHTER THAN YOU",
                "DRIFTS AWAY", "LOOKS DOWN ON YOU", "CARRIES YOUR NAME", "NEVER LANDS",
                "IS STILL CLIMBING", "SPINS SLOWLY", "HANGS FROM NOTHING",
                "SINGS WHEN IT RAINS", "REACHES THE MOON",
            ],
            abstracts: &[
                "VERTIGO", "OPEN SKY", "HIGH NOON", "THE UPDRAFT", "THIN AIR",
                "CLOUD COVER", "THE FALL", "WEIGHTLESSNESS",
            ],
            subjects: &[
                "the staircase", "a cloud", "the next step", "the banister", "a kite",
                "the balloon", "the wind", "a feather", "the sun", "the landing",
                "the weathervane", "the bell", "a paper plane", "the rainbow",
                "the wind chime", "the gondola", "the comet", "the halo", "the sky",
                "the ground",
            ],
            predicates: &[
                "is very far below", "goes up forever", "is holding you up",
                "is lighter than it looks", "turns the wrong way", "is made of fog",
                "keeps rising", "has no top", "is spinning, slowly", "is singing in the wind",
                "is just out of reach", "forgot how to fall", "hums like a kite string",
                "is warm from the sun", "is drifting sideways", "has one missing step",
                "is waiting at the top", "is thinner up here", "is leaning into the wind",
            ],
            whispers: &[
                "don't look down", "one more flight", "the air is thin here",
                "you can't fall if you don't stop", "the top is always further",
                "hold the banister", "the clouds are solid today", "breathe slowly",
            ],
        },
        MirrorHall => Vocab {
            adjectives: &[
                "MIRRORED", "REVERSED", "DOUBLED", "SILVERED", "GLASSY", "SYMMETRIC",
                "ECHOED", "REFLECTED", "SHIMMERING", "PEARLESCENT", "IRIDESCENT", "POLISHED",
                "TWINNED", "BACKWARDS", "INVERTED", "FRACTURED", "CRYSTAL", "PRISMATIC",
                "HALL-OF-MIRRORS", "OPALINE", "CLEAR", "TWO-FACED",
            ],
            nouns: &[
                "MIRROR HALL", "LOOKING GLASS", "REFLECTION", "TWIN", "DOPPELGANGER",
                "PRISM", "VANITY", "DRESSING ROOM", "KALEIDOSCOPE", "SILVER FRAME",
                "PANE", "SHARD OF GLASS", "PERISCOPE", "CHANDELIER OF GLASS", "GLOSS",
                "OPAL", "PEARL", "LENS", "HALL OF GLASS", "DOUBLE", "MIRAGE", "FACET",
            ],
            relatives: &[
                "LOOKS BACK", "MOVES FIRST", "IS NOT YOU", "SMILES LATE",
                "WALKS THE OTHER WAY", "KNOWS YOUR LEFT HAND", "NEVER BLINKS",
                "COPIES EVERYTHING", "SHOWS TOMORROW", "HAS NO BACK", "CRACKS WHEN YOU LIE",
                "IS ONE STEP BEHIND", "WEARS YOUR FACE", "WON'T HOLD STILL",
                "LEADS INTO ITSELF", "IS A LITTLE OFF", "SPLITS THE LIGHT",
                "REMEMBERS YOU DIFFERENTLY", "HAS A SECOND DOOR", "IS STILL WATCHING",
            ],
            abstracts: &[
                "SYMMETRY", "SEVEN YEARS", "SILVER", "THE OTHER SIDE", "REFRACTION",
                "SECOND SELVES", "CRACKED GLASS", "LEFT AND RIGHT",
            ],
            subjects: &[
                "your reflection", "the mirror", "the other you", "the prism", "a pane",
                "the silver frame", "the looking glass", "your twin", "the magnifier",
                "the vanity", "a facet", "the kaleidoscope", "the pearl", "the mirage",
                "the far wall", "the doorway", "your left hand", "the glass floor",
                "the chandelier", "a doppelganger",
            ],
            predicates: &[
                "moved first", "is not quite you", "is smiling a little late",
                "is walking the other way", "is watching you back", "has cracked, quietly",
                "shows a different room", "is one step behind", "is breathing on the glass",
                "is wearing your clothes backwards", "never blinks", "is waving goodbye",
                "keeps splitting the light", "is looking for the exit too",
                "has your voice", "is humming in harmony", "has a door on its side only",
                "is getting closer to the glass", "is counting in reverse",
            ],
            whispers: &[
                "left is right here", "don't touch the glass", "which one is you",
                "it copies everything", "the exit is behind you", "count the reflections",
                "one of you is dreaming", "smile back",
            ],
        },
        MyceliumGrove => Vocab {
            adjectives: &[
                "GLOWING", "THREADED", "SPORING", "UNDERGROUND", "ROOTED", "BIOLUMINESCENT",
                "HUMMING", "NETWORKED", "FRUITING", "DAMP", "LUMINOUS", "WEBBED", "SOFT-GILLED",
                "PULSING", "ANCIENT", "WHISPER-THIN", "FUNGAL", "ENTANGLED", "LISTENING",
                "CONNECTED", "MOULDY", "VELVET-CAPPED", "DECOMPOSING", "BREATHING GREEN",
            ],
            nouns: &[
                "MYCELIUM", "SPORE CLOUD", "FRUITING BODY", "GILL", "HYPHA", "CAP",
                "FAIRY RING", "PUFFBALL", "MOREL", "TRUFFLE", "ROOT BRIDGE", "LEAF LITTER",
                "UNDERSTORY", "MOSS BED", "LICHEN", "STINKHORN", "CHANTERELLE", "SLIME MOULD",
                "WOOD WIDE WEB", "GROVE", "NURSE LOG", "HUMUS",
            ],
            relatives: &[
                "KNOWS EVERY ROOT", "TALKS UNDERGROUND", "GLOWS WHEN YOU PASS",
                "REMEMBERS THE RAIN", "FEEDS ON QUIET", "SPREADS WHILE YOU SLEEP",
                "CONNECTS EVERYTHING", "BREATHES OUT LIGHT", "HAS NO EDGES", "SHARES ITS DREAMS",
                "LEADS YOU HOME", "IS ONE ORGANISM", "HUMS BELOW HEARING", "SPEAKS IN SPORES",
                "FRUITS AT MIDNIGHT", "GROWS TOWARD YOU", "DRINKS THE DARK", "NEVER ENDS",
                "IS OLDER THAN TREES", "HOLDS THE FOREST UP",
            ],
            abstracts: &[
                "SPORES", "SOFT LIGHT", "THE NETWORK", "UNDERGROWTH", "DECAY", "RENEWAL",
                "DAMP EARTH", "SLOW TIME",
            ],
            subjects: &[
                "the mycelium", "a spore", "the fairy ring", "a puffball", "the root bridge",
                "the glowing thread", "a cap", "the grove", "the moss", "a truffle",
                "the understory", "the slime mould", "a morel", "the nurse log",
                "the dark soil", "a chanterelle", "the web below", "the soil", "a gill", "the lichen",
            ],
            predicates: &[
                "is glowing softly", "is talking to the trees", "knows where you are going",
                "is breathing out light", "remembers you", "is sharing something with you",
                "is growing toward the shard", "hums below hearing", "is warm underfoot",
                "is older than the forest", "connects to everything", "is fruiting again",
                "leads the way", "is listening", "has no edges", "tastes of rain",
                "is spreading while you sleep", "is lit from inside", "pulses gently",
            ],
            whispers: &[
                "follow the light", "everything is connected", "the ground is listening",
                "the veins know the way", "walk where it glows", "the forest is one thing",
                "breathe with it", "you are part of it now",
            ],
        },
        TheTunnel => Vocab {
            adjectives: &[
                "ENDLESS", "NARROWING", "RECEDING", "SPIRALLING", "HOLLOW", "BRIGHT-ENDED",
                "WHOOSHING", "ELONGATED", "ECHOING DEEP", "CONVERGING", "RIBBED", "TELESCOPING",
                "RUSHING", "DOPPLER", "HYPNOTIC", "INFINITE", "TUBULAR", "LONG", "WARPED",
                "SUCTIONED", "STRETCHED", "RINGING",
            ],
            nouns: &[
                "TUNNEL", "PASSAGE", "CONDUIT", "VORTEX", "TUBE", "BURROWED WAY", "SHAFT",
                "WORMHOLE", "THROAT", "FUNNEL", "CULVERT", "SLIPSTREAM", "BORE", "CHUTE",
                "LIGHT AT THE END", "RIBCAGE", "HOOP", "SPIRAL", "FAR END", "UNDERPASS",
                "DRAIN", "SEWER PIPE",
            ],
            relatives: &[
                "PULLS YOU IN", "NEVER TURNS BACK", "GETS NARROWER", "HAS A LIGHT AT THE END",
                "BREATHES IN", "GOES ON FOREVER", "SPINS AS YOU WALK", "RINGS LIKE A BELL",
                "SWALLOWS SOUND", "OPENS AND CLOSES", "FOLDS INTO ITSELF", "CARRIES YOU FORWARD",
                "IS LONGER INSIDE", "KNOWS ONE DIRECTION", "HUMS AT ITS CENTRE", "IS ALIVE",
                "LEADS SOMEWHERE WHITE", "COUNTS YOUR STEPS", "BENDS LIGHT", "HAS NO WALLS",
            ],
            abstracts: &[
                "THE PULL", "VANISHING POINTS", "FORWARD", "THE WHITE LIGHT", "ACCELERATION",
                "THE LONG WAY", "RINGS", "GOING THROUGH",
            ],
            subjects: &[
                "the tunnel", "the light at the end", "the next ring", "the passage",
                "the vortex", "a hoop", "the throat of it", "the far end", "the echo",
                "the floor", "the curve ahead", "the funnel", "the wormhole", "the draft",
                "a ring of light", "the centre", "the rushing", "the shaft", "the spiral",
                "the way back",
            ],
            predicates: &[
                "is pulling you forward", "gets narrower", "is spinning slowly",
                "is further than it looks", "is breathing in", "rings when you pass",
                "is closing behind you", "is opening ahead", "never gets closer",
                "hums at the centre", "is brighter now", "has no end", "is folding in on itself",
                "is carrying you", "swallows every sound", "bends the light",
                "has always been this long", "is just ahead", "is a little warm",
            ],
            whispers: &[
                "keep going toward the light", "don't stop in the rings", "it only goes one way",
                "time the gates", "the end is white", "you're nearly through",
                "it breathes in, then out", "go when it opens",
            ],
        },
        FractalCathedral => Vocab {
            adjectives: &[
                "SELF-SIMILAR", "NESTED", "VAULTED", "SACRED", "INFINITE", "RECURSIVE",
                "STAINED-GLASS", "GEOMETRIC", "HALLOWED", "ROSE-WINDOWED", "SYMMETRIC GOLD",
                "LATTICED", "ENDLESSLY SMALLER", "RADIANT", "TESSELLATED", "SOLEMN",
                "KALEIDOSCOPIC", "FACETED", "CHORAL", "GILDED DEEP", "HOLY", "DIVIDING",
            ],
            nouns: &[
                "CATHEDRAL", "NAVE", "ROSE WINDOW", "APSE", "CLOISTER", "CHOIR", "ALTAR",
                "VAULT", "BELL TOWER OF GLASS", "FLYING BUTTRESS", "RELIQUARY", "CHAPEL", "ORGAN PIPE",
                "TRIFORIUM", "MANDALA", "TESSERA", "INNER SANCTUM", "BELL", "CRYPT",
                "PEW", "FRACTAL", "HALO OF GLASS",
            ],
            relatives: &[
                "HOLDS A SMALLER ONE", "REPEATS FOREVER", "IS INSIDE ITSELF", "SINGS IN FIFTHS",
                "HAS NO SMALLEST ROOM", "FOLDS INWARD", "LIGHTS FROM WITHIN", "COUNTS IN SEVENS",
                "IS ALWAYS CENTRED", "OPENS ONE DOOR AT A TIME", "REFLECTS ITS OWN PATTERN",
                "WAS BUILT BY ECHOES", "HAS A HEART OF GLASS", "RINGS AT EVERY SCALE",
                "HIDES THE MIDDLE", "NEVER ENDS INWARD", "IS MADE OF SMALLER CATHEDRALS",
                "GLOWS GOLD", "PRAYS IN PATTERNS", "IS PERFECTLY STILL",
            ],
            abstracts: &[
                "SYMMETRY", "INFINITY", "THE CENTRE", "GOLDEN RATIOS", "RECURSION",
                "STAINED LIGHT", "HYMNS", "SACRED GEOMETRY",
            ],
            subjects: &[
                "the cathedral", "the rose window", "the smaller room", "the altar",
                "the nave", "the spire", "the choir", "the inner door", "the mandala",
                "the vault", "a bell", "the centre", "the pattern", "the crypt",
                "the organ", "the chapel inside the chapel", "a tessera", "the stained light",
                "the cloister", "the fractal",
            ],
            predicates: &[
                "holds a smaller one", "repeats forever", "is singing in fifths",
                "is lit from within", "opens one door at a time", "is always in the middle",
                "is folding inward", "glows gold", "rings at every scale", "is perfectly still",
                "hides the centre", "was built by echoes", "is made of smaller cathedrals",
                "reflects its own pattern", "is counting in sevens", "is inside itself",
                "never ends inward", "has a heart of glass", "prays in patterns",
            ],
            whispers: &[
                "the way in is the way through", "there is always a smaller door",
                "the centre is waiting", "every room holds another", "follow the pattern inward",
                "it repeats. it repeats.", "look for the open side", "almost at the heart",
            ],
        },
        Elfworks => Vocab {
            adjectives: &[
                "JEWELLED", "SELF-TRANSFORMING", "GIGGLING", "CLOCKWORK", "IMPOSSIBLE",
                "CHATTERING", "PRISMATIC", "TOYLIKE", "BOUNCING", "SHAPESHIFTING", "HYPERACTIVE",
                "ELVEN", "WHIRLING", "TINKLING", "OVERJOYED", "MISCHIEVOUS", "FACETED BRIGHT",
                "SYNCOPATED", "FIZZING", "UNFOLDING", "TWINKLING", "WIND-UP",
            ],
            nouns: &[
                "ELF WORKSHOP", "MACHINE ELF", "JESTER", "JEWEL ENGINE", "TOYBOX", "MUSIC BOX",
                "TEETOTUM", "JACK-IN-THE-BOX", "GEAR GARDEN", "CONFETTI", "KALEIDOPHONE",
                "SPINNING TOP", "WIND-UP BIRD", "PRISM PUMP", "BUBBLE ORGAN", "TINKER",
                "GIGGLE WHEEL", "JINGLE", "PUZZLE BOX", "WHIRLIGIG", "FUNHOUSE", "SELF-DRAWING PEN",
            ],
            relatives: &[
                "WANTS TO SHOW YOU SOMETHING", "NEVER STOPS BUILDING", "GIGGLES AT YOU",
                "TURNS INTO SOMETHING ELSE", "SINGS IN COLOURS", "IS MADE OF LANGUAGE",
                "OFFERS YOU A GIFT", "WON'T HOLD STILL", "KNOWS A TRICK", "CHANGES THE FLOOR",
                "BUILDS ITSELF", "TALKS TOO FAST", "IS VERY PLEASED TO SEE YOU", "JUGGLES LIGHT",
                "SPINS FOREVER", "WAS EXPECTING YOU", "WINDS ITSELF UP", "LAUGHS IN GEOMETRY",
                "TOSSES YOU ASIDE", "HAS A HUNDRED FACES",
            ],
            abstracts: &[
                "MISCHIEF", "JEWELS", "GIGGLES", "TRANSFORMATION", "THE GIFT", "PLAY",
                "NONSENSE", "TINY MACHINES",
            ],
            subjects: &[
                "the machine elf", "a jester", "the workshop", "the music box", "a spinning top",
                "the jewel engine", "the toybox", "a wind-up bird", "the whirligig",
                "the giggle wheel", "a puzzle box", "the floor", "the confetti", "a tinker",
                "the funhouse", "the prism pump", "the bubble organ", "a jingle",
                "the gear garden", "the self-drawing pen",
            ],
            predicates: &[
                "wants to show you something", "is giggling", "turned into something else",
                "is building itself", "sings in colours", "is very pleased to see you",
                "won't hold still", "knows a trick", "moved the floor", "talks too fast",
                "is juggling light", "was expecting you", "is winding itself up",
                "laughs in geometry", "tosses you aside", "has a hundred faces",
                "is offering you a gift", "spins forever", "is made of language",
            ],
            whispers: &[
                "don't let them move you", "the floor is playing", "it's a game. probably.",
                "take the gift", "everything here is alive", "watch the tiles",
                "they only want to play", "you were expected",
            ],
        },
        AfterimageFields => AFTERIMAGE_FIELDS,
        SynesthesiaHall => SYNESTHESIA_HALL,
        MeltingClockworks => MELTING_CLOCKWORKS,
        JellyfishSky => JELLYFISH_SKY,
        WatchingWallpaper => WATCHING_WALLPAPER,
        WhiteDissolve => WHITE_DISSOLVE,
    }
}

// ── Six new dreams — word lists only; match arms wired in a later task ──

/// Afterimage dream: light that lags, smears, and lingers after you pass.
#[rustfmt::skip]
pub const AFTERIMAGE_FIELDS: Vocab = Vocab {
    adjectives: &[
        "SMEARED", "TRAILING", "PASTEL", "LINGERING", "DOUBLED", "OVEREXPOSED",
        "SLOW", "ECHOING", "STREAKED", "GHOSTED", "LAGGING", "SOFT-FOCUS",
        "BLURRED", "SHIMMERING", "REPEATING", "FADING", "TRACED", "LUMINOUS",
        "TWICE-SEEN", "WAVERING",
    ],
    nouns: &[
        "AFTERIMAGE", "SMEAR", "TRACER", "LONG EXPOSURE", "STREAK", "POLAROID",
        "CONTRAIL", "MOTION BLUR", "SHUTTER", "SPARKLER", "COMET TAIL",
        "PINWHEEL", "WINDSOCK", "DANDELION", "KITE STRING", "PASTEL FIELD",
        "SLOW WAVE", "LIGHT TRAIL", "RIBBON DANCER", "OPEN SHUTTER",
    ],
    relatives: &[
        "FOLLOWS YOU A LITTLE LATE", "IS STILL WHERE YOU LEFT IT", "LEAVES A TRAIL",
        "MOVES TWICE", "WON'T FINISH FADING", "REMEMBERS YOUR SHAPE",
        "SMEARS WHEN YOU BLINK", "ARRIVES BEFORE IT LEAVES", "IS MOSTLY TRAIL",
        "GLOWS WHERE YOU WERE", "REPEATS ITSELF", "DRAGS THE LIGHT BEHIND IT",
        "IS A SECOND LATE", "NEVER QUITE CATCHES UP", "OUTLINES EVERYTHING",
        "KEEPS YOUR LAST STEP", "LEAVES COLOUR ON THE AIR", "HANGS IN THE AIR",
    ],
    abstracts: &[
        "AFTERGLOW", "LATENESS", "TRAILS", "SOFT LIGHT", "THE LAST SECOND",
        "DOUBLE VISION", "SLOW MOTION", "LINGERING",
    ],
    subjects: &[
        "the afterimage", "a smear of light", "the tracer", "the long exposure",
        "a streak", "the polaroid", "the contrail", "the motion blur",
        "the shutter", "a sparkler", "the comet tail", "the pinwheel",
        "the windsock", "a dandelion", "the kite string", "the pastel field",
        "the slow wave", "the light trail", "the ribbon dancer",
    ],
    predicates: &[
        "is running late", "follows a second behind", "left a trail",
        "is still fading", "moved twice", "repeats itself",
        "smears when you blink", "glows where you were", "remembers your shape",
        "hangs in the air", "drags the light behind it", "is mostly trail",
        "never quite catches up", "outlines everything", "keeps your last step",
        "arrived before it left", "is overexposed", "won't finish fading",
    ],
    whispers: &[
        "you were just here", "the light is late", "everything leaves a mark",
        "look where you've been", "slow down and it catches up",
        "you are the brightest trail", "it's only an echo",
        "don't chase your shadow",
    ],
};

/// Synesthesia dream: a hall where senses cross — sound has a colour, colour has a beat.
#[rustfmt::skip]
pub const SYNESTHESIA_HALL: Vocab = Vocab {
    adjectives: &[
        "LOUD", "HUMMING", "CHROMATIC", "SUSTAINED", "AMPLIFIED", "RESONANT",
        "TUNED", "FEEDBACK", "BASS-HEAVY", "STROBING", "ELECTRIC", "RINGING",
        "HARMONIC", "OFF-KEY", "THROBBING", "VIVID", "SINGING", "PERCUSSIVE",
        "TREMBLING", "WIDESCREEN",
    ],
    nouns: &[
        "STAGE", "SPEAKER STACK", "SPECTRUM", "CHORD", "OSCILLOSCOPE",
        "AMPLIFIER", "TUNING FORK", "BASSLINE", "METRONOME", "XYLOPHONE",
        "TAMBOURINE", "SOUNDWAVE", "EQUALIZER", "DRUM KIT",
        "COLOUR ORGAN", "SUBWOOFER", "KEYBOARD", "TONE ROW", "WAVEFORM",
        "MIXING DESK",
    ],
    relatives: &[
        "TASTES LIKE BLUE", "SOUNDS LIKE YELLOW", "PLAYS YOUR HEARTBEAT",
        "HUMS IN COLOUR", "DROPS ON THE ONE", "SEES THE MUSIC",
        "CAN HEAR YOUR FOOTSTEPS", "COUNTS YOU IN", "NEVER MISSES A BEAT",
        "PAINTS THE FLOOR", "IS TUNED TO YOU", "FEEDS BACK",
        "PULSES IN TIME", "TURNS SOUND INTO LIGHT", "IS PLAYING TOO LOUD",
        "SMELLS LIKE A CHORD", "SHAKES THE WALLS", "SINGS IN RED",
    ],
    abstracts: &[
        "THE DOWNBEAT", "LOUD COLOURS", "THE BASS", "FEEDBACK",
        "THE CHORUS", "HARMONY", "THE ENCORE", "PERFECT PITCH",
    ],
    subjects: &[
        "the stage", "the speaker stack", "the spectrum", "the chord",
        "the oscilloscope", "the amplifier", "the tuning fork", "the bassline",
        "the metronome", "the xylophone", "the tambourine", "the soundwave",
        "the equalizer", "the drum kit", "the colour organ", "the subwoofer",
        "the keyboard", "the tone row", "the waveform", "the mixing desk",
    ],
    predicates: &[
        "tastes like blue", "sounds like yellow", "is playing your heartbeat",
        "hums in colour", "drops on the one", "can hear your footsteps",
        "is counting you in", "never misses a beat", "paints the floor",
        "is tuned to you", "feeds back", "pulses in time",
        "turns sound into light", "is playing too loud",
        "smells like a chord", "shakes the walls", "sings in red",
        "is louder in the dark",
    ],
    whispers: &[
        "cross on the off-beat", "listen to the floor",
        "it's too loud to think", "the colours have a rhythm",
        "wait for the quiet part", "count to two",
        "feel it in your teeth", "don't step on the one",
    ],
};

/// Melting clockworks dream: a workshop where clockwork softens, drips, and loops a single minute forever.
#[rustfmt::skip]
pub const MELTING_CLOCKWORKS: Vocab = Vocab {
    adjectives: &[
        "MELTING", "DRIPPING", "BRASS", "TICKING", "SLOW", "LOOPING",
        "UNWOUND", "OVERWOUND", "STOPPED", "ANTIQUE", "AMBER", "SAGGING",
        "RUNNING-LATE", "CYCLIC", "SOFT", "GILDED", "RECURRING",
        "CLOCKWISE", "GRANDFATHERLY", "RUSTED",
    ],
    nouns: &[
        "POCKET WATCH", "HOURGLASS", "PENDULUM", "CUCKOO CLOCK", "MAINSPRING",
        "CLOCK FACE", "ESCAPEMENT", "MINUTE HAND", "WATCHMAKER", "TIME LOOP",
        "WALL CLOCK", "CHRONOMETER", "BRASS KEY", "SOFT CLOCK", "CARILLON",
        "ALARM CLOCK", "GRANDFATHER CLOCK", "COGWHEEL", "SECOND HAND", "CALENDAR",
    ],
    relatives: &[
        "RUNS FIVE MINUTES FAST", "HAS DONE THIS BEFORE", "MELTS AT NOON",
        "IS ALWAYS NOW", "WINDS ITSELF BACK", "TICKS OUT OF ORDER",
        "FORGOT WHAT DAY IT IS", "DRIPS OFF THE SHELF", "STARTS AGAIN",
        "KEEPS THE WRONG TIME", "WAS HERE TOMORROW",
        "REPEATS EVERY TWENTY SECONDS", "SAGS OVER THE EDGE", "COUNTS BACKWARDS",
        "STOPPED AT THREE", "CHIMES FOR NO ONE", "NEVER RUNS OUT", "LOOPS FOREVER",
    ],
    abstracts: &[
        "LOST TIME", "THE SAME MINUTE", "NOON", "YESTERDAY", "THE LOOP",
        "BORROWED TIME", "THE HOUR", "DEJA VU",
    ],
    subjects: &[
        "the pocket watch", "the pendulum", "the cuckoo clock", "the mainspring",
        "the clock face", "the escapement", "the minute hand", "the watchmaker",
        "the time loop", "the wall clock", "the chronometer", "the brass key",
        "the soft clock", "the carillon", "the alarm clock",
        "the grandfather clock", "the cogwheel", "the second hand", "the calendar",
    ],
    predicates: &[
        "runs five minutes fast", "has done this before", "melts at noon",
        "is always now", "winds itself back", "ticks out of order",
        "forgot what day it is", "is dripping off the shelf",
        "is starting again", "keeps the wrong time", "was here tomorrow",
        "repeats every twenty seconds", "sags over the edge", "counts backwards",
        "stopped at three", "chimes for no one", "never runs out", "loops forever",
    ],
    whispers: &[
        "you've been here before", "twenty seconds", "the clock is lying",
        "it resets but you don't", "again. again.", "hurry, slowly",
        "time is soft here", "listen for the chime",
    ],
};

/// Jellyfish sky dream: a sky full of drifting gelatinous bodies that glow and pulse in the dark.
#[rustfmt::skip]
pub const JELLYFISH_SKY: Vocab = Vocab {
    adjectives: &[
        "DRIFTING", "BIOLUMINESCENT", "TRANSLUCENT", "WEIGHTLESS", "PULSING",
        "TENDRILLED", "GLOWING", "BLUE", "FLOATING", "SLOW-BLOOMING", "STINGING",
        "TIDAL", "DEEP", "SILENT", "HOVERING", "BOBBING", "LUMINOUS",
        "GLASSY", "ETHEREAL", "BUOYANT",
    ],
    nouns: &[
        "MEDUSA", "MAN O' WAR", "SEA NETTLE", "COMB JELLY", "SKY CURRENT",
        "PLANKTON CLOUD", "SIPHONOPHORE", "TENDRIL", "SWIM BELL", "GLOW DOME",
        "SEA ANGEL", "SALP", "NIGHT TIDE", "UPDRAFT", "AIR REEF",
        "DRIFTING ISLE", "LIGHT STING", "JELLY LANTERN", "PHOSPHOR SEA",
        "MOON JELLY",
    ],
    relatives: &[
        "DRIFTS WHERE IT LIKES", "GLOWS WHEN IT BREATHES", "HAS NO BONES",
        "STINGS SOFTLY", "SWIMS THROUGH AIR", "PULSES LIKE A HEART",
        "FLOATS ABOVE THE VOID", "IS MOSTLY WATER", "TRAILS ITS LIGHTS",
        "NEVER TOUCHES DOWN", "HUMS IN THE DARK", "CARRIES YOU UP",
        "IS OLDER THAN THE SKY", "LIGHTS THE WAY DOWN",
        "OPENS AND CLOSES", "SINGS IN BLUE", "HOLDS ITS BREATH",
        "SLEEPS MID-AIR",
    ],
    abstracts: &[
        "THE DEEP SKY", "WEIGHTLESSNESS", "SOFT LIGHT", "THE CURRENT",
        "BLUE HOURS", "BIOLUMINESCENCE", "DRIFTING", "THE UPPER DARK",
    ],
    subjects: &[
        "the medusa", "the man o' war", "the sea nettle", "the comb jelly",
        "the sky current", "the plankton cloud", "the siphonophore", "the tendril",
        "the swim bell", "the glow dome", "the sea angel", "the salp",
        "the night tide", "the updraft", "the air reef",
        "the drifting isle", "the light sting", "the jelly lantern",
        "the phosphor sea", "the moon jelly",
    ],
    predicates: &[
        "drifts where it likes", "glows when it breathes", "has no bones",
        "stings softly", "swims through air", "pulses like a heart",
        "floats above the void", "is mostly water", "trails its lights",
        "never touches down", "hums in the dark", "carries you up",
        "lights the way down", "opens and closes", "sings in blue",
        "is holding its breath", "sleeps mid-air", "is older than the sky",
    ],
    whispers: &[
        "you weigh almost nothing", "jump and let it carry you",
        "don't touch the tendrils", "the sky is an ocean",
        "float, don't fall", "follow the glow", "breathe slowly",
        "everything here is drifting",
    ],
};

/// Watching wallpaper dream: floral patterns, faces, and a room that watches.
#[rustfmt::skip]
pub const WATCHING_WALLPAPER: Vocab = Vocab {
    adjectives: &[
        "PEELING", "FLORAL", "FADED", "PATTERNED", "STARING", "YELLOWED",
        "DAMASK", "WATCHFUL", "REPEATING", "SMILING", "PAISLEY",
        "NURSERY-PINK", "SEAMLESS", "CURLING", "WHISPERING", "STAINED",
        "PRINTED", "VICTORIAN", "MUSTY", "KNOWING",
    ],
    nouns: &[
        "WALLPAPER", "DAMASK ROSE", "FACE IN THE PATTERN", "PAISLEY EYE",
        "PICTURE RAIL", "SKIRTING BOARD", "SEAM", "NURSERY", "RORSCHACH",
        "TOILE", "FLORAL BORDER", "SMILING FLOWER", "WATCHER IN THE WALL",
        "TRELLIS PRINT", "PEELING CORNER", "MOULDING", "PASTE BUCKET",
        "REPEAT", "SAMPLE BOOK", "CEILING ROSE",
    ],
    relatives: &[
        "IS WATCHING YOU", "HAS A FACE IN IT", "BLINKS WHEN YOU DO",
        "SMILES AT THE CORNERS", "NEVER LOOKS AWAY", "MOVES WHEN YOU TURN",
        "KNOWS WHERE THE SHARD IS", "REPEATS YOUR NAME",
        "GREW ANOTHER EYE", "IS PEELING TOWARD YOU", "HIDES IN THE PATTERN",
        "FOLLOWS YOU ROOM TO ROOM", "WAS NEVER HUNG STRAIGHT",
        "COUNTS YOUR STEPS", "LEANS IN", "IS ONLY PAPER",
        "WAITS BEHIND YOU", "SEES ROUND CORNERS",
    ],
    abstracts: &[
        "BEING WATCHED", "THE PATTERN", "FACES", "OLD PASTE", "THE NURSERY",
        "PAREIDOLIA", "SOMEONE BEHIND YOU", "YELLOW ROOMS",
    ],
    subjects: &[
        "the wallpaper", "the damask rose",
        "the face in the pattern", "the paisley eye", "the picture rail",
        "the skirting board", "the seam", "the nursery", "the rorschach",
        "the toile", "the floral border", "the smiling flower",
        "the watcher in the wall", "the trellis print",
        "the peeling corner", "the moulding", "the paste bucket",
        "the repeat", "the sample book", "the ceiling rose",
    ],
    predicates: &[
        "is watching you", "has a face in it", "blinks when you do",
        "smiles at the corners", "never looks away", "moves when you turn",
        "knows where the shard is", "is repeating your name",
        "grew another eye", "is peeling toward you", "hides in the pattern",
        "follows you room to room", "counts your steps", "leans in",
        "is only paper", "waits behind you", "sees round corners",
        "was never hung straight",
    ],
    whispers: &[
        "don't turn around", "it only moves when you look away",
        "keep it in front of you", "the pattern has a face",
        "it's just paper. it's just paper.", "you're being watched",
        "look back", "the eyes know where the shard is",
    ],
};

/// White dissolve dream: a white, edge-less dissolving where self, name, and outline soften toward nothing.
#[rustfmt::skip]
pub const WHITE_DISSOLVE: Vocab = Vocab {
    adjectives: &[
        "WHITE", "BLANK", "DISSOLVING", "WEIGHTLESS", "PALE", "EMPTY",
        "BRIGHT", "SELFLESS", "BOUNDLESS", "QUIET", "EDGELESS", "FADING",
        "CLEAN", "SNOWBLIND", "NAMELESS", "STILL", "VAST", "UNWRITTEN",
        "UNMADE", "OPEN",
    ],
    nouns: &[
        "WHITEOUT", "BLANK PAGE", "VANISHING POINT", "SNOWFIELD", "FOG BANK",
        "CLEAN SLATE", "PALE HORIZON", "OUTLINE", "EMPTY FRAME",
        "LAST THOUGHT", "SILHOUETTE", "WHITE NOISE", "SOFT EDGE", "UNDERTOW",
        "FADE", "EGO", "MELTWATER", "SELF", "OPEN SKY", "STILL POINT",
    ],
    relatives: &[
        "FORGETS YOUR NAME", "HAS NO EDGES", "IS ALMOST NOTHING",
        "DISSOLVES WHEN YOU MOVE", "COMES BACK WHEN YOU STOP",
        "WAS YOU ALL ALONG", "UNWRITES ITSELF", "HOLDS NOTHING",
        "IS BRIGHTER THAN WAKING", "LETS GO", "CAN'T REMEMBER ITSELF",
        "GOES ON FOREVER", "IS WAITING FOR YOU TO STOP",
        "MELTS INTO WHITE", "FEELS LIKE MORNING", "ASKS NOTHING",
        "IS THE LAST ROOM", "SOFTENS AT THE EDGES",
    ],
    abstracts: &[
        "NOTHING", "LETTING GO", "THE SELF", "STILLNESS", "THE WHITE",
        "BEFORE WAKING", "NO ONE", "SILENCE",
    ],
    subjects: &[
        "the whiteout", "the blank page", "the vanishing point", "the snowfield",
        "the fog bank", "the clean slate", "the pale horizon", "the outline",
        "the empty frame", "the last thought", "the silhouette",
        "the white noise", "the soft edge", "the undertow", "the fade",
        "the ego", "the meltwater", "the self", "the open sky",
        "the still point",
    ],
    predicates: &[
        "forgets your name", "has no edges", "is almost nothing",
        "dissolves when you move", "comes back when you stop",
        "was you all along", "unwrites itself", "holds nothing",
        "is brighter than waking", "lets go", "can't remember itself",
        "goes on forever", "is waiting for you to stop",
        "melts into white", "feels like morning", "asks nothing",
        "softens at the edges", "is the last room",
    ],
    whispers: &[
        "stop, and it comes back", "don't look behind you", "let go",
        "you are almost awake", "stand still",
        "there's nothing here, not even you",
        "move and it forgets you", "breathe",
    ],
};

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
