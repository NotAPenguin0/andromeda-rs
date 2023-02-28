mod paired_image_view;
pub mod alloc_wrapper;

use std::sync::{Arc, Mutex};
use anyhow::Result;

use phobos as ph;
use phobos::{GraphicsCmdBuffer, vk};
use tiny_tokio_actor::ActorRef;
use winit::window::Window;

pub use paired_image_view::PairedImageView;
pub use alloc_wrapper::ThreadSafeAllocator;

use crate::event::Event;
use crate::gui;
use crate::hot_reload::{IntoDynamic, ShaderReloadActor};

/// The entire graphics context.
#[derive(Debug)]
pub struct Context<'f> {
    pub debug_messenger: Option<ph::DebugMessenger>,
    pub frame: ph::FrameManager<'f>,
    pub surface: ph::Surface,
    pub shared: SharedContext,
    pub instance: ph::VkInstance,
}

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
    deferred_target_delete: ph::DeletionQueue<PairedImageView>,
}

impl<'f> Context<'f> {
    pub fn new(window: &Window) -> Result<Self> {
        let settings = ph::AppBuilder::new()
            .version((0, 0, 1))
            .name("Andromeda".to_owned())
            .validation(cfg!(debug_assertions))
            .window(window)
            .present_mode(vk::PresentModeKHR::MAILBOX)
            .scratch_size(1 * 1024)
            .gpu(ph::GPURequirements {
                dedicated: true,
                min_video_memory: 1 * 1024 * 1024 * 1024,
                min_dedicated_video_memory: 0,
                queues: vec![
                    ph::QueueRequest { dedicated: false, queue_type: ph::QueueType::Graphics },
                    ph::QueueRequest { dedicated: true, queue_type: ph::QueueType::Transfer },
                    ph::QueueRequest { dedicated: true, queue_type: ph::QueueType::Compute }
                ],
                ..Default::default()
            })
            .build();

        let instance = ph::VkInstance::new(&settings)?;
        #[cfg(debug_assertions)]
        let debug_messenger = Some(ph::DebugMessenger::new(&instance)?);
        #[cfg(not(debug_assertions))]
        let debug_messenger = None;
        let (surface, physical_device) = {
            let mut surface = ph::Surface::new(&instance, &settings)?;
            let physical_device = ph::PhysicalDevice::select(&instance, Some(&surface), &settings)?;
            surface.query_details(&physical_device)?;
            (surface, physical_device)
        };

        let device = ph::Device::new(&instance, &physical_device, &settings)?;
        let alloc = ph::create_allocator(&instance, device.clone(), &physical_device)?;
        let exec = ph::ExecutionManager::new(device.clone(), &physical_device)?;
        let frame  = {
            let swapchain = ph::Swapchain::new(&instance, device.clone(), &settings, &surface)?;
            ph::FrameManager::new(device.clone(), alloc.clone(), &settings, swapchain)?
        };

        let pipelines = ph::PipelineCache::new(device.clone())?;
        let descriptors = ph::DescriptorCache::new(device.clone())?;
        
        Ok(Self {
            debug_messenger,
            frame,
            surface,
            shared: SharedContext {
                allocator: ThreadSafeAllocator::new(alloc),
                exec,
                pipelines,
                descriptors,
                device,
            },
            instance,
        })
    }

    pub fn next_frame(&mut self) {
        self.shared.pipelines.lock().unwrap().next_frame(); // TODO: figure out how to properly implement '?' for this
        self.shared.descriptors.lock().unwrap().next_frame();
    }
}

// TODO: resize event bus?
// Note that the world should not be rendered at the window resolution though

// TODO: Phobos: Allocator trait accepted by functions instead of raw Arc<Mutex<Allocator>>

// TODO: move world renderer to different module and re-export it
impl WorldRenderer {
    pub fn new(hot_reload: ActorRef<Event, ShaderReloadActor>, ctx: SharedContext) -> Result<Self> {
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

    fn allocate_color_target(width: u32, height: u32, ctx: SharedContext) -> Result<PairedImageView> {
        PairedImageView::new(
            ph::Image::new(ctx.device, (*ctx.allocator).clone(), width, height, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::R8G8B8A8_SRGB)?,
            vk::ImageAspectFlags::COLOR
        )
    }

    pub fn output_image(&self) -> &PairedImageView {
        &self.output
    }

    pub fn resize_target(&mut self, size: gui::USize, ui: &mut gui::UIIntegration) -> Result<gui::Image> {
        info!("Resizing render targets. New size: {:?}", size);
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