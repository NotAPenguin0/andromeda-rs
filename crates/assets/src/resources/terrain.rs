use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use inject::DI;
use scheduler::EventBus;

use crate::asset::Asset;
use crate::handle::Handle;
use crate::storage::AssetStorage;
use crate::texture::format::SRgba;
use crate::texture::{Texture, TextureLoadInfo};
use crate::{Heightmap, HeightmapLoadInfo, NormalMap, NormalMapLoadInfo, TerrainPlane};

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
    pub height_map: Handle<Heightmap>,
    pub normal_map: Handle<NormalMap>,
    pub diffuse_map: Handle<Texture<SRgba<u8>>>,
    pub mesh: Handle<TerrainPlane>,
}

impl Terrain {
    pub fn with_if_ready<F, R>(&self, assets: &AssetStorage, f: F) -> Option<R>
    where
        F: FnOnce(&Heightmap, &NormalMap, &Texture<SRgba<u8>>, &TerrainPlane) -> R, {
        assets
            .with_if_ready(self.height_map, |heights| {
                assets.with_if_ready(self.normal_map, |normals| {
                    assets.with_if_ready(self.diffuse_map, |diffuse| {
                        assets.with_if_ready(self.mesh, |mesh| f(heights, normals, diffuse, mesh))
                    })
                })
            })
            .flatten()
            .flatten()
            .flatten()
    }

    pub fn with_when_ready<F, R>(&self, bus: &EventBus<DI>, f: F) -> Option<R>
    where
        F: FnOnce(&Heightmap, &NormalMap, &Texture<SRgba<u8>>, &TerrainPlane) -> R, {
        let di = bus.data().read().unwrap();
        let assets = di.get::<AssetStorage>().unwrap();
        assets
            .with_when_ready(self.height_map, |heights| {
                assets.with_when_ready(self.normal_map, |normals| {
                    assets.with_when_ready(self.diffuse_map, |diffuse| {
                        assets.with_when_ready(self.mesh, |mesh| f(heights, normals, diffuse, mesh))
                    })
                })
            })
            .flatten()
            .flatten()
            .flatten()
    }
}

pub enum TerrainLoadInfo {
    // Create a new terrain
    FromHeightmap {
        height_path: PathBuf,
        texture_path: PathBuf,
        options: TerrainOptions,
    },
    // Only recreate the mesh associated with the terrain
    FromNewMesh {
        old: Handle<Terrain>,
        options: TerrainOptions,
    },
}

impl Asset for Terrain {
    type LoadInfo = TerrainLoadInfo;

    fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> Result<Self>
    where
        Self: Sized, {
        match info {
            TerrainLoadInfo::FromHeightmap {
                height_path,
                texture_path,
                options,
            } => load_from_files(height_path, texture_path, options, bus),
            TerrainLoadInfo::FromNewMesh {
                old,
                options,
            } => load_new_mesh(old, options, bus),
        }
    }
}

fn load_from_files(
    heightmap_path: PathBuf,
    texture_path: PathBuf,
    options: TerrainOptions,
    bus: EventBus<DI>,
) -> Result<Terrain> {
    let di = bus.data().read().unwrap();
    let assets = di.get::<AssetStorage>().unwrap();
    let heights = assets.load(HeightmapLoadInfo {
        path: heightmap_path,
    });

    let texture: Handle<Texture<SRgba<u8>>> = assets.load(TextureLoadInfo::FromPath {
        path: texture_path,
        cpu_postprocess: None,
    });
    let normal_map = assets.load(NormalMapLoadInfo::FromHeightmap {
        heights,
    });
    let mesh = assets.load(options);
    Ok(Terrain {
        height_map: heights,
        normal_map,
        diffuse_map: texture,
        mesh,
    })
}

fn load_new_mesh(
    old: Handle<Terrain>,
    options: TerrainOptions,
    bus: EventBus<DI>,
) -> Result<Terrain> {
    let di = bus.data().read().unwrap();
    let assets = di.get::<AssetStorage>().unwrap();
    assets
        .with_when_ready(old, |terrain| {
            let di = bus.data().read().unwrap();
            let assets = di.get::<AssetStorage>().unwrap();
            let mesh = assets.load(options);
            Ok(Terrain {
                height_map: terrain.height_map,
                normal_map: terrain.normal_map,
                diffuse_map: terrain.diffuse_map,
                mesh,
            })
        })
        .ok_or_else(|| anyhow!("error creating terrain from old terrain: old terrain is invalid"))?
}
