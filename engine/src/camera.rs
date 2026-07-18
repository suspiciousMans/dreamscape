use glam::{Mat4, Vec3};

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
}

/// A free-look camera driven by yaw/pitch with no orbit target — the eye
/// position comes from whatever entity owns it (e.g. the player) each call
/// instead of being stored here.
pub struct FirstPersonCamera {
    pub yaw: f32,
    pub pitch: f32,
    pub fov_y_radians: f32,
    pub near: f32,
    pub far: f32,
}

impl FirstPersonCamera {
    pub fn new() -> Self {
        Self {
            yaw: 0.0,
            pitch: 0.0,
            fov_y_radians: 60f32.to_radians(),
            near: 0.05,
            far: 500.0,
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
        Mat4::look_at_rh(eye_position, eye_position + self.forward(), Vec3::Y)
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
