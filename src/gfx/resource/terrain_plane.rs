use anyhow::Result;
use phobos::prelude as ph;
use phobos::traits::*;
use poll_promise::Promise;
use rayon::prelude::*;

use crate::gfx;
use crate::gfx::world::TerrainOptions;
use crate::thread::spawn_promise;

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
        spawn_promise(move || {
            trace!("Regenerating terrain mesh");
            let width = options.scale;
            let height = options.scale;
            let resolution = options.patch_resolution as f32;
            let mut verts = Vec::new();
            // We make res * res quads, and each quad has 4 vertices
            verts.resize(options.patch_resolution as usize * options.patch_resolution as usize * 4, 0.0);
            let verts: Vec<f32> = (0..options.patch_resolution)
                .into_par_iter()
                .flat_map(|x| {
                    (0..options.patch_resolution)
                        .into_par_iter()
                        .flat_map(|y| {
                            let x = x as f32;
                            let y = y as f32;
                            [
                                -width / 2.0 + width * x / resolution,
                                -height / 2.0 + height * y / resolution,
                                -width / 2.0 + width * (x + 1.0) / resolution,
                                -height / 2.0 + height * y / resolution,
                                -width / 2.0 + width * (x + 1.0) / resolution,
                                -height / 2.0 + height * (y + 1.0) / resolution,
                                -width / 2.0 + width * x / resolution,
                                -height / 2.0 + height * (y + 1.0) / resolution,
                            ]
                        })
                        .collect::<Vec<f32>>()
                })
                .collect();
            let buffer = ph::staged_buffer_upload(gfx.device, gfx.allocator, gfx.exec, verts.as_slice())?.cooperative_wait()?;
            Ok(Self {
                vertices_view: buffer.view_full(),
                vertices: buffer,
                vertex_count: (4 * options.patch_resolution * options.patch_resolution) as u32,
            })
        })
    }
}
