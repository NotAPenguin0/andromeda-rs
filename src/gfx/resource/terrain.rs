use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use poll_promise::Promise;

use crate::gfx;
use crate::gfx::resource::deferred_delete::DeleteDeferred;
use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world::TerrainOptions;
use crate::thread::io::read_file;
use crate::thread::promise::{spawn_promise, MapPromise, ThenTry, TryMapPromise};

#[derive(Debug)]
pub struct Terrain {
    pub height_map: Arc<HeightMap>,
    pub mesh: TerrainPlane,
}

impl Terrain {
    /// Loads a terrain from a new heightmap and creates a mesh associated with it.
    pub fn from_new_heightmap<P: Into<PathBuf> + Debug + Send + 'static>(
        heightmap_path: P,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        let ctx2 = ctx.clone();
        trace!("Reading new heightmap from file: {:?}", &heightmap_path);
        read_file(heightmap_path)
            .then_try_map(move |buffer| {
                trace!("File I/O for heightmap complete");
                HeightMap::from_buffer(buffer.as_slice(), ctx)
            })
            .then_try(move |heightmap| TerrainPlane::generate(ctx2, options, heightmap.clone()))
            .then_map(move |result| Ok(result?.into()))
    }

    /// Loads a terrain from an existing heightmap but generates a new mesh.
    pub fn from_new_mesh(
        height_map: Arc<HeightMap>,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        spawn_promise(move || {
            let mesh = TerrainPlane::generate(ctx, options, height_map.clone())?;
            Ok::<_, anyhow::Error>((height_map, mesh))
        })
        .then_map(move |result| Ok(result?.into()))
    }
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
