//! Player settings: volumes, grain/vignette strength, reduced motion,
//! fullscreen and key bindings. Saved as RON next to the booklet. Controller
//! buttons map onto the same actions (`pad_key`). Pure and unit-tested; the
//! menu is drawn by `settings_ui`.

use engine::sdl2::controller::Button;
use engine::sdl2::keyboard::Keycode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SETTINGS_PATH: &str = "games/dreamscape/settings.ron";

pub fn settings_path() -> PathBuf {
    std::env::var_os("DREAMSCAPE_SETTINGS")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(SETTINGS_PATH))
}

/// Things a key can be bound to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Forward,
    Back,
    Left,
    Right,
    Jump,
    Ability1,
    Ability2,
}

pub const ALL_ACTIONS: [Action; 7] = [
    Action::Forward,
    Action::Back,
    Action::Left,
    Action::Right,
    Action::Jump,
    Action::Ability1,
    Action::Ability2,
];

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Forward => "forward",
            Action::Back => "back",
            Action::Left => "left",
            Action::Right => "right",
            Action::Jump => "jump",
            Action::Ability1 => "ability 1",
            Action::Ability2 => "ability 2",
        }
    }

    fn default_key(self) -> &'static str {
        match self {
            Action::Forward => "W",
            Action::Back => "S",
            Action::Left => "A",
            Action::Right => "D",
            Action::Jump => "Space",
            Action::Ability1 => "Left Shift",
            Action::Ability2 => "E",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub music: f32,
    pub sfx: f32,
    /// Film grain strength, 0..=1.
    pub grain: f32,
    /// Tunnel-vision vignette strength, 0..=1.
    pub vignette: f32,
    /// Tones down breathing, pulses and tracers.
    pub reduced_motion: bool,
    pub fullscreen: bool,
    /// (action, SDL key name).
    pub keys: Vec<(Action, String)>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            music: 0.8,
            sfx: 0.8,
            grain: 1.0,
            vignette: 1.0,
            reduced_motion: false,
            fullscreen: false,
            keys: ALL_ACTIONS
                .iter()
                .map(|&a| (a, a.default_key().to_string()))
                .collect(),
        }
    }
}

impl Settings {
    pub fn key_for(&self, action: Action) -> Option<Keycode> {
        let name = self
            .keys
            .iter()
            .find(|(a, _)| *a == action)
            .map(|(_, n)| n.as_str())
            .unwrap_or(action.default_key());
        Keycode::from_name(name)
    }

    /// Which action (if any) this key is bound to. The right shift always
    /// doubles as ability 1.
    pub fn action_for(&self, key: Keycode) -> Option<Action> {
        ALL_ACTIONS
            .iter()
            .copied()
            .find(|&a| self.key_for(a) == Some(key))
            .or((key == Keycode::RShift).then_some(Action::Ability1))
    }

    /// Binds `action` to `key`. A key can only do one thing: whatever it was
    /// bound to before takes this action's old key (a swap).
    pub fn bind(&mut self, action: Action, key: Keycode) {
        let old = self.key_for(action).map(|k| k.name()).unwrap_or_default();
        let name = key.name();
        for (a, n) in &mut self.keys {
            if *n == name && *a != action {
                *n = old.clone();
            }
        }
        match self.keys.iter_mut().find(|(a, _)| *a == action) {
            Some((_, n)) => *n = name,
            None => self.keys.push((action, name)),
        }
    }

    pub fn motion(&self) -> f32 {
        if self.reduced_motion {
            0.25
        } else {
            1.0
        }
    }
}

pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| ron::from_str(&t).ok())
        .unwrap_or_default()
}

pub fn save(path: &Path, s: &Settings) -> anyhow::Result<()> {
    let text = ron::ser::to_string_pretty(s, ron::ser::PrettyConfig::default())?;
    std::fs::write(path, text)?;
    Ok(())
}

/// Rows of the settings menu, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Row {
    Music,
    Sfx,
    Grain,
    Vignette,
    ReducedMotion,
    Fullscreen,
    Key(Action),
    Defaults,
}

pub fn rows() -> Vec<Row> {
    let mut r = vec![
        Row::Music,
        Row::Sfx,
        Row::Grain,
        Row::Vignette,
        Row::ReducedMotion,
        Row::Fullscreen,
    ];
    r.extend(ALL_ACTIONS.iter().map(|&a| Row::Key(a)));
    r.push(Row::Defaults);
    r
}

impl Row {
    pub fn label(self) -> String {
        match self {
            Row::Music => "music volume".into(),
            Row::Sfx => "sound volume".into(),
            Row::Grain => "film grain".into(),
            Row::Vignette => "vignette".into(),
            Row::ReducedMotion => "reduced motion".into(),
            Row::Fullscreen => "fullscreen".into(),
            Row::Key(a) => format!("key: {}", a.label()),
            Row::Defaults => "reset to defaults".into(),
        }
    }
}

/// Left/right on a row: sliders step by 10%, toggles flip.
pub fn adjust(s: &mut Settings, row: Row, dir: i32) {
    let step = |v: &mut f32| *v = (*v + 0.1 * dir as f32).clamp(0.0, 1.0);
    match row {
        Row::Music => step(&mut s.music),
        Row::Sfx => step(&mut s.sfx),
        Row::Grain => step(&mut s.grain),
        Row::Vignette => step(&mut s.vignette),
        Row::ReducedMotion => s.reduced_motion = !s.reduced_motion,
        Row::Fullscreen => s.fullscreen = !s.fullscreen,
        Row::Key(_) | Row::Defaults => {}
    }
}

