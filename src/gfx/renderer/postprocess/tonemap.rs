use anyhow::Result;
use phobos as ph;
use phobos::{vk, Allocator, GraphicsCmdBuffer};

use crate::gfx;
use crate::gfx::renderer::statistics::{RendererStatistics, TimedCommandBuffer};
use crate::gfx::util::graph::FrameGraph;
use crate::gfx::util::targets::{RenderTargets, SizeGroup};
use crate::hot_reload::IntoDynamic;

/// This stores all the resources and state needed for the tonemapper to work.
#[allow(dead_code)]
#[derive(Debug)]
pub struct Tonemap {
    ctx: gfx::SharedContext,
    sampler: ph::Sampler,
}

impl Tonemap {
    /// Initialize the tonemapper. Adds a new target with name [`Self::output_name()`] to the
    /// render target database, and creates pipelines and resources.
    pub fn new(ctx: gfx::SharedContext, targets: &mut RenderTargets) -> Result<Self> {
        ph::PipelineBuilder::new("tonemap")
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .cull_mask(vk::CullModeFlags::NONE)
            .depth(false, false, false, vk::CompareOp::ALWAYS)
            .blend_attachment_none()
            .into_dynamic()
            .attach_shader("shaders/src/fullscreen.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/tonemap.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(ctx.shader_reload.clone(), ctx.pipelines.clone())?;

        targets.register_color_target(
            Self::output_name(),
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R8G8B8A8_SRGB,
        )?;

        Ok(Self {
            ctx: ctx.clone(),
            sampler: ph::Sampler::default(ctx.device)?,
        })
    }

    /// Get the name of the output attachment.
    pub fn output_name() -> &'static str {
        "tonemap_output"
    }

    /// Tonemap the input attachment into the tonemapped output attachment.
    ///
    /// # Arguments
    ///
    /// * `graph` - The frame graph to add the tonemapper passes to.
    /// * `input` - The input resource that must be tonemapped. The latest version will be queried from the graph.
    pub fn render<'cb, A: Allocator>(
        &'cb self,
        graph: &mut FrameGraph<'cb, A>,
        input: &ph::VirtualResource,
    ) -> Result<()> {
        let input = graph.latest_version(input)?;
        let output = ph::VirtualResource::image(Self::output_name());
        let pass = ph::PassBuilder::render("tonemap")
            .color_attachment(
                &output,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                }),
            )?
            .sample_image(&input, ph::PipelineStage::FRAGMENT_SHADER)
            .execute_fn(move |mut cmd, _ifc, bindings, stats: &mut RendererStatistics| {
                cmd = cmd
                    .begin_section(stats, "tonemap")?
                    .bind_graphics_pipeline("tonemap")?
                    .full_viewport_scissor()
                    .resolve_and_bind_sampled_image(0, 0, &input, &self.sampler, bindings)?
                    .draw(6, 1, 0, 0)?
                    .end_section(stats, "tonemap")?;
                Ok(cmd)
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}
