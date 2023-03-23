use anyhow::Result;
use glam::{Mat3, Mat4, Vec3};
use phobos as ph;
use phobos::vk;
use tiny_tokio_actor::ActorRef;

use crate::app::RootActorSystem;
use crate::core::Event;
use crate::gfx::passes::AtmosphereInfo;
use crate::gfx::targets::{RenderTargets, SizeGroup};
use crate::gfx::world::World;
use crate::gfx::{passes, postprocess};
use crate::gui::util::image::Image;
use crate::gui::util::integration::UIIntegration;
use crate::gui::util::size::USize;
use crate::hot_reload::IntoDynamic;
use crate::{gfx, state};

#[derive(Debug, Default)]
pub struct RenderState {
    pub view: Mat4,
    pub projection: Mat4,
    pub projection_view: Mat4,
    pub inverse_projection: Mat4,
    pub inverse_projection_view: Mat4,
    pub inverse_view_rotation: Mat4,
    pub position: Vec3,
    pub atmosphere: AtmosphereInfo,
    pub sun_dir: Vec3,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct WorldRenderer {
    ctx: gfx::SharedContext,
    camera: ActorRef<Event, state::Camera>,
    state: RenderState,
    targets: RenderTargets,
    tonemap: postprocess::Tonemap,
    atmosphere: passes::AtmosphereRenderer,
}

impl WorldRenderer {
    pub fn new(actors: &RootActorSystem, ctx: gfx::SharedContext) -> Result<Self> {
        ph::PipelineBuilder::new("flat_draw")
            .vertex_input(0, vk::VertexInputRate::VERTEX)
            .vertex_attribute(0, 0, vk::Format::R32G32B32_SFLOAT)?
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .blend_attachment_none()
            .depth(true, true, false, vk::CompareOp::LESS)
            .cull_mask(vk::CullModeFlags::NONE)
            .samples(vk::SampleCountFlags::TYPE_8) // TODO: Config, etc.
            .into_dynamic()
            .attach_shader("shaders/src/simple_mesh.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/solid_color.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(actors.shader_reload.clone(), ctx.pipelines.clone())?;

        let mut targets = RenderTargets::new()?;
        targets.set_output_resolution(1, 1)?;

        targets.register_multisampled_color_target(
            "scene_output",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_multisampled_depth_target(
            "depth",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::Format::D32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_color_target(
            "resolved_output",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT,
        )?;

        Ok(Self {
            ctx: ctx.clone(),
            camera: actors.camera.clone(),
            state: RenderState::default(),
            tonemap: postprocess::Tonemap::new(ctx.clone(), &actors, &mut targets)?,
            atmosphere: passes::AtmosphereRenderer::new(ctx.clone(), &actors)?,
            targets,
        })
    }

    pub fn output_name() -> &'static str {
        postprocess::Tonemap::output_name()
    }

    pub fn output_image(&self) -> ph::ImageView {
        self.targets.get_target_view(Self::output_name()).unwrap()
    }

    pub fn resize_target(&mut self, size: USize, ui: &mut UIIntegration) -> Result<Image> {
        self.targets.set_output_resolution(size.x(), size.y())?;
        Ok(ui.register_texture(&self.targets.get_target_view(Self::output_name())?))
    }

    pub fn new_frame(&mut self) {
        self.targets.next_frame();
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.output_image().width() as f32 / self.output_image().height() as f32
    }

    async fn update_render_state(&mut self, world: &World) -> Result<()> {
        self.state.view = self.camera.ask(state::QueryCameraMatrix).await?.0;
        self.state.projection = Mat4::perspective_rh(
            self.camera.ask(state::QueryCameraFOV).await?.to_radians(),
            self.aspect_ratio(),
            0.1,
            100.0,
        );
        // Flip y because Vulkan
        let v = self.state.projection.col_mut(1).y;
        self.state.projection.col_mut(1).y = v * -1.0;
        self.state.position = self.camera.ask(state::QueryCameraPosition).await?.0;
        self.state.projection_view = self.state.projection * self.state.view;
        self.state.inverse_projection_view = self.state.projection_view.inverse();
        self.state.inverse_projection = self.state.projection.inverse();
        self.state.inverse_view_rotation = Mat4::from_mat3(Mat3::from_mat4(self.state.view)).inverse();
        self.state.atmosphere = world.atmosphere;
        self.state.sun_dir = -world.sun_direction.front_direction();
        Ok(())
    }

    pub async fn redraw_world<'s: 'e + 'q, 'q, 'e>(&'s mut self, world: &World) -> Result<(gfx::FrameGraph<'e, 'q>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();
        let mut graph = gfx::FrameGraph::new();
        self.targets.bind_targets(&mut bindings);

        self.update_render_state(world).await?;

        let scene_output = ph::VirtualResource::image("scene_output");
        let depth = ph::VirtualResource::image("depth");
        let resolved_output = ph::VirtualResource::image("resolved_output");
        let tonemapped_output = ph::VirtualResource::image(postprocess::Tonemap::output_name());

        let main_render = ph::PassBuilder::render("final_output")
            .color_attachment(
                &scene_output,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 0.0],
                }),
            )?
            .depth_attachment(
                &depth,
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }),
            )?
            .execute(|cmd, _ifc, _bindings| Ok(cmd))
            .build();

        // 1. Render main geometry pass
        graph.add_pass(main_render);
        // 2. Render atmosphere
        self.atmosphere
            .render(&mut graph, &mut bindings, scene_output.clone(), depth.clone(), &self.state)
            .await?;
        // 3. Resolve MSAA
        let resolve = ph::PassBuilder::render("msaa_resolve")
            .color_attachment(&graph.latest_version(scene_output.clone())?, vk::AttachmentLoadOp::LOAD, None)?
            // We dont currently need depth resolved
            // .depth_attachment(graph.latest_version(depth.clone())?,vk::AttachmentLoadOp::LOAD, None)?
            .resolve(&graph.latest_version(scene_output.clone())?, &resolved_output)
            .build();
        graph.add_pass(resolve);
        // 4. Apply tonemapping
        self.tonemap.render(resolved_output, &mut graph)?;
        // 5. Alias our final result to the expected name
        graph.alias("renderer_output", tonemapped_output);

        Ok((graph, bindings))
    }
}
