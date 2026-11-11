# NOTES-N: relic names

## for approval: word lists (owner approves lists, not outputs)

Theme modifiers (adjectives, theme nouns, relatives, abstracts) are NOT new: they are the existing `dream::vocab` lists. New lists below.

### Head nouns by shape (role)
- **Cube**: DIE, RELIQUARY, BRICK, SUGAR LUMP, CRATE, STRONGBOX, MUSIC BOX, LUNCHBOX, KEYSTONE, TILE, BLOCK, JEWEL CASE
- **Octahedron**: DIAMOND, LOZENGE, PRISM, CRYSTAL, JEWEL, GEM, SPINDLE, KITE, SPARKLER, TWIN PYRAMID
- **Orb**: PEARL, MARBLE, EYE, BAUBLE, GLOBE, BUBBLE, SNOW GLOBE, SEED, YOLK, MOON, BEAD, BERRY
- **Cylinder**: PILLAR, SCROLL, CANDLE, BATTERY, BARREL, TORCH, ROLLING PIN, TUBE, FLASK, TRUNCHEON, DRUM, SPOOL
- **Cone**: SPIRE, FUNNEL, PARTY HAT, THORN, FANG, DUNCE CAP, CARROT, STEEPLE, WIZARD HAT, NEEDLE, TUSK, SNOW CONE
- **Torus**: HOOP, HALO, RING, NOOSE, BANGLE, WREATH, LIFEBUOY, DOUGHNUT, BAGEL, GASKET, COLLAR, LOOP
- **Arch**: GATE, DOORWAY, ARCH, PORTAL, ARCHWAY, LINTEL, WICKET, TRELLIS, THRESHOLD, DOORFRAME, HORSESHOE, YOKE
- **Cloud**: CLOUD, FLEECE, HEAP, PUFF, CLUMP, TUFT, BUNDLE, COTTON BALL, SPONGE, MOP, DRIFT, NEST
- **Crescent**: SICKLE, HORN, SMILE, PARING, MOON-BLADE, SCYTHE, CLAW, TALON, SLIVER, SCIMITAR, GRIN, CRESCENT
- **Island**: CRAG, BOULDER, ISLET, MESA, MOLAR, PEBBLE, BARNACLE, STALACTITE, ANVIL, ISLAND, GEODE, KEEP
- **Stairs**: STAIRCASE, STEPS, LADDER, STAIR, RUNGS, ESCALATOR, ZIGGURAT, STEPLADDER, FLIGHT, LANDING, SPIRAL STAIR, PLINTH

### Cracked soft words
CHIPPED, CRACKED, SPLIT, BENT, MENDED, STAINED, SCUFFED, DENTED, FRAYED, HOLLOWED, PITTED, SECONDHAND, TARNISHED, NOTCHED, SPENT, GLUED, SCRATCHED, CHEAP, LOANED, FORGOTTEN

### Foil sheen words
GILT, PRISMATIC, SHIMMERING, OPALESCENT, IRIDESCENT, GLITTERING, PEARLESCENT, HOLOGRAPHIC, MOTHER-OF-PEARL, LUSTROUS, SILVERED, SPANGLED, MOONLIT, STARDUSTED

### Hint: dust
HOARDER'S, PLENTIFUL, MAGPIE'S, JINGLING, OVERFLOWING, TITHING, MINTED, PROSPEROUS, OF PLENTY, OF GOOD HARVEST, OF TITHES

### Hint: speed
SWIFT, FLEETING, HURRIED, WINDBORNE, RESTLESS, NIMBLE, LIGHT-FOOTED, FEATHERED, OF HASTE, OF LONG STRIDES, OF THE LATE BUS

### Hint: calm
HUSHED, SOOTHING, SLEEPY, MUTED, LULLING, PEACEFUL, DROWSY, STILLED, OF HUSH, OF QUIET BREATHS, OF SLOW TIDES

### Hint: memory
REMEMBERING, KEEPSAKE, RECALLING, NOSTALGIC, MINDFUL, ARCHIVAL, RETENTIVE, DOG-EARED, OF RECALL, OF GOOD MEMORY, OF OLD LETTERS

