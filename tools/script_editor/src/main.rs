//! A standalone text editor for `.pss` script files. Deliberately reuses
//! `engine::app`/`engine::ui` (SDL2 + egui + glow) rather than a separate
//! windowing stack — this app draws no 3D content at all (no `Renderer`,
//! no offscreen FBO), which is itself proof that those pieces of `engine`
//! are independently reusable outside a "real" game.

use std::path::PathBuf;

use engine::app::{App, Context, Game};
use engine::glow::{self, HasContext};
use engine::script::Interpreter;
use engine::sdl2::event::Event;
use engine::ui::EguiState;

fn default_source() -> String {
    "// A new Jame Engine script.\n\
     fn update(dt) {\n\
     }\n\
     \n\
     fn interact() {\n\
     \x20\x20\x20\x20log(\"Hello from script!\");\n\
     }\n"
        .to_string()
}

struct ScriptEditorApp {
    current_path: Option<PathBuf>,
    source: String,
    dirty: bool,
    status: String,
    status_is_error: bool,
    ui: Option<EguiState>,
}

impl ScriptEditorApp {
    fn new() -> Self {
        let initial_path = std::env::args().nth(1).map(PathBuf::from);
        let (current_path, source) = match initial_path {
            Some(path) => match std::fs::read_to_string(&path) {
                Ok(source) => (Some(path), source),
                Err(err) => {
                    log::error!("failed to read {path:?}: {err}");
                    (None, default_source())
                }
            },
            None => (None, default_source()),
        };
        Self {
            current_path,
            source,
            dirty: false,
            status: "Ready.".to_string(),
            status_is_error: false,
            ui: None,
        }
    }

    fn title(&self) -> String {
        let name = self
            .current_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled.pss".to_string());
        format!("{name}{}", if self.dirty { " *" } else { "" })
    }

    fn new_file(&mut self) {
        self.current_path = None;
        self.source = default_source();
        self.dirty = false;
        self.set_status("New script.", false);
    }

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("PS2 Script", &["pss"]).pick_file() else {
            return;
        };
        match std::fs::read_to_string(&path) {
            Ok(source) => {
                self.source = source;
                self.current_path = Some(path);
                self.dirty = false;
                self.set_status("Opened.", false);
            }
            Err(err) => self.set_status(&format!("Failed to open: {err}"), true),
        }
    }

    fn save(&mut self) {
        match self.current_path.clone() {
            Some(path) => self.save_to(path),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PS2 Script", &["pss"])
            .set_file_name("script.pss")
            .save_file()
        else {
            return;
        };
        self.save_to(path);
    }

    fn save_to(&mut self, path: PathBuf) {
        match std::fs::write(&path, &self.source) {
            Ok(()) => {
                self.current_path = Some(path);
                self.dirty = false;
                self.set_status("Saved.", false);
            }
            Err(err) => self.set_status(&format!("Failed to save: {err}"), true),
        }
    }

    /// Compiles the current source with `engine::script` directly (the
    /// same entry point the game uses) without running any of it — this
    /// tool has no game/`ScriptApi` to run against, only syntax to check.
    fn check_syntax(&mut self) {
        match Interpreter::compile(&self.source) {
            Ok(_) => self.set_status("OK \u{2014} no syntax errors.", false),
            Err(errors) => {
                let message = errors.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
                self.set_status(&message, true);
            }
        }
    }

    fn set_status(&mut self, message: &str, is_error: bool) {
        self.status = message.to_string();
        self.status_is_error = is_error;
    }
}

impl Game for ScriptEditorApp {
    fn init(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        self.ui = Some(EguiState::new(ctx.gl_arc())?);
        Ok(())
    }

    fn handle_event(&mut self, _ctx: &mut Context, event: &Event) {
        if let Some(ui) = self.ui.as_mut() {
            ui.handle_event(event);
        }
    }

    fn update(&mut self, _ctx: &mut Context, _dt: f32) -> anyhow::Result<()> {
        Ok(())
    }

    fn render(&mut self, ctx: &mut Context) -> anyhow::Result<()> {
        let drawable_size = ctx.drawable_size();
        unsafe {
            ctx.gl().clear_color(0.12, 0.12, 0.15, 1.0);
            ctx.gl().clear(glow::COLOR_BUFFER_BIT);
        }

        let mut new_file = false;
        let mut open_file = false;
        let mut save = false;
        let mut save_as = false;
        let mut check_syntax = false;
        let mut source = self.source.clone();
        let mut dirty = self.dirty;

        let title = self.title();
        let status = self.status.clone();
        let status_is_error = self.status_is_error;

        let mut ui_state = self.ui.take().expect("ui set up in init");
        let full_output = ui_state.run(drawable_size, |egui_ctx| {
            egui::TopBottomPanel::top("toolbar").show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong(&title);
                    ui.separator();
                    if ui.button("New").clicked() {
                        new_file = true;
                    }
                    if ui.button("Open...").clicked() {
                        open_file = true;
                    }
                    if ui.button("Save").clicked() {
                        save = true;
                    }
                    if ui.button("Save As...").clicked() {
                        save_as = true;
                    }
                    ui.separator();
                    if ui.button("Check Syntax").clicked() {
                        check_syntax = true;
                    }
                });
            });
            egui::TopBottomPanel::bottom("status").show(egui_ctx, |ui| {
                let color = if status_is_error {
                    egui::Color32::from_rgb(255, 120, 120)
                } else {
                    egui::Color32::from_rgb(150, 220, 150)
                };
                ui.colored_label(color, &status);
            });
            egui::CentralPanel::default().show(egui_ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let response = ui.add_sized(
                        ui.available_size(),
                        egui::TextEdit::multiline(&mut source).code_editor(),
                    );
                    if response.changed() {
                        dirty = true;
                    }
                });
            });
        });
        ui_state.paint(drawable_size, full_output);
        self.ui = Some(ui_state);

        self.source = source;
        self.dirty = dirty;

        if new_file {
            self.new_file();
        }
        if open_file {
            self.open_file();
        }
        if save {
            self.save();
        }
        if save_as {
            self.save_as();
        }
        if check_syntax {
            self.check_syntax();
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    App::run("Jame Engine Script Editor", 900, 700, ScriptEditorApp::new())
}
