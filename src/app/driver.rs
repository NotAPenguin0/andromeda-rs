use std::ops::Deref;
use anyhow::Result;

use futures::executor::block_on;

use phobos as ph;
use tokio::runtime::Handle;
use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

use crate::{gfx, gui};
use crate::app::RootActorSystem;
use crate::app::update_loop::UpdateLoop;
use crate::core::{ButtonState, InputEvent, Key, KeyState, MouseButton, MouseButtonState, MousePosition, ScrollInfo};

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
        gui::UIIntegration::new(event_loop, &window, gfx.shared.clone())
    }

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

            gui::build_ui(&self.ui.context(), &self.actors);

            Handle::current().block_on(async {
                self.actors.update_rt_size(&mut self.ui, &mut self.renderer).await?;

                self.update.update(
                    ifc,
                    &mut self.ui,
                    &self.window,
                    &mut self.renderer,
                    self.gfx.shared.clone(),
                    self.gfx.debug_messenger.as_ref()).await
                }
            )
        }).await?;

        self.gfx.next_frame();
        Ok(())
    }
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(value: winit::event::MouseButton) -> Self {
        match value {
            winit::event::MouseButton::Left => { MouseButton::Left },
            winit::event::MouseButton::Right => { MouseButton::Right },
            winit::event::MouseButton::Middle => { MouseButton::Middle },
            winit::event::MouseButton::Other(x) => { MouseButton::Other(x) },
        }
    }
}

impl From<ElementState> for ButtonState {
    fn from(value: ElementState) -> Self {
        match value {
            ElementState::Pressed => { ButtonState::Pressed }
            ElementState::Released => { ButtonState::Released }
        }
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
                WindowEvent::KeyboardInput { .. } => {

                }
                WindowEvent::ModifiersChanged(state) => {
                    if state.shift() {
                        driver.actors.input.tell(InputEvent::Button(KeyState {
                            state: ButtonState::Pressed,
                            button: Key::Shift,
                        }))?;
                    } else {
                        driver.actors.input.tell(InputEvent::Button(KeyState {
                            state: ButtonState::Released,
                            button: Key::Shift,
                        }))?;
                    }
                }
                WindowEvent::Ime(_) => {}
                WindowEvent::CursorMoved { position, .. } => {
                    driver.actors.input.tell(InputEvent::MouseMove(MousePosition { x: position.x, y: position.y }))?;
                }
                WindowEvent::CursorEntered { .. } => {}
                WindowEvent::CursorLeft { .. } => {}
                WindowEvent::MouseWheel { delta, .. } => {
                    match delta {
                        MouseScrollDelta::LineDelta(x, y) => {
                            driver.actors.input.tell(InputEvent::Scroll(ScrollInfo {
                                delta_x: x,
                                delta_y: y,
                            })).unwrap();
                        }
                        MouseScrollDelta::PixelDelta(px) => {
                            driver.actors.input.tell(InputEvent::Scroll(ScrollInfo {
                                delta_x: px.x as f32,
                                delta_y: px.y as f32,
                            })).unwrap();
                        }
                    };
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    driver.actors.input.tell(InputEvent::MouseButton(MouseButtonState {
                        state: state.into(),
                        button: button.into(),
                    }))?;
                }
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