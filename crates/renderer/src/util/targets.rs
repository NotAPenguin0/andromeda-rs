use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use derivative::Derivative;
use gfx::{PairedImageView, SharedContext};
use gui::util::image_provider::ImageProvider;
use gui::util::size::USize;
use inject::DI;
use phobos::{vk, DeletionQueue, Image, ImageView, PhysicalResourceBindings};
use scheduler::EventBus;

use crate::ui_integration::UIIntegration;

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct TargetSize {
    pub width: u32,
    pub height: u32,
}

impl TargetSize {
    pub fn new(width: u32, height: u32) -> Self {
        TargetSize {
            width,
            height,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SizeGroup {
    RenderResolution,
    OutputResolution,
    Custom(TargetSize),
}

#[derive(Derivative)]
#[derivative(Debug)]
struct RenderTargetEntry {
    pub size_group: SizeGroup,
    pub target: PairedImageView,
    #[derivative(Debug = "ignore")]
    pub recreate: Box<dyn Fn(TargetSize) -> Result<PairedImageView>>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct RenderTargets {
    ctx: SharedContext,
    targets: HashMap<String, RenderTargetEntry>,
    deferred_delete: DeletionQueue<PairedImageView>,
    output_resolution: TargetSize,
    render_resolution: TargetSize,
}

impl RenderTargets {
    pub fn new(ctx: SharedContext) -> Result<Self> {
        Ok(Self {
            ctx,
            targets: Default::default(),
            deferred_delete: DeletionQueue::new(4),
            output_resolution: TargetSize::default(),
            render_resolution: TargetSize::default(),
        })
    }

    pub fn set_output_resolution(&mut self, width: u32, height: u32) -> Result<()> {
        // no change
        if self.output_resolution.width == width && self.output_resolution.height == height {
            return Ok(());
        }
        self.output_resolution = TargetSize::new(width, height);
        for entry in self.targets.values_mut() {
            if entry.size_group == SizeGroup::OutputResolution {
                Self::resize_target(&mut self.deferred_delete, entry, width, height)?;
            }
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn set_render_resolution(&mut self, _width: u32, _height: u32) -> Result<()> {
        todo!()
    }

    pub fn register_simple_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        aspect: vk::ImageAspectFlags,
        samples: vk::SampleCountFlags,
    ) -> Result<()> {
        let alloc = self.ctx.allocator.clone();
        let device = self.ctx.device.clone();
        self.register_target(name, size, move |size| {
            PairedImageView::new(
                Image::new(
                    device.clone(),
                    &mut alloc.clone(),
                    size.width,
                    size.height,
                    usage,
                    format,
                    samples,
                )?,
                aspect,
            )
        })
    }

    pub fn register_color_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
    ) -> Result<()> {
        self.register_simple_target(
            name,
            size,
            usage,
            format,
            vk::ImageAspectFlags::COLOR,
            vk::SampleCountFlags::TYPE_1,
        )
    }

    #[allow(dead_code)]
    pub fn register_depth_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
    ) -> Result<()> {
        self.register_simple_target(
            name,
            size,
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
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        samples: vk::SampleCountFlags,
    ) -> Result<()> {
        self.register_simple_target(name, size, usage, format, vk::ImageAspectFlags::COLOR, samples)
    }

    pub fn register_multisampled_depth_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        samples: vk::SampleCountFlags,
    ) -> Result<()> {
        self.register_simple_target(name, size, usage, format, vk::ImageAspectFlags::DEPTH, samples)
    }

    pub fn register_target(
        &mut self,
        name: impl Into<String>,
        size: SizeGroup,
        recreate: impl Fn(TargetSize) -> Result<PairedImageView> + 'static,
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

    #[allow(dead_code)]
    pub fn target_size(&self, name: &str) -> Result<TargetSize> {
        let target = self
            .targets
            .get(name)
            .ok_or(anyhow!("Target {name} not found"))?;
        Ok(self.size_group_resolution(target.size_group))
    }

    pub fn size_group_resolution(&self, size_group: SizeGroup) -> TargetSize {
        match size_group {
            SizeGroup::RenderResolution => {
                todo!()
            }
            SizeGroup::OutputResolution => self.output_resolution,
            SizeGroup::Custom(size) => size,
        }
    }

    pub fn get_target_view(&self, name: &str) -> Result<ImageView> {
        Ok(self
            .targets
            .get(name)
            .ok_or(anyhow!("Target {name} not found"))?
            .target
            .view
            .clone())
    }

    pub fn bind_targets(&self, bindings: &mut PhysicalResourceBindings) {
        for (name, target) in &self.targets {
            bindings.bind_image(name.clone(), &target.target.view);
        }
    }

    fn create_target(
        &self,
        recreate: &impl Fn(TargetSize) -> Result<PairedImageView>,
        size: SizeGroup,
    ) -> Result<PairedImageView> {
        let size = self.size_group_resolution(size);
        recreate.call((size,))
    }

    fn resize_target(
        deferred_delete: &mut DeletionQueue<PairedImageView>,
        target: &mut RenderTargetEntry,
        width: u32,
        height: u32,
    ) -> Result<()> {
        // Store new size if this was a custom size group
        if let SizeGroup::Custom(size) = target.size_group {
            target.size_group = SizeGroup::Custom(size);
        }
        // Allocate new target
        let mut new_target = target.recreate.call((TargetSize::new(width, height),))?;
        // Swap old and new, push old onto deferred delete queue
        std::mem::swap(&mut new_target, &mut target.target);
        deferred_delete.push(new_target);

        Ok(())
    }
}
