use glam::{Mat4, Vec3};

use crate::math::{Position, Rotation};

#[derive(Debug)]
pub struct Camera {
    position: Position,
    rotation: Rotation,
    fov: f32,
}

/// Base vectors for the camera's coordinate space.
#[derive(Debug, Clone, Copy)]
pub struct CameraVectors {
    pub front: Vec3,
    pub right: Vec3,
    pub up: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            position: Default::default(),
            rotation: Default::default(),
            fov: 90.0,
        }
    }
}

impl Camera {
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

    #[allow(dead_code)]
    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    pub fn fov(&self) -> f32 {
        self.fov
    }

    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn update_fov(&mut self, fov: f32) {
        self.fov += fov;
    }
}
