//! Foil pack wrappers, painted with egui (no image files): the theme's own
//! texture as the body, crimped edges, a centre emblem. See
//! plans/PLAN-pick-a-pack.md.
//!
//! SKELETON: stream B replaces the body of `draw_pack`.
#![allow(dead_code)]

use crate::packs::Emblem;
use crate::dream::DreamTheme;
use engine::ui::egui::{self, Align2, FontId, Rect, Stroke, Vec2};

/// Wrapper textures, uploaded to egui on first sight and kept.
#[derive(Default)]
pub struct PackArt {
    cache: std::collections::HashMap<u64, egui::TextureHandle>,
}

/// Natural wrapper size at scale 1.0 (taller than wide, like a card).
pub const PACK_SIZE: Vec2 = Vec2::new(220.0, 320.0);

/// Draw one wrapper into `rect`.
/// - `pips`: card count, drawn on the crimp.
/// - `time`: seconds, drives the foil sweep.
/// - `motion`: 0.0 (reduced motion: no sweep or tilt) ..= 1.0.
/// - `tilt`: -1..1 on each axis, toward the cursor/stick.
#[allow(clippy::too_many_arguments)]
pub fn draw_pack(
    _ctx: &egui::Context,
    p: &egui::Painter,
    rect: Rect,
    _theme: DreamTheme,
    emblem: Emblem,
    _seed: u64,
    _pips: usize,
    _time: f32,
    _motion: f32,
    _tilt: Vec2,
    _art: &mut PackArt,
) {
    p.rect_filled(rect, 6.0, egui::Color32::from_gray(60));
    p.rect_stroke(rect, 6.0, Stroke::new(2.0_f32, egui::Color32::WHITE));
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        format!("{emblem:?}"),
        FontId::monospace(24.0),
        egui::Color32::WHITE,
    );
}
