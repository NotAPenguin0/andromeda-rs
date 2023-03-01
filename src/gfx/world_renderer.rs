use phobos as ph;

use anyhow::Result;
use glam::Mat4;
use phobos::{GraphicsCmdBuffer, vk};

use tiny_tokio_actor::ActorRef;

use crate::core::{ByteSize, Event};
use crate::{gfx, gui, state};
use crate::app::RootActorSystem;
use crate::hot_reload::IntoDynamic;

#[derive(Debug, Default)]
struct RenderState {
    view: Mat4,
}

#[derive(Debug)]
pub struct WorldRenderer {
    ctx: gfx::SharedContext,
    output: gfx::PairedImageView,
    depth: gfx::PairedImageView,
    deferred_target_delete: ph::DeletionQueue<gfx::PairedImageView>,
    camera: ActorRef<Event, state::Camera>,
    state: RenderState,
}

impl WorldRenderer {
    pub fn new(actors: &RootActorSystem, ctx: gfx::SharedContext) -> Result<Self> {
        ph::PipelineBuilder::new("solid_color")
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .blend_attachment_none()
            .cull_mask(vk::CullModeFlags::NONE)
            .into_dynamic()
            .attach_shader("shaders/src/fullscreen.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/solid_color.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(actors.shader_reload.clone(), ctx.pipelines.clone())?;

        ph::PipelineBuilder::new("flat_draw")
            .vertex_input(0, vk::VertexInputRate::VERTEX)
            .vertex_attribute(0, 0, vk::Format::R32G32B32_SFLOAT)?
            .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR])
            .blend_attachment_none()
            .depth(true, true, false, vk::CompareOp::LESS)
            .cull_mask(vk::CullModeFlags::NONE)
            .into_dynamic()
            .attach_shader("shaders/src/simple_mesh.vert.hlsl", vk::ShaderStageFlags::VERTEX)
            .attach_shader("shaders/src/solid_color.frag.hlsl", vk::ShaderStageFlags::FRAGMENT)
            .build(actors.shader_reload.clone(), ctx.pipelines.clone())?;

