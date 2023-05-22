use anyhow::Result;
use assets::handle::Handle;
use assets::storage::AssetStorage;
use assets::texture::format::{SRgba, TextureFormat};
use assets::texture::Texture;
use assets::{Heightmap, NormalMap, Terrain, TerrainOptions, TerrainPlane};
use gfx::Samplers;
use glam::{Vec2, Vec3};
use inject::DI;
use phobos::domain::ExecutionDomain;
use phobos::{vk, ComputeCmdBuffer, ComputeSupport, IncompleteCommandBuffer, PipelineStage};
use scheduler::EventBus;
use world::World;

/// Returns true if the position is on the terrain mesh, false if outside.
pub fn position_on_terrain(position: Vec3) -> bool {
    // If any of the values inside the position are NaN or infinite, the position is outside
    // of the rendered terrain mesh and we do not want to actually use the brush.
    !position.is_nan() && position.is_finite()
}

/// Returns terrain information of the current world.
/// # DI Access
/// Read [`World`]
pub fn get_terrain_info(bus: &EventBus<DI>) -> (Option<Handle<Terrain>>, TerrainOptions) {
    let di = bus.data().read().unwrap();
    let world = di.read_sync::<World>().unwrap();
    (world.terrain, world.terrain_options)
}

pub fn with_ready_terrain<F, R>(bus: &EventBus<DI>, handle: Handle<Terrain>, f: F) -> R
where
    F: FnOnce(&Heightmap, &NormalMap, &Texture<SRgba<u8>>, &TerrainPlane) -> R, {
    let di = bus.data().read().unwrap();
    let assets = di.get::<AssetStorage>().unwrap();
    // Note that this wait should complete instantly, since without a loaded
    // terrain we cannot use a brush.
    assets
        .with_when_ready(handle, |terrain| {
            terrain.with_when_ready(bus, |heights, normals, texture, mesh| {
                f(heights, normals, texture, mesh)
            })
        })
        .flatten()
        .unwrap()
}

/// Transition image to correct layout with an execution barrier to COMPUTE RW
pub fn prepare_for_write<'q, D: ExecutionDomain, F: TextureFormat>(
    texture: &Texture<F>,
    cmd: IncompleteCommandBuffer<'q, D>,
    src: PipelineStage,
) -> IncompleteCommandBuffer<'q, D> {
    cmd.transition_image(
        &texture.image.view,
        src,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    )
}

/// Transition image to correct layout with an execution barrier from COMPUTE RW
pub fn prepare_for_read<'q, D: ExecutionDomain, F: TextureFormat>(
    texture: &Texture<F>,
    cmd: IncompleteCommandBuffer<'q, D>,
    dst_stage: PipelineStage,
    dst_access: vk::AccessFlags2,
) -> IncompleteCommandBuffer<'q, D> {
    cmd.transition_image(
        &texture.image.view,
        PipelineStage::COMPUTE_SHADER,
        dst_stage,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        dst_access,
    )
}

pub fn dispatch_patch_rect<C: ComputeCmdBuffer>(cmd: C, radius: u32, local_size: u32) -> Result<C> {
    let invocations = (radius as f32 / local_size as f32).ceil() as u32;
    cmd.dispatch(invocations, invocations, 1)
}

/// Does no synchronization of accesses to `heights` and `normals`
pub fn update_normals_around_patch<'q, D: ExecutionDomain + ComputeSupport>(
    bus: &EventBus<DI>,
    cmd: IncompleteCommandBuffer<'q, D>,
    uv: Vec2,
    patch_radius: u32,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<IncompleteCommandBuffer<'q, D>> {
    // Grab a suitable sampler to sample the heightmap
    let di = bus.data().read().unwrap();
    let samplers = di.get::<Samplers>().unwrap();
    let sampler = &samplers.linear;
    // Add a small radius around the brush range because the normals around the entire area
    // also need to be updated
    let size = patch_radius + 4;
    let cmd = cmd.bind_compute_pipeline("normal_recompute")?;
    let cmd = cmd
        .bind_storage_image(0, 0, &normals.image.image.view)?
        .bind_sampled_image(0, 1, &heights.image.image.view, sampler)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &size);
    dispatch_patch_rect(cmd, size, 16)
}
