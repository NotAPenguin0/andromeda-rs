use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use poll_promise::Promise;

use crate::gfx;
use crate::gfx::resource::deferred_delete::DeleteDeferred;
use crate::gfx::resource::height_map::{FileType, HeightMap};
use crate::gfx::resource::normal_map::NormalMap;
use crate::gfx::resource::TerrainPlane;
use crate::gfx::world::TerrainOptions;
use crate::thread::io::{read_file, read_file_async};
use crate::thread::promise::{
    SpawnPromise, ThenMap, ThenTry, ThenTryInto, ThenTryMap, TryJoinPromise,
};

#[derive(Debug)]
pub struct Terrain {
    pub height_map: HeightMap,
    pub normal_map: NormalMap,
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
        let ctx3 = ctx.clone();
        trace!("Loading new heightmap from file: {heightmap_path:?}");
        Promise::spawn_blocking(move || {
            let mesh = TerrainPlane::generate(ctx2.clone(), options)?;
            info!("Terrain mesh from heightmap {heightmap_path:?} generated successfully");
            Ok(mesh)
        })
        .try_join(move || {
            let filetype = Self::detect_filetype(heightmap_path);
            // netcdf cannot be loaded from an in-memory buffer because the library is ass,
            // so we need this ugly match.
            match filetype {
                FileType::Unknown => Promise::spawn_blocking(move || {
                    bail!("Heightmap {heightmap_path:?} has unsupported file type")
                }),
                FileType::NetCDF => Promise::spawn_blocking(move || {
                    let image = HeightMap::load_netcdf(heightmap_path, ctx)?;
                    info!("Heightmap {heightmap_path:?} loaded successfully");
                    Ok(HeightMap {
                        image,
                    })
                }),
                _ => read_file_async(heightmap_path.as_ref().to_path_buf()).then_try_map(
                    move |buffer| {
                        trace!("Heightmap file {heightmap_path:?} loaded from disk ... decoding");
                        let height = HeightMap::from_buffer(filetype, buffer.as_slice(), ctx)?;
                        info!("Heightmap {heightmap_path:?} loaded successfully");
                        Ok(height)
                    },
                ),
            }
            // Once we have the height map we can generate the normal map
            .then_try(move |heightmap| {
                info!("Generating normal map from heightmap {heightmap_path:?}");
                let normal_map = NormalMap::generate_from_heights(ctx3, heightmap)?;
                info!("Normal map from heightmap {heightmap_path:?} generated successfully");
                Ok(normal_map)
            })
            .block_and_take()
        })
        .then_try_into()
    }

    /// Loads a terrain from an existing heightmap but generates a new mesh.
    pub fn from_new_mesh(
        height_map: HeightMap,
        normal_map: NormalMap,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        Promise::spawn(move || {
            let mesh = TerrainPlane::generate(ctx, options)?;
            info!("Terrain mesh regenerated successfully");
            Ok((mesh, (height_map, normal_map)))
        })
        .then_try_into()
    }
}

impl From<(TerrainPlane, (HeightMap, NormalMap))> for Terrain {
    fn from(value: (TerrainPlane, (HeightMap, NormalMap))) -> Self {
        Self {
            mesh: value.0,
            height_map: value.1 .0,
            normal_map: value.1 .1,
        }
    }
}

impl DeleteDeferred for Terrain {}