        Ok(Self {
            output: Self::allocate_color_target(1920, 1080, ctx.clone())?,
            depth: Self::allocate_depth_target(1920, 1080, ctx.clone())?,
            ctx,
            deferred_target_delete: ph::DeletionQueue::new(4),
            camera: actors.camera.clone(),
            state: RenderState::default(),
        })
    }

    fn allocate_color_target(width: u32, height: u32, ctx: gfx::SharedContext) -> Result<gfx::PairedImageView> {
        gfx::PairedImageView::new(
            ph::Image::new(ctx.device, (*ctx.allocator).clone(), width, height, vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::R8G8B8A8_SRGB)?,
            vk::ImageAspectFlags::COLOR
        )
    }

    fn allocate_depth_target(width: u32, height: u32, ctx: gfx::SharedContext) -> Result<gfx::PairedImageView> {
        gfx::PairedImageView::new(
            ph::Image::new(ctx.device, (*ctx.allocator).clone(), width, height, vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT | vk::ImageUsageFlags::SAMPLED, vk::Format::D32_SFLOAT)?,
            vk::ImageAspectFlags::DEPTH
        )
    }

    pub fn output_image(&self) -> &gfx::PairedImageView {
        &self.output
    }

    pub fn resize_target(&mut self, size: gui::USize, ui: &mut gui::UIIntegration) -> Result<gui::Image> {
        let mut new_target = Self::allocate_color_target(size.x(), size.y(), self.ctx.clone())?;
        std::mem::swap(&mut new_target, &mut self.output);

        let mut new_depth = Self::allocate_depth_target(size.x(), size.y(), self.ctx.clone())?;
        std::mem::swap(&mut new_depth, &mut self.depth);

        self.deferred_target_delete.push(new_depth);
        self.deferred_target_delete.push(new_target);
        Ok(ui.register_texture(&self.output.view))
    }

    pub fn new_frame(&mut self) {
        self.deferred_target_delete.next_frame();
    }

    pub fn aspect_ratio(&self) -> f32 {
        self.output.image.size.width as f32 / self.output.image.size.height as f32
    }

    fn draw_cube<'q>(&mut self, cmd: ph::IncompleteCommandBuffer<'q, ph::domain::All>, ifc: &mut ph::InFlightContext) -> Result<ph::IncompleteCommandBuffer<'q, ph::domain::All>> {
        // We need to allocate a vertex and uniform buffer from the ifc

        const VERTS: [f32; 24] = [-1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, 1.0, -1.0, -1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0];
        const IDX: [u32; 36] = [2, 6, 7, 2, 3, 7, 0, 4, 5, 0, 1, 5, 0, 2, 6, 0, 4, 6, 1, 3, 7, 1, 5, 7, 0, 2, 3, 0, 1, 3, 4, 6, 7, 4, 5, 7];

        let mut vtx = ifc.allocate_scratch_vbo(VERTS.byte_size() as vk::DeviceSize)?;
        let mut idx = ifc.allocate_scratch_ibo(IDX.byte_size() as vk::DeviceSize)?;
        vtx.mapped_slice()?.copy_from_slice(&VERTS);
        idx.mapped_slice()?.copy_from_slice(&IDX);

        let projection = Mat4::perspective_rh(90f32.to_radians(), self.aspect_ratio(), 0.1, 100.0);

        let pv = projection * self.state.view;

        let mut cam_ubo = ifc.allocate_scratch_ubo(pv.byte_size() as vk::DeviceSize)?;
        cam_ubo.mapped_slice::<Mat4>()?.copy_from_slice(std::slice::from_ref(&pv));

        let cmd =
            cmd.bind_graphics_pipeline("flat_draw", self.ctx.pipelines.clone())?
                .viewport(vk::Viewport{
                    x: 0.0,
                    y: 0.0,
                    width: self.output.view.size.width as f32,
                    height: self.output.view.size.height as f32,
                    min_depth: 0.0,
                    max_depth: 0.0,
                })
                .scissor(vk::Rect2D { offset: Default::default(), extent: vk::Extent2D { width: self.output.view.size.width, height: self.output.view.size.height } })
                .bind_new_descriptor_set(0, self.ctx.descriptors.clone(),
                                         ph::DescriptorSetBuilder::with_reflection(self.ctx.pipelines.lock().unwrap().reflection_info("flat_draw")?)
                                             .bind_named_uniform_buffer("Camera", cam_ubo)?
                                             .build())?
                .bind_vertex_buffer(0, vtx)
                .bind_index_buffer(idx, vk::IndexType::UINT32)
                .draw_indexed(IDX.len() as u32, 1, 0, 0, 0);
        Ok(cmd)
    }

    async fn update_render_state(&mut self) -> Result<()> {
        self.state.view = self.camera.ask(state::QueryCameraMatrix).await?.0;
        Ok(())
    }

    /// Conventions for output graph:
    /// - Contains a pass `final_output` which renders to a virtual resource named `output`.
    /// - This resource is bound to the internal output color attachment.
    pub async fn redraw_world<'s: 'e, 'e, 'q>(&'s mut self) -> Result<(ph::PassGraph<'e, 'q, ph::domain::All>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();
        bindings.bind_image("output", self.output.view.clone());
        bindings.bind_image("depth", self.depth.view.clone());

        self.update_render_state().await?;
        let final_output = ph::VirtualResource::image("output");
        let depth = ph::VirtualResource::image("depth");
        let output_pass = ph::PassBuilder::render("final_output")
            .color_attachment(
                final_output.clone(),
                vk::AttachmentLoadOp::CLEAR,
                Some(vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0]}))?
            .depth_attachment(
                depth.clone(), 
                vk::AttachmentLoadOp::CLEAR, 
                Some(vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }))?
            .execute(|cmd, mut ifc, _| {
                self.draw_cube(cmd, &mut ifc)
            })
            .build();

        let graph = ph::PassGraph::new()
            .add_pass(output_pass)?;

        Ok((graph, bindings))
    }
}