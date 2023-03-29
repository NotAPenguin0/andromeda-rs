use std::ffi::OsStr;
use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use half::f16;
use image::ImageFormat;
use ndarray::Ix2;
use ph::traits::*;
use ph::vk;
use phobos::domain::Transfer;
use phobos::vk::PipelineStageFlags;
use phobos::{prelude as ph, MemoryType, PipelineStage};
use poll_promise::Promise;
use rayon::prelude::*;

use crate::gfx;
use crate::gfx::resource::deferred_delete::{DeferredDelete, DeleteDeferred};
use crate::gfx::PairedImageView;
use crate::thread::SendSyncPtr;

#[derive(Debug)]
pub struct HeightMap {
    pub image: PairedImageView,
}

pub enum FileType {
    Png,
    NetCDF,
    Unknown,
}

impl HeightMap {
    fn alloc_staging_buffer(
        ctx: &mut gfx::SharedContext,
        width: usize,
        height: usize,
    ) -> Result<ph::Buffer> {
        ph::Buffer::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            (std::mem::size_of::<f16>() * width * height) as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryType::CpuToGpu,
        )
    }

    fn upload(
        mut ctx: gfx::SharedContext,
        buffer: &ph::BufferView,
        width: u32,
        height: u32,
    ) -> Result<PairedImageView> {
        trace!("Uploading heightmap data");
        let image = ph::Image::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            width as u32,
            height as u32,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            vk::Format::R16_SFLOAT,
            vk::SampleCountFlags::TYPE_1,
        )?;
        let image = PairedImageView::new(image, vk::ImageAspectFlags::COLOR)?;

        // Copy buffer to image
        let cmd = ctx
            .exec
            .on_domain::<Transfer>(None, None)?
            .transition_image(
                &image.view,
                PipelineStage::TOP_OF_PIPE,
                PipelineStage::TRANSFER,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::TRANSFER_WRITE,
            )
            .copy_buffer_to_image(&buffer, &image.view)?
            .transition_image(
                &image.view,
                PipelineStage::TRANSFER,
                PipelineStage::BOTTOM_OF_PIPE,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::AccessFlags2::NONE,
            )
            .finish()?;
        // We are inside a rayon task, so this is the best option for us
        ctx.exec.submit(cmd)?.wait_and_yield()?;
        Ok(image)
    }

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

    fn load_png(buffer: &[u8], mut ctx: gfx::SharedContext) -> Result<PairedImageView> {
        let mut reader = image::io::Reader::new(Cursor::new(buffer));
        reader.set_format(ImageFormat::Png);
        let image = reader.decode()?;
        let width = image.width();
        let height = image.height();
        trace!("png: heightmap size is {}x{}", width, height);
        trace!("png: heightmap color type is {:?}", image.color());
        let image = image.into_luma16();
        let staging_buffer = Self::alloc_staging_buffer(&mut ctx, width as usize, height as usize)?;
        let mut staging_view = staging_buffer.view_full();
        let data_slice = staging_view.mapped_slice::<f16>()?;
        data_slice
            .par_iter_mut()
            .zip(image.as_raw().as_slice().par_iter())
            .for_each(|(dst, src)| {
                *dst = f16::from_f32(*src as f32);
            });
        Self::normalize_height(data_slice);
        let image = Self::upload(ctx, &staging_view, width, height)?;
        Ok(image)
    }

    pub fn load_netcdf<P: AsRef<Path> + Debug>(
        path: P,
        mut ctx: gfx::SharedContext,
    ) -> Result<PairedImageView> {
        let file = netcdf::open(&path)?;
        trace!("netcdf: opened file {:?}", path);
        // Identify the variable name used for heightmap data. We'll just pick the first 2D variable
        let var = file
            .variables()
            .filter(|var| var.dimensions().len() == 2)
            .next();
        let var =
            var.ok_or(anyhow!("netcdf file {:?} does not appear to contain any 2D data", path))?;
        trace!("netcdf: identified heightmap variable: {}", var.name());
        trace!("netcdf: heightmap variable type is {:?}", var.vartype());
        // netcdf data has width and height switched, cool.
        let width = var.dimensions().get(1).unwrap().len();
        let height = var.dimensions().get(0).unwrap().len();
        trace!("netcdf: heightmap size is {}x{}", width, height);

        let staging_buffer = Self::alloc_staging_buffer(&mut ctx, width, height)?;
        let mut staging_view = staging_buffer.view_full();
        let data_slice = staging_view.mapped_slice::<u8>()?;
        var.raw_values(data_slice, ..)?;
        // Get the data slice as the correct type, then reverse it (because the heightmap loads upside down)
        let data_slice = staging_view.mapped_slice::<i16>()?;
        // Since our data is now in contiguous rows, we split in each half of the image first
        let (top, bottom) = data_slice.split_at_mut(data_slice.len() / 2);
        // Each chunk returned by this iterator is one row, we zip both halves together and swap each element.
        // Note that for the second iterator to go bottom to top we have to reverse it.
        trace!("netcdf: flipping image vertically");
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

        // SAFETY: Each invocation of for_each accesses a different offset of this pointer, so
        // we can safely iterate over it in parallel.
        let src_ptr = unsafe { SendSyncPtr::new(data_slice.as_ptr()) };
        let f16_slice = staging_view.mapped_slice::<f16>()?;
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
        Self::normalize_height(f16_slice);
        let image = Self::upload(ctx, &staging_view, width as u32, height as u32)?;
        // Cleanup is performed already, we're done.
        Ok(image)
    }

    pub fn from_buffer(
        ty: FileType,
        buffer: &[u8],
        mut ctx: gfx::SharedContext,
    ) -> Result<Arc<Self>> {
        let image = match ty {
            FileType::Png => Self::load_png(buffer, ctx),
            FileType::NetCDF => Err(anyhow!("netcdf: cannot load from in-memory buffer")),
            FileType::Unknown => Err(anyhow!("Unrecognized file type.")),
        }?;

        Ok(Arc::new(Self {
            image,
        }))
    }
}

impl DeleteDeferred for HeightMap {}
impl DeleteDeferred for Arc<HeightMap> {}
