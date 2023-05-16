use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::Result;
use gfx::PairedImageView;
use inject::DI;
use phobos::vk;
use scheduler::EventBus;

use crate::asset::Asset;
use crate::texture::format::TextureFormat;

pub mod buffer;
pub mod format;
pub mod pixel;

mod loader;

#[derive(Debug)]
pub struct Texture<F: TextureFormat> {
    pub image: PairedImageView,
    marker: PhantomData<F>,
}

pub enum TextureLoadInfo<F: TextureFormat> {
    FromPath {
        path: PathBuf,
        // Callback to do extra processing on the image data on the CPU.
        cpu_postprocess: Option<fn(u32, u32, &mut [F::Pixel]) -> Result<()>>,
        // Additional usage flags
        usage_flags: Option<vk::ImageUsageFlags>,
    },
    FromRawGpu {
        image: PairedImageView,
    },
}

impl<F: TextureFormat + 'static> Asset for Texture<F> {
    type LoadInfo = TextureLoadInfo<F>;

    fn load(info: Self::LoadInfo, bus: EventBus<DI>) -> Result<Self>
    where
        Self: Sized, {
        loader::load(info, bus)
    }
}
