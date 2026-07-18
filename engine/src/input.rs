use std::collections::HashSet;

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::GameControllerSubsystem;

/// Raw `i16` axis values below this fraction of full range are treated as
/// zero — cheap analog sticks rarely rest at exactly 0, so without a
/// deadzone a "centered" stick would still drive slow drift.
const STICK_DEADZONE: f32 = 0.15;

#[derive(Default)]
pub struct Input {
    keys_down: HashSet<Keycode>,
    keys_pressed: HashSet<Keycode>,
    keys_released: HashSet<Keycode>,
    buttons_down: HashSet<MouseButton>,
    buttons_pressed: HashSet<MouseButton>,
    mouse_pos: (i32, i32),
    mouse_delta: (i32, i32),
    // Only one controller is tracked at a time — the first one connected;
    // fine for a single local player. Held here (rather than on `Platform`)
    // since `Input` already owns every other input state.
    controller: Option<GameController>,
    controller_buttons_down: HashSet<Button>,
    controller_buttons_pressed: HashSet<Button>,
    left_stick: (f32, f32),
    right_stick: (f32, f32),
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears the per-frame edge-triggered state (just-pressed/released, mouse delta).
    /// Call once at the start of each frame, before draining events.
    pub fn begin_frame(&mut self) {
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.buttons_pressed.clear();
        self.controller_buttons_pressed.clear();
        self.mouse_delta = (0, 0);
    }

    fn normalize_axis(raw: i16) -> f32 {
        let value = raw as f32 / i16::MAX as f32;
        if value.abs() < STICK_DEADZONE {
            0.0
        } else {
            value.clamp(-1.0, 1.0)
        }
    }

    /// Handles keyboard/mouse events unconditionally; controller connect/
    /// disconnect events additionally need `controller_subsystem` to open/
    /// query the device (only available from `Platform`, not stored here).
    pub fn handle_event(&mut self, event: &Event, controller_subsystem: &GameControllerSubsystem) {
        match *event {
            Event::KeyDown {
                keycode: Some(k),
                repeat: false,
                ..
            } => {
                self.keys_down.insert(k);
                self.keys_pressed.insert(k);
            }
            Event::KeyUp {
                keycode: Some(k), ..
            } => {
                self.keys_down.remove(&k);
                self.keys_released.insert(k);
            }
            Event::MouseButtonDown { mouse_btn, .. } => {
                self.buttons_down.insert(mouse_btn);
                self.buttons_pressed.insert(mouse_btn);
            }
            Event::MouseButtonUp { mouse_btn, .. } => {
                self.buttons_down.remove(&mouse_btn);
            }
            Event::MouseMotion {
                x, y, xrel, yrel, ..
            } => {
                self.mouse_pos = (x, y);
                self.mouse_delta.0 += xrel;
                self.mouse_delta.1 += yrel;
            }
            Event::ControllerDeviceAdded { which, .. } => {
                if self.controller.is_none() {
                    match controller_subsystem.open(which) {
                        Ok(controller) => {
                            log::info!("controller connected: {}", controller.name());
                            self.controller = Some(controller);
                        }
                        Err(err) => log::warn!("failed to open controller {which}: {err}"),
                    }
                }
            }
            Event::ControllerDeviceRemoved { which, .. } => {
                if self.controller.as_ref().is_some_and(|c| c.instance_id() == which as u32) {
                    log::info!("controller disconnected");
                    self.controller = None;
                    self.controller_buttons_down.clear();
                    self.left_stick = (0.0, 0.0);
                    self.right_stick = (0.0, 0.0);
                }
            }
            Event::ControllerAxisMotion { axis, value, .. } => {
                let normalized = Self::normalize_axis(value);
                match axis {
                    Axis::LeftX => self.left_stick.0 = normalized,
                    Axis::LeftY => self.left_stick.1 = normalized,
                    Axis::RightX => self.right_stick.0 = normalized,
                    Axis::RightY => self.right_stick.1 = normalized,
                    _ => {}
                }
            }
            Event::ControllerButtonDown { button, .. } => {
                self.controller_buttons_down.insert(button);
                self.controller_buttons_pressed.insert(button);
            }
            Event::ControllerButtonUp { button, .. } => {
                self.controller_buttons_down.remove(&button);
            }
            _ => {}
        }
    }

    pub fn is_key_down(&self, key: Keycode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn just_pressed(&self, key: Keycode) -> bool {
        self.keys_pressed.contains(&key)
    }

    pub fn just_released(&self, key: Keycode) -> bool {
        self.keys_released.contains(&key)
    }

    pub fn is_button_down(&self, button: MouseButton) -> bool {
        self.buttons_down.contains(&button)
    }

    pub fn button_just_pressed(&self, button: MouseButton) -> bool {
        self.buttons_pressed.contains(&button)
    }

    pub fn mouse_pos(&self) -> (i32, i32) {
        self.mouse_pos
    }

    pub fn mouse_delta(&self) -> (i32, i32) {
        self.mouse_delta
    }

    /// `(x, y)`, each in `-1.0..=1.0`, deadzoned. `y` follows SDL's raw axis
    /// convention (positive = down) — negate it if you want positive = up.
    pub fn left_stick(&self) -> (f32, f32) {
        self.left_stick
    }

    pub fn right_stick(&self) -> (f32, f32) {
        self.right_stick
    }

    pub fn controller_button_down(&self, button: Button) -> bool {
        self.controller_buttons_down.contains(&button)
    }

    pub fn controller_just_pressed(&self, button: Button) -> bool {
        self.controller_buttons_pressed.contains(&button)
    }
}
