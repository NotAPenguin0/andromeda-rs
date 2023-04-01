use std::sync::{Arc, Mutex};

use anyhow::Result;
use ph::vk;
use phobos::{prelude as ph, Allocator, DefaultAllocator, FrameManager, Surface, WindowInterface};
pub use util::paired_image_view::PairedImageView;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::app::renderer::AppRenderer;
use crate::app::window::AppWindow;
use crate::hot_reload::{ShaderReload, SyncShaderReload};

pub mod renderer;
pub mod resource;
pub mod util;

/// All shared graphics objects, these are safely refcounted using `Arc` and `Arc<Mutex>` where necessary, so cloning this struct is acceptable.
#[derive(Debug, Clone)]
pub struct SharedContext<A: Allocator = DefaultAllocator> {
    pub allocator: A,
    pub exec: ph::ExecutionManager,
    pub pipelines: Arc<Mutex<ph::PipelineCache>>,
    pub descriptors: Arc<Mutex<ph::DescriptorCache>>,
    pub debug_messenger: Option<Arc<ph::DebugMessenger>>,
    pub instance: Arc<ph::VkInstance>,
    pub device: Arc<ph::Device>,
    pub shader_reload: SyncShaderReload,
}

fn fill_app_settings<W: WindowInterface>(window: &W) -> ph::AppSettings<W> {
    let features = vk::PhysicalDeviceFeatures {
        fill_mode_non_solid: vk::TRUE,
        tessellation_shader: vk::TRUE,
        sampler_anisotropy: vk::TRUE,
        ..Default::default()
    };

    // Allows wireframe polygon mode

    ph::AppBuilder::new()
        .version((0, 0, 1))
        .name("Andromeda")
        .validation(cfg!(debug_assertions))
        .window(window)
        .present_mode(vk::PresentModeKHR::MAILBOX)
        .scratch_size(8 * 1024 * 1024u64)
        .gpu(ph::GPURequirements {
            dedicated: false,
            min_video_memory: 1024 * 1024 * 1024,
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
            features,
            ..Default::default()
        })
        .build()
}

pub fn init_graphics(
    window: Window,
    event_loop: &EventLoop<()>,
) -> Result<(SharedContext, AppWindow, AppRenderer)> {
    let settings = fill_app_settings(&window);
    let instance = ph::VkInstance::new(&settings)?;
    #[cfg(debug_assertions)]
    let debug_messenger = Some(Arc::new(ph::DebugMessenger::new(&instance)?));
    #[cfg(not(debug_assertions))]
    let debug_messenger = None;
    let (surface, physical_device) = {
        let mut surface = Surface::new(&instance, &settings)?;
        let physical_device = ph::PhysicalDevice::select(&instance, Some(&surface), &settings)?;
        surface.query_details(&physical_device)?;
        (surface, physical_device)
    };

    let device = ph::Device::new(&instance, &physical_device, &settings)?;
    let allocator = DefaultAllocator::new(&instance, &device, &physical_device)?;
    let exec = ph::ExecutionManager::new(device.clone(), &physical_device)?;
    let frame = {
        let swapchain = ph::Swapchain::new(&instance, device.clone(), &settings, &surface)?;
        FrameManager::new(device.clone(), allocator.clone(), &settings, swapchain)?
    };

    let pipelines = ph::PipelineCache::new(device.clone())?;
    let descriptors = ph::DescriptorCache::new(device.clone())?;
    let shader_reload = ShaderReload::new(pipelines.clone(), "shaders/", true)?;

    let gfx = SharedContext {
        allocator,
        exec,
        pipelines,
        descriptors,
        debug_messenger,
        instance: Arc::new(instance),
        device,
        shader_reload,
    };

    let renderer = AppRenderer::new(gfx.clone(), &window, event_loop)?;
    let window = AppWindow::new(frame, window, surface, gfx.clone());

    Ok((gfx, window, renderer))
}
