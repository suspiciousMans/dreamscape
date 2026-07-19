use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// A camera that orbits a target point at a fixed distance, driven by yaw/pitch.
pub struct OrbitCamera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

const PITCH_LIMIT: f32 = 1.5533; // ~89 degrees, avoids the view-matrix singularity at the poles

impl OrbitCamera {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            distance,
            yaw: 0.0,
            pitch: 0.3,
            fov_y_radians: 45f32.to_radians(),
            near: 0.1,
            far: 500.0,
            min_distance: 0.5,
            max_distance: 100.0,
        }
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance + delta).clamp(self.min_distance, self.max_distance);
    }

    pub fn position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.position(), self.target, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_y_radians, aspect, self.near, self.far)
    }

    /// Looking direction (from the camera's current position toward
    /// `target`) — matches `FirstPersonCamera::forward`'s naming/shape so
    /// callers can flatten/use it the same way (e.g. WASD fly movement).
    pub fn forward(&self) -> Vec3 {
        (self.target - self.position()).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }
}

/// A free-look camera driven by yaw/pitch with no orbit target — the eye
/// position comes from whatever entity owns it (e.g. the player) each call
/// instead of being stored here.
pub struct FirstPersonCamera {
    pub yaw: f32,
    pub pitch: f32,
    /// Tilt around the look direction (radians) — drives strafe-tilt/camera
    /// shake; zero is level. Not touched by `look()`, which is yaw/pitch
    /// only; callers (e.g. `StrafeTilt`) set it directly.
    pub roll: f32,
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl FirstPersonCamera {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            roll: 0.0,
            fov_y_radians: 60f32.to_radians(),
            // `far` is kept far tighter than a generously large default
            // (e.g. 500) to preserve depth-buffer precision at range — a
            // standard (non-reversed) depth buffer concentrates almost all
            // of its precision near the near plane, so a wide near:far
            // ratio starves distant geometry of precision.
            //
            // `near`, though, has to stay small: the player's collider can
            // press right up against solid geometry (walls, pillars), and
            // at a grazing angle a nearby edge/corner can be closer to the
            // camera — in view-space depth — than the face's straight-on
            // distance suggests. If `near` isn't comfortably below that,
            // the near plane clips that corner away, letting whatever is
            // far behind it show through a solid-looking wall. Pulling
            // `near` back in (it was briefly raised to 0.1 chasing distant
            // z-fighting, which reintroduced exactly this) restores that
            // margin while `far` still does most of the precision work.
            near: 0.02,
            far: 100.0,
        }
    }

    pub fn look(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch = (self.pitch + delta_pitch).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    }

    /// Looking direction; yaw=0, pitch=0 faces -Z (the standard "into the
    /// screen" direction), matching `Mat4::look_at_rh`'s expectations.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn view_matrix(&self, eye_position: Vec3) -> Mat4 {
        let forward = self.forward();
        let up = if self.roll.abs() > 1e-6 {
            Quat::from_axis_angle(forward, self.roll) * Vec3::Y
        } else {
            Vec3::Y
        };
        Mat4::look_at_rh(eye_position, eye_position + forward, up)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh_gl(self.fov_y_radians, aspect, self.near, self.far)
    }
}

impl Default for FirstPersonCamera {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional walking view-bob for a first-person camera — a vertical bob
/// plus a lighter side-to-side sway, synced to how far the player has
/// actually walked (not just "is a key held"), so it naturally speeds up
/// or slows down with movement speed and settles rather than freezing
/// mid-cycle when you stop. `enabled` makes it a toggle, not a fixture —
/// off it's a strict no-op (`offset()` returns zero), so games that don't
/// want the effect pay nothing for it.
pub struct HeadBob {
    pub enabled: bool,
    /// Vertical bob height, world units.
    pub amplitude: f32,
    /// Lateral sway, world units — kept smaller than `amplitude` since real
    /// footfalls read as mostly-vertical with a light sideways drift.
    pub sway_amplitude: f32,
    /// Full bob cycles per unit of horizontal distance walked.
    pub cycles_per_unit: f32,
    phase: f32,
    /// Eases toward 1.0 while walking+grounded and 0.0 otherwise, so the
    /// bob fades in/out smoothly instead of snapping on/off or leaving the
    /// camera stuck mid-offset the instant you stop or leave the ground.
    intensity: f32,
}

impl HeadBob {
    pub fn new() -> Self {
        Self {
            enabled: true,
            amplitude: 0.045,
            sway_amplitude: 0.02,
            cycles_per_unit: 0.6,
            phase: 0.0,
            intensity: 0.0,
        }
    }

