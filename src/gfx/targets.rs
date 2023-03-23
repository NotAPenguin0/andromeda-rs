use std::collections::HashMap;

use anyhow::{anyhow, Result};
use phobos as ph;
use phobos::vk;

use crate::gfx;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SizeGroup {
    RenderResolution,
    OutputResolution,
    Custom(u32, u32),
}

#[derive(Derivative)]
#[derivative(Debug)]
struct RenderTargetEntry {
    pub size_group: SizeGroup,
    pub target: gfx::PairedImageView,
    #[derivative(Debug = "ignore")]
    pub recreate: Box<dyn Fn(u32, u32) -> Result<gfx::PairedImageView>>,
}

#[derive(Debug)]
pub struct RenderTargets {
    targets: HashMap<String, RenderTargetEntry>,
    deferred_delete: ph::DeletionQueue<gfx::PairedImageView>,
    output_resolution: (u32, u32),
    render_resolution: (u32, u32),
}

impl RenderTargets {
    pub fn new() -> Result<Self> {
        Ok(RenderTargets {
            targets: Default::default(),
            deferred_delete: ph::DeletionQueue::new(4),
            output_resolution: (0, 0),
            render_resolution: (0, 0),
        })
    }

    pub fn set_output_resolution(&mut self, width: u32, height: u32) -> Result<()> {
        // no change
        if self.output_resolution.0 == width && self.output_resolution.1 == height {
            return Ok(());
        }
        self.output_resolution = (width, height);
        for (_, entry) in &mut self.targets {
            if entry.size_group == SizeGroup::OutputResolution {
                Self::resize_target(&mut self.deferred_delete, entry, width, height)?;
            }
        }
        Ok(())
    }

    pub fn set_render_resolution(&mut self, _width: u32, _height: u32) -> Result<()> {
        todo!()
    }

    pub fn register_simple_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        mut ctx: gfx::SharedContext,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
        samples: vk::SampleCountFlags,
    ) -> Result<()> {
        let mut alloc = ctx.allocator.clone();
        self.register_target(name, size, move |width, height| {
            gfx::PairedImageView::new(
                ph::Image::new(ctx.device.clone(), &mut alloc.clone(), width, height, usage, format, samples)?,
                aspect,
            )
        })
    }

    pub fn register_color_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        ctx: gfx::SharedContext,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
    ) -> Result<()> {
        self.register_simple_target(
            name,
            size,
            ctx,
            usage,
            format,
            vk::ImageAspectFlags::COLOR,
            vk::SampleCountFlags::TYPE_1,
        )
    }

    pub fn register_depth_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        ctx: gfx::SharedContext,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
    ) -> Result<()> {
        self.register_simple_target(
            name,
            size,
            ctx,
            usage,
            format,
            vk::ImageAspectFlags::DEPTH,
            vk::SampleCountFlags::TYPE_1,
        )
    }

    pub fn register_multisampled_color_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        ctx: gfx::SharedContext,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        samples: vk::SampleCountFlags,
    ) -> Result<()> {
        self.register_simple_target(name, size, ctx, usage, format, vk::ImageAspectFlags::COLOR, samples)
    }

    pub fn register_multisampled_depth_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        ctx: gfx::SharedContext,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        samples: vk::SampleCountFlags,
    ) -> Result<()> {
        self.register_simple_target(name, size, ctx, usage, format, vk::ImageAspectFlags::DEPTH, samples)
    }

    pub fn register_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        recreate: impl Fn(u32, u32) -> Result<gfx::PairedImageView> + 'static,
    ) -> Result<()> {
        let target = self.create_target(&recreate, size)?;
        self.targets.insert(
            name.into(),
            RenderTargetEntry {
                size_group: size,
                target,
                recreate: Box::new(recreate),
            },
        );

        Ok(())
    }

    pub fn next_frame(&mut self) {
        self.deferred_delete.next_frame();
    }

    pub fn target_size(&self, name: &str) -> Result<(u32, u32)> {
        let target = self.targets.get(name).ok_or(anyhow!("Target {} not found", name))?;
        Ok(self.size_group_resolution(target.size_group))
    }

    pub fn size_group_resolution(&self, size_group: SizeGroup) -> (u32, u32) {
        match size_group {
            SizeGroup::RenderResolution => {
                todo!()
            }
            SizeGroup::OutputResolution => self.output_resolution,
            SizeGroup::Custom(w, h) => (w, h),
        }
    }

    pub fn get_target_view(&self, name: &str) -> Result<ph::ImageView> {
        Ok(self.targets.get(name).ok_or(anyhow!("Target not found: {}", name))?.target.view.clone())
    }

    pub fn bind_targets(&self, bindings: &mut ph::PhysicalResourceBindings) {
        for (name, target) in &self.targets {
            bindings.bind_image(name.clone(), &target.target.view);
        }
    }

    fn create_target(&self, recreate: &impl Fn(u32, u32) -> Result<gfx::PairedImageView>, size: SizeGroup) -> Result<gfx::PairedImageView> {
        let (w, h) = self.size_group_resolution(size);
        recreate.call((w, h))
    }

    fn resize_target(deferred_delete: &mut ph::DeletionQueue<gfx::PairedImageView>, target: &mut RenderTargetEntry, width: u32, height: u32) -> Result<()> {
        // Store new size if this was a custom size group
        if let SizeGroup::Custom(_, _) = target.size_group {
            target.size_group = SizeGroup::Custom(width, height);
        }
        // Allocate new target
        let mut new_target = target.recreate.call((width, height))?;
        // Swap old and new, push old onto deferred delete queue
        std::mem::swap(&mut new_target, &mut target.target);
        deferred_delete.push(new_target);

        Ok(())
    }
}
