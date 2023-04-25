use std::sync::{Arc, RwLock};

use glam::Vec3;
use math::Rotation;
use thread::promised_value::PromisedValue;

use crate::gfx::resource::terrain::Terrain;
use crate::{AtmosphereInfo, RenderOptions};

#[derive(Debug, Copy, Clone)]
pub struct TerrainOptions {
    /// Width and height of the terrain plane in meters.
    pub horizontal_scale: f32,
    /// Vertical scaling. The most extreme point of the terrain will have this as its height.
    pub vertical_scale: f32,
    /// Number of patches the terrain mesh will be divided in in each direction.
    pub patch_resolution: u32,
}

#[derive(Debug)]
pub struct World {
    /// Direction of the sun. This is represented as a rotation for easy editing.
    pub sun_direction: Rotation,
    pub atmosphere: AtmosphereInfo,
    pub terrain: PromisedValue<Terrain>,
    pub options: RenderOptions,
    pub terrain_options: TerrainOptions,
    pub camera: Arc<RwLock<Camera>>,
}

impl World {
    pub fn new(camera: Arc<RwLock<Camera>>) -> Self {
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
            camera,
        }
    }

    pub fn poll_all(&mut self) {
        self.terrain.poll();
    }
}
