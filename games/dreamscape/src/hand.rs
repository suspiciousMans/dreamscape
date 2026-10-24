//! The hand: the equipped cards that hold abilities, fanned along the bottom
//! centre of the screen like cards held in a hand: overlapping, each tilted a
//! little more than the last. A card straightens, rises, flashes and rings
//! when its ability is used.
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

/// Card size at 720p, and how much of each card's width the next leaves showing.
const CARD_W: f32 = 92.0;
const CARD_H: f32 = 118.0;
const VISIBLE: f32 = 0.82;
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
    /// The passive this card gives: name and colour.
    pub passive: Option<(&'static str, [u8; 3])>,
    /// Seconds since that passive last did something; [`NEVER`] if it has not.
    pub passive_since: f32,
    /// The combat edge this card's dream teaches, and when it last did something.
    pub edge: Option<(&'static str, [u8; 3])>,
    pub edge_since: f32,
    /// The run holds a curse: the card shows cracks.
    pub cracked: bool,
    /// Deck hands: seconds since this card was drawn; [`NEVER`] if not freshly.
    pub since_draw: f32,
    /// Deck hands: nothing in this slot right now (waiting to draw).
    pub empty: bool,
}

/// Two cards (or one, if `a == b`) that make a synergy together.
#[derive(Clone, Debug, PartialEq)]
pub struct HandLink {
    pub a: usize,
    pub b: usize,
    pub name: &'static str,
}

/// Seconds a passive's pulse and its floating name last.
pub const PULSE_SECS: f32 = 0.9;

/// 1 the moment a passive fires, easing to 0.
pub fn pulse(since: f32) -> f32 {
    if !(0.0..PULSE_SECS).contains(&since) {
        return 0.0;
    }
    let t = since / PULSE_SECS;
    (1.0 - t) * (1.0 - t)
}

/// A curve from `a` to `b` bowed up by `lift`, as `n + 1` points.
pub fn arc(a: Pos2, b: Pos2, lift: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let line = a + (b - a) * t;
            Pos2::new(line.x, line.y - lift * 4.0 * t * (1.0 - t))
        })
        .collect()
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
/// the left cards lean left, the right ones lean right, the middle stands.
pub fn tilt(i: usize, n: usize) -> f32 {
    (i as f32 - (n as f32 - 1.0) / 2.0) * TILT_STEP
}

/// Where each of `n` cards rests (before tilt): overlapped along the bottom
/// centre, the middle ones held a little higher, like a hand of cards.
pub fn layout(screen: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return Vec::new();
    }
    let u = crate::pixel_ui::scale(screen);
    let (w, h) = (CARD_W * u, CARD_H * u);
    let room = (screen.width() * 0.55).max(w);
    let mut step = w * VISIBLE;
    if w + (n as f32 - 1.0) * step > room && n > 1 {
        step = (room - w) / (n as f32 - 1.0);
    }
    let total = w + (n as f32 - 1.0) * step;
    let left = screen.center().x - total / 2.0;
    let base = screen.bottom() - 26.0 * u - h;
    let mid = (n as f32 - 1.0) / 2.0;
    (0..n)
        .map(|i| {
            let t = if mid > 0.0 { (i as f32 - mid) / mid } else { 0.0 };
            let sag = 14.0 * u * t * t;
            Rect::from_min_size(Pos2::new(left + i as f32 * step, base + sag), Vec2::new(w, h))
        })
        .collect()
}

/// The card's rect while popped: up toward the screen and a little larger.
/// Reduced motion keeps it in place.
pub fn popped(rest: Rect, amount: f32, u: f32, reduced: bool) -> Rect {
    if reduced || amount <= 0.0 {
        return rest;
    }
    let grown = Rect::from_center_size(rest.center(), rest.size() * (1.0 + 0.16 * amount));
    grown.translate(Vec2::new(0.0, -130.0 * u * amount))
}

// === deck === (the deck's hand: draw, flick, riffle, sway)

/// Seconds a drawn card takes from the stack to its place.
pub const DRAW_SECS: f32 = 0.40;
/// Seconds a used card takes to flick up to the discard spot.
pub const FLICK_SECS: f32 = 0.28;
/// Seconds it then takes to slide into the stack, which riffles meanwhile.
pub const RIFFLE_SECS: f32 = 0.42;
pub const LEAVE_TOTAL: f32 = FLICK_SECS + RIFFLE_SECS;
/// Sheets that interleave when the stack shuffles.
pub const RIFFLE_SHEETS: usize = 3;

