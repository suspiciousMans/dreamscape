//! Theme = the grammar of a dream. Every random choice the generator makes is
//! drawn from inside these bounds; that's what makes a dream feel like one place.

use super::meshes::Shape;
use super::texture::Pattern;
#[cfg(test)]
use crate::gameplay::CELL;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DreamTheme {
    Lobby,
    LiminalOffice,
    VoidPlatforms,
    Garden,
    NightmareFactory,
    Awakening,
    CursedForest,
    DrownedLibrary,
    SkyStairs,
    MirrorHall,
    MyceliumGrove,
    TheTunnel,
    FractalCathedral,
    Elfworks,
}

pub const ALL_THEMES: [DreamTheme; 14] = [
    DreamTheme::Lobby,
    DreamTheme::LiminalOffice,
    DreamTheme::VoidPlatforms,
    DreamTheme::Garden,
    DreamTheme::NightmareFactory,
    DreamTheme::Awakening,
    DreamTheme::CursedForest,
    DreamTheme::DrownedLibrary,
    DreamTheme::SkyStairs,
    DreamTheme::MirrorHall,
    DreamTheme::MyceliumGrove,
    DreamTheme::TheTunnel,
    DreamTheme::FractalCathedral,
    DreamTheme::Elfworks,
];

/// A dream's signature mechanic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Feature {
    None,
    /// Glowing veins along the way: follow them, and walk faster on them.
    Veins,
    /// Membranes across the corridor that open and close in sequence.
    Gates,
    /// Floor tiles on the way that sink and rise.
    Shifters,
}

/// How a dream *feels* beyond its colours: shader and post-effect levels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mood {
    /// Extra surface breathing, 0..=1 (on top of strangeness).
    pub breathe: f32,
    /// Tracers: how long moving things smear, 0..=0.9.
    pub trails: f32,
    /// Bright parts of textures glow in the dark, 0..=1.
    pub glow: f32,
    /// Beats per minute the dream pulses to (0 = no pulse).
    pub bpm: f32,
}

impl Mood {
    pub const PLAIN: Mood = Mood {
        breathe: 0.0,
        trails: 0.0,
        glow: 0.0,
        bpm: 0.0,
    };

    pub const fn new(breathe: f32, trails: f32, glow: f32, bpm: f32) -> Mood {
        Mood {
            breathe,
            trails,
            glow,
            bpm,
        }
    }

    /// 1 on the beat, falling off sharply until the next one.
    pub fn pulse(&self, time: f32) -> f32 {
        if self.bpm <= 0.0 {
            return 0.0;
        }
        let phase = (time * self.bpm / 60.0).fract();
        (1.0 - phase).powi(4)
    }
}

/// Special enemies (on top of the pacers every dream has).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum EnemyKind {
    /// Only moves while you can't see it.
    Stalker,
    /// Walks your own path, three seconds behind you.
    Mimic,
    /// Sweeps a beam; being seen calls the pacers.
    Sentry,
    /// Floats across the gaps you have to jump.
    Drifter,
    /// Harmless, but its touch throws you somewhere else.
    Jester,
}

pub const ALL_ENEMY_KINDS: [EnemyKind; 5] = [
    EnemyKind::Stalker,
    EnemyKind::Mimic,
    EnemyKind::Sentry,
    EnemyKind::Drifter,
    EnemyKind::Jester,
];

