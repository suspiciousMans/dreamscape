//! The hand: the equipped cards that hold abilities, fanned down the right
//! edge of the screen like cards held in a hand: overlapping, each tilted a
//! little more than the last. A card straightens, leaps toward the centre,
//! flashes and rings when its ability is used.
//!
//! Layout, tilt and timing are pure and unit-tested; `draw` only paints them.

use engine::ui::egui::{
    self, epaint::TextShape, Color32, FontId, Mesh, Pos2, Rect, Shape, Stroke, Vec2,
};

/// Seconds a card takes to leap out, hold, and settle back.
pub const POP_RISE: f32 = 0.08;
pub const POP_HOLD: f32 = 0.18;
pub const POP_FALL: f32 = 0.35;
pub const POP_TOTAL: f32 = POP_RISE + POP_HOLD + POP_FALL;
/// Seconds the expanding ring lives.
pub const RING_SECS: f32 = 0.5;

/// `since_use` for a card that has not been used (this dream).
pub const NEVER: f32 = f32::MAX;

/// Card size at 720p, and how much of each card the next one leaves showing.
const CARD_W: f32 = 92.0;
const CARD_H: f32 = 118.0;
const VISIBLE: f32 = 0.7;
/// Tilt between neighbouring cards, in radians (about 6 degrees).
const TILT_STEP: f32 = 0.105;

/// One card in the hand, as the HUD sees it.
#[derive(Clone, Debug)]
pub struct HandCard {
    pub ability: &'static str,
    pub key: &'static str,
    /// 0..1 recharge.
    pub charge: f32,
    /// The ability is running right now.
    pub active: bool,
    /// The card's tint and second colour.
    pub tint: [u8; 3],
    pub tint2: [u8; 3],
    /// Seconds since the ability was used; [`NEVER`] if it has not been.
    pub since_use: f32,
    /// The equipped card itself: its artwork and name fill the face.
    pub card: Option<crate::cards::Card>,
}

/// 0 at rest, 1 at the top of the pop: quick out, short hold, slow back.
pub fn pop(since: f32) -> f32 {
    if !(0.0..POP_TOTAL).contains(&since) {
        return 0.0;
    }
    if since < POP_RISE {
        let t = since / POP_RISE;
        return 1.0 - (1.0 - t) * (1.0 - t);
    }
    let held = since - POP_RISE;
    if held < POP_HOLD {
        return 1.0;
    }
    let t = (held - POP_HOLD) / POP_FALL;
    let e = t * t * (3.0 - 2.0 * t);
    1.0 - e
}

/// 0..1 progress of the ring, or `None` when it is not showing.
pub fn ring(since: f32) -> Option<f32> {
    (0.0..RING_SECS).contains(&since).then(|| since / RING_SECS)
}

/// How far card `i` of `n` leans at rest (radians, clockwise positive):
/// the top card leans back, the bottom one forward, the middle stands.
pub fn tilt(i: usize, n: usize) -> f32 {
    (i as f32 - (n as f32 - 1.0) / 2.0) * TILT_STEP
}

/// Where each of `n` cards rests (before tilt): overlapped down the right
/// edge, the middle ones bowed toward the centre, like a hand held out.
pub fn layout(screen: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let u = crate::pixel_ui::scale(screen);
    let (w, h) = (CARD_W * u, CARD_H * u);
    let top_min = screen.top() + 150.0 * u;
    let bottom_max = screen.bottom() - 40.0 * u;
    let room = (bottom_max - top_min).max(h);
    let mut step = h * VISIBLE;
    let mut total = h + (n as f32 - 1.0) * step;
    if total > room && n > 1 {
        step = (room - h) / (n as f32 - 1.0);
        total = h + (n as f32 - 1.0) * step;
    }
    let top = (screen.center().y - total / 2.0).clamp(top_min, (bottom_max - total).max(top_min));
    let mid = (n as f32 - 1.0) / 2.0;
    (0..n)
        .map(|i| {
            let t = if mid > 0.0 { (i as f32 - mid) / mid } else { 0.0 };
            let bow = -16.0 * u * (1.0 - t * t);
            Rect::from_min_size(
                Pos2::new(screen.right() - 26.0 * u - w + bow, top + i as f32 * step),
                Vec2::new(w, h),
            )
        })
        .collect()
}