/// What a row shows on the right.
pub fn value(s: &Settings, row: Row) -> String {
    let pct = |v: f32| format!("{:>3}%", (v * 100.0).round() as i32);
    let onoff = |b: bool| if b { "on" } else { "off" }.to_string();
    match row {
        Row::Music => pct(s.music),
        Row::Sfx => pct(s.sfx),
        Row::Grain => pct(s.grain),
        Row::Vignette => pct(s.vignette),
        Row::ReducedMotion => onoff(s.reduced_motion),
        Row::Fullscreen => onoff(s.fullscreen),
        Row::Key(a) => s.key_for(a).map(|k| k.name()).unwrap_or("?".into()),
        Row::Defaults => String::new(),
    }
}

/// Controller buttons, as the keys they stand in for. Movement comes from
/// the left stick (see `stick_velocity`); the d-pad drives menus.
pub fn pad_key(button: Button, s: &Settings) -> Option<Keycode> {
    match button {
        Button::A => s.key_for(Action::Jump),
        Button::B => Some(Keycode::Escape),
        Button::Start => Some(Keycode::Escape),
        Button::LeftShoulder | Button::RightShoulder => s.key_for(Action::Ability1),
        Button::X | Button::Y => s.key_for(Action::Ability2),
        Button::Back => Some(Keycode::B),
        Button::DPadUp => Some(Keycode::Up),
        Button::DPadDown => Some(Keycode::Down),
        Button::DPadLeft => Some(Keycode::Left),
        Button::DPadRight => Some(Keycode::Right),
        _ => None,
    }
}

/// Stick deflection below this is ignored.
pub const STICK_DEADZONE: f32 = 0.25;

/// Left stick → world direction (same convention as the keys: screen-right
/// is world -X, stick up is away from the camera, +Z). Length 0..=1.
pub fn stick_dir(stick: (f32, f32)) -> Option<(f32, f32)> {
    let (x, y) = stick;
    let len = (x * x + y * y).sqrt();
    if len < STICK_DEADZONE {
        return None;
    }
    let k = ((len - STICK_DEADZONE) / (1.0 - STICK_DEADZONE)).min(1.0) / len;
    Some((-x * k, -y * k))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_bind_every_action_to_a_real_key() {
        let s = Settings::default();
        for a in ALL_ACTIONS {
            let k = s.key_for(a).unwrap_or_else(|| panic!("{a:?} unbound"));
            assert_eq!(s.action_for(k), Some(a));
        }
        assert_eq!(s.action_for(Keycode::RShift), Some(Action::Ability1));
        assert_eq!(s.action_for(Keycode::F12), None);
    }

    #[test]
    fn binding_a_taken_key_swaps() {
        let mut s = Settings::default();
        s.bind(Action::Jump, Keycode::W);
        assert_eq!(s.key_for(Action::Jump), Some(Keycode::W));
        assert_eq!(s.key_for(Action::Forward), Some(Keycode::Space), "swapped");
        s.bind(Action::Ability2, Keycode::Q);
        assert_eq!(s.action_for(Keycode::Q), Some(Action::Ability2));
        assert_eq!(s.action_for(Keycode::E), None);
    }

    #[test]
    fn sliders_clamp_and_toggles_flip() {
        let mut s = Settings::default();
        for _ in 0..20 {
            adjust(&mut s, Row::Music, 1);
        }
        assert_eq!(s.music, 1.0);
        for _ in 0..20 {
            adjust(&mut s, Row::Grain, -1);
        }
        assert_eq!(s.grain, 0.0);
        adjust(&mut s, Row::ReducedMotion, 1);
        assert!(s.reduced_motion && s.motion() < 1.0);
        assert_eq!(value(&s, Row::Music), "100%");
        assert_eq!(value(&s, Row::Key(Action::Jump)), "Space");
        assert_eq!(rows().len(), 6 + ALL_ACTIONS.len() + 1);
    }

    #[test]
    fn save_load_round_trip_and_bad_files_fall_back() {
        let path = std::env::temp_dir().join(format!("ds_settings_{}.ron", std::process::id()));
        let mut s = Settings::default();
        s.music = 0.3;
        s.bind(Action::Jump, Keycode::J);
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
        std::fs::write(&path, "garbage").unwrap();
        assert_eq!(load(&path), Settings::default());
        // Old files missing fields still load.
        std::fs::write(&path, "(music: 0.5)").unwrap();
        assert_eq!(load(&path).music, 0.5);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn controller_maps_onto_the_same_actions() {
        let s = Settings::default();
        assert_eq!(pad_key(Button::A, &s), s.key_for(Action::Jump));
        assert_eq!(pad_key(Button::Start, &s), Some(Keycode::Escape));
        assert_eq!(stick_dir((0.1, 0.1)), None, "deadzone");
        let (x, z) = stick_dir((1.0, 0.0)).unwrap();
        assert!(x < -0.99 && z.abs() < 1e-6, "right on the stick = world -X");
        let (_, z) = stick_dir((0.0, -1.0)).unwrap();
        assert!(z > 0.99, "stick up = forward");
    }
}
