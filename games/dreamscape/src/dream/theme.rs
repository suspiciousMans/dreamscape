//! Theme = the grammar of a dream. Every random choice the generator makes is
//! drawn from inside these bounds; that's what makes a dream feel like one place.

use super::texture::Pattern;
#[cfg(test)]
use crate::gameplay::CELL;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DreamTheme {
    Lobby,
    LiminalOffice,
    VoidPlatforms,
    Garden,
    NightmareFactory,
    Awakening,
}

#[cfg(test)]
pub const ALL_THEMES: [DreamTheme; 6] = [
    DreamTheme::Lobby,
    DreamTheme::LiminalOffice,
    DreamTheme::VoidPlatforms,
    DreamTheme::Garden,
    DreamTheme::NightmareFactory,
    DreamTheme::Awakening,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutKind {
    OpenHall,
    Maze,
    PlatformChain,
    ScatterField,
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
}

#[cfg(test)]
pub const ALL_PROPS: [PropKind; 9] = [
    PropKind::Desk,
    PropKind::Partition,
    PropKind::Pillar,
    PropKind::Tree,
    PropKind::Hedge,
    PropKind::Machine,
    PropKind::Pipe,
    PropKind::Bench,
    PropKind::Lamp,
];

impl PropKind {
    /// Cube parts as (centre offset from the cell centre on the floor, size).
    pub fn parts(self) -> &'static [([f32; 3], [f32; 3])] {
        match self {
            PropKind::Desk => &[([0.0, 0.4, 0.0], [1.6, 0.8, 0.8])],
            PropKind::Partition => &[([0.0, 0.75, 0.0], [0.15, 1.5, 2.0])],
            PropKind::Pillar => &[([0.0, 1.5, 0.0], [0.8, 3.0, 0.8])],
            PropKind::Tree => &[
                ([0.0, 1.0, 0.0], [0.4, 2.0, 0.4]),
                ([0.0, 2.6, 0.0], [1.8, 1.4, 1.8]),
            ],
            PropKind::Hedge => &[([0.0, 0.6, 0.0], [2.0, 1.2, 1.0])],
            PropKind::Machine => &[
                ([0.0, 1.0, 0.0], [2.0, 2.0, 1.6]),
                ([0.6, 2.4, 0.0], [0.4, 0.8, 0.4]),
            ],
            PropKind::Pipe => &[([0.0, 1.5, 0.0], [0.3, 3.0, 0.3])],
            PropKind::Bench => &[([0.0, 0.25, 0.0], [1.8, 0.5, 0.6])],
            PropKind::Lamp => &[
                ([0.0, 1.2, 0.0], [0.15, 2.4, 0.15]),
                ([0.0, 2.5, 0.0], [0.6, 0.3, 0.6]),
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
}

impl DreamTheme {
    pub fn spec(self) -> ThemeSpec {
        use DreamTheme::*;
        match self {
            Lobby => ThemeSpec {
                layout: LayoutKind::OpenHall,
                grid_size: (9, 11),
                wall_height: 3.0,
                floor_colors: &[[225, 215, 235], [210, 205, 230]],
                wall_colors: &[[240, 235, 245]],
                prop_colors: &[[180, 170, 210], [250, 240, 200]],
                props: &[PropKind::Bench, PropKind::Pillar, PropKind::Lamp],
                prop_density: 0.12,
                strangeness: 0.05,
                fog_color: [0.7, 0.7, 0.9],
                ambient: [0.45, 0.45, 0.55],
                fog_start: 10.0,
                fog_end: 50.0,
                base_profile: "dream_lobby",
                music: "games/dreamscape/assets/music/dream_lobby.wav",
                enemies: (0, 0),
                next: &[(LiminalOffice, 3), (Garden, 2), (VoidPlatforms, 1)],
                patterns: &[Pattern::Plasma, Pattern::Rings],
                accents: &[[255, 150, 220], [150, 200, 255]],
            },
            LiminalOffice => ThemeSpec {
                layout: LayoutKind::Maze,
                grid_size: (11, 15),
                wall_height: 3.0,
                floor_colors: &[[196, 184, 120], [184, 172, 112]],
                wall_colors: &[[222, 214, 168], [210, 204, 160]],
                prop_colors: &[[150, 150, 140], [120, 110, 90], [240, 240, 210]],
                props: &[PropKind::Desk, PropKind::Partition, PropKind::Lamp],
                prop_density: 0.25,
                strangeness: 0.15,
                fog_color: [0.55, 0.55, 0.4],
                ambient: [0.4, 0.4, 0.3],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "dreamscape_liminal",
                music: "games/dreamscape/assets/music/liminal_office.wav",
                enemies: (0, 1),
                next: &[(VoidPlatforms, 2), (NightmareFactory, 2), (Garden, 1)],
                patterns: &[Pattern::Stripes, Pattern::Checker],
                accents: &[[210, 255, 60], [255, 220, 40]],
            },
            VoidPlatforms => ThemeSpec {
                layout: LayoutKind::PlatformChain,
                grid_size: (11, 15),
                wall_height: 0.0,
                floor_colors: &[[140, 160, 200], [110, 120, 170], [170, 150, 210]],
                wall_colors: &[[90, 90, 120]],
                prop_colors: &[[200, 200, 255], [60, 60, 90]],
                props: &[PropKind::Pillar, PropKind::Lamp],
                prop_density: 0.08,
                strangeness: 0.6,
                fog_color: [0.08, 0.06, 0.15],
                ambient: [0.25, 0.25, 0.4],
                fog_start: 8.0,
                fog_end: 30.0,
                base_profile: "shattered_realm",
                music: "games/dreamscape/assets/music/void_platform.wav",
                enemies: (0, 0),
                next: &[(Garden, 2), (NightmareFactory, 2), (LiminalOffice, 1)],
                patterns: &[Pattern::Swirl, Pattern::Plasma],
                accents: &[[255, 40, 220], [40, 240, 255]],
            },
            Garden => ThemeSpec {
                layout: LayoutKind::ScatterField,
                grid_size: (9, 12),
                wall_height: 1.2,
                floor_colors: &[[110, 170, 90], [120, 180, 100]],
                wall_colors: &[[60, 120, 60], [70, 130, 70]],
                prop_colors: &[[90, 60, 40], [80, 150, 80], [230, 180, 210]],
                props: &[PropKind::Tree, PropKind::Hedge, PropKind::Bench],
                prop_density: 0.3,
                strangeness: 0.25,
                fog_color: [0.6, 0.75, 0.6],
                ambient: [0.4, 0.5, 0.4],
                fog_start: 8.0,
                fog_end: 42.0,
                base_profile: "ethereal_sanctuary",
                music: "games/dreamscape/assets/music/dream_lobby.wav",
                enemies: (0, 1),
                next: &[
                    (LiminalOffice, 2),
                    (VoidPlatforms, 1),
                    (NightmareFactory, 2),
                ],
                patterns: &[Pattern::Cells, Pattern::Plasma, Pattern::Rings],
                accents: &[[255, 90, 200], [180, 255, 40]],
            },
            NightmareFactory => ThemeSpec {
                layout: LayoutKind::Maze,
                grid_size: (11, 15),
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
                enemies: (2, 3),
                next: &[(VoidPlatforms, 1), (LiminalOffice, 2), (Garden, 1)],
                patterns: &[Pattern::Checker, Pattern::Stripes, Pattern::Cells],
                accents: &[[255, 120, 0], [255, 20, 60]],
            },
            Awakening => ThemeSpec {
                layout: LayoutKind::OpenHall,
                grid_size: (9, 9),
                wall_height: 3.0,
                floor_colors: &[[240, 220, 180]],
                wall_colors: &[[250, 235, 200]],
                prop_colors: &[[255, 210, 150], [230, 200, 160]],
                props: &[PropKind::Bench, PropKind::Lamp],
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
            for &(off, size) in kind.parts() {
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
