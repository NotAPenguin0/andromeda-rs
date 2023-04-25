use std::ops::{Add, Div, Mul, Sub};

use glam::Vec3;

#[derive(Debug, Default, Clone, Copy)]
pub struct Position(pub Vec3);

#[derive(Debug, Default, Clone, Copy)]
pub struct Rotation(pub Vec3);

impl Rotation {
    /// Convert a rotation to a normalized direction vector pointing towards the front
    /// direction of that rotation.
    pub fn front_direction(&self) -> Vec3 {
        let cos_pitch = self.pitch().cos();
        let cos_yaw = self.yaw().cos();
        let sin_pitch = self.pitch().sin();
        let sin_yaw = self.yaw().sin();

        Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }

    pub fn x(&self) -> f32 {
        self.0.x
    }

    pub fn y(&self) -> f32 {
        self.0.y
    }

    #[allow(dead_code)]
    pub fn z(&self) -> f32 {
        self.0.z
    }

    pub fn pitch(&self) -> f32 {
        self.x()
    }

    pub fn yaw(&self) -> f32 {
        self.y()
    }

    #[allow(dead_code)]
    pub fn roll(&self) -> f32 {
        self.z()
    }
}

impl Add<Rotation> for Rotation {
    type Output = Self;

    fn add(self, rhs: Rotation) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Rotation> for Rotation {
    type Output = Self;

    fn sub(self, rhs: Rotation) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Mul<f32> for Rotation {
    type Output = Rotation;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<f32> for Rotation {
    type Output = Rotation;

    fn div(self, rhs: f32) -> Self::Output {
        Self(self.0 / rhs)
    }
}