### Hint: free rerolls
SECOND-GUESSING, DOUBTING, FICKLE, SHUFFLING, REDRAWN, UNDECIDED, RETRYING, TWO-MINDED, OF SECOND CHANCES, OF ANOTHER GO, OF CHANGED MINDS

### Hint: start shards
SHARD-BORN, GLINTING, SPLINTERED, FACETED, SHARD-HUNGRY, GLASSY, SPARKLING, SHATTERED, OF SHARDS, OF BROKEN GLASS, OF FALLEN STARS

### Hint: grace
FORGIVING, MERCIFUL, KINDLY, GENTLE, SPARING, LENIENT, PARDONING, FORBEARING, OF MERCY, OF KINDNESS, OF SMALL FAVOURS

### Hint: max nerve
STEADY, STEELY, BRAVE, UNFLINCHING, STOUT, STAUNCH, COURAGEOUS, UNSHAKEN, OF NERVE, OF STEADY HANDS, OF STILL KNEES

Wear prefixes: HEAVILY-WORN (Heavy), RUINED (Ruined). Mythic fixed words: LAST, THAT, BEFORE.

Flavour fragments (shape/rarity/wear lines) live in `relic_names.rs` `shape_lines`/`rarity_lines`/`wear_line`; they are lowercase whispers.

## Behaviour
- Wired nowhere yet (`#[allow(dead_code)] // R6`). API: `name`, `flavour`, `head_nouns`, `effect_hint`, `RelicSpec`.
- Decoration order when a name exceeds 40 chars: fresh word draws first, then drop hint, then trim template (tail), then drop foil; wear prefix is never dropped.
- Uniqueness: 12 seeded retries vs `taken`, then roman numeral suffix.
- `vocab` module made `pub(crate)` (only edit outside the file, plus the `mod` line).

