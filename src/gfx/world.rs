use std::rc::Rc;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use glam::Vec3;
use poll_promise::Promise;

use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world_renderer::RenderOptions;
use crate::gfx::AtmosphereInfo;
use crate::math::Rotation;
use crate::state::Camera;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FutureWorld {
    #[derivative(Debug = "ignore")]
    pub terrain_mesh: Option<Promise<Result<TerrainPlane>>>,
    #[derivative(Debug = "ignore")]
    pub heightmap: Option<Promise<Result<HeightMap>>>,
}

#[derive(Debug, Copy, Clone)]
pub struct TerrainOptions {
    /// Width and height of the terrain plane, in meters
    pub scale: f32,
    /// Number of patches the terrain mesh will be divided in in each direction.
    pub patch_resolution: u32,
}

#[derive(Debug)]
pub struct World {
    /// Direction for the sun. This is represented as a rotation for easy editing.
    pub sun_direction: Rotation,
    pub atmosphere: AtmosphereInfo,
    pub terrain_mesh: Option<Rc<TerrainPlane>>,
    pub height_map: Option<Rc<HeightMap>>,
    pub options: RenderOptions,
    pub terrain_options: TerrainOptions,
    pub camera: Arc<RwLock<Camera>>,
}

impl World {
    pub fn new(camera: Arc<RwLock<Camera>>) -> Self {
        World {
            sun_direction: Rotation(Vec3::new(45f32.to_radians(), 0.0, 0.0)),
            atmosphere: AtmosphereInfo::earth(),
            terrain_mesh: None,
            height_map: None,
            options: Default::default(),
            terrain_options: TerrainOptions {
                scale: 10000.0,
                patch_resolution: 5,
            },
            camera,
        }
    }
}
