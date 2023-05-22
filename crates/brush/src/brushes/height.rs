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
use strum_macros::Display;
use time::Time;
use world::World;

use crate::util::{
    dispatch_patch_rect, get_terrain_info, position_on_terrain, prepare_for_read,
    prepare_for_write, with_ready_terrain,
};
use crate::{Brush, BrushSettings};

#[derive(Debug, Copy, Clone, PartialEq, Display)]
pub enum WeightFunction {
    // Gaussian curve with given standard deviation
    Gaussian(f32),
}

impl Default for WeightFunction {
    fn default() -> Self {
        // Using 0.3 for the standard deviation means the weight will be near zero
        // at x = 1
        WeightFunction::Gaussian(0.3)
    }
}

/// Simple height brush that smoothly changes the height in the applied area
#[derive(Debug, Default, Copy, Clone)]
pub struct SmoothHeight {
    pub weight_fn: WeightFunction,
}

impl SmoothHeight {
    fn invert_weight(mut settings: BrushSettings) -> BrushSettings {
        // Inverted height brush is simply done by having a negative weight
        if settings.invert {
            settings.weight = -settings.weight;
        }

        settings
    }

    fn record_height_update<'q>(
        &self,
        bus: &EventBus<DI>,
        cmd: IncompleteCommandBuffer<'q, All>,
        uv: Vec2,
        radius: u32,
        settings: &BrushSettings,
        heights: &Heightmap,
    ) -> Result<IncompleteCommandBuffer<'q, All>> {
        // We are going to write to this image in a compute shader, so submit a barrier for this first.
        let cmd =
            prepare_for_write(&heights.image, cmd, PipelineStage::TESSELLATION_EVALUATION_SHADER);
        // Bind the pipeline we will use to update the heightmap
        let cmd = cmd.bind_compute_pipeline("height_brush")?;
        // Scale weight with frametime for consistency across runs and different frame rates
        let weight = {
            let di = bus.data().read().unwrap();
            let time = di.read_sync::<Time>().unwrap();
            settings.weight * time.delta.as_secs_f32()
        };

        // Bind the image to the descriptor, push our uvs to the shader and dispatch our compute shader
        let mut cmd = cmd
            .bind_storage_image(0, 0, &heights.image.image.view)?
            .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
            .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &weight)
            .push_constant(vk::ShaderStageFlags::COMPUTE, 12, &radius);
        match self.weight_fn {
            WeightFunction::Gaussian(sigma) => {
                cmd = cmd.push_constant(vk::ShaderStageFlags::COMPUTE, 16, &sigma);
            }
        };
        let cmd = dispatch_patch_rect(cmd, radius, 16)?;
        Ok(prepare_for_read(
            &heights.image,
            cmd,
            PipelineStage::COMPUTE_SHADER,
            vk::AccessFlags2::SHADER_SAMPLED_READ,
        ))
    }

    fn record_normals_update<'q>(
        &self,
        bus: &EventBus<DI>,
        cmd: IncompleteCommandBuffer<'q, All>,
        uv: Vec2,
        radius: u32,
        heights: &Heightmap,
        normals: &NormalMap,
    ) -> Result<IncompleteCommandBuffer<'q, All>> {
        // Grab a suitable sampler to sample or heightmap
        let di = bus.data().read().unwrap();
        let samplers = di.get::<Samplers>().unwrap();
        let sampler = &samplers.linear;

        let cmd = prepare_for_write(&normals.image, cmd, PipelineStage::FRAGMENT_SHADER);
        // Add a small radius around the brush range because the normals around the entire area
        // also need to be updated
        let size = radius + 4;
        let cmd = cmd.bind_compute_pipeline("normal_recompute")?;
        let cmd = cmd
            .bind_storage_image(0, 0, &normals.image.image.view)?
            .bind_sampled_image(0, 1, &heights.image.image.view, sampler)?
            .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
            .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &size);
        let cmd = dispatch_patch_rect(cmd, size, 16)?;
        Ok(prepare_for_read(
            &normals.image,
            cmd,
            PipelineStage::BOTTOM_OF_PIPE,
            vk::AccessFlags2::NONE,
        ))
    }

    fn record_update_commands(
        &self,
        bus: &EventBus<DI>,
        cmd: IncompleteCommandBuffer<All>,
        uv: Vec2,
        radius: u32,
        settings: &BrushSettings,
        heights: &Heightmap,
        normals: &NormalMap,
    ) -> Result<CommandBuffer<All>> {
        let cmd = self.record_height_update(bus, cmd, uv, radius, settings, heights)?;
        let cmd = self.record_normals_update(bus, cmd, uv, radius, heights, normals)?;
        cmd.finish()
    }

    fn apply_to_terrain(
        &self,
        bus: &EventBus<DI>,
        position: Vec3,
        uv: Vec2,
        settings: BrushSettings,
        options: TerrainOptions,
        heights: &Heightmap,
        normals: &NormalMap,
    ) -> Result<()> {
        let settings = Self::invert_weight(settings);
        // Allocate a command buffer and submit it to the current batch
        let di = bus.data().read().unwrap();
        let ctx = di.get::<SharedContext>().cloned().unwrap();
        let cmd = ctx
            .exec
            .on_domain::<All, _>(Some(ctx.pipelines.clone()), Some(ctx.descriptors.clone()))?;
        let radius = options.texel_radius(position, settings.radius, &heights.image);
        let cmd =
            self.record_update_commands(bus, cmd, uv, radius, &settings, &heights, &normals)?;
        GpuWork::with_batch(bus, move |batch| batch.submit(cmd))??;
        Ok(())
    }

    fn apply_at_uv(
        &self,
        bus: &EventBus<DI>,
        position: Vec3,
        uv: Vec2,
        settings: BrushSettings,
    ) -> Result<()> {
        // Grab the terrain info from the world
        let (terrain, terrain_options) = get_terrain_info(bus);
        // If no terrain handle was set, we cannot reasonably use a brush on it
        let Some(terrain) = terrain else { bail!("Used brush but terrain handle is not set.") };
        with_ready_terrain(bus, terrain, |heights, normals, _, _| {
            self.apply_to_terrain(bus, position, uv, settings, terrain_options, heights, normals)
        })?;
        Ok(())
    }
}

impl Brush for SmoothHeight {
    fn decal_shader(&self) -> &'static str {
        "shaders/src/height_brush_decal.fs.hlsl"
    }

    fn decal_data(&self) -> Option<[f32; 4]> {
        Some(match self.weight_fn {
            WeightFunction::Gaussian(sigma) => [sigma, 0.0, 0.0, 0.0],
        })
    }

    fn apply(&self, bus: &EventBus<DI>, position: Vec3, settings: &BrushSettings) -> Result<()> {
        if !position_on_terrain(position) {
            return Ok(());
        }

        let di = bus.data().read().unwrap();
        let world = di.read_sync::<World>().unwrap();

        // We will apply our brush mainly to the heightmap texture for now. To know how
        // to do this, we need to find the UV coordinates of the heightmap texture
        // at the position we clicked at.
        let uv = world.terrain_options.uv_at(position);
        self.apply_at_uv(bus, position, uv, *settings)?;
        Ok(())
    }
}
