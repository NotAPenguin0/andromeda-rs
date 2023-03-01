use std::ops::Deref;
use anyhow::Result;

use futures::executor::block_on;

use phobos as ph;
use tokio::runtime::Handle;
use winit::event::{VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

use crate::{gfx, gui, repaint};
use crate::app::RootActorSystem;
use crate::app::update_loop::UpdateLoop;

/// Main application driver. Hosts the event loop.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Driver<'f> {
    pub window: Window,
    renderer: gfx::WorldRenderer,
    ui: gui::UIIntegration,
    update: UpdateLoop,
    pub actors: RootActorSystem,
    pub gfx: gfx::Context<'f>
}

impl<'f> Driver<'f> {
    pub fn create_window() -> Result<(EventLoop<()>, Window)> {
        let event_loop = EventLoopBuilder::new().build();
        let window = WindowBuilder::new()
            .with_title("Andromeda")
            .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }

    fn create_gui_integration(event_loop: &EventLoop<()>, window: &Window, gfx: &gfx::Context) -> Result<gui::UIIntegration> {
        let queue = gfx.shared.exec.get_queue::<ph::domain::Graphics>().unwrap();
        gui::UIIntegration::new(event_loop,
                                &window,
                                gfx.shared.clone(),
                                queue.deref(),
                                unsafe { gfx.frame.get_swapchain() })
    }

    // TODO: Cleanup
    pub fn init(event_loop: &EventLoop<()>, window: Window) -> Result<Driver<'f>> {
        let gfx = gfx::Context::new(&window)?;
        let actors = block_on(RootActorSystem::new(&gfx.shared))?;
        let ui = Self::create_gui_integration(event_loop, &window, &gfx)?;
        let renderer = gfx::WorldRenderer::new(&actors, gfx.shared.clone())?;
        let update = UpdateLoop::new(&gfx)?;

        Ok(Driver {
            window,
            gfx,
            ui,
            renderer,
            actors,
            update,
        })
    }

    pub async fn process_frame(&mut self) -> Result<()> {
        self.gfx.frame.new_frame(self.gfx.shared.exec.clone(), &self.window, &self.gfx.surface,  |mut ifc| {
            // Do start of frame logic, we'll keep this here to keep things a bit easier
            self.ui.new_frame(&self.window);
            self.renderer.new_frame();

            gui::build_ui(&self.ui.context(), self.actors.scene_texture.clone());

            Handle::current().block_on(async {
                self.actors.update_rt_size(&mut self.ui, &mut self.renderer).await?;
                let scene_output = self.renderer.output_image().view.clone();

                // If we have a repaint, ask the graphics system for a redraw
                // In the future, we could even make this fully asynchronous and keep refreshing the UI while
                // we redraw, though this is only necessary if our frame time budget gets seriously
                // exceeded.
                let status = self.actors.update_repaint_status().await?;

                self.update.update(
                    ifc,
                    &mut self.ui,
                    &self.window,
                    scene_output,
                    &mut self.renderer,
                    status,
                    self.gfx.shared.clone(),
                    self.gfx.debug_messenger.as_ref()).await
                }
            )
        }).await?;

        self.gfx.next_frame();
        Ok(())
    }
}

pub fn process_event(driver: &mut Driver, event: winit::event::Event<()>) -> Result<ControlFlow> {
    use winit::event::Event;
    match event {
        Event::WindowEvent { event, window_id} => {
            driver.ui.process_event(&event);
            match event {
                WindowEvent::Resized(_) => {}
                WindowEvent::Moved(_) => {}
                WindowEvent::CloseRequested => {
                    if window_id == driver.window.id() {
                        driver.gfx.shared.device.wait_idle()?;
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
                            driver.actors.repaint.tell(repaint::RepaintAll)?;
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