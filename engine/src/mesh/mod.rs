mod gltf_loader;
mod obj_loader;
pub mod primitives;

pub use gltf_loader::{load_gltf, GltfImage, GltfMeshEntry, GltfScene};
pub use obj_loader::load_obj;

use bytemuck::{Pod, Zeroable};
use glow::HasContext;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

#[derive(Clone, Debug, Default)]
pub struct MeshData {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

/// GPU-resident mesh: a VAO plus its backing vertex/index buffers.
pub struct GpuMesh {
    pub vao: glow::VertexArray,
    vbo: glow::Buffer,
    ebo: glow::Buffer,
    pub index_count: i32,
}

const ATTR_POSITION: u32 = 0;
const ATTR_NORMAL: u32 = 1;
const ATTR_UV: u32 = 2;

impl GpuMesh {
    pub fn upload(gl: &glow::Context, mesh: &MeshData) -> anyhow::Result<Self> {
        unsafe {
            let vao = gl.create_vertex_array().map_err(anyhow::Error::msg)?;
            gl.bind_vertex_array(Some(vao));

            // On a buffer-creation failure, delete the objects already made
            // (the VAO, then the VBO too) before propagating — otherwise
            // they'd be orphaned since no `GpuMesh` is returned to `destroy`.
            let vbo = match gl.create_buffer().map_err(anyhow::Error::msg) {
                Ok(vbo) => vbo,
                Err(err) => {
                    gl.bind_vertex_array(None);
                    gl.delete_vertex_array(vao);
                    return Err(err);
                }
            };
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&mesh.vertices),
                glow::STATIC_DRAW,
            );

            let ebo = match gl.create_buffer().map_err(anyhow::Error::msg) {
                Ok(ebo) => ebo,
                Err(err) => {
                    gl.bind_vertex_array(None);
                    gl.delete_vertex_array(vao);
                    gl.delete_buffer(vbo);
                    return Err(err);
                }
            };
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            gl.buffer_data_u8_slice(
                glow::ELEMENT_ARRAY_BUFFER,
                bytemuck::cast_slice(&mesh.indices),
                glow::STATIC_DRAW,
            );

            let stride = std::mem::size_of::<Vertex>() as i32;
            gl.vertex_attrib_pointer_f32(ATTR_POSITION, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(ATTR_POSITION);
            gl.vertex_attrib_pointer_f32(
                ATTR_NORMAL,
                3,
                glow::FLOAT,
                false,
                stride,
                std::mem::size_of::<[f32; 3]>() as i32,
            );
            gl.enable_vertex_attrib_array(ATTR_NORMAL);
            gl.vertex_attrib_pointer_f32(
                ATTR_UV,
                2,
                glow::FLOAT,
                false,
                stride,
                std::mem::size_of::<[f32; 6]>() as i32,
            );
            gl.enable_vertex_attrib_array(ATTR_UV);

            gl.bind_vertex_array(None);

            Ok(Self {
                vao,
                vbo,
                ebo,
                index_count: mesh.indices.len() as i32,
            })
        }
    }

    pub fn draw(&self, gl: &glow::Context) {
        unsafe {
            gl.bind_vertex_array(Some(self.vao));
            gl.draw_elements(glow::TRIANGLES, self.index_count, glow::UNSIGNED_INT, 0);
        }
    }

    /// # Safety
    /// The mesh must not be used (bound or drawn) after this call.
    pub unsafe fn destroy(&self, gl: &glow::Context) {
        gl.delete_vertex_array(self.vao);
        gl.delete_buffer(self.vbo);
        gl.delete_buffer(self.ebo);
    }
}
