use anyhow::Result;
use phobos::prelude as ph;
use phobos::traits::*;
use poll_promise::Promise;

use crate::gfx;
use crate::gfx::world::TerrainOptions;

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
    pub fn generate(gfx: gfx::SharedContext, options: TerrainOptions) -> Promise<Result<Self>> {
        Promise::spawn_async(async move {
            trace!("Regenerating terrain mesh");
            let width = options.size;
            let height = options.size;
            let resolution = options.patch_resolution as f32;
            let mut verts = Vec::new();
            // We make res * res quads, and each quad has 4 vertices
            verts.reserve(options.patch_resolution as usize * options.patch_resolution as usize * 4);
            // vec2 position data for one quad
            for i in 0..options.patch_resolution {
                for j in 0..options.patch_resolution {
                    let i = i as f32;
                    let j = j as f32;

                    verts.push(-width / 2.0 + width * i / resolution);
                    verts.push(-height / 2.0 + height * j / resolution);

                    verts.push(-width / 2.0 + width * (i + 1.0) / resolution);
                    verts.push(-height / 2.0 + height * j / resolution);

                    verts.push(-width / 2.0 + width * (i + 1.0) / resolution);
                    verts.push(-height / 2.0 + height * (j + 1.0) / resolution);

                    verts.push(-width / 2.0 + width * i / resolution);
                    verts.push(-height / 2.0 + height * (j + 1.0) / resolution);
                }
            }
            let buffer = ph::staged_buffer_upload(gfx.device, gfx.allocator, gfx.exec, verts.as_slice()).await?;
            Ok(Self {
                vertices_view: buffer.view_full(),
                vertices: buffer,
                vertex_count: (4 * options.patch_resolution * options.patch_resolution) as u32,
            })
        })
    }
}
