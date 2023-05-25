use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use derivative::Derivative;
use gfx::{PairedImageView, SharedContext};
use glam::UVec2;
use log::warn;
use phobos::fsr2::{FfxDimensions2D, FfxFsr2QualityMode};
use phobos::{vk, DeletionQueue, Image, ImageView, PhysicalResourceBindings};

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

impl From<TargetSize> for UVec2 {
    fn from(value: TargetSize) -> Self {
        Self {
            x: value.width,
            y: value.height,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UpscaleQuality {
    Quality,
    Balanced,
    Performance,
    UltraPerformance,
}

impl From<UpscaleQuality> for FfxFsr2QualityMode {
    fn from(value: UpscaleQuality) -> Self {
        match value {
            UpscaleQuality::Quality => FfxFsr2QualityMode::Quality,
            UpscaleQuality::Balanced => FfxFsr2QualityMode::Balanced,
            UpscaleQuality::Performance => FfxFsr2QualityMode::Performance,
            UpscaleQuality::UltraPerformance => FfxFsr2QualityMode::UltraPerformance,
        }
    }
}

impl From<TargetSize> for FfxDimensions2D {
    fn from(value: TargetSize) -> Self {
        FfxDimensions2D {
            width: value.width,
            height: value.height,
        }
    }
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
    upscale_quality: UpscaleQuality,
}

impl RenderTargets {
    fn get_render_resolution_for_quality(
        &self,
        quality: UpscaleQuality,
    ) -> Result<FfxDimensions2D> {
        let mut fsr2 = self.ctx.device.fsr2_context();
        fsr2.get_render_resolution(quality.into())
    }
}

impl RenderTargets {
    pub fn new(ctx: SharedContext) -> Result<Self> {
        Ok(Self {
            ctx,
            targets: Default::default(),
            deferred_delete: DeletionQueue::new(4),
            output_resolution: TargetSize::default(),
            render_resolution: TargetSize::default(),
            upscale_quality: UpscaleQuality::Quality,
        })
    }

    pub fn set_upscale_quality(&mut self, quality: UpscaleQuality) -> Result<()> {
        self.upscale_quality = quality;
        let resolution = self.get_render_resolution_for_quality(self.upscale_quality)?;
        self.set_render_resolution(resolution.width, resolution.height)
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

        {
            let mut fsr2 = self.ctx.device.fsr2_context();
            fsr2.set_display_resolution(self.output_resolution.into(), None)?;
        }
        // If we change the output resolution we also need to change the render resolution accordingly
        let dims = self.get_render_resolution_for_quality(self.upscale_quality)?;
        self.set_render_resolution(dims.width, dims.height)?;

        Ok(())
    }

    #[allow(dead_code)]
    fn set_render_resolution(&mut self, width: u32, height: u32) -> Result<()> {
        if self.render_resolution.width == width && self.output_resolution.height == height {
            return Ok(());
        }

        if width > self.output_resolution.width || height > self.output_resolution.height {
            bail!("Cannot set render resolution above output resolution");
        }

        self.render_resolution = TargetSize::new(width, height);

        for entry in self.targets.values_mut() {
            if entry.size_group == SizeGroup::RenderResolution {
                Self::resize_target(&mut self.deferred_delete, entry, width, height)?;
            }
        }

        Ok(())
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
            .ok_or_else(|| anyhow!("Target {name} not found"))?;
        Ok(self.size_group_resolution(target.size_group))
    }

    pub fn size_group_resolution(&self, size_group: SizeGroup) -> TargetSize {
        match size_group {
            SizeGroup::RenderResolution => self.render_resolution,
            SizeGroup::OutputResolution => self.output_resolution,
            SizeGroup::Custom(size) => size,
        }
    }

    pub fn get_target_view(&self, name: &str) -> Result<ImageView> {
        Ok(self
            .targets
            .get(name)
            .ok_or_else(|| anyhow!("Target {name} not found"))?
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
