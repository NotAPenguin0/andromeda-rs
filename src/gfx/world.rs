use std::rc::Rc;

use anyhow::Result;
use poll_promise::Promise;

use crate::gfx::resource::TerrainPlane;
use crate::gfx::AtmosphereInfo;
use crate::math::Rotation;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct FutureWorld {
    #[derivative(Debug = "ignore")]
    pub terrain_mesh: Option<Promise<Result<TerrainPlane>>>,
}

#[derive(Debug)]
pub struct World {
    /// Direction for the sun. This is represented as a rotation for easy editing.
    pub sun_direction: Rotation,
    pub atmosphere: AtmosphereInfo,
    pub terrain_mesh: Option<Rc<TerrainPlane>>,
}
