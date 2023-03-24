use anyhow::Result;
use ph::vk;
use phobos::prelude as ph;
use phobos::prelude::traits::*;

use crate::app::RootActorSystem;
use crate::gfx;

#[derive(Debug)]
pub struct TerrainRenderer {}

impl TerrainRenderer {
    pub fn new(ctx: gfx::SharedContext, actors: &RootActorSystem) -> Result<Self> {
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