impl EnemyKind {
    /// Shown the first time each kind turns up in a run.
    pub fn hint(self) -> &'static str {
        match self {
            EnemyKind::Stalker => "something only moves when you're not looking",
            EnemyKind::Mimic => "something is walking in your footsteps. keep moving.",
            EnemyKind::Sentry => "a light is searching for you. it will call the others.",
            EnemyKind::Drifter => "something drifts across the gaps. time your jumps.",
            EnemyKind::Jester => "a jester. it won't hurt you, but it will move you.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutKind {
    OpenHall,
    Maze,
    PlatformChain,
    ScatterField,
    /// A square spiral of walkways winding in over the void.
    Spiral,
    /// A hall whose left half is reflected onto its right.
    Mirrored,
    /// Clearings over the void joined by root bridges.
    Network,
    /// One long winding corridor with side alcoves.
    Corridor,
    /// Square rooms nested inside each other, a door in each.
    Recursive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    Desk,
    Partition,
    Pillar,
    Tree,
    Hedge,
    Machine,
    Pipe,
    Bench,
    Lamp,
    Crystal,
    /// A doorframe standing on its own, leading nowhere.
    Doorway,
    /// Three steps up to nothing, with one more hanging above.
    FloatingStairs,
    CloudPuff,
    Moon,
    /// A hoop standing on its edge.
    RingGate,
    Bookshelf,
    Mushroom,
}

pub const ALL_PROPS: [PropKind; 17] = [
    PropKind::Desk,
    PropKind::Partition,
    PropKind::Pillar,
    PropKind::Tree,
    PropKind::Hedge,
    PropKind::Machine,
    PropKind::Pipe,
    PropKind::Bench,
    PropKind::Lamp,
    PropKind::Crystal,
    PropKind::Doorway,
    PropKind::FloatingStairs,
    PropKind::CloudPuff,
    PropKind::Moon,
    PropKind::RingGate,
    PropKind::Bookshelf,
    PropKind::Mushroom,
];

impl PropKind {
    /// Parts as (centre offset from the cell centre on the floor, size, shape).
    /// Every shape fills its size box (see `meshes`), so the AABB collider matches.
    pub fn parts(self) -> &'static [([f32; 3], [f32; 3], Shape)] {
        use Shape::*;
        match self {
            PropKind::Desk => &[
                ([0.0, 0.72, 0.0], [1.6, 0.12, 0.8], Cube),
                ([-0.65, 0.33, 0.0], [0.12, 0.66, 0.12], Cylinder),
                ([0.65, 0.33, 0.0], [0.12, 0.66, 0.12], Cylinder),
            ],
            PropKind::Partition => &[([0.0, 0.75, 0.0], [0.15, 1.5, 2.0], Cube)],
            PropKind::Pillar => &[([0.0, 1.5, 0.0], [0.8, 3.0, 0.8], Cylinder)],
            PropKind::Tree => &[
                ([0.0, 1.0, 0.0], [0.4, 2.0, 0.4], Cylinder),
                ([0.0, 2.6, 0.0], [1.8, 1.6, 1.8], Orb),
            ],
            PropKind::Hedge => &[([0.0, 0.6, 0.0], [2.0, 1.2, 1.0], Cube)],
            PropKind::Machine => &[
                ([0.0, 1.0, 0.0], [2.0, 2.0, 1.6], Cube),
                ([0.6, 2.4, 0.0], [0.4, 0.8, 0.4], Cylinder),
                ([-0.5, 2.25, 0.0], [0.6, 0.5, 0.6], Cone),
            ],
            PropKind::Pipe => &[([0.0, 1.5, 0.0], [0.3, 3.0, 0.3], Cylinder)],
            PropKind::Bench => &[([0.0, 0.25, 0.0], [1.8, 0.5, 0.6], Cube)],
            PropKind::Lamp => &[
                ([0.0, 1.2, 0.0], [0.15, 2.4, 0.15], Cylinder),
                ([0.0, 2.45, 0.0], [0.7, 0.4, 0.7], Cone),
            ],
            PropKind::Crystal => &[([0.0, 0.9, 0.0], [0.9, 1.8, 0.9], Octahedron)],
            PropKind::Doorway => &[([0.0, 1.3, 0.0], [2.0, 2.6, 0.4], Arch)],
            PropKind::FloatingStairs => &[
                ([0.0, 0.6, 0.0], [1.6, 1.2, 1.6], Stairs),
                ([0.0, 2.1, 0.5], [1.6, 0.25, 0.5], Cube),
            ],
            PropKind::CloudPuff => &[([0.0, 1.5, 0.0], [1.9, 1.1, 1.5], Cloud)],
            PropKind::Moon => &[
                ([0.0, 0.15, 0.0], [0.8, 0.3, 0.8], Cylinder),
                ([0.0, 1.4, 0.0], [1.8, 2.2, 0.4], Crescent),
            ],
            PropKind::RingGate => &[([0.0, 1.2, 0.0], [2.0, 2.4, 0.3], Torus)],
            PropKind::Bookshelf => &[
                ([0.0, 1.25, 0.0], [2.0, 2.5, 0.6], Cube),
                ([0.0, 2.65, 0.0], [2.1, 0.3, 0.7], Cube),
            ],
            PropKind::Mushroom => &[
                ([0.0, 0.5, 0.0], [0.35, 1.0, 0.35], Cylinder),
                ([0.0, 1.25, 0.0], [1.7, 0.7, 1.7], Orb),
            ],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ThemeSpec {
    pub layout: LayoutKind,
    /// Grid side length range (inclusive).
    pub grid_size: (i32, i32),
    pub wall_height: f32,
    pub floor_colors: &'static [[u8; 3]],
    pub wall_colors: &'static [[u8; 3]],
    pub prop_colors: &'static [[u8; 3]],
    pub props: &'static [PropKind],
    /// Chance a free floor cell gets a prop.
    pub prop_density: f32,
    /// 0 = orderly, 1 = unhinged: colour spread, prop scale spread, floating debris.
    pub strangeness: f32,
    pub fog_color: [f32; 3],
    pub ambient: [f32; 3],
    pub fog_start: f32,
    pub fog_end: f32,
    /// `name` of a profile in games/dreamscape/profiles/ (resolution/snap/affine feel).
    pub base_profile: &'static str,
    pub music: &'static str,
    /// Enemy count range at depth 0 (grows by 1 every 3 dreams).
    pub enemies: (u32, u32),
    /// Weighted exits. Lobby and Awakening are never listed — the director owns them.
    pub next: &'static [(DreamTheme, u32)],
    /// Texture patterns this dream draws from (floor/wall/prop each pick one).
    pub patterns: &'static [Pattern],
    /// Vivid colours that bleed into the textures once the dream gets strange.
    pub accents: &'static [[u8; 3]],
    /// Special enemies this dream can hold, weighted.
    pub specials: &'static [(EnemyKind, u32)],
    /// Slow fog pockets drift through this dream.
    pub fog_pockets: bool,
    pub mood: Mood,
    pub feature: Feature,
}

impl DreamTheme {
    pub fn spec(self) -> ThemeSpec {
        use DreamTheme::*;
        match self {
            Lobby => ThemeSpec {
                layout: LayoutKind::OpenHall,
                grid_size: (11, 13),
                wall_height: 3.0,
                floor_colors: &[[225, 215, 235], [210, 205, 230]],
                wall_colors: &[[240, 235, 245]],
                prop_colors: &[[180, 170, 210], [250, 240, 200]],
                props: &[
                    PropKind::Bench,
                    PropKind::Pillar,
                    PropKind::Lamp,
                    PropKind::Doorway,
                    PropKind::CloudPuff,
                ],
                prop_density: 0.12,
                strangeness: 0.05,
                fog_color: [0.7, 0.7, 0.9],
                ambient: [0.45, 0.45, 0.55],
                fog_start: 10.0,
                fog_end: 50.0,
                base_profile: "dream_lobby",
                music: "games/dreamscape/assets/music/dream_lobby.wav",
                enemies: (0, 0),
                next: &[
                    (LiminalOffice, 3),
                    (Garden, 2),
                    (VoidPlatforms, 1),
                    (CursedForest, 1),
                    (MirrorHall, 1),
                    (SkyStairs, 1),
                    (MyceliumGrove, 1),
                    (Elfworks, 1),
                ],
                patterns: &[Pattern::Plasma, Pattern::Rings, Pattern::Kaleido],
                accents: &[[255, 150, 220], [150, 200, 255]],
                specials: &[],
                fog_pockets: false,
                mood: Mood::PLAIN,
                feature: Feature::None,
            },
            LiminalOffice => ThemeSpec {
                layout: LayoutKind::Maze,
                grid_size: (15, 19),
                wall_height: 3.0,
                floor_colors: &[[196, 184, 120], [184, 172, 112]],
                wall_colors: &[[222, 214, 168], [210, 204, 160]],
                prop_colors: &[[150, 150, 140], [120, 110, 90], [240, 240, 210]],
                props: &[
                    PropKind::Desk,
                    PropKind::Partition,
                    PropKind::Lamp,
                    PropKind::Doorway,
                ],
                prop_density: 0.25,
                strangeness: 0.15,
                fog_color: [0.55, 0.55, 0.4],
                ambient: [0.4, 0.4, 0.3],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "dreamscape_liminal",
                music: "games/dreamscape/assets/music/liminal_office.wav",
                enemies: (1, 2),
                next: &[
                    (VoidPlatforms, 2),
                    (NightmareFactory, 2),
                    (Garden, 1),
                    (DrownedLibrary, 2),
                    (MirrorHall, 1),
                    (TheTunnel, 1),
                ],
                patterns: &[Pattern::Stripes, Pattern::Checker],
                accents: &[[210, 255, 60], [255, 220, 40]],
                specials: &[(EnemyKind::Stalker, 2), (EnemyKind::Sentry, 1)],
                fog_pockets: false,
                mood: Mood::new(0.15, 0.0, 0.0, 0.0),
                feature: Feature::None,
            },
            VoidPlatforms => ThemeSpec {
                layout: LayoutKind::PlatformChain,
                grid_size: (15, 19),
                wall_height: 0.0,
                floor_colors: &[[140, 160, 200], [110, 120, 170], [170, 150, 210]],
                wall_colors: &[[90, 90, 120]],
                prop_colors: &[[200, 200, 255], [60, 60, 90]],
                props: &[
                    PropKind::Pillar,
                    PropKind::Lamp,
                    PropKind::Crystal,
                    PropKind::FloatingStairs,
                    PropKind::RingGate,
                ],
                prop_density: 0.08,
                strangeness: 0.6,
                fog_color: [0.08, 0.06, 0.15],
                ambient: [0.25, 0.25, 0.4],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "shattered_realm",
                music: "games/dreamscape/assets/music/void_platform.wav",
                enemies: (0, 1),
                next: &[
                    (Garden, 2),
                    (NightmareFactory, 2),
                    (LiminalOffice, 1),
                    (SkyStairs, 2),
                    (TheTunnel, 1),
                ],
                patterns: &[Pattern::Swirl, Pattern::Plasma, Pattern::Kaleido],
                accents: &[[255, 40, 220], [40, 240, 255]],
                specials: &[(EnemyKind::Drifter, 1)],
                fog_pockets: false,
                mood: Mood::new(0.0, 0.35, 0.2, 0.0),
                feature: Feature::None,
            },
            Garden => ThemeSpec {
                layout: LayoutKind::ScatterField,
                grid_size: (13, 17),
                wall_height: 1.2,
                floor_colors: &[[110, 170, 90], [120, 180, 100]],
                wall_colors: &[[60, 120, 60], [70, 130, 70]],
                prop_colors: &[[90, 60, 40], [80, 150, 80], [230, 180, 210]],
                props: &[
                    PropKind::Tree,
                    PropKind::Hedge,
                    PropKind::Bench,
                    PropKind::Mushroom,
                    PropKind::Moon,
                ],
                prop_density: 0.3,
                strangeness: 0.25,
                fog_color: [0.6, 0.75, 0.6],
                ambient: [0.4, 0.5, 0.4],
                fog_start: 8.0,
                fog_end: 42.0,
                base_profile: "ethereal_sanctuary",
                music: "games/dreamscape/assets/music/dream_lobby.wav",
                enemies: (1, 2),
                next: &[
                    (LiminalOffice, 2),
                    (VoidPlatforms, 1),
                    (NightmareFactory, 2),
                    (CursedForest, 2),
                    (SkyStairs, 1),
                    (MyceliumGrove, 1),
                ],
                patterns: &[
                    Pattern::Cells,
                    Pattern::Plasma,
                    Pattern::Rings,
                    Pattern::Kaleido,
                ],
                accents: &[[255, 90, 200], [180, 255, 40]],
                specials: &[(EnemyKind::Jester, 1)],
                fog_pockets: true,
                mood: Mood::new(0.4, 0.0, 0.0, 0.0),
                feature: Feature::None,
            },
            NightmareFactory => ThemeSpec {
                layout: LayoutKind::Maze,
                grid_size: (15, 19),
                wall_height: 3.0,
                floor_colors: &[[70, 60, 55], [80, 70, 60]],
                wall_colors: &[[120, 60, 40], [100, 50, 40]],
                prop_colors: &[[140, 130, 120], [160, 80, 40], [60, 60, 60]],
                props: &[PropKind::Machine, PropKind::Pipe],
                prop_density: 0.35,
                strangeness: 0.5,
                fog_color: [0.2, 0.08, 0.08],
                ambient: [0.3, 0.12, 0.1],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "nightmare_factory",
                music: "games/dreamscape/assets/music/nightmare_factory.wav",
                enemies: (3, 4),
                next: &[
                    (VoidPlatforms, 1),
                    (LiminalOffice, 2),
                    (Garden, 1),
                    (CursedForest, 1),
                    (DrownedLibrary, 1),
                    (Elfworks, 1),
                ],
                patterns: &[
                    Pattern::Checker,
                    Pattern::Stripes,
                    Pattern::Cells,
                    Pattern::Eyes,
                ],
                accents: &[[255, 120, 0], [255, 20, 60]],
                specials: &[(EnemyKind::Sentry, 2), (EnemyKind::Mimic, 1)],
                fog_pockets: false,
                mood: Mood::new(0.1, 0.0, 0.1, 96.0),
                feature: Feature::None,
            },
            Awakening => ThemeSpec {
                layout: LayoutKind::OpenHall,
                grid_size: (9, 9),
                wall_height: 3.0,
                floor_colors: &[[240, 220, 180]],
                wall_colors: &[[250, 235, 200]],
                prop_colors: &[[255, 210, 150], [230, 200, 160]],
                props: &[PropKind::Bench, PropKind::Lamp, PropKind::CloudPuff],
                prop_density: 0.1,
                strangeness: 0.0,
                fog_color: [0.95, 0.85, 0.7],
                ambient: [0.6, 0.55, 0.45],
                fog_start: 10.0,
                fog_end: 50.0,
                base_profile: "desert_mirage",
                music: "games/dreamscape/assets/music/awakening.wav",
                enemies: (0, 0),
                next: &[],
                patterns: &[Pattern::Plasma],
                accents: &[[255, 240, 200]],
                specials: &[],
                fog_pockets: false,
                mood: Mood::PLAIN,
                feature: Feature::None,
            },
            CursedForest => ThemeSpec {
                layout: LayoutKind::ScatterField,
                grid_size: (13, 17),
                wall_height: 2.0,
                floor_colors: &[[44, 66, 42], [52, 74, 46]],
                wall_colors: &[[34, 50, 32], [58, 44, 34]],
                prop_colors: &[[70, 50, 36], [60, 110, 60], [200, 190, 120]],
                props: &[
                    PropKind::Tree,
                    PropKind::Mushroom,
                    PropKind::Hedge,
                    PropKind::Lamp,
                ],
                prop_density: 0.32,
                strangeness: 0.35,
                fog_color: [0.1, 0.2, 0.15],
                ambient: [0.28, 0.38, 0.3],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "cursed_forest",
                music: "games/dreamscape/assets/music/nightmare_factory.wav",
                enemies: (2, 3),
                next: &[
                    (Garden, 2),
                    (DrownedLibrary, 1),
                    (NightmareFactory, 1),
                    (MirrorHall, 1),
                    (MyceliumGrove, 2),
                ],
                patterns: &[Pattern::Cells, Pattern::Eyes, Pattern::Swirl],
                accents: &[[120, 255, 90], [255, 60, 200]],
                specials: &[(EnemyKind::Stalker, 2), (EnemyKind::Jester, 1)],
                fog_pockets: true,
                mood: Mood::new(0.5, 0.0, 0.25, 0.0),
                feature: Feature::None,
            },
            DrownedLibrary => ThemeSpec {
                layout: LayoutKind::Maze,
                grid_size: (15, 19),
                wall_height: 3.0,
                floor_colors: &[[44, 74, 112], [54, 84, 124]],
                wall_colors: &[[96, 64, 44], [76, 54, 42]],
                prop_colors: &[[120, 84, 56], [70, 140, 170], [230, 220, 190]],
                props: &[PropKind::Bookshelf, PropKind::Lamp, PropKind::Desk],
                prop_density: 0.25,
                strangeness: 0.3,
                fog_color: [0.12, 0.25, 0.4],
                ambient: [0.28, 0.38, 0.52],
                fog_start: 8.0,
                fog_end: 32.0,
                base_profile: "ocean_depths",
                music: "games/dreamscape/assets/music/liminal_office.wav",
                enemies: (1, 3),
                next: &[
                    (LiminalOffice, 2),
                    (MirrorHall, 1),
                    (CursedForest, 1),
                    (SkyStairs, 1),
                    (FractalCathedral, 1),
                ],
                patterns: &[Pattern::Stripes, Pattern::Rings, Pattern::Plasma],
                accents: &[[80, 220, 255], [180, 120, 255]],
                specials: &[(EnemyKind::Sentry, 1), (EnemyKind::Mimic, 1)],
                fog_pockets: true,
                mood: Mood::new(0.2, 0.25, 0.0, 0.0),
                feature: Feature::None,
            },
            SkyStairs => ThemeSpec {
                layout: LayoutKind::Spiral,
                grid_size: (11, 15),
                wall_height: 0.0,
                floor_colors: &[[240, 240, 255], [220, 230, 255], [255, 230, 240]],
                wall_colors: &[[200, 210, 240]],
                prop_colors: &[[255, 255, 255], [255, 220, 170], [190, 210, 255]],
                props: &[
                    PropKind::FloatingStairs,
                    PropKind::CloudPuff,
                    PropKind::RingGate,
                ],
                prop_density: 0.2,
                strangeness: 0.45,
                fog_color: [0.55, 0.7, 0.95],
                ambient: [0.55, 0.6, 0.7],
                fog_start: 10.0,
                fog_end: 45.0,
                base_profile: "sky_stairs",
                music: "games/dreamscape/assets/music/void_platform.wav",
                enemies: (0, 2),
                next: &[(VoidPlatforms, 1), (MirrorHall, 2), (Garden, 1)],
                patterns: &[Pattern::Plasma, Pattern::Swirl, Pattern::Rings],
                accents: &[[255, 200, 120], [140, 220, 255]],
                specials: &[(EnemyKind::Drifter, 1)],
                fog_pockets: false,
                mood: Mood::new(0.0, 0.3, 0.0, 0.0),
                feature: Feature::None,
            },
            MirrorHall => ThemeSpec {
                layout: LayoutKind::Mirrored,
                grid_size: (13, 17),
                wall_height: 3.0,
                floor_colors: &[[230, 225, 245], [210, 215, 240]],
                wall_colors: &[[200, 210, 230], [235, 235, 250]],
                prop_colors: &[[250, 250, 255], [200, 190, 240], [180, 240, 240]],
                props: &[
                    PropKind::Doorway,
                    PropKind::Crystal,
                    PropKind::Pillar,
                    PropKind::RingGate,
                ],
                prop_density: 0.18,
                strangeness: 0.4,
                fog_color: [0.75, 0.72, 0.9],
                ambient: [0.5, 0.5, 0.6],
                fog_start: 10.0,
                fog_end: 45.0,
                base_profile: "mirror_hall",
                music: "games/dreamscape/assets/music/liminal_office.wav",
                enemies: (1, 3),
                next: &[
                    (DrownedLibrary, 1),
                    (SkyStairs, 1),
                    (LiminalOffice, 1),
                    (NightmareFactory, 1),
                    (FractalCathedral, 1),
                ],
                patterns: &[Pattern::Kaleido, Pattern::Checker, Pattern::Rings],
                accents: &[[255, 160, 255], [120, 255, 240]],
                specials: &[(EnemyKind::Mimic, 2), (EnemyKind::Stalker, 1)],
                fog_pockets: false,
                mood: Mood::new(0.0, 0.3, 0.1, 0.0),
                feature: Feature::None,
            },
            MyceliumGrove => ThemeSpec {
                layout: LayoutKind::Network,
                grid_size: (15, 19),
                wall_height: 0.0,
                floor_colors: &[[42, 30, 52], [52, 36, 46]],
                wall_colors: &[[30, 24, 40]],
                prop_colors: &[[200, 170, 230], [90, 220, 170], [255, 150, 220]],
                props: &[PropKind::Mushroom, PropKind::Tree, PropKind::CloudPuff],
                prop_density: 0.25,
                strangeness: 0.4,
                fog_color: [0.08, 0.12, 0.1],
                ambient: [0.32, 0.3, 0.36],
                fog_start: 8.0,
                fog_end: 32.0,
                base_profile: "mycelium_grove",
                music: "games/dreamscape/assets/music/dream_lobby.wav",
                enemies: (1, 2),
                next: &[
                    (Garden, 2),
                    (CursedForest, 2),
                    (TheTunnel, 1),
                    (SkyStairs, 1),
                ],
                patterns: &[Pattern::Veins, Pattern::Cells, Pattern::Lattice],
                accents: &[[120, 255, 200], [255, 120, 240]],
                specials: &[(EnemyKind::Jester, 2), (EnemyKind::Drifter, 1)],
                fog_pockets: true,
                mood: Mood::new(0.6, 0.1, 0.8, 0.0),
                feature: Feature::Veins,
            },
            TheTunnel => ThemeSpec {
                layout: LayoutKind::Corridor,
                grid_size: (13, 17),
                wall_height: 3.0,
                floor_colors: &[[62, 42, 92], [72, 52, 102]],
                wall_colors: &[[122, 82, 182], [92, 62, 162]],
                prop_colors: &[[255, 240, 200], [180, 140, 255]],
                props: &[PropKind::RingGate, PropKind::Lamp],
                prop_density: 0.3,
                strangeness: 0.45,
                fog_color: [0.1, 0.05, 0.2],
                ambient: [0.36, 0.3, 0.46],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "the_tunnel",
                music: "games/dreamscape/assets/music/void_platform.wav",
                enemies: (1, 3),
                next: &[(FractalCathedral, 2), (MirrorHall, 1), (VoidPlatforms, 1)],
                patterns: &[Pattern::Tunnel, Pattern::Rings, Pattern::Swirl],
                accents: &[[255, 255, 200], [180, 120, 255]],
                specials: &[(EnemyKind::Mimic, 2), (EnemyKind::Stalker, 1)],
                fog_pockets: false,
                mood: Mood::new(0.2, 0.45, 0.3, 0.0),
                feature: Feature::Gates,
            },
            FractalCathedral => ThemeSpec {
                layout: LayoutKind::Recursive,
                grid_size: (15, 19),
                wall_height: 3.0,
                floor_colors: &[[200, 180, 240], [180, 200, 240]],
                wall_colors: &[[240, 222, 255], [212, 192, 250]],
                prop_colors: &[[255, 220, 120], [160, 220, 255], [255, 160, 220]],
                props: &[PropKind::Crystal, PropKind::Pillar, PropKind::Doorway],
                prop_density: 0.2,
                strangeness: 0.5,
                fog_color: [0.5, 0.45, 0.7],
                ambient: [0.5, 0.46, 0.6],
                fog_start: 10.0,
                fog_end: 45.0,
                base_profile: "fractal_cathedral",
                music: "games/dreamscape/assets/music/liminal_office.wav",
                enemies: (1, 3),
                next: &[
                    (Elfworks, 2),
                    (MirrorHall, 1),
                    (TheTunnel, 1),
                    (DrownedLibrary, 1),
                ],
                patterns: &[
                    Pattern::Mandala,
                    Pattern::Lattice,
                    Pattern::Kaleido,
                    Pattern::Cobweb,
                ],
                accents: &[[255, 200, 80], [120, 220, 255], [255, 120, 200]],
                specials: &[(EnemyKind::Sentry, 2), (EnemyKind::Stalker, 1)],
                fog_pockets: false,
                mood: Mood::new(0.15, 0.15, 0.35, 72.0),
                feature: Feature::None,
            },
            Elfworks => ThemeSpec {
                layout: LayoutKind::ScatterField,
                grid_size: (13, 17),
                wall_height: 1.2,
                floor_colors: &[[40, 200, 160], [255, 120, 200]],
                wall_colors: &[[255, 220, 60], [120, 80, 255]],
                prop_colors: &[[255, 230, 90], [90, 255, 230], [255, 90, 230]],
                props: &[
                    PropKind::Machine,
                    PropKind::Crystal,
                    PropKind::RingGate,
                    PropKind::Mushroom,
                ],
                prop_density: 0.3,
                strangeness: 0.6,
                fog_color: [0.4, 0.2, 0.5],
                ambient: [0.5, 0.42, 0.56],
                fog_start: 8.0,
                fog_end: 40.0,
                base_profile: "elfworks",
                music: "games/dreamscape/assets/music/nightmare_factory.wav",
                enemies: (1, 2),
                next: &[
                    (FractalCathedral, 1),
                    (MyceliumGrove, 1),
                    (Garden, 1),
                    (NightmareFactory, 1),
                ],
                patterns: &[
                    Pattern::Kaleido,
                    Pattern::Faces,
                    Pattern::Mandala,
                    Pattern::Checker,
                ],
                accents: &[[255, 255, 80], [80, 255, 255], [255, 80, 255]],
                specials: &[(EnemyKind::Jester, 3), (EnemyKind::Sentry, 1)],
                fog_pockets: false,
                mood: Mood::new(0.35, 0.25, 0.3, 128.0),
                feature: Feature::Shifters,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn repo_path(rel: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(rel)
    }

    #[test]
    fn moods_are_in_range_and_pulses_beat() {
        for t in ALL_THEMES {
            let m = t.spec().mood;
            assert!((0.0..=1.0).contains(&m.breathe) && (0.0..=0.9).contains(&m.trails));
            assert!((0.0..=1.0).contains(&m.glow) && m.bpm >= 0.0, "{t:?}");
        }
        let m = Mood::new(0.0, 0.0, 0.0, 120.0);
        assert!((m.pulse(0.0) - 1.0).abs() < 1e-5);
        assert!(m.pulse(0.25) < 0.1);
        assert!((m.pulse(0.5) - 1.0).abs() < 1e-4, "on the next beat");
        assert_eq!(Mood::PLAIN.pulse(3.3), 0.0);
    }

    #[test]
    fn dream_graph_is_well_formed() {
        for t in ALL_THEMES {
            let next = t.spec().next;
            if t == DreamTheme::Awakening {
                assert!(next.is_empty());
                continue;
            }
            assert!(!next.is_empty(), "{t:?} is a dead end");
            for &(n, w) in next {
                assert!(w > 0, "{t:?}->{n:?} has weight 0");
                assert_ne!(n, t, "{t:?} loops to itself");
                assert!(
                    n != DreamTheme::Awakening && n != DreamTheme::Lobby,
                    "{t:?} lists {n:?}"
                );
            }
        }
    }

    #[test]
    fn specs_are_sane() {
        for t in ALL_THEMES {
            let s = t.spec();
            assert!(
                !s.floor_colors.is_empty()
                    && !s.wall_colors.is_empty()
                    && !s.prop_colors.is_empty(),
                "{t:?}"
            );
            assert!(!s.props.is_empty(), "{t:?} has no props");
            assert!(
                !s.patterns.is_empty() && !s.accents.is_empty(),
                "{t:?} has no patterns/accents"
            );
            assert!(
                (0.0..=1.0).contains(&s.strangeness) && (0.0..=1.0).contains(&s.prop_density),
                "{t:?}"
            );
            assert!(s.enemies.0 <= s.enemies.1, "{t:?}");
            assert!(
                s.grid_size.0 >= 9 && s.grid_size.0 <= s.grid_size.1,
                "{t:?}"
            );
            assert!(
                s.wall_height <= 3.0,
                "{t:?}: walls over 3 hide the player from the camera"
            );
        }
    }

    #[test]
    fn fog_is_readable_at_camera_distance() {
        for t in ALL_THEMES {
            let s = t.spec();
            assert!(
                s.fog_start >= 8.0 && s.fog_end >= 30.0,
                "{t:?}: fog {}..{}",
                s.fog_start,
                s.fog_end
            );
        }
    }

    #[test]
    fn props_fit_inside_one_cell_even_when_scaled_up_30_percent() {
        for kind in ALL_PROPS {
            for &(off, size, _) in kind.parts() {
                for axis in [0, 2] {
                    let reach = (off[axis].abs() + size[axis] * 0.5) * 1.3;
                    assert!(
                        reach <= CELL * 0.475,
                        "{kind:?} reaches {reach} on axis {axis}"
                    );
                }
            }
        }
    }

    #[test]
    fn every_base_profile_exists() {
        let profiles = engine::profile::load_dir(&repo_path("games/dreamscape/profiles")).unwrap();
        for t in ALL_THEMES {
            let name = t.spec().base_profile;
            assert!(
                crate::gameplay::profile_index(&profiles, name).is_some(),
                "{t:?}: no profile '{name}'"
            );
        }
    }

    #[test]
    fn every_music_file_exists() {
        for t in ALL_THEMES {
            assert!(
                repo_path(t.spec().music).exists(),
                "{t:?}: missing {}",
                t.spec().music
            );
        }
    }
}
