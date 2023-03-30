use std::sync::{Arc, RwLock};

use anyhow::Result;
use glam::Vec3;
use poll_promise::Promise;

use crate::gfx::resource::terrain::Terrain;
use crate::gfx::world_renderer::RenderOptions;
use crate::gfx::AtmosphereInfo;
use crate::math::Rotation;
use crate::state::Camera;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FutureWorld {
    #[derivative(Debug = "ignore")]
    pub terrain: Option<Promise<Result<Terrain>>>,
}

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
    pub terrain: Option<Terrain>,
    pub options: RenderOptions,
    pub terrain_options: TerrainOptions,
    pub camera: Arc<RwLock<Camera>>,
}

impl World {
    pub fn new(camera: Arc<RwLock<Camera>>) -> Self {
        World {
            sun_direction: Rotation(Vec3::new(45f32.to_radians(), 0.0, 0.0)),
            atmosphere: AtmosphereInfo::earth(),
            terrain: None,
            options: Default::default(),
            terrain_options: TerrainOptions {
                horizontal_scale: 512.0,
                vertical_scale: 100.0,
                patch_resolution: 32,
            },
            camera,
        }
    }
}
