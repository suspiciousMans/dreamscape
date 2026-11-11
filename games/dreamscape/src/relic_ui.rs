//! R6: the relic screens. A fourth tab of the Lucid Store holds the relic case
//! (spend Ash, watch it open) and the pouch (browse and equip).
//!
//! Pure logic first (commit, order, cursor, beat timeline), then the painters.
//! The outcome is rolled ONCE, inside `commit_pull`, from a seed that depends
//! only on the save and the number of pulls already made, and it is saved by
//! the caller before anything animates. The animation only uncovers a relic
//! that is already in the booklet, so quitting mid-way cannot re-roll it, and
//! skipping only jumps to the final frame.
//!
//! Guardrails (plans/gamble-briefs/CONTEXT.md): Ash and relics only, exact
//! odds and the pity counters on screen, no near-miss wording, no countdowns.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::cards::Booklet;
use crate::dream::DreamTheme;
use crate::hud::{glitch_text, hue, ink, rgba, HudView};
use crate::pixel_ui;
use crate::relic_art::RelicArtCache;
use crate::relic_types::{Relic, RelicEffect, RelicRarity};
use crate::relics::{self, RelicPull};
use crate::retention;
use crate::screen_anim;
use crate::ui_motion::{self, UiJuice};
use crate::worth::mix;
use engine::ui::egui::{self, Align2, Color32, FontId, Pos2, Rect, Stroke, Vec2};

const PULL_SALT: u64 = 0x5E11_C0DE_0000_0A06;
const RELIC_SALT: u64 = 0x5E11_C0DE_0000_0B06;

// ---------------------------------------------------------------- commit

/// The dearest price a pull can end at: the Ash a pull needs in hand.
pub fn max_cost() -> u32 {
    relics::RELIC_COST.iter().copied().max().unwrap_or(0)
}

/// The seed of the next pull: a function of the save and the pulls already
/// committed, so an unsaved pull replays to the same relic.
pub fn pull_seed(b: &Booklet) -> u64 {
    mix(mix(b.lore.save_seed, PULL_SALT), b.relic_lottery.total_pulls as u64)
}

/// The next relic id (ids are never reused).
pub fn next_id(b: &Booklet) -> u32 {
    b.relics.iter().map(|r| r.id).max().map_or(1, |m| m + 1)
}

/// Builds the relic a pull produced. Pure in its arguments.
pub fn relic_from_pull(pull: &RelicPull, seed: u64, id: u32, taken: &[String]) -> Relic {
    let rseed = mix(seed, RELIC_SALT);
    let weapon = crate::relic_weapon::weapon_for(pull.rarity, pull.theme, rseed);
    let effect = crate::relic_effects::effect_for(pull.rarity, pull.theme, rseed);
    let spec = crate::relic_names::RelicSpec {
        theme: pull.theme,
        seed: rseed,
        rarity: pull.rarity,
        wear: relics::wear_tier(rseed),
        foil: pull.foil,
        shape: crate::relic_weapon::shape_of(&weapon),
        effect,
        taken,
    };
    Relic {
        id,
        theme: pull.theme,
        seed: rseed,
        rarity: pull.rarity,
        foil: pull.foil,
        name: crate::relic_names::name(&spec),
        flavour: crate::relic_names::flavour(&spec),
        effect,
        art: crate::relic_art::base_spec(pull.theme, rseed),
        weapon: Some(weapon),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PullError {
    TooPoor { need: u32 },
}

/// What a committed pull gives back.
#[derive(Clone, Debug)]
pub struct Committed {
    pub relic: Relic,
    pub pull: RelicPull,
}

/// Spends the Ash, rolls, builds and stores the relic. The caller saves the
/// booklet (and restores a clone if that fails). Needs `max_cost` Ash in hand
/// so the rolled price is always covered.
pub fn commit_pull(b: &mut Booklet, day: u64) -> Result<Committed, PullError> {
    let need = max_cost();
    if b.stash.ash < need {
        return Err(PullError::TooPoor { need: need - b.stash.ash });
    }
    let seed = pull_seed(b);
    let pull = relics::pull(&mut b.relic_lottery, seed, Some(relics::featured_relic_theme(day)));
    let paid = b.stash.spend_ash(pull.cost);
    debug_assert!(paid);
    let taken: Vec<String> = b.relics.iter().map(|r| r.name.clone()).collect();
    let relic = relic_from_pull(&pull, seed, next_id(b), &taken);
    let relic = retention::press_relic(b, relic);
    Ok(Committed { relic, pull })
}

// ---------------------------------------------------------------- pouch order

/// Ids in display order: rarest first, then oldest first.
pub fn order(relics: &[Relic]) -> Vec<u32> {
    let mut v: Vec<(u8, u32)> = relics.iter().map(|r| (3 - r.rarity as u8, r.id)).collect();
    v.sort();
    v.into_iter().map(|(_, id)| id).collect()
}

/// Moves the cursor `delta` places through `order()`, clamped. The cursor is
/// an id, never an index; an unknown or missing id starts at the top.
pub fn move_cursor(relics: &[Relic], cur: Option<u32>, delta: i32) -> Option<u32> {
    let ids = order(relics);
    if ids.is_empty() {
        return None;
    }
    let at = cur.and_then(|c| ids.iter().position(|&i| i == c));
    let to = match at {
        Some(i) => (i as i32 + delta).clamp(0, ids.len() as i32 - 1) as usize,
        None => 0,
    };
    Some(ids[to])
}

/// The cursor, or the first relic when it points at nothing.
pub fn cursor_or_first(relics: &[Relic], cur: Option<u32>) -> Option<u32> {
    move_cursor(relics, cur, 0)
}

/// First visible row of a list of `len` with `slots` rows and the cursor at `pos`.
pub fn window(pos: usize, len: usize, slots: usize) -> usize {
    pos.saturating_sub(slots / 2).min(len.saturating_sub(slots))
}

// ---------------------------------------------------------------- timeline

pub const PRESS: f32 = 0.22;
pub const LID: f32 = 0.38;
pub const LIGHT: f32 = 0.5;
pub const SLIDE: f32 = 0.7;
pub const TYPE: f32 = 0.7;
pub const SWEEP: f32 = 0.9;
pub const CROSSFADE: f32 = 0.5;
pub const FLASH: f32 = 0.3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Juice Off: the final card at once.
    Instant,
    /// Reduced motion: the final card fades in, nothing moves.
    Crossfade,
    Beats,
}

pub fn style(juice: UiJuice, reduced_motion: bool) -> Style {
    if juice == UiJuice::Off {
        Style::Instant
    } else if reduced_motion {
        Style::Crossfade
    } else {
        Style::Beats
    }
}

/// How long the finished card holds before the foil sweep: Cracked 0.4 s .. Mythic 1.4 s.
pub fn hold(r: RelicRarity) -> f32 {
    0.4 + r as u8 as f32 / 3.0
}

fn slide_start() -> f32 {
    PRESS + LID + LIGHT * 0.6
}

/// Seconds from the press to the end of the last beat.
pub fn total(style: Style, r: RelicRarity, foil: bool) -> f32 {
    match style {
        Style::Instant => 0.0,
        Style::Crossfade => CROSSFADE,
        Style::Beats => slide_start() + SLIDE + TYPE + hold(r) + if foil { SWEEP } else { 0.0 },
    }
}

/// Everything the painter needs `age` seconds after the pull was committed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Squash of the case (1.0 = none).
    pub press: f32,
    /// Lid lift, 0..~1.1 (springs past 1.0 and settles).
    pub lid: f32,
    pub light: f32,
    pub slide: f32,
    pub flip: f32,
    /// Share of the name typed in.
    pub name: f32,
    /// Foil sweep progress while it runs.
    pub sweep: Option<f32>,
    /// White flash 0..1 (always 0 under reduce_flashing).
    pub flash: f32,
    pub card_alpha: f32,
    pub done: bool,
}