/// The card's rect while popped: toward the centre and a little larger.
/// Reduced motion keeps it in place.
pub fn popped(rest: Rect, amount: f32, u: f32, reduced: bool) -> Rect {
    if reduced || amount <= 0.0 {
        return rest;
    }
    let grown = Rect::from_center_size(rest.center(), rest.size() * (1.0 + 0.16 * amount));
    grown.translate(Vec2::new(-120.0 * u * amount, 0.0))
}

/// A card's four corners after turning `angle` about its centre.
#[cfg(test)]
pub fn corners(r: Rect, angle: f32) -> [Pos2; 4] {
    let x = Xf::new(r.center(), angle);
    let (hw, hh) = (r.width() / 2.0, r.height() / 2.0);
    [
        x.at(Vec2::new(-hw, -hh)),
        x.at(Vec2::new(hw, -hh)),
        x.at(Vec2::new(hw, hh)),
        x.at(Vec2::new(-hw, hh)),
    ]
}

/// A turn about a card's centre: local offsets in, screen points out.
#[derive(Clone, Copy)]
struct Xf {
    centre: Pos2,
    cos: f32,
    sin: f32,
}

impl Xf {
    fn new(centre: Pos2, angle: f32) -> Self {
        Self {
            centre,
            cos: angle.cos(),
            sin: angle.sin(),
        }
    }

    fn at(&self, o: Vec2) -> Pos2 {
        self.centre + Vec2::new(o.x * self.cos - o.y * self.sin, o.x * self.sin + o.y * self.cos)
    }

    fn rect_points(&self, r: Rect) -> Vec<Pos2> {
        vec![
            self.at(r.left_top().to_vec2()),
            self.at(r.right_top().to_vec2()),
            self.at(r.right_bottom().to_vec2()),
            self.at(r.left_bottom().to_vec2()),
        ]
    }

    fn quad(&self, p: &egui::Painter, r: Rect, color: Color32) {
        p.add(Shape::convex_polygon(self.rect_points(r), color, Stroke::NONE));
    }

    fn outline(&self, p: &egui::Painter, r: Rect, width: f32, color: Color32) {
        p.add(Shape::closed_line(self.rect_points(r), Stroke::new(width, color)));
    }

    fn image(&self, p: &egui::Painter, id: egui::TextureId, r: Rect, uv: Rect, tint: Color32) {
        let mut mesh = Mesh::with_texture(id);
        let corners = [
            (r.left_top(), uv.left_top()),
            (r.right_top(), uv.right_top()),
            (r.right_bottom(), uv.right_bottom()),
            (r.left_bottom(), uv.left_bottom()),
        ];
        for (pos, uv) in corners {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: self.at(pos.to_vec2()),
                uv,
                color: tint,
            });
        }
        mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
        p.add(Shape::mesh(mesh));
    }

    /// Text centred on local point `o`, turned with the card.
    fn text(&self, p: &egui::Painter, o: Vec2, text: &str, size: f32, color: Color32, angle: f32) {
        let galley = p.layout_no_wrap(text.to_string(), FontId::monospace(size), color);
        let corner = self.at(o - galley.size() / 2.0);
        p.add(TextShape::new(corner, galley, color).with_angle(angle));
    }
}

fn rgba(rgb: [u8; 3], a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(rgb[0], rgb[1], rgb[2], (a.clamp(0.0, 1.0) * 255.0) as u8)
}

