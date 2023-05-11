use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::Result;
use half::f16;
use inject::DI;
use log::trace;
use rayon::prelude::*;
use scheduler::EventBus;

use crate::asset::Asset;
use crate::texture::format::Grayscale;
use crate::texture::pixel::LumaPixel;
use crate::texture::{Texture, TextureLoadInfo};

pub type HeightmapFormat = Grayscale<f16>;

#[derive(Debug)]
pub struct Heightmap {
    pub image: Texture<HeightmapFormat>,
}

pub struct HeightmapLoadInfo {
    pub path: PathBuf,
}

impl Asset for Heightmap {
    type LoadInfo = HeightmapLoadInfo;

    fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> Result<Self>
    where
        Self: Sized, {
        load_from_image(info, bus)
    }
}

// Normalizes height values in the height map to [-1, 1] based on the most extreme value
fn normalize_height(_width: u32, _height: u32, data: &mut [LumaPixel<f16>]) -> Result<()> {
    trace!("Normalizing heightmap data");
    // Find the largest absolute value in the dataset, and take the absolute value of it.
    let extreme_val = data
        .par_iter()
        .max_by(|lhs, rhs| lhs.to_f32().abs().total_cmp(&rhs.to_f32().abs()))
        .unwrap();
    let extreme_val = f16::from_f32(extreme_val.to_f32().abs());
    let extreme_val_inverse = f16::ONE / extreme_val;
    // Now divide every height value by this extreme value
    data.par_iter_mut().for_each(|value| {
        **value *= extreme_val_inverse;
    });
    Ok(())
}

fn load_from_image(info: HeightmapLoadInfo, bus: EventBus<DI>) -> Result<Heightmap> {
    let tex_info = TextureLoadInfo::FromPath {
        path: info.path,
        cpu_postprocess: Some(normalize_height),
    };
    // Because we only load one image, we can get away with not doing this in another
    // async task through the asset system. This also makes it a bit more ergonomic to
    // access the image inside the heightmap because we don't need to go through two layers of
    // handles.
    let image = Texture::load(tex_info, bus)?;
    Ok(Heightmap {
        image,
    })
}