## Samples: Cracked (40)
- Lobby / Stairs / Heavy: **A HEAVILY-WORN STILLED CONCIERGE PLINTH** - the edges have gone soft
- DrownedLibrary / Arch / Clean: **A RECALLING SCRATCHED GLOSSARY YOKE** - it opens inward, sometimes. it's only paint
- AfterimageFields / Stairs / Clean: **A SPANGLED GLUED STREAK STAIRCASE** - the pastel field won't finish fading
- LiminalOffice / Cone / Heavy: **A HEAVILY-WORN GLASSY HELP DESK FUNNEL** - the org chart is marked urgent
- SkyStairs / Arch / Clean: **AN IRIDESCENT CHIPPED TRACER WICKET** - a kite hums like a kite string
- SynesthesiaHall / Torus / Clean: **A SPLIT TUNING FORK HALO OF STEADY HANDS** - somebody else's, once
- VoidPlatforms / Octahedron / Clean: **GLUED UNANCHORED VOID DIAMOND OF NERVE** - it still mostly works
- MirrorHall / Cone / Clean: **A SCRATCHED REFLECTED CRYSTAL FUNNEL** - held together by habit
- MeltingClockworks / Cloud / Ruined: **A RUINED APSE CLUMP OF SLOW TIDES** - the watchmaker counts backwards
- Garden / Octahedron / Heavy: **A HEAVILY-WORN WILLOW PRISM OF RECALL** - the edges have gone soft
- MyceliumGrove / Crescent / Ruined: **RUINED VELVET-CAPPED LICHEN SCIMITAR** - better than nothing
- JellyfishSky / Orb / Ruined: **A RUINED MAN O' WAR BEAD OF STEADY HANDS** - the updraft trails its lights
- NightmareFactory / Cone / Ruined: **A RUINED CIRRUS ENDLESS TUSK OF PLENTY** - the crane never cools down
- TheTunnel / Island / Light: **A CRACKED WHOOSHING HOLLOW BOULDER** - the spiral has always been this long
- WatchingWallpaper / Island / Light: **A NIMBLE LOANED VICTORIAN YELLOWED ISLET** - held together by habit
- Awakening / Stairs / Ruined: **A RUINED TOAST LADDER OF SECOND CHANCES** - barely holding itself together
- FractalCathedral / Cube / Clean: **A HOARDER'S FRAYED FRACTAL RELIQUARY** - the centre hides the centre
- WhiteDissolve / Octahedron / Heavy: **A HEAVILY-WORN LUSTROUS UNDERTOW JEWEL** - it splits the light into two
- CursedForest / Cloud / Heavy: **A HEAVILY-WORN COPSE FLEECE** - it follows a step behind. not yet
- Elfworks / Octahedron / Heavy: **A HEAVILY-WORN KALEIDOPHONE KITE** - it still mostly works
- Lobby / Cube / Light: **AN UNSHAKEN PITTED SATIN STILL RELIQUARY** - it always lands the same way up
- DrownedLibrary / Torus / Ruined: **A RUINED ATLAS LIFEBUOY OF CHANGED MINDS** - somebody else's, once
- AfterimageFields / Cloud / Light: **FORGIVING SPENT LUMINOUS CLUMP FROM SALT** - found at the bottom of a pocket
- LiminalOffice / Torus / Light: **A PARDONING SPLIT STAPLER BANGLE** - something is missing from the middle
- SkyStairs / Crescent / Light: **A PEACEFUL CRACKED BANISTER SCIMITAR** - it hums when held
- SynesthesiaHall / Octahedron / Heavy: **A HEAVILY-WORN OPALESCENT CHORD LOZENGE** - the speaker stack is playing too loud
- VoidPlatforms / Octahedron / Ruined: **A RUINED UNFINISHED BRIDGE TWIN PYRAMID** - better than nothing
- MirrorHall / Crescent / Heavy: **A HEAVILY-WORN STAUNCH DOPPELGANGER CLAW** - it hums when held
- MeltingClockworks / Torus / Heavy: **A HEAVILY-WORN IRIDESCENT PENDULUM RING** - the hole is deeper than the ring
- Garden / Orb / Light: **A SPARKLING BENT HUNGRY SUNLESS SEED** - found at the bottom of a pocket
- MyceliumGrove / Cloud / Heavy: **A HEAVILY-WORN IRIDESCENT STINKHORN NEST** - a spore is growing toward the shard
- JellyfishSky / Cube / Ruined: **RUINED DRIFTING BRICK FROM YOUR MOTHER** - barely holding itself together
- NightmareFactory / Island / Ruined: **RUINED TIRELESS REFINERY MESA OF TITHES** - held together by habit
- TheTunnel / Torus / Light: **A SCRATCHED ENDLESS WHOOSHING GASKET** - the hole is deeper than the ring. it knows
- WatchingWallpaper / Torus / Ruined: **A RUINED REDRAWN PAISLEY EYE COLLAR** - it fits a larger hand than yours
- Awakening / Cloud / Ruined: **A RUINED QUIET WARM DRIFT** - barely holding itself together
- FractalCathedral / Island / Ruined: **A RUINED KALEIDOSCOPIC ATLAS MOLAR** - it drifts when you sleep. it's okay to cry here
- WhiteDissolve / Crescent / Ruined: **RUINED STILLED EMPTY PALE HORIZON GRIN** - barely holding itself together
- CursedForest / Cube / Ruined: **A RUINED COPSE BLOCK** - barely holding itself together
- Elfworks / Crescent / Light: **A PEARLESCENT PITTED MOREL SCIMITAR** - the music box is building itself

