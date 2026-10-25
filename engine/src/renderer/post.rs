use glow::HasContext;

use crate::shader::link_program;

const VERTEX_SRC: &str = r#"
#version 330 core
out vec2 vUV;
void main() {
    vec2 positions[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    vec2 uvs[3] = vec2[3](vec2(0.0, 0.0), vec2(2.0, 0.0), vec2(0.0, 2.0));
    gl_Position = vec4(positions[gl_VertexID], 0.0, 1.0);
    vUV = uvs[gl_VertexID];
}
"#;

/// Plain passthrough; the posterize/dither look is layered on top of this in
/// a later shader-profile revision without changing the composite pipeline.
pub const DEFAULT_FRAGMENT_SRC: &str = r#"
#version 330 core
in vec2 vUV;
out vec4 FragColor;
uniform sampler2D uSourceTex;
void main() {
    FragColor = texture(uSourceTex, vUV);
}
"#;

/// Draws a fullscreen triangle sampling the offscreen color texture into
/// whatever framebuffer/viewport is currently bound (the upscale is "free" —
/// GL_NEAREST sampling a small texture into a larger viewport).
pub struct CompositePass {
    program: glow::Program,
    empty_vao: glow::VertexArray,
    uniform_source_tex: Option<glow::UniformLocation>,
    uniform_color_levels: Option<glow::UniformLocation>,
    uniform_dither_strength: Option<glow::UniformLocation>,
    uniform_tint_color: Option<glow::UniformLocation>,
    uniform_tint_strength: Option<glow::UniformLocation>,
    overlay: OverlayLocs,
    pub(crate) grain_tex: Option<glow::Texture>,
}

/// Uniform locations of the optional overlay (all `None` in shaders without it).
struct OverlayLocs {
    on: Option<glow::UniformLocation>,
    centre: Option<glow::UniformLocation>,
    radii: Option<glow::UniformLocation>,
    ring: Option<glow::UniformLocation>,
    grain: Option<glow::UniformLocation>,
    grain_tex: Option<glow::UniformLocation>,
    view_h: Option<glow::UniformLocation>,
}

/// Rings in the vignette's alpha table (`ScreenOverlay::ring_alpha`).
pub const VIGNETTE_RINGS: usize = 11;

/// A vignette and film grain laid over the composited frame by the composite
/// pass itself (one shader, no extra full-screen blended layers). Only shaders
/// that declare the `uOv*` uniforms (the Dreamscape one) use it; the others
/// ignore it. Coordinates are window pixels, origin top-left.
#[derive(Clone, Copy, Debug)]
pub struct ScreenOverlay {
    /// Where the vignette is clear (the player's screen position).
    pub centre_px: [f32; 2],
    /// Half the screen diagonal in pixels: the unit the radii are fractions of.
    pub half_diag_px: f32,
    /// Fractions of `half_diag_px`: clear inside `inner`, fully dim past `outer`.
    pub inner: f32,
    pub outer: f32,
    /// Darkness (alpha of black) at `inner + (outer - inner) * i / 10`;
    /// linearly interpolated between them.
    pub ring_alpha: [f32; VIGNETTE_RINGS],
    /// Film grain: tile offset (UV), screen pixels per whole tile, and the
    /// strength (0 = off) as a 0..1 tint of the premultiplied texture.
    pub grain_offset: [f32; 2],
    pub grain_tile_px: f32,
    pub grain_strength: f32,
}

/// Values applied by the composite pass's posterize/dither stage. Levels this
/// high with zero dither strength is visually a no-op passthrough.
pub struct PostParams {
    pub color_levels: f32,
    pub dither_strength: f32,
    /// A full-screen color tint (e.g. a damage flash), mixed in before the
    /// posterize/dither stage so it still reads as native to the retro look
    /// rather than a crisp overlay. See `engine::screen_effect`.
    pub tint_color: [f32; 3],
    pub tint_strength: f32,
}

impl Default for PostParams {
    fn default() -> Self {
        Self {
            color_levels: 256.0,
            dither_strength: 0.0,
            tint_color: [0.0; 3],
            tint_strength: 0.0,
        }
    }
}

impl CompositePass {
    pub fn new(gl: &glow::Context, fragment_src: &str) -> anyhow::Result<Self> {
        let program = link_program(gl, VERTEX_SRC, fragment_src)?;
        let uniform_source_tex = unsafe { gl.get_uniform_location(program, "uSourceTex") };
        let uniform_color_levels = unsafe { gl.get_uniform_location(program, "uColorLevels") };
        let uniform_dither_strength =
            unsafe { gl.get_uniform_location(program, "uDitherStrength") };
        let uniform_tint_color = unsafe { gl.get_uniform_location(program, "uTintColor") };
        let uniform_tint_strength = unsafe { gl.get_uniform_location(program, "uTintStrength") };
        let empty_vao = unsafe { gl.create_vertex_array().map_err(anyhow::Error::msg)? };
        let loc = |n: &str| unsafe { gl.get_uniform_location(program, n) };
        let overlay = OverlayLocs {
            on: loc("uOvOn"),
            centre: loc("uOvCentre"),
            radii: loc("uOvRadii"),
            ring: loc("uOvRing[0]").or_else(|| loc("uOvRing")),
            grain: loc("uOvGrain"),
            grain_tex: loc("uOvGrainTex"),
            view_h: loc("uOvViewH"),
        };

        Ok(Self {
            overlay,
            grain_tex: None,
            program,
            empty_vao,
            uniform_source_tex,
            uniform_color_levels,
            uniform_dither_strength,
            uniform_tint_color,
            uniform_tint_strength,
        })
    }

    /// Uploads the film-grain tile (premultiplied RGBA8, `size` x `size`,
    /// nearest-filtered and repeating). Replaces any earlier one.
    pub fn set_grain_texture(&mut self, gl: &glow::Context, size: u32, rgba: &[u8]) {
        assert_eq!(rgba.len(), (size * size * 4) as usize, "grain tile size");
        unsafe {
            if let Some(old) = self.grain_tex.take() {
                gl.delete_texture(old);
            }
            let Ok(t) = gl.create_texture() else { return };
            gl.bind_texture(glow::TEXTURE_2D, Some(t));
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 1);
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                size as i32,
                size as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                Some(rgba),
            );
            for (pname, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::NEAREST),
                (glow::TEXTURE_MAG_FILTER, glow::NEAREST),
                (glow::TEXTURE_WRAP_S, glow::REPEAT),
                (glow::TEXTURE_WRAP_T, glow::REPEAT),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, pname, v as i32);
            }
            self.grain_tex = Some(t);
        }
    }

    pub fn draw(
        &self,
        gl: &glow::Context,
        source_texture: glow::Texture,
        params: &PostParams,
        overlay: Option<&ScreenOverlay>,
        viewport_h: f32,
    ) {
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(source_texture));
            gl.uniform_1_i32(self.uniform_source_tex.as_ref(), 0);
            gl.uniform_1_f32(self.uniform_color_levels.as_ref(), params.color_levels);
            gl.uniform_1_f32(
                self.uniform_dither_strength.as_ref(),
                params.dither_strength,
            );
            gl.uniform_3_f32(
                self.uniform_tint_color.as_ref(),
                params.tint_color[0],
                params.tint_color[1],
                params.tint_color[2],
            );
            gl.uniform_1_f32(self.uniform_tint_strength.as_ref(), params.tint_strength);
            let o = &self.overlay;
            match overlay.filter(|_| o.on.is_some()) {
                Some(ov) => {
                    gl.uniform_1_i32(o.on.as_ref(), 1);
                    gl.uniform_2_f32(o.centre.as_ref(), ov.centre_px[0], ov.centre_px[1]);
                    gl.uniform_3_f32(o.radii.as_ref(), ov.half_diag_px, ov.inner, ov.outer);
                    gl.uniform_1_f32_slice(o.ring.as_ref(), &ov.ring_alpha);
                    let strength = if self.grain_tex.is_some() { ov.grain_strength } else { 0.0 };
                    gl.uniform_4_f32(
                        o.grain.as_ref(),
                        ov.grain_offset[0],
                        ov.grain_offset[1],
                        ov.grain_tile_px.max(1.0),
                        strength,
                    );
                    gl.uniform_1_f32(o.view_h.as_ref(), viewport_h);
                    if let Some(t) = self.grain_tex {
                        gl.active_texture(glow::TEXTURE1);
                        gl.bind_texture(glow::TEXTURE_2D, Some(t));
                        gl.uniform_1_i32(o.grain_tex.as_ref(), 1);
                        gl.active_texture(glow::TEXTURE0);
                    }
                }
                None => {
                    if o.on.is_some() {
                        gl.uniform_1_i32(o.on.as_ref(), 0);
                    }
                }
            }
            gl.bind_vertex_array(Some(self.empty_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.enable(glow::DEPTH_TEST);
        }
    }

    /// # Safety
    /// The pass must not be used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        if let Some(t) = self.grain_tex {
            gl.delete_texture(t);
        }
        gl.delete_program(self.program);
        gl.delete_vertex_array(self.empty_vao);
    }
}

