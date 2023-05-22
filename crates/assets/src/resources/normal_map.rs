use anyhow::{anyhow, Result};
use error::publish_success;
use gfx::util::paired_image_view::PairedImageView;
use gfx::util::sampler::create_raw_sampler;
use gfx::SharedContext;
use hot_reload::IntoDynamic;
use inject::DI;
use log::info;
use phobos::domain::Compute;
use phobos::prelude::ComputePipelineBuilder;
use phobos::{vk, ComputeCmdBuffer, Image, IncompleteCmdBuffer, PipelineStage};
use scheduler::EventBus;

use crate::asset::Asset;
use crate::handle::Handle;
use crate::storage::AssetStorage;
use crate::texture::format::{Rgba, TextureFormat};
use crate::texture::{Texture, TextureLoadInfo};
use crate::{Heightmap, HeightmapFormat};

pub type NormalMapFormat = Rgba<u8>;

#[derive(Debug)]
pub struct NormalMap {
    pub image: Texture<NormalMapFormat>,
}

pub enum NormalMapLoadInfo {
    FromHeightmap {
        heights: Handle<Heightmap>,
    },
}

impl Asset for NormalMap {
    type LoadInfo = NormalMapLoadInfo;

    fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> Result<Self>
    where
        Self: Sized, {
        match info {
            NormalMapLoadInfo::FromHeightmap {
                heights,
            } => load_from_heights(heights, bus),
        }
    }
}

impl NormalMap {
    pub(crate) fn init_pipelines(ctx: SharedContext, bus: &mut EventBus<DI>) -> Result<()> {
        ComputePipelineBuilder::new("terrain_normals")
            .persistent()
            .into_dynamic()
            .set_shader("shaders/src/terrain_normals.cs.hlsl")
            .build(bus, ctx.pipelines)
    }
}

fn allocate_image(
    ctx: &mut SharedContext,
    heights: &Texture<HeightmapFormat>,
) -> Result<PairedImageView> {
    let image = Image::new(
        ctx.device.clone(),
        &mut ctx.allocator,
        heights.image.width(),
        heights.image.height(),
        vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
        NormalMapFormat::VK_FORMAT,
        vk::SampleCountFlags::TYPE_1,
    )?;
    PairedImageView::new(image, vk::ImageAspectFlags::COLOR)
}

fn load_from_heights(heights: Handle<Heightmap>, bus: EventBus<DI>) -> Result<NormalMap> {
    let di = bus.data().read().unwrap();
    let assets = di.get::<AssetStorage>().unwrap();
    assets
        .with_when_ready(heights, |heights| {
            let di = bus.data().read().unwrap();
            let mut ctx = di.get::<SharedContext>().cloned().unwrap();
            let image = allocate_image(&mut ctx, &heights.image)?;
            let sampler = create_raw_sampler(&ctx)?;
            let cmd = ctx.exec.on_domain::<Compute, _>(
                Some(ctx.pipelines.clone()),
                Some(ctx.descriptors.clone()),
            )?;
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
                .bind_sampled_image(0, 1, &heights.image.image.view, &sampler)?
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
            let image = Texture::load(
                TextureLoadInfo::FromRawGpu {
                    image,
                },
                bus.clone(),
            )?;
            info!("Generated normal map");
            publish_success!(bus, "Successfully generated normal map.");
            Ok(NormalMap {
                image,
            })
        })
        .ok_or_else(|| anyhow!("Error generating normal map: invalid heightmap handle."))?
}
