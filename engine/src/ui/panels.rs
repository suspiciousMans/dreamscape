use crate::profile::{LightingMode, RenderParams, TextureFilterMode};

/// Draws sliders/checkboxes/dropdowns for every `RenderParams` field. Returns
/// `true` if anything changed this frame, so callers can react (e.g. the
/// affine-mapping toggle needs a shader-variant swap, not just a uniform).
pub fn render_params_editor(ui: &mut egui::Ui, params: &mut RenderParams) -> bool {
    let mut changed = false;

    ui.heading("Resolution");
    changed |= ui
        .add(egui::Slider::new(&mut params.resolution_scale, 0.05..=1.0).text("Internal scale"))
        .changed();

    ui.separator();
    ui.heading("Lighting");
    egui::ComboBox::from_label("Lighting mode")
        .selected_text(format!("{:?}", params.lighting_mode))
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(&mut params.lighting_mode, LightingMode::Unlit, "Unlit")
                .changed();
            changed |= ui
                .selectable_value(
                    &mut params.lighting_mode,
                    LightingMode::VertexLit,
                    "Vertex Lit",
                )
                .changed();
        });
    changed |= color_edit(ui, "Ambient color", &mut params.ambient_color);

    ui.separator();
    ui.heading("Sky");
    changed |= color_edit(ui, "Horizon color", &mut params.sky_horizon_color);
    changed |= color_edit(ui, "Zenith color", &mut params.sky_zenith_color);

    ui.separator();
    ui.heading("Fog");
    changed |= color_edit(ui, "Fog color", &mut params.fog_color);
    changed |= ui
        .add(egui::Slider::new(&mut params.fog_start, 0.0..=50.0).text("Fog start"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut params.fog_end, 0.0..=100.0).text("Fog end"))
        .changed();

    ui.separator();
    ui.heading("Posterize / Dither");
    let mut levels = params.color_levels as f32;
    if ui
        .add(egui::Slider::new(&mut levels, 2.0..=256.0).text("Color levels"))
        .changed()
    {
        params.color_levels = levels.round() as u32;
        changed = true;
    }
    changed |= ui
        .add(egui::Slider::new(&mut params.dither_strength, 0.0..=1.0).text("Dither strength"))
        .changed();

    ui.separator();
    ui.heading("Geometry / Texturing");
    changed |= ui
        .add(
            egui::Slider::new(&mut params.vertex_snap_amount, 0.0..=0.2)
                .text("Vertex snap amount"),
        )
        .changed();
    changed |= ui
        .checkbox(&mut params.affine_texture_mapping, "Affine texture mapping")
        .changed();
    changed |= ui
        .checkbox(&mut params.backface_culling, "Backface culling")
        .on_hover_text("Hides a triangle's interior-facing side — stops seeing inside solid geometry when the camera clips into it")
        .changed();
    egui::ComboBox::from_label("Texture filter")
        .selected_text(format!("{:?}", params.texture_filter))
        .show_ui(ui, |ui| {
            changed |= ui
                .selectable_value(
                    &mut params.texture_filter,
                    TextureFilterMode::Nearest,
                    "Nearest",
                )
                .changed();
            changed |= ui
                .selectable_value(
                    &mut params.texture_filter,
                    TextureFilterMode::Bilinear,
                    "Bilinear",
                )
                .changed();
        });

    changed
}

fn color_edit(ui: &mut egui::Ui, label: &str, color: &mut [f32; 3]) -> bool {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.color_edit_button_rgb(color).changed()
    })
    .inner
}