    /// Call once per frame with the player's current horizontal speed
    /// (world units/sec) and whether they're grounded.
    pub fn update(&mut self, horizontal_speed: f32, grounded: bool, dt: f32) {
        let walking = grounded && horizontal_speed > 0.05;
        if self.enabled && walking {
            self.phase += horizontal_speed * self.cycles_per_unit * std::f32::consts::TAU * dt;
        }
        let target_intensity = if self.enabled && walking { 1.0 } else { 0.0 };
        const EASE_RATE: f32 = 8.0;
        self.intensity += (target_intensity - self.intensity) * (EASE_RATE * dt).min(1.0);
    }

    /// World-space offset to add to the eye position this frame. `right`
    /// is the camera's current right vector (`FirstPersonCamera::right`),
    /// so the sway stays camera-relative regardless of look direction.
    pub fn offset(&self, right: Vec3) -> Vec3 {
        if self.intensity <= 0.0001 {
            return Vec3::ZERO;
        }
        // Sway at half the vertical frequency: a real gait sways side to
        // side once per full left-right footstep pair, not once per step.
        let vertical = self.phase.sin() * self.amplitude * self.intensity;
        let lateral = (self.phase * 0.5).sin() * self.sway_amplitude * self.intensity;
        Vec3::Y * vertical + right * lateral
    }
}

impl Default for HeadBob {
    fn default() -> Self {
        Self::new()
    }
}

/// A momentary camera shake — authored on a trigger or fired from a
/// script/native `Behavior` via `ScriptApi::camera_shake`, mirroring
/// `engine::screen_effect::ScreenEffectSpec`'s role for full-screen color
/// flashes, but jostling the view instead of tinting it.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CameraShakeSpec {
    /// Peak jitter magnitude, world units.
    pub intensity: f32,
    pub duration_secs: f32,
}

/// Runtime state for the currently-playing camera shake, if any. Owned by
/// the game (like `ScreenEffectState`), ticked once per frame.
#[derive(Default)]
pub struct CameraShakeState {
    active: Option<(CameraShakeSpec, f32)>, // (spec, elapsed)
}

impl CameraShakeState {
    pub fn trigger(&mut self, spec: CameraShakeSpec) {
        self.active = Some((spec, 0.0));
    }

    pub fn tick(&mut self, dt: f32) {
        if let Some((spec, elapsed)) = &mut self.active {
            *elapsed += dt;
            if *elapsed >= spec.duration_secs {
                self.active = None;
            }
        }
    }

    /// World-space jitter to add to the eye position this frame, decaying
    /// linearly to zero over the shake's duration. Built from a few
    /// incommensurate sine waves (rather than a `rand` dependency) so it
    /// reads as jittery/non-repeating without pulling in an RNG crate for
    /// one effect — the mismatched frequencies keep it from ever visibly
    /// looping within a shake's short duration.
    pub fn offset(&self, right: Vec3, up: Vec3) -> Vec3 {
        let Some((spec, elapsed)) = &self.active else {
            return Vec3::ZERO;
        };
        let t = *elapsed;
        let decay = (1.0 - t / spec.duration_secs).max(0.0);
        let amount = spec.intensity * decay;
        let jitter_x = (t * 37.0).sin() + (t * 19.0 + 1.7).sin();
        let jitter_y = (t * 29.0 + 0.6).sin() + (t * 41.0 + 3.1).sin();
        right * (jitter_x * 0.5 * amount) + up * (jitter_y * 0.5 * amount)
    }
}

/// Optional camera roll while strafing — a subtle, classic FPS "lean into
/// the turn" effect. Eases toward a target roll proportional to strafe
/// input rather than snapping, and back to level the same way when you
/// stop strafing.
pub struct StrafeTilt {
    pub enabled: bool,
    pub max_roll_deg: f32,
    pub ease_rate: f32,
    current_roll: f32,
}

