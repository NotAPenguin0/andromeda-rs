use anyhow::Result;
use phobos as ph;
use phobos::vk;

#[derive(Debug)]
pub struct PairedImageView {
    pub image: ph::Image,
    pub view: ph::ImageView,
}

impl PairedImageView {
    pub fn new(image: ph::Image, aspect: vk::ImageAspectFlags) -> Result<Self> {
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
