use anyhow::Result;
use glam::Mat4;
use ph::vk;
use phobos::prelude as ph;
use phobos::prelude::traits::*;

use crate::gfx;
use crate::gfx::world::World;
use crate::gfx::world_renderer::RenderOptions;
use crate::hot_reload::{IntoDynamic, SyncShaderReload};

#[derive(Debug)]
pub struct TerrainRenderer {
    heightmap_sampler: ph::Sampler,
}

impl TerrainRenderer {
    pub fn new(ctx: gfx::SharedContext, shader_reload: &SyncShaderReload) -> Result<Self> {
        ph::PipelineBuilder::new("terrain")
            .samples(vk::SampleCountFlags::TYPE_8)
            .depth(true, true, false, vk::CompareOp::LESS)
            .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT])
            .vertex_input(0, vk::VertexInputRate::VERTEX)
            .vertex_attribute(0, 0, vk::Format::R32G32_SFLOAT)?
            .vertex_attribute(0, 1, vk::Format::R32G32_SFLOAT)?
            .polygon_mode(vk::PolygonMode::LINE)
            .blend_attachment_none()
            .tessellation(4, vk::PipelineTessellationStateCreateFlags::empty())
            .into_dynamic()
            .attach_shader("shaders/src/terrain.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/terrain.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .attach_shader("shaders/src/terrain.hull.hlsl", vk::ShaderStageFlags::TESSELLATION_CONTROL)
            .attach_shader("shaders/src/terrain.dom.hlsl", vk::ShaderStageFlags::TESSELLATION_EVALUATION)
            .build(shader_reload.clone(), ctx.pipelines)?;
        Ok(Self {
            heightmap_sampler: ph::Sampler::new(
                ctx.device,
                vk::SamplerCreateInfo {
                    s_type: vk::StructureType::SAMPLER_CREATE_INFO,
                    p_next: std::ptr::null(),
                    flags: Default::default(),
                    mag_filter: vk::Filter::NEAREST,
                    min_filter: vk::Filter::NEAREST,
                    mipmap_mode: vk::SamplerMipmapMode::NEAREST,
                    address_mode_u: vk::SamplerAddressMode::REPEAT,
                    address_mode_v: vk::SamplerAddressMode::REPEAT,
                    address_mode_w: vk::SamplerAddressMode::REPEAT,
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
            )?,
        })
    }

    pub async fn render<'s: 'e + 'q, 'state: 'e + 'q, 'world: 'e + 'q, 'e, 'q>(
        &'s mut self,
        world: &'world World,
        graph: &mut gfx::FrameGraph<'e, 'q>,
        _bindings: &mut ph::PhysicalResourceBindings,
        color: &ph::VirtualResource,
        depth: &ph::VirtualResource,
        state: &'state gfx::RenderState,
    ) -> Result<()> {
        let pass = ph::PassBuilder::render("terrain")
            .color_attachment(
                color,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                }),
            )?
            .depth_attachment(
                depth,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }),
            )?
            .execute(|cmd, ifc, _bindings| {
                if let Some(terrain) = &world.terrain_mesh {
                    if let Some(heightmap) = &world.height_map {
                        let mut cam_ubo = ifc.allocate_scratch_ubo(std::mem::size_of::<Mat4>() as vk::DeviceSize)?;
                        cam_ubo
                            .mapped_slice()?
                            .copy_from_slice(std::slice::from_ref(&state.projection_view));
                        let tess_factor: u32 = world.options.tessellation_level;
                        cmd.bind_graphics_pipeline("terrain")?
                            .full_viewport_scissor()
                            // .set_polygon_mode(vk::PolygonMode::LINE)?
                            .push_constants(
                                vk::ShaderStageFlags::TESSELLATION_CONTROL,
                                0,
                                std::slice::from_ref(&tess_factor),
                            )
                            .bind_uniform_buffer(0, 0, &cam_ubo)?
                            .bind_sampled_image(0, 1, &heightmap.image.view, &self.heightmap_sampler)?
                            .bind_vertex_buffer(0, &terrain.vertices_view)
                            .draw(terrain.vertex_count, 1, 0, 0)
                    } else {
                        Ok(cmd)
                    }
                } else {
                    Ok(cmd)
                }
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
