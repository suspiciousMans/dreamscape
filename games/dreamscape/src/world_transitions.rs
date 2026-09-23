use engine::glam::Vec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldType {
    DreamLobby,
    LiminalOffice,
    EndlessStaircase,
    VoidPlatform,
    DreamGarden,
    NightmareFactory,
    Awakening,  // Final escape level
}

impl WorldType {
    pub fn level_path(&self) -> &'static str {
        match self {
            WorldType::DreamLobby => "games/dreamscape/levels/dream_lobby.ron",
            WorldType::LiminalOffice => "games/dreamscape/levels/liminal_office.ron",
            WorldType::EndlessStaircase => "games/dreamscape/levels/endless_staircase.ron",
            WorldType::VoidPlatform => "games/dreamscape/levels/void_platform.ron",
            WorldType::DreamGarden => "games/dreamscape/levels/dream_garden.ron",
            WorldType::NightmareFactory => "games/dreamscape/levels/nightmare_factory.ron",
            WorldType::Awakening => "games/dreamscape/levels/awakening.ron",
        }
    }

    pub fn difficulty(&self) -> f32 {
        match self {
            WorldType::DreamLobby => 0.0,
            WorldType::LiminalOffice => 0.3,
            WorldType::EndlessStaircase => 0.4,
            WorldType::DreamGarden => 0.2,
            WorldType::VoidPlatform => 0.7,
            WorldType::NightmareFactory => 1.0,
            WorldType::Awakening => 0.5,
        }
    }

    /// Get the music track path for this world type
    pub fn music_path(&self) -> &'static str {
        match self {
            WorldType::DreamLobby => "games/dreamscape/assets/music/dream_lobby.wav",
            WorldType::LiminalOffice => "games/dreamscape/assets/music/liminal_office.wav",
            WorldType::EndlessStaircase => "games/dreamscape/assets/music/liminal_office.wav",
            WorldType::VoidPlatform => "games/dreamscape/assets/music/void_platform.wav",
            WorldType::DreamGarden => "games/dreamscape/assets/music/dream_lobby.wav",
            WorldType::NightmareFactory => "games/dreamscape/assets/music/nightmare_factory.wav",
            WorldType::Awakening => "games/dreamscape/assets/music/awakening.wav",
        }
    }

    /// Per-world wall/floor tint color (RGBA). Each world gets a distinct
    /// color so players can identify worlds beyond just fog color.
    pub fn tint_color(&self) -> [u8; 4] {
        match self {
            WorldType::DreamLobby => [230, 220, 240, 255],
            WorldType::LiminalOffice => [200, 210, 160, 255],
            WorldType::EndlessStaircase => [160, 160, 160, 255],
            WorldType::VoidPlatform => [140, 160, 190, 255],
            WorldType::DreamGarden => [170, 210, 160, 255],
            WorldType::NightmareFactory => [180, 90, 70, 255],
            WorldType::Awakening => [230, 200, 140, 255],
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorldTransition {
    pub from: WorldType,
    pub to: WorldType,
    pub trigger_pos: Vec3,
    pub portal_radius: f32,
}

pub struct TransitionManager {
    worlds: Vec<WorldType>,
    current_index: usize,
}

impl TransitionManager {
    pub fn new() -> Self {
        // Phase 12: All 7 worlds in proper sequence
        let worlds = vec![
            WorldType::DreamLobby,
            WorldType::LiminalOffice,
            WorldType::VoidPlatform,
            WorldType::DreamGarden,
            WorldType::NightmareFactory,
            WorldType::Awakening,
        ];
        Self {
            worlds,
            current_index: 0,
        }
    }

    pub fn current_world(&self) -> WorldType {
        self.worlds[self.current_index]
    }

    pub fn current_index(&self) -> usize {
        self.current_index
    }

    pub fn advance(&mut self) -> Option<WorldType> {
        if self.current_index + 1 < self.worlds.len() {
            self.current_index += 1;
            Some(self.worlds[self.current_index])
        } else {
            None  // Game complete
        }
    }

    pub fn is_final_world(&self) -> bool {
        self.current_index == self.worlds.len() - 1
    }
}
