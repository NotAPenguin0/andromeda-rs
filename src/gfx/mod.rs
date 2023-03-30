use std::sync::{Arc, Mutex};

use anyhow::Result;
pub use graph::FrameGraph;
pub use passes::AtmosphereInfo;
use ph::vk;
use phobos::domain::{All, ExecutionDomain};
use phobos::{
    prelude as ph, Allocator, CommandBuffer, DefaultAllocator, DeletionQueue, FrameManager,
    InFlightContext, IncompleteCmdBuffer, RecordGraphToCommandBuffer, Surface, WindowInterface,
};
use poll_promise::Promise;
pub use targets::{RenderTargets, SizeGroup};
pub use util::paired_image_view::PairedImageView;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowId};
pub(self) use world_renderer::RenderState;
pub use world_renderer::WorldRenderer;

use crate::gfx::world::World;
use crate::gui;
use crate::gui::image_provider::RenderTargetImageProvider;
use crate::gui::util::integration::UIIntegration;
use crate::hot_reload::{ShaderReload, SyncShaderReload};

mod graph;
mod passes;
mod postprocess;
pub mod resource;
mod targets;
pub mod util;
pub mod world;
mod world_renderer;

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
    let mut features = vk::PhysicalDeviceFeatures::default();
    // Allows wireframe polygon mode
    features.fill_mode_non_solid = vk::TRUE;
    features.tessellation_shader = vk::TRUE;

    ph::AppBuilder::new()
        .version((0, 0, 1))
        .name("Andromeda")
        .validation(cfg!(debug_assertions))
        .window(window)
        .present_mode(vk::PresentModeKHR::MAILBOX)
        .scratch_size(8 * 1024 * 1024u64)
        .gpu(ph::GPURequirements {
            dedicated: false,
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

    let renderer = AppRenderer::new(gfx.clone(), &window, &event_loop)?;

    Ok((
        gfx.clone(),
        AppWindow {
            frame,
            window,
            surface,
            gfx,
        },
        renderer,
    ))
}

#[derive(Debug)]
pub struct AppWindow<A: Allocator = DefaultAllocator> {
    pub frame: FrameManager<A>,
    pub window: Window,
    pub surface: Surface,
    pub gfx: SharedContext,
}

impl<A: Allocator> AppWindow<A> {
    pub async fn new_frame<
        D: ExecutionDomain + 'static,
        F: FnOnce(&Window, InFlightContext<A>) -> Result<CommandBuffer<D>>,
    >(
        &mut self,
        func: F,
    ) -> Result<()> {
        self.frame
            .new_frame(self.gfx.exec.clone(), &self.window, &self.surface, |ifc| {
                func(&self.window, ifc)
            })
            .await
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

#[derive(Debug)]
pub struct AppRenderer {
    gfx: SharedContext,
    renderer: WorldRenderer,
    ui: UIIntegration,
}

impl AppRenderer {
    pub fn new(gfx: SharedContext, window: &Window, event_loop: &EventLoop<()>) -> Result<Self> {
        Ok(Self {
            renderer: WorldRenderer::new(gfx.clone())?,
            ui: UIIntegration::new(&event_loop, &window, gfx.clone())?,
            gfx,
        })
    }

    pub fn ui(&self) -> egui::Context {
        self.ui.context()
    }

    pub fn gfx(&self) -> SharedContext {
        self.gfx.clone()
    }

    pub fn process_event(&mut self, event: &WindowEvent) {
        self.ui.process_event(event);
    }

    pub fn image_provider(&mut self) -> RenderTargetImageProvider {
        RenderTargetImageProvider {
            targets: self.renderer.targets(),
            integration: &mut self.ui,
        }
    }

    pub fn new_frame(&mut self, window: &Window) {
        self.ui.new_frame(window);
        self.renderer.new_frame();
        self.gfx.pipelines.lock().unwrap().next_frame();
        self.gfx.descriptors.lock().unwrap().next_frame();
    }

    pub fn render(
        &mut self,
        window: &Window,
        world: &World,
        mut ifc: InFlightContext,
    ) -> Result<CommandBuffer<All>> {
        let (mut graph, mut bindings) = self.renderer.redraw_world(world)?;
        let swapchain = graph.swapchain_resource();
        // Record UI commands
        self.ui.render(window, swapchain.clone(), &mut graph)?;
        // Add a present pass to the graph.
        let present_pass = ph::PassBuilder::present("present", &graph.latest_version(&swapchain)?);
        graph.add_pass(present_pass);
        let mut graph = graph.build()?;

        // Bind the swapchain resource.
        bindings.bind_image("swapchain", ifc.swapchain_image.as_ref().unwrap());
        // Record this frame.
        let cmd = self.gfx.exec.on_domain::<All>(
            Some(self.gfx.pipelines.clone()),
            Some(self.gfx.descriptors.clone()),
        )?;
        let cmd = graph.record(cmd, &bindings, &mut ifc, self.gfx.debug_messenger.clone())?;
        cmd.finish()
    }
}
