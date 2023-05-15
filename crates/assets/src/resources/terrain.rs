use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use glam::Vec2;
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

impl TerrainOptions {
    #[inline]
    pub fn patch_coords(&self, patch_x: u32, patch_y: u32) -> Vec2 {
        let resolution = self.patch_resolution as f32;
        let patch_size = self.horizontal_scale / resolution;
        let x = patch_x as f32;
        let y = patch_y as f32;
        Vec2::new(
            x * patch_size + patch_size / 2.0 - resolution * patch_size / 2.0,
            y * patch_size + patch_size / 2.0 - resolution * patch_size / 2.0,
        )
    }

    #[inline]
    pub fn patch_uvs(&self, patch_x: u32, patch_y: u32) -> Vec2 {
        let resolution = self.patch_resolution as f32;
        let x = patch_x as f32;
        let y = patch_y as f32;
        Vec2::new(x / resolution, y / resolution)
    }

    /// Returns the smallest x coordinate, this has uv.x == 0
    #[inline]
    pub fn min_x(&self) -> f32 {
        self.patch_coords(0, 0).x
    }

    // Returns the largest x coordinate, this has uv.x == 1
    #[inline]
    pub fn max_x(&self) -> f32 {
        self.patch_coords(self.patch_resolution - 1, 0).x
    }

    /// Returns the smallest y coordinate, this has uv.y == 0
    #[inline]
    pub fn min_y(&self) -> f32 {
        self.patch_coords(0, 0).y
    }

    // Returns the largest y coordinate, this has uv.y == 1
    #[inline]
    pub fn max_y(&self) -> f32 {
        self.patch_coords(0, self.patch_resolution - 1).y
    }
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
