use phobos as ph;
use phobos::{GraphicsCmdBuffer, vk};

use anyhow::Result;
use crate::app::RootActorSystem;
use crate::gfx;
use crate::hot_reload::IntoDynamic;

#[derive(Debug)]
pub struct MSAAResolve {
    ctx: gfx::SharedContext,
    sampler: ph::Sampler,
}

impl MSAAResolve {
    pub fn new(actors: &RootActorSystem, targets: &mut gfx::RenderTargets, ctx: gfx::SharedContext) -> Result<Self> {
        ph::PipelineBuilder::new("msaa_resolve")
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .cull_mask(vk::CullModeFlags::NONE)
            .depth(false, false, false, vk::CompareOp::ALWAYS)
            .blend_attachment_none()
            .into_dynamic()
            .attach_shader("shaders/src/fullscreen.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/resolve.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(actors.shader_reload.clone(), ctx.pipelines.clone())?;

        targets.register_color_target(Self::output_name(), gfx::SizeGroup::OutputResolution, ctx.clone(), vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::R32G32B32A32_SFLOAT)?;

        Ok(Self {
            sampler: ph::Sampler::default(ctx.device.clone())?,
            ctx,
        })
    }

    pub fn output_name() -> &'static str {
        "msaa_resolve_output"
    }

    pub fn add_pass<'s: 'e + 'q, 'e, 'q>(&'s self, in_resource: ph::VirtualResource, graph: &mut gfx::FrameGraph<'e, 'q>) -> Result<()> {
        let input = graph.latest_version(in_resource)?;
        let out = ph::VirtualResource::image(Self::output_name());
        let pass = ph::PassBuilder::render("msaa_resolve")
            .sample_image(input.clone(), ph::PipelineStage::FRAGMENT_SHADER)
            .color_attachment(out,
                              vk::AttachmentLoadOp::CLEAR,
                              Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}))?
            .execute(move |mut cmd, _, bindings| {
                cmd = cmd.bind_graphics_pipeline("msaa_resolve", self.ctx.pipelines.clone())?;
                cmd = cmd.full_viewport_scissor();
                cmd = cmd.bind_new_descriptor_set(0, self.ctx.descriptors.clone(),
                                                  ph::DescriptorSetBuilder::new()
                                                      .resolve_and_bind_sampled_image(0, input.clone(), &self.sampler, &bindings)?
                                                      .build()
                )?;
                let samples: u32 = 4;
                cmd = cmd.push_constants(vk::ShaderStageFlags::FRAGMENT, 0, std::slice::from_ref(&samples));
                cmd = cmd.draw(6, 1, 0, 0);
                Ok(cmd)
            })
            .build();

        graph.add_pass(pass);
        Ok(())
    }
}