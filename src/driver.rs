use std::ops::Deref;
use tiny_tokio_actor::*;
use anyhow::Result;

use futures::executor::block_on;

use phobos as ph;
use phobos::{IncompleteCmdBuffer, vk};
use winit::event::{VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

use crate::{event, gfx, gui, repaint};
use crate::repaint::RepaintListener;

/// Main application driver. Hosts the event loop.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Driver<'f> {
    #[derivative(Debug="ignore")]
    system: ActorSystem<event::Event>,
    pub window: Window,
    debug_messenger: Option<ph::DebugMessenger>,
    frame: ph::FrameManager<'f>,
    surface: ph::Surface,
    renderer: gfx::WorldRenderer,
    ui: gui::UIIntegration,
    pub gfx: gfx::SharedContext,
    instance: ph::VkInstance,
    repaint: ActorRef<event::Event, RepaintListener>,
}

impl<'f> Driver<'f> {
    pub fn create_window() -> Result<(EventLoop<()>, Window)> {
        let event_loop = EventLoopBuilder::new().build();
        let window = WindowBuilder::new()
            .with_title("Phobos test app")
            .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }

    // TODO: Cleanup, this maybe should not take ownership of the window
    pub fn init(event_loop: &EventLoop<()>, window: Window) -> Result<Driver<'f>> {
        let settings = ph::AppBuilder::new()
            .version((0, 0, 1))
            .name("Andromeda".to_owned())
            .validation(cfg!(debug_assertions))
            .window(&window)
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

        let bus = EventBus::new(100);
        let system = ActorSystem::new("Main task system", bus);
        let repaint = block_on(system.create_actor("repaint_listener", RepaintListener::default()))?;

        let gfx = gfx::SharedContext {
            device,
            allocator: gfx::ThreadSafeAllocator::new(alloc),
            exec,
            pipelines,
            descriptors,
        };

        let ui = {
            let queue = gfx.exec.get_queue::<ph::domain::Graphics>().unwrap();
            gui::UIIntegration::new(
                event_loop, &window, gfx.device.clone(), gfx.allocator.clone(), queue.deref(), unsafe { frame.get_swapchain() }
            )?
        };

        Ok(Driver {
            system,
            window,
            instance,
            surface,
            debug_messenger,
            gfx: gfx.clone(),
            ui,
            frame,
            repaint,
            renderer: gfx::WorldRenderer::new(gfx)?
        })
    }

    pub async fn update_repaint_status(&mut self) -> Result<repaint::RepaintStatus> {
        let status = self.repaint.ask(repaint::CheckRepaint).await?;
        // Only send a reset message if the repaint status was to repaint
        if status != repaint::RepaintStatus::None {
            self.repaint.tell(repaint::ResetRepaint)?;
        }
        Ok(status)
    }

    pub async fn process_frame(&mut self) -> Result<()> {
        let status = self.update_repaint_status().await?;
        self.frame.new_frame(self.gfx.exec.clone(), &self.window, &self.surface,  |mut ifc|  {
            // UI integration start of frame
            self.ui.new_frame(&self.window);

            egui::Window::new("Editor")
                .resizable(true)
                .movable(true)
                .interactable(true)
                .show(&self.ui.context(), |ui| {

                });

            // If we have a repaint, ask the graphics system for a redraw
            // In the future, we could even make this fully asynchronous and keep refreshing the UI while
            // we redraw, though this is only necessary if our frame time budget gets seriously
            // exceeded.
            let (graph, mut bindings) = match status {
                repaint::RepaintStatus::None => { (ph::PassGraph::new(), ph::PhysicalResourceBindings::new()) }
                repaint::RepaintStatus::UIOnly => { (ph::PassGraph::new(), ph::PhysicalResourceBindings::new()) }
                repaint::RepaintStatus::All => {
                    info!("Repainting world.");
                    self.renderer.redraw_world()?
                }
            };


            let swapchain = ph::VirtualResource::image("swapchain".to_owned());

            // Record UI commands
            let graph = graph.add_pass(ph::PassBuilder::render("ui".to_owned())
                .color_attachment(swapchain.clone(), vk::AttachmentLoadOp::CLEAR, Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}))?
                .execute(|cmd, ifc, _| {
                    self.ui.render(&self.window, unsafe { cmd.handle() }, ifc.swapchain_image_index.unwrap());
                    Ok(cmd)
                })
                .build()
            )?;

            // Add a present pass to the graph.
            let present_pass = ph::PassBuilder::present("present".to_owned(), swapchain.upgrade());
            let mut graph = graph.add_pass(present_pass)?.build()?;

            // Bind the swapchain resource.
            bindings.bind_image("swapchain".to_owned(), ifc.swapchain_image.clone().unwrap());
            // Record this frame.
            let cmd = self.gfx.exec.on_domain::<ph::domain::All>()?;
            let cmd = ph::record_graph(&mut graph, &bindings, &mut ifc, cmd, self.debug_messenger.as_ref())?;
            cmd.finish()
        }).await?;

        // Advance caches to the next frame.
        self.gfx.pipelines.lock().unwrap().next_frame(); // TODO: figure out how to properly implement '?' for this
        self.gfx.descriptors.lock().unwrap().next_frame();
        Ok(())
    }
}

pub fn process_event(driver: &mut Driver, event: winit::event::Event<()>) -> Result<ControlFlow> {
    use winit::event::Event;
    match event {
        Event::WindowEvent { event, window_id} => {
            match event {
                WindowEvent::Resized(_) => {}
                WindowEvent::Moved(_) => {}
                WindowEvent::CloseRequested => {
                    if window_id == driver.window.id() {
                        driver.gfx.device.wait_idle()?;
                        return Ok(ControlFlow::Exit);
                    }
                }
                WindowEvent::Destroyed => {}
                WindowEvent::DroppedFile(_) => {}
                WindowEvent::HoveredFile(_) => {}
                WindowEvent::HoveredFileCancelled => {}
                WindowEvent::ReceivedCharacter(_) => {}
                WindowEvent::Focused(_) => {}
                WindowEvent::KeyboardInput { input, .. } => {
                    // Register a key callback for repainting the scene.
                    // Note that we will abstract away input processing later
                    if let Some(keycode) = input.virtual_keycode {
                        if keycode == VirtualKeyCode::Return {
                            driver.repaint.tell(repaint::RepaintAll)?;
                        }
                    }
                }
                WindowEvent::ModifiersChanged(_) => {}
                WindowEvent::Ime(_) => {}
                WindowEvent::CursorMoved { .. } => {}
                WindowEvent::CursorEntered { .. } => {}
                WindowEvent::CursorLeft { .. } => {}
                WindowEvent::MouseWheel { .. } => {}
                WindowEvent::MouseInput { .. } => {}
                WindowEvent::TouchpadMagnify { .. } => {}
                WindowEvent::SmartMagnify { .. } => {}
                WindowEvent::TouchpadRotate { .. } => {}
                WindowEvent::TouchpadPressure { .. } => {}
                WindowEvent::AxisMotion { .. } => {}
                WindowEvent::Touch(_) => {}
                WindowEvent::ScaleFactorChanged { .. } => {}
                WindowEvent::ThemeChanged(_) => {}
                WindowEvent::Occluded(_) => {}
            }
        },
        Event::MainEventsCleared => {
            driver.window.request_redraw();
        }
        Event::RedrawRequested(_) => { // TODO: Multi-window
            block_on(driver.process_frame())?
        }
        _ => (),
    };

    Ok(ControlFlow::Wait)
}