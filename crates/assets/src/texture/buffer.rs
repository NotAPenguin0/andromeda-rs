use crate::texture::pixel::Pixel;

pub struct ImageBuffer<P: Pixel> {
    data: Vec<P::SubPixel>,
}

impl<P: Pixel> ImageBuffer<P> {
    pub fn from_raw(data: Vec<P::SubPixel>) -> Self {
        Self {
            data,
        }
    }

    pub fn into_raw(self) -> Vec<P::SubPixel> {
        self.data
    }

    pub fn into_bytes(self) -> Vec<u8> {
        bytemuck::cast_vec(self.data)
    }

    pub fn as_raw_slice(&self) -> &[P::SubPixel] {
        self.data.as_slice()
    }

    pub fn as_mut_raw_slice(&mut self) -> &mut [P::SubPixel] {
        self.data.as_mut_slice()
    }

    pub fn as_pixel_slice(&self) -> &[P] {
        bytemuck::cast_slice(self.as_raw_slice())
    }

    pub fn as_mut_pixel_slice(&mut self) -> &mut [P] {
        bytemuck::cast_slice_mut(self.as_mut_raw_slice())
    }
}
