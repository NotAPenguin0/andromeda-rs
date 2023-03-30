use anyhow::Result;
use phobos::domain::{ExecutionDomain, Transfer};
use phobos::traits::*;
use phobos::{vk, Buffer, BufferView, IncompleteCommandBuffer, MemoryType};
use rayon::prelude::*;

use crate::gfx;
use crate::gfx::resource::deferred_delete::DeleteDeferred;
use crate::gfx::world::TerrainOptions;

/// A plane terrain mesh, used as a base for tesselation and rendering the terrain.
#[derive(Debug)]
pub struct TerrainPlane {
    /// Vertex buffer layout:
    /// - Interleaved
    /// - Attribute 0: float2 Pos
    /// - Attribute 1: float2 UV
    pub vertices: Buffer,
    pub vertices_view: BufferView,
    pub indices: Buffer,
    pub indices_view: BufferView,
    pub index_count: u32,
}

impl TerrainPlane {
    // First buffer in return value is the resulting buffer, the second is the staging buffer used.
    fn copy_buffer<'a, T: Copy, D: ExecutionDomain + TransferSupport>(
        mut gfx: gfx::SharedContext,
        cmd: IncompleteCommandBuffer<'a, D>,
        src: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<(IncompleteCommandBuffer<'a, D>, Buffer, Buffer)> {
        let staging = Buffer::new(
            gfx.device.clone(),
            &mut gfx.allocator,
            (src.len() * std::mem::size_of::<T>()) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryType::CpuToGpu,
        )?;
        let mut staging_view = staging.view_full();
        let result = Buffer::new_device_local(
            gfx.device.clone(),
            &mut gfx.allocator,
            (src.len() * std::mem::size_of::<T>()) as u64,
            usage | vk::BufferUsageFlags::TRANSFER_DST,
        )?;
        let result_view = result.view_full();
        staging_view.mapped_slice::<T>()?.copy_from_slice(src);

        let cmd = cmd.copy_buffer(&staging_view, &result_view)?;
        Ok((cmd, result, staging))
    }

    pub fn generate(gfx: gfx::SharedContext, options: TerrainOptions) -> Result<Self> {
        trace!(
            "Generating terrain mesh with patch resolution {}x{}",
            options.patch_resolution,
            options.patch_resolution
        );
        let resolution = options.patch_resolution as f32;
        let patch_size = options.horizontal_scale / resolution;
        let verts: Vec<f32> = (0..options.patch_resolution)
            .into_par_iter()
            .flat_map(|x| {
                (0..options.patch_resolution)
                    .into_par_iter()
                    .flat_map(|y| {
                        let x = x as f32;
                        let y = y as f32;
                        [
                            x * patch_size + patch_size / 2.0 - resolution * patch_size / 2.0,
                            y * patch_size + patch_size / 2.0 - resolution * patch_size / 2.0,
                            x / resolution,
                            y / resolution,
                        ]
                    })
                    .collect::<Vec<f32>>()
            })
            .collect();
        let w = options.patch_resolution - 1;
        let indices: Vec<u32> = (0..w)
            .into_par_iter()
            .flat_map(|x| {
                (0..w).into_par_iter().flat_map(move |y| {
                    let base = x + y * options.patch_resolution;
                    [
                        base,
                        base + options.patch_resolution,
                        base + options.patch_resolution + 1,
                        base + 1,
                    ]
                })
            })
            .collect();

        trace!("Uploading terrain mesh to GPU");
        let cmd = gfx
            .exec
            .on_domain::<Transfer>(Some(gfx.pipelines.clone()), Some(gfx.descriptors.clone()))?;

        let (cmd, vertices, _vert_staging) = Self::copy_buffer(
            gfx.clone(),
            cmd,
            verts.as_slice(),
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;
        let vertices_view = vertices.view_full();

        let (cmd, indices, _idx_staging) = Self::copy_buffer(
            gfx.clone(),
            cmd,
            indices.as_slice(),
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;

        gfx.exec.submit(cmd.finish()?)?.wait_and_yield()?;

        Ok(Self {
            vertices_view,
            indices_view: indices.view_full(),
            vertices,
            indices,
            index_count: w * w * 4,
        })
    }
}

impl DeleteDeferred for TerrainPlane {}
