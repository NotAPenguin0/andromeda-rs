use anyhow::Result;
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
            .cull_mask(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT, vk::DynamicState::POLYGON_MODE_EXT])
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
            .execute(|cmd, _ifc, _bindings| Ok(cmd))
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