const TRAIL_FRAGMENT_SRC: &str = r#"
#version 330 core
in vec2 vUV;
out vec4 FragColor;
uniform sampler2D uCurrent;
uniform sampler2D uPrevious;
uniform float uAmount;
void main() {
    vec4 now = texture(uCurrent, vUV);
    vec4 before = texture(uPrevious, vUV);
    // Moving things leave fading afterimages: the old frame lingers, but
    // never darker than what's there now (so the scene doesn't dim).
    FragColor = vec4(max(now.rgb, mix(now.rgb, before.rgb, uAmount)), 1.0);
}
"#;

/// Tracers: blends the new frame with the last blended frame, so anything
/// that moves leaves a fading trail. Drawn into a ping-pong target before
/// the composite pass.
pub struct TrailPass {
    program: glow::Program,
    empty_vao: glow::VertexArray,
    uniform_current: Option<glow::UniformLocation>,
    uniform_previous: Option<glow::UniformLocation>,
    uniform_amount: Option<glow::UniformLocation>,
}

impl TrailPass {
    pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
        let program = link_program(gl, VERTEX_SRC, TRAIL_FRAGMENT_SRC)?;
        unsafe {
            Ok(Self {
                uniform_current: gl.get_uniform_location(program, "uCurrent"),
                uniform_previous: gl.get_uniform_location(program, "uPrevious"),
                uniform_amount: gl.get_uniform_location(program, "uAmount"),
                empty_vao: gl.create_vertex_array().map_err(anyhow::Error::msg)?,
                program,
            })
        }
    }

    pub fn draw(
        &self,
        gl: &glow::Context,
        current: glow::Texture,
        previous: glow::Texture,
        amount: f32,
    ) {
        unsafe {
            gl.disable(glow::DEPTH_TEST);
            gl.disable(glow::BLEND);
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(current));
            gl.uniform_1_i32(self.uniform_current.as_ref(), 0);
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(previous));
            gl.uniform_1_i32(self.uniform_previous.as_ref(), 1);
            gl.uniform_1_f32(self.uniform_amount.as_ref(), amount);
            gl.bind_vertex_array(Some(self.empty_vao));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
            gl.active_texture(glow::TEXTURE0);
            gl.enable(glow::DEPTH_TEST);
        }
    }
}
