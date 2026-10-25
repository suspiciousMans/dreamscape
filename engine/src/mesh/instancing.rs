//! Instanced drawing: one draw call for many copies of a mesh, each with its
//! own model matrix (a per-instance `mat4` read from vertex attribute
//! locations [`MODEL_ATTR`]..`MODEL_ATTR + 4`).
//!
//! The matrices are streamed into one shared buffer each call, so a frame's
//! hundreds of identical blocks cost a handful of draws instead of hundreds.

use glow::HasContext;

use super::GpuMesh;

/// First of the four attribute locations that carry the instance matrix.
pub const MODEL_ATTR: u32 = 4;

/// A streaming buffer of instance matrices, shared by every instanced draw.
pub struct InstanceBuffer {
    vbo: glow::Buffer,
}

impl InstanceBuffer {
    pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
        let vbo = unsafe { gl.create_buffer().map_err(anyhow::Error::msg)? };
        Ok(Self { vbo })
    }

    /// Draws `mesh` once per matrix in `matrices` (16 column-major floats
    /// each). Leaves `mesh`'s VAO bound, with the instance attributes turned
    /// off again so non-instanced draws through it are unaffected.
    pub fn draw(&self, gl: &glow::Context, mesh: &GpuMesh, matrices: &[f32]) {
        let n = matrices.len() / 16;
        if n == 0 {
            return;
        }
        unsafe {
            gl.bind_vertex_array(Some(mesh.vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&matrices[..n * 16]),
                glow::STREAM_DRAW,
            );
            for i in 0..4u32 {
                let loc = MODEL_ATTR + i;
                gl.enable_vertex_attrib_array(loc);
                gl.vertex_attrib_pointer_f32(loc, 4, glow::FLOAT, false, 64, (i * 16) as i32);
                gl.vertex_attrib_divisor(loc, 1);
            }
            gl.draw_elements_instanced(
                glow::TRIANGLES,
                mesh.index_count,
                glow::UNSIGNED_INT,
                0,
                n as i32,
            );
            for i in 0..4u32 {
                let loc = MODEL_ATTR + i;
                gl.vertex_attrib_divisor(loc, 0);
                gl.disable_vertex_attrib_array(loc);
            }
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }

    /// # Safety
    /// The buffer must not be used after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_buffer(self.vbo);
    }
}
