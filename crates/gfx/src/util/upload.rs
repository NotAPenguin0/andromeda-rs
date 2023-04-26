use anyhow::Result;
use phobos::domain::Transfer;
use phobos::{vk, DefaultAllocator, Image, IncompleteCmdBuffer, PipelineStage, TransferCmdBuffer};
use poll_promise::Promise;

use crate::util::paired_image_view::PairedImageView;
use crate::util::staging_buffer::StagingBuffer;
use crate::SharedContext;

pub fn upload_image_from_buffer(
    mut ctx: SharedContext,
    data: StagingBuffer,
    width: u32,
    height: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
) -> Promise<Result<PairedImageView>> {
    Promise::spawn_blocking(move || {
        // Explicitly take ownership of staging buffer, this is required or
        // otherwise it is destroyed.
        let data = data;
        let image = Image::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            width,
            height,
            usage | vk::ImageUsageFlags::TRANSFER_DST,
            format,
            vk::SampleCountFlags::TYPE_1,
        )?;
        let image = PairedImageView::new(image, vk::ImageAspectFlags::COLOR)?;

        let cmd = ctx
            .exec
            .on_domain::<Transfer, DefaultAllocator>(None, None)?
            .transition_image(
                &image.view,
                PipelineStage::TOP_OF_PIPE,
                PipelineStage::TRANSFER,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::TRANSFER_WRITE,
            )
            .copy_buffer_to_image(&data.view, &image.view)?
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
        ctx.exec.submit(cmd)?.wait()?;

        Ok(image)
    })
}

pub fn upload_image(
    mut ctx: SharedContext,
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
) -> Promise<Result<PairedImageView>> {
    Promise::spawn_blocking(move || {
        let mut buffer = StagingBuffer::new(&mut ctx, data.len())?;
        buffer.mapped_slice()?.copy_from_slice(data.as_slice());
        upload_image_from_buffer(ctx, buffer, width, height, format, usage).block_and_take()
    })
}