/// Paints the hand. `time` drives the ready-pulse only.
pub fn draw(
    ctx: &egui::Context,
    p: &egui::Painter,
    screen: Rect,
    cards: &[HandCard],
    time: f32,
    reduced: bool,
    art: &mut crate::booklet_ui::CardArt,
) {
    let rests = layout(screen, cards.len());
    let u = crate::pixel_ui::scale(screen);
    // Rest order (later cards lie on top), then whichever is mid-pop above all.
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by(|&a, &b| {
        pop(cards[a].since_use)
            .partial_cmp(&pop(cards[b].since_use))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for i in order {
        let c = &cards[i];
        let amount = pop(c.since_use);
        let r = popped(rests[i], amount, u, reduced);
        let angle = tilt(i, cards.len()) * (1.0 - amount);
        if let Some(t) = ring(c.since_use) {
            let grow = if reduced { 0.0 } else { 26.0 * u * t };
            Xf::new(r.center(), angle).outline(
                p,
                Rect::from_center_size(Pos2::ZERO, r.size()).expand(grow),
                2.0 * u,
                rgba(c.tint, 0.8 * (1.0 - t)),
            );
        }
        draw_card(ctx, p, r, angle, c, amount, time, u, art);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_card(
    ctx: &egui::Context,
    p: &egui::Painter,
    r: Rect,
    angle: f32,
    c: &HandCard,
    amount: f32,
    time: f32,
    u: f32,
    art_cache: &mut crate::booklet_ui::CardArt,
) {
    let x = Xf::new(r.center(), angle);
    let ready = c.charge >= 1.0;
    let lit = ready || c.active || amount > 0.0;
    // The frame is the card's own (rarity); the tint is for the ring and flash.
    let frame = c
        .card
        .as_ref()
        .map_or(c.tint, |card| crate::booklet_ui::card_frame(card, time));
    let edge = if lit { frame } else { frame.map(|v| v / 2 + 40) };
    let k = r.width() / (CARD_W * u); // 1.0 at rest, a little more popped
    let cell = (3.0 * u * k).round().max(2.0);
    let body = Rect::from_center_size(Pos2::ZERO, r.size());

    // A soft shadow, so the overlap reads; then frame and body.
    x.quad(p, body.translate(Vec2::new(-3.0 * u, 3.0 * u)), Color32::from_black_alpha(110));
    x.quad(p, body, rgba(edge, if lit { 1.0 } else { 0.75 }));
    x.quad(p, body.shrink(cell), rgba([14, 8, 28], 1.0));
    x.outline(p, body.shrink(cell * 2.0), 1.0, rgba(edge, 0.35));

    let inner = body.shrink(cell * 2.5);
    let font = |base: f32| (base * u * k).max(7.0);

    // The card's name across the top, on up to two lines.
    let name = c.card.as_ref().map_or(c.ability, |card| card.name.as_str());
    let name_h = 24.0 * u * k;
    let name_size = (10.0 * u * k).max(7.0);
    let per_line = (inner.width() / (name_size * 0.6)).floor().max(4.0) as usize;
    let lines = name_lines(name, per_line);
    let line_h = name_h / 2.0;
    let first = if lines.1.is_some() { 0.0 } else { 0.5 };
    for (n, line) in std::iter::once(&lines.0).chain(lines.1.iter()).enumerate() {
        x.text(
            p,
            Vec2::new(0.0, inner.top() + line_h * (first + n as f32) + line_h / 2.0),
            line,
            name_size,
            rgba(edge, 1.0),
            angle,
        );
    }

    // The card's own artwork. A card we can't find falls back to tint bands.
    let art = Rect::from_min_size(
        Pos2::new(inner.left(), inner.top() + name_h + 2.0 * u * k),
        Vec2::new(inner.width(), 46.0 * u * k),
    );
    match &c.card {
        Some(card) => {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, art.height() / art.width()));
            let d = if lit { 255 } else { 150 };
            x.image(p, art_cache.texture(ctx, card), art, uv, Color32::from_rgb(d, d, d));
        }
        None => {
            let bands = 6;
            for b in 0..bands {
                let f = b as f32 / (bands - 1) as f32;
                let mix = |i: usize| (c.tint[i] as f32 * (1.0 - f) + c.tint2[i] as f32 * f) as u8;
                let band = Rect::from_min_size(
                    Pos2::new(art.left(), art.top() + art.height() * b as f32 / bands as f32),
                    Vec2::new(art.width(), art.height() / bands as f32 + 0.5),
                );
                x.quad(p, band, rgba([mix(0), mix(1), mix(2)], if lit { 0.5 } else { 0.25 }));
            }
        }
    }
    x.outline(p, art, 1.0, rgba(edge, 0.8));

    // The key, as a badge on the art's top-right corner.
    let key_size = font(10.0);
    let key_w = (c.key.chars().count() as f32 * key_size * 0.6 + 6.0 * u * k).max(14.0 * u * k);
    let key_box = Rect::from_min_size(
        Pos2::new(art.right() - key_w - 2.0 * u, art.top() + 2.0 * u),
        Vec2::new(key_w, key_size + 4.0 * u),
    );
    x.quad(p, key_box, rgba([22, 14, 44], 0.92));
    x.outline(p, key_box, 1.0, rgba(edge, 0.9));
    x.text(p, key_box.center().to_vec2(), c.key, key_size, rgba(edge, 1.0), angle);

    // Recharge: a strip under the art, eight cells; ready ones pulse.
    let cells = 8;
    let filled = (c.charge.clamp(0.0, 1.0) * cells as f32).floor() as i32;
    let cw = art.width() / cells as f32;
    let bar_top = art.bottom() + 3.0 * u * k;
    for i in 0..cells {
        let cr = Rect::from_min_size(
            Pos2::new(art.left() + i as f32 * cw, bar_top),
            Vec2::new(cw - 1.0, 5.0 * u * k),
        );
        let a = if i < filled || ready {
            if ready { 0.7 + 0.3 * (time * 5.0 + i as f32 * 0.5).sin() } else { 0.9 }
        } else {
            0.15
        };
        x.quad(p, cr, rgba(edge, a));
    }

    // The ability this card holds, in the lower body.
    let lower = bar_top + 5.0 * u * k + 4.0 * u * k;
    let ability_size = crate::pixel_ui::fit(c.ability, font(12.0), inner.width(), 6.0);
    x.text(
        p,
        Vec2::new(0.0, (lower + inner.bottom()) / 2.0),
        c.ability,
        ability_size,
        rgba(if lit { [255, 245, 255] } else { [170, 150, 200] }, 0.95),
        angle,
    );

    // The flash: a tinted wash that fades with the pop.
    if amount > 0.0 {
        x.quad(p, body, rgba(c.tint.map(|v| v / 2 + 127), 0.35 * amount));
    }
}

/// `text` cut to `max` characters, ending in a dot if it had to be.
fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('.');
    out
}

/// A name on one line if it fits, else wrapped on a space into two (the
/// second clipped), each at most `per_line` characters.
fn name_lines(name: &str, per_line: usize) -> (String, Option<String>) {
    if name.chars().count() <= per_line {
        return (name.to_string(), None);
    }
    let mut first = String::new();
    let mut rest: Vec<&str> = Vec::new();
    for (i, word) in name.split_whitespace().enumerate() {
        let joined = if first.is_empty() {
            word.len()
        } else {
            first.chars().count() + 1 + word.chars().count()
        };
        if rest.is_empty() && joined <= per_line && (i == 0 || !first.is_empty()) {
            if !first.is_empty() {
                first.push(' ');
            }
            first.push_str(word);
        } else {
            rest.push(word);
        }
    }
    if first.is_empty() {
        // One word longer than a line: cut it.
        return (clip(name, per_line), None);
    }
    (first, Some(clip(&rest.join(" "), per_line)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0))
    }

    #[test]
    fn pop_rises_holds_and_settles() {
        assert_eq!(pop(NEVER), 0.0);
        assert_eq!(pop(-1.0), 0.0);
        assert_eq!(pop(0.0), 0.0);
        assert!((pop(POP_RISE) - 1.0).abs() < 1e-5);
        assert!((pop(POP_RISE + POP_HOLD * 0.5) - 1.0).abs() < 1e-5);
        let mid = pop(POP_RISE + POP_HOLD + POP_FALL * 0.5);
        assert!(mid > 0.0 && mid < 1.0);
        assert_eq!(pop(POP_TOTAL), 0.0);
        assert_eq!(pop(POP_TOTAL + 5.0), 0.0);
    }

    #[test]
    fn pop_never_leaves_zero_to_one() {
        let mut t = 0.0;
        while t < POP_TOTAL + 0.1 {
            let v = pop(t);
            assert!((0.0..=1.0).contains(&v), "pop({t}) = {v}");
            t += 0.005;
        }
    }

    #[test]
    fn ring_runs_once() {
        assert_eq!(ring(NEVER), None);
        assert_eq!(ring(-0.1), None);
        assert!(ring(0.25).is_some_and(|t| (t - 0.5).abs() < 1e-5));
        assert_eq!(ring(RING_SECS), None);
    }

    #[test]
    fn an_empty_hand_has_no_layout() {
        assert!(layout(screen(), 0).is_empty());
    }

    #[test]
    fn the_hand_stays_on_screen_even_tilted_for_every_size() {
        for (w, h) in [(1280.0, 720.0), (1280.0, 800.0), (1920.0, 1080.0), (960.0, 540.0)] {
            let s = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
            for n in 1..=crate::lore::MAX_SLOTS {
                let rects = layout(s, n);
                assert_eq!(rects.len(), n);
                for (i, r) in rects.iter().enumerate() {
                    for c in corners(*r, tilt(i, n)) {
                        assert!(c.x <= s.right(), "{n} cards at {w}x{h}: off the right");
                        assert!(c.x > s.center().x, "{n} cards at {w}x{h}: past the middle");
                        assert!(c.y >= s.top() && c.y <= s.bottom(), "{n} at {w}x{h}: {c:?}");
                    }
                }
                for pair in rects.windows(2) {
                    assert!(pair[1].top() > pair[0].top());
                }
            }
        }
    }

    #[test]
    fn cards_overlap_like_a_held_hand_but_the_top_stays_readable() {
        let rects = layout(screen(), crate::lore::MAX_SLOTS);
        for pair in rects.windows(2) {
            let shown = pair[1].top() - pair[0].top();
            assert!(shown < pair[0].height(), "cards should overlap");
            // The name, art and recharge strip (the top 60%) always show.
            assert!(shown >= pair[0].height() * 0.6, "shown {shown}");
        }
    }

    #[test]
    fn the_hand_fans_from_leaning_back_to_leaning_forward() {
        assert_eq!(tilt(0, 1), 0.0);
        assert_eq!(tilt(1, 3), 0.0);
        assert!(tilt(0, 3) < 0.0 && tilt(2, 3) > 0.0);
        assert!((tilt(0, 5) + tilt(4, 5)).abs() < 1e-6);
        assert!(tilt(4, 5) < 0.4, "a fan, not a spin");
    }

    #[test]
    fn the_middle_card_bows_toward_the_centre() {
        let rects = layout(screen(), 3);
        assert!(rects[1].left() < rects[0].left());
        assert!(rects[1].left() < rects[2].left());
        assert_eq!(rects[0].left(), rects[2].left());
    }

    #[test]
    fn popping_moves_toward_the_centre_and_grows() {
        let rest = layout(screen(), 2)[0];
        let up = popped(rest, 1.0, 1.0, false);
        assert!(up.center().x < rest.center().x);
        assert!(up.width() > rest.width());
        assert_eq!(popped(rest, 0.0, 1.0, false), rest);
    }

    #[test]
    fn reduced_motion_keeps_the_card_still() {
        let rest = layout(screen(), 2)[0];
        assert_eq!(popped(rest, 1.0, 1.0, true), rest);
    }

    #[test]
    fn a_turn_keeps_the_centre_and_quarter_turns_swap_axes() {
        let r = Rect::from_center_size(Pos2::new(500.0, 300.0), Vec2::new(40.0, 20.0));
        let flat = corners(r, 0.0);
        assert_eq!(flat[0], r.left_top());
        let q = corners(r, std::f32::consts::FRAC_PI_2);
        let mid = (q[0].to_vec2() + q[2].to_vec2()) / 2.0;
        assert!((mid - r.center().to_vec2()).length() < 1e-3);
        assert!((q[1].y - q[0].y - 40.0).abs() < 1e-3);
    }

    #[test]
    fn short_names_stay_on_one_line_and_long_ones_wrap_in_two() {
        assert_eq!(name_lines("the lobby", 12), ("the lobby".to_string(), None));
        let (a, b) = name_lines("the opulent corridor of late checkout", 14);
        assert_eq!(a, "the opulent");
        assert!(b.is_some_and(|b| b.chars().count() <= 14 && b.ends_with('.')));
        let (a, b) = name_lines("supercalifragilistic", 8);
        assert_eq!(b, None);
        assert_eq!(a.chars().count(), 8);
    }
}