pub fn frame(style: Style, r: RelicRarity, foil: bool, age: f32, reduce_flashing: bool) -> Frame {
    let fin = Frame {
        press: 1.0,
        lid: 1.0,
        light: 1.0,
        slide: 1.0,
        flip: 1.0,
        name: 1.0,
        sweep: None,
        flash: 0.0,
        card_alpha: 1.0,
        done: true,
    };
    let end = total(style, r, foil);
    if age >= end {
        return fin;
    }
    let age = age.max(0.0);
    match style {
        Style::Instant => fin,
        Style::Crossfade => Frame { card_alpha: (age / CROSSFADE).clamp(0.0, 1.0), done: false, ..fin },
        Style::Beats => {
            let s_slide = slide_start();
            let p = |start: f32, dur: f32| ((age - start) / dur).clamp(0.0, 1.0);
            let s_name = s_slide + SLIDE;
            let s_sweep = s_name + TYPE + hold(r);
            Frame {
                press: screen_anim::press_scale(age, 1.0),
                lid: ui_motion::ease_out_back(p(PRESS, LID)),
                light: ui_motion::ease_out_cubic(p(PRESS + LID, LIGHT)),
                slide: ui_motion::ease_out_cubic(p(s_slide, SLIDE)),
                flip: ui_motion::smoothstep(p(s_slide + SLIDE * 0.5, SLIDE * 0.5)),
                name: p(s_name, TYPE),
                sweep: (foil && age >= s_sweep).then(|| p(s_sweep, SWEEP)),
                flash: if reduce_flashing || age < s_slide {
                    0.0
                } else {
                    1.0 - p(s_slide, FLASH)
                },
                card_alpha: 1.0,
                done: false,
            }
        }
    }
}

// ---------------------------------------------------------------- copy

pub fn rarity_label(r: RelicRarity) -> &'static str {
    match r {
        RelicRarity::Cracked => "CRACKED",
        RelicRarity::Worn => "WORN",
        RelicRarity::Pristine => "PRISTINE",
        RelicRarity::Mythic => "MYTHIC",
    }
}

pub fn rarity_colour(r: RelicRarity) -> [u8; 3] {
    match r {
        RelicRarity::Cracked => [150, 150, 165],
        RelicRarity::Worn => [205, 140, 80],
        RelicRarity::Pristine => [150, 205, 255],
        RelicRarity::Mythic => [255, 210, 90],
    }
}

/// One line per bonus the relic gives.
pub fn buff_lines(e: &RelicEffect) -> Vec<String> {
    let mut v = Vec::new();
    if e.dust_bonus_pct > 0 {
        v.push(format!("+{}% DUST", e.dust_bonus_pct));
    }
    if e.memory_bonus > 0.0 {
        v.push(format!("+{:.2} MEMORY", e.memory_bonus));
    }
    if e.speed_pct > 0 {
        v.push(format!("+{}% SPEED", e.speed_pct));
    }
    if e.calm_pct > 0 {
        v.push(format!("+{}% CALM", e.calm_pct));
    }
    if e.grace_bonus > 0 {
        v.push(format!("+{} GRACE", e.grace_bonus));
    }
    if e.free_rerolls > 0 {
        v.push(format!("+{} FREE REROLL{}", e.free_rerolls, if e.free_rerolls == 1 { "" } else { "S" }));
    }
    if e.start_shards > 0 {
        v.push(format!("+{} START SHARD{}", e.start_shards, if e.start_shards == 1 { "" } else { "S" }));
    }
    if e.max_nerve > 0 {
        v.push(format!("+{} MAX NERVE", e.max_nerve));
    }
    v
}

/// The pity line: how many pulls until the Mythic is certain.
pub fn pity_line(pulls_to_pity: u32) -> String {
    if pulls_to_pity <= 1 {
        "MYTHIC GUARANTEED ON THE NEXT PULL".to_string()
    } else {
        format!("MYTHIC GUARANTEED IN {pulls_to_pity} PULLS")
    }
}

