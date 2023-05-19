use std::marker::PhantomData;

use half::f16;
use image::DynamicImage;
use phobos::vk;
use phobos::vk::Format;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::texture::buffer::ImageBuffer;
use crate::texture::pixel::{LumaPixel, Pixel, RgbPixel, RgbaPixel};

pub trait TextureFormat {
    type Pixel: Pixel;
    const VK_FORMAT: vk::Format;

    // Convert a dynamic image into an image buffer with this pixel format.
    // This potentially performs a conversion if the format does not match.
    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel>;
}

#[derive(Debug)]
pub struct Grayscale<T> {
    _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct Rgb<T> {
    _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct Rgba<T> {
    _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct SRgba<T> {
    _marker: PhantomData<T>,
}

impl TextureFormat for Grayscale<u8> {
    type Pixel = LumaPixel<u8>;
    const VK_FORMAT: vk::Format = vk::Format::R8_UNORM;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_luma8();
        ImageBuffer::from_raw(img.into_raw())
    }
}

impl TextureFormat for Grayscale<u16> {
    type Pixel = LumaPixel<u16>;
    const VK_FORMAT: vk::Format = vk::Format::R16_UNORM;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_luma16();
        ImageBuffer::from_raw(img.into_raw())
    }
}

impl TextureFormat for Grayscale<f32> {
    type Pixel = LumaPixel<f32>;
    const VK_FORMAT: vk::Format = vk::Format::R32_SFLOAT;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_luma16();
        let raw = img.into_raw();
        let as_fp = raw.into_par_iter().map(|px| px as f32).collect::<Vec<_>>();
        ImageBuffer::from_raw(as_fp)
    }
}

impl TextureFormat for Grayscale<f16> {
    type Pixel = LumaPixel<f16>;
    const VK_FORMAT: vk::Format = vk::Format::R16_SFLOAT;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_luma16();
        let raw = img.into_raw();
        let as_fp = raw
            .into_par_iter()
            .map(|px| f16::from_f32(px as f32))
            .collect::<Vec<_>>();
        ImageBuffer::from_raw(as_fp)
    }
}

impl TextureFormat for Rgb<u8> {
    type Pixel = RgbPixel<u8>;
    const VK_FORMAT: vk::Format = vk::Format::R8G8B8_UNORM;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_rgb8();
        ImageBuffer::from_raw(img.into_raw())
    }
}

impl TextureFormat for Rgba<u8> {
    type Pixel = RgbaPixel<u8>;
    const VK_FORMAT: vk::Format = vk::Format::R8G8B8A8_UNORM;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_rgba8();
        ImageBuffer::from_raw(img.into_raw())
    }
}

impl TextureFormat for SRgba<u8> {
    type Pixel = RgbaPixel<u8>;
    const VK_FORMAT: vk::Format = vk::Format::R8G8B8A8_SRGB;

    fn from_dynamic_image(img: DynamicImage) -> ImageBuffer<Self::Pixel> {
        let img = img.into_rgba8();
        ImageBuffer::from_raw(img.into_raw())
    }
}
