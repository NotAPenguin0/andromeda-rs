use anyhow::{bail, Result};
use assets::{Heightmap, NormalMap, TerrainOptions};
use gfx::SharedContext;
use glam::{Vec2, Vec3};
use inject::DI;
use pass::GpuWork;
use phobos::domain::All;
use phobos::{
    vk, CommandBuffer, ComputeCmdBuffer, IncompleteCmdBuffer, IncompleteCommandBuffer,
    PipelineStage,
};
use scheduler::EventBus;
use world::World;

use crate::util::{
    dispatch_patch_rect, get_terrain_info, position_on_terrain, prepare_for_read,
    prepare_for_write, update_normals_around_patch, with_ready_terrain,
};
use crate::{Brush, BrushSettings};

#[derive(Copy, Clone, Debug, Default)]
pub struct Equalize {}

impl Equalize {
    fn record_height_update<'q>(
        &self,
        cmd: IncompleteCommandBuffer<'q, All>,
        uv: Vec2,
        radius: u32,
        heights: &Heightmap,
    ) -> Result<IncompleteCommandBuffer<'q, All>> {
        // We are going to write to this image in a compute shader, so submit a barrier for this first.
        let cmd =
            prepare_for_write(&heights.image, cmd, PipelineStage::TESSELLATION_EVALUATION_SHADER);
        // Bind the pipeline we will use to update the heightmap
        let cmd = cmd.bind_compute_pipeline("blur_brush")?;
        // Bind the image to the descriptor, push our uvs to the shader and dispatch our compute shader
        let mut cmd = cmd
            .bind_storage_image(0, 0, &heights.image.image.view)?
            .push_constant(vk::ShaderStageFlags::COMPUTE, 0, &uv)
            .push_constant(vk::ShaderStageFlags::COMPUTE, 8, &radius);
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
        let cmd = prepare_for_write(&normals.image, cmd, PipelineStage::FRAGMENT_SHADER);
        let cmd = update_normals_around_patch(bus, cmd, uv, radius, heights, normals)?;
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
        heights: &Heightmap,
        normals: &NormalMap,
    ) -> Result<CommandBuffer<All>> {
        let cmd = self.record_height_update(cmd, uv, radius, heights)?;
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
        // Allocate a command buffer and submit it to the current batch
        let di = bus.data().read().unwrap();
        let ctx = di.get::<SharedContext>().cloned().unwrap();
        let cmd = ctx
            .exec
            .on_domain::<All, _>(Some(ctx.pipelines.clone()), Some(ctx.descriptors.clone()))?;
        let radius = options.texel_radius(position, settings.radius, &heights.image);
        let cmd = self.record_update_commands(bus, cmd, uv, radius, &heights, &normals)?;
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

impl Brush for Equalize {
    fn apply(&self, bus: &EventBus<DI>, position: Vec3, settings: &BrushSettings) -> Result<()> {
        if !position_on_terrain(position) {
            return Ok(());
        }

        let di = bus.data().read().unwrap();
        let uv = {
            let world = di.read_sync::<World>().unwrap();
            // We will apply our brush mainly to the heightmap texture for now. To know how
            // to do this, we need to find the UV coordinates of the heightmap texture
            // at the position we clicked at.
            world.terrain_options.uv_at(position)
        };

        self.apply_at_uv(bus, position, uv, *settings)?;

        Ok(())
    }
}
