use crate::app::RootActorSystem;
use crate::gfx;
use crate::hot_reload::IntoDynamic;

use phobos as ph;
use phobos::{GraphicsCmdBuffer, vk};

use anyhow::Result;

#[derive(Debug)]
pub struct Tonemap {
    ctx: gfx::SharedContext,
    sampler: ph::Sampler,
}

impl Tonemap {
    pub fn new(ctx: gfx::SharedContext, actors: &RootActorSystem, targets: &mut gfx::RenderTargets) -> Result<Self> {
        ph::PipelineBuilder::new("tonemap")
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .cull_mask(vk::CullModeFlags::NONE)
            .depth(false, false, false, vk::CompareOp::ALWAYS)
            .blend_attachment_none()
            .into_dynamic()
            .attach_shader("shaders/src/fullscreen.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/tonemap.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(actors.shader_reload.clone(), ctx.pipelines.clone())?;

        targets.register_color_target(
            Self::output_name(),
            gfx::SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R8G8B8A8_SRGB
        )?;

        Ok(Self {
            ctx: ctx.clone(),
            sampler: ph::Sampler::default(ctx.device)?
        })
    }

    pub fn output_name() -> &'static str {
        "tonemap_output"
    }

    pub fn render<'s: 'e + 'q, 'e, 'q>(&'s self, input: ph::VirtualResource, graph: &mut gfx::FrameGraph<'e, 'q>) -> Result<()> {
        let input = graph.latest_version(input)?;
        let output = ph::VirtualResource::image(Self::output_name());
        let pass = ph::PassBuilder::render("tonemap")
            .color_attachment(
                output,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0]}))?
            .sample_image(input.clone(), ph::PipelineStage::FRAGMENT_SHADER)
            .execute(move |mut cmd, ifc, bindings| {
                let set = ph::DescriptorSetBuilder::new()
                    .resolve_and_bind_sampled_image(0, input.clone(), &self.sampler, bindings)?
                    .build();
                cmd = cmd
                    .bind_graphics_pipeline("tonemap", self.ctx.pipelines.clone())?
                    .full_viewport_scissor()
                    .bind_new_descriptor_set(0, self.ctx.descriptors.clone(), set)?
                    .draw(6, 1, 0, 0);
                Ok(cmd)
            })
            .build();
        graph.add_pass(pass);
        Ok(())
    }
}