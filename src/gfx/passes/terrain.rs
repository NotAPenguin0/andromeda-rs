use anyhow::Result;
use glam::Mat4;
use ph::vk;
use phobos::prelude as ph;
use phobos::prelude::traits::*;

use crate::app::RootActorSystem;
use crate::gfx;
use crate::gfx::world_renderer::RenderOptions;
use crate::hot_reload::IntoDynamic;

#[derive(Debug)]
pub struct TerrainRenderer {}

impl TerrainRenderer {
    pub fn new(ctx: gfx::SharedContext, actors: &RootActorSystem) -> Result<Self> {
        ph::PipelineBuilder::new("terrain")
            .samples(vk::SampleCountFlags::TYPE_8)
            .depth(true, true, false, vk::CompareOp::LESS)
            .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT])
            .vertex_input(0, vk::VertexInputRate::VERTEX)
            .vertex_attribute(0, 0, vk::Format::R32G32_SFLOAT)?
            .polygon_mode(vk::PolygonMode::LINE)
            .blend_attachment_none()
            .tessellation(4, vk::PipelineTessellationStateCreateFlags::empty())
            .into_dynamic()
            .attach_shader("shaders/src/terrain.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/terrain.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .attach_shader("shaders/src/terrain.hull.hlsl", vk::ShaderStageFlags::TESSELLATION_CONTROL)
            .attach_shader("shaders/src/terrain.dom.hlsl", vk::ShaderStageFlags::TESSELLATION_EVALUATION)
            .build(actors.shader_reload.clone(), ctx.pipelines)?;
        Ok(Self {})
    }

    pub async fn render<'s: 'e + 'q, 'state: 'e + 'q, 'options: 'e + 'q, 'e, 'q>(
        &'s mut self,
        options: &'options RenderOptions,
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
                if let Some(terrain) = state.terrain_mesh.clone() {
                    let mut cam_ubo = ifc.allocate_scratch_ubo(std::mem::size_of::<Mat4>() as vk::DeviceSize)?;
                    cam_ubo
                        .mapped_slice()?
                        .copy_from_slice(std::slice::from_ref(&state.projection_view));
                    let tess_factor: u32 = options.tessellation_level;
                    cmd.bind_graphics_pipeline("terrain")?
                        .full_viewport_scissor()
                        // .set_polygon_mode(vk::PolygonMode::LINE)?
                        .push_constants(
                            vk::ShaderStageFlags::TESSELLATION_CONTROL,
                            0,
                            std::slice::from_ref(&tess_factor),
                        )
                        .bind_uniform_buffer(0, 0, &cam_ubo)?
                        .bind_vertex_buffer(0, &terrain.vertices_view)
                        .draw(terrain.vertex_count, 1, 0, 0)
                } else {
                    Ok(cmd)
                }
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
