use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;

use anyhow::{anyhow, bail, Result};
use half::f16;
use ph::vk;
use phobos::prelude as ph;
use rayon::prelude::*;

use crate::gfx;
use crate::gfx::util::staging_buffer::StagingBuffer;
use crate::gfx::util::upload::upload_image_from_buffer;
use crate::gfx::PairedImageView;
use crate::thread::SendSyncPtr;
use crate::util::file_type::FileType;

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
        *value = *value * extreme_val_inverse;
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
    fn load_image(buffer: &[u8], mut ctx: gfx::SharedContext) -> Result<PairedImageView> {
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

    pub fn from_netcdf<P: AsRef<Path> + Debug>(
        path: P,
        mut ctx: gfx::SharedContext,
    ) -> Result<Self> {
        let file = netcdf::open(&path)?;
        trace!("netcdf: opened file {path:?}");
        // Identify the variable name used for heightmap data. We'll just pick the first 2D variable
        let var = file
            .variables()
            .filter(|var| var.dimensions().len() == 2)
            .next();
        let var =
            var.ok_or(anyhow!("netcdf file {path:?} does not appear to contain any 2D data"))?;
        trace!("netcdf: identified heightmap variable: {}", var.name());
        trace!("netcdf: heightmap variable type is {:?}", var.vartype());
        // netcdf data has width and height switched, cool.
        let width = var.dimensions().get(1).unwrap().len();
        let height = var.dimensions().get(0).unwrap().len();
        trace!("netcdf: heightmap size is {}x{}", width, height);

        let mut staging_buffer =
            StagingBuffer::new(&mut ctx, width * height * std::mem::size_of::<f16>())?;
        let data_slice = staging_buffer.mapped_slice::<u8>()?;
        var.raw_values(data_slice, ..)?;
        // Get the data slice as the correct type, then reverse it (because the heightmap loads upside down)
        let data_slice = staging_buffer.mapped_slice::<i16>()?;
        // Since our data is now in contiguous rows, we split in each half of the image first
        trace!("netcdf: flipping image vertically");
        flip_image_vertical(data_slice, width);

        // SAFETY: Each invocation of for_each accesses a different offset of this pointer, so
        // we can safely iterate over it in parallel.
        let src_ptr = unsafe { SendSyncPtr::new(data_slice.as_ptr()) };
        let f16_slice = staging_buffer.mapped_slice::<f16>()?;
        trace!("netcdf: converting data to floating point");
        f16_slice
            .par_iter_mut()
            .enumerate()
            .for_each(|(idx, as_f16)| {
                // SAFETY: src_ptr refers to the same slice as before, and the size of a u16 is the same as the size
                // of an f16, so all these accesses are valid
                // As for safety of the get() call, see the comment above
                *as_f16 = f16::from_f32(unsafe { *src_ptr.get().offset(idx as isize) } as f32);
            });
        // Normalize all height values to [-1, 1]
        normalize_height(f16_slice);
        let image = upload_image_from_buffer(
            ctx,
            staging_buffer,
            width as u32,
            height as u32,
            vk::Format::R16_SFLOAT,
            vk::ImageUsageFlags::SAMPLED,
        )
        .block_and_take()?;
        Ok(Self {
            image,
        })
    }

    pub fn from_buffer(ty: FileType, buffer: &[u8], ctx: gfx::SharedContext) -> Result<Self> {
        let image = match ty {
            FileType::Png => Self::load_image(buffer, ctx),
            FileType::NetCDF => bail!("netcdf: cannot load from in-memory buffer"),
            FileType::Unknown(ext) => bail!("Unrecognized file type {ext}."),
        }?;

        Ok(Self {
            image,
        })
    }
}