// ---------------------------------------------------------------- view

/// The relic that was just pulled and how long ago.
#[derive(Clone, Debug)]
pub struct OpeningView {
    pub relic: Relic,
    pub age: f32,
}

/// Everything the relic tab needs for one frame.
#[derive(Clone, Debug, Default)]
pub struct RelicView {
    /// 0 = the case, 1 = the pouch.
    pub page: u8,
    pub ash: u32,
    pub max_cost: u32,
    /// (rarity, exact chance, price), rarest first.
    pub rows: Vec<(RelicRarity, f32, u32)>,
    pub foil: f32,
    pub pulls_to_pity: u32,
    pub since_mythic: u32,
    pub total_pulls: u32,
    pub featured: Option<(DreamTheme, f32)>,
    pub opening: Option<OpeningView>,
    /// Pouch in `order()`.
    pub pouch: Vec<Relic>,
    pub cursor: Option<u32>,
    pub equipped: Option<u32>,
    pub status: Option<String>,
}

pub fn view_rows(b: &Booklet, day: u64) -> (Vec<(RelicRarity, f32, u32)>, f32, u32, Option<(DreamTheme, f32)>) {
    let featured = relics::featured_relic_theme(day);
    let o = relics::odds(&b.relic_lottery, Some(featured));
    let rows = [RelicRarity::Mythic, RelicRarity::Pristine, RelicRarity::Worn, RelicRarity::Cracked]
        .into_iter()
        .map(|r| (r, o.rarity[r as u8 as usize], relics::cost_for(r)))
        .collect();
    let share = o.themes.iter().find(|t| t.0 == featured).map(|t| (t.0, t.1));
    (rows, o.foil, o.pulls_to_hard_pity, share)
}

// ---------------------------------------------------------------- textures

#[derive(Default)]
struct Textures {
    art: RelicArtCache,
    icons: HashMap<(u8, [u8; 3]), egui::TextureHandle>,
}

fn with_tex<R>(ctx: &egui::Context, f: impl FnOnce(&mut Textures) -> R) -> R {
    let shared: Arc<Mutex<Textures>> = ctx.data_mut(|d| {
        d.get_temp_mut_or_insert_with(egui::Id::new("relic_ui_tex"), || Arc::new(Mutex::new(Textures::default())))
            .clone()
    });
    let mut g = shared.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut g)
}

const ICON_PX: usize = 24;

fn icon_tex(ctx: &egui::Context, relic: &Relic) -> Option<egui::TextureId> {
    let w = relic.weapon?;
    with_tex(ctx, |t| {
        let h = t.icons.entry((w.shape, w.tint)).or_insert_with(|| {
            let img = egui::ColorImage::from_rgba_unmultiplied(
                [ICON_PX, ICON_PX],
                &crate::relic_art::weapon_icon_rgba(crate::relic_weapon::shape_of(&w), w.tint, ICON_PX),
            );
            ctx.load_texture(format!("relic-icon-{}", w.shape), img, egui::TextureOptions::NEAREST)
        });
        Some(h.id())
    })
}

/// The relic's art, warped like every card, turned `xscale` about its middle.
fn warped_art(ctx: &egui::Context, p: &egui::Painter, relic: &Relic, rect: Rect, hover: f32, alpha: f32, xscale: f32, v: &HudView) {
    let id = with_tex(ctx, |t| t.art.get(ctx, relic));
    let st = &v.settings_snapshot;
    let warp = crate::card_warp::WarpParams {
        theme: relic.theme,
        seed: relic.seed,
        t: v.time,
        hover,
        juice: st.ui_juice,
        reduced_motion: st.reduced_motion,
    };
    let cx = rect.center().x;
    let tint = Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8);
    let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
    p.add(egui::Shape::mesh(crate::card_warp::art_mesh(id, rect, uv, tint, &warp, |q| {
        Pos2::new(cx + (q.x - cx) * xscale, q.y)
    })));
}

/// The foil glint over `rect` at time `t` (`relic_art::foil_sweep` per column).
fn foil_band(p: &egui::Painter, rect: Rect, t: f32, soft: bool) {
    const COLS: usize = 12;
    let k = if soft { 0.45 } else { 0.8 };
    let w = rect.width() / COLS as f32;
    for i in 0..COLS {
        let u = (i as f32 + 0.5) / COLS as f32;
        let a = crate::relic_art::foil_sweep(t, u, 0.5, (0.0, 0.0)) * k * 0.35;
        if a > 0.01 {
            p.rect_filled(
                Rect::from_min_size(Pos2::new(rect.left() + w * i as f32, rect.top()), Vec2::new(w + 0.5, rect.height())),
                0.0,
                rgba([255, 240, 200], a),
            );
        }
    }
}

/// The `foil_sweep` clock for the one-off sweep at progress `s` (0..1) across the band.
fn sweep_clock(s: f32) -> f32 {
    s * 1.6 / 0.35
}

// ---------------------------------------------------------------- painters

/// Shared layout numbers for one frame.
struct L {
    u: f32,
    uf: f32,
    c: f32,
}

fn lay(screen: Rect) -> L {
    let u = pixel_ui::scale(screen);
    L { u, uf: u.max(0.8), c: pixel_ui::cell(u) }
}

