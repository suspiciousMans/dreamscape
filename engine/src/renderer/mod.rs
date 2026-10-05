mod framebuffer;
mod particles;
mod post;
mod skybox;

pub use framebuffer::OffscreenFramebuffer;
pub use particles::ParticlePass;
pub use post::{CompositePass, PostParams, DEFAULT_FRAGMENT_SRC};
pub use skybox::SkyboxPass;

use glow::HasContext;

use crate::mesh::GpuMesh;

/// Owns the low-res offscreen pass and the composite blit that upscales it
/// back to the window. Games render their scene between `begin_scene` and
/// `present`; everything in between lands in the low-res target.
pub struct Renderer {
    offscreen: OffscreenFramebuffer,
    composite: CompositePass,
    skybox: SkyboxPass,
    particles: ParticlePass,
    resolution_scale: f32,
}

fn scaled_size((w, h): (u32, u32), scale: f32) -> (u32, u32) {
    (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    )
}

impl Renderer {
    pub fn new(
        gl: &glow::Context,
        drawable_size: (u32, u32),
        resolution_scale: f32,
        post_fragment_src: &str,
    ) -> anyhow::Result<Self> {
        let (w, h) = scaled_size(drawable_size, resolution_scale);
        Ok(Self {
            offscreen: OffscreenFramebuffer::new(gl, w, h)?,
            composite: CompositePass::new(gl, post_fragment_src)?,
            skybox: SkyboxPass::new(gl)?,
            particles: ParticlePass::new(gl)?,
            resolution_scale,
        })
    }

    /// Draws the sky gradient — call right after `begin_scene`, before any
    /// mesh, so scene geometry draws over it normally.
    pub fn draw_skybox(&self, gl: &glow::Context, inv_view_proj: [f32; 16], horizon_color: [f32; 3], zenith_color: [f32; 3]) {
        self.skybox.draw(gl, inv_view_proj, horizon_color, zenith_color);
    }

    /// Draws every `(mesh, model, color)` particle, blended — call after
    /// the opaque mesh loop so particles composite over real geometry.
    pub fn draw_particles(
        &self,
        gl: &glow::Context,
        view: &[f32; 16],
        proj: &[f32; 16],
        particles: impl Iterator<Item = (std::sync::Arc<GpuMesh>, [f32; 16], [f32; 4])>,
    ) {
        self.particles.begin(gl, view, proj);
        for (mesh, model, color) in particles {
            self.particles.draw_particle(gl, &mesh, &model, color);
        }
        self.particles.end(gl);
    }

    /// Recompiles the composite pass with a different post fragment shader
    /// (e.g. when a shader profile is switched).
    pub fn set_composite_shader(
        &mut self,
        gl: &glow::Context,
        post_fragment_src: &str,
    ) -> anyhow::Result<()> {
        // Build the replacement first, so a compile/link failure leaves the
        // existing pass intact. Only once the new pass is ready do we tear
        // down the old one's GL program and VAO — `CompositePass` has no
        // `Drop`, so without this explicit `destroy` those objects would
        // leak in the driver on every shader-profile switch (unlike
        // `resize_if_needed`, which already destroys the old framebuffer).
        let composite = CompositePass::new(gl, post_fragment_src)?;
        unsafe {
            self.composite.destroy(gl);
        }
        self.composite = composite;
        Ok(())
    }

    pub fn set_resolution_scale(&mut self, scale: f32) {
        self.resolution_scale = scale;
    }

    /// Recreates the offscreen target if the window size or resolution scale
    /// changed since the last frame. Cheap no-op otherwise.
    pub fn resize_if_needed(
        &mut self,
        gl: &glow::Context,
        drawable_size: (u32, u32),
    ) -> anyhow::Result<()> {
        let (w, h) = scaled_size(drawable_size, self.resolution_scale);
        if (w, h) != (self.offscreen.width, self.offscreen.height) {
            unsafe {
                self.offscreen.destroy(gl);
            }
            self.offscreen = OffscreenFramebuffer::new(gl, w, h)?;
        }
        Ok(())
    }

    /// Binds the low-res offscreen target and sets its viewport. Call once
    /// per frame before issuing scene draw calls.
    ///
    /// Also unconditionally re-enables `GL_DEPTH_TEST`: an embedder's egui
    /// pass (drawn after `present()`, on top of the composited frame) draws
    /// through `egui_glow`'s own painter, which disables depth testing for
    /// its 2D drawing and does not restore it afterward. Since depth testing
    /// was otherwise only ever enabled once at app startup, every frame
    /// after the first egui paint would render the entire 3D scene with it
    /// off — occlusion silently falling back to raw draw order instead of
    /// actual distance, which reads as solid objects being invisible/
    /// see-through from certain angles. Re-enabling it here, before any
    /// scene geometry is drawn, makes each frame self-sufficient regardless
    /// of whatever GL state the previous frame's UI pass left behind.
    pub fn begin_scene(&self, gl: &glow::Context) {
        self.offscreen.bind(gl);
        unsafe {
            gl.enable(glow::DEPTH_TEST);
        }
    }

    /// Unbinds the offscreen target and blits it to the default framebuffer
    /// at the window's full drawable size.
    pub fn present(
        &self,
        gl: &glow::Context,
        window_drawable_size: (u32, u32),
        post_params: &PostParams,
    ) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(
                0,
                0,
                window_drawable_size.0 as i32,
                window_drawable_size.1 as i32,
            );
        }
        self.composite
            .draw(gl, self.offscreen.color_texture, post_params);
    }
}
