use phobos as ph;

use anyhow::Result;
use glam::{Mat3, Mat4, Vec3};
use phobos::{GraphicsCmdBuffer, vk};

use tiny_tokio_actor::ActorRef;

use crate::core::{ByteSize, Event};
use crate::{gfx, gui, math, state};
use crate::app::RootActorSystem;
use crate::gfx::{passes, postprocess};
use crate::gfx::passes::AtmosphereInfo;
use crate::gfx::targets::{RenderTargets, SizeGroup};
use crate::hot_reload::IntoDynamic;

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
            vk::SampleCountFlags::TYPE_8
        )?;

        targets.register_multisampled_depth_target(
            "depth",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::Format::D32_SFLOAT,
            vk::SampleCountFlags::TYPE_8
        )?;

        targets.register_color_target(
            "resolved_output",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT)?;

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

    pub fn resize_target(&mut self, size: gui::USize, ui: &mut gui::UIIntegration) -> Result<gui::Image> {
        self.targets.set_output_resolution(size.x(), size.y())?;
        Ok(ui.register_texture(&self.targets.get_target_view(Self::output_name())?))
    }

    pub fn new_frame(&mut self) {
        self.targets.next_frame();
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.output_image().size.width as f32 / self.output_image().size.height as f32
    }

    fn draw_cube<'q>(cmd: ph::IncompleteCommandBuffer<'q, ph::domain::All>, ifc: &mut ph::InFlightContext, state: &RenderState, ctx: gfx::SharedContext) -> Result<ph::IncompleteCommandBuffer<'q, ph::domain::All>> {
        // We need to allocate a vertex and uniform buffer from the ifc
        const VERTS: [f32; 108] = [
            -0.5, -0.5, -0.5,
            0.5, -0.5, -0.5,
            0.5,  0.5, -0.5,
            0.5,  0.5, -0.5,
            -0.5,  0.5, -0.5,
            -0.5, -0.5, -0.5,

            -0.5, -0.5,  0.5,
            0.5, -0.5,  0.5,
            0.5,  0.5,  0.5,
            0.5,  0.5,  0.5,
            -0.5,  0.5,  0.5,
            -0.5, -0.5,  0.5,

            -0.5,  0.5,  0.5,
            -0.5,  0.5, -0.5,
            -0.5, -0.5, -0.5,
            -0.5, -0.5, -0.5,
            -0.5, -0.5,  0.5,
            -0.5,  0.5,  0.5,

            0.5,  0.5,  0.5,
            0.5,  0.5, -0.5,
            0.5, -0.5, -0.5,
            0.5, -0.5, -0.5,
            0.5, -0.5,  0.5,
            0.5,  0.5,  0.5,

            -0.5, -0.5, -0.5,
            0.5, -0.5, -0.5,
            0.5, -0.5,  0.5,
            0.5, -0.5,  0.5,
            -0.5, -0.5,  0.5,
            -0.5, -0.5, -0.5,

            -0.5,  0.5, -0.5,
            0.5,  0.5, -0.5,
            0.5,  0.5,  0.5,
            0.5,  0.5,  0.5,
            -0.5,  0.5,  0.5,
            -0.5,  0.5, -0.5,
        ];

        let mut vtx = ifc.allocate_scratch_vbo(VERTS.byte_size() as vk::DeviceSize)?;
        vtx.mapped_slice()?.copy_from_slice(&VERTS);

        let mut cam_ubo = ifc.allocate_scratch_ubo(state.projection_view.byte_size() as vk::DeviceSize)?;
        cam_ubo.mapped_slice::<Mat4>()?.copy_from_slice(std::slice::from_ref(&state.projection_view));

        let cmd =
            cmd.bind_graphics_pipeline("flat_draw", ctx.pipelines.clone())?
                .full_viewport_scissor()
                .bind_new_descriptor_set(0, ctx.descriptors.clone(),
                                         ph::DescriptorSetBuilder::with_reflection(ctx.pipelines.lock().unwrap().reflection_info("flat_draw")?)
                                             .bind_named_uniform_buffer("Camera", cam_ubo)?
                                             .build())?
                .bind_vertex_buffer(0, vtx)
                .draw(36, 1, 0, 0);
        Ok(cmd)
    }

    async fn update_render_state(&mut self) -> Result<()> {
        self.state.view = self.camera.ask(state::QueryCameraMatrix).await?.0;
        self.state.projection = Mat4::perspective_rh(
            self.camera.ask(state::QueryCameraFOV).await?.to_radians(),
            self.aspect_ratio(),
            0.1,
            100.0
        );
        // Flip y because Vulkan
        let v = self.state.projection.col_mut(1).y;
        self.state.projection.col_mut(1).y = v * -1.0;
        self.state.position = self.camera.ask(state::QueryCameraPosition).await?.0;
        self.state.projection_view = self.state.projection * self.state.view;
        self.state.inverse_projection_view = self.state.projection_view.inverse();
        self.state.inverse_projection = self.state.projection.inverse();
        self.state.inverse_view_rotation = Mat4::from_mat3(Mat3::from_mat4(self.state.view)).inverse();
        self.state.atmosphere = AtmosphereInfo::earth();
        self.state.sun_dir = -self.camera.ask(state::QueryCameraVectors).await?.front;
        Ok(())
    }

    pub async fn redraw_world<'s: 'e + 'q, 'e, 'q>(&'s mut self) -> Result<(gfx::FrameGraph<'e, 'q>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();
        let mut graph = gfx::FrameGraph::new();
        self.targets.bind_targets(&mut bindings);

        self.update_render_state().await?;

        let scene_output = ph::VirtualResource::image("scene_output");
        let depth = ph::VirtualResource::image("depth");
        let resolved_output = ph::VirtualResource::image("resolved_output");
        let tonemapped_output = ph::VirtualResource::image(postprocess::Tonemap::output_name());

        let main_render = ph::PassBuilder::render("final_output")
            .color_attachment(
                scene_output.clone(),
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 0.0]}))?
            .depth_attachment(
                depth.clone(), 
                vk::AttachmentLoadOp::CLEAR, 
                Some(vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }))?
            .execute(|cmd, mut ifc, _| {
                Self::draw_cube(cmd, &mut ifc, &self.state, self.ctx.clone())
            })
            .build();

        // 1. Render main geometry pass
        graph.add_pass(main_render);
        // 2. Render atmosphere
        self.atmosphere.render(&mut graph, &mut bindings, scene_output.clone(), depth.clone(), &self.state).await?;
        // 3. Resolve MSAA
        let resolve = ph::PassBuilder::render("msaa_resolve")
            .color_attachment(graph.latest_version(scene_output.clone())?, vk::AttachmentLoadOp::LOAD, None)?
            // We dont currently need depth resolved
            // .depth_attachment(graph.latest_version(depth.clone())?,vk::AttachmentLoadOp::LOAD, None)?
            .resolve(graph.latest_version(scene_output.clone())?, resolved_output.clone())
            .build();
        graph.add_pass(resolve);
        // 4. Apply tonemapping
        self.tonemap.render(resolved_output, &mut graph)?;
        // 5. Alias our final result to the expected name
        graph.alias("renderer_output", tonemapped_output);

        Ok((graph, bindings))
    }
}