## Samples: Worn (40)
- Lobby / Stairs / Heavy: **THE HEAVILY-WORN STILLED STILL PLINTH** - the edges have gone soft
- DrownedLibrary / Arch / Clean: **THE RIPPLING THRESHOLD OF OLD LETTERS** - it opens inward, sometimes. it's only paint
- AfterimageFields / Stairs / Clean: **A SPANGLED TRAILING STREAK STAIRCASE** - the pastel field won't finish fading
- LiminalOffice / Cone / Heavy: **THE HEAVILY-WORN SPARKLING BUZZING THORN** - the org chart is marked urgent
- SkyStairs / Arch / Clean: **THE IRIDESCENT SPIRALLING ARCH OF SHARDS** - a kite hums like a kite string
- SynesthesiaHall / Torus / Clean: **THE HUMMING HALO OF STEADY HANDS** - it knows your grip
- VoidPlatforms / Octahedron / Clean: **THE UNANCHORED VOID DIAMOND OF NERVE** - nicely broken in
- MirrorHall / Cone / Clean: **THE OPAL FUNNEL FROM THE LONG HALL** - worn smooth by someone
- MeltingClockworks / Cloud / Ruined: **THE RUINED RECURRING CLUMP OF SLOW TIDES** - the watchmaker counts backwards
- Garden / Octahedron / Heavy: **THE HEAVILY-WORN ARCHIVAL THORNED GEM** - the edges have gone soft
- MyceliumGrove / Crescent / Ruined: **THE RUINED VELVET-CAPPED LICHEN SCIMITAR** - familiar as an old coat
- JellyfishSky / Orb / Ruined: **THE RUINED DRIFTING MOON OF STEADY HANDS** - the updraft trails its lights
- NightmareFactory / Cone / Ruined: **THE RUINED CIRRUS TUSK OF PLENTY** - the crane never cools down
- TheTunnel / Island / Light: **THE SPIRAL BOULDER FROM THE LONG WAY** - the spiral has always been this long
- WatchingWallpaper / Island / Light: **THE NIMBLE VICTORIAN YELLOWED ISLET** - worn smooth by someone
- Awakening / Stairs / Ruined: **THE RUINED SAFE LADDER OF SECOND CHANCES** - barely holding itself together
- FractalCathedral / Cube / Clean: **THE SYMMETRIC GOLD KEYSTONE OF PLENTY** - the centre hides the centre
- WhiteDissolve / Octahedron / Heavy: **THE HEAVILY-WORN LUSTROUS NAMELESS JEWEL** - it splits the light into two
- CursedForest / Cloud / Heavy: **THE HEAVILY-WORN WHISPERING FLEECE** - it follows a step behind. not yet
- Elfworks / Octahedron / Heavy: **THE HEAVILY-WORN SAGGING OVERWOUND KITE** - nicely broken in
- Lobby / Cube / Light: **THE UNSHAKEN SATIN RELIQUARY FROM DUST** - it always lands the same way up
- DrownedLibrary / Torus / Ruined: **THE RUINED REDRAWN CATALOGUED HALO** - it knows your grip
- AfterimageFields / Cloud / Light: **THE FORGIVING LUMINOUS CLUMP FROM SALT** - it has been carried far
- LiminalOffice / Torus / Light: **THE PARDONING MANDATORY UNPAID BANGLE** - something is missing from the middle
- SkyStairs / Crescent / Light: **A PEACEFUL UPSIDE-DOWN BANISTER SCIMITAR** - it hums when held
- SynesthesiaHall / Octahedron / Heavy: **THE HEAVILY-WORN OPALESCENT LOUD CRYSTAL** - the speaker stack is playing too loud
- VoidPlatforms / Octahedron / Ruined: **THE RUINED UNFINISHED TWIN PYRAMID** - familiar as an old coat
- MirrorHall / Crescent / Heavy: **THE HEAVILY-WORN UNSHAKEN WELDED TALON** - it hums when held
- MeltingClockworks / Torus / Heavy: **THE HEAVILY-WORN IRIDESCENT CYCLIC LOOP** - the hole is deeper than the ring
- Garden / Orb / Light: **THE SPARKLING GARDEN SEED FROM ECHOES** - it has been carried far
- MyceliumGrove / Cloud / Heavy: **THE HEAVILY-WORN IRIDESCENT PALE NEST** - a spore is growing toward the shard
- JellyfishSky / Cube / Ruined: **A RUINED DRIFTING TENDRIL BRICK** - barely holding itself together
- NightmareFactory / Island / Ruined: **THE RUINED FEVERED GEODE OF GOOD HARVEST** - worn smooth by someone
- TheTunnel / Torus / Light: **THE PASSAGE GASKET FROM OLD PHOTOGRAPHS** - the hole is deeper than the ring. it knows
- WatchingWallpaper / Torus / Ruined: **THE RUINED REDRAWN REPEATING COLLAR** - it fits a larger hand than yours
- Awakening / Cloud / Ruined: **RUINED QUIET WARM DAYLIGHT DRIFT** - barely holding itself together
- FractalCathedral / Island / Ruined: **THE RUINED KALEIDOSCOPIC MOLAR** - it drifts when you sleep. it's okay to cry here
- WhiteDissolve / Crescent / Ruined: **THE RUINED PALE TALON OF QUIET BREATHS** - barely holding itself together
- CursedForest / Cube / Ruined: **THE RUINED TANGLED BLOCK** - barely holding itself together
- Elfworks / Crescent / Light: **THE PEARLESCENT RETENTIVE FIZZING SMILE** - the music box is building itself

