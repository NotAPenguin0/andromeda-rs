use anyhow::Result;
use glam::{Mat3, Mat4, Vec3};
use phobos as ph;
use phobos::vk;

use crate::gfx;
use crate::gfx::renderer::passes::atmosphere::AtmosphereRenderer;
use crate::gfx::renderer::passes::terrain::TerrainRenderer;
use crate::gfx::renderer::postprocess::tonemap::Tonemap;
use crate::gfx::resource::normal_map::NormalMap;
use crate::gfx::util::graph::FrameGraph;
use crate::gfx::util::targets::{RenderTargets, SizeGroup};
use crate::gui::util::image_provider::RenderTargetImageProvider;
use crate::gui::util::integration::UIIntegration;
use crate::hot_reload::IntoDynamic;
use crate::state::world::World;

/// Stores world state in a format that the renderer needs, such as
/// normalized direction vectors instead of rotations,
/// camera view and projection matrices, etc.
#[derive(Debug, Default)]
pub struct RenderState {
    /// Camera view matrix
    pub view: Mat4,
    /// Camera projection matrix
    pub projection: Mat4,
    /// Premultiplied `projection * view` matrix
    pub projection_view: Mat4,
    /// Inverse of the projection matrix
    pub inverse_projection: Mat4,
    /// Inverse of `projection * view`
    pub inverse_projection_view: Mat4,
    /// Inverse of the camera's view matrix with the translation component removed
    pub inverse_view_rotation: Mat4,
    /// Direction vector pointing away from the sun
    pub sun_direction: Vec3,
    /// Camera position in world space
    pub cam_position: Vec3,
}

/// The world renderer is responsible for all the rendering logic
/// of the scene.
#[allow(dead_code)]
#[derive(Debug)]
pub struct WorldRenderer {
    targets: RenderTargets,
    ctx: gfx::SharedContext,
    state: RenderState,
    tonemap: Tonemap,
    atmosphere: AtmosphereRenderer,
    terrain: TerrainRenderer,
}

impl WorldRenderer {
    /// Initialize the world renderer.
    /// This will create pipelines, initialize render targets and create
    /// other necessary objects.
    pub fn new(ctx: gfx::SharedContext) -> Result<Self> {
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
            .build(ctx.shader_reload.clone(), ctx.pipelines.clone())?;

        NormalMap::init_pipelines(ctx.clone())?;

        let mut targets = RenderTargets::new(ctx.clone())?;
        targets.set_output_resolution(1, 1)?;

        targets.register_multisampled_color_target(
            "scene_output",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT,
            vk::Format::R32G32B32A32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_multisampled_depth_target(
            "depth",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            vk::Format::D32_SFLOAT,
            vk::SampleCountFlags::TYPE_8,
        )?;

        targets.register_color_target(
            "resolved_output",
            SizeGroup::OutputResolution,
            vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
            vk::Format::R32G32B32A32_SFLOAT,
        )?;

        Ok(Self {
            ctx: ctx.clone(),
            state: RenderState::default(),
            tonemap: Tonemap::new(ctx.clone(), &mut targets)?,
            atmosphere: AtmosphereRenderer::new(ctx.clone())?,
            terrain: TerrainRenderer::new(ctx)?,
            targets,
        })
    }

    /// Name of the rendertarget that is the final output of the
    /// scene rendering.
    pub fn output_name() -> &'static str {
        Tonemap::output_name()
    }

    /// Get an `ImageView` pointing to the final output of scene rendering.
    pub fn output_image(&self) -> ph::ImageView {
        self.targets.get_target_view(Self::output_name()).unwrap()
    }

    /// Update deferred deletion queues.
    pub fn new_frame(&mut self) {
        self.targets.next_frame();
    }

    /// Get the current render aspect ratio.
    pub fn aspect_ratio(&self) -> f32 {
        self.output_image().width() as f32 / self.output_image().height() as f32
    }

    /// Create an image provider that points to this renderer's render targets.
    pub fn image_provider<'s, 'i>(
        &'s mut self,
        ui: &'i mut UIIntegration,
    ) -> RenderTargetImageProvider<'s, 'i, 'static> {
        RenderTargetImageProvider {
            targets: &mut self.targets,
            integration: ui,
            name: Self::output_name(),
        }
    }

    /// Updates the internal render state with data from the world.
    fn update_render_state(&mut self, world: &World) -> Result<()> {
        let camera = world.camera.read().unwrap();
        self.state.view = camera.matrix();
        self.state.projection =
            Mat4::perspective_rh(camera.fov().to_radians(), self.aspect_ratio(), 0.1, 10000000.0);
        // Flip y because Vulkan
        let v = self.state.projection.col_mut(1).y;
        self.state.projection.col_mut(1).y = v * -1.0;
        self.state.cam_position = camera.position().0;
        self.state.projection_view = self.state.projection * self.state.view;
        self.state.inverse_projection_view = self.state.projection_view.inverse();
        self.state.inverse_projection = self.state.projection.inverse();
        self.state.inverse_view_rotation =
            Mat4::from_mat3(Mat3::from_mat4(self.state.view)).inverse();
        self.state.sun_direction = -world.sun_direction.front_direction();
        Ok(())
    }

    /// Redraw the world. Returns a frame graph and physical resource bindings that
    /// can be submitted to the GPU.
    pub fn redraw_world<'s: 'e + 'q, 'world: 'e + 'q, 'q, 'e>(
        &'s mut self,
        world: &'world World,
    ) -> Result<(FrameGraph<'e, 'q>, ph::PhysicalResourceBindings)> {
        let mut bindings = ph::PhysicalResourceBindings::new();
        let mut graph = FrameGraph::new();
        self.targets.bind_targets(&mut bindings);

        self.update_render_state(world)?;

        let scene_output = ph::VirtualResource::image("scene_output");
        let depth = ph::VirtualResource::image("depth");
        let resolved_output = ph::VirtualResource::image("resolved_output");
        let tonemapped_output = ph::VirtualResource::image(Tonemap::output_name());

        // 1. Render terrain
        self.terrain
            .render(&mut graph, &scene_output, &depth, world, &self.state)?;
        // 2. Render atmosphere
        self.atmosphere
            .render(&mut graph, &scene_output, &depth, world, &self.state)?;
        // 3. Resolve MSAA
        let resolve = ph::PassBuilder::render("msaa_resolve")
            .color_attachment(
                &graph.latest_version(&scene_output)?,
                vk::AttachmentLoadOp::LOAD,
                None,
            )?
            // We dont currently need depth resolved
            // .depth_attachment(graph.latest_version(depth.clone())?,vk::AttachmentLoadOp::LOAD, None)?
            .resolve(&graph.latest_version(&scene_output)?, &resolved_output)
            .build();
        graph.add_pass(resolve);
        // 4. Apply tonemapping
        self.tonemap.render(&mut graph, &resolved_output)?;
        // 5. Alias our final result to the expected name
        graph.alias("renderer_output", tonemapped_output);

        Ok((graph, bindings))
    }
}
