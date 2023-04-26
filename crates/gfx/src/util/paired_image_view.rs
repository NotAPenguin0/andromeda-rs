use anyhow::Result;
use phobos::{vk, Image, ImageView};

#[derive(Debug)]
pub struct PairedImageView {
    pub image: Image,
    pub view: ImageView,
}

impl PairedImageView {
    pub fn new(image: Image, aspect: vk::ImageAspectFlags) -> Result<Self> {
        Ok(Self {
            view: image.view(aspect)?,
            image,
        })
    }

    pub fn width(&self) -> u32 {
        self.view.width()
    }

    pub fn height(&self) -> u32 {
        self.view.height()
    }
}
