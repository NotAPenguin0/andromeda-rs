use crate::util::image::Image;
use crate::util::size::USize;

pub struct ImageProvider {
    pub handle: Option<Image>,
    pub size: USize,
}
