use std::sync::{Arc, Mutex};

use anyhow::Result;
use ph::vk;
use phobos::prelude as ph;
use winit::window::Window;

pub use graph::FrameGraph;
pub use paired_image_view::PairedImageView;
pub use targets::RenderTargets;
pub use targets::SizeGroup;
pub(self) use world_renderer::RenderState;
pub use world_renderer::WorldRenderer;

mod graph;
mod paired_image_view;
mod passes;
mod postprocess;
mod targets;
mod world_renderer;

/// The entire graphics context.
#[derive(Debug)]
pub struct Context {
    pub debug_messenger: Option<ph::DebugMessenger>,
    pub frame: ph::FrameManager,
    pub surface: ph::Surface,
    pub shared: SharedContext,
    pub instance: ph::VkInstance,
}

/// All shared graphics objects, these are safely refcounted using `Arc` and `Arc<Mutex>` where necessary, so cloning this struct is acceptable.
#[derive(Debug, Clone)]
pub struct SharedContext {
    pub allocator: ph::DefaultAllocator,
    pub exec: ph::ExecutionManager,
    pub pipelines: Arc<Mutex<ph::PipelineCache>>,
    pub descriptors: Arc<Mutex<ph::DescriptorCache>>,
    pub device: Arc<ph::Device>,
}

impl Context {
    pub fn new(window: &Window) -> Result<Self> {
        let settings = ph::AppBuilder::new()
            .version((0, 0, 1))
            .name("Andromeda".to_owned())
            .validation(cfg!(debug_assertions))
            .window(window)
            .present_mode(vk::PresentModeKHR::MAILBOX)
            .scratch_size(8 * 1024 * 1024u64)
            .gpu(ph::GPURequirements {
                dedicated: true,
                min_video_memory: 1 * 1024 * 1024 * 1024,
                min_dedicated_video_memory: 0,
                queues: vec![
                    ph::QueueRequest {
                        dedicated: false,
                        queue_type: ph::QueueType::Graphics,
                    },
                    ph::QueueRequest {
                        dedicated: true,
                        queue_type: ph::QueueType::Transfer,
                    },
                    ph::QueueRequest {
                        dedicated: true,
                        queue_type: ph::QueueType::Compute,
                    },
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
        let alloc = ph::DefaultAllocator::new(&instance, &device, &physical_device)?;
        let exec = ph::ExecutionManager::new(device.clone(), &physical_device)?;
        let frame = {
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
                allocator: alloc,
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