## Samples: Pristine (40)
- Lobby / Stairs / Heavy: **THE HEAVILY-WORN PASTEL PLINTH OF PAPER** - not a mark on it. remember this
- DrownedLibrary / Arch / Clean: **MINDFUL RIPPLING THRESHOLD OF LOST TIME** - it opens inward, sometimes. it's only paint
- AfterimageFields / Stairs / Clean: **SPANGLED STREAK STAIRCASE OF CHILDHOOD** - the pastel field won't finish fading
- LiminalOffice / Cone / Heavy: **HEAVILY-WORN HOLLOW FUNNEL OF PALE BLUE** - the org chart is marked urgent
- SkyStairs / Arch / Clean: **IRIDESCENT TRACER WICKET OF THIN AIR** - a kite hums like a kite string
- SynesthesiaHall / Torus / Clean: **STEELY HUMMING HALO OF FALSE AWAKENINGS** - kept in the good drawer. shh
- VoidPlatforms / Octahedron / Clean: **STOUT UNANCHORED VOID DIAMOND OF NOON** - it still smells new. you knew that
- MirrorHall / Cone / Clean: **THE OPAL FUNNEL OF THE LONG HALL** - never once dropped. no one is coming
- MeltingClockworks / Cloud / Ruined: **RUINED HUSHED SLOW CLOUD OF FORGETTING** - the watchmaker counts backwards
- Garden / Octahedron / Heavy: **HEAVILY-WORN THORNED GEM OF TALL GRASS** - the edges have gone soft
- MyceliumGrove / Crescent / Ruined: **THE RUINED LICHEN SCIMITAR OF NO RETURN** - not a mark on it
- JellyfishSky / Orb / Ruined: **RUINED STEADY DRIFTING MOON OF REHEARSAL** - the updraft trails its lights
- NightmareFactory / Cone / Ruined: **RUINED TITHING CIRRUS TUSK OF SMOKE** - the crane never cools down
- TheTunnel / Island / Light: **THE SPIRAL BOULDER OF THE LONG WAY** - the spiral has always been this long
- WatchingWallpaper / Island / Light: **NIMBLE VICTORIAN ISLET OF BEING WATCHED** - never once dropped. it heard that
- Awakening / Stairs / Ruined: **RUINED DOUBTING TOAST LADDER OF WAKING** - it has been waiting for you. keep going
- FractalCathedral / Cube / Clean: **THE MINTED TRIFORIUM CRATE OF WET PAINT** - the centre hides the centre
- WhiteDissolve / Octahedron / Heavy: **THE HEAVILY-WORN LUSTROUS NAMELESS JEWEL** - it splits the light into two
- CursedForest / Cloud / Heavy: **HEAVILY-WORN COPSE FLEECE OF OLD STORIES** - it follows a step behind. not yet
- Elfworks / Octahedron / Heavy: **THE HEAVILY-WORN SAGGING KITE OF GLASS** - it still smells new
- Lobby / Cube / Light: **UNSHAKEN SATIN RELIQUARY OF DUST** - it always lands the same way up
- DrownedLibrary / Torus / Ruined: **RUINED REDRAWN ARCHIVE HALO OF PRESSURE** - kept in the good drawer
- AfterimageFields / Cloud / Light: **FORGIVING MOTION BLUR CLUMP OF SALT** - it has been waiting for you
- LiminalOffice / Torus / Light: **LENIENT REDACTED DOUGHNUT OF LEFTOVERS** - something is missing from the middle
- SkyStairs / Crescent / Light: **LULLING RAINBOW PARING OF SOFT TEETH** - it hums when held
- SynesthesiaHall / Octahedron / Heavy: **HEAVILY-WORN OPALESCENT SUSTAINED JEWEL** - the speaker stack is playing too loud
- VoidPlatforms / Octahedron / Ruined: **THE RUINED HOLLOW SPINDLE OF SALT** - not a mark on it
- MirrorHall / Crescent / Heavy: **HEAVILY-WORN TWINNED TALON OF SILVER** - it hums when held
- MeltingClockworks / Torus / Heavy: **HEAVILY-WORN IRIDESCENT PENDULUM RING** - the hole is deeper than the ring
- Garden / Orb / Light: **THE SPARKLING GARDEN SEED OF ECHOES** - it has been waiting for you
- MyceliumGrove / Cloud / Heavy: **HEAVILY-WORN IRIDESCENT NURSE LOG CLOUD** - a spore is growing toward the shard
- JellyfishSky / Cube / Ruined: **RUINED TENDRIL BRICK OF YOUR MOTHER** - barely holding itself together
- NightmareFactory / Island / Ruined: **THE RUINED FEVERED GEODE OF SCRAP METAL** - never once dropped
- TheTunnel / Torus / Light: **THE PASSAGE GASKET OF OLD PHOTOGRAPHS** - the hole is deeper than the ring. it knows
- WatchingWallpaper / Torus / Ruined: **RUINED PAISLEY EYE COLLAR OF LULLABIES** - it fits a larger hand than yours
- Awakening / Cloud / Ruined: **THE RUINED DAYLIGHT DRIFT OF FLICKERING** - barely holding itself together
- FractalCathedral / Island / Ruined: **RUINED ATLAS MOLAR OF HYMNS** - it drifts when you sleep. it's okay to cry here
- WhiteDissolve / Crescent / Ruined: **RUINED STILLED EMPTY GRIN OF THE SELF** - it has been waiting for you. keep walking
- CursedForest / Cube / Ruined: **RUINED COPSE BLOCK OF THE WITCHING HOUR** - never once dropped. it knows
- Elfworks / Crescent / Light: **PEARLESCENT MOREL SCIMITAR OF MISCHIEF** - the music box is building itself

