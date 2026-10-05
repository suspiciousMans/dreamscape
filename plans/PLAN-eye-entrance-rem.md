# Plan: Eye Entrance + REM (Closed Eyes Only)

**Goal:** Add the eye falling asleep (entrance animation), arriving at screen centre while speaking, with REM (rapid eye movement) active only when openness < 0.35.

**Scope:**
- Eye REM dormant on title (draw_eye_preview), pack reveal, ending — only active during gameplay (lucidity_eye)
- REM only when `openness < 0.35` (the lid threshold where pupil cannot move off-centre)
- Entrance animation on all four run starts: Continue, Daily, Prologue, normal restart
- No changes to blink(), test suite, or HudView beyond entrance_age + stare pass-through

---

## 1. New Module: `src/eye_motion.rs`

**Entrance state machine.** Same pattern as `transition.rs`.

```rust
pub struct Entrance {
    pub t: f32,                          // seconds into entrance
    pub done: bool,
}

impl Entrance {
    pub fn new() -> Self {
        Self { t: 0.0, done: false }
    }

    pub fn tick(&mut self, dt: f32) {
        if !self.done {
            self.t += dt;
            if self.t >= TRAVEL_END {
                self.done = true;
            }
        }
    }

    pub fn reset(&mut self) {
        self.t = 0.0;
        self.done = false;
    }

    /// Screen-relative offset (pixels) from centre to off-screen-left start.
    pub fn offset(&self) -> [f32; 2] {
        // t=0..ARRIVE: travel from x=-80 to x=0
        // t=ARRIVE..TRAVEL_END: hold at centre
        if self.t < ARRIVE {
            let lerp = self.t / ARRIVE;  // 0..1 over travel
            [-(1.0 - lerp) * 80.0, 0.0]  // slides in
        } else {
            [0.0, 0.0]
        }
    }

    /// Lid openness during entrance (before the main eye_openness takes over).
    pub fn lid(&self) -> f32 {
        if self.t < TRAVEL_START {
            0.35  // partially closed at arrival, REM-ing
        } else if self.t < TRAVEL_END {
            // Opens as it arrives
            let opened = (self.t - TRAVEL_START) / (TRAVEL_END - TRAVEL_START);
            0.35 + (0.65 * (opened * opened * (3.0 - 2.0 * opened)))  // smoothstep to full
        } else {
            1.0
        }
    }

    /// Is the entrance actively speaking?
    pub fn speaking(&self) -> bool {
        self.t < TRAVEL_END
    }
}

pub const TRAVEL_START: f32 = 0.15;  // lid opens when travel begins
pub const ARRIVE: f32 = 0.9;         // arrival time
pub const TRAVEL_END: f32 = 1.8;     // line begins its 4.5s hold
```

**REM gaze generator.** Pure, seeded, unit-tested.

```rust
/// Bursty saccade pattern: active only when caller says openness < 0.35.
/// Returns (gaze direction, is_darting_right_now).
/// Seeded by time + caller's salt (position on screen, identity, etc).
pub fn rem_gaze(age: f32, seed: u64) -> ([f32; 2], bool) {
    let burst_cycle = 2.5;  // seconds per burst cycle (2.5s quiet, 0.5s burst)
    let phase = age.rem_euclid(burst_cycle);
    
    if phase > 2.0 {
        // Bursting: ~4 darts over 0.5s, ~50ms each, horizontal-dominant
        let dart_ix = ((age * 20.0) as u32) % 4;  // 4 darts per burst
        let seed_this = seed ^ (dart_ix as u64);
        let h = hash01(seed_this, 1);
        let v = hash01(seed_this, 2);
        
        // Horizontal: ±6 cells, vertical: ±1 cell (favour horizontal like real REM)
        let gaze = [
            -1.0 + 2.0 * h,  // -1..1 horizontal
            (v - 0.5) * 0.33,  // -0.165..0.165 vertical
        ];
        (gaze, true)  // is_darting
    } else {
        // Quiet: subtle drift
        let t = phase / 2.0;  // 0..1 over 2s
        let gaze = [
            (t * 4.0 - 2.0).sin() * 0.3,  // slow sine, small
            ((t * 2.0) * 3.14159).sin() * 0.15,
        ];
        (gaze, false)
    }
}

pub fn hash01(k: u64, salt: u32) -> f32 {
    let mut x = k ^ (salt as u64);
    x = x.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 33;
    x = x.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 33;
    (x as f32) / (u64::MAX as f32)
}
```

**Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entrance_starts_closed_and_off_screen() {
        let e = Entrance::new();
        assert_eq!(e.offset(), [-80.0, 0.0]);
        assert_eq!(e.lid(), 0.35);
        assert!(e.speaking());
    }

    #[test]
    fn entrance_arrives_and_opens() {
        let mut e = Entrance::new();
        e.t = ARRIVE;
        assert!((e.offset()[0] - 0.0).abs() < 1e-3, "at centre");
        assert_eq!(e.lid(), 0.35, "still half-closed at arrival");
        
        e.t = TRAVEL_END;
        assert_eq!(e.offset(), [0.0, 0.0]);
        assert!(e.lid() > 0.99, "fully open");
        assert!(!e.speaking());
    }

    #[test]
    fn offset_interpolates_smoothly_left_to_centre() {
        let mut e = Entrance::new();
        e.t = TRAVEL_START * 0.5;
        let at_half = e.offset()[0];
        assert!(at_half < -40.0 && at_half > -80.0);
        
        e.t = TRAVEL_START;
        assert!((e.offset()[0] - 0.0).abs() < 1e-3);
    }

    #[test]
    fn rem_bursts_are_fast_and_horizontal() {
        let (g, is_dart) = rem_gaze(2.2, 12345);  // in burst
        assert!(is_dart);
        assert!(g[0].abs() > 0.5, "horizontal burst");
        assert!(g[1].abs() < 0.2, "vertical is small");
    }

    #[test]
    fn rem_quiet_phase_is_slow_and_subtle() {
        let (g, is_dart) = rem_gaze(1.0, 12345);  // in quiet
        assert!(!is_dart);
        assert!(g[0].abs() < 0.5);
        assert!(g[1].abs() < 0.3);
    }
}
```

---

## 2. Changes to `src/main.rs`

**Add entrance to game state:**

```rust
pub struct DreamscapeGame {
    // ...
    entrance: eye_motion::Entrance,
    // ...
}

// In new():
    entrance: eye_motion::Entrance::new(),

// In begin_run() (called by all four start paths):
    self.entrance.reset();
    self.eye_line = Some((text.to_string(), 0.0));  // FirstDream line

// In update(), before HUD snapshot:
    if !self.entrance.done {
        self.entrance.tick(dt);
    }
```

**In HudView snapshot (`fn hud_view()`):**

```rust
    entrance_age: if !self.entrance.done {
        self.entrance.t
    } else {
        f32::MAX  // done, never consulted
    },
```

---

## 3. Changes to `src/hud.rs`

**In `struct HudView`:**

```rust
pub entrance_age: f32,  // f32::MAX when done
```

**In `lucidity_eye()` (line 678):**

```rust
fn lucidity_eye(p: &egui::Painter, screen: Rect, v: &HudView) {
    use crate::pixels;
    
    // Entrance animation (if still active)
    let entrance_offset = if v.entrance_age < f32::MAX {
        crate::eye_motion::Entrance { t: v.entrance_age, done: false }.offset()
    } else {
        [0.0, 0.0]
    };
    
    let mut c = Pos2::new(
        screen.center().x + entrance_offset[0],
        screen.bottom() - 96.0 + entrance_offset[1]
    );
    
    let lucid = v.lucidity >= v.lucid_target;
    
    // Entrance lidding overrides eye_openness briefly
    let base_open = if v.entrance_age < f32::MAX {
        crate::eye_motion::Entrance { t: v.entrance_age, done: false }.lid()
    } else {
        eye_openness(v.lucidity, v.lucid_target)
    };
    
    // Always apply breath + blink modulation
    let open = (base_open * (1.0 + 0.05 * (v.time * 1.7).sin()))
        .min(1.0)
        * pixels::blink(v.time);
    
    // **REM gaze: only when openness < 0.35**
    let (rem_gaze, is_darting) = if base_open < 0.35 {
        crate::eye_motion::rem_gaze(v.dream_age, v.seed)
    } else {
        ([0.0, 0.0], false)
    };
    
    // Gaze direction: shard compass wins (player control), else REM
    let gaze = if v.shard_dir.is_some() {
        v.shard_dir
    } else if is_darting || base_open < 0.30 {  // dart or very closed
        Some(rem_gaze)
    } else {
        None
    };
    
    let cells = pixels::eye_pixels(open, gaze, lucid);
    let iris = iris_color(v, lucid);
    paint_eye(p, c, &cells, iris, v);
    lucid_rays_and_pips(p, c, lucid, screen.width(), v);
    
    // Eye line (existing)
    if let Some((text, age)) = &v.eye_line {
        let a = crate::eye::alpha(*age);
        if a > 0.0 {
            // ... clamp line plate to not run off-screen when entrance offsets it
            let text = v.k(&if v.pad { text.replace(...) } else { text.clone() });
            let shown = crate::eye::shown(&text, *age);
            let font = FontId::proportional(24.0);
            let full = p.layout_no_wrap(text.clone(), font.clone(), Color32::WHITE);
            
            // Offset plate to stay onscreen during entrance
            let plate_x = if entrance_offset[0] < -20.0 {
                // During travel, right-align to eye
                c.x + (full.size().x * 0.5) + 14.0
            } else {
                c.x
            };
            
            let at = Pos2::new(plate_x, c.y - 8.0 * EYE_PX);
            let plate = Rect::from_center_size(
                at - Vec2::new(0.0, full.size().y * 0.5),
                full.size() + Vec2::new(28.0, 12.0),
            );
            p.rect_filled(plate, 4.0, rgba([8, 4, 18], 0.72 * a));
            p.text(
                Pos2::new(plate.left() + 14.0, plate.center().y),
                Align2::LEFT_CENTER,
                shown,
                font,
                rgba([226, 214, 250], a),
            );
        }
    }
}
```

---

## 4. Changes to `src/main.rs` — Four Start Paths

All four call `begin_run()`, so the entrance resets in one place. But we need to set the eye line so `FirstDream` doesn't fire a second time:

**In `start_from_title()`:**
```rust
    self.entrance.reset();
    self.begin_run();
    // Eye says FirstDream during entrance, don't think() it again
