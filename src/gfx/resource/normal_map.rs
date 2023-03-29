use std::sync::Arc;

use anyhow::Result;
use phobos::domain::Compute;
use phobos::{
    vk, ComputeCmdBuffer, ComputePipelineBuilder, Image, IncompleteCmdBuffer, PipelineStage,
    Sampler,
};

use crate::gfx;
use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::{PairedImageView, SharedContext};
use crate::hot_reload::{IntoDynamic, SyncShaderReload};

#[derive(Debug)]
pub struct NormalMap {
    pub image: PairedImageView,
}

impl NormalMap {
    pub fn init_pipelines(ctx: SharedContext, hot_reload: SyncShaderReload) -> Result<()> {
        ComputePipelineBuilder::new("terrain_normals")
            .persistent()
            .into_dynamic()
            .set_shader("shaders/src/terrain_normals.comp.hlsl")
            .build(hot_reload, ctx.pipelines)
    }

    fn create_sampler(ctx: &SharedContext) -> Result<Sampler> {
        Sampler::new(
            ctx.device.clone(),
            vk::SamplerCreateInfo {
                s_type: vk::StructureType::SAMPLER_CREATE_INFO,
                p_next: std::ptr::null(),
                flags: Default::default(),
                mag_filter: vk::Filter::NEAREST,
                min_filter: vk::Filter::NEAREST,
                mipmap_mode: vk::SamplerMipmapMode::NEAREST,
                address_mode_u: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_v: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
                mip_lod_bias: 0.0,
                anisotropy_enable: vk::FALSE,
                max_anisotropy: 0.0,
                compare_enable: vk::FALSE,
                compare_op: Default::default(),
                min_lod: vk::LOD_CLAMP_NONE,
                max_lod: vk::LOD_CLAMP_NONE,
                border_color: Default::default(),
                unnormalized_coordinates: vk::FALSE,
            },
        )
    }

    fn allocate_image(ctx: &mut SharedContext, heights: &HeightMap) -> Result<PairedImageView> {
        let image = Image::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            heights.image.image.width(),
            heights.image.image.height(),
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R8G8B8A8_UNORM,
            vk::SampleCountFlags::TYPE_1,
        )?;
        PairedImageView::new(image, vk::ImageAspectFlags::COLOR)
    }

    pub fn generate_from_heights(mut ctx: SharedContext, heights: &HeightMap) -> Result<Self> {
        let image = Self::allocate_image(&mut ctx, heights)?;
        let sampler = Self::create_sampler(&ctx)?;

        let cmd = ctx
            .exec
            .on_domain::<Compute>(Some(ctx.pipelines.clone()), Some(ctx.descriptors.clone()))?;

        let dispatches_x = (image.image.width() as f32 / 32.0).ceil() as u32;
        let dispatches_y = (image.image.height() as f32 / 32.0).ceil() as u32;
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

        ctx.exec.submit(cmd.finish()?)?.wait_and_yield()?;

        Ok(Self {
            image,
        })
    }
}