## Samples: Mythic (40)
- Lobby / Stairs / Heavy: **THE HEAVILY-WORN PASTEL PLINTH OF PAPER** - the dream bends toward it. remember this
- DrownedLibrary / Arch / Clean: **SECOND THOUGHTS' RECALLING BLUE-LIT YOKE** - it opens inward, sometimes. it's only paint
- AfterimageFields / Stairs / Clean: **CHILDHOOD'S SPANGLED STREAK STAIRCASE** - the pastel field won't finish fading
- LiminalOffice / Cone / Heavy: **DUST'S HEAVILY-WORN PARKING GARAGE SPIRE** - the org chart is marked urgent
- SkyStairs / Arch / Clean: **THIN AIR'S IRIDESCENT TRACER WICKET** - a kite hums like a kite string
- SynesthesiaHall / Torus / Clean: **THE STEELY HALO THAT PAINTS THE FLOOR** - it was here before the dream. shh
- VoidPlatforms / Octahedron / Clean: **NOON'S UNANCHORED VOID DIAMOND OF NERVE** - it remembers every owner. you knew that
- MirrorHall / Cone / Clean: **THE LONG HALL'S REFLECTED OPAL FUNNEL** - it chose you, not the other way. no one is coming
- MeltingClockworks / Cloud / Ruined: **FORGETTING'S RUINED SLOW CLOUD OF HUSH** - the watchmaker counts backwards
- Garden / Octahedron / Heavy: **TALL GRASS' HEAVILY-WORN THORNED GEM** - the edges have gone soft
- MyceliumGrove / Crescent / Ruined: **NO RETURN'S RUINED LICHEN SCIMITAR** - breathe with it
- JellyfishSky / Orb / Ruined: **THE LONG HALL'S RUINED MAN O' WAR BEAD** - the updraft trails its lights
- NightmareFactory / Cone / Ruined: **THE RUINED TITHING CIRRUS TUSK OF SMOKE** - the crane never cools down
- TheTunnel / Island / Light: **THE LONG WAY'S WHOOSHING SPIRAL BOULDER** - the spiral has always been this long
- WatchingWallpaper / Island / Light: **THE NIMBLE ISLET THAT IS ONLY PAPER** - it chose you, not the other way. it heard that
- Awakening / Stairs / Ruined: **FIRST LIGHT'S RUINED RETRYING SLOW RUNGS** - nobody agrees where it came from. keep going
- FractalCathedral / Cube / Clean: **THE MINTED LAST CRATE BEFORE WET PAINT** - the centre hides the centre
- WhiteDissolve / Octahedron / Heavy: **THE HEAVILY-WORN LUSTROUS NAMELESS JEWEL** - it splits the light into two
- CursedForest / Cloud / Heavy: **OLD STORIES' HEAVILY-WORN COPSE FLEECE** - it follows a step behind. not yet
- Elfworks / Octahedron / Heavy: **THE HEAVILY-WORN MOTHER-OF-PEARL KITE** - you were expected
- Lobby / Cube / Light: **THE UNSHAKEN SATIN RELIQUARY OF DUST** - it always lands the same way up
- DrownedLibrary / Torus / Ruined: **PRESSURE'S RUINED REDRAWN ARCHIVE HALO** - every book is a little wet
- AfterimageFields / Cloud / Light: **THE FORGIVING LAST CLUMP BEFORE SALT** - the light is late
- LiminalOffice / Torus / Light: **THE MERCIFUL BANGLE THAT PRINTED YOU** - something is missing from the middle
- SkyStairs / Crescent / Light: **THE PEACEFUL SCIMITAR THAT HAS NO TOP** - it hums when held
- SynesthesiaHall / Octahedron / Heavy: **THE HEAVILY-WORN OPALESCENT JEWEL** - the speaker stack is playing too loud
- VoidPlatforms / Octahedron / Ruined: **SALT'S RUINED HOLLOW HORIZON SPINDLE** - gravity is optional here
- MirrorHall / Crescent / Heavy: **THE HEAVILY-WORN TALON THAT IS NOT YOU** - it hums when held
- MeltingClockworks / Torus / Heavy: **THE HEAVILY-WORN IRIDESCENT RING** - the hole is deeper than the ring
- Garden / Orb / Light: **ECHOES' SPARKLING HUNGRY GARDEN SEED** - nobody agrees where it came from
- MyceliumGrove / Cloud / Heavy: **GLASS' HEAVILY-WORN IRIDESCENT PALE NEST** - a spore is growing toward the shard
- JellyfishSky / Cube / Ruined: **YOUR MOTHER'S RUINED TENDRIL BRICK** - barely holding itself together
- NightmareFactory / Island / Ruined: **THE RUINED GEODE THAT IS OVER QUOTA** - your shift never started
- TheTunnel / Torus / Light: **OLD PHOTOGRAPHS' ENDLESS PASSAGE GASKET** - the hole is deeper than the ring. it knows
- WatchingWallpaper / Torus / Ruined: **LULLABIES' RUINED REPEATING COLLAR** - it fits a larger hand than yours
- Awakening / Cloud / Ruined: **FLICKERING'S RUINED QUIET DAYLIGHT DRIFT** - barely holding itself together
- FractalCathedral / Island / Ruined: **HYMNS' RUINED KALEIDOSCOPIC MOLAR** - it drifts when you sleep. it's okay to cry here
- WhiteDissolve / Crescent / Ruined: **THE SELF'S RUINED STILLED EMPTY GRIN** - nobody agrees where it came from. keep walking
- CursedForest / Cube / Ruined: **THE WITCHING HOUR'S RUINED TANGLED BLOCK** - it chose you, not the other way. it knows
- Elfworks / Crescent / Light: **MISCHIEF'S PEARLESCENT MOREL SCIMITAR** - the music box is building itself
