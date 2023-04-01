use anyhow::Result;
use phobos::{vk, Buffer, BufferView, MemoryType};

use crate::gfx::SharedContext;

#[derive(Debug)]
pub struct StagingBuffer {
    pub buffer: Buffer,
    pub view: BufferView,
}

impl StagingBuffer {
    pub fn new(ctx: &mut SharedContext, size: impl Into<usize>) -> Result<Self> {
        let buffer = Buffer::new(
            ctx.device.clone(),
            &mut ctx.allocator,
            size.into() as u64,
            vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryType::CpuToGpu,
        )?;
        let view = buffer.view_full();
        Ok(Self {
            buffer,
            view,
        })
    }

    pub fn mapped_slice<T>(&mut self) -> Result<&mut [T]> {
        self.view.mapped_slice()
    }
}
