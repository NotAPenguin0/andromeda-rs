use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;

use anyhow::Result;
use gfx::util::paired_image_view::PairedImageView;
use gfx::util::upload::upload_image;
use gfx::SharedContext;
use inject::DI;
use log::{info, trace};
use phobos::vk;
use poll_promise::Promise;
use scheduler::EventBus;
use thread::io::read_file_async;
use thread::promise::ThenTryMap;

#[derive(Debug)]
pub struct Texture {
    pub image: PairedImageView,
}

impl Texture {
    pub fn from_file<P: AsRef<Path> + Debug + Send + 'static>(
        path: P,
        bus: EventBus<DI>,
    ) -> Promise<Result<Self>> {
        let ctx = bus
            .data()
            .read()
            .unwrap()
            .get::<SharedContext>()
            .cloned()
            .unwrap();
        trace!("Loading texture {path:?}");
        read_file_async(path.as_ref().to_path_buf()).then_try_map(move |buffer| {
            let reader = image::io::Reader::new(Cursor::new(buffer)).with_guessed_format()?;
            let image = reader.decode()?;
            let width = image.width();
            let height = image.height();
            trace!("texture size is {width}x{height}");
            trace!("texture color type is {:?}", image.color());
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