impl StrafeTilt {
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_roll_deg: 3.0,
            ease_rate: 6.0,
            current_roll: 0.0,
        }
    }

    /// `strafe_input` is -1..1 (negative = strafing left, positive =
    /// strafing right) — the same signed value `move_dir`'s right-axis
    /// component already is before it's normalized into `RigidBody.velocity`.
    pub fn update(&mut self, strafe_input: f32, dt: f32) {
        let target = if self.enabled {
            -strafe_input.clamp(-1.0, 1.0) * self.max_roll_deg.to_radians()
        } else {
            0.0
        };
        self.current_roll += (target - self.current_roll) * (self.ease_rate * dt).min(1.0);
    }

    /// Current roll, radians — write directly into `FirstPersonCamera::roll`.
    pub fn roll_radians(&self) -> f32 {
        self.current_roll
    }
}

impl Default for StrafeTilt {
    fn default() -> Self {
        Self::new()
    }
}

/// Optional FOV widening at speed — a subtle "sense of speed" cue. Additive
/// on top of whatever base FOV a game/ability system sets, so the two never
/// fight over `FirstPersonCamera::fov_y_radians` directly: a game computes
/// `base_fov + speed_fov.kick_radians()` itself each frame.
pub struct SpeedFov {
    pub enabled: bool,
    pub max_kick_deg: f32,
    /// Horizontal speed (world units/sec) at which the kick reaches
    /// `max_kick_deg` — linearly interpolated below that.
    pub speed_for_max_kick: f32,
    pub ease_rate: f32,
    current_kick_deg: f32,
}

impl SpeedFov {
    pub fn new() -> Self {
        Self {
            enabled: true,
            max_kick_deg: 6.0,
            speed_for_max_kick: 8.0,
            ease_rate: 5.0,
            current_kick_deg: 0.0,
        }
    }

    pub fn update(&mut self, horizontal_speed: f32, dt: f32) {
        let target = if self.enabled && self.speed_for_max_kick > 0.0 {
            (horizontal_speed / self.speed_for_max_kick).clamp(0.0, 1.0) * self.max_kick_deg
        } else {
            0.0
        };
        self.current_kick_deg += (target - self.current_kick_deg) * (self.ease_rate * dt).min(1.0);
    }

    pub fn kick_radians(&self) -> f32 {
        self.current_kick_deg.to_radians()
    }
}

impl Default for SpeedFov {
    fn default() -> Self {
        Self::new()
    }
}

/// A camera dip-and-recover on landing — a lightweight critically-damped
/// spring, kicked downward by `land()` (call once, with the fall speed the
/// player hit the ground at) and integrated every frame by `update()`
/// regardless of grounded state so it always settles back to rest.
pub struct LandingDip {
    pub enabled: bool,
    /// World units of dip per unit of fall speed at landing.
    pub fall_speed_to_dip: f32,
    pub max_dip: f32,
    pub stiffness: f32,
    pub damping: f32,
    offset: f32,
    velocity: f32,
}

impl LandingDip {
    pub fn new() -> Self {
        Self {
            enabled: true,
            fall_speed_to_dip: 0.03,
            max_dip: 0.3,
            // Critically damped (damping = 2*sqrt(stiffness)): dips and
            // recovers smoothly with no bounce-back overshoot.
            stiffness: 400.0,
            damping: 40.0,
            offset: 0.0,
            velocity: 0.0,
        }
    }

    /// Call once, the frame the player transitions from airborne to
    /// grounded, with the (positive) fall speed they landed at.
    pub fn land(&mut self, fall_speed: f32) {
        if !self.enabled {
            return;
        }
        let kick = (fall_speed * self.fall_speed_to_dip).min(self.max_dip);
        self.offset -= kick;
    }

    /// Call every frame to integrate the spring back toward rest.
    pub fn update(&mut self, dt: f32) {
        let accel = -self.stiffness * self.offset - self.damping * self.velocity;
        self.velocity += accel * dt;
        self.offset += self.velocity * dt;
    }

    pub fn offset(&self) -> Vec3 {
        Vec3::new(0.0, self.offset, 0.0)
    }
}

impl Default for LandingDip {
    fn default() -> Self {
        Self::new()
    }
}
