use phobos as ph;

use anyhow::Result;
use glam::Mat4;
use phobos::{GraphicsCmdBuffer, vk};

use tiny_tokio_actor::ActorRef;

use crate::core::{ByteSize, Event};
use crate::{gfx, gui, state};
use crate::app::RootActorSystem;
use crate::gfx::postprocess;
use crate::gfx::targets::{RenderTargets, SizeGroup};
use crate::hot_reload::IntoDynamic;

#[derive(Debug, Default)]
struct RenderState {
    view: Mat4,
    projection: Mat4,
}

#[derive(Debug)]
pub struct WorldRenderer {
    ctx: gfx::SharedContext,
    camera: ActorRef<Event, state::Camera>,
    resolve: postprocess::MSAAResolve,
    state: RenderState,
    targets: RenderTargets,
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
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::SampleCountFlags::TYPE_8
        )?;

        targets.register_multisampled_depth_target(
            "depth",
            SizeGroup::OutputResolution,
            ctx.clone(),
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::D32_SFLOAT,
            vk::SampleCountFlags::TYPE_8
        )?;

        Ok(Self {
            ctx: ctx.clone(),
            camera: actors.camera.clone(),
            resolve: postprocess::MSAAResolve::new(&actors, &mut targets,ctx.clone())?,
            state: RenderState::default(),
            targets,
        })
    }

    pub fn output_image(&self) -> ph::ImageView {
        self.targets.get_target_view("msaa_resolve_output").unwrap()
    }

    pub fn resize_target(&mut self, size: gui::USize, ui: &mut gui::UIIntegration) -> Result<gui::Image> {
        self.targets.set_output_resolution(size.x(), size.y())?;
        Ok(ui.register_texture(&self.targets.get_target_view("msaa_resolve_output")?))
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

        let pv = state.projection * state.view;

        let mut cam_ubo = ifc.allocate_scratch_ubo(pv.byte_size() as vk::DeviceSize)?;
        cam_ubo.mapped_slice::<Mat4>()?.copy_from_slice(std::slice::from_ref(&pv));

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
        Ok(())
    }

    /// Conventions for output graph:
    /// - Contains a pass `final_output` which renders to a virtual resource named `output`.
    /// - This resource is bound to the internal output color attachment.
    pub async fn redraw_world<'s: 'e + 'q, 'e, 'q>(&'s mut self) -> Result<(gfx::FrameGraph<'e, 'q>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();
        self.targets.bind_targets(&mut bindings);
        bindings.alias("output", "msaa_resolve_output")?;

        self.update_render_state().await?;
        let scene_output = ph::VirtualResource::image("scene_output");
        let depth = ph::VirtualResource::image("depth");
        let output_pass = ph::PassBuilder::render("final_output")
            .color_attachment(
                scene_output.clone(),
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}))?
            .depth_attachment(
                depth.clone(), 
                vk::AttachmentLoadOp::CLEAR, 
                Some(vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }))?
            .execute(|cmd, mut ifc, _| {
                Self::draw_cube(cmd, &mut ifc, &self.state, self.ctx.clone())
            })
            .build();

        let mut graph = gfx::FrameGraph::new();
        graph.add_pass(output_pass);
        self.resolve.add_pass(scene_output, &mut graph)?;

        Ok((graph, bindings))
    }
}