use crate::util::image::Image;
use crate::util::size::USize;

pub trait ImageProvider {
    fn get_image(&mut self, size: USize) -> Option<Image>;
}
