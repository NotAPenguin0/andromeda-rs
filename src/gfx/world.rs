use glam::Vec3;

use crate::gfx::AtmosphereInfo;
use crate::math::Rotation;

#[derive(Debug, Clone)]
pub struct World {
    /// Direction for the sun. This is represented as a rotation for easy editing.
    pub sun_direction: Rotation,
    pub atmosphere: AtmosphereInfo,
}
