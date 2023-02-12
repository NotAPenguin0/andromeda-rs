mod paired_image_view;
pub mod alloc_wrapper;

use std::sync::{Arc, Mutex};
use anyhow::Result;

use phobos as ph;
use phobos::vk;

pub use paired_image_view::PairedImageView;
pub use alloc_wrapper::ThreadSafeAllocator;

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
    pub fn new(ctx: SharedContext) -> Result<Self> {
        Ok(Self {
            output: PairedImageView::new(
                ph::Image::new(ctx.device.clone(), (*ctx.allocator).clone(), 1920, 1080, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::R8G8B8A8_SRGB)?,
                vk::ImageAspectFlags::COLOR
            )?,
            ctx
        })
    }

    /// Conventions for output graph:
    /// - Contains a pass `final_output` which renders to a virtual resource named `output`.
    /// - This resource is bound to the internal output color attachment.
    pub fn redraw_world<'e, 'q>(&mut self) -> Result<(ph::PassGraph<'e, 'q, ph::domain::All>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();

        let final_output = ph::VirtualResource::image("output");
        let output_pass = ph::PassBuilder::render("final_output")
            .color_attachment(
                final_output.clone(),
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue { float32: [1.0, 0.0, 0.0, 1.0]}))?
            .build();

        bindings.bind_image("output", self.output.view.clone());
        let graph = ph::PassGraph::new()
            .add_pass(output_pass)?;

        Ok((graph, bindings))
    }
}