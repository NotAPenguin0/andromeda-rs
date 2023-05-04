use std::fmt::Debug;
use std::path::Path;

use anyhow::{bail, Result};
use inject::DI;
use log::{info, trace};
use poll_promise::Promise;
use scheduler::EventBus;
use thread::io::read_file_async;
use thread::promise::SpawnPromise;
use util::FileType;

use crate::{HeightMap, NormalMap, TerrainPlane, Texture};

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
pub struct Terrain {
    pub height_map: HeightMap,
    pub normal_map: NormalMap,
    pub diffuse_map: Texture,
    pub mesh: TerrainPlane,
}

impl Terrain {
    /// Loads a terrain from a new heightmap and creates a mesh associated with it.
    pub fn from_new_heightmap<P: AsRef<Path> + Copy + Debug + Send + 'static>(
        heightmap_path: P,
        texture_path: P,
        options: TerrainOptions,
        bus: EventBus<DI>,
    ) -> Promise<Result<Terrain>> {
        let bus2 = bus.clone();
        let bus3 = bus.clone();
        trace!("Loading new heightmap from file: {heightmap_path:?}");
        let mesh = Promise::spawn_blocking(move || {
            let mesh = TerrainPlane::generate(options, bus)?;
            info!("Terrain mesh from heightmap {heightmap_path:?} generated successfully");
            Ok::<_, anyhow::Error>(mesh)
        });
        let texture = Promise::spawn_blocking(move || {
            Texture::from_file(texture_path, bus2).block_and_take()
        });
        let height_normal = Promise::spawn_blocking(move || {
            let filetype = FileType::from(heightmap_path);
            let height = match filetype {
                FileType::Png => {
                    let file_data =
                        read_file_async(heightmap_path.as_ref().to_path_buf()).block_and_take()?;
                    trace!("Heightmap file {heightmap_path:?} loaded from disk ... decoding");
                    let height =
                        HeightMap::from_buffer(filetype, file_data.as_slice(), bus3.clone())?;
                    info!("Heightmap {heightmap_path:?} loaded successfully");
                    Ok::<_, anyhow::Error>(height)
                }
                FileType::Unknown(ext) => {
                    bail!("Heightmap {heightmap_path:?} has unsupported file type {ext}")
                }
            }?;

            info!("Generating normal map from heightmap {heightmap_path:?}");
            let normal = NormalMap::from_heights(&height, bus3)?;
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
        bus: EventBus<DI>,
    ) -> Promise<Result<Terrain>> {
        Promise::spawn(move || {
            let mesh = TerrainPlane::generate(options, bus)?;
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
