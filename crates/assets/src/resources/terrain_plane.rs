use anyhow::Result;
use gfx::util::staging_buffer::StagingBuffer;
use gfx::SharedContext;
use inject::DI;
use log::trace;
use phobos::domain::{ExecutionDomain, Transfer};
use phobos::traits::*;
use phobos::{vk, Buffer, BufferView, IncompleteCommandBuffer};
use rayon::prelude::*;
use scheduler::EventBus;
use util::ByteSize;

use crate::asset::Asset;
use crate::TerrainOptions;

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

impl Asset for TerrainPlane {
    type LoadInfo = TerrainOptions;

    fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> Result<Self>
    where
        Self: Sized, {
        generate_terrain_mesh(info, bus)
    }
}

struct BufferCopyResult<'a, D: ExecutionDomain> {
    pub cmd: IncompleteCommandBuffer<'a, D>,
    pub buffer: Buffer,
    pub staging: StagingBuffer,
}

fn copy_buffer<'a, T: Copy, D: ExecutionDomain + TransferSupport>(
    mut gfx: SharedContext,
    cmd: IncompleteCommandBuffer<'a, D>,
    src: &[T],
    usage: vk::BufferUsageFlags,
) -> Result<BufferCopyResult<'a, D>> {
    let mut staging = StagingBuffer::new(&mut gfx, src.byte_size())?;
    let buffer = Buffer::new_device_local(
        gfx.device.clone(),
        &mut gfx.allocator,
        src.byte_size() as u64,
        usage | vk::BufferUsageFlags::TRANSFER_DST,
    )?;
    let result_view = buffer.view_full();
    staging.mapped_slice::<T>()?.copy_from_slice(src);

    let cmd = cmd.copy_buffer(&staging.view, &result_view)?;
    Ok(BufferCopyResult {
        cmd,
        buffer,
        staging,
    })
}

fn generate_terrain_mesh(options: TerrainOptions, bus: EventBus<DI>) -> Result<TerrainPlane> {
    let gfx = bus
        .data()
        .read()
        .unwrap()
        .get::<SharedContext>()
        .cloned()
        .unwrap();
    trace!(
        "Generating terrain mesh with patch resolution {}x{}",
        options.patch_resolution,
        options.patch_resolution
    );
    let verts: Vec<f32> = (0..options.patch_resolution)
        .into_par_iter()
        .flat_map(|x| {
            (0..options.patch_resolution)
                .into_par_iter()
                .flat_map(|y| {
                    let coords = options.patch_coords(x, y);
                    let uvs = options.patch_uvs(x, y);
                    [coords.x, coords.y, uvs.x, uvs.y]
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
        .on_domain::<Transfer, _>(Some(gfx.pipelines.clone()), Some(gfx.descriptors.clone()))?;

    let BufferCopyResult {
        cmd,
        buffer: vertices,
        staging: _staging_vertices,
    } = copy_buffer(
        gfx.clone(),
        cmd,
        verts.as_slice(),
        vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::STORAGE_BUFFER,
    )?;
    let vertices_view = vertices.view_full();

    let BufferCopyResult {
        cmd,
        buffer: indices,
        staging: _staging_indices,
    } = copy_buffer(gfx.clone(), cmd, indices.as_slice(), vk::BufferUsageFlags::INDEX_BUFFER)?;

    gfx.exec.submit(cmd.finish()?)?.wait_and_yield()?;

    Ok(TerrainPlane {
        vertices_view,
        indices_view: indices.view_full(),
        vertices,
        indices,
        index_count: w * w * 4,
    })
}
