use std::rc::Rc;

use anyhow::Result;
use glam::Vec3;
use poll_promise::Promise;

use crate::gfx::resource::TerrainPlane;
use crate::gfx::world_renderer::RenderOptions;
use crate::gfx::AtmosphereInfo;
use crate::math::Rotation;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FutureWorld {
    #[derivative(Debug = "ignore")]
    pub terrain_mesh: Option<Promise<Result<TerrainPlane>>>,
}

#[derive(Debug, Copy, Clone)]
pub struct TerrainOptions {
    /// Width and height of the terrain plane, in meters
    pub size: f32,
    /// Number of patches the terrain mesh will be divided in in each direction.
    pub patch_resolution: u32,
}

#[derive(Debug)]
pub struct World {
    /// Direction for the sun. This is represented as a rotation for easy editing.
    pub sun_direction: Rotation,
    pub atmosphere: AtmosphereInfo,
    pub terrain_mesh: Option<Rc<TerrainPlane>>,
    pub options: RenderOptions,
    pub terrain_options: TerrainOptions,
}

impl Default for World {
    fn default() -> Self {
        World {
            sun_direction: Rotation(Vec3::new(45f32.to_radians(), 0.0, 0.0)),
            atmosphere: AtmosphereInfo::earth(),
            terrain_mesh: None,
            options: Default::default(),
            terrain_options: TerrainOptions {
                size: 25.0,
                patch_resolution: 5,
            },
        }
    }
}
