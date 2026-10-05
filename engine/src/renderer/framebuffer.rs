use glow::HasContext;

/// A low-resolution color+depth render target. Its color texture is always
/// GL_NEAREST filtered with no mipmaps — sampling it into a larger viewport
/// during the composite pass is what produces the blocky PS2-style pixelation.
pub struct OffscreenFramebuffer {
    fbo: glow::Framebuffer,
    pub color_texture: glow::Texture,
    depth_renderbuffer: glow::Renderbuffer,
    pub width: u32,
    pub height: u32,
}

impl OffscreenFramebuffer {
    pub fn new(gl: &glow::Context, width: u32, height: u32) -> anyhow::Result<Self> {
        let width = width.max(1);
        let height = height.max(1);
        unsafe {
            let fbo = gl.create_framebuffer().map_err(anyhow::Error::msg)?;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));

            let color_texture = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(color_texture));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                None,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(color_texture),
                0,
            );

            let depth_renderbuffer = gl.create_renderbuffer().map_err(anyhow::Error::msg)?;
            gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth_renderbuffer));
            gl.renderbuffer_storage(
                glow::RENDERBUFFER,
                glow::DEPTH_COMPONENT24,
                width as i32,
                height as i32,
            );
            gl.framebuffer_renderbuffer(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::RENDERBUFFER,
                Some(depth_renderbuffer),
            );

            let status = gl.check_framebuffer_status(glow::FRAMEBUFFER);
            if status != glow::FRAMEBUFFER_COMPLETE {
                // Bail-out path (reachable via `resize_if_needed` if a driver
                // rejects a new size): free the three objects created above
                // and unbind, or they'd be orphaned with no handle to reclaim
                // them since `Self` is never returned here.
                gl.bind_texture(glow::TEXTURE_2D, None);
                gl.bind_renderbuffer(glow::RENDERBUFFER, None);
                gl.bind_framebuffer(glow::FRAMEBUFFER, None);
                gl.delete_renderbuffer(depth_renderbuffer);
                gl.delete_texture(color_texture);
                gl.delete_framebuffer(fbo);
                anyhow::bail!("offscreen framebuffer incomplete: 0x{status:x}");
            }

            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_renderbuffer(glow::RENDERBUFFER, None);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Ok(Self {
                fbo,
                color_texture,
                depth_renderbuffer,
                width,
                height,
            })
        }
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo));
            gl.viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    /// # Safety
    /// The framebuffer must not be bound or used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_framebuffer(self.fbo);
        gl.delete_texture(self.color_texture);
        gl.delete_renderbuffer(self.depth_renderbuffer);
    }
}
