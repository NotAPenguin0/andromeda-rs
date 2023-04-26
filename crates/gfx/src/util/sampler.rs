use anyhow::Result;
use phobos::{vk, Sampler};

use crate::SharedContext;

/// Create a sampler with no interpolation or anisotropic filtering.
pub fn create_raw_sampler(ctx: &SharedContext) -> Result<Sampler> {
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

/// Create a sampler with linear interpolation and anisotropic filtering enabled
pub fn create_linear_sampler(ctx: &SharedContext) -> Result<Sampler> {
    Sampler::new(
        ctx.device.clone(),
        vk::SamplerCreateInfo {
            s_type: vk::StructureType::SAMPLER_CREATE_INFO,
            p_next: std::ptr::null(),
            flags: Default::default(),
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            mipmap_mode: vk::SamplerMipmapMode::NEAREST,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            mip_lod_bias: 0.0,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: 8.0,
            compare_enable: vk::FALSE,
            compare_op: Default::default(),
            min_lod: vk::LOD_CLAMP_NONE,
            max_lod: vk::LOD_CLAMP_NONE,
            border_color: Default::default(),
            unnormalized_coordinates: vk::FALSE,
        },
    )
}