pub fn draw(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView) {
    let r = &v.relics;
    let l = lay(screen);
    let st = &v.settings_snapshot;
    p.rect_filled(screen, 0.0, rgba([6, 3, 16], 0.96));
    // Quiet drifting motes, like the other store tabs.
    for k in 0..24u32 {
        let h1 = crate::hud::hash01(k, 11);
        let h2 = crate::hud::hash01(k, 12);
        let rise = if st.reduced_motion || st.ui_juice == UiJuice::Off { 0.0 } else { v.time * (6.0 + 8.0 * h1) };
        let y = screen.bottom() - ((h2 * screen.height() + rise) % screen.height());
        p.rect_filled(
            Rect::from_center_size(Pos2::new(screen.left() + h1 * screen.width(), y), Vec2::splat(2.0 + 2.0 * h2)),
            0.0,
            rgba(hue(0.1 + 0.15 * h1), 0.12 + 0.15 * h2),
        );
    }
    let amount = ui_motion::effective(st.ui_juice, st.reduced_motion);
    let anim = screen_anim::advance(ctx, "r6_relics", v.time, r.page as u32, None, false, 0.0, amount);
    let age = anim.epoch_age(v.time);
    let tick = |n: u32| screen_anim::tick(n as i64, age, 0.0, amount);
    let cx = screen.center().x;
    glitch_text(
        p,
        Pos2::new(cx, screen.top() + 14.0 * l.uf),
        true,
        if r.page == 0 { "THE RELIC CASE" } else { "THE RELIC POUCH" },
        42.0 * l.uf,
        hue(0.1 + 0.03 * (v.time * 0.7).sin()),
        1.0,
        v,
    );
    p.text(
        Pos2::new(cx, screen.top() + 62.0 * l.uf),
        Align2::CENTER_TOP,
        format!(
            "{} ash   ·   {} {}   ·   {} {} so far",
            tick(r.ash),
            r.pouch.len(),
            if r.pouch.len() == 1 { "relic" } else { "relics" },
            tick(r.total_pulls),
            if r.total_pulls == 1 { "pull" } else { "pulls" }
        ),
        FontId::monospace(20.0 * l.uf),
        rgba([255, 210, 80], 1.0),
    );
    if r.page == 0 {
        draw_case_page(ctx, p, screen, v, &l, age, amount);
    } else {
        draw_pouch_page(ctx, p, screen, v, &l, age, amount);
    }
    if let Some(s) = &r.status {
        let size = pixel_ui::fit(s, 16.0 * l.uf, screen.width() - 40.0, 8.0);
        p.text(
            Pos2::new(cx, screen.bottom() - 62.0 * l.uf),
            Align2::CENTER_CENTER,
            s,
            FontId::monospace(size),
            rgba([255, 190, 150], 1.0),
        );
    }
    let hint = if r.page == 0 {
        "[enter] open   [p] pouch   [tab] next tab   [esc] leave"
    } else {
        "[w][s] choose   [enter] equip   [p] the case   [tab] next tab   [esc] leave"
    };
    crate::loadout_ui::hint_bar(p, screen, v, hint);
    pixel_ui::scanlines(p, screen, l.c, 8);
}

