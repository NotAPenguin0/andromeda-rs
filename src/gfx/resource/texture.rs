use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;

use anyhow::Result;
use image::ImageFormat;
use phobos::vk;
use poll_promise::Promise;

use crate::gfx::util::upload::upload_image;
use crate::gfx::{PairedImageView, SharedContext};
use crate::thread::io::{read_file, read_file_async};
use crate::thread::promise::ThenTryMap;

#[derive(Debug)]
pub struct Texture {
    pub image: PairedImageView,
}

impl Texture {
    pub fn from_file<P: AsRef<Path> + Debug + Send + 'static>(
        ctx: SharedContext,
        path: P,
    ) -> Promise<Result<Self>> {
        trace!("Loading texture {path:?}");
        read_file_async(path.as_ref().to_path_buf()).then_try_map(move |buffer| {
            let mut reader = image::io::Reader::new(Cursor::new(buffer));
            reader.set_format(ImageFormat::Png);
            let image = reader.decode()?;
            let width = image.width();
            let height = image.height();
            trace!("png: texture size is {width}x{height}");
            trace!("png: texture color type is {:?}", image.color());
            let image = image.into_rgba8();
            let image = upload_image(
                ctx,
                image.into_raw(),
                width,
                height,
                vk::Format::R8G8B8A8_SRGB,
                vk::ImageUsageFlags::SAMPLED,
            )
            .block_and_take()?;
            info!("Successfully loaded texture {path:?}");
            Ok(Self {
                image,
            })
        })
    }
}
