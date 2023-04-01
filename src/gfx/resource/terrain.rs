use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;

use anyhow::{bail, Result};
use poll_promise::Promise;

use crate::gfx;
use crate::gfx::resource::height_map::HeightMap;
use crate::gfx::resource::normal_map::NormalMap;
use crate::gfx::resource::texture::Texture;
use crate::gfx::resource::TerrainPlane;
use crate::state::world::TerrainOptions;
use crate::thread::io::read_file_async;
use crate::thread::promise::SpawnPromise;
use crate::util::file_type::FileType;

#[derive(Debug)]
pub struct Terrain {
    pub height_map: HeightMap,
    pub normal_map: NormalMap,
    pub diffuse_map: Texture,
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
            FileType::Unknown(extension.to_str().unwrap_or("").to_string())
        }
    }

    /// Loads a terrain from a new heightmap and creates a mesh associated with it.
    pub fn from_new_heightmap<P: AsRef<Path> + Copy + Debug + Send + 'static>(
        heightmap_path: P,
        texture_path: P,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        let ctx2 = ctx.clone();
        let ctx3 = ctx.clone();
        trace!("Loading new heightmap from file: {heightmap_path:?}");
        let mesh = Promise::spawn_blocking(move || {
            let mesh = TerrainPlane::generate(ctx, options)?;
            info!("Terrain mesh from heightmap {heightmap_path:?} generated successfully");
            Ok::<_, anyhow::Error>(mesh)
        });
        let texture = Promise::spawn_blocking(move || {
            Texture::from_file(ctx2, texture_path).block_and_take()
        });
        let height_normal = Promise::spawn_blocking(move || {
            let filetype = Self::detect_filetype(heightmap_path);
            let height = match filetype {
                FileType::Png => {
                    let file_data =
                        read_file_async(heightmap_path.as_ref().to_path_buf()).block_and_take()?;
                    trace!("Heightmap file {heightmap_path:?} loaded from disk ... decoding");
                    let height =
                        HeightMap::from_buffer(filetype, file_data.as_slice(), ctx3.clone())?;
                    info!("Heightmap {heightmap_path:?} loaded successfully");
                    Ok::<_, anyhow::Error>(height)
                }
                FileType::NetCDF => {
                    let height = HeightMap::from_netcdf(heightmap_path, ctx3.clone())?;
                    info!("Heightmap {heightmap_path:?} loaded successfully");
                    Ok(height)
                }
                FileType::Unknown(ext) => {
                    bail!("Heightmap {heightmap_path:?} has unsupported file type {ext}")
                }
            }?;

            info!("Generating normal map from heightmap {heightmap_path:?}");
            let normal = NormalMap::generate_from_heights(ctx3, &height)?;
            info!("Normal map from heightmap {heightmap_path:?} generated successfully");

            Ok((height, normal))
        });

        Promise::spawn_blocking(move || {
            let (height, normal) = height_normal.block_and_take()?;
            let terrain = Terrain {
                height_map: height,
                normal_map: normal,
                diffuse_map: texture.block_and_take()?,
                mesh: mesh.block_and_take()?,
            };
            info!("Terrain {heightmap_path:?} loaded successfully");
            Ok(terrain)
        })
    }

    /// Loads a terrain from an existing heightmap but generates a new mesh.
    pub fn from_new_mesh(
        height_map: HeightMap,
        normal_map: NormalMap,
        diffuse_map: Texture,
        options: TerrainOptions,
        ctx: gfx::SharedContext,
    ) -> Promise<Result<Terrain>> {
        Promise::spawn(move || {
            let mesh = TerrainPlane::generate(ctx, options)?;
            info!("Terrain mesh regenerated successfully");
            Ok(Terrain {
                height_map,
                normal_map,
                diffuse_map,
                mesh,
            })
        })
    }
}
