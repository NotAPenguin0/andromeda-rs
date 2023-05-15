use std::io::Cursor;
use std::marker::PhantomData;
use std::path::PathBuf;

use anyhow::Result;
use gfx::{upload_image, SharedContext};
use inject::DI;
use log::{info, trace};
use phobos::vk;
use scheduler::EventBus;
use thread::io::read_file;

use crate::texture::format::TextureFormat;
use crate::texture::{Texture, TextureLoadInfo};

pub(crate) fn load<F: TextureFormat>(
    info: TextureLoadInfo<F>,
    bus: EventBus<DI>,
) -> Result<Texture<F>> {
    match info {
        TextureLoadInfo::FromPath {
            path,
            cpu_postprocess,
        } => load_from_file(path, cpu_postprocess, bus),
        TextureLoadInfo::FromRawGpu {
            image,
        } => Ok(Texture {
            image,
            marker: PhantomData,
        }),
    }
}

fn load_from_file<F: TextureFormat>(
    path: PathBuf,
    cpu_postprocess: Option<fn(u32, u32, &mut [F::Pixel]) -> Result<()>>,
    bus: EventBus<DI>,
) -> Result<Texture<F>> {
    let ctx = bus
        .data()
        .read()
        .unwrap()
        .get::<SharedContext>()
        .cloned()
        .unwrap();

    trace!("Loading texture {path:?}");
    let buffer = read_file(path.clone())?;
    let reader = image::io::Reader::new(Cursor::new(buffer)).with_guessed_format()?;
    let image = reader.decode()?;
    let width = image.width();
    let height = image.height();
    trace!("texture size is {width}x{height}");
    trace!("texture color type is {:?}", image.color());
    let mut data = F::from_dynamic_image(image);
    if let Some(f) = cpu_postprocess {
        f(width, height, data.as_mut_pixel_slice())?;
    }
    let image = upload_image(
        ctx,
        data.as_raw_slice(),
        width,
        height,
        F::VK_FORMAT,
        vk::ImageUsageFlags::SAMPLED,
    )?;
    info!("Successfully loaded texture {path:?}");
    Ok(Texture {
        image,
        marker: PhantomData,
    })
}
