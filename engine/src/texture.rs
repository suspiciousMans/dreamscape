use std::path::Path;

use glow::HasContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureFilter {
    Nearest,
    Bilinear,
}

/// Writes raw RGBA8 pixels out as a PNG file — used by glTF import to
/// extract an embedded/referenced base-color texture onto disk (glTF
/// stores images as decoded pixel buffers via `gltf::import`, but
/// `LevelObject::texture_path` — like every other texture reference in
/// this engine — expects a real file, not in-memory bytes).
pub fn save_rgba8_png(path: &Path, rgba: &[u8], width: u32, height: u32) -> anyhow::Result<()> {
    let buffer = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| anyhow::anyhow!("RGBA8 buffer size doesn't match {width}x{height}"))?;
    buffer.save(path)?;
    Ok(())
}

pub struct GpuTexture {
    pub handle: glow::Texture,
    pub width: u32,
    pub height: u32,
}

impl GpuTexture {
    pub fn load_from_file(
        gl: &glow::Context,
        path: &Path,
        filter: TextureFilter,
    ) -> anyhow::Result<Self> {
        let img = image::open(path)?.to_rgba8();
        let (width, height) = img.dimensions();
        Self::from_rgba8(gl, &img, width, height, filter)
    }

    pub fn from_rgba8(
        gl: &glow::Context,
        rgba: &[u8],
        width: u32,
        height: u32,
        filter: TextureFilter,
    ) -> anyhow::Result<Self> {
        let gl_filter = match filter {
            TextureFilter::Nearest => glow::NEAREST,
            TextureFilter::Bilinear => glow::LINEAR,
        };

        unsafe {
            let handle = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(handle));
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                gl_filter as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                gl_filter as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::REPEAT as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::REPEAT as i32,
            );
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(rgba),
            );
            gl.bind_texture(glow::TEXTURE_2D, None);

            Ok(Self {
                handle,
                width,
                height,
            })
        }
    }

    pub fn bind(&self, gl: &glow::Context, unit: u32) {
        unsafe {
            gl.active_texture(glow::TEXTURE0 + unit);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.handle));
        }
    }

    /// # Safety
    /// The texture must not be bound or used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_texture(self.handle);
    }
}
