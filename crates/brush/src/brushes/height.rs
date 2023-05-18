use anyhow::{bail, Result};
use assets::storage::AssetStorage;
use assets::{Heightmap, NormalMap, TerrainOptions};
use gfx::{Samplers, SharedContext};
use glam::{Vec2, Vec3};
use inject::DI;
use log::trace;
use pass::GpuWork;
use phobos::domain::All;
use phobos::{
    vk, CommandBuffer, ComputeCmdBuffer, IncompleteCmdBuffer, IncompleteCommandBuffer,
    PipelineStage, Sampler,
};
use scheduler::EventBus;
use time::Time;
use world::World;

use crate::{ApplyBrush, BrushSettings};

/// Simple height brush that smoothly changes the height in the applied area
#[derive(Debug, Copy, Clone)]
pub struct SmoothHeight {}

fn record_update_normals<'q>(
    cmd: IncompleteCommandBuffer<'q, All>,
    uv: Vec2,
    pixel_radius: u32,
    settings: &BrushSettings,
    sampler: &Sampler,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<IncompleteCommandBuffer<'q, All>> {
    // Transition the normal map for writing
    let cmd = cmd.transition_image(
        &normals.image.image.view,
        PipelineStage::FRAGMENT_SHADER,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    // Add a small radius around the brush range because the normals around the entire area
    // also need to be updated
    let size = pixel_radius + 4;
    let cmd = cmd.bind_compute_pipeline("normal_recompute")?;
    let cmd = cmd
        .bind_storage_image(0, 0, &normals.image.image.view)?
        .bind_sampled_image(0, 1, &heights.image.image.view, sampler)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &size)
        .dispatch(
            (size as f32 / 16.0f32).ceil() as u32,
            (size as f32 / 16.0f32).ceil() as u32,
            1,
        )?;
    // Transition the normal map back to ShaderReadOnlyOptimal for drawing
    let cmd = cmd.transition_image(
        &normals.image.image.view,
        PipelineStage::COMPUTE_SHADER,
        PipelineStage::BOTTOM_OF_PIPE,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        vk::AccessFlags2::NONE,
    );
    Ok(cmd)
}

fn record_update_commands(
    bus: &EventBus<DI>,
    cmd: IncompleteCommandBuffer<All>,
    uv: Vec2,
    pixel_radius: u32,
    settings: &BrushSettings,
    sampler: &Sampler,
    heights: &Heightmap,
    normals: &NormalMap,
) -> Result<CommandBuffer<All>> {
    // We are going to write to this image in a compute shader, so submit a barrier for this first.
    let cmd = cmd.transition_image(
        &heights.image.image.view,
        PipelineStage::TESSELLATION_EVALUATION_SHADER,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::ImageLayout::GENERAL,
        vk::AccessFlags2::NONE,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
    );
    // Bind the pipeline we will use to update the heightmap
    let cmd = cmd.bind_compute_pipeline("height_brush")?;
    // Scale weight with frametime for consistency across runs and different speeds
    let weight = {
        let di = bus.data().read().unwrap();
        let time = di.read_sync::<Time>().unwrap();
        settings.weight * time.delta.as_secs_f32()
    };
    // Bind the image to the descriptor, push our uvs to the shader and dispatch our compute shader
    let cmd = cmd
        .bind_storage_image(0, 0, &heights.image.image.view)?
        .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &weight)
        .push_constant(vk::ShaderStageFlags::COMPUTE, 12, &pixel_radius)
        .dispatch(
            (pixel_radius as f32 / 16.0f32).ceil() as u32,
            (pixel_radius as f32 / 16.0f32).ceil() as u32,
            1,
        )?;
    // Transition back to ShaderReadOnlyOptimal for drawing. This also synchronizes access to the heightmap
    // with the normal map calculation shader
    let cmd = cmd.transition_image(
        &heights.image.image.view,
        PipelineStage::COMPUTE_SHADER,
        PipelineStage::COMPUTE_SHADER,
        vk::ImageLayout::GENERAL,
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        vk::AccessFlags2::SHADER_SAMPLED_READ,
    );
    let cmd = record_update_normals(cmd, uv, pixel_radius, settings, sampler, heights, normals)?;
    cmd.finish()
}

fn update_heightmap(
    position: Vec3,
    uv: Vec2,
    bus: &EventBus<DI>,
    sampler: &Sampler,
    settings: &BrushSettings,
) -> Result<()> {
    let di = bus.data().read().unwrap();
    let (terrain_handle, opts) = {
        let world = di.read_sync::<World>().unwrap();
        (world.terrain, world.terrain_options)
    };
    // If no terrain handle was set, we cannot reasonably use a brush on it
    let Some(terrain_handle) = terrain_handle else { bail!("Used brush but terrain handle is not set.") };
    // Get the asset system so we can wait until the terrain is loaded.
    // Note that this should usually complete quickly, since without a loaded
    // terrain we cannot use a brush.
    let assets = di.get::<AssetStorage>().unwrap();
    assets
        .with_when_ready(terrain_handle, |terrain| {
            terrain.with_when_ready(bus, |heights, normals, _, _| {
                // Get the graphics context and allocate a command buffer
                let ctx = di.get::<SharedContext>().cloned().unwrap();
                let cmd = ctx.exec.on_domain::<All, _>(
                    Some(ctx.pipelines.clone()),
                    Some(ctx.descriptors.clone()),
                )?;
                let pixel_radius = opts.texel_radius(position, settings.radius, &heights.image);
                let cmd = record_update_commands(
                    bus,
                    cmd,
                    uv,
                    pixel_radius,
                    settings,
                    sampler,
                    heights,
                    normals,
                )?;
                // Submit our commands once a batch is ready
                GpuWork::with_batch(bus, move |batch| batch.submit(cmd))??;
                Ok::<_, anyhow::Error>(())
            })
        })
        .flatten()
        .unwrap_or(Ok(()))?;
    Ok(())
}

impl ApplyBrush for SmoothHeight {
    fn apply(&self, bus: &EventBus<DI>, position: Vec3, settings: &BrushSettings) -> Result<()> {
        // If any of the values inside the position are NaN or infinite, the position is outside
        // of the rendered terrain mesh and we do not want to actually use the brush.
        if position.is_nan() || !position.is_finite() {
            return Ok(());
        }

        let di = bus.data().read().unwrap();
        let world = di.read_sync::<World>().unwrap();
        // Grab a linear sampler to use
        let samplers = di.get::<Samplers>().unwrap();

        // We will apply our brush mainly to the heightmap texture for now. To know how
        // to do this, we need to find the UV coordinates of the heightmap texture
        // at the position we clicked at.
        let uv = world.terrain_options.uv_at(position);
        update_heightmap(position, uv, bus, &samplers.linear, settings)?;
        Ok(())
    }
}
