use anyhow::Result;
use phobos::prelude as ph;
use phobos::traits::*;
use poll_promise::Promise;

use crate::gfx;

/// A plane terrain mesh, used as a base for tesselation and rendering the terrain.
#[derive(Debug)]
pub struct TerrainPlane {
    /// Vertex buffer layout:
    /// - Interleaved
    /// - Attribute 0: vec2 Pos
    pub vertices: ph::Buffer,
    pub vertices_view: ph::BufferView,
    pub vertex_count: u32,
}

impl TerrainPlane {
    pub fn generate(gfx: gfx::SharedContext) -> Promise<Result<Self>> {
        Promise::spawn_async(async move {
            trace!("Regenerating terrain mesh");
            // vec2 position data for two triangles, so a single quad (we will obviously improve this).
            let verts: [f32; 12] = [1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0].map(|val| val * 10.0);
            let buffer = ph::staged_buffer_upload(gfx.device, gfx.allocator, gfx.exec, &verts).await?;
            Ok(Self {
                vertices_view: buffer.view_full(),
                vertices: buffer,
                vertex_count: 6,
            })
        })
    }
}
