use serde::{Deserialize, Serialize};

/// A screen-space color effect authored on a trigger or fired from a script/
/// native `Behavior` — an envelope (fade in, hold, fade out) around a peak
/// tint strength, evaluated by `ScreenEffectState` and composited into the
/// frame by `CompositePass` alongside the posterize/dither post effect.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct ScreenEffectSpec {
    pub color: [f32; 3],
    pub strength: f32,
    #[serde(default)]
    pub fade_in_secs: f32,
    #[serde(default)]
    pub hold_secs: f32,
    #[serde(default)]
    pub fade_out_secs: f32,
}

/// Transient runtime state for the currently-playing screen effect, if any.
/// Owned directly by the game (like `HudState`), ticked once per frame, and
/// deliberately never serialized as part of a `ShaderProfile` — a flash must
/// never get baked into a saved "look".
#[derive(Default)]
pub struct ScreenEffectState {
    active: Option<(ScreenEffectSpec, f32)>,
}

impl ScreenEffectState {
    /// Starts (or restarts) the effect from the beginning, replacing
    /// whatever was previously playing.
    pub fn trigger(&mut self, spec: ScreenEffectSpec) {
        self.active = Some((spec, 0.0));
    }

    /// Advances elapsed time, clearing the effect once its envelope ends.
    pub fn tick(&mut self, dt: f32) {
        if let Some((spec, elapsed)) = &mut self.active {
            *elapsed += dt;
            if *elapsed >= spec.fade_in_secs + spec.hold_secs + spec.fade_out_secs {
                self.active = None;
            }
        }
    }

    /// The tint to composite this frame: `(color, strength)`, with
    /// `strength` at `0.0` (color irrelevant) when no effect is active.
    pub fn current(&self) -> ([f32; 3], f32) {
        let Some((spec, elapsed)) = &self.active else {
            return ([0.0; 3], 0.0);
        };
        let t = *elapsed;
        let envelope = if t < spec.fade_in_secs {
            if spec.fade_in_secs > 0.0 { t / spec.fade_in_secs } else { 1.0 }
        } else if t < spec.fade_in_secs + spec.hold_secs {
            1.0
        } else {
            let fade_t = t - spec.fade_in_secs - spec.hold_secs;
            if spec.fade_out_secs > 0.0 {
                (1.0 - fade_t / spec.fade_out_secs).max(0.0)
            } else {
                0.0
            }
        };
        (spec.color, envelope * spec.strength)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> ScreenEffectSpec {
        ScreenEffectSpec {
            color: [1.0, 0.0, 0.0],
            strength: 0.8,
            fade_in_secs: 0.2,
            hold_secs: 0.2,
            fade_out_secs: 0.4,
        }
    }

    #[test]
    fn idle_state_has_zero_strength() {
        let state = ScreenEffectState::default();
        let (_, strength) = state.current();
        assert_eq!(strength, 0.0);
    }

    #[test]
    fn mid_fade_in_ramps_up_linearly() {
        let mut state = ScreenEffectState::default();
        state.trigger(spec());
        state.tick(0.1);
        let (color, strength) = state.current();
        assert_eq!(color, [1.0, 0.0, 0.0]);
        assert!((strength - 0.4).abs() < 1e-5);
    }

    #[test]
    fn hold_is_full_strength() {
        let mut state = ScreenEffectState::default();
        state.trigger(spec());
        state.tick(0.3);
        let (_, strength) = state.current();
        assert!((strength - 0.8).abs() < 1e-5);
    }

    #[test]
    fn mid_fade_out_ramps_down_linearly() {
        let mut state = ScreenEffectState::default();
        state.trigger(spec());
        // fade_in (0.2) + hold (0.2) + half of fade_out (0.2 of 0.4) = 0.6
        state.tick(0.6);
        let (_, strength) = state.current();
        assert!((strength - 0.4).abs() < 1e-5);
    }

    #[test]
    fn past_the_end_clears_the_effect() {
        let mut state = ScreenEffectState::default();
        state.trigger(spec());
        state.tick(10.0);
        let (_, strength) = state.current();
        assert_eq!(strength, 0.0);
    }

    #[test]
    fn triggering_again_restarts_from_the_beginning() {
        let mut state = ScreenEffectState::default();
        state.trigger(spec());
        state.tick(10.0);
        state.trigger(spec());
        let (_, strength) = state.current();
        assert_eq!(strength, 0.0); // t=0, fade_in_secs > 0
    }
}
