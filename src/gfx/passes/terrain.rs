use anyhow::Result;
use glam::Mat4;
use ph::vk;
use phobos::prelude as ph;
use phobos::prelude::traits::*;

use crate::app::RootActorSystem;
use crate::gfx;
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
            .blend_attachment_none()
            .into_dynamic()
            .attach_shader("shaders/src/terrain.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/terrain.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(actors.shader_reload.clone(), ctx.pipelines)?;
        Ok(Self {})
    }

    pub async fn render<'s: 'e + 'q, 'state: 'e + 'q, 'e, 'q>(
        &'s mut self,
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
                    cmd.bind_graphics_pipeline("terrain")?
                        .full_viewport_scissor()
                        .bind_uniform_buffer(0, 0, &cam_ubo)?
                        .bind_vertex_buffer(0, &terrain.vertices_view)
                        .draw(6, 1, 0, 0)
                } else {
                    Ok(cmd)
                }
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
