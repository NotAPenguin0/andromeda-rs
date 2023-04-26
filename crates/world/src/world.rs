use assets::{Terrain, TerrainOptions};
use glam::Vec3;
use math::Rotation;
use thread::promised_value::PromisedValue;

use crate::{AtmosphereInfo, RenderOptions};

#[derive(Debug)]
pub struct World {
    /// Direction of the sun. This is represented as a rotation for easy editing.
    pub sun_direction: Rotation,
    pub atmosphere: AtmosphereInfo,
    pub terrain: PromisedValue<Terrain>,
    pub options: RenderOptions,
    pub terrain_options: TerrainOptions,
}

impl World {
    pub fn new() -> Self {
        World {
            sun_direction: Rotation(Vec3::new(45f32.to_radians(), 0.0, 0.0)),
            atmosphere: AtmosphereInfo::earth(),
            terrain: PromisedValue::new(),
            options: Default::default(),
            terrain_options: TerrainOptions {
                horizontal_scale: 512.0,
                vertical_scale: 100.0,
                patch_resolution: 32,
            },
        }
    }

    pub fn poll_all(&mut self) {
        self.terrain.poll();
    }
}
