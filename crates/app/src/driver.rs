use std::sync::{Arc, RwLock};

use anyhow::Result;
use assets::Terrain;
use derivative::Derivative;
use futures::executor::block_on;
use gfx::SharedContext;
use glam::Vec3;
use gui::editor::Editor;
use gui::util::size::USize;
use inject::DI;
use input::{
    ButtonState, Input, InputEvent, Key, KeyState, MouseButtonState, MousePosition, ScrollInfo,
};
use math::{Position, Rotation};
use scheduler::EventBus;
use statistics::RendererStatistics;
use winit::event::{Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;
use world::World;

use crate::renderer::AppRenderer;
use crate::window::AppWindow;

/// Main application driver. Holds core modules such as the renderer,
/// window and input systems. Feed this all winit events using [`Driver::process_event`] to run the application.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Driver {
    world: World,
    editor: Editor,
    bus: EventBus<DI>,
    renderer: AppRenderer,
    window: AppWindow,
}

impl Driver {
    /// Initialize the application driver with a window and event loop.
    pub fn init(event_loop: &EventLoop<()>, window: Window) -> Result<Driver> {
        // Create event bus and dependency injection module.
        let inject = DI::new();
        let mut bus = EventBus::new(inject.clone());

        // Initialize subsystems
        let (frame, surface) = gfx::initialize(&window, &bus)?;
        input::initialize(&bus);
        camera::initialize(
            Position(Vec3::new(0.0, 200.0, 0.0)),
            Rotation(Vec3::new(0.0, 0.0, 0.0)),
            90.0f32,
            &mut bus,
        )?;
        let mut world = World::new();

        world.terrain.promise(Terrain::from_new_heightmap(
            "data/heightmaps/mountain.png",
            "data/textures/blank.png",
            world.terrain_options,
            bus.clone(),
        ));

        let ctx = inject
            .read()
            .unwrap()
            .get::<SharedContext>()
            .cloned()
            .unwrap();
        hot_reload::initialize(ctx.pipelines.clone(), "shaders/", true, &mut bus)?;
        assets::initialize(bus.clone())?;
        let renderer = AppRenderer::new(ctx.clone(), &window, event_loop, bus.clone())?;
        let window = AppWindow::new(frame, window, surface, ctx.clone());
        let editor = Editor::new(renderer.ui(), bus.clone());

        let mut inject = inject.write().unwrap();
        let statistics = RendererStatistics::new(ctx, 32, 60)?;
        inject.put::<RendererStatistics>(statistics);

        Ok(Driver {
            world,
            editor,
            bus,
            renderer,
            window,
        })
    }

    /// Process one frame. This will update the UI and render the world.
    async fn process_frame(&mut self) -> Result<()> {
        self.window.request_redraw();
        self.window
            .new_frame(|window, ifc| {
                self.world.poll_all();
                self.renderer.new_frame(window);

                {
                    let mut inject = self.bus.data().write().unwrap();
                    inject.get_mut::<RendererStatistics>().unwrap().new_frame();
                }

                let target = self
                    .renderer
                    .get_output_image(USize::new(800, 600), self.bus.clone());
                self.editor.show(&mut self.world, target);
                self.renderer
                    .render(window, &self.world, self.bus.clone(), ifc)
            })
            .await?;
        Ok(())
    }

    /// Process a winit event. This forwards events to the input and UI systems, as well as
    /// renders a frame when a redraw is requested.
    pub fn process_event(&mut self, event: Event<()>) -> Result<ControlFlow> {
        match event {
            Event::WindowEvent {
                event,
                window_id,
            } => {
                self.renderer.process_event(&event);
                let mut inject = self.bus.data().write().unwrap();
                let input = inject.get_mut::<Input>().unwrap();
                match event {
                    WindowEvent::Resized(_) => {}
                    WindowEvent::Moved(_) => {}
                    WindowEvent::CloseRequested => {
                        if window_id == self.window.id() {
                            self.renderer.gfx().device.wait_idle()?;
                            return Ok(ControlFlow::Exit);
                        }
                    }
                    WindowEvent::Destroyed => {}
                    WindowEvent::DroppedFile(_) => {}
                    WindowEvent::HoveredFile(_) => {}
                    WindowEvent::HoveredFileCancelled => {}
                    WindowEvent::ReceivedCharacter(_) => {}
                    WindowEvent::Focused(_) => {}
                    WindowEvent::KeyboardInput {
                        ..
                    } => {}
                    WindowEvent::ModifiersChanged(state) => {
                        if state.shift() {
                            input.process_event(InputEvent::Button(KeyState {
                                state: ButtonState::Pressed,
                                button: Key::Shift,
                            }))?;
                        } else {
                            input.process_event(InputEvent::Button(KeyState {
                                state: ButtonState::Released,
                                button: Key::Shift,
                            }))?;
                        }
                    }
                    WindowEvent::Ime(_) => {}
                    WindowEvent::CursorMoved {
                        position,
                        ..
                    } => {
                        input.process_event(InputEvent::MousePosition(MousePosition {
                            x: position.x,
                            y: position.y,
                        }))?;
                    }
                    WindowEvent::CursorEntered {
                        ..
                    } => {}
                    WindowEvent::CursorLeft {
                        ..
                    } => {}
                    WindowEvent::MouseWheel {
                        delta,
                        ..
                    } => {
                        match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                input.process_event(InputEvent::Scroll(ScrollInfo {
                                    delta_x: x,
                                    delta_y: y,
                                }))?;
                            }
                            MouseScrollDelta::PixelDelta(px) => {
                                input.process_event(InputEvent::Scroll(ScrollInfo {
                                    delta_x: px.x as f32,
                                    delta_y: px.y as f32,
                                }))?;
                            }
                        };
                    }
                    WindowEvent::MouseInput {
                        state,
                        button,
                        ..
                    } => {
                        input.process_event(InputEvent::MouseButton(MouseButtonState {
                            state: state.into(),
                            button: button.into(),
                        }))?;
                    }
                    WindowEvent::TouchpadMagnify {
                        ..
                    } => {}
                    WindowEvent::SmartMagnify {
                        ..
                    } => {}
                    WindowEvent::TouchpadRotate {
                        ..
                    } => {}
                    WindowEvent::TouchpadPressure {
                        ..
                    } => {}
                    WindowEvent::AxisMotion {
                        ..
                    } => {}
                    WindowEvent::Touch(_) => {}
                    WindowEvent::ScaleFactorChanged {
                        ..
                    } => {}
                    WindowEvent::ThemeChanged(_) => {}
                    WindowEvent::Occluded(_) => {}
                }
            }
            Event::MainEventsCleared => {
                self.window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // TODO: Multi-window
                block_on(self.process_frame())?;
            }
            _ => (),
        };
        {
            let mut inject = self.bus.data().read().unwrap();
            let input = inject.get::<Input>().unwrap();
            input.flush(self.bus.clone())?;
        }
        {
            let mut inject = self.bus.data().write().unwrap();
            let input = inject.get_mut::<Input>().unwrap();
            input.clear_buffer()
        }
        Ok(ControlFlow::Wait)
    }
}
