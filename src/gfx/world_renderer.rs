use phobos as ph;

use anyhow::Result;
use phobos::{GraphicsCmdBuffer, vk};

use tiny_tokio_actor::ActorRef;

use crate::core::Event;
use crate::{gfx, gui};
use crate::hot_reload::{IntoDynamic, ShaderReloadActor};

#[derive(Debug)]
pub struct WorldRenderer {
    ctx: gfx::SharedContext,
    output: gfx::PairedImageView,
    deferred_target_delete: ph::DeletionQueue<gfx::PairedImageView>,
}

impl WorldRenderer {
    pub fn new(hot_reload: ActorRef<Event, ShaderReloadActor>, ctx: gfx::SharedContext) -> Result<Self> {
        ph::PipelineBuilder::new("solid_color")
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .blend_attachment_none()
            .cull_mask(vk::CullModeFlags::NONE)
            .into_dynamic()
            .attach_shader("shaders/src/fullscreen.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/solid_color.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(hot_reload, ctx.pipelines.clone())?;

        Ok(Self {
            output: Self::allocate_color_target(1920, 1080, ctx.clone())?,
            ctx,
            deferred_target_delete: ph::DeletionQueue::new(4),
        })
    }

    fn allocate_color_target(width: u32, height: u32, ctx: gfx::SharedContext) -> Result<gfx::PairedImageView> {
        gfx::PairedImageView::new(
            ph::Image::new(ctx.device, (*ctx.allocator).clone(), width, height, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::R8G8B8A8_SRGB)?,
            vk::ImageAspectFlags::COLOR
        )
    }

    pub fn output_image(&self) -> &gfx::PairedImageView {
        &self.output
    }

    pub fn resize_target(&mut self, size: gui::USize, ui: &mut gui::UIIntegration) -> Result<gui::Image> {
        let mut new_target = Self::allocate_color_target(size.x(), size.y(), self.ctx.clone())?;
        std::mem::swap(&mut new_target, &mut self.output);

        self.deferred_target_delete.push(new_target);
        Ok(ui.register_texture(&self.output.view))
    }

    pub fn new_frame(&mut self) {
        self.deferred_target_delete.next_frame();
    }

    /// Conventions for output graph:
    /// - Contains a pass `final_output` which renders to a virtual resource named `output`.
    /// - This resource is bound to the internal output color attachment.
    pub fn redraw_world<'s: 'e, 'e, 'q>(&'s mut self) -> Result<(ph::PassGraph<'e, 'q, ph::domain::All>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();

        let final_output = ph::VirtualResource::image("output");
        let output_pass = ph::PassBuilder::render("final_output")
            .color_attachment(
                final_output.clone(),
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}))?
            .execute(|cmd, _, _| {
                Ok(cmd.bind_graphics_pipeline("solid_color", self.ctx.pipelines.clone())?
                    .viewport(vk::Viewport{
                        x: 0.0,
                        y: 0.0,
                        width: self.output.view.size.width as f32,
                        height: self.output.view.size.height as f32,
                        min_depth: 0.0,
                        max_depth: 0.0,
                    })
                    .scissor(vk::Rect2D { offset: Default::default(), extent: vk::Extent2D { width: self.output.view.size.width, height: self.output.view.size.height } })
                    .draw(6, 1, 0, 0))
            })
            .build();

        bindings.bind_image("output", self.output.view.clone());
        let graph = ph::PassGraph::new()
            .add_pass(output_pass)?;

        Ok((graph, bindings))
    }
}