use std::ops::Deref;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use glam::{Mat4, Vec3};
use inject::DI;
use input::{ButtonState, InputEvent, InputState, Key, MouseButton, MouseDelta, ScrollInfo};
use log::trace;
use math::{Position, Rotation};
use scheduler::{Event, EventBus, EventContext, StoredSystem, System};

#[derive(Debug, Copy, Clone)]
pub struct CameraState {
    position: Position,
    rotation: Rotation,
    fov: f32,
}

#[derive(Debug)]
pub struct EnableCameraEvent {
    pub enabled: bool,
}

impl Event for EnableCameraEvent {}

#[derive(Debug, Clone)]
pub struct Camera {
    enable_controls: bool,
}

impl Camera {
    pub fn new() -> Self {
        Self {
            enable_controls: true,
        }
    }
}

/// Base vectors for the camera's coordinate space.
#[derive(Debug, Clone, Copy)]
pub struct CameraVectors {
    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            position: Default::default(),
            rotation: Default::default(),
            fov: 90.0,
        }
    }
}

impl CameraState {
    fn clamp_rotation(rot: Rotation) -> Rotation {
        const MAX_ANGLE: f32 = std::f32::consts::PI / 2.0 - 0.0001;
        const UNBOUNDED: f32 = f32::MAX;
        Rotation(
            rot.0.clamp(
                Vec3::new(-MAX_ANGLE, -UNBOUNDED, 0.0),
                Vec3::new(MAX_ANGLE, UNBOUNDED, 0.0),
            ),
        )
    }

    pub fn front(&self) -> Vec3 {
        self.rotation.front_direction()
    }

    pub fn right(&self) -> Vec3 {
        self.front().cross(Vec3::new(0.0, 1.0, 0.0)).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.front()).normalize()
    }

    pub fn matrix(&self) -> Mat4 {
        let front = self.front();
        let up = self.up();
        Mat4::look_at_rh(self.position.0, self.position.0 + front, up)
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn fov(&self) -> f32 {
        self.fov
    }

    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    pub fn set_rotation(&mut self, rot: Rotation) {
        self.rotation = Self::clamp_rotation(rot);
    }

    pub fn update_position(&mut self, pos: Position) {
        self.position.0 += pos.0;
    }

    pub fn update_rotation(&mut self, rot: Rotation) {
        self.rotation.0 += rot.0;
        self.rotation = Self::clamp_rotation(self.rotation);
    }

    pub fn update_fov(&mut self, fov: f32) {
        self.fov += fov;
    }

    fn handle_move(&mut self, mouse: &MouseDelta) -> Result<()> {
        let delta = self.up() * (mouse.y as f32) + self.right() * (-mouse.x as f32);
        const SPEED: f32 = 5.0;
        self.update_position(Position(delta * SPEED));
        Ok(())
    }

    fn handle_rotate(&mut self, mouse: &MouseDelta) -> Result<()> {
        let delta = Vec3::new(-mouse.y as f32, mouse.x as f32, 0.0);
        const SPEED: f32 = 0.01;
        self.update_rotation(Rotation(delta * SPEED));
        Ok(())
    }

    fn handle_scroll(&mut self, scroll: &ScrollInfo) -> Result<()> {
        let delta = self.front() * scroll.delta_y;
        const SPEED: f32 = 50.0;
        self.update_position(Position(delta * SPEED));
        Ok(())
    }

    pub fn handle_event(&mut self, event: &InputEvent, input: &InputState) -> Result<()> {
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

impl System<DI> for Camera {
    fn initialize(event_bus: &mut EventBus<DI>, system: &StoredSystem<Self>)
    where
        Self: Sized, {
        event_bus.subscribe(system, handle_input_event);
        event_bus.subscribe(system, handle_enabled_event);
    }
}

fn handle_enabled_event(
    camera: &mut Camera,
    event: &EnableCameraEvent,
    _ctx: &mut EventContext<DI>,
) -> Result<()> {
    camera.enable_controls = event.enabled;
    Ok(())
}

fn handle_input_event(
    camera: &mut Camera,
    event: &InputEvent,
    ctx: &mut EventContext<DI>,
) -> Result<()> {
    if camera.enable_controls {
        let di = ctx.read().unwrap();
        let mut state = di.write_sync::<CameraState>().unwrap();
        let input = di.read_sync::<InputState>().unwrap();
        state.handle_event(event, &input)?;
    }
    Ok(())
}

pub fn initialize(
    position: Position,
    rotation: Rotation,
    fov: f32,
    bus: &mut EventBus<DI>,
) -> Result<()> {
    // Create the camera state object and register it into the DI system
    let state = CameraState {
        position,
        rotation,
        fov,
    };
    bus.data_mut().write().unwrap().put_sync(state);
    // Add the camera controller system
    bus.add_system(Camera::new());
    Ok(())
}
