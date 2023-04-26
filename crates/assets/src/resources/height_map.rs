use std::fmt::Debug;
use std::io::Cursor;

use anyhow::{bail, Result};
use gfx::util::paired_image_view::PairedImageView;
use gfx::util::staging_buffer::StagingBuffer;
use gfx::util::upload::upload_image_from_buffer;
use gfx::SharedContext;
use half::f16;
use inject::DI;
use log::trace;
use phobos::vk;
use rayon::prelude::*;
use scheduler::EventBus;
use util::FileType;

// Normalizes height values in the height map to [-1, 1] based on the most extreme value
fn normalize_height(data: &mut [f16]) {
    trace!("Normalizing heightmap data");
    // Find the largest absolute value in the dataset, and take the absolute value of it.
    let extreme_val = data
        .par_iter()
        .max_by(|lhs, rhs| lhs.to_f32().abs().total_cmp(&rhs.to_f32().abs()))
        .unwrap();
    let extreme_val = f16::from_f32(extreme_val.to_f32().abs());
    let extreme_val_inverse = f16::ONE / extreme_val;
    // Now divide every height value by this extreme value
    data.par_iter_mut().for_each(|value| {
        *value *= extreme_val_inverse;
    });
}

fn flip_image_vertical<T: Send>(data: &mut [T], row_length: impl Into<usize>) {
    let width = row_length.into();
    let (top, bottom) = data.split_at_mut(data.len() / 2);
    // Each chunk returned by this iterator is one row, we zip both halves together and swap each element.
    // Note that for the second iterator to go bottom to top we have to reverse it.
    top.par_chunks_mut(width)
        .zip(bottom.par_chunks_mut(width).rev())
        .for_each(|(top_row, bottom_row)| {
            top_row
                .par_iter_mut()
                .zip(bottom_row.par_iter_mut())
                .for_each(|(top, bottom)| {
                    std::mem::swap(top, bottom);
                });
        });
}

#[derive(Debug)]
pub struct HeightMap {
    pub image: PairedImageView,
}

impl HeightMap {
    fn load_image(buffer: &[u8], bus: EventBus<DI>) -> Result<PairedImageView> {
        let mut ctx = bus
            .data()
            .read()
            .unwrap()
            .get::<SharedContext>()
            .cloned()
            .unwrap();

        let reader = image::io::Reader::new(Cursor::new(buffer)).with_guessed_format()?;
        let image = reader.decode()?;
        let width = image.width() as usize;
        let height = image.height() as usize;
        trace!("heightmap size is {width}x{height}");
        trace!("heightmap color type is {:?}", image.color());
        let image = image.into_luma16();
        let mut staging_buffer =
            StagingBuffer::new(&mut ctx, width * height * std::mem::size_of::<f16>())?;
        let data_slice = staging_buffer.mapped_slice::<f16>()?;
        data_slice
            .par_iter_mut()
            .zip(image.as_raw().as_slice().par_iter())
            .for_each(|(dst, src)| {
                *dst = f16::from_f32(*src as f32);
            });
        normalize_height(data_slice);
        trace!("Uploading heightmap data");
        let image = upload_image_from_buffer(
            ctx,
            staging_buffer,
            width as u32,
            height as u32,
            vk::Format::R16_SFLOAT,
            vk::ImageUsageFlags::SAMPLED,
        )
        .block_and_take()?;
        Ok(image)
    }

    pub fn from_buffer(ty: FileType, buffer: &[u8], bus: EventBus<DI>) -> Result<Self> {
        let image = match ty {
            FileType::Png => Self::load_image(buffer, bus),
            FileType::Unknown(ext) => bail!("Unrecognized file type {ext}."),
        }?;

        Ok(Self {
            image,
        })
    }
}
