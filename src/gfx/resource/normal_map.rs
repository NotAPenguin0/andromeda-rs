use anyhow::Result;
use phobos::domain::Compute;
use phobos::{
    vk, ComputeCmdBuffer, ComputePipelineBuilder, Image, IncompleteCmdBuffer, PipelineStage,
};

use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::util::sampler::create_raw_sampler;
use crate::gfx::{PairedImageView, SharedContext};
use crate::hot_reload::IntoDynamic;

#[derive(Debug)]
pub struct NormalMap {
    pub image: PairedImageView,
}

impl NormalMap {
    pub fn init_pipelines(ctx: SharedContext) -> Result<()> {
        ComputePipelineBuilder::new("terrain_normals")
            .persistent()
            .into_dynamic()
            .set_shader("shaders/src/terrain_normals.comp.hlsl")
            .build(ctx.shader_reload, ctx.pipelines)
    }

    fn allocate_image(ctx: &mut SharedContext, heights: &HeightMap) -> Result<PairedImageView> {
        let image = Image::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            heights.image.width(),
            heights.image.height(),
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R8G8B8A8_UNORM,
            vk::SampleCountFlags::TYPE_1,
        )?;
        PairedImageView::new(image, vk::ImageAspectFlags::COLOR)
    }

    pub fn from_heights(mut ctx: SharedContext, heights: &HeightMap) -> Result<Self> {
        let image = Self::allocate_image(&mut ctx, heights)?;
        let sampler = create_raw_sampler(&ctx)?;

        let cmd = ctx
            .exec
            .on_domain::<Compute>(Some(ctx.pipelines.clone()), Some(ctx.descriptors.clone()))?;

        let dispatches_x = (image.width() as f32 / 32.0).ceil() as u32;
        let dispatches_y = (image.height() as f32 / 32.0).ceil() as u32;
        let cmd = cmd
            .transition_image(
                &image.view,
                PipelineStage::TOP_OF_PIPE,
                PipelineStage::COMPUTE_SHADER,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::SHADER_STORAGE_WRITE,
            )
            .bind_compute_pipeline("terrain_normals")?
            .bind_storage_image(0, 0, &image.view)?
            .bind_sampled_image(0, 1, &heights.image.view, &sampler)?
            .dispatch(dispatches_x, dispatches_y, 1)?
            .transition_image(
                &image.view,
                PipelineStage::COMPUTE_SHADER,
                PipelineStage::BOTTOM_OF_PIPE,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::AccessFlags2::SHADER_STORAGE_WRITE,
                vk::AccessFlags2::NONE,
            );

        ctx.exec.submit(cmd.finish()?)?.wait()?;

        Ok(Self {
            image,
        })
    }
}
