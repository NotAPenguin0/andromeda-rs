use anyhow::Result;
use phobos::domain::Transfer;
use phobos::{
    vk, Buffer, Image, IncompleteCmdBuffer, MemoryType, PipelineStage, TransferCmdBuffer,
};
use poll_promise::Promise;

use crate::gfx::{PairedImageView, SharedContext};

pub fn upload_image(
    mut ctx: SharedContext,
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: vk::Format,
    usage: vk::ImageUsageFlags,
) -> Promise<Result<PairedImageView>> {
    Promise::spawn_blocking(move || {
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

        let buffer = Buffer::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            data.len() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryType::CpuToGpu,
        )?;
        let mut view = buffer.view_full();
        view.mapped_slice()?.copy_from_slice(data.as_slice());

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
            .copy_buffer_to_image(&view, &image.view)?
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
