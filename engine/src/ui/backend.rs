use std::sync::Arc;
use std::time::Instant;

use sdl2::event::Event as SdlEvent;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton as SdlMouseButton;

/// Bridges SDL2 input/window state to egui and paints egui's output via
/// `egui_glow`. One instance per window; feed it every SDL2 event via
/// `handle_event`, then wrap your UI closure in `run`, and `paint` the result.
pub struct EguiState {
    ctx: egui::Context,
    painter: egui_glow::Painter,
    raw_input: egui::RawInput,
    start: Instant,
    pointer_pos: egui::Pos2,
    modifiers: egui::Modifiers,
}

impl EguiState {
    pub fn new(gl: Arc<glow::Context>) -> anyhow::Result<Self> {
        let painter = egui_glow::Painter::new(gl, "", None, false).map_err(anyhow::Error::msg)?;
        Ok(Self {
            ctx: egui::Context::default(),
            painter,
            raw_input: egui::RawInput::default(),
            start: Instant::now(),
            pointer_pos: egui::Pos2::ZERO,
            modifiers: egui::Modifiers::default(),
        })
    }

    /// Registers an already-uploaded GL texture with egui's painter so it
    /// can be drawn via `egui::Image`/`ImageButton` — used by
    /// `engine::ui::asset_browser` for thumbnails. Must be called outside
    /// `run`'s closure (the painter is exclusively borrowed for its
    /// duration); see that module's doc comment. The registered texture is
    /// never freed (`egui_glow::Painter::free_texture` would also delete
    /// the underlying GL texture, which the caller may still own) — an
    /// accepted small per-thumbnail leak for a dev-tool cache, not
    /// something a shipped build ever hits (asset browsing is Edit-mode
    /// only).
    pub fn register_texture(&mut self, texture: glow::Texture) -> egui::TextureId {
        self.painter.register_native_texture(texture)
    }

    pub fn wants_pointer_input(&self) -> bool {
        self.ctx.wants_pointer_input()
    }

    pub fn wants_keyboard_input(&self) -> bool {
        self.ctx.wants_keyboard_input()
    }

    pub fn handle_event(&mut self, event: &SdlEvent) {
        match *event {
            SdlEvent::MouseMotion { x, y, .. } => {
                self.pointer_pos = egui::pos2(x as f32, y as f32);
                self.raw_input
                    .events
                    .push(egui::Event::PointerMoved(self.pointer_pos));
            }
            SdlEvent::MouseButtonDown { mouse_btn, .. } => {
                if let Some(button) = translate_mouse_button(mouse_btn) {
                    self.raw_input.events.push(egui::Event::PointerButton {
                        pos: self.pointer_pos,
                        button,
                        pressed: true,
                        modifiers: self.modifiers,
                    });
                }
            }
            SdlEvent::MouseButtonUp { mouse_btn, .. } => {
                if let Some(button) = translate_mouse_button(mouse_btn) {
                    self.raw_input.events.push(egui::Event::PointerButton {
                        pos: self.pointer_pos,
                        button,
                        pressed: false,
                        modifiers: self.modifiers,
                    });
                }
            }
            SdlEvent::MouseWheel { x, y, .. } => {
                self.raw_input.events.push(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(x as f32, y as f32),
                    modifiers: self.modifiers,
                });
            }
            SdlEvent::KeyDown {
                keycode: Some(keycode),
                keymod,
                repeat,
                ..
            } => {
                self.modifiers = translate_modifiers(keymod);
                if let Some(key) = translate_keycode(keycode) {
                    self.raw_input.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: true,
                        repeat,
                        modifiers: self.modifiers,
                    });
                }
            }
            SdlEvent::KeyUp {
                keycode: Some(keycode),
                keymod,
                ..
            } => {
                self.modifiers = translate_modifiers(keymod);
                if let Some(key) = translate_keycode(keycode) {
                    self.raw_input.events.push(egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed: false,
                        repeat: false,
                        modifiers: self.modifiers,
                    });
                }
            }
            SdlEvent::TextInput { ref text, .. } => {
                self.raw_input.events.push(egui::Event::Text(text.clone()));
            }
            _ => {}
        }
    }

    pub fn run(
        &mut self,
        drawable_size: (u32, u32),
        run_ui: impl FnMut(&egui::Context),
    ) -> egui::FullOutput {
        self.raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(drawable_size.0 as f32, drawable_size.1 as f32),
        ));
        self.raw_input.time = Some(self.start.elapsed().as_secs_f64());
        self.raw_input.modifiers = self.modifiers;

        let input = self.raw_input.take();
        self.ctx.run(input, run_ui)
    }

    pub fn paint(&mut self, drawable_size: (u32, u32), output: egui::FullOutput) {
        for (id, delta) in &output.textures_delta.set {
            self.painter.set_texture(*id, delta);
        }

        let clipped_primitives = self
            .ctx
            .tessellate(output.shapes, output.pixels_per_point);
        self.painter.paint_primitives(
            [drawable_size.0, drawable_size.1],
            output.pixels_per_point,
            &clipped_primitives,
        );

        for id in &output.textures_delta.free {
            self.painter.free_texture(*id);
        }
    }
}

fn translate_mouse_button(button: SdlMouseButton) -> Option<egui::PointerButton> {
    match button {
        SdlMouseButton::Left => Some(egui::PointerButton::Primary),
        SdlMouseButton::Right => Some(egui::PointerButton::Secondary),
        SdlMouseButton::Middle => Some(egui::PointerButton::Middle),
        _ => None,
    }
}

fn translate_modifiers(keymod: Mod) -> egui::Modifiers {
    egui::Modifiers {
        alt: keymod.intersects(Mod::LALTMOD | Mod::RALTMOD),
        ctrl: keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
        shift: keymod.intersects(Mod::LSHIFTMOD | Mod::RSHIFTMOD),
        mac_cmd: false,
        command: keymod.intersects(Mod::LCTRLMOD | Mod::RCTRLMOD),
    }
}

fn translate_keycode(keycode: Keycode) -> Option<egui::Key> {
    use egui::Key;
    Some(match keycode {
        Keycode::Backspace => Key::Backspace,
        Keycode::Delete => Key::Delete,
        Keycode::Return | Keycode::KpEnter => Key::Enter,
        Keycode::Escape => Key::Escape,
        Keycode::Tab => Key::Tab,
        Keycode::Space => Key::Space,
        Keycode::Left => Key::ArrowLeft,
        Keycode::Right => Key::ArrowRight,
        Keycode::Up => Key::ArrowUp,
        Keycode::Down => Key::ArrowDown,
        Keycode::Home => Key::Home,
        Keycode::End => Key::End,
        Keycode::PageUp => Key::PageUp,
        Keycode::PageDown => Key::PageDown,
        Keycode::A => Key::A,
        Keycode::C => Key::C,
        Keycode::V => Key::V,
        Keycode::X => Key::X,
        Keycode::Z => Key::Z,
        _ => return None,
    })
}
