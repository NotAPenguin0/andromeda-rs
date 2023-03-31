use std::sync::{Arc, RwLock};

use anyhow::Result;
use futures::executor::block_on;
use glam::Vec3;
use winit::event::{ElementState, Event, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

use crate::app::renderer::AppRenderer;
use crate::app::window::AppWindow;
use crate::core::{
    ButtonState, Input, InputEvent, Key, KeyState, MouseButtonState, MousePosition, ScrollInfo,
};
use crate::gfx;
use crate::gfx::resource::terrain::Terrain;
use crate::gfx::world::World;
use crate::gui::editor::camera_controller::{CameraController, CameraInputListener};
use crate::gui::editor::Editor;
use crate::math::Position;
use crate::state::Camera;

/// Main application driver. Holds core modules such as the renderer,
/// window and input systems. Feed this all winit events using [`Driver::process_event`] to run the application.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Driver {
    window: AppWindow,
    renderer: AppRenderer,
    world: World,
    input: Arc<RwLock<Input>>,
    editor: Editor,
}

impl Driver {
    /// Initialize the application driver with a window and event loop.
    pub fn init(event_loop: &EventLoop<()>, window: Window) -> Result<Driver> {
        let (gfx, window, renderer) = gfx::init_graphics(window, &event_loop)?;

        let input = Arc::new(RwLock::new(Input::default()));
        let mut camera = Camera::default();
        camera.set_position(Position(Vec3::new(0.0, 200.0, 0.0)));
        let camera = Arc::new(RwLock::new(camera));
        let camera_controller = Arc::new(RwLock::new(CameraController::new(camera.clone())));
        input
            .write()
            .unwrap()
            .add_listener(CameraInputListener::new(camera_controller.clone()));
        let mut world = World::new(camera);

        world.terrain.promise(Terrain::from_new_heightmap(
            "data/heightmaps/mountain.png",
            "data/textures/blank.png",
            world.terrain_options,
            gfx.clone(),
        ));

        let editor = Editor::new(renderer.ui(), gfx, camera_controller);

        Ok(Driver {
            window,
            renderer,
            world,
            input,
            editor,
        })
    }

    /// Process one frame. This will update the UI and render the world.
    async fn process_frame(&mut self) -> Result<()> {
        self.window
            .new_frame(|window, ifc| {
                self.world.poll_all();
                self.renderer.new_frame(window);

                self.editor
                    .show(&mut self.world, self.renderer.image_provider());

                self.renderer.render(&window, &self.world, ifc)
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
                let mut input = self.input.write().unwrap();
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
                        input,
                        ..
                    } => {
                        if input.state == ElementState::Pressed {
                            match input.virtual_keycode {
                                None => {}
                                _ => {}
                            }
                        }
                    }
                    WindowEvent::ModifiersChanged(state) => {
                        if state.shift() {
                            input.process_event(InputEvent::Button(KeyState {
                                state: ButtonState::Pressed,
                                button: Key::Shift,
                            }));
                        } else {
                            input.process_event(InputEvent::Button(KeyState {
                                state: ButtonState::Released,
                                button: Key::Shift,
                            }));
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
                        }));
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
                                }));
                            }
                            MouseScrollDelta::PixelDelta(px) => {
                                input.process_event(InputEvent::Scroll(ScrollInfo {
                                    delta_x: px.x as f32,
                                    delta_y: px.y as f32,
                                }));
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
                        }));
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
                block_on(self.process_frame())?
            }
            _ => (),
        };

        Ok(ControlFlow::Wait)
    }
}
