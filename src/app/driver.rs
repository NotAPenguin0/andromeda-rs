use std::sync::{Arc, RwLock};

use anyhow::Result;
use futures::executor::block_on;
use glam::Vec3;
use tokio::runtime::Handle;
use winit::event::{ElementState, MouseScrollDelta, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop, EventLoopBuilder};
use winit::window::{Window, WindowBuilder};

use crate::app::update_loop::UpdateLoop;
use crate::core::{ButtonState, Input, InputEvent, Key, KeyState, MouseButton, MouseButtonState, MousePosition, ScrollInfo};
use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world::{FutureWorld, World};
use crate::gui::editor::camera_controller::{CameraController, CameraInputListener};
use crate::gui::util::integration::UIIntegration;
use crate::hot_reload::{ShaderReload, SyncShaderReload};
use crate::math::Position;
use crate::state::Camera;
use crate::{gfx, gui};

/// Main application driver. Hosts the event loop.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Driver {
    pub window: Window,
    renderer: gfx::WorldRenderer,
    ui: UIIntegration,
    update: UpdateLoop,
    pub world: World,
    pub future: FutureWorld,
    pub gfx: gfx::Context,
    pub input: Arc<RwLock<Input>>,
    pub camera_controller: Arc<RwLock<CameraController>>,
    pub shader_reload: SyncShaderReload,
}

impl Driver {
    pub fn create_window() -> Result<(EventLoop<()>, Window)> {
        let event_loop = EventLoopBuilder::new().build();
        let window = WindowBuilder::new()
            .with_title("Andromeda")
            .with_inner_size(winit::dpi::LogicalSize::new(1920.0, 1080.0))
            .build(&event_loop)?;
        Ok((event_loop, window))
    }

    fn create_gui_integration(event_loop: &EventLoop<()>, window: &Window, gfx: &gfx::Context) -> Result<UIIntegration> {
        UIIntegration::new(event_loop, &window, gfx.shared.clone())
    }

    pub fn init(event_loop: &EventLoop<()>, window: Window) -> Result<Driver> {
        let gfx = gfx::Context::new(&window)?;
        let ui = Self::create_gui_integration(event_loop, &window, &gfx)?;
        let shader_reload = ShaderReload::new(gfx.shared.pipelines.clone(), "shaders/", true)?;
        let renderer = gfx::WorldRenderer::new(shader_reload.clone(), gfx.shared.clone())?;
        let update = UpdateLoop::new(&gfx)?;

        let input = Arc::new(RwLock::new(Input::default()));
        let mut camera = Camera::default();
        camera.set_position(Position(Vec3::new(0.0, 2000.0, 0.0)));
        let camera = Arc::new(RwLock::new(camera));
        let camera_controller = Arc::new(RwLock::new(CameraController::new(camera.clone())));
        input
            .write()
            .unwrap()
            .add_listener(CameraInputListener::new(camera_controller.clone()));
        let world = World::new(camera);
        // Initially generate a mesh already
        let future = FutureWorld {
            terrain_mesh: Some(TerrainPlane::generate(gfx.shared.clone(), world.terrain_options)),
            heightmap: Some(HeightMap::from_file("data/heightmaps/iceland.nc", gfx.shared.clone())),
        };
        Ok(Driver {
            window,
            gfx,
            ui,
            renderer,
            update,
            world,
            future,
            input,
            camera_controller,
            shader_reload,
        })
    }

    pub async fn process_frame(&mut self) -> Result<()> {
        self.gfx
            .frame
            .new_frame(self.gfx.shared.exec.clone(), &self.window, &self.gfx.surface, |ifc| {
                // Do start of frame logic, we'll keep this here to keep things a bit easier
                self.ui.new_frame(&self.window);
                self.renderer.new_frame();

                gui::build_ui(
                    &self.ui.context(),
                    &mut self.ui,
                    self.gfx.shared.clone(),
                    &mut self.renderer.targets,
                    &self.camera_controller,
                    &mut self.future,
                    &mut self.world,
                );

                Handle::current().block_on(async {
                    self.update
                        .update(
                            ifc,
                            &mut self.ui,
                            &self.window,
                            &mut self.world,
                            &mut self.future,
                            &mut self.renderer,
                            self.gfx.shared.clone(),
                            self.gfx.debug_messenger.as_ref(),
                        )
                        .await
                })
            })
            .await?;

        self.gfx.next_frame();
        Ok(())
    }
}

impl From<winit::event::MouseButton> for MouseButton {
    fn from(value: winit::event::MouseButton) -> Self {
        match value {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            winit::event::MouseButton::Other(x) => MouseButton::Other(x),
        }
    }
}

impl From<ElementState> for ButtonState {
    fn from(value: ElementState) -> Self {
        match value {
            ElementState::Pressed => ButtonState::Pressed,
            ElementState::Released => ButtonState::Released,
        }
    }
}

pub fn process_event(driver: &mut Driver, event: winit::event::Event<()>) -> Result<ControlFlow> {
    use winit::event::Event;
    match event {
        Event::WindowEvent {
            event,
            window_id,
        } => {
            driver.ui.process_event(&event);
            let mut input = driver.input.write().unwrap();
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
                WindowEvent::KeyboardInput {
                    input,
                    ..
                } => {
                    if input.state == ElementState::Pressed {
                        match input.virtual_keycode {
                            None => {}
                            Some(keycode) => match keycode {
                                VirtualKeyCode::R => {
                                    driver.future.terrain_mesh = Some(TerrainPlane::generate(driver.gfx.shared.clone(), driver.world.terrain_options))
                                }
                                _ => {}
                            },
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
            driver.window.request_redraw();
        }
        Event::RedrawRequested(_) => {
            // TODO: Multi-window
            block_on(driver.process_frame())?
        }
        _ => (),
    };

    Ok(ControlFlow::Wait)
}
