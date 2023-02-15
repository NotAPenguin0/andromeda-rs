mod paired_image_view;
pub mod alloc_wrapper;

use std::sync::{Arc, Mutex};
use anyhow::Result;
use futures::executor::block_on;

use phobos as ph;
use phobos::{GraphicsCmdBuffer, vk};
use tiny_tokio_actor::ActorRef;

pub use paired_image_view::PairedImageView;
pub use alloc_wrapper::ThreadSafeAllocator;
use crate::event::Event;
use crate::hot_reload::{AddShader, ShaderReloadActor};

/// All shared graphics objects, these are safely refcounted using Arc and Arc<Mutex> where necessary, so cloning this struct is acceptable.
#[derive(Debug, Clone)]
pub struct SharedContext {
    pub allocator: ThreadSafeAllocator,
    pub exec: Arc<ph::ExecutionManager>,
    pub pipelines: Arc<Mutex<ph::PipelineCache>>,
    pub descriptors: Arc<Mutex<ph::DescriptorCache>>,
    pub device: Arc<ph::Device>
}

#[derive(Debug)]
pub struct WorldRenderer {
    ctx: SharedContext,
    output: PairedImageView,
}

// TODO: resize event bus?
// Note that the world should not be rendered at the window resolution though

// TODO: Phobos: Allocator trait accepted by functions instead of raw Arc<Mutex<Allocator>>

// TODO: move world renderer to different module and re-export it
impl WorldRenderer {
    pub fn new(hot_reload: ActorRef<Event, ShaderReloadActor>, ctx: SharedContext) -> Result<Self> {
        // Note how we don't add any shaders to this!
        let pci = ph::PipelineBuilder::new("solid_color")
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .blend_attachment_none()
            .cull_mask(vk::CullModeFlags::NONE)
            .build();
        ctx.pipelines.lock().unwrap().create_named_pipeline(pci)?;
        block_on(async {
            hot_reload.ask(AddShader{
                path: "shaders/src/fullscreen.vert.glsl".into(),
                stage: vk::ShaderStageFlags::VERTEX,
                pipeline: "solid_color".to_string(),
            }).await?;
            hot_reload.ask(AddShader {
                path: "shaders/src/solid_color.frag.glsl".into(),
                stage: vk::ShaderStageFlags::FRAGMENT,
                pipeline: "solid_color".to_string(),
            }).await
        })?;

        Ok(Self {
            output: PairedImageView::new(
                ph::Image::new(ctx.device.clone(), (*ctx.allocator).clone(), 1920, 1080, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::R8G8B8A8_SRGB)?,
                vk::ImageAspectFlags::COLOR
            )?,
            ctx
        })
    }

    pub fn output_image(&self) -> &PairedImageView {
        &self.output
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
                        width: 1920.0,
                        height: 1080.0,
                        min_depth: 0.0,
                        max_depth: 0.0,
                    })
                    .scissor(vk::Rect2D { offset: Default::default(), extent: vk::Extent2D { width: 1920, height: 1080 } })
                    .draw(6, 1, 0, 0))
            })
            .build();

        bindings.bind_image("output", self.output.view.clone());
        let graph = ph::PassGraph::new()
            .add_pass(output_pass)?;

        Ok((graph, bindings))
    }
}