use anyhow::Result;
use assets::storage::AssetStorage;
use assets::TerrainLoadInfo;
use derivative::Derivative;
use events::Tick;
use futures::executor::block_on;
use glam::Vec3;
use inject::DI;
use input::{
    ButtonState, InputEvent, InputState, Key, KeyState, MouseButtonState, MouseDelta,
    MousePosition, ScrollInfo,
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
        let (frame, surface, ctx) = gfx::initialize(&window, &bus)?;
        input::initialize(&mut bus);
        camera::initialize(
            Position(Vec3::new(0.0, 200.0, 0.0)),
            Rotation(Vec3::new(0.0, 0.0, 0.0)),
            90.0f32,
            &mut bus,
        )?;

        world::initialize(&bus)?;
        hot_reload::initialize(ctx.pipelines.clone(), "shaders/", true, &mut bus)?;
        assets::initialize(bus.clone())?;

        let renderer = AppRenderer::new(ctx.clone(), &window, event_loop, bus.clone())?;
        let window = AppWindow::new(frame, window, surface, ctx.clone());
        gui::initialize(renderer.ui(), &mut bus);

        {
            let mut inject = inject.write().unwrap();
            let statistics = RendererStatistics::new(ctx, 32, 60)?;
            inject.put_sync::<RendererStatistics>(statistics);
        }

        {
            let inject = inject.read().unwrap();
            let mut world = inject.write_sync::<World>().unwrap();
            let assets = inject.get::<AssetStorage>().unwrap();
            world.terrain = Some(assets.load(TerrainLoadInfo::FromHeightmap {
                height_path: "data/heightmaps/mountain.png".into(),
                texture_path: "data/textures/blank.png".into(),
                options: world.terrain_options,
            }));
        }

        Ok(Driver {
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
                self.renderer.new_frame(window);

                {
                    let inject = self.bus.data().read().unwrap();
                    inject
                        .write_sync::<RendererStatistics>()
                        .unwrap()
                        .new_frame();
                }

                self.bus.publish(&Tick)?;

                let inject = self.bus.data().read().unwrap();
                let world = inject.read_sync::<World>().unwrap();
                self.renderer.render(window, &world, &self.bus, ifc)
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
                            self.bus.publish(&InputEvent::Button(KeyState {
                                state: ButtonState::Pressed,
                                button: Key::Shift,
                            }))?;
                        } else {
                            self.bus.publish(&InputEvent::Button(KeyState {
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
                        let prev = self
                            .bus
                            .data()
                            .read()
                            .unwrap()
                            .read_sync::<InputState>()
                            .unwrap()
                            .mouse();
                        // Publish two events: One for the absolute mouse position, one for the mouse movement
                        self.bus.publish(&InputEvent::MousePosition(MousePosition {
                            x: position.x,
                            y: position.y,
                        }))?;
                        self.bus.publish(&InputEvent::MouseMove(MouseDelta {
                            x: position.x - prev.x,
                            y: position.y - prev.y,
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
                                self.bus.publish(&InputEvent::Scroll(ScrollInfo {
                                    delta_x: x,
                                    delta_y: y,
                                }))?;
                            }
                            MouseScrollDelta::PixelDelta(px) => {
                                self.bus.publish(&InputEvent::Scroll(ScrollInfo {
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
                        self.bus
                            .publish(&InputEvent::MouseButton(MouseButtonState {
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
        Ok(ControlFlow::Wait)
    }
}
