mod framebuffer;
mod post;

pub use framebuffer::OffscreenFramebuffer;
pub use post::{CompositePass, PostParams, DEFAULT_FRAGMENT_SRC};

use glow::HasContext;

/// Owns the low-res offscreen pass and the composite blit that upscales it
/// back to the window. Games render their scene between `begin_scene` and
/// `present`; everything in between lands in the low-res target.
pub struct Renderer {
    offscreen: OffscreenFramebuffer,
    composite: CompositePass,
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
            resolution_scale,
        })
    }

    /// Recompiles the composite pass with a different post fragment shader
    /// (e.g. when a shader profile is switched).
    pub fn set_composite_shader(
        &mut self,
        gl: &glow::Context,
        post_fragment_src: &str,
    ) -> anyhow::Result<()> {
        self.composite = CompositePass::new(gl, post_fragment_src)?;
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
    pub fn begin_scene(&self, gl: &glow::Context) {
        self.offscreen.bind(gl);
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
