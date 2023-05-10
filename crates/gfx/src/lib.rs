use std::sync::Arc;

use anyhow::Result;
use inject::DI;
use phobos::{
    vk, Allocator, AppBuilder, AppSettings, DebugMessenger, DefaultAllocator, DescriptorCache,
    Device, ExecutionManager, FrameManager, GPURequirements, PhysicalDevice, PipelineCache,
    QueueRequest, QueueType, Surface, Swapchain, VkInstance, WindowInterface,
};
use scheduler::EventBus;
pub use util::*;
use winit::window::Window;

pub mod util;

/// All shared graphics objects, these are safely refcounted using `Arc` and `Arc<Mutex>` where necessary, so cloning this struct is acceptable.
#[derive(Debug, Clone)]
pub struct SharedContext<A: Allocator = DefaultAllocator> {
    pub allocator: A,
    pub exec: ExecutionManager,
    pub pipelines: PipelineCache,
    pub descriptors: DescriptorCache,
    pub debug_messenger: Option<Arc<DebugMessenger>>,
    pub instance: Arc<VkInstance>,
    pub device: Device,
}

fn fill_app_settings<W: WindowInterface>(window: &W) -> AppSettings<W> {
    let features = vk::PhysicalDeviceFeatures {
        fill_mode_non_solid: vk::TRUE,
        tessellation_shader: vk::TRUE,
        sampler_anisotropy: vk::TRUE,
        ..Default::default()
    };

    // Allows wireframe polygon mode

    AppBuilder::new()
        .version((0, 0, 1))
        .name("Andromeda")
        .validation(cfg!(debug_assertions))
        .window(window)
        .present_mode(vk::PresentModeKHR::MAILBOX)
        .scratch_size(8 * 1024 * 1024u64)
        .gpu(GPURequirements {
            dedicated: false,
            min_video_memory: 1024 * 1024 * 1024,
            min_dedicated_video_memory: 0,
            queues: vec![
                QueueRequest {
                    dedicated: false,
                    queue_type: QueueType::Graphics,
                },
                QueueRequest {
                    dedicated: true,
                    queue_type: QueueType::Transfer,
                },
                QueueRequest {
                    dedicated: true,
                    queue_type: QueueType::Compute,
                },
            ],
            features,
            ..Default::default()
        })
        .build()
}

/// Injects the graphics context into the DI system, and returns the frame manager and surface
pub fn initialize(
    window: &Window,
    bus: &EventBus<DI>,
) -> Result<(FrameManager, Surface, SharedContext)> {
    let settings = fill_app_settings(window);
    let instance = VkInstance::new(&settings)?;
    #[cfg(debug_assertions)]
    let debug_messenger = Some(Arc::new(DebugMessenger::new(&instance)?));
    #[cfg(not(debug_assertions))]
    let debug_messenger = None;
    let (surface, physical_device) = {
        let mut surface = Surface::new(&instance, &settings)?;
        let physical_device = PhysicalDevice::select(&instance, Some(&surface), &settings)?;
        surface.query_details(&physical_device)?;
        (surface, physical_device)
    };

    let device = Device::new(&instance, &physical_device, &settings)?;
    let allocator = DefaultAllocator::new(&instance, &device, &physical_device)?;
    let exec = ExecutionManager::new(device.clone(), &physical_device)?;
    let frame = {
        let swapchain = Swapchain::new(&instance, device.clone(), &settings, &surface)?;
        FrameManager::new(device.clone(), allocator.clone(), &settings, swapchain)?
    };

    let pipelines = PipelineCache::new(device.clone(), allocator.clone())?;
    let descriptors = DescriptorCache::new(device.clone())?;

    let gfx = SharedContext {
        allocator,
        exec,
        pipelines,
        descriptors,
        debug_messenger,
        instance: Arc::new(instance),
        device,
    };

    bus.data().write().unwrap().put(gfx.clone());

    Ok((frame, surface, gfx))
}