fn draw_case_page(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, l: &L, age: f32, amount: f32) {
    let r = &v.relics;
    let st = &v.settings_snapshot;
    let (w, h) = (screen.width(), screen.height());
    let c = l.c;
    // The odds, exactly as rolled.
    let panel = Rect::from_min_size(
        Pos2::new(screen.left() + 0.04 * w, screen.top() + 0.22 * h),
        Vec2::new(0.38 * w, 0.58 * h),
    );
    pixel_ui::titled_panel(p, panel, "WHAT THE CASE HOLDS", [200, 170, 230], rgba([8, 4, 20], 0.85), c, 15.0 * l.uf);
    let (il, ir) = (panel.left() + c * 3.0, panel.right() - c * 3.0);
    let mut y = panel.top() + 0.06 * h;
    let step = 0.065 * h;
    for (row, &(rar, chance, cost)) in r.rows.iter().enumerate() {
        let e = screen_anim::enter(row, age, amount);
        let ry = y + (1.0 - e) * 6.0 * l.u;
        let col = rarity_colour(rar);
        p.text(Pos2::new(il, ry), Align2::LEFT_TOP, rarity_label(rar), FontId::monospace(16.0 * l.uf), rgba(col, e));
        p.text(
            Pos2::new((il + ir) * 0.5 + 0.02 * w, ry),
            Align2::RIGHT_TOP,
            format!("{:.1}%", chance * 100.0),
            FontId::monospace(16.0 * l.uf),
            rgba(ink(v), 0.95 * e),
        );
        p.text(Pos2::new(ir, ry), Align2::RIGHT_TOP, format!("{cost} ash"), FontId::monospace(16.0 * l.uf), rgba([255, 210, 80], 0.9 * e));
        y += step;
    }
    let small = |p: &egui::Painter, y: f32, text: &str, col: [u8; 3]| {
        let size = pixel_ui::fit(text, 14.0 * l.uf, ir - il, 7.0);
        p.text(Pos2::new(il, y), Align2::LEFT_TOP, text, FontId::monospace(size), rgba(col, 0.85));
    };
    y += 0.01 * h;
    small(p, y, &format!("FOIL: {:.1}% OF ANY RELIC", r.foil * 100.0), [255, 235, 170]);
    y += step * 0.8;
    if let Some((t, share)) = r.featured {
        small(p, y, &format!("FEATURED: {} ({:.0}% OF RELICS)", crate::cards::Attribute::of(t).label(), share * 100.0), ink(v));
        y += step * 0.8;
    }
    small(p, y, &pity_line(r.pulls_to_pity), [190, 230, 255]);
    y += step * 0.8;
    small(p, y, &format!("{} PULLS SINCE YOUR LAST MYTHIC", r.since_mythic), ink(v));

    // The case and what it holds.
    let area = Rect::from_min_size(
        Pos2::new(screen.left() + 0.46 * w, screen.top() + 0.2 * h),
        Vec2::new(0.5 * w, 0.46 * h),
    );
    let sty = style(st.ui_juice, st.reduced_motion);
    let (fr, shown) = match &r.opening {
        Some(o) => (frame(sty, o.relic.rarity, o.relic.foil, o.age, st.reduce_flashing), Some(&o.relic)),
        None => (
            Frame { press: 1.0, lid: 0.0, light: 0.0, slide: 0.0, flip: 0.0, name: 0.0, sweep: None, flash: 0.0, card_alpha: 0.0, done: false },
            None,
        ),
    };
    let col = shown.map_or([210, 180, 130], |s| rarity_colour(s.rarity));
    draw_case(ctx, p, area, v, l, &fr, shown, col, amount);
    // Name, rarity and flavour, typed in once the card has turned.
    if let Some(rel) = shown {
        let line_y = area.bottom() + 0.025 * h;
        let n = ((rel.name.chars().count() as f32) * fr.name).ceil() as usize;
        let name: String = rel.name.chars().take(n).collect();
        let size = pixel_ui::fit(&rel.name, 20.0 * l.uf, w * 0.52, 9.0);
        p.text(Pos2::new(area.center().x, line_y), Align2::CENTER_TOP, &name, FontId::monospace(size), rgba(col, fr.card_alpha));
        if fr.name >= 1.0 {
            let tag = if rel.foil { format!("{}  ·  FOIL", rarity_label(rel.rarity)) } else { rarity_label(rel.rarity).to_string() };
            let fl = format!("{tag}   {}", rel.flavour);
            let size = pixel_ui::fit(&fl, 14.0 * l.uf, w * 0.52, 7.0);
            p.text(Pos2::new(area.center().x, line_y + 0.058 * h), Align2::CENTER_TOP, fl, FontId::monospace(size), rgba(ink(v), 0.85));
        }
    }
    // The button.
    let busy = r.opening.as_ref().is_some_and(|o| {
        !frame(sty, o.relic.rarity, o.relic.foil, o.age, st.reduce_flashing).done
    });
    let can = r.ash >= r.max_cost && !busy;
    let strip = Rect::from_center_size(
        Pos2::new(area.center().x, screen.top() + 0.855 * h - 0.05 * h),
        Vec2::new(0.4 * w, 0.075 * h),
    );
    let strip = strip.translate(Vec2::new(0.0, 0.0));
    let label = format!("OPEN THE CASE  ·  UP TO {} ASH", r.max_cost);
    pixel_ui::panel(p, strip, if can { [255, 210, 120] } else { [90, 80, 110] }, rgba([22, 12, 40], 0.9), c, 0.9);
    let size = pixel_ui::fit(&label, 18.0 * l.uf, strip.width() - c * 4.0, 8.0);
    p.text(strip.center(), Align2::CENTER_CENTER, label, FontId::monospace(size), rgba(ink(v), if can { 0.95 } else { 0.45 }));
    if can {
        let key = crate::prompts::label(crate::prompts::ActionId::Confirm, v.mode, st, v.pad);
        let at = Pos2::new(strip.right() + 30.0 * l.uf, strip.center().y);
        if at.x + 30.0 < screen.right() {
            crate::hud::prompt_chip(p, at, &key, "", crate::prompts::State::Available, v.time, st);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_case(
    ctx: &egui::Context,
    p: &egui::Painter,
    area: Rect,
    v: &HudView,
    l: &L,
    f: &Frame,
    relic: Option<&Relic>,
    col: [u8; 3],
    amount: f32,
) {
    let c = l.c;
    let sway = (v.time * 1.3).sin() * 3.0 * amount * if f.lid <= 0.0 { 1.0 } else { 0.0 };
    let bw = area.width() * 0.5;
    let bh = area.height() * 0.3 * (2.0 - f.press);
    let sq = f.press;
    let body = Rect::from_min_max(
        Pos2::new(area.center().x - bw * 0.5 / sq + sway, area.bottom() - bh * sq),
        Pos2::new(area.center().x + bw * 0.5 / sq + sway, area.bottom()),
    );
    let lid_h = body.height() * 0.45;
    let lid_y = body.top() - lid_h - f.lid * lid_h * 1.1;
    let lid = Rect::from_min_size(Pos2::new(body.left() - c, lid_y), Vec2::new(body.width() + c * 2.0, lid_h));
    // Light rising out of the open case.
    if f.light > 0.0 {
        let beam_h = f.light * area.height() * 0.7;
        let fade = 1.0 - 0.55 * f.slide;
        for k in 0..5 {
            let w = body.width() * (0.62 - 0.1 * k as f32);
            let r = Rect::from_min_max(
                Pos2::new(body.center().x - w * 0.5, body.top() - beam_h),
                Pos2::new(body.center().x + w * 0.5, body.top() + 2.0),
            );
            p.rect_filled(r, 0.0, rgba(col, 0.07 * fade));
        }
    }
    // The card behind the lid.
    if let Some(rel) = relic {
        if f.slide > 0.0 || f.card_alpha < 1.0 || f.done {
            let ch = area.height() * 0.62;
            let cw = ch * 0.72;
            let from = body.top();
            let to = area.top() + ch * 0.5 + 2.0;
            let cy = if f.card_alpha < 1.0 || f.done { to } else { from + (to - from) * f.slide };
            let card = Rect::from_center_size(Pos2::new(body.center().x, cy), Vec2::new(cw, ch));
            let xs = (std::f32::consts::PI * f.flip).cos().abs().max(0.02);
            let a = f.card_alpha;
            let shown = Rect::from_center_size(card.center(), Vec2::new(cw * xs, ch));
            pixel_ui::panel(p, shown, col, rgba([14, 8, 26], a), c, a);
            if f.flip >= 0.5 {
                let art = Rect::from_center_size(card.center(), Vec2::splat(cw - c * 4.0));
                warped_art(ctx, p, rel, art.translate(Vec2::new(0.0, -c)), if f.done { 0.6 } else { 0.0 }, a, xs, v);
                if rel.foil {
                    let s = f.sweep.map_or_else(|| if amount > 0.0 { v.time } else { 0.0 }, sweep_clock);
                    let band = Rect::from_center_size(art.center(), Vec2::new(art.width() * xs, art.height()));
                    foil_band(p, band, s, v.settings_snapshot.reduce_flashing);
                }
            } else {
                pixel_ui::hatch(p, shown.shrink(c * 2.0), c, rgba(col, 0.35 * a));
            }
        }
    }
    pixel_ui::panel(p, body, [200, 170, 120], rgba([40, 26, 52], 1.0), c, 1.0);
    p.rect_filled(
        Rect::from_center_size(Pos2::new(body.center().x, body.top() + body.height() * 0.45), Vec2::new(c * 3.0, c * 4.0)),
        0.0,
        rgba(col, 0.95),
    );
    pixel_ui::panel(p, lid, [220, 190, 140], rgba([54, 34, 66], 1.0), c, 1.0);
    if f.flash > 0.0 {
        p.rect_filled(area.expand(20.0), 0.0, rgba([255, 255, 255], 0.22 * f.flash));
    }
}

fn draw_pouch_page(ctx: &egui::Context, p: &egui::Painter, screen: Rect, v: &HudView, l: &L, age: f32, amount: f32) {
    let r = &v.relics;
    let (w, h) = (screen.width(), screen.height());
    let c = l.c;
    let list = Rect::from_min_size(
        Pos2::new(screen.left() + 0.04 * w, screen.top() + 0.2 * h),
        Vec2::new(0.5 * w, 0.62 * h),
    );
    pixel_ui::titled_panel(p, list, "YOUR RELICS", [200, 170, 230], rgba([8, 4, 20], 0.85), c, 15.0 * l.uf);
    if r.pouch.is_empty() {
        p.text(
            list.center() - Vec2::new(0.0, 0.03 * h),
            Align2::CENTER_CENTER,
            "THE POUCH IS EMPTY",
            FontId::monospace(pixel_ui::fit("THE POUCH IS EMPTY", 22.0 * l.uf, list.width() - 20.0, 9.0)),
            rgba(ink(v), 0.85),
        );
        let hint = "OPEN THE CASE TO FIND A RELIC";
        p.text(
            list.center() + Vec2::new(0.0, 0.04 * h),
            Align2::CENTER_CENTER,
            hint,
            FontId::monospace(pixel_ui::fit(hint, 15.0 * l.uf, list.width() - 20.0, 8.0)),
            rgba(ink(v), 0.6),
        );
        return;
    }
    let ids: Vec<u32> = r.pouch.iter().map(|x| x.id).collect();
    let cur = cursor_or_first(&r.pouch, r.cursor);
    let pos = cur.and_then(|c| ids.iter().position(|&i| i == c)).unwrap_or(0);
    let slots = 5usize;
    let first = window(pos, ids.len(), slots);
    let row_h = list.height() * 0.9 / slots as f32;
    let mut focused: Option<&Relic> = None;
    for (n, rel) in r.pouch.iter().enumerate().skip(first).take(slots) {
        let i = n - first;
        let e = screen_anim::enter(i, age, amount);
        let top = list.top() + list.height() * 0.06 + i as f32 * row_h + (1.0 - e) * 6.0 * l.u;
        let row = Rect::from_min_size(Pos2::new(list.left() + c * 2.0, top), Vec2::new(list.width() - c * 4.0, row_h - c * 0.5));
        let on = Some(rel.id) == cur;
        if on {
            focused = Some(rel);
            pixel_ui::selected_row(p, row, rarity_colour(rel.rarity), c);
        }
        let thumb = Rect::from_min_size(row.left_top() + Vec2::splat(c * 0.5), Vec2::splat(row.height() - c));
        warped_art(ctx, p, rel, thumb, if on { 1.0 } else { 0.0 }, e, 1.0, v);
        p.rect_stroke(thumb, 0.0, Stroke::new(1.5_f32, rgba(rarity_colour(rel.rarity), 0.8 * e)));
        let tx = thumb.right() + c * 1.5;
        let tw = row.right() - tx - c * 6.0;
        let eq = Some(rel.id) == r.equipped;
        let size = pixel_ui::fit(&rel.name, 17.0 * l.uf, tw, 8.0);
        p.text(Pos2::new(tx, row.top() + row.height() * 0.1), Align2::LEFT_TOP, &rel.name, FontId::monospace(size), rgba(ink(v), 0.95 * e));
        let mut sub = rarity_label(rel.rarity).to_string();
        if rel.foil {
            sub.push_str(" · FOIL");
        }
        let b = buff_lines(&rel.effect);
        if !b.is_empty() {
            sub.push_str(" · ");
            sub.push_str(&b.iter().take(2).cloned().collect::<Vec<_>>().join(" "));
        }
        let size = pixel_ui::fit(&sub, 14.0 * l.uf, tw, 7.0);
        p.text(Pos2::new(tx, row.top() + row.height() * 0.55), Align2::LEFT_TOP, sub, FontId::monospace(size), rgba(rarity_colour(rel.rarity), 0.9 * e));
        if eq {
            pixel_ui::check(p, Pos2::new(row.right() - c * 3.0, row.center().y), c, rgba([120, 255, 140], e));
        }
    }
    // Scroll hints.
    if first > 0 {
        p.text(Pos2::new(list.right() - c * 3.0, list.top() + c * 3.0), Align2::CENTER_CENTER, "^", FontId::monospace(16.0 * l.uf), rgba(ink(v), 0.6));
    }
    if first + slots < ids.len() {
        p.text(Pos2::new(list.right() - c * 3.0, list.bottom() - c * 3.0), Align2::CENTER_CENTER, "v", FontId::monospace(16.0 * l.uf), rgba(ink(v), 0.6));
    }
    // The focused relic in full.
    let Some(rel) = focused else { return };
    let det = Rect::from_min_size(
        Pos2::new(screen.left() + 0.58 * w, screen.top() + 0.2 * h),
        Vec2::new(0.38 * w, 0.62 * h),
    );
    let col = rarity_colour(rel.rarity);
    pixel_ui::panel(p, det, col, rgba([8, 4, 20], 0.85), c, 0.9);
    let art = Rect::from_min_size(det.left_top() + Vec2::splat(c * 2.0), Vec2::splat(det.height() * 0.38));
    warped_art(ctx, p, rel, art, 1.0, 1.0, 1.0, v);
    p.rect_stroke(art, 0.0, Stroke::new(2.0_f32, rgba(col, 0.9)));
    if rel.foil {
        foil_band(p, art, if amount > 0.0 { v.time } else { 0.0 }, v.settings_snapshot.reduce_flashing);
    }
    // Weapon icon beside the art.
    if let Some(tex) = icon_tex(ctx, rel) {
        let ir = Rect::from_min_size(Pos2::new(art.right() + c * 2.0, art.top()), Vec2::splat(det.height() * 0.16));
        p.image(tex, ir, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
        if let Some(wp) = rel.weapon {
            let t = format!("SIZE {}%", wp.scale_pct);
            let size = pixel_ui::fit(&t, 13.0 * l.uf, det.right() - ir.left() - c * 2.0, 7.0);
            p.text(Pos2::new(ir.left(), ir.bottom() + c * 0.5), Align2::LEFT_TOP, t, FontId::monospace(size), rgba(ink(v), 0.7));
        }
    }
    let tx = det.left() + c * 2.0;
    let tw = det.width() - c * 4.0;
    let mut y = art.bottom() + c * 1.5;
    let size = pixel_ui::fit(&rel.name, 17.0 * l.uf, tw, 8.0);
    p.text(Pos2::new(tx, y), Align2::LEFT_TOP, &rel.name, FontId::monospace(size), rgba(col, 1.0));
    y += size * 1.3;
    let g = p.layout(rel.flavour.clone(), FontId::monospace(13.0 * l.uf), rgba(ink(v), 0.8), tw);
    let gh = g.size().y;
    p.galley(Pos2::new(tx, y), g, rgba(ink(v), 0.8));
    y += gh + c;
    for line in buff_lines(&rel.effect) {
        let size = pixel_ui::fit(&line, 14.0 * l.uf, tw, 7.0);
        p.text(Pos2::new(tx, y), Align2::LEFT_TOP, line, FontId::monospace(size), rgba([120, 235, 210], 0.95));
        y += size * 1.2;
    }
    // Equip.
    let eq = Some(rel.id) == r.equipped;
    let btn = Rect::from_min_max(
        Pos2::new(det.left() + c * 2.0, det.bottom() - c * 2.0 - det.height() * 0.1),
        Pos2::new(det.right() - c * 2.0, det.bottom() - c * 2.0),
    );
    let label = if eq { "EQUIPPED" } else { "EQUIP" };
    pixel_ui::panel(p, btn, if eq { [120, 255, 140] } else { [255, 210, 120] }, rgba([22, 12, 40], 0.9), c, 0.9);
    p.text(
        btn.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::monospace(pixel_ui::fit(label, 18.0 * l.uf, btn.width() * 0.5, 8.0)),
        rgba(ink(v), 0.95),
    );
    if !eq {
        let st = &v.settings_snapshot;
        let key = crate::prompts::label(crate::prompts::ActionId::Confirm, v.mode, st, v.pad);
        if let Some(at) = screen_anim::chip_beside(btn.right(), btn.center().y, &key, screen.right()) {
            crate::hud::prompt_chip(p, at, &key, "", crate::prompts::State::Available, v.time, st);
        } else {
            let at = Pos2::new(btn.left() + c * 4.0, btn.center().y);
            crate::hud::prompt_chip(p, at, &key, "", crate::prompts::State::Available, v.time, st);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relic_types::RelicState;

    fn booklet(ash: u32) -> Booklet {
        let mut b = Booklet::default();
        b.lore.save_seed = 77;
        b.stash.earn_ash(ash);
        b
    }

    const ALL: [RelicRarity; 4] = [RelicRarity::Cracked, RelicRarity::Worn, RelicRarity::Pristine, RelicRarity::Mythic];

    #[test]
    fn relic_from_pull_is_deterministic() {
        let pull = RelicPull {
            rarity: RelicRarity::Pristine,
            theme: DreamTheme::MeltingClockworks,
            foil: true,
            pity: false,
            near_miss: None,
            cost: 80,
        };
        let a = relic_from_pull(&pull, 9, 3, &[]);
        let b = relic_from_pull(&pull, 9, 3, &[]);
        assert_eq!(a, b);
        assert_eq!((a.rarity, a.theme, a.foil, a.id), (RelicRarity::Pristine, DreamTheme::MeltingClockworks, true, 3));
        assert!(a.weapon.is_some() && !a.name.is_empty() && !a.flavour.is_empty());
        assert_ne!(a.seed, relic_from_pull(&pull, 10, 3, &[]).seed);
    }

    #[test]
    fn committed_pulls_get_unique_increasing_ids_and_names() {
        let mut b = booklet(100_000);
        let mut ids = Vec::new();
        for _ in 0..40 {
            let got = commit_pull(&mut b, 5).unwrap();
            ids.push(got.relic.id);
        }
        assert_eq!(ids, (1..=40).collect::<Vec<u32>>());
        let mut names: Vec<_> = b.relics.iter().map(|r| r.name.clone()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 40, "owned names are avoided");
        assert_eq!(b.relic_lottery.total_pulls, 40);
    }

    #[test]
    fn a_pull_needs_the_dearest_price_and_pays_the_rolled_one() {
        let mut b = booklet(max_cost() - 1);
        assert_eq!(commit_pull(&mut b, 1).unwrap_err(), PullError::TooPoor { need: 1 });
        assert!(b.relics.is_empty() && b.relic_lottery == RelicState::default());
        b.stash.earn_ash(1 + 50);
        let got = commit_pull(&mut b, 1).unwrap();
        assert_eq!(b.stash.ash, max_cost() + 50 - got.pull.cost);
        assert_eq!(got.pull.cost, relics::cost_for(got.relic.rarity));
    }

    #[test]
    fn the_outcome_is_fixed_before_the_save_so_replaying_it_cannot_reroll() {
        // Quit before the save lands: the booklet on disk is the one before the pull.
        let saved = booklet(1000);
        let mut first = saved.clone();
        let a = commit_pull(&mut first, 5).unwrap();
        let mut again = saved.clone();
        let b = commit_pull(&mut again, 5).unwrap();
        assert_eq!(a.relic, b.relic);
        assert_eq!(first.relics, again.relics);
        // And the stored relic is what the screen shows: reopening reads the booklet.
        assert_eq!(first.relics.last(), Some(&a.relic));
    }

    #[test]
    fn the_timeline_is_monotone_and_ends_on_the_final_frame() {
        for &r in &ALL {
            for foil in [false, true] {
                for fl in [false, true] {
                    let end = total(Style::Beats, r, foil);
                    let mut last: Option<Frame> = None;
                    for k in 0..=(end * 200.0) as usize + 20 {
                        let t = k as f32 * 0.005;
                        let f = frame(Style::Beats, r, foil, t, fl);
                        if let Some(o) = last {
                            assert!(f.light >= o.light - 1e-6, "light");
                            assert!(f.slide >= o.slide - 1e-6, "slide");
                            assert!(f.flip >= o.flip - 1e-6, "flip");
                            assert!(f.name >= o.name - 1e-6, "name");
                        }
                        assert!((0.0..=1.15).contains(&f.lid) && (0.9..=1.0).contains(&f.press));
                        if fl {
                            assert_eq!(f.flash, 0.0, "reduce_flashing: no flash");
                        }
                        last = Some(f);
                    }
                    assert!(last.unwrap().done);
                    assert!(!frame(Style::Beats, r, foil, end - 0.01, fl).done);
                }
            }
        }
    }

    #[test]
    fn holds_scale_with_rarity_and_skip_lands_on_the_final_state() {
        assert!((hold(RelicRarity::Cracked) - 0.4).abs() < 1e-6 && (hold(RelicRarity::Mythic) - 1.4).abs() < 1e-6);
        assert!(hold(RelicRarity::Worn) < hold(RelicRarity::Pristine));
        for sty in [Style::Beats, Style::Crossfade, Style::Instant] {
            let skipped = frame(sty, RelicRarity::Mythic, true, 1e6, false);
            assert!(skipped.done && skipped.name == 1.0 && skipped.flip == 1.0 && skipped.card_alpha == 1.0);
            assert_eq!(skipped.sweep, None);
        }
        assert!(frame(Style::Instant, RelicRarity::Worn, false, 0.0, false).done);
        let x = frame(Style::Crossfade, RelicRarity::Worn, false, 0.1, false);
        assert!(!x.done && x.card_alpha < 1.0 && x.slide == 1.0 && x.flash == 0.0, "crossfade: nothing moves");
        assert_eq!(style(UiJuice::Off, true), Style::Instant);
        assert_eq!(style(UiJuice::Full, true), Style::Crossfade);
        assert_eq!(style(UiJuice::Soft, false), Style::Beats);
    }

    fn pouch() -> Vec<Relic> {
        let mut b = booklet(100_000);
        for _ in 0..30 {
            commit_pull(&mut b, 2).unwrap();
        }
        b.relics
    }

    #[test]
    fn pouch_order_is_rarest_first_then_oldest_and_stable() {
        let rs = pouch();
        let ids = order(&rs);
        assert_eq!(ids.len(), 30);
        let rank = |id: u32| rs.iter().find(|r| r.id == id).unwrap().rarity as u8;
        for w in ids.windows(2) {
            assert!(rank(w[0]) > rank(w[1]) || (rank(w[0]) == rank(w[1]) && w[0] < w[1]));
        }
        let mut shuffled = rs.clone();
        shuffled.reverse();
        assert_eq!(order(&shuffled), ids, "input order does not matter");
    }

    #[test]
    fn the_cursor_is_an_id_that_survives_reordering() {
        let mut rs = pouch();
        let ids = order(&rs);
        let cur = move_cursor(&rs, Some(ids[4]), 1);
        assert_eq!(cur, Some(ids[5]));
        assert_eq!(move_cursor(&rs, Some(ids[0]), -3), Some(ids[0]), "clamped at the top");
        assert_eq!(move_cursor(&rs, Some(ids[29]), 9), Some(ids[29]), "clamped at the bottom");
        assert_eq!(move_cursor(&rs, Some(9999), 0), Some(ids[0]), "unknown id starts at the top");
        rs.reverse();
        assert_eq!(move_cursor(&rs, cur, 1), Some(ids[6]), "same relic, whatever the vec order");
        assert_eq!(move_cursor(&[], None, 1), None);
    }

    #[test]
    fn the_window_keeps_the_cursor_visible() {
        for len in 0..12 {
            for pos in 0..len.max(1) {
                let f = window(pos, len, 5);
                assert!(f + 5 >= len.min(f + 5) && (len <= 5 || f + 5 <= len));
                assert!(pos >= f && pos < f + 5 || len == 0);
            }
        }
    }

    #[test]
    fn copy_helpers() {
        assert_eq!(pity_line(24), "MYTHIC GUARANTEED IN 24 PULLS");
        assert_eq!(pity_line(1), "MYTHIC GUARANTEED ON THE NEXT PULL");
        assert!(buff_lines(&RelicEffect::default()).is_empty());
        let e = RelicEffect { dust_bonus_pct: 5, free_rerolls: 1, ..Default::default() };
        assert_eq!(buff_lines(&e), vec!["+5% DUST", "+1 FREE REROLL"]);
    }

    #[test]
    fn an_old_booklet_without_relics_loads_with_an_empty_pouch() {
        let b: Booklet = ron::from_str("(runs: [])").unwrap();
        assert!(b.relics.is_empty() && b.equipped_relic.is_none());
        assert!(order(&b.relics).is_empty());
    }
}
