use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use poll_promise::Promise;

use crate::gfx;
use crate::gfx::resource::deferred_delete::DeleteDeferred;
use crate::gfx::resource::height_map::{FileType, HeightMap};
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world::TerrainOptions;
use crate::thread::io::{read_file, read_file_async};
use crate::thread::promise::{SpawnPromise, ThenMap, ThenTry, ThenTryInto, ThenTryMap};

#[derive(Debug)]
pub struct Terrain {
    pub height_map: Arc<HeightMap>,
    pub mesh: TerrainPlane,
}

impl Terrain {
    pub fn detect_filetype<P: AsRef<Path>>(path: P) -> FileType {
        let path = path.as_ref();
        let extension = path.extension().unwrap_or(OsStr::new(""));
        if extension == OsStr::new("png") {
            FileType::Png
        } else if extension == OsStr::new("nc") {
            FileType::NetCDF
        } else {
            FileType::Unknown
        }
    }

    /// Loads a terrain from a new heightmap and creates a mesh associated with it.
    pub fn from_new_heightmap<P: AsRef<Path> + Copy + Debug + Send + 'static>(
        heightmap_path: P,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        let ctx2 = ctx.clone();
        trace!("Loading new heightmap from file: {:?}", heightmap_path);
        let filetype = Self::detect_filetype(heightmap_path);
        // netcdf cannot be loaded from an in-memory buffer because the library is ass,
        // so we need this ugly match.
        match filetype {
            FileType::Unknown => Promise::spawn(move || {
                Err(anyhow!("Heightmap {:?} has unsupported file type", heightmap_path))
            }),
            FileType::NetCDF => Promise::spawn(move || {
                let image = HeightMap::load_netcdf(heightmap_path, ctx)?;
                info!("Heightmap {:?} loaded successfully", heightmap_path);
                Ok(Arc::new(HeightMap {
                    image,
                }))
            }),
            _ => {
                read_file_async(heightmap_path.as_ref().to_path_buf()).then_try_map(move |buffer| {
                    trace!("Heightmap file {:?} loaded from disk ... decoding", heightmap_path);
                    let height = HeightMap::from_buffer(filetype, buffer.as_slice(), ctx)?;
                    info!("Heightmap {:?} loaded successfully", heightmap_path);
                    Ok(height)
                })
            }
        }
        .then_try(move |heightmap| {
            let mesh = TerrainPlane::generate(ctx2, options, heightmap.clone())?;
            info!("Terrain from heightmap {:?} generated successfully", heightmap_path);
            Ok(mesh)
        })
        .then_try_into()
    }

    /// Loads a terrain from an existing heightmap but generates a new mesh.
    pub fn from_new_mesh(
        height_map: Arc<HeightMap>,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        Promise::spawn(move || {
            let mesh = TerrainPlane::generate(ctx, options, height_map.clone())?;
            info!("Terrain mesh regenerated successfully");
            Ok((height_map, mesh))
        })
        .then_try_into()
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
