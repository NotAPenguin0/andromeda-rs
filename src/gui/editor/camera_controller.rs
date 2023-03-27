use std::sync::{Arc, RwLock};

use anyhow::Result;
use glam::Vec3;

use crate::core::{ButtonState, Event, Input, InputEvent, InputListener, Key, MouseButton, MouseDelta, ScrollInfo};
use crate::math::{Position, Rotation};
use crate::state::Camera;

#[derive(Debug)]
pub struct CameraController {
    camera: Arc<RwLock<Camera>>,
    enabled: bool,
}

#[derive(Debug)]
pub struct CameraInputListener {
    controller: Arc<RwLock<CameraController>>,
}

impl CameraController {
    pub fn new(camera: Arc<RwLock<Camera>>) -> Self {
        Self {
            camera,
            enabled: false,
        }
    }

    pub fn set_enabled(&mut self, enable: bool) {
        self.enabled = enable;
    }

    fn handle_move(&self, mouse: MouseDelta) -> Result<()> {
        let mut camera = self.camera.write().unwrap();
        let delta = camera.up() * (mouse.y as f32) + camera.right() * (-mouse.x as f32);
        const SPEED: f32 = 0.02;
        camera.update_position(Position(delta * SPEED));
        Ok(())
    }

    fn handle_rotate(&self, mouse: MouseDelta) -> Result<()> {
        let delta = Vec3::new(-mouse.y as f32, mouse.x as f32, 0.0);
        const SPEED: f32 = 0.01;
        let mut camera = self.camera.write().unwrap();
        camera.update_rotation(Rotation(delta * SPEED));
        Ok(())
    }

    fn handle_scroll(&self, scroll: ScrollInfo) -> Result<()> {
        let mut camera = self.camera.write().unwrap();
        let delta = camera.front() * scroll.delta_y;
        const SPEED: f32 = 0.5;
        camera.update_position(Position(delta * SPEED));
        Ok(())
    }

    pub fn handle_event(&mut self, event: InputEvent, input: &Input) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        match event {
            InputEvent::MouseMove(delta) => {
                if input.get_mouse_key(MouseButton::Middle) == ButtonState::Pressed {
                    if input.get_key(Key::Shift) == ButtonState::Pressed {
                        self.handle_move(delta)?;
                    } else {
                        self.handle_rotate(delta)?;
                    }
                }
            }
            InputEvent::Scroll(scroll) => {
                self.handle_scroll(scroll)?;
            }
            _ => {}
        }

        Ok(())
    }
}

impl CameraInputListener {
    pub fn new(camera: Arc<RwLock<CameraController>>) -> Self {
        Self {
            controller: camera,
        }
    }
}

impl InputListener for CameraInputListener {
    fn handle(&self, event: InputEvent, input: &Input) -> Result<()> {
        let mut controller = self.controller.write().unwrap();
        controller.handle_event(event, input)?;
        Ok(())
    }
}

/// Enable the camera controls when this widget is hovered
pub fn enable_camera_over(response: &egui::Response, controller: &Arc<RwLock<CameraController>>) {
    let hover = response.hovered();
    let mut controller = controller.write().unwrap();
    controller.set_enabled(hover);
}
