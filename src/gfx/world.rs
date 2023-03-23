use glam::Vec3;

use crate::gfx::AtmosphereInfo;

#[derive(Debug, Clone)]
pub struct World {
    /// Direction vector for the sun. This must point away from the sun.
    pub sun_direction: Vec3,
    pub atmosphere: AtmosphereInfo,
}