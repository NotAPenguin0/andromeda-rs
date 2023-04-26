use anyhow::Result;
use gfx::util::paired_image_view::PairedImageView;
use gfx::util::sampler::create_raw_sampler;
use gfx::SharedContext;
use hot_reload::IntoDynamic;
use inject::DI;
use phobos::domain::Compute;
use phobos::prelude::ComputePipelineBuilder;
use phobos::{vk, ComputeCmdBuffer, Image, IncompleteCmdBuffer, PipelineStage};
use scheduler::EventBus;

use crate::HeightMap;

#[derive(Debug)]
pub struct NormalMap {
    pub image: PairedImageView,
}

impl NormalMap {
    pub(crate) fn init_pipelines(ctx: SharedContext, bus: &mut EventBus<DI>) -> Result<()> {
        ComputePipelineBuilder::new("terrain_normals")
            .persistent()
            .into_dynamic()
            .set_shader("shaders/src/terrain_normals.comp.hlsl")
            .build(bus, ctx.pipelines)
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

    pub fn from_heights(heights: &HeightMap, bus: EventBus<DI>) -> Result<Self> {
        let mut ctx = bus
            .data()
            .read()
            .unwrap()
            .get::<SharedContext>()
            .cloned()
            .unwrap();
        let image = Self::allocate_image(&mut ctx, heights)?;
        let sampler = create_raw_sampler(&ctx)?;

        let cmd = ctx
            .exec
            .on_domain::<Compute, _>(Some(ctx.pipelines.clone()), Some(ctx.descriptors.clone()))?;

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