/// Where a card is drawn: its rectangle and an extra turn (radians, fading
/// to 0 as it settles).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub rect: Rect,
    pub angle_off: f32,
}

/// One sheet of the riffle, offset from the stack (`dx`/`dy` as fractions of
/// a card width; `angle` in radians).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sheet {
    pub dx: f32,
    pub dy: f32,
    pub angle: f32,
}

/// A card on its way out, snapshotted when it was used.
#[derive(Clone, Debug)]
pub struct Leaving {
    pub card: HandCard,
    pub slot: usize,
    /// Seconds since it was used.
    pub since: f32,
}

/// The piles beside the hand, as the HUD sees them. `Default` = no deck.
#[derive(Clone, Debug)]
pub struct HandPile {
    /// Draw a stack at all (a deck run).
    pub shown: bool,
    pub draw_count: usize,
    /// Seconds since a card was last shuffled in; [`NEVER`] if none.
    pub riffle_since: f32,
    pub leaving: Vec<Leaving>,
}

impl Default for HandPile {
    fn default() -> Self {
        Self {
            shown: false,
            draw_count: 0,
            riffle_since: NEVER,
            leaving: Vec::new(),
        }
    }
}

/// Eases out past 1 and back: a small overshoot (about 8%) then rest.
pub fn ease_out_back(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let (c1, c3) = (1.2_f32, 2.2_f32);
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The draw stack: right of the hand, a little smaller than a card.
pub fn stack_rect(screen: Rect, rests: &[Rect], u: f32) -> Rect {
    let (w, h) = (CARD_W * 0.8 * u, CARD_H * 0.8 * u);
    let (left, bottom) = rests.iter().fold((screen.center().x, screen.bottom() - 26.0 * u), |a, r| {
        (a.0.max(r.right()), a.1.max(r.bottom()))
    });
    let x = (left + 28.0 * u).min(screen.right() - w - 10.0 * u);
    Rect::from_min_size(Pos2::new(x, bottom - h), Vec2::new(w, h))
}

/// Where a used card lands before it is shuffled in: above the stack.
pub fn discard_spot(stack: Rect, u: f32) -> Pos2 {
    Pos2::new(stack.center().x, stack.center().y - CARD_H * 0.95 * u)
}

/// A card being drawn: slides from the stack along a small arc, settles with
/// a tiny overshoot, turning flat as it lands. Reduced motion: already there.
pub fn draw_pose(since: f32, rest: Rect, stack: Rect, u: f32, reduced: bool) -> Pose {
    let at_rest = Pose { rect: rest, angle_off: 0.0 };
    if reduced || !(0.0..DRAW_SECS).contains(&since) {
        return at_rest;
    }
    let t = since / DRAW_SECS;
    let p = ease_out_back(t);
    let from = stack.center();
    let to = rest.center();
    let mut c = from + (to - from) * p;
    c.y -= 44.0 * u * (std::f32::consts::PI * t).sin();
    let scale = 0.7 + 0.3 * smooth(t * 1.4);
    Pose {
        rect: Rect::from_center_size(c, rest.size() * scale),
        angle_off: 0.7 * (1.0 - smooth(t)),
    }
}

/// A used card: flicks up and out to the discard spot (turning over), then
/// slides into the stack shrinking. `None` once it is in, or when reduced.
pub fn leave_pose(since: f32, rest: Rect, stack: Rect, u: f32, reduced: bool) -> Option<Pose> {
    if reduced || !(0.0..LEAVE_TOTAL).contains(&since) {
        return None;
    }
    let spot = discard_spot(stack, u);
    if since < FLICK_SECS {
        let t = since / FLICK_SECS;
        let e = 1.0 - (1.0 - t).powi(3); // fast out
        let mut c = rest.center() + (spot - rest.center()) * e;
        c.y -= 30.0 * u * (std::f32::consts::PI * t).sin();
        return Some(Pose {
            rect: Rect::from_center_size(c, rest.size() * (1.0 + 0.08 * (1.0 - t))),
            angle_off: 0.8 * e,
        });
    }
    let t = (since - FLICK_SECS) / RIFFLE_SECS;
    let e = smooth(t);
    let c = spot + (stack.center() - spot) * e;
    let size = rest.size() + (stack.size() - rest.size()) * e;
    Some(Pose {
        rect: Rect::from_center_size(c, size * (1.0 - 0.15 * e)),
        angle_off: 0.8 * (1.0 - e),
    })
}

/// The sheets of a riffle shuffle: the stack splits in two, then the halves
/// interleave back into one. Even sheets go left, odd ones right.
pub fn riffle(since: f32, reduced: bool) -> [Sheet; RIFFLE_SHEETS] {
    let still = Sheet { dx: 0.0, dy: 0.0, angle: 0.0 };
    let mut out = [still; RIFFLE_SHEETS];
    if reduced || !(0.0..RIFFLE_SECS).contains(&since) {
        return out;
    }
    let t = since / RIFFLE_SECS;
    let spread = (std::f32::consts::PI * t).sin();
    // Past the midpoint the sheets drop in one after another.
    for (k, s) in out.iter_mut().enumerate() {
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let drop = smooth((t - 0.4 - 0.15 * k as f32) * 4.0);
        let k_spread = spread * (1.0 - drop * 0.5);
        *s = Sheet {
            dx: side * 0.55 * k_spread,
            dy: -0.08 * (k as f32 + 1.0) * spread,
            angle: side * 0.14 * k_spread,
        };
    }
    out
}

/// Idle sway of card `i`, in radians: a few degrees, each out of step.
pub fn sway(time: f32, i: usize, reduced: bool) -> f32 {
    if reduced {
        return 0.0;
    }
    0.02 * (time * 1.3 + i as f32 * 1.9).sin()
}

/// 0..1: a ready card breathes up and down a little. Not ready: flat.
pub fn ready_lift(ready: bool, time: f32, i: usize, reduced: bool) -> f32 {
    if reduced || !ready {
        return 0.0;
    }
    0.5 + 0.5 * (time * 2.0 + i as f32 * 1.3).sin()
}

/// How many card backs the stack shows for a draw pile of `n`.
pub fn stack_layers(n: usize) -> usize {
    n.min(4)
}
// === end deck ===

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
    links: &[HandLink],
    pile: &HandPile,
    time: f32,
    reduced: bool,
    art: &mut crate::booklet_ui::CardArt,
) {
    let rests = layout(screen, cards.len());
    let u = crate::pixel_ui::scale(screen);
    draw_links(p, &rests, cards, links, time, u);
    // === deck ===
    let stack = pile.shown.then(|| stack_rect(screen, &rests, u));
    if let Some(s) = stack {
        draw_stack(p, s, pile, u, time, reduced);
    }
    // === end deck ===
    // Rest order (later cards lie on top), then whichever is mid-pop above all.
    let rise = |c: &HandCard| pop(c.since_use).max(if (0.0..DRAW_SECS).contains(&c.since_draw) { 1.0 } else { 0.0 });
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by(|&a, &b| {
        rise(&cards[a])
            .partial_cmp(&rise(&cards[b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for i in order {
        let c = &cards[i];
        if c.empty {
            continue;
        }
        let amount = pop(c.since_use);
        let mut r = popped(rests[i], amount, u, reduced);
        let mut angle = tilt(i, cards.len()) * (1.0 - amount);
        // === deck === idle sway, ready lift, and the draw from the stack
        if let Some(s) = stack {
            let lift = ready_lift(c.charge >= 1.0 && !c.active, time, i, reduced);
            r = r.translate(Vec2::new(0.0, -7.0 * u * lift));
            angle += sway(time, i, reduced);
            if (0.0..DRAW_SECS).contains(&c.since_draw) {
                let pose = draw_pose(c.since_draw, rests[i], s, u, reduced);
                r = pose.rect;
                angle += pose.angle_off;
            }
        }
        // === end deck ===
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
        // The edge's name floats up too, higher than the passive's.
        let eu = pulse(c.edge_since);
        if let (Some((name, rgb)), true) = (c.edge, eu > 0.0) {
            let rise = if reduced { 0.0 } else { 22.0 * u * (1.0 - eu) };
            p.text(
                Pos2::new(r.center().x, r.top() - 26.0 * u - rise),
                egui::Align2::CENTER_BOTTOM,
                name,
                FontId::monospace((12.0 * u).max(8.0)),
                rgba(rgb, eu.min(1.0)),
            );
        }
        // The passive's name floats up off the card and fades.
        let pu = pulse(c.passive_since);
        if let (Some((name, rgb)), true) = (c.passive, pu > 0.0) {
            let rise = if reduced { 0.0 } else { 22.0 * u * (1.0 - pu) };
            p.text(
                Pos2::new(r.center().x, r.top() - 10.0 * u - rise),
                egui::Align2::CENTER_BOTTOM,
                name,
                FontId::monospace((12.0 * u).max(8.0)),
                rgba(rgb, pu.min(1.0)),
            );
        }
    }
    // === deck === cards flicking out to the discard and into the stack
    if let Some(s) = stack {
        for l in &pile.leaving {
            let (Some(&rest), Some(pose)) = (
                rests.get(l.slot),
                rests
                    .get(l.slot)
                    .and_then(|&r| leave_pose(l.since, r, s, u, reduced)),
            ) else {
                continue;
            };
            if let Some(t) = ring(l.since) {
                let grow = if reduced { 0.0 } else { 26.0 * u * t };
                Xf::new(rest.center(), tilt(l.slot, cards.len())).outline(
                    p,
                    Rect::from_center_size(Pos2::ZERO, rest.size()).expand(grow),
                    2.0 * u,
                    rgba(l.card.tint, 0.8 * (1.0 - t)),
                );
            }
            let angle = tilt(l.slot, cards.len()) * (1.0 - (l.since / FLICK_SECS).min(1.0)) + pose.angle_off;
            draw_card(ctx, p, pose.rect, angle, &l.card, 1.0, time, u, art);
        }
    }
    // === end deck ===
}

// === deck ===
/// The draw pile: card backs stacked, a count badge, and the riffle.
fn draw_stack(p: &egui::Painter, stack: Rect, pile: &HandPile, u: f32, time: f32, reduced: bool) {
    let back = |x: &Xf, r: Rect, a: f32| {
        x.quad(p, r.translate(Vec2::new(-2.0 * u, 2.0 * u)), Color32::from_black_alpha((90.0 * a) as u8));
        x.quad(p, r, rgba([150, 120, 210], a));
        x.quad(p, r.shrink(2.5 * u), rgba([26, 14, 52], a));
        x.outline(p, r.shrink(6.0 * u), 1.0, rgba([150, 120, 210], 0.45 * a));
        let c = Rect::from_center_size(r.center(), Vec2::splat(9.0 * u));
        x.quad(p, c, rgba([150, 120, 210], 0.35 * a));
    };
    let layers = stack_layers(pile.draw_count);
    let body = Rect::from_center_size(Pos2::ZERO, stack.size());
    if layers == 0 {
        Xf::new(stack.center(), 0.0).outline(p, body, 1.5 * u, rgba([150, 120, 210], 0.3));
    }
    for k in 0..layers {
        let off = (layers - 1 - k) as f32;
        let at = stack.center() + Vec2::new(-1.8 * u * off, 1.8 * u * off);
        back(&Xf::new(at, 0.0), body, 1.0);
    }
    // The riffle: sheets split and interleave on top of the stack.
    for (k, s) in riffle(pile.riffle_since, reduced).iter().enumerate() {
        let moving = s.dx != 0.0 || s.dy != 0.0 || s.angle != 0.0;
        if !moving {
            continue;
        }
        let at = stack.center() + Vec2::new(s.dx * stack.width(), s.dy * stack.width() - k as f32 * 1.5 * u);
        back(&Xf::new(at, s.angle), body, 0.95);
    }
    // The badge.
    let badge = Pos2::new(stack.right() - 2.0 * u, stack.top() + 2.0 * u);
    let (cx, cy) = (badge.x, badge.y);
    p.circle_filled(Pos2::new(cx, cy), 10.0 * u, rgba([20, 10, 40], 0.95));
    p.circle_stroke(Pos2::new(cx, cy), 10.0 * u, Stroke::new(1.5 * u, rgba([180, 150, 240], 0.9)));
    p.text(
        Pos2::new(cx, cy),
        egui::Align2::CENTER_CENTER,
        pile.draw_count.to_string(),
        FontId::monospace((11.0 * u).max(8.0)),
        rgba([235, 225, 255], 1.0),
    );
    let pulse_a = if reduced { 0.55 } else { 0.5 + 0.1 * (time * 2.0).sin() };
    p.text(
        Pos2::new(stack.center().x, stack.bottom() + 3.0 * u),
        egui::Align2::CENTER_TOP,
        "DECK",
        FontId::monospace((9.0 * u).max(7.0)),
        rgba([190, 170, 235], pulse_a),
    );
}
// === end deck ===

/// Synergies: a glowing arc between the two cards that make one, with its
/// name above; one card that makes it alone just shimmers its name.
fn draw_links(
    p: &egui::Painter,
    rests: &[Rect],
    cards: &[HandCard],
    links: &[HandLink],
    time: f32,
    u: f32,
) {
    for (k, l) in links.iter().enumerate() {
        let (Some(ra), Some(rb)) = (rests.get(l.a), rests.get(l.b)) else {
            continue;
        };
        let pop_a = cards.get(l.a).map_or(0.0, |c| pop(c.since_use));
        let pop_b = cards.get(l.b).map_or(0.0, |c| pop(c.since_use));
        if pop_a > 0.0 || pop_b > 0.0 {
            continue; // a card in the air: let it have the screen
        }
        let rgb = crate::hud::hue(time * 0.25 + k as f32 * 0.37);
        let glow = 0.55 + 0.25 * (time * 3.0 + k as f32).sin();
        let lift = 18.0 * u + 6.0 * u * k as f32;
        let pa = Pos2::new(ra.center().x, ra.top() - 3.0 * u);
        let pb = Pos2::new(rb.center().x, rb.top() - 3.0 * u);
        let label_at = if l.a == l.b {
            Pos2::new(pa.x, pa.y - 4.0 * u)
        } else {
            let pts = arc(pa, pb, lift, 14);
            p.add(Shape::line(pts.clone(), Stroke::new(4.0 * u, rgba(rgb, glow * 0.25))));
            p.add(Shape::line(pts, Stroke::new(1.5 * u, rgba(rgb, glow + 0.2))));
            for end in [pa, pb] {
                p.circle_filled(end, 2.5 * u, rgba(rgb, 0.95));
            }
            Pos2::new((pa.x + pb.x) / 2.0, pa.y.min(pb.y) - lift - 2.0 * u)
        };
        p.text(
            label_at,
            egui::Align2::CENTER_BOTTOM,
            l.name,
            FontId::monospace((10.0 * u).max(7.0)),
            rgba(rgb, glow + 0.2),
        );
    }
}

/// Jagged cracks across a card face, in local coordinates (the same on every
/// card, so they do not shimmer).
fn draw_cracks(p: &egui::Painter, x: &Xf, body: Rect, u: f32, k: f32) {
    let w = body.width();
    let h = body.height();
    let spine = [(-0.16, -0.5), (-0.04, -0.3), (-0.2, -0.12), (0.02, 0.08), (-0.1, 0.28), (0.06, 0.5)];
    let branch = [(-0.04, -0.3), (0.16, -0.22), (0.3, -0.04)];
    for line in [&spine[..], &branch[..]] {
        let pts: Vec<Pos2> = line
            .iter()
            .map(|&(fx, fy)| x.at(Vec2::new(fx * w, fy * h)))
            .collect();
        p.add(Shape::line(pts.clone(), Stroke::new(3.5 * u * k, rgba([255, 40, 70], 0.28))));
        p.add(Shape::line(pts, Stroke::new(1.3 * u * k, rgba([6, 0, 10], 0.95))));
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
    let edge = if c.cracked {
        [0, 1, 2].map(|i| (edge[i] as u16 * 3 / 5 + [255u16, 40, 70][i] * 2 / 5) as u8)
    } else {
        edge
    };
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
    // Resting cards are overlapped on the right, so their words keep to the
    // part that shows; a popped card has nothing on top of it.
    let shown = (CARD_W * u * k * VISIBLE - cell * 3.0).min(inner.width());
    let text_w = shown + (inner.width() - shown) * amount;
    let text_x = inner.left() + text_w / 2.0;

    // The card's name across the top, on up to two lines.
    let name = c.card.as_ref().map_or(c.ability, |card| card.name.as_str());
    let name_h = 24.0 * u * k;
    let name_size = (10.0 * u * k).max(7.0);
    let per_line = (text_w / (name_size * 0.6)).floor().max(4.0) as usize;
    let lines = name_lines(name, per_line);
    let line_h = name_h / 2.0;
    let first = if lines.1.is_some() { 0.0 } else { 0.5 };
    for (n, line) in std::iter::once(&lines.0).chain(lines.1.iter()).enumerate() {
        x.text(
            p,
            Vec2::new(text_x, inner.top() + line_h * (first + n as f32) + line_h / 2.0),
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

    // The key, as a badge on the art's top-left corner (the right is covered).
    let key_size = font(10.0);
    let key_w = (c.key.chars().count() as f32 * key_size * 0.6 + 6.0 * u * k).max(14.0 * u * k);
    let key_box = Rect::from_min_size(
        Pos2::new(art.left() + 2.0 * u, art.top() + 2.0 * u),
        Vec2::new(key_w, key_size + 4.0 * u),
    );
    x.quad(p, key_box, rgba([22, 14, 44], 0.92));
    x.outline(p, key_box, 1.0, rgba(edge, 0.9));
    x.text(p, key_box.center().to_vec2(), c.key, key_size, rgba(edge, 1.0), angle);

    // The passive: a small diamond on the art's bottom-left, flaring when it fires.
    if let Some((_, rgb)) = c.passive {
        let pu = pulse(c.passive_since);
        let s = (3.5 + 3.0 * pu) * u * k;
        let at = Pos2::new(art.left() + 7.0 * u * k, art.bottom() - 7.0 * u * k).to_vec2();
        let diamond = Rect::from_center_size(at.to_pos2(), Vec2::splat(s * 2.0));
        x.quad(p, diamond.expand(1.5 * u), rgba([10, 4, 20], 0.9));
        let turn = Xf::new(x.at(at), angle + std::f32::consts::FRAC_PI_4);
        let sq = Rect::from_center_size(Pos2::ZERO, Vec2::splat(s * 1.3));
        turn.quad(p, sq, rgba(rgb, 0.75 + 0.25 * pu));
        if pu > 0.0 {
            turn.outline(p, sq.expand(10.0 * u * (1.0 - pu)), 1.5, rgba(rgb, pu));
        }
    }

    // The edge: a red blade-shaped badge on the art's bottom-right, flaring when it acts.
    if let Some((_, rgb)) = c.edge {
        let eu = pulse(c.edge_since);
        let s = (3.5 + 3.0 * eu) * u * k;
        let at = Pos2::new(art.right() - 7.0 * u * k, art.bottom() - 7.0 * u * k).to_vec2();
        let blade = Rect::from_center_size(at.to_pos2(), Vec2::new(s * 1.1, s * 2.4));
        x.quad(p, blade.expand(1.5 * u), rgba([10, 4, 20], 0.9));
        let turn = Xf::new(x.at(at), angle + std::f32::consts::FRAC_PI_4 * 0.5);
        let sq = Rect::from_center_size(Pos2::ZERO, Vec2::new(s * 0.8, s * 2.0));
        turn.quad(p, sq, rgba(rgb, 0.8 + 0.2 * eu));
        if eu > 0.0 {
            turn.outline(p, sq.expand(10.0 * u * (1.0 - eu)), 1.5, rgba(rgb, eu));
        }
    }

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
        Vec2::new(text_x, (lower + inner.bottom()) / 2.0),
        c.ability,
        ability_size,
        rgba(if lit { [255, 245, 255] } else { [170, 150, 200] }, 0.95),
        angle,
    );

    if c.cracked {
        draw_cracks(p, &x, body, u, k);
    }

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
                        assert!(c.x >= s.left() && c.x <= s.right(), "{n} at {w}x{h}: {c:?}");
                        assert!(c.y >= s.top() && c.y <= s.bottom(), "{n} at {w}x{h}: {c:?}");
                        assert!(c.y > s.center().y, "{n} at {w}x{h}: stays in the lower half");
                    }
                }
                for pair in rects.windows(2) {
                    assert!(pair[1].left() > pair[0].left());
                }
            }
        }
    }

    #[test]
    fn the_hand_is_centred_along_the_bottom() {
        for n in 1..=crate::lore::MAX_SLOTS {
            let rects = layout(screen(), n);
            let l = rects.iter().map(|r| r.left()).fold(f32::MAX, f32::min);
            let r = rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max);
            assert!(((l + r) / 2.0 - screen().center().x).abs() < 0.5, "n = {n}");
        }
    }

    #[test]
    fn cards_overlap_like_a_held_hand_but_the_left_edge_stays_readable() {
        let rects = layout(screen(), crate::lore::MAX_SLOTS);
        for pair in rects.windows(2) {
            let shown = pair[1].left() - pair[0].left();
            assert!(shown < pair[0].width(), "cards should overlap");
            assert!(shown >= pair[0].width() * 0.6, "shown {shown}");
        }
    }

    #[test]
    fn the_hand_fans_from_leaning_left_to_leaning_right() {
        assert_eq!(tilt(0, 1), 0.0);
        assert_eq!(tilt(1, 3), 0.0);
        assert!(tilt(0, 3) < 0.0 && tilt(2, 3) > 0.0);
        assert!((tilt(0, 5) + tilt(4, 5)).abs() < 1e-6);
        assert!(tilt(4, 5) < 0.4, "a fan, not a spin");
    }

    #[test]
    fn the_middle_card_is_held_highest() {
        let rects = layout(screen(), 3);
        assert!(rects[1].top() < rects[0].top());
        assert!(rects[1].top() < rects[2].top());
        assert_eq!(rects[0].top(), rects[2].top());
    }

    fn stack_of(rests: &[Rect]) -> Rect {
        stack_rect(screen(), rests, 1.0)
    }

    #[test]
    fn ease_out_back_overshoots_a_little_then_lands() {
        assert_eq!(ease_out_back(0.0), 0.0);
        assert!((ease_out_back(1.0) - 1.0).abs() < 1e-5);
        let peak = (0..=100).map(|i| ease_out_back(i as f32 / 100.0)).fold(0.0, f32::max);
        assert!(peak > 1.02 && peak < 1.15, "tiny overshoot, got {peak}");
    }

    #[test]
    fn the_stack_sits_right_of_the_hand_on_screen() {
        for n in 1..=3 {
            let rests = layout(screen(), n);
            let s = stack_of(&rests);
            let last = rests.last().unwrap();
            assert!(s.left() > last.right(), "n = {n}");
            assert!(screen().contains_rect(s), "n = {n}: {s:?}");
        }
    }

    #[test]
    fn a_drawn_card_starts_on_the_stack_and_ends_at_rest() {
        let rests = layout(screen(), 3);
        let stack = stack_of(&rests);
        let start = draw_pose(0.0, rests[1], stack, 1.0, false);
        assert!((start.rect.center() - stack.center()).length() < 1.0);
        assert!(start.rect.width() < rests[1].width(), "small on the stack");
        let end = draw_pose(DRAW_SECS, rests[1], stack, 1.0, false);
        assert_eq!(end.rect, rests[1]);
        assert_eq!(end.angle_off, 0.0);
        assert_eq!(draw_pose(NEVER, rests[1], stack, 1.0, false).rect, rests[1]);
    }

    #[test]
    fn a_drawn_card_arcs_above_the_straight_line_and_overshoots_its_rest() {
        let rests = layout(screen(), 3);
        let stack = stack_of(&rests);
        let rest = rests[0];
        let mid = draw_pose(DRAW_SECS * 0.5, rest, stack, 1.0, false);
        let line_y = (stack.center().y + rest.center().y) / 2.0;
        assert!(mid.rect.center().y < line_y - 5.0, "an arc, not a line");
        // Moving left from the stack: somewhere it is further left than rest.
        let over = (0..=100)
            .map(|i| draw_pose(DRAW_SECS * i as f32 / 100.0, rest, stack, 1.0, false).rect.center().x)
            .fold(f32::MAX, f32::min);
        assert!(over < rest.center().x, "settles with an overshoot");
        assert!(rest.center().x - over < rest.width() * 0.2, "a small one");
    }

    #[test]
    fn reduced_motion_moves_instantly() {
        let rests = layout(screen(), 3);
        let stack = stack_of(&rests);
        assert_eq!(draw_pose(0.05, rests[2], stack, 1.0, true).rect, rests[2]);
        assert!(leave_pose(0.05, rests[2], stack, 1.0, true).is_none());
        assert!(riffle(0.1, true).iter().all(|s| s.dx == 0.0 && s.dy == 0.0 && s.angle == 0.0));
        assert_eq!(sway(1.0, 1, true), 0.0);
        assert_eq!(ready_lift(true, 1.0, 1, true), 0.0);
    }

    #[test]
    fn a_used_card_flicks_up_out_then_slides_into_the_stack() {
        let rests = layout(screen(), 3);
        let stack = stack_of(&rests);
        let rest = rests[1];
        let at = |t: f32| leave_pose(t, rest, stack, 1.0, false).unwrap();
        assert_eq!(at(0.0).rect.center(), rest.center());
        let spot = discard_spot(stack, 1.0);
        let flicked = at(FLICK_SECS);
        assert!((flicked.rect.center() - spot).length() < 1.0, "the flick ends at the discard spot");
        assert!(spot.y < rest.center().y, "up and out");
        let into = at(FLICK_SECS + RIFFLE_SECS * 0.99);
        assert!((into.rect.center() - stack.center()).length() < 12.0);
        assert!(into.rect.width() < rest.width());
        assert!(leave_pose(LEAVE_TOTAL, rest, stack, 1.0, false).is_none());
        assert!(leave_pose(NEVER, rest, stack, 1.0, false).is_none());
        assert!(leave_pose(-0.1, rest, stack, 1.0, false).is_none());
    }

    #[test]
    fn the_riffle_splits_interleaves_and_comes_back_together() {
        assert_eq!(riffle(NEVER, false).len(), RIFFLE_SHEETS);
        for s in riffle(NEVER, false).iter().chain(riffle(RIFFLE_SECS, false).iter()) {
            assert_eq!((s.dx, s.dy, s.angle), (0.0, 0.0, 0.0), "square when still");
        }
        let mid = riffle(RIFFLE_SECS * 0.5, false);
        assert!(mid[0].dx * mid[1].dx < 0.0, "neighbouring sheets sit on opposite sides");
        assert!(mid[0].dx * mid[2].dx > 0.0, "every other sheet shares a side");
        let spread = mid.iter().map(|s| s.dx.abs()).fold(0.0, f32::max);
        assert!(spread > 0.5 && spread <= 1.0);
    }

    #[test]
    fn ready_cards_lift_and_idle_cards_sway_gently() {
        assert_eq!(ready_lift(false, 3.0, 0, false), 0.0);
        let lifts: Vec<f32> = (0..60).map(|i| ready_lift(true, i as f32 * 0.1, 1, false)).collect();
        assert!(lifts.iter().all(|&l| (0.0..=1.0).contains(&l)));
        assert!(lifts.iter().any(|&l| l > 0.5), "it rises");
        let sways: Vec<f32> = (0..200).map(|i| sway(i as f32 * 0.1, 2, false)).collect();
        assert!(sways.iter().all(|s| s.abs() <= 0.03), "a few degrees at most");
        assert!(sways.iter().any(|s| s.abs() > 0.005), "but it does move");
        assert_ne!(sway(1.0, 0, false), sway(1.0, 1, false), "cards sway out of step");
    }

    #[test]
    fn cards_in_flight_stay_on_screen_at_every_moment_and_size() {
        for (w, h) in [(1280.0, 720.0), (960.0, 540.0), (1920.0, 1080.0)] {
            let s = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
            let u = crate::pixel_ui::scale(s);
            for n in 1..=3 {
                let rests = layout(s, n);
                let stack = stack_rect(s, &rests, u);
                assert!(s.contains_rect(stack), "{w}x{h} n={n}");
                for (i, rest) in rests.iter().enumerate() {
                    let mut t = 0.0;
                    while t < LEAVE_TOTAL.max(DRAW_SECS) {
                        for pose in [leave_pose(t, *rest, stack, u, false), Some(draw_pose(t, *rest, stack, u, false))]
                            .into_iter()
                            .flatten()
                        {
                            let c = pose.rect.center();
                            assert!(c.x > 0.0 && c.x < w && c.y > 0.0 && c.y < h, "{w}x{h} n={n} i={i} t={t}: {c:?}");
                        }
                        t += 0.01;
                    }
                }
            }
        }
    }

    #[test]
    fn the_stack_shows_up_to_four_layers() {
        assert_eq!(stack_layers(0), 0);
        assert_eq!(stack_layers(1), 1);
        assert_eq!(stack_layers(3), 3);
        assert_eq!(stack_layers(40), 4);
    }

    #[test]
    fn popping_rises_and_grows() {
        let rest = layout(screen(), 2)[0];
        let up = popped(rest, 1.0, 1.0, false);
        assert!(up.center().y < rest.center().y);
        assert_eq!(up.center().x, rest.center().x);
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

    #[test]
    fn a_passive_pulse_flares_then_fades() {
        assert_eq!(pulse(NEVER), 0.0);
        assert_eq!(pulse(-0.5), 0.0);
        assert_eq!(pulse(0.0), 1.0);
        assert!(pulse(0.3) > pulse(0.6) && pulse(0.6) > 0.0);
        assert_eq!(pulse(PULSE_SECS), 0.0);
    }

    #[test]
    fn a_link_arc_starts_and_ends_on_its_cards_and_bows_upward() {
        let (a, b) = (Pos2::new(100.0, 500.0), Pos2::new(220.0, 520.0));
        let pts = arc(a, b, 20.0, 10);
        assert_eq!(pts.len(), 11);
        assert_eq!(pts[0], a);
        assert_eq!(pts[10], b);
        let top = pts.iter().map(|p| p.y).fold(f32::MAX, f32::min);
        assert!(top < (a.y + b.y) / 2.0 - 19.0, "bows up by the lift at the middle");
    }
}
