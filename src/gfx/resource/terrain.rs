use std::sync::Arc;

use crate::gfx::resource::deferred_delete::DeleteDeferred;
use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::resource::TerrainPlane;

#[derive(Debug)]
pub struct Terrain {
    pub height_map: Arc<HeightMap>,
    pub mesh: TerrainPlane,
}

impl From<(Arc<HeightMap>, TerrainPlane)> for Terrain {
    fn from(value: (Arc<HeightMap>, TerrainPlane)) -> Self {
        Self {
            height_map: value.0,
            mesh: value.1,
        }
    }
}

impl DeleteDeferred for Terrain {}