```

**In `restart()`:**
```rust
    self.entrance.reset();
    self.begin_run();
    self.load_dream(ctx)?;
```

**In `start_pending()` (Continue, Daily, Prologue):**
```rust
    self.entrance.reset();
    self.begin_run();
    self.load_dream(ctx)?;
    if let Some(p) = self.prologue { self.show_prompt(p.step); }
```

They all already end with `self.mode = Mode::Playing` or call `load_dream()`, so the entrance ticks naturally.

---

## 5. Changes to `src/eye.rs`

**Add FirstDream to the entrance call sites.** Currently `think(FirstDream)` fires in `restart()` only. Move it to `begin_run()`:

```rust
// In begin_run():
    self.eye_sticky = false;
    let you = lore::dreamer(self.booklet.lore.save_seed, 0);
    let seed = self.director.dream_seed() ^ (eye::Moment::FirstDream as u64);
    let text = eye::line(eye::Moment::FirstDream, &you, seed);
    self.eye_line = Some((text, 0.0));  // fires once per run, regardless of path
```

This ensures all four start paths get the line without calling `think()` multiple times.

---

## 6. Accessibility & Polish

**In `pixels.rs`, gate REM during reduced motion:**

```rust
// In rem_gaze():
pub fn rem_gaze(age: f32, seed: u64, motion_scale: f32) -> ([f32; 2], bool) {
    // If motion_scale < 0.3 (user has reduced_motion on), suppress bursts
    let suppress = motion_scale < 0.3;
    
    let burst_cycle = 2.5;
    let phase = age.rem_euclid(burst_cycle);
    
    if !suppress && phase > 2.0 {
        // bursts...
    } else {
        // quiet drift (stays on)
    }
}
```

And in `lucidity_eye()`:
```rust
    let (rem_gaze, is_darting) = if base_open < 0.35 {
        crate::eye_motion::rem_gaze(v.dream_age, v.seed, v.settings_snapshot.motion())
    } else {
        ([0.0, 0.0], false)
    };
```

---

## Build Order

1. **Create `src/eye_motion.rs`** — Entrance + rem_gaze, all tests green.
2. **Wire HudView.entrance_age** — add field, pass in hud_view().
3. **Update `lucidity_eye()`** — entrance offsets, REM gaze, gate on openness < 0.35, shard_dir wins.
4. **Fix four start paths** — move `think(FirstDream)` to `begin_run()`, reset entrance.
5. **Clamp eye line plate** — stay onscreen during travel.
6. **Test:** DREAMSCAPE_SEED=X cargo run, watch run starts. Entrance should slide in, eye half-lidded and REM-ing, line types, lid opens, eye goes fully awake. REM silent once above 0.35.

---

## What Stays Untouched

- `blink()` — fires normally, REM stops during close anyway because openness dips
- Test suite — no changes to pixels tests, eye.rs tests, hud.rs tests
- Title preview, pack card, ending eye — REM never fires (entrance_age = f32::MAX)
- Stalker stare win — still tracks, just read on `v.stare`, not in eye motion

---

## Files

- **new:** `src/eye_motion.rs` (~150 lines, ~50 tests)
- **edit:** `src/main.rs` (entrance field, begin_run, 4 start paths)
- **edit:** `src/hud.rs` (HudView field, lucidity_eye logic, line plate clamp)
- **edit:** `src/eye.rs` (FirstDream in begin_run, remove from restart)
- **edit:** `src/pixels.rs` (rem_gaze, hash01; gate on motion_scale)

---

## Risk

**Lowest:** All logic is pure and tested before wiring. The REM is visual-only, gated to below 0.35 so it never fires until the eye is mechanically half-closed. Entrance is a state machine with a clear end, lives in one place, doesn't touch gameplay or choice logic.

**Watch:** Entrance must not steal the shard compass gaze while the player is mid-run (entrance_age only lives from first frame of new dream, gets set to f32::MAX the moment it ends). Prologue needs special care — its sticky tutorial prompts might want the eye to fully open before showing "Move" so the pupil has room to lock on.
